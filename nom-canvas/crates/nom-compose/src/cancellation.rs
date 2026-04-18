#![deny(unsafe_code)]

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// A read-only handle that queries whether a cancellation has been requested.
pub struct CancelSignal(Arc<AtomicBool>);

impl CancelSignal {
    /// Returns `true` if the associated cancel function has been called.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// Create a paired (`CancelSignal`, cancel function).
///
/// Calling the returned `Box<dyn Fn()>` sets the flag; `CancelSignal::is_cancelled`
/// will then return `true` on any clone that shares the same allocation.
pub fn make_cancel_signal() -> (CancelSignal, Box<dyn Fn() + Send>) {
    let flag = Arc::new(AtomicBool::new(false));
    let flag2 = Arc::clone(&flag);
    let cancel = Box::new(move || flag2.store(true, Ordering::Relaxed));
    (CancelSignal(flag), cancel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cancel_signal_initially_not_cancelled() {
        let (signal, _cancel) = make_cancel_signal();
        assert!(!signal.is_cancelled(), "fresh signal must not be cancelled");
    }

    #[test]
    fn test_cancel_signal_after_cancel() {
        let (signal, cancel) = make_cancel_signal();
        cancel();
        assert!(signal.is_cancelled(), "signal must be cancelled after cancel()");
    }

    #[test]
    fn test_cancel_signal_clone_shares_state() {
        // Two CancelSignals created from the same make_cancel_signal share the Arc.
        let flag = Arc::new(AtomicBool::new(false));
        let flag2 = Arc::clone(&flag);
        let cancel: Box<dyn Fn() + Send> = Box::new(move || flag2.store(true, Ordering::Relaxed));
        let s1 = CancelSignal(Arc::clone(&flag));
        let s2 = CancelSignal(flag);
        assert!(!s1.is_cancelled());
        cancel();
        assert!(s1.is_cancelled(), "s1 must see cancellation");
        assert!(s2.is_cancelled(), "s2 must see cancellation via shared Arc");
    }
}
