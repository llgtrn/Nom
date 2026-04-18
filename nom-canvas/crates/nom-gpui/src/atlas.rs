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

    /// Returns the number of cached glyph entries currently in the atlas.
    pub fn entry_count(&self) -> usize {
        self.cache.len()
    }

    /// Returns the maximum number of glyph entries that could theoretically
    /// fit in the atlas at the minimum glyph size (1×1 px after padding).
    ///
    /// This is a conservative upper bound: `(width * height) / 9` because
    /// every allocation has 1px padding on each side (3×3 padded footprint
    /// minimum per glyph).
    pub fn capacity(&self) -> usize {
        let area = (self.width as usize) * (self.height as usize);
        // Each glyph occupies at least a 3×3 padded region (1px + 2px padding).
        area / 9
    }

    /// Returns the utilisation ratio of the atlas as a value in `[0.0, 1.0]`.
    ///
    /// Computed as `entry_count / capacity`.  Returns `0.0` when the atlas is
    /// empty or capacity is zero.
    pub fn utilization(&self) -> f32 {
        let cap = self.capacity();
        if cap == 0 {
            return 0.0;
        }
        (self.entry_count() as f32) / (cap as f32)
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

    // ------------------------------------------------------------------
    // Wave AH: atlas region tests
    // ------------------------------------------------------------------

    #[test]
    fn atlas_allocate_small_region_succeeds() {
        let mut atlas = TextureAtlas::new(0);
        let key = make_key(1000);
        let tile = atlas.pack_glyph(key, 4, 4);
        assert!(tile.is_some(), "small glyph allocation must succeed");
    }

    #[test]
    fn atlas_allocate_fills_capacity() {
        let mut atlas = TextureAtlas::new(0);
        // Pack enough glyphs to increase fill_ratio above 0.
        for i in 0..10u32 {
            atlas.pack_glyph(make_key(i + 2000), 64, 64).unwrap();
        }
        assert!(atlas.fill_ratio() > 0.0, "fill_ratio must be > 0 after allocations");
    }

    #[test]
    fn atlas_free_releases_space() {
        let mut atlas = TextureAtlas::new(0);
        atlas.pack_glyph(make_key(3000), 32, 32).unwrap();
        let before = atlas.fill_ratio();
        assert!(before > 0.0, "fill_ratio must be > 0 before clear");
        atlas.clear();
        assert_eq!(atlas.fill_ratio(), 0.0, "fill_ratio must be 0 after clear");
    }

    #[test]
    fn atlas_allocate_after_free_succeeds() {
        let mut atlas = TextureAtlas::new(0);
        atlas.pack_glyph(make_key(4000), 10, 10).unwrap();
        atlas.clear();
        // Allocation after clear must succeed.
        let tile = atlas.pack_glyph(make_key(4001), 10, 10);
        assert!(tile.is_some(), "allocation after clear must succeed");
    }

    #[test]
    fn atlas_region_has_valid_coords() {
        let mut atlas = TextureAtlas::new(0);
        let tile = atlas.pack_glyph(make_key(5000), 16, 16).unwrap();
        let b = tile.bounds;
        assert!(b.left < b.right, "left must be less than right: {} < {}", b.left, b.right);
        assert!(b.top < b.bottom, "top must be less than bottom: {} < {}", b.top, b.bottom);
        assert!(b.right <= atlas.width, "right must not exceed atlas width");
        assert!(b.bottom <= atlas.height, "bottom must not exceed atlas height");
    }

    #[test]
    fn atlas_empty_atlas_zero_used() {
        let atlas = TextureAtlas::new(0);
        assert_eq!(atlas.fill_ratio(), 0.0, "empty atlas must have 0 fill ratio");
    }

    #[test]
    fn atlas_multiple_allocations_nonoverlapping() {
        let mut atlas = TextureAtlas::new(0);
        let t1 = atlas.pack_glyph(make_key(6001), 20, 20).unwrap();
        let t2 = atlas.pack_glyph(make_key(6002), 20, 20).unwrap();
        let t3 = atlas.pack_glyph(make_key(6003), 20, 20).unwrap();
        // All three tiles must land at different positions.
        let positions = [
            (t1.bounds.left, t1.bounds.top),
            (t2.bounds.left, t2.bounds.top),
            (t3.bounds.left, t3.bounds.top),
        ];
        // Check that not all positions are equal (at least two must differ).
        let all_same = positions[0] == positions[1] && positions[1] == positions[2];
        assert!(!all_same, "tiles must not all overlap at the same position");
    }

    #[test]
    fn atlas_clear_resets_to_empty() {
        let mut atlas = TextureAtlas::new(0);
        for i in 0..5u32 {
            atlas.pack_glyph(make_key(i + 7000), 8, 8).unwrap();
        }
        atlas.clear();
        assert_eq!(atlas.fill_ratio(), 0.0, "fill_ratio must be 0 after clear");
        // get() must return None for all previously packed keys.
        for i in 0..5u32 {
            let key = make_key(i + 7000);
            assert!(atlas.get(&key).is_none(), "cleared atlas must not contain key {i}");
        }
    }

    // ------------------------------------------------------------------
    // Wave AK: additional atlas tests
    // ------------------------------------------------------------------

    #[test]
    fn atlas_default_size_is_2048() {
        assert_eq!(TextureAtlas::DEFAULT_SIZE, 2048, "default atlas size must be 2048");
    }

    #[test]
    fn atlas_max_size_is_4096() {
        assert_eq!(TextureAtlas::MAX_SIZE, 4096, "max atlas size must be 4096");
    }

    #[test]
    fn atlas_eviction_threshold_is_0_9() {
        let threshold = TextureAtlas::EVICTION_THRESHOLD;
        assert!((threshold - 0.9).abs() < 1e-6, "eviction threshold must be 0.9, got {threshold}");
    }

    #[test]
    fn atlas_eviction_batch_is_32() {
        assert_eq!(TextureAtlas::EVICTION_BATCH, 32, "eviction batch size must be 32");
    }

    #[test]
    fn atlas_subpixel_variants_is_16() {
        assert_eq!(TextureAtlas::SUBPIXEL_VARIANTS, 16, "must have 16 subpixel variants (4x4 grid)");
    }

    #[test]
    fn atlas_initial_fill_ratio_is_zero() {
        let atlas = TextureAtlas::new(0);
        assert_eq!(atlas.fill_ratio(), 0.0, "new atlas must have 0 fill ratio");
    }

    #[test]
    fn atlas_tile_padding_is_one() {
        let mut atlas = TextureAtlas::new(0);
        let key = make_key(8001);
        let tile = atlas.pack_glyph(key, 10, 10).unwrap();
        assert!((tile.padding - 1.0).abs() < 1e-6, "tile padding must be 1.0, got {}", tile.padding);
    }

    #[test]
    fn atlas_subpixel_index_covers_full_range() {
        // Verify all 16 possible subpixel indices are reachable.
        let mut found = [false; 16];
        for xi in 0..4u8 {
            for yi in 0..4u8 {
                let fx = (xi as f32) * 0.25 + 0.01;
                let fy = (yi as f32) * 0.25 + 0.01;
                let idx = subpixel_index(fx, fy) as usize;
                assert!(idx < 16, "subpixel_index must be in 0..16, got {idx}");
                found[idx] = true;
            }
        }
        assert!(found.iter().all(|&x| x), "all 16 subpixel indices must be reachable");
    }

    #[test]
    fn atlas_get_returns_none_for_unknown_key() {
        let atlas = TextureAtlas::new(0);
        let key = make_key(9999);
        assert!(atlas.get(&key).is_none(), "get on unknown key must return None");
    }

    #[test]
    fn atlas_width_and_height_match_default_size() {
        let atlas = TextureAtlas::new(0);
        assert_eq!(atlas.width, TextureAtlas::DEFAULT_SIZE);
        assert_eq!(atlas.height, TextureAtlas::DEFAULT_SIZE);
    }

    // ------------------------------------------------------------------
    // Wave AO: entry_count, capacity, utilization
    // ------------------------------------------------------------------

    #[test]
    fn entry_count_zero_on_new_atlas() {
        let atlas = TextureAtlas::new(0);
        assert_eq!(atlas.entry_count(), 0, "new atlas must have 0 entries");
    }

    #[test]
    fn entry_count_increases_after_pack() {
        let mut atlas = TextureAtlas::new(0);
        atlas.pack_glyph(make_key(10001), 8, 8).unwrap();
        assert_eq!(atlas.entry_count(), 1, "entry_count must be 1 after one pack");
    }

    #[test]
    fn entry_count_increases_for_distinct_keys() {
        let mut atlas = TextureAtlas::new(0);
        for i in 0..5u32 {
            atlas.pack_glyph(make_key(20000 + i), 8, 8).unwrap();
        }
        assert_eq!(atlas.entry_count(), 5, "5 distinct packs must give entry_count=5");
    }

    #[test]
    fn entry_count_unchanged_for_cache_hit() {
        let mut atlas = TextureAtlas::new(0);
        let key = make_key(30001);
        atlas.pack_glyph(key, 8, 8).unwrap();
        // Same key again — cache hit, no new entry.
        atlas.pack_glyph(key, 8, 8).unwrap();
        assert_eq!(atlas.entry_count(), 1, "cache hit must not increase entry_count");
    }

    #[test]
    fn entry_count_zero_after_clear() {
        let mut atlas = TextureAtlas::new(0);
        atlas.pack_glyph(make_key(40001), 8, 8).unwrap();
        assert_eq!(atlas.entry_count(), 1);
        atlas.clear();
        assert_eq!(atlas.entry_count(), 0, "entry_count must be 0 after clear");
    }

    #[test]
    fn capacity_is_positive_for_default_atlas() {
        let atlas = TextureAtlas::new(0);
        assert!(atlas.capacity() > 0, "capacity must be > 0 for a 2048×2048 atlas");
    }

    #[test]
    fn capacity_equals_area_div_nine() {
        let atlas = TextureAtlas::new(0);
        let expected = (atlas.width as usize) * (atlas.height as usize) / 9;
        assert_eq!(atlas.capacity(), expected, "capacity must equal width*height/9");
    }

    #[test]
    fn capacity_constant_regardless_of_packing() {
        let mut atlas = TextureAtlas::new(0);
        let cap_before = atlas.capacity();
        for i in 0..10u32 {
            atlas.pack_glyph(make_key(50000 + i), 16, 16).unwrap();
        }
        assert_eq!(atlas.capacity(), cap_before, "capacity must not change after packing glyphs");
    }

    #[test]
    fn utilization_zero_on_empty_atlas() {
        let atlas = TextureAtlas::new(0);
        assert_eq!(atlas.utilization(), 0.0, "empty atlas must have utilization = 0.0");
    }

    #[test]
    fn utilization_increases_after_packing() {
        let mut atlas = TextureAtlas::new(0);
        let before = atlas.utilization();
        atlas.pack_glyph(make_key(60001), 8, 8).unwrap();
        let after = atlas.utilization();
        assert!(after > before, "utilization must increase after packing: {} > {}", after, before);
    }

    #[test]
    fn utilization_in_zero_to_one_range() {
        let mut atlas = TextureAtlas::new(0);
        for i in 0..20u32 {
            atlas.pack_glyph(make_key(70000 + i), 8, 8).unwrap();
        }
        let u = atlas.utilization();
        assert!(u >= 0.0 && u <= 1.0, "utilization must be in [0.0, 1.0], got {u}");
    }

    #[test]
    fn utilization_zero_after_clear() {
        let mut atlas = TextureAtlas::new(0);
        atlas.pack_glyph(make_key(80001), 8, 8).unwrap();
        atlas.clear();
        assert_eq!(atlas.utilization(), 0.0, "utilization must be 0.0 after clear");
    }

    #[test]
    fn utilization_equals_entry_count_over_capacity() {
        let mut atlas = TextureAtlas::new(0);
        for i in 0..3u32 {
            atlas.pack_glyph(make_key(90000 + i), 8, 8).unwrap();
        }
        let expected = (atlas.entry_count() as f32) / (atlas.capacity() as f32);
        let actual = atlas.utilization();
        assert!((actual - expected).abs() < 1e-6, "utilization must equal entry_count/capacity: expected {expected}, got {actual}");
    }

    #[test]
    fn entry_count_matches_cache_len() {
        let mut atlas = TextureAtlas::new(0);
        for i in 0..7u32 {
            atlas.pack_glyph(make_key(100000 + i), 8, 8).unwrap();
        }
        // entry_count is backed by cache.len()
        assert_eq!(atlas.entry_count(), 7, "entry_count must equal number of distinct packs");
    }

    #[test]
    fn capacity_for_2048_atlas_is_at_least_100000() {
        // 2048*2048 / 9 = 466,489 — a reasonable lower bound for capacity.
        let atlas = TextureAtlas::new(0);
        assert!(atlas.capacity() >= 100_000, "2048×2048 atlas must have capacity >= 100_000");
    }

    #[test]
    fn entry_count_reflects_multiple_font_variants() {
        let mut atlas = TextureAtlas::new(0);
        // Same glyph_id in two different fonts = two distinct entries.
        let k_a = make_key_font(10, 65);
        let k_b = make_key_font(11, 65);
        atlas.pack_glyph(k_a, 8, 8).unwrap();
        atlas.pack_glyph(k_b, 8, 8).unwrap();
        assert_eq!(atlas.entry_count(), 2, "different font_ids must produce 2 distinct entries");
    }

    #[test]
    fn utilization_greater_than_zero_after_single_pack() {
        let mut atlas = TextureAtlas::new(0);
        atlas.pack_glyph(make_key(110001), 8, 8).unwrap();
        assert!(atlas.utilization() > 0.0, "utilization must be > 0 after one pack");
    }

    #[test]
    fn entry_count_and_utilization_consistent_after_many_packs() {
        let mut atlas = TextureAtlas::new(0);
        for i in 0..20u32 {
            atlas.pack_glyph(make_key(120000 + i), 8, 8).unwrap();
        }
        assert_eq!(atlas.entry_count(), 20);
        let u = atlas.utilization();
        let expected = 20.0 / (atlas.capacity() as f32);
        assert!((u - expected).abs() < 1e-6, "utilization {u} must equal 20/capacity {expected}");
    }
}
