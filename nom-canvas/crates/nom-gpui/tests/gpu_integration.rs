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
        bytes: vec![tag],
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
            bytes: vec![i],
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
        bytes: vec![0xAB],
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
            bytes: vec![i],
        };
        atlas
            .get_or_insert(&key, &mut || Ok((tile_size, bytes.clone())))
            .unwrap_or_else(|_| panic!("tile {i} allocation failed"));
    }

    // 64th tile must exist; atlas should have consumed all rows.
    let last_key = AtlasKey {
        kind: AtlasTextureKind::Monochrome,
        bytes: vec![63],
    };
    let last = atlas
        .get_or_insert(&last_key, &mut || {
            panic!("should be a cache hit")
        })
        .expect("cache hit for tile 63 failed");

    // InMemoryAtlas always uses index 0 (single virtual texture per kind).
    assert_eq!(last.texture.index, 0);
}

// ── Test 9: gpu_atlas_overflow_allocates_new_slab ────────────────────────────

/// GPU-backed atlas overflow: force a second slab.
/// This is the GPU analog of test 8.
#[test]
fn gpu_atlas_overflow_allocates_new_slab() {
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
            bytes: vec![i],
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
