//! Rect/Bounds utility helpers: inflate, deflate, union, intersect, contains.
#![deny(unsafe_code)]

use crate::geometry::{Bounds, Pixels, Point, Size};

/// Inflate bounds outward by `dx` horizontally and `dy` vertically.
pub fn inflate(bounds: Bounds<Pixels>, dx: f32, dy: f32) -> Bounds<Pixels> {
    Bounds {
        origin: Point { x: Pixels(bounds.origin.x.0 - dx), y: Pixels(bounds.origin.y.0 - dy) },
        size: Size { width: Pixels(bounds.size.width.0 + 2.0 * dx), height: Pixels(bounds.size.height.0 + 2.0 * dy) },
    }
}

/// Deflate bounds inward by the same amount.  Zero-clamps width/height.
pub fn deflate(bounds: Bounds<Pixels>, dx: f32, dy: f32) -> Bounds<Pixels> {
    let new_w = (bounds.size.width.0 - 2.0 * dx).max(0.0);
    let new_h = (bounds.size.height.0 - 2.0 * dy).max(0.0);
    Bounds {
        origin: Point { x: Pixels(bounds.origin.x.0 + dx), y: Pixels(bounds.origin.y.0 + dy) },
        size: Size { width: Pixels(new_w), height: Pixels(new_h) },
    }
}

/// Union of two bounds: the smallest bounds containing both.
pub fn union(a: Bounds<Pixels>, b: Bounds<Pixels>) -> Bounds<Pixels> {
    let left = a.origin.x.0.min(b.origin.x.0);
    let top = a.origin.y.0.min(b.origin.y.0);
    let right = (a.origin.x.0 + a.size.width.0).max(b.origin.x.0 + b.size.width.0);
    let bottom = (a.origin.y.0 + a.size.height.0).max(b.origin.y.0 + b.size.height.0);
    Bounds {
        origin: Point { x: Pixels(left), y: Pixels(top) },
        size: Size { width: Pixels(right - left), height: Pixels(bottom - top) },
    }
}

/// Intersection of two bounds.  Returns None when they don't overlap.
pub fn intersect(a: Bounds<Pixels>, b: Bounds<Pixels>) -> Option<Bounds<Pixels>> {
    let left = a.origin.x.0.max(b.origin.x.0);
    let top = a.origin.y.0.max(b.origin.y.0);
    let right = (a.origin.x.0 + a.size.width.0).min(b.origin.x.0 + b.size.width.0);
    let bottom = (a.origin.y.0 + a.size.height.0).min(b.origin.y.0 + b.size.height.0);
    if right <= left || bottom <= top {
        return None;
    }
    Some(Bounds {
        origin: Point { x: Pixels(left), y: Pixels(top) },
        size: Size { width: Pixels(right - left), height: Pixels(bottom - top) },
    })
}

/// Does `outer` fully contain `inner`?
pub fn contains_rect(outer: Bounds<Pixels>, inner: Bounds<Pixels>) -> bool {
    let outer_right = outer.origin.x.0 + outer.size.width.0;
    let outer_bottom = outer.origin.y.0 + outer.size.height.0;
    let inner_right = inner.origin.x.0 + inner.size.width.0;
    let inner_bottom = inner.origin.y.0 + inner.size.height.0;
    inner.origin.x.0 >= outer.origin.x.0
        && inner.origin.y.0 >= outer.origin.y.0
        && inner_right <= outer_right
        && inner_bottom <= outer_bottom
}

/// Does `bounds` contain a single point?
pub fn contains_point(bounds: Bounds<Pixels>, point: Point<Pixels>) -> bool {
    let right = bounds.origin.x.0 + bounds.size.width.0;
    let bottom = bounds.origin.y.0 + bounds.size.height.0;
    point.x.0 >= bounds.origin.x.0
        && point.x.0 < right
        && point.y.0 >= bounds.origin.y.0
        && point.y.0 < bottom
}

/// Clip `child` to `clip_region`.  Returns None if no overlap.
pub fn clip_to(child: Bounds<Pixels>, clip_region: Bounds<Pixels>) -> Option<Bounds<Pixels>> {
    intersect(child, clip_region)
}

/// Area of bounds in pixels squared.
pub fn area(bounds: Bounds<Pixels>) -> f32 {
    bounds.size.width.0.max(0.0) * bounds.size.height.0.max(0.0)
}

/// Center point of bounds.
pub fn center(bounds: Bounds<Pixels>) -> Point<Pixels> {
    Point {
        x: Pixels(bounds.origin.x.0 + bounds.size.width.0 / 2.0),
        y: Pixels(bounds.origin.y.0 + bounds.size.height.0 / 2.0),
    }
}

/// Translate bounds by `dx`, `dy`.
pub fn translate(bounds: Bounds<Pixels>, dx: f32, dy: f32) -> Bounds<Pixels> {
    Bounds {
        origin: Point { x: Pixels(bounds.origin.x.0 + dx), y: Pixels(bounds.origin.y.0 + dy) },
        size: bounds.size,
    }
}

/// Scale bounds about its origin.
pub fn scale_from_origin(bounds: Bounds<Pixels>, sx: f32, sy: f32) -> Bounds<Pixels> {
    Bounds {
        origin: bounds.origin,
        size: Size { width: Pixels(bounds.size.width.0 * sx), height: Pixels(bounds.size.height.0 * sy) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
        Bounds {
            origin: Point { x: Pixels(x), y: Pixels(y) },
            size: Size { width: Pixels(w), height: Pixels(h) },
        }
    }

    fn p(x: f32, y: f32) -> Point<Pixels> {
        Point { x: Pixels(x), y: Pixels(y) }
    }

    #[test]
    fn inflate_expands_outward() {
        let r = inflate(b(10.0, 10.0, 20.0, 20.0), 5.0, 5.0);
        assert_eq!(r.origin.x.0, 5.0);
        assert_eq!(r.origin.y.0, 5.0);
        assert_eq!(r.size.width.0, 30.0);
        assert_eq!(r.size.height.0, 30.0);
    }

    #[test]
    fn deflate_clamps_to_zero() {
        let r = deflate(b(10.0, 10.0, 20.0, 20.0), 100.0, 100.0);
        assert_eq!(r.origin.x.0, 110.0);
        assert_eq!(r.origin.y.0, 110.0);
        assert_eq!(r.size.width.0, 0.0);
        assert_eq!(r.size.height.0, 0.0);
    }

    #[test]
    fn union_disjoint_spans_both() {
        let r = union(b(0.0, 0.0, 10.0, 10.0), b(20.0, 20.0, 10.0, 10.0));
        assert_eq!(r.origin.x.0, 0.0);
        assert_eq!(r.origin.y.0, 0.0);
        assert_eq!(r.size.width.0, 30.0);
        assert_eq!(r.size.height.0, 30.0);
    }

    #[test]
    fn union_nested_equals_outer() {
        let outer = b(0.0, 0.0, 100.0, 100.0);
        let inner = b(10.0, 10.0, 20.0, 20.0);
        let r = union(outer, inner);
        assert_eq!(r.origin.x.0, 0.0);
        assert_eq!(r.size.width.0, 100.0);
        assert_eq!(r.size.height.0, 100.0);
    }

    #[test]
    fn intersect_nested_returns_inner() {
        let outer = b(0.0, 0.0, 100.0, 100.0);
        let inner = b(10.0, 10.0, 20.0, 20.0);
        let r = intersect(outer, inner).unwrap();
        assert_eq!(r.origin.x.0, 10.0);
        assert_eq!(r.origin.y.0, 10.0);
        assert_eq!(r.size.width.0, 20.0);
        assert_eq!(r.size.height.0, 20.0);
    }

    #[test]
    fn intersect_disjoint_returns_none() {
        assert!(intersect(b(0.0, 0.0, 10.0, 10.0), b(20.0, 20.0, 10.0, 10.0)).is_none());
    }

    #[test]
    fn intersect_adjacent_returns_none() {
        // Touch at x=10 but no actual overlap.
        assert!(intersect(b(0.0, 0.0, 10.0, 10.0), b(10.0, 0.0, 10.0, 10.0)).is_none());
    }

    #[test]
    fn contains_rect_true_for_nested() {
        assert!(contains_rect(b(0.0, 0.0, 100.0, 100.0), b(10.0, 10.0, 20.0, 20.0)));
    }

    #[test]
    fn contains_rect_false_for_partial_overlap() {
        assert!(!contains_rect(b(0.0, 0.0, 20.0, 20.0), b(10.0, 10.0, 20.0, 20.0)));
    }

    #[test]
    fn contains_rect_false_for_disjoint() {
        assert!(!contains_rect(b(0.0, 0.0, 10.0, 10.0), b(20.0, 20.0, 5.0, 5.0)));
    }

    #[test]
    fn contains_point_true_inside() {
        assert!(contains_point(b(0.0, 0.0, 10.0, 10.0), p(5.0, 5.0)));
    }

    #[test]
    fn contains_point_false_at_right_bottom_edge() {
        // Right and bottom edges are exclusive.
        assert!(!contains_point(b(0.0, 0.0, 10.0, 10.0), p(10.0, 10.0)));
        assert!(!contains_point(b(0.0, 0.0, 10.0, 10.0), p(10.0, 5.0)));
        assert!(!contains_point(b(0.0, 0.0, 10.0, 10.0), p(5.0, 10.0)));
    }

    #[test]
    fn contains_point_true_at_top_left_edge() {
        // Left and top edges are inclusive.
        assert!(contains_point(b(5.0, 5.0, 10.0, 10.0), p(5.0, 5.0)));
    }

    #[test]
    fn clip_to_is_alias_for_intersect() {
        let a = b(0.0, 0.0, 20.0, 20.0);
        let clip = b(10.0, 10.0, 20.0, 20.0);
        assert_eq!(clip_to(a, clip), intersect(a, clip));
    }

    #[test]
    fn area_correct() {
        assert_eq!(area(b(0.0, 0.0, 5.0, 4.0)), 20.0);
    }

    #[test]
    fn area_zero_size() {
        assert_eq!(area(b(0.0, 0.0, 0.0, 0.0)), 0.0);
    }

    #[test]
    fn center_correct() {
        let c = center(b(10.0, 10.0, 20.0, 20.0));
        assert_eq!(c.x.0, 20.0);
        assert_eq!(c.y.0, 20.0);
    }

    #[test]
    fn translate_shifts_origin_only() {
        let r = translate(b(5.0, 5.0, 10.0, 10.0), 3.0, 7.0);
        assert_eq!(r.origin.x.0, 8.0);
        assert_eq!(r.origin.y.0, 12.0);
        assert_eq!(r.size.width.0, 10.0);
        assert_eq!(r.size.height.0, 10.0);
    }

    #[test]
    fn scale_from_origin_scales_size_only() {
        let r = scale_from_origin(b(5.0, 5.0, 10.0, 10.0), 2.0, 3.0);
        assert_eq!(r.origin.x.0, 5.0);
        assert_eq!(r.origin.y.0, 5.0);
        assert_eq!(r.size.width.0, 20.0);
        assert_eq!(r.size.height.0, 30.0);
    }
}
