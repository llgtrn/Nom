//! Viewport — model-space center + zoom + pixel size.
//!
//! The viewport is the lens through which the infinite canvas is observed.
//! It holds three pieces of state:
//!
//! - `center`  — the model-space point that appears at the middle of the screen.
//! - `zoom`    — scale factor: 1 model unit → `zoom` pixels.
//! - `size`    — the current pixel dimensions of the rendering surface.
//!
//! All coordinate-transform helpers live in [`crate::coords`].

#![deny(unsafe_code)]

use nom_gpui::{Pixels, Point, Size};

/// The active view of the canvas.
#[derive(Clone, Debug)]
pub struct Viewport {
    /// Model-space point shown at the centre of the screen.
    pub center: Point<Pixels>,
    /// Scale factor: 1 model unit → `zoom` screen pixels.  Clamped to
    /// `[ZOOM_MIN, ZOOM_MAX]` on every write.
    pub zoom: f32,
    /// Pixel dimensions of the rendering surface.
    pub size: Size<Pixels>,
}

impl Viewport {
    /// Minimum zoom (10 % — enough to see very large diagrams).
    pub const ZOOM_MIN: f32 = 0.1;
    /// Maximum zoom (10 × — deeper than most canvas tools for map-like views).
    pub const ZOOM_MAX: f32 = 10.0;

    /// Create a viewport centred on the model origin with zoom = 1.
    pub fn new(size: Size<Pixels>) -> Self {
        Self {
            center: Point::new(Pixels(0.0), Pixels(0.0)),
            zoom: 1.0,
            size,
        }
    }

    /// Move the model-space centre shown at the middle of the screen.
    pub fn center_on(&mut self, model_point: Point<Pixels>) {
        self.center = model_point;
    }

    /// Set the zoom level, clamping to `[ZOOM_MIN, ZOOM_MAX]`.
    pub fn set_zoom(&mut self, z: f32) {
        self.zoom = z.clamp(Self::ZOOM_MIN, Self::ZOOM_MAX);
    }

    /// Update the pixel dimensions of the rendering surface (e.g. on window
    /// resize).  Does not move the centre.
    pub fn set_size(&mut self, size: Size<Pixels>) {
        self.size = size;
    }
}

// ─── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_vp() -> Viewport {
        Viewport::new(Size::new(Pixels(800.0), Pixels(600.0)))
    }

    #[test]
    fn new_has_zoom_1() {
        let vp = make_vp();
        assert!((vp.zoom - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn set_zoom_clamps_to_range() {
        let mut vp = make_vp();

        vp.set_zoom(0.0);
        assert!(
            (vp.zoom - Viewport::ZOOM_MIN).abs() < f32::EPSILON,
            "below ZOOM_MIN should clamp to ZOOM_MIN"
        );

        vp.set_zoom(999.0);
        assert!(
            (vp.zoom - Viewport::ZOOM_MAX).abs() < f32::EPSILON,
            "above ZOOM_MAX should clamp to ZOOM_MAX"
        );

        vp.set_zoom(2.5);
        assert!(
            (vp.zoom - 2.5).abs() < f32::EPSILON,
            "in-range value should be stored as-is"
        );
    }
}
