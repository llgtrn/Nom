//! Marquee (rubber-band) selection.
//!
//! A `Marquee` records a drag from `start` to `current` in canvas (logical-
//! pixel) space and can enumerate which elements fall inside or overlap the
//! resulting rectangle.  The rectangle is always normalised so `origin` is the
//! top-left corner regardless of drag direction.

#![deny(unsafe_code)]

use nom_gpui::{Bounds, Pixels, Point, Size};

use crate::element::{Element, ElementId};

// ── MarqueeMode ───────────────────────────────────────────────────────────────

/// Controls which elements are captured by the marquee.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarqueeMode {
    /// Only elements whose bounding box is **fully contained** by the marquee.
    Contain,
    /// Any element whose bounding box **overlaps** (intersects) the marquee.
    Overlap,
}

// ── Marquee ───────────────────────────────────────────────────────────────────

/// An in-progress rubber-band selection drag.
#[derive(Clone, Debug, PartialEq)]
pub struct Marquee {
    /// The pointer position at drag start (canvas space).
    pub start: Point<Pixels>,
    /// The current pointer position (canvas space).
    pub current: Point<Pixels>,
    /// How elements are captured.
    pub mode: MarqueeMode,
}

impl Marquee {
    /// Start a new marquee at `start` with the given `mode`.
    pub fn new(start: Point<Pixels>, mode: MarqueeMode) -> Self {
        Self {
            start,
            current: start,
            mode,
        }
    }

    /// Update the live end of the drag.
    pub fn update(&mut self, current: Point<Pixels>) {
        self.current = current;
    }

    /// Return the normalised bounding box of the drag rectangle.
    ///
    /// "Normalised" means `origin` is always the top-left corner even when
    /// the drag goes upward or to the left (i.e. `start > current`).
    pub fn bounds(&self) -> Bounds<Pixels> {
        let min_x = self.start.x.0.min(self.current.x.0);
        let min_y = self.start.y.0.min(self.current.y.0);
        let max_x = self.start.x.0.max(self.current.x.0);
        let max_y = self.start.y.0.max(self.current.y.0);

        Bounds::new(
            Point::new(Pixels(min_x), Pixels(min_y)),
            Size::new(Pixels(max_x - min_x), Pixels(max_y - min_y)),
        )
    }

    /// Collect the IDs of elements captured by this marquee.
    ///
    /// Soft-deleted elements (`is_deleted == true`) are silently skipped.
    pub fn collect<'a, I>(&self, elements: I) -> Vec<ElementId>
    where
        I: IntoIterator<Item = &'a Element>,
    {
        let marquee_bounds = self.bounds();
        elements
            .into_iter()
            .filter(|e| !e.is_deleted)
            .filter(|e| match self.mode {
                MarqueeMode::Contain => contains_bounds(marquee_bounds, e.bounds),
                MarqueeMode::Overlap => overlaps_bounds(marquee_bounds, e.bounds),
            })
            .map(|e| e.id)
            .collect()
    }
}

// ── private geometry helpers ──────────────────────────────────────────────────

/// Return `true` when `inner` is fully contained by `outer`.
fn contains_bounds(outer: Bounds<Pixels>, inner: Bounds<Pixels>) -> bool {
    inner.origin.x.0 >= outer.origin.x.0
        && inner.origin.y.0 >= outer.origin.y.0
        && inner.origin.x.0 + inner.size.width.0 <= outer.origin.x.0 + outer.size.width.0
        && inner.origin.y.0 + inner.size.height.0 <= outer.origin.y.0 + outer.size.height.0
}

/// Return `true` when `a` and `b` overlap (AABB intersection).
fn overlaps_bounds(a: Bounds<Pixels>, b: Bounds<Pixels>) -> bool {
    a.origin.x.0 < b.origin.x.0 + b.size.width.0
        && a.origin.x.0 + a.size.width.0 > b.origin.x.0
        && a.origin.y.0 < b.origin.y.0 + b.size.height.0
        && a.origin.y.0 + a.size.height.0 > b.origin.y.0
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shapes::{Rectangle, Shape};

    fn pt(x: f32, y: f32) -> Point<Pixels> {
        Point::new(Pixels(x), Pixels(y))
    }

    fn make_elem(id: ElementId, x: f32, y: f32, w: f32, h: f32) -> Element {
        let bounds = Bounds::new(
            Point::new(Pixels(x), Pixels(y)),
            Size::new(Pixels(w), Pixels(h)),
        );
        Element::new(id, Shape::Rectangle(Rectangle {}), bounds)
    }

    // --- 1. bounds normalisation: start > current --------------------------------

    #[test]
    fn bounds_normalised_when_dragged_upward_left() {
        let m = Marquee {
            start: pt(100.0, 100.0),
            current: pt(10.0, 20.0),
            mode: MarqueeMode::Contain,
        };
        let b = m.bounds();
        assert_eq!(b.origin.x.0, 10.0);
        assert_eq!(b.origin.y.0, 20.0);
        assert_eq!(b.size.width.0, 90.0);
        assert_eq!(b.size.height.0, 80.0);
    }

    // --- 2. empty marquee (zero-area) --------------------------------------------

    #[test]
    fn empty_marquee_zero_area() {
        let m = Marquee::new(pt(50.0, 50.0), MarqueeMode::Contain);
        let b = m.bounds();
        assert_eq!(b.size.width.0, 0.0);
        assert_eq!(b.size.height.0, 0.0);
    }

    // --- 3. Contain mode: element fully inside ───────────────────────────────────

    #[test]
    fn contain_mode_collects_element_fully_inside() {
        let m = Marquee {
            start: pt(0.0, 0.0),
            current: pt(200.0, 200.0),
            mode: MarqueeMode::Contain,
        };
        let elem = make_elem(1, 50.0, 50.0, 50.0, 50.0);
        let ids = m.collect([&elem]);
        assert_eq!(ids, vec![1]);
    }

    // --- 4. Contain mode: element only partially inside — not collected ──────────

    #[test]
    fn contain_mode_skips_partial_overlap() {
        let m = Marquee {
            start: pt(0.0, 0.0),
            current: pt(60.0, 60.0),
            mode: MarqueeMode::Contain,
        };
        // Straddles the right edge of marquee
        let elem = make_elem(2, 50.0, 10.0, 50.0, 20.0);
        let ids = m.collect([&elem]);
        assert!(ids.is_empty());
    }

    // --- 5. Overlap mode: partial overlap is collected ───────────────────────────

    #[test]
    fn overlap_mode_collects_partial_overlap() {
        let m = Marquee {
            start: pt(0.0, 0.0),
            current: pt(60.0, 60.0),
            mode: MarqueeMode::Overlap,
        };
        let elem = make_elem(3, 50.0, 10.0, 50.0, 20.0);
        let ids = m.collect([&elem]);
        assert_eq!(ids, vec![3]);
    }

    // --- 6. Overlap mode: completely outside is not collected ────────────────────

    #[test]
    fn overlap_mode_skips_element_outside() {
        let m = Marquee {
            start: pt(0.0, 0.0),
            current: pt(50.0, 50.0),
            mode: MarqueeMode::Overlap,
        };
        let elem = make_elem(4, 100.0, 100.0, 20.0, 20.0);
        let ids = m.collect([&elem]);
        assert!(ids.is_empty());
    }

    // --- 7. Soft-deleted elements are skipped ────────────────────────────────────

    #[test]
    fn deleted_elements_skipped() {
        let m = Marquee {
            start: pt(0.0, 0.0),
            current: pt(200.0, 200.0),
            mode: MarqueeMode::Contain,
        };
        let mut elem = make_elem(5, 10.0, 10.0, 20.0, 20.0);
        elem.is_deleted = true;
        let ids = m.collect([&elem]);
        assert!(ids.is_empty());
    }

    // --- 8. Multiple elements, mixed containment ─────────────────────────────────

    #[test]
    fn collect_all_inside_contain_mode() {
        let m = Marquee {
            start: pt(0.0, 0.0),
            current: pt(300.0, 300.0),
            mode: MarqueeMode::Contain,
        };
        let e1 = make_elem(10, 10.0, 10.0, 50.0, 50.0);
        let e2 = make_elem(11, 100.0, 100.0, 50.0, 50.0);
        let e3 = make_elem(12, 400.0, 400.0, 50.0, 50.0); // outside
        let mut ids = m.collect([&e1, &e2, &e3]);
        ids.sort_unstable();
        assert_eq!(ids, vec![10, 11]);
    }
}
