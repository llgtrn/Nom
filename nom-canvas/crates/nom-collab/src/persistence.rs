//! Persistence backend trait + in-memory implementation.

use crate::doc_id::DocId;
use crate::snapshot::DocSnapshot;
use parking_lot::Mutex;
use std::collections::HashMap;

/// Backend for loading and saving document snapshots.
pub trait PersistenceBackend: Send + Sync {
    fn load(&self, doc_id: &DocId) -> Option<DocSnapshot>;
    fn save(&self, snapshot: &DocSnapshot);
    fn list(&self) -> Vec<DocId>;
}

// ---- in-memory implementation ----------------------------------------------

pub struct InMemoryPersistence {
    snapshots: Mutex<HashMap<String, StoredSnapshot>>,
}

/// Internal storable version (DocSnapshot is not Clone, so we store parts).
struct StoredSnapshot {
    doc_id: DocId,
    state_vector: Vec<u8>,
    update_v2: Vec<u8>,
    timestamp_ms: u64,
}

impl InMemoryPersistence {
    pub fn new() -> Self {
        Self { snapshots: Mutex::new(HashMap::new()) }
    }
}

impl PersistenceBackend for InMemoryPersistence {
    fn load(&self, doc_id: &DocId) -> Option<DocSnapshot> {
        let guard = self.snapshots.lock();
        guard.get(&doc_id.0).map(|s| DocSnapshot {
            doc_id: s.doc_id.clone(),
            state_vector: s.state_vector.clone(),
            update_v2: s.update_v2.clone(),
            timestamp_ms: s.timestamp_ms,
        })
    }

    fn save(&self, snapshot: &DocSnapshot) {
        let mut guard = self.snapshots.lock();
        guard.insert(snapshot.doc_id.0.clone(), StoredSnapshot {
            doc_id: snapshot.doc_id.clone(),
            state_vector: snapshot.state_vector.clone(),
            update_v2: snapshot.update_v2.clone(),
            timestamp_ms: snapshot.timestamp_ms,
        });
    }

    fn list(&self) -> Vec<DocId> {
        let guard = self.snapshots.lock();
        guard.values().map(|s| s.doc_id.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(id: &str, sv: Vec<u8>, upd: Vec<u8>) -> DocSnapshot {
        DocSnapshot::new(DocId::from(id), sv, upd)
    }

    #[test]
    fn load_missing_returns_none() {
        let p = InMemoryPersistence::new();
        assert!(p.load(&DocId::from("missing")).is_none());
    }

    #[test]
    fn save_then_load_roundtrip() {
        let p = InMemoryPersistence::new();
        p.save(&snap("doc-1", vec![1], vec![2, 3]));
        let loaded = p.load(&DocId::from("doc-1")).unwrap();
        assert_eq!(loaded.state_vector, vec![1]);
        assert_eq!(loaded.update_v2, vec![2, 3]);
    }

    #[test]
    fn save_overwrites_previous() {
        let p = InMemoryPersistence::new();
        p.save(&snap("doc-1", vec![1], vec![1]));
        p.save(&snap("doc-1", vec![9], vec![9]));
        let loaded = p.load(&DocId::from("doc-1")).unwrap();
        assert_eq!(loaded.state_vector, vec![9]);
    }

    #[test]
    fn list_returns_all_saved_ids() {
        let p = InMemoryPersistence::new();
        p.save(&snap("a", vec![], vec![]));
        p.save(&snap("b", vec![], vec![]));
        let mut ids: Vec<String> = p.list().into_iter().map(|d| d.0).collect();
        ids.sort();
        assert_eq!(ids, vec!["a", "b"]);
    }
}
