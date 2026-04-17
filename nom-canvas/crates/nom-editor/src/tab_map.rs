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
}
