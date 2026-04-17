//! Syntax-aware range tracking (tree-sitter-compatible layered view).
//!
//! Minimal viable implementation: a flat Vec of SyntaxLayerEntry sorted by
//! byte offset. Full SumTree from Zed is deferred; the interface models it
//! so later swap-in is trivial.
#![deny(unsafe_code)]

use std::ops::Range;

pub type LayerId = u64;

#[derive(Clone, Debug, PartialEq)]
pub struct SyntaxLayerEntry {
    pub layer_id: LayerId,
    pub language: String,
    pub byte_range: Range<usize>,
    pub version: u64,
}

#[derive(Default)]
pub struct SyntaxMap {
    layers: Vec<SyntaxLayerEntry>,
    next_id: LayerId,
    next_version: u64,
}

impl SyntaxMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_layer(&mut self, language: impl Into<String>, byte_range: Range<usize>) -> LayerId {
        let id = self.next_id;
        self.next_id += 1;
        self.layers.push(SyntaxLayerEntry {
            layer_id: id,
            language: language.into(),
            byte_range,
            version: self.next_version,
        });
        self.next_version += 1;
        id
    }

    pub fn remove_layer(&mut self, id: LayerId) -> bool {
        if let Some(pos) = self.layers.iter().position(|l| l.layer_id == id) {
            self.layers.remove(pos);
            true
        } else {
            false
        }
    }

    /// Return all layer IDs whose byte_range overlaps `range`.
    pub fn layers_intersecting(&self, range: Range<usize>) -> Vec<LayerId> {
        self.layers
            .iter()
            .filter(|l| l.byte_range.start < range.end && l.byte_range.end > range.start)
            .map(|l| l.layer_id)
            .collect()
    }

    /// Called after a buffer edit. Shift affected layers' byte_range to match
    /// the new offsets. Reparse counter bumped for any layer whose range
    /// contains the edit (signals the caller to queue a tree-sitter re-parse).
    ///
    /// Returns the set of layer IDs that need to be re-parsed.
    pub fn on_edit(&mut self, edit_range: Range<usize>, new_len: usize) -> Vec<LayerId> {
        let old_len = edit_range.end - edit_range.start;
        let delta: i64 = new_len as i64 - old_len as i64;
        let mut needs_reparse = Vec::new();

        for layer in &mut self.layers {
            let overlaps = layer.byte_range.start < edit_range.end
                && layer.byte_range.end > edit_range.start;

            if overlaps {
                layer.version += 1;
                needs_reparse.push(layer.layer_id);
            }

            // Shift ranges that start at or after edit_range.end
            if layer.byte_range.start >= edit_range.end {
                layer.byte_range = (layer.byte_range.start as i64 + delta) as usize
                    ..(layer.byte_range.end as i64 + delta) as usize;
            } else if layer.byte_range.end > edit_range.end {
                // Partially overlapping: only shift end
                layer.byte_range =
                    layer.byte_range.start..(layer.byte_range.end as i64 + delta) as usize;
            } else if overlaps && layer.byte_range.end <= edit_range.end {
                // Fully inside edit: clamp end
                let new_end = if new_len > 0 {
                    layer.byte_range.start + new_len
                } else {
                    layer.byte_range.start
                };
                layer.byte_range = layer.byte_range.start..new_end.max(layer.byte_range.start);
            }
        }

        needs_reparse
    }

    pub fn layer(&self, id: LayerId) -> Option<&SyntaxLayerEntry> {
        self.layers.iter().find(|l| l.layer_id == id)
    }

    pub fn len(&self) -> usize {
        self.layers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_empty() {
        let map = SyntaxMap::new();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn insert_returns_unique_ids() {
        let mut map = SyntaxMap::new();
        let a = map.insert_layer("rust", 0..100);
        let b = map.insert_layer("nom", 50..200);
        assert_ne!(a, b);
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn layers_intersecting_returns_only_overlapping() {
        let mut map = SyntaxMap::new();
        let a = map.insert_layer("rust", 0..50);
        let _b = map.insert_layer("nom", 100..200);
        let c = map.insert_layer("markdown", 30..80);

        let hits = map.layers_intersecting(40..60);
        assert!(hits.contains(&a));
        assert!(hits.contains(&c));
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn layers_intersecting_empty_query_returns_empty() {
        let mut map = SyntaxMap::new();
        map.insert_layer("rust", 10..50);
        // empty range at 5 — before any layer
        let hits = map.layers_intersecting(5..5);
        assert!(hits.is_empty());
    }

    #[test]
    fn on_edit_shifts_ranges_after_edit() {
        let mut map = SyntaxMap::new();
        let _a = map.insert_layer("rust", 0..10);
        let b = map.insert_layer("nom", 20..40);
        // insert 5 bytes at offset 15 (after layer a, before layer b)
        map.on_edit(15..15, 5);
        let layer_b = map.layer(b).unwrap();
        assert_eq!(layer_b.byte_range, 25..45);
    }

    #[test]
    fn on_edit_reports_reparse_needed_for_affected_layer() {
        let mut map = SyntaxMap::new();
        let a = map.insert_layer("rust", 0..50);
        let _b = map.insert_layer("nom", 100..200);
        let needs = map.on_edit(20..30, 5);
        assert!(needs.contains(&a));
        assert_eq!(needs.len(), 1);
    }

    #[test]
    fn on_edit_shrinking_edit_shrinks_ranges() {
        let mut map = SyntaxMap::new();
        let b = map.insert_layer("nom", 20..40);
        // replace 10 bytes (20..30) with 5 bytes: delta = -5
        map.on_edit(20..30, 5);
        let layer_b = map.layer(b).unwrap();
        // end shifts by -5: 40 - 5 = 35
        assert_eq!(layer_b.byte_range.end, 35);
    }

    #[test]
    fn remove_layer_true_on_hit_false_on_miss() {
        let mut map = SyntaxMap::new();
        let id = map.insert_layer("rust", 0..100);
        assert!(map.remove_layer(id));
        assert!(!map.remove_layer(id));
        assert!(map.is_empty());
    }
}
