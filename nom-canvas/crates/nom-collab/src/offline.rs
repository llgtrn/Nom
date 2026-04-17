//! Offline queue — buffers transactions for documents while the server is unreachable.

use crate::doc_id::DocId;
use crate::transaction::Transaction;

pub struct OfflineQueue {
    pending: Vec<Transaction>,
}

impl OfflineQueue {
    pub fn new() -> Self {
        Self { pending: Vec::new() }
    }

    /// Enqueue a transaction for later replay.
    pub fn enqueue(&mut self, tx: Transaction) {
        self.pending.push(tx);
    }

    /// Drain all transactions for `doc_id`, returning them in insertion order.
    /// The drained entries are removed from the queue.
    pub fn drain_for(&mut self, doc_id: &DocId) -> Vec<Transaction> {
        let mut drained = Vec::new();
        let mut remaining = Vec::new();
        for tx in self.pending.drain(..) {
            if &tx.doc_id == doc_id {
                drained.push(tx);
            } else {
                remaining.push(tx);
            }
        }
        self.pending = remaining;
        drained
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tx(doc: &str, client: u64) -> Transaction {
        Transaction {
            doc_id: DocId::from(doc),
            client_id: client,
            timestamp_ms: 0,
            update_v2: vec![],
        }
    }

    #[test]
    fn new_queue_is_empty() {
        let q = OfflineQueue::new();
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn enqueue_and_drain() {
        let mut q = OfflineQueue::new();
        q.enqueue(tx("doc-a", 1));
        q.enqueue(tx("doc-b", 2));
        let drained = q.drain_for(&DocId::from("doc-a"));
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].client_id, 1);
        assert_eq!(q.len(), 1); // doc-b remains
    }

    #[test]
    fn drain_returns_empty_for_unknown_doc() {
        let mut q = OfflineQueue::new();
        q.enqueue(tx("doc-x", 1));
        let drained = q.drain_for(&DocId::from("doc-z"));
        assert!(drained.is_empty());
        assert_eq!(q.len(), 1);
    }
}
