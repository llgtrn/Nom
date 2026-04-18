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
}
