#![deny(unsafe_code)]

pub mod animation;
pub mod atlas;
pub mod element;
pub mod event;
pub mod focus;
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

pub use scene_diff::{DiffKind, PatchApplier, SceneDiff, SceneNodeId, ScenePatch};
pub use text_layout::{GlyphRun, TextAlign, TextLayoutEngine, TextStyle};
pub use texture_atlas::{AtlasAllocator, AtlasRegion, AtlasShelf, TextureAtlas};
pub use types::*;

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
}
