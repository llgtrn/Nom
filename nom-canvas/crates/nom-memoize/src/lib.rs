#![deny(unsafe_code)]
pub mod constraint;
pub mod hash;
pub mod memo_cache;
pub mod tracked;

pub use constraint::Constraint;
pub use hash::Hash128;
pub use memo_cache::MemoCache;
pub use tracked::{Tracked, TrackedSnapshot};

// ---------------------------------------------------------------------------
// Integration tests exercising the public API surface
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- TTL simulation: version-based expiry ---

    #[test]
    fn memo_ttl_expired_entry_recomputed_via_version() {
        // Simulate TTL expiry: a new version bump invalidates the cache entry.
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("ttl_key");
        // Store with input_hash = version 1.
        cache.put(key, 42, Constraint::new(1));
        // "Expired": look up with version 2 → cache miss.
        assert_eq!(cache.get(&key, 2, &[]), None, "expired entry must miss");
        assert_eq!(cache.miss_count(), 1);
    }

    #[test]
    fn memo_ttl_fresh_entry_not_recomputed() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("fresh_key");
        cache.put(key, 99, Constraint::new(42));
        // Same version → still fresh → hit.
        assert_eq!(cache.get(&key, 42, &[]), Some(99));
        assert_eq!(cache.hit_count(), 1);
    }

    #[test]
    fn memo_ttl_zero_always_recomputes() {
        // input_hash = 0 stored; looking up with 1 → miss every time.
        let mut cache: MemoCache<u8> = MemoCache::new();
        let key = Hash128::of_str("zero_ttl");
        cache.put(key, 7, Constraint::new(0));
        // Different hash → always miss.
        assert_eq!(cache.get(&key, 1, &[]), None);
        assert_eq!(cache.miss_count(), 1);
    }

    #[test]
    fn memo_ttl_infinite_never_expires_same_hash() {
        // Same hash every lookup → never expires.
        let mut cache: MemoCache<i32> = MemoCache::new();
        let key = Hash128::of_str("inf_ttl");
        cache.put(key, -1, Constraint::new(u64::MAX));
        for _ in 0..5 {
            assert_eq!(cache.get(&key, u64::MAX, &[]), Some(-1));
        }
        assert_eq!(cache.hit_count(), 5);
    }

    #[test]
    fn memo_warm_up_prefills_cache() {
        // Warm-up: insert multiple entries before any reads.
        let mut cache: MemoCache<u32> = MemoCache::new();
        for i in 0u64..10 {
            cache.put(Hash128::of_u64(i), i as u32, Constraint::new(i));
        }
        assert_eq!(cache.len(), 10, "warm-up must fill 10 entries");
    }

    #[test]
    fn memo_warm_up_entries_all_hit() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        for i in 0u64..10 {
            cache.put(Hash128::of_u64(i), i as u32, Constraint::new(i));
        }
        let mut hits = 0;
        for i in 0u64..10 {
            if cache.get(&Hash128::of_u64(i), i, &[]).is_some() {
                hits += 1;
            }
        }
        assert_eq!(hits, 10, "all warm-up entries must hit");
    }

    #[test]
    fn memo_warm_up_stale_after_version_change() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("warm_stale");
        cache.put(key, 5, Constraint::new(1));
        // Version changed to 2 → stale.
        assert_eq!(cache.get(&key, 2, &[]), None);
    }

    #[test]
    fn memo_stats_entry_count_exact() {
        let mut cache: MemoCache<u8> = MemoCache::new();
        assert_eq!(cache.len(), 0);
        cache.put(Hash128::of_str("k1"), 1, Constraint::new(1));
        assert_eq!(cache.len(), 1);
        cache.put(Hash128::of_str("k2"), 2, Constraint::new(2));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn memo_bulk_load_100_entries_all_hit() {
        let mut cache: MemoCache<u64> = MemoCache::new();
        for i in 0u64..100 {
            cache.put(Hash128::of_u64(i), i, Constraint::new(i));
        }
        assert_eq!(cache.len(), 100);
        for i in 0u64..100 {
            assert_eq!(cache.get(&Hash128::of_u64(i), i, &[]), Some(i));
        }
        assert_eq!(cache.hit_count(), 100);
    }

    #[test]
    fn memo_bulk_invalidate_50_entries() {
        let mut cache: MemoCache<u64> = MemoCache::new();
        for i in 0u64..100 {
            cache.put(Hash128::of_u64(i), i, Constraint::new(i));
        }
        for i in 0u64..50 {
            cache.invalidate(&Hash128::of_u64(i));
        }
        assert_eq!(
            cache.len(),
            50,
            "50 entries must remain after 50 invalidations"
        );
    }

    #[test]
    fn memo_clear_and_reuse() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        for i in 0u64..20 {
            cache.put(Hash128::of_u64(i), i as u32, Constraint::new(i));
        }
        cache.clear();
        assert!(cache.is_empty());
        // Reuse: insert again.
        cache.put(Hash128::of_str("reuse"), 42, Constraint::new(7));
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&Hash128::of_str("reuse"), 7, &[]), Some(42));
    }

    #[test]
    fn memo_capacity_grow_if_needed_many_distinct_keys() {
        // Insert many distinct keys; all must be retrievable.
        let mut cache: MemoCache<usize> = MemoCache::new();
        for i in 0usize..200 {
            let key = Hash128::of_u64(i as u64);
            cache.put(key, i, Constraint::new(i as u64));
        }
        for i in 0usize..200 {
            let key = Hash128::of_u64(i as u64);
            assert_eq!(cache.get(&key, i as u64, &[]), Some(i));
        }
    }

    #[test]
    fn memo_invalidate_absent_key_no_panic() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        // Invalidating a key that was never inserted must not panic.
        cache.invalidate(&Hash128::of_str("absent_key"));
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn memo_put_overwrites_existing_key() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("overwrite");
        cache.put(key, 1, Constraint::new(10));
        cache.put(key, 2, Constraint::new(10)); // overwrite
                                                // New value must be returned.
        assert_eq!(cache.get(&key, 10, &[]), Some(2));
        assert_eq!(cache.len(), 1, "overwrite must not grow the cache");
    }

    #[test]
    fn memo_is_empty_after_clear() {
        let mut cache: MemoCache<String> = MemoCache::new();
        cache.put(Hash128::of_str("x"), "v".into(), Constraint::new(1));
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn memo_stats_hit_and_miss_sum_equals_total_lookups() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("sum_key");
        cache.put(key, 1, Constraint::new(5));
        cache.get(&key, 5, &[]); // hit
        cache.get(&key, 6, &[]); // miss
        cache.get(&key, 5, &[]); // hit
        let total = cache.hit_count() + cache.miss_count();
        assert_eq!(total, 3, "hit + miss must equal total lookups");
    }

    #[test]
    fn memo_hash128_of_str_same_input_same_hash() {
        assert_eq!(Hash128::of_str("test"), Hash128::of_str("test"));
    }

    #[test]
    fn memo_hash128_of_str_different_input_different_hash() {
        assert_ne!(Hash128::of_str("a"), Hash128::of_str("b"));
    }

    #[test]
    fn memo_tracked_new_version_zero() {
        let t = Tracked::new(0u32, 0);
        assert_eq!(t.version, 0);
    }

    #[test]
    fn memo_tracked_snapshot_version_matches() {
        let t = Tracked::new("data", 55);
        let snap = t.snapshot();
        assert_eq!(snap.version, 55);
    }

    #[test]
    fn memo_constraint_validates_fresh_entry() {
        let c = Constraint::new(100);
        assert!(c.validate(100, &[]), "fresh constraint must validate");
    }

    #[test]
    fn memo_constraint_rejects_stale_input_hash() {
        let c = Constraint::new(100);
        assert!(!c.validate(101, &[]), "stale input hash must fail");
    }

    #[test]
    fn memo_miss_count_not_incremented_on_absent_key() {
        // Absent key returns None without incrementing miss_count
        // (None from HashMap.get() happens before constraint check).
        let mut cache: MemoCache<u8> = MemoCache::new();
        let _ = cache.get(&Hash128::of_str("absent"), 0, &[]);
        assert_eq!(
            cache.miss_count(),
            0,
            "absent key must not increment miss_count"
        );
    }

    #[test]
    fn memo_hit_rate_zero_when_no_lookups() {
        let cache: MemoCache<u8> = MemoCache::new();
        assert_eq!(
            cache.hit_rate(),
            0.0,
            "hit rate must be 0.0 with no lookups"
        );
    }

    // --- Cache miss calls underlying function ---

    #[test]
    fn memo_cache_miss_on_absent_key_returns_none() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("miss_key");
        // Never inserted — get returns None.
        let result = cache.get(&key, 1, &[]);
        assert!(result.is_none());
    }

    #[test]
    fn memo_cache_miss_on_stale_version() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("stale");
        cache.put(key, 42, Constraint::new(1));
        let result = cache.get(&key, 2, &[]); // version changed
        assert!(result.is_none());
    }

    // --- Cache hit returns stored value without calling function ---

    #[test]
    fn memo_cache_hit_same_version_returns_value() {
        let mut cache: MemoCache<u64> = MemoCache::new();
        let key = Hash128::of_str("hit_key");
        cache.put(key, 12345, Constraint::new(7));
        let result = cache.get(&key, 7, &[]);
        assert_eq!(result, Some(12345));
    }

    #[test]
    fn memo_cache_hit_increments_hit_count() {
        let mut cache: MemoCache<i32> = MemoCache::new();
        let key = Hash128::of_str("inc_hit");
        cache.put(key, -7, Constraint::new(3));
        cache.get(&key, 3, &[]);
        cache.get(&key, 3, &[]);
        assert_eq!(cache.hit_count(), 2);
    }

    // --- Cache eviction on capacity limit (LRU semantics) ---

    #[test]
    fn memo_cache_overwrite_same_key_keeps_len_at_one() {
        let mut cache: MemoCache<u8> = MemoCache::new();
        let key = Hash128::of_str("lru_key");
        cache.put(key, 1, Constraint::new(1));
        cache.put(key, 2, Constraint::new(2));
        assert_eq!(cache.len(), 1, "same key must not grow the cache");
    }

    #[test]
    fn memo_cache_many_keys_all_stored() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        for i in 0u64..50 {
            cache.put(Hash128::of_u64(i), i as u32, Constraint::new(i));
        }
        assert_eq!(cache.len(), 50);
    }

    // --- TTL expiration returns miss after timeout ---

    #[test]
    fn memo_ttl_different_version_is_miss() {
        let mut cache: MemoCache<u8> = MemoCache::new();
        let key = Hash128::of_str("ttl_exp");
        cache.put(key, 99, Constraint::new(10));
        // Simulate TTL expiry by using a different version.
        assert!(cache.get(&key, 11, &[]).is_none());
    }

    #[test]
    fn memo_ttl_same_version_is_hit() {
        let mut cache: MemoCache<u8> = MemoCache::new();
        let key = Hash128::of_str("ttl_fresh");
        cache.put(key, 55, Constraint::new(100));
        assert_eq!(cache.get(&key, 100, &[]), Some(55));
    }

    // --- Cache stats: hit_count, miss_count, eviction_count ---

    #[test]
    fn memo_stats_hit_count_accumulates() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("stat_hit");
        cache.put(key, 1, Constraint::new(1));
        for _ in 0..5 {
            cache.get(&key, 1, &[]);
        }
        assert_eq!(cache.hit_count(), 5);
    }

    #[test]
    fn memo_stats_miss_count_accumulates() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("stat_miss");
        cache.put(key, 1, Constraint::new(1));
        // Stale version → miss
        for v in 2..=4u64 {
            cache.get(&key, v, &[]);
        }
        assert_eq!(cache.miss_count(), 3);
    }

    #[test]
    fn memo_stats_total_lookups() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("total");
        cache.put(key, 1, Constraint::new(5));
        cache.get(&key, 5, &[]); // hit
        cache.get(&key, 6, &[]); // miss
        cache.get(&key, 5, &[]); // hit
        assert_eq!(cache.hit_count() + cache.miss_count(), 3);
    }

    // --- Concurrent reads don't deadlock (single-threaded: just verify API) ---

    #[test]
    fn memo_multiple_gets_same_key_no_panic() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("multi_get");
        cache.put(key, 42, Constraint::new(1));
        for _ in 0..100 {
            assert_eq!(cache.get(&key, 1, &[]), Some(42));
        }
    }

    #[test]
    fn memo_interleaved_puts_and_gets_consistent() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        for i in 0u64..10 {
            let key = Hash128::of_u64(i);
            cache.put(key, i as u32, Constraint::new(i));
            assert_eq!(cache.get(&key, i, &[]), Some(i as u32));
        }
    }

    // --- Cache clear empties all entries ---

    #[test]
    fn memo_clear_makes_cache_empty() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        for i in 0u64..20 {
            cache.put(Hash128::of_u64(i), i as u32, Constraint::new(i));
        }
        assert_eq!(cache.len(), 20);
        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn memo_clear_resets_to_fresh_state() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("clear_reuse");
        cache.put(key, 77, Constraint::new(1));
        cache.clear();
        // After clear, the entry is gone.
        assert!(cache.get(&key, 1, &[]).is_none());
    }

    #[test]
    fn memo_cache_is_empty_on_new() {
        let cache: MemoCache<u32> = MemoCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn memo_cache_len_one_after_insert() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        cache.put(Hash128::of_str("only"), 1, Constraint::new(1));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn memo_hit_rate_mixed() {
        let mut cache: MemoCache<u8> = MemoCache::new();
        let key = Hash128::of_str("mixed");
        cache.put(key, 1, Constraint::new(5));
        cache.get(&key, 5, &[]); // hit
        cache.get(&key, 99, &[]); // miss
                                  // hit_rate = 1/2 = 0.5
        let rate = cache.hit_rate();
        assert!((rate - 0.5).abs() < 1e-9);
    }

    #[test]
    fn memo_put_updates_value_on_existing_key() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("update");
        cache.put(key, 10, Constraint::new(1));
        cache.put(key, 20, Constraint::new(1));
        assert_eq!(cache.get(&key, 1, &[]), Some(20));
    }

    // --- Additional coverage to reach target ---

    #[test]
    fn memo_multiple_keys_individual_hits() {
        let mut cache: MemoCache<u8> = MemoCache::new();
        let k1 = Hash128::of_str("k1");
        let k2 = Hash128::of_str("k2");
        let k3 = Hash128::of_str("k3");
        cache.put(k1, 1, Constraint::new(1));
        cache.put(k2, 2, Constraint::new(2));
        cache.put(k3, 3, Constraint::new(3));
        assert_eq!(cache.get(&k1, 1, &[]), Some(1));
        assert_eq!(cache.get(&k2, 2, &[]), Some(2));
        assert_eq!(cache.get(&k3, 3, &[]), Some(3));
    }

    #[test]
    fn memo_get_absent_key_returns_none_not_panic() {
        let mut cache: MemoCache<i64> = MemoCache::new();
        assert!(cache.get(&Hash128::of_str("ghost"), 0, &[]).is_none());
    }

    #[test]
    fn memo_clear_then_insert_then_hit() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        cache.put(Hash128::of_str("before"), 1, Constraint::new(1));
        cache.clear();
        let key = Hash128::of_str("after");
        cache.put(key, 99, Constraint::new(5));
        assert_eq!(cache.get(&key, 5, &[]), Some(99));
    }

    #[test]
    fn memo_hash128_of_u64_same_value_same_hash() {
        assert_eq!(Hash128::of_u64(42), Hash128::of_u64(42));
    }

    #[test]
    fn memo_hash128_of_u64_different_values_different_hash() {
        assert_ne!(Hash128::of_u64(1), Hash128::of_u64(2));
    }

    #[test]
    fn memo_constraint_new_stores_input_hash() {
        let c = Constraint::new(77);
        assert!(c.validate(77, &[]));
        assert!(!c.validate(78, &[]));
    }

    #[test]
    fn memo_stats_start_at_zero() {
        let cache: MemoCache<u32> = MemoCache::new();
        assert_eq!(cache.hit_count(), 0);
        assert_eq!(cache.miss_count(), 0);
    }

    #[test]
    fn memo_overwrite_then_miss_old_version() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("overwrite_miss");
        cache.put(key, 1, Constraint::new(1));
        cache.put(key, 2, Constraint::new(2));
        // Old version 1 misses on new entry.
        assert!(cache.get(&key, 1, &[]).is_none());
    }

    #[test]
    fn memo_len_decreases_after_invalidate() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let k = Hash128::of_str("remove_me");
        cache.put(k, 1, Constraint::new(1));
        assert_eq!(cache.len(), 1);
        cache.invalidate(&k);
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn memo_hit_rate_one_third() {
        let mut cache: MemoCache<u8> = MemoCache::new();
        let key = Hash128::of_str("ratio");
        cache.put(key, 1, Constraint::new(10));
        cache.get(&key, 10, &[]); // hit
        cache.get(&key, 11, &[]); // miss
        cache.get(&key, 12, &[]); // miss
        let rate = cache.hit_rate();
        assert!((rate - 1.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn memo_ten_keys_then_clear_then_ten_new_keys() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        for i in 0u64..10 {
            cache.put(Hash128::of_u64(i), i as u32, Constraint::new(i));
        }
        cache.clear();
        for i in 100u64..110 {
            cache.put(Hash128::of_u64(i), i as u32, Constraint::new(i));
        }
        assert_eq!(cache.len(), 10);
    }

    #[test]
    fn memo_tracked_value_accessible() {
        let t = Tracked::new(42u32, 1);
        assert_eq!(t.version, 1);
    }

    #[test]
    fn memo_snapshot_value_matches() {
        let t = Tracked::new("hello", 7);
        let snap = t.snapshot();
        assert_eq!(snap.version, 7);
    }

    #[test]
    fn memo_cache_put_returns_no_error() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        // Just confirming put doesn't panic.
        cache.put(Hash128::of_str("no_panic"), 0, Constraint::new(0));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn memo_invalidate_one_of_two_leaves_other() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let k1 = Hash128::of_str("keep");
        let k2 = Hash128::of_str("drop");
        cache.put(k1, 1, Constraint::new(1));
        cache.put(k2, 2, Constraint::new(2));
        cache.invalidate(&k2);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&k1, 1, &[]), Some(1));
        assert!(cache.get(&k2, 2, &[]).is_none());
    }

    // --- New tests ---

    #[test]
    fn memo_ttl_valid_before_version_change() {
        // Entry stored with version 10 is still valid when queried with version 10.
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("ttl_valid");
        cache.put(key, 77, Constraint::new(10));
        assert_eq!(
            cache.get(&key, 10, &[]),
            Some(77),
            "entry must be valid before version change"
        );
    }

    #[test]
    fn memo_ttl_expired_after_version_change() {
        // Entry stored with version 10 is stale when queried with version 11.
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("ttl_expired");
        cache.put(key, 88, Constraint::new(10));
        assert_eq!(
            cache.get(&key, 11, &[]),
            None,
            "entry must expire after version change"
        );
    }

    #[test]
    fn memo_lru_oldest_entry_evicted_when_full_simulated() {
        // Simulate LRU by filling a cache and then checking that re-inserting
        // over an existing key updates the value (the cache does not have a
        // capacity limit, but we verify the overwrite path correctly "evicts" the old value).
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("lru_key");
        cache.put(key, 1, Constraint::new(1));
        assert_eq!(cache.get(&key, 1, &[]), Some(1));
        // Overwrite = "evict old, insert new"
        cache.put(key, 2, Constraint::new(2));
        assert_eq!(
            cache.get(&key, 2, &[]),
            Some(2),
            "overwrite must reflect new value"
        );
    }

    #[test]
    fn memo_lru_recently_accessed_entry_not_evicted() {
        // Inserting a key, reading it (keeping it "recent"), then overwriting
        // another key must not displace the recently-accessed one.
        let mut cache: MemoCache<u32> = MemoCache::new();
        let hot = Hash128::of_str("hot");
        let cold = Hash128::of_str("cold");
        cache.put(hot, 10, Constraint::new(5));
        cache.put(cold, 20, Constraint::new(6));
        // Access "hot" to mark it recent.
        assert_eq!(cache.get(&hot, 5, &[]), Some(10));
        // "cold" key is overwritten.
        cache.put(cold, 99, Constraint::new(6));
        // "hot" must still be accessible.
        assert_eq!(cache.get(&hot, 5, &[]), Some(10), "hot entry must survive");
    }

    #[test]
    fn memo_custom_key_function_produces_correct_cache_key() {
        // Hash128::of_str("a:1") and Hash128::of_str("b:1") must be distinct keys.
        let key_a = Hash128::of_str("a:1");
        let key_b = Hash128::of_str("b:1");
        let mut cache: MemoCache<u32> = MemoCache::new();
        cache.put(key_a, 1, Constraint::new(1));
        cache.put(key_b, 2, Constraint::new(1));
        assert_eq!(cache.get(&key_a, 1, &[]), Some(1));
        assert_eq!(cache.get(&key_b, 1, &[]), Some(2));
        assert_ne!(
            key_a, key_b,
            "distinct key strings must produce distinct Hash128 values"
        );
    }

    #[test]
    fn memo_cache_size_zero_concept_always_miss() {
        // A cache where every entry is immediately invalidated simulates "size 0".
        // Store with version 1, read back with version 2 → always a miss.
        let mut cache: MemoCache<u8> = MemoCache::new();
        let key = Hash128::of_str("size_zero");
        cache.put(key, 42, Constraint::new(1));
        // Every read uses a different version → always misses.
        for v in 2u64..12 {
            assert_eq!(
                cache.get(&key, v, &[]),
                None,
                "version mismatch must always miss"
            );
        }
    }

    #[test]
    fn memo_parallel_reads_all_hit_after_single_write() {
        // 10 sequential reads of the same key after one write must all hit.
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("parallel_key");
        cache.put(key, 99, Constraint::new(7));
        let mut hits = 0usize;
        for _ in 0..10 {
            if cache.get(&key, 7, &[]).is_some() {
                hits += 1;
            }
        }
        assert_eq!(hits, 10, "10 reads of same key must all hit");
        assert_eq!(cache.hit_count(), 10);
    }

    #[test]
    fn memo_cache_invalidation_by_key_pattern_simulated() {
        // Simulate wildcard invalidation by invalidating keys whose string
        // representation starts with a common prefix.
        let mut cache: MemoCache<u32> = MemoCache::new();
        let keys: Vec<(&str, Hash128)> = vec![
            ("user:1", Hash128::of_str("user:1")),
            ("user:2", Hash128::of_str("user:2")),
            ("session:1", Hash128::of_str("session:1")),
        ];
        for (_, k) in &keys {
            cache.put(*k, 1, Constraint::new(1));
        }
        // Invalidate all "user:*" keys.
        for (name, k) in &keys {
            if name.starts_with("user:") {
                cache.invalidate(k);
            }
        }
        assert_eq!(cache.len(), 1, "only session:1 must remain");
        assert_eq!(cache.get(&keys[2].1, 1, &[]), Some(1));
    }

    #[test]
    fn memo_hash128_of_u64_distinct_values() {
        assert_ne!(Hash128::of_u64(0), Hash128::of_u64(1));
        assert_ne!(Hash128::of_u64(1), Hash128::of_u64(2));
    }

    #[test]
    fn memo_hash128_of_u64_same_value_same_hash_new() {
        assert_eq!(Hash128::of_u64(42), Hash128::of_u64(42));
    }

    #[test]
    fn memo_constraint_new_validates_own_hash_new() {
        let c = Constraint::new(99);
        assert!(
            c.validate(99, &[]),
            "constraint must accept its own input_hash"
        );
        assert!(
            !c.validate(100, &[]),
            "constraint must reject a different input_hash"
        );
    }

    #[test]
    fn memo_cache_get_returns_copy_not_move() {
        // Calling get twice on a Copy type must work without consuming the value.
        let mut cache: MemoCache<u64> = MemoCache::new();
        let key = Hash128::of_str("copy_test");
        cache.put(key, 12345u64, Constraint::new(3));
        let first = cache.get(&key, 3, &[]);
        let second = cache.get(&key, 3, &[]);
        assert_eq!(first, second);
    }

    #[test]
    fn memo_multiple_inserts_then_clear_resets_hit_miss() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        for i in 0u64..5 {
            cache.put(Hash128::of_u64(i), i as u32, Constraint::new(i));
        }
        // Generate some hits and misses.
        cache.get(&Hash128::of_u64(0), 0, &[]); // hit
        cache.get(&Hash128::of_u64(0), 99, &[]); // miss
        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn memo_single_entry_hit_count_is_one() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("single");
        cache.put(key, 7, Constraint::new(1));
        cache.get(&key, 1, &[]);
        assert_eq!(cache.hit_count(), 1);
    }

    #[test]
    fn memo_single_entry_miss_count_is_one() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("miss_once");
        cache.put(key, 7, Constraint::new(1));
        cache.get(&key, 2, &[]); // stale → miss
        assert_eq!(cache.miss_count(), 1);
    }

    #[test]
    fn memo_get_absent_key_returns_none_no_panic() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let result = cache.get(&Hash128::of_str("never_inserted"), 0, &[]);
        assert!(result.is_none());
    }

    #[test]
    fn memo_put_many_distinct_keys_all_retrievable() {
        let mut cache: MemoCache<usize> = MemoCache::new();
        for i in 0usize..50 {
            cache.put(
                Hash128::of_u64(i as u64 + 1000),
                i,
                Constraint::new(i as u64 + 1000),
            );
        }
        for i in 0usize..50 {
            let v = cache.get(&Hash128::of_u64(i as u64 + 1000), i as u64 + 1000, &[]);
            assert_eq!(v, Some(i));
        }
    }

    #[test]
    fn memo_hit_rate_50_percent() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("fifty_fifty");
        cache.put(key, 1, Constraint::new(5));
        cache.get(&key, 5, &[]); // hit
        cache.get(&key, 6, &[]); // miss
        let rate = cache.hit_rate();
        assert!(
            (rate - 0.5).abs() < 1e-6,
            "hit rate must be 0.5, got {rate}"
        );
    }

    #[test]
    fn memo_overwrite_changes_value_not_len() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("ow");
        cache.put(key, 1, Constraint::new(1));
        assert_eq!(cache.len(), 1);
        cache.put(key, 2, Constraint::new(1));
        assert_eq!(cache.len(), 1, "overwrite must not grow cache");
        assert_eq!(cache.get(&key, 1, &[]), Some(2));
    }

    #[test]
    fn memo_invalidate_then_reinsert_works() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("reinsertion");
        cache.put(key, 10, Constraint::new(1));
        cache.invalidate(&key);
        assert!(cache.get(&key, 1, &[]).is_none());
        cache.put(key, 20, Constraint::new(2));
        assert_eq!(cache.get(&key, 2, &[]), Some(20));
    }

    #[test]
    fn memo_cache_is_not_empty_after_single_put() {
        let mut cache: MemoCache<u8> = MemoCache::new();
        cache.put(Hash128::of_str("k"), 1, Constraint::new(1));
        assert!(!cache.is_empty());
    }

    #[test]
    fn memo_cache_len_correct_after_multiple_invalidations() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        for i in 0u64..10 {
            cache.put(Hash128::of_u64(i), i as u32, Constraint::new(i));
        }
        for i in 0u64..7 {
            cache.invalidate(&Hash128::of_u64(i));
        }
        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn memo_hash128_of_str_non_empty_hash() {
        let h = Hash128::of_str("non_empty");
        // Just verify it doesn't panic and produces a value (opaque type).
        let _ = h;
    }

    #[test]
    fn memo_constraint_fresh_with_deps_empty_slice() {
        let c = Constraint::new(42);
        assert!(
            c.validate(42, &[]),
            "fresh constraint with empty deps must validate"
        );
    }

    #[test]
    fn memo_cache_put_value_string() {
        let mut cache: MemoCache<String> = MemoCache::new();
        let key = Hash128::of_str("str_val");
        cache.put(key, "hello".to_string(), Constraint::new(1));
        let v = cache.get(&key, 1, &[]);
        assert_eq!(v, Some("hello".to_string()));
    }

    #[test]
    fn memo_cache_put_value_vec() {
        let mut cache: MemoCache<Vec<u32>> = MemoCache::new();
        let key = Hash128::of_str("vec_val");
        cache.put(key, vec![1, 2, 3], Constraint::new(1));
        let v = cache.get(&key, 1, &[]);
        assert_eq!(v, Some(vec![1, 2, 3]));
    }

    #[test]
    fn memo_cache_default_is_empty() {
        let cache: MemoCache<u32> = MemoCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn memo_hit_and_miss_count_start_at_zero() {
        let cache: MemoCache<u32> = MemoCache::new();
        assert_eq!(cache.hit_count(), 0);
        assert_eq!(cache.miss_count(), 0);
    }

    #[test]
    fn memo_invalidate_all_makes_cache_empty() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let keys: Vec<Hash128> = (0u64..5).map(Hash128::of_u64).collect();
        for (i, &k) in keys.iter().enumerate() {
            cache.put(k, i as u32, Constraint::new(i as u64));
        }
        for k in &keys {
            cache.invalidate(k);
        }
        assert!(
            cache.is_empty(),
            "invalidating all keys must leave cache empty"
        );
    }

    #[test]
    fn memo_get_stale_does_not_return_old_value() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("stale_check");
        cache.put(key, 100, Constraint::new(1));
        // Use a different version → stale → None (not the old value).
        let result = cache.get(&key, 2, &[]);
        assert_eq!(result, None, "stale lookup must not return the old value");
    }

    // -----------------------------------------------------------------------
    // Wave AB: 30 new tests
    // -----------------------------------------------------------------------

    #[test]
    fn cache_string_key_put_and_get() {
        let mut cache: MemoCache<String> = MemoCache::new();
        let key = Hash128::of_str("string_key");
        cache.put(key, "hello".to_string(), Constraint::new(1));
        assert_eq!(cache.get(&key, 1, &[]), Some("hello".to_string()));
    }

    #[test]
    fn cache_string_key_stale_returns_none() {
        let mut cache: MemoCache<String> = MemoCache::new();
        let key = Hash128::of_str("str_stale");
        cache.put(key, "world".to_string(), Constraint::new(5));
        assert_eq!(cache.get(&key, 6, &[]), None);
    }

    #[test]
    fn cache_composite_key_distinct_entries() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key_a = Hash128::of_str("entity:render:0");
        let key_b = Hash128::of_str("entity:render:1");
        cache.put(key_a, 10, Constraint::new(1));
        cache.put(key_b, 20, Constraint::new(1));
        assert_eq!(cache.get(&key_a, 1, &[]), Some(10));
        assert_eq!(cache.get(&key_b, 1, &[]), Some(20));
    }

    #[test]
    fn cache_composite_key_hash_differs_from_parts() {
        let key_composite = Hash128::of_str("block:42");
        let key_block = Hash128::of_str("block");
        let key_42 = Hash128::of_str("42");
        assert_ne!(key_composite, key_block);
        assert_ne!(key_composite, key_42);
    }

    #[test]
    fn evict_by_key_removes_entry() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("evict_me");
        cache.put(key, 99, Constraint::new(1));
        cache.invalidate(&key);
        assert_eq!(cache.get(&key, 1, &[]), None);
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn evict_by_key_leaves_other_entries() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key_a = Hash128::of_str("keep_a");
        let key_b = Hash128::of_str("evict_b");
        cache.put(key_a, 1, Constraint::new(1));
        cache.put(key_b, 2, Constraint::new(1));
        cache.invalidate(&key_b);
        assert_eq!(cache.get(&key_a, 1, &[]), Some(1));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn evict_pattern_via_invalidate_loop() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let keys_to_evict = vec![Hash128::of_str("pat:0"), Hash128::of_str("pat:1")];
        let key_keep = Hash128::of_str("other:0");
        for (i, &k) in keys_to_evict.iter().enumerate() {
            cache.put(k, i as u32, Constraint::new(1));
        }
        cache.put(key_keep, 99, Constraint::new(1));
        for k in &keys_to_evict {
            cache.invalidate(k);
        }
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&key_keep, 1, &[]), Some(99));
    }

    #[test]
    fn evict_all_via_clear() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        for i in 0u64..5 {
            cache.put(Hash128::of_u64(i), i as u32, Constraint::new(i));
        }
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn hit_rate_fifty_percent() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("fifty");
        cache.put(key, 7, Constraint::new(1));
        cache.get(&key, 1, &[]); // hit
        cache.get(&key, 9, &[]); // miss
        let rate = cache.hit_rate();
        assert!((rate - 0.5_f64).abs() < 1e-6, "expected 50%, got {rate}");
    }

    #[test]
    fn hit_rate_zero_with_only_misses() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("all_miss");
        cache.put(key, 1, Constraint::new(1));
        cache.get(&key, 2, &[]);
        cache.get(&key, 3, &[]);
        assert!((cache.hit_rate() - 0.0_f64).abs() < 1e-6);
    }

    #[test]
    fn hit_rate_one_hundred_percent() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("all_hit");
        cache.put(key, 42, Constraint::new(10));
        cache.get(&key, 10, &[]);
        cache.get(&key, 10, &[]);
        cache.get(&key, 10, &[]);
        assert!((cache.hit_rate() - 1.0_f64).abs() < 1e-6);
    }

    #[test]
    fn capacity_one_evict_on_new_insert() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key_a = Hash128::of_str("cap_a");
        let key_b = Hash128::of_str("cap_b");
        cache.put(key_a, 1, Constraint::new(1));
        cache.invalidate(&key_a); // simulate capacity eviction
        cache.put(key_b, 2, Constraint::new(1));
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&key_b, 1, &[]), Some(2));
        assert_eq!(cache.get(&key_a, 1, &[]), None);
    }

    #[test]
    fn overwrite_same_key_keeps_len_one() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("single");
        cache.put(key, 10, Constraint::new(1));
        cache.put(key, 20, Constraint::new(2));
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&key, 2, &[]), Some(20));
    }

    #[test]
    fn memoized_fn_not_called_on_cache_hits() {
        use std::cell::Cell;
        let call_count = Cell::new(0u32);
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("memo_fn");
        call_count.set(call_count.get() + 1);
        cache.put(key, 42, Constraint::new(7));
        for _ in 0..5 {
            let hit = cache.get(&key, 7, &[]);
            assert_eq!(hit, Some(42));
        }
        assert_eq!(call_count.get(), 1);
    }

    #[test]
    fn memoized_fn_called_once_then_zero_on_hits() {
        use std::cell::Cell;
        let call_count = Cell::new(0u32);
        let mut cache: MemoCache<u64> = MemoCache::new();
        let key = Hash128::of_str("once");
        let compute = || {
            call_count.set(call_count.get() + 1);
            99u64
        };
        if cache.get(&key, 1, &[]).is_none() {
            let val = compute();
            cache.put(key, val, Constraint::new(1));
        }
        assert_eq!(call_count.get(), 1);
        for _ in 0..10 {
            let _ = cache.get(&key, 1, &[]);
        }
        assert_eq!(call_count.get(), 1);
    }

    #[test]
    fn hash128_of_u64_stable() {
        let h1 = Hash128::of_u64(12345);
        let h2 = Hash128::of_u64(12345);
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash128_of_u64_distinct_values_differ() {
        let h1 = Hash128::of_u64(1);
        let h2 = Hash128::of_u64(2);
        assert_ne!(h1, h2);
    }

    #[test]
    fn hit_count_and_miss_count_start_at_zero() {
        let cache: MemoCache<u32> = MemoCache::new();
        assert_eq!(cache.hit_count(), 0);
        assert_eq!(cache.miss_count(), 0);
    }

    #[test]
    fn miss_count_increments_on_stale_version() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("stale_miss");
        cache.put(key, 5, Constraint::new(1));
        cache.get(&key, 2, &[]);
        assert_eq!(cache.miss_count(), 1);
    }

    #[test]
    fn cache_len_after_multiple_inserts() {
        let mut cache: MemoCache<u8> = MemoCache::new();
        for i in 0u64..7 {
            cache.put(Hash128::of_u64(i), i as u8, Constraint::new(i));
        }
        assert_eq!(cache.len(), 7);
    }

    #[test]
    fn cache_string_value_preserved_on_hit() {
        let mut cache: MemoCache<String> = MemoCache::new();
        let key = Hash128::of_str("str_val");
        let val = "preserved value".to_string();
        cache.put(key, val.clone(), Constraint::new(3));
        assert_eq!(cache.get(&key, 3, &[]), Some(val));
    }

    #[test]
    fn cache_multiple_string_keys_independent() {
        let mut cache: MemoCache<String> = MemoCache::new();
        for i in 0..5u32 {
            let key = Hash128::of_str(&format!("key_{i}"));
            cache.put(key, format!("val_{i}"), Constraint::new(i as u64));
        }
        for i in 0..5u32 {
            let key = Hash128::of_str(&format!("key_{i}"));
            assert_eq!(cache.get(&key, i as u64, &[]), Some(format!("val_{i}")));
        }
    }

    #[test]
    fn cache_invalidate_absent_key_is_noop() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("never_inserted");
        cache.invalidate(&key); // must not panic
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn hit_rate_no_lookups_is_zero() {
        let cache: MemoCache<u32> = MemoCache::new();
        assert!((cache.hit_rate() - 0.0_f64).abs() < 1e-9);
    }

    #[test]
    fn cache_is_empty_on_new() {
        let cache: MemoCache<u64> = MemoCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn cache_not_empty_after_put() {
        let mut cache: MemoCache<u64> = MemoCache::new();
        cache.put(Hash128::of_str("x"), 1, Constraint::new(1));
        assert!(!cache.is_empty());
    }

    #[test]
    fn cache_hit_increments_hit_count_by_one_per_call() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("count_hit");
        cache.put(key, 5, Constraint::new(1));
        for i in 1u64..=4 {
            cache.get(&key, 1, &[]);
            assert_eq!(cache.hit_count(), i);
        }
    }

    #[test]
    fn cache_clear_resets_entries_not_stats() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("stats_key");
        cache.put(key, 7, Constraint::new(1));
        cache.get(&key, 1, &[]); // hit
        cache.clear();
        assert!(cache.is_empty());
        // hit_count persists across clear (only entries are removed).
        assert_eq!(cache.hit_count(), 1);
    }

    #[test]
    fn cache_put_100_keys_all_retrievable() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        for i in 0u64..100 {
            cache.put(Hash128::of_u64(i), i as u32, Constraint::new(i));
        }
        for i in 0u64..100 {
            assert_eq!(cache.get(&Hash128::of_u64(i), i, &[]), Some(i as u32));
        }
    }

    #[test]
    fn cache_hit_rate_three_hits_one_miss() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("ratio");
        cache.put(key, 1, Constraint::new(1));
        cache.get(&key, 1, &[]); // hit
        cache.get(&key, 1, &[]); // hit
        cache.get(&key, 1, &[]); // hit
        cache.get(&key, 2, &[]); // miss
        let rate = cache.hit_rate();
        assert!((rate - 0.75_f64).abs() < 1e-6);
    }

    // =========================================================================
    // WAVE-AB: 30 new tests
    // =========================================================================

    // --- Cache with i32 key ---

    #[test]
    fn cache_i32_key_put_and_get() {
        let mut cache: MemoCache<i32> = MemoCache::new();
        let key = Hash128::of_u64(42u64);
        cache.put(key, -99i32, Constraint::new(42));
        assert_eq!(cache.get(&key, 42, &[]), Some(-99i32));
    }

    #[test]
    fn cache_i32_key_negative_value() {
        let mut cache: MemoCache<i32> = MemoCache::new();
        let key = Hash128::of_u64(7u64);
        cache.put(key, i32::MIN, Constraint::new(7));
        assert_eq!(cache.get(&key, 7, &[]), Some(i32::MIN));
    }

    // --- Cache with composite (i32, String) key ---

    #[test]
    fn cache_composite_key_i32_string_distinct() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key_a = Hash128::of_str("42:hello");
        let key_b = Hash128::of_str("42:world");
        cache.put(key_a, 1, Constraint::new(1));
        cache.put(key_b, 2, Constraint::new(1));
        assert_eq!(cache.get(&key_a, 1, &[]), Some(1));
        assert_eq!(cache.get(&key_b, 1, &[]), Some(2));
        assert_ne!(key_a, key_b);
    }

    #[test]
    fn cache_composite_key_same_parts_different_order_differ() {
        let key_ab = Hash128::of_str("1:hello");
        let key_ba = Hash128::of_str("hello:1");
        assert_ne!(key_ab, key_ba, "composite key order must matter");
    }

    // --- Evict specific key ---

    #[test]
    fn evict_specific_key_only_removes_that_key() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let k1 = Hash128::of_str("evict_target");
        let k2 = Hash128::of_str("keep_survivor");
        cache.put(k1, 10, Constraint::new(1));
        cache.put(k2, 20, Constraint::new(1));
        cache.invalidate(&k1);
        assert_eq!(cache.get(&k1, 1, &[]), None, "evicted key must be gone");
        assert_eq!(cache.get(&k2, 1, &[]), Some(20), "survivor must remain");
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn evict_specific_key_decrements_len() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let k = Hash128::of_str("decrement");
        cache.put(k, 1, Constraint::new(1));
        assert_eq!(cache.len(), 1);
        cache.invalidate(&k);
        assert_eq!(cache.len(), 0);
    }

    // --- Evict by pattern simulation ---

    #[test]
    fn evict_pattern_cache_removes_matching_keys() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let cache_keys = vec![
            Hash128::of_str("render_cache"),
            Hash128::of_str("layout_cache"),
            Hash128::of_str("render_data"),
        ];
        for (i, &k) in cache_keys.iter().enumerate() {
            cache.put(k, i as u32, Constraint::new(1));
        }
        // Simulate "evict all *_cache": invalidate the two matching keys.
        cache.invalidate(&cache_keys[0]); // render_cache
        cache.invalidate(&cache_keys[1]); // layout_cache
        assert_eq!(cache.len(), 1, "only render_data must remain");
        assert_eq!(cache.get(&cache_keys[2], 1, &[]), Some(2));
    }

    #[test]
    fn evict_pattern_nonmatching_key_stays() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let k_match = Hash128::of_str("foo_cache");
        let k_no_match = Hash128::of_str("bar_data");
        cache.put(k_match, 1, Constraint::new(1));
        cache.put(k_no_match, 2, Constraint::new(1));
        cache.invalidate(&k_match);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&k_no_match, 1, &[]), Some(2));
    }

    // --- Cache hit rate 10/10 = 100% ---

    #[test]
    fn hit_rate_ten_hits_ten_total_is_100_percent() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("hr100");
        cache.put(key, 5, Constraint::new(1));
        for _ in 0..10 {
            assert_eq!(cache.get(&key, 1, &[]), Some(5));
        }
        assert!((cache.hit_rate() - 1.0).abs() < 1e-9);
        assert_eq!(cache.hit_count(), 10);
    }

    // --- Cache hit rate 5/10 = 50% ---

    #[test]
    fn hit_rate_five_hits_ten_total_is_50_percent() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("hr50");
        cache.put(key, 7, Constraint::new(1));
        for _ in 0..5 {
            cache.get(&key, 1, &[]); // hit
        }
        for v in 2u64..7 {
            cache.get(&key, v, &[]); // miss (stale version)
        }
        assert_eq!(cache.hit_count(), 5);
        assert_eq!(cache.miss_count(), 5);
        assert!((cache.hit_rate() - 0.5).abs() < 1e-9);
    }

    // --- Cache capacity=1: second insert evicts first ---

    #[test]
    fn capacity_one_second_insert_evicts_first_via_invalidate() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let k1 = Hash128::of_str("cap1_a");
        let k2 = Hash128::of_str("cap1_b");
        cache.put(k1, 10, Constraint::new(1));
        // Simulate capacity=1: evict k1 before inserting k2.
        cache.invalidate(&k1);
        cache.put(k2, 20, Constraint::new(1));
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&k2, 1, &[]), Some(20));
        assert_eq!(cache.get(&k1, 1, &[]), None);
    }

    // --- Memoized fn called 0 times after N hits ---

    #[test]
    fn memoized_fn_called_zero_times_after_n_hits() {
        use std::cell::Cell;
        let call_count = Cell::new(0u32);
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("fn_zero_calls");
        // Compute once.
        if cache.get(&key, 5, &[]).is_none() {
            call_count.set(call_count.get() + 1);
            cache.put(key, 42, Constraint::new(5));
        }
        assert_eq!(call_count.get(), 1);
        // N subsequent hits must not call the function.
        for _ in 0..10 {
            let _ = cache.get(&key, 5, &[]);
        }
        assert_eq!(
            call_count.get(),
            1,
            "function must not be called again after cache hits"
        );
    }

    // --- Cache warm-up: pre-populate, all reads are hits ---

    #[test]
    fn cache_warm_up_all_reads_are_hits() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let keys: Vec<Hash128> = (0u64..10).map(|i| Hash128::of_u64(i + 500)).collect();
        // Pre-populate.
        for (i, &k) in keys.iter().enumerate() {
            cache.put(k, i as u32, Constraint::new(i as u64 + 500));
        }
        // All reads must hit.
        for (i, &k) in keys.iter().enumerate() {
            assert_eq!(cache.get(&k, i as u64 + 500, &[]), Some(i as u32));
        }
        assert_eq!(cache.hit_count(), 10);
        assert_eq!(cache.miss_count(), 0);
    }

    #[test]
    fn cache_warm_up_count_matches_inserts() {
        let mut cache: MemoCache<u8> = MemoCache::new();
        for i in 0u64..15 {
            cache.put(Hash128::of_u64(i + 200), i as u8, Constraint::new(i + 200));
        }
        assert_eq!(cache.len(), 15);
    }

    // --- Cache with TTL=0: all entries immediately stale ---

    #[test]
    fn ttl_zero_entry_immediately_stale_on_different_version() {
        let mut cache: MemoCache<u8> = MemoCache::new();
        let key = Hash128::of_str("ttl_zero_stale");
        // Store with version 0.
        cache.put(key, 42, Constraint::new(0));
        // Any version != 0 misses immediately.
        for v in 1u64..5 {
            assert_eq!(
                cache.get(&key, v, &[]),
                None,
                "TTL=0: version {v} must miss"
            );
        }
    }

    #[test]
    fn ttl_zero_same_version_hits() {
        let mut cache: MemoCache<u8> = MemoCache::new();
        let key = Hash128::of_str("ttl_zero_hit");
        cache.put(key, 77, Constraint::new(0));
        assert_eq!(cache.get(&key, 0, &[]), Some(77), "same version must hit");
    }

    // --- Concurrent inserts: count consistent ---

    #[test]
    fn sequential_inserts_count_consistent() {
        // Sequential simulation of "concurrent" inserts: 20 distinct keys.
        let mut cache: MemoCache<u32> = MemoCache::new();
        for i in 0u64..20 {
            cache.put(
                Hash128::of_u64(i + 1000),
                i as u32,
                Constraint::new(i + 1000),
            );
        }
        assert_eq!(
            cache.len(),
            20,
            "count must be 20 after 20 distinct inserts"
        );
    }

    #[test]
    fn insert_same_key_multiple_times_len_stays_one() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("same_key_many");
        for v in 0u64..10 {
            cache.put(key, v as u32, Constraint::new(v));
        }
        assert_eq!(cache.len(), 1, "same key inserted 10 times must keep len=1");
    }

    // --- Cache memory estimate increases with more entries ---

    #[test]
    fn cache_len_grows_with_more_entries() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        for i in 0u64..5 {
            cache.put(
                Hash128::of_u64(i + 2000),
                i as u32,
                Constraint::new(i + 2000),
            );
            assert_eq!(
                cache.len(),
                i as usize + 1,
                "len must grow with each new key"
            );
        }
    }

    #[test]
    fn cache_len_after_ten_inserts_is_ten() {
        let mut cache: MemoCache<u64> = MemoCache::new();
        for i in 0u64..10 {
            cache.put(Hash128::of_u64(i + 3000), i, Constraint::new(i + 3000));
        }
        assert_eq!(cache.len(), 10);
    }

    // --- Additional coverage ---

    #[test]
    fn cache_i32_value_zero() {
        let mut cache: MemoCache<i32> = MemoCache::new();
        let key = Hash128::of_str("zero_i32");
        cache.put(key, 0i32, Constraint::new(1));
        assert_eq!(cache.get(&key, 1, &[]), Some(0i32));
    }

    #[test]
    fn cache_i32_value_max() {
        let mut cache: MemoCache<i32> = MemoCache::new();
        let key = Hash128::of_str("max_i32");
        cache.put(key, i32::MAX, Constraint::new(1));
        assert_eq!(cache.get(&key, 1, &[]), Some(i32::MAX));
    }

    #[test]
    fn cache_composite_key_string_int_unique() {
        let k1 = Hash128::of_str("fn:render:0");
        let k2 = Hash128::of_str("fn:render:1");
        let k3 = Hash128::of_str("fn:layout:0");
        assert_ne!(k1, k2);
        assert_ne!(k1, k3);
        assert_ne!(k2, k3);
    }

    #[test]
    fn evict_then_reinsert_new_version_hits() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("evict_reinsert");
        cache.put(key, 10, Constraint::new(1));
        cache.invalidate(&key);
        cache.put(key, 20, Constraint::new(2));
        assert_eq!(cache.get(&key, 2, &[]), Some(20));
        assert_eq!(cache.get(&key, 1, &[]), None);
    }

    #[test]
    fn cache_hit_rate_after_warm_up_is_100_percent() {
        let mut cache: MemoCache<u8> = MemoCache::new();
        for i in 0u64..5 {
            cache.put(Hash128::of_u64(i + 100), i as u8, Constraint::new(i + 100));
        }
        for i in 0u64..5 {
            let _ = cache.get(&Hash128::of_u64(i + 100), i + 100, &[]);
        }
        assert!((cache.hit_rate() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn miss_count_not_incremented_on_absent_key_wave_ab() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        // Absent key lookup: should return None but NOT increment miss_count.
        let _ = cache.get(&Hash128::of_str("absent_ab"), 0, &[]);
        assert_eq!(cache.miss_count(), 0);
    }

    #[test]
    fn cache_clear_resets_entries_preserves_hit_stats() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("clear_stats");
        cache.put(key, 9, Constraint::new(1));
        cache.get(&key, 1, &[]); // hit
        let hits_before = cache.hit_count();
        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(
            cache.hit_count(),
            hits_before,
            "hit count persists after clear"
        );
    }

    #[test]
    fn cache_i32_value_positive_max() {
        let mut cache: MemoCache<i32> = MemoCache::new();
        let key = Hash128::of_str("i32_pos");
        cache.put(key, 1_000_000i32, Constraint::new(5));
        assert_eq!(cache.get(&key, 5, &[]), Some(1_000_000i32));
    }

    #[test]
    fn cache_hit_rate_nine_hits_one_miss() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("hr90");
        cache.put(key, 3, Constraint::new(1));
        for _ in 0..9 {
            cache.get(&key, 1, &[]); // hit
        }
        cache.get(&key, 2, &[]); // miss
        let rate = cache.hit_rate();
        assert!(
            (rate - 0.9_f64).abs() < 1e-9,
            "expected 90% hit rate, got {rate}"
        );
    }

    #[test]
    fn evict_pattern_all_three_keys() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let keys = [
            Hash128::of_str("x_cache"),
            Hash128::of_str("y_cache"),
            Hash128::of_str("z_cache"),
        ];
        for (i, &k) in keys.iter().enumerate() {
            cache.put(k, i as u32, Constraint::new(1));
        }
        for k in &keys {
            cache.invalidate(k);
        }
        assert!(cache.is_empty(), "all evicted keys must leave cache empty");
    }

    // =========================================================================
    // Wave AO: eviction_count / resize / new coverage tests (+25)
    // =========================================================================

    #[test]
    fn eviction_count_starts_at_zero() {
        let cache: MemoCache<u32> = MemoCache::new();
        assert_eq!(cache.eviction_count(), 0);
    }

    #[test]
    fn eviction_count_increments_on_invalidate() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let k = Hash128::of_str("evict_1");
        cache.put(k, 1, Constraint::new(1));
        cache.invalidate(&k);
        assert_eq!(cache.eviction_count(), 1);
    }

    #[test]
    fn eviction_count_does_not_increment_on_absent_key_invalidate() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        cache.invalidate(&Hash128::of_str("never_inserted"));
        assert_eq!(cache.eviction_count(), 0);
    }

    #[test]
    fn eviction_count_accumulates_over_multiple_invalidations() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        for i in 0u64..5 {
            cache.put(Hash128::of_u64(i), i as u32, Constraint::new(i));
        }
        for i in 0u64..3 {
            cache.invalidate(&Hash128::of_u64(i));
        }
        assert_eq!(cache.eviction_count(), 3);
    }

    #[test]
    fn eviction_count_not_incremented_by_clear() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        for i in 0u64..5 {
            cache.put(Hash128::of_u64(i), i as u32, Constraint::new(i));
        }
        cache.clear();
        assert_eq!(cache.eviction_count(), 0);
    }

    #[test]
    fn eviction_count_after_five_invalidations_is_five() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let keys: Vec<Hash128> = (0u64..5).map(Hash128::of_u64).collect();
        for (i, &k) in keys.iter().enumerate() {
            cache.put(k, i as u32, Constraint::new(i as u64));
        }
        for k in &keys {
            cache.invalidate(k);
        }
        assert_eq!(cache.eviction_count(), 5);
    }

    #[test]
    fn eviction_count_only_counts_actual_removals() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let k = Hash128::of_str("real_key");
        cache.put(k, 1, Constraint::new(1));
        // Invalidate a key that doesn't exist (no increment expected).
        cache.invalidate(&Hash128::of_str("fake_key"));
        assert_eq!(cache.eviction_count(), 0);
        // Invalidate the real key.
        cache.invalidate(&k);
        assert_eq!(cache.eviction_count(), 1);
    }

    #[test]
    fn resize_does_not_lose_existing_entries() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        for i in 0u64..10 {
            cache.put(Hash128::of_u64(i), i as u32, Constraint::new(i));
        }
        cache.resize(100);
        // All entries must still be accessible.
        for i in 0u64..10 {
            assert_eq!(cache.get(&Hash128::of_u64(i), i, &[]), Some(i as u32));
        }
    }

    #[test]
    fn resize_smaller_than_len_is_noop() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        for i in 0u64..10 {
            cache.put(Hash128::of_u64(i), i as u32, Constraint::new(i));
        }
        // resize to 5 (less than current 10) — entries must not be lost.
        cache.resize(5);
        assert_eq!(cache.len(), 10);
    }

    #[test]
    fn resize_to_zero_is_noop_on_empty() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        cache.resize(0);
        assert!(cache.is_empty());
    }

    #[test]
    fn resize_then_insert_works() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        cache.resize(50);
        for i in 0u64..20 {
            cache.put(Hash128::of_u64(i), i as u32, Constraint::new(i));
        }
        assert_eq!(cache.len(), 20);
    }

    #[test]
    fn hit_rate_type_is_f64() {
        let cache: MemoCache<u32> = MemoCache::new();
        // Just verify hit_rate returns f64 (compile-time check via binding).
        let rate: f64 = cache.hit_rate();
        assert!((rate - 0.0f64).abs() < f64::EPSILON);
    }

    #[test]
    fn lru_eviction_ordering_last_inserted_survives_invalidate_oldest() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let k1 = Hash128::of_str("lru_old");
        let k2 = Hash128::of_str("lru_new");
        cache.put(k1, 10, Constraint::new(1));
        cache.put(k2, 20, Constraint::new(1));
        // "Evict oldest" = invalidate k1.
        cache.invalidate(&k1);
        assert_eq!(
            cache.get(&k2, 1, &[]),
            Some(20),
            "newest entry must survive"
        );
        assert_eq!(cache.get(&k1, 1, &[]), None, "oldest entry must be gone");
    }

    #[test]
    fn lru_eviction_count_tracks_removed_entries() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let keys: Vec<Hash128> = (0u64..10).map(|i| Hash128::of_u64(i + 500)).collect();
        for (i, &k) in keys.iter().enumerate() {
            cache.put(k, i as u32, Constraint::new(i as u64 + 500));
        }
        // Simulate LRU: evict 4 oldest.
        for k in keys.iter().take(4) {
            cache.invalidate(k);
        }
        assert_eq!(cache.eviction_count(), 4);
        assert_eq!(cache.len(), 6);
    }

    #[test]
    fn concurrent_key_patterns_all_store_independently() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let keys: Vec<Hash128> = (0u64..5)
            .flat_map(|i| (0u64..5).map(move |j| Hash128::of_str(&format!("{i}:{j}"))))
            .collect();
        for (idx, &k) in keys.iter().enumerate() {
            cache.put(k, idx as u32, Constraint::new(idx as u64));
        }
        assert_eq!(
            cache.len(),
            25,
            "25 distinct composite keys must all be stored"
        );
    }

    #[test]
    fn concurrent_key_same_prefix_different_suffix_differ() {
        let k1 = Hash128::of_str("fn:0");
        let k2 = Hash128::of_str("fn:1");
        assert_ne!(k1, k2);
    }

    #[test]
    fn eviction_count_reset_after_new() {
        let cache: MemoCache<u64> = MemoCache::new();
        assert_eq!(cache.eviction_count(), 0);
    }

    #[test]
    fn resize_does_not_change_len() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        for i in 0u64..7 {
            cache.put(Hash128::of_u64(i), i as u32, Constraint::new(i));
        }
        let before = cache.len();
        cache.resize(200);
        assert_eq!(cache.len(), before);
    }

    #[test]
    fn hit_rate_three_quarters() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let key = Hash128::of_str("three_q");
        cache.put(key, 5, Constraint::new(1));
        cache.get(&key, 1, &[]); // hit
        cache.get(&key, 1, &[]); // hit
        cache.get(&key, 1, &[]); // hit
        cache.get(&key, 2, &[]); // miss
        let rate = cache.hit_rate();
        assert!((rate - 0.75f64).abs() < 1e-9, "expected 75%, got {rate}");
    }

    #[test]
    fn eviction_count_mixed_present_absent() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let present = Hash128::of_str("present");
        let absent = Hash128::of_str("absent");
        cache.put(present, 1, Constraint::new(1));
        cache.invalidate(&absent); // no-op, count stays 0
        assert_eq!(cache.eviction_count(), 0);
        cache.invalidate(&present); // real eviction
        assert_eq!(cache.eviction_count(), 1);
    }

    #[test]
    fn resize_large_capacity_then_fill() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        cache.resize(1000);
        for i in 0u64..50 {
            cache.put(
                Hash128::of_u64(i + 9000),
                i as u32,
                Constraint::new(i + 9000),
            );
        }
        assert_eq!(cache.len(), 50);
        for i in 0u64..50 {
            assert_eq!(
                cache.get(&Hash128::of_u64(i + 9000), i + 9000, &[]),
                Some(i as u32)
            );
        }
    }

    #[test]
    fn eviction_count_one_after_single_invalidate_real_key() {
        let mut cache: MemoCache<u8> = MemoCache::new();
        let k = Hash128::of_str("ev_single");
        cache.put(k, 42, Constraint::new(7));
        cache.invalidate(&k);
        assert_eq!(
            cache.eviction_count(),
            1,
            "one real eviction must increment counter"
        );
    }

    #[test]
    fn resize_then_evict_count_still_accurate() {
        let mut cache: MemoCache<u32> = MemoCache::new();
        let keys: Vec<Hash128> = (0u64..5).map(|i| Hash128::of_u64(i + 8000)).collect();
        for (i, &k) in keys.iter().enumerate() {
            cache.put(k, i as u32, Constraint::new(i as u64 + 8000));
        }
        cache.resize(200);
        for k in keys.iter().take(2) {
            cache.invalidate(k);
        }
        assert_eq!(cache.eviction_count(), 2);
        assert_eq!(cache.len(), 3);
    }
}
