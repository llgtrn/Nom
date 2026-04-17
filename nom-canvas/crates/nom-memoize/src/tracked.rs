#![deny(unsafe_code)]
use std::sync::Arc;
use crate::hash::Hash128;

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
            calls.push(MethodCall { method_id, return_hash });
        }
    }

    /// Drain and return all recorded method calls (resets the list).
    pub fn take_calls(&self) -> Vec<MethodCall> {
        self.method_calls.lock().map(|mut v| std::mem::take(&mut *v)).unwrap_or_default()
    }

    /// Number of method calls recorded since last take_calls.
    pub fn call_count(&self) -> usize {
        self.method_calls.lock().map(|v| v.len()).unwrap_or(0)
    }

    /// Create a constraint snapshot capturing current call state
    pub fn snapshot(&self) -> TrackedSnapshot {
        let calls = self.method_calls.lock()
            .map(|v| v.iter().map(|c| (c.method_id, c.return_hash)).collect())
            .unwrap_or_default();
        TrackedSnapshot { version: self.version, method_call_pairs: calls }
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
}
