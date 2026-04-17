/// Grid and edge-snapping for the canvas.
///
/// Constants sourced from Excalidraw:
///   - SNAP_THRESHOLD = 8 px  (within how many canvas px a snap fires)
///   - GRID_SIZE      = 24 px (distance between grid lines)

pub const SNAP_THRESHOLD: f32 = 8.0;
pub const GRID_SIZE: f32 = 24.0;

// ─── Guide lines ────────────────────────────────────────────────────────────

/// Which axis a snap guide runs along.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapAxis {
    /// A vertical guide line (constant-x).
    Vertical,
    /// A horizontal guide line (constant-y).
    Horizontal,
}

/// A rendered snap guide overlay line.
#[derive(Debug, Clone)]
pub struct SnapGuide {
    pub axis: SnapAxis,
    /// The constant coordinate of the guide (x for Vertical, y for Horizontal).
    pub position: f32,
    /// Start extent of the guide along the perpendicular axis.
    pub from: f32,
    /// End extent of the guide along the perpendicular axis.
    pub to: f32,
    /// RGBA colour for rendering.
    pub color: [f32; 4],
}

// ─── Snap result ────────────────────────────────────────────────────────────

/// The adjusted position and any guides to render after snapping.
pub struct SnapResult {
    /// Adjusted x origin of the moving element.
    pub x: f32,
    /// Adjusted y origin of the moving element.
    pub y: f32,
    /// Guide lines to overlay while the snap is active.
    pub guides: Vec<SnapGuide>,
}

// ─── Grid snapping ──────────────────────────────────────────────────────────

/// Round a canvas-space point to the nearest grid intersection.
pub fn snap_to_grid(pt: [f32; 2]) -> [f32; 2] {
    [
        (pt[0] / GRID_SIZE).round() * GRID_SIZE,
        (pt[1] / GRID_SIZE).round() * GRID_SIZE,
    ]
}

// ─── Edge + center snapping ─────────────────────────────────────────────────

/// Snap a moving element (given by `moving_origin` + `moving_size`) against a
/// set of stationary elements and, optionally, the grid.
///
/// Returns the adjusted origin and any guide lines that fired.
///
/// `others` is a slice of `(origin, size)` pairs for the stationary elements.
pub fn snap_with_guides(
    moving_origin: [f32; 2],
    moving_size: [f32; 2],
    others: &[([f32; 2], [f32; 2])],
    grid_snap: bool,
) -> SnapResult {
    let mut x = moving_origin[0];
    let mut y = moving_origin[1];
    let mut guides: Vec<SnapGuide> = Vec::new();

    // ── Grid snap (applied first so element edges can override) ──────────────
    if grid_snap {
        let snapped = snap_to_grid([x, y]);
        if (x - snapped[0]).abs() < SNAP_THRESHOLD {
            x = snapped[0];
        }
        if (y - snapped[1]).abs() < SNAP_THRESHOLD {
            y = snapped[1];
        }
    }

    // Pre-compute moving element edges & centre from the (possibly grid-adjusted) origin.
    let mx2 = x + moving_size[0];
    let my2 = y + moving_size[1];
    let mcx = x + moving_size[0] / 2.0;
    let mcy = y + moving_size[1] / 2.0;

    let guide_color: [f32; 4] = [0.133, 0.773, 0.369, 0.8]; // Excalidraw green

    for &(o_origin, o_size) in others {
        let ox = o_origin[0];
        let oy = o_origin[1];
        let ox2 = ox + o_size[0];
        let oy2 = oy + o_size[1];
        let ocx = ox + o_size[0] / 2.0;
        let ocy = oy + o_size[1] / 2.0;

        // ── X (vertical guide) snap pairs ────────────────────────────────────
        // Pairs: (moving feature, target feature) — left, right, centre, cross-left, cross-right
        let x_pairs: [(f32, f32); 5] = [
            (x, ox),   // left → left
            (mx2, ox2), // right → right
            (mcx, ocx), // centre → centre
            (x, ox2),  // left → right (butting)
            (mx2, ox), // right → left (butting)
        ];
        for (mv, ov) in x_pairs {
            if (mv - ov).abs() < SNAP_THRESHOLD {
                let delta = ov - mv;
                x += delta;
                // Recompute dependent values after snap.
                let new_my2 = y + moving_size[1];
                guides.push(SnapGuide {
                    axis: SnapAxis::Vertical,
                    position: ov,
                    from: y.min(oy),
                    to: new_my2.max(oy2),
                    color: guide_color,
                });
                break; // one X snap per other element
            }
        }

        // ── Y (horizontal guide) snap pairs ──────────────────────────────────
        let y_pairs: [(f32, f32); 5] = [
            (y, oy),
            (my2, oy2),
            (mcy, ocy),
            (y, oy2),
            (my2, oy),
        ];
        for (mv, ov) in y_pairs {
            if (mv - ov).abs() < SNAP_THRESHOLD {
                let delta = ov - mv;
                y += delta;
                let new_mx2 = x + moving_size[0];
                guides.push(SnapGuide {
                    axis: SnapAxis::Horizontal,
                    position: ov,
                    from: x.min(ox),
                    to: new_mx2.max(ox2),
                    color: guide_color,
                });
                break;
            }
        }
    }

    SnapResult { x, y, guides }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── snap_to_grid ─────────────────────────────────────────────────────────

    #[test]
    fn snap_to_grid_origin_stays() {
        let snapped = snap_to_grid([0.0, 0.0]);
        assert!((snapped[0]).abs() < 1e-6);
        assert!((snapped[1]).abs() < 1e-6);
    }

    #[test]
    fn snap_to_grid_rounds_to_nearest() {
        // 13px is closer to 24 (grid) than 0; nearest grid = 24
        let snapped = snap_to_grid([13.0, 0.0]);
        assert!(
            (snapped[0] - 24.0).abs() < 1e-6,
            "expected 24, got {}",
            snapped[0]
        );
    }

    #[test]
    fn snap_to_grid_rounds_down() {
        // 11px is closer to 0 than 24
        let snapped = snap_to_grid([11.0, 0.0]);
        assert!(
            snapped[0].abs() < 1e-6,
            "expected 0, got {}",
            snapped[0]
        );
    }

    #[test]
    fn snap_to_grid_two_cells() {
        let snapped = snap_to_grid([48.0, 48.0]);
        assert!((snapped[0] - 48.0).abs() < 1e-6);
        assert!((snapped[1] - 48.0).abs() < 1e-6);
    }

    #[test]
    fn snap_to_grid_negative() {
        // -13 rounds to -24
        let snapped = snap_to_grid([-13.0, 0.0]);
        assert!((snapped[0] - (-24.0)).abs() < 1e-6, "got {}", snapped[0]);
    }

    // ── snap_with_guides ─────────────────────────────────────────────────────

    #[test]
    fn snap_no_others_no_guides() {
        let result = snap_with_guides([50.0, 50.0], [30.0, 20.0], &[], false);
        assert_eq!(result.guides.len(), 0);
        assert!((result.x - 50.0).abs() < 1e-6);
    }

    #[test]
    fn snap_fires_on_left_left_alignment() {
        // Moving element left edge at x=102, target left edge at x=100 → within 8px → snap
        let result = snap_with_guides(
            [102.0, 50.0],
            [50.0, 30.0],
            &[([100.0, 50.0], [60.0, 30.0])],
            false,
        );
        assert!((result.x - 100.0).abs() < 1e-6, "expected x=100, got {}", result.x);
        assert!(!result.guides.is_empty());
    }

    #[test]
    fn snap_does_not_fire_beyond_threshold() {
        // Moving element left edge at x=120, target left edge at x=100 → 20px apart (> 8px threshold)
        // Y positions are deliberately offset so no Y-axis snap fires either.
        let result = snap_with_guides(
            [120.0, 500.0],
            [50.0, 30.0],
            &[([100.0, 200.0], [60.0, 30.0])],
            false,
        );
        assert!((result.x - 120.0).abs() < 1e-6, "expected x unchanged at 120, got {}", result.x);
        assert_eq!(result.guides.len(), 0);
    }

    #[test]
    fn snap_grid_adjusts_near_grid_line() {
        // x=2 with grid_snap=true → snapped to 0
        let result = snap_with_guides([2.0, 2.0], [10.0, 10.0], &[], true);
        assert!((result.x).abs() < 1e-6, "expected x=0, got {}", result.x);
        assert!((result.y).abs() < 1e-6, "expected y=0, got {}", result.y);
    }

    #[test]
    fn snap_guide_axis_is_vertical_for_x_snap() {
        let result = snap_with_guides(
            [103.0, 200.0],
            [40.0, 30.0],
            &[([100.0, 200.0], [40.0, 30.0])],
            false,
        );
        let has_vertical = result.guides.iter().any(|g| g.axis == SnapAxis::Vertical);
        assert!(has_vertical, "expected a vertical guide for X snap");
    }
}
