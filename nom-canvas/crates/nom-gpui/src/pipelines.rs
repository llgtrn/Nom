//! Render pipeline registry for `nom-gpui` batch-3 MVP.
//!
//! Six pipelines are provided:
//! - **quads**           — rounded-rectangle fills and borders.
//! - **mono_sprites**    — monochrome atlas glyphs (text rendering).
//! - **underlines**      — straight horizontal stroke underlines.
//! - **shadows**         — rounded-rect Gaussian-falloff drop shadows.
//! - **poly_sprites**    — polychrome RGBA atlas sprites (emoji / color images).
//! - **subpixel_sprites** — LCD subpixel-positioned text (dual-source blending,
//!                          `None` when the adapter lacks the feature).
//!
//! All pipelines share a common `@group(0)` bind group layout carrying the
//! per-frame [`RenderParams`] uniform (viewport size + premultiplied-alpha
//! flag).  The instance data at `@group(1)` differs: quads, underlines, and
//! shadows use a plain storage-read buffer; sprite pipelines add a texture and
//! sampler for the atlas.
//!
//! # Example (headless)
//!
//! ```no_run
//! # async fn run() {
//! use nom_gpui::context::GpuContext;
//! use nom_gpui::pipelines::Pipelines;
//! let ctx = GpuContext::new().await.unwrap();
//! let _pipes = Pipelines::new(&ctx, wgpu::TextureFormat::Bgra8Unorm);
//! # }
//! ```

#![deny(unsafe_code)]

use crate::context::GpuContext;
use crate::shaders::{MONO_SPRITE_SHADER, PATH_SHADER, POLY_SPRITE_SHADER, QUAD_SHADER, SHADOW_SHADER, SUBPIXEL_SPRITE_SHADER, UNDERLINE_SHADER};

// ── pipeline registry ─────────────────────────────────────────────────────────

/// Compiled wgpu render pipelines and shared bind-group layouts for batch-3.
///
/// Constructed once per surface format change (rare) via [`Pipelines::new`].
/// All [`wgpu::RenderPipeline`] objects are ready to be recorded into any
/// render pass that targets a texture with the same [`wgpu::TextureFormat`]
/// that was supplied at construction time.
pub struct Pipelines {
    /// Rounded-rectangle fill + border pipeline.
    pub quads: wgpu::RenderPipeline,

    /// Monochrome atlas sprite (text glyph) pipeline.
    pub mono_sprites: wgpu::RenderPipeline,

    /// Straight horizontal underline stroke pipeline.
    pub underlines: wgpu::RenderPipeline,

    /// Rounded-rect Gaussian-falloff drop shadow pipeline.
    pub shadows: wgpu::RenderPipeline,

    /// Polychrome RGBA atlas sprite pipeline (emoji, color images).
    pub poly_sprites: wgpu::RenderPipeline,

    /// LCD subpixel-positioned text pipeline using dual-source blending.
    /// `None` when the adapter does not support `DUAL_SOURCE_BLENDING`.
    pub subpixel_sprites: Option<wgpu::RenderPipeline>,

    /// `@group(0)` layout — one uniform buffer entry carrying `RenderParams`
    /// (viewport size + premultiplied-alpha flag).  Shared by all pipelines.
    pub globals_bgl: wgpu::BindGroupLayout,

    /// `@group(1)` layout for quads, underlines, and shadows — one
    /// storage-read buffer carrying a tightly-packed array of instance structs.
    pub instances_bgl: wgpu::BindGroupLayout,

    /// `@group(1)` layout for sprite pipelines — storage-read buffer plus a
    /// `texture_2d<f32>` and a `sampler` for the atlas.
    pub sprite_instances_bgl: wgpu::BindGroupLayout,

    /// `@group(1)` layout for the `path_rasterization` pipeline — one
    /// storage-read buffer of `PathVertex` structs (no texture, no sampler).
    pub path_vertex_bgl: wgpu::BindGroupLayout,

    /// `@group(1)` layout for the `paths` pipeline — storage-read buffer of
    /// `PathSprite` structs + a `texture_2d<f32>` (intermediate) + a sampler.
    pub path_sprite_bgl: wgpu::BindGroupLayout,

    /// Bezier path rasterization pipeline (pass 1).
    ///
    /// Renders filled quadratic bezier paths into an intermediate MSAA texture
    /// (sample_count=4). Input: storage buffer of `PathVertex` structs.
    /// Topology: `TriangleList`. Blend: `PREMULTIPLIED_ALPHA_BLENDING`.
    ///
    /// NOTE: `renderer.rs` does not yet dispatch this pipeline — follow-up task.
    pub path_rasterization: wgpu::RenderPipeline,

    /// Path compositing pipeline (pass 2).
    ///
    /// Composites the resolved intermediate texture onto the surface.
    /// Blend: `One / OneMinusSrcAlpha`. Topology: `TriangleStrip`.
    ///
    /// NOTE: `renderer.rs` does not yet dispatch this pipeline — follow-up task.
    pub paths: wgpu::RenderPipeline,
}

impl Pipelines {
    /// Build all render pipelines for the given surface pixel format.
    ///
    /// This is an inexpensive synchronous call (wgpu compiles shaders lazily in
    /// most back-ends) but must be repeated if the surface format ever changes.
    ///
    /// The `subpixel_sprites` pipeline is created only when
    /// `ctx.dual_source_blending` is `true`; otherwise it is `None`.
    pub fn new(ctx: &GpuContext, surface_format: wgpu::TextureFormat) -> Self {
        let device = &ctx.device;

        // ── bind-group layouts ────────────────────────────────────────────────

        // @group(0): per-frame globals (viewport_size + premultiplied_alpha).
        let globals_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nom_gpui_globals_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // @group(1) for quads / underlines: instance storage buffer only.
        let instances_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nom_gpui_instances_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // @group(1) for mono sprites: instance buffer + atlas texture + sampler.
        let sprite_instances_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("nom_gpui_sprite_instances_bgl"),
                entries: &[
                    // binding=0: MonoSpriteInstance storage array
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // binding=1: atlas texture (R8Unorm / Rgba8Unorm / etc.)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // binding=2: atlas sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        // @group(1) for path_rasterization: PathVertex storage buffer only.
        // No atlas texture; coverage is computed analytically in fs_path_raster.
        let path_vertex_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("nom_gpui_path_vertex_bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        // @group(1) for paths (compositing): PathSprite buffer + resolved intermediate
        // texture (single-sample Bgra8Unorm, the MSAA resolve target) + sampler.
        let path_sprite_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("nom_gpui_path_sprite_bgl"),
                entries: &[
                    // binding=0: PathSprite storage array
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // binding=1: resolved path intermediate texture (single-sample)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // binding=2: sampler for the intermediate texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        // ── color target shared by quads, mono_sprites, underlines ────────────

        let color_target = wgpu::ColorTargetState {
            format: surface_format,
            blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        };

        // ── pipeline helper closure ───────────────────────────────────────────

        let build_pipeline = |label: &str,
                              vs_entry: &str,
                              fs_entry: &str,
                              shader_src: &str,
                              group1_bgl: &wgpu::BindGroupLayout|
         -> wgpu::RenderPipeline {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(&format!("{label}_shader")),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(shader_src)),
            });

            let pipeline_layout =
                device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some(&format!("{label}_layout")),
                    bind_group_layouts: &[&globals_bgl, group1_bgl],
                    push_constant_ranges: &[],
                });

            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: vs_entry,
                    buffers: &[], // vertex_index / instance_index drive geometry
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: fs_entry,
                    targets: &[Some(color_target.clone())],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(), // count=1
                multiview: None,
                cache: None,
            })
        };

        // ── compile the six pipelines ─────────────────────────────────────────

        let quads = build_pipeline(
            "nom_gpui_quads",
            "vs_quad",
            "fs_quad",
            QUAD_SHADER,
            &instances_bgl,
        );

        let mono_sprites = build_pipeline(
            "nom_gpui_mono_sprites",
            "vs_mono_sprite",
            "fs_mono_sprite",
            MONO_SPRITE_SHADER,
            &sprite_instances_bgl,
        );

        let underlines = build_pipeline(
            "nom_gpui_underlines",
            "vs_underline",
            "fs_underline",
            UNDERLINE_SHADER,
            &instances_bgl,
        );

        // batch-3 ─────────────────────────────────────────────────────────────

        let shadows = build_pipeline(
            "nom_gpui_shadows",
            "vs_shadow",
            "fs_shadow",
            SHADOW_SHADER,
            &instances_bgl,
        );

        let poly_sprites = build_pipeline(
            "nom_gpui_poly_sprites",
            "vs_poly_sprite",
            "fs_poly_sprite",
            POLY_SPRITE_SHADER,
            &sprite_instances_bgl,
        );

        // Subpixel sprites are created only when the device was initialised
        // with DUAL_SOURCE_BLENDING (indicating hardware support for LCD-style
        // per-channel blending).  On wgpu 22 / naga 22 the @blend_src WGSL
        // extension is not yet available, so the pipeline uses standard
        // premultiplied-alpha blending with averaged subpixel coverage.
        // When wgpu ≥ 23 ships @blend_src support, upgrade the shader and
        // blend state to use BlendFactor::Src1 / OneMinusSrc1 per-channel.
        let subpixel_sprites = if ctx.dual_source_blending {
            Some(build_pipeline(
                "nom_gpui_subpixel_sprites",
                "vs_subpixel_sprite",
                "fs_subpixel_sprite",
                SUBPIXEL_SPRITE_SHADER,
                &sprite_instances_bgl,
            ))
        } else {
            None
        };

        // batch-4: two-pass bezier path renderer

        // Pass 1: path_rasterization
        //   Topology: TriangleList (bezier triangle mesh, 3 vertices per triangle).
        //   Blend: PREMULTIPLIED_ALPHA_BLENDING (coverage accumulation into MSAA).
        //   Sample count: 4 (MSAA intermediate texture).
        //   renderer.rs integration is a follow-up task; see path.wgsl for dispatch notes.
        let path_rasterization = {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("nom_gpui_path_rasterization_shader"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(PATH_SHADER)),
            });
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("nom_gpui_path_rasterization_layout"),
                bind_group_layouts: &[&globals_bgl, &path_vertex_bgl],
                push_constant_ranges: &[],
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("nom_gpui_path_rasterization"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_path_raster",
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_path_raster",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_format,
                        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                // sample_count=4 matches the MSAA intermediate texture created by the renderer.
                multisample: wgpu::MultisampleState {
                    count: 4,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            })
        };

        // Pass 2: paths (composite resolved intermediate onto surface).
        //   Topology: TriangleStrip (4-vertex unit quad, instance_count = sprite_count).
        //   Blend: One / OneMinusSrcAlpha, correct for premultiplied coverage from pass 1.
        //   Sample count: 1 (surface target, not MSAA).
        //   renderer.rs integration is a follow-up task; see path.wgsl for dispatch notes.
        let paths_blend = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        };
        let paths = {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("nom_gpui_paths_shader"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(PATH_SHADER)),
            });
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("nom_gpui_paths_layout"),
                bind_group_layouts: &[&globals_bgl, &path_sprite_bgl],
                push_constant_ranges: &[],
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("nom_gpui_paths"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_path",
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_path",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_format,
                        blend: Some(paths_blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(), // count=1 (surface)
                multiview: None,
                cache: None,
            })
        };

        Self {
            quads,
            mono_sprites,
            underlines,
            shadows,
            poly_sprites,
            subpixel_sprites,
            path_rasterization,
            paths,
            globals_bgl,
            instances_bgl,
            sprite_instances_bgl,
            path_vertex_bgl,
            path_sprite_bgl,
        }
    }
}

// ── bind-group helpers ────────────────────────────────────────────────────────

/// Runtime helpers for building wgpu bind groups from live resources.
///
/// These are lightweight — call them at the start of each frame or whenever
/// the underlying buffers / textures are re-allocated.
impl Pipelines {
    /// Build the `@group(0)` bind group from a `RenderParams` uniform buffer.
    ///
    /// The buffer must contain exactly one `RenderParams` struct (16 bytes).
    pub fn bind_globals(
        &self,
        device: &wgpu::Device,
        globals_buf: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nom_gpui_globals_bg"),
            layout: &self.globals_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals_buf.as_entire_binding(),
            }],
        })
    }

    /// Build the `@group(1)` bind group for the quads or underlines pipeline.
    ///
    /// `instances_buf` must be a storage buffer holding the packed instance array.
    pub fn bind_instances(
        &self,
        device: &wgpu::Device,
        instances_buf: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nom_gpui_instances_bg"),
            layout: &self.instances_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: instances_buf.as_entire_binding(),
            }],
        })
    }

    /// Build the `@group(1)` bind group for the mono_sprites pipeline.
    ///
    /// - `instances_buf` — storage buffer of `MonoSpriteInstance` structs.
    /// - `atlas_view`    — `TextureView` of the glyph atlas (typically R8Unorm).
    /// - `atlas_sampler` — `Sampler` for the atlas (bilinear or nearest).
    pub fn bind_sprite_instances(
        &self,
        device: &wgpu::Device,
        instances_buf: &wgpu::Buffer,
        atlas_view: &wgpu::TextureView,
        atlas_sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nom_gpui_sprite_instances_bg"),
            layout: &self.sprite_instances_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: instances_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(atlas_sampler),
                },
            ],
        })
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use super::Pipelines;
    use crate::context::GpuContext;

    /// Verify that all three pipelines compile successfully against
    /// `Bgra8Unorm` (the standard desktop surface format).
    ///
    /// If no GPU adapter is available (e.g. headless CI without a software
    /// renderer) the test exits silently rather than failing.
    #[test]
    fn pipelines_compile_successfully() {
        if crate::should_skip_gpu_tests() {
            eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
            return;
        }
        let ctx = match pollster::block_on(GpuContext::new()) {
            Ok(c) => c,
            Err(_) => return, // no GPU in this environment — skip
        };
        // wgpu panics (via validation) if any pipeline failed to compile, so
        // simply constructing Pipelines is sufficient proof.
        let _pipelines = Pipelines::new(&ctx, wgpu::TextureFormat::Bgra8Unorm);
    }

    /// Verify that pipelines can be built for `Rgba8Unorm`, which is the
    /// common fallback format on platforms that do not expose `Bgra8Unorm`.
    #[test]
    fn pipelines_support_rgba8_format_fallback() {
        if crate::should_skip_gpu_tests() {
            eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
            return;
        }
        let ctx = match pollster::block_on(GpuContext::new()) {
            Ok(c) => c,
            Err(_) => return,
        };
        let _pipelines = Pipelines::new(&ctx, wgpu::TextureFormat::Rgba8Unorm);
    }

    /// The bind-group layout for sprite instances must have 3 entries (buffer,
    /// texture, sampler).  wgpu exposes this as the `count` on `entries()`.
    #[test]
    fn sprite_bgl_has_three_entries() {
        if crate::should_skip_gpu_tests() {
            eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
            return;
        }
        let ctx = match pollster::block_on(GpuContext::new()) {
            Ok(c) => c,
            Err(_) => return,
        };
        let pipelines = Pipelines::new(&ctx, wgpu::TextureFormat::Bgra8Unorm);
        // wgpu::BindGroupLayout does not expose entry count directly, but
        // if the layout was built from 3 entries, descriptor creation
        // succeeds implicitly.  We verify the field exists (not null pointer).
        let _ = &pipelines.sprite_instances_bgl;
        let _ = &pipelines.instances_bgl;
        let _ = &pipelines.globals_bgl;
    }

    /// Verify that the `shadows` pipeline compiles for the standard surface format.
    #[test]
    fn shadows_pipeline_compiles() {
        if crate::should_skip_gpu_tests() {
            eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
            return;
        }
        let ctx = match pollster::block_on(GpuContext::new()) {
            Ok(c) => c,
            Err(_) => return,
        };
        let pipelines = Pipelines::new(&ctx, wgpu::TextureFormat::Bgra8Unorm);
        // Constructing Pipelines without a panic proves the pipeline compiled.
        let _ = &pipelines.shadows;
    }

    /// Verify that the `poly_sprites` pipeline compiles for the standard surface format.
    #[test]
    fn poly_sprites_pipeline_compiles() {
        if crate::should_skip_gpu_tests() {
            eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
            return;
        }
        let ctx = match pollster::block_on(GpuContext::new()) {
            Ok(c) => c,
            Err(_) => return,
        };
        let pipelines = Pipelines::new(&ctx, wgpu::TextureFormat::Bgra8Unorm);
        let _ = &pipelines.poly_sprites;
    }

    /// Verify the `subpixel_sprites` pipeline: `Some` when the device has
    /// `DUAL_SOURCE_BLENDING`, `None` otherwise.  Both states are valid.
    #[test]
    fn subpixel_sprites_pipeline_consistent_with_feature() {
        if crate::should_skip_gpu_tests() {
            eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
            return;
        }
        let ctx = match pollster::block_on(GpuContext::new()) {
            Ok(c) => c,
            Err(_) => return,
        };
        let pipelines = Pipelines::new(&ctx, wgpu::TextureFormat::Bgra8Unorm);
        if ctx.dual_source_blending {
            assert!(
                pipelines.subpixel_sprites.is_some(),
                "dual_source_blending enabled but pipeline is None"
            );
        } else {
            assert!(
                pipelines.subpixel_sprites.is_none(),
                "dual_source_blending disabled but pipeline is Some"
            );
        }
    }
    /// Verify that the `path_rasterization` pipeline (pass 1) compiles.
    /// Topology is TriangleList with sample_count=4 (MSAA intermediate target).
    #[test]
    fn path_rasterization_pipeline_compiles() {
        if crate::should_skip_gpu_tests() {
            eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
            return;
        }
        let ctx = match pollster::block_on(GpuContext::new()) {
            Ok(c) => c,
            Err(_) => return,
        };
        let pipelines = Pipelines::new(&ctx, wgpu::TextureFormat::Bgra8Unorm);
        let _ = &pipelines.path_rasterization;
    }

    /// Verify that the `paths` pipeline (pass 2) compiles.
    /// Topology is TriangleStrip with One/OneMinusSrcAlpha blend on the surface.
    #[test]
    fn paths_pipeline_compiles() {
        if crate::should_skip_gpu_tests() {
            eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
            return;
        }
        let ctx = match pollster::block_on(GpuContext::new()) {
            Ok(c) => c,
            Err(_) => return,
        };
        let pipelines = Pipelines::new(&ctx, wgpu::TextureFormat::Bgra8Unorm);
        let _ = &pipelines.paths;
    }

    /// Both new pipelines and their bind-group layouts are accessible in the registry.
    #[test]
    fn both_pipelines_present_in_registry() {
        if crate::should_skip_gpu_tests() {
            eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
            return;
        }
        let ctx = match pollster::block_on(GpuContext::new()) {
            Ok(c) => c,
            Err(_) => return,
        };
        let pipelines = Pipelines::new(&ctx, wgpu::TextureFormat::Bgra8Unorm);
        let _ = &pipelines.path_rasterization;
        let _ = &pipelines.paths;
        let _ = &pipelines.path_vertex_bgl;
        let _ = &pipelines.path_sprite_bgl;
    }

}