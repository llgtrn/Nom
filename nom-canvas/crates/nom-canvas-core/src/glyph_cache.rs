//! Glyph cache: key hashing, metrics, cached entries, LRU-free cache with hit/miss tracking,
//! and a stub rasterizer for populating the cache with synthetic glyphs.

use std::collections::HashMap;

/// Identifies a single glyph rendition by codepoint, pixel size, and font.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphKey {
    /// Unicode codepoint.
    pub codepoint: u32,
    /// Render size in pixels.
    pub size_px: u16,
    /// Font identifier.
    pub font_id: u32,
}

impl GlyphKey {
    /// Returns a packed u64 cache key.
    ///
    /// Formula: `codepoint * 1_000_000 + font_id * 1000 + size_px`
    pub fn cache_key(&self) -> u64 {
        self.codepoint as u64 * 1_000_000
            + self.font_id as u64 * 1000
            + self.size_px as u64
    }

    /// Returns `true` if the codepoint is in the ASCII range (0–127).
    pub fn is_ascii(&self) -> bool {
        self.codepoint < 128
    }
}

/// Typographic metrics for a rendered glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphMetrics {
    /// Glyph bitmap width in pixels.
    pub width: u16,
    /// Glyph bitmap height in pixels.
    pub height: u16,
    /// Horizontal advance in pixels (cursor advance after this glyph).
    pub advance: u16,
    /// Horizontal bearing (left-side bearing), signed.
    pub bearing_x: i16,
}

impl GlyphMetrics {
    /// Returns the pixel area of the glyph bitmap.
    pub fn area(&self) -> u32 {
        self.width as u32 * self.height as u32
    }

    /// Returns `true` when the glyph has no visible pixels (e.g. space).
    pub fn is_whitespace(&self) -> bool {
        self.width == 0 && self.height == 0
    }
}

/// A glyph that has been rasterized and placed in a texture atlas.
#[derive(Debug, Clone)]
pub struct CachedGlyph {
    /// The key that uniquely identifies this glyph rendition.
    pub key: GlyphKey,
    /// Typographic metrics.
    pub metrics: GlyphMetrics,
    /// Top-left X coordinate inside the texture atlas.
    pub atlas_x: u16,
    /// Top-left Y coordinate inside the texture atlas.
    pub atlas_y: u16,
}

impl CachedGlyph {
    /// Returns the (x, y) origin of this glyph inside the texture atlas.
    pub fn uv_origin(&self) -> (u16, u16) {
        (self.atlas_x, self.atlas_y)
    }

    /// Returns `true` when the glyph has visible pixels and can be drawn.
    pub fn is_renderable(&self) -> bool {
        !self.metrics.is_whitespace()
    }
}

/// In-memory glyph cache with hit/miss counters.
pub struct GlyphCache {
    /// Stored glyphs keyed by `GlyphKey::cache_key()`.
    pub entries: HashMap<u64, CachedGlyph>,
    /// Total number of successful lookups.
    pub hit_count: u64,
    /// Total number of failed lookups.
    pub miss_count: u64,
}

impl GlyphCache {
    /// Creates an empty cache.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            hit_count: 0,
            miss_count: 0,
        }
    }

    /// Inserts a glyph into the cache.
    pub fn insert(&mut self, glyph: CachedGlyph) {
        let k = glyph.key.cache_key();
        self.entries.insert(k, glyph);
    }

    /// Looks up a glyph by key, incrementing hit or miss counters accordingly.
    pub fn get(&mut self, key: &GlyphKey) -> Option<&CachedGlyph> {
        let k = key.cache_key();
        if self.entries.contains_key(&k) {
            self.hit_count += 1;
            self.entries.get(&k)
        } else {
            self.miss_count += 1;
            None
        }
    }

    /// Returns the hit rate as a value in `[0.0, 1.0]`.
    ///
    /// Returns `0.0` when neither hits nor misses have been recorded.
    pub fn hit_rate(&self) -> f64 {
        let total = self.hit_count + self.miss_count;
        if total == 0 {
            0.0
        } else {
            self.hit_count as f64 / total as f64
        }
    }
}

impl Default for GlyphCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Produces synthetic (stub) glyphs for testing and warm-up purposes.
pub struct GlyphRasterizer;

impl GlyphRasterizer {
    /// Creates a stub `CachedGlyph` with simple proportional metrics.
    ///
    /// - `width`  = `size_px / 2`
    /// - `height` = `size_px`
    /// - `advance` = `size_px / 2 + 1`
    /// - `bearing_x` = `0`
    /// - Atlas origin = `(0, 0)`
    pub fn make_stub(codepoint: u32, size_px: u16, font_id: u32) -> CachedGlyph {
        let key = GlyphKey { codepoint, size_px, font_id };
        let metrics = GlyphMetrics {
            width: size_px / 2,
            height: size_px,
            advance: size_px / 2 + 1,
            bearing_x: 0,
        };
        CachedGlyph { key, metrics, atlas_x: 0, atlas_y: 0 }
    }

    /// Inserts stub glyphs for all printable ASCII codepoints (32–126) into `cache`.
    pub fn rasterize_ascii_range(cache: &mut GlyphCache, font_id: u32, size_px: u16) {
        for cp in 32u32..=126 {
            cache.insert(Self::make_stub(cp, size_px, font_id));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── GlyphKey ────────────────────────────────────────────────────────────

    #[test]
    fn glyph_key_cache_key_is_unique_for_different_inputs() {
        let a = GlyphKey { codepoint: 65, size_px: 16, font_id: 1 };
        let b = GlyphKey { codepoint: 66, size_px: 16, font_id: 1 };
        let c = GlyphKey { codepoint: 65, size_px: 32, font_id: 1 };
        let d = GlyphKey { codepoint: 65, size_px: 16, font_id: 2 };
        // All four keys must produce distinct values.
        let keys = [a.cache_key(), b.cache_key(), c.cache_key(), d.cache_key()];
        let unique: std::collections::HashSet<u64> = keys.iter().copied().collect();
        assert_eq!(unique.len(), 4, "cache keys must be unique across differing inputs");
    }

    #[test]
    fn glyph_key_is_ascii_boundary() {
        let ascii = GlyphKey { codepoint: 127, size_px: 16, font_id: 0 };
        let non_ascii = GlyphKey { codepoint: 128, size_px: 16, font_id: 0 };
        let zero = GlyphKey { codepoint: 0, size_px: 16, font_id: 0 };
        assert!(ascii.is_ascii(), "codepoint 127 must be ASCII");
        assert!(!non_ascii.is_ascii(), "codepoint 128 must NOT be ASCII");
        assert!(zero.is_ascii(), "codepoint 0 must be ASCII");
    }

    // ── GlyphMetrics ────────────────────────────────────────────────────────

    #[test]
    fn glyph_metrics_area_is_width_times_height() {
        let m = GlyphMetrics { width: 10, height: 20, advance: 12, bearing_x: 1 };
        assert_eq!(m.area(), 200);
    }

    #[test]
    fn glyph_metrics_is_whitespace_requires_both_zero() {
        let ws = GlyphMetrics { width: 0, height: 0, advance: 8, bearing_x: 0 };
        let not_ws = GlyphMetrics { width: 0, height: 10, advance: 8, bearing_x: 0 };
        assert!(ws.is_whitespace());
        assert!(!not_ws.is_whitespace());
    }

    // ── CachedGlyph ─────────────────────────────────────────────────────────

    #[test]
    fn cached_glyph_is_renderable_for_non_whitespace() {
        let key = GlyphKey { codepoint: 65, size_px: 16, font_id: 0 };
        let metrics = GlyphMetrics { width: 8, height: 16, advance: 9, bearing_x: 0 };
        let glyph = CachedGlyph { key, metrics, atlas_x: 0, atlas_y: 0 };
        assert!(glyph.is_renderable(), "non-whitespace glyph must be renderable");

        let ws_metrics = GlyphMetrics { width: 0, height: 0, advance: 8, bearing_x: 0 };
        let ws_glyph = CachedGlyph { key, metrics: ws_metrics, atlas_x: 0, atlas_y: 0 };
        assert!(!ws_glyph.is_renderable(), "whitespace glyph must NOT be renderable");
    }

    #[test]
    fn cached_glyph_uv_origin_returns_atlas_coords() {
        let key = GlyphKey { codepoint: 65, size_px: 16, font_id: 0 };
        let metrics = GlyphMetrics { width: 8, height: 16, advance: 9, bearing_x: 0 };
        let glyph = CachedGlyph { key, metrics, atlas_x: 32, atlas_y: 64 };
        assert_eq!(glyph.uv_origin(), (32, 64));
    }

    // ── GlyphCache ──────────────────────────────────────────────────────────

    #[test]
    fn glyph_cache_insert_and_get_increments_hit() {
        let mut cache = GlyphCache::new();
        let key = GlyphKey { codepoint: 65, size_px: 16, font_id: 0 };
        let metrics = GlyphMetrics { width: 8, height: 16, advance: 9, bearing_x: 0 };
        cache.insert(CachedGlyph { key, metrics, atlas_x: 0, atlas_y: 0 });

        let result = cache.get(&key);
        assert!(result.is_some(), "inserted glyph must be retrievable");
        assert_eq!(cache.hit_count, 1);
        assert_eq!(cache.miss_count, 0);
    }

    #[test]
    fn glyph_cache_missing_key_increments_miss() {
        let mut cache = GlyphCache::new();
        let key = GlyphKey { codepoint: 999, size_px: 16, font_id: 0 };
        let result = cache.get(&key);
        assert!(result.is_none());
        assert_eq!(cache.miss_count, 1);
        assert_eq!(cache.hit_count, 0);
    }

    #[test]
    fn glyph_cache_hit_rate_reflects_ratio() {
        let mut cache = GlyphCache::new();
        assert_eq!(cache.hit_rate(), 0.0, "empty cache must return 0.0");

        let key = GlyphKey { codepoint: 65, size_px: 16, font_id: 0 };
        let metrics = GlyphMetrics { width: 8, height: 16, advance: 9, bearing_x: 0 };
        cache.insert(CachedGlyph { key, metrics, atlas_x: 0, atlas_y: 0 });

        // 1 hit
        cache.get(&key);
        // 1 miss
        let missing = GlyphKey { codepoint: 66, size_px: 16, font_id: 0 };
        cache.get(&missing);

        let rate = cache.hit_rate();
        assert!(
            (rate - 0.5).abs() < f64::EPSILON,
            "1 hit + 1 miss must give hit_rate = 0.5, got {}",
            rate
        );
    }
}
