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
        Self {
            tab_size,
            folds: Vec::new(),
        }
    }

    pub fn add_fold(&mut self, range: Range<usize>, placeholder: impl Into<String>) {
        self.folds.push(FoldRegion {
            buffer_range: range,
            placeholder: placeholder.into(),
        });
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
            if ch == '\n' {
                row += 1;
                col = 0;
            } else if ch == '\t' {
                col += self.tab_size - (col % self.tab_size);
            } else {
                col += 1;
            }
        }
        (row, col)
    }
}

// ---------------------------------------------------------------------------
// Line-based fold/wrap map
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FoldState {
    Expanded,
    Collapsed,
}

/// A line-range fold region for the line-based display map.
#[derive(Debug, Clone)]
pub struct LineFoldRegion {
    pub start_line: u32,
    pub end_line: u32,
    pub state: FoldState,
    pub summary: String,
}

impl LineFoldRegion {
    pub fn new(start_line: u32, end_line: u32, summary: &str) -> Self {
        Self {
            start_line,
            end_line,
            state: FoldState::Expanded,
            summary: summary.to_string(),
        }
    }

    pub fn toggle(mut self) -> Self {
        self.state = match self.state {
            FoldState::Expanded => FoldState::Collapsed,
            FoldState::Collapsed => FoldState::Expanded,
        };
        self
    }

    pub fn line_count(&self) -> u32 {
        self.end_line - self.start_line + 1
    }

    pub fn is_collapsed(&self) -> bool {
        self.state == FoldState::Collapsed
    }
}

/// Line-based display map: tracks fold regions and optional wrap width.
pub struct LineDisplayMap {
    pub folds: Vec<LineFoldRegion>,
    pub wrap_width: Option<u32>,
}

impl LineDisplayMap {
    pub fn new() -> Self {
        Self {
            folds: Vec::new(),
            wrap_width: None,
        }
    }

    pub fn add_fold(mut self, fold: LineFoldRegion) -> Self {
        self.folds.push(fold);
        self
    }

    pub fn toggle_fold(mut self, start_line: u32) -> Self {
        for fold in &mut self.folds {
            if fold.start_line == start_line {
                fold.state = match fold.state {
                    FoldState::Expanded => FoldState::Collapsed,
                    FoldState::Collapsed => FoldState::Expanded,
                };
                break;
            }
        }
        self
    }

    pub fn collapsed_regions(&self) -> Vec<&LineFoldRegion> {
        self.folds.iter().filter(|f| f.is_collapsed()).collect()
    }

    /// Total visible lines = total_lines minus hidden lines from collapsed folds.
    /// Each collapsed fold hides (line_count - 1) lines (the first line remains visible as header).
    pub fn visible_line_count(&self, total_lines: u32) -> u32 {
        let hidden: u32 = self
            .folds
            .iter()
            .filter(|f| f.is_collapsed())
            .map(|f| f.line_count() - 1)
            .sum();
        total_lines.saturating_sub(hidden)
    }
}

impl Default for LineDisplayMap {
    fn default() -> Self {
        Self::new()
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

    #[test]
    fn display_map_maps_offset_to_row_col() {
        let dm = DisplayMap::new(4);
        let buf = Buffer::new(1, "abc\ndef\nghi");
        // offset 0 = 'a' on row 0, col 0
        let (row, col) = dm.buffer_to_display(&buf, 0);
        assert_eq!((row, col), (0, 0));
        // offset 4 = 'd' on row 1, col 0
        let (row, col) = dm.buffer_to_display(&buf, 4);
        assert_eq!((row, col), (1, 0));
        // offset 6 = 'f' on row 1, col 2
        let (row, col) = dm.buffer_to_display(&buf, 6);
        assert_eq!((row, col), (1, 2));
    }

    #[test]
    fn display_map_handles_wrapping() {
        // Tab expansion: a tab at col 0 with tab_size 4 should advance col to 4
        let dm = DisplayMap::new(4);
        let buf = Buffer::new(1, "\thello");
        let (row, col) = dm.buffer_to_display(&buf, 2); // after '\t' + 'h'
        assert_eq!(row, 0);
        assert_eq!(col, 5); // 4 (tab) + 1 (h)
    }

    #[test]
    fn display_map_handles_empty_buffer() {
        let dm = DisplayMap::new(4);
        let buf = Buffer::new(1, "");
        let (row, col) = dm.buffer_to_display(&buf, 0);
        assert_eq!((row, col), (0, 0));
        assert!(buf.is_empty());
    }

    #[test]
    fn display_map_fold_text_replaces_range() {
        let mut dm = DisplayMap::new(4);
        dm.add_fold(5..10, "…");
        let folded = dm.fold_text("hello world!");
        // characters 5..10 (" worl") replaced by the fold placeholder
        assert!(folded.contains('\u{2026}'));
        assert!(!folded.contains("worl"));
    }

    #[test]
    fn display_map_remove_fold_restores_text() {
        let mut dm = DisplayMap::new(4);
        let range = 5..10;
        dm.add_fold(range.clone(), "…");
        dm.remove_fold(&range);
        let text = "hello world!";
        assert_eq!(dm.fold_text(text), text);
    }

    #[test]
    fn display_map_empty_rope() {
        let dm = DisplayMap::new(4);
        let buf = Buffer::new(1, "");
        // An empty buffer has no newlines, so row stays 0 and col stays 0
        let (row, col) = dm.buffer_to_display(&buf, 0);
        assert_eq!((row, col), (0, 0));
    }

    #[test]
    fn display_map_single_line() {
        let dm = DisplayMap::new(4);
        let buf = Buffer::new(1, "hello");
        // Scanning all 5 chars, no newline encountered — still row 0
        let (row, _col) = dm.buffer_to_display(&buf, buf.len());
        assert_eq!(row, 0);
    }

    #[test]
    fn display_map_two_lines() {
        let dm = DisplayMap::new(4);
        let buf = Buffer::new(1, "hello\nworld");
        // After the newline we are on row 1
        let (row, _col) = dm.buffer_to_display(&buf, buf.len());
        assert_eq!(row, 1);
    }

    #[test]
    fn display_map_line_text_first_line() {
        let dm = DisplayMap::new(4);
        let buf = Buffer::new(1, "hello\n");
        // Position 0..5 should yield col 5, still row 0
        let (row, col) = dm.buffer_to_display(&buf, 5);
        assert_eq!(row, 0);
        assert_eq!(col, 5);
    }

    #[test]
    fn display_map_line_count_matches_newlines() {
        let dm = DisplayMap::new(4);
        let text = "line1\nline2\nline3";
        let buf = Buffer::new(1, text);
        // After scanning all text there should be 2 newlines → row 2
        let (row, _col) = dm.buffer_to_display(&buf, buf.len());
        let newline_count = text.chars().filter(|&c| c == '\n').count();
        assert_eq!(row, newline_count);
    }

    // --- LineFoldRegion + LineDisplayMap tests ---

    #[test]
    fn fold_region_toggle() {
        let region = LineFoldRegion::new(0, 5, "...");
        assert_eq!(region.state, FoldState::Expanded);
        let collapsed = region.toggle();
        assert_eq!(collapsed.state, FoldState::Collapsed);
        assert!(collapsed.is_collapsed());
        let expanded = collapsed.toggle();
        assert_eq!(expanded.state, FoldState::Expanded);
        assert!(!expanded.is_collapsed());
    }

    #[test]
    fn fold_region_line_count() {
        let region = LineFoldRegion::new(2, 7, "...");
        assert_eq!(region.line_count(), 6); // 7 - 2 + 1
    }

    #[test]
    fn display_map_add_fold() {
        let map = LineDisplayMap::new()
            .add_fold(LineFoldRegion::new(0, 3, "fold A"))
            .add_fold(LineFoldRegion::new(5, 8, "fold B"));
        assert_eq!(map.folds.len(), 2);
    }

    #[test]
    fn display_map_visible_line_count() {
        // 20 total lines, one collapsed fold covering lines 2..6 (5 lines → hides 4)
        let map = LineDisplayMap::new().add_fold({
            let mut r = LineFoldRegion::new(2, 6, "...");
            r.state = FoldState::Collapsed;
            r
        });
        assert_eq!(map.visible_line_count(20), 16); // 20 - 4
                                                    // No collapsed folds — all lines visible
        let map2 = LineDisplayMap::new().add_fold(LineFoldRegion::new(0, 5, "..."));
        assert_eq!(map2.visible_line_count(20), 20);
    }
}
