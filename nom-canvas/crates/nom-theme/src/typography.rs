//! Typography scale: heading levels, line-heights, letter-spacing.
//!
//! Pairs with `tokens.rs` raw sizes and `fonts.rs` family/weight data.
//! Units: `size_px` + `line_height_px` are absolute pixels.
//! `letter_spacing_em` is fractional em (e.g. -0.01 = tighter).
#![deny(unsafe_code)]

use crate::fonts::FontWeight;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TypographyLevel {
    DisplayLarge,
    DisplayMedium,
    DisplaySmall,
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
    BodyLarge,
    Body,
    BodySmall,
    Caption,
    Code,
    CodeSmall,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TypographyStyle {
    pub level: TypographyLevel,
    pub size_px: f32,
    pub line_height_px: f32,
    pub letter_spacing_em: f32,
    pub weight: FontWeight,
}

impl TypographyStyle {
    pub const DISPLAY_LARGE: Self = Self {
        level: TypographyLevel::DisplayLarge,
        size_px: 57.0,
        line_height_px: 64.0,
        letter_spacing_em: -0.025,
        weight: FontWeight::Regular,
    };
    pub const DISPLAY_MEDIUM: Self = Self {
        level: TypographyLevel::DisplayMedium,
        size_px: 45.0,
        line_height_px: 52.0,
        letter_spacing_em: 0.0,
        weight: FontWeight::Regular,
    };
    pub const DISPLAY_SMALL: Self = Self {
        level: TypographyLevel::DisplaySmall,
        size_px: 36.0,
        line_height_px: 44.0,
        letter_spacing_em: 0.0,
        weight: FontWeight::Regular,
    };
    pub const H1: Self = Self {
        level: TypographyLevel::H1,
        size_px: 32.0,
        line_height_px: 40.0,
        letter_spacing_em: 0.0,
        weight: FontWeight::SemiBold,
    };
    pub const H2: Self = Self {
        level: TypographyLevel::H2,
        size_px: 28.0,
        line_height_px: 36.0,
        letter_spacing_em: 0.0,
        weight: FontWeight::SemiBold,
    };
    pub const H3: Self = Self {
        level: TypographyLevel::H3,
        size_px: 24.0,
        line_height_px: 32.0,
        letter_spacing_em: 0.0,
        weight: FontWeight::SemiBold,
    };
    pub const H4: Self = Self {
        level: TypographyLevel::H4,
        size_px: 22.0,
        line_height_px: 28.0,
        letter_spacing_em: 0.0,
        weight: FontWeight::SemiBold,
    };
    pub const H5: Self = Self {
        level: TypographyLevel::H5,
        size_px: 18.0,
        line_height_px: 24.0,
        letter_spacing_em: 0.0,
        weight: FontWeight::SemiBold,
    };
    pub const H6: Self = Self {
        level: TypographyLevel::H6,
        size_px: 16.0,
        line_height_px: 24.0,
        letter_spacing_em: 0.0,
        weight: FontWeight::SemiBold,
    };
    pub const BODY_LARGE: Self = Self {
        level: TypographyLevel::BodyLarge,
        size_px: 16.0,
        line_height_px: 24.0,
        letter_spacing_em: 0.0,
        weight: FontWeight::Regular,
    };
    pub const BODY: Self = Self {
        level: TypographyLevel::Body,
        size_px: 14.0,
        line_height_px: 20.0,
        letter_spacing_em: 0.0,
        weight: FontWeight::Regular,
    };
    pub const BODY_SMALL: Self = Self {
        level: TypographyLevel::BodySmall,
        size_px: 12.0,
        line_height_px: 16.0,
        letter_spacing_em: 0.01,
        weight: FontWeight::Regular,
    };
    pub const CAPTION: Self = Self {
        level: TypographyLevel::Caption,
        size_px: 11.0,
        line_height_px: 14.0,
        letter_spacing_em: 0.02,
        weight: FontWeight::Regular,
    };
    pub const CODE: Self = Self {
        level: TypographyLevel::Code,
        size_px: 13.0,
        line_height_px: 20.0,
        letter_spacing_em: 0.0,
        weight: FontWeight::Regular,
    };
    pub const CODE_SMALL: Self = Self {
        level: TypographyLevel::CodeSmall,
        size_px: 11.0,
        line_height_px: 16.0,
        letter_spacing_em: 0.0,
        weight: FontWeight::Regular,
    };

    pub const ALL: &'static [TypographyStyle] = &[
        Self::DISPLAY_LARGE,
        Self::DISPLAY_MEDIUM,
        Self::DISPLAY_SMALL,
        Self::H1,
        Self::H2,
        Self::H3,
        Self::H4,
        Self::H5,
        Self::H6,
        Self::BODY_LARGE,
        Self::BODY,
        Self::BODY_SMALL,
        Self::CAPTION,
        Self::CODE,
        Self::CODE_SMALL,
    ];

    pub fn by_level(level: TypographyLevel) -> Self {
        Self::ALL
            .iter()
            .find(|s| s.level == level)
            .copied()
            .expect("TypographyStyle::ALL covers every variant")
    }

    pub fn ratio(&self) -> f32 {
        self.line_height_px / self.size_px
    }

    pub fn letter_spacing_px(&self) -> f32 {
        self.size_px * self.letter_spacing_em
    }

    /// Whether this style is intended for monospace families (CODE*).
    pub fn is_monospace(&self) -> bool {
        matches!(
            self.level,
            TypographyLevel::Code | TypographyLevel::CodeSmall
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn display_large_size_is_57() {
        assert_eq!(TypographyStyle::DISPLAY_LARGE.size_px, 57.0);
    }

    #[test]
    fn h1_weight_is_semibold() {
        assert_eq!(TypographyStyle::H1.weight, FontWeight::SemiBold);
    }

    #[test]
    fn body_line_height_is_20() {
        assert_eq!(TypographyStyle::BODY.line_height_px, 20.0);
    }

    #[test]
    fn body_large_ratio_between_1_4_and_1_6() {
        let r = TypographyStyle::BODY_LARGE.ratio();
        assert!(r >= 1.4 && r <= 1.6, "ratio was {r}");
    }

    #[test]
    fn caption_letter_spacing_px_approx_0_22() {
        let px = TypographyStyle::CAPTION.letter_spacing_px();
        assert!((px - 0.22_f32).abs() < 0.01, "letter_spacing_px was {px}");
    }

    #[test]
    fn all_has_15_entries() {
        assert_eq!(TypographyStyle::ALL.len(), 15);
    }

    #[test]
    fn by_level_h3_returns_h3() {
        let s = TypographyStyle::by_level(TypographyLevel::H3);
        assert_eq!(s.level, TypographyLevel::H3);
        assert_eq!(s.size_px, 24.0);
    }

    #[test]
    fn by_level_code_is_monospace() {
        assert!(TypographyStyle::by_level(TypographyLevel::Code).is_monospace());
    }

    #[test]
    fn by_level_body_is_not_monospace() {
        assert!(!TypographyStyle::by_level(TypographyLevel::Body).is_monospace());
    }

    #[test]
    fn all_sizes_positive() {
        for s in TypographyStyle::ALL {
            assert!(s.size_px > 0.0, "size_px <= 0 for {:?}", s.level);
        }
    }

    #[test]
    fn all_line_heights_gte_sizes() {
        for s in TypographyStyle::ALL {
            assert!(
                s.line_height_px >= s.size_px,
                "line_height_px < size_px for {:?}",
                s.level
            );
        }
    }

    #[test]
    fn caption_letter_spacing_positive_and_below_0_05() {
        let em = TypographyStyle::CAPTION.letter_spacing_em;
        assert!(em > 0.0 && em < 0.05, "caption letter_spacing_em was {em}");
    }

    #[test]
    fn all_levels_are_distinct() {
        let levels: HashSet<_> = TypographyStyle::ALL.iter().map(|s| s.level).collect();
        assert_eq!(levels.len(), TypographyStyle::ALL.len());
    }
}
