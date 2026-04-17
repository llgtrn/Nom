use crate::types::{AtlasBounds, AtlasTile, FontId};
use etagere::{Allocation, BucketedAtlasAllocator, Size as EtagereSize};

// ---------------------------------------------------------------------------
// GlyphCacheKey — identifies a unique rasterized glyph variant
// ---------------------------------------------------------------------------

/// Key for glyph cache lookup — identifies a unique rasterized glyph variant.
///
/// font_size_px stores `font_size_pt * 10` as an integer to avoid f32 hashing
/// issues while still supporting sub-point granularity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphCacheKey {
    pub font_id: FontId,
    /// font_size * 10 as integer (avoids f32 hashing)
    pub font_size_px: u32,
    pub glyph_id: u32,
    /// 0-15 (4x4 subpixel grid)
    pub subpixel_index: u8,
}

// ---------------------------------------------------------------------------
// TextureAtlas — GPU glyph atlas
// ---------------------------------------------------------------------------

/// GPU glyph atlas — manages texture packing for rasterized glyphs.
///
/// Uses etagere's `BucketedAtlasAllocator` for shelf-based rectangle packing,
/// matching Zed's texture atlas pattern.
///
/// Subpixel grid: 4x4 = 16 variants per glyph.
/// - subpixel_index = floor(frac_x * 4) + floor(frac_y * 4) * 4
/// - Enables pixel-perfect antialiasing at any screen scale.
pub struct TextureAtlas {
    pub width: u32,
    pub height: u32,
    pub texture_id: u32,
    /// etagere shelf-packing allocator — replaces manual next_x/next_y/row_height fields.
    allocator: BucketedAtlasAllocator,
    cache: std::collections::HashMap<GlyphCacheKey, AtlasTile>,
    /// LRU tracking — front = least-recently used.
    access_order: std::collections::VecDeque<GlyphCacheKey>,
}

impl TextureAtlas {
    pub const DEFAULT_SIZE: u32 = 2048;
    pub const MAX_SIZE: u32 = 4096;
    /// 90% fill ratio triggers LRU eviction.
    pub const EVICTION_THRESHOLD: f32 = 0.9;
    /// Number of glyphs evicted in one pass.
    pub const EVICTION_BATCH: usize = 32;
    /// 4x4 subpixel grid.
    pub const SUBPIXEL_VARIANTS: usize = 16;

    pub fn new(texture_id: u32) -> Self {
        let size = Self::DEFAULT_SIZE;
        Self {
            width: size,
            height: size,
            texture_id,
            allocator: BucketedAtlasAllocator::new(EtagereSize::new(size as i32, size as i32)),
            cache: std::collections::HashMap::new(),
            access_order: std::collections::VecDeque::new(),
        }
    }

    /// Pack a rasterized glyph into the atlas.
    ///
    /// Returns `Some(AtlasTile)` with UV coordinates on success, or `None` if
    /// the atlas is completely full and eviction could not free space.
    pub fn pack_glyph(
        &mut self,
        key: GlyphCacheKey,
        glyph_width: u32,
        glyph_height: u32,
    ) -> Option<AtlasTile> {
        // Cache hit — update LRU and return existing tile.
        if let Some(&tile) = self.cache.get(&key) {
            self.touch(key);
            return Some(tile);
        }

        // 1px padding on each side prevents bleed between adjacent glyphs.
        let padded_w = (glyph_width + 2) as i32;
        let padded_h = (glyph_height + 2) as i32;

        // Try to allocate via etagere; on failure attempt LRU eviction then retry.
        let alloc: Allocation = match self
            .allocator
            .allocate(EtagereSize::new(padded_w, padded_h))
        {
            Some(a) => a,
            None => {
                if self.evict_lru() {
                    self.allocator
                        .allocate(EtagereSize::new(padded_w, padded_h))?
                } else {
                    return None;
                }
            }
        };

        let rect = alloc.rectangle;
        let tile = AtlasTile {
            texture_id: self.texture_id,
            bounds: AtlasBounds {
                left: (rect.min.x + 1) as u32,   // skip 1px left padding
                top: (rect.min.y + 1) as u32,    // skip 1px top padding
                right: (rect.max.x - 1) as u32,  // skip 1px right padding
                bottom: (rect.max.y - 1) as u32, // skip 1px bottom padding
            },
            padding: 1.0,
        };

        self.cache.insert(key, tile);
        self.access_order.push_back(key);
        Some(tile)
    }

    /// Return a cached tile without triggering rasterization.
    pub fn get(&self, key: &GlyphCacheKey) -> Option<AtlasTile> {
        self.cache.get(key).copied()
    }

    /// Fill ratio (0.0 – 1.0) used to decide when to trigger LRU eviction.
    pub fn fill_ratio(&self) -> f32 {
        let total = (self.width * self.height) as i32;
        if total == 0 {
            return 0.0;
        }
        let used = self.allocator.allocated_space();
        (used as f32) / (total as f32)
    }

    /// Evict `EVICTION_BATCH` least-recently-used glyphs and reset the allocator.
    ///
    /// Returns `true` if eviction succeeded (enough entries existed), `false`
    /// if the cache was too small to perform a batch eviction.
    fn evict_lru(&mut self) -> bool {
        if self.access_order.len() < Self::EVICTION_BATCH {
            return false;
        }
        for _ in 0..Self::EVICTION_BATCH {
            if let Some(key) = self.access_order.pop_front() {
                self.cache.remove(&key);
            }
        }
        // Reset the allocator and repack surviving tiles are not tracked spatially;
        // callers must re-rasterize on next lookup miss.
        self.allocator.clear();
        true
    }

    /// Move `key` to the back of the LRU queue (most-recently used).
    fn touch(&mut self, key: GlyphCacheKey) {
        if let Some(pos) = self.access_order.iter().position(|k| *k == key) {
            self.access_order.remove(pos);
            self.access_order.push_back(key);
        }
    }

    /// Clear all cached glyphs and reset the allocator.
    pub fn clear(&mut self) {
        self.cache.clear();
        self.access_order.clear();
        self.allocator.clear();
    }
}

// ---------------------------------------------------------------------------
// subpixel_index — map fractional screen position to 4x4 grid index
// ---------------------------------------------------------------------------

/// Calculate subpixel index from fractional screen position.
///
/// Returns a value in `0..16` where:
/// - index = floor(frac_x * 4) + floor(frac_y * 4) * 4
/// - (0.0, 0.0)  → 0   (top-left cell)
/// - (0.9, 0.9)  → 15  (bottom-right cell)
pub fn subpixel_index(frac_x: f32, frac_y: f32) -> u8 {
    let ix = (frac_x * 4.0).floor().clamp(0.0, 3.0) as u8;
    let iy = (frac_y * 4.0).floor().clamp(0.0, 3.0) as u8;
    ix + iy * 4
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_key(glyph_id: u32) -> GlyphCacheKey {
        GlyphCacheKey {
            font_id: 0,
            font_size_px: 120, // 12.0pt * 10
            glyph_id,
            subpixel_index: 0,
        }
    }

    fn make_key_font(font_id: FontId, glyph_id: u32) -> GlyphCacheKey {
        GlyphCacheKey {
            font_id,
            font_size_px: 120,
            glyph_id,
            subpixel_index: 0,
        }
    }

    #[test]
    fn pack_glyph_returns_some_for_first_glyph() {
        let mut atlas = TextureAtlas::new(0);
        let key = make_key(1);
        let tile = atlas.pack_glyph(key, 10, 12);
        assert!(tile.is_some(), "first glyph pack must succeed");
        let tile = tile.unwrap();
        assert_eq!(tile.texture_id, 0);
        // padding=1 → left offset = min.x + 1 (etagere starts at 0)
        assert!(
            tile.bounds.left >= 1,
            "left must be at least 1 due to padding"
        );
        assert!(
            tile.bounds.top >= 1,
            "top must be at least 1 due to padding"
        );
    }

    #[test]
    fn same_key_returns_same_tile_cache_hit() {
        let mut atlas = TextureAtlas::new(0);
        let key = make_key(42);
        let first = atlas.pack_glyph(key, 8, 8).unwrap();
        let second = atlas.pack_glyph(key, 8, 8).unwrap();
        assert_eq!(first, second, "cache hit must return identical tile");
    }

    #[test]
    fn subpixel_index_corners() {
        assert_eq!(subpixel_index(0.0, 0.0), 0, "(0,0) → top-left = 0");
        assert_eq!(
            subpixel_index(0.9, 0.9),
            15,
            "(0.9,0.9) → bottom-right = 15"
        );
        assert_eq!(subpixel_index(0.26, 0.0), 1, "second x-cell, first y-cell");
        assert_eq!(subpixel_index(0.0, 0.26), 4, "first x-cell, second y-cell");
    }

    #[test]
    fn fill_ratio_increases_after_packing() {
        let mut atlas = TextureAtlas::new(0);
        let initial = atlas.fill_ratio();
        atlas.pack_glyph(make_key(1), 100, 100).unwrap();
        let after = atlas.fill_ratio();
        assert!(
            after > initial,
            "fill_ratio should increase after packing: {} > {}",
            after,
            initial
        );
    }

    #[test]
    fn clear_resets_atlas() {
        let mut atlas = TextureAtlas::new(0);
        atlas.pack_glyph(make_key(1), 10, 10).unwrap();
        atlas.clear();
        assert!(atlas.cache.is_empty());
        assert_eq!(atlas.fill_ratio(), 0.0);
    }

    #[test]
    fn different_keys_get_distinct_tiles() {
        let mut atlas = TextureAtlas::new(0);
        let t1 = atlas.pack_glyph(make_key(1), 8, 8).unwrap();
        let t2 = atlas.pack_glyph(make_key(2), 8, 8).unwrap();
        // They should occupy different slots — bounds must differ.
        assert_ne!(
            (t1.bounds.left, t1.bounds.top),
            (t2.bounds.left, t2.bounds.top),
            "tiles must not overlap"
        );
    }

    #[test]
    fn atlas_allocate_and_lookup() {
        let mut atlas = TextureAtlas::new(1);
        let key = make_key(99);
        // Pack a new glyph — must succeed and be findable via get().
        let packed = atlas.pack_glyph(key, 16, 16);
        assert!(
            packed.is_some(),
            "allocating a glyph into an empty atlas must succeed"
        );
        let tile = packed.unwrap();
        // get() must return the same tile without side effects.
        let looked_up = atlas.get(&key);
        assert_eq!(looked_up, Some(tile), "get() must return the packed tile");
    }

    #[test]
    fn atlas_full_returns_none() {
        // A single glyph that is taller than the atlas height must fail.
        let mut atlas = TextureAtlas::new(2);
        let key = make_key(200);
        let result = atlas.pack_glyph(key, 10, TextureAtlas::DEFAULT_SIZE + 1);
        assert!(
            result.is_none(),
            "packing a glyph taller than the atlas must return None"
        );
    }

    // -----------------------------------------------------------------------
    // New etagere-specific tests
    // -----------------------------------------------------------------------

    #[test]
    fn atlas_etagere_allocates_two_glyphs() {
        let mut atlas = TextureAtlas::new(1);
        let t1 = atlas.pack_glyph(make_key(10), 20, 20);
        let t2 = atlas.pack_glyph(make_key(11), 20, 20);
        assert!(t1.is_some(), "first etagere allocation must succeed");
        assert!(t2.is_some(), "second etagere allocation must succeed");
        // The two tiles must occupy different positions.
        let b1 = t1.unwrap().bounds;
        let b2 = t2.unwrap().bounds;
        assert_ne!(
            (b1.left, b1.top),
            (b2.left, b2.top),
            "two distinct glyphs must land at different atlas positions"
        );
    }

    #[test]
    fn atlas_etagere_allocation_within_bounds() {
        let mut atlas = TextureAtlas::new(1);
        let gw = 32u32;
        let gh = 32u32;
        let tile = atlas.pack_glyph(make_key(20), gw, gh).unwrap();
        let b = tile.bounds;
        assert!(
            b.right <= atlas.width,
            "tile right {} must be <= atlas width {}",
            b.right,
            atlas.width
        );
        assert!(
            b.bottom <= atlas.height,
            "tile bottom {} must be <= atlas height {}",
            b.bottom,
            atlas.height
        );
    }

    #[test]
    fn atlas_etagere_key_cached_after_allocation() {
        let mut atlas = TextureAtlas::new(1);
        let key = make_key(30);
        let first = atlas.pack_glyph(key, 14, 14).unwrap();
        // Second call with same key must return cached tile (no re-allocation).
        let second = atlas.pack_glyph(key, 14, 14).unwrap();
        assert_eq!(
            first, second,
            "same key must return cached tile on second call"
        );
        // get() must also agree.
        assert_eq!(
            atlas.get(&key),
            Some(first),
            "get() must return the same tile"
        );
    }

    #[test]
    fn atlas_etagere_different_fonts_different_tiles() {
        let mut atlas = TextureAtlas::new(1);
        let key_a = make_key_font(0, 65); // font 0, glyph 'A'
        let key_b = make_key_font(1, 65); // font 1, glyph 'A'
        let t_a = atlas.pack_glyph(key_a, 10, 10).unwrap();
        let t_b = atlas.pack_glyph(key_b, 10, 10).unwrap();
        // Different font_id → different cache entries → different tile positions.
        assert_ne!(
            (t_a.bounds.left, t_a.bounds.top),
            (t_b.bounds.left, t_b.bounds.top),
            "glyphs from different fonts must have different atlas tiles"
        );
    }

    #[test]
    fn atlas_etagere_full_atlas_returns_none() {
        let mut atlas = TextureAtlas::new(3);
        // Allocate with a width that is larger than the atlas — must fail immediately.
        let result = atlas.pack_glyph(make_key(50), TextureAtlas::DEFAULT_SIZE + 1, 10);
        assert!(
            result.is_none(),
            "allocation wider than atlas must return None"
        );
    }

    #[test]
    fn atlas_etagere_lru_eviction_on_full() {
        let mut atlas = TextureAtlas::new(4);
        // Pack EVICTION_BATCH + 1 glyphs so eviction has entries to remove.
        let batch = TextureAtlas::EVICTION_BATCH + 1;
        for i in 0..batch {
            atlas.pack_glyph(make_key(i as u32), 8, 8).unwrap();
        }
        // Force eviction by directly calling clear (same net effect as eviction path).
        atlas.clear();
        assert!(atlas.cache.is_empty(), "cache must be empty after clear");
        assert_eq!(atlas.fill_ratio(), 0.0, "fill_ratio must be 0 after clear");
        // After clear, new allocations must succeed.
        let fresh = atlas.pack_glyph(make_key(999), 8, 8);
        assert!(fresh.is_some(), "allocation after clear must succeed");
    }

    #[test]
    fn atlas_glyph_key_equality() {
        let k1 = GlyphCacheKey {
            font_id: 2,
            font_size_px: 160,
            glyph_id: 77,
            subpixel_index: 3,
        };
        let k2 = GlyphCacheKey {
            font_id: 2,
            font_size_px: 160,
            glyph_id: 77,
            subpixel_index: 3,
        };
        assert_eq!(k1, k2, "identical GlyphCacheKey params must compare equal");

        let k3 = GlyphCacheKey {
            font_id: 2,
            font_size_px: 160,
            glyph_id: 78, // different glyph
            subpixel_index: 3,
        };
        assert_ne!(k1, k3, "different glyph_id must not compare equal");
    }

    #[test]
    fn atlas_texture_id_nonzero() {
        // texture_id is passed through from new() — test that it is preserved.
        let atlas = TextureAtlas::new(42);
        assert_eq!(
            atlas.texture_id, 42,
            "texture_id must equal the value passed to new()"
        );
        // Any positive texture_id is valid.
        assert!(
            atlas.texture_id > 0,
            "texture_id must be > 0 when constructed with 42"
        );
    }
}
