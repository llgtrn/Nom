/// A byte-range span in source text. Stores start and end as u32 so
/// the struct is Copy (Range<u32> is not Copy).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub fn len(&self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn contains(&self, offset: u32) -> bool {
        offset >= self.start && offset < self.end
    }

    pub fn contains_range(&self, other: &Span) -> bool {
        self.start <= other.start && other.end <= self.end
    }

    pub fn union(&self, other: &Span) -> Span {
        Span::new(self.start.min(other.start), self.end.max(other.end))
    }
}

/// Convert a byte offset into 1-based line and 0-based column.
pub fn byte_offset_to_line_col(source: &str, offset: u32) -> (u32, u32) {
    let offset = offset as usize;
    let clamped = offset.min(source.len());
    let prefix = &source[..clamped];
    let mut line = 1u32;
    let mut col = 0u32;
    let mut chars = prefix.char_indices().peekable();
    while let Some((i, ch)) = chars.next() {
        if ch == '\r' {
            // CRLF or bare CR both advance line
            line += 1;
            col = 0;
        } else if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            let char_bytes = ch.len_utf8() as u32;
            if i + ch.len_utf8() <= clamped {
                col += char_bytes;
            }
        }
    }
    (line, col)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_and_len() {
        let s = Span::new(2, 7);
        assert_eq!(s.len(), 5);
    }

    #[test]
    fn is_empty_true() {
        assert!(Span::new(3, 3).is_empty());
    }

    #[test]
    fn is_empty_false() {
        assert!(!Span::new(0, 1).is_empty());
    }

    #[test]
    fn contains_offset() {
        let s = Span::new(5, 10);
        assert!(s.contains(5));
        assert!(s.contains(9));
        assert!(!s.contains(10));
        assert!(!s.contains(4));
    }

    #[test]
    fn contains_range() {
        let outer = Span::new(0, 20);
        let inner = Span::new(5, 15);
        assert!(outer.contains_range(&inner));
        assert!(!inner.contains_range(&outer));
    }

    #[test]
    fn union_spans() {
        let a = Span::new(2, 8);
        let b = Span::new(5, 15);
        let u = a.union(&b);
        assert_eq!(u, Span::new(2, 15));
    }

    #[test]
    fn line_col_first_line() {
        let source = "hello world";
        let (line, col) = byte_offset_to_line_col(source, 6);
        assert_eq!(line, 1);
        assert_eq!(col, 6);
    }

    #[test]
    fn line_col_after_newline() {
        let source = "abc\ndef";
        // offset 4 = 'd', line 2, col 0
        let (line, col) = byte_offset_to_line_col(source, 4);
        assert_eq!(line, 2);
        assert_eq!(col, 0);
    }

    #[test]
    fn line_col_crlf() {
        // "abc\r\ndef": bytes 0=a,1=b,2=c,3=\r,4=\n,5=d,6=e,7=f
        // After \r at idx 3: line=2, col=0; \n at idx 4: line=3, col=0
        // 'd' at idx 5: col=1; 'e' at idx 6: col=2
        let source = "abc\r\ndef";
        let (line, col) = byte_offset_to_line_col(source, 6);
        assert_eq!(line, 3);
        assert_eq!(col, 1);
    }

    #[test]
    fn line_col_multibyte_utf8() {
        // "é" = 2 bytes (U+00E9), "a" = 1 byte
        let source = "éa";
        // offset 2 = 'a', line 1, col 2 (byte-based col)
        let (line, col) = byte_offset_to_line_col(source, 2);
        assert_eq!(line, 1);
        assert_eq!(col, 2);
    }
}
