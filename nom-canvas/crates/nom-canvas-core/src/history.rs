//! Undo/redo history for canvas mutations.
#![deny(unsafe_code)]

use std::collections::HashSet;

use crate::element::{Element, ElementId};

/// A single unit of change applied to one element.
#[derive(Clone, Debug)]
pub enum ElementDiff {
    /// An element was added to the canvas.
    Inserted(Element),
    /// An element was removed from the canvas.
    Removed(Element),
    /// An element's properties changed.
    ///
    /// Stores full before and after copies; field-level diffing is a future
    /// optimization.
    Modified {
        /// State before the mutation.
        before: Box<Element>,
        /// State after the mutation.
        after: Box<Element>,
    },
}

/// A recorded snapshot of one undoable action.
#[derive(Clone, Debug)]
pub struct HistoryEntry {
    /// Monotonically increasing identifier for this entry.
    pub id: u64,
    /// Wall-clock timestamp in milliseconds at time of recording.
    pub timestamp_ms: u64,
    /// Set of selected element IDs before the action.
    pub selection_before: HashSet<ElementId>,
    /// Set of selected element IDs after the action.
    pub selection_after: HashSet<ElementId>,
    /// All element changes that make up this action.
    pub element_diffs: Vec<ElementDiff>,
}

/// Bounded undo/redo stack for canvas mutations.
///
/// `cursor` points to the entry that would be re-applied on redo.
/// The range `[0, cursor)` is the undo history; `[cursor, len)` is the redo stack.
pub struct History {
    entries: Vec<HistoryEntry>,
    cursor: usize,
    next_id: u64,
    capacity: usize,
}

impl History {
    /// Create a new history with the given capacity.
    ///
    /// Capacity is clamped to a minimum of 1.
    pub fn new(capacity: usize) -> Self {
        History {
            entries: Vec::new(),
            cursor: 0,
            next_id: 1,
            capacity: capacity.max(1),
        }
    }

    /// Push a new entry, discarding any forward redo history and rotating out
    /// the oldest entry if the stack exceeds capacity.
    ///
    /// Returns the new entry's id.
    pub fn push(
        &mut self,
        selection_before: HashSet<ElementId>,
        selection_after: HashSet<ElementId>,
        element_diffs: Vec<ElementDiff>,
    ) -> u64 {
        // Drop any redo entries (forward history).
        self.entries.truncate(self.cursor);

        let id = self.next_id;
        self.next_id += 1;

        self.entries.push(HistoryEntry {
            id,
            timestamp_ms: 0, // Caller may fill in; no std time dependency at this layer.
            selection_before,
            selection_after,
            element_diffs,
        });
        self.cursor = self.entries.len();

        // Rotate out the oldest entry if we exceed capacity.
        if self.entries.len() > self.capacity {
            self.entries.remove(0);
            self.cursor = self.entries.len();
        }

        id
    }

    /// Return `true` if there is at least one entry that can be undone.
    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }

    /// Return `true` if there is at least one entry that can be redone.
    pub fn can_redo(&self) -> bool {
        self.cursor < self.entries.len()
    }

    /// Move the cursor back and return the entry to invert.
    ///
    /// Returns `None` when there is nothing to undo.
    pub fn undo(&mut self) -> Option<&HistoryEntry> {
        if !self.can_undo() {
            return None;
        }
        self.cursor -= 1;
        Some(&self.entries[self.cursor])
    }

    /// Move the cursor forward and return the entry to re-apply.
    ///
    /// Returns `None` when there is nothing to redo.
    pub fn redo(&mut self) -> Option<&HistoryEntry> {
        if !self.can_redo() {
            return None;
        }
        let entry = &self.entries[self.cursor];
        self.cursor += 1;
        Some(entry)
    }

    /// Peek at the entry that would be undone without moving the cursor.
    pub fn peek_undo(&self) -> Option<&HistoryEntry> {
        if self.cursor == 0 {
            return None;
        }
        Some(&self.entries[self.cursor - 1])
    }

    /// Peek at the entry that would be redone without moving the cursor.
    pub fn peek_redo(&self) -> Option<&HistoryEntry> {
        self.entries.get(self.cursor)
    }

    /// Total number of entries (undo + redo combined).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return `true` if the history contains no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all history and reset the cursor.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.cursor = 0;
    }
}

/// Produce the inverse of a diff so callers can apply undo operations.
///
/// - `Inserted` becomes `Removed` (and vice-versa).
/// - `Modified { before, after }` becomes `Modified { before: after, after: before }`.
pub fn invert_diff(d: &ElementDiff) -> ElementDiff {
    match d {
        ElementDiff::Inserted(el) => ElementDiff::Removed(el.clone()),
        ElementDiff::Removed(el) => ElementDiff::Inserted(el.clone()),
        ElementDiff::Modified { before, after } => ElementDiff::Modified {
            before: after.clone(),
            after: before.clone(),
        },
    }
}

// ─── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shapes::{Rectangle, Shape};
    use nom_gpui::{Bounds, Pixels, Point, Size};

    fn make_element(id: ElementId) -> Element {
        let bounds = Bounds::new(
            Point::new(Pixels(0.0), Pixels(0.0)),
            Size::new(Pixels(10.0), Pixels(10.0)),
        );
        Element::new(id, Shape::Rectangle(Rectangle {}), bounds)
    }

    fn empty_push(h: &mut History) -> u64 {
        h.push(HashSet::new(), HashSet::new(), vec![])
    }

    #[test]
    fn empty_history_cannot_undo_or_redo() {
        let h = History::new(10);
        assert!(!h.can_undo());
        assert!(!h.can_redo());
    }

    #[test]
    fn push_then_undo_moves_cursor() {
        let mut h = History::new(10);
        empty_push(&mut h);
        assert!(h.can_undo());
        assert!(!h.can_redo());
        let entry = h.undo();
        assert!(entry.is_some());
        assert!(!h.can_undo());
        assert!(h.can_redo());
    }

    #[test]
    fn redo_back_to_same_state() {
        let mut h = History::new(10);
        let id = empty_push(&mut h);
        h.undo();
        let redone = h.redo().expect("redo should succeed");
        assert_eq!(redone.id, id);
        assert!(!h.can_redo());
        assert!(h.can_undo());
    }

    #[test]
    fn capacity_rotation_drops_oldest() {
        let mut h = History::new(3);
        let id1 = empty_push(&mut h);
        empty_push(&mut h);
        empty_push(&mut h);
        // Push a 4th entry — should evict id1.
        empty_push(&mut h);
        assert_eq!(h.len(), 3);
        // The oldest surviving entry should not be id1.
        h.undo(); h.undo(); h.undo();
        let oldest = h.redo().expect("should redo to oldest");
        assert_ne!(oldest.id, id1, "id1 should have been evicted");
    }

    #[test]
    fn new_push_after_undo_drops_redo_stack() {
        let mut h = History::new(10);
        empty_push(&mut h);
        empty_push(&mut h);
        h.undo();
        assert!(h.can_redo());
        empty_push(&mut h);
        assert!(!h.can_redo(), "redo stack must be cleared on new push");
    }

    #[test]
    fn invert_inserted_becomes_removed() {
        let el = make_element(1);
        let diff = ElementDiff::Inserted(el.clone());
        match invert_diff(&diff) {
            ElementDiff::Removed(inv) => assert_eq!(inv.id, el.id),
            other => panic!("expected Removed, got {:?}", other),
        }
    }

    #[test]
    fn invert_modified_swaps_before_after() {
        let before = make_element(1);
        let mut after = make_element(1);
        after.opacity = 0.5;
        let diff = ElementDiff::Modified {
            before: Box::new(before.clone()),
            after: Box::new(after.clone()),
        };
        match invert_diff(&diff) {
            ElementDiff::Modified { before: inv_before, after: inv_after } => {
                assert!((inv_before.opacity - after.opacity).abs() < f32::EPSILON);
                assert!((inv_after.opacity - before.opacity).abs() < f32::EPSILON);
            }
            other => panic!("expected Modified, got {:?}", other),
        }
    }

    #[test]
    fn peek_undo_and_redo_do_not_move_cursor() {
        let mut h = History::new(10);
        let id = empty_push(&mut h);
        assert_eq!(h.peek_undo().map(|e| e.id), Some(id));
        assert!(h.peek_redo().is_none());

        h.undo();
        assert!(h.peek_undo().is_none());
        assert_eq!(h.peek_redo().map(|e| e.id), Some(id));
        // Cursor unchanged by peeking.
        assert!(!h.can_undo());
        assert!(h.can_redo());
    }
}
