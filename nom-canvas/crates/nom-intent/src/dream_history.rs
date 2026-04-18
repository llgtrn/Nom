/// One completed dream iteration: score, iteration index, timestamp, and epic flag.
#[derive(Debug, Clone)]
pub struct DreamHistoryEntry {
    pub iteration: u32,
    pub score: f32,
    pub timestamp_ms: u64,
    pub reached_epic: bool,
}

impl DreamHistoryEntry {
    pub fn new(iteration: u32, score: f32, timestamp_ms: u64) -> Self {
        let reached_epic = score >= 95.0;
        Self {
            iteration,
            score,
            timestamp_ms,
            reached_epic,
        }
    }

    pub fn is_epic(&self) -> bool {
        self.reached_epic
    }
}

/// Persists dream history entries ordered by iteration.
#[derive(Debug, Clone, Default)]
pub struct DreamHistoryStore {
    entries: Vec<DreamHistoryEntry>,
}

impl DreamHistoryStore {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn record(&mut self, entry: DreamHistoryEntry) {
        self.entries.push(entry);
    }

    pub fn latest(&self) -> Option<&DreamHistoryEntry> {
        self.entries.last()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the highest score across all entries, or 0.0 if empty.
    pub fn best_score(&self) -> f32 {
        self.entries
            .iter()
            .map(|e| e.score)
            .fold(0.0_f32, f32::max)
    }

    /// Returns the iteration number of the first entry that reached epic (score >= 95.0).
    pub fn first_epic_iteration(&self) -> Option<u32> {
        self.entries.iter().find(|e| e.is_epic()).map(|e| e.iteration)
    }
}

/// Wraps a DreamHistoryStore and provides per-session summary statistics.
#[derive(Debug, Clone)]
pub struct DreamJournal {
    pub store: DreamHistoryStore,
    pub session_id: String,
}

impl DreamJournal {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            store: DreamHistoryStore::new(),
            session_id: session_id.into(),
        }
    }

    pub fn record_iteration(&mut self, iteration: u32, score: f32, timestamp_ms: u64) {
        self.store.record(DreamHistoryEntry::new(iteration, score, timestamp_ms));
    }

    /// Returns a human-readable summary: "session={} iterations={} best_score={:.1} epic={}".
    pub fn summary(&self) -> String {
        let epic = self.has_reached_epic();
        format!(
            "session={} iterations={} best_score={:.1} epic={}",
            self.session_id,
            self.store.len(),
            self.store.best_score(),
            epic,
        )
    }

    /// Returns true if any recorded entry has reached epic score.
    pub fn has_reached_epic(&self) -> bool {
        self.store.first_epic_iteration().is_some()
    }
}

#[cfg(test)]
mod dream_history_tests {
    use super::*;

    #[test]
    fn dream_history_entry_is_epic_true() {
        let entry = DreamHistoryEntry::new(1, 95.0, 1000);
        assert!(entry.is_epic());
    }

    #[test]
    fn dream_history_entry_is_epic_false() {
        let entry = DreamHistoryEntry::new(1, 94.9, 1000);
        assert!(!entry.is_epic());
    }

    #[test]
    fn dream_history_store_record_and_len() {
        let mut store = DreamHistoryStore::new();
        assert_eq!(store.len(), 0);
        store.record(DreamHistoryEntry::new(0, 50.0, 0));
        store.record(DreamHistoryEntry::new(1, 70.0, 1));
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn dream_history_store_latest_after_records() {
        let mut store = DreamHistoryStore::new();
        assert!(store.latest().is_none());
        store.record(DreamHistoryEntry::new(0, 40.0, 100));
        store.record(DreamHistoryEntry::new(1, 80.0, 200));
        let latest = store.latest().unwrap();
        assert_eq!(latest.iteration, 1);
        assert!((latest.score - 80.0).abs() < f32::EPSILON);
    }

    #[test]
    fn dream_history_store_best_score() {
        let mut store = DreamHistoryStore::new();
        assert!((store.best_score() - 0.0).abs() < f32::EPSILON);
        store.record(DreamHistoryEntry::new(0, 60.0, 0));
        store.record(DreamHistoryEntry::new(1, 88.5, 1));
        store.record(DreamHistoryEntry::new(2, 72.0, 2));
        assert!((store.best_score() - 88.5).abs() < f32::EPSILON);
    }

    #[test]
    fn dream_history_store_first_epic_iteration() {
        let mut store = DreamHistoryStore::new();
        assert!(store.first_epic_iteration().is_none());
        store.record(DreamHistoryEntry::new(0, 80.0, 0));
        store.record(DreamHistoryEntry::new(1, 96.0, 1));
        store.record(DreamHistoryEntry::new(2, 98.0, 2));
        assert_eq!(store.first_epic_iteration(), Some(1));
    }

    #[test]
    fn dream_journal_record_and_summary() {
        let mut journal = DreamJournal::new("sess-abc");
        journal.record_iteration(0, 70.0, 100);
        journal.record_iteration(1, 85.0, 200);
        let s = journal.summary();
        assert!(s.contains("session=sess-abc"));
        assert!(s.contains("iterations=2"));
        assert!(s.contains("best_score=85.0"));
        assert!(s.contains("epic=false"));
    }

    #[test]
    fn dream_journal_has_reached_epic_false() {
        let mut journal = DreamJournal::new("sess-x");
        journal.record_iteration(0, 90.0, 0);
        journal.record_iteration(1, 94.9, 1);
        assert!(!journal.has_reached_epic());
    }

    #[test]
    fn dream_journal_has_reached_epic_true() {
        let mut journal = DreamJournal::new("sess-y");
        journal.record_iteration(0, 90.0, 0);
        journal.record_iteration(1, 95.0, 1);
        assert!(journal.has_reached_epic());
    }
}
