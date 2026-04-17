//! Set of text selections (multi-cursor) with disjoint + pending variants.
#![deny(unsafe_code)]

use crate::anchor::{Anchor, Bias};
use crate::selection::{SelectionId, TextSelection};

/// Manages a set of non-overlapping selections (multi-cursor support).
///
/// Internally splits selections into `disjoint` (committed) and `pending`
/// (in-progress drag/extend). `all()` merges and coalesces them.
pub struct SelectionsCollection {
    disjoint: Vec<TextSelection>,
    pending: Option<TextSelection>,
    next_id: SelectionId,
}

impl SelectionsCollection {
    /// Create a collection with a single default cursor at offset 0.
    pub fn new() -> Self {
        let default = TextSelection::new(
            0,
            Anchor::new(0, Bias::Left),
            Anchor::new(0, Bias::Left),
        );
        Self {
            disjoint: vec![default],
            pending: None,
            next_id: 1,
        }
    }

    /// All selections merged and coalesced (overlapping ranges folded).
    pub fn all(&self) -> Vec<TextSelection> {
        let mut combined: Vec<TextSelection> = self.disjoint.clone();
        if let Some(ref p) = self.pending {
            combined.push(p.clone());
        }
        // Sort by ascending start offset.
        combined.sort_by_key(|s| s.range().start);
        // Coalesce overlapping / touching selections.
        let mut result: Vec<TextSelection> = Vec::with_capacity(combined.len());
        for sel in combined {
            if let Some(last) = result.last_mut() {
                let last_end = last.range().end;
                let sel_start = sel.range().start;
                if sel_start <= last_end {
                    // Merge: extend last selection to cover the union.
                    let new_end = last_end.max(sel.range().end);
                    last.end = Anchor::new(new_end, last.end.bias);
                    continue;
                }
            }
            result.push(sel);
        }
        result
    }

    /// Placeholder — same as `all()` for now; callers that need display-adjusted
    /// offsets will use this hook.
    pub fn all_adjusted(&self) -> Vec<TextSelection> {
        self.all()
    }

    /// The selection with the highest id (most recently added).
    pub fn newest(&self) -> Option<&TextSelection> {
        let best_disjoint = self.disjoint.iter().max_by_key(|s| s.id);
        match &self.pending {
            Some(p) => {
                let p_id = p.id;
                if best_disjoint.map_or(true, |d| p_id >= d.id) {
                    self.pending.as_ref()
                } else {
                    best_disjoint
                }
            }
            None => best_disjoint,
        }
    }

    /// Number of coalesced selections.
    pub fn count(&self) -> usize {
        self.all().len()
    }

    /// Replace the pending selection.
    pub fn set_pending(&mut self, sel: TextSelection) {
        self.pending = Some(sel);
    }

    /// Move pending selection into the disjoint set.
    pub fn commit_pending(&mut self) {
        if let Some(p) = self.pending.take() {
            self.disjoint.push(p);
        }
    }

    /// Clear all selections and reset to a single default cursor at offset 0.
    pub fn clear(&mut self) {
        self.disjoint.clear();
        self.pending = None;
        self.disjoint.push(TextSelection::new(
            0,
            Anchor::new(0, Bias::Left),
            Anchor::new(0, Bias::Left),
        ));
    }

    /// Apply a mutation closure to a snapshot of all selections, then store
    /// the result back (replacing disjoint; clears pending).
    pub fn change_selections<F>(&mut self, mutate: F)
    where
        F: FnOnce(&mut Vec<TextSelection>),
    {
        let mut snapshot = self.all();
        mutate(&mut snapshot);
        self.disjoint = snapshot;
        self.pending = None;
    }

    /// Push a new selection, assigning the next available id.  Returns the id.
    pub fn push(&mut self, mut sel: TextSelection) -> SelectionId {
        let id = self.next_id;
        self.next_id += 1;
        sel.id = id;
        self.disjoint.push(sel);
        id
    }
}

impl Default for SelectionsCollection {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sel(id: SelectionId, start: usize, end: usize) -> TextSelection {
        TextSelection::new(
            id,
            Anchor::new(start, Bias::Left),
            Anchor::new(end, Bias::Left),
        )
    }

    #[test]
    fn new_has_one_default_selection() {
        let c = SelectionsCollection::new();
        assert_eq!(c.all().len(), 1);
        assert_eq!(c.all()[0].range(), 0..0);
    }

    #[test]
    fn push_increments_id() {
        let mut c = SelectionsCollection::new();
        let id1 = c.push(sel(0, 5, 10));
        let id2 = c.push(sel(0, 15, 20));
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    #[test]
    fn overlapping_selections_merged_in_all() {
        let mut c = SelectionsCollection::new();
        // Replace default with two overlapping selections.
        c.disjoint = vec![sel(0, 0, 5), sel(1, 3, 9)];
        let result = c.all();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].range(), 0..9);
    }

    #[test]
    fn newest_returns_highest_id() {
        let mut c = SelectionsCollection::new();
        c.push(sel(0, 10, 15));
        c.push(sel(0, 20, 25));
        let newest = c.newest().expect("should have selections");
        assert_eq!(newest.id, 2);
    }

    #[test]
    fn pending_visible_in_all() {
        let mut c = SelectionsCollection::new();
        c.set_pending(sel(99, 50, 60));
        let all = c.all();
        assert!(all.iter().any(|s| s.range() == (50..60)));
    }

    #[test]
    fn commit_pending_moves_to_disjoint() {
        let mut c = SelectionsCollection::new();
        c.set_pending(sel(0, 5, 10));
        c.commit_pending();
        assert!(c.pending.is_none());
        assert!(c.disjoint.iter().any(|s| s.range() == (5..10)));
    }

    #[test]
    fn change_selections_mutates_and_clears_pending() {
        let mut c = SelectionsCollection::new();
        c.set_pending(sel(0, 5, 8));
        c.change_selections(|sels| {
            for s in sels.iter_mut() {
                s.start = Anchor::new(s.start.offset + 1, Bias::Left);
                s.end = Anchor::new(s.end.offset + 1, Bias::Left);
            }
        });
        assert!(c.pending.is_none());
        // All offsets shifted by 1.
        for s in c.all() {
            assert!(s.range().start >= 1 || s.range().end >= 1);
        }
    }
}
