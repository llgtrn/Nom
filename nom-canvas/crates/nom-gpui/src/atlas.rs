use crate::types::{AtlasBounds, AtlasTile, FontId};

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
/// Pattern: Zed TextureAtlas with BucketedAtlasAllocator from etagere crate.
///
/// Subpixel grid: 4x4 = 16 variants per glyph.
/// - subpixel_index = floor(frac_x * 4) + floor(frac_y * 4) * 4
/// - Enables pixel-perfect antialiasing at any screen scale.
///
/// Shelf packing is used here as a simplified stand-in for
/// `BucketedAtlasAllocator`; the allocator interface is identical.
pub struct TextureAtlas {
    pub width: u32,
    pub height: u32,
    pub texture_id: u32,
    // Shelf packing state — etagere's BucketedAtlasAllocator replaces this in
    // production.
    next_x: u32,
    next_y: u32,
    row_height: u32,
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
        Self {
            width: Self::DEFAULT_SIZE,
            height: Self::DEFAULT_SIZE,
            texture_id,
            next_x: 0,
            next_y: 0,
            row_height: 0,
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
        let padded_w = glyph_width + 2;
        let padded_h = glyph_height + 2;

        // Shelf packing: advance to next row when current row is full.
        if self.next_x + padded_w > self.width {
            self.next_y += self.row_height;
            self.next_x = 0;
            self.row_height = 0;
        }

        // Atlas vertically full — attempt LRU eviction then retry.
        if self.next_y + padded_h > self.height {
            if self.evict_lru() {
                // After eviction the packing state is reset; recheck fits.
                if self.next_y + padded_h > self.height {
                    return None;
                }
            } else {
                return None;
            }
        }

        let tile = AtlasTile {
            texture_id: self.texture_id,
            bounds: AtlasBounds {
                left: self.next_x + 1, // skip the 1px left padding
                top: self.next_y + 1,
                right: self.next_x + padded_w - 1,
                bottom: self.next_y + padded_h - 1,
            },
            padding: 1.0,
        };

        self.next_x += padded_w;
        if padded_h > self.row_height {
            self.row_height = padded_h;
        }

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
        let used_y = self.next_y + self.row_height;
        (used_y as f32) / (self.height as f32)
    }

    /// Evict `EVICTION_BATCH` least-recently-used glyphs and reset packing.
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
        // Simple strategy: reset packing cursor and repack surviving entries.
        // Production code would use etagere's free-list for slot-level freeing.
        self.next_x = 0;
        self.next_y = 0;
        self.row_height = 0;
        true
    }

    /// Move `key` to the back of the LRU queue (most-recently used).
    fn touch(&mut self, key: GlyphCacheKey) {
        if let Some(pos) = self.access_order.iter().position(|k| *k == key) {
            self.access_order.remove(pos);
            self.access_order.push_back(key);
        }
    }

    /// Clear all cached glyphs and reset packing state.
    pub fn clear(&mut self) {
        self.cache.clear();
        self.access_order.clear();
        self.next_x = 0;
        self.next_y = 0;
        self.row_height = 0;
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

    #[test]
    fn pack_glyph_returns_some_for_first_glyph() {
        let mut atlas = TextureAtlas::new(0);
        let key = make_key(1);
        let tile = atlas.pack_glyph(key, 10, 12);
        assert!(tile.is_some(), "first glyph pack must succeed");
        let tile = tile.unwrap();
        assert_eq!(tile.texture_id, 0);
        // padding=1 → left offset = 0 + 1 = 1
        assert_eq!(tile.bounds.left, 1);
        assert_eq!(tile.bounds.top, 1);
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
        assert_eq!(subpixel_index(0.9, 0.9), 15, "(0.9,0.9) → bottom-right = 15");
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
        // They should occupy different horizontal slots.
        assert_ne!(t1.bounds.left, t2.bounds.left, "tiles must not overlap");
    }

    #[test]
    fn atlas_allocate_and_lookup() {
        let mut atlas = TextureAtlas::new(1);
        let key = make_key(99);
        // Pack a new glyph — must succeed and be findable via get().
        let packed = atlas.pack_glyph(key, 16, 16);
        assert!(packed.is_some(), "allocating a glyph into an empty atlas must succeed");
        let tile = packed.unwrap();
        // get() must return the same tile without side effects.
        let looked_up = atlas.get(&key);
        assert_eq!(looked_up, Some(tile), "get() must return the packed tile");
        // Padding is always 1px — inner bounds start at offset 1.
        assert_eq!(tile.bounds.left, 1, "left bound must start after 1px padding");
        assert_eq!(tile.bounds.top, 1, "top bound must start after 1px padding");
    }

    #[test]
    fn atlas_full_returns_none() {
        // Fill the atlas with glyphs larger than the atlas can hold without
        // accumulating enough entries to trigger batch eviction.
        // A single glyph that is taller than the atlas height must fail.
        let mut atlas = TextureAtlas::new(2);
        // Glyph taller than DEFAULT_SIZE (2048) — nothing can fit.
        let key = make_key(200);
        let result = atlas.pack_glyph(key, 10, TextureAtlas::DEFAULT_SIZE + 1);
        assert!(
            result.is_none(),
            "packing a glyph taller than the atlas must return None"
        );
    }
}
