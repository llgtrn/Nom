#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Keyword,
    Identifier,
    StringLit,
    NumberLit,
    Operator,
    Comment,
    Whitespace,
    Unknown,
}

impl TokenKind {
    pub fn kind_name(&self) -> &str {
        match self {
            TokenKind::Keyword => "keyword",
            TokenKind::Identifier => "identifier",
            TokenKind::StringLit => "string_lit",
            TokenKind::NumberLit => "number_lit",
            TokenKind::Operator => "operator",
            TokenKind::Comment => "comment",
            TokenKind::Whitespace => "whitespace",
            TokenKind::Unknown => "unknown",
        }
    }

    pub fn is_trivia(&self) -> bool {
        matches!(self, TokenKind::Whitespace | TokenKind::Comment)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxToken {
    pub kind: TokenKind,
    pub start: usize,
    pub len: usize,
}

impl SyntaxToken {
    pub fn new(kind: TokenKind, start: usize, len: usize) -> Self {
        Self { kind, start, len }
    }

    pub fn end(&self) -> usize {
        self.start + self.len
    }

    pub fn text<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.start + self.len]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightRange {
    pub start: usize,
    pub end: usize,
    pub color_r: u8,
    pub color_g: u8,
    pub color_b: u8,
}

impl HighlightRange {
    pub fn new(start: usize, end: usize, r: u8, g: u8, b: u8) -> Self {
        Self { start, end, color_r: r, color_g: g, color_b: b }
    }

    pub fn length(&self) -> usize {
        self.end - self.start
    }

    pub fn overlaps(&self, other: &HighlightRange) -> bool {
        self.start < other.end && other.start < self.end
    }
}

pub struct SyntaxHighlighter;

impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self
    }

    pub fn color_for_kind(kind: &TokenKind) -> (u8, u8, u8) {
        match kind {
            TokenKind::Keyword => (99, 102, 241),
            TokenKind::Identifier => (240, 240, 240),
            TokenKind::StringLit => (134, 239, 172),
            TokenKind::NumberLit => (251, 191, 36),
            TokenKind::Operator => (248, 113, 113),
            TokenKind::Comment => (107, 114, 128),
            TokenKind::Whitespace => (0, 0, 0),
            TokenKind::Unknown => (128, 128, 128),
        }
    }

    pub fn token_to_range(token: &SyntaxToken) -> HighlightRange {
        let (r, g, b) = Self::color_for_kind(&token.kind);
        HighlightRange::new(token.start, token.end(), r, g, b)
    }

    pub fn highlight(tokens: &[SyntaxToken]) -> Vec<HighlightRange> {
        tokens
            .iter()
            .filter(|t| !t.kind.is_trivia())
            .map(Self::token_to_range)
            .collect()
    }
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod syntax_highlight_tests {
    use super::*;

    #[test]
    fn test_is_trivia() {
        assert!(TokenKind::Whitespace.is_trivia());
        assert!(TokenKind::Comment.is_trivia());
        assert!(!TokenKind::Keyword.is_trivia());
        assert!(!TokenKind::Identifier.is_trivia());
        assert!(!TokenKind::StringLit.is_trivia());
        assert!(!TokenKind::NumberLit.is_trivia());
        assert!(!TokenKind::Operator.is_trivia());
        assert!(!TokenKind::Unknown.is_trivia());
    }

    #[test]
    fn test_syntax_token_text() {
        let source = "define foo 42";
        let token = SyntaxToken::new(TokenKind::Keyword, 0, 6);
        assert_eq!(token.text(source), "define");
    }

    #[test]
    fn test_syntax_token_end() {
        let token = SyntaxToken::new(TokenKind::Identifier, 7, 3);
        assert_eq!(token.end(), 10);
    }

    #[test]
    fn test_highlight_range_length() {
        let range = HighlightRange::new(5, 12, 0, 0, 0);
        assert_eq!(range.length(), 7);
    }

    #[test]
    fn test_highlight_range_overlaps_true() {
        let a = HighlightRange::new(0, 10, 0, 0, 0);
        let b = HighlightRange::new(5, 15, 0, 0, 0);
        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));
    }

    #[test]
    fn test_highlight_range_overlaps_false() {
        let a = HighlightRange::new(0, 5, 0, 0, 0);
        let b = HighlightRange::new(5, 10, 0, 0, 0);
        assert!(!a.overlaps(&b));
        assert!(!b.overlaps(&a));
    }

    #[test]
    fn test_color_for_kind_keyword() {
        let color = SyntaxHighlighter::color_for_kind(&TokenKind::Keyword);
        assert_eq!(color, (99, 102, 241));
    }

    #[test]
    fn test_highlight_skips_trivia() {
        let tokens = vec![
            SyntaxToken::new(TokenKind::Keyword, 0, 6),
            SyntaxToken::new(TokenKind::Whitespace, 6, 1),
            SyntaxToken::new(TokenKind::Identifier, 7, 3),
            SyntaxToken::new(TokenKind::Comment, 10, 5),
        ];
        let ranges = SyntaxHighlighter::highlight(&tokens);
        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0].start, 0);
        assert_eq!(ranges[1].start, 7);
    }

    #[test]
    fn test_token_to_range_bounds() {
        let token = SyntaxToken::new(TokenKind::NumberLit, 3, 4);
        let range = SyntaxHighlighter::token_to_range(&token);
        assert_eq!(range.start, 3);
        assert_eq!(range.end, 7);
        assert_eq!((range.color_r, range.color_g, range.color_b), (251, 191, 36));
    }
}
