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
        assert_eq!(cache.len(), 50, "50 entries must remain after 50 invalidations");
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
    fn memo_hit_rate_all_hits() {
        let mut cache: MemoCache<u8> = MemoCache::new();
        let key = Hash128::of_str("h");
        cache.put(key, 1, Constraint::new(5));
        cache.get(&key, 5, &[]);
        cache.get(&key, 5, &[]);
        assert!((cache.hit_rate() - 1.0).abs() < 1e-9, "100% hit rate");
    }

    #[test]
    fn memo_hit_rate_all_misses() {
        let mut cache: MemoCache<u8> = MemoCache::new();
        let key = Hash128::of_str("m");
        cache.put(key, 1, Constraint::new(5));
        cache.get(&key, 99, &[]); // stale
        cache.get(&key, 88, &[]); // stale
        assert!((cache.hit_rate() - 0.0).abs() < 1e-9, "0% hit rate");
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
        assert_eq!(cache.miss_count(), 0, "absent key must not increment miss_count");
    }

    #[test]
    fn memo_hit_rate_zero_when_no_lookups() {
        let cache: MemoCache<u8> = MemoCache::new();
        assert_eq!(cache.hit_rate(), 0.0, "hit rate must be 0.0 with no lookups");
    }
}
