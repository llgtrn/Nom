#![deny(unsafe_code)]
use std::collections::HashMap;
use crate::constraint::Constraint;
use crate::hash::Hash128;
use crate::tracked::TrackedSnapshot;

/// A single cached computation result
pub struct CachedResult<T: Clone> {
    pub value: T,
    pub constraint: Constraint,
    pub hash: Hash128,
}

/// Memoization cache: key (Hash128) → (T, Constraint)
/// Validates constraint before returning cached value
pub struct MemoCache<T: Clone> {
    entries: HashMap<u64, CachedResult<T>>,
    hit_count: u64,
    miss_count: u64,
}

impl<T: Clone> MemoCache<T> {
    pub fn new() -> Self {
        Self { entries: HashMap::new(), hit_count: 0, miss_count: 0 }
    }

    /// Try to retrieve a cached result. Validates constraint before returning.
    /// `current_snapshots` holds fresh (method_id, return_hash) snapshots for validation.
    pub fn get(&mut self, key: &Hash128, current_input_hash: u64, current_snapshots: &[TrackedSnapshot]) -> Option<T> {
        let entry = self.entries.get(&key.as_u64())?;
        if entry.constraint.validate(current_input_hash, current_snapshots) {
            self.hit_count += 1;
            Some(entry.value.clone())
        } else {
            self.miss_count += 1;
            None
        }
    }

    pub fn put(&mut self, key: Hash128, value: T, constraint: Constraint) {
        self.entries.insert(key.as_u64(), CachedResult { value, constraint, hash: key });
    }

    pub fn invalidate(&mut self, key: &Hash128) {
        self.entries.remove(&key.as_u64());
    }

    pub fn clear(&mut self) { self.entries.clear(); }
    pub fn len(&self) -> usize { self.entries.len() }
    pub fn hit_count(&self) -> u64 { self.hit_count }
    pub fn miss_count(&self) -> u64 { self.miss_count }
    pub fn hit_rate(&self) -> f64 {
        let total = self.hit_count + self.miss_count;
        if total == 0 { 0.0 } else { self.hit_count as f64 / total as f64 }
    }
}

impl<T: Clone> Default for MemoCache<T> { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memo_cache_hit() {
        let mut cache: MemoCache<String> = MemoCache::new();
        let key = Hash128::of_str("f(x)");
        let constraint = Constraint::new(42);
        cache.put(key, "result".into(), constraint);
        let result = cache.get(&key, 42, &[]);
        assert_eq!(result, Some("result".into()));
        assert_eq!(cache.hit_count(), 1);
    }

    #[test]
    fn memo_cache_miss_on_stale_input() {
        let mut cache: MemoCache<u64> = MemoCache::new();
        let key = Hash128::of_str("g(x)");
        let constraint = Constraint::new(100);
        cache.put(key, 999, constraint);
        // input hash changed: 100 → 200
        let result = cache.get(&key, 200, &[]);
        assert_eq!(result, None);
        assert_eq!(cache.miss_count(), 1);
    }

    #[test]
    fn memo_cache_miss_on_absent_key() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("absent");
        let result = cache.get(&key, 0, &[]);
        assert_eq!(result, None);
        assert_eq!(cache.hit_count(), 0);
        assert_eq!(cache.miss_count(), 0);
    }

    #[test]
    fn memo_cache_invalidate_removes_entry() {
        let mut cache: MemoCache<i32> = MemoCache::new();
        let key = Hash128::of_str("inv_key");
        cache.put(key, 7, Constraint::new(1));
        assert_eq!(cache.len(), 1);
        cache.invalidate(&key);
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.get(&key, 1, &[]), None);
    }

    #[test]
    fn memo_cache_hit_rate_tracks_correctly() {
        let mut cache: MemoCache<u8> = MemoCache::new();
        let key = Hash128::of_str("rate_key");
        cache.put(key, 1, Constraint::new(55));
        cache.get(&key, 55, &[]); // hit
        cache.get(&key, 99, &[]); // miss (stale input)
        assert_eq!(cache.hit_count(), 1);
        assert_eq!(cache.miss_count(), 1);
        assert!((cache.hit_rate() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn cache_store_and_retrieve() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("store_key");
        cache.put(key, 42_u32, Constraint::new(7));
        let result = cache.get(&key, 7, &[]);
        assert_eq!(result, Some(42));
        assert_eq!(cache.hit_count(), 1);
    }

    #[test]
    fn cache_miss_returns_none() {
        let mut cache: MemoCache<String> = MemoCache::new();
        let key = Hash128::of_str("missing_key");
        // Key was never inserted.
        let result = cache.get(&key, 0, &[]);
        assert_eq!(result, None);
        // Absent key is not counted as miss (returns None from the HashMap, not constraint failure).
        assert_eq!(cache.miss_count(), 0);
    }

    #[test]
    fn cache_clear_removes_all_entries() {
        // Insert several entries, clear, confirm length drops to zero.
        let mut cache: MemoCache<u8> = MemoCache::new();
        for i in 0u64..5 {
            let key = Hash128::of_u64(i);
            cache.put(key, i as u8, Constraint::new(i));
        }
        assert_eq!(cache.len(), 5);
        cache.clear();
        assert_eq!(cache.len(), 0);
        // After clear, a previously-stored key returns None.
        let key = Hash128::of_u64(0);
        assert_eq!(cache.get(&key, 0, &[]), None);
    }

    #[test]
    fn hash128_of_empty_string_is_deterministic() {
        let h1 = Hash128::of_str("");
        let h2 = Hash128::of_str("");
        assert_eq!(h1, h2, "Hash128::of_str(\"\") must be deterministic");
        // Empty string hash must differ from non-empty string hash.
        assert_ne!(h1, Hash128::of_str("non-empty"));
    }

    #[test]
    fn tracked_version_increments() {
        // Create two Tracked values with consecutive versions; the later one must
        // have the higher version number.
        use crate::tracked::Tracked;
        let t1 = Tracked::new("v1", 1);
        let t2 = Tracked::new("v2", 2);
        assert!(t2.version > t1.version, "later Tracked must have higher version");
        assert_eq!(t1.version, 1);
        assert_eq!(t2.version, 2);
    }

    #[test]
    fn memo_cache_hit_rate_zero_empty() {
        let cache: MemoCache<u32> = MemoCache::new();
        assert_eq!(cache.hit_rate(), 0.0);
    }

    #[test]
    fn memo_cache_insert_same_key_updates() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("dup_key");
        cache.put(key, 1, Constraint::new(10));
        cache.put(key, 2, Constraint::new(10));
        // Second put overwrites; get should return the latest value
        let result = cache.get(&key, 10, &[]);
        assert_eq!(result, Some(2));
    }

    #[test]
    fn memo_cache_invalidate_key() {
        let mut cache: MemoCache<String> = MemoCache::new();
        let key = Hash128::of_str("to_invalidate");
        cache.put(key, "value".into(), Constraint::new(1));
        cache.invalidate(&key);
        assert_eq!(cache.get(&key, 1, &[]), None);
    }

    #[test]
    fn memo_cache_multiple_keys_independent() {
        let mut cache: MemoCache<u8> = MemoCache::new();
        let k1 = Hash128::of_str("k1");
        let k2 = Hash128::of_str("k2");
        cache.put(k1, 10, Constraint::new(1));
        cache.put(k2, 20, Constraint::new(2));
        assert_eq!(cache.get(&k1, 1, &[]), Some(10));
        assert_eq!(cache.get(&k2, 2, &[]), Some(20));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn memo_cache_default_equals_new() {
        let cache_default: MemoCache<u32> = MemoCache::default();
        assert_eq!(cache_default.len(), 0);
        assert_eq!(cache_default.hit_count(), 0);
        assert_eq!(cache_default.miss_count(), 0);
    }

    #[test]
    fn memo_cache_miss_increments_miss_count() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("present");
        cache.put(key, 5, Constraint::new(100));
        // stale input → miss
        cache.get(&key, 999, &[]);
        assert_eq!(cache.miss_count(), 1);
    }

    #[test]
    fn memo_cache_hit_rate_all_hits() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("all_hits");
        cache.put(key, 1, Constraint::new(7));
        cache.get(&key, 7, &[]);
        cache.get(&key, 7, &[]);
        assert_eq!(cache.hit_count(), 2);
        assert_eq!(cache.miss_count(), 0);
        assert!((cache.hit_rate() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn memo_cache_invalidate_absent_key_is_noop() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("absent");
        // Should not panic on removing a key that was never inserted
        cache.invalidate(&key);
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn memo_cache_many_entries_stored() {
        let mut cache: MemoCache<u64> = MemoCache::new();
        for i in 0u64..20 {
            let key = Hash128::of_u64(i);
            cache.put(key, i * 2, Constraint::new(i));
        }
        assert_eq!(cache.len(), 20);
        for i in 0u64..20 {
            let key = Hash128::of_u64(i);
            assert_eq!(cache.get(&key, i, &[]), Some(i * 2));
        }
    }

    #[test]
    fn memo_cache_store_string_key() {
        let mut cache: MemoCache<String> = MemoCache::new();
        let key = Hash128::of_str("string_key");
        cache.put(key, "hello".to_string(), Constraint::new(1));
        assert_eq!(cache.get(&key, 1, &[]), Some("hello".to_string()));
    }

    #[test]
    fn memo_cache_store_u64_key() {
        let mut cache: MemoCache<u64> = MemoCache::new();
        let raw: u64 = 0xdeadbeef_cafebabe;
        let key = Hash128::of_u64(raw);
        cache.put(key, 999u64, Constraint::new(raw));
        assert_eq!(cache.get(&key, raw, &[]), Some(999u64));
    }

    #[test]
    fn memo_cache_len_empty_is_zero() {
        let cache: MemoCache<u32> = MemoCache::new();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn memo_cache_len_after_three_inserts() {
        let mut cache: MemoCache<u8> = MemoCache::new();
        cache.put(Hash128::of_str("a"), 1, Constraint::new(1));
        cache.put(Hash128::of_str("b"), 2, Constraint::new(2));
        cache.put(Hash128::of_str("c"), 3, Constraint::new(3));
        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn memo_cache_contains_after_insert() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("present_key");
        cache.put(key, 42, Constraint::new(7));
        // A successful get with correct hash confirms the entry exists
        assert_eq!(cache.get(&key, 7, &[]), Some(42));
    }

    #[test]
    fn memo_cache_not_contains_miss() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("never_inserted");
        // get on absent key returns None without counting as a miss
        assert_eq!(cache.get(&key, 0, &[]), None);
        assert_eq!(cache.miss_count(), 0);
    }
}
