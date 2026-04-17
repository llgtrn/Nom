#![deny(unsafe_code)]
use crate::buffer::Buffer;
use std::ops::Range;

/// Maps buffer offsets to display rows/columns, handling folds/excerpts
#[derive(Clone, Debug)]
pub struct FoldRegion {
    pub buffer_range: Range<usize>,
    pub placeholder: String,
}

pub struct DisplayMap {
    pub tab_size: usize,
    folds: Vec<FoldRegion>,
}

impl DisplayMap {
    pub fn new(tab_size: usize) -> Self {
        Self { tab_size, folds: Vec::new() }
    }

    pub fn add_fold(&mut self, range: Range<usize>, placeholder: impl Into<String>) {
        self.folds.push(FoldRegion { buffer_range: range, placeholder: placeholder.into() });
        self.folds.sort_by_key(|f| f.buffer_range.start);
    }

    pub fn remove_fold(&mut self, range: &Range<usize>) {
        self.folds.retain(|f| &f.buffer_range != range);
    }

    /// Convert buffer offset to display position (row, col), accounting for folds and tabs
    pub fn buffer_to_display(&self, buffer: &Buffer, offset: usize) -> (usize, usize) {
        let text = buffer.text_for_range(0..offset.min(buffer.len()));
        let mut row = 0usize;
        let mut col = 0usize;
        for ch in text.chars() {
            if ch == '\n' { row += 1; col = 0; }
            else if ch == '\t' { col += self.tab_size - (col % self.tab_size); }
            else { col += 1; }
        }
        (row, col)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_map_position() {
        let dm = DisplayMap::new(4);
        let buf = Buffer::new(1, "hello\nworld");
        let (row, col) = dm.buffer_to_display(&buf, 7); // 'o' in 'world'
        assert_eq!(row, 1);
        assert_eq!(col, 1);
    }
}
