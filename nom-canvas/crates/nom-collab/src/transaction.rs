//! CRDT transaction log — ordered list of update blobs per document.

use crate::doc_id::DocId;

/// A single CRDT update issued by one client.
pub struct Transaction {
    pub doc_id: DocId,
    pub client_id: u64,
    pub timestamp_ms: u64,
    pub update_v2: Vec<u8>,
}

/// Append-only log of transactions across all documents.
pub struct TransactionLog {
    entries: Vec<Transaction>,
}

impl TransactionLog {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Append a transaction to the log.
    pub fn append(&mut self, tx: Transaction) {
        self.entries.push(tx);
    }

    /// Return all transactions for `doc_id` with `timestamp_ms <= until_ms`,
    /// in insertion order.
    pub fn replay_until<'a>(
        &'a self,
        doc_id: &DocId,
        until_ms: u64,
    ) -> Vec<&'a Transaction> {
        self.entries
            .iter()
            .filter(|t| &t.doc_id == doc_id && t.timestamp_ms <= until_ms)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tx(doc: &str, client: u64, ts: u64) -> Transaction {
        Transaction {
            doc_id: DocId::from(doc),
            client_id: client,
            timestamp_ms: ts,
            update_v2: vec![ts as u8],
        }
    }

    #[test]
    fn empty_log_replays_nothing() {
        let log = TransactionLog::new();
        assert!(log.replay_until(&DocId::from("d"), 999).is_empty());
    }

    #[test]
    fn replay_filters_by_doc_id() {
        let mut log = TransactionLog::new();
        log.append(tx("doc-a", 1, 10));
        log.append(tx("doc-b", 2, 10));
        let result = log.replay_until(&DocId::from("doc-a"), 100);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].client_id, 1);
    }

    #[test]
    fn replay_filters_by_timestamp() {
        let mut log = TransactionLog::new();
        log.append(tx("doc", 1, 5));
        log.append(tx("doc", 2, 15));
        log.append(tx("doc", 3, 25));
        let result = log.replay_until(&DocId::from("doc"), 15);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn replay_includes_exact_timestamp_boundary() {
        let mut log = TransactionLog::new();
        log.append(tx("doc", 1, 100));
        let result = log.replay_until(&DocId::from("doc"), 100);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn append_preserves_order() {
        let mut log = TransactionLog::new();
        for i in 0..5u64 {
            log.append(tx("doc", i, i * 10));
        }
        let result = log.replay_until(&DocId::from("doc"), 1000);
        assert_eq!(result.len(), 5);
        for (i, t) in result.iter().enumerate() {
            assert_eq!(t.client_id, i as u64);
        }
    }
}
