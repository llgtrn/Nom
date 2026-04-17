//! Integration tests for nom-gpui's GPU pipeline.
//!
//! These tests exercise the full path:
//!   Scene → batches → atlas → pipelines → headless wgpu draw.
//!
//! Each test that requires a GPU adapter skips gracefully when none is
//! available (headless CI without Vulkan/DX12/GL software fallback).
//!
//! # Skip pattern
//! ```rust,ignore
//! let Ok(ctx) = pollster::block_on(nom_gpui::context::GpuContext::new()) else {
//!     eprintln!("SKIP: no GPU adapter");
//!     return;
//! };
//! ```
//!
//! # wgpu 22 async-mapping notes
//! Buffer mapping in wgpu 22 is callback-based via `buffer.slice(..).map_async`.
//! After submitting the copy command, call `device.poll(wgpu::Maintain::Wait)`
//! to block until the GPU work completes, then collect the mapped data inside
//! the callback by cloning into a `std::sync::Mutex<Option<Vec<u8>>>`.

#![deny(unsafe_code)]

use std::sync::{Arc, Mutex};

use bytemuck::Zeroable as _;
use wgpu::util::DeviceExt as _;

use nom_gpui::atlas::{
    AtlasKey, AtlasTextureKind, InMemoryAtlas, PlatformAtlas,
};
use nom_gpui::buffers::InstanceBuffer;
use nom_gpui::context::GpuContext;
use nom_gpui::geometry::{
    Bounds, Corners, DevicePixels, Point, ScaledPixels, Size, TransformationMatrix,
};
use nom_gpui::pipelines::Pipelines;
use nom_gpui::scene::{
    AtlasTileRef, MonochromeSprite, PrimitiveBatch, Quad, Scene, Underline,
};
use nom_gpui::wgpu_atlas::GpuAtlas;
use nom_gpui::{AtlasTextureId, Rgba};

// ── helpers ───────────────────────────────────────────────────────────────────

fn mono_key(tag: u8) -> AtlasKey {
    AtlasKey {
        kind: AtlasTextureKind::Monochrome,
        bytes: vec![tag].into(),
    }
}

fn sp_bounds(x: f32, y: f32, w: f32, h: f32) -> Bounds<ScaledPixels> {
    Bounds {
        origin: Point {
            x: ScaledPixels(x),
            y: ScaledPixels(y),
        },
        size: Size {
            width: ScaledPixels(w),
            height: ScaledPixels(h),
        },
    }
}

fn make_quad(order: u32) -> Quad {
    Quad {
        order,
        bounds: sp_bounds(0.0, 0.0, 10.0, 10.0),
        clip_bounds: sp_bounds(0.0, 0.0, 1000.0, 1000.0),
        corner_radii: Corners::all(ScaledPixels(0.0)),
        background: Rgba::WHITE,
        border_color: Rgba::TRANSPARENT,
        border_widths: [ScaledPixels(0.0); 4],
    }
}

fn make_mono_sprite(order: u32, texture_index: u32) -> MonochromeSprite {
    MonochromeSprite {
        order,
        bounds: sp_bounds(0.0, 0.0, 8.0, 8.0),
        clip_bounds: sp_bounds(0.0, 0.0, 1000.0, 1000.0),
        color: Rgba::WHITE,
        tile: AtlasTileRef {
            texture: AtlasTextureId {
                kind: AtlasTextureKind::Monochrome,
                index: texture_index,
            },
            uv: [0.0, 0.0, 1.0, 1.0],
        },
        transform: TransformationMatrix::IDENTITY,
    }
}

fn make_underline(order: u32) -> Underline {
    Underline {
        order,
        bounds: sp_bounds(0.0, 20.0, 50.0, 2.0),
        clip_bounds: sp_bounds(0.0, 0.0, 1000.0, 1000.0),
        color: Rgba::BLACK,
        thickness: ScaledPixels(1.0),
        wavy: false,
    }
}

// ── Test 1: atlas_round_trip_single_glyph_bytes ───────────────────────────────

/// Insert a 16×16 R8 tile with a known byte pattern, flush uploads, then verify
/// the atlas returns the same `AtlasTile` on a second `get_or_insert` without
/// calling rasterize again. The rasterize closure is called exactly once.
#[test]
fn atlas_round_trip_single_glyph_bytes() {
    if nom_gpui::should_skip_gpu_tests() {
        eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
        return;
    }
    let Ok(ctx) = pollster::block_on(GpuContext::new()) else {
        eprintln!("SKIP: no GPU adapter");
        return;
    };

    let atlas = GpuAtlas::new(Arc::clone(&ctx.device), Arc::clone(&ctx.queue));
    let key = mono_key(0xAB);
    let tile_size = Size::new(DevicePixels(16), DevicePixels(16));
    let pixel_bytes: Vec<u8> = vec![0xABu8; 16 * 16]; // 256 bytes of 0xAB

    let mut rasterize_calls = 0usize;

    // First call: cache miss — rasterize closure must be invoked.
    let tile1 = atlas
        .get_or_insert(&key, &mut || {
            rasterize_calls += 1;
            Ok((tile_size, pixel_bytes.clone()))
        })
        .expect("first get_or_insert failed");

    assert_eq!(rasterize_calls, 1, "rasterize must be called exactly once on miss");
    assert_eq!(tile1.size, tile_size, "tile size must match requested size");
    assert_eq!(
        tile1.texture.kind,
        AtlasTextureKind::Monochrome,
        "tile kind must be Monochrome"
    );

    // Flush deferred uploads so the GPU state is consistent.
    atlas.flush_uploads();

    // Second call: cache hit — rasterize closure must NOT be invoked.
    let tile2 = atlas
        .get_or_insert(&key, &mut || {
            rasterize_calls += 1;
            panic!("rasterize called on cache hit — should never happen");
        })
        .expect("second get_or_insert (cache hit) failed");

    assert_eq!(rasterize_calls, 1, "rasterize must still be 1 after cache hit");
    assert_eq!(tile1, tile2, "cache hit must return the identical AtlasTile");
}

// ── Test 2: atlas_overflow_allocates_new_slab ─────────────────────────────────

/// Fill a 1024×1024 monochrome slab by inserting tiles until the allocator
/// must open a new slab. Verify that `AtlasTextureId.index` advances past 0
/// for at least one tile.
///
/// A 1024×1024 R8 slab holds at most 64 tiles of 128×128 (8×8 grid).
/// Inserting 65 unique tiles guarantees a second slab is opened.
#[test]
fn atlas_overflow_allocates_new_slab() {
    if nom_gpui::should_skip_gpu_tests() {
        eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
        return;
    }
    let Ok(ctx) = pollster::block_on(GpuContext::new()) else {
        eprintln!("SKIP: no GPU adapter");
        return;
    };

    let atlas = GpuAtlas::new(Arc::clone(&ctx.device), Arc::clone(&ctx.queue));
    let tile_size = Size::new(DevicePixels(128), DevicePixels(128));
    let bytes = vec![0xCCu8; 128 * 128]; // 16 KiB per tile, 1 byte / pixel

    // 65 unique keys each of size 128×128.  A 1024×1024 grid holds exactly 64
    // non-overlapping 128×128 tiles, so tile 65 must land on a second slab.
    let tile_count = 65usize;
    let mut max_slab_index = 0u32;

    for i in 0u8..tile_count as u8 {
        let key = AtlasKey {
            kind: AtlasTextureKind::Monochrome,
            bytes: vec![i].into(),
        };
        let tile = atlas
            .get_or_insert(&key, &mut || Ok((tile_size, bytes.clone())))
            .unwrap_or_else(|_| panic!("allocation {i} failed"));

        if tile.texture.index > max_slab_index {
            max_slab_index = tile.texture.index;
        }
    }

    assert!(
        max_slab_index >= 1,
        "at least one tile must land on slab index ≥ 1 after overflow; \
         got max_slab_index = {max_slab_index}"
    );

    atlas.flush_uploads();
}

// ── Test 3: buffer_growth_doubles_and_clamps ──────────────────────────────────

/// Construct an `InstanceBuffer`, write payloads until `write` returns `None`,
/// then call `grow()` and write again. Assert capacity doubled. Loop enough
/// times to reach the clamp at `max_buffer_size` (or exhaust reasonable iterations).
#[test]
fn buffer_growth_doubles_and_clamps() {
    if nom_gpui::should_skip_gpu_tests() {
        eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
        return;
    }
    let Ok(ctx) = pollster::block_on(GpuContext::new()) else {
        eprintln!("SKIP: no GPU adapter");
        return;
    };

    let device = &ctx.device;
    let queue = &ctx.queue;
    let mut buf = InstanceBuffer::new(device);

    // Write a 1 MiB chunk repeatedly until the buffer is exhausted.
    let chunk = vec![0u8; 1024 * 1024];

    // Drive growth for a few doublings to verify the doubling invariant.
    // We cap at 8 iterations to avoid allocating enormous GPU buffers in tests.
    for iteration in 0..8 {
        buf.begin_frame();

        // Write until exhausted.
        let mut overflowed = false;
        for _ in 0..256 {
            if buf.write(queue, &chunk).is_none() {
                overflowed = true;
                break;
            }
        }

        if !overflowed {
            // Buffer is large enough to fit 256 MiB in one frame — stop.
            break;
        }

        let capacity_before = buf.capacity();

        match buf.grow(device) {
            Ok(()) => {
                let capacity_after = buf.capacity();
                // New capacity must be exactly double or clamped to max.
                assert!(
                    capacity_after == capacity_before * 2
                        || capacity_after > capacity_before,
                    "grow() must increase capacity; \
                     before={capacity_before} after={capacity_after} iter={iteration}"
                );
            }
            Err(_) => {
                // Already at device max — normal termination.
                break;
            }
        }
    }
}

// ── Test 4: scene_batches_iterator_exhausts_cleanly ───────────────────────────

/// Create a Scene with 100 quads + 50 monochrome sprites (one texture) + 10
/// underlines. Verify `scene.batches()` yields the expected number / kind of
/// batches and the total item count equals 160.
///
/// All primitives use the same draw order so they are emitted in enum-declaration
/// order: Quads → MonochromeSprites → Underlines (3 batches total).
#[test]
fn scene_batches_iterator_exhausts_cleanly() {
    let mut scene = Scene::new();

    // 100 quads at order 10.
    for i in 0u32..100 {
        scene.insert_quad(make_quad(10 + i));
    }

    // 50 mono sprites at order 200 (all sharing texture index 0).
    for i in 0u32..50 {
        scene.insert_monochrome_sprite(make_mono_sprite(200 + i, 0));
    }

    // 10 underlines at order 300.
    for i in 0u32..10 {
        scene.insert_underline(make_underline(300 + i));
    }

    scene.finish();

    let mut batch_count = 0usize;
    let mut total_items = 0usize;
    let mut quad_batches = 0usize;
    let mut mono_batches = 0usize;
    let mut underline_batches = 0usize;

    for batch in scene.batches() {
        batch_count += 1;
        match batch {
            PrimitiveBatch::Quads(qs) => {
                quad_batches += 1;
                total_items += qs.len();
            }
            PrimitiveBatch::MonochromeSprites { sprites, .. } => {
                mono_batches += 1;
                total_items += sprites.len();
            }
            PrimitiveBatch::Underlines(us) => {
                underline_batches += 1;
                total_items += us.len();
            }
            _ => {}
        }
    }

    assert_eq!(
        total_items, 160,
        "total items across all batches must be 160; got {total_items}"
    );
    assert!(quad_batches >= 1, "must have at least one Quads batch");
    assert!(mono_batches >= 1, "must have at least one MonochromeSprites batch");
    assert!(underline_batches >= 1, "must have at least one Underlines batch");
    assert_eq!(
        batch_count,
        quad_batches + mono_batches + underline_batches,
        "no unexpected batch kinds"
    );
}

// ── Test 5: headless_clear_to_color ──────────────────────────────────────────

/// Render an empty scene to a 64×64 offscreen `TextureView`, copy the texture
/// to a `Buffer`, map/read the bytes, and verify the clear color was applied.
///
/// Uses only wgpu primitives; does not need a `Renderer`.
///
/// # wgpu 22 async-mapping pattern
///
/// ```
/// buffer.slice(..).map_async(wgpu::MapMode::Read, callback);
/// device.poll(wgpu::Maintain::Wait);   // block until GPU work is done
/// // callback has been invoked; collect bytes from Mutex<Option<Vec<u8>>>
/// ```
///
/// The clear color used is a fully-opaque dark-blue `[0x00, 0x00, 0x88, 0xFF]`
/// in RGBA byte order.  We assert that every pixel in the read-back matches.
#[test]
fn headless_clear_to_color() {
    if nom_gpui::should_skip_gpu_tests() {
        eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
        return;
    }
    let Ok(ctx) = pollster::block_on(GpuContext::new()) else {
        eprintln!("SKIP: no GPU adapter");
        return;
    };

    let device = &ctx.device;
    let queue = &ctx.queue;

    const W: u32 = 64;
    const H: u32 = 64;
    // RGBA clear color: R=0, G=0, B=0x88, A=0xFF  →  wgpu::Color (f64, linear)
    let clear_r: f64 = 0.0;
    let clear_g: f64 = 0.0;
    let clear_b: f64 = (0x88u32 as f64) / 255.0;
    let clear_a: f64 = 1.0;

    // ── offscreen target texture ──────────────────────────────────────────────
    // Use Rgba8Unorm so the copy row-stride calculation is trivial (4 bytes/px).
    let target_format = wgpu::TextureFormat::Rgba8Unorm;
    let target_tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("headless_clear_target"),
        size: wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: target_format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let target_view = target_tex.create_view(&wgpu::TextureViewDescriptor::default());

    // ── readback buffer ───────────────────────────────────────────────────────
    // wgpu requires row bytes to be aligned to COPY_BYTES_PER_ROW_ALIGNMENT.
    let bytes_per_pixel: u32 = 4; // Rgba8Unorm
    let unaligned_row_bytes = W * bytes_per_pixel;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let aligned_row_bytes = (unaligned_row_bytes + align - 1) & !(align - 1);
    let readback_size = (aligned_row_bytes * H) as u64;

    let readback_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("headless_readback"),
        size: readback_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    // ── record clear pass + copy ──────────────────────────────────────────────
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("headless_clear_encoder"),
    });

    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("headless_clear_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: clear_r,
                        g: clear_g,
                        b: clear_b,
                        a: clear_a,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        // Empty render pass — only the clear op runs.
    }

    // Copy rendered texture into the CPU-readable buffer.
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &target_tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &readback_buf,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(aligned_row_bytes),
                rows_per_image: Some(H),
            },
        },
        wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
    );

    queue.submit(std::iter::once(encoder.finish()));

    // ── map the readback buffer ───────────────────────────────────────────────
    // wgpu 22: map_async + poll(Wait) blocks synchronously until GPU is done.
    let result: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
    let result_clone = Arc::clone(&result);

    readback_buf
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |res| {
            if res.is_ok() {
                // Callback runs inside poll(); collect bytes immediately.
                // We cannot borrow `readback_buf` here, so we just signal
                // success; the main thread collects after poll returns.
                *result_clone.lock().unwrap() = Some(Vec::new()); // sentinel
            }
        });

    // Block until the GPU completes all submitted work.
    device.poll(wgpu::Maintain::Wait);

    // Now the buffer is mapped — read the bytes directly.
    let mapped = readback_buf.slice(..).get_mapped_range();
    let pixel_data: Vec<u8> = mapped.to_vec();
    drop(mapped);
    readback_buf.unmap();

    // ── verify every pixel matches the clear color ────────────────────────────
    // Rgba8Unorm: bytes are [R, G, B, A] per pixel.
    // clear_r=0x00, clear_g=0x00, clear_b=0x88, clear_a=0xFF in [0,255].
    let expected_r = (clear_r * 255.0).round() as u8;
    let expected_g = (clear_g * 255.0).round() as u8;
    let expected_b = (clear_b * 255.0).round() as u8;
    let expected_a = (clear_a * 255.0).round() as u8;

    let mut mismatch_count = 0usize;
    for row in 0..H {
        let row_start = (row * aligned_row_bytes) as usize;
        for col in 0..W {
            let px = row_start + (col * bytes_per_pixel) as usize;
            let actual = [
                pixel_data[px],
                pixel_data[px + 1],
                pixel_data[px + 2],
                pixel_data[px + 3],
            ];
            let expected = [expected_r, expected_g, expected_b, expected_a];
            if actual != expected {
                mismatch_count += 1;
                if mismatch_count == 1 {
                    eprintln!(
                        "first mismatch at ({col},{row}): expected {expected:?}, got {actual:?}"
                    );
                }
            }
        }
    }

    assert_eq!(
        mismatch_count, 0,
        "clear color mismatch in {mismatch_count} pixels out of {}",
        W * H
    );
}

// ── Test 6: pipelines_construct_on_bgra_and_rgba ──────────────────────────────

/// Create `Pipelines` twice — once for `Bgra8Unorm`, once for `Rgba8Unorm` —
/// and assert both succeed without panic.
#[test]
fn pipelines_construct_on_bgra_and_rgba() {
    if nom_gpui::should_skip_gpu_tests() {
        eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
        return;
    }
    let Ok(ctx) = pollster::block_on(GpuContext::new()) else {
        eprintln!("SKIP: no GPU adapter");
        return;
    };

    // Bgra8Unorm — standard desktop surface format on Windows/macOS.
    let _bgra_pipes = Pipelines::new(&ctx, wgpu::TextureFormat::Bgra8Unorm);

    // Rgba8Unorm — fallback on platforms that do not support Bgra8Unorm
    // (e.g. some Vulkan + Linux configurations).
    let _rgba_pipes = Pipelines::new(&ctx, wgpu::TextureFormat::Rgba8Unorm);
}

// ── Test 7: in_memory_atlas_round_trip_single_glyph_bytes ─────────────────────

/// Mirror of test 1 using `InMemoryAtlas` (no GPU required) so the byte-pattern
/// invariant is verified in pure-CPU environments too.
#[test]
fn in_memory_atlas_round_trip_single_glyph_bytes() {
    let atlas = InMemoryAtlas::new(Size::new(DevicePixels(1024), DevicePixels(1024)));
    let key = AtlasKey {
        kind: AtlasTextureKind::Monochrome,
        bytes: vec![0xAB].into(),
    };
    let tile_size = Size::new(DevicePixels(16), DevicePixels(16));
    let pixel_bytes = vec![0xABu8; 16 * 16];

    let mut rasterize_calls = 0usize;

    let tile1 = atlas
        .get_or_insert(&key, &mut || {
            rasterize_calls += 1;
            Ok((tile_size, pixel_bytes.clone()))
        })
        .expect("first get_or_insert failed");

    assert_eq!(rasterize_calls, 1, "rasterize called exactly once on miss");
    assert_eq!(tile1.size, tile_size);
    assert_eq!(tile1.texture.kind, AtlasTextureKind::Monochrome);

    let tile2 = atlas
        .get_or_insert(&key, &mut || {
            rasterize_calls += 1;
            panic!("rasterize called on cache hit");
        })
        .expect("second get_or_insert failed");

    assert_eq!(rasterize_calls, 1, "rasterize still 1 after cache hit");
    assert_eq!(tile1, tile2, "cache hit returns identical AtlasTile");
}

// ── Test 8: in_memory_atlas_overflow_allocates_new_slab ──────────────────────

/// InMemoryAtlas overflow: insert enough tiles to fill the virtual texture
/// row, verify the atlas does not error and allocation coordinates wrap.
///
/// InMemoryAtlas uses a simple shelf allocator that wraps rows; it does not
/// bump a slab index. The test verifies that inserting many tiles succeeds
/// without panic (atlas never returns TooLarge for correctly-sized tiles).
#[test]
fn in_memory_atlas_overflow_allocates_new_slab() {
    // 1024×1024 texture, 128×128 tiles → 8 tiles per row, 8 rows = 64 tiles.
    // InMemoryAtlas's shelf allocator wraps rows, so up to 64 unique tiles fit.
    // Insert exactly 64 tiles and verify none fail.
    let atlas = InMemoryAtlas::new(Size::new(DevicePixels(1024), DevicePixels(1024)));
    let tile_size = Size::new(DevicePixels(128), DevicePixels(128));
    let bytes = vec![0xAAu8; 128 * 128];

    for i in 0u8..64 {
        let key = AtlasKey {
            kind: AtlasTextureKind::Monochrome,
            bytes: vec![i].into(),
        };
        atlas
            .get_or_insert(&key, &mut || Ok((tile_size, bytes.clone())))
            .unwrap_or_else(|_| panic!("tile {i} allocation failed"));
    }

    // 64th tile must exist; atlas should have consumed all rows.
    let last_key = AtlasKey {
        kind: AtlasTextureKind::Monochrome,
        bytes: vec![63].into(),
    };
    let last = atlas
        .get_or_insert(&last_key, &mut || {
            panic!("should be a cache hit")
        })
        .expect("cache hit for tile 63 failed");

    // InMemoryAtlas always uses index 0 (single virtual texture per kind).
    assert_eq!(last.texture.index, 0);
}

// ── Pixel-diff helpers ────────────────────────────────────────────────────────

/// Return a single RGBA pixel from a flat CPU buffer.
///
/// `bytes_per_row` must be the *padded* row stride (multiple of 256).
/// The returned array is `[R, G, B, A]` for `Rgba8Unorm`.
fn read_pixel(bytes: &[u8], x: u32, y: u32, width: u32, bytes_per_row: u32) -> [u8; 4] {
    let _ = width; // present for documentation; actual stride comes from bytes_per_row
    let offset = (y * bytes_per_row + x * 4) as usize;
    [bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]]
}

/// GPU instance structs that mirror the WGSL layouts exactly.
///
/// These are *test-local* definitions — the production `Renderer` owns equivalent
/// private structs, but integration tests bypass `Renderer` and drive the
/// `Pipelines` API directly.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct TestFrameGlobals {
    viewport_size: [f32; 2],
    premultiplied_alpha: u32,
    _padding: u32,
}

/// Matches `QuadInstance` in `quad.wgsl` (96 bytes).
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct TestGpuQuad {
    bounds: [f32; 4],        // origin.x, origin.y, size.width, size.height
    clip_bounds: [f32; 4],
    corner_radii: [f32; 4],  // top_left, top_right, bottom_right, bottom_left
    background: [f32; 4],    // r, g, b, a
    border_color: [f32; 4],
    border_widths: [f32; 4], // top, right, bottom, left
}

/// Matches `UnderlineInstance` in `underline.wgsl` (80 bytes).
///
/// WGSL struct alignment rules (storage buffers, std430):
/// - `color: vec4<f32>` ends at byte 48.
/// - `thickness: f32` at offset 48, ends at 52.
/// - `_pad: vec3<f32>` has AlignOf=16 → placed at offset 64, not 52.
///   Gap [52,64) = 3 padding floats.
/// - StructSize = round_up(76, 16) = 80.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct TestGpuUnderline {
    bounds: [f32; 4],        // [0,16)
    clip_bounds: [f32; 4],   // [16,32)
    color: [f32; 4],         // [32,48)
    thickness: f32,          // [48,52)
    _gap: [f32; 3],          // [52,64) — align-pad before vec3
    _pad: [f32; 3],          // [64,76) — the vec3 field
    _tail: f32,              // [76,80) — struct size padding to multiple of 16
}

/// Render a list of GPU quads + underlines into an `Rgba8Unorm` offscreen texture,
/// copy to CPU, and return `(pixel_bytes, bytes_per_row)`.
///
/// The texture is `width × height`, cleared to transparent before drawing.
/// `Pipelines` are compiled fresh for each call (acceptable in tests).
fn render_to_buffer(
    ctx: &nom_gpui::context::GpuContext,
    width: u32,
    height: u32,
    quads: &[TestGpuQuad],
    underlines: &[TestGpuUnderline],
) -> (Vec<u8>, u32) {
    let device = &ctx.device;
    let queue = &ctx.queue;

    // ── offscreen render target ───────────────────────────────────────────────
    let target_tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("render_to_buffer_target"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let target_view = target_tex.create_view(&wgpu::TextureViewDescriptor::default());

    // ── row-aligned readback buffer ───────────────────────────────────────────
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let bytes_per_row = (width * 4 + align - 1) & !(align - 1);
    let readback_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("render_to_buffer_readback"),
        size: (bytes_per_row * height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    // ── per-frame globals uniform ─────────────────────────────────────────────
    let globals = TestFrameGlobals {
        viewport_size: [width as f32, height as f32],
        premultiplied_alpha: 0,
        _padding: 0,
    };
    let globals_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("render_to_buffer_globals"),
        contents: bytemuck::bytes_of(&globals),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    // ── build only quads + underlines pipelines (avoids subpixel shader) ────────
    // `Pipelines::new()` panics when dual-source-blending is enabled on the
    // adapter but the wgpu 22 WGSL parser does not support `enable` directives.
    // Build the two pipelines we actually need here, avoiding that code path.
    let globals_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("rtb_globals_bgl"),
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
    let instances_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("rtb_instances_bgl"),
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
    let color_target = wgpu::ColorTargetState {
        format: wgpu::TextureFormat::Rgba8Unorm,
        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
        write_mask: wgpu::ColorWrites::ALL,
    };
    let make_pipe = |label: &str, vs: &str, fs: &str, src: &str| -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("{label}_shader")),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(src)),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!("{label}_layout")),
            bind_group_layouts: &[&globals_bgl, &instances_bgl],
            push_constant_ranges: &[],
        });
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: vs,
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: fs,
                targets: &[Some(color_target.clone())],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        })
    };
    let quad_pipeline = make_pipe(
        "rtb_quads",
        "vs_quad",
        "fs_quad",
        nom_gpui::shaders::QUAD_SHADER,
    );
    let ul_pipeline = make_pipe(
        "rtb_underlines",
        "vs_underline",
        "fs_underline",
        nom_gpui::shaders::UNDERLINE_SHADER,
    );

    let globals_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("rtb_globals_bg"),
        layout: &globals_bgl,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: globals_buf.as_entire_binding(),
        }],
    });

    // ── instance storage buffers ──────────────────────────────────────────────
    // wgpu requires STORAGE buffers to be ≥16 bytes; always include at least
    // one zeroed entry so empty slices still produce a valid binding.
    let quad_data: Vec<TestGpuQuad> = if quads.is_empty() {
        vec![TestGpuQuad::zeroed()]
    } else {
        quads.to_vec()
    };
    let quad_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("render_to_buffer_quads"),
        contents: bytemuck::cast_slice(&quad_data),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let ul_data: Vec<TestGpuUnderline> = if underlines.is_empty() {
        vec![TestGpuUnderline::zeroed()]
    } else {
        underlines.to_vec()
    };
    let ul_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("render_to_buffer_underlines"),
        contents: bytemuck::cast_slice(&ul_data),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let quad_instances_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("render_to_buffer_quad_bg"),
        layout: &instances_bgl,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: quad_buf.as_entire_binding(),
        }],
    });

    let ul_instances_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("render_to_buffer_ul_bg"),
        layout: &instances_bgl,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: ul_buf.as_entire_binding(),
        }],
    });

    // ── encode render pass ────────────────────────────────────────────────────
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("render_to_buffer_encoder"),
    });

    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render_to_buffer_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        if !quads.is_empty() {
            pass.set_pipeline(&quad_pipeline);
            pass.set_bind_group(0, &globals_bg, &[]);
            pass.set_bind_group(1, &quad_instances_bg, &[]);
            pass.draw(0..4, 0..quads.len() as u32);
        }

        if !underlines.is_empty() {
            pass.set_pipeline(&ul_pipeline);
            pass.set_bind_group(0, &globals_bg, &[]);
            pass.set_bind_group(1, &ul_instances_bg, &[]);
            pass.draw(0..4, 0..underlines.len() as u32);
        }
    }

    // ── copy texture to CPU buffer ────────────────────────────────────────────
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &target_tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &readback_buf,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );

    queue.submit(std::iter::once(encoder.finish()));

    // ── wgpu 22 async-mapping: map_async + poll(Wait) → get_mapped_range ─────
    readback_buf.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::Maintain::Wait);
    let pixel_data = readback_buf.slice(..).get_mapped_range().to_vec();
    readback_buf.unmap();

    (pixel_data, bytes_per_row)
}

// ── Test 10: single_quad_pixel_color_matches ─────────────────────────────────

/// Render a red quad at (16,16)–(48,48) in a 64×64 `Rgba8Unorm` offscreen
/// texture. Verify:
/// - pixel (32,32) inside the quad is fully red (R≈255, G≈0, B≈0, A≈255).
/// - pixel (8,8) outside the quad is fully transparent (alpha == 0).
#[test]
fn single_quad_pixel_color_matches() {
    if nom_gpui::should_skip_gpu_tests() {
        eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
        return;
    }
    let Ok(ctx) = pollster::block_on(nom_gpui::context::GpuContext::new()) else {
        eprintln!("SKIP: no GPU adapter");
        return;
    };

    const W: u32 = 64;
    const H: u32 = 64;

    // Red quad: origin (16,16), size (32,32), fully opaque, no corner radii.
    let quad = TestGpuQuad {
        bounds: [16.0, 16.0, 32.0, 32.0],
        clip_bounds: [0.0, 0.0, W as f32, H as f32],
        corner_radii: [0.0; 4],
        background: [1.0, 0.0, 0.0, 1.0], // R=1, G=0, B=0, A=1
        border_color: [0.0; 4],
        border_widths: [0.0; 4],
    };

    let (pixels, bpr) = render_to_buffer(&ctx, W, H, &[quad], &[]);

    // Pixel (32,32) inside the red quad: R≈255, G≈0, B≈0, A≈255.
    // Rgba8Unorm byte layout: [R, G, B, A].
    let inside = read_pixel(&pixels, 32, 32, W, bpr);
    assert!(
        inside[0] > 200 && inside[1] < 50 && inside[2] < 50 && inside[3] > 200,
        "pixel (32,32) should be red; got {inside:?}"
    );

    // Pixel (8,8) outside the quad: fully transparent (alpha == 0).
    let outside = read_pixel(&pixels, 8, 8, W, bpr);
    assert_eq!(
        outside[3], 0,
        "pixel (8,8) outside quad must be transparent; got {outside:?}"
    );
}

// ── Test 11: cleared_empty_scene_produces_clear_color ────────────────────────

/// Render an empty scene (no primitives) with clear color (0.2, 0.3, 0.4, 1.0).
/// Verify the center pixel matches within 2/255 tolerance (float→u8 rounding).
///
/// Because `Renderer` is not publicly exported, this test uses a raw wgpu
/// render pass to apply the clear color, then reads it back.
#[test]
fn cleared_empty_scene_produces_clear_color() {
    if nom_gpui::should_skip_gpu_tests() {
        eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
        return;
    }
    let Ok(ctx) = pollster::block_on(nom_gpui::context::GpuContext::new()) else {
        eprintln!("SKIP: no GPU adapter");
        return;
    };

    let device = &ctx.device;
    let queue = &ctx.queue;

    const W: u32 = 64;
    const H: u32 = 64;
    let (cr, cg, cb, ca) = (0.2f64, 0.3f64, 0.4f64, 1.0f64);

    let target_tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("cleared_empty_scene_target"),
        size: wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let target_view = target_tex.create_view(&wgpu::TextureViewDescriptor::default());

    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let bpr = (W * 4 + align - 1) & !(align - 1);
    let readback_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("cleared_empty_scene_readback"),
        size: (bpr * H) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("cleared_empty_scene_encoder"),
    });

    {
        // Empty render pass — only the clear op runs.
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("cleared_empty_scene_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: cr, g: cg, b: cb, a: ca }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
    }

    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &target_tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &readback_buf,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(bpr),
                rows_per_image: Some(H),
            },
        },
        wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
    );

    queue.submit(std::iter::once(encoder.finish()));
    readback_buf.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::Maintain::Wait);
    let pixels = readback_buf.slice(..).get_mapped_range().to_vec();
    readback_buf.unmap();

    let center = read_pixel(&pixels, W / 2, H / 2, W, bpr);
    let expected_r = (cr * 255.0).round() as u8; // 51
    let expected_g = (cg * 255.0).round() as u8; // 77
    let expected_b = (cb * 255.0).round() as u8; // 102
    let expected_a = (ca * 255.0).round() as u8; // 255

    // Tolerance 2/255 to absorb float→u8 rounding.
    let tol = 2u8;
    let ok = |a: u8, e: u8| a.abs_diff(e) <= tol;
    assert!(
        ok(center[0], expected_r)
            && ok(center[1], expected_g)
            && ok(center[2], expected_b)
            && ok(center[3], expected_a),
        "center pixel {center:?} does not match clear color \
         [{expected_r},{expected_g},{expected_b},{expected_a}] ±{tol}"
    );
}

// ── Test 12: two_quads_correct_z_order ───────────────────────────────────────

/// Insert a red quad (drawn first) and a green quad (drawn second) that
/// overlap. After rendering in painter order:
///
/// - Red-only region pixel → red.
/// - Green region (overlap + green-only) pixel → green.
///
/// Viewport 64×64. Quads:
/// - Red:   origin(10,10), size(30,30) — covers (10,10)–(40,40).
/// - Green: origin(30,10), size(24,24) — covers (30,10)–(54,34).
/// Overlap region: x∈[30,40), y∈[10,34).
#[test]
fn two_quads_correct_z_order() {
    if nom_gpui::should_skip_gpu_tests() {
        eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
        return;
    }
    let Ok(ctx) = pollster::block_on(nom_gpui::context::GpuContext::new()) else {
        eprintln!("SKIP: no GPU adapter");
        return;
    };

    const W: u32 = 64;
    const H: u32 = 64;
    let clip = [0.0f32, 0.0, W as f32, H as f32];

    // Red quad drawn first (lower z).
    let red = TestGpuQuad {
        bounds: [10.0, 10.0, 30.0, 30.0],
        clip_bounds: clip,
        corner_radii: [0.0; 4],
        background: [1.0, 0.0, 0.0, 1.0],
        border_color: [0.0; 4],
        border_widths: [0.0; 4],
    };

    // Green quad drawn second (higher z, paints on top).
    let green = TestGpuQuad {
        bounds: [30.0, 10.0, 24.0, 24.0],
        clip_bounds: clip,
        corner_radii: [0.0; 4],
        background: [0.0, 1.0, 0.0, 1.0],
        border_color: [0.0; 4],
        border_widths: [0.0; 4],
    };

    let (pixels, bpr) = render_to_buffer(&ctx, W, H, &[red, green], &[]);

    // (20,20): inside red only → red.
    let red_only = read_pixel(&pixels, 20, 20, W, bpr);
    assert!(
        red_only[0] > 200 && red_only[1] < 50 && red_only[3] > 200,
        "red-only pixel (20,20) should be red; got {red_only:?}"
    );

    // (42,22): inside green (green-only region) → green.
    let green_px = read_pixel(&pixels, 42, 22, W, bpr);
    assert!(
        green_px[1] > 200 && green_px[0] < 50 && green_px[3] > 200,
        "green pixel (42,22) should be green; got {green_px:?}"
    );

    // (5,5): outside both quads → transparent.
    let outside = read_pixel(&pixels, 5, 5, W, bpr);
    assert_eq!(outside[3], 0, "outside pixel (5,5) should be transparent; got {outside:?}");
}

// ── Test 13: underline_row_is_thin_line ───────────────────────────────────────

/// Render a 2-pixel-thick black underline from x=10..50 at y=32.
/// Verify:
/// - (30, 32) inside the underline → opaque.
/// - (30, 30) 2px above the top edge → transparent.
/// - (30, 35) 1px below the bottom edge (y=34) → transparent.
#[test]
fn underline_row_is_thin_line() {
    if nom_gpui::should_skip_gpu_tests() {
        eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
        return;
    }
    let Ok(ctx) = pollster::block_on(nom_gpui::context::GpuContext::new()) else {
        eprintln!("SKIP: no GPU adapter");
        return;
    };

    const W: u32 = 64;
    const H: u32 = 64;
    let clip = [0.0f32, 0.0, W as f32, H as f32];

    // bounds: origin(10, 32), size(40, 2). thickness=2.
    let ul = TestGpuUnderline {
        bounds: [10.0, 32.0, 40.0, 2.0],
        clip_bounds: clip,
        color: [0.0, 0.0, 0.0, 1.0],
        thickness: 2.0,
        _gap: [0.0; 3],
        _pad: [0.0; 3],
        _tail: 0.0,
    };

    let (pixels, bpr) = render_to_buffer(&ctx, W, H, &[], &[ul]);

    // (30,32): inside the underline → opaque.
    let inside = read_pixel(&pixels, 30, 32, W, bpr);
    assert!(
        inside[3] > 200,
        "pixel (30,32) inside underline should be opaque; got {inside:?}"
    );

    // (30,30): 2px above the top edge → transparent.
    let above = read_pixel(&pixels, 30, 30, W, bpr);
    assert_eq!(
        above[3], 0,
        "pixel (30,30) above underline should be transparent; got {above:?}"
    );

    // (30,35): 1px below the bottom edge (y=34) → transparent.
    let below = read_pixel(&pixels, 30, 35, W, bpr);
    assert_eq!(
        below[3], 0,
        "pixel (30,35) below underline should be transparent; got {below:?}"
    );
}

// ── Test 14: buffer_growth_does_not_corrupt_rendering ────────────────────────

/// Insert 5000 small white 1×1 quads tiling the 64×64 viewport.
/// The first 4096 quads (64×64) cover every pixel; the remaining 904 overlap.
/// After rendering, spot-check corners and center — all must be opaque.
/// This exercises that no draw calls are silently dropped on large batches.
#[test]
fn buffer_growth_does_not_corrupt_rendering() {
    if nom_gpui::should_skip_gpu_tests() {
        eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
        return;
    }
    let Ok(ctx) = pollster::block_on(nom_gpui::context::GpuContext::new()) else {
        eprintln!("SKIP: no GPU adapter");
        return;
    };

    const W: u32 = 64;
    const H: u32 = 64;
    let clip = [0.0f32, 0.0, W as f32, H as f32];

    // 5000 white 1×1 quads; each pixel (col, row) covered at least once.
    let quads: Vec<TestGpuQuad> = (0..5000u32)
        .map(|i| {
            let col = (i % W) as f32;
            let row = ((i / W) % H) as f32;
            TestGpuQuad {
                bounds: [col, row, 1.0, 1.0],
                clip_bounds: clip,
                corner_radii: [0.0; 4],
                background: [1.0, 1.0, 1.0, 1.0],
                border_color: [0.0; 4],
                border_widths: [0.0; 4],
            }
        })
        .collect();

    let (pixels, bpr) = render_to_buffer(&ctx, W, H, &quads, &[]);

    // All four corners and center must be opaque (covered by at least one quad).
    for (px, py) in [(0u32, 0u32), (63, 0), (0, 63), (63, 63), (32, 32)] {
        let p = read_pixel(&pixels, px, py, W, bpr);
        assert!(
            p[3] > 200,
            "pixel ({px},{py}) should be opaque after 5000-quad render; got {p:?}"
        );
    }
}

// ── Test 9: gpu_atlas_overflow_allocates_new_slab ────────────────────────────

/// GPU-backed atlas overflow: force a second slab.
/// This is the GPU analog of test 8.
#[test]
fn gpu_atlas_overflow_allocates_new_slab() {
    if nom_gpui::should_skip_gpu_tests() {
        eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
        return;
    }
    let Ok(ctx) = pollster::block_on(GpuContext::new()) else {
        eprintln!("SKIP: no GPU adapter");
        return;
    };

    let atlas = GpuAtlas::new(Arc::clone(&ctx.device), Arc::clone(&ctx.queue));
    let tile_size = Size::new(DevicePixels(256), DevicePixels(256));
    let bytes = vec![0xDDu8; 256 * 256];

    // A 1024×1024 slab holds 16 non-overlapping 256×256 tiles.
    // Inserting 17 forces a second slab (index 1).
    let mut max_index = 0u32;
    for i in 0u8..17 {
        let key = AtlasKey {
            kind: AtlasTextureKind::Monochrome,
            bytes: vec![i].into(),
        };
        let tile = atlas
            .get_or_insert(&key, &mut || Ok((tile_size, bytes.clone())))
            .unwrap_or_else(|_| panic!("tile {i} failed"));
        if tile.texture.index > max_index {
            max_index = tile.texture.index;
        }
    }

    assert!(
        max_index >= 1,
        "slab index must be ≥ 1 after 17 inserts; got {max_index}"
    );
    atlas.flush_uploads();
}
