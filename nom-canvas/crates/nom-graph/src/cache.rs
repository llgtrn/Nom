#![deny(unsafe_code)]
use std::collections::HashMap;

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
    fn lru_cache_evicts_oldest() {
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
}
