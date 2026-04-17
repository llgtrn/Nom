//! Fit-to-bounds: adjust viewport zoom + center to contain a model-space rect.
//!
//! Used for "fit all" and "fit selection" commands.
//!
//! ## Algorithm
//!
//! 1. Subtract padding from the available viewport area.
//! 2. Compute scale factors for width and height separately.
//! 3. Take the smaller (most restrictive) scale so the entire rect fits.
//! 4. Centre the viewport on the midpoint of the rect.

#![deny(unsafe_code)]

use nom_gpui::{Bounds, Pixels, Point};

use crate::viewport::Viewport;

/// Padding to apply around the fitted bounds.
pub enum Padding {
    /// Padding expressed as a fraction of the viewport's dimension (0 … 1).
    /// E.g. `Fraction(0.1)` leaves 10 % of the viewport width/height as margin
    /// on each side.
    Fraction(f32),
    /// Absolute padding in view-space pixels on each side.
    Pixels(f32),
}

/// Adjust `vp` so that `model_bounds` fits entirely within the viewport with
/// the given `padding` on every side.
///
/// The function:
/// - Computes the tightest zoom that fully contains the bounds.
/// - Clamps the result to `[Viewport::ZOOM_MIN, Viewport::ZOOM_MAX]`.
/// - Sets the center to the geometric midpoint of `model_bounds`.
pub fn fit_to_bounds(vp: &mut Viewport, model_bounds: Bounds<Pixels>, padding: Padding) {
    let (pad_w, pad_h) = match padding {
        Padding::Fraction(f) => (vp.size.width.0 * f, vp.size.height.0 * f),
        Padding::Pixels(p) => (p, p),
    };

    // Available area after subtracting padding on both sides.
    let avail_w = (vp.size.width.0 - 2.0 * pad_w).max(1.0);
    let avail_h = (vp.size.height.0 - 2.0 * pad_h).max(1.0);

    // Scale factors that would make the bounds exactly fill each axis.
    let zoom_w = avail_w / model_bounds.size.width.0.max(1.0);
    let zoom_h = avail_h / model_bounds.size.height.0.max(1.0);

    // Use the more restrictive (smaller) zoom so both axes fit.
    vp.set_zoom(zoom_w.min(zoom_h));

    // Centre the viewport on the midpoint of the fitted bounds.
    vp.center = Point::new(
        Pixels(model_bounds.origin.x.0 + model_bounds.size.width.0 / 2.0),
        Pixels(model_bounds.origin.y.0 + model_bounds.size.height.0 / 2.0),
    );
}

// ─── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{coords, viewport::Viewport};
    use nom_gpui::{Bounds, Pixels, Point, Size};

    fn make_vp() -> Viewport {
        Viewport::new(Size::new(Pixels(800.0), Pixels(600.0)))
    }

    /// After `fit_to_bounds`, the four corners of `model_bounds` must map to
    /// view coords that lie strictly inside `[pad, W-pad] × [pad, H-pad]`.
    #[test]
    fn fit_to_bounds_fully_contains_them_with_padding() {
        let mut vp = make_vp();
        let pad_px = 50.0_f32;

        let model_bounds = Bounds::new(
            Point::new(Pixels(100.0), Pixels(200.0)),
            Size::new(Pixels(400.0), Pixels(300.0)),
        );

        fit_to_bounds(&mut vp, model_bounds, Padding::Pixels(pad_px));

        // The four corners of the model rect in view space.
        let tl = coords::to_view(&vp, model_bounds.origin);
        let br = coords::to_view(
            &vp,
            Point::new(
                Pixels(model_bounds.origin.x.0 + model_bounds.size.width.0),
                Pixels(model_bounds.origin.y.0 + model_bounds.size.height.0),
            ),
        );

        // Every corner should be at least `pad_px` inside each edge.
        assert!(
            tl.x.0 >= pad_px - 1e-3,
            "left edge too close to screen left: {}",
            tl.x.0
        );
        assert!(
            tl.y.0 >= pad_px - 1e-3,
            "top edge too close to screen top: {}",
            tl.y.0
        );
        assert!(
            br.x.0 <= vp.size.width.0 - pad_px + 1e-3,
            "right edge too close to screen right: {}",
            br.x.0
        );
        assert!(
            br.y.0 <= vp.size.height.0 - pad_px + 1e-3,
            "bottom edge too close to screen bottom: {}",
            br.y.0
        );
    }
}
