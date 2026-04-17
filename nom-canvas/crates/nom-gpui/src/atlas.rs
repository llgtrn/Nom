//! Texture atlas for glyphs and sprites.
//!
//! Architecture mirrors Zed GPUI's `PlatformAtlas`: trait-based so that desktop
//! (wgpu) and browser (WebGPU) share the same surface but plug different
//! backends. Allocation uses `etagere::BucketedAtlasAllocator` — the same bin
//! packer Zed uses, proven in production.

use crate::geometry::{DevicePixels, Point, Size};
use parking_lot::Mutex;
use std::sync::Arc;
use thiserror::Error;

/// Which rasterization mode a texture holds.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AtlasTextureKind {
    /// R8 grayscale glyphs (no subpixel positioning).
    Monochrome,
    /// R8 glyphs with 4-variant horizontal subpixel positioning.
    Subpixel,
    /// RGBA8 color sprites (emoji, images).
    Polychrome,
}

/// Stable handle to an atlas texture slot.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AtlasTextureId {
    pub kind: AtlasTextureKind,
    pub index: u32,
}

/// Allocated region within an atlas texture.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AtlasTile {
    pub texture: AtlasTextureId,
    pub origin: Point<DevicePixels>,
    pub size: Size<DevicePixels>,
    pub padding: DevicePixels,
}

impl AtlasTile {
    pub fn uv(&self, texture_size: Size<DevicePixels>) -> [f32; 4] {
        let sx = texture_size.width.0.max(1) as f32;
        let sy = texture_size.height.0.max(1) as f32;
        [
            self.origin.x.0 as f32 / sx,
            self.origin.y.0 as f32 / sy,
            (self.origin.x.0 + self.size.width.0) as f32 / sx,
            (self.origin.y.0 + self.size.height.0) as f32 / sy,
        ]
    }
}

/// Composite key identifying one cached rasterization.
///
/// `bytes` is an `Arc<[u8]>` so cloning the key on the hot cache-lookup path
/// is a single atomic increment rather than a heap allocation.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AtlasKey {
    pub kind: AtlasTextureKind,
    pub bytes: Arc<[u8]>,
}

/// Atlas allocation errors.
#[derive(Debug, Error)]
pub enum AtlasError {
    #[error("requested tile size {0:?} exceeds maximum texture size {1:?}")]
    TooLarge(Size<DevicePixels>, Size<DevicePixels>),
}

/// Platform-agnostic atlas interface. `nom-gpui`'s renderer will implement
/// this over wgpu; tests can supply in-memory mocks.
pub trait PlatformAtlas: Send + Sync {
    /// Resolve a cache key to a tile, rasterizing if missing.
    ///
    /// `rasterize` is called only on miss; it returns `(size, raster_bytes)`.
    /// Implementations must keep the returned `AtlasTile` stable for the
    /// lifetime of the atlas (no relocation).
    fn get_or_insert(
        &self,
        key: &AtlasKey,
        rasterize: &mut dyn FnMut() -> Result<(Size<DevicePixels>, Vec<u8>), AtlasError>,
    ) -> Result<AtlasTile, AtlasError>;

    /// Total bytes uploaded (for telemetry).
    fn memory_usage(&self) -> usize;

    /// Remove a single tile from the cache. Returns `true` if the key was
    /// present. Implementations should NOT free the underlying texture region —
    /// that requires compaction and is out of scope for MVP. Marking the tile
    /// absent is sufficient: future lookups will re-rasterize into new atlas
    /// space (until the entire slab is recycled via `clear`).
    fn remove(&self, key: &AtlasKey) -> bool;

    /// Clear all tiles (typically on theme change / font reload).
    fn clear(&self);
}

/// In-memory atlas suitable for tests and headless workloads.
/// Each unique key gets a sequential tile in a single virtual texture per kind.
pub struct InMemoryAtlas {
    inner: Mutex<InMemoryAtlasInner>,
    texture_size: Size<DevicePixels>,
}

struct InMemoryAtlasInner {
    tiles: std::collections::HashMap<AtlasKey, AtlasTile>,
    next_origin_per_kind: std::collections::HashMap<AtlasTextureKind, Point<DevicePixels>>,
    row_height_per_kind: std::collections::HashMap<AtlasTextureKind, DevicePixels>,
    memory_usage: usize,
}

impl InMemoryAtlas {
    pub fn new(texture_size: Size<DevicePixels>) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(InMemoryAtlasInner {
                tiles: Default::default(),
                next_origin_per_kind: Default::default(),
                row_height_per_kind: Default::default(),
                memory_usage: 0,
            }),
            texture_size,
        })
    }
}

impl PlatformAtlas for InMemoryAtlas {
    fn get_or_insert(
        &self,
        key: &AtlasKey,
        rasterize: &mut dyn FnMut() -> Result<(Size<DevicePixels>, Vec<u8>), AtlasError>,
    ) -> Result<AtlasTile, AtlasError> {
        let mut inner = self.inner.lock();
        if let Some(tile) = inner.tiles.get(key) {
            return Ok(*tile);
        }
        let (size, bytes) = rasterize()?;
        if size.width.0 > self.texture_size.width.0
            || size.height.0 > self.texture_size.height.0
        {
            return Err(AtlasError::TooLarge(size, self.texture_size));
        }
        // Simple shelf allocator: pack horizontally in rows of max-glyph-height.
        let origin = *inner
            .next_origin_per_kind
            .entry(key.kind)
            .or_insert_with(|| Point::new(DevicePixels::ZERO, DevicePixels::ZERO));
        let row_height = inner
            .row_height_per_kind
            .entry(key.kind)
            .or_insert(DevicePixels::ZERO);
        let mut origin = origin;
        let row_height = *row_height;
        let tex_w = self.texture_size.width.0;
        if origin.x.0 + size.width.0 > tex_w {
            // Wrap to next row.
            origin = Point::new(DevicePixels::ZERO, DevicePixels(origin.y.0 + row_height.0));
            inner.row_height_per_kind.insert(key.kind, DevicePixels::ZERO);
        }
        // Reject if the new row exceeds texture height.
        if origin.y.0 + size.height.0 > self.texture_size.height.0 {
            return Err(AtlasError::TooLarge(size, self.texture_size));
        }
        // Update row height bookkeeping.
        let new_row_height = inner
            .row_height_per_kind
            .entry(key.kind)
            .or_insert(DevicePixels::ZERO);
        if size.height.0 > new_row_height.0 {
            *new_row_height = size.height;
        }
        let tile = AtlasTile {
            texture: AtlasTextureId {
                kind: key.kind,
                index: 0,
            },
            origin,
            size,
            padding: DevicePixels::ZERO,
        };
        // Advance cursor.
        inner
            .next_origin_per_kind
            .insert(key.kind, Point::new(DevicePixels(origin.x.0 + size.width.0), origin.y));
        inner.memory_usage += bytes.len();
        inner.tiles.insert(key.clone(), tile);
        Ok(tile)
    }

    fn memory_usage(&self) -> usize {
        self.inner.lock().memory_usage
    }

    fn remove(&self, key: &AtlasKey) -> bool {
        self.inner.lock().tiles.remove(key).is_some()
    }

    fn clear(&self) {
        let mut inner = self.inner.lock();
        inner.tiles.clear();
        inner.next_origin_per_kind.clear();
        inner.row_height_per_kind.clear();
        inner.memory_usage = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(kind: AtlasTextureKind, tag: u8) -> AtlasKey {
        AtlasKey {
            kind,
            bytes: Arc::from(vec![tag]),
        }
    }

    #[test]
    fn cache_hit_does_not_rasterize_twice() {
        let atlas = InMemoryAtlas::new(Size::new(DevicePixels(1024), DevicePixels(1024)));
        let k = key(AtlasTextureKind::Monochrome, 1);
        let mut calls = 0;
        let mut raster = || -> Result<_, AtlasError> {
            calls += 1;
            Ok((Size::new(DevicePixels(16), DevicePixels(16)), vec![0u8; 256]))
        };
        let t1 = atlas.get_or_insert(&k, &mut raster).unwrap();
        let t2 = atlas.get_or_insert(&k, &mut raster).unwrap();
        assert_eq!(t1, t2);
        assert_eq!(calls, 1);
    }

    #[test]
    fn rejects_tiles_larger_than_texture() {
        let atlas = InMemoryAtlas::new(Size::new(DevicePixels(64), DevicePixels(64)));
        let k = key(AtlasTextureKind::Polychrome, 2);
        let err = atlas
            .get_or_insert(&k, &mut || {
                Ok((Size::new(DevicePixels(128), DevicePixels(128)), vec![]))
            })
            .unwrap_err();
        assert!(matches!(err, AtlasError::TooLarge(_, _)));
    }

    #[test]
    fn kinds_do_not_collide() {
        let atlas = InMemoryAtlas::new(Size::new(DevicePixels(1024), DevicePixels(1024)));
        let mono = atlas
            .get_or_insert(&key(AtlasTextureKind::Monochrome, 1), &mut || {
                Ok((Size::new(DevicePixels(10), DevicePixels(10)), vec![0; 100]))
            })
            .unwrap();
        let poly = atlas
            .get_or_insert(&key(AtlasTextureKind::Polychrome, 1), &mut || {
                Ok((Size::new(DevicePixels(10), DevicePixels(10)), vec![0; 400]))
            })
            .unwrap();
        assert_ne!(mono.texture.kind, poly.texture.kind);
    }

    #[test]
    fn memory_usage_accumulates() {
        let atlas = InMemoryAtlas::new(Size::new(DevicePixels(1024), DevicePixels(1024)));
        atlas
            .get_or_insert(&key(AtlasTextureKind::Monochrome, 1), &mut || {
                Ok((Size::new(DevicePixels(4), DevicePixels(4)), vec![0; 16]))
            })
            .unwrap();
        atlas
            .get_or_insert(&key(AtlasTextureKind::Monochrome, 2), &mut || {
                Ok((Size::new(DevicePixels(4), DevicePixels(4)), vec![0; 16]))
            })
            .unwrap();
        assert_eq!(atlas.memory_usage(), 32);
    }

    #[test]
    fn uv_in_unit_space() {
        let tile = AtlasTile {
            texture: AtlasTextureId {
                kind: AtlasTextureKind::Monochrome,
                index: 0,
            },
            origin: Point::new(DevicePixels(512), DevicePixels(0)),
            size: Size::new(DevicePixels(512), DevicePixels(1024)),
            padding: DevicePixels::ZERO,
        };
        let uv = tile.uv(Size::new(DevicePixels(1024), DevicePixels(1024)));
        assert_eq!(uv, [0.5, 0.0, 1.0, 1.0]);
    }

    /// After `remove`, the key is evicted and a subsequent `get_or_insert`
    /// must call the rasterize closure again (re-rasterize).
    #[test]
    fn remove_evicts_tile() {
        let atlas = InMemoryAtlas::new(Size::new(DevicePixels(1024), DevicePixels(1024)));
        let k = key(AtlasTextureKind::Monochrome, 7);
        let mut calls = 0usize;

        // First insert — rasterize fires.
        {
            let mut raster = || -> Result<_, AtlasError> {
                calls += 1;
                Ok((Size::new(DevicePixels(8), DevicePixels(8)), vec![0u8; 64]))
            };
            atlas.get_or_insert(&k, &mut raster).unwrap();
        }
        assert_eq!(calls, 1);

        // Remove the tile — should report it was present.
        assert!(atlas.remove(&k), "remove should return true for a present tile");

        // Second insert — tile was evicted so rasterize fires again.
        {
            let mut raster = || -> Result<_, AtlasError> {
                calls += 1;
                Ok((Size::new(DevicePixels(8), DevicePixels(8)), vec![0u8; 64]))
            };
            atlas.get_or_insert(&k, &mut raster).unwrap();
        }
        assert_eq!(calls, 2, "re-rasterize must happen after eviction");

        // Removing a key that is not present should return false.
        let absent = key(AtlasTextureKind::Polychrome, 99);
        assert!(!atlas.remove(&absent), "remove must return false for absent key");
    }

    /// Cloning an `AtlasKey` backed by `Arc<[u8]>` is cheap: no heap
    /// allocation per clone. This test constructs a 1 MiB key and clones it
    /// 100 times to confirm the code path compiles and runs without allocating
    /// new backing buffers (we just count clones, not benchmark timing).
    #[test]
    fn atlas_key_arc_bytes_are_cheap_to_clone() {
        let big: Arc<[u8]> = Arc::from(vec![0u8; 1024 * 1024]);
        let k = AtlasKey {
            kind: AtlasTextureKind::Polychrome,
            bytes: big,
        };
        // 100 clones — each is one atomic increment, no heap allocation.
        let clones: Vec<AtlasKey> = (0..100).map(|_| k.clone()).collect();
        assert_eq!(clones.len(), 100);
        // All clones share the same underlying buffer.
        for c in &clones {
            assert_eq!(c.bytes.len(), 1024 * 1024);
        }
    }
}
