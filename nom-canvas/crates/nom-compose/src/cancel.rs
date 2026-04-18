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

    #[test]
    fn cancel_token_initially_not_cancelled() {
        let token = InterruptFlag::new();
        assert!(!token.is_set(), "fresh token must report not cancelled");
    }

    #[test]
    fn cancel_sets_cancelled_flag() {
        let token = InterruptFlag::new();
        token.set();
        assert!(token.is_set(), "after cancel, flag must be set");
    }

    #[test]
    fn cancelled_check_is_thread_safe() {
        use std::sync::Arc;
        use std::thread;
        let token = Arc::new(InterruptFlag::new());
        let t = token.clone();
        let handle = thread::spawn(move || {
            t.set();
        });
        handle.join().unwrap();
        assert!(
            token.is_set(),
            "flag set from another thread must be visible"
        );
    }

    #[test]
    fn cancel_clear_after_set_from_thread() {
        use std::sync::Arc;
        use std::thread;
        let token = Arc::new(InterruptFlag::new());
        let t = token.clone();
        let h = thread::spawn(move || {
            t.set();
        });
        h.join().unwrap();
        assert!(token.is_set());
        token.clear();
        assert!(!token.is_set(), "clear after thread-set must work");
    }

    #[test]
    fn multiple_threads_all_see_cancelled_state() {
        use std::sync::Arc;
        use std::thread;
        let token = Arc::new(InterruptFlag::new());
        token.set();
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let t = token.clone();
                thread::spawn(move || t.is_set())
            })
            .collect();
        for h in handles {
            assert!(
                h.join().unwrap(),
                "all threads must see the cancelled state"
            );
        }
    }

    #[test]
    fn interrupt_flag_new_creates_unique_flags() {
        let a = InterruptFlag::new();
        let b = InterruptFlag::new();
        a.set();
        // b is independent — must not be affected.
        assert!(
            !b.is_set(),
            "separate InterruptFlag instances must be independent"
        );
    }

    #[test]
    fn interrupt_flag_multiple_clears_are_idempotent() {
        let flag = InterruptFlag::new();
        flag.clear();
        flag.clear();
        flag.clear();
        assert!(!flag.is_set(), "multiple clears must leave flag unset");
    }

    #[test]
    fn cancel_token_toggle_multiple_times() {
        let flag = InterruptFlag::new();
        for i in 0..10 {
            if i % 2 == 0 {
                flag.set();
            } else {
                flag.clear();
            }
        }
        // After 10 toggles (0-9), last was odd so clear was last.
        assert!(
            !flag.is_set(),
            "after even number of toggles ending on clear, must be unset"
        );
    }

    #[test]
    fn two_independent_tokens_do_not_share_state() {
        let a = InterruptFlag::new();
        let b = InterruptFlag::new();
        a.set();
        assert!(a.is_set());
        assert!(!b.is_set(), "b must be independent from a");
        b.set();
        assert!(b.is_set());
        a.clear();
        assert!(!a.is_set());
        assert!(b.is_set(), "clearing a must not affect b");
    }

    #[test]
    fn interrupt_flag_thread_set_main_reads() {
        use std::sync::Arc;
        use std::thread;
        let flag = Arc::new(InterruptFlag::new());
        let f = flag.clone();
        thread::spawn(move || f.set()).join().unwrap();
        assert!(flag.is_set(), "flag set in thread must be visible in main");
    }
}
