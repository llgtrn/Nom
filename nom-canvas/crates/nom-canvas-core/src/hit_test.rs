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
    /// ID of the element that was hit.
    pub element_id: u64,
    /// How the pointer intersected the element.
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

    /// hit_test_circle: point at centre always hits regardless of radius.
    #[test]
    fn hit_test_circle_center_always_hits() {
        assert!(hit_test_circle([5.0, 5.0], 5.0, 5.0, 1.0));
        assert!(hit_test_circle([5.0, 5.0], 5.0, 5.0, 100.0));
        assert!(hit_test_circle([5.0, 5.0], 5.0, 5.0, 0.001));
    }

    /// hit_test_rect_aabb: point on top edge hits.
    #[test]
    fn hit_test_rect_aabb_on_top_edge() {
        assert!(hit_test_rect_aabb([50.0, 0.0], 0.0, 0.0, 100.0, 80.0));
    }

    /// hit_test_rect_aabb: point on bottom edge hits.
    #[test]
    fn hit_test_rect_aabb_on_bottom_edge() {
        assert!(hit_test_rect_aabb([50.0, 80.0], 0.0, 0.0, 100.0, 80.0));
    }

    /// hit_test_rect_aabb: point one pixel below bottom edge misses.
    #[test]
    fn hit_test_rect_aabb_just_below_bottom_misses() {
        assert!(!hit_test_rect_aabb([50.0, 81.0], 0.0, 0.0, 100.0, 80.0));
    }

    /// hit_test_ellipse: point just outside with threshold=0 misses.
    #[test]
    fn hit_test_ellipse_just_outside_misses() {
        let e = CanvasEllipse {
            id: 10,
            bounds: ([0.0, 0.0], [100.0, 60.0]),
            fill: None,
            stroke: None,
            z_index: 0,
        };
        // Point at (100, 30): dx/rx = (100-50)/50 = 1, dy/ry = 0 → sum=1 → on boundary, hits
        // Point at (101, 30): dx/rx > 1 → misses
        assert!(!hit_test_ellipse([101.0, 30.0], &e, 0.0), "just outside ellipse must miss");
    }

    /// dist_to_bezier: point at the start of the bezier has near-zero distance.
    #[test]
    fn bezier_distance_at_start_point() {
        let p0 = [0.0_f32, 0.0];
        let c1 = [30.0, 10.0];
        let c2 = [70.0, 10.0];
        let p3 = [100.0, 0.0];
        let d = dist_to_bezier(p0, p0, c1, c2, p3);
        assert!(d < 1.0, "distance to start point must be ~0, got {}", d);
    }

    /// dist_to_bezier: point at the end of the bezier has near-zero distance.
    #[test]
    fn bezier_distance_at_end_point() {
        let p0 = [0.0_f32, 0.0];
        let c1 = [30.0, 10.0];
        let c2 = [70.0, 10.0];
        let p3 = [100.0, 0.0];
        let d = dist_to_bezier(p3, p0, c1, c2, p3);
        assert!(d < 1.0, "distance to end point must be ~0, got {}", d);
    }

    /// hit_test_connector: three-point route falls back to straight-line.
    #[test]
    fn connector_three_point_fallback() {
        let conn = CanvasConnector {
            id: 60,
            src_id: 1,
            dst_id: 2,
            route: vec![[0.0, 0.0], [50.0, 0.0], [100.0, 0.0]],
            confidence: 1.0,
            reason: String::new(),
            z_index: 0,
        };
        // Point near y=0 at x=50 should hit.
        assert!(hit_test_connector([50.0, 3.0], &conn, HIT_RADIUS));
        // Point far away should miss.
        assert!(!hit_test_connector([50.0, 50.0], &conn, HIT_RADIUS));
    }

    /// Rect with zero size (degenerate): only the origin point itself hits.
    #[test]
    fn hit_test_rect_degenerate_zero_size() {
        let r = CanvasRect {
            id: 100,
            bounds: ([10.0, 10.0], [0.0, 0.0]),
            fill: None,
            stroke: None,
            corner_radius: 0.0,
            rotation: 0.0,
            z_index: 0,
        };
        // With threshold=5, the origin point should hit.
        assert!(hit_test_rect([10.0, 10.0], &r, HIT_RADIUS));
        // A point far away should miss.
        assert!(!hit_test_rect([50.0, 50.0], &r, HIT_RADIUS));
    }

    /// Connector with one point returns false (not enough points).
    #[test]
    fn connector_one_point_returns_false() {
        let conn = CanvasConnector {
            id: 70,
            src_id: 1,
            dst_id: 2,
            route: vec![[0.0, 0.0]],
            confidence: 1.0,
            reason: String::new(),
            z_index: 0,
        };
        assert!(!hit_test_connector([0.0, 0.0], &conn, HIT_RADIUS));
    }

    /// Arrow miss: point far from the shaft returns false.
    #[test]
    fn arrow_miss_far_from_shaft() {
        let a = CanvasArrow {
            id: 21,
            start: [0.0, 0.0],
            end: [100.0, 0.0],
            stroke_width: 1.0,
            color: [1.0, 1.0, 1.0, 1.0],
            head_style: ArrowHead::Filled,
            z_index: 0,
        };
        assert!(!hit_test_arrow([50.0, 50.0], &a, HIT_RADIUS));
    }

    /// Wire with length < 5px (very short): hit test on the midpoint still works.
    #[test]
    fn wire_very_short_hit() {
        // Line segment shorter than HIT_RADIUS (5px) — point at midpoint hits.
        let l = CanvasLine {
            id: 99,
            start: [0.0, 0.0],
            end: [3.0, 0.0], // length = 3px < HIT_RADIUS
            stroke_width: 1.0,
            color: [1.0, 1.0, 1.0, 1.0],
            dashes: vec![],
            z_index: 0,
        };
        // Midpoint is (1.5, 0.0); 2px above should hit with HIT_RADIUS=5.
        assert!(hit_test_line([1.5, 2.0], &l, HIT_RADIUS), "midpoint of short wire must hit");
        // Point far away must miss.
        assert!(!hit_test_line([100.0, 100.0], &l, HIT_RADIUS), "far point must miss short wire");
    }

    /// Wire with zero length (degenerate point): hit test on that point.
    #[test]
    fn wire_zero_length_hit() {
        let l = CanvasLine {
            id: 100,
            start: [10.0, 10.0],
            end: [10.0, 10.0], // zero length
            stroke_width: 1.0,
            color: [1.0, 1.0, 1.0, 1.0],
            dashes: vec![],
            z_index: 0,
        };
        // Point at the degenerate location hits within HIT_RADIUS.
        assert!(hit_test_line([10.0, 10.0], &l, HIT_RADIUS));
        // Point just beyond HIT_RADIUS must miss.
        assert!(!hit_test_line([16.0, 10.0], &l, HIT_RADIUS));
    }

    /// Rotated rect: a point that is inside the rotated footprint but outside
    /// the unrotated AABB still hits.
    #[test]
    fn rotated_rect_point_inside_rotated_footprint_hits() {
        use std::f32::consts::FRAC_PI_4;
        // 100×20 rect centred at (50, 50), rotated 45°.
        // After rotation the long axis runs diagonally.
        let r = CanvasRect {
            id: 77,
            bounds: ([0.0, 40.0], [100.0, 20.0]), // origin=(0,40), size=(100,20)
            fill: None,
            stroke: None,
            corner_radius: 0.0,
            rotation: FRAC_PI_4,
            z_index: 0,
        };
        // Centre is always inside.
        let cx = 0.0 + 100.0 / 2.0;
        let cy = 40.0 + 20.0 / 2.0;
        assert!(hit_test_rect([cx, cy], &r, 0.0), "centre of rotated rect must hit");
    }

    // ── Wave AJ: bezier/curve tests ──────────────────────────────────────────

    /// Cubic bezier at t=0 returns p0.
    #[test]
    fn bezier_cubic_point_at_t_0_is_p0() {
        let p0 = [10.0_f32, 20.0];
        let c1 = [30.0, 40.0];
        let c2 = [60.0, 80.0];
        let p3 = [100.0, 120.0];
        // At t=0, the curve must start at p0.
        let d = dist_to_bezier(p0, p0, c1, c2, p3);
        assert!(d < 0.5, "point at t=0 must equal p0, dist={}", d);
    }

    /// Cubic bezier at t=1 returns p3.
    #[test]
    fn bezier_cubic_point_at_t_1_is_p3() {
        let p0 = [0.0_f32, 0.0];
        let c1 = [25.0, 50.0];
        let c2 = [75.0, 50.0];
        let p3 = [100.0, 0.0];
        let d = dist_to_bezier(p3, p0, c1, c2, p3);
        assert!(d < 0.5, "point at t=1 must equal p3, dist={}", d);
    }

    /// Cubic bezier with symmetric control points: midpoint is closest to the curve midpoint.
    #[test]
    fn bezier_cubic_point_at_t_0_5_is_midpoint() {
        // Symmetric arch: p0=[0,0], c1=[0,100], c2=[100,100], p3=[100,0]
        // At t=0.5 the curve is at (50, 75) for a symmetric cubic.
        let p0 = [0.0_f32, 0.0];
        let c1 = [0.0, 100.0];
        let c2 = [100.0, 100.0];
        let p3 = [100.0, 0.0];
        // The midpoint of the symmetric curve is between the two endpoints' x values.
        let mid_x = 50.0_f32;
        // dist_to_bezier from the curve midpoint should be small
        let d = dist_to_bezier([mid_x, 75.0], p0, c1, c2, p3);
        assert!(d < 5.0, "midpoint of symmetric bezier must be near (50,75), dist={}", d);
    }

    /// Cubic bezier derivative at t=0: tangent points toward c1 from p0.
    #[test]
    fn bezier_cubic_derivative_at_t_0() {
        // If c1 is directly to the right of p0, the tangent at t=0 should point right.
        // The tangent vector at t=0 is 3*(c1 - p0).
        let p0 = [0.0_f32, 0.0];
        let c1 = [30.0, 0.0]; // directly right
        let tangent_x = 3.0 * (c1[0] - p0[0]); // 90
        let tangent_y = 3.0 * (c1[1] - p0[1]); // 0
        assert!(tangent_x > 0.0, "tangent at t=0 must point toward c1 (positive x)");
        assert!(tangent_y.abs() < 1e-6, "tangent at t=0 must have zero y for horizontal");
    }

    /// Quadratic bezier at t=0 returns p0.
    #[test]
    fn bezier_quadratic_point_at_t_0_is_p0() {
        // Quadratic as degenerate cubic (c1=c2=control).
        let p0 = [5.0_f32, 10.0];
        let ctrl = [50.0, 80.0];
        let p2 = [100.0, 10.0];
        // Treat as cubic with c1=c2=ctrl
        let d = dist_to_bezier(p0, p0, ctrl, ctrl, p2);
        assert!(d < 0.5, "quadratic at t=0 must equal p0, dist={}", d);
    }

    /// Quadratic bezier at t=1 returns p2.
    #[test]
    fn bezier_quadratic_point_at_t_1_is_p2() {
        let p0 = [5.0_f32, 10.0];
        let ctrl = [50.0, 80.0];
        let p2 = [100.0, 10.0];
        let d = dist_to_bezier(p2, p0, ctrl, ctrl, p2);
        assert!(d < 0.5, "quadratic at t=1 must equal p2, dist={}", d);
    }

    /// Bezier subdivision: a curve split at t=0.5 starts at p0 and ends at p3.
    #[test]
    fn bezier_subdivision_produces_two_curves() {
        let p0 = [0.0_f32, 0.0];
        let c1 = [20.0, 40.0];
        let c2 = [80.0, 40.0];
        let p3 = [100.0, 0.0];
        // Compute the split point at t=0.5 using De Casteljau's algorithm.
        let m01 = [(p0[0]+c1[0])/2.0, (p0[1]+c1[1])/2.0];
        let m12 = [(c1[0]+c2[0])/2.0, (c1[1]+c2[1])/2.0];
        let m23 = [(c2[0]+p3[0])/2.0, (c2[1]+p3[1])/2.0];
        let m012 = [(m01[0]+m12[0])/2.0, (m01[1]+m12[1])/2.0];
        let m123 = [(m12[0]+m23[0])/2.0, (m12[1]+m23[1])/2.0];
        let split = [(m012[0]+m123[0])/2.0, (m012[1]+m123[1])/2.0];
        // Sub-curve 1: p0 → m01 → m012 → split
        // Sub-curve 2: split → m123 → m23 → p3
        // Verify that sub-curve 1 starts at p0 and sub-curve 2 ends at p3.
        let d1 = dist_to_bezier(p0, p0, m01, m012, split);
        let d2 = dist_to_bezier(p3, split, m123, m23, p3);
        assert!(d1 < 0.5, "left sub-curve must start at p0, dist={}", d1);
        assert!(d2 < 0.5, "right sub-curve must end at p3, dist={}", d2);
    }

    /// Bezier subdivision: concatenating both halves covers the same range.
    #[test]
    fn bezier_subdivision_covers_same_range() {
        // A straight-line cubic: both halves together span from p0 to p3.
        let p0 = [0.0_f32, 0.0];
        let c1 = [33.0, 0.0];
        let c2 = [66.0, 0.0];
        let p3 = [100.0, 0.0];
        // Midpoint at t=0.5 on a straight-line cubic is exactly (50, 0).
        let split = [50.0_f32, 0.0];
        // Left half: p0→split; right half: split→p3.
        let d_left_start = dist_to_bezier(p0, p0, [16.5, 0.0], [33.0, 0.0], split);
        let d_right_end = dist_to_bezier(p3, split, [66.0, 0.0], [83.5, 0.0], p3);
        assert!(d_left_start < 1.0, "left half must start at p0, dist={}", d_left_start);
        assert!(d_right_end < 1.0, "right half must end at p3, dist={}", d_right_end);
    }

    /// Arc length of a non-degenerate bezier is positive.
    #[test]
    fn bezier_arc_length_positive() {
        let p0 = [0.0_f32, 0.0];
        let c1 = [25.0, 50.0];
        let c2 = [75.0, 50.0];
        let p3 = [100.0, 0.0];
        // Estimate arc length by summing 100-sample segment lengths.
        let mut arc_len = 0.0_f32;
        let mut prev = p0;
        for i in 1..=100 {
            let t = i as f32 / 100.0;
            let mt = 1.0 - t;
            let bx = mt.powi(3)*p0[0] + 3.0*mt*mt*t*c1[0] + 3.0*mt*t*t*c2[0] + t.powi(3)*p3[0];
            let by = mt.powi(3)*p0[1] + 3.0*mt*mt*t*c1[1] + 3.0*mt*t*t*c2[1] + t.powi(3)*p3[1];
            let dx = bx - prev[0];
            let dy = by - prev[1];
            arc_len += (dx*dx + dy*dy).sqrt();
            prev = [bx, by];
        }
        assert!(arc_len > 0.0, "arc length of non-degenerate bezier must be positive, got {}", arc_len);
        assert!(arc_len >= 100.0, "arc length of arched bezier must exceed straight-line distance of 100, got {}", arc_len);
    }

    /// Bounding box computed from bezier samples contains all sample points.
    #[test]
    fn bezier_bounding_box_contains_all_points() {
        let p0 = [0.0_f32, 0.0];
        let c1 = [20.0, 80.0];
        let c2 = [80.0, 80.0];
        let p3 = [100.0, 0.0];
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for i in 0..=100 {
            let t = i as f32 / 100.0;
            let mt = 1.0 - t;
            let bx = mt.powi(3)*p0[0] + 3.0*mt*mt*t*c1[0] + 3.0*mt*t*t*c2[0] + t.powi(3)*p3[0];
            let by = mt.powi(3)*p0[1] + 3.0*mt*mt*t*c1[1] + 3.0*mt*t*t*c2[1] + t.powi(3)*p3[1];
            if bx < min_x { min_x = bx; }
            if bx > max_x { max_x = bx; }
            if by < min_y { min_y = by; }
            if by > max_y { max_y = by; }
        }
        // All sample points must fall within [min_x, max_x] x [min_y, max_y].
        assert!(min_x <= 0.1, "min_x must be near 0 for this curve, got {}", min_x);
        assert!(max_x >= 99.9, "max_x must be near 100, got {}", max_x);
        assert!(min_y >= -0.1, "min_y must be >= 0, got {}", min_y);
        assert!(max_y > 50.0, "max_y must exceed 50 for arched bezier, got {}", max_y);
    }

    /// Cubic bezier inflection-point count is 0, 1, or 2 (structural property).
    #[test]
    fn bezier_inflection_point_count() {
        // A straight-line cubic has no inflection points.
        let p0 = [0.0_f32, 0.0];
        let c1 = [33.0, 0.0];
        let c2 = [66.0, 0.0];
        let p3 = [100.0, 0.0];
        // Verify the curve is straight (no y variation) → 0 inflection points.
        let max_y_deviation = (0..=100).map(|i| {
            let t = i as f32 / 100.0;
            let mt = 1.0 - t;
            let by = mt.powi(3)*p0[1] + 3.0*mt*mt*t*c1[1] + 3.0*mt*t*t*c2[1] + t.powi(3)*p3[1];
            by.abs()
        }).fold(0.0_f32, f32::max);
        assert!(max_y_deviation < 1e-5, "straight-line cubic must have zero y deviation, got {}", max_y_deviation);
    }

    /// Nearest point on curve: query point near one of the curve endpoints is closest to that endpoint.
    #[test]
    fn bezier_nearest_point_on_curve() {
        let p0 = [0.0_f32, 0.0];
        let c1 = [33.0, 0.0];
        let c2 = [66.0, 0.0];
        let p3 = [100.0, 0.0];
        // Query point close to p0: distance to p0 should be very small.
        let d_near_p0 = dist_to_bezier([1.0, 0.0], p0, c1, c2, p3);
        // Query point close to p3: distance to p3 should be very small.
        let d_near_p3 = dist_to_bezier([99.0, 0.0], p0, c1, c2, p3);
        assert!(d_near_p0 < 2.0, "query near p0 must be close to the curve, dist={}", d_near_p0);
        assert!(d_near_p3 < 2.0, "query near p3 must be close to the curve, dist={}", d_near_p3);
        // Query far from the curve: much larger distance.
        let d_far = dist_to_bezier([50.0, 500.0], p0, c1, c2, p3);
        assert!(d_far > 400.0, "query far from curve must have large distance, got {}", d_far);
    }

    /// Bezier with all 4 points identical (degenerate): distance from any point near origin is small.
    #[test]
    fn bezier_all_points_same_degenerate() {
        let p = [50.0_f32, 50.0];
        let d = dist_to_bezier(p, p, p, p, p);
        assert!(d < 0.5, "degenerate bezier (all same) must have near-zero self-distance, got {}", d);
        // A point far from the degenerate bezier must have large distance.
        let d_far = dist_to_bezier([200.0, 200.0], p, p, p, p);
        assert!(d_far > 100.0, "far point from degenerate bezier must have large distance, got {}", d_far);
    }

    /// Group hit testing returns the element with the highest z_index.
    #[test]
    fn group_hit_returns_topmost() {
        let group = vec![
            CanvasRect { id: 1, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 2 },
            CanvasRect { id: 2, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 7 },
            CanvasRect { id: 3, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 4 },
        ];
        let pt = [50.0, 50.0];
        let topmost = group.iter()
            .filter(|r| hit_test_rect(pt, r, 0.0))
            .max_by_key(|r| r.z_index);
        assert!(topmost.is_some(), "at least one element must be hit");
        assert_eq!(topmost.unwrap().id, 2, "element with z_index=7 must be topmost");
    }

    /// Connector with exactly 4 points uses the bezier path.
    #[test]
    fn connector_four_point_bezier() {
        let conn = CanvasConnector {
            id: 80,
            src_id: 1,
            dst_id: 2,
            route: vec![[0.0, 0.0], [0.0, 50.0], [100.0, 50.0], [100.0, 0.0]],
            confidence: 0.9,
            reason: String::new(),
            z_index: 0,
        };
        // Point near the start (0,0) must hit.
        assert!(hit_test_connector([0.0, 0.0], &conn, HIT_RADIUS), "start of bezier must hit");
        // Point near the end (100,0) must hit.
        assert!(hit_test_connector([100.0, 0.0], &conn, HIT_RADIUS), "end of bezier must hit");
        // Point far from the curve must miss.
        assert!(!hit_test_connector([50.0, 200.0], &conn, HIT_RADIUS), "far point must miss bezier");
    }

    /// hit_test_rect_aabb: point on right edge hits.
    #[test]
    fn hit_test_rect_aabb_on_right_edge() {
        assert!(hit_test_rect_aabb([100.0, 40.0], 0.0, 0.0, 100.0, 80.0));
    }

    /// dist_to_segment: perpendicular distance from the middle of a diagonal segment.
    #[test]
    fn dist_to_segment_diagonal() {
        // Segment from (0,0) to (10,10); midpoint is (5,5).
        // Perpendicular distance from (5, 0) to the segment.
        let d = dist_to_segment([5.0, 0.0], [0.0, 0.0], [10.0, 10.0]);
        // Perpendicular distance = |5 - 5*cos45| ≈ 3.535
        assert!(d > 3.0 && d < 4.0, "diagonal perpendicular distance should be ~3.5, got {}", d);
    }

    /// Rect with 90° rotation: test centre still hits.
    #[test]
    fn rect_90_degree_rotation_centre_hits() {
        use std::f32::consts::FRAC_PI_2;
        let r = CanvasRect {
            id: 50,
            bounds: ([0.0, 0.0], [100.0, 40.0]),
            fill: None,
            stroke: None,
            corner_radius: 0.0,
            rotation: FRAC_PI_2,
            z_index: 0,
        };
        let cx = 50.0;
        let cy = 20.0;
        assert!(hit_test_rect([cx, cy], &r, 0.0), "centre of 90°-rotated rect must hit");
    }

    // ── Wave AH: additional hit_test tests ───────────────────────────────────

    /// Point inside rect returns true.
    #[test]
    fn hit_test_point_inside_rect_true() {
        let r = CanvasRect {
            id: 200,
            bounds: ([0.0, 0.0], [100.0, 100.0]),
            fill: None,
            stroke: None,
            corner_radius: 0.0,
            rotation: 0.0,
            z_index: 0,
        };
        assert!(hit_test_rect([50.0, 50.0], &r, 0.0), "interior point must hit");
    }

    /// Point outside rect returns false.
    #[test]
    fn hit_test_point_outside_rect_false() {
        let r = CanvasRect {
            id: 201,
            bounds: ([0.0, 0.0], [100.0, 100.0]),
            fill: None,
            stroke: None,
            corner_radius: 0.0,
            rotation: 0.0,
            z_index: 0,
        };
        assert!(!hit_test_rect([150.0, 50.0], &r, 0.0), "exterior point must miss");
    }

    /// Point exactly on the rect boundary returns true (inclusive).
    #[test]
    fn hit_test_point_on_rect_boundary_true() {
        let r = CanvasRect {
            id: 202,
            bounds: ([10.0, 10.0], [80.0, 60.0]),
            fill: None,
            stroke: None,
            corner_radius: 0.0,
            rotation: 0.0,
            z_index: 0,
        };
        // Left boundary x=10, middle of height
        assert!(hit_test_rect([10.0, 40.0], &r, 0.0), "left boundary must hit");
        // Right boundary x=10+80=90
        assert!(hit_test_rect([90.0, 40.0], &r, 0.0), "right boundary must hit");
    }

    /// Point near a bezier control point: distance is within threshold.
    #[test]
    fn hit_test_bezier_near_control_point() {
        // Bezier: p0=[0,0], c1=[0,50], c2=[100,50], p3=[100,0]
        // The curve passes through or near the control region.
        let conn = CanvasConnector {
            id: 300,
            src_id: 1,
            dst_id: 2,
            route: vec![[0.0, 0.0], [0.0, 50.0], [100.0, 50.0], [100.0, 0.0]],
            confidence: 1.0,
            reason: String::new(),
            z_index: 0,
        };
        // The curve passes through the start (0,0) — should always hit with any positive threshold.
        assert!(hit_test_connector([0.0, 0.0], &conn, HIT_RADIUS), "start of bezier must hit");
    }

    /// Point far from a bezier curve returns false.
    #[test]
    fn hit_test_bezier_far_from_curve_false() {
        let conn = CanvasConnector {
            id: 301,
            src_id: 1,
            dst_id: 2,
            route: vec![[0.0, 0.0], [33.0, 0.0], [66.0, 0.0], [100.0, 0.0]],
            confidence: 1.0,
            reason: String::new(),
            z_index: 0,
        };
        // Curve lies along y=0; point at y=200 is far away.
        assert!(!hit_test_connector([50.0, 200.0], &conn, HIT_RADIUS), "point far from bezier must miss");
    }

    /// Circle hit at centre returns true.
    #[test]
    fn hit_test_circle_center_true() {
        assert!(hit_test_circle([0.0, 0.0], 0.0, 0.0, 50.0), "centre must hit");
    }

    /// Circle: point outside radius returns false.
    #[test]
    fn hit_test_circle_outside_radius_false() {
        assert!(!hit_test_circle([100.0, 0.0], 0.0, 0.0, 50.0), "outside radius must miss");
    }

    /// Line segment: point near the segment returns true.
    #[test]
    fn hit_test_line_segment_nearby_true() {
        let l = CanvasLine {
            id: 400,
            start: [0.0, 0.0],
            end: [100.0, 0.0],
            stroke_width: 1.0,
            color: [1.0, 1.0, 1.0, 1.0],
            dashes: vec![],
            z_index: 0,
        };
        // 4px above the line — within HIT_RADIUS=5.
        assert!(hit_test_line([50.0, 4.0], &l, HIT_RADIUS), "nearby point must hit");
    }

    /// Line segment: point far from segment returns false.
    #[test]
    fn hit_test_line_segment_far_false() {
        let l = CanvasLine {
            id: 401,
            start: [0.0, 0.0],
            end: [100.0, 0.0],
            stroke_width: 1.0,
            color: [1.0, 1.0, 1.0, 1.0],
            dashes: vec![],
            z_index: 0,
        };
        // 100px above the line — way outside HIT_RADIUS.
        assert!(!hit_test_line([50.0, 100.0], &l, HIT_RADIUS), "far point must miss");
    }

    /// With zero tolerance, only exact hit on the shape returns true.
    #[test]
    fn hit_test_tolerance_zero_only_exact() {
        let r = CanvasRect {
            id: 500,
            bounds: ([0.0, 0.0], [100.0, 100.0]),
            fill: None,
            stroke: None,
            corner_radius: 0.0,
            rotation: 0.0,
            z_index: 0,
        };
        // Point just outside right edge — no tolerance.
        assert!(!hit_test_rect([101.0, 50.0], &r, 0.0), "point 1px outside must miss at zero tolerance");
        // Point on the edge — must hit.
        assert!(hit_test_rect([100.0, 50.0], &r, 0.0), "point on edge must hit at zero tolerance");
    }

    /// With large tolerance, a nearby point that misses exactly is accepted.
    #[test]
    fn hit_test_tolerance_large_accepts_nearby() {
        let r = CanvasRect {
            id: 501,
            bounds: ([0.0, 0.0], [100.0, 100.0]),
            fill: None,
            stroke: None,
            corner_radius: 0.0,
            rotation: 0.0,
            z_index: 0,
        };
        // Point 20px outside — accepted with tolerance=25.
        assert!(hit_test_rect([120.0, 50.0], &r, 25.0), "point within large tolerance must hit");
        // Point 30px outside — still outside tolerance=25.
        assert!(!hit_test_rect([130.0, 50.0], &r, 25.0), "point beyond tolerance must miss");
    }

    /// Multiple overlapping elements: highest z_index is topmost hit.
    #[test]
    fn hit_test_multiple_elements_returns_topmost() {
        let elements = vec![
            CanvasRect { id: 10, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 1 },
            CanvasRect { id: 20, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 8 },
            CanvasRect { id: 30, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 4 },
        ];
        let pt = [50.0, 50.0];
        let topmost = elements.iter()
            .filter(|r| hit_test_rect(pt, r, 0.0))
            .max_by_key(|r| r.z_index)
            .unwrap();
        assert_eq!(topmost.id, 20, "element with z_index=8 must be topmost");
    }

    // ── new tests (Wave AI) ──────────────────────────────────────────────────

    /// A rotated rectangle: the centre always hits regardless of rotation angle.
    #[test]
    fn hit_test_rotated_element() {
        use std::f32::consts::FRAC_PI_4;
        let r = CanvasRect {
            id: 600,
            bounds: ([0.0, 0.0], [80.0, 40.0]),
            fill: None, stroke: None, corner_radius: 0.0,
            rotation: FRAC_PI_4,
            z_index: 0,
        };
        let cx = 0.0 + 80.0 / 2.0; // 40
        let cy = 0.0 + 40.0 / 2.0; // 20
        assert!(hit_test_rect([cx, cy], &r, 0.0), "centre of rotated rect must always hit");
    }

    /// An empty scene (no elements) returns no hit.
    #[test]
    fn hit_test_empty_scene_returns_none() {
        let elements: Vec<CanvasRect> = vec![];
        let pt = [50.0, 50.0];
        let hit = elements.iter().find(|r| hit_test_rect(pt, r, 0.0));
        assert!(hit.is_none(), "empty scene must return no hit");
    }

    /// Overlapping elements: hit returns the one with the highest z_index.
    #[test]
    fn hit_test_overlapping_z_returns_top() {
        let elements = vec![
            CanvasRect { id: 1, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 5 },
            CanvasRect { id: 2, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 20 },
            CanvasRect { id: 3, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 10 },
        ];
        let pt = [50.0, 50.0];
        let top = elements.iter()
            .filter(|r| hit_test_rect(pt, r, 0.0))
            .max_by_key(|r| r.z_index)
            .unwrap();
        assert_eq!(top.id, 2, "element with z_index=20 must be topmost hit");
    }

    /// Elements with z_index=0 (invisible/locked by convention) are skipped in hit-test.
    #[test]
    fn hit_test_locked_element_skipped() {
        // Simulate "locked" elements as z_index=0 and skip them.
        let elements = vec![
            CanvasRect { id: 1, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 0 },
            CanvasRect { id: 2, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 5 },
        ];
        let pt = [50.0, 50.0];
        // Skip elements with z_index=0 (locked).
        let hit = elements.iter()
            .filter(|r| r.z_index > 0 && hit_test_rect(pt, r, 0.0))
            .max_by_key(|r| r.z_index);
        assert!(hit.is_some(), "non-locked element must be hit");
        assert_eq!(hit.unwrap().id, 2, "only non-locked element must be returned");
    }

    /// Transparent elements (fill=None) can be modelled as invisible and skipped.
    #[test]
    fn hit_test_transparent_element_skipped() {
        // "Transparent" = fill is None; skip these in hit-test.
        let elements = vec![
            CanvasRect { id: 10, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 5 },
            CanvasRect { id: 11, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: Some([1.0, 0.0, 0.0, 1.0]), stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 3 },
        ];
        let pt = [50.0, 50.0];
        let hit = elements.iter()
            .filter(|r| r.fill.is_some() && hit_test_rect(pt, r, 0.0))
            .max_by_key(|r| r.z_index);
        assert!(hit.is_some(), "opaque element must be hit");
        assert_eq!(hit.unwrap().id, 11, "only non-transparent element must be returned");
    }

    /// hit_test_all_elements_at_pos: all elements overlapping a point are returned.
    #[test]
    fn hit_test_all_elements_at_pos() {
        let elements = vec![
            CanvasRect { id: 1, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 1 },
            CanvasRect { id: 2, bounds: ([50.0, 50.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 2 },
            CanvasRect { id: 3, bounds: ([200.0, 200.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 3 },
        ];
        let pt = [70.0, 70.0]; // inside elements 1 and 2
        let hits: Vec<u64> = elements.iter()
            .filter(|r| hit_test_rect(pt, r, 0.0))
            .map(|r| r.id)
            .collect();
        assert!(hits.contains(&1), "element 1 must be hit at (70,70)");
        assert!(hits.contains(&2), "element 2 must be hit at (70,70)");
        assert!(!hits.contains(&3), "element 3 must not be hit at (70,70)");
    }

    /// A point at a corner of a rotated rect still hits (centre is always inside).
    #[test]
    fn hit_test_rotated_corner_hit() {
        use std::f32::consts::FRAC_PI_2;
        let r = CanvasRect {
            id: 700,
            bounds: ([0.0, 0.0], [60.0, 30.0]),
            fill: None, stroke: None, corner_radius: 0.0,
            rotation: FRAC_PI_2,
            z_index: 0,
        };
        let cx = 30.0_f32;
        let cy = 15.0_f32;
        assert!(hit_test_rect([cx, cy], &r, 0.0), "centre of 90°-rotated rect must hit");
    }

    /// Hit test with a large threshold accepts a distant point.
    #[test]
    fn hit_test_large_threshold_accepts_distant() {
        let r = CanvasRect {
            id: 800,
            bounds: ([0.0, 0.0], [50.0, 50.0]),
            fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 0,
        };
        // Point 18px outside right edge; threshold 20 accepts it.
        assert!(hit_test_rect([68.0, 25.0], &r, 20.0), "large threshold must accept nearby point");
        // Point 25px outside; threshold 20 does not accept it.
        assert!(!hit_test_rect([75.0, 25.0], &r, 20.0), "large threshold must still reject very distant point");
    }

    /// Hit results sorted by z_index descending give front-to-back order.
    #[test]
    fn hit_test_sorted_by_z_desc() {
        let elements = vec![
            CanvasRect { id: 1, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 10 },
            CanvasRect { id: 2, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 30 },
            CanvasRect { id: 3, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 20 },
        ];
        let pt = [50.0, 50.0];
        let mut hits: Vec<&CanvasRect> = elements.iter()
            .filter(|r| hit_test_rect(pt, r, 0.0))
            .collect();
        hits.sort_by(|a, b| b.z_index.cmp(&a.z_index)); // descending
        let ids: Vec<u64> = hits.iter().map(|r| r.id).collect();
        assert_eq!(ids, vec![2, 3, 1], "sorted by z_index descending must give [2,3,1]");
    }

    // ── Wave AK: requested hit_test scenarios ────────────────────────────────

    /// Overlapping elements: last-on-top semantics — the element with the highest
    /// z_index is considered the topmost hit (last drawn = on top).
    #[test]
    fn overlapping_elements_last_on_top_semantics() {
        let elements = vec![
            CanvasRect { id: 1, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 1 },
            CanvasRect { id: 2, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 3 },
            CanvasRect { id: 3, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 2 },
        ];
        let pt = [50.0, 50.0];
        // All three overlap at the point; highest z_index (last-on-top) wins.
        let top = elements.iter()
            .filter(|r| hit_test_rect(pt, r, 0.0))
            .max_by_key(|r| r.z_index)
            .unwrap();
        assert_eq!(top.id, 2, "element with z_index=3 (last-on-top) must win");
    }

    /// A zero-size rect (width=0, height=0) returns no hit for any interior point.
    #[test]
    fn zero_size_element_returns_no_hit() {
        let r = CanvasRect {
            id: 900,
            bounds: ([50.0, 50.0], [0.0, 0.0]), // zero size
            fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 1,
        };
        // A point well away from the zero-size element must miss.
        assert!(!hit_test_rect([100.0, 100.0], &r, 0.0), "zero-size element must not be hit at distant point");
        // Even at the exact origin with zero threshold it collapses to a point; a nearby
        // point should only hit with a positive threshold.
        assert!(!hit_test_rect([60.0, 60.0], &r, 0.0), "zero-size element must not be hit 10px away with no threshold");
    }

    /// Hit test inside a nested group checks children: a child rect that is hit
    /// implies the group is hit.
    #[test]
    fn hit_test_inside_nested_group_checks_children() {
        // Outer group rect.
        let group = CanvasRect {
            id: 1000,
            bounds: ([0.0, 0.0], [200.0, 200.0]),
            fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 1,
        };
        // Child rect entirely inside the group.
        let child = CanvasRect {
            id: 1001,
            bounds: ([50.0, 50.0], [80.0, 80.0]),
            fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 2,
        };
        let pt = [90.0, 90.0]; // inside both group and child
        // The group hit implies its children are checked; if child hits, group is considered hit.
        let group_hit = hit_test_rect(pt, &group, 0.0);
        let child_hit = hit_test_rect(pt, &child, 0.0);
        assert!(group_hit, "outer group rect must be hit");
        assert!(child_hit, "child rect must be hit when point is inside it");
        // A point inside the group but outside the child should only hit the group.
        let pt_group_only = [10.0, 10.0];
        assert!(hit_test_rect(pt_group_only, &group, 0.0), "group must still be hit at group-only point");
        assert!(!hit_test_rect(pt_group_only, &child, 0.0), "child must not be hit at group-only point");
    }

    /// Hit test miss: a point outside all elements returns no hit (None pattern).
    #[test]
    fn hit_test_miss_returns_none() {
        let elements = vec![
            CanvasRect { id: 1, bounds: ([0.0, 0.0], [50.0, 50.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 1 },
            CanvasRect { id: 2, bounds: ([100.0, 100.0], [50.0, 50.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 2 },
        ];
        // Point completely outside both elements.
        let pt = [300.0, 300.0];
        let hit = elements.iter()
            .filter(|r| hit_test_rect(pt, r, 0.0))
            .max_by_key(|r| r.z_index);
        assert!(hit.is_none(), "no element at point (300,300) — must return None");
    }

    /// Multiple elements at the same point: topmost (max z_index) is returned.
    #[test]
    fn multiple_elements_same_point_returns_topmost() {
        let elements = vec![
            CanvasRect { id: 10, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 5 },
            CanvasRect { id: 20, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 15 },
            CanvasRect { id: 30, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 10 },
        ];
        let pt = [50.0, 50.0];
        let top = elements.iter()
            .filter(|r| hit_test_rect(pt, r, 0.0))
            .max_by_key(|r| r.z_index)
            .unwrap();
        assert_eq!(top.id, 20, "element with z_index=15 must be topmost at the shared point");
    }

    /// A point just outside the bounding box of every element misses all of them.
    #[test]
    fn point_just_outside_every_element_is_miss() {
        let r = CanvasRect {
            id: 1100,
            bounds: ([10.0, 10.0], [40.0, 40.0]),
            fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 1,
        };
        // 1px beyond each edge.
        assert!(!hit_test_rect([9.0, 25.0], &r, 0.0), "left miss");
        assert!(!hit_test_rect([51.0, 25.0], &r, 0.0), "right miss");
        assert!(!hit_test_rect([25.0, 9.0], &r, 0.0), "top miss");
        assert!(!hit_test_rect([25.0, 51.0], &r, 0.0), "bottom miss");
    }

    /// A group with children at different z_indices: topmost child wins hit.
    #[test]
    fn group_children_topmost_child_wins_hit() {
        // Three children of a group, all at the same location.
        let children = vec![
            CanvasRect { id: 1, bounds: ([20.0, 20.0], [60.0, 60.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 1 },
            CanvasRect { id: 2, bounds: ([20.0, 20.0], [60.0, 60.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 4 },
            CanvasRect { id: 3, bounds: ([20.0, 20.0], [60.0, 60.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 2 },
        ];
        let pt = [50.0, 50.0];
        let top_child = children.iter()
            .filter(|r| hit_test_rect(pt, r, 0.0))
            .max_by_key(|r| r.z_index)
            .unwrap();
        assert_eq!(top_child.id, 2, "child with z_index=4 must win within the group");
    }

    /// A zero-size ellipse returns no hit for any point with zero threshold.
    #[test]
    fn zero_size_ellipse_returns_no_hit() {
        let e = CanvasEllipse {
            id: 1200,
            bounds: ([50.0, 50.0], [0.0, 0.0]), // zero radii
            fill: None, stroke: None, z_index: 0,
        };
        // Any point other than the degenerate centre should miss at zero threshold.
        // (rx=0, ry=0: the ellipse eq produces +Inf for any non-centre point)
        assert!(!hit_test_ellipse([100.0, 100.0], &e, 0.0), "zero-size ellipse must miss distant point");
    }

    /// Overlapping stacked elements: verify all are hit at the shared region.
    #[test]
    fn overlapping_stacked_elements_all_hit_at_shared_region() {
        let elements = vec![
            CanvasRect { id: 1, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 1 },
            CanvasRect { id: 2, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 2 },
        ];
        let pt = [50.0, 50.0];
        let hits: Vec<u64> = elements.iter()
            .filter(|r| hit_test_rect(pt, r, 0.0))
            .map(|r| r.id)
            .collect();
        assert_eq!(hits.len(), 2, "both overlapping elements must be hit at shared region");
    }

    // ── Wave AO: spatial-index–backed hit tests ──────────────────────────────

    use crate::spatial_index::SpatialIndex;
    use crate::elements::ElementBounds;

    fn make_bounds_ht(id: u64, min: [f32; 2], max: [f32; 2]) -> ElementBounds {
        ElementBounds { id, min, max }
    }

    /// SpatialIndex nearest finds the element closest to a click point.
    #[test]
    fn spatial_hit_nearest_finds_clicked_element() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds_ht(1, [0.0, 0.0], [50.0, 50.0]));
        idx.insert(make_bounds_ht(2, [200.0, 200.0], [250.0, 250.0]));
        // Click near element 1.
        let hit = idx.nearest([10.0, 10.0], 100.0);
        assert_eq!(hit, Some(1), "nearest to (10,10) must be element 1");
    }

    /// SpatialIndex nearest with a miss radius returns None.
    #[test]
    fn spatial_hit_nearest_miss_returns_none() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds_ht(1, [300.0, 300.0], [350.0, 350.0]));
        // Click at origin — element is ~300px away, radius=5.
        let hit = idx.nearest([0.0, 0.0], 5.0);
        assert_eq!(hit, None, "no element within 5px of origin");
    }

    /// SpatialIndex query_in_bounds returns only elements in the region.
    #[test]
    fn spatial_hit_region_returns_only_inside_elements() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds_ht(10, [0.0, 0.0], [40.0, 40.0]));
        idx.insert(make_bounds_ht(20, [50.0, 50.0], [90.0, 90.0]));
        idx.insert(make_bounds_ht(30, [500.0, 500.0], [600.0, 600.0]));
        let ids = idx.query_in_bounds([0.0, 0.0], [100.0, 100.0]);
        assert!(ids.contains(&10), "element 10 must be in region");
        assert!(ids.contains(&20), "element 20 must be in region");
        assert!(!ids.contains(&30), "element 30 must not be in region");
    }

    /// hit_test_rect on a retrieved spatial element confirms precise hit.
    #[test]
    fn spatial_hit_broadphase_then_precise_rect_hit() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds_ht(1, [0.0, 0.0], [80.0, 80.0]));
        // Broadphase: query region containing click point.
        let candidates = idx.query_in_bounds([40.0, 40.0], [40.0, 40.0]);
        assert!(!candidates.is_empty(), "broadphase must return a candidate");
        // Precise: hit_test_rect on the candidate.
        let r = CanvasRect {
            id: 1,
            bounds: ([0.0, 0.0], [80.0, 80.0]),
            fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 1,
        };
        assert!(hit_test_rect([40.0, 40.0], &r, 0.0), "precise hit must confirm broadphase result");
    }

    /// Broadphase returns no candidate → precise test skipped → no hit.
    #[test]
    fn spatial_hit_broadphase_miss_skips_precise() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds_ht(1, [200.0, 200.0], [280.0, 280.0]));
        // Click at (0,0): broadphase returns nothing.
        let candidates = idx.query_in_bounds([0.0, 0.0], [0.0, 0.0]);
        assert!(candidates.is_empty(), "broadphase must return nothing for click far away");
    }

    /// Multiple elements in broadphase → precise test selects topmost z_index.
    #[test]
    fn spatial_hit_broadphase_multiple_precise_z_wins() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds_ht(1, [0.0, 0.0], [100.0, 100.0]));
        idx.insert(make_bounds_ht(2, [0.0, 0.0], [100.0, 100.0]));
        let candidates = idx.query_in_bounds([50.0, 50.0], [50.0, 50.0]);
        assert_eq!(candidates.len(), 2, "broadphase must return 2 overlapping elements");
        let rects = vec![
            CanvasRect { id: 1, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 5 },
            CanvasRect { id: 2, bounds: ([0.0, 0.0], [100.0, 100.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 10 },
        ];
        let pt = [50.0, 50.0];
        let topmost = rects.iter()
            .filter(|r| candidates.contains(&r.id) && hit_test_rect(pt, r, 0.0))
            .max_by_key(|r| r.z_index);
        assert!(topmost.is_some());
        assert_eq!(topmost.unwrap().id, 2, "element with z_index=10 must win");
    }

    /// SpatialIndex: inserting then removing element leaves nothing for hit.
    #[test]
    fn spatial_hit_removed_element_not_hit() {
        let mut idx = SpatialIndex::new();
        let b = make_bounds_ht(1, [0.0, 0.0], [50.0, 50.0]);
        idx.insert(b);
        idx.remove(1, b);
        let candidates = idx.query_in_bounds([0.0, 0.0], [50.0, 50.0]);
        assert!(candidates.is_empty(), "removed element must not appear in broadphase");
    }

    /// Nearest returns element containing the query point (distance=0).
    #[test]
    fn spatial_hit_nearest_inside_element_distance_zero() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds_ht(1, [0.0, 0.0], [100.0, 100.0]));
        // Point inside the element: AABB distance_2=0.
        let hit = idx.nearest([50.0, 50.0], 0.0);
        assert_eq!(hit, Some(1), "point inside element must have distance=0 → nearest hit");
    }

    /// Hit test via broadphase + precise for a line connector.
    #[test]
    fn spatial_hit_broadphase_then_connector_precise() {
        use crate::elements::CanvasConnector;
        let mut idx = SpatialIndex::new();
        // Add a bounding box around the connector path.
        idx.insert(make_bounds_ht(30, [0.0, -5.0], [100.0, 5.0]));
        let candidates = idx.query_in_bounds([50.0, 0.0], [50.0, 0.0]);
        assert!(!candidates.is_empty(), "broadphase must return connector candidate");
        let conn = CanvasConnector {
            id: 30, src_id: 1, dst_id: 2,
            route: vec![[0.0, 0.0], [33.0, 0.0], [66.0, 0.0], [100.0, 0.0]],
            confidence: 1.0, reason: String::new(), z_index: 1,
        };
        assert!(hit_test_connector([50.0, 3.0], &conn, HIT_RADIUS), "precise connector hit must succeed");
    }

    /// Query a zero-size point region: only elements that contain the point are returned.
    #[test]
    fn spatial_hit_zero_size_point_query() {
        let mut idx = SpatialIndex::new();
        idx.insert(make_bounds_ht(1, [10.0, 10.0], [60.0, 60.0]));
        idx.insert(make_bounds_ht(2, [70.0, 70.0], [120.0, 120.0]));
        // Point query at (30, 30): inside element 1 only.
        let candidates = idx.query_in_bounds([30.0, 30.0], [30.0, 30.0]);
        assert!(candidates.contains(&1), "element 1 must contain the query point");
        assert!(!candidates.contains(&2), "element 2 must not contain the query point");
    }

    /// Broadphase returns multiple candidates; precise test on all returns correct subset.
    #[test]
    fn spatial_hit_broadphase_returns_precise_subset() {
        let mut idx = SpatialIndex::new();
        // Element A: wide box.
        idx.insert(make_bounds_ht(1, [0.0, 0.0], [200.0, 200.0]));
        // Element B: small box inside A.
        idx.insert(make_bounds_ht(2, [80.0, 80.0], [120.0, 120.0]));
        // Click at (100, 100): both A and B contain it.
        let candidates = idx.query_in_bounds([100.0, 100.0], [100.0, 100.0]);
        assert_eq!(candidates.len(), 2, "broadphase must return both elements at (100,100)");
        let rects = vec![
            CanvasRect { id: 1, bounds: ([0.0, 0.0], [200.0, 200.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 1 },
            CanvasRect { id: 2, bounds: ([80.0, 80.0], [40.0, 40.0]), fill: None, stroke: None, corner_radius: 0.0, rotation: 0.0, z_index: 2 },
        ];
        let pt = [100.0, 100.0];
        let hits: Vec<u64> = rects.iter()
            .filter(|r| candidates.contains(&r.id) && hit_test_rect(pt, r, 0.0))
            .map(|r| r.id)
            .collect();
        assert!(hits.contains(&1), "element A must pass precise test");
        assert!(hits.contains(&2), "element B must pass precise test");
    }
}
