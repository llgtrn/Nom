use crate::scene::Scene;
use crate::types::Hsla;

// ---------------------------------------------------------------------------
// Instance types — GPU-buffer-aligned per-primitive data
// ---------------------------------------------------------------------------

/// Instance data for the quad pipeline (one entry per `Quad`).
///
/// Packed with `#[repr(C)]` to match wgpu vertex/instance buffer layout.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
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
#[derive(Debug, Clone, Copy, Default)]
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
#[derive(Debug, Clone, Copy)]
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
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            pipeline_count: 8,
            frame_count: 0,
        }
    }

    /// Submit a complete scene to the GPU.
    ///
    /// Calls `Scene::sort_and_batch` to establish painter's order, then
    /// dispatches each primitive bucket to its dedicated pipeline.
    pub fn draw(&mut self, scene: &mut Scene) {
        scene.sort_and_batch();
        self.frame_count += 1;

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
    }

    /// Shadow blur pass — renders each shadow to a temporary texture,
    /// applies a Gaussian blur, then composites onto the main surface.
    fn draw_shadows(&mut self, _scene: &Scene) {
        // Real impl: dispatch wgpu compute pass for Gaussian blur.
    }

    /// Quad pipeline — instanced draw with one `QuadInstance` per quad.
    fn draw_quads(&mut self, _scene: &Scene) {
        // Real impl: upload QuadInstance array to vertex buffer, draw_indexed.
    }

    /// Path pipeline — uploads tessellated PathVertex data per path.
    fn draw_paths(&mut self, _scene: &Scene) {
        // Real impl: tessellate beziers → vertex buffer → draw.
    }

    /// Monochrome sprite pipeline — single-color glyph atlas sprites.
    fn draw_monochrome_sprites(&mut self, _scene: &Scene) {
        // Real impl: upload SpriteInstance array, bind atlas texture, draw.
    }

    /// Polychrome sprite pipeline — full-color (emoji / image) atlas sprites.
    fn draw_polychrome_sprites(&mut self, _scene: &Scene) {
        // Real impl: same as mono but without color tinting uniform.
    }

    /// Underline pipeline — thin horizontal line segments with optional wavy
    /// sine-wave modulation.
    fn draw_underlines(&mut self, _scene: &Scene) {
        // Real impl: upload underline instance data, draw_indexed.
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
}
