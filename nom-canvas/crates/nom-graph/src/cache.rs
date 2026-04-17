#![deny(unsafe_code)]
use std::collections::HashMap;
use crate::node::NodeId;

/// ComfyUI 4-tier cache hierarchy
/// Tier 0: NullCache — no caching
/// Tier 1: BasicCache — exact key match only
/// Tier 2: LruCache — LRU eviction at capacity
/// Tier 3: RAMPressureCache — evicts when system RAM is low

#[derive(Clone, Debug)]
pub enum CachedValue {
    Bytes(Vec<u8>),
    String(String),
    Json(String),
}

pub trait ExecutionCache: Send + Sync {
    fn get(&self, key: u64) -> Option<CachedValue>;
    fn put(&mut self, key: u64, value: CachedValue);
    fn invalidate(&mut self, key: u64);
    fn clear(&mut self);
    fn len(&self) -> usize;
}

/// Tier 0: No caching — always re-execute
pub struct NullCache;
impl ExecutionCache for NullCache {
    fn get(&self, _key: u64) -> Option<CachedValue> { None }
    fn put(&mut self, _key: u64, _value: CachedValue) {}
    fn invalidate(&mut self, _key: u64) {}
    fn clear(&mut self) {}
    fn len(&self) -> usize { 0 }
}

/// Tier 1: Basic cache — unbounded HashMap
pub struct BasicCache { data: HashMap<u64, CachedValue> }
impl BasicCache { pub fn new() -> Self { Self { data: HashMap::new() } } }
impl Default for BasicCache { fn default() -> Self { Self::new() } }
impl ExecutionCache for BasicCache {
    fn get(&self, key: u64) -> Option<CachedValue> { self.data.get(&key).cloned() }
    fn put(&mut self, key: u64, value: CachedValue) { self.data.insert(key, value); }
    fn invalidate(&mut self, key: u64) { self.data.remove(&key); }
    fn clear(&mut self) { self.data.clear(); }
    fn len(&self) -> usize { self.data.len() }
}

/// Tier 2: LRU cache — fixed capacity with LRU eviction
pub struct LruCache {
    capacity: usize,
    data: HashMap<u64, CachedValue>,
    order: Vec<u64>, // front = LRU (oldest), back = MRU (newest)
}

impl LruCache {
    pub fn new(capacity: usize) -> Self {
        Self { capacity, data: HashMap::new(), order: Vec::new() }
    }
    fn touch(&mut self, key: u64) {
        self.order.retain(|k| *k != key);
        self.order.push(key);
    }
}

impl ExecutionCache for LruCache {
    fn get(&self, key: u64) -> Option<CachedValue> { self.data.get(&key).cloned() }
    fn put(&mut self, key: u64, value: CachedValue) {
        if self.data.len() >= self.capacity && !self.data.contains_key(&key) {
            if let Some(lru_key) = self.order.first().cloned() {
                self.order.remove(0);
                self.data.remove(&lru_key);
            }
        }
        self.data.insert(key, value);
        self.touch(key);
    }
    fn invalidate(&mut self, key: u64) {
        self.data.remove(&key);
        self.order.retain(|k| *k != key);
    }
    fn clear(&mut self) { self.data.clear(); self.order.clear(); }
    fn len(&self) -> usize { self.data.len() }
}

/// Tier 3: RAM-pressure-aware cache — evicts oldest 25% when len > threshold
pub struct RamPressureCache {
    threshold: usize,
    data: HashMap<u64, CachedValue>,
    order: Vec<u64>,
}

impl RamPressureCache {
    pub fn new(threshold: usize) -> Self {
        Self { threshold, data: HashMap::new(), order: Vec::new() }
    }
    fn evict_if_needed(&mut self) {
        if self.data.len() >= self.threshold {
            let evict_count = self.threshold / 4;
            let to_evict: Vec<u64> = self.order.drain(..evict_count.min(self.order.len())).collect();
            for key in to_evict { self.data.remove(&key); }
        }
    }
}

impl ExecutionCache for RamPressureCache {
    fn get(&self, key: u64) -> Option<CachedValue> { self.data.get(&key).cloned() }
    fn put(&mut self, key: u64, value: CachedValue) {
        self.evict_if_needed();
        self.data.insert(key, value);
        self.order.retain(|k| *k != key);
        self.order.push(key);
    }
    fn invalidate(&mut self, key: u64) {
        self.data.remove(&key);
        self.order.retain(|k| *k != key);
    }
    fn clear(&mut self) { self.data.clear(); self.order.clear(); }
    fn len(&self) -> usize { self.data.len() }
}

/// Hierarchical cache: try L1 (fast LRU) then L2 (larger RAM-pressure)
pub struct HierarchicalCache {
    pub l1: LruCache,
    pub l2: RamPressureCache,
}

impl HierarchicalCache {
    pub fn new(l1_cap: usize, l2_threshold: usize) -> Self {
        Self { l1: LruCache::new(l1_cap), l2: RamPressureCache::new(l2_threshold) }
    }
}

impl ExecutionCache for HierarchicalCache {
    fn get(&self, key: u64) -> Option<CachedValue> {
        self.l1.get(key).or_else(|| self.l2.get(key))
    }
    fn put(&mut self, key: u64, value: CachedValue) {
        self.l1.put(key, value.clone());
        self.l2.put(key, value);
    }
    fn invalidate(&mut self, key: u64) {
        self.l1.invalidate(key);
        self.l2.invalidate(key);
    }
    fn clear(&mut self) { self.l1.clear(); self.l2.clear(); }
    fn len(&self) -> usize { self.l1.len() }
}

/// 4-tier cache strategy — ComfyUI execution model pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheStrategy {
    NoCache,
    Lru { capacity: usize },
    RamPressure { max_entries: usize },
    Classic,  // IS_CHANGED-gated
}

/// IS_CHANGED flag per node — tracks whether node needs recomputation.
#[derive(Debug, Default)]
pub struct ChangedFlags {
    flags: HashMap<NodeId, bool>,
}

impl ChangedFlags {
    pub fn new() -> Self { Self::default() }
    pub fn mark_changed(&mut self, id: NodeId) { self.flags.insert(id, true); }
    pub fn mark_clean(&mut self, id: NodeId) { self.flags.insert(id, false); }
    /// Nodes are changed by default (unknown == needs recompute).
    pub fn is_changed(&self, id: &NodeId) -> bool { self.flags.get(id).copied().unwrap_or(true) }
    pub fn changed_count(&self) -> usize { self.flags.values().filter(|&&v| v).count() }
}

/// Strategy-backed node-level result cache.
pub struct NodeCache {
    pub strategy: CacheStrategy,
    entries: HashMap<NodeId, CachedValue>,
    lru_order: Vec<NodeId>,
}

impl NodeCache {
    pub fn new(strategy: CacheStrategy) -> Self {
        Self { strategy, entries: HashMap::new(), lru_order: Vec::new() }
    }

    pub fn get(&mut self, id: &NodeId) -> Option<&CachedValue> {
        if self.strategy == CacheStrategy::NoCache { return None; }
        if self.entries.contains_key(id) {
            self.lru_order.retain(|x| x != id);
            self.lru_order.push(id.clone());
        }
        self.entries.get(id)
    }

    pub fn put(&mut self, id: NodeId, output: CachedValue) {
        if self.strategy == CacheStrategy::NoCache { return; }
        if let CacheStrategy::Lru { capacity } = self.strategy {
            if self.entries.len() >= capacity && !self.entries.contains_key(&id) {
                if let Some(oldest) = self.lru_order.first().cloned() {
                    self.entries.remove(&oldest);
                    self.lru_order.remove(0);
                }
            }
        }
        self.lru_order.retain(|x| x != &id);
        self.lru_order.push(id.clone());
        self.entries.insert(id, output);
    }

    pub fn evict(&mut self, id: &NodeId) {
        self.entries.remove(id);
        self.lru_order.retain(|x| x != id);
    }

    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_cache_never_hits() {
        let cache = NullCache;
        assert!(cache.get(42).is_none());
    }

    #[test]
    fn basic_cache_get_put() {
        let mut cache = BasicCache::new();
        cache.put(1, CachedValue::String("hello".into()));
        assert!(cache.get(1).is_some());
        cache.invalidate(1);
        assert!(cache.get(1).is_none());
    }

    #[test]
    fn lru_cache_evicts_oldest() {
        let mut cache = LruCache::new(3);
        cache.put(1, CachedValue::String("a".into()));
        cache.put(2, CachedValue::String("b".into()));
        cache.put(3, CachedValue::String("c".into()));
        cache.put(4, CachedValue::String("d".into())); // should evict key 1
        assert!(cache.get(1).is_none(), "key 1 should have been evicted");
        assert!(cache.get(4).is_some());
    }

    #[test]
    fn ram_pressure_evicts_batch() {
        let mut cache = RamPressureCache::new(4);
        for i in 0..4u64 { cache.put(i, CachedValue::String(format!("{i}"))); }
        cache.put(4, CachedValue::String("4".into())); // triggers eviction of 25%
        assert!(cache.len() <= 4);
    }

    #[test]
    fn hierarchical_cache_l1_first() {
        let mut cache = HierarchicalCache::new(10, 100);
        cache.put(99, CachedValue::Bytes(vec![1, 2, 3]));
        assert!(cache.get(99).is_some());
        cache.clear();
        assert!(cache.get(99).is_none());
    }

    #[test]
    fn basic_cache_stores_and_retrieves() {
        let mut cache = BasicCache::new();
        cache.put(7, CachedValue::Bytes(vec![0xde, 0xad]));
        let val = cache.get(7).expect("should retrieve stored value");
        match val {
            CachedValue::Bytes(b) => assert_eq!(b, vec![0xde, 0xad]),
            _ => panic!("wrong variant"),
        }
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn lru_cache_evicts_oldest_at_capacity() {
        let mut cache = LruCache::new(2);
        cache.put(10, CachedValue::String("first".into()));
        cache.put(20, CachedValue::String("second".into()));
        // Adding a third entry must evict key 10 (oldest).
        cache.put(30, CachedValue::String("third".into()));
        assert!(cache.get(10).is_none(), "oldest entry should be evicted");
        assert!(cache.get(20).is_some());
        assert!(cache.get(30).is_some());
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn ram_pressure_cache_creates() {
        let cache = RamPressureCache::new(8);
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn hierarchical_cache_l1_hit() {
        // Items stored via put() land in both L1 and L2.
        // A get() should find them via L1 (checked first).
        let mut cache = HierarchicalCache::new(4, 100);
        cache.put(42, CachedValue::Json(r#"{"x":1}"#.into()));
        // L1 hit
        assert!(cache.l1.get(42).is_some(), "L1 should hold the value");
        // L2 also holds it
        assert!(cache.l2.get(42).is_some(), "L2 should hold the value");
        // Top-level get returns something
        let result = cache.get(42);
        assert!(result.is_some());
        match result.unwrap() {
            CachedValue::Json(s) => assert_eq!(s, r#"{"x":1}"#),
            _ => panic!("wrong variant"),
        }
    }

    // --- 4-tier CacheStrategy + ChangedFlags tests ---

    #[test]
    fn cache_no_cache_strategy_always_misses() {
        let mut cache = NodeCache::new(CacheStrategy::NoCache);
        cache.put("n1".to_string(), CachedValue::String("v".into()));
        // NoCache never stores, so get must return None
        assert!(cache.get(&"n1".to_string()).is_none());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn cache_lru_evicts_oldest() {
        let mut cache = NodeCache::new(CacheStrategy::Lru { capacity: 2 });
        cache.put("a".to_string(), CachedValue::String("1".into()));
        cache.put("b".to_string(), CachedValue::String("2".into()));
        // Adding "c" must evict "a" (oldest, LRU)
        cache.put("c".to_string(), CachedValue::String("3".into()));
        assert!(cache.get(&"a".to_string()).is_none(), "a should have been evicted");
        assert!(cache.get(&"b".to_string()).is_some());
        assert!(cache.get(&"c".to_string()).is_some());
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn cache_classic_stores_and_retrieves() {
        let mut cache = NodeCache::new(CacheStrategy::Classic);
        cache.put("node1".to_string(), CachedValue::Bytes(vec![1, 2, 3]));
        let val = cache.get(&"node1".to_string()).expect("Classic cache should store and retrieve");
        match val {
            CachedValue::Bytes(b) => assert_eq!(b, &[1u8, 2, 3]),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn changed_flags_default_to_changed() {
        let flags = ChangedFlags::new();
        // Unknown nodes are considered changed (needs recompute)
        assert!(flags.is_changed(&"unknown_node".to_string()));
        assert_eq!(flags.changed_count(), 0); // no explicit marks yet
    }

    #[test]
    fn changed_flags_mark_clean() {
        let mut flags = ChangedFlags::new();
        flags.mark_changed("n1".to_string());
        assert!(flags.is_changed(&"n1".to_string()));
        assert_eq!(flags.changed_count(), 1);
        flags.mark_clean("n1".to_string());
        assert!(!flags.is_changed(&"n1".to_string()));
        assert_eq!(flags.changed_count(), 0);
    }
}
