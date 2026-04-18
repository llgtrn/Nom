#![deny(unsafe_code)]
use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenRole {
    Keyword,
    Identifier,
    Literal,
    Operator,
    Comment,
    NomtuRef,         // entity resolved via dict
    ClauseConnective, // grammar clause keywords
    Unknown,
}

#[derive(Clone, Debug)]
pub struct HighlightSpan {
    pub range: Range<usize>,
    pub token_role: TokenRole,
}

impl HighlightSpan {
    pub fn new(range: Range<usize>, token_role: TokenRole) -> Self {
        Self { range, token_role }
    }
}

// Color as [h,s,l,a] since we can't import nom-gpui Hsla directly here
// (nom-gpui dep not in nom-editor Cargo.toml by default — colors looked up from theme at render)
#[derive(Clone, Copy, Debug)]
pub struct SpanColor {
    pub h: f32,
    pub s: f32,
    pub l: f32,
    pub a: f32,
}

impl SpanColor {
    pub const KEYWORD: SpanColor = SpanColor {
        h: 0.586,
        s: 1.0,
        l: 0.65,
        a: 1.0,
    }; // accent-blue
    pub const NOMTU_REF: SpanColor = SpanColor {
        h: 0.75,
        s: 0.91,
        l: 0.70,
        a: 1.0,
    }; // accent-purple
    pub const LITERAL: SpanColor = SpanColor {
        h: 0.403,
        s: 0.63,
        l: 0.49,
        a: 1.0,
    }; // accent-green
    pub const COMMENT: SpanColor = SpanColor {
        h: 0.0,
        s: 0.0,
        l: 0.45,
        a: 1.0,
    };
    pub const OPERATOR: SpanColor = SpanColor {
        h: 0.0,
        s: 0.0,
        l: 0.75,
        a: 1.0,
    };
    pub const DEFAULT: SpanColor = SpanColor {
        h: 0.0,
        s: 0.0,
        l: 0.98,
        a: 1.0,
    };
}

pub struct Highlighter;

impl Highlighter {
    /// Convert highlight spans to (range, color) pairs.
    /// Wave B: applies static color map. Wave C: spans come from stage1_tokenize via bridge.
    pub fn color_runs(spans: &[HighlightSpan]) -> Vec<(Range<usize>, SpanColor)> {
        spans
            .iter()
            .map(|span| {
                let color = match span.token_role {
                    TokenRole::Keyword | TokenRole::ClauseConnective => SpanColor::KEYWORD,
                    TokenRole::NomtuRef => SpanColor::NOMTU_REF,
                    TokenRole::Literal => SpanColor::LITERAL,
                    TokenRole::Operator => SpanColor::OPERATOR,
                    TokenRole::Comment => SpanColor::COMMENT,
                    _ => SpanColor::DEFAULT,
                };
                (span.range.clone(), color)
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Simple character-level scanner producing TokenClass spans
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenClass {
    Keyword,
    Identifier,
    Literal,  // quoted strings
    Operator, // + - * / =
    Comment,
    Whitespace,
    Unknown,
}

/// Highlight a Nom source string into byte-offset spans.
///
/// Rules (in scan order):
/// - `"define"` or `"that"` → Keyword
/// - word chars (alphanumeric + `_`) → Identifier (downgraded to Keyword if word is "define"/"that")
/// - double-quoted strings → Literal
/// - `+ - * / =` → Operator
/// - ASCII whitespace → Whitespace
/// - anything else → Unknown
pub fn highlight_nom_source(source: &str) -> Vec<SyntaxSpan> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut spans: Vec<SyntaxSpan> = Vec::new();
    let mut i = 0;

    while i < len {
        let b = bytes[i];

        // Quoted string literal
        if b == b'"' {
            let start = i;
            i += 1;
            while i < len && bytes[i] != b'"' {
                i += 1;
            }
            if i < len {
                i += 1; // consume closing quote
            }
            spans.push(SyntaxSpan { start, end: i, class: TokenClass::Literal });
            continue;
        }

        // Word (identifier or keyword)
        if b.is_ascii_alphanumeric() || b == b'_' {
            let start = i;
            while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let word = &source[start..i];
            let class = if word == "define" || word == "that" {
                TokenClass::Keyword
            } else {
                TokenClass::Identifier
            };
            spans.push(SyntaxSpan { start, end: i, class });
            continue;
        }

        // Whitespace
        if b.is_ascii_whitespace() {
            let start = i;
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            spans.push(SyntaxSpan { start, end: i, class: TokenClass::Whitespace });
            continue;
        }

        // Operator
        if matches!(b, b'+' | b'-' | b'*' | b'/' | b'=') {
            spans.push(SyntaxSpan { start: i, end: i + 1, class: TokenClass::Operator });
            i += 1;
            continue;
        }

        // Unknown single byte
        spans.push(SyntaxSpan { start: i, end: i + 1, class: TokenClass::Unknown });
        i += 1;
    }

    spans
}

/// A highlight span produced by the character-level scanner.
#[derive(Debug, Clone)]
pub struct SyntaxSpan {
    pub start: usize,
    pub end: usize,
    pub class: TokenClass,
}

impl SyntaxSpan {
    pub fn new(start: usize, end: usize, class: TokenClass) -> Self {
        Self { start, end, class }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlight_color_runs() {
        let spans = vec![
            HighlightSpan::new(0..4, TokenRole::Keyword),
            HighlightSpan::new(5..9, TokenRole::NomtuRef),
        ];
        let runs = Highlighter::color_runs(&spans);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].1.h, SpanColor::KEYWORD.h);
        assert_eq!(runs[1].1.h, SpanColor::NOMTU_REF.h);
    }

    #[test]
    fn highlighter_empty_returns_empty_runs() {
        let runs = Highlighter::color_runs(&[]);
        assert!(runs.is_empty());
    }

    #[test]
    fn highlighter_assigns_color_run() {
        let spans = vec![HighlightSpan::new(0..5, TokenRole::Literal)];
        let runs = Highlighter::color_runs(&spans);
        assert_eq!(runs.len(), 1);
    }

    #[test]
    fn color_run_range_nonempty() {
        let spans = vec![
            HighlightSpan::new(0..3, TokenRole::Keyword),
            HighlightSpan::new(4..10, TokenRole::Comment),
        ];
        let runs = Highlighter::color_runs(&spans);
        for (range, _color) in &runs {
            assert!(range.end > range.start, "run range must be non-empty");
        }
    }

    #[test]
    fn color_run_color_is_valid_hsla() {
        let spans = vec![
            HighlightSpan::new(0..4, TokenRole::Operator),
            HighlightSpan::new(5..9, TokenRole::Literal),
            HighlightSpan::new(10..15, TokenRole::Comment),
        ];
        let runs = Highlighter::color_runs(&spans);
        for (_range, color) in &runs {
            assert!((0.0..=1.0).contains(&color.h), "hue must be 0..=1");
            assert!((0.0..=1.0).contains(&color.s), "saturation must be 0..=1");
            assert!((0.0..=1.0).contains(&color.l), "lightness must be 0..=1");
            assert!((0.0..=1.0).contains(&color.a), "alpha must be 0..=1");
        }
    }

    #[test]
    fn highlight_clause_connective_uses_keyword_color() {
        let spans = vec![HighlightSpan::new(0..4, TokenRole::ClauseConnective)];
        let runs = Highlighter::color_runs(&spans);
        assert_eq!(runs[0].1.h, SpanColor::KEYWORD.h);
    }

    #[test]
    fn highlight_unknown_uses_default_color() {
        let spans = vec![HighlightSpan::new(0..3, TokenRole::Unknown)];
        let runs = Highlighter::color_runs(&spans);
        assert_eq!(runs[0].1.h, SpanColor::DEFAULT.h);
        assert_eq!(runs[0].1.a, 1.0);
    }

    #[test]
    fn highlight_comment_color() {
        let comment_spans = vec![HighlightSpan::new(0..5, TokenRole::Comment)];
        let keyword_spans = vec![HighlightSpan::new(0..5, TokenRole::Keyword)];
        let comment_runs = Highlighter::color_runs(&comment_spans);
        let keyword_runs = Highlighter::color_runs(&keyword_spans);
        // Comment and keyword must have distinct hues
        let comment_h = comment_runs[0].1.h;
        let keyword_h = keyword_runs[0].1.h;
        assert_ne!(
            comment_h, keyword_h,
            "comment color must differ from keyword color"
        );
    }

    #[test]
    fn highlight_string_color() {
        // Literal (string) token must have a distinct color from keyword
        let literal_spans = vec![HighlightSpan::new(0..5, TokenRole::Literal)];
        let keyword_spans = vec![HighlightSpan::new(0..5, TokenRole::Keyword)];
        let literal_runs = Highlighter::color_runs(&literal_spans);
        let keyword_runs = Highlighter::color_runs(&keyword_spans);
        let literal_h = literal_runs[0].1.h;
        let keyword_h = keyword_runs[0].1.h;
        assert_ne!(
            literal_h, keyword_h,
            "string/literal color must differ from keyword color"
        );
    }

    // --- SyntaxSpan + highlight_nom_source tests ---

    #[test]
    fn highlight_span_len() {
        let span = SyntaxSpan::new(3, 9, TokenClass::Identifier);
        assert_eq!(span.len(), 6);
        assert!(!span.is_empty());
        let empty = SyntaxSpan::new(4, 4, TokenClass::Whitespace);
        assert!(empty.is_empty());
    }

    #[test]
    fn highlight_nom_source_keywords() {
        let spans = highlight_nom_source("define greet that");
        let kw_spans: Vec<&SyntaxSpan> = spans
            .iter()
            .filter(|s| s.class == TokenClass::Keyword)
            .collect();
        assert_eq!(kw_spans.len(), 2, "expected keyword spans for 'define' and 'that'");
        assert_eq!(&"define greet that"[kw_spans[0].start..kw_spans[0].end], "define");
        assert_eq!(&"define greet that"[kw_spans[1].start..kw_spans[1].end], "that");
    }

    #[test]
    fn highlight_nom_source_identifier() {
        let spans = highlight_nom_source("my_var");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].class, TokenClass::Identifier);
        assert_eq!(spans[0].start, 0);
        assert_eq!(spans[0].end, 6);
    }

    #[test]
    fn highlight_nom_source_literal() {
        let source = r#""hello""#;
        let spans = highlight_nom_source(source);
        let lit: Vec<&SyntaxSpan> = spans
            .iter()
            .filter(|s| s.class == TokenClass::Literal)
            .collect();
        assert_eq!(lit.len(), 1, "expected one literal span");
        assert_eq!(&source[lit[0].start..lit[0].end], r#""hello""#);
    }
}
