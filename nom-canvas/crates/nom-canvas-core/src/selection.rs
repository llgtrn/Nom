use std::collections::BTreeSet;

use crate::elements::ElementBounds;
use crate::spatial_index::SpatialIndex;

// ─── NomtuRef ────────────────────────────────────────────────────────────────

/// A lightweight reference to a dictionary entry: a numeric hash and a human
/// word.  Used as the stable identity for canvas elements that are linked to
/// `nomtu` entries.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NomtuRef {
    /// Numeric hash of the `nomtu` entry.
    pub hash: u64,
    /// Human-readable word component of the entry.
    pub word: String,
}

// ─── Selection ──────────────────────────────────────────────────────────────

/// Tracks which canvas elements are currently selected together with their
/// shared transform origin.
pub struct Selection {
    /// IDs of the selected elements (ordered for deterministic iteration).
    pub ids: BTreeSet<u64>,
    /// The anchor point used when scaling or rotating the selection.
    pub transform_origin: [f32; 2],
}

impl Selection {
    /// Creates an empty selection with no elements and a zeroed transform origin.
    pub fn empty() -> Self {
        Self {
            ids: BTreeSet::new(),
            transform_origin: [0.0, 0.0],
        }
    }

    /// Creates a selection containing a single element with the given transform origin.
    pub fn single(id: u64, origin: [f32; 2]) -> Self {
        let mut ids = BTreeSet::new();
        ids.insert(id);
        Self {
            ids,
            transform_origin: origin,
        }
    }

    /// Returns `true` if no elements are selected.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Returns `true` if the given element ID is selected.
    pub fn contains(&self, id: u64) -> bool {
        self.ids.contains(&id)
    }

    /// Adds an element ID to the selection (idempotent).
    pub fn add(&mut self, id: u64) {
        self.ids.insert(id);
    }

    /// Removes an element ID from the selection (no-op if not present).
    pub fn remove(&mut self, id: u64) {
        self.ids.remove(&id);
    }

    /// Clears all selected element IDs and resets the transform origin.
    pub fn clear(&mut self) {
        self.ids.clear();
        self.transform_origin = [0.0, 0.0];
    }

    /// Returns the number of selected elements.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Returns the number of selected elements (alias for `len`).
    pub fn selected_count(&self) -> usize {
        self.ids.len()
    }

    /// Clears all selected element IDs and resets the transform origin (alias
    /// for `clear` — provided for ergonomic naming consistency with
    /// `selected_count` and `toggle_selection`).
    pub fn clear_selection(&mut self) {
        self.clear();
    }

    /// Toggles the selection state of the element identified by `id`.
    ///
    /// If the element's hash is currently selected it is removed; otherwise it
    /// is added.
    pub fn toggle_selection(&mut self, id: &NomtuRef) {
        if self.ids.contains(&id.hash) {
            self.ids.remove(&id.hash);
        } else {
            self.ids.insert(id.hash);
        }
    }
}

// ─── Spatial-index–backed region selection ──────────────────────────────────

/// Returns the IDs of all elements whose bounding boxes intersect the given
/// canvas-space region `[min, max]`, using the R-tree spatial index for
/// O(log n) performance instead of brute-force iteration.
///
/// The returned `Vec<u64>` can be fed directly to `Selection::add`.
pub fn select_in_region(index: &SpatialIndex, min: [f32; 2], max: [f32; 2]) -> Vec<u64> {
    index.query_in_bounds(min, max)
}

// ─── Rubber-band ────────────────────────────────────────────────────────────

/// Tracks an in-progress rubber-band (marquee) drag selection.
#[derive(Debug, Clone)]
pub struct RubberBand {
    /// Canvas-space start point (where the drag began).
    pub start: [f32; 2],
    /// Canvas-space current end point (updates with mouse move).
    pub end: [f32; 2],
}

impl RubberBand {
    /// Creates a new zero-area rubber-band anchored at `start`.
    pub fn new(start: [f32; 2]) -> Self {
        Self { start, end: start }
    }

    /// Updates the moving end-point as the user drags.
    pub fn update(&mut self, end: [f32; 2]) {
        self.end = end;
    }

    /// Returns the normalised AABB of the rubber-band rectangle
    /// as `(min, max)` such that `min.x <= max.x` and `min.y <= max.y`.
    pub fn aabb(&self) -> ([f32; 2], [f32; 2]) {
        (
            [
                self.start[0].min(self.end[0]),
                self.start[1].min(self.end[1]),
            ],
            [
                self.start[0].max(self.end[0]),
                self.start[1].max(self.end[1]),
            ],
        )
    }

    /// Returns `true` if `bounds` overlaps (or touches) the rubber-band AABB.
    pub fn intersects(&self, bounds: &ElementBounds) -> bool {
        let (min, max) = self.aabb();
        // Separation-axis test — not overlapping if separated along either axis.
        !(bounds.max[0] < min[0]
            || bounds.min[0] > max[0]
            || bounds.max[1] < min[1]
            || bounds.min[1] > max[1])
    }
}

// ─── Transform handles ──────────────────────────────────────────────────────

/// The 9 handles drawn around a selected element (8 resize + 1 rotate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleKind {
    /// Top-left resize handle.
    NW,
    /// Top-centre resize handle.
    N,
    /// Top-right resize handle.
    NE,
    /// Middle-right resize handle.
    E,
    /// Bottom-right resize handle.
    SE,
    /// Bottom-centre resize handle.
    S,
    /// Bottom-left resize handle.
    SW,
    /// Middle-left resize handle.
    W,
    /// Rotate handle — rendered above the centre of the selection.
    Rotate,
}

/// A single transform handle with its canvas-space position.
pub struct TransformHandle {
    /// Which of the 9 handle positions this represents.
    pub kind: HandleKind,
    /// Top-left corner of the 8×8 handle square in canvas space.
    pub position: [f32; 2],
}

/// Compute the 9 standard transform handles for a bounding box.
///
/// `bounds` is `(origin, size)` in canvas space.  The returned positions are
/// the top-left corners of each 8×8 handle square (half-size = 4 px).
pub fn compute_handles(bounds: ([f32; 2], [f32; 2])) -> Vec<TransformHandle> {
    let (origin, size) = bounds;
    let (x, y, w, h) = (origin[0], origin[1], size[0], size[1]);
    let hs = 4.0_f32; // half handle size

    vec![
        TransformHandle {
            kind: HandleKind::NW,
            position: [x - hs, y - hs],
        },
        TransformHandle {
            kind: HandleKind::N,
            position: [x + w / 2.0 - hs, y - hs],
        },
        TransformHandle {
            kind: HandleKind::NE,
            position: [x + w - hs, y - hs],
        },
        TransformHandle {
            kind: HandleKind::E,
            position: [x + w - hs, y + h / 2.0 - hs],
        },
        TransformHandle {
            kind: HandleKind::SE,
            position: [x + w - hs, y + h - hs],
        },
        TransformHandle {
            kind: HandleKind::S,
            position: [x + w / 2.0 - hs, y + h - hs],
        },
        TransformHandle {
            kind: HandleKind::SW,
            position: [x - hs, y + h - hs],
        },
        TransformHandle {
            kind: HandleKind::W,
            position: [x - hs, y + h / 2.0 - hs],
        },
        TransformHandle {
            kind: HandleKind::Rotate,
            position: [x + w / 2.0 - hs, y - 30.0],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_empty_initially() {
        let sel = Selection::empty();
        assert!(sel.is_empty());
        assert_eq!(sel.len(), 0);
    }

    #[test]
    fn selection_single_contains_id() {
        let sel = Selection::single(42, [10.0, 20.0]);
        assert!(!sel.is_empty());
        assert!(sel.contains(42));
        assert!(!sel.contains(43));
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn selection_add_remove() {
        let mut sel = Selection::empty();
        sel.add(1);
        sel.add(2);
        sel.add(3);
        assert_eq!(sel.len(), 3);
        sel.remove(2);
        assert_eq!(sel.len(), 2);
        assert!(!sel.contains(2));
    }

    #[test]
    fn selection_clear_resets() {
        let mut sel = Selection::single(7, [5.0, 5.0]);
        sel.clear();
        assert!(sel.is_empty());
        assert!((sel.transform_origin[0]).abs() < 1e-6);
        assert!((sel.transform_origin[1]).abs() < 1e-6);
    }

    #[test]
    fn rubber_band_aabb_normalised_when_dragged_left() {
        let mut rb = RubberBand::new([100.0, 100.0]);
        rb.update([50.0, 60.0]); // drag up-left
        let (min, max) = rb.aabb();
        assert!(min[0] <= max[0], "min.x must be <= max.x");
        assert!(min[1] <= max[1], "min.y must be <= max.y");
        assert!((min[0] - 50.0).abs() < 1e-6);
        assert!((min[1] - 60.0).abs() < 1e-6);
        assert!((max[0] - 100.0).abs() < 1e-6);
        assert!((max[1] - 100.0).abs() < 1e-6);
    }

    #[test]
    fn rubber_band_intersects_overlapping_element() {
        let rb = RubberBand {
            start: [0.0, 0.0],
            end: [100.0, 100.0],
        };
        let elem = ElementBounds {
            id: 1,
            min: [50.0, 50.0],
            max: [150.0, 150.0],
        };
        assert!(rb.intersects(&elem));
    }

    #[test]
    fn rubber_band_does_not_intersect_distant_element() {
        let rb = RubberBand {
            start: [0.0, 0.0],
            end: [40.0, 40.0],
        };
        let elem = ElementBounds {
            id: 2,
            min: [100.0, 100.0],
            max: [200.0, 200.0],
        };
        assert!(!rb.intersects(&elem));
    }

    #[test]
    fn compute_handles_returns_nine() {
        let handles = compute_handles(([0.0, 0.0], [100.0, 80.0]));
        assert_eq!(handles.len(), 9);
    }

    #[test]
    fn compute_handles_rotate_above_top_edge() {
        let handles = compute_handles(([0.0, 0.0], [100.0, 80.0]));
        let rotate = handles
            .iter()
            .find(|h| h.kind == HandleKind::Rotate)
            .unwrap();
        // Rotate handle should be above y=0 (negative y)
        assert!(
            rotate.position[1] < 0.0,
            "Rotate handle should be above top edge"
        );
    }

    #[test]
    fn compute_handles_nw_at_top_left() {
        let handles = compute_handles(([10.0, 20.0], [60.0, 40.0]));
        let nw = handles.iter().find(|h| h.kind == HandleKind::NW).unwrap();
        // NW is at (x - hs, y - hs) = (10 - 4, 20 - 4) = (6, 16)
        assert!((nw.position[0] - 6.0).abs() < 1e-6);
        assert!((nw.position[1] - 16.0).abs() < 1e-6);
    }

    #[test]
    fn selection_add_element() {
        let mut sel = Selection::empty();
        sel.add(99);
        assert!(sel.contains(99));
        assert_eq!(sel.len(), 1);
        // Adding the same ID twice is idempotent (BTreeSet semantics)
        sel.add(99);
        assert_eq!(sel.len(), 1);
        sel.add(100);
        assert_eq!(sel.len(), 2);
    }

    #[test]
    fn selection_clear() {
        let mut sel = Selection::empty();
        sel.add(1);
        sel.add(2);
        sel.add(3);
        assert_eq!(sel.len(), 3);
        sel.clear();
        assert!(sel.is_empty());
        assert_eq!(sel.len(), 0);
        // transform_origin must be reset to [0,0]
        assert!((sel.transform_origin[0]).abs() < 1e-6);
        assert!((sel.transform_origin[1]).abs() < 1e-6);
    }

    #[test]
    fn selection_contains_element() {
        let mut sel = Selection::empty();
        assert!(!sel.contains(5));
        sel.add(5);
        assert!(sel.contains(5));
        sel.remove(5);
        assert!(!sel.contains(5));
    }

    #[test]
    fn selection_contains_element_fully_inside() {
        // Rubber-band [0,0]→[200,200]; element [50,50]→[100,100] is fully inside.
        let rb = RubberBand {
            start: [0.0, 0.0],
            end: [200.0, 200.0],
        };
        let elem = ElementBounds {
            id: 7,
            min: [50.0, 50.0],
            max: [100.0, 100.0],
        };
        assert!(
            rb.intersects(&elem),
            "element fully inside rubber-band must intersect"
        );
    }

    #[test]
    fn selection_excludes_element_outside() {
        // Rubber-band [0,0]→[50,50]; element [100,100]→[150,150] is completely outside.
        let rb = RubberBand {
            start: [0.0, 0.0],
            end: [50.0, 50.0],
        };
        let elem = ElementBounds {
            id: 8,
            min: [100.0, 100.0],
            max: [150.0, 150.0],
        };
        assert!(
            !rb.intersects(&elem),
            "element outside rubber-band must not intersect"
        );
    }

    #[test]
    fn selection_intersects_partial_overlap() {
        // Rubber-band [0,0]→[80,80]; element [60,60]→[120,120] partially overlaps.
        let rb = RubberBand {
            start: [0.0, 0.0],
            end: [80.0, 80.0],
        };
        let elem = ElementBounds {
            id: 9,
            min: [60.0, 60.0],
            max: [120.0, 120.0],
        };
        assert!(
            rb.intersects(&elem),
            "element partially overlapping rubber-band must intersect"
        );
    }

    #[test]
    fn selection_bounding_box() {
        // Build a selection of 3 elements and compute the union bounding box manually.
        let bounds = vec![
            ElementBounds {
                id: 1,
                min: [0.0, 0.0],
                max: [10.0, 10.0],
            },
            ElementBounds {
                id: 2,
                min: [5.0, 5.0],
                max: [20.0, 25.0],
            },
            ElementBounds {
                id: 3,
                min: [-5.0, 2.0],
                max: [3.0, 8.0],
            },
        ];
        let mut sel = Selection::empty();
        for b in &bounds {
            sel.add(b.id);
        }
        // Union AABB of selected elements
        let selected_bounds: Vec<_> = bounds.iter().filter(|b| sel.contains(b.id)).collect();
        let min_x = selected_bounds
            .iter()
            .map(|b| b.min[0])
            .fold(f32::INFINITY, f32::min);
        let min_y = selected_bounds
            .iter()
            .map(|b| b.min[1])
            .fold(f32::INFINITY, f32::min);
        let max_x = selected_bounds
            .iter()
            .map(|b| b.max[0])
            .fold(f32::NEG_INFINITY, f32::max);
        let max_y = selected_bounds
            .iter()
            .map(|b| b.max[1])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!((min_x - (-5.0)).abs() < 1e-6, "min_x={}", min_x);
        assert!((min_y - 0.0).abs() < 1e-6, "min_y={}", min_y);
        assert!((max_x - 20.0).abs() < 1e-6, "max_x={}", max_x);
        assert!((max_y - 25.0).abs() < 1e-6, "max_y={}", max_y);
    }

    /// Adding an id that is already in the selection does not increase len.
    #[test]
    fn selection_toggle_twice_deselects() {
        let mut sel = Selection::empty();
        sel.add(10);
        assert_eq!(sel.len(), 1);
        // "toggle" = remove if present
        if sel.contains(10) {
            sel.remove(10);
        }
        assert!(
            !sel.contains(10),
            "element must be deselected after toggle-remove"
        );
        assert_eq!(sel.len(), 0);
    }

    /// select_range equivalent: add a slice of ids, all are contained.
    #[test]
    fn selection_range() {
        let ids = [1u64, 2, 3, 4, 5];
        let mut sel = Selection::empty();
        for &id in &ids {
            sel.add(id);
        }
        assert_eq!(sel.len(), ids.len());
        for &id in &ids {
            assert!(sel.contains(id), "id {} must be in selection", id);
        }
    }

    /// Intersection of two selections returns only common elements.
    #[test]
    fn selection_intersection() {
        let mut a = Selection::empty();
        for id in [1u64, 2, 3, 4] {
            a.add(id);
        }
        let mut b = Selection::empty();
        for id in [3u64, 4, 5, 6] {
            b.add(id);
        }
        // Intersection: ids in both a and b
        let inter: std::collections::BTreeSet<u64> = a.ids.intersection(&b.ids).copied().collect();
        assert_eq!(inter.len(), 2);
        assert!(inter.contains(&3));
        assert!(inter.contains(&4));
        assert!(!inter.contains(&1));
        assert!(!inter.contains(&5));
    }

    /// Union of two selections returns all elements from both.
    #[test]
    fn selection_union() {
        let mut a = Selection::empty();
        for id in [1u64, 2, 3] {
            a.add(id);
        }
        let mut b = Selection::empty();
        for id in [3u64, 4, 5] {
            b.add(id);
        }
        // Union: ids in a or b
        let union: std::collections::BTreeSet<u64> = a.ids.union(&b.ids).copied().collect();
        assert_eq!(union.len(), 5);
        for id in [1u64, 2, 3, 4, 5] {
            assert!(union.contains(&id), "union must contain {}", id);
        }
    }

    /// "Move" selected elements: verify new positions differ from originals.
    #[test]
    fn selection_move() {
        // Represent positions as a vec of (id, pos) pairs.
        let mut positions: Vec<(u64, [f32; 2])> =
            vec![(1, [10.0, 20.0]), (2, [50.0, 60.0]), (3, [100.0, 120.0])];
        let mut sel = Selection::empty();
        sel.add(1);
        sel.add(2);
        let delta = [5.0_f32, -10.0];
        // Apply delta to selected elements.
        for (id, pos) in &mut positions {
            if sel.contains(*id) {
                pos[0] += delta[0];
                pos[1] += delta[1];
            }
        }
        let pos1 = positions.iter().find(|(id, _)| *id == 1).unwrap().1;
        let pos2 = positions.iter().find(|(id, _)| *id == 2).unwrap().1;
        let pos3 = positions.iter().find(|(id, _)| *id == 3).unwrap().1;
        assert!((pos1[0] - 15.0).abs() < 1e-6);
        assert!((pos1[1] - 10.0).abs() < 1e-6);
        assert!((pos2[0] - 55.0).abs() < 1e-6);
        assert!((pos2[1] - 50.0).abs() < 1e-6);
        // Unselected element must be unchanged.
        assert!((pos3[0] - 100.0).abs() < 1e-6);
        assert!((pos3[1] - 120.0).abs() < 1e-6);
    }

    /// rubber_band start sets is_active equivalent (start != end initially false after update).
    #[test]
    fn rubber_band_start() {
        let rb = RubberBand::new([50.0, 50.0]);
        // After construction start==end, so aabb has zero area.
        let (min, max) = rb.aabb();
        assert!((min[0] - max[0]).abs() < 1e-6, "zero-width initially");
        assert!((min[1] - max[1]).abs() < 1e-6, "zero-height initially");
    }

    /// rubber_band_update changes the end point, aabb reflects new bounds.
    #[test]
    fn rubber_band_update() {
        let mut rb = RubberBand::new([10.0, 10.0]);
        rb.update([80.0, 90.0]);
        let (min, max) = rb.aabb();
        assert!((min[0] - 10.0).abs() < 1e-6);
        assert!((min[1] - 10.0).abs() < 1e-6);
        assert!((max[0] - 80.0).abs() < 1e-6);
        assert!((max[1] - 90.0).abs() < 1e-6);
    }

    /// rubber_band_finish equivalent: after updating end to start, aabb collapses.
    #[test]
    fn rubber_band_finish() {
        let mut rb = RubberBand::new([30.0, 40.0]);
        rb.update([200.0, 150.0]);
        // "Finish" by collapsing end back to start.
        rb.update(rb.start);
        let (min, max) = rb.aabb();
        assert!(
            (max[0] - min[0]).abs() < 1e-6,
            "finished rubber_band must have zero width"
        );
        assert!(
            (max[1] - min[1]).abs() < 1e-6,
            "finished rubber_band must have zero height"
        );
    }

    /// rubber_band selects enclosed elements via intersects.
    #[test]
    fn rubber_band_selects_enclosed() {
        let rb = RubberBand {
            start: [0.0, 0.0],
            end: [100.0, 100.0],
        };
        let elements = vec![
            ElementBounds {
                id: 1,
                min: [10.0, 10.0],
                max: [40.0, 40.0],
            },
            ElementBounds {
                id: 2,
                min: [200.0, 200.0],
                max: [300.0, 300.0],
            },
            ElementBounds {
                id: 3,
                min: [50.0, 50.0],
                max: [80.0, 80.0],
            },
        ];
        let selected: Vec<u64> = elements
            .iter()
            .filter(|e| rb.intersects(e))
            .map(|e| e.id)
            .collect();
        assert!(selected.contains(&1), "element 1 must be selected");
        assert!(!selected.contains(&2), "element 2 must not be selected");
        assert!(selected.contains(&3), "element 3 must be selected");
    }

    /// rubber_band selects elements with partial overlap.
    #[test]
    fn rubber_band_partial_overlap() {
        let rb = RubberBand {
            start: [0.0, 0.0],
            end: [60.0, 60.0],
        };
        // Element partially overlaps the rubber-band.
        let elem = ElementBounds {
            id: 5,
            min: [40.0, 40.0],
            max: [100.0, 100.0],
        };
        assert!(
            rb.intersects(&elem),
            "partially overlapping element must be selected"
        );
    }

    /// Toggle: add then remove deselects the element.
    #[test]
    fn toggle_selected_to_deselected() {
        let mut sel = Selection::empty();
        sel.add(55);
        assert!(sel.contains(55), "must be selected after add");
        sel.remove(55);
        assert!(!sel.contains(55), "must be deselected after remove");
        assert_eq!(sel.len(), 0);
    }

    /// select_range: adding a range preserves existing selections.
    #[test]
    fn select_range_preserves_existing() {
        let mut sel = Selection::empty();
        sel.add(1);
        // Now add a range of IDs
        for id in [10u64, 11, 12] {
            sel.add(id);
        }
        // Original element 1 must still be present
        assert!(sel.contains(1), "original element must still be selected");
        assert_eq!(sel.len(), 4);
        for id in [10u64, 11, 12] {
            assert!(sel.contains(id), "id {} must be selected", id);
        }
    }

    /// clear_all: after clear, len is 0 and no ids remain.
    #[test]
    fn clear_all_removes_every_element() {
        let mut sel = Selection::empty();
        for id in [1u64, 2, 3, 4, 5] {
            sel.add(id);
        }
        sel.clear();
        assert!(sel.is_empty());
        assert_eq!(sel.len(), 0);
        for id in [1u64, 2, 3, 4, 5] {
            assert!(
                !sel.contains(id),
                "id {} must not be selected after clear",
                id
            );
        }
    }

    /// Intersection with empty set is always empty.
    #[test]
    fn intersection_with_empty_set_is_empty() {
        let mut a = Selection::empty();
        for id in [1u64, 2, 3] {
            a.add(id);
        }
        let b = Selection::empty();
        let inter: std::collections::BTreeSet<u64> = a.ids.intersection(&b.ids).copied().collect();
        assert!(inter.is_empty(), "intersection with empty must be empty");
    }

    /// Intersection with self (full set) equals self.
    #[test]
    fn intersection_with_full_set_is_self() {
        let mut a = Selection::empty();
        for id in [10u64, 20, 30] {
            a.add(id);
        }
        let mut b = Selection::empty();
        for id in [10u64, 20, 30] {
            b.add(id);
        }
        let inter: std::collections::BTreeSet<u64> = a.ids.intersection(&b.ids).copied().collect();
        assert_eq!(inter.len(), 3);
        for id in [10u64, 20, 30] {
            assert!(inter.contains(&id));
        }
    }

    /// RubberBand: begin (new) then drag, aabb shows correct size.
    #[test]
    fn rubber_band_begin_drag_end() {
        let mut rb = RubberBand::new([0.0, 0.0]);
        // Simulate drag sequence
        rb.update([30.0, 20.0]);
        rb.update([60.0, 40.0]);
        let (min, max) = rb.aabb();
        assert!((min[0]).abs() < 1e-6);
        assert!((min[1]).abs() < 1e-6);
        assert!((max[0] - 60.0).abs() < 1e-6, "max.x={}", max[0]);
        assert!((max[1] - 40.0).abs() < 1e-6, "max.y={}", max[1]);
    }

    /// RubberBand dragged backwards (end < start) normalises correctly.
    #[test]
    fn rubber_band_backwards_drag_normalises() {
        let mut rb = RubberBand::new([200.0, 150.0]);
        rb.update([50.0, 30.0]);
        let (min, max) = rb.aabb();
        assert!(min[0] <= max[0], "min.x must be <= max.x");
        assert!(min[1] <= max[1], "min.y must be <= max.y");
        assert!((min[0] - 50.0).abs() < 1e-6);
        assert!((min[1] - 30.0).abs() < 1e-6);
    }

    /// Selection with one element: single() constructor sets transform_origin.
    #[test]
    fn single_selection_has_correct_origin() {
        let origin = [42.0_f32, -7.0];
        let sel = Selection::single(99, origin);
        assert_eq!(sel.len(), 1);
        assert!(sel.contains(99));
        assert!((sel.transform_origin[0] - 42.0).abs() < 1e-6);
        assert!((sel.transform_origin[1] - (-7.0)).abs() < 1e-6);
    }

    /// Removing an element that was never added keeps len unchanged.
    #[test]
    fn remove_nonexistent_is_safe() {
        let mut sel = Selection::empty();
        sel.add(1);
        sel.add(2);
        sel.remove(99); // id 99 was never added
        assert_eq!(
            sel.len(),
            2,
            "len must remain 2 after removing nonexistent id"
        );
        assert!(sel.contains(1));
        assert!(sel.contains(2));
    }

    // ── rubber-band contains mode ────────────────────────────────────────────

    /// contains_fully: rubber-band contains an element only when element is fully inside.
    #[test]
    fn rubber_band_contains_mode_fully_inside() {
        let rb = RubberBand {
            start: [0.0, 0.0],
            end: [100.0, 100.0],
        };
        let inner = ElementBounds {
            id: 1,
            min: [10.0, 10.0],
            max: [90.0, 90.0],
        };
        let (min, max) = rb.aabb();
        let fully_contained = inner.min[0] >= min[0]
            && inner.min[1] >= min[1]
            && inner.max[0] <= max[0]
            && inner.max[1] <= max[1];
        assert!(
            fully_contained,
            "element fully inside rubber-band must be contained"
        );
    }

    /// contains_mode: element partially outside is NOT fully contained.
    #[test]
    fn rubber_band_contains_mode_partial_overlap_excluded() {
        let rb = RubberBand {
            start: [0.0, 0.0],
            end: [80.0, 80.0],
        };
        let partial = ElementBounds {
            id: 2,
            min: [60.0, 60.0],
            max: [120.0, 120.0],
        };
        let (min, max) = rb.aabb();
        let fully_contained = partial.min[0] >= min[0]
            && partial.min[1] >= min[1]
            && partial.max[0] <= max[0]
            && partial.max[1] <= max[1];
        assert!(
            !fully_contained,
            "partially-overlapping element is NOT fully contained"
        );
        // But intersects mode should still pick it up.
        assert!(
            rb.intersects(&partial),
            "intersects mode still finds partial overlap"
        );
    }

    /// contains mode vs intersects mode: same rubber-band, one selects more elements.
    #[test]
    fn rubber_band_intersects_selects_more_than_contains() {
        let rb = RubberBand {
            start: [0.0, 0.0],
            end: [60.0, 60.0],
        };
        let elements = vec![
            // fully inside
            ElementBounds {
                id: 1,
                min: [5.0, 5.0],
                max: [50.0, 50.0],
            },
            // partially outside
            ElementBounds {
                id: 2,
                min: [40.0, 40.0],
                max: [90.0, 90.0],
            },
        ];
        let (rmin, rmax) = rb.aabb();
        let intersecting: Vec<u64> = elements
            .iter()
            .filter(|e| rb.intersects(e))
            .map(|e| e.id)
            .collect();
        let contained: Vec<u64> = elements
            .iter()
            .filter(|e| {
                e.min[0] >= rmin[0]
                    && e.min[1] >= rmin[1]
                    && e.max[0] <= rmax[0]
                    && e.max[1] <= rmax[1]
            })
            .map(|e| e.id)
            .collect();
        assert!(
            intersecting.len() >= contained.len(),
            "intersects selects >= elements vs contains"
        );
        assert!(intersecting.contains(&1));
        assert!(intersecting.contains(&2));
        assert!(contained.contains(&1));
        assert!(!contained.contains(&2));
    }

    // ── viewport fit-to-selection ────────────────────────────────────────────

    /// Fit-to-selection: union AABB of selected elements contains all selected element bounds.
    #[test]
    fn fit_to_selection_union_aabb() {
        let bounds = vec![
            ElementBounds {
                id: 1,
                min: [10.0, 20.0],
                max: [50.0, 60.0],
            },
            ElementBounds {
                id: 2,
                min: [30.0, 10.0],
                max: [80.0, 70.0],
            },
            ElementBounds {
                id: 3,
                min: [-5.0, 5.0],
                max: [15.0, 35.0],
            },
        ];
        let mut sel = Selection::empty();
        for b in &bounds {
            sel.add(b.id);
        }
        let selected: Vec<&ElementBounds> = bounds.iter().filter(|b| sel.contains(b.id)).collect();
        let min_x = selected
            .iter()
            .map(|b| b.min[0])
            .fold(f32::INFINITY, f32::min);
        let min_y = selected
            .iter()
            .map(|b| b.min[1])
            .fold(f32::INFINITY, f32::min);
        let max_x = selected
            .iter()
            .map(|b| b.max[0])
            .fold(f32::NEG_INFINITY, f32::max);
        let max_y = selected
            .iter()
            .map(|b| b.max[1])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!((min_x - (-5.0)).abs() < 1e-6);
        assert!((min_y - 5.0).abs() < 1e-6);
        assert!((max_x - 80.0).abs() < 1e-6);
        assert!((max_y - 70.0).abs() < 1e-6);
    }

    /// Fit-to-selection with single element returns that element's bounds.
    #[test]
    fn fit_to_selection_single_element() {
        let bounds = vec![ElementBounds {
            id: 42,
            min: [100.0, 200.0],
            max: [300.0, 400.0],
        }];
        let mut sel = Selection::empty();
        sel.add(42);
        let selected: Vec<&ElementBounds> = bounds.iter().filter(|b| sel.contains(b.id)).collect();
        let min_x = selected
            .iter()
            .map(|b| b.min[0])
            .fold(f32::INFINITY, f32::min);
        let max_y = selected
            .iter()
            .map(|b| b.max[1])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!((min_x - 100.0).abs() < 1e-6);
        assert!((max_y - 400.0).abs() < 1e-6);
    }

    // ── hit-test priority stack with z-ordering ──────────────────────────────

    /// Z-order: when two elements overlap, the one with higher z_index is "on top".
    #[test]
    fn z_order_higher_z_index_is_on_top() {
        // Represent layered selection by z_index ordering.
        let ids_and_z: Vec<(u64, i32)> = vec![(1, 10), (2, 5), (3, 15), (4, 1)];
        let top = ids_and_z.iter().max_by_key(|(_, z)| *z).unwrap();
        assert_eq!(top.0, 3, "element with z_index=15 is on top");
    }

    /// Multiple elements at same z_index: all tie — last insertion wins by convention.
    #[test]
    fn z_order_tie_uses_last_in_list() {
        let ids_and_z: Vec<(u64, i32)> = vec![(1, 5), (2, 5), (3, 5)];
        // last() simulates "last inserted on top" tiebreak.
        let top = ids_and_z.iter().rfind(|(_, z)| *z == 5).unwrap();
        assert_eq!(top.0, 3);
    }

    /// Priority stack: bring-to-front moves an element to the top z_index.
    #[test]
    fn z_order_bring_to_front() {
        let mut stack: Vec<(u64, i32)> = vec![(1, 1), (2, 2), (3, 3)];
        // Bring element 1 to front: assign max_z + 1
        let max_z = stack.iter().map(|(_, z)| *z).max().unwrap_or(0);
        if let Some(entry) = stack.iter_mut().find(|(id, _)| *id == 1) {
            entry.1 = max_z + 1;
        }
        let top = stack.iter().max_by_key(|(_, z)| *z).unwrap();
        assert_eq!(top.0, 1, "element 1 should be on top after bring-to-front");
    }

    // ── spatial index nearest-k query ────────────────────────────────────────

    /// nearest-k: selecting k nearest elements by distance.
    #[test]
    fn nearest_k_elements() {
        // Simulate nearest-k by sorting elements by distance to a query point.
        let elements = vec![
            (1u64, [5.0_f32, 5.0_f32]),     // dist ≈ 7.07 from (0,0)
            (2u64, [20.0_f32, 20.0_f32]),   // dist ≈ 28.28
            (3u64, [3.0_f32, 4.0_f32]),     // dist = 5.0
            (4u64, [100.0_f32, 100.0_f32]), // dist ≈ 141.4
        ];
        let query = [0.0_f32, 0.0_f32];
        let k = 2;
        let mut by_dist: Vec<(u64, f32)> = elements
            .iter()
            .map(|(id, pos)| {
                let dx = pos[0] - query[0];
                let dy = pos[1] - query[1];
                (*id, (dx * dx + dy * dy).sqrt())
            })
            .collect();
        by_dist.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        let k_nearest: Vec<u64> = by_dist.iter().take(k).map(|(id, _)| *id).collect();
        assert_eq!(k_nearest.len(), k);
        assert!(
            k_nearest.contains(&3),
            "element 3 (dist=5) must be in k=2 nearest"
        );
        assert!(
            k_nearest.contains(&1),
            "element 1 (dist≈7) must be in k=2 nearest"
        );
        assert!(!k_nearest.contains(&2), "element 2 is not among nearest-2");
    }

    /// nearest-k with k >= all elements returns all.
    #[test]
    fn nearest_k_larger_than_count_returns_all() {
        let elements: Vec<(u64, [f32; 2])> = vec![(1, [1.0, 1.0]), (2, [2.0, 2.0])];
        let k = 10;
        let query = [0.0_f32, 0.0_f32];
        let mut by_dist: Vec<(u64, f32)> = elements
            .iter()
            .map(|(id, pos)| {
                let dx = pos[0] - query[0];
                let dy = pos[1] - query[1];
                (*id, (dx * dx + dy * dy).sqrt())
            })
            .collect();
        by_dist.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        let k_nearest: Vec<u64> = by_dist.iter().take(k).map(|(id, _)| *id).collect();
        assert_eq!(
            k_nearest.len(),
            elements.len(),
            "k >= len returns all elements"
        );
    }

    /// nearest-k with empty set returns empty.
    #[test]
    fn nearest_k_empty_returns_empty() {
        let elements: Vec<(u64, [f32; 2])> = vec![];
        let k = 3;
        let query = [0.0_f32, 0.0_f32];
        let mut by_dist: Vec<(u64, f32)> = elements
            .iter()
            .map(|(id, pos)| {
                let dx = pos[0] - query[0];
                let dy = pos[1] - query[1];
                (*id, (dx * dx + dy * dy).sqrt())
            })
            .collect();
        by_dist.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        let k_nearest: Vec<u64> = by_dist.iter().take(k).map(|(id, _)| *id).collect();
        assert!(k_nearest.is_empty());
    }

    // ── additional coverage ───────────────────────────────────────────────────

    /// Selection with many elements: removing half leaves correct count.
    #[test]
    fn selection_remove_half() {
        let mut sel = Selection::empty();
        for id in 1_u64..=10 {
            sel.add(id);
        }
        assert_eq!(sel.len(), 10);
        for id in 1_u64..=5 {
            sel.remove(id);
        }
        assert_eq!(sel.len(), 5);
        for id in 1_u64..=5 {
            assert!(!sel.contains(id), "id {id} must be removed");
        }
        for id in 6_u64..=10 {
            assert!(sel.contains(id), "id {id} must remain");
        }
    }

    /// RubberBand: start == end means zero-area; no element is fully contained.
    #[test]
    fn rubber_band_zero_area_contains_nothing() {
        let rb = RubberBand {
            start: [50.0, 50.0],
            end: [50.0, 50.0],
        };
        let (min, max) = rb.aabb();
        let elem = ElementBounds {
            id: 1,
            min: [10.0, 10.0],
            max: [90.0, 90.0],
        };
        let fully_contained = elem.min[0] >= min[0]
            && elem.min[1] >= min[1]
            && elem.max[0] <= max[0]
            && elem.max[1] <= max[1];
        assert!(
            !fully_contained,
            "large element cannot be contained by zero-area rubber-band"
        );
    }

    /// Z-order: bring-to-back moves element to minimum z_index.
    #[test]
    fn z_order_bring_to_back() {
        let mut stack: Vec<(u64, i32)> = vec![(1, 5), (2, 10), (3, 15)];
        let min_z = stack.iter().map(|(_, z)| *z).min().unwrap_or(0);
        if let Some(entry) = stack.iter_mut().find(|(id, _)| *id == 3) {
            entry.1 = min_z - 1;
        }
        let bottom = stack.iter().min_by_key(|(_, z)| *z).unwrap();
        assert_eq!(
            bottom.0, 3,
            "element 3 should be at bottom after bring-to-back"
        );
    }

    /// Selection: ids are ordered (BTreeSet) — iteration is deterministic.
    #[test]
    fn selection_ids_ordered() {
        let mut sel = Selection::empty();
        sel.add(30);
        sel.add(10);
        sel.add(20);
        let ids: Vec<u64> = sel.ids.iter().copied().collect();
        assert_eq!(ids, vec![10, 20, 30], "BTreeSet must be sorted ascending");
    }

    /// rubber_band intersects with element touching only one corner.
    #[test]
    fn rubber_band_corner_touch_intersects() {
        let rb = RubberBand {
            start: [0.0, 0.0],
            end: [50.0, 50.0],
        };
        // Element touching only the bottom-right corner of the rubber-band.
        let elem = ElementBounds {
            id: 99,
            min: [50.0, 50.0],
            max: [100.0, 100.0],
        };
        // separation-axis: rb max[0]=50 == elem min[0]=50 → not separated → intersects.
        assert!(
            rb.intersects(&elem),
            "corner-touching element must intersect rubber-band"
        );
    }

    // ── additional selection tests ────────────────────────────────────────────

    /// select_all: adding all ids one by one gives correct total count.
    #[test]
    fn select_all_elements() {
        let all_ids = [1u64, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut sel = Selection::empty();
        for &id in &all_ids {
            sel.add(id);
        }
        assert_eq!(
            sel.len(),
            all_ids.len(),
            "select_all must contain every element"
        );
        for &id in &all_ids {
            assert!(
                sel.contains(id),
                "id {id} must be selected after select_all"
            );
        }
    }

    /// deselect_all: clear() resets selection to empty.
    #[test]
    fn deselect_all_elements() {
        let mut sel = Selection::empty();
        for id in [10u64, 20, 30] {
            sel.add(id);
        }
        assert_eq!(sel.len(), 3);
        sel.clear();
        assert!(sel.is_empty(), "deselect_all must leave selection empty");
        assert_eq!(sel.len(), 0, "len must be 0 after deselect_all");
        for id in [10u64, 20, 30] {
            assert!(
                !sel.contains(id),
                "id {id} must not be present after deselect_all"
            );
        }
    }

    /// toggle_selection: adding an already-selected element again is idempotent,
    /// but removing it deselects.
    #[test]
    fn toggle_selection_on_already_selected() {
        let mut sel = Selection::empty();
        sel.add(42);
        assert!(sel.contains(42), "must be selected after first add");
        assert_eq!(sel.len(), 1);
        // Adding the same id again must not change the count (BTreeSet semantics).
        sel.add(42);
        assert_eq!(sel.len(), 1, "duplicate add must be idempotent");
        // Now toggle-off (remove).
        sel.remove(42);
        assert!(!sel.contains(42), "must be deselected after remove");
        assert_eq!(sel.len(), 0);
        // Toggle back on.
        sel.add(42);
        assert!(sel.contains(42), "must be re-selected after second add");
    }

    /// rubber_band with zero area (start == end) does not intersect an element
    /// that doesn't cover that exact point.
    #[test]
    fn rubber_band_zero_area_no_intersect_outside() {
        // Zero-area band at (50, 50); element entirely to the right.
        let rb = RubberBand {
            start: [50.0, 50.0],
            end: [50.0, 50.0],
        };
        let elem = ElementBounds {
            id: 1,
            min: [100.0, 100.0],
            max: [200.0, 200.0],
        };
        assert!(
            !rb.intersects(&elem),
            "zero-area rubber-band must not intersect distant element"
        );
    }

    /// rubber_band with zero area at a point inside an element's bounds returns intersects=true.
    #[test]
    fn rubber_band_zero_area_inside_element_intersects() {
        // Zero-area band at (50, 50); element wraps around that point.
        let rb = RubberBand {
            start: [50.0, 50.0],
            end: [50.0, 50.0],
        };
        let elem = ElementBounds {
            id: 2,
            min: [0.0, 0.0],
            max: [100.0, 100.0],
        };
        // Separation axis: rb [50,50] vs elem [0,100] — no separation on either axis.
        assert!(
            rb.intersects(&elem),
            "zero-area rubber-band inside element must intersect"
        );
    }

    /// selection_size: len() tracks additions and removals precisely.
    #[test]
    fn selection_size_tracks_changes() {
        let mut sel = Selection::empty();
        assert_eq!(sel.len(), 0);
        sel.add(1);
        assert_eq!(sel.len(), 1);
        sel.add(2);
        assert_eq!(sel.len(), 2);
        sel.remove(1);
        assert_eq!(sel.len(), 1);
        sel.remove(999); // nonexistent
        assert_eq!(sel.len(), 1);
        sel.clear();
        assert_eq!(sel.len(), 0);
    }

    /// Adding many elements and checking none are missing.
    #[test]
    fn selection_bulk_add_all_present() {
        let mut sel = Selection::empty();
        for id in 100u64..=120 {
            sel.add(id);
        }
        assert_eq!(sel.len(), 21);
        for id in 100u64..=120 {
            assert!(sel.contains(id), "bulk-added id {id} must be present");
        }
    }

    // ── Wave AK: requested selection scenarios ────────────────────────────────

    /// Rubber-band selection in exclusive (contains) mode captures only elements
    /// fully inside the rubber-band rect.
    #[test]
    fn rubber_band_contains_mode_fully_inside_captures() {
        let rb = RubberBand {
            start: [0.0, 0.0],
            end: [100.0, 100.0],
        };
        let elements = vec![
            ElementBounds {
                id: 1,
                min: [10.0, 10.0],
                max: [80.0, 80.0],
            }, // fully inside
            ElementBounds {
                id: 2,
                min: [60.0, 60.0],
                max: [120.0, 120.0],
            }, // partially outside
            ElementBounds {
                id: 3,
                min: [200.0, 200.0],
                max: [300.0, 300.0],
            }, // fully outside
        ];
        let (rmin, rmax) = rb.aabb();
        let captured: Vec<u64> = elements
            .iter()
            .filter(|e| {
                e.min[0] >= rmin[0]
                    && e.min[1] >= rmin[1]
                    && e.max[0] <= rmax[0]
                    && e.max[1] <= rmax[1]
            })
            .map(|e| e.id)
            .collect();
        assert!(
            captured.contains(&1),
            "element fully inside must be captured in exclusive mode"
        );
        assert!(
            !captured.contains(&2),
            "element partially outside must NOT be captured in exclusive mode"
        );
        assert!(
            !captured.contains(&3),
            "element fully outside must not be captured"
        );
    }

    /// Rubber-band in exclusive mode misses elements only partially overlapping.
    #[test]
    fn rubber_band_exclusive_mode_misses_partial_overlap() {
        let rb = RubberBand {
            start: [0.0, 0.0],
            end: [60.0, 60.0],
        };
        // Element that overlaps by one side only.
        let partial = ElementBounds {
            id: 99,
            min: [50.0, 50.0],
            max: [100.0, 100.0],
        };
        let (rmin, rmax) = rb.aabb();
        let fully_contained = partial.min[0] >= rmin[0]
            && partial.min[1] >= rmin[1]
            && partial.max[0] <= rmax[0]
            && partial.max[1] <= rmax[1];
        assert!(
            !fully_contained,
            "partially-overlapping element must be excluded in exclusive mode"
        );
        // But intersects mode includes it.
        assert!(
            rb.intersects(&partial),
            "intersects mode must still find partial element"
        );
    }

    /// Select all N elements, deselect one — count becomes N-1.
    #[test]
    fn select_all_deselect_one_count_is_n_minus_1() {
        let all_ids = [1u64, 2, 3, 4, 5];
        let mut sel = Selection::empty();
        for &id in &all_ids {
            sel.add(id);
        }
        assert_eq!(sel.len(), 5);
        sel.remove(3);
        assert_eq!(
            sel.len(),
            4,
            "count must be N-1 after deselecting one element"
        );
        assert!(!sel.contains(3), "deselected id must not be present");
        // Remaining ids must still be present.
        for &id in &[1u64, 2, 4, 5] {
            assert!(sel.contains(id), "id {id} must remain selected");
        }
    }

    /// Toggle selection on an already-selected element removes it (deselects).
    #[test]
    fn toggle_on_already_selected_removes_it() {
        let mut sel = Selection::empty();
        sel.add(7);
        sel.add(8);
        assert_eq!(sel.len(), 2);
        // Toggle id=7: it's already selected, so remove it.
        if sel.contains(7) {
            sel.remove(7);
        } else {
            sel.add(7);
        }
        assert!(
            !sel.contains(7),
            "toggling an already-selected element must deselect it"
        );
        assert_eq!(
            sel.len(),
            1,
            "only one element must remain after toggle-off"
        );
    }

    /// Selection.contains returns true for selected IDs and false for unselected IDs.
    #[test]
    fn selection_contains_true_for_selected_false_for_unselected() {
        let mut sel = Selection::empty();
        for id in [10u64, 20, 30] {
            sel.add(id);
        }
        // Selected IDs.
        assert!(sel.contains(10), "10 must be contained (selected)");
        assert!(sel.contains(20), "20 must be contained (selected)");
        assert!(sel.contains(30), "30 must be contained (selected)");
        // Unselected IDs.
        assert!(!sel.contains(11), "11 must not be contained (not selected)");
        assert!(!sel.contains(0), "0 must not be contained");
        assert!(!sel.contains(99), "99 must not be contained");
    }

    /// Rubber-band captures multiple elements fully inside its rect.
    #[test]
    fn rubber_band_captures_multiple_fully_inside() {
        let rb = RubberBand {
            start: [0.0, 0.0],
            end: [200.0, 200.0],
        };
        let elements = vec![
            ElementBounds {
                id: 1,
                min: [10.0, 10.0],
                max: [50.0, 50.0],
            },
            ElementBounds {
                id: 2,
                min: [60.0, 60.0],
                max: [120.0, 120.0],
            },
            ElementBounds {
                id: 3,
                min: [150.0, 150.0],
                max: [190.0, 190.0],
            },
            ElementBounds {
                id: 4,
                min: [180.0, 180.0],
                max: [250.0, 250.0],
            }, // partially outside
        ];
        let (rmin, rmax) = rb.aabb();
        let captured: Vec<u64> = elements
            .iter()
            .filter(|e| {
                e.min[0] >= rmin[0]
                    && e.min[1] >= rmin[1]
                    && e.max[0] <= rmax[0]
                    && e.max[1] <= rmax[1]
            })
            .map(|e| e.id)
            .collect();
        assert!(
            captured.contains(&1),
            "element 1 fully inside must be captured"
        );
        assert!(
            captured.contains(&2),
            "element 2 fully inside must be captured"
        );
        assert!(
            captured.contains(&3),
            "element 3 fully inside must be captured"
        );
        assert!(
            !captured.contains(&4),
            "element 4 partially outside must not be captured"
        );
    }

    /// Toggle selection on an unselected element adds it.
    #[test]
    fn toggle_on_unselected_element_adds_it() {
        let mut sel = Selection::empty();
        // Toggle id=42: not selected → add it.
        if sel.contains(42) {
            sel.remove(42);
        } else {
            sel.add(42);
        }
        assert!(
            sel.contains(42),
            "toggling an unselected element must select it"
        );
        assert_eq!(sel.len(), 1);
    }

    /// Select all 10, deselect all but 1 using remove — count is 1.
    #[test]
    fn select_all_deselect_nine_leaves_one() {
        let mut sel = Selection::empty();
        for id in 1u64..=10 {
            sel.add(id);
        }
        for id in 2u64..=10 {
            sel.remove(id);
        }
        assert_eq!(sel.len(), 1, "only id=1 should remain");
        assert!(sel.contains(1), "id=1 must remain selected");
    }

    /// Rubber-band with zero area at origin: no element is "fully inside" since
    /// the band has zero extent and elements have positive size.
    #[test]
    fn rubber_band_zero_area_captures_no_full_element() {
        let rb = RubberBand {
            start: [50.0, 50.0],
            end: [50.0, 50.0],
        };
        let (rmin, rmax) = rb.aabb();
        let elem = ElementBounds {
            id: 1,
            min: [40.0, 40.0],
            max: [60.0, 60.0],
        };
        // An element with positive size cannot be fully contained in a zero-area band.
        let fully_contained = elem.min[0] >= rmin[0]
            && elem.min[1] >= rmin[1]
            && elem.max[0] <= rmax[0]
            && elem.max[1] <= rmax[1];
        assert!(
            !fully_contained,
            "positive-size element cannot be fully inside a zero-area band"
        );
    }

    // ── additional selection tests (wave AG) ─────────────────────────────────

    #[test]
    fn selection_add_and_contains() {
        let mut sel = Selection::empty();
        sel.add(77);
        assert!(sel.contains(77), "must contain newly added id");
        assert!(!sel.contains(78), "must not contain unadded id");
    }

    #[test]
    fn selection_remove_item() {
        let mut sel = Selection::empty();
        sel.add(10);
        sel.add(20);
        sel.remove(10);
        assert!(!sel.contains(10), "removed id must not be present");
        assert!(sel.contains(20), "non-removed id must still be present");
        assert_eq!(sel.len(), 1);
    }

    #[test]
    fn selection_clear_empties() {
        let mut sel = Selection::empty();
        for id in [1u64, 2, 3, 4, 5] {
            sel.add(id);
        }
        sel.clear();
        assert!(sel.is_empty(), "selection must be empty after clear");
        assert_eq!(sel.len(), 0);
    }

    #[test]
    fn selection_count_correct() {
        let mut sel = Selection::empty();
        assert_eq!(sel.len(), 0);
        sel.add(1);
        assert_eq!(sel.len(), 1);
        sel.add(2);
        sel.add(3);
        assert_eq!(sel.len(), 3);
        sel.remove(2);
        assert_eq!(sel.len(), 2);
    }

    #[test]
    fn selection_toggle_adds_then_removes() {
        let mut sel = Selection::empty();
        // Toggle on.
        sel.add(50);
        assert!(sel.contains(50), "toggle-on must add the id");
        // Toggle off.
        if sel.contains(50) {
            sel.remove(50);
        }
        assert!(!sel.contains(50), "toggle-off must remove the id");
    }

    #[test]
    fn selection_bounding_box_single_element() {
        let bounds = vec![ElementBounds {
            id: 1,
            min: [10.0, 20.0],
            max: [50.0, 60.0],
        }];
        let mut sel = Selection::empty();
        sel.add(1);
        let selected: Vec<&ElementBounds> = bounds.iter().filter(|b| sel.contains(b.id)).collect();
        let min_x = selected
            .iter()
            .map(|b| b.min[0])
            .fold(f32::INFINITY, f32::min);
        let max_y = selected
            .iter()
            .map(|b| b.max[1])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!((min_x - 10.0).abs() < 1e-5);
        assert!((max_y - 60.0).abs() < 1e-5);
    }

    #[test]
    fn selection_bounding_box_multiple_elements() {
        let bounds = vec![
            ElementBounds {
                id: 1,
                min: [0.0, 0.0],
                max: [10.0, 10.0],
            },
            ElementBounds {
                id: 2,
                min: [20.0, 30.0],
                max: [50.0, 70.0],
            },
        ];
        let mut sel = Selection::empty();
        sel.add(1);
        sel.add(2);
        let selected: Vec<&ElementBounds> = bounds.iter().filter(|b| sel.contains(b.id)).collect();
        let min_x = selected
            .iter()
            .map(|b| b.min[0])
            .fold(f32::INFINITY, f32::min);
        let max_x = selected
            .iter()
            .map(|b| b.max[0])
            .fold(f32::NEG_INFINITY, f32::max);
        let min_y = selected
            .iter()
            .map(|b| b.min[1])
            .fold(f32::INFINITY, f32::min);
        let max_y = selected
            .iter()
            .map(|b| b.max[1])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!((min_x).abs() < 1e-5);
        assert!((max_x - 50.0).abs() < 1e-5);
        assert!((min_y).abs() < 1e-5);
        assert!((max_y - 70.0).abs() < 1e-5);
    }

    #[test]
    fn selection_deselect_all() {
        let mut sel = Selection::empty();
        for id in [10u64, 20, 30, 40] {
            sel.add(id);
        }
        sel.clear();
        assert!(sel.is_empty());
        for id in [10u64, 20, 30, 40] {
            assert!(
                !sel.contains(id),
                "id {id} must not remain after deselect_all"
            );
        }
    }

    #[test]
    fn selection_select_all_from_list() {
        let all_ids = [1u64, 5, 10, 15, 20];
        let mut sel = Selection::empty();
        for &id in &all_ids {
            sel.add(id);
        }
        assert_eq!(sel.len(), all_ids.len());
        for &id in &all_ids {
            assert!(
                sel.contains(id),
                "id {id} must be in selection after select_all"
            );
        }
    }

    #[test]
    fn selection_is_empty_initially() {
        let sel = Selection::empty();
        assert!(sel.is_empty(), "fresh selection must be empty");
        assert_eq!(sel.len(), 0);
    }

    #[test]
    fn selection_contains_after_remove_false() {
        let mut sel = Selection::empty();
        sel.add(99);
        sel.remove(99);
        assert!(!sel.contains(99), "removed id must not be contained");
    }

    #[test]
    fn selection_multi_select_append() {
        // Starting with one selection, add more without clearing.
        let mut sel = Selection::empty();
        sel.add(1);
        // Append additional ids.
        for id in [2u64, 3, 4] {
            sel.add(id);
        }
        assert_eq!(sel.len(), 4, "all 4 ids must be selected");
        assert!(sel.contains(1), "original id must still be selected");
        for id in [2u64, 3, 4] {
            assert!(sel.contains(id), "appended id {id} must be selected");
        }
    }

    // ── Wave AO: NomtuRef + spatial-index selection tests ────────────────────

    use super::super::elements::ElementBounds as EB;
    use super::super::spatial_index::SpatialIndex;
    use super::select_in_region;
    use super::NomtuRef;

    fn make_eb(id: u64, min: [f32; 2], max: [f32; 2]) -> EB {
        EB { id, min, max }
    }

    /// selected_count returns 0 for an empty selection.
    #[test]
    fn selected_count_empty_is_zero() {
        let sel = Selection::empty();
        assert_eq!(
            sel.selected_count(),
            0,
            "selected_count must be 0 for empty selection"
        );
    }

    /// selected_count matches len() at all times.
    #[test]
    fn selected_count_matches_len() {
        let mut sel = Selection::empty();
        for id in [1u64, 2, 3, 4, 5] {
            sel.add(id);
        }
        assert_eq!(
            sel.selected_count(),
            sel.len(),
            "selected_count must equal len()"
        );
    }

    /// selected_count decreases after remove.
    #[test]
    fn selected_count_decreases_after_remove() {
        let mut sel = Selection::empty();
        sel.add(10);
        sel.add(20);
        assert_eq!(sel.selected_count(), 2);
        sel.remove(10);
        assert_eq!(
            sel.selected_count(),
            1,
            "selected_count must decrease after remove"
        );
    }

    /// clear_selection empties the selection.
    #[test]
    fn clear_selection_empties() {
        let mut sel = Selection::empty();
        for id in [1u64, 2, 3] {
            sel.add(id);
        }
        sel.clear_selection();
        assert!(
            sel.is_empty(),
            "selection must be empty after clear_selection"
        );
        assert_eq!(
            sel.selected_count(),
            0,
            "selected_count must be 0 after clear_selection"
        );
    }

    /// clear_selection resets transform_origin.
    #[test]
    fn clear_selection_resets_transform_origin() {
        let mut sel = Selection::single(7, [5.0, 5.0]);
        sel.clear_selection();
        assert!(
            (sel.transform_origin[0]).abs() < 1e-6,
            "origin.x must be 0 after clear_selection"
        );
        assert!(
            (sel.transform_origin[1]).abs() < 1e-6,
            "origin.y must be 0 after clear_selection"
        );
    }

    /// toggle_selection adds an unselected element.
    #[test]
    fn toggle_selection_adds_unselected() {
        let mut sel = Selection::empty();
        let id = NomtuRef {
            hash: 42,
            word: "test".to_string(),
        };
        sel.toggle_selection(&id);
        assert!(
            sel.contains(42),
            "toggle on unselected must add the element"
        );
        assert_eq!(sel.selected_count(), 1);
    }

    /// toggle_selection removes an already-selected element.
    #[test]
    fn toggle_selection_removes_selected() {
        let mut sel = Selection::empty();
        let id = NomtuRef {
            hash: 99,
            word: "block".to_string(),
        };
        sel.add(99);
        assert!(sel.contains(99), "element must be selected before toggle");
        sel.toggle_selection(&id);
        assert!(
            !sel.contains(99),
            "toggle on selected must remove the element"
        );
        assert_eq!(sel.selected_count(), 0);
    }

    /// toggle_selection twice returns to original state.
    #[test]
    fn toggle_selection_twice_is_idempotent() {
        let mut sel = Selection::empty();
        let id = NomtuRef {
            hash: 7,
            word: "node".to_string(),
        };
        sel.toggle_selection(&id);
        assert!(sel.contains(7), "first toggle must add");
        sel.toggle_selection(&id);
        assert!(!sel.contains(7), "second toggle must remove");
        assert_eq!(
            sel.selected_count(),
            0,
            "count must be 0 after double-toggle"
        );
    }

    /// toggle_selection on multiple refs works independently.
    #[test]
    fn toggle_selection_multiple_refs_independent() {
        let mut sel = Selection::empty();
        let a = NomtuRef {
            hash: 1,
            word: "a".to_string(),
        };
        let b = NomtuRef {
            hash: 2,
            word: "b".to_string(),
        };
        sel.toggle_selection(&a);
        sel.toggle_selection(&b);
        assert!(sel.contains(1), "a must be selected");
        assert!(sel.contains(2), "b must be selected");
        assert_eq!(sel.selected_count(), 2);
        sel.toggle_selection(&a);
        assert!(!sel.contains(1), "a must be deselected after second toggle");
        assert!(sel.contains(2), "b must still be selected");
    }

    /// select_in_region returns elements within the query bounds from the spatial index.
    #[test]
    fn select_in_region_returns_elements_in_bounds() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_eb(1, [0.0, 0.0], [50.0, 50.0]));
        idx.insert(make_eb(2, [100.0, 100.0], [200.0, 200.0]));
        idx.insert(make_eb(3, [25.0, 25.0], [75.0, 75.0]));
        let ids = select_in_region(&idx, [0.0, 0.0], [60.0, 60.0]);
        assert!(ids.contains(&1), "element 1 must be in region");
        assert!(ids.contains(&3), "element 3 must be in region");
        assert!(!ids.contains(&2), "element 2 must NOT be in region");
    }

    /// select_in_region on an empty index returns empty vec.
    #[test]
    fn select_in_region_empty_index_returns_empty() {
        let idx = SpatialIndex::new();
        let ids = select_in_region(&idx, [0.0, 0.0], [1000.0, 1000.0]);
        assert!(ids.is_empty(), "empty index must return no ids");
    }

    /// select_in_region with a region covering nothing returns empty.
    #[test]
    fn select_in_region_no_match_returns_empty() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_eb(1, [500.0, 500.0], [600.0, 600.0]));
        let ids = select_in_region(&idx, [0.0, 0.0], [10.0, 10.0]);
        assert!(
            ids.is_empty(),
            "no elements in small region far from all elements"
        );
    }

    /// select_in_region: result can be fed directly to Selection::add.
    #[test]
    fn select_in_region_feeds_selection() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_eb(10, [0.0, 0.0], [40.0, 40.0]));
        idx.insert(make_eb(20, [50.0, 50.0], [90.0, 90.0]));
        idx.insert(make_eb(30, [200.0, 200.0], [300.0, 300.0]));
        let mut sel = Selection::empty();
        for id in select_in_region(&idx, [0.0, 0.0], [100.0, 100.0]) {
            sel.add(id);
        }
        assert!(sel.contains(10), "element 10 must be selected");
        assert!(sel.contains(20), "element 20 must be selected");
        assert!(
            !sel.contains(30),
            "element 30 must not be selected (outside region)"
        );
        assert_eq!(sel.selected_count(), 2);
    }

    /// select_in_region covers entire space: all elements are returned.
    #[test]
    fn select_in_region_large_region_returns_all() {
        let mut idx = SpatialIndex::new();
        for i in 1_u64..=5 {
            let base = i as f32 * 30.0;
            idx.insert(make_eb(i, [base, base], [base + 20.0, base + 20.0]));
        }
        let ids = select_in_region(&idx, [-1e6, -1e6], [1e6, 1e6]);
        assert_eq!(ids.len(), 5, "large region must return all 5 elements");
    }

    /// selected_count stays at 0 when clear_selection called on empty.
    #[test]
    fn clear_selection_on_empty_is_safe() {
        let mut sel = Selection::empty();
        sel.clear_selection(); // must not panic
        assert_eq!(sel.selected_count(), 0);
    }

    /// NomtuRef with the same hash selects the same slot regardless of word.
    #[test]
    fn toggle_selection_hash_is_key_not_word() {
        let mut sel = Selection::empty();
        let a = NomtuRef {
            hash: 100,
            word: "alpha".to_string(),
        };
        let b = NomtuRef {
            hash: 100,
            word: "beta".to_string(),
        }; // same hash, different word
        sel.toggle_selection(&a);
        assert!(sel.contains(100), "toggle must add hash 100");
        // b has the same hash: toggling it removes the selection.
        sel.toggle_selection(&b);
        assert!(
            !sel.contains(100),
            "toggle with same hash must remove the selection"
        );
    }

    /// selected_count after adding and removing many elements is correct.
    #[test]
    fn selected_count_bulk_operations() {
        let mut sel = Selection::empty();
        for id in 1_u64..=20 {
            sel.add(id);
        }
        assert_eq!(sel.selected_count(), 20, "must be 20 after bulk add");
        for id in 1_u64..=10 {
            sel.remove(id);
        }
        assert_eq!(sel.selected_count(), 10, "must be 10 after removing half");
        sel.clear_selection();
        assert_eq!(sel.selected_count(), 0, "must be 0 after clear_selection");
    }

    /// select_in_region with touching boundary includes the element.
    #[test]
    fn select_in_region_touching_boundary_included() {
        let mut idx = SpatialIndex::new();
        // Element at [50,50]→[100,100]; query rect right edge touches element's left edge.
        idx.insert(make_eb(1, [50.0, 50.0], [100.0, 100.0]));
        let ids = select_in_region(&idx, [0.0, 0.0], [50.0, 50.0]);
        assert!(
            ids.contains(&1),
            "touching-boundary element must be included"
        );
    }

    /// toggle_selection count increments and decrements correctly for many refs.
    #[test]
    fn toggle_selection_count_tracks_correctly() {
        let mut sel = Selection::empty();
        let refs: Vec<NomtuRef> = (1_u64..=5)
            .map(|h| NomtuRef {
                hash: h,
                word: format!("w{h}"),
            })
            .collect();
        for r in &refs {
            sel.toggle_selection(r);
        }
        assert_eq!(
            sel.selected_count(),
            5,
            "all 5 must be selected after toggle-on"
        );
        // Toggle off all of them.
        for r in &refs {
            sel.toggle_selection(r);
        }
        assert_eq!(
            sel.selected_count(),
            0,
            "all 5 must be deselected after toggle-off"
        );
    }

    /// select_in_region with a single-element index returns that element when in range.
    #[test]
    fn select_in_region_single_element_in_range() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_eb(77, [10.0, 10.0], [30.0, 30.0]));
        let ids = select_in_region(&idx, [5.0, 5.0], [35.0, 35.0]);
        assert_eq!(
            ids,
            vec![77],
            "single element must be returned when in range"
        );
    }

    /// select_in_region: point query at the element centre returns the element.
    #[test]
    fn select_in_region_point_query_at_element_centre() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_eb(55, [0.0, 0.0], [100.0, 100.0]));
        let ids = select_in_region(&idx, [50.0, 50.0], [50.0, 50.0]);
        assert!(
            ids.contains(&55),
            "point query at element centre must return the element"
        );
    }
}
