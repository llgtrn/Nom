#![deny(unsafe_code)]

pub mod animation;
pub mod atlas;
pub mod render_batch;
pub mod element;
pub mod event;
pub mod focus;
pub mod layer_stack;
pub mod layout;
pub mod pixel_diff;
pub mod platform;
pub mod renderer;
pub mod scene;
pub mod scene_builder;
pub mod scene_diff;
pub mod scene_traversal;
pub mod shaders;
pub mod styled;
pub mod text_layout;
pub mod types;
pub mod texture_atlas;
pub mod window;

pub use render_batch::{BatchKind, BatchSorter, BatchStats, DrawCall, RenderBatch};
pub use scene_diff::{DiffKind, PatchApplier, SceneDiff, SceneNodeId, ScenePatch};
pub use text_layout::{GlyphRun, TextAlign, TextLayoutEngine, TextStyle};
pub use texture_atlas::{AtlasAllocator, AtlasRegion, AtlasShelf, TextureAtlas};
pub use types::*;
pub use layer_stack::{LayerKind, LayerId, Layer, LayerStack, LayerCompositor};

#[cfg(test)]
mod tests {
    #[test]
    fn lib_exports_scene() {
        // nom_gpui::scene::Scene must be accessible via the crate path.
        let _scene: crate::scene::Scene = crate::scene::Scene::new();
        // If this compiles, Scene is reachable from the crate root.
    }

    #[test]
    fn lib_exports_renderer() {
        // nom_gpui::renderer::Renderer must be accessible via the crate path.
        // We verify it is accessible by referencing the type in a way that
        // requires it to be resolvable at compile time.
        fn _accepts_renderer_type() {
            let _: Option<crate::renderer::Renderer> = None;
        }
        _accepts_renderer_type();
    }

    #[test]
    fn window_first_paint_example_compiles() {
        // Smoke test: the example code in examples/window_first_paint.rs
        // must compile. The types and traits exercised here mirror that
        // example without requiring a real window or GPU.
        fn _example_types_compile() {
            use crate::scene::{Quad, Scene};
            use crate::types::{Bounds, Hsla, Pixels, Point, Size};
            use crate::window::{ApplicationHandler, Window, WindowEvent};

            struct _Demo;
            impl ApplicationHandler for _Demo {
                fn resumed(&mut self, window: &mut Window) {
                    window.request_redraw();
                }
                fn window_event(&mut self, window: &mut Window, event: WindowEvent) {
                    if let WindowEvent::CloseRequested = event {
                        window.request_close();
                    }
                }
                fn about_to_wait(&mut self, _window: &mut Window) {}
                fn draw(&mut self, _window: &mut Window, scene: &mut Scene) {
                    scene.push_quad(Quad {
                        bounds: Bounds::new(
                            Point::new(Pixels(100.0), Pixels(100.0)),
                            Size::new(Pixels(200.0), Pixels(200.0)),
                        ),
                        background: Some(Hsla::new(0.0, 1.0, 0.5, 1.0)),
                        ..Default::default()
                    });
                }
            }
        }
        _example_types_compile();
    }

    #[test]
    fn renderer_draw_populates_pending_quads() {
        let mut renderer = crate::renderer::Renderer::new();
        let mut scene = crate::scene::Scene::new();
        scene.push_quad(crate::scene::Quad {
            bounds: crate::types::Bounds::new(
                crate::types::Point::new(crate::types::Pixels(100.0), crate::types::Pixels(100.0)),
                crate::types::Size::new(crate::types::Pixels(200.0), crate::types::Pixels(200.0)),
            ),
            background: Some(crate::types::Hsla::new(0.0, 1.0, 0.5, 1.0)),
            ..Default::default()
        });
        renderer.draw(&mut scene);
        assert!(
            !renderer.pending_quads().is_empty(),
            "draw must populate pending_quads"
        );
        assert_eq!(renderer.pending_quads().len(), 1);
    }
}
