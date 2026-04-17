//! Tab-width pre-computation for display-layer expansion.
#![deny(unsafe_code)]

// TODO: unify with display_map BufferOffset when both land.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferOffset(pub usize);

// TODO: unify with display_map DisplayOffset when both land.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplayOffset(pub usize);

/// Column index after expanding a tab starting at `current_col`.
pub fn next_tab_stop(current_col: u32, tab_width: u32) -> u32 {
    if tab_width == 0 {
        return current_col + 1;
    }
    current_col + (tab_width - current_col % tab_width)
}

#[derive(Clone, Debug, PartialEq)]
pub struct TabMap {
    pub tab_width: u32,
}

impl TabMap {
    pub fn new(tab_width: u32) -> Self {
        Self { tab_width }
    }

    /// Given a line's byte content, compute the display column after each character.
    /// Returns a Vec parallel to the input chars; `cols[i]` is the display column
    /// AFTER consuming char i (cols[0] = next_tab_stop(0, tab_width) for tab,
    /// else 1; subsequent values accumulate).
    pub fn expand_line(&self, line: &str) -> Vec<u32> {
        let mut cols = Vec::new();
        let mut current_col: u32 = 0;
        for ch in line.chars() {
            if ch == '\t' {
                current_col = next_tab_stop(current_col, self.tab_width);
            } else {
                current_col += 1;
            }
            cols.push(current_col);
        }
        cols
    }

    /// Map a display column back to a char index in `line`. Saturates at end-of-line.
    pub fn buffer_col_at_display(&self, line: &str, display_col: u32) -> usize {
        let mut current_col: u32 = 0;
        for (i, ch) in line.chars().enumerate() {
            if ch == '\t' {
                current_col = next_tab_stop(current_col, self.tab_width);
            } else {
                current_col += 1;
            }
            if current_col >= display_col {
                return i;
            }
        }
        line.chars().count().saturating_sub(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_tab_stop_zero_col() {
        assert_eq!(next_tab_stop(0, 4), 4);
    }

    #[test]
    fn next_tab_stop_mid_col() {
        assert_eq!(next_tab_stop(1, 4), 4);
    }

    #[test]
    fn next_tab_stop_at_boundary() {
        assert_eq!(next_tab_stop(4, 4), 8);
    }

    #[test]
    fn next_tab_stop_zero_width_always_plus_one() {
        assert_eq!(next_tab_stop(0, 0), 1);
        assert_eq!(next_tab_stop(5, 0), 6);
        assert_eq!(next_tab_stop(99, 0), 100);
    }

    #[test]
    fn expand_line_plain_ascii() {
        let tm = TabMap::new(4);
        assert_eq!(tm.expand_line("abc"), vec![1, 2, 3]);
    }

    #[test]
    fn expand_line_leading_tab() {
        let tm = TabMap::new(4);
        // '\t' at col 0 → col 4; 'a' → 5; 'b' → 6
        assert_eq!(tm.expand_line("\tab"), vec![4, 5, 6]);
    }

    #[test]
    fn expand_line_tab_after_char() {
        let tm = TabMap::new(4);
        // 'a' → 1; '\t' at col 1 → col 4; 'b' → 5; 'c' → 6
        assert_eq!(tm.expand_line("a\tbc"), vec![1, 4, 5, 6]);
    }

    #[test]
    fn buffer_col_at_display_saturates() {
        let tm = TabMap::new(4);
        let line = "ab";
        // display_col beyond line length → saturate at last index
        let idx = tm.buffer_col_at_display(line, 100);
        assert!(idx <= line.chars().count());
    }

    #[test]
    fn buffer_col_at_display_basic() {
        let tm = TabMap::new(4);
        // "abc": col 1 → index 0, col 2 → index 1, col 3 → index 2
        assert_eq!(tm.buffer_col_at_display("abc", 1), 0);
        assert_eq!(tm.buffer_col_at_display("abc", 2), 1);
    }
}
