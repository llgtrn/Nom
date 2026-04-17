//! Line width measurement data structures — shapes + cache.
//!
//! Actual cosmic-text shaping happens in nom-gpui.  This module stores the
//! results and exposes lazy lookup with stale-snapshot protection during
//! typing (`interpolated: true` propagates from wrap_map).
#![deny(unsafe_code)]

use std::ops::Range;

/// Per-line layout snapshot.
#[derive(Clone, Debug, PartialEq)]
pub struct LineLayout {
    pub line_index: u32,
    pub width_px: f32,
    pub height_px: f32,
    pub char_x_offsets: Vec<f32>, // cumulative x per char, len = char_count + 1
    /// True if this snapshot was produced by a stale shape call (still typing);
    /// the caller should avoid scroll-snap or cursor-anchor rendering until a
    /// fresh layout lands.
    pub interpolated: bool,
}

impl LineLayout {
    pub fn new(line_index: u32) -> Self {
        Self {
            line_index,
            width_px: 0.0,
            height_px: 0.0,
            char_x_offsets: vec![0.0],
            interpolated: false,
        }
    }

    /// Add a character at the given advance width.  Convenience builder used
    /// by shaping integration tests — real shaping happens in nom-gpui.
    pub fn push_char(&mut self, advance_px: f32) {
        let last = *self
            .char_x_offsets
            .last()
            .expect("invariant: starts with 0.0");
        let new_x = last + advance_px;
        self.char_x_offsets.push(new_x);
        self.width_px = new_x;
    }

    pub fn char_count(&self) -> usize {
        self.char_x_offsets.len() - 1
    }

    /// Return the x coordinate at the given char index.  Saturates at line end.
    pub fn x_at_char(&self, char_idx: usize) -> f32 {
        let i = char_idx.min(self.char_x_offsets.len() - 1);
        self.char_x_offsets[i]
    }

    /// Return the char index whose x is closest to `x_px` (nearest-neighbour,
    /// biased left on tie).
    pub fn char_at_x(&self, x_px: f32) -> usize {
        if self.char_x_offsets.len() < 2 {
            return 0;
        }
        let mut best = 0usize;
        let mut best_dist = f32::INFINITY;
        for (i, &x) in self.char_x_offsets.iter().enumerate() {
            let d = (x - x_px).abs();
            if d < best_dist {
                best_dist = d;
                best = i;
            }
        }
        best
    }
}

/// LRU-ish cache of line layouts keyed on (line_index, buffer_version).
pub struct LayoutCache {
    entries: std::collections::HashMap<(u32, u64), LineLayout>,
    max_entries: usize,
    access_counter: u64,
    access_order: std::collections::HashMap<(u32, u64), u64>,
}

impl LayoutCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: std::collections::HashMap::new(),
            max_entries,
            access_counter: 0,
            access_order: std::collections::HashMap::new(),
        }
    }

    pub fn get(&mut self, line_index: u32, buffer_version: u64) -> Option<&LineLayout> {
        let key = (line_index, buffer_version);
        self.access_counter += 1;
        self.access_order.insert(key, self.access_counter);
        self.entries.get(&key)
    }

    pub fn insert(&mut self, buffer_version: u64, layout: LineLayout) {
        let key = (layout.line_index, buffer_version);
        self.evict_if_needed(1);
        self.access_counter += 1;
        self.access_order.insert(key, self.access_counter);
        self.entries.insert(key, layout);
    }

    pub fn invalidate_line(&mut self, line_index: u32) {
        self.entries.retain(|(l, _), _| *l != line_index);
        self.access_order.retain(|(l, _), _| *l != line_index);
    }

    pub fn invalidate_version(&mut self, buffer_version: u64) {
        self.entries.retain(|(_, v), _| *v != buffer_version);
        self.access_order.retain(|(_, v), _| *v != buffer_version);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.access_order.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    fn evict_if_needed(&mut self, incoming: usize) {
        while self.entries.len() + incoming > self.max_entries && !self.entries.is_empty() {
            if let Some((&oldest_key, _)) = self.access_order.iter().min_by_key(|(_, ac)| **ac) {
                self.entries.remove(&oldest_key);
                self.access_order.remove(&oldest_key);
            } else {
                break;
            }
        }
    }
}

/// Range-based invalidation helper: given an edit over buffer byte range,
/// return which line indices (by line_index → byte_range mapping provided) should be invalidated.
pub fn lines_affected_by_edit(
    edit: Range<usize>,
    line_ranges: impl IntoIterator<Item = (u32, Range<usize>)>,
) -> Vec<u32> {
    line_ranges
        .into_iter()
        .filter(|(_, r)| r.start < edit.end && r.end > edit.start)
        .map(|(i, _)| i)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_starts_with_zero_offset_and_zero_char_count() {
        let ll = LineLayout::new(0);
        assert_eq!(ll.char_count(), 0);
        assert_eq!(ll.char_x_offsets, vec![0.0]);
        assert_eq!(ll.width_px, 0.0);
        assert!(!ll.interpolated);
    }

    #[test]
    fn push_char_increments_char_count_and_width() {
        let mut ll = LineLayout::new(0);
        ll.push_char(10.0);
        assert_eq!(ll.char_count(), 1);
        assert_eq!(ll.width_px, 10.0);
        ll.push_char(8.0);
        assert_eq!(ll.char_count(), 2);
        assert_eq!(ll.width_px, 18.0);
    }

    #[test]
    fn char_count_equals_chars_pushed() {
        let mut ll = LineLayout::new(5);
        for _ in 0..7 {
            ll.push_char(6.0);
        }
        assert_eq!(ll.char_count(), 7);
    }

    #[test]
    fn x_at_char_hits_expected_offset() {
        let mut ll = LineLayout::new(0);
        ll.push_char(10.0);
        ll.push_char(20.0);
        assert_eq!(ll.x_at_char(0), 0.0);
        assert_eq!(ll.x_at_char(1), 10.0);
        assert_eq!(ll.x_at_char(2), 30.0);
    }

    #[test]
    fn x_at_char_saturates_past_end() {
        let mut ll = LineLayout::new(0);
        ll.push_char(10.0);
        assert_eq!(ll.x_at_char(100), 10.0);
    }

    #[test]
    fn char_at_x_nearest_neighbour() {
        let mut ll = LineLayout::new(0);
        ll.push_char(10.0); // offsets: [0, 10]
        ll.push_char(10.0); // offsets: [0, 10, 20]
        // x=4 is closer to 0 than to 10
        assert_eq!(ll.char_at_x(4.0), 0);
        // x=6 is closer to 10 than to 0
        assert_eq!(ll.char_at_x(6.0), 1);
        // x=15 is closer to 10 (|15-10|=5) than 20 (|15-20|=5) — tie biased left
        assert_eq!(ll.char_at_x(15.0), 1);
        // x=20 maps to index 2
        assert_eq!(ll.char_at_x(20.0), 2);
    }

    #[test]
    fn char_at_x_empty_line_returns_zero() {
        let ll = LineLayout::new(0);
        assert_eq!(ll.char_at_x(999.0), 0);
    }

    #[test]
    fn layout_cache_get_insert_round_trip() {
        let mut cache = LayoutCache::new(10);
        let mut ll = LineLayout::new(3);
        ll.push_char(12.0);
        cache.insert(42, ll.clone());
        let got = cache.get(3, 42).expect("should find entry");
        assert_eq!(got.line_index, 3);
        assert_eq!(got.width_px, 12.0);
    }

    #[test]
    fn layout_cache_miss_returns_none() {
        let mut cache = LayoutCache::new(10);
        assert!(cache.get(0, 99).is_none());
    }

    #[test]
    fn layout_cache_invalidate_line_drops_only_that_line() {
        let mut cache = LayoutCache::new(10);
        cache.insert(1, LineLayout::new(0));
        cache.insert(1, LineLayout::new(1));
        cache.insert(1, LineLayout::new(2));
        cache.invalidate_line(1);
        assert!(cache.get(1, 1).is_none());
        assert!(cache.get(0, 1).is_some());
        assert!(cache.get(2, 1).is_some());
    }

    #[test]
    fn layout_cache_invalidate_version_drops_only_that_version() {
        let mut cache = LayoutCache::new(10);
        cache.insert(1, LineLayout::new(0));
        cache.insert(2, LineLayout::new(0));
        cache.insert(2, LineLayout::new(1));
        cache.invalidate_version(1);
        assert!(cache.get(0, 1).is_none());
        assert!(cache.get(0, 2).is_some());
        assert!(cache.get(1, 2).is_some());
    }

    #[test]
    fn layout_cache_evicts_lru_at_capacity() {
        let mut cache = LayoutCache::new(2);
        cache.insert(1, LineLayout::new(0)); // key (0,1)
        cache.insert(1, LineLayout::new(1)); // key (1,1) — cache full
        // Access (0,1) to make (1,1) the LRU
        let _ = cache.get(0, 1);
        // Insert third entry — (1,1) should be evicted
        cache.insert(1, LineLayout::new(2)); // key (2,1)
        assert_eq!(cache.len(), 2);
        assert!(cache.get(1, 1).is_none(), "(1,1) should have been evicted");
        assert!(cache.get(0, 1).is_some());
        assert!(cache.get(2, 1).is_some());
    }

    #[test]
    fn lines_affected_by_edit_matches_overlapping_ranges_only() {
        let line_ranges = vec![
            (0u32, 0usize..10usize),
            (1u32, 10usize..20usize),
            (2u32, 20usize..30usize),
        ];
        let mut affected = lines_affected_by_edit(12..18, line_ranges);
        affected.sort();
        assert_eq!(affected, vec![1]);
    }

    #[test]
    fn lines_affected_by_edit_spans_multiple_lines() {
        let line_ranges = vec![
            (0u32, 0usize..10usize),
            (1u32, 10usize..20usize),
            (2u32, 20usize..30usize),
        ];
        let mut affected = lines_affected_by_edit(5..25, line_ranges);
        affected.sort();
        assert_eq!(affected, vec![0, 1, 2]);
    }
}
