//! Coordinate transforms between model space and view (screen) space.
//!
//! ## Coordinate systems
//!
//! - **Model space** — the infinite canvas.  Units are logical pixels.
//!   The Y axis grows downward (same as screen conventions).
//! - **View space** — screen pixels measured from the top-left corner of the
//!   viewport rectangle.
//!
//! ## Transform derivation
//!
//! Let `C` = `vp.center` (model coords at screen centre),
//!     `Z` = `vp.zoom`,
//!     `W` = `vp.size.width`, `H` = `vp.size.height`.
//!
//! ```text
//! to_view:
//!   vx = (mx - Cx) * Z + W/2
//!   vy = (my - Cy) * Z + H/2
//!
//! to_model (inverse):
//!   mx = (vx - W/2) / Z + Cx
//!   my = (vy - H/2) / Z + Cy
//! ```
//!
//! The two operations are exact inverses; floating-point round-trips may differ
//! by < 1 ulp.

#![deny(unsafe_code)]

use nom_gpui::{Pixels, Point};

use crate::viewport::Viewport;

/// Convert a model-space point to view-space (pixels from the top-left corner
/// of the viewport).
#[inline]
pub fn to_view(vp: &Viewport, model: Point<Pixels>) -> Point<Pixels> {
    let vx = (model.x.0 - vp.center.x.0) * vp.zoom + vp.size.width.0 / 2.0;
    let vy = (model.y.0 - vp.center.y.0) * vp.zoom + vp.size.height.0 / 2.0;
    Point::new(Pixels(vx), Pixels(vy))
}

/// Convert a view-space point (pixels from top-left) back to model space.
/// This is the exact inverse of [`to_view`].
#[inline]
pub fn to_model(vp: &Viewport, view: Point<Pixels>) -> Point<Pixels> {
    let mx = (view.x.0 - vp.size.width.0 / 2.0) / vp.zoom + vp.center.x.0;
    let my = (view.y.0 - vp.size.height.0 / 2.0) / vp.zoom + vp.center.y.0;
    Point::new(Pixels(mx), Pixels(my))
}

// ─── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewport::Viewport;
    use nom_gpui::Size;

    fn make_vp(zoom: f32) -> Viewport {
        let mut vp = Viewport::new(Size::new(Pixels(800.0), Pixels(600.0)));
        vp.set_zoom(zoom);
        vp
    }

    /// Round-trip: model → view → model should recover the original point.
    #[test]
    fn round_trip_model_to_view_to_model() {
        let vp = make_vp(2.0);
        let original = Point::new(Pixels(123.0), Pixels(-456.0));
        let recovered = to_model(&vp, to_view(&vp, original));
        assert!(
            (recovered.x.0 - original.x.0).abs() < 1e-4,
            "x round-trip failed: {} vs {}",
            recovered.x.0,
            original.x.0
        );
        assert!(
            (recovered.y.0 - original.y.0).abs() < 1e-4,
            "y round-trip failed: {} vs {}",
            recovered.y.0,
            original.y.0
        );
    }

    /// The model-space centre always maps to the geometric centre of the screen.
    #[test]
    fn center_point_maps_to_viewport_center() {
        let vp = make_vp(1.5);
        let view_pos = to_view(&vp, vp.center);
        let expected_x = vp.size.width.0 / 2.0;
        let expected_y = vp.size.height.0 / 2.0;
        assert!(
            (view_pos.x.0 - expected_x).abs() < 1e-4,
            "center x: {} vs {}",
            view_pos.x.0,
            expected_x
        );
        assert!(
            (view_pos.y.0 - expected_y).abs() < 1e-4,
            "center y: {} vs {}",
            view_pos.y.0,
            expected_y
        );
    }
}
