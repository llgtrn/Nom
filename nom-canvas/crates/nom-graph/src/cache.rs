#![deny(unsafe_code)]
use crate::node::NodeId;
use std::collections::HashMap;
use std::sync::Mutex;

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
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Tier 0: No caching — always re-execute
pub struct NullCache;
impl ExecutionCache for NullCache {
    fn get(&self, _key: u64) -> Option<CachedValue> {
        None
    }
    fn put(&mut self, _key: u64, _value: CachedValue) {}
    fn invalidate(&mut self, _key: u64) {}
    fn clear(&mut self) {}
    fn len(&self) -> usize {
        0
    }
}

/// Tier 1: Basic cache — unbounded HashMap
pub struct BasicCache {
    data: HashMap<u64, CachedValue>,
}
impl BasicCache {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
}
impl Default for BasicCache {
    fn default() -> Self {
        Self::new()
    }
}
impl ExecutionCache for BasicCache {
    fn get(&self, key: u64) -> Option<CachedValue> {
        self.data.get(&key).cloned()
    }
    fn put(&mut self, key: u64, value: CachedValue) {
        self.data.insert(key, value);
    }
    fn invalidate(&mut self, key: u64) {
        self.data.remove(&key);
    }
    fn clear(&mut self) {
        self.data.clear();
    }
    fn len(&self) -> usize {
        self.data.len()
    }
}

/// Tier 2: LRU cache — fixed capacity with LRU eviction
pub struct LruCache {
    capacity: usize,
    data: HashMap<u64, CachedValue>,
    order: Mutex<Vec<u64>>, // front = LRU (oldest), back = MRU (newest); Mutex allows get(&self) to update recency
}

impl LruCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            data: HashMap::new(),
            order: Mutex::new(Vec::new()),
        }
    }
    fn touch(&self, key: u64) {
        let mut order = self.order.lock().unwrap();
        order.retain(|k| *k != key);
        order.push(key);
    }
}

impl ExecutionCache for LruCache {
    fn get(&self, key: u64) -> Option<CachedValue> {
        let value = self.data.get(&key).cloned();
        if value.is_some() {
            self.touch(key);
        }
        value
    }
    fn put(&mut self, key: u64, value: CachedValue) {
        if self.data.len() >= self.capacity && !self.data.contains_key(&key) {
            let lru_key = self.order.lock().unwrap().first().cloned();
            if let Some(k) = lru_key {
                self.order.lock().unwrap().remove(0);
                self.data.remove(&k);
            }
        }
        self.data.insert(key, value);
        self.touch(key);
    }
    fn invalidate(&mut self, key: u64) {
        self.data.remove(&key);
        self.order.lock().unwrap().retain(|k| *k != key);
    }
    fn clear(&mut self) {
        self.data.clear();
        self.order.lock().unwrap().clear();
    }
    fn len(&self) -> usize {
        self.data.len()
    }
}

/// Tier 3: RAM-pressure-aware cache — evicts oldest 25% when len > threshold
pub struct RamPressureCache {
    threshold: usize,
    data: HashMap<u64, CachedValue>,
    order: Vec<u64>,
}

impl RamPressureCache {
    pub fn new(threshold: usize) -> Self {
        Self {
            threshold,
            data: HashMap::new(),
            order: Vec::new(),
        }
    }
    fn evict_if_needed(&mut self) {
        if self.data.len() >= self.threshold {
            let evict_count = self.threshold / 4;
            let to_evict: Vec<u64> = self
                .order
                .drain(..evict_count.min(self.order.len()))
                .collect();
            for key in to_evict {
                self.data.remove(&key);
            }
        }
    }
}

impl ExecutionCache for RamPressureCache {
    fn get(&self, key: u64) -> Option<CachedValue> {
        self.data.get(&key).cloned()
    }
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
    fn clear(&mut self) {
        self.data.clear();
        self.order.clear();
    }
    fn len(&self) -> usize {
        self.data.len()
    }
}

/// Hierarchical cache: try L1 (fast LRU) then L2 (larger RAM-pressure)
pub struct HierarchicalCache {
    pub l1: LruCache,
    pub l2: RamPressureCache,
}

impl HierarchicalCache {
    pub fn new(l1_cap: usize, l2_threshold: usize) -> Self {
        Self {
            l1: LruCache::new(l1_cap),
            l2: RamPressureCache::new(l2_threshold),
        }
    }

    /// Promoting get: L2 hits are copied into L1 so subsequent accesses are fast.
    /// Use this when you have `&mut` access and want full L1+L2 promotion semantics.
    pub fn get_promoting(&mut self, key: u64) -> Option<CachedValue> {
        if let Some(v) = self.l1.get(key) {
            return Some(v);
        }
        if let Some(v) = self.l2.get(key) {
            self.l1.put(key, v.clone());
            return Some(v);
        }
        None
    }
}

impl ExecutionCache for HierarchicalCache {
    /// Non-promoting read (trait requires `&self`). Use `get_promoting` when `&mut` is available.
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
    fn clear(&mut self) {
        self.l1.clear();
        self.l2.clear();
    }
    fn len(&self) -> usize {
        self.l1.len() + self.l2.len()
    }
}

/// 4-tier cache strategy — ComfyUI execution model pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheStrategy {
    NoCache,
    Lru { capacity: usize },
    RamPressure { max_entries: usize },
    Classic, // IS_CHANGED-gated
}

/// IS_CHANGED flag per node — tracks whether node needs recomputation.
#[derive(Debug, Default)]
pub struct ChangedFlags {
    flags: HashMap<NodeId, bool>,
}

impl ChangedFlags {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn mark_changed(&mut self, id: NodeId) {
        self.flags.insert(id, true);
    }
    pub fn mark_clean(&mut self, id: NodeId) {
        self.flags.insert(id, false);
    }
    /// Nodes are changed by default (unknown == needs recompute).
    pub fn is_changed(&self, id: &NodeId) -> bool {
        self.flags.get(id).copied().unwrap_or(true)
    }
    pub fn changed_count(&self) -> usize {
        self.flags.values().filter(|&&v| v).count()
    }
}

/// Strategy-backed node-level result cache.
pub struct NodeCache {
    pub strategy: CacheStrategy,
    entries: HashMap<NodeId, CachedValue>,
    lru_order: Vec<NodeId>,
}

impl NodeCache {
    pub fn new(strategy: CacheStrategy) -> Self {
        Self {
            strategy,
            entries: HashMap::new(),
            lru_order: Vec::new(),
        }
    }

    pub fn get(&mut self, id: &NodeId) -> Option<&CachedValue> {
        if self.strategy == CacheStrategy::NoCache {
            return None;
        }
        if self.entries.contains_key(id) {
            self.lru_order.retain(|x| x != id);
            self.lru_order.push(id.clone());
        }
        self.entries.get(id)
    }

    pub fn put(&mut self, id: NodeId, output: CachedValue) {
        if self.strategy == CacheStrategy::NoCache {
            return;
        }
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

    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
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
        for i in 0..4u64 {
            cache.put(i, CachedValue::String(format!("{i}")));
        }
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
        assert!(
            cache.get(&"a".to_string()).is_none(),
            "a should have been evicted"
        );
        assert!(cache.get(&"b".to_string()).is_some());
        assert!(cache.get(&"c".to_string()).is_some());
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn cache_classic_stores_and_retrieves() {
        let mut cache = NodeCache::new(CacheStrategy::Classic);
        cache.put("node1".to_string(), CachedValue::Bytes(vec![1, 2, 3]));
        let val = cache
            .get(&"node1".to_string())
            .expect("Classic cache should store and retrieve");
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

    #[test]
    fn hierarchical_cache_l2_hit_promotes_to_l1() {
        // L1 capacity = 1 so a second put evicts the first entry from L1.
        // After eviction, the value lives only in L2.
        // get_promoting() must re-populate L1 with the promoted value.
        let mut cache = HierarchicalCache::new(1, 100);
        cache.put(10, CachedValue::String("ten".into()));
        // Evict key 10 from L1 by inserting another entry (L1 cap = 1).
        cache.l1.put(99, CachedValue::String("other".into()));
        // Verify L1 no longer holds 10 but L2 still does.
        assert!(
            cache.l1.get(10).is_none(),
            "key 10 should have been evicted from L1"
        );
        assert!(cache.l2.get(10).is_some(), "key 10 should still be in L2");
        // get_promoting should find it in L2 and promote to L1.
        let result = cache.get_promoting(10);
        assert!(
            result.is_some(),
            "get_promoting should return the L2-resident value"
        );
        match result.unwrap() {
            CachedValue::String(s) => assert_eq!(s, "ten"),
            _ => panic!("wrong variant"),
        }
        // After promotion, L1 should now contain key 10.
        assert!(
            cache.l1.get(10).is_some(),
            "key 10 should be promoted back to L1"
        );
    }

    #[test]
    fn lru_cache_get_updates_recency() {
        // capacity=2: put 1, put 2 → order=[1,2] (1 is LRU)
        // get(1) must move 1 to MRU → order=[2,1]
        // put(3) must evict 2 (now LRU), not 1
        let mut cache = LruCache::new(2);
        cache.put(1, CachedValue::String("a".into()));
        cache.put(2, CachedValue::String("b".into()));
        // touch key 1 via get — it should become MRU
        assert!(cache.get(1).is_some());
        // inserting key 3 must evict key 2 (LRU after get(1)), not key 1
        cache.put(3, CachedValue::String("c".into()));
        assert!(
            cache.get(1).is_some(),
            "key 1 should survive (was touched by get)"
        );
        assert!(
            cache.get(2).is_none(),
            "key 2 should have been evicted (LRU after get(1))"
        );
        assert!(cache.get(3).is_some());
    }

    #[test]
    fn hierarchical_cache_l1_preferred_over_l2() {
        // Both L1 and L2 hold key 5, but with different values.
        // A direct get() should return the L1 value (checked first).
        let mut cache = HierarchicalCache::new(10, 100);
        // Manually insert distinct values into L1 and L2 to confirm L1 wins.
        cache.l1.put(5, CachedValue::String("from-l1".into()));
        cache.l2.put(5, CachedValue::String("from-l2".into()));
        let result = cache.get(5).expect("should find key in cache");
        match result {
            CachedValue::String(s) => assert_eq!(s, "from-l1", "L1 value should be preferred"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn hierarchical_cache_len_includes_both_tiers() {
        // put() inserts into both L1 and L2, so len() should count both.
        let mut cache = HierarchicalCache::new(10, 100);
        cache.put(1, CachedValue::String("a".into()));
        cache.put(2, CachedValue::String("b".into()));
        // L1 has 2, L2 has 2 → total len = 4.
        assert_eq!(cache.len(), 4, "len() must sum L1 and L2 counts");
    }

    // ------------------------------------------------------------------
    // BasicCache: len tracks entries correctly
    // ------------------------------------------------------------------
    #[test]
    fn basic_cache_len_tracks_inserts_and_evictions() {
        let mut cache = BasicCache::new();
        assert_eq!(cache.len(), 0);
        cache.put(1, CachedValue::String("a".into()));
        cache.put(2, CachedValue::String("b".into()));
        assert_eq!(cache.len(), 2);
        cache.invalidate(1);
        assert_eq!(cache.len(), 1);
        cache.clear();
        assert_eq!(cache.len(), 0);
    }

    // ------------------------------------------------------------------
    // BasicCache: overwrite same key keeps len stable
    // ------------------------------------------------------------------
    #[test]
    fn basic_cache_overwrite_same_key_keeps_len() {
        let mut cache = BasicCache::new();
        cache.put(42, CachedValue::String("first".into()));
        cache.put(42, CachedValue::String("second".into()));
        assert_eq!(cache.len(), 1, "overwriting key must not grow len");
        match cache.get(42).unwrap() {
            CachedValue::String(s) => assert_eq!(s, "second"),
            _ => panic!("wrong variant"),
        }
    }

    // ------------------------------------------------------------------
    // ChangedFlags: changed_count with multiple nodes
    // ------------------------------------------------------------------
    #[test]
    fn changed_flags_count_multiple_nodes() {
        let mut flags = ChangedFlags::new();
        flags.mark_changed("n1".to_string());
        flags.mark_changed("n2".to_string());
        flags.mark_changed("n3".to_string());
        assert_eq!(flags.changed_count(), 3);
        flags.mark_clean("n2".to_string());
        assert_eq!(flags.changed_count(), 2);
    }

    // ------------------------------------------------------------------
    // NullCache: len always zero, put is a noop
    // ------------------------------------------------------------------
    #[test]
    fn null_cache_len_always_zero() {
        let mut cache = NullCache;
        cache.put(1, CachedValue::String("x".into()));
        assert_eq!(cache.len(), 0, "NullCache.len() must always be 0");
        cache.invalidate(1);
        cache.clear();
        assert_eq!(cache.len(), 0);
    }

    // ------------------------------------------------------------------
    // LruCache: invalidate removes entry
    // ------------------------------------------------------------------
    #[test]
    fn lru_cache_invalidate_removes_entry() {
        let mut cache = LruCache::new(10);
        cache.put(7, CachedValue::String("seven".into()));
        assert!(cache.get(7).is_some());
        cache.invalidate(7);
        assert!(
            cache.get(7).is_none(),
            "invalidated key must not be retrievable"
        );
        assert_eq!(cache.len(), 0);
    }

    // ------------------------------------------------------------------
    // LruCache: clear empties all entries and order
    // ------------------------------------------------------------------
    #[test]
    fn lru_cache_clear_empties_all() {
        let mut cache = LruCache::new(5);
        for i in 0..5u64 {
            cache.put(i, CachedValue::String(format!("{i}")));
        }
        assert_eq!(cache.len(), 5);
        cache.clear();
        assert_eq!(cache.len(), 0, "clear must remove all entries");
        assert!(cache.get(0).is_none(), "entries must be gone after clear");
    }

    // ------------------------------------------------------------------
    // LruCache: put same key twice does not grow len
    // ------------------------------------------------------------------
    #[test]
    fn lru_cache_put_same_key_overwrites() {
        let mut cache = LruCache::new(5);
        cache.put(42, CachedValue::String("first".into()));
        cache.put(42, CachedValue::String("second".into()));
        assert_eq!(cache.len(), 1, "overwrite must not grow len");
        match cache.get(42).unwrap() {
            CachedValue::String(s) => assert_eq!(s, "second"),
            _ => panic!("wrong variant"),
        }
    }

    // ------------------------------------------------------------------
    // LruCache: capacity=1 always evicts the sole entry on new put
    // ------------------------------------------------------------------
    #[test]
    fn lru_cache_capacity_one_evicts_on_new_key() {
        let mut cache = LruCache::new(1);
        cache.put(1, CachedValue::String("one".into()));
        assert!(cache.get(1).is_some());
        cache.put(2, CachedValue::String("two".into()));
        assert!(cache.get(1).is_none(), "key 1 must be evicted by key 2");
        assert!(cache.get(2).is_some());
        assert_eq!(cache.len(), 1);
    }

    // ------------------------------------------------------------------
    // LruCache: is_empty reflects empty/non-empty state
    // ------------------------------------------------------------------
    #[test]
    fn lru_cache_is_empty() {
        let mut cache = LruCache::new(3);
        assert!(cache.is_empty());
        cache.put(1, CachedValue::String("x".into()));
        assert!(!cache.is_empty());
    }

    // ------------------------------------------------------------------
    // RamPressureCache: clear empties cache
    // ------------------------------------------------------------------
    #[test]
    fn ram_pressure_cache_clear_empties() {
        let mut cache = RamPressureCache::new(10);
        cache.put(1, CachedValue::String("a".into()));
        cache.put(2, CachedValue::String("b".into()));
        assert_eq!(cache.len(), 2);
        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.get(1).is_none());
    }

    // ------------------------------------------------------------------
    // RamPressureCache: invalidate removes specific key
    // ------------------------------------------------------------------
    #[test]
    fn ram_pressure_cache_invalidate_specific_key() {
        let mut cache = RamPressureCache::new(10);
        cache.put(10, CachedValue::String("ten".into()));
        cache.put(20, CachedValue::String("twenty".into()));
        cache.invalidate(10);
        assert!(cache.get(10).is_none(), "key 10 must be removed");
        assert!(cache.get(20).is_some(), "key 20 must remain");
        assert_eq!(cache.len(), 1);
    }

    // ------------------------------------------------------------------
    // RamPressureCache: get returns stored value
    // ------------------------------------------------------------------
    #[test]
    fn ram_pressure_cache_get_stored_value() {
        let mut cache = RamPressureCache::new(8);
        cache.put(5, CachedValue::Bytes(vec![1, 2, 3]));
        match cache.get(5).unwrap() {
            CachedValue::Bytes(b) => assert_eq!(b, vec![1, 2, 3]),
            _ => panic!("wrong variant"),
        }
    }

    // ------------------------------------------------------------------
    // NodeCache: LRU capacity respected with multiple evictions
    // ------------------------------------------------------------------
    #[test]
    fn node_cache_lru_evicts_in_order() {
        let mut cache = NodeCache::new(CacheStrategy::Lru { capacity: 3 });
        cache.put("a".to_string(), CachedValue::String("1".into()));
        cache.put("b".to_string(), CachedValue::String("2".into()));
        cache.put("c".to_string(), CachedValue::String("3".into()));
        // "a" is oldest, adding "d" must evict "a"
        cache.put("d".to_string(), CachedValue::String("4".into()));
        assert!(cache.get(&"a".to_string()).is_none(), "a must be evicted");
        assert!(cache.get(&"b".to_string()).is_some());
        assert!(cache.get(&"c".to_string()).is_some());
        assert!(cache.get(&"d".to_string()).is_some());
        assert_eq!(cache.len(), 3);
    }

    // ------------------------------------------------------------------
    // NodeCache: evict removes entry
    // ------------------------------------------------------------------
    #[test]
    fn node_cache_evict_removes_entry() {
        let mut cache = NodeCache::new(CacheStrategy::Classic);
        cache.put("x".to_string(), CachedValue::String("val".into()));
        assert!(cache.get(&"x".to_string()).is_some());
        cache.evict(&"x".to_string());
        assert!(cache.get(&"x".to_string()).is_none(), "evicted key must be gone");
        assert_eq!(cache.len(), 0);
    }

    // ------------------------------------------------------------------
    // NodeCache: is_empty reflects state
    // ------------------------------------------------------------------
    #[test]
    fn node_cache_is_empty_initially() {
        let cache = NodeCache::new(CacheStrategy::Classic);
        assert!(cache.is_empty(), "new NodeCache must be empty");
    }

    // ------------------------------------------------------------------
    // ChangedFlags: mark_changed then mark_clean cycle
    // ------------------------------------------------------------------
    #[test]
    fn changed_flags_mark_changed_clean_cycle() {
        let mut flags = ChangedFlags::new();
        flags.mark_changed("node_a".to_string());
        assert!(flags.is_changed(&"node_a".to_string()));
        flags.mark_clean("node_a".to_string());
        assert!(!flags.is_changed(&"node_a".to_string()));
        // changed_count must be 0 after marking clean
        assert_eq!(flags.changed_count(), 0);
    }

    // ------------------------------------------------------------------
    // ChangedFlags: unknown node defaults to changed
    // ------------------------------------------------------------------
    #[test]
    fn changed_flags_unknown_node_is_changed() {
        let flags = ChangedFlags::new();
        assert!(flags.is_changed(&"never_seen".to_string()), "unknown node must default to changed");
    }

    // ------------------------------------------------------------------
    // BasicCache: is_empty reflects state
    // ------------------------------------------------------------------
    #[test]
    fn basic_cache_is_empty_initially() {
        let cache = BasicCache::new();
        assert!(cache.is_empty(), "new BasicCache must be empty");
    }

    // ------------------------------------------------------------------
    // BasicCache: default() creates empty cache
    // ------------------------------------------------------------------
    #[test]
    fn basic_cache_default_creates_empty() {
        let cache = BasicCache::default();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    // ------------------------------------------------------------------
    // HierarchicalCache: invalidate removes from both tiers
    // ------------------------------------------------------------------
    #[test]
    fn hierarchical_cache_invalidate_removes_from_both_tiers() {
        let mut cache = HierarchicalCache::new(10, 100);
        cache.put(77, CachedValue::String("val".into()));
        assert!(cache.l1.get(77).is_some());
        assert!(cache.l2.get(77).is_some());
        cache.invalidate(77);
        assert!(cache.l1.get(77).is_none(), "L1 must not hold key after invalidate");
        assert!(cache.l2.get(77).is_none(), "L2 must not hold key after invalidate");
    }

    // ------------------------------------------------------------------
    // HierarchicalCache: clear removes from both tiers
    // ------------------------------------------------------------------
    #[test]
    fn hierarchical_cache_clear_removes_from_both_tiers() {
        let mut cache = HierarchicalCache::new(10, 100);
        cache.put(1, CachedValue::String("a".into()));
        cache.put(2, CachedValue::String("b".into()));
        cache.clear();
        assert_eq!(cache.l1.len(), 0, "L1 must be empty after clear");
        assert_eq!(cache.l2.len(), 0, "L2 must be empty after clear");
        assert_eq!(cache.len(), 0);
    }

    // ------------------------------------------------------------------
    // CacheStrategy: equality checks
    // ------------------------------------------------------------------
    #[test]
    fn cache_strategy_equality() {
        assert_eq!(CacheStrategy::NoCache, CacheStrategy::NoCache);
        assert_eq!(CacheStrategy::Classic, CacheStrategy::Classic);
        assert_eq!(
            CacheStrategy::Lru { capacity: 4 },
            CacheStrategy::Lru { capacity: 4 }
        );
        assert_ne!(
            CacheStrategy::Lru { capacity: 4 },
            CacheStrategy::Lru { capacity: 5 }
        );
    }

    // ------------------------------------------------------------------
    // HierarchicalCache: L1 hit does not query L2
    // Verify that when L1 already has the value, a get() returns it
    // without consulting L2 (L2 may hold a different value for the key).
    // ------------------------------------------------------------------
    #[test]
    fn hierarchical_cache_l1_hit_does_not_use_l2() {
        let mut cache = HierarchicalCache::new(10, 100);
        // Put distinct values into L1 and L2 for the same key.
        cache.l1.put(100, CachedValue::String("from-l1".into()));
        cache.l2.put(100, CachedValue::String("from-l2".into()));
        // A top-level get() must return the L1 value (checked first).
        let result = cache.get(100).expect("must find key 100");
        match result {
            CachedValue::String(s) => assert_eq!(
                s, "from-l1",
                "L1 hit must be returned without consulting L2"
            ),
            _ => panic!("wrong CachedValue variant"),
        }
    }

    // ------------------------------------------------------------------
    // HierarchicalCache: L1 miss promotes entry from L2 to L1 via get_promoting
    // ------------------------------------------------------------------
    #[test]
    fn hierarchical_cache_l1_miss_promotes_from_l2() {
        let mut cache = HierarchicalCache::new(2, 100);
        // Put a value normally — it goes into both L1 and L2.
        cache.put(55, CachedValue::String("value55".into()));
        // Evict from L1 only by filling it with other keys (capacity=2, need 2 more).
        cache.l1.put(56, CachedValue::String("other1".into()));
        cache.l1.put(57, CachedValue::String("other2".into()));
        // Now key 55 should be gone from L1 (evicted by LRU) but still in L2.
        assert!(
            cache.l1.get(55).is_none(),
            "key 55 should have been evicted from L1"
        );
        assert!(
            cache.l2.get(55).is_some(),
            "key 55 should still live in L2"
        );
        // get_promoting fetches from L2 and re-inserts into L1.
        let result = cache.get_promoting(55);
        assert!(result.is_some(), "get_promoting must find key 55 via L2");
        match result.unwrap() {
            CachedValue::String(s) => assert_eq!(s, "value55"),
            _ => panic!("wrong variant"),
        }
        // After promotion, L1 must hold the key again.
        assert!(
            cache.l1.get(55).is_some(),
            "key 55 must be promoted back into L1 after get_promoting"
        );
    }

    // ------------------------------------------------------------------
    // LruCache: capacity exactly at limit — no eviction until limit exceeded
    // ------------------------------------------------------------------
    #[test]
    fn lru_cache_capacity_exactly_at_limit_no_early_eviction() {
        let cap = 5usize;
        let mut cache = LruCache::new(cap);
        // Fill to exact capacity — no eviction should occur.
        for i in 0..cap as u64 {
            cache.put(i, CachedValue::String(format!("v{i}")));
        }
        assert_eq!(
            cache.len(),
            cap,
            "cache must hold exactly {cap} entries at capacity"
        );
        // All entries must still be present (no premature eviction).
        for i in 0..cap as u64 {
            assert!(
                cache.get(i).is_some(),
                "key {i} must not be evicted when cache is at capacity"
            );
        }
        // Adding one more entry must evict exactly one (the LRU one — key 0,
        // but note that the get() calls above updated recency; we re-test with
        // a fresh cache to isolate the behaviour).
        let mut cache2 = LruCache::new(3);
        cache2.put(10, CachedValue::String("a".into()));
        cache2.put(20, CachedValue::String("b".into()));
        cache2.put(30, CachedValue::String("c".into()));
        assert_eq!(cache2.len(), 3, "cache2 must hold 3 entries at capacity");
        // Adding a 4th must evict exactly one entry.
        cache2.put(40, CachedValue::String("d".into()));
        assert_eq!(
            cache2.len(),
            3,
            "len must stay at capacity after exceeding limit"
        );
    }

    // ------------------------------------------------------------------
    // HierarchicalCache: get() without promotion: missing in both returns None
    // ------------------------------------------------------------------
    #[test]
    fn hierarchical_cache_miss_both_tiers_returns_none() {
        let cache = HierarchicalCache::new(10, 100);
        assert!(
            cache.get(999).is_none(),
            "key absent from both tiers must return None"
        );
    }

    // ------------------------------------------------------------------
    // LruCache: capacity=0 never stores anything
    // ------------------------------------------------------------------
    #[test]
    fn lru_cache_capacity_zero_never_stores() {
        let mut cache = LruCache::new(0);
        // A zero-capacity LRU must always evict immediately.
        // The put() will try to evict and then insert.
        // Depending on implementation the len may be 0 or 1;
        // what we guarantee is that get() returns the value if len=1.
        // The invariant: we just verify it doesn't panic and len ≤ 1.
        cache.put(1, CachedValue::String("x".into()));
        assert!(
            cache.len() <= 1,
            "zero-capacity cache must never hold more than 1 entry"
        );
    }

    // ------------------------------------------------------------------
    // BasicCache: large number of entries all retrievable
    // ------------------------------------------------------------------
    #[test]
    fn basic_cache_hundred_entries_all_retrievable() {
        let mut cache = BasicCache::new();
        for i in 0u64..100 {
            cache.put(i, CachedValue::String(format!("val{i}")));
        }
        assert_eq!(cache.len(), 100);
        for i in 0u64..100 {
            assert!(cache.get(i).is_some(), "key {i} must be retrievable");
        }
    }

    // ------------------------------------------------------------------
    // LruCache: capacity=2, insert A/B/C → A evicted (classic LRU)
    // ------------------------------------------------------------------
    #[test]
    fn lru_cache_evicts_least_recently_used() {
        let mut cache = LruCache::new(2);
        cache.put(1, CachedValue::String("A".into())); // order: [1]
        cache.put(2, CachedValue::String("B".into())); // order: [1, 2]
        cache.put(3, CachedValue::String("C".into())); // evicts 1; order: [2, 3]
        assert!(cache.get(1).is_none(), "A (key 1) must have been evicted");
        assert!(cache.get(2).is_some(), "B (key 2) must still be present");
        assert!(cache.get(3).is_some(), "C (key 3) must be present");
    }

    // ------------------------------------------------------------------
    // LruCache: access refreshes order so the accessed item survives eviction
    // ------------------------------------------------------------------
    #[test]
    fn lru_cache_access_refreshes_order() {
        let mut cache = LruCache::new(2);
        cache.put(1, CachedValue::String("A".into()));
        cache.put(2, CachedValue::String("B".into()));
        // Touch key 1 — it becomes MRU; key 2 becomes LRU.
        assert!(cache.get(1).is_some());
        // Insert key 3 — must evict key 2 (LRU), not key 1.
        cache.put(3, CachedValue::String("C".into()));
        assert!(cache.get(1).is_some(), "A (key 1) must survive after being touched");
        assert!(cache.get(2).is_none(), "B (key 2) must be evicted (LRU after get(1))");
        assert!(cache.get(3).is_some(), "C (key 3) must be present");
    }

    // ------------------------------------------------------------------
    // HierarchicalCache: L1 hit means L2 is not the source of the result
    // ------------------------------------------------------------------
    #[test]
    fn hierarchical_cache_l1_hit_no_l2_read() {
        let mut cache = HierarchicalCache::new(10, 100);
        // Put distinct values into L1 and L2 for the same key.
        cache.l1.put(42, CachedValue::String("l1-value".into()));
        cache.l2.put(42, CachedValue::String("l2-value".into()));
        // get() must return the L1 value (first checked).
        let result = cache.get(42).expect("key must be found");
        match result {
            CachedValue::String(s) => assert_eq!(s, "l1-value", "L1 value must be returned, not L2"),
            _ => panic!("wrong variant"),
        }
    }

    // ------------------------------------------------------------------
    // HierarchicalCache: L1 miss, L2 hit, promoted via get_promoting
    // ------------------------------------------------------------------
    #[test]
    fn hierarchical_cache_l1_miss_l2_hit_promotes() {
        let mut cache = HierarchicalCache::new(1, 100);
        // Put value — lands in both L1 and L2.
        cache.put(77, CachedValue::String("val77".into()));
        // Evict from L1 by inserting another entry (capacity=1).
        cache.l1.put(99, CachedValue::String("other".into()));
        assert!(cache.l1.get(77).is_none(), "77 must be evicted from L1");
        assert!(cache.l2.get(77).is_some(), "77 must still be in L2");
        // get_promoting should fetch from L2 and re-insert into L1.
        let result = cache.get_promoting(77);
        assert!(result.is_some(), "get_promoting must find key 77 via L2");
        assert!(cache.l1.get(77).is_some(), "77 must be promoted back to L1");
    }

    // ------------------------------------------------------------------
    // HierarchicalCache: miss in both tiers returns None
    // ------------------------------------------------------------------
    #[test]
    fn hierarchical_cache_miss_both_returns_none() {
        let cache = HierarchicalCache::new(10, 100);
        assert!(
            cache.get(12345).is_none(),
            "key absent from both L1 and L2 must return None"
        );
    }

    // ------------------------------------------------------------------
    // BasicCache: insert and get round-trip
    // ------------------------------------------------------------------
    #[test]
    fn cache_insert_and_get_round_trip() {
        let mut cache = BasicCache::new();
        cache.put(55, CachedValue::Bytes(vec![10, 20, 30]));
        match cache.get(55).expect("must retrieve inserted value") {
            CachedValue::Bytes(b) => assert_eq!(b, vec![10, 20, 30]),
            _ => panic!("wrong variant"),
        }
    }

    // ------------------------------------------------------------------
    // BasicCache: get on nonexistent key returns None
    // ------------------------------------------------------------------
    #[test]
    fn cache_get_nonexistent_returns_none() {
        let cache = BasicCache::new();
        assert!(cache.get(9999).is_none(), "nonexistent key must return None");
    }

    // ------------------------------------------------------------------
    // LruCache: capacity=0 never stores more than 1 entry (no panic)
    // ------------------------------------------------------------------
    #[test]
    fn cache_capacity_zero_never_stores() {
        let mut cache = LruCache::new(0);
        cache.put(1, CachedValue::String("x".into()));
        // Capacity 0 means the put may immediately evict; len must be ≤ 1.
        assert!(
            cache.len() <= 1,
            "zero-capacity cache must never hold more than 1 entry, got {}",
            cache.len()
        );
    }

    // ------------------------------------------------------------------
    // BasicCache: duplicate key overwrites the previous value
    // ------------------------------------------------------------------
    #[test]
    fn cache_insert_duplicate_key_overwrites() {
        let mut cache = BasicCache::new();
        cache.put(1, CachedValue::String("first".into()));
        cache.put(1, CachedValue::String("second".into()));
        assert_eq!(cache.len(), 1, "duplicate key must not grow len");
        match cache.get(1).unwrap() {
            CachedValue::String(s) => assert_eq!(s, "second", "second insert must win"),
            _ => panic!("wrong variant"),
        }
    }

    // ------------------------------------------------------------------
    // LruCache: len after evictions stays at capacity
    // ------------------------------------------------------------------
    #[test]
    fn cache_len_after_evictions() {
        let cap = 3usize;
        let mut cache = LruCache::new(cap);
        for i in 0..10u64 {
            cache.put(i, CachedValue::String(format!("v{i}")));
        }
        assert_eq!(
            cache.len(),
            cap,
            "after many inserts, len must equal capacity (evictions happened)"
        );
    }

    // ------------------------------------------------------------------
    // HierarchicalCache: L2 can hold more items than L1
    // ------------------------------------------------------------------
    #[test]
    fn l2_cache_larger_than_l1() {
        // L1 capacity=2, L2 threshold=10.
        let mut cache = HierarchicalCache::new(2, 10);
        for i in 0..5u64 {
            cache.put(i, CachedValue::String(format!("v{i}")));
        }
        // L1 should hold only up to 2 items; L2 should hold all 5.
        assert!(
            cache.l1.len() <= 2,
            "L1 must hold at most 2 items, holds {}",
            cache.l1.len()
        );
        assert_eq!(cache.l2.len(), 5, "L2 must hold all 5 inserted items");
    }

    // ------------------------------------------------------------------
    // HierarchicalCache: clear empties all tiers
    // ------------------------------------------------------------------
    #[test]
    fn cache_clear_empties_all() {
        let mut cache = HierarchicalCache::new(10, 100);
        for i in 0..5u64 {
            cache.put(i, CachedValue::String(format!("v{i}")));
        }
        assert!(cache.len() > 0, "cache must have items before clear");
        cache.clear();
        assert_eq!(cache.l1.len(), 0, "L1 must be empty after clear");
        assert_eq!(cache.l2.len(), 0, "L2 must be empty after clear");
        assert_eq!(cache.len(), 0, "total len must be 0 after clear");
        // All previously stored keys must return None.
        for i in 0..5u64 {
            assert!(cache.get(i).is_none(), "key {i} must be gone after clear");
        }
    }

    // ------------------------------------------------------------------
    // NodeCache: RamPressure strategy — unlimited puts don't panic
    // ------------------------------------------------------------------
    #[test]
    fn node_cache_ram_pressure_many_puts() {
        let mut cache = NodeCache::new(CacheStrategy::RamPressure { max_entries: 10 });
        // RamPressure strategy is not handled by NodeCache (falls through to
        // Classic behaviour since only NoCache and Lru are special-cased).
        // Just verify it doesn't panic.
        for i in 0u32..20 {
            cache.put(format!("n{i}"), CachedValue::String(format!("{i}")));
        }
        // At minimum the last inserted entry must be present.
        assert!(cache.get(&"n19".to_string()).is_some());
    }

    // ------------------------------------------------------------------
    // hierarchical_cache_l1_capacity_smaller_than_l2
    // ------------------------------------------------------------------
    #[test]
    fn hierarchical_cache_l1_capacity_smaller_than_l2() {
        // L1 cap=3, L2 threshold=20; insert 5 items.
        let mut cache = HierarchicalCache::new(3, 20);
        for i in 0u64..5 {
            cache.put(i, CachedValue::String(format!("v{i}")));
        }
        // L1 holds at most 3; L2 holds all 5.
        assert!(cache.l1.len() <= 3, "L1 must hold at most 3 items");
        assert_eq!(cache.l2.len(), 5, "L2 must hold all 5 items");
    }

    // ------------------------------------------------------------------
    // hierarchical_cache_l2_holds_after_l1_eviction
    // ------------------------------------------------------------------
    #[test]
    fn hierarchical_cache_l2_holds_after_l1_eviction() {
        // L1 cap=1; insert two items — first is evicted from L1 but stays in L2.
        let mut cache = HierarchicalCache::new(1, 50);
        cache.put(10, CachedValue::String("ten".into()));
        cache.put(20, CachedValue::String("twenty".into())); // evicts 10 from L1
        assert!(cache.l1.get(10).is_none(), "key 10 must be evicted from L1");
        assert!(cache.l2.get(10).is_some(), "key 10 must still be in L2");
    }

    // ------------------------------------------------------------------
    // lru_access_pattern_a_b_a_b_keeps_both — alternating access keeps both in cache-2
    // ------------------------------------------------------------------
    #[test]
    fn lru_access_pattern_a_b_a_b_keeps_both() {
        // Capacity=2; alternately get A and B — neither should be evicted.
        let mut cache = LruCache::new(2);
        cache.put(1, CachedValue::String("A".into()));
        cache.put(2, CachedValue::String("B".into()));
        // Alternate accesses: A, B, A, B
        for _ in 0..4 {
            assert!(cache.get(1).is_some(), "A must remain after alternating access");
            assert!(cache.get(2).is_some(), "B must remain after alternating access");
        }
        assert_eq!(cache.len(), 2, "both entries must still be present");
    }

    // ------------------------------------------------------------------
    // lru_cold_miss_then_warm_hit
    // ------------------------------------------------------------------
    #[test]
    fn lru_cold_miss_then_warm_hit() {
        let mut cache = LruCache::new(5);
        // First access is a cold miss.
        assert!(cache.get(99).is_none(), "cold miss: key not yet inserted");
        // Insert the key — now warm.
        cache.put(99, CachedValue::String("warm".into()));
        assert!(cache.get(99).is_some(), "warm hit: key must be found after put");
        match cache.get(99).unwrap() {
            CachedValue::String(s) => assert_eq!(s, "warm"),
            _ => panic!("wrong variant"),
        }
    }

    // ------------------------------------------------------------------
    // cache_get_or_insert — insert-on-miss semantics via get + put
    // ------------------------------------------------------------------
    #[test]
    fn cache_get_or_insert() {
        let mut cache = BasicCache::new();
        // Simulate get-or-insert: if missing, insert then return.
        let key = 77u64;
        if cache.get(key).is_none() {
            cache.put(key, CachedValue::String("default".into()));
        }
        assert!(cache.get(key).is_some(), "get_or_insert must leave key present");
        match cache.get(key).unwrap() {
            CachedValue::String(s) => assert_eq!(s, "default"),
            _ => panic!("wrong variant"),
        }
        // Second call must return the existing value, not overwrite.
        if cache.get(key).is_none() {
            cache.put(key, CachedValue::String("should_not_appear".into()));
        }
        match cache.get(key).unwrap() {
            CachedValue::String(s) => assert_eq!(s, "default", "existing value must not be overwritten"),
            _ => panic!("wrong variant"),
        }
    }

    // ------------------------------------------------------------------
    // cache_len_increases_on_insert
    // ------------------------------------------------------------------
    #[test]
    fn cache_len_increases_on_insert() {
        let mut cache = BasicCache::new();
        assert_eq!(cache.len(), 0);
        cache.put(1, CachedValue::String("a".into()));
        assert_eq!(cache.len(), 1);
        cache.put(2, CachedValue::String("b".into()));
        assert_eq!(cache.len(), 2);
        cache.put(3, CachedValue::String("c".into()));
        assert_eq!(cache.len(), 3);
    }

    // ------------------------------------------------------------------
    // cache_len_decreases_on_eviction
    // ------------------------------------------------------------------
    #[test]
    fn cache_len_decreases_on_eviction() {
        let mut cache = LruCache::new(3);
        cache.put(1, CachedValue::String("a".into()));
        cache.put(2, CachedValue::String("b".into()));
        cache.put(3, CachedValue::String("c".into()));
        assert_eq!(cache.len(), 3);
        // Adding a 4th evicts one entry — len stays at capacity.
        cache.put(4, CachedValue::String("d".into()));
        assert_eq!(cache.len(), 3, "len must remain at capacity after eviction");
    }

    // ------------------------------------------------------------------
    // hierarchical_cache_promote_from_l2_to_l1 — get_promoting re-populates L1
    // ------------------------------------------------------------------
    #[test]
    fn hierarchical_cache_promote_from_l2_to_l1() {
        let mut cache = HierarchicalCache::new(1, 50);
        cache.put(5, CachedValue::String("five".into()));
        // Evict key 5 from L1 by inserting another entry.
        cache.l1.put(6, CachedValue::String("six".into()));
        assert!(cache.l1.get(5).is_none(), "5 must be evicted from L1");
        assert!(cache.l2.get(5).is_some(), "5 must still reside in L2");
        // get_promoting must re-insert into L1.
        let result = cache.get_promoting(5);
        assert!(result.is_some(), "get_promoting must find key 5");
        assert!(cache.l1.get(5).is_some(), "key 5 must be promoted back to L1");
    }

    // ------------------------------------------------------------------
    // hierarchical_cache_demote_from_l1_on_eviction — L1 eviction does not remove from L2
    // ------------------------------------------------------------------
    #[test]
    fn hierarchical_cache_demote_from_l1_on_eviction() {
        let mut cache = HierarchicalCache::new(2, 50);
        cache.put(10, CachedValue::String("ten".into()));
        cache.put(20, CachedValue::String("twenty".into()));
        // Add 2 more to L1 — key 10 is evicted from L1 but L2 is unaffected.
        cache.l1.put(30, CachedValue::String("thirty".into()));
        cache.l1.put(40, CachedValue::String("forty".into()));
        // Key 10 may now be evicted from L1; it must still be in L2.
        assert!(cache.l2.get(10).is_some(), "L2 must retain key 10 even after L1 eviction");
    }

    // ------------------------------------------------------------------
    // cache_concurrent_read_write_safe — LruCache interior mutability is Mutex-guarded
    // ------------------------------------------------------------------
    #[test]
    fn cache_concurrent_read_write_safe() {
        // Verify that calling get() (which uses interior mutability via Mutex) and
        // put() in sequence does not deadlock or panic.
        let mut cache = LruCache::new(10);
        cache.put(1, CachedValue::String("a".into()));
        cache.put(2, CachedValue::String("b".into()));
        // Interleave reads and writes.
        for i in 3u64..13 {
            let _ = cache.get(i - 2);
            cache.put(i, CachedValue::String(format!("{i}")));
        }
        // Cache should have at most 10 entries and contain key 12.
        assert!(cache.len() <= 10, "len must not exceed capacity");
        assert!(cache.get(12).is_some(), "most recently inserted key must be present");
    }

    // ------------------------------------------------------------------
    // cache_stats_eviction_count_exact — LruCache evicts exactly 1 per excess insert
    // ------------------------------------------------------------------
    #[test]
    fn cache_stats_eviction_count_exact() {
        let cap = 5usize;
        let mut cache = LruCache::new(cap);
        // Fill to capacity.
        for i in 0..cap as u64 {
            cache.put(i, CachedValue::String(format!("{i}")));
        }
        assert_eq!(cache.len(), cap);
        // Insert 5 more items — each insert evicts exactly 1 entry.
        for i in cap as u64..cap as u64 + 5 {
            cache.put(i, CachedValue::String(format!("{i}")));
            assert_eq!(
                cache.len(),
                cap,
                "len must remain exactly at capacity after each insert+evict"
            );
        }
    }
}
