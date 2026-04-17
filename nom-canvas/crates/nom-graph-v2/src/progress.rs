use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use crate::node_schema::NodeId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressEvent {
    pub node: NodeId,
    pub value: u64,
    pub max: u64,
}

pub trait ProgressHandler {
    fn update(&self, node: NodeId, value: u64, max: u64);
    fn check_interrupted(&self) -> bool;
}

pub struct ChannelProgress {
    pub sender: mpsc::Sender<ProgressEvent>,
    pub interrupt: Arc<AtomicBool>,
}

impl ChannelProgress {
    pub fn new(sender: mpsc::Sender<ProgressEvent>, interrupt: Arc<AtomicBool>) -> Self {
        Self { sender, interrupt }
    }
}

impl ProgressHandler for ChannelProgress {
    fn update(&self, node: NodeId, value: u64, max: u64) {
        let _ = self.sender.send(ProgressEvent { node, value, max });
    }

    fn check_interrupted(&self) -> bool {
        self.interrupt.load(Ordering::Relaxed)
    }
}

/// No-op progress handler for tests that don't need progress events.
pub struct NoProgress;

impl ProgressHandler for NoProgress {
    fn update(&self, _node: NodeId, _value: u64, _max: u64) {}
    fn check_interrupted(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_sends_event() {
        let (tx, rx) = mpsc::channel();
        let flag = Arc::new(AtomicBool::new(false));
        let p = ChannelProgress::new(tx, flag);
        p.update(42, 3, 10);
        let ev = rx.try_recv().unwrap();
        assert_eq!(ev, ProgressEvent { node: 42, value: 3, max: 10 });
    }

    #[test]
    fn interrupt_flag_false_by_default() {
        let (tx, _rx) = mpsc::channel();
        let flag = Arc::new(AtomicBool::new(false));
        let p = ChannelProgress::new(tx, flag);
        assert!(!p.check_interrupted());
    }

    #[test]
    fn interrupt_flag_reads_true() {
        let (tx, _rx) = mpsc::channel();
        let flag = Arc::new(AtomicBool::new(false));
        let p = ChannelProgress::new(tx, Arc::clone(&flag));
        flag.store(true, Ordering::Relaxed);
        assert!(p.check_interrupted());
    }

    #[test]
    fn no_progress_never_interrupted() {
        let p = NoProgress;
        assert!(!p.check_interrupted());
    }
}
