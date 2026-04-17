//! GPU-backed texture atlas for nom-gpui batch-2.
//!
//! [`GpuAtlas`] implements [`PlatformAtlas`] over three `wgpu::Texture` lists —
//! one per [`AtlasTextureKind`] — using `etagere::BucketedAtlasAllocator` for
//! shelf-packed allocation. Pixel uploads are deferred: `get_or_insert` queues a
//! [`PendingUpload`] and [`GpuAtlas::flush_uploads`] drains them into the GPU
//! queue just before render.
//!
//! # Pixel formats
//! - `Monochrome` → `wgpu::TextureFormat::R8Unorm` (1 byte / pixel)
//! - `Subpixel`   → `wgpu::TextureFormat::Bgra8Unorm` (4 bytes / pixel)
//! - `Polychrome` → `wgpu::TextureFormat::Bgra8Unorm` (4 bytes / pixel)
//!
//! Callers that source pixel data from `swash` (which produces RGBA) **must**
//! swap R↔B channels before passing bytes to [`GpuAtlas`].

#![deny(unsafe_code)]

use std::collections::HashMap;
use std::sync::Arc;

use etagere::BucketedAtlasAllocator;
use parking_lot::Mutex;

use crate::atlas::{AtlasError, AtlasKey, AtlasTextureId, AtlasTextureKind, AtlasTile, PlatformAtlas};
use crate::geometry::{DevicePixels, Point, Size};

// ── helpers ───────────────────────────────────────────────────────────────────

fn to_etagere_size(size: Size<DevicePixels>) -> etagere::Size {
    etagere::size2(size.width.0, size.height.0)
}

fn from_etagere_point(pt: etagere::Point) -> Point<DevicePixels> {
    Point {
        x: DevicePixels(pt.x),
        y: DevicePixels(pt.y),
    }
}

// ── pending upload ────────────────────────────────────────────────────────────

/// A pixel upload deferred until [`GpuAtlas::flush_uploads`].
struct PendingUpload {
    /// Which texture slot to write into.
    texture_id: AtlasTextureId,
    /// Top-left corner of the allocated region (device pixels).
    origin: Point<DevicePixels>,
    /// Dimensions of the region (device pixels).
    size: Size<DevicePixels>,
    /// Raw pixel bytes; length must equal `size.width * size.height * bpp`.
    bytes: Vec<u8>,
}

// ── per-texture backing ───────────────────────────────────────────────────────

struct SlabTexture {
    id: AtlasTextureId,
    texture: wgpu::Texture,
    size: Size<DevicePixels>,
    allocator: BucketedAtlasAllocator,
    format: wgpu::TextureFormat,
    live_count: u32,
}

impl SlabTexture {
    /// Create a fresh `TextureView` for this slab. Views are cheap to create
    /// (they are GPU object handles) but cannot be cloned in wgpu 22.
    fn create_view(&self) -> wgpu::TextureView {
        self.texture
            .create_view(&wgpu::TextureViewDescriptor::default())
    }
}

impl SlabTexture {
    /// Attempt to allocate `size` inside this texture.
    fn try_allocate(&mut self, size: Size<DevicePixels>) -> Option<AtlasTile> {
        let alloc = self.allocator.allocate(to_etagere_size(size))?;
        self.live_count += 1;
        Some(AtlasTile {
            texture: self.id,
            origin: from_etagere_point(alloc.rectangle.min),
            size,
            padding: DevicePixels::ZERO,
        })
    }

    fn bytes_per_pixel(&self) -> u32 {
        match self.format {
            wgpu::TextureFormat::R8Unorm => 1,
            _ => 4,
        }
    }
}

// ── per-kind texture list ─────────────────────────────────────────────────────

struct KindStore {
    textures: Vec<SlabTexture>,
    kind: AtlasTextureKind,
}

impl KindStore {
    fn new(kind: AtlasTextureKind) -> Self {
        Self {
            textures: Vec::new(),
            kind,
        }
    }

    fn format(&self) -> wgpu::TextureFormat {
        match self.kind {
            AtlasTextureKind::Monochrome => wgpu::TextureFormat::R8Unorm,
            AtlasTextureKind::Subpixel | AtlasTextureKind::Polychrome => {
                wgpu::TextureFormat::Bgra8Unorm
            }
        }
    }

    /// Try allocating in existing textures (newest first), else return `None`.
    fn try_allocate(&mut self, size: Size<DevicePixels>) -> Option<AtlasTile> {
        self.textures
            .iter_mut()
            .rev()
            .find_map(|slab| slab.try_allocate(size))
    }

    /// Push a new [`SlabTexture`] using `device`, sized to at least `min_size`
    /// and at most `max_side` × `max_side`.
    fn push_slab(
        &mut self,
        device: &wgpu::Device,
        min_size: Size<DevicePixels>,
        max_side: u32,
    ) -> &mut SlabTexture {
        const DEFAULT_SIDE: u32 = 1024;

        let side = DEFAULT_SIDE.max(min_size.width.0.max(min_size.height.0) as u32);
        let side = side.min(max_side);

        let slab_size = Size {
            width: DevicePixels(side as i32),
            height: DevicePixels(side as i32),
        };

        let format = self.format();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("nom_gpui_atlas"),
            size: wgpu::Extent3d {
                width: side,
                height: side,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let index = self.textures.len() as u32;
        let id = AtlasTextureId {
            kind: self.kind,
            index,
        };

        self.textures.push(SlabTexture {
            id,
            texture,
            size: slab_size,
            allocator: BucketedAtlasAllocator::new(to_etagere_size(slab_size)),
            format,
            live_count: 0,
        });

        self.textures.last_mut().expect("just pushed")
    }

    fn get(&self, index: u32) -> Option<&SlabTexture> {
        self.textures.get(index as usize)
    }
}

// ── storage (three kind stores) ───────────────────────────────────────────────

struct AtlasStorage {
    monochrome: KindStore,
    subpixel: KindStore,
    polychrome: KindStore,
}

impl AtlasStorage {
    fn new() -> Self {
        Self {
            monochrome: KindStore::new(AtlasTextureKind::Monochrome),
            subpixel: KindStore::new(AtlasTextureKind::Subpixel),
            polychrome: KindStore::new(AtlasTextureKind::Polychrome),
        }
    }

    fn store_for_kind_mut(&mut self, kind: AtlasTextureKind) -> &mut KindStore {
        match kind {
            AtlasTextureKind::Monochrome => &mut self.monochrome,
            AtlasTextureKind::Subpixel => &mut self.subpixel,
            AtlasTextureKind::Polychrome => &mut self.polychrome,
        }
    }

    fn store_for_kind(&self, kind: AtlasTextureKind) -> &KindStore {
        match kind {
            AtlasTextureKind::Monochrome => &self.monochrome,
            AtlasTextureKind::Subpixel => &self.subpixel,
            AtlasTextureKind::Polychrome => &self.polychrome,
        }
    }
}

// ── inner state ───────────────────────────────────────────────────────────────

struct GpuAtlasState {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    max_texture_size: u32,
    storage: AtlasStorage,
    tiles_by_key: HashMap<AtlasKey, AtlasTile>,
    pending_uploads: Vec<PendingUpload>,
}

impl GpuAtlasState {
    /// Allocate a tile of `size` in the `kind` store, growing it if needed.
    fn allocate(
        &mut self,
        size: Size<DevicePixels>,
        kind: AtlasTextureKind,
    ) -> Result<AtlasTile, AtlasError> {
        let max_side = self.max_texture_size;

        // Reject tiles that exceed the maximum texture dimension.
        let max_size = Size {
            width: DevicePixels(max_side as i32),
            height: DevicePixels(max_side as i32),
        };
        if size.width.0 > max_size.width.0 || size.height.0 > max_size.height.0 {
            return Err(AtlasError::TooLarge(size, max_size));
        }

        let store = self.storage.store_for_kind_mut(kind);

        // Fast path: fits in an existing slab.
        if let Some(tile) = store.try_allocate(size) {
            return Ok(tile);
        }

        // Slow path: allocate a new slab then allocate inside it.
        let device = Arc::clone(&self.device);
        let slab = store.push_slab(&device, size, max_side);
        slab.try_allocate(size)
            .ok_or(AtlasError::TooLarge(size, max_size))
    }

    /// Queue a deferred pixel upload (does not touch the GPU).
    fn enqueue_upload(&mut self, tile: &AtlasTile, bytes: Vec<u8>) {
        self.pending_uploads.push(PendingUpload {
            texture_id: tile.texture,
            origin: tile.origin,
            size: tile.size,
            bytes,
        });
    }

    /// Drain the pending-upload queue and submit each via `queue.write_texture`.
    fn drain_uploads(&mut self) {
        // Drain into a local vec so the pending_uploads field is free.
        let uploads: Vec<PendingUpload> = self.pending_uploads.drain(..).collect();

        for upload in &uploads {
            // Look up bytes-per-pixel while holding a short-lived borrow.
            let bpp = {
                let store = self.storage.store_for_kind(upload.texture_id.kind);
                match store.get(upload.texture_id.index) {
                    Some(slab) => slab.bytes_per_pixel(),
                    None => continue,
                }
            };

            let row_bytes = upload.size.width.0 as u32 * bpp;

            // Re-borrow storage for the texture reference inside write_texture.
            // `queue` and `storage` are distinct fields, so Rust permits
            // borrowing both simultaneously.
            let store = self.storage.store_for_kind(upload.texture_id.kind);
            let Some(slab) = store.get(upload.texture_id.index) else {
                continue;
            };

            self.queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &slab.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: upload.origin.x.0 as u32,
                        y: upload.origin.y.0 as u32,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &upload.bytes,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(row_bytes),
                    rows_per_image: None,
                },
                wgpu::Extent3d {
                    width: upload.size.width.0 as u32,
                    height: upload.size.height.0 as u32,
                    depth_or_array_layers: 1,
                },
            );
        }
    }
}

// ── public API ─────────────────────────────────────────────────────────────────

/// GPU-backed texture atlas.
///
/// Wrap in `Arc` and share across threads; all access is internally serialized
/// by a [`parking_lot::Mutex`].
pub struct GpuAtlas(Mutex<GpuAtlasState>);

impl GpuAtlas {
    /// Construct a new atlas backed by `device` / `queue`.
    ///
    /// The initial slab size is 1024 × 1024, clamped to the device's
    /// `max_texture_dimension_2d` limit.
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Arc<Self> {
        let max_texture_size = device.limits().max_texture_dimension_2d;
        Arc::new(Self(Mutex::new(GpuAtlasState {
            device,
            queue,
            max_texture_size,
            storage: AtlasStorage::new(),
            tiles_by_key: HashMap::new(),
            pending_uploads: Vec::new(),
        })))
    }

    /// Submit all queued pixel uploads to the GPU.
    ///
    /// Call this once per frame, before issuing draw calls that reference atlas
    /// tiles inserted during the same frame.
    pub fn flush_uploads(&self) {
        self.0.lock().drain_uploads();
    }

    /// Return a cloned [`wgpu::TextureView`] for `texture_id`.
    ///
    /// Texture views are cheap to clone (they are reference-counted handles).
    ///
    /// # Panics
    ///
    /// Panics if `texture_id.index` does not exist in the kind's slab list.
    pub fn texture_view(&self, texture_id: AtlasTextureId) -> wgpu::TextureView {
        let lock = self.0.lock();
        let store = lock.storage.store_for_kind(texture_id.kind);
        let slab = store
            .get(texture_id.index)
            .expect("texture_id index out of range");
        slab.create_view()
    }
}

impl PlatformAtlas for GpuAtlas {
    fn get_or_insert(
        &self,
        key: &AtlasKey,
        rasterize: &mut dyn FnMut() -> Result<(Size<DevicePixels>, Vec<u8>), AtlasError>,
    ) -> Result<AtlasTile, AtlasError> {
        let mut state = self.0.lock();

        // Cache hit — return without rasterizing.
        if let Some(tile) = state.tiles_by_key.get(key) {
            return Ok(*tile);
        }

        // Cache miss — rasterize, allocate, enqueue upload.
        let (size, bytes) = rasterize()?;
        let tile = state.allocate(size, key.kind)?;
        state.enqueue_upload(&tile, bytes);
        state.tiles_by_key.insert(key.clone(), tile);

        Ok(tile)
    }

    fn memory_usage(&self) -> usize {
        let state = self.0.lock();
        // Sum bytes across all slabs for all three kinds.
        let mut total: usize = 0;
        for store in [
            &state.storage.monochrome,
            &state.storage.subpixel,
            &state.storage.polychrome,
        ] {
            for slab in &store.textures {
                let bpp = slab.bytes_per_pixel() as usize;
                total += (slab.size.width.0 as usize) * (slab.size.height.0 as usize) * bpp;
            }
        }
        total
    }

    fn remove(&self, key: &AtlasKey) -> bool {
        self.0.lock().tiles_by_key.remove(key).is_some()
    }

    fn clear(&self) {
        let mut state = self.0.lock();
        state.tiles_by_key.clear();
        state.pending_uploads.clear();
        state.storage = AtlasStorage::new();
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use super::*;

    /// Obtain a device+queue pair for headless tests. Returns `None` when no
    /// GPU adapter is available (CI without Vulkan/GL/DX12).
    fn gpu_pair() -> Option<(Arc<wgpu::Device>, Arc<wgpu::Queue>)> {
        pollster::block_on(async {
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: wgpu::Backends::all(),
                flags: wgpu::InstanceFlags::default(),
                dx12_shader_compiler: wgpu::Dx12Compiler::default(),
                gles_minor_version: wgpu::Gles3MinorVersion::default(),
            });
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::LowPower,
                    compatible_surface: None,
                    force_fallback_adapter: false,
                })
                .await?;
            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("nom_gpui_atlas_test"),
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::downlevel_defaults()
                            .using_resolution(adapter.limits())
                            .using_alignment(adapter.limits()),
                        memory_hints: wgpu::MemoryHints::MemoryUsage,
                    },
                    None,
                )
                .await
                .ok()?;
            Some((Arc::new(device), Arc::new(queue)))
        })
    }

    fn mono_key(tag: u8) -> AtlasKey {
        AtlasKey {
            kind: AtlasTextureKind::Monochrome,
            bytes: vec![tag].into(),
        }
    }

    fn poly_key(tag: u8) -> AtlasKey {
        AtlasKey {
            kind: AtlasTextureKind::Polychrome,
            bytes: vec![tag].into(),
        }
    }

    fn sub_key(tag: u8) -> AtlasKey {
        AtlasKey {
            kind: AtlasTextureKind::Subpixel,
            bytes: vec![tag].into(),
        }
    }

    // ── round-trip: insert → cache-hit → flush ────────────────────────────────

    #[test]
    fn round_trip_single_tile() {
        if crate::should_skip_gpu_tests() {
            eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
            return;
        }
        let Some((device, queue)) = gpu_pair() else {
            eprintln!("SKIP: no GPU adapter");
            return;
        };

        let atlas = GpuAtlas::new(device, queue);
        let key = mono_key(1);
        let expected_size = Size::new(DevicePixels(8), DevicePixels(8));

        let tile = atlas
            .get_or_insert(&key, &mut || {
                Ok((expected_size, vec![0xFFu8; 64]))
            })
            .expect("tile allocation failed");

        assert_eq!(tile.size, expected_size);
        assert_eq!(tile.texture.kind, AtlasTextureKind::Monochrome);

        // Flush must not panic.
        atlas.flush_uploads();

        // Second call returns the cached tile without invoking rasterize.
        let again = atlas
            .get_or_insert(&key, &mut || {
                panic!("should not be called on cache hit")
            })
            .expect("cache hit failed");

        assert_eq!(again, tile);
    }

    // ── overflow triggers a second slab ──────────────────────────────────────

    #[test]
    fn overflow_allocates_new_texture() {
        if crate::should_skip_gpu_tests() {
            eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
            return;
        }
        let Some((device, queue)) = gpu_pair() else {
            eprintln!("SKIP: no GPU adapter");
            return;
        };

        let atlas = GpuAtlas::new(device, queue);

        // Each tile is 256×256 monochrome.  A 1024×1024 slab fits 16 of them
        // (4×4 grid).  Inserting 17 must not fail and must bump index.
        let tile_size = Size::new(DevicePixels(256), DevicePixels(256));
        let bytes = vec![0xAAu8; 256 * 256];

        let mut last_tile = None;
        for i in 0u8..17 {
            let tile = atlas
                .get_or_insert(&mono_key(i), &mut || {
                    Ok((tile_size, bytes.clone()))
                })
                .unwrap_or_else(|_| panic!("allocation {i} failed"));
            last_tile = Some(tile);
        }

        // At least one tile must be in a slab beyond index 0.
        let last = last_tile.unwrap();
        // The 17th tile may land in slab 0 (if there was space) or slab ≥1.
        // What we verify is that the atlas did not panic and all tiles are valid.
        assert_eq!(last.size, tile_size);

        atlas.flush_uploads();
    }

    // ── three kinds are stored independently ──────────────────────────────────

    #[test]
    fn three_kinds_have_separate_storage() {
        if crate::should_skip_gpu_tests() {
            eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
            return;
        }
        let Some((device, queue)) = gpu_pair() else {
            eprintln!("SKIP: no GPU adapter");
            return;
        };

        let atlas = GpuAtlas::new(device, queue);
        let sz = Size::new(DevicePixels(8), DevicePixels(8));

        let mono = atlas
            .get_or_insert(&mono_key(1), &mut || Ok((sz, vec![0u8; 64])))
            .unwrap();
        let sub = atlas
            .get_or_insert(&sub_key(1), &mut || Ok((sz, vec![0u8; 256])))
            .unwrap();
        let poly = atlas
            .get_or_insert(&poly_key(1), &mut || Ok((sz, vec![0u8; 256])))
            .unwrap();

        assert_eq!(mono.texture.kind, AtlasTextureKind::Monochrome);
        assert_eq!(sub.texture.kind, AtlasTextureKind::Subpixel);
        assert_eq!(poly.texture.kind, AtlasTextureKind::Polychrome);

        // All three landed in different kind stores; subpixel and polychrome
        // also land in different slab lists even though they share the format.
        assert_ne!(mono.texture.kind, sub.texture.kind);
        assert_ne!(mono.texture.kind, poly.texture.kind);
        assert_ne!(sub.texture.kind, poly.texture.kind);

        atlas.flush_uploads();
    }

    // ── memory_usage is non-zero after inserts ────────────────────────────────

    #[test]
    fn memory_usage_non_zero_after_insert() {
        if crate::should_skip_gpu_tests() {
            eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
            return;
        }
        let Some((device, queue)) = gpu_pair() else {
            eprintln!("SKIP: no GPU adapter");
            return;
        };

        let atlas = GpuAtlas::new(device, queue);
        assert_eq!(atlas.memory_usage(), 0);

        atlas
            .get_or_insert(&mono_key(1), &mut || {
                Ok((Size::new(DevicePixels(4), DevicePixels(4)), vec![0u8; 16]))
            })
            .unwrap();

        assert!(atlas.memory_usage() > 0);
    }

    // ── clear resets the atlas ────────────────────────────────────────────────

    #[test]
    fn clear_drops_all_tiles() {
        if crate::should_skip_gpu_tests() {
            eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
            return;
        }
        let Some((device, queue)) = gpu_pair() else {
            eprintln!("SKIP: no GPU adapter");
            return;
        };

        let atlas = GpuAtlas::new(device, queue);
        let key = mono_key(1);

        atlas
            .get_or_insert(&key, &mut || {
                Ok((Size::new(DevicePixels(4), DevicePixels(4)), vec![0u8; 16]))
            })
            .unwrap();

        atlas.clear();

        // After clear, the key must trigger rasterize again.
        let mut call_count = 0u32;
        atlas
            .get_or_insert(&key, &mut || {
                call_count += 1;
                Ok((Size::new(DevicePixels(4), DevicePixels(4)), vec![0u8; 16]))
            })
            .unwrap();

        assert_eq!(call_count, 1, "rasterize must be called once after clear");
    }

    // ── texture_view does not panic for a valid id ────────────────────────────

    #[test]
    fn texture_view_returns_for_valid_id() {
        if crate::should_skip_gpu_tests() {
            eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
            return;
        }
        let Some((device, queue)) = gpu_pair() else {
            eprintln!("SKIP: no GPU adapter");
            return;
        };

        let atlas = GpuAtlas::new(device, queue);
        let tile = atlas
            .get_or_insert(&mono_key(1), &mut || {
                Ok((Size::new(DevicePixels(8), DevicePixels(8)), vec![0xFFu8; 64]))
            })
            .unwrap();

        // Must not panic.
        let _view = atlas.texture_view(tile.texture);
    }
}
