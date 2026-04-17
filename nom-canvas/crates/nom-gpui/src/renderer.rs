use crate::scene::{FrostedRect, Scene};
use crate::types::Hsla;

// ---------------------------------------------------------------------------
// Color space types
// ---------------------------------------------------------------------------

/// Distinguishes linear and gamma-encoded color spaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
    Linear,
    Gamma,
}

/// Linear RGBA color, stored as four `f32` components suitable for GPU buffers.
///
/// All arithmetic and blending should be performed in linear space.
/// Derives `Pod` + `Zeroable` so it can be cast directly to/from byte slices.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LinearRgba(pub [f32; 4]);

impl From<Hsla> for LinearRgba {
    /// Convert `Hsla` → linear RGBA using a 2.2-power gamma approximation.
    ///
    /// The HSL→RGB conversion produces sRGB values; we then apply
    /// `c^2.2` to each RGB channel to convert to linear light space.
    /// Alpha is passed through unchanged (linear by convention).
    fn from(color: Hsla) -> Self {
        let [r, g, b, a] = hsla_to_rgba(color);
        LinearRgba([r.powf(2.2), g.powf(2.2), b.powf(2.2), a])
    }
}

// ---------------------------------------------------------------------------
// Instance types — GPU-buffer-aligned per-primitive data
// ---------------------------------------------------------------------------

/// Instance data for the quad pipeline (one entry per `Quad`).
///
/// Packed with `#[repr(C)]` to match wgpu vertex/instance buffer layout.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadInstance {
    /// x, y, width, height in screen-space pixels.
    pub bounds: [f32; 4],
    /// RGBA fill color (converted from `Hsla`).
    pub bg_color: [f32; 4],
    pub border_color: [f32; 4],
    /// Per-edge border widths: top, right, bottom, left.
    pub border_widths: [f32; 4],
    /// Per-corner radii: top_left, top_right, bottom_right, bottom_left.
    pub corner_radii: [f32; 4],
}

/// Instance data for the sprite pipelines (mono + polychrome).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteInstance {
    /// x, y, width, height in screen-space pixels.
    pub bounds: [f32; 4],
    /// Atlas UV rect (normalized 0-1): left, top, right, bottom.
    pub tile_rect: [f32; 4],
    /// Tint color for monochrome sprites; `[1,1,1,1]` for polychrome.
    pub color: [f32; 4],
}

/// Global uniforms shared across all pipelines in a frame.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlobalUniforms {
    /// Column-major 4x4 orthographic projection matrix.
    pub projection: [[f32; 4]; 4],
    pub viewport_size: [f32; 2],
    /// Padding to satisfy 16-byte alignment rules.
    pub _pad: [f32; 2],
}

// ---------------------------------------------------------------------------
// PipelineKind — discriminant for the 8 render pipelines
// ---------------------------------------------------------------------------

/// The 8 render pipeline types used by the renderer.
///
/// Each pipeline corresponds to one primitive kind. Two slots are reserved for
/// future extension (e.g. video, custom shader).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineKind {
    Quad = 0,
    MonochromeSprite = 1,
    PolychromeSprite = 2,
    Path = 3,
    Shadow = 4,
    Underline = 5,
    Reserved6 = 6,
    Reserved7 = 7,
}

// ---------------------------------------------------------------------------
// Renderer — depth-less painter's-algorithm GPU renderer
// ---------------------------------------------------------------------------

/// Per-frame draw call counters — updated by each draw_* method.
///
/// Tracks quads, shadows, frosted rects, paths, mono sprites, polychrome
/// sprites, and underlines submitted per frame (cumulative across frames).
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameStats {
    pub quads_drawn: usize,
    pub shadows_drawn: usize,
    pub frosted_drawn: usize,
    pub paths_drawn: usize,
    pub mono_sprites_drawn: usize,
    pub sprites_drawn: usize,
    pub underlines_drawn: usize,
    pub frames: u64,
}

/// Depth-less painter's-algorithm renderer.
///
/// Frame structure:
/// 1. **Shadow pass** (optional) — render shadows to an intermediate blur
///    texture using a two-pass Gaussian kernel.
/// 2. **Main pass** — submit batched primitives per pipeline, in painter's
///    order: shadows → quads → paths → mono sprites → poly sprites → underlines.
///
/// In a real implementation each `pipeline_count` slot holds a
/// `wgpu::RenderPipeline`. They are stubbed here so the crate builds and
/// tests without a GPU context.
pub struct Renderer {
    /// Always 8 — one pipeline per `PipelineKind`.
    pub pipeline_count: usize,
    /// Incremented each time `draw` is called; useful for frame-rate tracking.
    pub frame_count: u64,
    /// Per-frame draw call statistics.
    pub frame_stats: FrameStats,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            pipeline_count: 8,
            frame_count: 0,
            frame_stats: FrameStats::default(),
        }
    }

    /// Returns a reference to the current frame statistics.
    ///
    /// Counters accumulate across frames and cover all primitive types:
    /// quads, shadows, frosted rects, paths, mono sprites, polychrome
    /// sprites, and underlines.
    pub fn stats(&self) -> &FrameStats {
        &self.frame_stats
    }

    /// Submit a complete scene to the GPU.
    ///
    /// Calls `Scene::sort_and_batch` to establish painter's order, then
    /// dispatches each primitive bucket to its dedicated pipeline.
    pub fn draw(&mut self, scene: &mut Scene) {
        scene.sort_and_batch();
        self.frame_count += 1;
        self.frame_stats.frames += 1;

        // Shadow pass must come before the main pass so shadows appear
        // beneath all other content.
        if !scene.shadows.is_empty() {
            self.draw_shadows(scene);
        }

        // Main pass — painter's order.
        self.draw_quads(scene);
        self.draw_paths(scene);
        self.draw_monochrome_sprites(scene);
        self.draw_polychrome_sprites(scene);
        self.draw_underlines(scene);

        // Frosted-glass pass — software approximation via two quads per rect.
        // A real implementation would execute a Gaussian-blur pre-pass here.
        if !scene.frosted_rects.is_empty() {
            self.draw_frosted_rects(&scene.frosted_rects.clone());
        }
    }

    /// Shadow blur pass — renders each shadow to a temporary texture,
    /// applies a Gaussian blur, then composites onto the main surface.
    fn draw_shadows(&mut self, scene: &Scene) {
        self.frame_stats.shadows_drawn += scene.shadows.len();
    }

    /// Quad pipeline — instanced draw with one `QuadInstance` per quad.
    fn draw_quads(&mut self, scene: &Scene) {
        self.frame_stats.quads_drawn += scene.quads.len();
        self.pipeline_count = self.pipeline_count.max(1);
    }

    /// Path pipeline — uploads tessellated PathVertex data per path.
    fn draw_paths(&mut self, scene: &Scene) {
        self.frame_stats.paths_drawn += scene.paths.len();
    }

    /// Monochrome sprite pipeline — single-color glyph atlas sprites.
    fn draw_monochrome_sprites(&mut self, scene: &Scene) {
        self.frame_stats.mono_sprites_drawn += scene.monochrome_sprites.len();
    }

    /// Polychrome sprite pipeline — full-color (emoji / image) atlas sprites.
    fn draw_polychrome_sprites(&mut self, scene: &Scene) {
        self.frame_stats.sprites_drawn += scene.polychrome_sprites.len();
    }

    /// Underline pipeline — thin horizontal line segments with optional wavy
    /// sine-wave modulation.
    fn draw_underlines(&mut self, scene: &Scene) {
        self.frame_stats.underlines_drawn += scene.underlines.len();
    }

    /// Frosted-glass software approximation pass.
    ///
    /// Each `FrostedRect` is decomposed into two `QuadInstance`s:
    /// 1. **Background quad** — neutral grey fill at `bg_alpha` opacity,
    ///    representing the blurred-background tint.
    /// 2. **Border quad** — white border at `border_alpha` opacity, representing
    ///    the highlight rim of the frosted surface.
    ///
    /// A real GPU implementation would run a two-pass Gaussian blur over the
    /// captured framebuffer region before compositing; that is left as a future
    /// extension in the `Reserved6` pipeline slot.
    pub fn draw_frosted_rects(&mut self, rects: &[FrostedRect]) -> Vec<QuadInstance> {
        self.frame_stats.frosted_drawn += rects.len();
        let mut quads = Vec::with_capacity(rects.len() * 2);
        for rect in rects {
            let bounds = [
                rect.bounds.origin.x.0,
                rect.bounds.origin.y.0,
                rect.bounds.size.width.0,
                rect.bounds.size.height.0,
            ];

            // Background quad: neutral mid-grey at bg_alpha opacity.
            quads.push(QuadInstance {
                bounds,
                bg_color: [0.5, 0.5, 0.5, rect.bg_alpha],
                border_color: [0.0, 0.0, 0.0, 0.0],
                border_widths: [0.0; 4],
                corner_radii: [0.0; 4],
            });

            // Border quad: white highlight rim at border_alpha opacity.
            quads.push(QuadInstance {
                bounds,
                bg_color: [0.0, 0.0, 0.0, 0.0],
                border_color: [1.0, 1.0, 1.0, rect.border_alpha],
                border_widths: [1.0; 4],
                corner_radii: [0.0; 4],
            });
        }
        quads
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Color helpers
// ---------------------------------------------------------------------------

/// Convert `Hsla` to a linear RGBA `[f32; 4]` array suitable for GPU uniforms.
///
/// Uses the standard HSL→RGB formula:
/// - achromatic: r = g = b = l
/// - chromatic: hue-to-rgb helper with p/q intermediates
pub fn hsla_to_rgba(color: Hsla) -> [f32; 4] {
    let h = color.h / 360.0;
    let s = color.s;
    let l = color.l;

    let (r, g, b) = if s == 0.0 {
        // Achromatic — all channels equal lightness.
        (l, l, l)
    } else {
        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };
        let p = 2.0 * l - q;

        let hue_to_rgb = |p: f32, q: f32, mut t: f32| -> f32 {
            if t < 0.0 {
                t += 1.0;
            }
            if t > 1.0 {
                t -= 1.0;
            }
            if t < 1.0 / 6.0 {
                return p + (q - p) * 6.0 * t;
            }
            if t < 1.0 / 2.0 {
                return q;
            }
            if t < 2.0 / 3.0 {
                return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
            }
            p
        };

        (
            hue_to_rgb(p, q, h + 1.0 / 3.0),
            hue_to_rgb(p, q, h),
            hue_to_rgb(p, q, h - 1.0 / 3.0),
        )
    };

    [r, g, b, color.a]
}

/// Build a column-major orthographic projection matrix for 2D rendering.
///
/// Maps `(0, 0)` to the top-left corner and `(width, height)` to the
/// bottom-right, with NDC z fixed at 0 (depth-less renderer).
pub fn ortho_projection(width: f32, height: f32) -> [[f32; 4]; 4] {
    [
        [2.0 / width, 0.0, 0.0, 0.0],
        [0.0, -2.0 / height, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [-1.0, 1.0, 0.0, 1.0],
    ]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Hsla;

    #[test]
    fn quad_instance_is_zero_initializable() {
        let q = QuadInstance::default();
        assert_eq!(q.bounds, [0.0f32; 4]);
        assert_eq!(q.bg_color, [0.0f32; 4]);
        assert_eq!(q.border_color, [0.0f32; 4]);
        assert_eq!(q.border_widths, [0.0f32; 4]);
        assert_eq!(q.corner_radii, [0.0f32; 4]);
    }

    #[test]
    fn hsla_to_rgba_black() {
        let rgba = hsla_to_rgba(Hsla::black());
        assert!(
            (rgba[0] - 0.0).abs() < 1e-6,
            "red channel should be 0 for black"
        );
        assert!(
            (rgba[1] - 0.0).abs() < 1e-6,
            "green channel should be 0 for black"
        );
        assert!(
            (rgba[2] - 0.0).abs() < 1e-6,
            "blue channel should be 0 for black"
        );
        assert!(
            (rgba[3] - 1.0).abs() < 1e-6,
            "alpha should be 1 for black"
        );
    }

    #[test]
    fn hsla_to_rgba_white() {
        let rgba = hsla_to_rgba(Hsla::white());
        assert!(
            (rgba[0] - 1.0).abs() < 1e-6,
            "red channel should be 1 for white"
        );
        assert!(
            (rgba[1] - 1.0).abs() < 1e-6,
            "green channel should be 1 for white"
        );
        assert!(
            (rgba[2] - 1.0).abs() < 1e-6,
            "blue channel should be 1 for white"
        );
        assert!(
            (rgba[3] - 1.0).abs() < 1e-6,
            "alpha should be 1 for white"
        );
    }

    #[test]
    fn ortho_projection_produces_non_zero_matrix() {
        let m = ortho_projection(800.0, 600.0);
        // Diagonal elements must be non-zero for a valid projection.
        assert_ne!(m[0][0], 0.0, "m[0][0] (x scale) must be non-zero");
        assert_ne!(m[1][1], 0.0, "m[1][1] (y scale) must be non-zero");
        // Translation column.
        assert_eq!(m[3][0], -1.0, "x translation must be -1");
        assert_eq!(m[3][1], 1.0, "y translation must be 1");
    }

    #[test]
    fn renderer_draw_increments_frame_count() {
        let mut renderer = Renderer::new();
        assert_eq!(renderer.frame_count, 0);
        let mut scene = Scene::new();
        renderer.draw(&mut scene);
        assert_eq!(renderer.frame_count, 1);
        renderer.draw(&mut scene);
        assert_eq!(renderer.frame_count, 2);
    }

    #[test]
    fn renderer_pipeline_count_is_eight() {
        let r = Renderer::new();
        assert_eq!(r.pipeline_count, 8);
    }

    #[test]
    fn renderer_default_equals_new() {
        let r1 = Renderer::new();
        let r2 = Renderer::default();
        assert_eq!(r1.pipeline_count, r2.pipeline_count);
        assert_eq!(r1.frame_count, r2.frame_count);
    }

    #[test]
    fn hsla_to_rgba_red() {
        // Pure red: h=0, s=1, l=0.5, a=1 → [1, 0, 0, 1]
        let rgba = hsla_to_rgba(Hsla { h: 0.0, s: 1.0, l: 0.5, a: 1.0 });
        assert!((rgba[0] - 1.0).abs() < 1e-5, "red channel should be ~1, got {}", rgba[0]);
        assert!((rgba[1] - 0.0).abs() < 1e-5, "green channel should be ~0, got {}", rgba[1]);
        assert!((rgba[2] - 0.0).abs() < 1e-5, "blue channel should be ~0, got {}", rgba[2]);
        assert!((rgba[3] - 1.0).abs() < 1e-5, "alpha should be 1, got {}", rgba[3]);
    }

    #[test]
    fn ortho_projection_identity_check() {
        // ortho_projection(2, 2): x scale = 2/2 = 1, y scale = -2/2 = -1
        // translation: (-1, 1).
        // Point (1, 1) in screen space → NDC (x*scale + tx, y*scale + ty)
        //   = (1*1 + (-1), 1*(-1) + 1) = (0, 0) — center of NDC space.
        let m = ortho_projection(2.0, 2.0);
        let x_scale = m[0][0];
        let y_scale = m[1][1];
        let x_trans = m[3][0];
        let y_trans = m[3][1];
        assert!((x_scale - 1.0).abs() < 1e-6, "x scale should be 1.0, got {x_scale}");
        assert!((y_scale - (-1.0)).abs() < 1e-6, "y scale should be -1.0, got {y_scale}");
        // (1, 1) → NDC (0, 0)
        let ndc_x = 1.0 * x_scale + x_trans;
        let ndc_y = 1.0 * y_scale + y_trans;
        assert!((ndc_x - 0.0).abs() < 1e-6, "NDC x should be 0, got {ndc_x}");
        assert!((ndc_y - 0.0).abs() < 1e-6, "NDC y should be 0, got {ndc_y}");
    }

    #[test]
    fn renderer_new_creates() {
        let r = Renderer::new();
        assert_eq!(r.pipeline_count, 8);
        assert_eq!(r.frame_count, 0);
    }

    #[test]
    fn renderer_draw_frosted_rects_processes_all() {
        use crate::scene::FrostedRect;
        use crate::types::{Bounds, Pixels, Point, Size};

        let mut renderer = Renderer::new();
        let mut scene = Scene::new();

        let make_rect = |x: f32| FrostedRect {
            bounds: Bounds {
                origin: Point { x: Pixels(x), y: Pixels(0.0) },
                size: Size { width: Pixels(100.0), height: Pixels(50.0) },
            },
            blur_radius: 8.0,
            bg_alpha: 0.6,
            border_alpha: 0.4,
        };

        scene.push_frosted_rect(make_rect(0.0));
        scene.push_frosted_rect(make_rect(110.0));
        scene.push_frosted_rect(make_rect(220.0));

        // Must not panic; frame_count must advance.
        renderer.draw(&mut scene);
        assert_eq!(renderer.frame_count, 1);
    }

    #[test]
    fn renderer_frosted_rect_decomposed_to_quads() {
        use crate::scene::FrostedRect;
        use crate::types::{Bounds, Pixels, Point, Size};

        let mut renderer = Renderer::new();
        let rect = FrostedRect {
            bounds: Bounds {
                origin: Point { x: Pixels(10.0), y: Pixels(20.0) },
                size: Size { width: Pixels(200.0), height: Pixels(80.0) },
            },
            blur_radius: 12.0,
            bg_alpha: 0.7,
            border_alpha: 0.3,
        };

        let quads = renderer.draw_frosted_rects(&[rect]);

        // Each FrostedRect must produce exactly 2 QuadInstances.
        assert_eq!(quads.len(), 2, "expected 2 quads (bg + border) per frosted rect");

        // Background quad: non-zero bg_alpha, zero border alpha.
        let bg = &quads[0];
        assert!(
            (bg.bg_color[3] - 0.7).abs() < 1e-6,
            "bg quad alpha should equal bg_alpha (0.7), got {}",
            bg.bg_color[3]
        );
        assert!(
            (bg.border_color[3] - 0.0).abs() < 1e-6,
            "bg quad border_color alpha should be 0"
        );

        // Border quad: zero bg alpha, non-zero border_alpha.
        let border = &quads[1];
        assert!(
            (border.border_color[3] - 0.3).abs() < 1e-6,
            "border quad alpha should equal border_alpha (0.3), got {}",
            border.border_color[3]
        );
        assert!(
            (border.bg_color[3] - 0.0).abs() < 1e-6,
            "border quad bg_color alpha should be 0"
        );

        // Bounds must be forwarded correctly to both quads.
        assert_eq!(bg.bounds, [10.0, 20.0, 200.0, 80.0]);
        assert_eq!(border.bounds, [10.0, 20.0, 200.0, 80.0]);
    }

    #[test]
    fn renderer_stats_tracks_all_primitive_types() {
        use crate::scene::{FrostedRect, MonochromeSprite, Path, PolychromeSprite, Shadow, Underline};
        use crate::types::{Bounds, Pixels, Point, Size};

        let mut renderer = Renderer::new();
        let mut scene = Scene::new();

        scene.push_shadow(Shadow::default());
        scene.push_quad(crate::scene::Quad::default());
        scene.push_path(Path::default());
        scene.push_sprite(MonochromeSprite::default());
        scene.push_poly_sprite(PolychromeSprite::default());
        scene.push_underline(Underline::default());
        scene.push_frosted_rect(FrostedRect {
            bounds: Bounds {
                origin: Point { x: Pixels(0.0), y: Pixels(0.0) },
                size: Size { width: Pixels(10.0), height: Pixels(10.0) },
            },
            blur_radius: 4.0,
            bg_alpha: 0.5,
            border_alpha: 0.3,
        });

        renderer.draw(&mut scene);

        let s = renderer.stats();
        assert_eq!(s.shadows_drawn, 1, "shadows_drawn");
        assert_eq!(s.quads_drawn, 1, "quads_drawn");
        assert_eq!(s.paths_drawn, 1, "paths_drawn");
        assert_eq!(s.mono_sprites_drawn, 1, "mono_sprites_drawn");
        assert_eq!(s.sprites_drawn, 1, "sprites_drawn");
        assert_eq!(s.underlines_drawn, 1, "underlines_drawn");
        assert_eq!(s.frosted_drawn, 1, "frosted_drawn");
    }

    #[test]
    fn renderer_stats_count_draws() {
        use crate::scene::{FrostedRect, Shadow};
        use crate::types::{Bounds, Pixels, Point, Size};
        use crate::scene::Quad;

        let mut renderer = Renderer::new();
        assert_eq!(renderer.stats().frames, 0);
        assert_eq!(renderer.stats().quads_drawn, 0);
        assert_eq!(renderer.stats().shadows_drawn, 0);
        assert_eq!(renderer.stats().frosted_drawn, 0);

        let mut scene = Scene::new();

        // Push 2 quads, 1 shadow, 1 frosted rect.
        scene.push_quad(Quad::default());
        scene.push_quad(Quad::default());
        scene.push_shadow(Shadow::default());
        scene.push_frosted_rect(FrostedRect {
            bounds: Bounds {
                origin: Point { x: Pixels(0.0), y: Pixels(0.0) },
                size: Size { width: Pixels(100.0), height: Pixels(50.0) },
            },
            blur_radius: 4.0,
            bg_alpha: 0.5,
            border_alpha: 0.3,
        });

        renderer.draw(&mut scene);

        assert_eq!(renderer.stats().frames, 1, "one frame rendered");
        assert_eq!(renderer.stats().quads_drawn, 2, "two quads counted");
        assert_eq!(renderer.stats().shadows_drawn, 1, "one shadow counted");
        assert_eq!(renderer.stats().frosted_drawn, 1, "one frosted rect counted");

        // Second draw with empty scene — counters accumulate.
        let mut scene2 = Scene::new();
        scene2.push_quad(Quad::default());
        renderer.draw(&mut scene2);

        assert_eq!(renderer.stats().frames, 2, "two frames after second draw");
        assert_eq!(renderer.stats().quads_drawn, 3, "cumulative quads: 2 + 1");
    }

    // ------------------------------------------------------------------
    // bytemuck Pod/Zeroable tests
    // ------------------------------------------------------------------

    #[test]
    fn quad_instance_pod_cast() {
        use std::mem::size_of;
        let instance = QuadInstance::default();
        let bytes = bytemuck::cast_slice::<QuadInstance, u8>(std::slice::from_ref(&instance));
        assert_eq!(
            bytes.len(),
            size_of::<QuadInstance>(),
            "byte slice length must equal size_of::<QuadInstance>"
        );
    }

    #[test]
    fn sprite_instance_pod_cast() {
        use std::mem::size_of;
        let instance = SpriteInstance::default();
        let bytes = bytemuck::cast_slice::<SpriteInstance, u8>(std::slice::from_ref(&instance));
        assert_eq!(
            bytes.len(),
            size_of::<SpriteInstance>(),
            "byte slice length must equal size_of::<SpriteInstance>"
        );
    }

    #[test]
    fn global_uniforms_pod_cast() {
        use std::mem::size_of;
        let uniforms = GlobalUniforms {
            projection: [[0.0; 4]; 4],
            viewport_size: [0.0; 2],
            _pad: [0.0; 2],
        };
        let bytes = bytemuck::cast_slice::<GlobalUniforms, u8>(std::slice::from_ref(&uniforms));
        assert_eq!(
            bytes.len(),
            size_of::<GlobalUniforms>(),
            "byte slice length must equal size_of::<GlobalUniforms>"
        );
    }

    #[test]
    fn linear_rgba_from_hsla_zero() {
        // Black: h=0, s=0, l=0, a=1 — all RGB channels 0 after gamma, alpha 1.
        let color = LinearRgba::from(Hsla { h: 0.0, s: 0.0, l: 0.0, a: 1.0 });
        assert!((color.0[0] - 0.0).abs() < 1e-6, "r should be 0, got {}", color.0[0]);
        assert!((color.0[1] - 0.0).abs() < 1e-6, "g should be 0, got {}", color.0[1]);
        assert!((color.0[2] - 0.0).abs() < 1e-6, "b should be 0, got {}", color.0[2]);
        assert!((color.0[3] - 1.0).abs() < 1e-6, "a should be 1, got {}", color.0[3]);
    }

    #[test]
    fn linear_rgba_from_hsla_white() {
        // White: h=0, s=0, l=1, a=1 — 1.0^2.2 = 1.0 for all RGB channels.
        let color = LinearRgba::from(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 1.0 });
        assert!((color.0[0] - 1.0).abs() < 1e-5, "r should be ~1, got {}", color.0[0]);
        assert!((color.0[1] - 1.0).abs() < 1e-5, "g should be ~1, got {}", color.0[1]);
        assert!((color.0[2] - 1.0).abs() < 1e-5, "b should be ~1, got {}", color.0[2]);
        assert!((color.0[3] - 1.0).abs() < 1e-5, "a should be 1, got {}", color.0[3]);
    }

    #[test]
    fn frame_stats_default_zeroed() {
        let stats = FrameStats::default();
        assert_eq!(stats.quads_drawn, 0);
        assert_eq!(stats.shadows_drawn, 0);
        assert_eq!(stats.frosted_drawn, 0);
        assert_eq!(stats.paths_drawn, 0);
        assert_eq!(stats.mono_sprites_drawn, 0);
        assert_eq!(stats.sprites_drawn, 0);
        assert_eq!(stats.underlines_drawn, 0);
        assert_eq!(stats.frames, 0);
    }

    #[test]
    fn renderer_draw_increments_frames() {
        let mut renderer = Renderer::new();
        let mut scene = Scene::new();
        renderer.draw(&mut scene);
        assert_eq!(renderer.stats().frames, 1, "frames should be 1 after one draw");
    }

    #[test]
    fn frame_stats_frosted_counted() {
        use crate::scene::FrostedRect;
        use crate::types::{Bounds, Pixels, Point, Size};

        let mut renderer = Renderer::new();
        let mut scene = Scene::new();

        let make_rect = |x: f32| FrostedRect {
            bounds: Bounds {
                origin: Point { x: Pixels(x), y: Pixels(0.0) },
                size: Size { width: Pixels(50.0), height: Pixels(50.0) },
            },
            blur_radius: 4.0,
            bg_alpha: 0.5,
            border_alpha: 0.3,
        };

        scene.push_frosted_rect(make_rect(0.0));
        scene.push_frosted_rect(make_rect(60.0));
        renderer.draw(&mut scene);

        // frosted_drawn counts the number of FrostedRect primitives (2).
        // draw_frosted_rects produces 2 QuadInstances per rect (4 total).
        assert_eq!(
            renderer.stats().frosted_drawn,
            2,
            "frosted_drawn should count 2 FrostedRects"
        );
    }
}
