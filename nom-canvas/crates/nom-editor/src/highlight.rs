//! Syntax highlighting driven by S1-S2 pipeline tokens.
//!
//! Stage 1 (tokenize) produces a `Vec<Token>` with byte ranges and kinds.
//! Stage 2 (classify) tags each token with a semantic role (keyword, ident,
//! literal, operator, ...). This module maps S1-S2 output to GPU-paintable
//! color runs.

#![deny(unsafe_code)]

use std::ops::Range;

/// A 32-bit RGBA color value. Hex constructor interprets `0xRRGGBB` with
/// full opacity (alpha = 0xFF).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rgba(pub u32);

impl Rgba {
    /// Construct from a 24-bit `0xRRGGBB` literal; alpha is set to 0xFF.
    pub const fn hex(rgb: u32) -> Self {
        Self((rgb << 8) | 0xFF)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenRole {
    Keyword,
    Ident,
    StringLit,
    NumberLit,
    Operator,
    Comment,
    Punctuation,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct HighlightSpan {
    pub byte_range: Range<usize>,
    pub role: TokenRole,
}

pub struct Highlighter {
    palette: Palette,
}

pub struct Palette {
    pub keyword: Rgba,
    pub ident: Rgba,
    pub string_lit: Rgba,
    pub number_lit: Rgba,
    pub operator: Rgba,
    pub comment: Rgba,
    pub punctuation: Rgba,
    pub default: Rgba,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            keyword:     Rgba::hex(0x569c_d6),
            ident:       Rgba::hex(0x9cdc_fe),
            string_lit:  Rgba::hex(0xce91_78),
            number_lit:  Rgba::hex(0xb5ce_a8),
            operator:    Rgba::hex(0xd4d4_d4),
            comment:     Rgba::hex(0x6a99_55),
            punctuation: Rgba::hex(0xd4d4_d4),
            default:     Rgba::hex(0xd4d4_d4),
        }
    }
}

impl Highlighter {
    pub fn new(palette: Palette) -> Self {
        Self { palette }
    }

    /// Map a role to its palette color.
    pub fn color_for(&self, role: TokenRole) -> Rgba {
        match role {
            TokenRole::Keyword     => self.palette.keyword,
            TokenRole::Ident       => self.palette.ident,
            TokenRole::StringLit   => self.palette.string_lit,
            TokenRole::NumberLit   => self.palette.number_lit,
            TokenRole::Operator    => self.palette.operator,
            TokenRole::Comment     => self.palette.comment,
            TokenRole::Punctuation => self.palette.punctuation,
            TokenRole::Unknown     => self.palette.default,
        }
    }

    /// Given a list of spans covering the full document, produce color runs.
    /// Adjacent runs with the same color are merged.
    pub fn color_runs(&self, spans: &[HighlightSpan]) -> Vec<ColorRun> {
        let mut runs: Vec<ColorRun> = Vec::with_capacity(spans.len());
        for span in spans {
            let color = self.color_for(span.role);
            if let Some(last) = runs.last_mut() {
                if last.color == color && last.byte_range.end == span.byte_range.start {
                    last.byte_range.end = span.byte_range.end;
                    continue;
                }
            }
            runs.push(ColorRun { byte_range: span.byte_range.clone(), color });
        }
        runs
    }
}

#[derive(Clone, Debug)]
pub struct ColorRun {
    pub byte_range: Range<usize>,
    pub color: Rgba,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_runs_merges_adjacent_same_color() {
        let h = Highlighter::new(Palette::default());
        let spans = vec![
            HighlightSpan { byte_range: 0..5,   role: TokenRole::Keyword },
            HighlightSpan { byte_range: 5..10,  role: TokenRole::Keyword },
            HighlightSpan { byte_range: 10..15, role: TokenRole::Ident },
        ];
        let runs = h.color_runs(&spans);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].byte_range, 0..10);
        assert_eq!(runs[1].byte_range, 10..15);
    }

    #[test]
    fn color_runs_does_not_merge_non_adjacent() {
        let h = Highlighter::new(Palette::default());
        let spans = vec![
            HighlightSpan { byte_range: 0..5,  role: TokenRole::Keyword },
            HighlightSpan { byte_range: 6..10, role: TokenRole::Keyword }, // gap at 5
        ];
        let runs = h.color_runs(&spans);
        assert_eq!(runs.len(), 2);
    }

    #[test]
    fn color_for_uses_palette() {
        let h = Highlighter::new(Palette::default());
        assert_eq!(h.color_for(TokenRole::Keyword), Rgba::hex(0x569cd6));
    }

    #[test]
    fn color_for_all_roles_no_panic() {
        let h = Highlighter::new(Palette::default());
        let roles = [
            TokenRole::Keyword, TokenRole::Ident, TokenRole::StringLit,
            TokenRole::NumberLit, TokenRole::Operator, TokenRole::Comment,
            TokenRole::Punctuation, TokenRole::Unknown,
        ];
        for role in roles {
            let _ = h.color_for(role);
        }
    }

    #[test]
    fn empty_spans_produces_empty_runs() {
        let h = Highlighter::new(Palette::default());
        assert!(h.color_runs(&[]).is_empty());
    }
}
