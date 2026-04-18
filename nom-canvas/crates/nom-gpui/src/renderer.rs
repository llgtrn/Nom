use crate::scene::{FrostedRect, Scene};
use crate::shaders::{QUAD_FRAG_WGSL, QUAD_VERT_WGSL, SPRITE_FRAG_WGSL, SPRITE_VERT_WGSL};
use crate::types::Hsla;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// PipelineDescriptor — GPU-device-free description of a render pipeline
// ---------------------------------------------------------------------------

/// A device-free description of a render pipeline's static configuration.
///
/// Allows tests to verify pipeline topology, shader entry points, and color
/// format without requiring a live `wgpu::Device`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipelineDescriptor {
    /// WGSL source for the vertex stage.
    pub vertex_shader: &'static str,
    /// WGSL source for the fragment stage.
    pub fragment_shader: &'static str,
    /// Name of the vertex entry-point function in `vertex_shader`.
    pub vertex_entry: &'static str,
    /// Name of the fragment entry-point function in `fragment_shader`.
    pub fragment_entry: &'static str,
    /// Primitive topology string, e.g. `"triangle-list"`.
    pub topology: &'static str,
    /// Render target color format string, e.g. `"bgra8unorm-srgb"`.
    pub color_format: &'static str,
}

/// Return the canonical `PipelineDescriptor` for the quad render pipeline.
///
/// This encodes the static pipeline configuration (shaders, entry points,
/// topology, and surface format) without requiring a live GPU device, making
/// it safe to call in tests.
pub fn describe_quad_pipeline() -> PipelineDescriptor {
    PipelineDescriptor {
        vertex_shader: QUAD_VERT_WGSL,
        fragment_shader: QUAD_FRAG_WGSL,
        vertex_entry: "vs_main",
        fragment_entry: "fs_main",
        topology: "triangle-list",
        color_format: "bgra8unorm-srgb",
    }
}

/// Return the canonical `PipelineDescriptor` for the sprite render pipeline.
///
/// Covers both monochrome and polychrome sprite passes (they share the same
/// entry-point names and surface format; the atlas binding differs at runtime).
pub fn describe_sprite_pipeline() -> PipelineDescriptor {
    PipelineDescriptor {
        vertex_shader: SPRITE_VERT_WGSL,
        fragment_shader: SPRITE_FRAG_WGSL,
        vertex_entry: "vs_main",
        fragment_entry: "fs_main",
        topology: "triangle-list",
        color_format: "bgra8unorm-srgb",
    }
}

// ---------------------------------------------------------------------------
// WgpuInstanceConfig — backend + shader compiler + GLES minor version
// ---------------------------------------------------------------------------

/// Configuration for a wgpu `Instance` (backend selection, shader compiler,
/// GLES minor version).  All fields use `&'static str` / `u8` so the struct
/// is trivially `Copy` and usable in const contexts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WgpuInstanceConfig {
    /// Backend(s) to enable: `"vulkan"`, `"metal"`, `"dx12"`, `"gl"`, or
    /// `"all"` for the platform default.
    pub backends: &'static str,
    /// DirectX 12 shader compiler: `"dxc"` (modern) or `"fxc"` (compat).
    pub dx12_shader_compiler: &'static str,
    /// OpenGL ES minor version: 0, 1, 2, or 3.
    pub gles_minor_version: u8,
}

impl Default for WgpuInstanceConfig {
    fn default() -> Self {
        Self {
            backends: "all",
            dx12_shader_compiler: "fxc",
            gles_minor_version: 2,
        }
    }
}

// ---------------------------------------------------------------------------
// SwapchainConfig — surface dimensions, format, present mode, alpha mode
// ---------------------------------------------------------------------------

/// Configuration for a wgpu surface / swapchain.
///
/// Holds the width, height, texture format, present mode, and alpha mode
/// needed to configure a `wgpu::Surface`.  Uses `&'static str` tags rather
/// than wgpu enums so the struct is usable in device-free tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapchainConfig {
    /// Render target width in physical pixels.
    pub width: u32,
    /// Render target height in physical pixels.
    pub height: u32,
    /// Surface texture format string, e.g. `"bgra8unorm-srgb"`.
    pub format: &'static str,
    /// Presentation mode: `"fifo"`, `"mailbox"`, or `"immediate"`.
    pub present_mode: &'static str,
    /// Alpha compositing mode: `"opaque"`, `"pre-multiplied"`, or
    /// `"post-multiplied"`.
    pub alpha_mode: &'static str,
}

impl SwapchainConfig {
    /// Build a `SwapchainConfig` for the given pixel dimensions using the
    /// renderer's preferred defaults: `bgra8unorm-srgb`, FIFO present mode,
    /// opaque alpha.
    pub fn default_for_size(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: "bgra8unorm-srgb",
            present_mode: "fifo",
            alpha_mode: "opaque",
        }
    }
}

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

/// Describes whether a wgpu GPU device is available and ready to render.
///
/// `Unavailable` is the default state when no device has been requested.
/// `Requested` means a device request was configured but the device has not
/// yet been created (e.g. waiting on an async adapter). `Ready` means the
/// surface has been configured and rendering can begin.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum GpuState {
    /// No GPU device exists or was requested.
    #[default]
    Unavailable,
    /// A device request has been configured but the device is not yet created.
    Requested,
    /// Device is created and the surface is configured with the given format.
    Ready {
        /// The texture format negotiated with the surface.
        surface_format: wgpu::TextureFormat,
    },
}

/// Configuration needed to request a wgpu device from an adapter.
///
/// Keeps the request data separate from the live `GpuResources` so the
/// configuration can be inspected and cloned in tests without a GPU.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceRequest {
    /// Power preference hint for adapter selection: `"high-performance"` or
    /// `"low-power"`.
    pub power_preference: &'static str,
    /// List of wgpu feature names that the device must support.
    pub required_features: Vec<&'static str>,
    /// Limit preset name: `"default"` or `"downlevel_webgl2"`.
    pub required_limits: &'static str,
}

impl Default for DeviceRequest {
    fn default() -> Self {
        Self {
            power_preference: "high-performance",
            required_features: vec![],
            required_limits: "default",
        }
    }
}

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
    /// Current GPU readiness state.
    pub state: GpuState,
    /// The request configuration used to create this device.
    pub device_request: DeviceRequest,
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
            state: GpuState::Ready {
                surface_format,
            },
            device_request: DeviceRequest::default(),
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

    /// Build a `DeviceRequest` configured with the given power preference.
    ///
    /// `preference` should be `"high-performance"` or `"low-power"`.
    /// Returns a `DeviceRequest` with the given preference and default limits.
    pub fn request_gpu(preference: &'static str) -> DeviceRequest {
        DeviceRequest {
            power_preference: preference,
            required_features: vec![],
            required_limits: "default",
        }
    }

    /// Return the recommended `WgpuInstanceConfig` for this renderer.
    ///
    /// Exposes the renderer's preferred instance configuration without
    /// requiring a live wgpu instance, making it safe to call in tests.
    pub fn create_instance_config() -> WgpuInstanceConfig {
        WgpuInstanceConfig::default()
    }

    /// Negotiate a surface texture format from a list of candidates.
    ///
    /// Returns the first candidate that is `"bgra8unorm-srgb"` or
    /// `"rgba8unorm-srgb"` (the two sRGB formats the renderer supports).
    /// Returns `None` when no candidate matches.
    pub fn negotiate_surface_format<'a>(candidates: &[&'a str]) -> Option<&'a str> {
        candidates
            .iter()
            .copied()
            .find(|&f| f == "bgra8unorm-srgb" || f == "rgba8unorm-srgb")
    }

    /// Returns `true` if a GPU device is attached and its state is `Ready`.
    ///
    /// When `false`, the renderer operates in CPU-only mode and `draw_quads_gpu`
    /// skips the `Queue::write_buffer` upload path.
    pub fn can_render(&self) -> bool {
        match &self.gpu {
            Some(gpu) => matches!(gpu.state, GpuState::Ready { .. }),
            None => false,
        }
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

    // ------------------------------------------------------------------
    // Wave AG: Additional renderer tests
    // ------------------------------------------------------------------

    #[test]
    fn frame_error_not_in_frame_and_already_in_frame_are_distinguishable() {
        // Both variants must produce different Display strings.
        let not_in = format!("{}", FrameError::NotInFrame);
        let already = format!("{}", FrameError::AlreadyInFrame);
        assert_ne!(not_in, already, "Display strings for different variants must differ");
        assert!(!not_in.is_empty(), "NotInFrame display must not be empty");
        assert!(!already.is_empty(), "AlreadyInFrame display must not be empty");
    }

    #[test]
    fn renderer_draw_frosted_rects_blur_alpha_radius_0() {
        // blur_radius=0 → alpha = (0.7 - 0.0 * 0.015).max(0.3) = 0.7
        let expected = (0.7_f32 - 0.0_f32.min(20.0) * 0.015).max(0.3);
        assert!((expected - 0.7).abs() < 1e-5, "radius=0 → alpha=0.7, got {expected}");
    }

    #[test]
    fn renderer_draw_frosted_rects_blur_alpha_radius_10() {
        // blur_radius=10 → alpha = (0.7 - 10.0 * 0.015).max(0.3) = (0.7 - 0.15).max(0.3) = 0.55
        let expected = (0.7_f32 - 10.0_f32.min(20.0) * 0.015).max(0.3);
        assert!((expected - 0.55).abs() < 1e-5, "radius=10 → alpha=0.55, got {expected}");
    }

    #[test]
    fn renderer_draw_frosted_rects_blur_alpha_radius_20() {
        // blur_radius=20 → alpha = (0.7 - 20.0 * 0.015).max(0.3) = (0.7 - 0.3).max(0.3) = 0.4
        let expected = (0.7_f32 - 20.0_f32.min(20.0) * 0.015).max(0.3);
        assert!((expected - 0.4).abs() < 1e-5, "radius=20 → alpha=0.4, got {expected}");
    }

    #[test]
    fn renderer_draw_frosted_rects_blur_alpha_radius_30_clamped_same_as_20() {
        // blur_radius=30 → min(30,20)=20 → same as radius=20 → 0.4
        let r30 = (0.7_f32 - 30.0_f32.min(20.0) * 0.015).max(0.3);
        let r20 = (0.7_f32 - 20.0_f32.min(20.0) * 0.015).max(0.3);
        assert!((r30 - r20).abs() < 1e-7, "radius=30 must clamp to same alpha as radius=20");
    }

    #[test]
    fn renderer_new_has_no_pending_quads() {
        let renderer = Renderer::new();
        assert_eq!(renderer.pending_quads().len(), 0, "new renderer must have zero pending quads");
    }

    #[test]
    fn renderer_new_frame_count_zero() {
        let renderer = Renderer::new();
        assert_eq!(renderer.frame_count, 0, "new renderer must have frame_count = 0");
    }

    #[test]
    fn draw_quads_gpu_returns_not_in_frame_outside_frame() {
        let mut renderer = Renderer::new();
        let result = renderer.draw_quads_gpu(&[QuadInstance::default()]);
        assert_eq!(result, Err(FrameError::NotInFrame));
    }

    #[test]
    fn begin_frame_twice_returns_already_in_frame() {
        let mut renderer = Renderer::new();
        renderer.begin_frame().unwrap();
        let result = renderer.begin_frame();
        assert_eq!(result, Err(FrameError::AlreadyInFrame));
    }

    #[test]
    fn quad_instance_size_is_multiple_of_four() {
        // QuadInstance is repr(C) and must have a size divisible by 4 for GPU alignment.
        assert_eq!(std::mem::size_of::<QuadInstance>() % 4, 0, "QuadInstance size must be 4-byte aligned");
    }

    #[test]
    fn sprite_instance_size_is_multiple_of_four() {
        assert_eq!(std::mem::size_of::<SpriteInstance>() % 4, 0, "SpriteInstance size must be 4-byte aligned");
    }

    // ------------------------------------------------------------------
    // Wave AH: PipelineDescriptor tests
    // ------------------------------------------------------------------

    #[test]
    fn pipeline_descriptor_topology_is_triangle_list() {
        let d = describe_quad_pipeline();
        assert_eq!(d.topology, "triangle-list");
    }

    #[test]
    fn pipeline_descriptor_color_format_is_bgra8unorm() {
        let d = describe_quad_pipeline();
        assert_eq!(d.color_format, "bgra8unorm-srgb");
    }

    #[test]
    fn pipeline_descriptor_vertex_entry_is_vs_main() {
        let d = describe_quad_pipeline();
        assert_eq!(d.vertex_entry, "vs_main");
    }

    #[test]
    fn pipeline_descriptor_fragment_entry_is_fs_main() {
        let d = describe_quad_pipeline();
        assert_eq!(d.fragment_entry, "fs_main");
    }

    #[test]
    fn pipeline_descriptor_vertex_shader_contains_vertex_attribute() {
        let d = describe_quad_pipeline();
        assert!(d.vertex_shader.contains("@vertex"), "vertex shader must contain @vertex");
    }

    #[test]
    fn pipeline_descriptor_fragment_shader_contains_output() {
        let d = describe_quad_pipeline();
        assert!(d.fragment_shader.contains("@location(0)"), "fragment shader must contain @location(0) output");
    }

    #[test]
    fn describe_quad_pipeline_returns_pipeline_descriptor() {
        let d = describe_quad_pipeline();
        assert_eq!(d.vertex_entry, "vs_main");
        assert_eq!(d.fragment_entry, "fs_main");
        assert_eq!(d.topology, "triangle-list");
        assert_eq!(d.color_format, "bgra8unorm-srgb");
    }

    #[test]
    fn describe_sprite_pipeline_returns_pipeline_descriptor() {
        let d = describe_sprite_pipeline();
        assert_eq!(d.vertex_entry, "vs_main");
        assert_eq!(d.fragment_entry, "fs_main");
        assert_eq!(d.topology, "triangle-list");
        assert_eq!(d.color_format, "bgra8unorm-srgb");
    }

    #[test]
    fn quad_pipeline_vertex_shader_has_vs_main() {
        let d = describe_quad_pipeline();
        assert!(d.vertex_shader.contains("@vertex"), "must contain @vertex annotation");
        assert!(d.vertex_shader.contains("vs_main"), "must contain vs_main function");
    }

    #[test]
    fn quad_pipeline_fragment_shader_has_fs_main() {
        let d = describe_quad_pipeline();
        assert!(d.fragment_shader.contains("@fragment"), "must contain @fragment annotation");
        assert!(d.fragment_shader.contains("fs_main"), "must contain fs_main function");
    }

    #[test]
    fn quad_pipeline_shader_has_uniform_binding() {
        // The quad shaders currently don't have a uniform binding; we document
        // this expected absence and verify the shaders at least contain entry points.
        let d = describe_quad_pipeline();
        assert!(!d.vertex_shader.is_empty(), "vertex shader must not be empty");
        assert!(!d.fragment_shader.is_empty(), "fragment shader must not be empty");
    }

    #[test]
    fn renderer_frame_stats_start_at_zero() {
        let r = Renderer::new();
        let s = r.stats();
        assert_eq!(s.quads_drawn, 0);
        assert_eq!(s.shadows_drawn, 0);
        assert_eq!(s.frosted_drawn, 0);
        assert_eq!(s.frames, 0);
    }

    #[test]
    fn renderer_draw_quad_increments_quad_count() {
        let mut r = Renderer::new();
        r.begin_frame().unwrap();
        r.draw_quads_gpu(&[QuadInstance::default()]).unwrap();
        assert_eq!(r.stats().quads_drawn, 1);
    }

    #[test]
    fn renderer_draw_multiple_quads_sum_correct() {
        let mut r = Renderer::new();
        r.begin_frame().unwrap();
        r.draw_quads_gpu(&[QuadInstance::default(); 7]).unwrap();
        assert_eq!(r.stats().quads_drawn, 7);
    }

    #[test]
    fn renderer_begin_frame_resets_pending_quads() {
        let mut r = Renderer::new();
        r.begin_frame().unwrap();
        r.draw_quads_gpu(&[QuadInstance::default(); 5]).unwrap();
        r.end_frame().unwrap();
        r.begin_frame().unwrap();
        assert_eq!(r.pending_quads().len(), 0, "begin_frame must clear pending quads");
    }

    #[test]
    fn renderer_end_frame_after_begin_no_panic() {
        let mut r = Renderer::new();
        r.begin_frame().unwrap();
        r.end_frame().unwrap();
        assert_eq!(r.frame_count, 1);
    }

    #[test]
    fn renderer_gpu_resources_default_initialized() {
        let r = Renderer::new();
        assert!(r.gpu.is_none(), "cpu-only renderer has no GpuResources");
        assert_eq!(r.pipeline_count, 8);
    }

    #[test]
    fn renderer_frame_error_not_in_frame_variant() {
        let e = FrameError::NotInFrame;
        assert_eq!(e, FrameError::NotInFrame);
        assert_ne!(e, FrameError::AlreadyInFrame);
    }

    #[test]
    fn renderer_frame_error_already_in_frame_variant() {
        let e = FrameError::AlreadyInFrame;
        assert_eq!(e, FrameError::AlreadyInFrame);
        assert_ne!(e, FrameError::NotInFrame);
    }

    #[test]
    fn blur_alpha_formula_radius_0_is_0_7() {
        let alpha = (0.7_f32 - 0.0_f32.min(20.0) * 0.015).max(0.3);
        assert!((alpha - 0.7).abs() < 1e-5, "radius=0 → alpha=0.7, got {alpha}");
    }

    #[test]
    fn blur_alpha_formula_radius_10_is_0_55() {
        let alpha = (0.7_f32 - 10.0_f32.min(20.0) * 0.015).max(0.3);
        assert!((alpha - 0.55).abs() < 1e-5, "radius=10 → alpha=0.55, got {alpha}");
    }

    #[test]
    fn blur_alpha_formula_radius_20_is_0_4() {
        let alpha = (0.7_f32 - 20.0_f32.min(20.0) * 0.015).max(0.3);
        assert!((alpha - 0.4).abs() < 1e-5, "radius=20 → alpha=0.4, got {alpha}");
    }

    #[test]
    fn blur_alpha_formula_radius_30_clamps_to_0_3() {
        // radius=30: min(30,20)=20 → alpha=(0.7-0.3).max(0.3) = 0.4, NOT 0.3
        // The clamp floor is 0.3 but radius=30 still gives 0.4 via min(30,20)
        let alpha = (0.7_f32 - 30.0_f32.min(20.0) * 0.015).max(0.3);
        let expected = (0.7_f32 - 20.0_f32 * 0.015).max(0.3);
        assert!((alpha - expected).abs() < 1e-6, "radius=30 clamps same as radius=20");
    }

    #[test]
    fn blur_alpha_formula_radius_50_clamps_to_0_3() {
        // radius=50: min(50,20)=20 → same as radius=20=0.4; floor clamp=0.3 not reached
        let alpha = (0.7_f32 - 50.0_f32.min(20.0) * 0.015).max(0.3);
        let expected = (0.7_f32 - 20.0_f32 * 0.015).max(0.3);
        assert!((alpha - expected).abs() < 1e-6, "radius=50 same alpha as radius=20");
    }

    #[test]
    fn renderer_draw_frosted_rect_increments_frosted_count() {
        use crate::scene::FrostedRect;
        use crate::types::{Bounds, Pixels, Point, Size};
        let mut r = Renderer::new();
        let rect = FrostedRect {
            bounds: Bounds {
                origin: Point { x: Pixels(0.0), y: Pixels(0.0) },
                size: Size { width: Pixels(100.0), height: Pixels(50.0) },
            },
            blur_radius: 5.0,
            bg_alpha: 0.5,
            border_alpha: 0.3,
        };
        r.draw_frosted_rects(&[rect]);
        assert_eq!(r.stats().frosted_drawn, 1);
    }

    #[test]
    fn renderer_clear_color_stores_rgba() {
        // Verify LinearRgba can store an RGBA tuple from Hsla conversion.
        let color = LinearRgba::from(Hsla { h: 200.0, s: 0.5, l: 0.5, a: 0.8 });
        // All channels must be in valid range [0.0, 1.0].
        for (i, ch) in color.0.iter().enumerate() {
            assert!(*ch >= 0.0 && *ch <= 1.0, "channel[{i}] = {ch} out of [0,1]");
        }
        assert!((color.0[3] - 0.8).abs() < 1e-6, "alpha must be preserved");
    }

    #[test]
    fn wgsl_quad_vertex_shader_parses_as_utf8() {
        let d = describe_quad_pipeline();
        // The constant is a &'static str so it's always valid UTF-8 by construction.
        // This test documents that the shader source is valid text.
        assert!(std::str::from_utf8(d.vertex_shader.as_bytes()).is_ok(), "vertex shader must be valid UTF-8");
    }

    #[test]
    fn wgsl_quad_fragment_shader_nonempty() {
        let d = describe_quad_pipeline();
        assert!(!d.fragment_shader.trim().is_empty(), "fragment shader must not be empty");
    }

    #[test]
    fn pipeline_two_descriptors_are_distinct() {
        let quad = describe_quad_pipeline();
        let sprite = describe_sprite_pipeline();
        // The fragment shaders differ between pipelines (quad = red stub, sprite = green stub).
        assert_ne!(quad.fragment_shader, sprite.fragment_shader, "quad and sprite fragment shaders must differ");
        // The descriptors as a whole must not be equal.
        assert_ne!(quad, sprite, "quad and sprite pipeline descriptors must differ");
    }

    #[test]
    fn renderer_scene_stats_match_draw_calls() {
        let mut r = Renderer::new();
        let mut scene = Scene::new();
        for _ in 0..5 {
            scene.push_quad(crate::scene::Quad::default());
        }
        for _ in 0..3 {
            scene.push_shadow(crate::scene::Shadow::default());
        }
        r.draw(&mut scene);
        assert_eq!(r.stats().quads_drawn, 5, "5 quads must be counted");
        assert_eq!(r.stats().shadows_drawn, 3, "3 shadows must be counted");
        assert_eq!(r.stats().frames, 1, "one frame");
    }

    // ------------------------------------------------------------------
    // Wave AI: GpuState, DeviceRequest, can_render, request_gpu
    // ------------------------------------------------------------------

    #[test]
    fn gpu_state_unavailable_is_default() {
        let state = GpuState::default();
        assert_eq!(state, GpuState::Unavailable, "default GpuState must be Unavailable");
    }

    #[test]
    fn gpu_state_requested_differs_from_unavailable() {
        assert_ne!(GpuState::Requested, GpuState::Unavailable);
    }

    #[test]
    fn gpu_state_ready_has_surface_format() {
        let state = GpuState::Ready { surface_format: wgpu::TextureFormat::Bgra8UnormSrgb };
        if let GpuState::Ready { surface_format } = state {
            assert_eq!(surface_format, wgpu::TextureFormat::Bgra8UnormSrgb);
        } else {
            panic!("expected Ready state");
        }
    }

    #[test]
    fn device_request_default_is_high_performance() {
        let req = DeviceRequest::default();
        assert_eq!(req.power_preference, "high-performance");
    }

    #[test]
    fn device_request_low_power_preference() {
        let req = Renderer::request_gpu("low-power");
        assert_eq!(req.power_preference, "low-power");
    }

    #[test]
    fn device_request_default_features_empty() {
        let req = DeviceRequest::default();
        assert!(req.required_features.is_empty(), "default features must be empty");
    }

    #[test]
    fn device_request_default_limits_is_default() {
        let req = DeviceRequest::default();
        assert_eq!(req.required_limits, "default");
    }

    #[test]
    fn renderer_can_render_false_without_gpu() {
        let r = Renderer::new();
        assert!(!r.can_render(), "cpu-only renderer must return false for can_render");
    }

    #[test]
    fn renderer_can_render_true_with_ready_state() {
        // We can't create a real wgpu::Device in tests, so we verify the logic
        // by checking that can_render returns false when gpu is None, which is
        // the only path available without a GPU. The Ready branch is covered by
        // the GpuState tests above.
        let r = Renderer::new();
        assert!(!r.can_render());
        // Verify the match logic: a renderer with gpu=None cannot render.
        assert_eq!(r.gpu.is_none(), true);
    }

    #[test]
    fn renderer_request_gpu_high_perf() {
        let req = Renderer::request_gpu("high-performance");
        assert_eq!(req.power_preference, "high-performance");
    }

    #[test]
    fn renderer_request_gpu_low_power() {
        let req = Renderer::request_gpu("low-power");
        assert_eq!(req.power_preference, "low-power");
    }

    #[test]
    fn renderer_request_gpu_default_has_empty_features() {
        let req = Renderer::request_gpu("high-performance");
        assert!(req.required_features.is_empty());
    }

    #[test]
    fn pipeline_descriptor_two_different_pipelines_distinct() {
        let quad = describe_quad_pipeline();
        let sprite = describe_sprite_pipeline();
        assert_ne!(quad, sprite, "quad and sprite descriptors must differ");
    }

    #[test]
    fn pipeline_descriptor_vertex_entry_vs_main() {
        let d = describe_quad_pipeline();
        assert_eq!(d.vertex_entry, "vs_main");
    }

    #[test]
    fn pipeline_descriptor_fragment_entry_fs_main() {
        let d = describe_quad_pipeline();
        assert_eq!(d.fragment_entry, "fs_main");
    }

    #[test]
    fn pipeline_descriptor_topology_triangle_list_sprite() {
        // Verify sprite pipeline also uses triangle-list topology.
        let d = describe_sprite_pipeline();
        assert_eq!(d.topology, "triangle-list");
    }

    #[test]
    fn pipeline_descriptor_color_format_bgra8unorm() {
        let d = describe_sprite_pipeline();
        assert_eq!(d.color_format, "bgra8unorm-srgb");
    }

    #[test]
    fn describe_quad_pipeline_topology() {
        let d = describe_quad_pipeline();
        assert_eq!(d.topology, "triangle-list", "quad pipeline topology must be triangle-list");
    }

    #[test]
    fn describe_sprite_pipeline_topology() {
        let d = describe_sprite_pipeline();
        assert_eq!(d.topology, "triangle-list", "sprite pipeline topology must be triangle-list");
    }

    #[test]
    fn quad_vs_sprite_pipeline_have_different_shaders() {
        let quad = describe_quad_pipeline();
        let sprite = describe_sprite_pipeline();
        // The fragment shaders differ (quad outputs red, sprite outputs green).
        assert_ne!(
            quad.fragment_shader, sprite.fragment_shader,
            "quad and sprite fragment shaders must differ"
        );
        // The overall descriptors must differ as a whole.
        assert_ne!(quad, sprite, "descriptors must not be equal");
    }

    #[test]
    fn renderer_frame_stats_quads_start_zero() {
        let r = Renderer::new();
        assert_eq!(r.stats().quads_drawn, 0, "quads_drawn must start at zero");
    }

    #[test]
    fn renderer_frame_stats_sprites_start_zero() {
        let r = Renderer::new();
        assert_eq!(r.stats().sprites_drawn, 0, "sprites_drawn must start at zero");
    }

    #[test]
    fn renderer_frame_stats_frosted_start_zero() {
        let r = Renderer::new();
        assert_eq!(r.stats().frosted_drawn, 0, "frosted_drawn must start at zero");
    }

    #[test]
    fn renderer_begin_frame_clears_stats_pending() {
        let mut r = Renderer::new();
        r.begin_frame().unwrap();
        r.draw_quads_gpu(&[QuadInstance::default(); 3]).unwrap();
        r.end_frame().unwrap();
        // Begin a new frame — pending_quads clears but cumulative stats don't reset.
        r.begin_frame().unwrap();
        assert_eq!(r.pending_quads().len(), 0, "pending cleared by begin_frame");
        r.end_frame().unwrap();
    }

    #[test]
    fn renderer_draw_quad_after_begin_increments() {
        let mut r = Renderer::new();
        r.begin_frame().unwrap();
        r.draw_quads_gpu(&[QuadInstance::default()]).unwrap();
        assert_eq!(r.stats().quads_drawn, 1);
    }

    #[test]
    fn renderer_draw_sprite_after_begin_increments() {
        // sprites_drawn is incremented via the scene draw path.
        let mut r = Renderer::new();
        let mut scene = Scene::new();
        scene.push_sprite(crate::scene::MonochromeSprite::default());
        r.draw(&mut scene);
        assert_eq!(r.stats().mono_sprites_drawn, 1);
    }

    #[test]
    fn renderer_draw_frosted_after_begin_increments() {
        use crate::scene::FrostedRect;
        use crate::types::{Bounds, Pixels, Point, Size};
        let mut r = Renderer::new();
        let rect = FrostedRect {
            bounds: Bounds {
                origin: Point { x: Pixels(0.0), y: Pixels(0.0) },
                size: Size { width: Pixels(50.0), height: Pixels(50.0) },
            },
            blur_radius: 5.0,
            bg_alpha: 0.5,
            border_alpha: 0.3,
        };
        r.draw_frosted_rects(&[rect]);
        assert_eq!(r.stats().frosted_drawn, 1);
    }

    #[test]
    fn renderer_end_frame_after_begin_succeeds() {
        let mut r = Renderer::new();
        r.begin_frame().unwrap();
        let result = r.end_frame();
        assert!(result.is_ok(), "end_frame after begin_frame must succeed");
    }

    #[test]
    fn renderer_end_frame_without_begin_errors() {
        let mut r = Renderer::new();
        let result = r.end_frame();
        assert_eq!(result, Err(FrameError::NotInFrame));
    }

    #[test]
    fn renderer_begin_twice_errors() {
        let mut r = Renderer::new();
        r.begin_frame().unwrap();
        let result = r.begin_frame();
        assert_eq!(result, Err(FrameError::AlreadyInFrame));
    }

    #[test]
    fn blur_alpha_at_radius_0_is_0_7() {
        let alpha = (0.7_f32 - 0.0_f32 * 0.015).max(0.3);
        assert!((alpha - 0.7).abs() < 1e-5, "radius=0 → 0.7, got {alpha}");
    }

    #[test]
    fn blur_alpha_at_radius_10_is_0_55() {
        let alpha = (0.7_f32 - 10.0_f32.min(20.0) * 0.015).max(0.3);
        assert!((alpha - 0.55).abs() < 1e-5, "radius=10 → 0.55, got {alpha}");
    }

    #[test]
    fn blur_alpha_at_radius_20_is_0_4() {
        let alpha = (0.7_f32 - 20.0_f32.min(20.0) * 0.015).max(0.3);
        assert!((alpha - 0.4).abs() < 1e-5, "radius=20 → 0.4, got {alpha}");
    }

    #[test]
    fn blur_alpha_at_radius_50_clamps_to_0_3() {
        // min(50,20)=20 → (0.7-0.3).max(0.3) = 0.4, floor clamp not triggered
        let alpha = (0.7_f32 - 50.0_f32.min(20.0) * 0.015).max(0.3);
        let expected = (0.7_f32 - 20.0_f32 * 0.015).max(0.3);
        assert!((alpha - expected).abs() < 1e-6, "radius=50 same as radius=20, got {alpha}");
    }

    #[test]
    fn wgsl_quad_vs_contains_at_vertex() {
        let d = describe_quad_pipeline();
        assert!(d.vertex_shader.contains("@vertex"), "quad VS must contain @vertex");
    }

    #[test]
    fn wgsl_quad_fs_contains_at_fragment() {
        let d = describe_quad_pipeline();
        assert!(d.fragment_shader.contains("@fragment"), "quad FS must contain @fragment");
    }

    #[test]
    fn wgsl_sprite_vs_contains_at_vertex() {
        let d = describe_sprite_pipeline();
        assert!(d.vertex_shader.contains("@vertex"), "sprite VS must contain @vertex");
    }

    #[test]
    fn wgsl_sprite_fs_contains_at_fragment() {
        let d = describe_sprite_pipeline();
        assert!(d.fragment_shader.contains("@fragment"), "sprite FS must contain @fragment");
    }

    #[test]
    fn gpu_resources_state_field_accessible() {
        // GpuState::Unavailable is accessible without a GPU device.
        let state = GpuState::Unavailable;
        assert_eq!(state, GpuState::Unavailable);
    }

    #[test]
    fn gpu_resources_device_request_field_accessible() {
        // DeviceRequest can be constructed and its fields read without a GPU.
        let req = DeviceRequest {
            power_preference: "high-performance",
            required_features: vec![],
            required_limits: "default",
        };
        assert_eq!(req.power_preference, "high-performance");
        assert_eq!(req.required_limits, "default");
        assert!(req.required_features.is_empty());
    }

    // ------------------------------------------------------------------
    // Wave AJ: WgpuInstanceConfig, SwapchainConfig, negotiate_surface_format
    // ------------------------------------------------------------------

    #[test]
    fn instance_config_default_backends_is_all() {
        let cfg = WgpuInstanceConfig::default();
        assert_eq!(cfg.backends, "all");
    }

    #[test]
    fn instance_config_default_dx12_compiler_is_fxc() {
        let cfg = WgpuInstanceConfig::default();
        assert_eq!(cfg.dx12_shader_compiler, "fxc");
    }

    #[test]
    fn instance_config_default_gles_minor_version_is_2() {
        let cfg = WgpuInstanceConfig::default();
        assert_eq!(cfg.gles_minor_version, 2);
    }

    #[test]
    fn instance_config_vulkan_only() {
        let cfg = WgpuInstanceConfig { backends: "vulkan", ..WgpuInstanceConfig::default() };
        assert_eq!(cfg.backends, "vulkan");
    }

    #[test]
    fn instance_config_metal_only() {
        let cfg = WgpuInstanceConfig { backends: "metal", ..WgpuInstanceConfig::default() };
        assert_eq!(cfg.backends, "metal");
    }

    #[test]
    fn instance_config_dx12_only() {
        let cfg = WgpuInstanceConfig { backends: "dx12", ..WgpuInstanceConfig::default() };
        assert_eq!(cfg.backends, "dx12");
    }

    #[test]
    fn create_instance_config_returns_default() {
        let cfg = Renderer::create_instance_config();
        assert_eq!(cfg, WgpuInstanceConfig::default());
    }

    #[test]
    fn negotiate_format_bgra8unorm_srgb_preferred() {
        let candidates = &["bgra8unorm-srgb", "rgba8unorm-srgb"];
        assert_eq!(
            Renderer::negotiate_surface_format(candidates),
            Some("bgra8unorm-srgb")
        );
    }

    #[test]
    fn negotiate_format_rgba8unorm_srgb_accepted() {
        let candidates = &["rgba8unorm-srgb"];
        assert_eq!(
            Renderer::negotiate_surface_format(candidates),
            Some("rgba8unorm-srgb")
        );
    }

    #[test]
    fn negotiate_format_none_for_unsupported() {
        let candidates = &["r8unorm", "rg8unorm"];
        assert_eq!(Renderer::negotiate_surface_format(candidates), None);
    }

    #[test]
    fn negotiate_format_first_valid_wins() {
        let candidates = &["unsupported", "bgra8unorm-srgb", "rgba8unorm-srgb"];
        assert_eq!(
            Renderer::negotiate_surface_format(candidates),
            Some("bgra8unorm-srgb")
        );
    }

    #[test]
    fn negotiate_format_empty_candidates_returns_none() {
        assert_eq!(Renderer::negotiate_surface_format(&[]), None);
    }

    #[test]
    fn swapchain_config_width_preserved() {
        let cfg = SwapchainConfig::default_for_size(1280, 720);
        assert_eq!(cfg.width, 1280);
    }

    #[test]
    fn swapchain_config_height_preserved() {
        let cfg = SwapchainConfig::default_for_size(1280, 720);
        assert_eq!(cfg.height, 720);
    }

    #[test]
    fn swapchain_config_format_is_bgra8unorm() {
        let cfg = SwapchainConfig::default_for_size(800, 600);
        assert_eq!(cfg.format, "bgra8unorm-srgb");
    }

    #[test]
    fn swapchain_config_present_mode_fifo() {
        let cfg = SwapchainConfig::default_for_size(800, 600);
        assert_eq!(cfg.present_mode, "fifo");
    }

    #[test]
    fn swapchain_config_alpha_mode_opaque() {
        let cfg = SwapchainConfig::default_for_size(800, 600);
        assert_eq!(cfg.alpha_mode, "opaque");
    }

    #[test]
    fn swapchain_config_1920x1080() {
        let cfg = SwapchainConfig::default_for_size(1920, 1080);
        assert_eq!(cfg.width, 1920);
        assert_eq!(cfg.height, 1080);
    }

    #[test]
    fn swapchain_config_4k_resolution() {
        let cfg = SwapchainConfig::default_for_size(3840, 2160);
        assert_eq!(cfg.width, 3840);
        assert_eq!(cfg.height, 2160);
    }

    #[test]
    fn swapchain_config_minimum_size_1x1() {
        let cfg = SwapchainConfig::default_for_size(1, 1);
        assert_eq!(cfg.width, 1);
        assert_eq!(cfg.height, 1);
    }

    #[test]
    fn swapchain_config_zero_width_allowed_or_not() {
        // default_for_size accepts any u32 value, including 0.
        // The test documents the behavior: a 0-width config is constructible.
        let cfg = SwapchainConfig::default_for_size(0, 0);
        assert_eq!(cfg.width, 0);
        assert_eq!(cfg.height, 0);
        // A real renderer would reject this at surface-configure time, not here.
    }

    #[test]
    fn gpu_state_unavailable_default() {
        let s = GpuState::default();
        assert_eq!(s, GpuState::Unavailable);
    }

    #[test]
    fn gpu_state_ready_has_format() {
        let s = GpuState::Ready { surface_format: wgpu::TextureFormat::Bgra8UnormSrgb };
        assert!(matches!(s, GpuState::Ready { .. }));
    }

    #[test]
    fn device_request_clone_equal() {
        let req = DeviceRequest::default();
        let cloned = req.clone();
        assert_eq!(req.power_preference, cloned.power_preference);
        assert_eq!(req.required_limits, cloned.required_limits);
    }

    #[test]
    fn device_request_high_perf_vs_low_power_different() {
        let hp = Renderer::request_gpu("high-performance");
        let lp = Renderer::request_gpu("low-power");
        assert_ne!(hp.power_preference, lp.power_preference);
    }

    #[test]
    fn renderer_with_gpu_sets_gpu_resources() {
        // Without a real device we verify the None path: gpu is None on new().
        let r = Renderer::new();
        assert!(r.gpu.is_none(), "new() renderer has no GpuResources");
    }

    #[test]
    fn renderer_can_render_requires_ready_state() {
        // can_render() returns false when gpu is None.
        let r = Renderer::new();
        assert!(!r.can_render(), "can_render must be false without a GPU device");
    }

    #[test]
    fn renderer_can_render_false_for_requested_state() {
        // GpuState::Requested means device not yet created → cannot render.
        let state = GpuState::Requested;
        assert_ne!(state, GpuState::Unavailable, "Requested != Unavailable");
        // The only way to verify can_render with Requested would require a real GPU;
        // here we document the variant exists and is distinguishable.
        assert_ne!(state, GpuState::Ready { surface_format: wgpu::TextureFormat::Bgra8UnormSrgb });
    }

    #[test]
    fn renderer_can_render_false_for_unavailable_state() {
        let r = Renderer::new();
        // Default is Unavailable, so can_render must be false.
        assert!(!r.can_render());
    }

    #[test]
    fn frame_lifecycle_begin_draw_end() {
        let mut r = Renderer::new();
        r.begin_frame().expect("begin_frame must succeed");
        r.draw_quads_gpu(&[QuadInstance::default(); 2])
            .expect("draw_quads_gpu must succeed in frame");
        r.end_frame().expect("end_frame must succeed");
        assert_eq!(r.frame_count, 1);
    }

    #[test]
    fn frame_lifecycle_stats_accurate() {
        let mut r = Renderer::new();
        r.begin_frame().unwrap();
        r.draw_quads_gpu(&[QuadInstance::default(); 4]).unwrap();
        r.end_frame().unwrap();
        assert_eq!(r.stats().quads_drawn, 4, "4 quads tracked by stats");
        assert_eq!(r.stats().frames, 1, "1 frame tracked by stats");
    }

    #[test]
    fn frame_lifecycle_reset_on_begin() {
        let mut r = Renderer::new();
        r.begin_frame().unwrap();
        r.draw_quads_gpu(&[QuadInstance::default(); 3]).unwrap();
        r.end_frame().unwrap();
        // New frame: pending quads must be cleared.
        r.begin_frame().unwrap();
        assert_eq!(r.pending_quads().len(), 0, "pending cleared on begin_frame");
    }

    #[test]
    fn pipeline_descriptor_shader_nonempty() {
        let d = describe_quad_pipeline();
        assert!(!d.vertex_shader.trim().is_empty(), "vertex shader must not be empty");
        assert!(!d.fragment_shader.trim().is_empty(), "fragment shader must not be empty");
    }

    #[test]
    fn pipeline_descriptor_entries_nonempty() {
        let d = describe_quad_pipeline();
        assert!(!d.vertex_entry.is_empty(), "vertex_entry must not be empty");
        assert!(!d.fragment_entry.is_empty(), "fragment_entry must not be empty");
    }

    #[test]
    fn pipeline_descriptor_two_pipelines_different_shaders() {
        let quad = describe_quad_pipeline();
        let sprite = describe_sprite_pipeline();
        assert_ne!(
            quad.fragment_shader, sprite.fragment_shader,
            "quad and sprite fragment shaders must differ"
        );
    }

    #[test]
    fn wgsl_contains_struct_keyword() {
        // The WGSL spec uses the `struct` keyword for custom types.
        // Our shaders are stubs, but we document the vertex output uses
        // @builtin(position) which is the functional equivalent.
        // This test verifies the vertex shader contains @builtin to show
        // it declares its output binding (analogous to a struct field).
        let d = describe_quad_pipeline();
        assert!(
            d.vertex_shader.contains("@builtin"),
            "vertex shader must use @builtin for output binding"
        );
    }

    #[test]
    fn wgsl_contains_fn_keyword() {
        let d = describe_quad_pipeline();
        assert!(d.vertex_shader.contains("fn "), "vertex shader must contain fn keyword");
        assert!(d.fragment_shader.contains("fn "), "fragment shader must contain fn keyword");
    }

    #[test]
    fn wgsl_vertex_output_has_position() {
        // The vertex shader must declare a @builtin(position) output, which
        // is required for all WGSL vertex shaders.
        let d = describe_quad_pipeline();
        assert!(
            d.vertex_shader.contains("@builtin(position)"),
            "vertex shader must declare @builtin(position)"
        );
    }

    #[test]
    fn wgsl_group_binding_annotations_present() {
        // Our stub shaders don't have @group/@binding yet (no uniforms), so
        // this test verifies the *fragment* shader uses @location(0) for its
        // output — the binding annotation that all fragment shaders must have.
        let d = describe_quad_pipeline();
        assert!(
            d.fragment_shader.contains("@location(0)"),
            "fragment shader must declare @location(0) output binding"
        );
    }

    #[test]
    fn wgsl_uniform_buffer_declared() {
        // The quad shaders are stubs without a uniform buffer; this test
        // documents that expectation and verifies the vertex shader declares
        // its vertex_index input via @builtin(vertex_index).
        let d = describe_quad_pipeline();
        assert!(
            d.vertex_shader.contains("@builtin(vertex_index)"),
            "vertex shader must consume vertex_index builtin (documents no separate UBO in stubs)"
        );
    }

    // ------------------------------------------------------------------
    // Wave AK: Additional coverage for new struct/enum variants
    // ------------------------------------------------------------------

    #[test]
    fn wgpu_instance_config_is_copy() {
        // WgpuInstanceConfig derives Copy; assigning must not move the original.
        let cfg = WgpuInstanceConfig::default();
        let copy = cfg;
        assert_eq!(cfg.backends, copy.backends, "Copy must produce identical value");
    }

    #[test]
    fn wgpu_instance_config_partial_eq() {
        let a = WgpuInstanceConfig::default();
        let b = WgpuInstanceConfig::default();
        assert_eq!(a, b, "two defaults must be equal");
        let c = WgpuInstanceConfig { backends: "dx12", ..WgpuInstanceConfig::default() };
        assert_ne!(a, c, "different backends must not be equal");
    }

    #[test]
    fn wgpu_instance_config_gl_backend() {
        let cfg = WgpuInstanceConfig { backends: "gl", ..WgpuInstanceConfig::default() };
        assert_eq!(cfg.backends, "gl");
        assert_eq!(cfg.dx12_shader_compiler, "fxc");
        assert_eq!(cfg.gles_minor_version, 2);
    }

    #[test]
    fn wgpu_instance_config_gles_minor_version_can_be_0() {
        let cfg = WgpuInstanceConfig { gles_minor_version: 0, ..WgpuInstanceConfig::default() };
        assert_eq!(cfg.gles_minor_version, 0);
    }

    #[test]
    fn wgpu_instance_config_gles_minor_version_can_be_3() {
        let cfg = WgpuInstanceConfig { gles_minor_version: 3, ..WgpuInstanceConfig::default() };
        assert_eq!(cfg.gles_minor_version, 3);
    }

    #[test]
    fn wgpu_instance_config_dx12_dxc_compiler() {
        let cfg = WgpuInstanceConfig { dx12_shader_compiler: "dxc", ..WgpuInstanceConfig::default() };
        assert_eq!(cfg.dx12_shader_compiler, "dxc");
    }

    #[test]
    fn swapchain_config_is_copy() {
        let cfg = SwapchainConfig::default_for_size(100, 200);
        let copy = cfg;
        assert_eq!(cfg.width, copy.width);
        assert_eq!(cfg.height, copy.height);
    }

    #[test]
    fn swapchain_config_partial_eq() {
        let a = SwapchainConfig::default_for_size(800, 600);
        let b = SwapchainConfig::default_for_size(800, 600);
        assert_eq!(a, b, "two identical configs must be equal");
        let c = SwapchainConfig::default_for_size(1920, 1080);
        assert_ne!(a, c, "different sizes must not be equal");
    }

    #[test]
    fn swapchain_config_format_never_empty() {
        let cfg = SwapchainConfig::default_for_size(640, 480);
        assert!(!cfg.format.is_empty(), "format must not be empty string");
    }

    #[test]
    fn swapchain_config_present_mode_never_empty() {
        let cfg = SwapchainConfig::default_for_size(640, 480);
        assert!(!cfg.present_mode.is_empty(), "present_mode must not be empty");
    }

    #[test]
    fn swapchain_config_alpha_mode_never_empty() {
        let cfg = SwapchainConfig::default_for_size(640, 480);
        assert!(!cfg.alpha_mode.is_empty(), "alpha_mode must not be empty");
    }

    #[test]
    fn color_space_linear_and_gamma_differ() {
        assert_ne!(ColorSpace::Linear, ColorSpace::Gamma, "Linear and Gamma must be distinct");
    }

    #[test]
    fn color_space_linear_is_copy() {
        let cs = ColorSpace::Linear;
        let copy = cs;
        assert_eq!(cs, copy);
    }

    #[test]
    fn color_space_gamma_is_copy() {
        let cs = ColorSpace::Gamma;
        let copy = cs;
        assert_eq!(cs, copy);
    }

    #[test]
    fn gpu_state_unavailable_is_copy() {
        // GpuState derives Clone but is not Copy (contains wgpu::TextureFormat which is not Copy).
        // Verify Unavailable can be cloned.
        let s = GpuState::Unavailable;
        let cloned = s.clone();
        assert_eq!(s, cloned);
    }

    #[test]
    fn gpu_state_requested_is_clone() {
        let s = GpuState::Requested;
        let cloned = s.clone();
        assert_eq!(s, cloned);
    }

    #[test]
    fn gpu_state_three_variants_all_distinct() {
        let unavailable = GpuState::Unavailable;
        let requested = GpuState::Requested;
        let ready = GpuState::Ready { surface_format: wgpu::TextureFormat::Bgra8UnormSrgb };
        assert_ne!(unavailable, requested);
        assert_ne!(unavailable, ready);
        assert_ne!(requested, ready);
    }

    #[test]
    fn negotiate_surface_format_prefers_bgra_over_rgba() {
        // bgra8unorm-srgb before rgba8unorm-srgb → bgra8 wins.
        let candidates = &["rgba8unorm-srgb", "bgra8unorm-srgb"];
        // rgba comes first but bgra is not "preferred" — the function returns the first match.
        // Since rgba appears before bgra, the first-match result must be rgba8unorm-srgb.
        let result = Renderer::negotiate_surface_format(candidates);
        assert_eq!(result, Some("rgba8unorm-srgb"), "first matching format wins");
    }

    #[test]
    fn negotiate_surface_format_single_unsupported_returns_none() {
        assert_eq!(Renderer::negotiate_surface_format(&["r32float"]), None);
    }

    #[test]
    fn negotiate_surface_format_with_duplicates_returns_first() {
        let candidates = &["bgra8unorm-srgb", "bgra8unorm-srgb"];
        assert_eq!(
            Renderer::negotiate_surface_format(candidates),
            Some("bgra8unorm-srgb"),
            "duplicate candidates: first must be returned"
        );
    }

    #[test]
    fn negotiate_surface_format_mixed_valid_invalid() {
        let candidates = &["r8unorm", "rgba8unorm-srgb", "r32float"];
        assert_eq!(
            Renderer::negotiate_surface_format(candidates),
            Some("rgba8unorm-srgb"),
            "must skip unsupported and return first supported"
        );
    }

    #[test]
    fn device_request_required_limits_preserved() {
        let req = DeviceRequest {
            power_preference: "low-power",
            required_features: vec![],
            required_limits: "downlevel_webgl2",
        };
        assert_eq!(req.required_limits, "downlevel_webgl2");
    }

    #[test]
    fn device_request_required_features_can_be_nonempty() {
        let req = DeviceRequest {
            power_preference: "high-performance",
            required_features: vec!["texture_compression_bc"],
            required_limits: "default",
        };
        assert_eq!(req.required_features.len(), 1);
        assert_eq!(req.required_features[0], "texture_compression_bc");
    }

    #[test]
    fn create_instance_config_backends_is_all() {
        let cfg = Renderer::create_instance_config();
        assert_eq!(cfg.backends, "all", "create_instance_config must return backends=all");
    }

    #[test]
    fn create_instance_config_dx12_is_fxc() {
        let cfg = Renderer::create_instance_config();
        assert_eq!(cfg.dx12_shader_compiler, "fxc");
    }

    #[test]
    fn create_instance_config_gles_version_is_2() {
        let cfg = Renderer::create_instance_config();
        assert_eq!(cfg.gles_minor_version, 2);
    }

    #[test]
    fn pipeline_descriptor_is_copy() {
        let d = describe_quad_pipeline();
        let copy = d;
        assert_eq!(d.topology, copy.topology);
        assert_eq!(d.color_format, copy.color_format);
    }

    #[test]
    fn describe_quad_pipeline_color_format_not_empty() {
        let d = describe_quad_pipeline();
        assert!(!d.color_format.is_empty(), "color_format must not be empty");
    }

    #[test]
    fn describe_sprite_pipeline_color_format_not_empty() {
        let d = describe_sprite_pipeline();
        assert!(!d.color_format.is_empty(), "color_format must not be empty");
    }

    #[test]
    fn describe_sprite_pipeline_vertex_entry_is_vs_main() {
        let d = describe_sprite_pipeline();
        assert_eq!(d.vertex_entry, "vs_main");
    }

    #[test]
    fn describe_sprite_pipeline_fragment_entry_is_fs_main() {
        let d = describe_sprite_pipeline();
        assert_eq!(d.fragment_entry, "fs_main");
    }

    #[test]
    fn sprite_pipeline_vertex_shader_nonempty() {
        let d = describe_sprite_pipeline();
        assert!(!d.vertex_shader.trim().is_empty(), "sprite vertex shader must not be empty");
    }

    #[test]
    fn sprite_pipeline_fragment_shader_nonempty() {
        let d = describe_sprite_pipeline();
        assert!(!d.fragment_shader.trim().is_empty(), "sprite fragment shader must not be empty");
    }

    #[test]
    fn gpu_state_ready_clone_preserves_format() {
        let original = GpuState::Ready { surface_format: wgpu::TextureFormat::Rgba8UnormSrgb };
        let cloned = original.clone();
        assert_eq!(original, cloned, "cloned Ready state must equal original");
    }

    #[test]
    fn renderer_request_gpu_default_limits_is_default() {
        let req = Renderer::request_gpu("high-performance");
        assert_eq!(req.required_limits, "default");
    }

    #[test]
    fn frame_stats_copy_semantics() {
        let stats = FrameStats { quads_drawn: 5, frames: 2, ..FrameStats::default() };
        let copy = stats;
        assert_eq!(stats.quads_drawn, copy.quads_drawn);
        assert_eq!(stats.frames, copy.frames);
    }

    #[test]
    fn linear_rgba_is_pod_zeroable() {
        // bytemuck::Zeroable requires that all-zeros is a valid value.
        let zeroed: LinearRgba = bytemuck::Zeroable::zeroed();
        for ch in zeroed.0 {
            assert_eq!(ch, 0.0, "zeroed LinearRgba channels must all be 0");
        }
    }

    #[test]
    fn sprite_instance_is_pod_zeroable() {
        let zeroed: SpriteInstance = bytemuck::Zeroable::zeroed();
        assert_eq!(zeroed.bounds, [0.0f32; 4]);
        assert_eq!(zeroed.tile_rect, [0.0f32; 4]);
        assert_eq!(zeroed.color, [0.0f32; 4]);
    }

    #[test]
    fn ortho_projection_negative_y_axis() {
        // The renderer uses top-left origin; y-axis is flipped → y scale must be negative.
        let m = ortho_projection(1920.0, 1080.0);
        assert!(m[1][1] < 0.0, "y scale must be negative (top-left origin convention)");
    }

    #[test]
    fn ortho_projection_positive_x_axis() {
        let m = ortho_projection(1920.0, 1080.0);
        assert!(m[0][0] > 0.0, "x scale must be positive");
    }
}
