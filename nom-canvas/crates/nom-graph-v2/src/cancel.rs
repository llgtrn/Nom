use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct InterruptFlag(pub Arc<AtomicBool>);

impl InterruptFlag {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    pub fn trigger(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_triggered(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    pub fn reset(&self) {
        self.0.store(false, Ordering::Relaxed);
    }
}

/// Stub external blocker — holds a pending flag; no async integration yet.
pub struct ExternalBlocker {
    pending: AtomicBool,
}

impl ExternalBlocker {
    pub fn new() -> Self {
        Self {
            pending: AtomicBool::new(false),
        }
    }

    pub fn block(&self) {
        self.pending.store(true, Ordering::Relaxed);
    }

    pub fn unblock(&self) {
        self.pending.store(false, Ordering::Relaxed);
    }

    pub fn is_pending(&self) -> bool {
        self.pending.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrupt_default_false() {
        let f = InterruptFlag::new();
        assert!(!f.is_triggered());
    }

    #[test]
    fn trigger_sets_flag() {
        let f = InterruptFlag::new();
        f.trigger();
        assert!(f.is_triggered());
    }

    #[test]
    fn reset_clears_flag() {
        let f = InterruptFlag::new();
        f.trigger();
        f.reset();
        assert!(!f.is_triggered());
    }

    #[test]
    fn clone_shares_state() {
        let f = InterruptFlag::new();
        let f2 = f.clone();
        f.trigger();
        assert!(f2.is_triggered());
    }

    #[test]
    fn external_blocker_pending_state() {
        let b = ExternalBlocker::new();
        assert!(!b.is_pending());
        b.block();
        assert!(b.is_pending());
        b.unblock();
        assert!(!b.is_pending());
    }
}
