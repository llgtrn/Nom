//! Document snapshot — placeholder for yrs Doc::encode_state_as_update_v2.

use crate::doc_id::DocId;

/// A point-in-time snapshot of a collaborative document.
///
/// `state_vector` encodes which operations the document has seen (used for
/// differential sync).  `update_v2` is the full document state encoded as a
/// CRDT update blob (placeholder until the yrs crate is wired in).
pub struct DocSnapshot {
    pub doc_id: DocId,
    pub state_vector: Vec<u8>,
    pub update_v2: Vec<u8>,
    pub timestamp_ms: u64,
}

impl DocSnapshot {
    /// Create a new snapshot.  `timestamp_ms` is set to 0 for callers that
    /// do not yet have a clock source; update via field access when needed.
    pub fn new(doc_id: DocId, state_vector: Vec<u8>, update_v2: Vec<u8>) -> Self {
        Self { doc_id, state_vector, update_v2, timestamp_ms: 0 }
    }

    /// Total bytes of the encoded payloads (state_vector + update_v2).
    pub fn size_bytes(&self) -> usize {
        self.state_vector.len() + self.update_v2.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sets_fields() {
        let snap = DocSnapshot::new(
            DocId::from("doc-a"),
            vec![1, 2, 3],
            vec![4, 5],
        );
        assert_eq!(snap.doc_id, DocId::from("doc-a"));
        assert_eq!(snap.state_vector, vec![1, 2, 3]);
        assert_eq!(snap.update_v2, vec![4, 5]);
        assert_eq!(snap.timestamp_ms, 0);
    }

    #[test]
    fn size_bytes_sums_both_vecs() {
        let snap = DocSnapshot::new(DocId::from("x"), vec![0u8; 10], vec![0u8; 20]);
        assert_eq!(snap.size_bytes(), 30);
    }

    #[test]
    fn empty_snapshot_is_zero_bytes() {
        let snap = DocSnapshot::new(DocId::from("empty"), vec![], vec![]);
        assert_eq!(snap.size_bytes(), 0);
    }
}
