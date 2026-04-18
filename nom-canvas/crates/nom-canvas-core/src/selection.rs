use std::collections::BTreeSet;

use crate::elements::ElementBounds;

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
    pub fn empty() -> Self {
        Self {
            ids: BTreeSet::new(),
            transform_origin: [0.0, 0.0],
        }
    }

    pub fn single(id: u64, origin: [f32; 2]) -> Self {
        let mut ids = BTreeSet::new();
        ids.insert(id);
        Self {
            ids,
            transform_origin: origin,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn contains(&self, id: u64) -> bool {
        self.ids.contains(&id)
    }

    pub fn add(&mut self, id: u64) {
        self.ids.insert(id);
    }

    pub fn remove(&mut self, id: u64) {
        self.ids.remove(&id);
    }

    pub fn clear(&mut self) {
        self.ids.clear();
        self.transform_origin = [0.0, 0.0];
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }
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
    pub fn new(start: [f32; 2]) -> Self {
        Self { start, end: start }
    }

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
    NW,
    N,
    NE,
    E,
    SE,
    S,
    SW,
    W,
    /// Rotate handle — rendered above the centre of the selection.
    Rotate,
}

/// A single transform handle with its canvas-space position.
pub struct TransformHandle {
    pub kind: HandleKind,
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
        assert_eq!(sel.len(), 2, "len must remain 2 after removing nonexistent id");
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
        assert!(fully_contained, "element fully inside rubber-band must be contained");
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
        assert!(!fully_contained, "partially-overlapping element is NOT fully contained");
        // But intersects mode should still pick it up.
        assert!(rb.intersects(&partial), "intersects mode still finds partial overlap");
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
            ElementBounds { id: 1, min: [5.0, 5.0], max: [50.0, 50.0] },
            // partially outside
            ElementBounds { id: 2, min: [40.0, 40.0], max: [90.0, 90.0] },
        ];
        let (rmin, rmax) = rb.aabb();
        let intersecting: Vec<u64> = elements.iter().filter(|e| rb.intersects(e)).map(|e| e.id).collect();
        let contained: Vec<u64> = elements.iter().filter(|e| {
            e.min[0] >= rmin[0] && e.min[1] >= rmin[1] && e.max[0] <= rmax[0] && e.max[1] <= rmax[1]
        }).map(|e| e.id).collect();
        assert!(intersecting.len() >= contained.len(), "intersects selects >= elements vs contains");
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
            ElementBounds { id: 1, min: [10.0, 20.0], max: [50.0, 60.0] },
            ElementBounds { id: 2, min: [30.0, 10.0], max: [80.0, 70.0] },
            ElementBounds { id: 3, min: [-5.0, 5.0], max: [15.0, 35.0] },
        ];
        let mut sel = Selection::empty();
        for b in &bounds { sel.add(b.id); }
        let selected: Vec<&ElementBounds> = bounds.iter().filter(|b| sel.contains(b.id)).collect();
        let min_x = selected.iter().map(|b| b.min[0]).fold(f32::INFINITY, f32::min);
        let min_y = selected.iter().map(|b| b.min[1]).fold(f32::INFINITY, f32::min);
        let max_x = selected.iter().map(|b| b.max[0]).fold(f32::NEG_INFINITY, f32::max);
        let max_y = selected.iter().map(|b| b.max[1]).fold(f32::NEG_INFINITY, f32::max);
        assert!((min_x - (-5.0)).abs() < 1e-6);
        assert!((min_y - 5.0).abs() < 1e-6);
        assert!((max_x - 80.0).abs() < 1e-6);
        assert!((max_y - 70.0).abs() < 1e-6);
    }

    /// Fit-to-selection with single element returns that element's bounds.
    #[test]
    fn fit_to_selection_single_element() {
        let bounds = vec![ElementBounds { id: 42, min: [100.0, 200.0], max: [300.0, 400.0] }];
        let mut sel = Selection::empty();
        sel.add(42);
        let selected: Vec<&ElementBounds> = bounds.iter().filter(|b| sel.contains(b.id)).collect();
        let min_x = selected.iter().map(|b| b.min[0]).fold(f32::INFINITY, f32::min);
        let max_y = selected.iter().map(|b| b.max[1]).fold(f32::NEG_INFINITY, f32::max);
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
        let top = ids_and_z.iter().filter(|(_, z)| *z == 5).last().unwrap();
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
            (1u64, [5.0_f32, 5.0_f32]),    // dist ≈ 7.07 from (0,0)
            (2u64, [20.0_f32, 20.0_f32]),   // dist ≈ 28.28
            (3u64, [3.0_f32, 4.0_f32]),     // dist = 5.0
            (4u64, [100.0_f32, 100.0_f32]), // dist ≈ 141.4
        ];
        let query = [0.0_f32, 0.0_f32];
        let k = 2;
        let mut by_dist: Vec<(u64, f32)> = elements.iter().map(|(id, pos)| {
            let dx = pos[0] - query[0];
            let dy = pos[1] - query[1];
            (*id, (dx * dx + dy * dy).sqrt())
        }).collect();
        by_dist.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        let k_nearest: Vec<u64> = by_dist.iter().take(k).map(|(id, _)| *id).collect();
        assert_eq!(k_nearest.len(), k);
        assert!(k_nearest.contains(&3), "element 3 (dist=5) must be in k=2 nearest");
        assert!(k_nearest.contains(&1), "element 1 (dist≈7) must be in k=2 nearest");
        assert!(!k_nearest.contains(&2), "element 2 is not among nearest-2");
    }

    /// nearest-k with k >= all elements returns all.
    #[test]
    fn nearest_k_larger_than_count_returns_all() {
        let elements: Vec<(u64, [f32; 2])> = vec![(1, [1.0, 1.0]), (2, [2.0, 2.0])];
        let k = 10;
        let query = [0.0_f32, 0.0_f32];
        let mut by_dist: Vec<(u64, f32)> = elements.iter().map(|(id, pos)| {
            let dx = pos[0] - query[0];
            let dy = pos[1] - query[1];
            (*id, (dx * dx + dy * dy).sqrt())
        }).collect();
        by_dist.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        let k_nearest: Vec<u64> = by_dist.iter().take(k).map(|(id, _)| *id).collect();
        assert_eq!(k_nearest.len(), elements.len(), "k >= len returns all elements");
    }

    /// nearest-k with empty set returns empty.
    #[test]
    fn nearest_k_empty_returns_empty() {
        let elements: Vec<(u64, [f32; 2])> = vec![];
        let k = 3;
        let query = [0.0_f32, 0.0_f32];
        let mut by_dist: Vec<(u64, f32)> = elements.iter().map(|(id, pos)| {
            let dx = pos[0] - query[0];
            let dy = pos[1] - query[1];
            (*id, (dx * dx + dy * dy).sqrt())
        }).collect();
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
        let rb = RubberBand { start: [50.0, 50.0], end: [50.0, 50.0] };
        let (min, max) = rb.aabb();
        let elem = ElementBounds { id: 1, min: [10.0, 10.0], max: [90.0, 90.0] };
        let fully_contained = elem.min[0] >= min[0]
            && elem.min[1] >= min[1]
            && elem.max[0] <= max[0]
            && elem.max[1] <= max[1];
        assert!(!fully_contained, "large element cannot be contained by zero-area rubber-band");
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
        assert_eq!(bottom.0, 3, "element 3 should be at bottom after bring-to-back");
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
        let rb = RubberBand { start: [0.0, 0.0], end: [50.0, 50.0] };
        // Element touching only the bottom-right corner of the rubber-band.
        let elem = ElementBounds { id: 99, min: [50.0, 50.0], max: [100.0, 100.0] };
        // separation-axis: rb max[0]=50 == elem min[0]=50 → not separated → intersects.
        assert!(rb.intersects(&elem), "corner-touching element must intersect rubber-band");
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
            assert!(sel.contains(id), "id {id} must be selected after select_all");
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
            assert!(!sel.contains(id), "id {id} must not be present after deselect_all");
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
        let rb = RubberBand { start: [50.0, 50.0], end: [50.0, 50.0] };
        let elem = ElementBounds { id: 1, min: [100.0, 100.0], max: [200.0, 200.0] };
        assert!(
            !rb.intersects(&elem),
            "zero-area rubber-band must not intersect distant element"
        );
    }

    /// rubber_band with zero area at a point inside an element's bounds returns intersects=true.
    #[test]
    fn rubber_band_zero_area_inside_element_intersects() {
        // Zero-area band at (50, 50); element wraps around that point.
        let rb = RubberBand { start: [50.0, 50.0], end: [50.0, 50.0] };
        let elem = ElementBounds { id: 2, min: [0.0, 0.0], max: [100.0, 100.0] };
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
}
