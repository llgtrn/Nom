//! Main draw loop: consumes a [`Scene`] and issues wgpu draw calls.
//!
//! [`Renderer`] owns the six instance buffers (one per dispatched primitive kind),
//! the shared globals uniform buffer, and the shared atlas sampler. Each frame
//! the caller builds a [`wgpu::CommandEncoder`], calls [`Renderer::draw`], then
//! submits the encoder.
//!
//! # Batch-2 primitives (handled)
//! - [`PrimitiveBatch::Quads`] — rounded rectangles via `pipelines.quads`
//! - [`PrimitiveBatch::MonochromeSprites`] — atlas glyphs via `pipelines.mono_sprites`
//! - [`PrimitiveBatch::Underlines`] — horizontal strokes via `pipelines.underlines`
//!
//! # Batch-3 primitives (handled)
//! - [`PrimitiveBatch::Shadows`] — Gaussian drop shadows via `pipelines.shadows`
//! - [`PrimitiveBatch::PolychromeSprites`] — RGBA atlas sprites via `pipelines.poly_sprites`
//! - [`PrimitiveBatch::SubpixelSprites`] — LCD subpixel text via `pipelines.subpixel_sprites`
//!
//! # Batch-4 primitives (skipped / logged)
//! - `Paths` — two-pass bezier renderer, not yet dispatched

#![deny(unsafe_code)]

use std::collections::HashMap;
use std::sync::Arc;

use crate::context::GpuContext;
use crate::pipelines::Pipelines;
use crate::buffers::InstanceBuffer;
use crate::scene::{
    PrimitiveBatch, Quad, MonochromeSprite, Underline, Shadow, PolychromeSprite, SubpixelSprite,
    Scene,
};
use crate::wgpu_atlas::GpuAtlas;

// ── Per-frame globals uniform ─────────────────────────────────────────────────

/// Matches `RenderParams` in all three WGSL shaders:
/// ```wgsl
/// struct RenderParams {
///     viewport_size: vec2<f32>,
///     premultiplied_alpha: u32,
///     _padding: u32,
/// }
/// ```
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct FrameGlobals {
    viewport_size: [f32; 2],
    premultiplied_alpha: u32,
    _padding: u32,
}

// ── Instance structs (must match WGSL layouts exactly) ────────────────────────

/// Matches `QuadInstance` in `quad.wgsl`:
/// ```wgsl
/// struct QuadInstance {
///     bounds:        Rect,        // origin(xy) + size(xy) = 4×f32
///     clip_bounds:   Rect,
///     corner_radii:  vec4<f32>,   // tl, tr, br, bl
///     background:    vec4<f32>,
///     border_color:  vec4<f32>,
///     border_widths: vec4<f32>,   // top, right, bottom, left
/// }
/// ```
/// Total: 6 × 16 = 96 bytes. No padding holes.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuQuad {
    bounds: [f32; 4],        // origin.x, origin.y, size.width, size.height
    clip_bounds: [f32; 4],
    corner_radii: [f32; 4],  // top_left, top_right, bottom_right, bottom_left
    background: [f32; 4],    // r, g, b, a
    border_color: [f32; 4],
    border_widths: [f32; 4], // top, right, bottom, left
}

/// Matches `MonoSpriteInstance` in `mono_sprite.wgsl`:
/// ```wgsl
/// struct MonoSpriteInstance {
///     bounds:      Rect,        // 4×f32
///     clip_bounds: Rect,        // 4×f32
///     color:       vec4<f32>,
///     uv_min:      vec2<f32>,
///     uv_max:      vec2<f32>,
/// }
/// ```
/// Total: 4×16 = 64 bytes. No padding holes.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuMonoSprite {
    bounds: [f32; 4],
    clip_bounds: [f32; 4],
    color: [f32; 4],
    uv_min: [f32; 2],
    uv_max: [f32; 2],
}

/// Matches `UnderlineInstance` in `underline.wgsl`:
/// ```wgsl
/// struct UnderlineInstance {
///     bounds:      Rect,        // 4×f32
///     clip_bounds: Rect,        // 4×f32
///     color:       vec4<f32>,
///     thickness:   f32,
///     _pad:        vec3<f32>,   // 3×f32 explicit padding
/// }
/// ```
/// Total: 4×16 = 64 bytes. The `_pad` field aligns `thickness` to a full vec4.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuUnderline {
    bounds: [f32; 4],
    clip_bounds: [f32; 4],
    color: [f32; 4],
    thickness: f32,
    _pad: [f32; 3],
}

/// Matches `ShadowInstance` in `shadow.wgsl`:
/// ```wgsl
/// struct ShadowInstance {
///     bounds:       Rect,        // 4×f32  @offset(0)
///     clip_bounds:  Rect,        // 4×f32  @offset(16)
///     corner_radii: vec4<f32>,           @offset(32)
///     color:        vec4<f32>,           @offset(48)
///     blur_radius:  f32,                 @offset(64)
///     _pad:         vec3<f32>,           @offset(80)
/// }
/// ```
/// WGSL layout: blur_radius at 64 (4B), vec3<f32> has AlignOf=16 so
/// _pad lands at offset 80 (12B implicit gap at 68..80), _pad occupies 80..92,
/// struct rounds up to AlignOf=16 → total 96 bytes.
/// Rust repr: blur_radius(4) + _pad(7×f32=28) = 32 bytes after color, total 96.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuShadow {
    bounds: [f32; 4],       // offset  0, size 16
    clip_bounds: [f32; 4],  // offset 16, size 16
    corner_radii: [f32; 4], // offset 32, size 16
    color: [f32; 4],        // offset 48, size 16
    blur_radius: f32,       // offset 64, size  4
    _pad: [f32; 7],         // offset 68, size 28 → total 96
}

/// Matches `PolySpriteInstance` in `poly_sprite.wgsl`:
/// ```wgsl
/// struct PolySpriteInstance {
///     bounds:      Rect,         // 4×f32
///     clip_bounds: Rect,         // 4×f32
///     uv_min:      vec2<f32>,
///     uv_max:      vec2<f32>,
///     grayscale:   u32,
///     _pad:        vec3<u32>,    // 3×u32 explicit padding
/// }
/// ```
/// Total: 4×16 = 64 bytes.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuPolySprite {
    bounds: [f32; 4],
    clip_bounds: [f32; 4],
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    grayscale: u32,
    _pad: [u32; 3],
}

/// Matches `SubpixelSpriteInstance` in `subpixel_sprite.wgsl`:
/// ```wgsl
/// struct SubpixelSpriteInstance {
///     bounds:      Rect,         // 4×f32
///     clip_bounds: Rect,         // 4×f32
///     color:       vec4<f32>,
///     uv_min:      vec2<f32>,
///     uv_max:      vec2<f32>,
/// }
/// ```
/// Total: 4×16 = 64 bytes.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuSubpixelSprite {
    bounds: [f32; 4],
    clip_bounds: [f32; 4],
    color: [f32; 4],
    uv_min: [f32; 2],
    uv_max: [f32; 2],
}

// ── Conversion helpers ────────────────────────────────────────────────────────

fn rgba_to_array(c: crate::color::LinearRgba) -> [f32; 4] {
    [c.r, c.g, c.b, c.a]
}

// ── Renderer ──────────────────────────────────────────────────────────────────

/// Records wgpu draw calls for a [`Scene`] into a caller-supplied encoder.
///
/// Owns six [`InstanceBuffer`]s (quad, sprite, underline, shadow, poly_sprite,
/// subpixel_sprite) that are reset and refilled every frame, plus one uniform
/// buffer for [`FrameGlobals`] and one shared [`wgpu::Sampler`] for all sprite
/// pipelines.
pub struct Renderer {
    pipelines: Pipelines,
    quad_buffer: InstanceBuffer,
    sprite_buffer: InstanceBuffer,
    underline_buffer: InstanceBuffer,
    shadow_buffer: InstanceBuffer,
    poly_sprite_buffer: InstanceBuffer,
    subpixel_sprite_buffer: InstanceBuffer,
    globals_buffer: wgpu::Buffer,
    sampler: wgpu::Sampler,
}

impl Renderer {
    /// Construct a new renderer for the given surface pixel format.
    ///
    /// Compiles the render pipelines and allocates initial 2 MiB instance
    /// buffers for each primitive kind.
    pub fn new(ctx: &GpuContext, surface_format: wgpu::TextureFormat) -> Self {
        let device = &ctx.device;

        let pipelines = Pipelines::new(ctx, surface_format);

        let quad_buffer = InstanceBuffer::new(device);
        let sprite_buffer = InstanceBuffer::new(device);
        let underline_buffer = InstanceBuffer::new(device);
        let shadow_buffer = InstanceBuffer::new(device);
        let poly_sprite_buffer = InstanceBuffer::new(device);
        let subpixel_sprite_buffer = InstanceBuffer::new(device);

        // 16-byte uniform buffer for FrameGlobals (one RenderParams struct).
        let globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nom_gpui_globals_buf"),
            size: std::mem::size_of::<FrameGlobals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Bilinear sampler shared by all sprite pipelines.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("nom_gpui_atlas_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            pipelines,
            quad_buffer,
            sprite_buffer,
            underline_buffer,
            shadow_buffer,
            poly_sprite_buffer,
            subpixel_sprite_buffer,
            globals_buffer,
            sampler,
        }
    }

    /// Record draw commands for `scene` into `encoder`, targeting `view`.
    ///
    /// Steps:
    /// 1. `atlas.flush_uploads()` — submit any queued glyph/image pixel uploads.
    /// 2. Write [`FrameGlobals`] to the uniform buffer.
    /// 3. Reset all instance buffers (`begin_frame`).
    /// 4. Pre-collect atlas texture views (must outlive the render pass borrow).
    /// 5. Open a render pass (clear to transparent black).
    /// 6. For each [`PrimitiveBatch`]:
    ///    - Quads / MonochromeSprites / Underlines / Shadows / PolychromeSprites /
    ///      SubpixelSprites → convert instances, write buffer, build bind groups,
    ///      set pipeline, draw.
    ///    - Paths → skipped (batch-4, two-pass renderer not yet integrated).
    /// 7. End render pass. Caller submits the encoder.
    pub fn draw(
        &mut self,
        ctx: &GpuContext,
        atlas: &Arc<GpuAtlas>,
        scene: &Scene,
        view: &wgpu::TextureView,
        viewport_size: (u32, u32),
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let device = &ctx.device;
        let queue = &ctx.queue;

        // Step 1: flush queued atlas pixel uploads before the render pass.
        atlas.flush_uploads();

        // Step 2: write per-frame globals.
        let globals = FrameGlobals {
            viewport_size: [viewport_size.0 as f32, viewport_size.1 as f32],
            premultiplied_alpha: 0, // non-premultiplied for MVP
            _padding: 0,
        };
        queue.write_buffer(&self.globals_buffer, 0, bytemuck::bytes_of(&globals));

        // Step 3: reset instance buffer cursors.
        self.quad_buffer.begin_frame();
        self.sprite_buffer.begin_frame();
        self.underline_buffer.begin_frame();
        self.shadow_buffer.begin_frame();
        self.poly_sprite_buffer.begin_frame();
        self.subpixel_sprite_buffer.begin_frame();

        // Build the globals bind group (recreated each frame; cheap).
        let globals_bg = self.pipelines.bind_globals(device, &self.globals_buffer);

        // Step 4: pre-collect atlas texture views keyed by (SpriteKind, AtlasTextureId).
        //
        // Views must outlive the render pass borrow, so we gather them before
        // opening the pass. Keying by (kind, texture_id) eliminates the fragile
        // positional iterator that could silently desync when the two traversals
        // visit batches in different sequences.
        #[derive(Clone, Copy, Hash, Eq, PartialEq)]
        enum SpriteKind { Mono, Poly, Subpixel }

        let mut sprite_views: HashMap<(SpriteKind, crate::atlas::AtlasTextureId), wgpu::TextureView> =
            HashMap::new();
        for batch in scene.batches() {
            match batch {
                PrimitiveBatch::MonochromeSprites { texture_id, .. } => {
                    if !sprite_views.contains_key(&(SpriteKind::Mono, texture_id)) {
                        if let Some(view) = atlas.texture_view(texture_id) {
                            sprite_views.insert((SpriteKind::Mono, texture_id), view);
                        }
                    }
                }
                PrimitiveBatch::PolychromeSprites { texture_id, .. } => {
                    if !sprite_views.contains_key(&(SpriteKind::Poly, texture_id)) {
                        if let Some(view) = atlas.texture_view(texture_id) {
                            sprite_views.insert((SpriteKind::Poly, texture_id), view);
                        }
                    }
                }
                PrimitiveBatch::SubpixelSprites { texture_id, .. } => {
                    if !sprite_views.contains_key(&(SpriteKind::Subpixel, texture_id)) {
                        if let Some(view) = atlas.texture_view(texture_id) {
                            sprite_views.insert((SpriteKind::Subpixel, texture_id), view);
                        }
                    }
                }
                _ => {}
            }
        }

        // Step 5: open render pass and process each batch.
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("nom_gpui_main_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            for batch in scene.batches() {
                match batch {
                    PrimitiveBatch::Quads(quads) => {
                        self.draw_quads(device, queue, &globals_bg, quads, &mut pass);
                    }
                    PrimitiveBatch::MonochromeSprites { texture_id, sprites } => {
                        if let Some(atlas_view) =
                            sprite_views.get(&(SpriteKind::Mono, texture_id))
                        {
                            self.draw_mono_sprites(
                                device,
                                queue,
                                &globals_bg,
                                sprites,
                                atlas_view,
                                &mut pass,
                            );
                        } else {
                            eprintln!("nom_gpui: no atlas view for MonochromeSprites batch; batch dropped");
                        }
                    }
                    PrimitiveBatch::Underlines(underlines) => {
                        self.draw_underlines(device, queue, &globals_bg, underlines, &mut pass);
                    }
                    PrimitiveBatch::Shadows(shadows) => {
                        self.draw_shadows(device, queue, &globals_bg, shadows, &mut pass);
                    }
                    PrimitiveBatch::PolychromeSprites { texture_id, sprites } => {
                        if let Some(atlas_view) =
                            sprite_views.get(&(SpriteKind::Poly, texture_id))
                        {
                            self.draw_poly_sprites(
                                device,
                                queue,
                                &globals_bg,
                                sprites,
                                atlas_view,
                                &mut pass,
                            );
                        } else {
                            eprintln!("nom_gpui: no atlas view for PolychromeSprites batch; batch dropped");
                        }
                    }
                    PrimitiveBatch::SubpixelSprites { texture_id, sprites } => {
                        if let Some(atlas_view) =
                            sprite_views.get(&(SpriteKind::Subpixel, texture_id))
                        {
                            self.draw_subpixel_sprites(
                                device,
                                queue,
                                &globals_bg,
                                sprites,
                                atlas_view,
                                &mut pass,
                            );
                        } else {
                            eprintln!("nom_gpui: no atlas view for SubpixelSprites batch; batch dropped");
                        }
                    }
                    PrimitiveBatch::Paths(paths) => {
                        self.draw_paths(paths);
                    }
                }
            }
        }
        // Step 7: render pass dropped here; caller submits the encoder.
    }

    /// Batch-4 stub: log the paths batch and return without issuing draw calls.
    ///
    /// # Full two-pass drop-pass algorithm (implement in batch-4)
    ///
    /// Path rendering requires MSAA anti-aliasing via a two-pass approach.
    /// The render pass that is currently open (`nom_gpui_main_pass`) must be
    /// **ended** before the path passes begin, then **reopened** with
    /// `LoadOp::Load` afterward so previously rendered quads/sprites are
    /// preserved.  The full sequence is:
    ///
    /// 1. **Drop the surface render pass** — end the current
    ///    `nom_gpui_main_pass` by dropping the `RenderPass` guard.
    ///
    /// 2. **Allocate MSAA intermediate texture** — create (or reuse from cache)
    ///    a 4x MSAA `wgpu::Texture` sized to `(viewport_size.0, viewport_size.1)`
    ///    with format matching the surface.  Also allocate a matching non-MSAA
    ///    resolve target texture.  Cache both textures keyed by size to avoid
    ///    per-frame allocations.
    ///
    /// 3. **Path rasterization pass** — begin a new `RenderPass` targeting the
    ///    MSAA texture with `resolve_target = Some(&resolve_view)`.
    ///    For each `Path` in the slice:
    ///    a. Tessellate `path.vertices` (a polygon) into a triangle fan:
    ///       fan triangulation: `v0-v1-v2, v0-v2-v3, ..., v0-v(n-2)-v(n-1)`.
    ///       Write `PathVertex { position: [x, y], color: [r, g, b, a] }` for
    ///       each vertex into the `path_vertex_buffer` (a plain vertex buffer,
    ///       not a storage buffer).
    ///    b. Issue `draw(0..vertex_count, 0..1)` with the `path_rasterization`
    ///       pipeline.  The pipeline writes to the MSAA attachment; WGPU
    ///       auto-resolves to the non-MSAA texture when the pass ends.
    ///    End this pass.
    ///
    /// 4. **Composite pass** — reopen the surface render pass with
    ///    `LoadOp::Load` (to preserve already-drawn content).  Bind the
    ///    resolved non-MSAA texture view as a sampled texture in bind group 1.
    ///    Set the `paths` pipeline (a full-screen quad that samples the resolved
    ///    texture and alpha-composites it over the existing surface content).
    ///    Issue `draw(0..4, 0..1)` to emit the full-screen quad.
    ///    Continue with remaining primitive batches inside this reopened pass.
    ///
    /// Reference: Zed `wgpu_renderer.rs:1218-1256`.
    fn draw_paths(&self, paths: &[crate::scene::Path]) {
        if paths.is_empty() {
            return;
        }
        let total_vertices: usize = paths.iter().map(|p| p.vertices.len()).sum();
        // batch-4 two-pass path render deferred — log via eprintln when NOM_LOG_PATHS is set
        if std::env::var("NOM_LOG_PATHS").is_ok() {
            eprintln!(
                "nom_gpui: Paths batch (count={}, vertices={}) — batch-4 two-pass render not yet implemented",
                paths.len(), total_vertices
            );
        }
    }

    // ── private batch dispatchers ─────────────────────────────────────────────

    fn draw_quads<'pass>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        globals_bg: &'pass wgpu::BindGroup,
        quads: &[Quad],
        pass: &mut wgpu::RenderPass<'pass>,
    ) {
        if quads.is_empty() {
            return;
        }

        let gpu_quads: Vec<GpuQuad> = quads.iter().map(quad_to_gpu).collect();
        let bytes = bytemuck::cast_slice(&gpu_quads);

        let slice = match self.quad_buffer.write(queue, bytes) {
            Some(s) => s,
            None => {
                // Overflow: grow and retry once.
                if self.quad_buffer.grow(device).is_err() {
                    eprintln!("nom_gpui: quad_buffer at device max; batch dropped");
                    return;
                }
                match self.quad_buffer.write(queue, bytes) {
                    Some(s) => s,
                    None => {
                        eprintln!("nom_gpui: quad_buffer write failed after grow; batch dropped");
                        return;
                    }
                }
            }
        };

        let instances_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nom_gpui_quad_instances_bg"),
            layout: &self.pipelines.instances_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: self.quad_buffer.buffer(),
                    offset: slice.offset,
                    size: Some(slice.size),
                }),
            }],
        });

        pass.set_pipeline(&self.pipelines.quads);
        pass.set_bind_group(0, globals_bg, &[]);
        pass.set_bind_group(1, &instances_bg, &[]);
        // 4 vertices (TriangleStrip quad), N instances.
        pass.draw(0..4, 0..quads.len() as u32);
    }

    fn draw_mono_sprites<'pass>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        globals_bg: &'pass wgpu::BindGroup,
        sprites: &[MonochromeSprite],
        atlas_view: &'pass wgpu::TextureView,
        pass: &mut wgpu::RenderPass<'pass>,
    ) {
        if sprites.is_empty() {
            return;
        }

        let gpu_sprites: Vec<GpuMonoSprite> = sprites.iter().map(mono_sprite_to_gpu).collect();
        let bytes = bytemuck::cast_slice(&gpu_sprites);

        let slice = match self.sprite_buffer.write(queue, bytes) {
            Some(s) => s,
            None => {
                if self.sprite_buffer.grow(device).is_err() {
                    eprintln!("nom_gpui: sprite_buffer at device max; batch dropped");
                    return;
                }
                match self.sprite_buffer.write(queue, bytes) {
                    Some(s) => s,
                    None => {
                        eprintln!("nom_gpui: sprite_buffer write failed after grow; batch dropped");
                        return;
                    }
                }
            }
        };

        let instances_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nom_gpui_sprite_instances_bg"),
            layout: &self.pipelines.sprite_instances_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: self.sprite_buffer.buffer(),
                        offset: slice.offset,
                        size: Some(slice.size),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        pass.set_pipeline(&self.pipelines.mono_sprites);
        pass.set_bind_group(0, globals_bg, &[]);
        pass.set_bind_group(1, &instances_bg, &[]);
        pass.draw(0..4, 0..sprites.len() as u32);
    }

    fn draw_underlines<'pass>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        globals_bg: &'pass wgpu::BindGroup,
        underlines: &[Underline],
        pass: &mut wgpu::RenderPass<'pass>,
    ) {
        if underlines.is_empty() {
            return;
        }

        let gpu_underlines: Vec<GpuUnderline> =
            underlines.iter().map(underline_to_gpu).collect();
        let bytes = bytemuck::cast_slice(&gpu_underlines);

        let slice = match self.underline_buffer.write(queue, bytes) {
            Some(s) => s,
            None => {
                if self.underline_buffer.grow(device).is_err() {
                    eprintln!("nom_gpui: underline_buffer at device max; batch dropped");
                    return;
                }
                match self.underline_buffer.write(queue, bytes) {
                    Some(s) => s,
                    None => {
                        eprintln!("nom_gpui: underline_buffer write failed after grow; batch dropped");
                        return;
                    }
                }
            }
        };

        let instances_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nom_gpui_underline_instances_bg"),
            layout: &self.pipelines.instances_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: self.underline_buffer.buffer(),
                    offset: slice.offset,
                    size: Some(slice.size),
                }),
            }],
        });

        pass.set_pipeline(&self.pipelines.underlines);
        pass.set_bind_group(0, globals_bg, &[]);
        pass.set_bind_group(1, &instances_bg, &[]);
        pass.draw(0..4, 0..underlines.len() as u32);
    }

    fn draw_shadows<'pass>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        globals_bg: &'pass wgpu::BindGroup,
        shadows: &[Shadow],
        pass: &mut wgpu::RenderPass<'pass>,
    ) {
        if shadows.is_empty() {
            return;
        }

        let gpu_shadows: Vec<GpuShadow> = shadows.iter().map(shadow_to_gpu).collect();
        let bytes = bytemuck::cast_slice(&gpu_shadows);

        let slice = match self.shadow_buffer.write(queue, bytes) {
            Some(s) => s,
            None => {
                if self.shadow_buffer.grow(device).is_err() {
                    eprintln!("nom_gpui: shadow_buffer at device max; batch dropped");
                    return;
                }
                match self.shadow_buffer.write(queue, bytes) {
                    Some(s) => s,
                    None => {
                        eprintln!("nom_gpui: shadow_buffer write failed after grow; batch dropped");
                        return;
                    }
                }
            }
        };

        // Shadows use instances_bgl (plain storage buffer, no texture).
        let instances_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nom_gpui_shadow_instances_bg"),
            layout: &self.pipelines.instances_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: self.shadow_buffer.buffer(),
                    offset: slice.offset,
                    size: Some(slice.size),
                }),
            }],
        });

        pass.set_pipeline(&self.pipelines.shadows);
        pass.set_bind_group(0, globals_bg, &[]);
        pass.set_bind_group(1, &instances_bg, &[]);
        pass.draw(0..4, 0..shadows.len() as u32);
    }

    fn draw_poly_sprites<'pass>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        globals_bg: &'pass wgpu::BindGroup,
        sprites: &[PolychromeSprite],
        atlas_view: &'pass wgpu::TextureView,
        pass: &mut wgpu::RenderPass<'pass>,
    ) {
        if sprites.is_empty() {
            return;
        }

        let gpu_sprites: Vec<GpuPolySprite> = sprites.iter().map(poly_sprite_to_gpu).collect();
        let bytes = bytemuck::cast_slice(&gpu_sprites);

        let slice = match self.poly_sprite_buffer.write(queue, bytes) {
            Some(s) => s,
            None => {
                if self.poly_sprite_buffer.grow(device).is_err() {
                    eprintln!("nom_gpui: poly_sprite_buffer at device max; batch dropped");
                    return;
                }
                match self.poly_sprite_buffer.write(queue, bytes) {
                    Some(s) => s,
                    None => {
                        eprintln!("nom_gpui: poly_sprite_buffer write failed after grow; batch dropped");
                        return;
                    }
                }
            }
        };

        let instances_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nom_gpui_poly_sprite_instances_bg"),
            layout: &self.pipelines.sprite_instances_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: self.poly_sprite_buffer.buffer(),
                        offset: slice.offset,
                        size: Some(slice.size),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        pass.set_pipeline(&self.pipelines.poly_sprites);
        pass.set_bind_group(0, globals_bg, &[]);
        pass.set_bind_group(1, &instances_bg, &[]);
        pass.draw(0..4, 0..sprites.len() as u32);
    }

    fn draw_subpixel_sprites<'pass>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        globals_bg: &'pass wgpu::BindGroup,
        sprites: &[SubpixelSprite],
        atlas_view: &'pass wgpu::TextureView,
        pass: &mut wgpu::RenderPass<'pass>,
    ) {
        if sprites.is_empty() {
            return;
        }

        // Guard: subpixel_sprites pipeline may be None if dual_source_blending unsupported.
        if self.pipelines.subpixel_sprites.is_none() {
            return;
        }

        let gpu_sprites: Vec<GpuSubpixelSprite> =
            sprites.iter().map(subpixel_sprite_to_gpu).collect();
        let bytes = bytemuck::cast_slice(&gpu_sprites);

        let slice = match self.subpixel_sprite_buffer.write(queue, bytes) {
            Some(s) => s,
            None => {
                if self.subpixel_sprite_buffer.grow(device).is_err() {
                    eprintln!("nom_gpui: subpixel_sprite_buffer at device max; batch dropped");
                    return;
                }
                match self.subpixel_sprite_buffer.write(queue, bytes) {
                    Some(s) => s,
                    None => {
                        eprintln!("nom_gpui: subpixel_sprite_buffer write failed after grow; batch dropped");
                        return;
                    }
                }
            }
        };

        let instances_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nom_gpui_subpixel_sprite_instances_bg"),
            layout: &self.pipelines.sprite_instances_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: self.subpixel_sprite_buffer.buffer(),
                        offset: slice.offset,
                        size: Some(slice.size),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        // Safe: checked is_some() above.
        let pipeline = self.pipelines.subpixel_sprites.as_ref().unwrap();
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, globals_bg, &[]);
        pass.set_bind_group(1, &instances_bg, &[]);
        pass.draw(0..4, 0..sprites.len() as u32);
    }
}

// ── Scene-type → GPU-type conversions ────────────────────────────────────────

fn quad_to_gpu(q: &Quad) -> GpuQuad {
    GpuQuad {
        bounds: [
            q.bounds.origin.x.0,
            q.bounds.origin.y.0,
            q.bounds.size.width.0,
            q.bounds.size.height.0,
        ],
        clip_bounds: [
            q.clip_bounds.origin.x.0,
            q.clip_bounds.origin.y.0,
            q.clip_bounds.size.width.0,
            q.clip_bounds.size.height.0,
        ],
        // Corners order matches WGSL comment: (tl, tr, br, bl)
        corner_radii: [
            q.corner_radii.top_left.0,
            q.corner_radii.top_right.0,
            q.corner_radii.bottom_right.0,
            q.corner_radii.bottom_left.0,
        ],
        background: rgba_to_array(q.background),
        border_color: rgba_to_array(q.border_color),
        // border_widths: [top, right, bottom, left]
        border_widths: [
            q.border_widths[0].0, // top
            q.border_widths[1].0, // right
            q.border_widths[2].0, // bottom
            q.border_widths[3].0, // left
        ],
    }
}

fn mono_sprite_to_gpu(s: &MonochromeSprite) -> GpuMonoSprite {
    // uv is stored as [uv_min_x, uv_min_y, uv_max_x, uv_max_y] in AtlasTileRef.uv
    GpuMonoSprite {
        bounds: [
            s.bounds.origin.x.0,
            s.bounds.origin.y.0,
            s.bounds.size.width.0,
            s.bounds.size.height.0,
        ],
        clip_bounds: [
            s.clip_bounds.origin.x.0,
            s.clip_bounds.origin.y.0,
            s.clip_bounds.size.width.0,
            s.clip_bounds.size.height.0,
        ],
        color: rgba_to_array(s.color),
        uv_min: [s.tile.uv[0], s.tile.uv[1]],
        uv_max: [s.tile.uv[2], s.tile.uv[3]],
    }
}

fn underline_to_gpu(u: &Underline) -> GpuUnderline {
    GpuUnderline {
        bounds: [
            u.bounds.origin.x.0,
            u.bounds.origin.y.0,
            u.bounds.size.width.0,
            u.bounds.size.height.0,
        ],
        clip_bounds: [
            u.clip_bounds.origin.x.0,
            u.clip_bounds.origin.y.0,
            u.clip_bounds.size.width.0,
            u.clip_bounds.size.height.0,
        ],
        color: rgba_to_array(u.color),
        thickness: u.thickness.0,
        _pad: [0.0; 3],
    }
}

fn shadow_to_gpu(s: &Shadow) -> GpuShadow {
    GpuShadow {
        bounds: [
            s.bounds.origin.x.0,
            s.bounds.origin.y.0,
            s.bounds.size.width.0,
            s.bounds.size.height.0,
        ],
        clip_bounds: [
            s.clip_bounds.origin.x.0,
            s.clip_bounds.origin.y.0,
            s.clip_bounds.size.width.0,
            s.clip_bounds.size.height.0,
        ],
        corner_radii: [
            s.corner_radii.top_left.0,
            s.corner_radii.top_right.0,
            s.corner_radii.bottom_right.0,
            s.corner_radii.bottom_left.0,
        ],
        color: rgba_to_array(s.color),
        blur_radius: s.blur_radius.0,
        _pad: [0.0; 7],
    }
}

fn poly_sprite_to_gpu(s: &PolychromeSprite) -> GpuPolySprite {
    GpuPolySprite {
        bounds: [
            s.bounds.origin.x.0,
            s.bounds.origin.y.0,
            s.bounds.size.width.0,
            s.bounds.size.height.0,
        ],
        clip_bounds: [
            s.clip_bounds.origin.x.0,
            s.clip_bounds.origin.y.0,
            s.clip_bounds.size.width.0,
            s.clip_bounds.size.height.0,
        ],
        uv_min: [s.tile.uv[0], s.tile.uv[1]],
        uv_max: [s.tile.uv[2], s.tile.uv[3]],
        grayscale: s.grayscale as u32,
        _pad: [0; 3],
    }
}

fn subpixel_sprite_to_gpu(s: &SubpixelSprite) -> GpuSubpixelSprite {
    GpuSubpixelSprite {
        bounds: [
            s.bounds.origin.x.0,
            s.bounds.origin.y.0,
            s.bounds.size.width.0,
            s.bounds.size.height.0,
        ],
        clip_bounds: [
            s.clip_bounds.origin.x.0,
            s.clip_bounds.origin.y.0,
            s.clip_bounds.size.width.0,
            s.clip_bounds.size.height.0,
        ],
        color: rgba_to_array(s.color),
        uv_min: [s.tile.uv[0], s.tile.uv[1]],
        uv_max: [s.tile.uv[2], s.tile.uv[3]],
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use super::*;
    use crate::context::GpuContext;
    use crate::scene::{Quad, Shadow, Scene};
    use crate::geometry::{Bounds, Corners, Point, ScaledPixels, Size};
    use crate::color::LinearRgba;
    use crate::bounds_tree::DrawOrder;

    fn make_offscreen_target(device: &wgpu::Device) -> (wgpu::Texture, wgpu::TextureView) {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("nom_gpui_test_target"),
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = tex.create_view(&Default::default());
        (tex, view)
    }

    fn sp_bounds(x: f32, y: f32, w: f32, h: f32) -> Bounds<ScaledPixels> {
        Bounds {
            origin: Point { x: ScaledPixels(x), y: ScaledPixels(y) },
            size: Size { width: ScaledPixels(w), height: ScaledPixels(h) },
        }
    }

    #[test]
    fn renderer_constructs() {
        if crate::should_skip_gpu_tests() {
            eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
            return;
        }
        let Ok(ctx) = pollster::block_on(GpuContext::new()) else {
            eprintln!("SKIP: no GPU adapter");
            return;
        };
        let _r = Renderer::new(&ctx, wgpu::TextureFormat::Bgra8Unorm);
    }

    #[test]
    fn empty_scene_renders_without_panic() {
        if crate::should_skip_gpu_tests() {
            eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
            return;
        }
        let Ok(ctx) = pollster::block_on(GpuContext::new()) else {
            eprintln!("SKIP: no GPU adapter");
            return;
        };
        let mut renderer = Renderer::new(&ctx, wgpu::TextureFormat::Bgra8Unorm);
        let atlas = GpuAtlas::new(
            Arc::clone(&ctx.device),
            Arc::clone(&ctx.queue),
        );
        let mut scene = Scene::new();
        scene.finish();
        let (_tex, view) = make_offscreen_target(&ctx.device);
        let mut encoder = ctx.device.create_command_encoder(&Default::default());
        renderer.draw(&ctx, &atlas, &scene, &view, (64, 64), &mut encoder);
        ctx.queue.submit([encoder.finish()]);
        ctx.device.poll(wgpu::Maintain::Wait);
    }

    #[test]
    fn single_quad_renders() {
        if crate::should_skip_gpu_tests() {
            eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
            return;
        }
        let Ok(ctx) = pollster::block_on(GpuContext::new()) else {
            eprintln!("SKIP: no GPU adapter");
            return;
        };
        let mut renderer = Renderer::new(&ctx, wgpu::TextureFormat::Bgra8Unorm);
        let atlas = GpuAtlas::new(
            Arc::clone(&ctx.device),
            Arc::clone(&ctx.queue),
        );
        let mut scene = Scene::new();
        scene.insert_quad(Quad {
            order: 0 as DrawOrder,
            bounds: sp_bounds(0.0, 0.0, 32.0, 32.0),
            clip_bounds: sp_bounds(0.0, 0.0, 64.0, 64.0),
            corner_radii: Corners::all(ScaledPixels(0.0)),
            background: LinearRgba::new(1.0, 0.0, 0.0, 1.0),
            border_color: LinearRgba::TRANSPARENT,
            border_widths: [ScaledPixels(0.0); 4],
        });
        scene.finish();
        let (_tex, view) = make_offscreen_target(&ctx.device);
        let mut encoder = ctx.device.create_command_encoder(&Default::default());
        renderer.draw(&ctx, &atlas, &scene, &view, (64, 64), &mut encoder);
        ctx.queue.submit([encoder.finish()]);
        ctx.device.poll(wgpu::Maintain::Wait);
    }

    #[test]
    fn gpu_quad_size_is_96_bytes() {
        assert_eq!(std::mem::size_of::<GpuQuad>(), 96);
    }

    #[test]
    fn gpu_mono_sprite_size_is_64_bytes() {
        assert_eq!(std::mem::size_of::<GpuMonoSprite>(), 64);
    }

    #[test]
    fn gpu_underline_size_is_64_bytes() {
        assert_eq!(std::mem::size_of::<GpuUnderline>(), 64);
    }

    #[test]
    fn gpu_shadow_size_matches_wgsl() {
        // ShadowInstance in shadow.wgsl: bounds(16)+clip_bounds(16)+corner_radii(16)
        // +color(16)+blur_radius(4)+implicit_align_gap(12)+_pad vec3(12)+struct_pad(4)
        // = 96 bytes. vec3<f32> AlignOf=16 pushes _pad to offset 80; struct end at 92
        // rounded to 96.
        assert_eq!(std::mem::size_of::<GpuShadow>(), 96);
    }

    #[test]
    fn gpu_poly_sprite_size_matches_wgsl() {
        // PolySpriteInstance: bounds(16) + clip_bounds(16) + uv_min(8) +
        // uv_max(8) + grayscale u32(4) + _pad vec3<u32>(12) = 64 bytes.
        assert_eq!(std::mem::size_of::<GpuPolySprite>(), 64);
    }

    #[test]
    fn gpu_subpixel_sprite_size_matches_wgsl() {
        // SubpixelSpriteInstance: bounds(16) + clip_bounds(16) + color(16) +
        // uv_min(8) + uv_max(8) = 64 bytes.
        assert_eq!(std::mem::size_of::<GpuSubpixelSprite>(), 64);
    }

    #[test]
    fn scene_with_shadow_quad_sprite_renders_without_panic() {
        if crate::should_skip_gpu_tests() {
            eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
            return;
        }
        let Ok(ctx) = pollster::block_on(GpuContext::new()) else {
            eprintln!("SKIP: no GPU adapter");
            return;
        };
        let mut renderer = Renderer::new(&ctx, wgpu::TextureFormat::Bgra8Unorm);
        let atlas = GpuAtlas::new(
            Arc::clone(&ctx.device),
            Arc::clone(&ctx.queue),
        );
        let mut scene = Scene::new();

        // Insert a shadow behind a quad.
        scene.insert_shadow(Shadow {
            order: 0 as DrawOrder,
            bounds: sp_bounds(2.0, 2.0, 30.0, 30.0),
            clip_bounds: sp_bounds(0.0, 0.0, 64.0, 64.0),
            corner_radii: Corners::all(ScaledPixels(4.0)),
            color: LinearRgba::new(0.0, 0.0, 0.0, 0.5),
            blur_radius: ScaledPixels(8.0),
        });
        scene.insert_quad(Quad {
            order: 1 as DrawOrder,
            bounds: sp_bounds(0.0, 0.0, 32.0, 32.0),
            clip_bounds: sp_bounds(0.0, 0.0, 64.0, 64.0),
            corner_radii: Corners::all(ScaledPixels(4.0)),
            background: LinearRgba::new(0.2, 0.4, 0.8, 1.0),
            border_color: LinearRgba::TRANSPARENT,
            border_widths: [ScaledPixels(0.0); 4],
        });
        scene.finish();

        let (_tex, view) = make_offscreen_target(&ctx.device);
        let mut encoder = ctx.device.create_command_encoder(&Default::default());
        renderer.draw(&ctx, &atlas, &scene, &view, (64, 64), &mut encoder);
        ctx.queue.submit([encoder.finish()]);
        ctx.device.poll(wgpu::Maintain::Wait);
    }
}
