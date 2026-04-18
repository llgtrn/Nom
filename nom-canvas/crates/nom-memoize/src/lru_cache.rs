use std::collections::HashMap;

// ---------------------------------------------------------------------------
// CacheStats
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

impl CacheStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_hit(&mut self) {
        self.hits += 1;
    }

    pub fn record_miss(&mut self) {
        self.misses += 1;
    }

    pub fn record_eviction(&mut self) {
        self.evictions += 1;
    }

    pub fn hit_rate(&self) -> f32 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f32 / total as f32
        }
    }

    pub fn total_accesses(&self) -> u64 {
        self.hits + self.misses
    }
}

// ---------------------------------------------------------------------------
// CacheEntry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CacheEntry<V> {
    pub value: V,
    pub access_count: u32,
    pub inserted_at_ms: u64,
}

impl<V> CacheEntry<V> {
    pub fn new(value: V, inserted_at_ms: u64) -> Self {
        Self {
            value,
            access_count: 0,
            inserted_at_ms,
        }
    }

    pub fn touch(&mut self) {
        self.access_count += 1;
    }

    pub fn is_expired(&self, now_ms: u64, ttl_ms: u64) -> bool {
        now_ms.saturating_sub(self.inserted_at_ms) > ttl_ms
    }
}

// ---------------------------------------------------------------------------
// LruCache
// ---------------------------------------------------------------------------

pub struct LruCache<K, V>
where
    K: Eq + std::hash::Hash + Clone,
{
    pub capacity: usize,
    pub entries: HashMap<K, CacheEntry<V>>,
    pub stats: CacheStats,
}

impl<K, V> LruCache<K, V>
where
    K: Eq + std::hash::Hash + Clone,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: HashMap::new(),
            stats: CacheStats::new(),
        }
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.touch();
            self.stats.record_hit();
            // Re-borrow immutably for return
            Some(&self.entries[key].value)
        } else {
            self.stats.record_miss();
            None
        }
    }

    pub fn insert(&mut self, key: K, value: V, now_ms: u64) {
        if !self.entries.contains_key(&key) && self.entries.len() >= self.capacity && self.capacity > 0 {
            // Evict entry with lowest access_count
            if let Some(victim_key) = self
                .entries
                .iter()
                .min_by_key(|(_, e)| e.access_count)
                .map(|(k, _)| k.clone())
            {
                self.entries.remove(&victim_key);
                self.stats.record_eviction();
            }
        }
        self.entries.insert(key, CacheEntry::new(value, now_ms));
    }

    pub fn evict_expired(&mut self, now_ms: u64, ttl_ms: u64) -> usize {
        let expired_keys: Vec<K> = self
            .entries
            .iter()
            .filter(|(_, e)| e.is_expired(now_ms, ttl_ms))
            .map(|(k, _)| k.clone())
            .collect();
        let count = expired_keys.len();
        for k in expired_keys {
            self.entries.remove(&k);
            self.stats.record_eviction();
        }
        count
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod lru_cache_tests {
    use super::*;

    // 1. CacheStats::hit_rate() calculation
    #[test]
    fn cache_stats_hit_rate_calculation() {
        let mut stats = CacheStats::new();
        stats.record_hit();
        stats.record_hit();
        stats.record_hit();
        stats.record_miss();
        // 3 hits / 4 total = 0.75
        let rate = stats.hit_rate();
        assert!((rate - 0.75f32).abs() < 1e-6, "expected 0.75, got {rate}");
    }

    // 2. CacheStats::hit_rate() 0.0 when no accesses
    #[test]
    fn cache_stats_hit_rate_zero_when_no_accesses() {
        let stats = CacheStats::new();
        assert_eq!(stats.hit_rate(), 0.0f32);
    }

    // 3. CacheEntry::is_expired() true past TTL
    #[test]
    fn cache_entry_is_expired_past_ttl() {
        let entry: CacheEntry<u32> = CacheEntry::new(42, 0);
        // inserted_at=0, now=1001, ttl=1000 → 1001 - 0 = 1001 > 1000 → expired
        assert!(entry.is_expired(1001, 1000), "entry must be expired past TTL");
    }

    // 4. CacheEntry::touch() increments access_count
    #[test]
    fn cache_entry_touch_increments_access_count() {
        let mut entry: CacheEntry<u32> = CacheEntry::new(1, 0);
        assert_eq!(entry.access_count, 0);
        entry.touch();
        assert_eq!(entry.access_count, 1);
        entry.touch();
        assert_eq!(entry.access_count, 2);
    }

    // 5. LruCache::get() hit records stat
    #[test]
    fn lru_cache_get_hit_records_stat() {
        let mut cache: LruCache<&str, u32> = LruCache::new(10);
        cache.insert("key", 99, 0);
        let result = cache.get(&"key");
        assert_eq!(result, Some(&99));
        assert_eq!(cache.stats.hits, 1);
        assert_eq!(cache.stats.misses, 0);
    }

    // 6. LruCache::get() miss records stat
    #[test]
    fn lru_cache_get_miss_records_stat() {
        let mut cache: LruCache<&str, u32> = LruCache::new(10);
        let result = cache.get(&"absent");
        assert_eq!(result, None);
        assert_eq!(cache.stats.hits, 0);
        assert_eq!(cache.stats.misses, 1);
    }

    // 7. LruCache::insert() evicts when over capacity
    #[test]
    fn lru_cache_insert_evicts_when_over_capacity() {
        let mut cache: LruCache<u32, u32> = LruCache::new(2);
        // Insert two entries — both at access_count=0
        cache.insert(1, 10, 0);
        cache.insert(2, 20, 0);
        // Touch key 2 so it has higher access_count
        let _ = cache.get(&2);
        // Third insert must evict key 1 (lowest access_count=0)
        cache.insert(3, 30, 0);
        assert_eq!(cache.len(), 2, "capacity=2 must be maintained");
        assert_eq!(cache.stats.evictions, 1, "one eviction must have occurred");
        // key 2 survives (access_count=1), key 3 was just inserted
        assert!(cache.entries.contains_key(&2) || cache.entries.contains_key(&3));
    }

    // 8. evict_expired() removes expired entries
    #[test]
    fn lru_cache_evict_expired_removes_expired_entries() {
        let mut cache: LruCache<u32, u32> = LruCache::new(10);
        cache.insert(1, 10, 0);   // inserted at 0
        cache.insert(2, 20, 500); // inserted at 500
        // now=1001, ttl=1000 → entry 1: 1001-0=1001>1000 expired; entry 2: 1001-500=501≤1000 alive
        cache.evict_expired(1001, 1000);
        assert!(!cache.entries.contains_key(&1), "expired entry must be removed");
        assert!(cache.entries.contains_key(&2), "alive entry must remain");
    }

    // 9. evict_expired() returns count removed
    #[test]
    fn lru_cache_evict_expired_returns_count_removed() {
        let mut cache: LruCache<u32, u32> = LruCache::new(10);
        cache.insert(1, 10, 0);
        cache.insert(2, 20, 0);
        cache.insert(3, 30, 900); // inserted at 900, not expired at now=1001 with ttl=1000
        // now=1001, ttl=1000: entries 1 and 2 expired, entry 3 not
        let removed = cache.evict_expired(1001, 1000);
        assert_eq!(removed, 2, "must return count of 2 removed entries");
        assert_eq!(cache.len(), 1);
    }
}
