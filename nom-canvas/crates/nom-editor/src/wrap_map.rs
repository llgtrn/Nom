//! Soft-wrap layer: splits long lines into display rows of bounded width.
#![deny(unsafe_code)]

use std::ops::Range;

// TODO: unify with display_map BufferOffset when both land.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferOffset(pub usize);

// TODO: unify with display_map DisplayOffset when both land.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplayOffset(pub usize);

#[derive(Clone, Debug, PartialEq)]
pub struct WrapConfig {
    /// Target visual width in columns (approximate; measured by cosmic-text in prod).
    pub wrap_col: u32,
    /// True = break on word boundaries, False = anywhere-break for fill.
    pub word_break: bool,
}

impl Default for WrapConfig {
    fn default() -> Self {
        Self {
            wrap_col: 80,
            word_break: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WrapRow {
    pub byte_range: Range<usize>,
    pub display_row: u32,
    /// True if this row was re-wrapped asynchronously and the snapshot may
    /// drift while the background computation completes.  Matches Zed's
    /// `interpolated: true` flag during flight.
    pub interpolated: bool,
}

pub struct WrapMap {
    config: WrapConfig,
    rows: Vec<WrapRow>,
}

impl WrapMap {
    pub fn new(config: WrapConfig) -> Self {
        Self {
            config,
            rows: Vec::new(),
        }
    }

    /// Re-wrap an entire source, returning the rows.  Simple word-boundary
    /// algorithm: scan ASCII whitespace for break points; emit a row whose
    /// byte_range advances up to wrap_col columns (counted in chars).
    pub fn wrap(&mut self, source: &str) -> &[WrapRow] {
        self.rows.clear();
        if source.is_empty() {
            return &self.rows;
        }

        let wrap_col = self.config.wrap_col as usize;
        let word_break = self.config.word_break;
        let mut display_row: u32 = 0;
        let mut row_start = 0usize; // byte offset

        // Work in chars; track corresponding byte positions.
        let char_indices: Vec<(usize, char)> = source.char_indices().collect();
        let total_chars = char_indices.len();
        let mut char_pos = 0usize; // index into char_indices

        while char_pos < total_chars {
            // Determine the end of this wrap window (up to wrap_col chars).
            let window_end_char = (char_pos + wrap_col).min(total_chars);

            if window_end_char == total_chars {
                // Last segment — emit remainder as final row.
                let byte_end = source.len();
                self.rows.push(WrapRow {
                    byte_range: row_start..byte_end,
                    display_row,
                    interpolated: false,
                });
                break;
            }

            // Find break point.
            let break_char = if word_break {
                // Search backwards from window_end_char for ASCII whitespace.
                let mut bp = window_end_char;
                while bp > char_pos {
                    if char_indices[bp - 1].1.is_ascii_whitespace() {
                        break;
                    }
                    bp -= 1;
                }
                // If no whitespace found, fall back to anywhere-break.
                if bp == char_pos { window_end_char } else { bp }
            } else {
                window_end_char
            };

            let byte_end = char_indices[break_char].0;
            self.rows.push(WrapRow {
                byte_range: row_start..byte_end,
                display_row,
                interpolated: false,
            });
            display_row += 1;

            // Skip any leading whitespace on next row when word_break is true.
            let mut next_char = break_char;
            if word_break {
                while next_char < total_chars
                    && char_indices[next_char].1.is_ascii_whitespace()
                {
                    next_char += 1;
                }
            }

            if next_char >= total_chars {
                break;
            }

            row_start = char_indices[next_char].0;
            char_pos = next_char;
        }

        &self.rows
    }

    /// Return the number of display rows.
    pub fn row_count(&self) -> u32 {
        self.rows.len() as u32
    }

    /// Return the WrapRow containing a given buffer offset, or None.
    pub fn row_for_offset(&self, offset: usize) -> Option<&WrapRow> {
        self.rows.iter().find(|r| r.byte_range.contains(&offset))
    }

    /// Start an async re-wrap (marks existing rows interpolated=true in prod;
    /// in this skeleton we just set the flag without changing contents).
    pub fn begin_interpolated_rewrap(&mut self) {
        for r in &mut self.rows {
            r.interpolated = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_wrap_col_is_80() {
        let cfg = WrapConfig::default();
        assert_eq!(cfg.wrap_col, 80);
        assert!(cfg.word_break);
    }

    #[test]
    fn new_has_empty_rows() {
        let wm = WrapMap::new(WrapConfig::default());
        assert_eq!(wm.row_count(), 0);
    }

    #[test]
    fn wrap_empty_string() {
        let mut wm = WrapMap::new(WrapConfig::default());
        let rows = wm.wrap("");
        assert!(rows.is_empty());
    }

    #[test]
    fn wrap_short_line_single_row() {
        let mut wm = WrapMap::new(WrapConfig::default());
        let rows = wm.wrap("hello world");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].byte_range, 0..11);
    }

    #[test]
    fn wrap_long_line_word_break() {
        // "hello world" with wrap_col=5 and word_break=true
        // chars: h e l l o   w o r l d
        // first 5 chars = "hello", look back for space → not found → anywhere break at 5
        // next starts at 'world' (after skipping space)
        let mut wm = WrapMap::new(WrapConfig {
            wrap_col: 5,
            word_break: true,
        });
        let rows = wm.wrap("hello world");
        assert!(rows.len() >= 2, "expected at least 2 rows, got {}", rows.len());
    }

    #[test]
    fn wrap_long_line_no_word_break() {
        // "hello world" with wrap_col=5 and word_break=false → breaks anywhere at col 5
        let mut wm = WrapMap::new(WrapConfig {
            wrap_col: 5,
            word_break: false,
        });
        let rows = wm.wrap("hello world");
        assert!(rows.len() >= 2, "expected at least 2 rows, got {}", rows.len());
        // First row should be exactly 5 chars = "hello"
        assert_eq!(rows[0].byte_range, 0..5);
    }

    #[test]
    fn row_count_matches_len() {
        let mut wm = WrapMap::new(WrapConfig {
            wrap_col: 5,
            word_break: false,
        });
        wm.wrap("hello world");
        assert_eq!(wm.row_count() as usize, wm.rows.len());
    }

    #[test]
    fn row_for_offset_hit() {
        let mut wm = WrapMap::new(WrapConfig::default());
        wm.wrap("hello world");
        let row = wm.row_for_offset(0);
        assert!(row.is_some());
    }

    #[test]
    fn row_for_offset_miss() {
        let mut wm = WrapMap::new(WrapConfig::default());
        wm.wrap("hello");
        let row = wm.row_for_offset(999);
        assert!(row.is_none());
    }

    #[test]
    fn begin_interpolated_rewrap_sets_flag() {
        let mut wm = WrapMap::new(WrapConfig::default());
        wm.wrap("hello world");
        assert!(!wm.rows[0].interpolated);
        wm.begin_interpolated_rewrap();
        assert!(wm.rows.iter().all(|r| r.interpolated));
    }
}
