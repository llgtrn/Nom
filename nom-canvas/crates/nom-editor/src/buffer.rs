use ropey::Rope;

pub struct Buffer {
    rope: Rope,
    version: u64,
}

impl Buffer {
    pub fn new() -> Self {
        Self { rope: Rope::new(), version: 0 }
    }

    pub fn from_str(s: &str) -> Self {
        Self { rope: Rope::from_str(s), version: 0 }
    }

    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn line(&self, idx: usize) -> String {
        self.rope.line(idx).to_string()
    }

    /// Insert at character offset. Returns new cursor position.
    pub fn insert(&mut self, char_offset: usize, text: &str) -> usize {
        self.rope.insert(char_offset, text);
        self.version += 1;
        char_offset + text.chars().count()
    }

    /// Delete a character range. Returns the char_offset after deletion.
    pub fn delete(&mut self, range: std::ops::Range<usize>) -> usize {
        self.rope.remove(range.clone());
        self.version += 1;
        range.start
    }

    /// Convert (line, column) to char offset. Both 0-based.
    pub fn line_col_to_offset(&self, line: usize, col: usize) -> usize {
        let line_start = self.rope.line_to_char(line);
        line_start + col
    }

    /// Inverse: char offset to (line, column).
    pub fn offset_to_line_col(&self, offset: usize) -> (usize, usize) {
        let line = self.rope.char_to_line(offset);
        let line_start = self.rope.line_to_char(line);
        (line, offset - line_start)
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn to_string(&self) -> String {
        self.rope.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_buffer_is_empty() {
        let buf = Buffer::new();
        assert_eq!(buf.len_chars(), 0);
        assert_eq!(buf.version(), 0);
    }

    #[test]
    fn insert_then_read_matches() {
        let mut buf = Buffer::new();
        buf.insert(0, "hello");
        assert_eq!(buf.to_string(), "hello");
        assert_eq!(buf.len_chars(), 5);
    }

    #[test]
    fn delete_shrinks_buffer() {
        let mut buf = Buffer::from_str("hello world");
        buf.delete(5..11);
        assert_eq!(buf.to_string(), "hello");
    }

    #[test]
    fn line_col_round_trip() {
        let mut buf = Buffer::new();
        buf.insert(0, "abc\ndef\nghi");
        // line 1, col 2 → offset 6
        let offset = buf.line_col_to_offset(1, 2);
        assert_eq!(offset, 6);
        let (line, col) = buf.offset_to_line_col(6);
        assert_eq!((line, col), (1, 2));
    }
}
