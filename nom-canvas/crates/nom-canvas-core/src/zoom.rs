//! Zoom operations: zoom-to-point and wheel-driven incremental zoom.
//!
//! ## Zoom-to-point formula
//!
//! Goal: given a pivot point `P` in view space, change the zoom so that `P`
//! appears to stay in place on screen.  In other words, `to_view(pivot_model)`
//! must equal `pivot_view` both before and after the zoom change.
//!
//! ### Derivation
//!
//! Let `C` = current center (model), `Z_old` = current zoom, `Z_new` = new
//! zoom, `P_m` = pivot in model space, `P_v` = pivot in view space (constant).
//!
//! From `coords::to_view`:
//! ```text
//! P_v = (P_m - C_new) * Z_new + W/2
//! ```
//!
//! Solving for `C_new`:
//! ```text
//! C_new = P_m - (P_v - W/2) / Z_new
//! ```
//!
//! Alternatively, express `P_v - W/2` via the old center:
//! ```text
//! P_v - W/2 = (P_m - C_old) * Z_old
//! ```
//!
//! Substituting:
//! ```text
//! C_new = P_m - (P_m - C_old) * Z_old / Z_new
//!       = P_m + (C_old - P_m) * (Z_old / Z_new)
//! ```
//!
//! This is the compact closed form:
//! `new_center = pivot + (center - pivot) * (prev_zoom / new_zoom)`.

#![deny(unsafe_code)]

use nom_gpui::{Pixels, Point};

use crate::{coords, viewport::Viewport};

/// Zoom the viewport toward `pivot_view` so that `new_zoom` becomes active.
///
/// The pivot point in model space remains under the same screen pixel after the
/// operation (exact — see module-level derivation).
pub fn zoom_to_point(vp: &mut Viewport, pivot_view: Point<Pixels>, new_zoom: f32) {
    // Capture pivot in model space before changing zoom.
    let pivot_model = coords::to_model(vp, pivot_view);

    let prev_zoom = vp.zoom;
    vp.set_zoom(new_zoom);
    let clamped_zoom = vp.zoom; // may differ from new_zoom due to clamping

    // Formula: new_center = pivot_model + (old_center - pivot_model) * (prev_zoom / clamped_zoom)
    let ratio = prev_zoom / clamped_zoom;
    let cx = pivot_model.x.0 + (vp.center.x.0 - pivot_model.x.0) * ratio;
    let cy = pivot_model.y.0 + (vp.center.y.0 - pivot_model.y.0) * ratio;
    vp.center = Point::new(Pixels(cx), Pixels(cy));
}

/// Apply one wheel notch of zoom centred on `pivot_view`.
///
/// `wheel_delta > 0` means zoom in (scroll up), `< 0` means zoom out.
/// Each notch multiplies the current zoom by `(1 + delta × 0.1)`, giving a
/// log-linear response (equal notches produce equal perceived zoom steps).
/// Discrete step size: 0.25 zoom-levels at `|delta| == 2.5`.
pub fn wheel_zoom_step(vp: &mut Viewport, pivot_view: Point<Pixels>, wheel_delta: f32) {
    let new_zoom = (vp.zoom * (1.0 + wheel_delta * 0.1))
        .clamp(Viewport::ZOOM_MIN, Viewport::ZOOM_MAX);
    zoom_to_point(vp, pivot_view, new_zoom);
}

// ─── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{coords, viewport::Viewport};
    use nom_gpui::{Pixels, Point, Size};

    fn make_vp(zoom: f32) -> Viewport {
        let mut vp = Viewport::new(Size::new(Pixels(800.0), Pixels(600.0)));
        vp.set_zoom(zoom);
        vp
    }

    /// After `zoom_to_point`, the model coord that was under the pivot should
    /// still map to the exact same view pixel.
    #[test]
    fn zoom_to_point_keeps_pivot_stationary() {
        let mut vp = make_vp(1.0);
        // A pivot somewhere off-centre.
        let pivot_view = Point::new(Pixels(200.0), Pixels(150.0));

        // Record model coord under the pivot before zooming.
        let pivot_model_before = coords::to_model(&vp, pivot_view);

        zoom_to_point(&mut vp, pivot_view, 2.5);

        // The same model point must map to the same view pixel after zooming.
        let pivot_view_after = coords::to_view(&vp, pivot_model_before);

        assert!(
            (pivot_view_after.x.0 - pivot_view.x.0).abs() < 1e-3,
            "pivot x drifted: before={} after={}",
            pivot_view.x.0,
            pivot_view_after.x.0
        );
        assert!(
            (pivot_view_after.y.0 - pivot_view.y.0).abs() < 1e-3,
            "pivot y drifted: before={} after={}",
            pivot_view.y.0,
            pivot_view_after.y.0
        );
    }
}
