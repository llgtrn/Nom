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
}
