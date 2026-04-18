#![deny(unsafe_code)]

pub struct TabMap {
    pub tab_size: usize,
}

impl TabMap {
    pub fn new(tab_size: usize) -> Self {
        Self { tab_size }
    }

    /// Expand tabs to spaces, return (expanded_text, visual_column_offsets)
    pub fn expand_tabs(&self, line: &str) -> (String, Vec<usize>) {
        let mut expanded = String::with_capacity(line.len() * 2);
        let mut offsets = Vec::with_capacity(line.len());
        let mut col = 0usize;
        for ch in line.chars() {
            offsets.push(col);
            if ch == '\t' {
                let spaces = self.tab_size - (col % self.tab_size);
                for _ in 0..spaces {
                    expanded.push(' ');
                    col += 1;
                }
            } else {
                expanded.push(ch);
                col += 1;
            }
        }
        (expanded, offsets)
    }

    /// Visual column of char at byte offset in original line
    pub fn visual_column(&self, line: &str, char_offset: usize) -> usize {
        let mut col = 0usize;
        for (i, ch) in line.chars().enumerate() {
            if i == char_offset {
                break;
            }
            if ch == '\t' {
                col += self.tab_size - (col % self.tab_size);
            } else {
                col += 1;
            }
        }
        col
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tab_expansion() {
        let tm = TabMap::new(4);
        let (expanded, _) = tm.expand_tabs("\thello");
        assert_eq!(expanded, "    hello");
    }
    #[test]
    fn visual_column() {
        let tm = TabMap::new(4);
        assert_eq!(tm.visual_column("\t", 0), 0);
        assert_eq!(tm.visual_column("\thello", 1), 4);
    }

    #[test]
    fn tab_map_replaces_tabs_with_spaces() {
        let tm = TabMap::new(4);
        let (expanded, _offsets) = tm.expand_tabs("\t\tcode");
        // Two tabs at size 4 each → 8 leading spaces
        assert_eq!(&expanded[..8], "        ");
        assert!(expanded.ends_with("code"));
    }

    #[test]
    fn tab_map_mid_line_tab_alignment() {
        let tm = TabMap::new(4);
        // "ab\t" — tab at col 2 should pad to col 4 (2 spaces added)
        let (expanded, _) = tm.expand_tabs("ab\t");
        assert_eq!(expanded, "ab  ");
    }

    #[test]
    fn tab_map_offsets_length_matches_chars() {
        let tm = TabMap::new(4);
        let line = "a\tb\tc";
        let (_, offsets) = tm.expand_tabs(line);
        // One offset per character in the original line
        assert_eq!(offsets.len(), line.chars().count());
    }

    #[test]
    fn tab_map_tab_to_spaces() {
        let tm = TabMap::new(4);
        let (expanded, _) = tm.expand_tabs("\thello");
        // One tab at col 0 with tab_size=4 expands to 4 spaces
        assert!(expanded.starts_with("    "));
        assert!(expanded.ends_with("hello"));
    }

    #[test]
    fn tab_map_preserves_newlines() {
        // expand_tabs operates on a single line; a line without tabs is unchanged
        let tm = TabMap::new(4);
        let (expanded, _) = tm.expand_tabs("no tabs here");
        assert_eq!(expanded, "no tabs here");
    }

    #[test]
    fn tab_expansion_at_col_zero() {
        // Tab at col 0 with size 4 expands to exactly 4 spaces
        let tm = TabMap::new(4);
        let (expanded, offsets) = tm.expand_tabs("\t");
        assert_eq!(expanded, "    ");
        assert_eq!(offsets.len(), 1);
        assert_eq!(offsets[0], 0); // visual col of the tab char itself
    }

    #[test]
    fn tab_expansion_at_col_one() {
        // "a\t" — tab at col 1 with size 4 pads to col 4 (3 spaces)
        let tm = TabMap::new(4);
        let (expanded, _) = tm.expand_tabs("a\t");
        assert_eq!(expanded, "a   ");
    }

    #[test]
    fn tab_expansion_at_col_three() {
        // "abc\t" — tab at col 3 with size 4 pads to col 4 (1 space)
        let tm = TabMap::new(4);
        let (expanded, _) = tm.expand_tabs("abc\t");
        assert_eq!(expanded, "abc ");
    }

    #[test]
    fn tab_expansion_size_8() {
        let tm = TabMap::new(8);
        let (expanded, _) = tm.expand_tabs("\t");
        assert_eq!(expanded.len(), 8);
    }

    #[test]
    fn tab_expansion_multiple_tabs_align_to_stops() {
        // Three tabs of size 4 → 12 leading spaces
        let tm = TabMap::new(4);
        let (expanded, _) = tm.expand_tabs("\t\t\t");
        assert_eq!(expanded, "            "); // 12 spaces
    }

    #[test]
    fn visual_column_after_multiple_chars() {
        // "abc" — char at index 3 is at visual col 3
        let tm = TabMap::new(4);
        assert_eq!(tm.visual_column("abcdef", 3), 3);
    }

    #[test]
    fn visual_column_tab_at_col_4() {
        // "abcd\t" — tab at col 4 pads to col 8; char at index 5 is at col 8
        let tm = TabMap::new(4);
        let col = tm.visual_column("abcd\tx", 5);
        assert_eq!(col, 8);
    }

    #[test]
    fn tab_map_size_2_expansion() {
        let tm = TabMap::new(2);
        let (expanded, _) = tm.expand_tabs("\t");
        assert_eq!(expanded, "  "); // 2 spaces
    }

    #[test]
    fn tab_map_size_2_mid_line() {
        // "a\t" with size 2: tab at col 1, pads to col 2 (1 space)
        let tm = TabMap::new(2);
        let (expanded, _) = tm.expand_tabs("a\t");
        assert_eq!(expanded, "a "); // 1 space to reach next stop
    }

    #[test]
    fn tab_map_no_tabs_offsets_sequential() {
        let tm = TabMap::new(4);
        let (_, offsets) = tm.expand_tabs("abc");
        assert_eq!(offsets, vec![0, 1, 2]);
    }

    #[test]
    fn tab_map_empty_line() {
        let tm = TabMap::new(4);
        let (expanded, offsets) = tm.expand_tabs("");
        assert_eq!(expanded, "");
        assert!(offsets.is_empty());
    }

    #[test]
    fn tab_map_visual_col_no_tab() {
        let tm = TabMap::new(4);
        assert_eq!(tm.visual_column("hello", 3), 3);
    }

    #[test]
    fn tab_map_visual_col_at_start() {
        let tm = TabMap::new(4);
        assert_eq!(tm.visual_column("anything", 0), 0);
    }

    #[test]
    fn tab_map_mixed_indent_detection_spaces() {
        // A line with only spaces (no tabs) — visual col matches char col
        let tm = TabMap::new(4);
        let line = "    code";
        // char 4 ('c') is at visual col 4
        assert_eq!(tm.visual_column(line, 4), 4);
    }

    #[test]
    fn tab_map_tab_size_8_mid_line() {
        // "abcdefg\t" — tab at col 7, size 8, pads to col 8 (1 space)
        let tm = TabMap::new(8);
        let (expanded, _) = tm.expand_tabs("abcdefg\t");
        assert_eq!(expanded, "abcdefg "); // 1 space to reach col 8
    }

    #[test]
    fn tab_map_offsets_for_tab_line() {
        // "\ta" — tab at col 0 produces visual col 0 for tab, then 4 for 'a'
        let tm = TabMap::new(4);
        let (_, offsets) = tm.expand_tabs("\ta");
        assert_eq!(offsets.len(), 2);
        assert_eq!(offsets[0], 0); // tab itself at visual col 0
        assert_eq!(offsets[1], 4); // 'a' at visual col 4
    }

    // ── wave AH-7: new tab_map tests ─────────────────────────────────────────

    #[test]
    fn tab_map_tab_to_spaces_correct_width() {
        // A tab at column 0 with tab_size=4 produces exactly 4 spaces
        let tm = TabMap::new(4);
        let (expanded, _) = tm.expand_tabs("\t");
        assert_eq!(expanded, "    ");
        assert_eq!(expanded.len(), 4);
    }

    #[test]
    fn tab_map_column_accounting_after_tab() {
        // After a tab at col 0 (size 4), the next char is at visual col 4
        let tm = TabMap::new(4);
        let col = tm.visual_column("\ta", 1);
        assert_eq!(col, 4);
    }

    #[test]
    fn tab_map_visual_vs_logical_position() {
        // Logical position 1 in "\ta" is 'a'; visual col = 4 (tab expanded to 4 spaces)
        let tm = TabMap::new(4);
        let (expanded, offsets) = tm.expand_tabs("\ta");
        // logical char 1 ('a') has visual col 4
        assert_eq!(offsets[1], 4);
        // expanded starts with spaces
        assert!(expanded.starts_with("    "));
        assert!(expanded.ends_with('a'));
    }
}
