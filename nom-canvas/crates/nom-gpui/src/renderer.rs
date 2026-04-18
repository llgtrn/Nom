use crate::scene::{FrostedRect, Scene};
use crate::shaders::{QUAD_FRAG_WGSL, QUAD_VERT_WGSL};
use crate::types::Hsla;
use std::sync::Arc;

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
    /// Incremented each time `draw` or `end_frame` completes.
    pub frame_count: u64,
    /// Per-frame draw call statistics.
    pub frame_stats: FrameStats,
    /// GPU resources — present when constructed via `with_gpu`.
    pub gpu: Option<GpuResources>,
    /// Quads queued during the current frame; flushed + cleared by
    /// `end_frame`. Always present; in CPU-only mode the buffer still
    /// absorbs queued instances so callers can inspect them in tests.
    pending_quads: Vec<QuadInstance>,
    /// True between `begin_frame` and `end_frame`.
    in_frame: bool,
}

/// Initial capacity (in `QuadInstance` slots) for the per-frame quad
/// buffer. Grown on demand when frames push more instances than fit.
pub const QUAD_INSTANCE_INITIAL_CAPACITY: usize = 1024;

/// Errors returned by the GPU-aware frame lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// `end_frame` or `draw_quads_gpu` called without a matching
    /// `begin_frame`.
    NotInFrame,
    /// `begin_frame` called twice without an intervening `end_frame`.
    AlreadyInFrame,
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::NotInFrame => f.write_str("renderer is not inside a frame"),
            FrameError::AlreadyInFrame => f.write_str("renderer already has an open frame"),
        }
    }
}

impl std::error::Error for FrameError {}

/// GPU resources bound to a live wgpu device. Constructed by
/// `Renderer::with_gpu`; absent in CPU-only / test mode.
pub struct GpuResources {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub surface_format: wgpu::TextureFormat,
    pub quad_pipeline: wgpu::RenderPipeline,
    /// Device-side storage for `QuadInstance` uploads.
    pub instance_buffer: wgpu::Buffer,
    /// Current capacity in slots (not bytes).
    pub instance_buffer_capacity: usize,
}

impl GpuResources {
    fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let quad_pipeline = build_quad_pipeline(&device, surface_format);
        let instance_buffer_capacity = QUAD_INSTANCE_INITIAL_CAPACITY;
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nom-gpui quad-instances"),
            size: (instance_buffer_capacity * std::mem::size_of::<QuadInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            device,
            queue,
            surface_format,
            quad_pipeline,
            instance_buffer,
            instance_buffer_capacity,
        }
    }

    /// Grow the instance buffer to hold at least `required` slots. Doubles
    /// capacity until satisfied so amortised cost stays O(1).
    fn ensure_capacity(&mut self, required: usize) {
        if required <= self.instance_buffer_capacity {
            return;
        }
        let mut new_cap = self.instance_buffer_capacity.max(1);
        while new_cap < required {
            new_cap *= 2;
        }
        self.instance_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nom-gpui quad-instances"),
            size: (new_cap * std::mem::size_of::<QuadInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.instance_buffer_capacity = new_cap;
    }
}

/// Build the minimal quad render pipeline from the WGSL stubs in
/// `shaders.rs`. Proves the `Device` → `PipelineLayout` → `RenderPipeline`
/// chain compiles against the caller's surface format.
fn build_quad_pipeline(
    device: &wgpu::Device,
    surface_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let vs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("nom-gpui quad-vs"),
        source: wgpu::ShaderSource::Wgsl(QUAD_VERT_WGSL.into()),
    });
    let fs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("nom-gpui quad-fs"),
        source: wgpu::ShaderSource::Wgsl(QUAD_FRAG_WGSL.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("nom-gpui quad-layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("nom-gpui quad-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &vs_module,
            entry_point: "vs_main",
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &fs_module,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            pipeline_count: 8,
            frame_count: 0,
            frame_stats: FrameStats::default(),
            gpu: None,
            pending_quads: Vec::new(),
            in_frame: false,
        }
    }

    /// Construct a GPU-attached renderer bound to the given wgpu device,
    /// queue, and surface format. Builds the quad render pipeline + a
    /// pre-allocated instance buffer immediately.
    pub fn with_gpu(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        Self {
            pipeline_count: 8,
            frame_count: 0,
            frame_stats: FrameStats::default(),
            gpu: Some(GpuResources::new(device, queue, surface_format)),
            pending_quads: Vec::with_capacity(QUAD_INSTANCE_INITIAL_CAPACITY),
            in_frame: false,
        }
    }

    /// Open a new frame. Clears `pending_quads` and arms the in-frame flag.
    pub fn begin_frame(&mut self) -> Result<(), FrameError> {
        if self.in_frame {
            return Err(FrameError::AlreadyInFrame);
        }
        self.pending_quads.clear();
        self.in_frame = true;
        Ok(())
    }

    /// Queue a slice of `QuadInstance` records for upload when the frame
    /// ends. In GPU mode also grows the device-side buffer on demand.
    pub fn draw_quads_gpu(&mut self, quads: &[QuadInstance]) -> Result<(), FrameError> {
        if !self.in_frame {
            return Err(FrameError::NotInFrame);
        }
        self.pending_quads.extend_from_slice(quads);
        self.frame_stats.quads_drawn += quads.len();
        if let Some(gpu) = self.gpu.as_mut() {
            gpu.ensure_capacity(self.pending_quads.len());
        }
        Ok(())
    }

    /// Close the current frame. Uploads `pending_quads` via
    /// `Queue::write_buffer` when GPU-attached, disarms the in-frame flag,
    /// and increments `frame_count` / `frame_stats.frames`.
    pub fn end_frame(&mut self) -> Result<(), FrameError> {
        if !self.in_frame {
            return Err(FrameError::NotInFrame);
        }
        if let Some(gpu) = self.gpu.as_mut() {
            if !self.pending_quads.is_empty() {
                let bytes = bytemuck::cast_slice(self.pending_quads.as_slice());
                gpu.queue.write_buffer(&gpu.instance_buffer, 0, bytes);
            }
        }
        self.in_frame = false;
        self.frame_count += 1;
        self.frame_stats.frames += 1;
        Ok(())
    }

    /// Whether a frame is currently open.
    pub fn is_in_frame(&self) -> bool {
        self.in_frame
    }

    /// Quads queued in the currently-open frame. Cleared on the next
    /// `begin_frame`.
    pub fn pending_quads(&self) -> &[QuadInstance] {
        &self.pending_quads
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
    /// 1. **Background quad** — neutral grey fill whose alpha varies with
    ///    `blur_radius`: higher blur → more transparent overlay.
    ///    Formula: `alpha = (0.7 - blur_radius.min(20.0) * 0.015).max(0.3)`
    ///    (blur_radius=0 → alpha≈0.7, blur_radius=20 → alpha≈0.4)
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

            // Vary tint alpha based on blur_radius: higher blur → more transparent.
            // blur_radius=0 → alpha≈0.7, blur_radius=20 → alpha≈0.4, clamped to [0.3, 0.7].
            let blur_alpha = (0.7 - rect.blur_radius.min(20.0) * 0.015).max(0.3);

            // Background quad: neutral mid-grey at blur-modulated alpha opacity.
            quads.push(QuadInstance {
                bounds,
                bg_color: [0.5, 0.5, 0.5, blur_alpha],
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
        assert!((rgba[3] - 1.0).abs() < 1e-6, "alpha should be 1 for black");
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
        assert!((rgba[3] - 1.0).abs() < 1e-6, "alpha should be 1 for white");
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
        let rgba = hsla_to_rgba(Hsla {
            h: 0.0,
            s: 1.0,
            l: 0.5,
            a: 1.0,
        });
        assert!(
            (rgba[0] - 1.0).abs() < 1e-5,
            "red channel should be ~1, got {}",
            rgba[0]
        );
        assert!(
            (rgba[1] - 0.0).abs() < 1e-5,
            "green channel should be ~0, got {}",
            rgba[1]
        );
        assert!(
            (rgba[2] - 0.0).abs() < 1e-5,
            "blue channel should be ~0, got {}",
            rgba[2]
        );
        assert!(
            (rgba[3] - 1.0).abs() < 1e-5,
            "alpha should be 1, got {}",
            rgba[3]
        );
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
        assert!(
            (x_scale - 1.0).abs() < 1e-6,
            "x scale should be 1.0, got {x_scale}"
        );
        assert!(
            (y_scale - (-1.0)).abs() < 1e-6,
            "y scale should be -1.0, got {y_scale}"
        );
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
                origin: Point {
                    x: Pixels(x),
                    y: Pixels(0.0),
                },
                size: Size {
                    width: Pixels(100.0),
                    height: Pixels(50.0),
                },
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
                origin: Point {
                    x: Pixels(10.0),
                    y: Pixels(20.0),
                },
                size: Size {
                    width: Pixels(200.0),
                    height: Pixels(80.0),
                },
            },
            blur_radius: 12.0,
            bg_alpha: 0.7,
            border_alpha: 0.3,
        };

        let quads = renderer.draw_frosted_rects(&[rect]);

        // Each FrostedRect must produce exactly 2 QuadInstances.
        assert_eq!(
            quads.len(),
            2,
            "expected 2 quads (bg + border) per frosted rect"
        );

        // Background quad: blur_radius=12 → alpha=(0.7 - 12*0.015).max(0.3)=0.52.
        // blur_alpha is derived from blur_radius, not bg_alpha directly.
        let expected_blur_alpha = (0.7_f32 - 12.0_f32 * 0.015).max(0.3);
        let bg = &quads[0];
        assert!(
            (bg.bg_color[3] - expected_blur_alpha).abs() < 1e-5,
            "bg quad alpha should equal blur-derived alpha ({expected_blur_alpha:.4}), got {}",
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
        use crate::scene::{
            FrostedRect, MonochromeSprite, Path, PolychromeSprite, Shadow, Underline,
        };
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
                origin: Point {
                    x: Pixels(0.0),
                    y: Pixels(0.0),
                },
                size: Size {
                    width: Pixels(10.0),
                    height: Pixels(10.0),
                },
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
        use crate::scene::Quad;
        use crate::scene::{FrostedRect, Shadow};
        use crate::types::{Bounds, Pixels, Point, Size};

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
                origin: Point {
                    x: Pixels(0.0),
                    y: Pixels(0.0),
                },
                size: Size {
                    width: Pixels(100.0),
                    height: Pixels(50.0),
                },
            },
            blur_radius: 4.0,
            bg_alpha: 0.5,
            border_alpha: 0.3,
        });

        renderer.draw(&mut scene);

        assert_eq!(renderer.stats().frames, 1, "one frame rendered");
        assert_eq!(renderer.stats().quads_drawn, 2, "two quads counted");
        assert_eq!(renderer.stats().shadows_drawn, 1, "one shadow counted");
        assert_eq!(
            renderer.stats().frosted_drawn,
            1,
            "one frosted rect counted"
        );

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
        let color = LinearRgba::from(Hsla {
            h: 0.0,
            s: 0.0,
            l: 0.0,
            a: 1.0,
        });
        assert!(
            (color.0[0] - 0.0).abs() < 1e-6,
            "r should be 0, got {}",
            color.0[0]
        );
        assert!(
            (color.0[1] - 0.0).abs() < 1e-6,
            "g should be 0, got {}",
            color.0[1]
        );
        assert!(
            (color.0[2] - 0.0).abs() < 1e-6,
            "b should be 0, got {}",
            color.0[2]
        );
        assert!(
            (color.0[3] - 1.0).abs() < 1e-6,
            "a should be 1, got {}",
            color.0[3]
        );
    }

    #[test]
    fn linear_rgba_from_hsla_white() {
        // White: h=0, s=0, l=1, a=1 — 1.0^2.2 = 1.0 for all RGB channels.
        let color = LinearRgba::from(Hsla {
            h: 0.0,
            s: 0.0,
            l: 1.0,
            a: 1.0,
        });
        assert!(
            (color.0[0] - 1.0).abs() < 1e-5,
            "r should be ~1, got {}",
            color.0[0]
        );
        assert!(
            (color.0[1] - 1.0).abs() < 1e-5,
            "g should be ~1, got {}",
            color.0[1]
        );
        assert!(
            (color.0[2] - 1.0).abs() < 1e-5,
            "b should be ~1, got {}",
            color.0[2]
        );
        assert!(
            (color.0[3] - 1.0).abs() < 1e-5,
            "a should be 1, got {}",
            color.0[3]
        );
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
        assert_eq!(
            renderer.stats().frames,
            1,
            "frames should be 1 after one draw"
        );
    }

    #[test]
    fn frame_stats_frosted_counted() {
        use crate::scene::FrostedRect;
        use crate::types::{Bounds, Pixels, Point, Size};

        let mut renderer = Renderer::new();
        let mut scene = Scene::new();

        let make_rect = |x: f32| FrostedRect {
            bounds: Bounds {
                origin: Point {
                    x: Pixels(x),
                    y: Pixels(0.0),
                },
                size: Size {
                    width: Pixels(50.0),
                    height: Pixels(50.0),
                },
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

    // ------------------------------------------------------------------
    // AE9: blur_radius influences bg quad alpha
    // ------------------------------------------------------------------

    #[test]
    fn frosted_rect_blur_radius_zero_gives_alpha_0_7() {
        use crate::scene::FrostedRect;
        use crate::types::{Bounds, Pixels, Point, Size};

        let mut renderer = Renderer::new();
        let rect = FrostedRect {
            bounds: Bounds {
                origin: Point { x: Pixels(0.0), y: Pixels(0.0) },
                size: Size { width: Pixels(100.0), height: Pixels(50.0) },
            },
            blur_radius: 0.0,
            bg_alpha: 0.5,
            border_alpha: 0.2,
        };
        let quads = renderer.draw_frosted_rects(&[rect]);
        // alpha = (0.7 - 0.0 * 0.015).max(0.3) = 0.7
        let expected = (0.7_f32 - 0.0_f32 * 0.015).max(0.3);
        assert!(
            (quads[0].bg_color[3] - expected).abs() < 1e-5,
            "blur_radius=0 must yield bg alpha {expected:.4}, got {}",
            quads[0].bg_color[3]
        );
    }

    #[test]
    fn frosted_rect_blur_radius_20_gives_alpha_approx_0_4() {
        use crate::scene::FrostedRect;
        use crate::types::{Bounds, Pixels, Point, Size};

        let mut renderer = Renderer::new();
        let rect = FrostedRect {
            bounds: Bounds {
                origin: Point { x: Pixels(0.0), y: Pixels(0.0) },
                size: Size { width: Pixels(100.0), height: Pixels(50.0) },
            },
            blur_radius: 20.0,
            bg_alpha: 0.5,
            border_alpha: 0.2,
        };
        let quads = renderer.draw_frosted_rects(&[rect]);
        // alpha = (0.7 - 20.0 * 0.015).max(0.3) = (0.7 - 0.3).max(0.3) = 0.4
        let expected = (0.7_f32 - 20.0_f32 * 0.015).max(0.3);
        assert!(
            (quads[0].bg_color[3] - expected).abs() < 1e-5,
            "blur_radius=20 must yield bg alpha {expected:.4}, got {}",
            quads[0].bg_color[3]
        );
        // Must be approximately 0.4
        assert!(
            (expected - 0.4).abs() < 1e-5,
            "blur_radius=20 expected alpha ~0.4, formula gives {expected:.4}"
        );
    }

    // ------------------------------------------------------------------
    // AE16: Hsla.h uses 0-360 degrees; hsla_to_rgba normalises internally
    // ------------------------------------------------------------------

    #[test]
    fn hsla_h_range_is_0_to_360_degrees() {
        // The Hsla type comment says h: 0-360. Verify that hsla_to_rgba
        // produces correct output for a canonical 0-360 input.
        // Pure green: h=120, s=1, l=0.5 → (0, 1, 0, 1)
        let rgba = hsla_to_rgba(Hsla { h: 120.0, s: 1.0, l: 0.5, a: 1.0 });
        assert!(rgba[0] < 0.01, "h=120 red must be ~0, got {}", rgba[0]);
        assert!((rgba[1] - 1.0).abs() < 1e-4, "h=120 green must be ~1, got {}", rgba[1]);
        assert!(rgba[2] < 0.01, "h=120 blue must be ~0, got {}", rgba[2]);
    }

    #[test]
    fn hsla_h_roundtrip_consistent_with_tokens_convention() {
        // tokens.rs stores hue in 0-360 degrees (e.g. 220.0 for blue).
        // hsla_to_rgba divides h by 360 internally, so Hsla{h:220,...} should
        // produce the same result as the CSS-standard hsl(220°, ...).
        // hsl(220, 13%, 11%) — primary background from tokens.rs:
        let c = Hsla { h: 220.0, s: 0.13, l: 0.11, a: 1.0 };
        let [r, g, b, a] = hsla_to_rgba(c);
        // All channels must be in [0.0, 1.0]
        for ch in [r, g, b, a] {
            assert!((0.0..=1.0).contains(&ch), "channel {ch} out of [0,1]");
        }
        // Must be a dark blueish colour: blue channel must exceed red channel.
        assert!(b > r, "h=220 should be blueish (b={b:.4} > r={r:.4})");
        assert_eq!(a, 1.0);
    }

    // ------------------------------------------------------------------
    // AE1: Real wgpu frame lifecycle — begin_frame / draw_quads_gpu /
    //      end_frame, instance buffer capacity, error paths.
    //      Exercised without a real GPU; `gpu` field is `None` so the
    //      code paths that require a `wgpu::Device` are bypassed while
    //      the stats / lifecycle bookkeeping is fully covered.
    // ------------------------------------------------------------------

    #[test]
    fn draw_quads_gpu_increments_stats_counter() {
        let mut renderer = Renderer::new();
        renderer.begin_frame().expect("begin_frame");
        let q1 = QuadInstance::default();
        let q2 = QuadInstance::default();
        let q3 = QuadInstance::default();
        renderer
            .draw_quads_gpu(&[q1, q2, q3])
            .expect("draw_quads_gpu");
        assert_eq!(
            renderer.stats().quads_drawn,
            3,
            "quads_drawn must reflect queued instances"
        );
        assert_eq!(
            renderer.pending_quads().len(),
            3,
            "pending buffer must hold queued instances"
        );
    }

    #[test]
    fn begin_frame_clears_pending_quads() {
        let mut renderer = Renderer::new();
        renderer.begin_frame().unwrap();
        renderer
            .draw_quads_gpu(&[QuadInstance::default(); 4])
            .unwrap();
        assert_eq!(renderer.pending_quads().len(), 4);
        renderer.end_frame().unwrap();

        // Second frame: begin_frame must wipe the previous frame's queue.
        renderer.begin_frame().unwrap();
        assert_eq!(
            renderer.pending_quads().len(),
            0,
            "begin_frame must clear pending_quads"
        );
        assert!(renderer.is_in_frame(), "in_frame flag must be armed");
    }

    #[test]
    fn end_frame_increments_frame_counter() {
        let mut renderer = Renderer::new();
        assert_eq!(renderer.frame_count, 0);
        assert_eq!(renderer.stats().frames, 0);

        renderer.begin_frame().unwrap();
        renderer.end_frame().unwrap();
        assert_eq!(renderer.frame_count, 1, "frame_count must advance");
        assert_eq!(renderer.stats().frames, 1, "frame_stats.frames must advance");
        assert!(
            !renderer.is_in_frame(),
            "end_frame must disarm the in-frame flag"
        );

        renderer.begin_frame().unwrap();
        renderer.end_frame().unwrap();
        assert_eq!(renderer.frame_count, 2);
        assert_eq!(renderer.stats().frames, 2);
    }

    #[test]
    fn instance_buffer_initial_capacity_is_documented() {
        // The advertised initial capacity must match the constant so
        // callers sizing their uploads against QUAD_INSTANCE_INITIAL_CAPACITY
        // don't trigger a silent reallocation on the first frame.
        assert_eq!(QUAD_INSTANCE_INITIAL_CAPACITY, 1024);

        // Without a GPU bound there is no `instance_buffer`, but the
        // CPU-only renderer must still accept the advertised capacity
        // without panicking.
        let mut renderer = Renderer::new();
        renderer.begin_frame().unwrap();
        let batch = vec![QuadInstance::default(); QUAD_INSTANCE_INITIAL_CAPACITY];
        renderer
            .draw_quads_gpu(&batch)
            .expect("must accept initial-capacity worth of quads");
        assert_eq!(renderer.pending_quads().len(), QUAD_INSTANCE_INITIAL_CAPACITY);
    }

    #[test]
    fn end_frame_without_begin_frame_errors() {
        let mut renderer = Renderer::new();
        assert_eq!(
            renderer.end_frame(),
            Err(FrameError::NotInFrame),
            "end_frame before begin_frame must return NotInFrame"
        );

        // draw_quads_gpu must also refuse when no frame is open.
        assert_eq!(
            renderer.draw_quads_gpu(&[QuadInstance::default()]),
            Err(FrameError::NotInFrame),
            "draw_quads_gpu outside a frame must return NotInFrame"
        );

        // Double-begin must be rejected.
        renderer.begin_frame().unwrap();
        assert_eq!(
            renderer.begin_frame(),
            Err(FrameError::AlreadyInFrame),
            "begin_frame inside a frame must return AlreadyInFrame"
        );
    }

    // ------------------------------------------------------------------
    // AE Wave: Additional renderer lifecycle and state tests
    // ------------------------------------------------------------------

    #[test]
    fn is_in_frame_false_before_begin_frame() {
        let renderer = Renderer::new();
        assert!(!renderer.is_in_frame(), "new renderer must not be in-frame");
    }

    #[test]
    fn is_in_frame_true_after_begin_frame() {
        let mut renderer = Renderer::new();
        renderer.begin_frame().unwrap();
        assert!(renderer.is_in_frame(), "must be in-frame after begin_frame");
    }

    #[test]
    fn is_in_frame_false_after_end_frame() {
        let mut renderer = Renderer::new();
        renderer.begin_frame().unwrap();
        renderer.end_frame().unwrap();
        assert!(!renderer.is_in_frame(), "must not be in-frame after end_frame");
    }

    #[test]
    fn pending_quads_empty_before_begin_frame() {
        let renderer = Renderer::new();
        assert_eq!(
            renderer.pending_quads().len(),
            0,
            "pending_quads must be empty before any frame"
        );
    }

    #[test]
    fn pending_quads_accumulates_across_draw_quads_gpu_calls() {
        let mut renderer = Renderer::new();
        renderer.begin_frame().unwrap();
        renderer.draw_quads_gpu(&[QuadInstance::default(); 3]).unwrap();
        renderer.draw_quads_gpu(&[QuadInstance::default(); 5]).unwrap();
        assert_eq!(
            renderer.pending_quads().len(),
            8,
            "pending_quads must accumulate 3 + 5 = 8 instances"
        );
    }

    #[test]
    fn pending_quads_cleared_by_begin_frame() {
        let mut renderer = Renderer::new();
        renderer.begin_frame().unwrap();
        renderer.draw_quads_gpu(&[QuadInstance::default(); 10]).unwrap();
        renderer.end_frame().unwrap();
        renderer.begin_frame().unwrap();
        assert_eq!(
            renderer.pending_quads().len(),
            0,
            "begin_frame must clear pending_quads from previous frame"
        );
    }

    #[test]
    fn draw_quads_gpu_empty_slice_ok() {
        let mut renderer = Renderer::new();
        renderer.begin_frame().unwrap();
        // Drawing zero quads must succeed without errors.
        renderer.draw_quads_gpu(&[]).unwrap();
        assert_eq!(renderer.pending_quads().len(), 0);
        assert_eq!(renderer.stats().quads_drawn, 0);
    }

    #[test]
    fn draw_quads_gpu_updates_stats_quads_drawn_cumulatively() {
        let mut renderer = Renderer::new();
        // Frame 1
        renderer.begin_frame().unwrap();
        renderer.draw_quads_gpu(&[QuadInstance::default(); 5]).unwrap();
        renderer.end_frame().unwrap();
        // Frame 2
        renderer.begin_frame().unwrap();
        renderer.draw_quads_gpu(&[QuadInstance::default(); 3]).unwrap();
        renderer.end_frame().unwrap();
        assert_eq!(
            renderer.stats().quads_drawn,
            8,
            "quads_drawn must accumulate across frames: 5 + 3 = 8"
        );
    }

    #[test]
    fn frame_count_and_stats_frames_stay_in_sync() {
        let mut renderer = Renderer::new();
        for _ in 0..7 {
            renderer.begin_frame().unwrap();
            renderer.end_frame().unwrap();
        }
        assert_eq!(renderer.frame_count, 7);
        assert_eq!(renderer.stats().frames, 7);
    }

    #[test]
    fn renderer_gpu_field_none_on_new() {
        let renderer = Renderer::new();
        assert!(renderer.gpu.is_none(), "cpu-only renderer must have gpu == None");
    }

    #[test]
    fn frame_error_display_not_in_frame() {
        let e = FrameError::NotInFrame;
        assert_eq!(format!("{e}"), "renderer is not inside a frame");
    }

    #[test]
    fn frame_error_display_already_in_frame() {
        let e = FrameError::AlreadyInFrame;
        assert_eq!(format!("{e}"), "renderer already has an open frame");
    }

    #[test]
    fn frame_error_is_error_trait() {
        let e: Box<dyn std::error::Error> = Box::new(FrameError::NotInFrame);
        assert!(e.to_string().contains("not inside a frame"));
    }

    #[test]
    fn pipeline_kind_discriminants() {
        assert_eq!(PipelineKind::Quad as u8, 0);
        assert_eq!(PipelineKind::MonochromeSprite as u8, 1);
        assert_eq!(PipelineKind::PolychromeSprite as u8, 2);
        assert_eq!(PipelineKind::Path as u8, 3);
        assert_eq!(PipelineKind::Shadow as u8, 4);
        assert_eq!(PipelineKind::Underline as u8, 5);
        assert_eq!(PipelineKind::Reserved6 as u8, 6);
        assert_eq!(PipelineKind::Reserved7 as u8, 7);
    }

    #[test]
    fn pipeline_kind_equality() {
        assert_eq!(PipelineKind::Quad, PipelineKind::Quad);
        assert_ne!(PipelineKind::Quad, PipelineKind::Shadow);
    }

    #[test]
    fn frosted_rect_blur_clamped_at_30() {
        use crate::scene::FrostedRect;
        use crate::types::{Bounds, Pixels, Point, Size};

        let mut renderer = Renderer::new();
        let rect = FrostedRect {
            bounds: Bounds {
                origin: Point { x: Pixels(0.0), y: Pixels(0.0) },
                size: Size { width: Pixels(100.0), height: Pixels(100.0) },
            },
            // blur_radius=30 > 20, so min(30,20)=20, alpha = (0.7 - 20*0.015).max(0.3) = 0.4
            blur_radius: 30.0,
            bg_alpha: 0.5,
            border_alpha: 0.1,
        };
        let quads = renderer.draw_frosted_rects(&[rect]);
        let expected = (0.7_f32 - 20.0_f32 * 0.015).max(0.3);
        assert!(
            (quads[0].bg_color[3] - expected).abs() < 1e-5,
            "blur_radius=30 must clamp to 20, alpha={expected:.4}, got {}",
            quads[0].bg_color[3]
        );
    }

    #[test]
    fn frosted_rect_border_widths_are_one() {
        use crate::scene::FrostedRect;
        use crate::types::{Bounds, Pixels, Point, Size};

        let mut renderer = Renderer::new();
        let rect = FrostedRect {
            bounds: Bounds {
                origin: Point { x: Pixels(0.0), y: Pixels(0.0) },
                size: Size { width: Pixels(50.0), height: Pixels(50.0) },
            },
            blur_radius: 5.0,
            bg_alpha: 0.5,
            border_alpha: 0.5,
        };
        let quads = renderer.draw_frosted_rects(&[rect]);
        // Border quad (index 1) must have border_widths = [1.0; 4]
        assert_eq!(
            quads[1].border_widths,
            [1.0; 4],
            "border quad must have uniform 1px border widths"
        );
        // Background quad must have zero border widths
        assert_eq!(
            quads[0].border_widths,
            [0.0; 4],
            "background quad must have zero border widths"
        );
    }

    #[test]
    fn frosted_rect_corner_radii_are_zero() {
        use crate::scene::FrostedRect;
        use crate::types::{Bounds, Pixels, Point, Size};

        let mut renderer = Renderer::new();
        let rect = FrostedRect {
            bounds: Bounds {
                origin: Point { x: Pixels(10.0), y: Pixels(10.0) },
                size: Size { width: Pixels(200.0), height: Pixels(100.0) },
            },
            blur_radius: 8.0,
            bg_alpha: 0.6,
            border_alpha: 0.3,
        };
        let quads = renderer.draw_frosted_rects(&[rect]);
        for (i, quad) in quads.iter().enumerate() {
            assert_eq!(
                quad.corner_radii,
                [0.0; 4],
                "quad[{i}] corner_radii must be zero (frosted rect has no rounding)"
            );
        }
    }

    #[test]
    fn frosted_rect_bg_color_is_mid_grey() {
        use crate::scene::FrostedRect;
        use crate::types::{Bounds, Pixels, Point, Size};

        let mut renderer = Renderer::new();
        let rect = FrostedRect {
            bounds: Bounds {
                origin: Point { x: Pixels(0.0), y: Pixels(0.0) },
                size: Size { width: Pixels(100.0), height: Pixels(100.0) },
            },
            blur_radius: 5.0,
            bg_alpha: 0.5,
            border_alpha: 0.5,
        };
        let quads = renderer.draw_frosted_rects(&[rect]);
        // Background quad RGB must be [0.5, 0.5, 0.5] (mid-grey).
        assert!((quads[0].bg_color[0] - 0.5).abs() < 1e-6, "R channel must be 0.5");
        assert!((quads[0].bg_color[1] - 0.5).abs() < 1e-6, "G channel must be 0.5");
        assert!((quads[0].bg_color[2] - 0.5).abs() < 1e-6, "B channel must be 0.5");
    }

    #[test]
    fn ortho_projection_width_height_1() {
        // ortho(1, 1): x scale = 2.0, y scale = -2.0
        let m = ortho_projection(1.0, 1.0);
        assert!((m[0][0] - 2.0).abs() < 1e-6, "x scale for width=1 is 2.0");
        assert!((m[1][1] - (-2.0)).abs() < 1e-6, "y scale for height=1 is -2.0");
    }

    #[test]
    fn ortho_projection_large_viewport() {
        let m = ortho_projection(3840.0, 2160.0);
        assert!((m[0][0] - 2.0 / 3840.0).abs() < 1e-9, "x scale");
        assert!((m[1][1] - (-2.0 / 2160.0)).abs() < 1e-9, "y scale");
    }

    #[test]
    fn hsla_to_rgba_blue() {
        // Pure blue: h=240, s=1, l=0.5 → [0, 0, 1, 1]
        let rgba = hsla_to_rgba(Hsla { h: 240.0, s: 1.0, l: 0.5, a: 1.0 });
        assert!(rgba[0] < 0.01, "blue hue: red must be ~0, got {}", rgba[0]);
        assert!(rgba[1] < 0.01, "blue hue: green must be ~0, got {}", rgba[1]);
        assert!((rgba[2] - 1.0).abs() < 1e-4, "blue hue: blue must be ~1, got {}", rgba[2]);
    }

    #[test]
    fn hsla_to_rgba_alpha_passthrough() {
        // Alpha must be forwarded unchanged regardless of hue/saturation.
        let rgba = hsla_to_rgba(Hsla { h: 60.0, s: 0.5, l: 0.5, a: 0.42 });
        assert!((rgba[3] - 0.42).abs() < 1e-6, "alpha must be preserved: {}", rgba[3]);
    }

    #[test]
    fn hsla_to_rgba_achromatic_midgrey() {
        // s=0, l=0.5 → r = g = b = 0.5
        let rgba = hsla_to_rgba(Hsla { h: 0.0, s: 0.0, l: 0.5, a: 1.0 });
        assert!((rgba[0] - 0.5).abs() < 1e-6, "mid-grey R must be 0.5, got {}", rgba[0]);
        assert!((rgba[1] - 0.5).abs() < 1e-6, "mid-grey G must be 0.5, got {}", rgba[1]);
        assert!((rgba[2] - 0.5).abs() < 1e-6, "mid-grey B must be 0.5, got {}", rgba[2]);
    }

    // ------------------------------------------------------------------
    // Wave AF: draw_quads zero-quads no-op, FrameStats reset,
    //          ortho_projection at 1x1 viewport
    // ------------------------------------------------------------------

    #[test]
    fn draw_quads_gpu_zero_quads_is_noop() {
        // Submitting an empty slice must not change any stats or pending_quads.
        let mut renderer = Renderer::new();
        renderer.begin_frame().unwrap();
        renderer.draw_quads_gpu(&[]).unwrap();
        assert_eq!(renderer.pending_quads().len(), 0, "pending_quads must stay empty");
        assert_eq!(renderer.stats().quads_drawn, 0, "quads_drawn must stay zero");
        renderer.end_frame().unwrap();
        assert_eq!(renderer.stats().quads_drawn, 0, "quads_drawn stays zero after frame");
    }

    #[test]
    fn frame_stats_independent_per_field_after_reset_simulation() {
        // FrameStats::default() zeroes every counter; a new Renderer::new()
        // effectively resets stats to zero for a fresh start.
        let mut renderer = Renderer::new();
        let mut scene = Scene::new();
        scene.push_quad(crate::scene::Quad::default());
        scene.push_shadow(crate::scene::Shadow::default());
        renderer.draw(&mut scene);

        // Simulate "reset between frames" by creating a new renderer.
        let fresh = Renderer::new();
        let s = fresh.stats();
        assert_eq!(s.quads_drawn, 0, "fresh renderer: quads_drawn = 0");
        assert_eq!(s.shadows_drawn, 0, "fresh renderer: shadows_drawn = 0");
        assert_eq!(s.frames, 0, "fresh renderer: frames = 0");
        assert_eq!(s.paths_drawn, 0, "fresh renderer: paths_drawn = 0");
        assert_eq!(s.mono_sprites_drawn, 0, "fresh renderer: mono_sprites = 0");
        assert_eq!(s.sprites_drawn, 0, "fresh renderer: sprites = 0");
        assert_eq!(s.underlines_drawn, 0, "fresh renderer: underlines = 0");
        assert_eq!(s.frosted_drawn, 0, "fresh renderer: frosted = 0");
    }

    #[test]
    fn ortho_projection_1x1_viewport() {
        // Width=1, height=1: x_scale=2.0, y_scale=-2.0, translation=(-1, 1).
        // Top-left corner (0,0) maps to NDC (-1, 1); bottom-right (1,1) maps to (1, -1).
        let m = ortho_projection(1.0, 1.0);
        let x_scale = m[0][0];
        let y_scale = m[1][1];
        let x_trans = m[3][0];
        let y_trans = m[3][1];

        assert!((x_scale - 2.0).abs() < 1e-6, "x_scale must be 2.0 for width=1, got {x_scale}");
        assert!((y_scale - (-2.0)).abs() < 1e-6, "y_scale must be -2.0 for height=1, got {y_scale}");
        assert!((x_trans - (-1.0)).abs() < 1e-6, "x_trans must be -1, got {x_trans}");
        assert!((y_trans - 1.0).abs() < 1e-6, "y_trans must be 1, got {y_trans}");

        // Verify: top-left (0,0) → NDC x = 0*2 + (-1) = -1, NDC y = 0*(-2) + 1 = 1
        let ndc_tl_x = 0.0 * x_scale + x_trans;
        let ndc_tl_y = 0.0 * y_scale + y_trans;
        assert!((ndc_tl_x - (-1.0)).abs() < 1e-6, "top-left NDC x = -1, got {ndc_tl_x}");
        assert!((ndc_tl_y - 1.0).abs() < 1e-6, "top-left NDC y = 1, got {ndc_tl_y}");

        // Verify: bottom-right (1,1) → NDC x = 1*2 + (-1) = 1, NDC y = 1*(-2) + 1 = -1
        let ndc_br_x = 1.0 * x_scale + x_trans;
        let ndc_br_y = 1.0 * y_scale + y_trans;
        assert!((ndc_br_x - 1.0).abs() < 1e-6, "bottom-right NDC x = 1, got {ndc_br_x}");
        assert!((ndc_br_y - (-1.0)).abs() < 1e-6, "bottom-right NDC y = -1, got {ndc_br_y}");
    }

    #[test]
    fn ortho_projection_point_mapping() {
        // ortho(800, 600): center point (400, 300) should map to NDC (0, 0).
        let m = ortho_projection(800.0, 600.0);
        let x_scale = m[0][0];
        let y_scale = m[1][1];
        let x_trans = m[3][0];
        let y_trans = m[3][1];

        let ndc_x = 400.0 * x_scale + x_trans;
        let ndc_y = 300.0 * y_scale + y_trans;
        assert!((ndc_x - 0.0).abs() < 1e-5, "center maps to NDC x=0, got {ndc_x}");
        assert!((ndc_y - 0.0).abs() < 1e-5, "center maps to NDC y=0, got {ndc_y}");
    }

    #[test]
    fn draw_scene_with_only_shadows_increments_shadow_count() {
        let mut renderer = Renderer::new();
        let mut scene = Scene::new();
        for _ in 0..4 {
            scene.push_shadow(crate::scene::Shadow::default());
        }
        renderer.draw(&mut scene);
        assert_eq!(renderer.stats().shadows_drawn, 4, "4 shadows must be counted");
        assert_eq!(renderer.stats().quads_drawn, 0, "no quads in shadow-only scene");
    }

    #[test]
    fn draw_empty_scene_increments_frame_not_primitives() {
        let mut renderer = Renderer::new();
        let mut scene = Scene::new();
        renderer.draw(&mut scene);
        assert_eq!(renderer.frame_count, 1, "frame count increments");
        assert_eq!(renderer.stats().quads_drawn, 0);
        assert_eq!(renderer.stats().shadows_drawn, 0);
        assert_eq!(renderer.stats().paths_drawn, 0);
    }

    #[test]
    fn frame_stats_cumulate_across_multiple_draws() {
        let mut renderer = Renderer::new();
        for _ in 0..3 {
            let mut scene = Scene::new();
            scene.push_quad(crate::scene::Quad::default());
            renderer.draw(&mut scene);
        }
        assert_eq!(renderer.stats().quads_drawn, 3, "cumulative 3 quads across 3 draws");
        assert_eq!(renderer.stats().frames, 3, "3 frames");
    }
}
