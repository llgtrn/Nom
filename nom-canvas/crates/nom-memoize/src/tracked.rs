#![deny(unsafe_code)]
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Typst comemo-pattern: Tracked<T> wraps a value and tracks access for validation.
/// Every read through the tracker is recorded so the constraint can verify
/// that cached computation results are still valid.
pub struct Tracked<T> {
    inner: Arc<T>,
    /// Access counter — increments on each read (used for constraint validation)
    access_count: Arc<AtomicU64>,
    /// Version of the value when this Tracked<T> was created
    pub version: u64,
}

impl<T: Clone> Tracked<T> {
    pub fn new(value: T, version: u64) -> Self {
        Self {
            inner: Arc::new(value),
            access_count: Arc::new(AtomicU64::new(0)),
            version,
        }
    }

    /// Access the tracked value. Records one access for constraint validation.
    pub fn get(&self) -> &T {
        self.access_count.fetch_add(1, Ordering::Relaxed);
        &self.inner
    }

    pub fn access_count(&self) -> u64 {
        self.access_count.load(Ordering::Relaxed)
    }

    /// Create a constraint snapshot capturing current access count
    pub fn snapshot(&self) -> TrackedSnapshot {
        TrackedSnapshot {
            version: self.version,
            access_count_at_snapshot: self.access_count.load(Ordering::Relaxed),
        }
    }
}

impl<T: Clone + Send + Sync> Clone for Tracked<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            access_count: Arc::new(AtomicU64::new(0)),
            version: self.version,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TrackedSnapshot {
    pub version: u64,
    pub access_count_at_snapshot: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracked_access_counting() {
        let t = Tracked::new(vec![1, 2, 3], 1);
        assert_eq!(t.access_count(), 0);
        let _ = t.get();
        let _ = t.get();
        assert_eq!(t.access_count(), 2);
    }

    #[test]
    fn tracked_snapshot() {
        let t = Tracked::new("hello", 42);
        let _ = t.get();
        let snap = t.snapshot();
        assert_eq!(snap.version, 42);
        assert_eq!(snap.access_count_at_snapshot, 1);
    }
}
