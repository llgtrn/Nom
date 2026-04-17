//! Collaborator presence view-model.
//!
//! Wraps the raw `awareness` state into a UI-friendly struct with deterministic
//! colour assignment, sorted display order, and cursor/selection accessors.
#![deny(unsafe_code)]

use std::collections::HashMap;

pub type ClientId = u64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CursorPosition {
    pub line: u32,
    pub column: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelectionRange {
    pub anchor_line: u32,
    pub anchor_column: u32,
    pub head_line: u32,
    pub head_column: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Collaborator {
    pub client_id: ClientId,
    pub display_name: String,
    pub cursor: Option<CursorPosition>,
    pub selection: Option<SelectionRange>,
    pub colour_index: u8,     // 0..=11, used as index into the presence-palette
    pub focused_doc_id: Option<String>,
    pub last_seen_ms: u64,
}

impl Collaborator {
    pub fn new(client_id: ClientId, display_name: impl Into<String>) -> Self {
        let display_name = display_name.into();
        Self {
            colour_index: Self::colour_for(client_id),
            client_id,
            display_name,
            cursor: None,
            selection: None,
            focused_doc_id: None,
            last_seen_ms: 0,
        }
    }

    /// Deterministic colour-index assignment: stable hash of client_id mod 12.
    pub fn colour_for(client_id: ClientId) -> u8 {
        use std::hash::{DefaultHasher, Hash, Hasher};
        let mut h = DefaultHasher::new();
        client_id.hash(&mut h);
        (h.finish() % 12) as u8
    }

    pub fn with_cursor(mut self, pos: CursorPosition) -> Self {
        self.cursor = Some(pos);
        self
    }
    pub fn with_selection(mut self, sel: SelectionRange) -> Self {
        self.selection = Some(sel);
        self
    }
    pub fn with_focus(mut self, doc_id: impl Into<String>) -> Self {
        self.focused_doc_id = Some(doc_id.into());
        self
    }
    pub fn touch(&mut self, now_ms: u64) {
        self.last_seen_ms = now_ms;
    }
}

#[derive(Default)]
pub struct PresenceView {
    collaborators: HashMap<ClientId, Collaborator>,
}

impl PresenceView {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn upsert(&mut self, c: Collaborator) {
        self.collaborators.insert(c.client_id, c);
    }
    pub fn remove(&mut self, id: ClientId) -> bool {
        self.collaborators.remove(&id).is_some()
    }
    pub fn get(&self, id: ClientId) -> Option<&Collaborator> {
        self.collaborators.get(&id)
    }
    pub fn len(&self) -> usize {
        self.collaborators.len()
    }
    pub fn is_empty(&self) -> bool {
        self.collaborators.is_empty()
    }

    /// Deterministic display order: sort by last_seen_ms descending then by client_id.
    pub fn ordered(&self) -> Vec<&Collaborator> {
        let mut v: Vec<&Collaborator> = self.collaborators.values().collect();
        v.sort_by(|a, b| {
            b.last_seen_ms
                .cmp(&a.last_seen_ms)
                .then(a.client_id.cmp(&b.client_id))
        });
        v
    }

    /// Filter to collaborators focused on the same document.
    pub fn on_document(&self, doc_id: &str) -> Vec<&Collaborator> {
        self.collaborators
            .values()
            .filter(|c| c.focused_doc_id.as_deref() == Some(doc_id))
            .collect()
    }

    /// Prune collaborators whose `last_seen_ms + stale_threshold_ms < now_ms`.
    pub fn prune_stale(&mut self, now_ms: u64, stale_threshold_ms: u64) -> usize {
        let before = self.collaborators.len();
        self.collaborators
            .retain(|_, c| c.last_seen_ms + stale_threshold_ms >= now_ms);
        before - self.collaborators.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_position_construct() {
        let pos = CursorPosition { line: 10, column: 5 };
        assert_eq!(pos.line, 10);
        assert_eq!(pos.column, 5);
    }

    #[test]
    fn selection_range_construct() {
        let sel = SelectionRange {
            anchor_line: 1,
            anchor_column: 0,
            head_line: 3,
            head_column: 7,
        };
        assert_eq!(sel.anchor_line, 1);
        assert_eq!(sel.head_column, 7);
    }

    #[test]
    fn collaborator_new_sets_colour_via_hash() {
        let c = Collaborator::new(42, "Alice");
        assert_eq!(c.colour_index, Collaborator::colour_for(42));
        assert!(c.colour_index < 12);
    }

    #[test]
    fn colour_for_is_deterministic() {
        let a = Collaborator::colour_for(999);
        let b = Collaborator::colour_for(999);
        assert_eq!(a, b);
    }

    #[test]
    fn colour_for_distributes_across_ids() {
        let colours: std::collections::HashSet<u8> =
            (1u64..=50).map(Collaborator::colour_for).collect();
        assert!(colours.len() >= 3, "expected at least 3 distinct colours, got {}", colours.len());
    }

    #[test]
    fn builder_chain_cursor_selection_focus() {
        let pos = CursorPosition { line: 2, column: 4 };
        let sel = SelectionRange {
            anchor_line: 1,
            anchor_column: 0,
            head_line: 2,
            head_column: 4,
        };
        let c = Collaborator::new(1, "Bob")
            .with_cursor(pos)
            .with_selection(sel.clone())
            .with_focus("doc-abc");
        assert_eq!(c.cursor, Some(pos));
        assert_eq!(c.selection, Some(sel));
        assert_eq!(c.focused_doc_id.as_deref(), Some("doc-abc"));
    }

    #[test]
    fn touch_updates_last_seen_ms() {
        let mut c = Collaborator::new(1, "Carol");
        assert_eq!(c.last_seen_ms, 0);
        c.touch(12345);
        assert_eq!(c.last_seen_ms, 12345);
    }

    #[test]
    fn presence_view_new_is_empty() {
        let pv = PresenceView::new();
        assert!(pv.is_empty());
        assert_eq!(pv.len(), 0);
    }

    #[test]
    fn upsert_adds_and_replaces() {
        let mut pv = PresenceView::new();
        let c1 = Collaborator::new(1, "Alice");
        pv.upsert(c1);
        assert_eq!(pv.len(), 1);
        // upsert again with different name — should replace
        let c2 = Collaborator::new(1, "Alice-v2");
        pv.upsert(c2);
        assert_eq!(pv.len(), 1);
        assert_eq!(pv.get(1).unwrap().display_name, "Alice-v2");
    }

    #[test]
    fn remove_hit_and_miss() {
        let mut pv = PresenceView::new();
        pv.upsert(Collaborator::new(7, "Dave"));
        assert!(pv.remove(7));
        assert!(!pv.remove(7)); // already gone
    }

    #[test]
    fn ordered_sorts_by_last_seen_desc_then_client_id_asc() {
        let mut pv = PresenceView::new();
        let mut c1 = Collaborator::new(1, "A");
        c1.touch(100);
        let mut c2 = Collaborator::new(2, "B");
        c2.touch(200);
        let mut c3 = Collaborator::new(3, "C");
        c3.touch(200); // same time as c2 — tie-break by client_id
        pv.upsert(c1);
        pv.upsert(c2);
        pv.upsert(c3);
        let ord = pv.ordered();
        assert_eq!(ord[0].client_id, 2); // last_seen 200, id 2
        assert_eq!(ord[1].client_id, 3); // last_seen 200, id 3
        assert_eq!(ord[2].client_id, 1); // last_seen 100
    }

    #[test]
    fn on_document_filters_by_focus() {
        let mut pv = PresenceView::new();
        pv.upsert(Collaborator::new(1, "A").with_focus("doc-1"));
        pv.upsert(Collaborator::new(2, "B").with_focus("doc-2"));
        pv.upsert(Collaborator::new(3, "C").with_focus("doc-1"));
        let on1 = pv.on_document("doc-1");
        let ids: std::collections::HashSet<u64> = on1.iter().map(|c| c.client_id).collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&1));
        assert!(ids.contains(&3));
    }

    #[test]
    fn prune_stale_removes_old_returns_count() {
        let mut pv = PresenceView::new();
        let mut old = Collaborator::new(1, "Old");
        old.touch(100);
        let mut fresh = Collaborator::new(2, "Fresh");
        fresh.touch(9000);
        pv.upsert(old);
        pv.upsert(fresh);
        // now_ms=10000, threshold=1000 → 100+1000=1100 < 10000 → old pruned
        let pruned = pv.prune_stale(10000, 1000);
        assert_eq!(pruned, 1);
        assert_eq!(pv.len(), 1);
        assert!(pv.get(2).is_some());
    }

    #[test]
    fn prune_stale_keeps_recent_entries() {
        let mut pv = PresenceView::new();
        let mut c = Collaborator::new(5, "Recent");
        c.touch(9500);
        pv.upsert(c);
        let pruned = pv.prune_stale(10000, 1000);
        assert_eq!(pruned, 0);
        assert_eq!(pv.len(), 1);
    }
}
