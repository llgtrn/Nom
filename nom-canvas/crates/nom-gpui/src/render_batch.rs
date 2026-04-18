/// Classifies the kind of primitives in a draw call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BatchKind {
    Quads,
    Glyphs,
    Lines,
    Images,
}

impl BatchKind {
    /// Returns `true` for geometry-class kinds (Quads and Lines).
    pub fn is_geometry(&self) -> bool {
        matches!(self, BatchKind::Quads | BatchKind::Lines)
    }

    /// Returns the byte stride of a single vertex for this kind.
    pub fn vertex_stride(&self) -> u32 {
        match self {
            BatchKind::Quads => 32,
            BatchKind::Glyphs => 24,
            BatchKind::Lines => 16,
            BatchKind::Images => 48,
        }
    }
}

/// A single GPU draw call within a batch.
#[derive(Debug, Clone)]
pub struct DrawCall {
    pub kind: BatchKind,
    pub vertex_offset: u32,
    pub vertex_count: u32,
    pub instance_count: u32,
}

impl DrawCall {
    /// Total number of vertices across all instances.
    pub fn total_vertices(&self) -> u32 {
        self.vertex_count * self.instance_count
    }

    /// Returns `true` when more than one instance is drawn.
    pub fn is_instanced(&self) -> bool {
        self.instance_count > 1
    }

    /// Total byte size consumed by this draw call.
    pub fn byte_size(&self) -> u32 {
        self.total_vertices() * self.kind.vertex_stride()
    }
}

/// An ordered collection of draw calls sharing a single batch ID.
pub struct RenderBatch {
    pub calls: Vec<DrawCall>,
    pub batch_id: u32,
}

impl RenderBatch {
    pub fn new(batch_id: u32) -> Self {
        Self {
            calls: Vec::new(),
            batch_id,
        }
    }

    pub fn add_call(&mut self, call: DrawCall) {
        self.calls.push(call);
    }

    /// Sum of byte_size across all calls.
    pub fn total_byte_size(&self) -> u32 {
        self.calls.iter().map(|c| c.byte_size()).sum()
    }

    pub fn call_count(&self) -> usize {
        self.calls.len()
    }

    /// Returns references to calls whose kind is geometry (Quads or Lines).
    pub fn geometry_calls(&self) -> Vec<&DrawCall> {
        self.calls.iter().filter(|c| c.kind.is_geometry()).collect()
    }
}

/// Utility for reordering and deduplicating slices of draw calls.
pub struct BatchSorter;

impl BatchSorter {
    /// Stable-sorts `calls` by vertex_stride ascending (lowest stride first).
    pub fn sort_by_kind(calls: &mut Vec<DrawCall>) {
        calls.sort_by_key(|c| c.kind.vertex_stride());
    }

    /// Returns the first call per unique `vertex_offset`, preserving input order.
    pub fn deduplicate_offsets<'a>(calls: &'a [DrawCall]) -> Vec<&'a DrawCall> {
        let mut seen = std::collections::HashSet::new();
        calls
            .iter()
            .filter(|c| seen.insert(c.vertex_offset))
            .collect()
    }
}

/// Aggregate statistics derived from a `RenderBatch`.
pub struct BatchStats {
    pub total_calls: u32,
    pub total_bytes: u32,
    pub geometry_ratio: f32,
}

impl BatchStats {
    pub fn from_batch(batch: &RenderBatch) -> Self {
        let total_calls = batch.call_count() as u32;
        let total_bytes = batch.total_byte_size();
        let geometry_ratio = if total_calls == 0 {
            0.0
        } else {
            batch.geometry_calls().len() as f32 / total_calls as f32
        };
        Self {
            total_calls,
            total_bytes,
            geometry_ratio,
        }
    }

    /// Returns `true` when more than half of all calls are geometry calls.
    pub fn is_geometry_heavy(&self) -> bool {
        self.geometry_ratio > 0.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 1. BatchKind::is_geometry
    #[test]
    fn test_batch_kind_is_geometry() {
        assert!(BatchKind::Quads.is_geometry());
        assert!(BatchKind::Lines.is_geometry());
        assert!(!BatchKind::Glyphs.is_geometry());
        assert!(!BatchKind::Images.is_geometry());
    }

    // 2. BatchKind::vertex_stride
    #[test]
    fn test_batch_kind_vertex_stride() {
        assert_eq!(BatchKind::Quads.vertex_stride(), 32);
        assert_eq!(BatchKind::Glyphs.vertex_stride(), 24);
        assert_eq!(BatchKind::Lines.vertex_stride(), 16);
        assert_eq!(BatchKind::Images.vertex_stride(), 48);
    }

    // 3. DrawCall::total_vertices
    #[test]
    fn test_draw_call_total_vertices() {
        let dc = DrawCall {
            kind: BatchKind::Quads,
            vertex_offset: 0,
            vertex_count: 6,
            instance_count: 4,
        };
        assert_eq!(dc.total_vertices(), 24);
    }

    // 4. DrawCall::is_instanced
    #[test]
    fn test_draw_call_is_instanced() {
        let single = DrawCall {
            kind: BatchKind::Lines,
            vertex_offset: 0,
            vertex_count: 2,
            instance_count: 1,
        };
        let multi = DrawCall {
            kind: BatchKind::Lines,
            vertex_offset: 0,
            vertex_count: 2,
            instance_count: 3,
        };
        assert!(!single.is_instanced());
        assert!(multi.is_instanced());
    }

    // 5. DrawCall::byte_size
    #[test]
    fn test_draw_call_byte_size() {
        // Glyphs: stride=24, vertex_count=5, instance_count=2 => total_vertices=10, byte_size=240
        let dc = DrawCall {
            kind: BatchKind::Glyphs,
            vertex_offset: 0,
            vertex_count: 5,
            instance_count: 2,
        };
        assert_eq!(dc.byte_size(), 240);
    }

    // 6. RenderBatch::total_byte_size
    #[test]
    fn test_render_batch_total_byte_size() {
        let mut batch = RenderBatch::new(1);
        // Quads: 6 * 1 * 32 = 192
        batch.add_call(DrawCall {
            kind: BatchKind::Quads,
            vertex_offset: 0,
            vertex_count: 6,
            instance_count: 1,
        });
        // Images: 4 * 1 * 48 = 192
        batch.add_call(DrawCall {
            kind: BatchKind::Images,
            vertex_offset: 6,
            vertex_count: 4,
            instance_count: 1,
        });
        assert_eq!(batch.total_byte_size(), 384);
    }

    // 7. RenderBatch::geometry_calls filter
    #[test]
    fn test_render_batch_geometry_calls_filter() {
        let mut batch = RenderBatch::new(2);
        batch.add_call(DrawCall {
            kind: BatchKind::Quads,
            vertex_offset: 0,
            vertex_count: 4,
            instance_count: 1,
        });
        batch.add_call(DrawCall {
            kind: BatchKind::Glyphs,
            vertex_offset: 4,
            vertex_count: 6,
            instance_count: 1,
        });
        batch.add_call(DrawCall {
            kind: BatchKind::Lines,
            vertex_offset: 10,
            vertex_count: 2,
            instance_count: 1,
        });
        let geo = batch.geometry_calls();
        assert_eq!(geo.len(), 2);
        assert!(geo.iter().all(|c| c.kind.is_geometry()));
    }

    // 8. BatchSorter::sort_by_kind order
    #[test]
    fn test_batch_sorter_sort_by_kind_order() {
        let mut calls = vec![
            DrawCall { kind: BatchKind::Images,  vertex_offset: 0, vertex_count: 1, instance_count: 1 },
            DrawCall { kind: BatchKind::Quads,   vertex_offset: 1, vertex_count: 1, instance_count: 1 },
            DrawCall { kind: BatchKind::Glyphs,  vertex_offset: 2, vertex_count: 1, instance_count: 1 },
            DrawCall { kind: BatchKind::Lines,   vertex_offset: 3, vertex_count: 1, instance_count: 1 },
        ];
        BatchSorter::sort_by_kind(&mut calls);
        let strides: Vec<u32> = calls.iter().map(|c| c.kind.vertex_stride()).collect();
        // Expected ascending order: Lines=16, Glyphs=24, Quads=32, Images=48
        assert_eq!(strides, vec![16, 24, 32, 48]);
    }

    // 9. BatchStats::from_batch geometry_ratio
    #[test]
    fn test_batch_stats_geometry_ratio() {
        let mut batch = RenderBatch::new(3);
        // 3 geometry calls (Quads, Lines, Lines) + 1 non-geometry (Glyphs) = ratio 0.75
        for kind in &[BatchKind::Quads, BatchKind::Lines, BatchKind::Lines, BatchKind::Glyphs] {
            batch.add_call(DrawCall {
                kind: *kind,
                vertex_offset: 0,
                vertex_count: 1,
                instance_count: 1,
            });
        }
        let stats = BatchStats::from_batch(&batch);
        assert_eq!(stats.total_calls, 4);
        assert!((stats.geometry_ratio - 0.75).abs() < f32::EPSILON);
        assert!(stats.is_geometry_heavy());

        // Empty batch — ratio should be 0.0
        let empty = RenderBatch::new(99);
        let empty_stats = BatchStats::from_batch(&empty);
        assert_eq!(empty_stats.geometry_ratio, 0.0);
        assert!(!empty_stats.is_geometry_heavy());
    }
}
