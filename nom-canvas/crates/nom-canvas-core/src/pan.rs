//! Pan operations: drag-pan and auto-pan at viewport edges.
//!
//! ## Pan-by derivation
//!
//! A view-space delta `dv` moves the camera.  Because the camera's `center` is
//! in model space, we invert the zoom:
//!
//! ```text
//! C_new = C_old - dv / Z
//! ```
//!
//! The subtraction (not addition) is intentional: dragging right in view space
//! means the canvas appears to move right, so the model-space window shifts
//! left — i.e. the center decreases in x.

#![deny(unsafe_code)]

use nom_gpui::{Pixels, Point};

use crate::viewport::Viewport;

/// Translate the viewport by a view-space delta.
///
/// - Positive `delta_view.x` pans the canvas leftward (camera moves right).
/// - Positive `delta_view.y` pans the canvas upward  (camera moves down).
///
/// This is the operation for space+drag, middle-mouse drag, and trackpad
/// two-finger scroll (where scroll distance is already in view pixels).
pub fn pan_by(vp: &mut Viewport, delta_view: Point<Pixels>) {
    vp.center.x.0 -= delta_view.x.0 / vp.zoom;
    vp.center.y.0 -= delta_view.y.0 / vp.zoom;
}

/// Compute the auto-pan view-space delta for the current tick.
///
/// When the cursor approaches a viewport edge during a drag operation, the
/// canvas should scroll automatically so the user can drag items beyond the
/// initial visible area.
///
/// Returns a `Point<Pixels>` that should be passed to [`pan_by`] once per
/// animation tick.  Returns `(0, 0)` when the cursor is not near any edge.
///
/// - `EDGE`  — zone width in pixels along each edge that triggers auto-pan.
/// - `SPEED` — view-space pixels scrolled per tick (no inertia).
pub fn auto_pan_delta(cursor_view: Point<Pixels>, vp: &Viewport) -> Point<Pixels> {
    const EDGE: f32 = 30.0;
    const SPEED: f32 = 30.0; // px per tick

    let mut dx = 0.0_f32;
    let mut dy = 0.0_f32;

    if cursor_view.x.0 < EDGE {
        dx = -SPEED;
    } else if cursor_view.x.0 > vp.size.width.0 - EDGE {
        dx = SPEED;
    }

    if cursor_view.y.0 < EDGE {
        dy = -SPEED;
    } else if cursor_view.y.0 > vp.size.height.0 - EDGE {
        dy = SPEED;
    }

    Point::new(Pixels(dx), Pixels(dy))
}

// ─── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewport::Viewport;
    use nom_gpui::{Pixels, Point, Size};

    fn make_vp(zoom: f32) -> Viewport {
        let mut vp = Viewport::new(Size::new(Pixels(800.0), Pixels(600.0)));
        vp.set_zoom(zoom);
        vp
    }

    /// Panning right in view space should move the model-space centre to the
    /// left (so the canvas appears to scroll left).
    #[test]
    fn pan_right_moves_center_left_in_model_space() {
        let mut vp = make_vp(1.0);
        let center_before = vp.center.x.0;

        // 100 px rightward drag at zoom = 1
        pan_by(&mut vp, Point::new(Pixels(100.0), Pixels(0.0)));

        assert!(
            vp.center.x.0 < center_before,
            "center.x should decrease after panning right: {} vs {}",
            vp.center.x.0,
            center_before
        );
        assert!(
            (vp.center.x.0 - (center_before - 100.0)).abs() < 1e-4,
            "expected center.x = {}, got {}",
            center_before - 100.0,
            vp.center.x.0
        );
    }

    /// A cursor safely in the middle of the viewport should produce no auto-pan.
    #[test]
    fn auto_pan_zero_when_not_near_edges() {
        let vp = make_vp(1.0);
        // Centre of an 800×600 viewport, well away from every 30 px edge zone.
        let cursor = Point::new(Pixels(400.0), Pixels(300.0));
        let delta = auto_pan_delta(cursor, &vp);
        assert!(
            delta.x.0.abs() < f32::EPSILON && delta.y.0.abs() < f32::EPSILON,
            "expected zero auto-pan delta, got ({}, {})",
            delta.x.0,
            delta.y.0
        );
    }
}
