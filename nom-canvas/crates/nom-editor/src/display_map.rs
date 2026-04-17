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

    /// Apply folds to a raw text slice, replacing folded ranges with '…'.
    pub fn fold_text(&self, text: &str) -> String {
        if self.folds.is_empty() {
            return text.to_string();
        }
        let mut sorted_folds = self.folds.clone();
        sorted_folds.sort_by_key(|f| f.buffer_range.start);
        let mut result = String::new();
        let mut pos = 0usize;
        for fold in &sorted_folds {
            let start = fold.buffer_range.start;
            let end = fold.buffer_range.end;
            if start > pos {
                let slice_end = start.min(text.len());
                result.push_str(&text[pos..slice_end]);
            }
            if start < text.len() {
                result.push('\u{2026}'); // fold placeholder '…'
            }
            pos = end.min(text.len());
        }
        if pos < text.len() {
            result.push_str(&text[pos..]);
        }
        result
    }

    /// Convert buffer offset to display position (row, col), accounting for folds and tabs.
    pub fn buffer_to_display(&self, buffer: &Buffer, offset: usize) -> (usize, usize) {
        let raw = buffer.text_for_range(0..offset.min(buffer.len()));
        let text = self.fold_text(&raw);
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
