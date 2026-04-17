#![deny(unsafe_code)]

pub struct TabMap { pub tab_size: usize }

impl TabMap {
    pub fn new(tab_size: usize) -> Self { Self { tab_size } }

    /// Expand tabs to spaces, return (expanded_text, visual_column_offsets)
    pub fn expand_tabs(&self, line: &str) -> (String, Vec<usize>) {
        let mut expanded = String::with_capacity(line.len() * 2);
        let mut offsets = Vec::with_capacity(line.len());
        let mut col = 0usize;
        for ch in line.chars() {
            offsets.push(col);
            if ch == '\t' {
                let spaces = self.tab_size - (col % self.tab_size);
                for _ in 0..spaces { expanded.push(' '); col += 1; }
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
            if i == char_offset { break; }
            if ch == '\t' { col += self.tab_size - (col % self.tab_size); }
            else { col += 1; }
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
}
