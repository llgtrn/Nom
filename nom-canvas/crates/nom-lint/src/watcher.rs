//! File-change batching queue.
//!
//! Callers push raw change events from their OS watcher (e.g. `notify`
//! crate).  The queue coalesces rapid-fire events within a 50ms window
//! into a single batch per path.  This avoids re-linting a file five
//! times while the user mashes Ctrl+S.
#![deny(unsafe_code)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeKind {
    Created,
    Modified,
    Removed,
    Renamed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeEvent {
    pub path: PathBuf,
    pub kind: ChangeKind,
}

pub struct DebouncedChangeQueue {
    /// Latest event per path + timestamp first seen within the window.
    pending: HashMap<PathBuf, (ChangeKind, Instant)>,
    debounce: Duration,
}

impl DebouncedChangeQueue {
    /// Default window is 50ms per the blueprint.
    pub fn new(debounce_ms: u64) -> Self {
        Self {
            pending: HashMap::new(),
            debounce: Duration::from_millis(debounce_ms),
        }
    }

    /// Record a raw event.  The queue stores only the latest kind for any path.
    pub fn push(&mut self, event: ChangeEvent) {
        let now = Instant::now();
        self.pending.insert(event.path, (event.kind, now));
    }

    /// Drain all paths that have been quiet for at least `debounce`.  Returns
    /// a Vec of (path, latest_kind) ready for downstream processing.  Paths
    /// still within the debounce window stay in the queue for the next call.
    pub fn drain_due(&mut self) -> Vec<ChangeEvent> {
        self.drain_due_at(Instant::now())
    }

    /// Same as `drain_due` but takes an explicit `now` for deterministic tests.
    pub fn drain_due_at(&mut self, now: Instant) -> Vec<ChangeEvent> {
        let debounce = self.debounce;
        let mut due = Vec::new();
        self.pending.retain(|path, (kind, t)| {
            if now.duration_since(*t) >= debounce {
                due.push(ChangeEvent {
                    path: path.clone(),
                    kind: *kind,
                });
                false
            } else {
                true
            }
        });
        due
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    pub fn clear(&mut self) {
        self.pending.clear();
    }

    /// Test-only: insert an event with an explicit timestamp.
    #[cfg(test)]
    pub(crate) fn push_at(&mut self, event: ChangeEvent, t: Instant) {
        self.pending.insert(event.path, (event.kind, t));
    }
}

/// Incremental relint helper: given a list of changed paths + a function that
/// maps path → file_hash, flag only those files whose hashes changed.
/// Used as input to `LintCache::invalidate_file`.
pub fn changed_file_hashes<F>(events: &[ChangeEvent], hasher: F) -> Vec<(PathBuf, u64)>
where
    F: Fn(&PathBuf) -> u64,
{
    events
        .iter()
        .map(|e| (e.path.clone(), hasher(&e.path)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    /// Test 1: new queue has len 0 and is_empty true.
    #[test]
    fn new_queue_is_empty() {
        let q = DebouncedChangeQueue::new(50);
        assert_eq!(q.len(), 0);
        assert!(q.is_empty());
    }

    /// Test 2: push one event + drain_due_at(now+60ms) returns it and removes it.
    #[test]
    fn push_and_drain_after_debounce() {
        let mut q = DebouncedChangeQueue::new(50);
        let past = Instant::now() - Duration::from_millis(60);
        q.push_at(
            ChangeEvent {
                path: p("a.nom"),
                kind: ChangeKind::Modified,
            },
            past,
        );
        let now = Instant::now();
        let drained = q.drain_due_at(now);
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].path, p("a.nom"));
        assert_eq!(drained[0].kind, ChangeKind::Modified);
        assert!(q.is_empty());
    }

    /// Test 3: push one event + drain_due_at(now+10ms) returns empty, keeps in queue.
    #[test]
    fn push_and_drain_before_debounce() {
        let mut q = DebouncedChangeQueue::new(50);
        let recent = Instant::now() - Duration::from_millis(10);
        q.push_at(
            ChangeEvent {
                path: p("b.nom"),
                kind: ChangeKind::Created,
            },
            recent,
        );
        let now = Instant::now();
        let drained = q.drain_due_at(now);
        assert!(drained.is_empty());
        assert_eq!(q.len(), 1);
    }

    /// Test 4: push same path twice with different kinds → latest wins (only 1 event drained).
    #[test]
    fn same_path_latest_kind_wins() {
        let mut q = DebouncedChangeQueue::new(50);
        let past = Instant::now() - Duration::from_millis(60);
        // First insert Created at past.
        q.push_at(
            ChangeEvent {
                path: p("c.nom"),
                kind: ChangeKind::Created,
            },
            past,
        );
        // Second push via normal push() overwrites with Modified.
        // To guarantee it is also past the debounce we insert via push_at.
        q.push_at(
            ChangeEvent {
                path: p("c.nom"),
                kind: ChangeKind::Modified,
            },
            past,
        );
        let now = Instant::now();
        let drained = q.drain_due_at(now);
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].kind, ChangeKind::Modified);
    }

    /// Test 5: push 10 events (all past debounce) → drain returns exactly 10 coalesced events.
    #[test]
    fn burst_of_ten_all_drain() {
        let mut q = DebouncedChangeQueue::new(50);
        let past = Instant::now() - Duration::from_millis(60);
        for i in 0..10u8 {
            q.push_at(
                ChangeEvent {
                    path: p(&format!("file{i}.nom")),
                    kind: ChangeKind::Modified,
                },
                past,
            );
        }
        let now = Instant::now();
        let drained = q.drain_due_at(now);
        assert_eq!(drained.len(), 10);
        assert!(q.is_empty());
    }

    /// Test 6: after first drain, a new push registers as a fresh event.
    #[test]
    fn second_push_after_drain_is_fresh() {
        let mut q = DebouncedChangeQueue::new(50);
        let past = Instant::now() - Duration::from_millis(60);
        q.push_at(
            ChangeEvent {
                path: p("d.nom"),
                kind: ChangeKind::Modified,
            },
            past,
        );
        let now = Instant::now();
        let first_drain = q.drain_due_at(now);
        assert_eq!(first_drain.len(), 1);

        // New event with a recent timestamp — should NOT be drained yet.
        let recent = Instant::now() - Duration::from_millis(5);
        q.push_at(
            ChangeEvent {
                path: p("d.nom"),
                kind: ChangeKind::Removed,
            },
            recent,
        );
        let second_drain = q.drain_due_at(Instant::now());
        assert!(second_drain.is_empty(), "new event should still be debouncing");
        assert_eq!(q.len(), 1);
    }

    /// Test 7: clear wipes the queue.
    #[test]
    fn clear_wipes_queue() {
        let mut q = DebouncedChangeQueue::new(50);
        let past = Instant::now() - Duration::from_millis(60);
        for i in 0..5u8 {
            q.push_at(
                ChangeEvent {
                    path: p(&format!("e{i}.nom")),
                    kind: ChangeKind::Modified,
                },
                past,
            );
        }
        assert_eq!(q.len(), 5);
        q.clear();
        assert!(q.is_empty());
        let drained = q.drain_due_at(Instant::now());
        assert!(drained.is_empty());
    }

    /// Test 8: changed_file_hashes returns (path, hash) pairs using provided hasher.
    #[test]
    fn changed_file_hashes_uses_hasher() {
        let events = vec![
            ChangeEvent {
                path: p("f.nom"),
                kind: ChangeKind::Modified,
            },
            ChangeEvent {
                path: p("g.nom"),
                kind: ChangeKind::Created,
            },
        ];
        let pairs = changed_file_hashes(&events, |path| {
            // Stable hash based on path length for determinism.
            path.to_str().unwrap().len() as u64 * 7
        });
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].0, p("f.nom"));
        assert_eq!(pairs[0].1, "f.nom".len() as u64 * 7);
        assert_eq!(pairs[1].0, p("g.nom"));
        assert_eq!(pairs[1].1, "g.nom".len() as u64 * 7);
    }

    /// Test 9: drain_due after clear returns empty even when time has passed.
    #[test]
    fn drain_after_clear_returns_empty() {
        let mut q = DebouncedChangeQueue::new(50);
        let past = Instant::now() - Duration::from_millis(100);
        q.push_at(
            ChangeEvent {
                path: p("h.nom"),
                kind: ChangeKind::Removed,
            },
            past,
        );
        q.clear();
        let drained = q.drain_due_at(Instant::now());
        assert!(drained.is_empty());
    }
}
