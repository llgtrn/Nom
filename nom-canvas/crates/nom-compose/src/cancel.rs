#![deny(unsafe_code)]

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// A shareable flag that signals cancellation / interrupt to a long-running compose operation.
#[derive(Clone, Debug)]
pub struct InterruptFlag {
    inner: Arc<AtomicBool>,
}

impl InterruptFlag {
    /// Create a new flag in the "not set" state.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AtomicBool::new(false)),
        }
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
    fn default() -> Self {
        Self::new()
    }
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

    #[test]
    fn interrupt_flag_default_not_set() {
        let flag = InterruptFlag::default();
        assert!(!flag.is_set(), "default() must produce an unset flag");
    }

    #[test]
    fn interrupt_flag_clone_shares_state() {
        // Two clones must see the same underlying AtomicBool.
        let a = InterruptFlag::new();
        let b = a.clone();
        let c = b.clone();
        assert!(!c.is_set());
        a.set();
        assert!(b.is_set(), "b must see the set via a");
        assert!(c.is_set(), "c must see the set via a");
        c.clear();
        assert!(!a.is_set(), "a must see the clear via c");
        assert!(!b.is_set(), "b must see the clear via c");
    }

    #[test]
    fn interrupt_flag_set_idempotent_across_clones() {
        let flag = InterruptFlag::new();
        let clone1 = flag.clone();
        clone1.set();
        flag.set(); // second set — must be idempotent
        assert!(flag.is_set());
        assert!(clone1.is_set());
    }

    #[test]
    fn interrupt_flag_clear_when_not_set_is_noop() {
        let flag = InterruptFlag::new();
        flag.clear(); // clear on an already-clear flag
        assert!(!flag.is_set());
    }

    #[test]
    fn interrupt_flag_set_clear_set_cycle() {
        let flag = InterruptFlag::new();
        flag.set();
        flag.clear();
        flag.set();
        assert!(flag.is_set(), "after set→clear→set the flag must be set");
    }
}
