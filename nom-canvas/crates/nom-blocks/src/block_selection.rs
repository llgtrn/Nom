use crate::block_model::BlockId;

/// Granular selection of content inside the canvas.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockSelection {
    /// A character-range within a text block.
    Text {
        block_id: BlockId,
        /// Inclusive start offset (Unicode scalar index).
        start: u32,
        /// Exclusive end offset.
        end: u32,
    },
    /// Whole-block selection (e.g. non-text blocks).
    Block { block_id: BlockId },
    /// Selection of (an optional crop region within) an image block.
    Image {
        block_id: BlockId,
        /// Optional `(x, y, width, height)` in normalised [0,1] coordinates.
        region: Option<(f32, f32, f32, f32)>,
    },
    /// A cell within a table/database block.
    Database {
        block_id: BlockId,
        row: u32,
        col: u32,
    },
}

/// An ordered set of active selections.
#[derive(Debug, Clone, Default)]
pub struct SelectionSet(pub Vec<BlockSelection>);

impl SelectionSet {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, sel: BlockSelection) {
        self.0.push(sel);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns `true` if any selection references `id`.
    pub fn contains_block(&self, id: BlockId) -> bool {
        self.0.iter().any(|s| match s {
            BlockSelection::Text { block_id, .. } => *block_id == id,
            BlockSelection::Block { block_id } => *block_id == id,
            BlockSelection::Image { block_id, .. } => *block_id == id,
            BlockSelection::Database { block_id, .. } => *block_id == id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_set_is_empty() {
        let s = SelectionSet::new();
        assert!(s.is_empty());
        assert!(!s.contains_block(1));
    }

    #[test]
    fn push_then_contains() {
        let mut s = SelectionSet::new();
        s.push(BlockSelection::Block { block_id: 42 });
        assert!(!s.is_empty());
        assert!(s.contains_block(42));
        assert!(!s.contains_block(99));
    }

    #[test]
    fn distinct_variants_filtered_correctly() {
        let mut s = SelectionSet::new();
        s.push(BlockSelection::Text { block_id: 1, start: 0, end: 5 });
        s.push(BlockSelection::Image { block_id: 2, region: None });
        s.push(BlockSelection::Database { block_id: 3, row: 0, col: 0 });

        // only text blocks
        let text_ids: Vec<BlockId> = s
            .0
            .iter()
            .filter_map(|sel| {
                if let BlockSelection::Text { block_id, .. } = sel {
                    Some(*block_id)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(text_ids, vec![1]);

        // contains_block is variant-agnostic
        assert!(s.contains_block(1));
        assert!(s.contains_block(2));
        assert!(s.contains_block(3));
        assert!(!s.contains_block(4));
    }
}
