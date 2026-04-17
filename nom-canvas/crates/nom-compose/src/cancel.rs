#![deny(unsafe_code)]

use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

/// A shareable flag that signals cancellation / interrupt to a long-running compose operation.
#[derive(Clone, Debug)]
pub struct InterruptFlag {
    inner: Arc<AtomicBool>,
}

impl InterruptFlag {
    /// Create a new flag in the "not set" state.
    pub fn new() -> Self {
        Self { inner: Arc::new(AtomicBool::new(false)) }
    }

    /// Signal cancellation. Idempotent — safe to call multiple times.
    pub fn set(&self) {
        self.inner.store(true, Ordering::SeqCst);
    }

    /// Reset the flag to the "not set" state.
    pub fn clear(&self) {
        self.inner.store(false, Ordering::SeqCst);
    }

    /// Returns `true` if the flag has been set.
    pub fn is_set(&self) -> bool {
        self.inner.load(Ordering::SeqCst)
    }
}

impl Default for InterruptFlag {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrupt_flag_not_set_by_default() {
        let flag = InterruptFlag::new();
        assert!(!flag.is_set());
    }

    #[test]
    fn interrupt_flag_set() {
        let flag = InterruptFlag::new();
        flag.set();
        assert!(flag.is_set());
    }

    #[test]
    fn interrupt_flag_idempotent() {
        let flag = InterruptFlag::new();
        flag.set();
        flag.set();
        assert!(flag.is_set());
    }

    #[test]
    fn interrupt_flag_arc_shared() {
        let flag = InterruptFlag::new();
        let clone = flag.clone();
        clone.set();
        // original observes the change because both share the same Arc<AtomicBool>
        assert!(flag.is_set());
    }

    #[test]
    fn interrupt_flag_clear() {
        let flag = InterruptFlag::new();
        flag.set();
        assert!(flag.is_set());
        flag.clear();
        assert!(!flag.is_set());
    }
}
