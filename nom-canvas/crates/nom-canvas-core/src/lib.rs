#![deny(unsafe_code)]
#![warn(missing_docs)]

//! Core canvas primitives: elements, hit-testing, selection, snapping, spatial
//! indexing, and viewport management for the NomCanvas IDE.

/// Undo/redo command stack for canvas operations.
pub mod commands;
/// Canvas element primitives (rects, ellipses, lines, arrows, graph nodes, wires, connectors).
pub mod elements;
/// Gesture recognizer: tap, double-tap, long-press, pan, and pinch detection.
pub mod gesture;
/// Hit-testing for canvas elements (AABB, rotated rect, ellipse, bezier connector).
pub mod hit_test;
/// Pointer capture state machine (Idle → Pressed → Dragging → Idle).
pub mod pointer;
/// Selection state and rubber-band drag selection.
pub mod selection;
/// Grid snapping and edge/center snap-with-guides.
pub mod snapping;
/// Spatial index for efficient element lookup by position.
pub mod spatial_index;
/// Infinite-canvas viewport: zoom, pan, and coordinate transforms.
pub mod viewport;
pub use viewport::{AabbIndex, SnapGrid, ViewportSnap};
/// WebGPU renderer variant for WASM targets.
pub mod webgpu;
pub use webgpu::{WebGpuConfig, WebGpuPowerPreference, WebGpuRenderer};
/// Frosted-glass render pass descriptor (two-pass blur for panel backgrounds).
pub mod frosted_pass;
pub use frosted_pass::{FrostedPassConfig, FrostedPassState, FrostedRenderPass};
/// Chart primitives: types, data series, config, and chart composition.
pub mod chart;
pub use chart::{Chart, ChartConfig, ChartType, DataSeries};
/// Render pipeline coordinator: phase ordering, draw command queues, and
/// stub frame-graph toward a real wgpu draw loop.
pub mod render_pipeline;
pub use render_pipeline::{DrawCommand, FrameGraph, RenderPhase, RenderPipelineCoordinator, RenderQueue};

#[cfg(test)]
mod integration_tests {
    use crate::elements::{paint_graph_node, paint_wire};
    use crate::elements::{ElementBounds, GraphNodeElement, WireElement};
    use crate::hit_test::hit_test_bounds;
    use crate::selection::RubberBand;
    use crate::viewport::Viewport;
    use nom_gpui::types::{Bounds, Pixels, Point, Size};

    /// Creates a viewport at scale 1.0, creates a rubber band, tests that an
    /// element within viewport bounds intersects the rubber band.
    #[test]
    fn viewport_with_rubber_band_selection() {
        let vp = Viewport::new(800.0, 600.0);
        // Viewport at zoom=1, pan=0: canvas visible from (-400,-300) to (400,300).
        // Create a rubber band that covers the centre of the canvas.
        let mut rb = RubberBand::new([-100.0, -100.0]);
        rb.update([100.0, 100.0]);

        // An element at canvas (0,0) with size 50×50 is well inside the rubber band.
        let elem = ElementBounds {
            id: 1,
            min: [0.0, 0.0],
            max: [50.0, 50.0],
        };

        // The element is inside the viewport visible area.
        assert!(
            vp.is_point_visible([0.0, 0.0]),
            "canvas origin must be visible"
        );
        // The rubber band must intersect the element.
        assert!(
            rb.intersects(&elem),
            "element within viewport bounds must intersect rubber band"
        );
    }

    /// Creates a GraphNodeElement and WireElement, calls paint_graph_node and
    /// paint_wire on the same Scene, verifies combined quad count = 11 (5 node + 6 wire).
    #[test]
    fn elements_paint_to_shared_scene() {
        let node = GraphNodeElement {
            id: 1,
            node_id: 10,
            position: [10.0, 20.0],
            size: [120.0, 60.0],
            label: "TestNode".to_string(),
            confidence: 0.9,
        };
        let wire = WireElement {
            id: 2,
            from_node: 1,
            to_node: 3,
            confidence: 0.85,
            waypoints: vec![[60.0, 50.0]],
        };
        let mut scene = nom_gpui::scene::Scene::new();
        paint_graph_node(&node, &mut scene);
        paint_wire(&wire, [0.0, 0.0], [120.0, 80.0], &mut scene);
        assert_eq!(
            scene.quads.len(),
            11,
            "expected 5 node quads + 6 wire quads = 11 total, got {}",
            scene.quads.len()
        );
    }

    /// Creates a Bounds<Pixels> from nom_gpui::types, calls hit_test_bounds with
    /// a point inside, verifies true.
    #[test]
    fn hit_test_with_gpui_bounds() {
        let bounds: Bounds<Pixels> = Bounds {
            origin: Point {
                x: Pixels(50.0),
                y: Pixels(50.0),
            },
            size: Size {
                width: Pixels(200.0),
                height: Pixels(150.0),
            },
        };
        // Point well inside: (100, 100) is inside [50,50]→[250,200].
        let inside_pt = [100.0_f32, 100.0];
        assert!(
            hit_test_bounds(inside_pt, &bounds),
            "point inside Bounds<Pixels> must return true from hit_test_bounds"
        );
        // Point outside.
        let outside_pt = [300.0_f32, 300.0];
        assert!(
            !hit_test_bounds(outside_pt, &bounds),
            "point outside Bounds<Pixels> must return false from hit_test_bounds"
        );
    }
}
