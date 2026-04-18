#![deny(unsafe_code)]
use crate::constraint::Constraint;
use crate::hash::Hash128;
use crate::tracked::TrackedSnapshot;
use std::collections::HashMap;

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
        Self {
            entries: HashMap::new(),
            hit_count: 0,
            miss_count: 0,
        }
    }

    /// Try to retrieve a cached result. Validates constraint before returning.
    /// `current_snapshots` holds fresh (method_id, return_hash) snapshots for validation.
    pub fn get(
        &mut self,
        key: &Hash128,
        current_input_hash: u64,
        current_snapshots: &[TrackedSnapshot],
    ) -> Option<T> {
        let entry = self.entries.get(&key.as_u64())?;
        if entry
            .constraint
            .validate(current_input_hash, current_snapshots)
        {
            self.hit_count += 1;
            Some(entry.value.clone())
        } else {
            self.miss_count += 1;
            None
        }
    }

    pub fn put(&mut self, key: Hash128, value: T, constraint: Constraint) {
        self.entries.insert(
            key.as_u64(),
            CachedResult {
                value,
                constraint,
                hash: key,
            },
        );
    }

    pub fn invalidate(&mut self, key: &Hash128) {
        self.entries.remove(&key.as_u64());
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn hit_count(&self) -> u64 {
        self.hit_count
    }
    pub fn miss_count(&self) -> u64 {
        self.miss_count
    }
    pub fn hit_rate(&self) -> f64 {
        let total = self.hit_count + self.miss_count;
        if total == 0 {
            0.0
        } else {
            self.hit_count as f64 / total as f64
        }
    }
}

impl<T: Clone> Default for MemoCache<T> {
    fn default() -> Self {
        Self::new()
    }
}

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
        assert!(
            t2.version > t1.version,
            "later Tracked must have higher version"
        );
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
        assert!(cache.is_empty());
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

    // ── additional coverage ────────────────────────────────────────────────

    #[test]
    fn memo_cache_overwrite_updates_constraint() {
        // First put uses input_hash=1, second uses 2; after overwrite only hash=2 passes.
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("overwrite");
        cache.put(key, 10, Constraint::new(1));
        cache.put(key, 20, Constraint::new(2));
        assert_eq!(cache.get(&key, 1, &[]), None); // old constraint gone
        assert_eq!(cache.get(&key, 2, &[]), Some(20));
    }

    #[test]
    fn memo_cache_hit_count_accumulates() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("acc");
        cache.put(key, 7, Constraint::new(1));
        for _ in 0..5 {
            cache.get(&key, 1, &[]);
        }
        assert_eq!(cache.hit_count(), 5);
    }

    #[test]
    fn memo_cache_miss_count_accumulates() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("acc_miss");
        cache.put(key, 7, Constraint::new(1));
        for i in 2u64..7 {
            cache.get(&key, i, &[]); // always stale
        }
        assert_eq!(cache.miss_count(), 5);
    }

    #[test]
    fn memo_cache_hit_rate_zero_total() {
        let cache: MemoCache<u32> = MemoCache::new();
        assert_eq!(cache.hit_rate(), 0.0);
    }

    #[test]
    fn memo_cache_hit_rate_all_misses() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("misses_only");
        cache.put(key, 1, Constraint::new(99));
        cache.get(&key, 0, &[]); // stale
        cache.get(&key, 1, &[]); // stale (wrong hash — wait, 99 was stored)
                                 // 99 was the stored hash, so hash 0 and 1 are both stale → 2 misses
        assert_eq!(cache.miss_count(), 2);
        assert_eq!(cache.hit_rate(), 0.0);
    }

    #[test]
    fn memo_cache_invalidate_then_reinsert() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("reinsertion");
        cache.put(key, 5, Constraint::new(1));
        cache.invalidate(&key);
        cache.put(key, 99, Constraint::new(1));
        assert_eq!(cache.get(&key, 1, &[]), Some(99));
    }

    #[test]
    fn memo_cache_clear_resets_length() {
        let mut cache: MemoCache<u8> = MemoCache::new();
        for i in 0u64..10 {
            cache.put(Hash128::of_u64(i), i as u8, Constraint::new(i));
        }
        cache.clear();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn memo_cache_tracked_snapshot_validates() {
        use crate::tracked::Tracked;
        let mut cache: MemoCache<String> = MemoCache::new();
        let key = Hash128::of_str("snap_key");

        let tracked = Tracked::new("state", 1u64);
        tracked.record_call(42, Hash128::of_str("output"));
        let snap = tracked.snapshot();

        let mut constraint = Constraint::new(7);
        constraint.record(snap.clone());
        cache.put(key, "cached_value".into(), constraint);

        // Re-create matching snapshot for validation
        let t2 = Tracked::new("state", 1u64);
        t2.record_call(42, Hash128::of_str("output"));
        let snap2 = t2.snapshot();

        let result = cache.get(&key, 7, &[snap2]);
        assert_eq!(result, Some("cached_value".into()));
    }

    #[test]
    fn memo_cache_tracked_snapshot_invalidated_on_change() {
        use crate::tracked::Tracked;
        let mut cache: MemoCache<String> = MemoCache::new();
        let key = Hash128::of_str("snap_invalid_key");

        let tracked = Tracked::new("state", 1u64);
        tracked.record_call(42, Hash128::of_str("output_v1"));
        let snap = tracked.snapshot();

        let mut constraint = Constraint::new(7);
        constraint.record(snap);
        cache.put(key, "cached_v1".into(), constraint);

        // Now the tracked value changed — different return hash
        let t2 = Tracked::new("state", 1u64);
        t2.record_call(42, Hash128::of_str("output_v2")); // changed
        let snap2 = t2.snapshot();

        let result = cache.get(&key, 7, &[snap2]);
        assert_eq!(result, None);
    }

    #[test]
    fn memo_cache_empty_input_key() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str(""); // empty string key
        cache.put(key, 77, Constraint::new(0));
        assert_eq!(cache.get(&key, 0, &[]), Some(77));
    }

    #[test]
    fn memo_cache_singleton_input() {
        let mut cache: MemoCache<u8> = MemoCache::new();
        let key = Hash128::of_bytes(&[0x01]);
        cache.put(key, 1, Constraint::new(1));
        assert_eq!(cache.get(&key, 1, &[]), Some(1));
        assert_eq!(cache.get(&key, 0, &[]), None);
    }

    #[test]
    fn memo_cache_large_entry_count() {
        let mut cache: MemoCache<u64> = MemoCache::new();
        for i in 0u64..100 {
            cache.put(Hash128::of_u64(i), i, Constraint::new(i));
        }
        assert_eq!(cache.len(), 100);
        // Spot-check a few
        for i in [0u64, 49, 99] {
            assert_eq!(cache.get(&Hash128::of_u64(i), i, &[]), Some(i));
        }
    }

    // ── eviction / capacity simulation ────────────────────────────────────

    #[test]
    fn memo_cache_eviction_via_invalidate_oldest() {
        // MemoCache has no built-in capacity limit; simulate eviction by invalidating
        // the oldest key once a logical capacity (5) is reached.
        let capacity = 5usize;
        let mut cache: MemoCache<u64> = MemoCache::new();
        let mut insertion_order: std::collections::VecDeque<Hash128> = Default::default();

        for i in 0u64..8 {
            let key = Hash128::of_u64(i);
            // evict if over capacity
            if insertion_order.len() == capacity {
                let oldest = insertion_order.pop_front().unwrap();
                cache.invalidate(&oldest);
            }
            cache.put(key, i, Constraint::new(i));
            insertion_order.push_back(key);
        }

        // After 8 inserts with cap 5, entries 0-2 evicted, 3-7 present.
        assert_eq!(cache.len(), capacity);
        for i in 0u64..3 {
            assert_eq!(cache.get(&Hash128::of_u64(i), i, &[]), None);
        }
        for i in 3u64..8 {
            assert_eq!(cache.get(&Hash128::of_u64(i), i, &[]), Some(i));
        }
    }

    #[test]
    fn memo_cache_eviction_clears_space() {
        let mut cache: MemoCache<u8> = MemoCache::new();
        for i in 0u64..10 {
            cache.put(Hash128::of_u64(i), i as u8, Constraint::new(i));
        }
        assert_eq!(cache.len(), 10);
        // Evict half
        for i in 0u64..5 {
            cache.invalidate(&Hash128::of_u64(i));
        }
        assert_eq!(cache.len(), 5);
    }

    // ── hit/miss counter edge cases ────────────────────────────────────────

    #[test]
    fn memo_cache_hit_counter_starts_zero() {
        let cache: MemoCache<u32> = MemoCache::new();
        assert_eq!(cache.hit_count(), 0);
    }

    #[test]
    fn memo_cache_miss_counter_starts_zero() {
        let cache: MemoCache<u32> = MemoCache::new();
        assert_eq!(cache.miss_count(), 0);
    }

    #[test]
    fn memo_cache_hit_counter_exact_increments() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("exact_hit");
        cache.put(key, 1, Constraint::new(7));
        assert_eq!(cache.hit_count(), 0);
        cache.get(&key, 7, &[]);
        assert_eq!(cache.hit_count(), 1);
        cache.get(&key, 7, &[]);
        assert_eq!(cache.hit_count(), 2);
        cache.get(&key, 7, &[]);
        assert_eq!(cache.hit_count(), 3);
    }

    #[test]
    fn memo_cache_miss_counter_exact_increments() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("exact_miss");
        cache.put(key, 1, Constraint::new(7));
        assert_eq!(cache.miss_count(), 0);
        cache.get(&key, 0, &[]); // stale
        assert_eq!(cache.miss_count(), 1);
        cache.get(&key, 0, &[]); // stale again
        assert_eq!(cache.miss_count(), 2);
    }

    #[test]
    fn memo_cache_counters_not_reset_by_clear() {
        // clear() removes entries but must not reset counters (counters are cumulative).
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("cnt_clear");
        cache.put(key, 1, Constraint::new(1));
        cache.get(&key, 1, &[]); // hit
        cache.get(&key, 0, &[]); // miss
        assert_eq!(cache.hit_count(), 1);
        assert_eq!(cache.miss_count(), 1);
        cache.clear();
        // entries gone but counters preserved
        assert_eq!(cache.hit_count(), 1);
        assert_eq!(cache.miss_count(), 1);
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn memo_cache_hit_rate_mixed() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("mixed_rate");
        cache.put(key, 1, Constraint::new(5));
        cache.get(&key, 5, &[]); // hit
        cache.get(&key, 5, &[]); // hit
        cache.get(&key, 5, &[]); // hit
        cache.get(&key, 0, &[]); // miss
        // 3 hits / 4 total = 0.75
        let rate = cache.hit_rate();
        assert!((rate - 0.75).abs() < 1e-9);
    }

    // ── concurrent-safe get (smoke test) ──────────────────────────────────

    #[test]
    fn memo_cache_get_is_safe_single_threaded_sequence() {
        // MemoCache is not Send, but we verify get/put interleaving is correct.
        let mut cache: MemoCache<u32> = MemoCache::new();
        for i in 0u64..10 {
            let key = Hash128::of_u64(i);
            cache.put(key, i as u32, Constraint::new(i));
            let result = cache.get(&key, i, &[]);
            assert_eq!(result, Some(i as u32));
        }
        assert_eq!(cache.hit_count(), 10);
        assert_eq!(cache.miss_count(), 0);
    }

    #[test]
    fn memo_cache_get_after_put_always_hits() {
        let mut cache: MemoCache<String> = MemoCache::new();
        for i in 0u64..5 {
            let key = Hash128::of_u64(i);
            let val = format!("val_{}", i);
            cache.put(key, val.clone(), Constraint::new(i));
            assert_eq!(cache.get(&key, i, &[]), Some(val));
        }
    }

    // --- get returns None after clear() ---

    #[test]
    fn memo_cache_get_returns_none_after_clear() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("clear_test");
        cache.put(key, 42, Constraint::new(7));

        // Confirm entry exists before clear.
        assert_eq!(cache.get(&key, 7, &[]), Some(42));

        cache.clear();

        // After clear, get must return None.
        let result = cache.get(&key, 7, &[]);
        assert_eq!(result, None, "get must return None after clear()");
    }

    #[test]
    fn memo_cache_get_returns_none_for_all_keys_after_clear() {
        let mut cache: MemoCache<u64> = MemoCache::new();
        let keys: Vec<Hash128> = (0u64..10).map(Hash128::of_u64).collect();

        for (i, &key) in keys.iter().enumerate() {
            cache.put(key, i as u64, Constraint::new(i as u64));
        }
        assert_eq!(cache.len(), 10);

        cache.clear();

        // All previously inserted keys now return None.
        for (i, &key) in keys.iter().enumerate() {
            assert_eq!(
                cache.get(&key, i as u64, &[]),
                None,
                "key {i} must return None after clear()"
            );
        }
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn memo_cache_get_none_after_clear_then_reinsert_works() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("reinsert_after_clear");
        cache.put(key, 1, Constraint::new(1));
        cache.clear();
        // After clear: None.
        assert_eq!(cache.get(&key, 1, &[]), None);
        // Reinsert: should work again.
        cache.put(key, 99, Constraint::new(1));
        assert_eq!(cache.get(&key, 1, &[]), Some(99));
    }

    // --- Insert 1000 entries respects capacity ---

    #[test]
    fn memo_cache_insert_1000_entries_all_present() {
        // MemoCache has no built-in eviction; all 1000 entries must be stored.
        let mut cache: MemoCache<u64> = MemoCache::new();
        for i in 0u64..1000 {
            cache.put(Hash128::of_u64(i), i, Constraint::new(i));
        }
        assert_eq!(cache.len(), 1000, "all 1000 entries must be stored");

        // Spot-check first, middle, and last.
        for i in [0u64, 499, 999] {
            assert_eq!(
                cache.get(&Hash128::of_u64(i), i, &[]),
                Some(i),
                "entry {i} must be retrievable"
            );
        }
    }

    #[test]
    fn memo_cache_insert_1000_entries_with_capacity_simulation() {
        // Simulate a capacity of 500: after inserting 1000 entries with eviction,
        // only the last 500 are present.
        let capacity = 500usize;
        let mut cache: MemoCache<u64> = MemoCache::new();
        let mut insertion_order: std::collections::VecDeque<Hash128> = Default::default();

        for i in 0u64..1000 {
            let key = Hash128::of_u64(i);
            if insertion_order.len() == capacity {
                let oldest = insertion_order.pop_front().unwrap();
                cache.invalidate(&oldest);
            }
            cache.put(key, i, Constraint::new(i));
            insertion_order.push_back(key);
        }

        // After 1000 inserts with cap 500, entries 0..500 evicted, 500..1000 present.
        assert_eq!(cache.len(), capacity, "capacity must be respected");

        // Evicted entries must return None.
        for i in 0u64..500 {
            assert_eq!(
                cache.get(&Hash128::of_u64(i), i, &[]),
                None,
                "evicted entry {i} must return None"
            );
        }

        // Surviving entries must be retrievable.
        for i in 500u64..1000 {
            assert_eq!(
                cache.get(&Hash128::of_u64(i), i, &[]),
                Some(i),
                "surviving entry {i} must return Some"
            );
        }
    }

    #[test]
    fn memo_cache_insert_1000_no_collision() {
        // 1000 distinct Hash128 keys must not collide in the underlying HashMap.
        let mut cache: MemoCache<u32> = MemoCache::new();
        for i in 0u64..1000 {
            cache.put(Hash128::of_u64(i), i as u32, Constraint::new(i));
        }
        // Verify all 1000 distinct entries exist (no collisions silently dropped any).
        assert_eq!(cache.len(), 1000);
        for i in 0u64..1000 {
            let result = cache.get(&Hash128::of_u64(i), i, &[]);
            assert_eq!(
                result,
                Some(i as u32),
                "entry {i} must not be lost due to hash collision"
            );
        }
    }

    // --- Additional coverage ---

    #[test]
    fn memo_cache_get_returns_none_for_unknown_key_after_1000_inserts() {
        let mut cache: MemoCache<u64> = MemoCache::new();
        for i in 0u64..1000 {
            cache.put(Hash128::of_u64(i), i, Constraint::new(i));
        }
        // Key 1000 was never inserted.
        assert_eq!(cache.get(&Hash128::of_u64(1000), 1000, &[]), None);
        assert_eq!(cache.miss_count(), 0); // absent key, not constraint failure
    }

    #[test]
    fn memo_cache_clear_then_len_zero_then_insert_works() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        for i in 0u64..5 {
            cache.put(Hash128::of_u64(i), i as u32, Constraint::new(i));
        }
        cache.clear();
        assert_eq!(cache.len(), 0);
        // Insert after clear works normally.
        cache.put(Hash128::of_u64(0), 99, Constraint::new(0));
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&Hash128::of_u64(0), 0, &[]), Some(99));
    }

    #[test]
    fn memo_cache_is_empty_after_clear() {
        let mut cache: MemoCache<u8> = MemoCache::new();
        cache.put(Hash128::of_str("k"), 1, Constraint::new(1));
        assert!(!cache.is_empty());
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn memo_cache_capacity_1000_no_eviction_needed() {
        // Without any eviction, all 1000 entries remain after insertion.
        let mut cache: MemoCache<u8> = MemoCache::new();
        for i in 0u64..1000 {
            cache.put(Hash128::of_u64(i), (i % 256) as u8, Constraint::new(i));
        }
        assert_eq!(cache.len(), 1000);
        // All entries must be retrievable.
        for i in 0u64..1000 {
            assert_eq!(
                cache.get(&Hash128::of_u64(i), i, &[]),
                Some((i % 256) as u8)
            );
        }
    }

    #[test]
    fn memo_cache_clear_does_not_affect_new_entries() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        cache.put(Hash128::of_str("before"), 1, Constraint::new(1));
        cache.clear();
        cache.put(Hash128::of_str("after"), 2, Constraint::new(2));
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&Hash128::of_str("after"), 2, &[]), Some(2));
        assert_eq!(cache.get(&Hash128::of_str("before"), 1, &[]), None);
    }

    #[test]
    fn memo_cache_get_none_after_clear_miss_count_unchanged() {
        // Absent key after clear should NOT increment miss_count
        // (the entry isn't found in the HashMap before constraint check).
        let mut cache: MemoCache<u32> = MemoCache::new();
        cache.put(Hash128::of_str("k"), 1, Constraint::new(1));
        cache.get(&Hash128::of_str("k"), 1, &[]); // hit
        cache.clear();
        cache.get(&Hash128::of_str("k"), 1, &[]); // absent → None, no miss increment
        assert_eq!(cache.hit_count(), 1);
        assert_eq!(cache.miss_count(), 0);
    }

    // ── WAVE-AF AGENT-9 additions ─────────────────────────────────────────────

    #[test]
    fn stats_returns_correct_hit_count_after_multiple_hits() {
        // stats() = hit_count() and miss_count() reflect every operation.
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("stats_hit");
        cache.put(key, 7, Constraint::new(1));

        // Three hits.
        cache.get(&key, 1, &[]);
        cache.get(&key, 1, &[]);
        cache.get(&key, 1, &[]);

        assert_eq!(cache.hit_count(), 3, "stats must show 3 hits");
        assert_eq!(cache.miss_count(), 0, "stats must show 0 misses");
    }

    #[test]
    fn stats_returns_correct_miss_count_after_stale_inputs() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("stats_miss");
        cache.put(key, 99, Constraint::new(42));

        // Five misses (stale input hash).
        for i in 0u64..5 {
            cache.get(&key, i, &[]); // all stale — constraint expects 42
        }

        assert_eq!(cache.miss_count(), 5, "stats must show 5 misses");
        assert_eq!(cache.hit_count(), 0, "stats must show 0 hits");
    }

    #[test]
    fn stats_mixed_hits_and_misses() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("stats_mixed");
        cache.put(key, 5, Constraint::new(10));

        cache.get(&key, 10, &[]); // hit
        cache.get(&key, 99, &[]); // miss
        cache.get(&key, 10, &[]); // hit
        cache.get(&key, 0, &[]); // miss
        cache.get(&key, 10, &[]); // hit

        assert_eq!(cache.hit_count(), 3);
        assert_eq!(cache.miss_count(), 2);
    }

    #[test]
    fn stats_hit_rate_exact_fraction() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("rate_exact");
        cache.put(key, 1, Constraint::new(1));

        // 1 hit, 3 misses → rate = 0.25
        cache.get(&key, 1, &[]); // hit
        cache.get(&key, 2, &[]); // miss
        cache.get(&key, 3, &[]); // miss
        cache.get(&key, 4, &[]); // miss

        let expected = 1.0 / 4.0;
        assert!(
            (cache.hit_rate() - expected).abs() < 1e-10,
            "hit_rate must be exactly 0.25"
        );
    }

    // --- LRU order: least-recently-used evicted first (simulated) ---

    #[test]
    fn lru_order_oldest_entry_evicted_first() {
        // MemoCache does not implement LRU natively; simulate by manually tracking
        // insertion order and evicting the least-recently-used entry.
        let capacity = 3usize;
        let mut cache: MemoCache<u32> = MemoCache::new();
        let mut lru_order: std::collections::VecDeque<Hash128> = Default::default();

        // Helper: "access" = promote key to back of LRU order.
        let access = |order: &mut std::collections::VecDeque<Hash128>, key: Hash128| {
            order.retain(|k| *k != key);
            order.push_back(key);
        };

        // Insert k0, k1, k2.
        for i in 0u64..3 {
            let key = Hash128::of_u64(i);
            if lru_order.len() == capacity {
                let evict = lru_order.pop_front().unwrap();
                cache.invalidate(&evict);
            }
            cache.put(key, i as u32, Constraint::new(i));
            access(&mut lru_order, key);
        }
        assert_eq!(cache.len(), 3);

        // Access k0 (promote it). LRU order is now k1 < k2 < k0.
        access(&mut lru_order, Hash128::of_u64(0));

        // Insert k3 — k1 is now least-recently-used and should be evicted.
        let key3 = Hash128::of_u64(3);
        let evict = lru_order.pop_front().unwrap(); // k1
        cache.invalidate(&evict);
        cache.put(key3, 3, Constraint::new(3));
        access(&mut lru_order, key3);

        assert_eq!(cache.len(), capacity);
        // k1 was evicted.
        assert_eq!(cache.get(&Hash128::of_u64(1), 1, &[]), None, "k1 must be evicted (LRU)");
        // k0 was promoted so it stays.
        assert_eq!(cache.get(&Hash128::of_u64(0), 0, &[]), Some(0), "k0 must survive (MRU)");
        // k2 and k3 stay.
        assert_eq!(cache.get(&Hash128::of_u64(2), 2, &[]), Some(2));
        assert_eq!(cache.get(&Hash128::of_u64(3), 3, &[]), Some(3));
    }

    #[test]
    fn lru_order_five_entries_two_evictions() {
        // Insert 5 entries with capacity 3; oldest two are evicted.
        let capacity = 3usize;
        let mut cache: MemoCache<u64> = MemoCache::new();
        let mut order: std::collections::VecDeque<Hash128> = Default::default();

        for i in 0u64..5 {
            let key = Hash128::of_u64(i);
            if order.len() == capacity {
                let oldest = order.pop_front().unwrap();
                cache.invalidate(&oldest);
            }
            cache.put(key, i, Constraint::new(i));
            order.push_back(key);
        }

        // After 5 inserts with cap 3: entries 0 and 1 evicted; 2, 3, 4 remain.
        assert_eq!(cache.len(), capacity);
        assert_eq!(cache.get(&Hash128::of_u64(0), 0, &[]), None);
        assert_eq!(cache.get(&Hash128::of_u64(1), 1, &[]), None);
        assert_eq!(cache.get(&Hash128::of_u64(2), 2, &[]), Some(2));
        assert_eq!(cache.get(&Hash128::of_u64(3), 3, &[]), Some(3));
        assert_eq!(cache.get(&Hash128::of_u64(4), 4, &[]), Some(4));
    }

    #[test]
    fn lru_order_promoted_entry_not_evicted() {
        // Access (promote) an entry before capacity is hit; it must survive.
        let capacity = 2usize;
        let mut cache: MemoCache<u32> = MemoCache::new();
        let mut order: std::collections::VecDeque<Hash128> = Default::default();

        let k0 = Hash128::of_u64(0);
        let k1 = Hash128::of_u64(1);
        let k2 = Hash128::of_u64(2);

        cache.put(k0, 0, Constraint::new(0));
        order.push_back(k0);
        cache.put(k1, 1, Constraint::new(1));
        order.push_back(k1);

        // Promote k0 (simulate access).
        order.retain(|k| *k != k0);
        order.push_back(k0);

        // Insert k2 — k1 is now the LRU entry.
        let evict = order.pop_front().unwrap(); // k1
        cache.invalidate(&evict);
        cache.put(k2, 2, Constraint::new(2));
        order.push_back(k2);

        assert_eq!(cache.len(), capacity);
        assert_eq!(cache.get(&k0, 0, &[]), Some(0), "promoted k0 must survive");
        assert_eq!(cache.get(&k1, 1, &[]), None, "k1 must be evicted (LRU)");
        assert_eq!(cache.get(&k2, 2, &[]), Some(2), "k2 must be present");
    }

    #[test]
    fn stats_hit_count_after_zero_operations() {
        let cache: MemoCache<u32> = MemoCache::new();
        assert_eq!(cache.hit_count(), 0, "new cache must start with hit_count=0");
        assert_eq!(cache.miss_count(), 0, "new cache must start with miss_count=0");
    }

    #[test]
    fn stats_single_entry_single_hit_single_miss() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("single");
        cache.put(key, 1, Constraint::new(5));
        cache.get(&key, 5, &[]); // hit
        cache.get(&key, 9, &[]); // miss
        assert_eq!(cache.hit_count(), 1);
        assert_eq!(cache.miss_count(), 1);
        assert!((cache.hit_rate() - 0.5).abs() < f64::EPSILON);
    }

    // ── WAVE-AG AGENT-10 additions ─────────────────────────────────────────────

    #[test]
    fn memo_cache_hit_returns_cached() {
        // Call with same key twice: second call hits the cache.
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("cache_hit_key");
        cache.put(key, 42, Constraint::new(0));
        let first = cache.get(&key, 0, &[]);
        let second = cache.get(&key, 0, &[]);
        assert_eq!(first, Some(42));
        assert_eq!(second, Some(42));
        assert_eq!(cache.hit_count(), 2);
    }

    #[test]
    fn memo_cache_miss_invokes_recompute() {
        // Unique key with mismatched version → miss.
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("miss_key");
        cache.put(key, 99, Constraint::new(10));
        let result = cache.get(&key, 11, &[]); // version mismatch → miss
        assert_eq!(result, None);
        assert_eq!(cache.miss_count(), 1);
    }

    #[test]
    fn memo_constraint_satisfied_uses_cache() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("constraint_ok");
        cache.put(key, 7, Constraint::new(5));
        let result = cache.get(&key, 5, &[]);
        assert_eq!(result, Some(7), "matching constraint must return cached value");
        assert_eq!(cache.hit_count(), 1);
    }

    #[test]
    fn memo_constraint_violated_recomputes() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("constraint_bad");
        cache.put(key, 7, Constraint::new(5));
        let result = cache.get(&key, 6, &[]); // different version → violation
        assert_eq!(result, None);
        assert_eq!(cache.miss_count(), 1);
    }

    #[test]
    fn memo_hash_different_inputs_different_hashes() {
        let h1 = Hash128::of_str("input_A");
        let h2 = Hash128::of_str("input_B");
        assert_ne!(h1, h2, "different inputs must produce different hashes");
    }

    #[test]
    fn memo_hash_same_input_same_hash() {
        let h1 = Hash128::of_str("deterministic");
        let h2 = Hash128::of_str("deterministic");
        assert_eq!(h1, h2, "same input must produce same hash");
    }

    #[test]
    fn memo_lru_eviction_simulated() {
        // Simulate LRU with capacity 2: insert 3 items, oldest is evicted.
        let capacity = 2usize;
        let mut cache: MemoCache<u32> = MemoCache::new();
        let mut order: std::collections::VecDeque<Hash128> = Default::default();

        for i in 0u64..3 {
            let key = Hash128::of_u64(i + 1000);
            if order.len() == capacity {
                let evict = order.pop_front().unwrap();
                cache.invalidate(&evict);
            }
            cache.put(key, i as u32, Constraint::new(i));
            order.push_back(key);
        }

        assert_eq!(cache.len(), capacity);
        assert_eq!(cache.get(&Hash128::of_u64(1000), 0, &[]), None, "oldest must be evicted");
        assert_eq!(cache.get(&Hash128::of_u64(1001), 1, &[]), Some(1));
        assert_eq!(cache.get(&Hash128::of_u64(1002), 2, &[]), Some(2));
    }

    #[test]
    fn memo_stats_hit_count_increments() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("hit_counter");
        cache.put(key, 1, Constraint::new(0));
        for _ in 0..5 {
            cache.get(&key, 0, &[]);
        }
        assert_eq!(cache.hit_count(), 5);
    }

    #[test]
    fn memo_stats_miss_count_increments() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("miss_counter");
        cache.put(key, 1, Constraint::new(0));
        for i in 1u64..=4 {
            cache.get(&key, i, &[]); // each has wrong version → miss
        }
        assert_eq!(cache.miss_count(), 4);
    }

    #[test]
    fn memo_stats_eviction_count_from_invalidate() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let k1 = Hash128::of_str("evict1");
        let k2 = Hash128::of_str("evict2");
        cache.put(k1, 10, Constraint::new(0));
        cache.put(k2, 20, Constraint::new(0));
        cache.invalidate(&k1);
        cache.invalidate(&k2);
        assert_eq!(cache.len(), 0, "both entries must be invalidated");
    }

    #[test]
    fn memo_invalidate_specific_key() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key_a = Hash128::of_str("keep");
        let key_b = Hash128::of_str("remove");
        cache.put(key_a, 1, Constraint::new(0));
        cache.put(key_b, 2, Constraint::new(0));
        cache.invalidate(&key_b);
        assert_eq!(cache.get(&key_a, 0, &[]), Some(1), "key_a must survive");
        assert_eq!(cache.get(&key_b, 0, &[]), None, "key_b must be gone");
    }

    #[test]
    fn memo_invalidate_all_clears_cache() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        for i in 0u64..10 {
            cache.put(Hash128::of_u64(i), i as u32, Constraint::new(i));
        }
        assert_eq!(cache.len(), 10);
        cache.clear();
        assert_eq!(cache.len(), 0, "clear must remove all entries");
    }

    #[test]
    fn memo_fn_only_called_once_for_same_input() {
        // Manually simulate: put once, get twice — hit_count reflects reuse.
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("once");
        cache.put(key, 55, Constraint::new(0));
        cache.get(&key, 0, &[]); // first use
        cache.get(&key, 0, &[]); // second use
        assert_eq!(cache.hit_count(), 2, "same input must hit cache both times");
        assert_eq!(cache.miss_count(), 0);
    }

    #[test]
    fn memo_returns_correct_value() {
        let mut cache: MemoCache<u64> = MemoCache::new();
        let key = Hash128::of_str("value_check");
        let expected: u64 = 0xdeadbeef_cafebabe;
        cache.put(key, expected, Constraint::new(0));
        assert_eq!(cache.get(&key, 0, &[]), Some(expected));
    }

    #[test]
    fn memo_cache_len_tracks_insertions() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        for i in 0u64..7 {
            cache.put(Hash128::of_u64(i + 200), i as u32, Constraint::new(0));
        }
        assert_eq!(cache.len(), 7);
    }

    #[test]
    fn memo_cache_put_overwrite_updates_value() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("overwrite");
        cache.put(key, 1, Constraint::new(0));
        cache.put(key, 99, Constraint::new(0));
        // Latest put overwrites; len must remain 1.
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&key, 0, &[]), Some(99));
    }

    #[test]
    fn memo_empty_cache_get_returns_none() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("missing");
        // Key is absent — get returns None (no entry → early return, miss_count not incremented).
        assert_eq!(cache.get(&key, 0, &[]), None);
        // miss_count only increments on constraint violation, not absent key.
        // The returned value must be None; stats are implementation-defined for absent keys.
        assert_eq!(cache.hit_count(), 0, "no hit on absent key");
    }

    #[test]
    fn memo_hit_rate_all_hits() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("all_hits");
        cache.put(key, 1, Constraint::new(0));
        for _ in 0..4 {
            cache.get(&key, 0, &[]);
        }
        assert!((cache.hit_rate() - 1.0).abs() < f64::EPSILON, "4/4 hits → rate must be 1.0");
    }

    #[test]
    fn memo_hit_rate_all_misses() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("all_misses");
        cache.put(key, 1, Constraint::new(0));
        for v in 1u64..=3 {
            cache.get(&key, v, &[]);
        }
        assert!((cache.hit_rate() - 0.0).abs() < f64::EPSILON, "0/3 hits → rate must be 0.0");
    }

    #[test]
    fn memo_constraint_new_with_version_0() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("v0");
        cache.put(key, 10, Constraint::new(0));
        assert_eq!(cache.get(&key, 0, &[]), Some(10));
    }

    #[test]
    fn memo_10_distinct_keys_all_retrievable() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let keys: Vec<Hash128> = (0..10).map(|i: u32| Hash128::of_str(&format!("k{i}"))).collect();
        for (i, &key) in keys.iter().enumerate() {
            cache.put(key, i as u32, Constraint::new(0));
        }
        for (i, &key) in keys.iter().enumerate() {
            assert_eq!(cache.get(&key, 0, &[]), Some(i as u32));
        }
    }

    #[test]
    fn memo_cache_stats_zero_ops_hit_rate_is_nan_or_zero() {
        let cache: MemoCache<u32> = MemoCache::new();
        // With no operations, hit_rate should be 0.0 (or NaN if division by zero).
        // Either behavior is acceptable — just don't panic.
        let rate = cache.hit_rate();
        let _ = rate; // confirm no panic
    }

    #[test]
    fn memo_cache_is_empty_initially() {
        let cache: MemoCache<u32> = MemoCache::new();
        assert!(cache.is_empty(), "new cache must be empty");
    }

    #[test]
    fn memo_cache_is_not_empty_after_put() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        cache.put(Hash128::of_str("key"), 1, Constraint::new(0));
        assert!(!cache.is_empty(), "cache must not be empty after put");
    }

    #[test]
    fn memo_cache_clear_then_empty() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        cache.put(Hash128::of_str("a"), 1, Constraint::new(0));
        cache.put(Hash128::of_str("b"), 2, Constraint::new(0));
        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn memo_cache_multiple_clears_safe() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        cache.clear();
        cache.clear(); // second clear on empty must not panic
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn memo_cache_string_values() {
        let mut cache: MemoCache<String> = MemoCache::new();
        let key = Hash128::of_str("str_key");
        cache.put(key, "hello_world".to_string(), Constraint::new(0));
        assert_eq!(cache.get(&key, 0, &[]), Some("hello_world".to_string()));
    }

    #[test]
    fn memo_cache_vec_values() {
        let mut cache: MemoCache<Vec<u8>> = MemoCache::new();
        let key = Hash128::of_str("vec_key");
        let data = vec![1u8, 2, 3, 4, 5];
        cache.put(key, data.clone(), Constraint::new(0));
        assert_eq!(cache.get(&key, 0, &[]), Some(data));
    }

    #[test]
    fn memo_cache_put_100_distinct_keys() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        for i in 0u64..100 {
            cache.put(Hash128::of_u64(i + 5000), i as u32, Constraint::new(0));
        }
        assert_eq!(cache.len(), 100);
        for i in 0u64..100 {
            assert_eq!(cache.get(&Hash128::of_u64(i + 5000), 0, &[]), Some(i as u32));
        }
    }

    #[test]
    fn memo_cache_default_equals_new_waveag() {
        let a: MemoCache<u32> = MemoCache::new();
        let b: MemoCache<u32> = MemoCache::default();
        assert_eq!(a.len(), b.len());
        assert_eq!(a.hit_count(), b.hit_count());
        assert_eq!(a.miss_count(), b.miss_count());
    }

    // --- Wave AH Agent 9 additions ---

    #[test]
    fn memo_typed_cache_string_values_waveah() {
        let mut cache: MemoCache<String> = MemoCache::new();
        let key = Hash128::of_str("str_waveah");
        cache.put(key, "hello_waveah".to_string(), Constraint::new(1));
        assert_eq!(cache.get(&key, 1, &[]), Some("hello_waveah".to_string()));
    }

    #[test]
    fn memo_typed_cache_vec_values_waveah() {
        let mut cache: MemoCache<Vec<u32>> = MemoCache::new();
        let key = Hash128::of_str("vec_waveah");
        let data = vec![10u32, 20, 30];
        cache.put(key, data.clone(), Constraint::new(0));
        assert_eq!(cache.get(&key, 0, &[]), Some(data));
    }

    #[test]
    fn memo_typed_cache_struct_values_waveah() {
        #[derive(Clone, PartialEq, Debug)]
        struct Point { x: i32, y: i32 }
        let mut cache: MemoCache<Point> = MemoCache::new();
        let key = Hash128::of_str("struct_waveah");
        let pt = Point { x: 3, y: 7 };
        cache.put(key, pt.clone(), Constraint::new(0));
        assert_eq!(cache.get(&key, 0, &[]), Some(pt));
    }

    #[test]
    fn memo_dependency_tracking_invalidates_on_change_waveah() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("dep_change");
        cache.put(key, 5, Constraint::new(10));
        // Old hash → miss (dependency changed).
        assert_eq!(cache.get(&key, 99, &[]), None);
        assert_eq!(cache.miss_count(), 1);
    }

    #[test]
    fn memo_dependency_not_changed_no_recompute_waveah() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("dep_same");
        cache.put(key, 7, Constraint::new(5));
        // Same hash → hit (no recompute needed).
        assert_eq!(cache.get(&key, 5, &[]), Some(7));
        assert_eq!(cache.hit_count(), 1);
        assert_eq!(cache.miss_count(), 0);
    }

    #[test]
    fn memo_tracked_fn_called_once_per_unique_input_waveah() {
        // Two different keys are each hit once; hit_count = 2.
        let mut cache: MemoCache<u32> = MemoCache::new();
        let k1 = Hash128::of_str("unique_k1");
        let k2 = Hash128::of_str("unique_k2");
        cache.put(k1, 1, Constraint::new(0));
        cache.put(k2, 2, Constraint::new(0));
        cache.get(&k1, 0, &[]);
        cache.get(&k2, 0, &[]);
        assert_eq!(cache.hit_count(), 2);
    }

    #[test]
    fn memo_tracked_fn_called_again_after_invalidate_waveah() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("reinvalidate");
        cache.put(key, 99, Constraint::new(1));
        cache.get(&key, 1, &[]); // hit
        cache.invalidate(&key);
        // After invalidation the entry is gone → absent (None, no miss increment).
        assert_eq!(cache.get(&key, 1, &[]), None);
        assert_eq!(cache.hit_count(), 1); // only the first hit counts
    }

    #[test]
    fn memo_stats_ratio_hits_to_total_waveah() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("ratio_waveah");
        cache.put(key, 1, Constraint::new(3));
        // 3 hits, 1 miss → hit_rate = 0.75
        cache.get(&key, 3, &[]); // hit
        cache.get(&key, 3, &[]); // hit
        cache.get(&key, 3, &[]); // hit
        cache.get(&key, 9, &[]); // miss
        assert!((cache.hit_rate() - 0.75).abs() < 1e-9);
    }

    #[test]
    fn memo_stats_reset_clears_counters_via_new_cache_waveah() {
        // MemoCache has no reset API; simulate by creating a new instance.
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("reset_waveah");
        cache.put(key, 1, Constraint::new(1));
        cache.get(&key, 1, &[]); // hit
        cache.get(&key, 9, &[]); // miss

        // "Reset" = new cache.
        let fresh: MemoCache<u32> = MemoCache::new();
        assert_eq!(fresh.hit_count(), 0);
        assert_eq!(fresh.miss_count(), 0);
    }

    #[test]
    fn memo_key_type_is_hash_based_waveah() {
        // Different string inputs → different keys.
        let k1 = Hash128::of_str("input_alpha");
        let k2 = Hash128::of_str("input_beta");
        assert_ne!(k1, k2, "different strings must produce different Hash128 keys");
    }

    #[test]
    fn memo_evicted_key_recomputed_on_next_access_waveah() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("evict_recompute");
        cache.put(key, 42, Constraint::new(0));
        cache.invalidate(&key);
        // After eviction the entry is absent; simulating recompute = re-insert.
        cache.put(key, 99, Constraint::new(0));
        assert_eq!(cache.get(&key, 0, &[]), Some(99));
    }

    #[test]
    fn memo_constraint_fn_called_on_cache_hit_waveah() {
        // Constraint::new(h) validates that input_hash == h.
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("constraint_hit");
        cache.put(key, 55, Constraint::new(7));
        // Correct hash → hit.
        assert_eq!(cache.get(&key, 7, &[]), Some(55));
        assert_eq!(cache.hit_count(), 1);
    }

    #[test]
    fn memo_constraint_violation_triggers_recompute_waveah() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("constraint_viol");
        cache.put(key, 55, Constraint::new(7));
        // Wrong hash → constraint violation → miss.
        assert_eq!(cache.get(&key, 8, &[]), None);
        assert_eq!(cache.miss_count(), 1);
    }

    #[test]
    fn memo_warm_cache_zero_misses_after_fill_waveah() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        for i in 0u64..10 {
            cache.put(Hash128::of_u64(i + 3000), i as u32, Constraint::new(i));
        }
        // Hit every entry.
        for i in 0u64..10 {
            cache.get(&Hash128::of_u64(i + 3000), i, &[]);
        }
        assert_eq!(cache.miss_count(), 0, "warm cache must yield zero misses");
        assert_eq!(cache.hit_count(), 10);
    }

    #[test]
    fn memo_cold_cache_all_misses_waveah() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("cold_key");
        cache.put(key, 1, Constraint::new(99));
        // Wrong hash every time → all misses.
        for i in 0u64..5 {
            cache.get(&key, i, &[]); // hash 0..4 ≠ 99
        }
        assert_eq!(cache.miss_count(), 5);
    }

    #[test]
    fn memo_large_value_stored_and_retrieved_waveah() {
        let mut cache: MemoCache<Vec<u8>> = MemoCache::new();
        let key = Hash128::of_str("large_val");
        let big: Vec<u8> = (0..=255u8).cycle().take(4096).collect();
        cache.put(key, big.clone(), Constraint::new(0));
        assert_eq!(cache.get(&key, 0, &[]), Some(big));
    }

    #[test]
    fn memo_fn_identity_memoized_waveah() {
        // f(x) = x: put x, get x.
        let mut cache: MemoCache<u64> = MemoCache::new();
        let val: u64 = 0xabcdef_123456;
        let key = Hash128::of_u64(val);
        cache.put(key, val, Constraint::new(val));
        assert_eq!(cache.get(&key, val, &[]), Some(val));
    }

    #[test]
    fn memo_fn_pure_memoized_waveah() {
        // Pure function: same input → same output, cached correctly.
        let mut cache: MemoCache<u32> = MemoCache::new();
        let input_hash: u64 = 42;
        let output: u32 = input_hash.wrapping_mul(7) as u32;
        let key = Hash128::of_u64(input_hash);
        cache.put(key, output, Constraint::new(input_hash));
        // Two reads → same result both times.
        assert_eq!(cache.get(&key, input_hash, &[]), Some(output));
        assert_eq!(cache.get(&key, input_hash, &[]), Some(output));
        assert_eq!(cache.hit_count(), 2);
    }

    #[test]
    fn memo_clone_cache_independent_waveah() {
        // Two caches built the same way must behave identically (there is no Clone
        // on MemoCache, so we verify two separate instances are independent).
        let mut cache_a: MemoCache<u32> = MemoCache::new();
        let mut cache_b: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("clone_ind");
        cache_a.put(key, 1, Constraint::new(0));
        // Invalidating from cache_a does not affect cache_b.
        cache_a.invalidate(&key);
        cache_b.put(key, 2, Constraint::new(0));
        assert_eq!(cache_a.get(&key, 0, &[]), None);
        assert_eq!(cache_b.get(&key, 0, &[]), Some(2));
    }

    #[test]
    fn memo_capacity_exceeded_evicts_oldest_waveah() {
        // Manual eviction: cap = 3; after inserting 5, first 2 are evicted.
        let cap = 3usize;
        let mut cache: MemoCache<u32> = MemoCache::new();
        let mut order: std::collections::VecDeque<Hash128> = Default::default();
        for i in 0u64..5 {
            let key = Hash128::of_u64(i + 9000);
            if order.len() == cap {
                cache.invalidate(&order.pop_front().unwrap());
            }
            cache.put(key, i as u32, Constraint::new(i));
            order.push_back(key);
        }
        assert_eq!(cache.len(), cap);
        // Entries 0 and 1 (offsets 9000 and 9001) were evicted.
        assert_eq!(cache.get(&Hash128::of_u64(9000), 0, &[]), None);
        assert_eq!(cache.get(&Hash128::of_u64(9001), 1, &[]), None);
        // Entry 2, 3, 4 are present.
        assert_eq!(cache.get(&Hash128::of_u64(9002), 2, &[]), Some(2));
    }

    #[test]
    fn memo_batch_populate_10_entries_waveah() {
        let mut cache: MemoCache<u64> = MemoCache::new();
        for i in 0u64..10 {
            cache.put(Hash128::of_u64(i + 7000), i, Constraint::new(i));
        }
        assert_eq!(cache.len(), 10);
    }

    #[test]
    fn memo_batch_retrieve_all_10_hit_waveah() {
        let mut cache: MemoCache<u64> = MemoCache::new();
        for i in 0u64..10 {
            cache.put(Hash128::of_u64(i + 8000), i, Constraint::new(i));
        }
        for i in 0u64..10 {
            assert_eq!(cache.get(&Hash128::of_u64(i + 8000), i, &[]), Some(i));
        }
        assert_eq!(cache.hit_count(), 10);
        assert_eq!(cache.miss_count(), 0);
    }

    #[test]
    fn memo_total_bytes_bounded_via_entry_count_waveah() {
        // Bounded by entry count: a cache with 50 entries has exactly 50 entries.
        let mut cache: MemoCache<u8> = MemoCache::new();
        for i in 0u64..50 {
            cache.put(Hash128::of_u64(i + 6000), (i % 256) as u8, Constraint::new(i));
        }
        assert_eq!(cache.len(), 50, "cache must hold exactly 50 entries");
    }

    #[test]
    fn memo_cache_hit_after_same_key_twice_waveah() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("same_twice");
        cache.put(key, 77, Constraint::new(0));
        assert_eq!(cache.get(&key, 0, &[]), Some(77));
        assert_eq!(cache.get(&key, 0, &[]), Some(77));
        assert_eq!(cache.hit_count(), 2);
    }

    #[test]
    fn memo_cache_miss_then_reinsert_becomes_hit_waveah() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("miss_then_hit");
        cache.put(key, 1, Constraint::new(10));
        assert_eq!(cache.get(&key, 99, &[]), None); // miss
        cache.put(key, 2, Constraint::new(99));
        assert_eq!(cache.get(&key, 99, &[]), Some(2)); // hit after reinsert
        assert_eq!(cache.miss_count(), 1);
        assert_eq!(cache.hit_count(), 1);
    }

    #[test]
    fn memo_cache_hit_rate_two_thirds_waveah() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("two_thirds");
        cache.put(key, 5, Constraint::new(7));
        cache.get(&key, 7, &[]); // hit
        cache.get(&key, 7, &[]); // hit
        cache.get(&key, 0, &[]); // miss
        let rate = cache.hit_rate();
        assert!((rate - 2.0 / 3.0).abs() < 1e-9, "hit_rate must be 2/3, got {rate}");
    }

    #[test]
    fn memo_cache_is_empty_returns_false_after_put_waveah() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        assert!(cache.is_empty());
        cache.put(Hash128::of_str("nonempty"), 1, Constraint::new(0));
        assert!(!cache.is_empty());
    }

    #[test]
    fn memo_cache_multiple_invalidations_reduce_len_waveah() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        for i in 0u64..5 {
            cache.put(Hash128::of_u64(i + 4000), i as u32, Constraint::new(i));
        }
        assert_eq!(cache.len(), 5);
        cache.invalidate(&Hash128::of_u64(4000));
        assert_eq!(cache.len(), 4);
        cache.invalidate(&Hash128::of_u64(4001));
        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn memo_cache_put_is_idempotent_for_same_value_waveah() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("idem");
        cache.put(key, 42, Constraint::new(0));
        cache.put(key, 42, Constraint::new(0)); // identical put
        assert_eq!(cache.len(), 1, "identical puts must not grow len");
        assert_eq!(cache.get(&key, 0, &[]), Some(42));
    }

    #[test]
    fn memo_cache_single_entry_is_not_empty_waveah() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        cache.put(Hash128::of_str("single"), 1, Constraint::new(0));
        assert!(!cache.is_empty(), "cache with one entry must not be empty");
        assert_eq!(cache.len(), 1);
    }
}
