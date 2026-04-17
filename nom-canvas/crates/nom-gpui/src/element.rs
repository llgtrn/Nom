//! Element trait: the three-phase lifecycle that every UI node follows.
//!
//! 1. `request_layout` — ask the layout engine for a slot, return opaque state.
//! 2. `prepaint`       — receive final bounds, commit hit regions, save paint data.
//! 3. `paint`          — emit primitives into the [`Scene`].
//!
//! State types are **caller-owned**: the element declares `RequestLayoutState`
//! and `PrepaintState` as associated types, returns them from each phase, and
//! the framework threads them forward. No global mutable element state.

use crate::geometry::{Bounds, ScaledPixels};
use crate::scene::Scene;
use crate::taffy_layout::{LayoutEngine, LayoutId};

/// Stable identifier for an element across frames.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ElementId(pub u64);

/// Passed to every lifecycle method; holds rem size, scale factor, layout engine.
pub struct ElementCx<'a> {
    pub layout: &'a mut LayoutEngine,
    /// Root font-size in logical pixels. Used to convert rem units in `Style` to absolute pixels.
    pub rem_size: f32,
    /// DPI scale factor (e.g. 2.0 on Retina displays). Multiplies logical coords to device coords.
    pub scale_factor: f32,
}

impl<'a> std::fmt::Debug for ElementCx<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ElementCx")
            .field("rem_size", &self.rem_size)
            .field("scale_factor", &self.scale_factor)
            .finish_non_exhaustive()
    }
}

impl<'a> ElementCx<'a> {
    pub fn new(layout: &'a mut LayoutEngine, rem_size: f32, scale_factor: f32) -> Self {
        Self {
            layout,
            rem_size,
            scale_factor,
        }
    }
}

/// Every UI node implements this trait.
pub trait Element {
    /// State handed from `request_layout` to `prepaint`.
    type RequestLayoutState;
    /// State handed from `prepaint` to `paint`.
    type PrepaintState;

    /// Optional stable identifier (enables persistent state / focus).
    fn id(&self) -> Option<ElementId> {
        None
    }

    /// Phase 1: register with the layout engine, return a LayoutId and state.
    fn request_layout(&mut self, cx: &mut ElementCx) -> (LayoutId, Self::RequestLayoutState);

    /// Phase 2: receive resolved bounds, save anything needed for paint.
    fn prepaint(
        &mut self,
        bounds: Bounds<ScaledPixels>,
        request_state: &mut Self::RequestLayoutState,
        cx: &mut ElementCx,
    ) -> Self::PrepaintState;

    /// Phase 3: emit primitives into the scene.
    fn paint(
        &mut self,
        bounds: Bounds<ScaledPixels>,
        request_state: &mut Self::RequestLayoutState,
        prepaint_state: &mut Self::PrepaintState,
        scene: &mut Scene,
        cx: &mut ElementCx,
    );
}

/// Run all three phases on an element. Returns the element's final bounds.
pub fn draw_element<E: Element>(
    element: &mut E,
    scene: &mut Scene,
    cx: &mut ElementCx,
) -> Bounds<ScaledPixels> {
    let (layout_id, mut request_state) = element.request_layout(cx);
    let bounds = cx.layout.resolve_bounds(layout_id);
    let mut prepaint_state = element.prepaint(bounds, &mut request_state, cx);
    element.paint(bounds, &mut request_state, &mut prepaint_state, scene, cx);
    bounds
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::LinearRgba;
    use crate::geometry::{Corners, Point, Size};
    use crate::scene::Quad;
    use crate::style::Style;

    struct SimpleQuad {
        style: Style,
    }

    impl Element for SimpleQuad {
        type RequestLayoutState = ();
        type PrepaintState = ();

        fn request_layout(&mut self, cx: &mut ElementCx) -> (LayoutId, ()) {
            let id = cx.layout.request_layout(&self.style, &[]);
            (id, ())
        }

        fn prepaint(
            &mut self,
            _: Bounds<ScaledPixels>,
            _: &mut (),
            _: &mut ElementCx,
        ) {
        }

        fn paint(
            &mut self,
            bounds: Bounds<ScaledPixels>,
            _: &mut (),
            _: &mut (),
            scene: &mut Scene,
            _: &mut ElementCx,
        ) {
            scene.insert_quad(Quad {
                order: 0,
                bounds,
                clip_bounds: bounds,
                corner_radii: Corners::all(ScaledPixels(0.0)),
                background: self.style.background.unwrap_or(LinearRgba::TRANSPARENT),
                border_color: LinearRgba::TRANSPARENT,
                border_widths: [ScaledPixels(0.0); 4],
            });
        }
    }

    #[test]
    fn three_phase_lifecycle_emits_one_quad() {
        let mut layout = LayoutEngine::new();
        let mut cx = ElementCx::new(&mut layout, 16.0, 1.0);
        let mut scene = Scene::new();
        let mut q = SimpleQuad {
            style: Style {
                width: crate::style::Length::Pixels(crate::geometry::Pixels(50.0)),
                height: crate::style::Length::Pixels(crate::geometry::Pixels(30.0)),
                background: Some(LinearRgba::WHITE),
                ..Default::default()
            },
        };
        let bounds = draw_element(&mut q, &mut scene, &mut cx);
        // Compute layout — bounds come from the engine. For unit test, verify primitive was emitted.
        assert_eq!(scene.quads.len(), 1);
        // Bounds originate at zero; size should be at least (0, 0) and at most (50, 30).
        assert!(bounds.size.width.0 >= 0.0);
        // Prevent unused_mut on bounds.
        let _ = bounds;
        let _ = Point::new(0.0, 0.0);
        let _ = Size::new(0.0, 0.0);
    }
}
