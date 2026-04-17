//! Grid + alignment-guide snapping.
#![deny(unsafe_code)]

use nom_gpui::{Bounds, Pixels, Point};
use smallvec::SmallVec;

/// Configuration for grid snapping.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridConfig {
    /// Grid cell size in logical pixels.
    pub size_px: f32,
    /// Whether grid snapping is active.
    pub enabled: bool,
}

impl Default for GridConfig {
    fn default() -> Self {
        GridConfig { size_px: 10.0, enabled: false }
    }
}

/// Snap a point to the nearest grid intersection.
///
/// Returns the point unchanged when `grid.enabled` is false.
pub fn snap_to_grid(point: Point<Pixels>, grid: &GridConfig) -> Point<Pixels> {
    if !grid.enabled || grid.size_px <= 0.0 {
        return point;
    }
    let s = grid.size_px;
    let x = (point.x.0 / s).round() * s;
    let y = (point.y.0 / s).round() * s;
    Point::new(Pixels(x), Pixels(y))
}

/// Axis for an alignment guide line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GuideAxis {
    /// A vertical guide line (constant x position).
    Vertical,
    /// A horizontal guide line (constant y position).
    Horizontal,
}

/// Semantic kind of an alignment guide.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GuideKind {
    /// Snapped to the edge of a reference element.
    Edge,
    /// Snapped to the centre of a reference element.
    Center,
    /// Snapped to the midpoint between two reference elements.
    Midpoint,
    /// Elements are equally spaced along the axis.
    EqualSpacing,
}

/// A single alignment guide that should be rendered as a line on the canvas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AlignmentGuide {
    /// Which axis this guide runs along.
    pub axis: GuideAxis,
    /// Why this guide was emitted.
    pub kind: GuideKind,
    /// Position of the guide line in canvas pixels.
    pub position_px: f32,
    /// Start of the visual extent along the perpendicular axis.
    pub from_px: f32,
    /// End of the visual extent along the perpendicular axis.
    pub to_px: f32,
}

/// Compute alignment guides for a moving element against a set of reference elements.
///
/// The snap threshold scales with zoom: `threshold_px = 8.0 / zoom`.
/// Returns up to 8 guides and a snapped offset `(dx, dy)` that should be applied
/// to the moving element's position.
///
/// When `viewport_center` is supplied a centre-of-viewport guide is also considered.
pub fn compute_guides(
    moving: Bounds<Pixels>,
    refs: &[Bounds<Pixels>],
    zoom: f32,
    viewport_center: Option<Point<Pixels>>,
) -> (SmallVec<[AlignmentGuide; 8]>, (f32, f32)) {
    let threshold = 8.0 / zoom.max(f32::EPSILON);
    let mut guides: SmallVec<[AlignmentGuide; 8]> = SmallVec::new();
    let mut snap_dx = 0.0f32;
    let mut snap_dy = 0.0f32;

    // Precompute edges of the moving element.
    let m_left = moving.origin.x.0;
    let m_right = m_left + moving.size.width.0;
    let m_top = moving.origin.y.0;
    let m_bottom = m_top + moving.size.height.0;
    let m_cx = (m_left + m_right) * 0.5;
    let m_cy = (m_top + m_bottom) * 0.5;

    // Helper: try to emit a vertical guide when two x values are within threshold.
    let mut best_dx_dist = threshold;
    let mut best_dy_dist = threshold;

    for r in refs {
        let r_left = r.origin.x.0;
        let r_right = r_left + r.size.width.0;
        let r_top = r.origin.y.0;
        let r_bottom = r_top + r.size.height.0;
        let r_cx = (r_left + r_right) * 0.5;
        let r_cy = (r_top + r_bottom) * 0.5;

        // Vertical guides (snap along x-axis).
        let x_candidates: [(f32, f32, GuideKind); 5] = [
            (m_left, r_left, GuideKind::Edge),
            (m_right, r_right, GuideKind::Edge),
            (m_left, r_right, GuideKind::Edge),
            (m_right, r_left, GuideKind::Edge),
            (m_cx, r_cx, GuideKind::Center),
        ];
        for (m_val, r_val, kind) in x_candidates {
            let diff = r_val - m_val;
            if diff.abs() < best_dx_dist {
                best_dx_dist = diff.abs();
                snap_dx = diff;
                let from_px = m_top.min(r_top);
                let to_px = m_bottom.max(r_bottom);
                // Replace any existing vertical guide if this is closer.
                guides.retain(|g| g.axis != GuideAxis::Vertical);
                guides.push(AlignmentGuide {
                    axis: GuideAxis::Vertical,
                    kind,
                    position_px: r_val,
                    from_px,
                    to_px,
                });
            }
        }

        // Horizontal guides (snap along y-axis).
        let y_candidates: [(f32, f32, GuideKind); 5] = [
            (m_top, r_top, GuideKind::Edge),
            (m_bottom, r_bottom, GuideKind::Edge),
            (m_top, r_bottom, GuideKind::Edge),
            (m_bottom, r_top, GuideKind::Edge),
            (m_cy, r_cy, GuideKind::Center),
        ];
        for (m_val, r_val, kind) in y_candidates {
            let diff = r_val - m_val;
            if diff.abs() < best_dy_dist {
                best_dy_dist = diff.abs();
                snap_dy = diff;
                let from_px = m_left.min(r_left);
                let to_px = m_right.max(r_right);
                guides.retain(|g| g.axis != GuideAxis::Horizontal);
                guides.push(AlignmentGuide {
                    axis: GuideAxis::Horizontal,
                    kind,
                    position_px: r_val,
                    from_px,
                    to_px,
                });
            }
        }
    }

    // Viewport centre guide.
    if let Some(vc) = viewport_center {
        let vc_x = vc.x.0;
        let vc_y = vc.y.0;
        let dx = vc_x - m_cx;
        if dx.abs() < best_dx_dist {
            snap_dx = dx;
            guides.retain(|g| g.axis != GuideAxis::Vertical);
            guides.push(AlignmentGuide {
                axis: GuideAxis::Vertical,
                kind: GuideKind::Center,
                position_px: vc_x,
                from_px: m_top,
                to_px: m_bottom,
            });
        }
        let dy = vc_y - m_cy;
        if dy.abs() < best_dy_dist {
            guides.retain(|g| g.axis != GuideAxis::Horizontal);
            guides.push(AlignmentGuide {
                axis: GuideAxis::Horizontal,
                kind: GuideKind::Center,
                position_px: vc_y,
                from_px: m_left,
                to_px: m_right,
            });
        }
    }

    (guides, (snap_dx, snap_dy))
}

/// Detect whether three or more bounding boxes along `axis` are equally spaced
/// (centre-to-centre).
///
/// Returns the spacing value when detected; `None` otherwise. `tolerance_px`
/// controls how much variation is allowed between gaps.
pub fn detect_equal_spacing(
    bounds: &[Bounds<Pixels>],
    axis: GuideAxis,
    tolerance_px: f32,
) -> Option<f32> {
    if bounds.len() < 3 {
        return None;
    }

    // Extract sorted centre positions along the relevant axis.
    let mut centres: Vec<f32> = bounds
        .iter()
        .map(|b| match axis {
            GuideAxis::Horizontal => b.origin.x.0 + b.size.width.0 * 0.5,
            GuideAxis::Vertical => b.origin.y.0 + b.size.height.0 * 0.5,
        })
        .collect();
    centres.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let first_gap = centres[1] - centres[0];
    for i in 2..centres.len() {
        let gap = centres[i] - centres[i - 1];
        if (gap - first_gap).abs() > tolerance_px {
            return None;
        }
    }
    Some(first_gap)
}

// ─── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use nom_gpui::{Pixels, Point, Size};

    fn bounds(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
        Bounds::new(
            Point::new(Pixels(x), Pixels(y)),
            Size::new(Pixels(w), Pixels(h)),
        )
    }

    // Grid snap tests.

    #[test]
    fn snap_disabled_returns_unchanged() {
        let grid = GridConfig { size_px: 10.0, enabled: false };
        let p = Point::new(Pixels(3.7), Pixels(9.2));
        assert_eq!(snap_to_grid(p, &grid), p);
    }

    #[test]
    fn snap_4_rounds_to_0() {
        let grid = GridConfig { size_px: 10.0, enabled: true };
        let p = Point::new(Pixels(4.0), Pixels(0.0));
        let snapped = snap_to_grid(p, &grid);
        assert!((snapped.x.0).abs() < f32::EPSILON, "4 should round to 0 on 10px grid");
    }

    #[test]
    fn snap_6_rounds_to_10() {
        let grid = GridConfig { size_px: 10.0, enabled: true };
        let p = Point::new(Pixels(6.0), Pixels(0.0));
        let snapped = snap_to_grid(p, &grid);
        assert!((snapped.x.0 - 10.0).abs() < f32::EPSILON, "6 should round to 10 on 10px grid");
    }

    #[test]
    fn snap_exactly_on_grid_unchanged() {
        let grid = GridConfig { size_px: 10.0, enabled: true };
        let p = Point::new(Pixels(30.0), Pixels(50.0));
        let snapped = snap_to_grid(p, &grid);
        assert!((snapped.x.0 - 30.0).abs() < f32::EPSILON);
        assert!((snapped.y.0 - 50.0).abs() < f32::EPSILON);
    }

    // Guide tests.

    #[test]
    fn guide_edge_left_match() {
        let moving = bounds(95.0, 0.0, 50.0, 50.0);
        let reference = bounds(100.0, 0.0, 50.0, 50.0);
        let (guides, (dx, _dy)) = compute_guides(moving, &[reference], 1.0, None);
        assert!(!guides.is_empty(), "should produce at least one guide");
        // Moving left edge (95) vs reference left edge (100): diff = 5 < threshold 8.
        assert!((dx - 5.0).abs() < 0.01, "dx should snap moving left to reference left");
        assert!(guides.iter().any(|g| g.axis == GuideAxis::Vertical));
    }

    #[test]
    fn no_guide_beyond_threshold() {
        // Both axes are well beyond the 8px threshold so no guide should fire.
        let moving = bounds(0.0, 0.0, 50.0, 50.0);
        let reference = bounds(200.0, 300.0, 50.0, 50.0);
        let (guides, (dx, dy)) = compute_guides(moving, &[reference], 1.0, None);
        assert!(guides.is_empty(), "no guide when far apart on both axes");
        assert!((dx).abs() < f32::EPSILON);
        assert!((dy).abs() < f32::EPSILON);
    }

    #[test]
    fn threshold_scales_with_zoom() {
        // At zoom=0.5 threshold = 8/0.5 = 16; diff of 12 should snap.
        let moving = bounds(88.0, 0.0, 50.0, 50.0);
        let reference = bounds(100.0, 0.0, 50.0, 50.0);
        let (guides, _) = compute_guides(moving, &[reference], 0.5, None);
        assert!(!guides.is_empty(), "12px diff < 16px threshold at zoom=0.5 should snap");
    }

    #[test]
    fn viewport_center_guide_emitted() {
        let vc = Point::new(Pixels(100.0), Pixels(0.0));
        // Moving centre x = 73 + 25 = 98; vc_x = 100; diff = 2 < threshold 8 → snap.
        let moving = bounds(73.0, 0.0, 50.0, 50.0);
        let (guides, _) = compute_guides(moving, &[], 1.0, Some(vc));
        assert!(
            guides.iter().any(|g| g.axis == GuideAxis::Vertical && g.kind == GuideKind::Center),
            "viewport centre guide should be emitted"
        );
    }

    #[test]
    fn equal_spacing_three_elements_detected() {
        let b = vec![
            bounds(0.0, 0.0, 10.0, 10.0),
            bounds(20.0, 0.0, 10.0, 10.0),
            bounds(40.0, 0.0, 10.0, 10.0),
        ];
        // Centres: 5, 25, 45 — gaps of 20.
        let spacing = detect_equal_spacing(&b, GuideAxis::Horizontal, 0.5);
        assert!(spacing.is_some(), "should detect equal spacing");
        assert!((spacing.unwrap() - 20.0).abs() < 0.01);
    }

    #[test]
    fn equal_spacing_irregular_returns_none() {
        let b = vec![
            bounds(0.0, 0.0, 10.0, 10.0),
            bounds(20.0, 0.0, 10.0, 10.0),
            bounds(50.0, 0.0, 10.0, 10.0),
        ];
        // Centres: 5, 25, 55 — gaps 20, 30.
        let spacing = detect_equal_spacing(&b, GuideAxis::Horizontal, 0.5);
        assert!(spacing.is_none(), "irregular spacing should return None");
    }
}
