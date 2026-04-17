//! Element selection state.
//!
//! Tracks which elements are currently selected, the hovered element, and
//! any in-progress marquee drag.  Locked and soft-deleted elements are
//! silently skipped during selection so callers never need to guard.

#![deny(unsafe_code)]

use std::collections::HashSet;

use crate::element::{Element, ElementId};

// ── Selection ─────────────────────────────────────────────────────────────────

/// Active selection state for the canvas.
#[derive(Debug, Default)]
pub struct Selection {
    /// IDs of currently selected elements (in stable iteration order via HashSet).
    pub selected_ids: HashSet<ElementId>,
    /// The element the pointer is currently hovering over, if any.
    pub hovered: Option<ElementId>,
    /// A marquee drag in progress, if any.
    pub pending: Option<crate::marquee::Marquee>,
}

impl Selection {
    /// Create an empty selection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add `id` to the selection unconditionally.
    ///
    /// Callers that want to respect element flags (locked / deleted) should use
    /// [`Self::select_respecting_flags`] instead.
    pub fn select(&mut self, id: ElementId) {
        self.selected_ids.insert(id);
    }

    /// Add every id from `ids` to the selection unconditionally.
    pub fn select_many(&mut self, ids: impl IntoIterator<Item = ElementId>) {
        self.selected_ids.extend(ids);
    }

    /// Remove `id` from the selection.  No-op if not selected.
    pub fn deselect(&mut self, id: ElementId) {
        self.selected_ids.remove(&id);
    }

    /// Remove all selected ids.
    pub fn clear(&mut self) {
        self.selected_ids.clear();
    }

    /// Return `true` if `id` is currently selected.
    pub fn is_selected(&self, id: ElementId) -> bool {
        self.selected_ids.contains(&id)
    }

    /// Number of selected elements.
    pub fn len(&self) -> usize {
        self.selected_ids.len()
    }

    /// Return `true` when nothing is selected.
    pub fn is_empty(&self) -> bool {
        self.selected_ids.is_empty()
    }

    /// Update the hovered element.  Pass `None` to clear.
    pub fn hover(&mut self, id: Option<ElementId>) {
        self.hovered = id;
    }

    /// Add `id` to the selection only if the element is neither locked nor
    /// soft-deleted.  `elements` is a caller-supplied lookup function so this
    /// module stays independent of any storage layer.
    pub fn select_respecting_flags<'a>(
        &mut self,
        id: ElementId,
        elements: impl Fn(ElementId) -> Option<&'a Element>,
    ) {
        if let Some(elem) = elements(id) {
            if !elem.locked && !elem.is_deleted {
                self.selected_ids.insert(id);
            }
        }
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shapes::{Rectangle, Shape};
    use nom_gpui::{Bounds, Pixels, Point, Size};

    fn bounds_100() -> Bounds<Pixels> {
        Bounds::new(
            Point::new(Pixels(0.0), Pixels(0.0)),
            Size::new(Pixels(100.0), Pixels(100.0)),
        )
    }

    fn rect_elem(id: ElementId) -> Element {
        Element::new(id, Shape::Rectangle(Rectangle {}), bounds_100())
    }

    // --- 1. new() yields empty selection -----------------------------------------

    #[test]
    fn new_is_empty() {
        let sel = Selection::new();
        assert!(sel.is_empty());
        assert_eq!(sel.len(), 0);
        assert!(sel.hovered.is_none());
        assert!(sel.pending.is_none());
    }

    // --- 2. select / deselect roundtrip ------------------------------------------

    #[test]
    fn select_deselect_roundtrip() {
        let mut sel = Selection::new();
        sel.select(1);
        assert!(sel.is_selected(1));
        assert_eq!(sel.len(), 1);

        sel.deselect(1);
        assert!(!sel.is_selected(1));
        assert!(sel.is_empty());
    }

    // --- 3. select_many ----------------------------------------------------------

    #[test]
    fn select_many_adds_all() {
        let mut sel = Selection::new();
        sel.select_many([10, 20, 30]);
        assert_eq!(sel.len(), 3);
        assert!(sel.is_selected(10));
        assert!(sel.is_selected(20));
        assert!(sel.is_selected(30));
    }

    // --- 4. clear ----------------------------------------------------------------

    #[test]
    fn clear_removes_all() {
        let mut sel = Selection::new();
        sel.select_many([1, 2, 3]);
        sel.clear();
        assert!(sel.is_empty());
    }

    // --- 5. is_selected miss -----------------------------------------------------

    #[test]
    fn is_selected_miss() {
        let sel = Selection::new();
        assert!(!sel.is_selected(99));
    }

    // --- 6. select_respecting_flags skips locked element -------------------------

    #[test]
    fn respecting_flags_skips_locked() {
        let mut elem = rect_elem(42);
        elem.locked = true;

        let mut sel = Selection::new();
        sel.select_respecting_flags(42, |_id| Some(&elem));
        assert!(!sel.is_selected(42));
    }

    // --- 7. select_respecting_flags skips deleted element ------------------------

    #[test]
    fn respecting_flags_skips_deleted() {
        let mut elem = rect_elem(43);
        elem.is_deleted = true;

        let mut sel = Selection::new();
        sel.select_respecting_flags(43, |_id| Some(&elem));
        assert!(!sel.is_selected(43));
    }

    // --- 8. select_respecting_flags adds normal element --------------------------

    #[test]
    fn respecting_flags_adds_normal() {
        let elem = rect_elem(44);
        let mut sel = Selection::new();
        sel.select_respecting_flags(44, |_id| Some(&elem));
        assert!(sel.is_selected(44));
    }

    // --- 9. hover round-trip -----------------------------------------------------

    #[test]
    fn hover_roundtrip() {
        let mut sel = Selection::new();
        sel.hover(Some(7));
        assert_eq!(sel.hovered, Some(7));
        sel.hover(None);
        assert!(sel.hovered.is_none());
    }
}
