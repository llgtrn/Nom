//! Recently-executed command history with bounded capacity + pinning.
//!
//! Separate from the canvas-core undo/redo stack (that one stores element
//! diffs; this one stores symbolic command ids for recall in the palette).
#![deny(unsafe_code)]

use std::collections::VecDeque;

pub type CommandId = String;

#[derive(Clone, Debug, PartialEq)]
pub struct CommandHistoryEntry {
    pub id: CommandId,
    pub executed_at_ms: u64,
    pub pinned: bool,
}

pub struct CommandHistory {
    entries: VecDeque<CommandHistoryEntry>,
    max_entries: usize,
    pinned_cap: usize,
}

impl CommandHistory {
    pub fn new(max_entries: usize, pinned_cap: usize) -> Self {
        Self { entries: VecDeque::new(), max_entries, pinned_cap }
    }

    /// Record an execution.  If the id already exists in history, it is
    /// removed from its old position so the new timestamp wins.  Oldest
    /// unpinned entries are evicted when the queue exceeds `max_entries`.
    pub fn record(&mut self, id: impl Into<CommandId>, executed_at_ms: u64) {
        let id = id.into();
        // Remove old occurrence if present, preserving pinned state.
        let prior_pinned = self.entries.iter().find(|e| e.id == id).map(|e| e.pinned).unwrap_or(false);
        self.entries.retain(|e| e.id != id);
        self.entries.push_front(CommandHistoryEntry { id, executed_at_ms, pinned: prior_pinned });
        self.evict_if_needed();
    }

    /// Pin or unpin.  Pinned entries are exempt from capacity-based eviction
    /// up to `pinned_cap`; beyond that, the oldest pin unpins.
    pub fn set_pinned(&mut self, id: &str, pinned: bool) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) {
            entry.pinned = pinned;
        }
        if pinned {
            // Enforce pinned_cap by unpinning oldest pins if we exceeded it.
            let mut pinned_ids: Vec<(usize, u64)> = self.entries.iter().enumerate()
                .filter(|(_, e)| e.pinned).map(|(i, e)| (i, e.executed_at_ms)).collect();
            if pinned_ids.len() > self.pinned_cap {
                pinned_ids.sort_by_key(|(_, t)| *t);
                let over = pinned_ids.len() - self.pinned_cap;
                for (idx, _) in pinned_ids.iter().take(over) {
                    if let Some(e) = self.entries.get_mut(*idx) { e.pinned = false; }
                }
            }
        }
    }

    pub fn recent(&self, limit: usize) -> Vec<&CommandHistoryEntry> {
        self.entries.iter().take(limit).collect()
    }

    pub fn pinned(&self) -> Vec<&CommandHistoryEntry> {
        self.entries.iter().filter(|e| e.pinned).collect()
    }

    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }
    pub fn clear_unpinned(&mut self) { self.entries.retain(|e| e.pinned); }

    fn evict_if_needed(&mut self) {
        while self.entries.len() > self.max_entries {
            // Find the OLDEST unpinned entry (iterate from back).
            let oldest = self.entries.iter().enumerate().rev().find(|(_, e)| !e.pinned).map(|(i, _)| i);
            match oldest {
                Some(i) => { self.entries.remove(i); }
                None => break,   // Everything pinned — honour pinned_cap semantics.
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_empty() {
        let h = CommandHistory::new(10, 3);
        assert!(h.is_empty());
        assert_eq!(h.len(), 0);
    }

    #[test]
    fn record_adds_to_front() {
        let mut h = CommandHistory::new(10, 3);
        h.record("a", 1);
        h.record("b", 2);
        assert_eq!(h.recent(2)[0].id, "b");
        assert_eq!(h.recent(2)[1].id, "a");
    }

    #[test]
    fn record_same_id_moves_to_front_no_duplicate() {
        let mut h = CommandHistory::new(10, 3);
        h.record("a", 1);
        h.record("b", 2);
        h.record("a", 3);
        assert_eq!(h.len(), 2);
        assert_eq!(h.recent(1)[0].id, "a");
        assert_eq!(h.recent(1)[0].executed_at_ms, 3);
    }

    #[test]
    fn record_preserves_pinned_state_on_re_record() {
        let mut h = CommandHistory::new(10, 3);
        h.record("a", 1);
        h.set_pinned("a", true);
        h.record("a", 2);
        assert!(h.recent(1)[0].pinned);
    }

    #[test]
    fn evict_at_max_leaves_newest() {
        let mut h = CommandHistory::new(3, 1);
        for i in 0u64..5 {
            h.record(format!("cmd{}", i), i);
        }
        assert_eq!(h.len(), 3);
        let ids: Vec<&str> = h.recent(3).iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["cmd4", "cmd3", "cmd2"]);
    }

    #[test]
    fn set_pinned_survives_eviction() {
        let mut h = CommandHistory::new(3, 3);
        h.record("keep", 1);
        h.set_pinned("keep", true);
        for i in 0u64..10 {
            h.record(format!("x{}", i), i + 2);
        }
        let ids: Vec<&str> = h.entries.iter().map(|e| e.id.as_str()).collect();
        assert!(ids.contains(&"keep"), "pinned entry should survive eviction");
    }

    #[test]
    fn set_pinned_too_many_honors_pinned_cap() {
        let mut h = CommandHistory::new(10, 2);
        h.record("a", 1);
        h.record("b", 2);
        h.record("c", 3);
        h.set_pinned("a", true);
        h.set_pinned("b", true);
        // Adding a third pin should cause the oldest to unpin.
        h.set_pinned("c", true);
        let pinned_count = h.pinned().len();
        assert_eq!(pinned_count, 2);
    }

    #[test]
    fn recent_returns_up_to_n() {
        let mut h = CommandHistory::new(10, 3);
        for i in 0u64..7 {
            h.record(format!("c{}", i), i);
        }
        assert_eq!(h.recent(4).len(), 4);
        assert_eq!(h.recent(100).len(), 7);
    }

    #[test]
    fn pinned_returns_only_pinned() {
        let mut h = CommandHistory::new(10, 3);
        h.record("a", 1);
        h.record("b", 2);
        h.record("c", 3);
        h.set_pinned("b", true);
        let p = h.pinned();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].id, "b");
    }

    #[test]
    fn clear_unpinned_keeps_only_pinned() {
        let mut h = CommandHistory::new(10, 3);
        h.record("a", 1);
        h.record("b", 2);
        h.record("c", 3);
        h.set_pinned("b", true);
        h.clear_unpinned();
        assert_eq!(h.len(), 1);
        assert_eq!(h.recent(1)[0].id, "b");
    }
}
