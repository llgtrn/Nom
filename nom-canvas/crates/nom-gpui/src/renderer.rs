//! Main draw loop: consumes a [`Scene`] and issues wgpu draw calls.
//!
//! [`Renderer`] owns the three instance buffers (one per batch-2 primitive kind),
//! the shared globals uniform buffer, and the shared atlas sampler. Each frame
//! the caller builds a [`wgpu::CommandEncoder`], calls [`Renderer::draw`], then
//! submits the encoder.
//!
//! # Batch-2 primitives (handled)
//! - [`PrimitiveBatch::Quads`] — rounded rectangles via `pipelines.quads`
//! - [`PrimitiveBatch::MonochromeSprites`] — atlas glyphs via `pipelines.mono_sprites`
//! - [`PrimitiveBatch::Underlines`] — horizontal strokes via `pipelines.underlines`
//!
//! # Batch-3 primitives (skipped / logged)
//! - `Shadows`, `SubpixelSprites`, `PolychromeSprites`, `Paths`

#![deny(unsafe_code)]

use std::sync::Arc;

use crate::context::GpuContext;
use crate::pipelines::Pipelines;
use crate::buffers::InstanceBuffer;
use crate::scene::{PrimitiveBatch, Quad, MonochromeSprite, Underline, Scene};
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

// ── Conversion helpers ────────────────────────────────────────────────────────

fn rgba_to_array(c: crate::color::Rgba) -> [f32; 4] {
    [c.r, c.g, c.b, c.a]
}

// ── Renderer ──────────────────────────────────────────────────────────────────

/// Records wgpu draw calls for a [`Scene`] into a caller-supplied encoder.
///
/// Owns three [`InstanceBuffer`]s (quad, sprite, underline) that are reset and
/// refilled every frame, plus one uniform buffer for [`FrameGlobals`] and one
/// shared [`wgpu::Sampler`] for the sprite pipeline.
pub struct Renderer {
    pipelines: Pipelines,
    quad_buffer: InstanceBuffer,
    sprite_buffer: InstanceBuffer,
    underline_buffer: InstanceBuffer,
    globals_buffer: wgpu::Buffer,
    sampler: wgpu::Sampler,
}

impl Renderer {
    /// Construct a new renderer for the given surface pixel format.
    ///
    /// Compiles the three render pipelines (quads, mono_sprites, underlines) and
    /// allocates initial 2 MiB instance buffers for each.
    pub fn new(ctx: &GpuContext, surface_format: wgpu::TextureFormat) -> Self {
        let device = &ctx.device;

        let pipelines = Pipelines::new(ctx, surface_format);

        let quad_buffer = InstanceBuffer::new(device);
        let sprite_buffer = InstanceBuffer::new(device);
        let underline_buffer = InstanceBuffer::new(device);

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
    /// 4. Open a render pass (clear to transparent black).
    /// 5. For each [`PrimitiveBatch`]:
    ///    - Quads / MonochromeSprites / Underlines → convert instances, write
    ///      buffer, build bind groups, set pipeline, draw.
    ///    - All other variants → skipped (batch-3, logged once at trace level).
    /// 6. End render pass. Caller submits the encoder.
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

        // Build the globals bind group (recreated each frame; cheap).
        let globals_bg = self.pipelines.bind_globals(device, &self.globals_buffer);

        // Step 4: pre-collect atlas texture views so they outlive the render pass
        // borrow. `TextureView` values must live longer than any render pass that
        // references them, so we gather them before opening the pass.
        let sprite_views: Vec<(crate::atlas::AtlasTextureId, wgpu::TextureView)> = scene
            .batches()
            .filter_map(|b| match b {
                PrimitiveBatch::MonochromeSprites { texture_id, .. } => {
                    Some((texture_id, atlas.texture_view(texture_id)))
                }
                _ => None,
            })
            .collect();
        let mut sprite_view_iter = sprite_views.iter();

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
                    PrimitiveBatch::MonochromeSprites { sprites, .. } => {
                        if let Some((_, atlas_view)) = sprite_view_iter.next() {
                            self.draw_mono_sprites(
                                device,
                                queue,
                                &globals_bg,
                                sprites,
                                atlas_view,
                                &mut pass,
                            );
                        }
                    }
                    PrimitiveBatch::Underlines(underlines) => {
                        self.draw_underlines(device, queue, &globals_bg, underlines, &mut pass);
                    }
                    // Batch-3 variants: not yet implemented.
                    PrimitiveBatch::Shadows(_) => {}
                    PrimitiveBatch::SubpixelSprites { .. } => {}
                    PrimitiveBatch::PolychromeSprites { .. } => {}
                    PrimitiveBatch::Paths(_) => {}
                }
            }
        }
        // Step 6: render pass dropped here; caller submits the encoder.
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use super::*;
    use crate::context::GpuContext;
    use crate::scene::{Quad, Scene};
    use crate::geometry::{Bounds, Corners, Point, ScaledPixels, Size};
    use crate::color::Rgba;
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
        let Ok(ctx) = pollster::block_on(GpuContext::new()) else {
            eprintln!("SKIP: no GPU adapter");
            return;
        };
        let _r = Renderer::new(&ctx, wgpu::TextureFormat::Bgra8Unorm);
    }

    #[test]
    fn empty_scene_renders_without_panic() {
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
            background: Rgba::new(1.0, 0.0, 0.0, 1.0),
            border_color: Rgba::TRANSPARENT,
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
}
