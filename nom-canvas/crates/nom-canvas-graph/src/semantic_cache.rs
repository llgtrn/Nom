use std::collections::HashMap;

// ---------------------------------------------------------------------------
// SemanticKey
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SemanticKey {
    pub namespace: String,
    pub name: String,
    pub version: u32,
}

impl SemanticKey {
    pub fn new(namespace: impl Into<String>, name: impl Into<String>, version: u32) -> Self {
        Self { namespace: namespace.into(), name: name.into(), version }
    }

    pub fn cache_key(&self) -> String {
        format!("{}::{}@{}", self.namespace, self.name, self.version)
    }

    pub fn is_versioned(&self) -> bool {
        self.version > 0
    }
}

// ---------------------------------------------------------------------------
// SemanticEntry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SemanticEntry<V: Clone> {
    pub key: SemanticKey,
    pub value: V,
    pub hit_count: u32,
}

impl<V: Clone> SemanticEntry<V> {
    pub fn new(key: SemanticKey, value: V) -> Self {
        Self { key, value, hit_count: 0 }
    }

    pub fn record_hit(&mut self) {
        self.hit_count += 1;
    }

    pub fn is_hot(&self) -> bool {
        self.hit_count > 5
    }
}

// ---------------------------------------------------------------------------
// CacheEviction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheEviction {
    LRU,
    LFU,
    TTL(u64),
}

impl CacheEviction {
    pub fn policy_name(&self) -> &'static str {
        match self {
            CacheEviction::LRU => "lru",
            CacheEviction::LFU => "lfu",
            CacheEviction::TTL(_) => "ttl",
        }
    }
}

// ---------------------------------------------------------------------------
// SemanticCache
// ---------------------------------------------------------------------------

pub struct SemanticCache<V: Clone> {
    pub entries: HashMap<String, SemanticEntry<V>>,
    pub eviction: CacheEviction,
    pub max_size: usize,
}

impl<V: Clone> SemanticCache<V> {
    pub fn new(eviction: CacheEviction, max_size: usize) -> Self {
        Self { entries: HashMap::new(), eviction, max_size }
    }

    pub fn insert(&mut self, key: SemanticKey, value: V) {
        if self.entries.len() >= self.max_size {
            self.evict_one();
        }
        let cache_key = key.cache_key();
        self.entries.insert(cache_key, SemanticEntry::new(key, value));
    }

    pub fn get(&mut self, key: &SemanticKey) -> Option<&V> {
        let cache_key = key.cache_key();
        if let Some(entry) = self.entries.get_mut(&cache_key) {
            entry.record_hit();
            Some(&entry.value)
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn evict_one(&mut self) {
        if let Some(k) = self.entries.keys().next().cloned() {
            self.entries.remove(&k);
        }
    }
}

// ---------------------------------------------------------------------------
// CacheStats
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub total_hits: u64,
    pub total_misses: u64,
    pub eviction_count: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.total_hits + self.total_misses;
        if total == 0 {
            0.0
        } else {
            self.total_hits as f64 / total as f64
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod semantic_cache_tests {
    use super::*;

    fn key(ns: &str, name: &str, ver: u32) -> SemanticKey {
        SemanticKey::new(ns, name, ver)
    }

    // 1. cache_key format
    #[test]
    fn test_cache_key_format() {
        let k = key("graph", "node", 3);
        assert_eq!(k.cache_key(), "graph::node@3");
    }

    // 2. is_versioned
    #[test]
    fn test_is_versioned() {
        assert!(!key("ns", "x", 0).is_versioned());
        assert!(key("ns", "x", 1).is_versioned());
    }

    // 3. record_hit + is_hot
    #[test]
    fn test_entry_record_hit_and_is_hot() {
        let mut entry = SemanticEntry::new(key("ns", "x", 1), 42u32);
        assert!(!entry.is_hot());
        for _ in 0..6 {
            entry.record_hit();
        }
        assert_eq!(entry.hit_count, 6);
        assert!(entry.is_hot());
    }

    // 4. eviction policy_name
    #[test]
    fn test_eviction_policy_name() {
        assert_eq!(CacheEviction::LRU.policy_name(), "lru");
        assert_eq!(CacheEviction::LFU.policy_name(), "lfu");
        assert_eq!(CacheEviction::TTL(300).policy_name(), "ttl");
    }

    // 5. insert + len
    #[test]
    fn test_cache_insert_and_len() {
        let mut cache: SemanticCache<u32> = SemanticCache::new(CacheEviction::LRU, 10);
        assert_eq!(cache.len(), 0);
        cache.insert(key("a", "b", 1), 99);
        assert_eq!(cache.len(), 1);
        cache.insert(key("a", "c", 2), 100);
        assert_eq!(cache.len(), 2);
    }

    // 6. get increments hit_count
    #[test]
    fn test_cache_get_increments_hit() {
        let mut cache: SemanticCache<&str> = SemanticCache::new(CacheEviction::LFU, 10);
        let k = key("ns", "val", 1);
        cache.insert(k.clone(), "hello");
        let _ = cache.get(&k);
        let _ = cache.get(&k);
        let entry = cache.entries.get(&k.cache_key()).unwrap();
        assert_eq!(entry.hit_count, 2);
    }

    // 7. evict_one reduces len
    #[test]
    fn test_evict_one_reduces_len() {
        let mut cache: SemanticCache<i32> = SemanticCache::new(CacheEviction::LRU, 10);
        cache.insert(key("x", "a", 1), 1);
        cache.insert(key("x", "b", 2), 2);
        assert_eq!(cache.len(), 2);
        cache.evict_one();
        assert_eq!(cache.len(), 1);
    }

    // 8. max_size eviction: insert 3 into max=2 → len=2
    #[test]
    fn test_max_size_eviction() {
        let mut cache: SemanticCache<u8> = SemanticCache::new(CacheEviction::LRU, 2);
        cache.insert(key("n", "a", 1), 1);
        cache.insert(key("n", "b", 2), 2);
        cache.insert(key("n", "c", 3), 3); // triggers evict_one
        assert_eq!(cache.len(), 2);
    }

    // 9. CacheStats hit_rate
    #[test]
    fn test_stats_hit_rate() {
        let zero = CacheStats::default();
        assert_eq!(zero.hit_rate(), 0.0);

        let stats = CacheStats { total_hits: 3, total_misses: 1, eviction_count: 0 };
        assert!((stats.hit_rate() - 0.75).abs() < 1e-9);

        let all_miss = CacheStats { total_hits: 0, total_misses: 5, eviction_count: 0 };
        assert_eq!(all_miss.hit_rate(), 0.0);
    }
}
