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
pub fn dist_to_bezier(pt: [f32; 2], p0: [f32; 2], c1: [f32; 2], c2: [f32; 2], p3: [f32; 2]) -> f32 {
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
    use crate::elements::{
        ArrowHead, CanvasArrow, CanvasConnector, CanvasEllipse, CanvasLine, CanvasRect,
    };

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
        assert!(
            d < 2.0,
            "expected ~0 distance to midpoint of diagonal bezier, got {}",
            d
        );
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
            origin: Point {
                x: Pixels(10.0),
                y: Pixels(20.0),
            },
            size: Size {
                width: Pixels(100.0),
                height: Pixels(80.0),
            },
        };
        // Point well inside the bounds.
        assert!(hit_test_bounds([60.0, 60.0], &bounds));
    }

    #[test]
    fn hit_test_bounds_outside() {
        use nom_gpui::types::{Bounds, Pixels, Point, Size};
        let bounds = Bounds {
            origin: Point {
                x: Pixels(10.0),
                y: Pixels(20.0),
            },
            size: Size {
                width: Pixels(100.0),
                height: Pixels(80.0),
            },
        };
        // Point to the right of the bounds.
        assert!(!hit_test_bounds([200.0, 60.0], &bounds));
    }

    #[test]
    fn hit_test_bounds_on_edge() {
        use nom_gpui::types::{Bounds, Pixels, Point, Size};
        let bounds = Bounds {
            origin: Point {
                x: Pixels(0.0),
                y: Pixels(0.0),
            },
            size: Size {
                width: Pixels(50.0),
                height: Pixels(50.0),
            },
        };
        // Exactly on the right edge — inclusive boundary.
        assert!(hit_test_bounds([50.0, 25.0], &bounds));
        // One pixel beyond the right edge — miss.
        assert!(!hit_test_bounds([51.0, 25.0], &bounds));
    }

    /// A rotated rect hit-tests correctly for a point inside it.
    #[test]
    fn hit_test_rotated_rect() {
        use std::f32::consts::FRAC_PI_4; // 45 degrees
        let r = CanvasRect {
            id: 50,
            // 100×100 box centred at (50,50).
            bounds: ([0.0, 0.0], [100.0, 100.0]),
            fill: None,
            stroke: None,
            corner_radius: 0.0,
            rotation: FRAC_PI_4,
            z_index: 0,
        };
        // The centre (50,50) is always inside regardless of rotation.
        assert!(
            hit_test_rect([50.0, 50.0], &r, 0.0),
            "centre must always hit"
        );
        // A point well outside the original AABB diagonally (beyond the rotated corners)
        // should miss when using exact hit (no threshold).
        assert!(
            !hit_test_rect([95.0, 5.0], &r, 0.0),
            "corner outside rotated rect must miss"
        );
    }

    /// A point just off a thin line does not hit with exact threshold.
    #[test]
    fn hit_test_thin_line_near_miss() {
        let l = CanvasLine {
            id: 11,
            start: [0.0, 0.0],
            end: [100.0, 0.0],
            stroke_width: 1.0,
            color: [1.0, 1.0, 1.0, 1.0],
            dashes: vec![],
            z_index: 0,
        };
        // Point exactly 6px above the line — just outside HIT_RADIUS (5.0).
        assert!(
            !hit_test_line([50.0, 6.0], &l, HIT_RADIUS),
            "6px away must miss with HIT_RADIUS=5"
        );
        // Point exactly 4px above — inside HIT_RADIUS.
        assert!(
            hit_test_line([50.0, 4.0], &l, HIT_RADIUS),
            "4px away must hit with HIT_RADIUS=5"
        );
    }

    /// Group hit if any child is hit: test by checking two overlapping rects,
    /// topmost by z_index is the group winner.
    #[test]
    fn hit_test_group_element() {
        // Simulate a group: two rects sharing the same area.
        let child1 = CanvasRect {
            id: 1,
            bounds: ([0.0, 0.0], [100.0, 100.0]),
            fill: None,
            stroke: None,
            corner_radius: 0.0,
            rotation: 0.0,
            z_index: 1,
        };
        let child2 = CanvasRect {
            id: 2,
            bounds: ([20.0, 20.0], [60.0, 60.0]),
            fill: None,
            stroke: None,
            corner_radius: 0.0,
            rotation: 0.0,
            z_index: 2,
        };
        let pt = [50.0, 50.0];
        // Both children are hit; the group is hit if any child is hit.
        let hit_any = hit_test_rect(pt, &child1, 0.0) || hit_test_rect(pt, &child2, 0.0);
        assert!(hit_any, "group must be hit when any child is hit");
    }

    /// Returns topmost (highest z_index) element when overlapping rects are tested.
    #[test]
    fn hit_test_z_order() {
        let rects = vec![
            CanvasRect {
                id: 10,
                bounds: ([0.0, 0.0], [100.0, 100.0]),
                fill: None,
                stroke: None,
                corner_radius: 0.0,
                rotation: 0.0,
                z_index: 1,
            },
            CanvasRect {
                id: 20,
                bounds: ([0.0, 0.0], [100.0, 100.0]),
                fill: None,
                stroke: None,
                corner_radius: 0.0,
                rotation: 0.0,
                z_index: 5,
            },
            CanvasRect {
                id: 30,
                bounds: ([0.0, 0.0], [100.0, 100.0]),
                fill: None,
                stroke: None,
                corner_radius: 0.0,
                rotation: 0.0,
                z_index: 3,
            },
        ];
        let pt = [50.0, 50.0];
        // Find the topmost hit (highest z_index).
        let topmost = rects
            .iter()
            .filter(|r| hit_test_rect(pt, r, 0.0))
            .max_by_key(|r| r.z_index);
        assert!(topmost.is_some(), "at least one rect must be hit");
        assert_eq!(
            topmost.unwrap().id,
            20,
            "topmost hit must be id=20 (z_index=5)"
        );
    }

    /// Rotated element: point outside the unrotated AABB but inside the rotated rect
    /// should NOT hit when using the inverse-rotation test.
    #[test]
    fn rotated_rect_outside_unrotated_aabb_hits() {
        use std::f32::consts::FRAC_PI_4;
        // A 60×20 rect centred at (0,0), rotated 45°.
        // After rotation its footprint extends diagonally.
        let r = CanvasRect {
            id: 99,
            bounds: ([-30.0, -10.0], [60.0, 20.0]),
            fill: None,
            stroke: None,
            corner_radius: 0.0,
            rotation: FRAC_PI_4,
            z_index: 0,
        };
        // Centre is always inside.
        assert!(hit_test_rect([0.0, 0.0], &r, 0.0), "centre must hit");
    }

    /// Three overlapping elements: hit picks the one with highest z_index.
    #[test]
    fn multiple_overlapping_highest_z_wins() {
        let elements = vec![
            CanvasRect {
                id: 1,
                bounds: ([0.0, 0.0], [80.0, 80.0]),
                fill: None,
                stroke: None,
                corner_radius: 0.0,
                rotation: 0.0,
                z_index: 10,
            },
            CanvasRect {
                id: 2,
                bounds: ([0.0, 0.0], [80.0, 80.0]),
                fill: None,
                stroke: None,
                corner_radius: 0.0,
                rotation: 0.0,
                z_index: 99,
            },
            CanvasRect {
                id: 3,
                bounds: ([0.0, 0.0], [80.0, 80.0]),
                fill: None,
                stroke: None,
                corner_radius: 0.0,
                rotation: 0.0,
                z_index: 50,
            },
        ];
        let pt = [40.0, 40.0];
        let winner = elements
            .iter()
            .filter(|r| hit_test_rect(pt, r, 0.0))
            .max_by_key(|r| r.z_index)
            .unwrap();
        assert_eq!(winner.id, 2, "element with z_index=99 must win");
    }

    /// A point exactly at the corner of a rect hits (inclusive boundary).
    #[test]
    fn hit_at_corner_is_inclusive() {
        let r = CanvasRect {
            id: 1,
            bounds: ([10.0, 10.0], [80.0, 60.0]),
            fill: None,
            stroke: None,
            corner_radius: 0.0,
            rotation: 0.0,
            z_index: 0,
        };
        // Top-left corner
        assert!(
            hit_test_rect([10.0, 10.0], &r, 0.0),
            "top-left corner must hit"
        );
        // Bottom-right corner
        assert!(
            hit_test_rect([90.0, 70.0], &r, 0.0),
            "bottom-right corner must hit"
        );
    }

    /// hit_test_circle: point exactly on the boundary (distance == r) hits.
    #[test]
    fn hit_test_circle_on_boundary() {
        // Circle at (0,0) radius 10; point at (10, 0) is exactly on boundary.
        assert!(
            hit_test_circle([10.0, 0.0], 0.0, 0.0, 10.0),
            "boundary must hit"
        );
        // Point just outside.
        assert!(
            !hit_test_circle([10.01, 0.0], 0.0, 0.0, 10.0),
            "just outside must miss"
        );
    }

    /// Connector with fewer than 4 points falls back to straight-line distance.
    #[test]
    fn connector_fallback_two_points() {
        let conn = CanvasConnector {
            id: 40,
            src_id: 1,
            dst_id: 2,
            route: vec![[0.0, 0.0], [100.0, 0.0]],
            confidence: 1.0,
            reason: String::new(),
            z_index: 0,
        };
        // Midpoint of the two-point line, 3px above.
        assert!(hit_test_connector([50.0, 3.0], &conn, HIT_RADIUS));
        // Far above — miss.
        assert!(!hit_test_connector([50.0, 20.0], &conn, HIT_RADIUS));
    }

    /// Connector with zero points returns false.
    #[test]
    fn connector_empty_route_returns_false() {
        let conn = CanvasConnector {
            id: 50,
            src_id: 1,
            dst_id: 2,
            route: vec![],
            confidence: 1.0,
            reason: String::new(),
            z_index: 0,
        };
        assert!(!hit_test_connector([0.0, 0.0], &conn, HIT_RADIUS));
    }

    /// hit_test_rect_aabb: point on left edge hits.
    #[test]
    fn hit_test_rect_aabb_on_left_edge() {
        assert!(hit_test_rect_aabb([0.0, 40.0], 0.0, 0.0, 100.0, 80.0));
    }

    /// dist_to_segment: point beyond the end of segment clamps to endpoint.
    #[test]
    fn dist_to_segment_beyond_end() {
        // Segment from (0,0) to (10,0); point at (20,0) → distance = 10.
        let d = dist_to_segment([20.0, 0.0], [0.0, 0.0], [10.0, 0.0]);
        assert!((d - 10.0).abs() < 1e-4, "got {}", d);
    }

    /// dist_to_segment: point before start clamps to start.
    #[test]
    fn dist_to_segment_before_start() {
        // Segment from (10,0) to (20,0); point at (0,0) → distance = 10.
        let d = dist_to_segment([0.0, 0.0], [10.0, 0.0], [20.0, 0.0]);
        assert!((d - 10.0).abs() < 1e-4, "got {}", d);
    }
}
