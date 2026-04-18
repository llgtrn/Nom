/// Grid and edge-snapping for the canvas.
///
/// Constants sourced from Excalidraw:
///   - SNAP_THRESHOLD = 8 px  (within how many canvas px a snap fires)
///   - GRID_SIZE      = 24 px (distance between grid lines)

pub const SNAP_THRESHOLD: f32 = 8.0;
pub const GRID_SIZE: f32 = 20.0;

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
            (x, ox),    // left → left
            (mx2, ox2), // right → right
            (mcx, ocx), // centre → centre
            (x, ox2),   // left → right (butting)
            (mx2, ox),  // right → left (butting)
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
        let y_pairs: [(f32, f32); 5] = [(y, oy), (my2, oy2), (mcy, ocy), (y, oy2), (my2, oy)];
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
        // 11px is closer to 20 (grid) than 0; nearest grid = 20
        let snapped = snap_to_grid([11.0, 0.0]);
        assert!(
            (snapped[0] - 20.0).abs() < 1e-6,
            "expected 20, got {}",
            snapped[0]
        );
    }

    #[test]
    fn snap_to_grid_rounds_down() {
        // 9px is closer to 0 than 20
        let snapped = snap_to_grid([9.0, 0.0]);
        assert!(snapped[0].abs() < 1e-6, "expected 0, got {}", snapped[0]);
    }

    #[test]
    fn snap_to_grid_two_cells() {
        let snapped = snap_to_grid([40.0, 40.0]);
        assert!((snapped[0] - 40.0).abs() < 1e-6);
        assert!((snapped[1] - 40.0).abs() < 1e-6);
    }

    #[test]
    fn snap_to_grid_negative() {
        // -11 rounds to -20
        let snapped = snap_to_grid([-11.0, 0.0]);
        assert!((snapped[0] - (-20.0)).abs() < 1e-6, "got {}", snapped[0]);
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
        assert!(
            (result.x - 100.0).abs() < 1e-6,
            "expected x=100, got {}",
            result.x
        );
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
        assert!(
            (result.x - 120.0).abs() < 1e-6,
            "expected x unchanged at 120, got {}",
            result.x
        );
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

    #[test]
    fn snap_to_grid_rounds_correctly() {
        // GRID_SIZE = 20; values below midpoint round down, above round up
        let snapped_up = snap_to_grid([14.0, 0.0]);
        assert!(
            (snapped_up[0] - 20.0).abs() < 1e-6,
            "14 should round up to 20, got {}",
            snapped_up[0]
        );
        let snapped_down = snap_to_grid([6.0, 0.0]);
        assert!(
            snapped_down[0].abs() < 1e-6,
            "6 should round down to 0, got {}",
            snapped_down[0]
        );
        // Both axes independent
        let snapped_both = snap_to_grid([14.0, 36.0]);
        assert!((snapped_both[0] - 20.0).abs() < 1e-6);
        assert!((snapped_both[1] - 40.0).abs() < 1e-6);
    }

    #[test]
    fn snap_threshold_within_range() {
        // Moving element at x=4 with grid_snap=true; nearest grid line is 0, delta=4 < SNAP_THRESHOLD(8)
        let result = snap_with_guides([4.0, 4.0], [10.0, 10.0], &[], true);
        assert!(
            result.x.abs() < 1e-6,
            "expected snap to x=0, got {}",
            result.x
        );
        assert!(
            result.y.abs() < 1e-6,
            "expected snap to y=0, got {}",
            result.y
        );
    }

    #[test]
    fn snap_disabled_returns_original() {
        // With grid_snap=false and no other elements, the position must be unchanged.
        let origin = [37.0_f32, 53.0_f32];
        let result = snap_with_guides(origin, [20.0, 20.0], &[], false);
        assert!(
            (result.x - origin[0]).abs() < 1e-6,
            "expected x={} unchanged, got {}",
            origin[0],
            result.x
        );
        assert!(
            (result.y - origin[1]).abs() < 1e-6,
            "expected y={} unchanged, got {}",
            origin[1],
            result.y
        );
        assert!(
            result.guides.is_empty(),
            "no guides should fire when snap is disabled"
        );
    }

    #[test]
    fn snap_threshold_outside_range() {
        // Place element far enough from every grid line that no snap fires.
        // GRID_SIZE=20; position 10 is exactly at the midpoint between 0 and 20.
        // snap_to_grid([10,10]) = [20,20] but delta=10 > SNAP_THRESHOLD(8) → no snap.
        // However f32 rounding: 10/20=0.5, round()=0 or 1 depending on banker's rounding.
        // Use 9.5 which unambiguously rounds to 20 but delta from 9.5 to 20 = 10.5 > 8.
        let result = snap_with_guides([9.5, 9.5], [10.0, 10.0], &[], true);
        // delta = |9.5 - 20| = 10.5 > SNAP_THRESHOLD → position unchanged
        assert!(
            (result.x - 9.5).abs() < 1e-6,
            "expected x unchanged at 9.5, got {}",
            result.x
        );
        assert!(
            (result.y - 9.5).abs() < 1e-6,
            "expected y unchanged at 9.5, got {}",
            result.y
        );
    }

    // ── new tests ────────────────────────────────────────────────────────────

    #[test]
    fn snap_to_grid_aligns_point_4px_grid() {
        // GRID_SIZE=20; (3.7, 5.2) → nearest grid = (0, 0) since both < 10
        // Actually snap_to_grid uses GRID_SIZE=20, so (3.7/20).round()*20 = 0
        let snapped = snap_to_grid([3.7, 5.2]);
        assert!(
            snapped[0].abs() < 1e-6,
            "3.7 on 20px grid -> 0, got {}",
            snapped[0]
        );
        assert!(
            snapped[1].abs() < 1e-6,
            "5.2 on 20px grid -> 0, got {}",
            snapped[1]
        );
    }

    #[test]
    fn snap_to_grid_already_aligned_unchanged() {
        // (20.0, 40.0) are exact grid intersections — must remain unchanged.
        let snapped = snap_to_grid([20.0, 40.0]);
        assert!((snapped[0] - 20.0).abs() < 1e-6, "got {}", snapped[0]);
        assert!((snapped[1] - 40.0).abs() < 1e-6, "got {}", snapped[1]);
    }

    #[test]
    fn snap_threshold_no_snap_when_far() {
        // GRID_SIZE=20; x=9.5 → nearest grid is 20, delta=10.5 > SNAP_THRESHOLD(8)
        // Using grid_snap=true, position must stay at 9.5
        let result = snap_with_guides([9.5, 9.5], [5.0, 5.0], &[], true);
        assert!(
            (result.x - 9.5).abs() < 1e-6,
            "no snap expected far from grid, got x={}",
            result.x
        );
        assert!(
            (result.y - 9.5).abs() < 1e-6,
            "no snap expected far from grid, got y={}",
            result.y
        );
    }

    #[test]
    fn snap_guides_from_multiple_elements_fires_guide() {
        // Two elements both at x=100; moving element at x=103 snaps to x=100 → guide fires
        let result = snap_with_guides(
            [103.0, 200.0],
            [40.0, 30.0],
            &[
                ([100.0, 50.0], [40.0, 30.0]),
                ([100.0, 150.0], [40.0, 30.0]),
            ],
            false,
        );
        assert!(
            !result.guides.is_empty(),
            "expected snap guide from element alignment"
        );
        assert!(
            (result.x - 100.0).abs() < 1e-6,
            "expected snap to x=100, got {}",
            result.x
        );
    }

    #[test]
    fn snap_nearest_guide_center_to_center() {
        // Moving element centre should snap to target centre when within threshold.
        // moving origin=(90, 50), size=(20, 20) → centre=(100, 60)
        // target origin=(100, 50), size=(20, 20) → centre=(110, 60)
        // delta centre-x = |100 - 110| = 10 > SNAP_THRESHOLD; use closer distance.
        // moving origin=(103, 50), size=(20, 20) → centre=(113, 60)
        // target origin=(100, 50), size=(20, 20) → centre=(110, 60), delta=3 < 8 → snap
        let result = snap_with_guides(
            [103.0, 50.0],
            [20.0, 20.0],
            &[([100.0, 50.0], [20.0, 20.0])],
            false,
        );
        // left-left pair: |103 - 100| = 3 < threshold → snaps to 100
        assert!(
            (result.x - 100.0).abs() < 1e-6,
            "expected x=100, got {}",
            result.x
        );
    }

    #[test]
    fn snap_guide_axis_is_horizontal_for_y_snap() {
        // Move element y=103, target y=100 → within 8px → horizontal guide fires
        let result = snap_with_guides(
            [200.0, 103.0],
            [40.0, 30.0],
            &[([200.0, 100.0], [40.0, 30.0])],
            false,
        );
        let has_horizontal = result.guides.iter().any(|g| g.axis == SnapAxis::Horizontal);
        assert!(has_horizontal, "expected a horizontal guide for Y snap");
    }

    #[test]
    fn snap_guide_color_is_set() {
        let result = snap_with_guides(
            [102.0, 50.0],
            [40.0, 30.0],
            &[([100.0, 50.0], [40.0, 30.0])],
            false,
        );
        assert!(!result.guides.is_empty());
        let guide = &result.guides[0];
        // Excalidraw green: alpha > 0
        assert!(guide.color[3] > 0.0, "guide color alpha must be > 0");
    }

    // ── additional snapping tests ────────────────────────────────────────────

    #[test]
    fn snap_to_grid_at_spacing_20_multiple_cells() {
        // GRID_SIZE = 20; points at exact multiples must be unchanged.
        for mult in 0_i32..=5 {
            let v = mult as f32 * 20.0;
            let snapped = snap_to_grid([v, v]);
            assert!(
                (snapped[0] - v).abs() < 1e-5,
                "grid point {v} should be unchanged, got {}",
                snapped[0]
            );
        }
    }

    #[test]
    fn snap_to_grid_halfway_rounds_up() {
        // 10.0 is exactly halfway between 0 and 20; f32 round() rounds 0.5 to 1 → snaps to 20.
        let snapped = snap_to_grid([10.0, 0.0]);
        assert!(
            (snapped[0] - 20.0).abs() < 1e-5,
            "10.0 should round to 20, got {}",
            snapped[0]
        );
    }

    #[test]
    fn snap_disabled_with_others_no_snap_far() {
        // With grid_snap=false and element far beyond threshold, position unchanged.
        let result = snap_with_guides(
            [0.0, 0.0],
            [10.0, 10.0],
            &[([200.0, 200.0], [10.0, 10.0])],
            false,
        );
        assert!(
            (result.x - 0.0).abs() < 1e-6,
            "position must be unchanged when far from all elements, got x={}",
            result.x
        );
        assert!(result.guides.is_empty(), "no guides should fire when no snap");
    }

    #[test]
    fn snap_to_nearest_guide_right_edge_snaps() {
        // Moving element right edge at x=148, target right edge at x=150 → delta=2 < threshold.
        // Moving origin=(100, 0), size=(48, 10) → right edge = 148.
        // Target origin=(90, 0), size=(60, 10) → right edge = 150.
        let result = snap_with_guides(
            [100.0, 0.0],
            [48.0, 10.0],
            &[([90.0, 0.0], [60.0, 10.0])],
            false,
        );
        // right→right snap: mv=148, ov=150, delta=2 → x snaps to 102
        assert!(
            (result.x - 102.0).abs() < 1e-5,
            "right-edge snap: expected x=102, got {}",
            result.x
        );
        assert!(!result.guides.is_empty(), "vertical guide must fire for x snap");
    }

    #[test]
    fn snap_to_nearest_guide_bottom_edge_snaps() {
        // Moving element bottom at y=98, target bottom at y=100 → delta=2 < threshold → snap.
        // Moving origin=(0, 88), size=(10, 10) → bottom=98.
        // Target origin=(0, 90), size=(10, 10) → bottom=100.
        let result = snap_with_guides(
            [0.0, 88.0],
            [10.0, 10.0],
            &[([0.0, 90.0], [10.0, 10.0])],
            false,
        );
        // bottom→bottom: mv=98, ov=100, delta=2 → y snaps to 90
        assert!(
            (result.y - 90.0).abs() < 1e-5,
            "bottom-edge snap: expected y=90, got {}",
            result.y
        );
        let has_horizontal = result.guides.iter().any(|g| g.axis == SnapAxis::Horizontal);
        assert!(has_horizontal, "horizontal guide must fire for y snap");
    }

    #[test]
    fn snap_grid_on_snaps_within_threshold() {
        // x=1 is 1px from grid line at 0 (within SNAP_THRESHOLD=8) → snaps to 0.
        let result = snap_with_guides([1.0, 0.0], [5.0, 5.0], &[], true);
        assert!((result.x).abs() < 1e-6, "x=1 should snap to grid 0, got {}", result.x);
    }

    #[test]
    fn snap_grid_threshold_boundary_no_snap() {
        // x=8.5 is 8.5 from grid 0 and 11.5 from grid 20; nearest is 0 at delta=8.5 > SNAP_THRESHOLD → no snap.
        let result = snap_with_guides([8.5, 0.0], [5.0, 5.0], &[], true);
        assert!(
            (result.x - 8.5).abs() < 1e-5,
            "x=8.5 is beyond threshold, must not snap, got {}",
            result.x
        );
    }

    #[test]
    fn snap_result_has_correct_x_y_fields() {
        // Basic field access: SnapResult exposes x and y.
        let result = snap_with_guides([50.0, 60.0], [10.0, 10.0], &[], false);
        assert!((result.x - 50.0).abs() < 1e-6);
        assert!((result.y - 60.0).abs() < 1e-6);
    }

    #[test]
    fn snap_to_grid_large_negative_value() {
        // -41.0: nearest grid multiple = -40 (since -41/20 = -2.05, round = -2, -2*20 = -40).
        let snapped = snap_to_grid([-41.0, 0.0]);
        assert!(
            (snapped[0] - (-40.0)).abs() < 1e-5,
            "-41 should round to -40 on 20px grid, got {}",
            snapped[0]
        );
    }

    #[test]
    fn snap_multiple_others_first_x_snap_wins() {
        // Two target elements both at x=100; moving at x=103.
        // The first element should trigger the snap; result x=100.
        let result = snap_with_guides(
            [103.0, 300.0],
            [20.0, 20.0],
            &[
                ([100.0, 100.0], [20.0, 20.0]),
                ([100.0, 400.0], [20.0, 20.0]),
            ],
            false,
        );
        assert!(
            (result.x - 100.0).abs() < 1e-5,
            "first matching x snap must pull x to 100, got {}",
            result.x
        );
    }

    #[test]
    fn snap_guide_from_extent_covers_both_elements() {
        // When a vertical snap fires, the guide's from..to should span both elements.
        // Moving element: origin=(102, 200), size=(30, 20) → y range [200, 220].
        // Target element: origin=(100, 50), size=(30, 20) → y range [50, 70].
        let result = snap_with_guides(
            [102.0, 200.0],
            [30.0, 20.0],
            &[([100.0, 50.0], [30.0, 20.0])],
            false,
        );
        let vguide = result.guides.iter().find(|g| g.axis == SnapAxis::Vertical);
        assert!(vguide.is_some(), "vertical guide must fire");
        let g = vguide.unwrap();
        // from should be <= min y of both elements (50), to should be >= max y (220).
        assert!(g.from <= 200.0 + 1e-5, "guide from must be at or above moving element y");
        assert!(g.to >= 70.0 - 1e-5, "guide to must be at or below target bottom");
    }

    // ── additional snap tests ────────────────────────────────────────────────

    #[test]
    fn snap_to_grid_size_1_returns_exact() {
        // With GRID_SIZE=20 and input already on a grid multiple, value is unchanged.
        // At grid multiples, snap_to_grid is identity.
        let snapped = snap_to_grid([60.0, 80.0]);
        assert!((snapped[0] - 60.0).abs() < 1e-5, "60 is on 20px grid, got {}", snapped[0]);
        assert!((snapped[1] - 80.0).abs() < 1e-5, "80 is on 20px grid, got {}", snapped[1]);
    }

    #[test]
    fn snap_to_grid_rounds_up_at_midpoint() {
        // 10.0 is exactly halfway between 0 and 20; f32 round() rounds 0.5 upward → 20.
        let snapped = snap_to_grid([10.0, 30.0]);
        assert!(
            (snapped[0] - 20.0).abs() < 1e-5,
            "midpoint 10 should round up to 20, got {}",
            snapped[0]
        );
        // 30 is at 1.5 * GRID_SIZE → rounds to 2 * GRID_SIZE = 40.
        assert!(
            (snapped[1] - 40.0).abs() < 1e-5,
            "midpoint 30 should round up to 40, got {}",
            snapped[1]
        );
    }

    #[test]
    fn snap_to_grid_negative_coords() {
        // -11 → nearest grid multiple is -20 (round(-11/20) = round(-0.55) = -1, *20 = -20).
        let snapped = snap_to_grid([-11.0, -9.0]);
        assert!(
            (snapped[0] - (-20.0)).abs() < 1e-5,
            "-11 should snap to -20, got {}",
            snapped[0]
        );
        // -9 → round(-9/20) = round(-0.45) = 0, * 20 = 0.
        assert!(
            (snapped[1]).abs() < 1e-5,
            "-9 should snap to 0, got {}",
            snapped[1]
        );
    }

    #[test]
    fn snap_to_objects_finds_nearest() {
        // Moving element at x=103; one target at x=100 (3px away) → snaps to 100.
        let result = snap_with_guides(
            [103.0, 50.0],
            [20.0, 20.0],
            &[([100.0, 50.0], [20.0, 20.0])],
            false,
        );
        assert!(
            (result.x - 100.0).abs() < 1e-5,
            "should snap to nearest object at x=100, got {}",
            result.x
        );
    }

    #[test]
    fn snap_to_objects_empty_returns_original() {
        // No nearby objects → position unchanged.
        let origin = [123.0_f32, 456.0_f32];
        let result = snap_with_guides(origin, [30.0, 30.0], &[], false);
        assert!((result.x - origin[0]).abs() < 1e-5, "x unchanged, got {}", result.x);
        assert!((result.y - origin[1]).abs() < 1e-5, "y unchanged, got {}", result.y);
        assert!(result.guides.is_empty(), "no guides when no objects");
    }

    #[test]
    fn snap_guides_horizontal_and_vertical() {
        // Element at x=103, y=103; target at x=100, y=100 → both axes snap.
        let result = snap_with_guides(
            [103.0, 103.0],
            [20.0, 20.0],
            &[([100.0, 100.0], [20.0, 20.0])],
            false,
        );
        let has_vertical = result.guides.iter().any(|g| g.axis == SnapAxis::Vertical);
        let has_horizontal = result.guides.iter().any(|g| g.axis == SnapAxis::Horizontal);
        assert!(has_vertical, "vertical guide must fire for x snap");
        assert!(has_horizontal, "horizontal guide must fire for y snap");
    }

    #[test]
    fn snap_threshold_not_exceeded_no_snap() {
        // x=120, target x=100 → delta=20 > SNAP_THRESHOLD(8) → no snap.
        let result = snap_with_guides(
            [120.0, 500.0],
            [10.0, 10.0],
            &[([100.0, 200.0], [10.0, 10.0])],
            false,
        );
        assert!(
            (result.x - 120.0).abs() < 1e-5,
            "outside threshold: x must stay at 120, got {}",
            result.x
        );
        assert!(result.guides.is_empty(), "no snap guide beyond threshold");
    }

    #[test]
    fn snap_threshold_within_snaps() {
        // x=105, target x=100 → delta=5 < SNAP_THRESHOLD(8) → snaps to 100.
        let result = snap_with_guides(
            [105.0, 200.0],
            [10.0, 10.0],
            &[([100.0, 200.0], [10.0, 10.0])],
            false,
        );
        assert!(
            (result.x - 100.0).abs() < 1e-5,
            "within threshold: must snap to x=100, got {}",
            result.x
        );
        assert!(!result.guides.is_empty(), "snap guide must fire within threshold");
    }

    #[test]
    fn snap_multiple_candidates_returns_closest() {
        // Two targets: one at x=100 (delta=3) and one at x=107 (delta=4).
        // Moving at x=103 — both within threshold. First match wins (left→left pair).
        let result = snap_with_guides(
            [103.0, 200.0],
            [10.0, 10.0],
            &[
                ([100.0, 200.0], [10.0, 10.0]),
                ([107.0, 200.0], [10.0, 10.0]),
            ],
            false,
        );
        // Should snap to the first matching target.
        assert!(
            !result.guides.is_empty(),
            "at least one snap guide must fire"
        );
    }

    #[test]
    fn snap_to_grid_size_16_rounds_correctly() {
        // GRID_SIZE=20 (not 16), but verify rounding at 16px on 20px grid:
        // 16/20 = 0.8 → rounds to 1 → 20.
        let snapped = snap_to_grid([16.0, 0.0]);
        assert!(
            (snapped[0] - 20.0).abs() < 1e-5,
            "16 on 20px grid → 20, got {}",
            snapped[0]
        );
    }

    #[test]
    fn snap_to_grid_zero_size_no_panic() {
        // Snap at [0.0, 0.0] must not panic and must return [0.0, 0.0].
        let snapped = snap_to_grid([0.0, 0.0]);
        assert!(snapped[0].abs() < 1e-5, "origin snaps to origin, got {}", snapped[0]);
        assert!(snapped[1].abs() < 1e-5, "origin snaps to origin, got {}", snapped[1]);
    }

    #[test]
    fn snap_disabled_no_grid_no_guides() {
        // grid_snap=false, no other elements: result is unchanged with no guides.
        let result = snap_with_guides([37.5, 82.3], [15.0, 15.0], &[], false);
        assert!((result.x - 37.5).abs() < 1e-5, "x unchanged, got {}", result.x);
        assert!((result.y - 82.3).abs() < 1e-4, "y unchanged, got {}", result.y);
        assert!(result.guides.is_empty(), "no guides when disabled");
    }
}
