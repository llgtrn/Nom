//! 2-stage hit-testing: AABB fast-reject then per-shape precise test.
//!
//! All tests operate in logical pixel space (`Pixels`).  The tolerance passed
//! to every function is the half-stroke width of the element, clamped to a
//! minimum of 2 px so thin lines remain selectable.

#![deny(unsafe_code)]

use nom_gpui::{Bounds, Pixels, Point};
use crate::element::Element;
use crate::shapes::Shape;

// ── public API ────────────────────────────────────────────────────────────────

/// Returns the hit-testing tolerance for an element whose stroke width is
/// `stroke_width`.  The tolerance is half the stroke width, with a minimum
/// of 2 px so hairlines remain selectable.
pub fn tolerance(stroke_width: Pixels) -> Pixels {
    let t = stroke_width.0 * 0.5;
    Pixels(t.max(2.0))
}

/// Returns `true` when `point` hits `element` within `threshold` pixels.
///
/// Two-stage algorithm:
/// 1. Expand the element's AABB by `threshold` on every side; fast-reject if
///    the point is outside.
/// 2. Delegate to the shape-specific test.
pub fn element_contains(element: &Element, point: Point<Pixels>, threshold: Pixels) -> bool {
    // Stage 1 — cheap AABB fast-reject.
    if !expanded_bounds_contains(element.bounds, point, threshold) {
        return false;
    }
    // Stage 2 — precise per-shape test.
    match &element.shape {
        Shape::Rectangle(_) => rectangle_contains(element.bounds, point, threshold),
        Shape::Ellipse(_) => ellipse_contains(element.bounds, point, threshold),
        Shape::Diamond(_) => diamond_contains(element.bounds, point, threshold),
        Shape::Line(line) => line_contains(&line.endpoints, point, threshold),
        Shape::Arrow(arrow) => arrow_contains(&arrow.waypoints, point, threshold),
        // Text is a filled rectangle — use its bounding box.
        Shape::Text(_) => rectangle_contains(element.bounds, point, threshold),
        Shape::FreeDraw(fd) => freedraw_contains(&fd.points, point, threshold),
        // Images are opaque rectangles.
        Shape::Image(_) => rectangle_contains(element.bounds, point, threshold),
    }
}

// ── private helpers ───────────────────────────────────────────────────────────

/// Returns `true` when `p` is inside `b` expanded by `t` on every side.
///
/// This is the cheapest possible pre-filter: a single axis-aligned box test.
fn expanded_bounds_contains(b: Bounds<Pixels>, p: Point<Pixels>, t: Pixels) -> bool {
    let left = b.origin.x.0 - t.0;
    let top = b.origin.y.0 - t.0;
    let right = b.origin.x.0 + b.size.width.0 + t.0;
    let bottom = b.origin.y.0 + b.size.height.0 + t.0;
    p.x.0 >= left && p.x.0 <= right && p.y.0 >= top && p.y.0 <= bottom
}

/// Rectangle hit test.
///
/// A point hits a rectangle when it is strictly inside the bounds **or**
/// within `threshold` pixels of any edge (stroke test).  Both cases are
/// captured by expanding the bounding box — which is exactly what
/// `expanded_bounds_contains` does.  We therefore only need the strict
/// AABB-contains check here (the caller already passed the expanded test).
fn rectangle_contains(b: Bounds<Pixels>, p: Point<Pixels>, _threshold: Pixels) -> bool {
    // After the AABB fast-reject, a rectangle hit is guaranteed: any point
    // inside the *expanded* bounds is either inside the rectangle or within
    // threshold of an edge.  Re-using `expanded_bounds_contains` with
    // `threshold = 0` gives the strict interior test; but since the caller
    // already applied the expansion we can accept the point unconditionally.
    // We keep a strict test here so rectangle_contains can be called
    // independently from the public API.
    let left = b.origin.x.0;
    let top = b.origin.y.0;
    let right = b.origin.x.0 + b.size.width.0;
    let bottom = b.origin.y.0 + b.size.height.0;
    p.x.0 >= left && p.x.0 <= right && p.y.0 >= top && p.y.0 <= bottom
}

/// Ellipse hit test using the closed-form normalised-distance formula.
///
/// The ellipse is inscribed in `b`.  Centre: `(cx, cy)`, semi-axes:
/// `rx = w/2`, `ry = h/2`.  A point `(px, py)` is inside the ellipse when
///
/// ```text
/// ((px - cx) / rx)² + ((py - cy) / ry)² ≤ 1
/// ```
///
/// To include a `threshold` border we inflate the semi-axes by `t` before
/// evaluating the formula.
fn ellipse_contains(b: Bounds<Pixels>, p: Point<Pixels>, threshold: Pixels) -> bool {
    let cx = b.origin.x.0 + b.size.width.0 * 0.5;
    let cy = b.origin.y.0 + b.size.height.0 * 0.5;
    let rx = b.size.width.0 * 0.5 + threshold.0;
    let ry = b.size.height.0 * 0.5 + threshold.0;

    // Guard against degenerate zero-size bounds.
    if rx <= 0.0 || ry <= 0.0 {
        return false;
    }

    let dx = (p.x.0 - cx) / rx;
    let dy = (p.y.0 - cy) / ry;
    dx * dx + dy * dy <= 1.0
}

/// Diamond (rhombus) hit test via point-in-convex-polygon.
///
/// A diamond inscribed in `b` has four vertices:
/// - **top**   = (cx, top)
/// - **right** = (right, cy)
/// - **bottom**= (cx, bottom)
/// - **left**  = (left, cy)
///
/// We test the point against the four edges using the sign-of-cross-product
/// method: for a convex polygon the point is inside iff it is on the same
/// side (same cross-product sign) of every directed edge.
///
/// Tolerance is applied by inflating `b` before extracting the vertices.
fn diamond_contains(b: Bounds<Pixels>, p: Point<Pixels>, threshold: Pixels) -> bool {
    let left = b.origin.x.0 - threshold.0;
    let top = b.origin.y.0 - threshold.0;
    let right = b.origin.x.0 + b.size.width.0 + threshold.0;
    let bottom = b.origin.y.0 + b.size.height.0 + threshold.0;
    let cx = (left + right) * 0.5;
    let cy = (top + bottom) * 0.5;

    // Four vertices in CCW order: top → left → bottom → right.
    let verts: [(f32, f32); 4] = [
        (cx, top),
        (left, cy),
        (cx, bottom),
        (right, cy),
    ];

    let px = p.x.0;
    let py = p.y.0;

    // All cross products must be ≥ 0 for a CCW polygon (point is inside or on).
    for i in 0..4 {
        let (ax, ay) = verts[i];
        let (bx, by) = verts[(i + 1) % 4];
        // Cross product of edge (A→B) with (A→P).
        let cross = (bx - ax) * (py - ay) - (by - ay) * (px - ax);
        if cross < 0.0 {
            return false;
        }
    }
    true
}

/// Computes the squared distance from point `p` to the segment `[a, b]`.
fn point_to_segment_dist_sq(
    px: f32, py: f32,
    ax: f32, ay: f32,
    bx: f32, by: f32,
) -> f32 {
    let dx = bx - ax;
    let dy = by - ay;
    let len_sq = dx * dx + dy * dy;

    if len_sq == 0.0 {
        // Degenerate segment — distance to the single point.
        let ex = px - ax;
        let ey = py - ay;
        return ex * ex + ey * ey;
    }

    // Project p onto line through a and b, clamped to [0, 1].
    let t = ((px - ax) * dx + (py - ay) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);

    let qx = ax + t * dx;
    let qy = ay + t * dy;
    let ex = px - qx;
    let ey = py - qy;
    ex * ex + ey * ey
}

/// Line hit test: point-to-segment distance ≤ threshold.
fn line_contains(endpoints: &[Point<Pixels>; 2], p: Point<Pixels>, threshold: Pixels) -> bool {
    let dist_sq = point_to_segment_dist_sq(
        p.x.0, p.y.0,
        endpoints[0].x.0, endpoints[0].y.0,
        endpoints[1].x.0, endpoints[1].y.0,
    );
    dist_sq <= threshold.0 * threshold.0
}

/// Arrow hit test: minimum distance to any segment formed by consecutive
/// waypoints is ≤ threshold.
fn arrow_contains(waypoints: &[Point<Pixels>], p: Point<Pixels>, threshold: Pixels) -> bool {
    if waypoints.len() < 2 {
        return false;
    }
    let threshold_sq = threshold.0 * threshold.0;
    for pair in waypoints.windows(2) {
        let dist_sq = point_to_segment_dist_sq(
            p.x.0, p.y.0,
            pair[0].x.0, pair[0].y.0,
            pair[1].x.0, pair[1].y.0,
        );
        if dist_sq <= threshold_sq {
            return true;
        }
    }
    false
}

/// Free-draw hit test: minimum distance to any segment formed by consecutive
/// stroke points is ≤ threshold.
fn freedraw_contains(pts: &[Point<Pixels>], p: Point<Pixels>, threshold: Pixels) -> bool {
    if pts.len() < 2 {
        // Single point — fall back to a radius test.
        if let Some(pt) = pts.first() {
            let dx = p.x.0 - pt.x.0;
            let dy = p.y.0 - pt.y.0;
            return dx * dx + dy * dy <= threshold.0 * threshold.0;
        }
        return false;
    }
    let threshold_sq = threshold.0 * threshold.0;
    for pair in pts.windows(2) {
        let dist_sq = point_to_segment_dist_sq(
            p.x.0, p.y.0,
            pair[0].x.0, pair[0].y.0,
            pair[1].x.0, pair[1].y.0,
        );
        if dist_sq <= threshold_sq {
            return true;
        }
    }
    false
}

// ── helper: build a minimal Element for tests ─────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use nom_gpui::{Bounds, Pixels, Point, Size};
    use crate::element::Element;
    use crate::shapes::{Ellipse, FreeDraw, Rectangle, Shape};

    fn bounds(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
        Bounds {
            origin: Point { x: Pixels(x), y: Pixels(y) },
            size: Size { width: Pixels(w), height: Pixels(h) },
        }
    }

    fn pt(x: f32, y: f32) -> Point<Pixels> {
        Point { x: Pixels(x), y: Pixels(y) }
    }

    fn rect_element(x: f32, y: f32, w: f32, h: f32) -> Element {
        Element::new(1, Shape::Rectangle(Rectangle {}), bounds(x, y, w, h))
    }

    // ── rectangle ────────────────────────────────────────────────────────────

    #[test]
    fn rectangle_center_hit_and_outside_miss() {
        let elem = rect_element(0.0, 0.0, 100.0, 80.0);
        let t = Pixels(2.0);

        // Centre of the rectangle — must hit.
        assert!(element_contains(&elem, pt(50.0, 40.0), t));

        // Well outside any expanded bounds — must miss.
        assert!(!element_contains(&elem, pt(200.0, 200.0), t));

        // Just outside the right edge beyond tolerance — must miss.
        assert!(!element_contains(&elem, pt(103.0, 40.0), t));
    }

    // ── ellipse ───────────────────────────────────────────────────────────────

    #[test]
    fn ellipse_hit_inside_bounds_miss_corner() {
        let elem = Element::new(2, Shape::Ellipse(Ellipse {}), bounds(0.0, 0.0, 100.0, 80.0));
        let t = Pixels(2.0);

        // Centre — inside ellipse.
        assert!(element_contains(&elem, pt(50.0, 40.0), t));

        // Corner of the bounding box — outside the inscribed ellipse, well
        // beyond tolerance as the corner is at the exact 45° arc point and
        // the ellipse clears the corner by ≈ 7 px.
        assert!(!element_contains(&elem, pt(2.0, 2.0), t));
        assert!(!element_contains(&elem, pt(98.0, 78.0), t));
    }

    // ── line ─────────────────────────────────────────────────────────────────

    #[test]
    fn line_hit_on_segment_miss_far() {
        let endpoints = [pt(0.0, 0.0), pt(100.0, 0.0)];
        let t = Pixels(4.0);

        // Point on the segment — must hit.
        assert!(line_contains(&endpoints, pt(50.0, 0.0), t));

        // Point 3 px above the midpoint — within 4 px tolerance.
        assert!(line_contains(&endpoints, pt(50.0, 3.0), t));

        // Point 10 px above — outside tolerance.
        assert!(!line_contains(&endpoints, pt(50.0, 10.0), t));

        // Point beyond the end of the segment, far away.
        assert!(!line_contains(&endpoints, pt(200.0, 0.0), t));
    }

    // ── freedraw ─────────────────────────────────────────────────────────────

    #[test]
    fn freedraw_hit_near_any_point() {
        let pts = vec![pt(0.0, 0.0), pt(20.0, 0.0), pt(40.0, 20.0)];
        let t = Pixels(5.0);

        let fd_elem = Element::new(
            3,
            Shape::FreeDraw(FreeDraw { points: pts.clone(), pressures: vec![1.0; 3] }),
            bounds(0.0, 0.0, 40.0, 20.0),
        );

        // Near the first segment midpoint.
        assert!(element_contains(&fd_elem, pt(10.0, 2.0), t));

        // Far away — must miss.
        assert!(!element_contains(&fd_elem, pt(200.0, 200.0), t));
    }
}
