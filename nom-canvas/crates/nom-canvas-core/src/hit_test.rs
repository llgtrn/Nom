/// Hit-testing for canvas elements.
///
/// Phase 1: AABB broadphase (check `bounds.contains(pt)`).
/// Phase 2: Precise — rect uses inverse-rotation test; ellipse uses normalised
///          ellipse equation; connectors use `dist_to_bezier < HIT_RADIUS`.

use crate::elements::{CanvasArrow, CanvasConnector, CanvasEllipse, CanvasLine, CanvasRect};
use nom_gpui::types::{Bounds, Pixels};

/// Pixel radius within which a click is considered a hit on a curve/line.
/// Matches Excalidraw's `HIT_THRESHOLD = 5.0`.
pub const HIT_RADIUS: f32 = 5.0;

// ─── Hit types ──────────────────────────────────────────────────────────────

/// How the pointer intersected with an element.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HitType {
    /// Hit the element body.
    Body,
    /// Hit a transform handle (index into `compute_handles` order: 0=NW…7=W, 8=Rotate).
    Handle(u8),
    /// Hit a connector curve.
    Connector,
}

/// Result of a successful hit test.
pub struct HitResult {
    pub element_id: u64,
    pub hit_type: HitType,
}

// ─── Rectangle ──────────────────────────────────────────────────────────────

/// Axis-aligned bounding box hit test using raw rect parameters.
///
/// Returns `true` if `pt` falls inside the rectangle defined by
/// `(rect_x, rect_y, rect_x + w, rect_y + h)`.
pub fn hit_test_rect_aabb(pt: [f32; 2], rect_x: f32, rect_y: f32, w: f32, h: f32) -> bool {
    pt[0] >= rect_x && pt[0] <= rect_x + w && pt[1] >= rect_y && pt[1] <= rect_y + h
}

/// Circle hit test using raw centre and radius parameters.
///
/// Returns `true` if `pt` falls inside (or on the boundary of) the circle
/// centred at `(cx, cy)` with radius `r`.
pub fn hit_test_circle(pt: [f32; 2], cx: f32, cy: f32, r: f32) -> bool {
    let dx = pt[0] - cx;
    let dy = pt[1] - cy;
    dx * dx + dy * dy <= r * r
}

/// Returns `true` if the raw canvas-space point `pt` falls inside a
/// `nom_gpui::types::Bounds<Pixels>` region.
///
/// This bridges the raw `[f32; 2]` hit-testing layer with the nom_gpui typed
/// bounds used by the renderer, enabling the broadphase to operate directly on
/// GPU-typed regions without an intermediate conversion step.
pub fn hit_test_bounds(pt: [f32; 2], bounds: &Bounds<Pixels>) -> bool {
    let x = bounds.origin.x.0;
    let y = bounds.origin.y.0;
    let w = bounds.size.width.0;
    let h = bounds.size.height.0;
    hit_test_rect_aabb(pt, x, y, w, h)
}

/// Returns `true` if `pt` (canvas-space) hits the rectangle.
///
/// The test inverse-rotates `pt` around the rect centre before doing an AABB
/// check, so rotation is fully respected.  `threshold` expands the hit area
/// (use 0.0 for exact, `HIT_RADIUS` for interactive tolerance).
pub fn hit_test_rect(pt: [f32; 2], rect: &CanvasRect, threshold: f32) -> bool {
    let (ox, oy) = (rect.bounds.0[0], rect.bounds.0[1]);
    let (w, h) = (rect.bounds.1[0], rect.bounds.1[1]);
    let cx = ox + w / 2.0;
    let cy = oy + h / 2.0;

    // Inverse-rotate the point around the rectangle centre.
    let dx = pt[0] - cx;
    let dy = pt[1] - cy;
    let cos = (-rect.rotation).cos();
    let sin = (-rect.rotation).sin();
    let rx = dx * cos - dy * sin + cx;
    let ry = dx * sin + dy * cos + cy;

    rx >= ox - threshold
        && rx <= ox + w + threshold
        && ry >= oy - threshold
        && ry <= oy + h + threshold
}

// ─── Ellipse ────────────────────────────────────────────────────────────────

/// Returns `true` if `pt` hits the ellipse.
///
/// Uses the normalised ellipse equation: `(dx/rx)² + (dy/ry)² ≤ 1`.
pub fn hit_test_ellipse(pt: [f32; 2], ellipse: &CanvasEllipse, threshold: f32) -> bool {
    let (ox, oy) = (ellipse.bounds.0[0], ellipse.bounds.0[1]);
    let (w, h) = (ellipse.bounds.1[0], ellipse.bounds.1[1]);
    let cx = ox + w / 2.0;
    let cy = oy + h / 2.0;
    let rx = w / 2.0 + threshold;
    let ry = h / 2.0 + threshold;
    let dx = (pt[0] - cx) / rx;
    let dy = (pt[1] - cy) / ry;
    dx * dx + dy * dy <= 1.0
}

// ─── Line ───────────────────────────────────────────────────────────────────

/// Returns `true` if `pt` is within `threshold` pixels of the line segment.
pub fn hit_test_line(pt: [f32; 2], line: &CanvasLine, threshold: f32) -> bool {
    dist_to_segment(pt, line.start, line.end) <= threshold
}

// ─── Arrow ──────────────────────────────────────────────────────────────────

/// Returns `true` if `pt` is within `threshold` pixels of the arrow shaft.
pub fn hit_test_arrow(pt: [f32; 2], arrow: &CanvasArrow, threshold: f32) -> bool {
    dist_to_segment(pt, arrow.start, arrow.end) <= threshold
}

// ─── Connector (bezier) ─────────────────────────────────────────────────────

/// Returns `true` if `pt` is within `threshold` pixels of the connector path.
///
/// Requires `connector.route` to have at least 4 points (p0, c1, c2, p3).
/// Shorter routes fall back to straight-line distance between first and last.
pub fn hit_test_connector(pt: [f32; 2], connector: &CanvasConnector, threshold: f32) -> bool {
    let r = &connector.route;
    if r.len() >= 4 {
        dist_to_bezier(pt, r[0], r[1], r[2], r[3]) <= threshold
    } else if r.len() >= 2 {
        dist_to_segment(pt, r[0], r[r.len() - 1]) <= threshold
    } else {
        false
    }
}

// ─── Geometry helpers ───────────────────────────────────────────────────────

/// Minimum distance from `pt` to the line segment `[a, b]`.
pub fn dist_to_segment(pt: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-10 {
        // Degenerate segment — distance to the point itself.
        let ex = pt[0] - a[0];
        let ey = pt[1] - a[1];
        return (ex * ex + ey * ey).sqrt();
    }
    let t = ((pt[0] - a[0]) * dx + (pt[1] - a[1]) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);
    let proj_x = a[0] + t * dx;
    let proj_y = a[1] + t * dy;
    let ex = pt[0] - proj_x;
    let ey = pt[1] - proj_y;
    (ex * ex + ey * ey).sqrt()
}

/// Minimum distance from `pt` to a cubic Bezier curve defined by `p0, c1, c2, p3`.
///
/// Uses 101-sample linear approximation — sufficient for interactive hit-testing.
pub fn dist_to_bezier(
    pt: [f32; 2],
    p0: [f32; 2],
    c1: [f32; 2],
    c2: [f32; 2],
    p3: [f32; 2],
) -> f32 {
    (0..=100)
        .map(|i| {
            let t = i as f32 / 100.0;
            let mt = 1.0 - t;
            let bx = mt.powi(3) * p0[0]
                + 3.0 * mt * mt * t * c1[0]
                + 3.0 * mt * t * t * c2[0]
                + t.powi(3) * p3[0];
            let by = mt.powi(3) * p0[1]
                + 3.0 * mt * mt * t * c1[1]
                + 3.0 * mt * t * t * c2[1]
                + t.powi(3) * p3[1];
            let dx = pt[0] - bx;
            let dy = pt[1] - by;
            (dx * dx + dy * dy).sqrt()
        })
        .fold(f32::INFINITY, f32::min)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elements::{ArrowHead, CanvasArrow, CanvasConnector, CanvasEllipse, CanvasLine, CanvasRect};

    // ── hit_test_rect ────────────────────────────────────────────────────────

    #[test]
    fn rect_hit_inside() {
        let r = CanvasRect {
            id: 1,
            bounds: ([0.0, 0.0], [100.0, 80.0]),
            fill: None,
            stroke: None,
            corner_radius: 0.0,
            rotation: 0.0,
            z_index: 0,
        };
        assert!(hit_test_rect([50.0, 40.0], &r, 0.0));
    }

    #[test]
    fn rect_hit_outside() {
        let r = CanvasRect {
            id: 1,
            bounds: ([0.0, 0.0], [100.0, 80.0]),
            fill: None,
            stroke: None,
            corner_radius: 0.0,
            rotation: 0.0,
            z_index: 0,
        };
        assert!(!hit_test_rect([200.0, 40.0], &r, 0.0));
    }

    #[test]
    fn rect_hit_with_threshold_near_edge() {
        let r = CanvasRect {
            id: 1,
            bounds: ([0.0, 0.0], [100.0, 80.0]),
            fill: None,
            stroke: None,
            corner_radius: 0.0,
            rotation: 0.0,
            z_index: 0,
        };
        // 3px outside the right edge
        assert!(hit_test_rect([103.0, 40.0], &r, HIT_RADIUS));
    }

    // ── hit_test_ellipse ─────────────────────────────────────────────────────

    #[test]
    fn ellipse_hit_center() {
        let e = CanvasEllipse {
            id: 2,
            bounds: ([0.0, 0.0], [100.0, 60.0]),
            fill: None,
            stroke: None,
            z_index: 0,
        };
        // Centre of the ellipse at (50, 30)
        assert!(hit_test_ellipse([50.0, 30.0], &e, 0.0));
    }

    #[test]
    fn ellipse_miss_far_point() {
        let e = CanvasEllipse {
            id: 2,
            bounds: ([0.0, 0.0], [100.0, 60.0]),
            fill: None,
            stroke: None,
            z_index: 0,
        };
        assert!(!hit_test_ellipse([200.0, 200.0], &e, 0.0));
    }

    #[test]
    fn ellipse_hit_on_boundary() {
        let e = CanvasEllipse {
            id: 3,
            bounds: ([0.0, 0.0], [100.0, 100.0]),
            fill: None,
            stroke: None,
            z_index: 0,
        };
        // Right-most point of a circle centred at (50,50) with r=50 is (100,50).
        // With threshold=1 it should hit.
        assert!(hit_test_ellipse([100.0, 50.0], &e, 1.0));
    }

    // ── dist_to_bezier ───────────────────────────────────────────────────────

    #[test]
    fn bezier_diagonal_line_midpoint_near_zero() {
        // Degenerate bezier where all control points lie on the diagonal [0,0]→[100,100]
        let p0 = [0.0_f32, 0.0];
        let c1 = [33.0, 33.0];
        let c2 = [66.0, 66.0];
        let p3 = [100.0, 100.0];
        let midpt = [50.0_f32, 50.0_f32];
        let d = dist_to_bezier(midpt, p0, c1, c2, p3);
        assert!(d < 2.0, "expected ~0 distance to midpoint of diagonal bezier, got {}", d);
    }

    #[test]
    fn bezier_far_point_large_distance() {
        let p0 = [0.0_f32, 0.0];
        let c1 = [33.0, 0.0];
        let c2 = [66.0, 0.0];
        let p3 = [100.0, 0.0];
        let far = [50.0_f32, 500.0_f32];
        let d = dist_to_bezier(far, p0, c1, c2, p3);
        assert!(d > 400.0, "expected large distance, got {}", d);
    }

    // ── hit_test_line ────────────────────────────────────────────────────────

    #[test]
    fn line_hit_on_segment() {
        let l = CanvasLine {
            id: 10,
            start: [0.0, 0.0],
            end: [100.0, 0.0],
            stroke_width: 1.0,
            color: [1.0, 1.0, 1.0, 1.0],
            dashes: vec![],
            z_index: 0,
        };
        assert!(hit_test_line([50.0, 3.0], &l, HIT_RADIUS));
    }

    #[test]
    fn line_miss_off_segment() {
        let l = CanvasLine {
            id: 10,
            start: [0.0, 0.0],
            end: [100.0, 0.0],
            stroke_width: 1.0,
            color: [1.0, 1.0, 1.0, 1.0],
            dashes: vec![],
            z_index: 0,
        };
        assert!(!hit_test_line([50.0, 100.0], &l, HIT_RADIUS));
    }

    // ── hit_test_arrow ───────────────────────────────────────────────────────

    #[test]
    fn arrow_hit_shaft() {
        let a = CanvasArrow {
            id: 20,
            start: [0.0, 0.0],
            end: [100.0, 0.0],
            stroke_width: 1.0,
            color: [1.0, 1.0, 1.0, 1.0],
            head_style: ArrowHead::Open,
            z_index: 0,
        };
        assert!(hit_test_arrow([50.0, 4.0], &a, HIT_RADIUS));
    }

    // ── hit_test_connector ───────────────────────────────────────────────────

    #[test]
    fn connector_hit_on_curve() {
        let conn = CanvasConnector {
            id: 30,
            src_id: 1,
            dst_id: 2,
            route: vec![[0.0, 0.0], [33.0, 0.0], [66.0, 0.0], [100.0, 0.0]],
            confidence: 0.9,
            reason: String::new(),
            z_index: 0,
        };
        // Curve lies along y=0; point 3px above midpoint should hit.
        assert!(hit_test_connector([50.0, 3.0], &conn, HIT_RADIUS));
    }

    // ── dist_to_segment ──────────────────────────────────────────────────────

    #[test]
    fn segment_perpendicular_distance() {
        let d = dist_to_segment([5.0, 5.0], [0.0, 0.0], [10.0, 0.0]);
        assert!((d - 5.0).abs() < 1e-4, "got {}", d);
    }

    #[test]
    fn segment_degenerate_point() {
        let d = dist_to_segment([3.0, 4.0], [0.0, 0.0], [0.0, 0.0]);
        assert!((d - 5.0).abs() < 1e-4, "got {}", d);
    }

    // ── hit_test_rect_aabb (flat params) ─────────────────────────────────────

    #[test]
    fn hit_test_rect_inside() {
        assert!(hit_test_rect_aabb([50.0, 40.0], 0.0, 0.0, 100.0, 80.0));
    }

    #[test]
    fn hit_test_rect_outside() {
        assert!(!hit_test_rect_aabb([200.0, 40.0], 0.0, 0.0, 100.0, 80.0));
    }

    // ── hit_test_circle (flat params) ────────────────────────────────────────

    #[test]
    fn hit_test_circle_inside() {
        // Point at distance 3 from centre (4, 4); radius 5 — inside.
        assert!(hit_test_circle([4.0, 7.0], 4.0, 4.0, 5.0));
    }

    // ── hit_test_bounds (nom_gpui types) ─────────────────────────────────────

    #[test]
    fn hit_test_bounds_inside() {
        use nom_gpui::types::{Bounds, Pixels, Point, Size};
        let bounds = Bounds {
            origin: Point { x: Pixels(10.0), y: Pixels(20.0) },
            size: Size { width: Pixels(100.0), height: Pixels(80.0) },
        };
        // Point well inside the bounds.
        assert!(hit_test_bounds([60.0, 60.0], &bounds));
    }

    #[test]
    fn hit_test_bounds_outside() {
        use nom_gpui::types::{Bounds, Pixels, Point, Size};
        let bounds = Bounds {
            origin: Point { x: Pixels(10.0), y: Pixels(20.0) },
            size: Size { width: Pixels(100.0), height: Pixels(80.0) },
        };
        // Point to the right of the bounds.
        assert!(!hit_test_bounds([200.0, 60.0], &bounds));
    }

    #[test]
    fn hit_test_bounds_on_edge() {
        use nom_gpui::types::{Bounds, Pixels, Point, Size};
        let bounds = Bounds {
            origin: Point { x: Pixels(0.0), y: Pixels(0.0) },
            size: Size { width: Pixels(50.0), height: Pixels(50.0) },
        };
        // Exactly on the right edge — inclusive boundary.
        assert!(hit_test_bounds([50.0, 25.0], &bounds));
        // One pixel beyond the right edge — miss.
        assert!(!hit_test_bounds([51.0, 25.0], &bounds));
    }
}
