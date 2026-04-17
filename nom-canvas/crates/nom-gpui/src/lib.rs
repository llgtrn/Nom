//! `nom-gpui` — GPU-native scene graph, layout, and element framework.
//!
//! Architecture replicated from Zed's GPUI (scene.rs, bounds_tree.rs,
//! element.rs, taffy.rs, platform.rs) with a clean Nom-native implementation:
//! zero foreign identities, zero wrappers, zero adapters.
//!
//! # Modules
//!
//! - [`geometry`]  — `Point`, `Size`, `Bounds`, `Pixels`, `TransformationMatrix`.
//! - [`color`]     — `Rgba`, `Hsla`, alpha compositing.
//! - [`bounds_tree`] — R-tree assigning stable `DrawOrder` to paint calls.
//! - [`scene`]     — 7 typed primitive collections with batched iteration.
//! - [`atlas`]     — Texture atlas trait + in-memory implementation.
//! - [`style`]     — Layout + paint style (converts to `taffy::Style`).
//! - [`styled`]    — Fluent builder trait (`.flex_col().padding(8.0).bg(...)`).
//! - [`element`]   — Three-phase lifecycle trait.
//! - [`taffy_layout`] — Thin wrapper over `taffy::TaffyTree`.
//!
//! See `docs/superpowers/specs/2026-04-17-nomcanvas-gpui-design.md`.

pub mod shaders;
pub mod atlas;
pub mod text;
pub mod wgpu_atlas;
pub mod buffers;
pub mod context;
#[cfg(feature = "native")]
pub mod device_lost;
#[cfg(feature = "native")]
pub mod frame_loop;
pub mod pipelines;
pub mod renderer;
pub mod bounds_tree;
pub mod color;
pub mod element;
pub mod geometry;
pub mod scene;
pub mod style;
pub mod styled;
pub mod taffy_layout;
#[cfg(feature = "native")]
pub mod window;

pub use atlas::{AtlasKey, AtlasTextureId, AtlasTextureKind, AtlasTile, InMemoryAtlas, PlatformAtlas};
pub use bounds_tree::{BoundsTree, DrawOrder};
pub use color::{Hsla, Rgba};
pub use element::{draw_element, Element, ElementCx, ElementId};
pub use geometry::{
    Bounds, Corners, DevicePixels, Edges, Pixels, Point, ScaledPixels, Size, TransformationMatrix,
};
pub use scene::{
    AtlasTileRef, HitResult, MonochromeSprite, Path, PolychromeSprite, PrimitiveBatch, Quad,
    Scene, Shadow, SubpixelSprite, Underline,
};
#[cfg(feature = "native")]
pub use frame_loop::ElementStateMap;
pub use style::{AlignItems, Display, FlexDirection, JustifyContent, Length, Overflow, Style};
pub use styled::{Styled, StyledBox};
pub use taffy_layout::{LayoutEngine, LayoutError, LayoutId, MeasureFn, NodeContext};

// ── CI skip helper ────────────────────────────────────────────────────────────

/// Returns `true` when GPU-dependent or display-dependent tests should be
/// skipped in the current environment.
///
/// Conditions that trigger a skip:
/// - `NOM_SKIP_GPU_TESTS` env-var is set (explicit opt-out, any value).
/// - On Linux, neither `DISPLAY` nor `WAYLAND_DISPLAY` is set — this is the
///   default state on GitHub Actions `ubuntu-latest` headless runners where
///   winit's `EventLoop::new()` would otherwise panic.
///
/// On Windows and macOS, this function always returns `false` so tests run
/// normally on those platforms.
///
/// Integration tests in `tests/` can call this as `nom_gpui::should_skip_gpu_tests()`.
#[doc(hidden)]
pub fn should_skip_gpu_tests() -> bool {
    if std::env::var("NOM_SKIP_GPU_TESTS").is_ok() {
        return true;
    }
    #[cfg(target_os = "linux")]
    {
        if std::env::var("DISPLAY").is_err() && std::env::var("WAYLAND_DISPLAY").is_err() {
            return true;
        }
    }
    false
}
