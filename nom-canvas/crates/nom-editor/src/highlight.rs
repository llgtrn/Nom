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
pub struct SpanColor { pub h: f32, pub s: f32, pub l: f32, pub a: f32 }

impl SpanColor {
    pub const KEYWORD: SpanColor = SpanColor { h: 0.586, s: 1.0, l: 0.65, a: 1.0 }; // accent-blue
    pub const NOMTU_REF: SpanColor = SpanColor { h: 0.75, s: 0.91, l: 0.70, a: 1.0 }; // accent-purple
    pub const LITERAL: SpanColor = SpanColor { h: 0.403, s: 0.63, l: 0.49, a: 1.0 }; // accent-green
    pub const COMMENT: SpanColor = SpanColor { h: 0.0, s: 0.0, l: 0.45, a: 1.0 };
    pub const OPERATOR: SpanColor = SpanColor { h: 0.0, s: 0.0, l: 0.75, a: 1.0 };
    pub const DEFAULT: SpanColor = SpanColor { h: 0.0, s: 0.0, l: 0.98, a: 1.0 };
}

pub struct Highlighter;

impl Highlighter {
    /// Convert highlight spans to (range, color) pairs.
    /// Wave B: applies static color map. Wave C: spans come from stage1_tokenize via bridge.
    pub fn color_runs(spans: &[HighlightSpan]) -> Vec<(Range<usize>, SpanColor)> {
        spans.iter().map(|span| {
            let color = match span.token_role {
                TokenRole::Keyword | TokenRole::ClauseConnective => SpanColor::KEYWORD,
                TokenRole::NomtuRef => SpanColor::NOMTU_REF,
                TokenRole::Literal => SpanColor::LITERAL,
                TokenRole::Operator => SpanColor::OPERATOR,
                TokenRole::Comment => SpanColor::COMMENT,
                _ => SpanColor::DEFAULT,
            };
            (span.range.clone(), color)
        }).collect()
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
}
