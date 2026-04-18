#![deny(unsafe_code)]
use crate::hash::Hash128;
use std::sync::Arc;

/// A single recorded method invocation: which method was called and what it returned.
/// Used by constraints to validate that cached results are still correct.
pub struct MethodCall {
    pub method_id: u32,
    pub return_hash: Hash128,
}

/// Typst comemo-pattern: Tracked<T> wraps a value and records per-method call hashes
/// so the constraint can validate "did the methods you called return the same values?".
pub struct Tracked<T> {
    inner: Arc<T>,
    /// Version of the value when this Tracked<T> was created
    pub version: u64,
    method_calls: Arc<std::sync::Mutex<Vec<MethodCall>>>,
}

impl<T: Clone> Tracked<T> {
    pub fn new(value: T, version: u64) -> Self {
        Self {
            inner: Arc::new(value),
            version,
            method_calls: Arc::new(std::sync::Mutex::new(vec![])),
        }
    }

    /// Access the tracked value.
    pub fn get(&self) -> &T {
        &self.inner
    }

    /// Record a method call with its return hash for constraint validation.
    pub fn record_call(&self, method_id: u32, return_hash: Hash128) {
        if let Ok(mut calls) = self.method_calls.lock() {
            calls.push(MethodCall {
                method_id,
                return_hash,
            });
        }
    }

    /// Drain and return all recorded method calls (resets the list).
    pub fn take_calls(&self) -> Vec<MethodCall> {
        self.method_calls
            .lock()
            .map(|mut v| std::mem::take(&mut *v))
            .unwrap_or_default()
    }

    /// Number of method calls recorded since last take_calls.
    pub fn call_count(&self) -> usize {
        self.method_calls.lock().map(|v| v.len()).unwrap_or(0)
    }

    /// Create a constraint snapshot capturing current call state
    pub fn snapshot(&self) -> TrackedSnapshot {
        let calls = self
            .method_calls
            .lock()
            .map(|v| v.iter().map(|c| (c.method_id, c.return_hash)).collect())
            .unwrap_or_default();
        TrackedSnapshot {
            version: self.version,
            method_call_pairs: calls,
        }
    }
}

impl<T: Clone + Send + Sync> Clone for Tracked<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            method_calls: Arc::new(std::sync::Mutex::new(vec![])),
            version: self.version,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TrackedSnapshot {
    pub version: u64,
    /// (method_id, return_hash) pairs recorded during the computation
    pub method_call_pairs: Vec<(u32, Hash128)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracked_get() {
        let t = Tracked::new(vec![1, 2, 3], 1);
        assert_eq!(t.get(), &vec![1, 2, 3]);
    }

    #[test]
    fn tracked_record_and_take_calls() {
        let t = Tracked::new("hello", 42);
        assert_eq!(t.call_count(), 0);
        t.record_call(1, Hash128::of_str("result_a"));
        t.record_call(2, Hash128::of_str("result_b"));
        assert_eq!(t.call_count(), 2);
        let calls = t.take_calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].method_id, 1);
        assert_eq!(calls[1].method_id, 2);
        // After take, list is drained
        assert_eq!(t.call_count(), 0);
    }

    #[test]
    fn tracked_snapshot_captures_pairs() {
        let t = Tracked::new("hello", 42);
        let h = Hash128::of_str("value");
        t.record_call(7, h);
        let snap = t.snapshot();
        assert_eq!(snap.version, 42);
        assert_eq!(snap.method_call_pairs.len(), 1);
        assert_eq!(snap.method_call_pairs[0].0, 7);
        assert_eq!(snap.method_call_pairs[0].1, h);
    }

    #[test]
    fn tracked_clone_starts_fresh() {
        let t = Tracked::new("hello", 1);
        t.record_call(1, Hash128::of_str("x"));
        let t2 = t.clone();
        assert_eq!(t2.call_count(), 0); // clone starts with empty call list
    }

    #[test]
    fn tracked_version_exposed() {
        let t = Tracked::new(42u32, 99);
        assert_eq!(t.version, 99);
    }

    #[test]
    fn tracked_inner_accessible() {
        let t = Tracked::new("hello", 1);
        assert_eq!(*t.get(), "hello");
    }

    #[test]
    fn tracked_call_count_starts_zero() {
        let t = Tracked::new(0u32, 0);
        assert_eq!(t.call_count(), 0);
    }

    #[test]
    fn tracked_take_calls_drains() {
        let t = Tracked::new("x", 5);
        t.record_call(1, Hash128::of_str("r1"));
        t.record_call(2, Hash128::of_str("r2"));
        t.record_call(3, Hash128::of_str("r3"));
        assert_eq!(t.call_count(), 3);
        let drained = t.take_calls();
        assert_eq!(drained.len(), 3);
        assert_eq!(t.call_count(), 0);
    }

    #[test]
    fn tracked_snapshot_empty_pairs() {
        let t = Tracked::new("fresh", 10);
        let snap = t.snapshot();
        assert!(snap.method_call_pairs.is_empty());
    }

    #[test]
    fn tracked_snapshot_version_matches() {
        let t = Tracked::new("data", 77);
        t.record_call(9, Hash128::of_u64(1));
        let snap = t.snapshot();
        assert_eq!(snap.version, 77);
    }

    #[test]
    fn tracked_clone_does_not_share_calls() {
        let t = Tracked::new("original", 3);
        t.record_call(1, Hash128::of_str("v"));
        let t2 = t.clone();
        // original still has its call
        assert_eq!(t.call_count(), 1);
        // clone starts fresh
        assert_eq!(t2.call_count(), 0);
    }

    #[test]
    fn method_call_fields() {
        let mc = MethodCall {
            method_id: 42,
            return_hash: Hash128::of_str("out"),
        };
        assert_eq!(mc.method_id, 42);
        assert_eq!(mc.return_hash, Hash128::of_str("out"));
    }

    #[test]
    fn tracked_multiple_record_calls() {
        let t = Tracked::new("data", 1);
        for i in 0..10u32 {
            t.record_call(i, Hash128::of_u64(i as u64));
        }
        assert_eq!(t.call_count(), 10);
    }

    #[test]
    fn tracked_snapshot_after_take_is_empty() {
        let t = Tracked::new("data", 1);
        t.record_call(1, Hash128::of_str("r"));
        t.record_call(2, Hash128::of_str("s"));
        t.take_calls();
        let snap = t.snapshot();
        assert_eq!(snap.method_call_pairs.len(), 0);
    }

    #[test]
    fn tracked_record_same_method_twice() {
        let t = Tracked::new("data", 1);
        let h = Hash128::of_str("result");
        t.record_call(5, h);
        t.record_call(5, h);
        assert_eq!(t.call_count(), 2);
        let calls = t.take_calls();
        assert_eq!(calls[0].method_id, 5);
        assert_eq!(calls[1].method_id, 5);
    }

    #[test]
    fn tracked_version_zero() {
        let t = Tracked::new("value", 0);
        assert_eq!(t.version, 0);
        assert_eq!(t.call_count(), 0);
    }

    // ── additional coverage ────────────────────────────────────────────────

    #[test]
    fn tracked_change_detection_via_version() {
        // Simulated "dirty" detection: a higher version number signals a change.
        let t_old = Tracked::new(42u32, 1);
        let t_new = Tracked::new(42u32, 2);
        assert!(t_new.version > t_old.version, "version bump signals change");
    }

    #[test]
    fn tracked_mark_dirty_by_bumping_version() {
        // Conventional pattern: re-wrap with version+1 to mark dirty.
        let val = 99u64;
        let t = Tracked::new(val, 5);
        let t_dirty = Tracked::new(*t.get(), t.version + 1);
        assert_eq!(t_dirty.version, 6);
        assert_eq!(*t_dirty.get(), val);
    }

    #[test]
    fn tracked_clear_dirty_by_taking_calls() {
        let t = Tracked::new("data", 1);
        t.record_call(1, Hash128::of_str("r1"));
        t.record_call(2, Hash128::of_str("r2"));
        // take_calls() acts as "clear dirty"
        let cleared = t.take_calls();
        assert_eq!(cleared.len(), 2);
        assert_eq!(t.call_count(), 0, "calls cleared after take");
    }

    #[test]
    fn tracked_nested_tracked_values() {
        // Outer Tracked wraps inner Tracked; both track independently.
        let inner = Tracked::new(vec![1, 2, 3], 1);
        let outer = Tracked::new(inner, 2);
        assert_eq!(outer.version, 2);
        assert_eq!(outer.get().version, 1);
        // Record on outer doesn't affect inner
        outer.record_call(10, Hash128::of_str("outer_result"));
        assert_eq!(outer.call_count(), 1);
        assert_eq!(outer.get().call_count(), 0);
    }

    #[test]
    fn tracked_snapshot_pairs_match_recorded_calls() {
        let t = Tracked::new("snap_test", 3);
        let h1 = Hash128::of_str("out1");
        let h2 = Hash128::of_str("out2");
        t.record_call(10, h1);
        t.record_call(20, h2);
        let snap = t.snapshot();
        assert_eq!(snap.method_call_pairs.len(), 2);
        assert_eq!(snap.method_call_pairs[0], (10, h1));
        assert_eq!(snap.method_call_pairs[1], (20, h2));
    }

    #[test]
    fn tracked_snapshot_not_consumed() {
        // snapshot() does NOT drain calls; take_calls() does.
        let t = Tracked::new("x", 1);
        t.record_call(1, Hash128::of_str("r"));
        let _snap = t.snapshot();
        assert_eq!(t.call_count(), 1, "snapshot must not drain calls");
    }

    #[test]
    fn tracked_get_returns_same_reference_value() {
        let t = Tracked::new(String::from("hello"), 1);
        let a = t.get().clone();
        let b = t.get().clone();
        assert_eq!(a, b);
    }

    #[test]
    fn tracked_concurrent_record_calls() {
        use std::sync::Arc;
        use std::thread;
        let t = Arc::new(Tracked::new(0u64, 1));
        let mut handles = vec![];
        for i in 0u32..8 {
            let tc = Arc::clone(&t);
            handles.push(thread::spawn(move || {
                tc.record_call(i, Hash128::of_u64(i as u64));
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(t.call_count(), 8);
    }

    #[test]
    fn tracked_dirty_cycle_mark_check_reset() {
        // Simulate a mark-dirty → check-dirty → reset cycle using version bumps and take_calls.
        let t = Tracked::new("state", 1);

        // mark dirty: record a call
        t.record_call(1, Hash128::of_str("dirty_value"));
        // check dirty: call_count > 0 means dirty
        assert!(t.call_count() > 0, "should be dirty after recording");

        // reset: drain calls
        let _drained = t.take_calls();
        assert_eq!(t.call_count(), 0, "should be clean after reset");
    }

    #[test]
    fn tracked_repeated_dirty_reset_cycles() {
        let t = Tracked::new(0u32, 1);
        for cycle in 0u32..5 {
            // mark dirty
            t.record_call(cycle, Hash128::of_u64(cycle as u64));
            assert_eq!(t.call_count(), 1);
            // reset
            let drained = t.take_calls();
            assert_eq!(drained.len(), 1);
            assert_eq!(t.call_count(), 0);
        }
    }

    #[test]
    fn tracked_check_dirty_via_snapshot() {
        // A fresh Tracked has an empty snapshot — "not dirty".
        let t = Tracked::new("clean", 1);
        let snap = t.snapshot();
        assert!(snap.method_call_pairs.is_empty(), "fresh tracked is not dirty");

        // After recording, snapshot is non-empty — "dirty".
        t.record_call(99, Hash128::of_str("result"));
        let snap2 = t.snapshot();
        assert!(!snap2.method_call_pairs.is_empty(), "tracked is dirty after record");
    }

    #[test]
    fn tracked_reset_via_new_instance() {
        // Conventional reset: create a new Tracked with version+1 (signals change to consumers).
        let t_old = Tracked::new("old_state", 3);
        t_old.record_call(1, Hash128::of_str("r"));

        let t_new = Tracked::new(*t_old.get(), t_old.version + 1);
        assert_eq!(t_new.call_count(), 0, "new instance starts clean");
        assert_eq!(t_new.version, 4, "version bumped");
    }

    #[test]
    fn tracked_take_then_snapshot_empty() {
        let t = Tracked::new("data", 10);
        t.record_call(1, Hash128::of_str("r1"));
        t.record_call(2, Hash128::of_str("r2"));
        let _ = t.take_calls(); // reset
        let snap = t.snapshot();
        assert!(snap.method_call_pairs.is_empty());
    }

    #[test]
    fn tracked_call_count_increments_one_by_one() {
        let t = Tracked::new(0u8, 0);
        for i in 0u32..5 {
            assert_eq!(t.call_count(), i as usize);
            t.record_call(i, Hash128::of_u64(i as u64));
        }
        assert_eq!(t.call_count(), 5);
    }

    // --- call_count() never decrements after reset via take_calls() ---

    #[test]
    fn call_count_never_decrements_after_reset() {
        // After take_calls() the count goes to zero (reset), but it never goes
        // BELOW the zero it was reset to — subsequent records only add.
        let t = Tracked::new("data", 1);
        t.record_call(1, Hash128::of_str("a"));
        t.record_call(2, Hash128::of_str("b"));
        assert_eq!(t.call_count(), 2);

        // Reset via take_calls.
        let _ = t.take_calls();
        assert_eq!(t.call_count(), 0, "count must be zero after reset");

        // Now record again — count should increment from 0, never go negative.
        t.record_call(3, Hash128::of_str("c"));
        assert_eq!(t.call_count(), 1, "count must increment after reset, never decrement");

        t.record_call(4, Hash128::of_str("d"));
        assert_eq!(t.call_count(), 2);

        // Second reset.
        let _ = t.take_calls();
        assert_eq!(t.call_count(), 0);

        // Always non-negative.
        for i in 0u32..5 {
            let before = t.call_count();
            t.record_call(i, Hash128::of_u64(i as u64));
            let after = t.call_count();
            assert!(after > before, "call_count must strictly increase after record_call");
        }
    }

    #[test]
    fn call_count_stays_zero_without_records_after_reset() {
        let t = Tracked::new("x", 1);
        t.record_call(1, Hash128::of_str("v"));
        let _ = t.take_calls();
        // No new records — count stays at zero, never below.
        assert_eq!(t.call_count(), 0);
        assert_eq!(t.call_count(), 0); // second call also zero, not decremented
    }

    #[test]
    fn call_count_multiple_resets_never_negative() {
        let t = Tracked::new(0u32, 0);
        for _ in 0u32..10 {
            t.record_call(1, Hash128::of_str("x"));
            assert!(t.call_count() >= 1);
            let _ = t.take_calls();
            // After reset, count is 0 (non-negative floor).
            assert_eq!(t.call_count(), 0);
        }
    }

    // --- Additional tracked coverage ---

    #[test]
    fn tracked_large_version_number() {
        let t = Tracked::new("big", u64::MAX);
        assert_eq!(t.version, u64::MAX);
    }

    #[test]
    fn tracked_record_100_calls_count_is_100() {
        let t = Tracked::new("stress", 1);
        for i in 0u32..100 {
            t.record_call(i, Hash128::of_u64(i as u64));
        }
        assert_eq!(t.call_count(), 100);
    }

    #[test]
    fn tracked_take_all_100_calls() {
        let t = Tracked::new("stress", 1);
        for i in 0u32..100 {
            t.record_call(i, Hash128::of_u64(i as u64));
        }
        let calls = t.take_calls();
        assert_eq!(calls.len(), 100);
        assert_eq!(t.call_count(), 0);
    }

    #[test]
    fn tracked_snapshot_after_100_calls_has_100_pairs() {
        let t = Tracked::new("many", 1);
        for i in 0u32..100 {
            t.record_call(i, Hash128::of_u64(i as u64));
        }
        let snap = t.snapshot();
        assert_eq!(snap.method_call_pairs.len(), 100);
        assert_eq!(snap.version, 1);
    }
}
