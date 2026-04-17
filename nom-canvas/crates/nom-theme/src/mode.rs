use crate::{
    color::Hsla,
    tokens::{
        BORDER_PRIMARY, DARK_BORDER_PRIMARY, DARK_PRIMARY_BRAND, DARK_SURFACE_BACKGROUND,
        DARK_SURFACE_PRIMARY, DARK_SURFACE_SECONDARY, DARK_TEXT_PRIMARY, DARK_TEXT_SECONDARY,
        PRIMARY_BRAND, SURFACE_BACKGROUND, SURFACE_PRIMARY, SURFACE_SECONDARY, TEXT_PRIMARY,
        TEXT_SECONDARY,
    },
};

/// Light or dark rendering mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Light,
    Dark,
}

/// Resolved design tokens for one rendering mode.
///
/// All fields are `Hsla` so callers can convert to RGBA directly via
/// `Hsla::to_rgba()` without an additional lookup.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeTokens {
    pub text_primary: Hsla,
    pub text_secondary: Hsla,
    pub surface_background: Hsla,
    pub surface_primary: Hsla,
    pub surface_secondary: Hsla,
    pub border_primary: Hsla,
    pub primary_brand: Hsla,
}

/// A complete theme combining mode + resolved tokens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub mode: ThemeMode,
    pub tokens: ThemeTokens,
}

impl Theme {
    /// Construct the light-mode theme.
    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
            tokens: ThemeTokens {
                text_primary: Hsla::from_tuple(TEXT_PRIMARY),
                text_secondary: Hsla::from_tuple(TEXT_SECONDARY),
                surface_background: Hsla::from_tuple(SURFACE_BACKGROUND),
                surface_primary: Hsla::from_tuple(SURFACE_PRIMARY),
                surface_secondary: Hsla::from_tuple(SURFACE_SECONDARY),
                border_primary: Hsla::from_tuple(BORDER_PRIMARY),
                primary_brand: Hsla::from_tuple(PRIMARY_BRAND),
            },
        }
    }

    /// Construct the dark-mode theme.
    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            tokens: ThemeTokens {
                text_primary: Hsla::from_tuple(DARK_TEXT_PRIMARY),
                text_secondary: Hsla::from_tuple(DARK_TEXT_SECONDARY),
                surface_background: Hsla::from_tuple(DARK_SURFACE_BACKGROUND),
                surface_primary: Hsla::from_tuple(DARK_SURFACE_PRIMARY),
                surface_secondary: Hsla::from_tuple(DARK_SURFACE_SECONDARY),
                border_primary: Hsla::from_tuple(DARK_BORDER_PRIMARY),
                primary_brand: Hsla::from_tuple(DARK_PRIMARY_BRAND),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_and_dark_modes_are_distinct() {
        let light = Theme::light();
        let dark = Theme::dark();
        assert_ne!(light.mode, dark.mode);
        // The primary backgrounds must differ between modes.
        assert_ne!(
            light.tokens.surface_background,
            dark.tokens.surface_background
        );
    }

    #[test]
    fn light_primary_background_is_brighter_than_dark() {
        let light = Theme::light();
        let dark = Theme::dark();
        // Light mode: high lightness; dark mode: low lightness.
        assert!(
            light.tokens.surface_background.l > dark.tokens.surface_background.l,
            "light surface_background should have higher lightness than dark"
        );
    }

    #[test]
    fn theme_mode_enum_values() {
        assert_eq!(ThemeMode::Light, ThemeMode::Light);
        assert_ne!(ThemeMode::Light, ThemeMode::Dark);
    }

    #[test]
    fn brand_color_is_present_in_both_modes() {
        // brand hue must stay in the blue range (0.55–0.65) in both modes
        let light = Theme::light();
        let dark = Theme::dark();
        assert!((0.55..=0.65).contains(&light.tokens.primary_brand.h));
        assert!((0.55..=0.65).contains(&dark.tokens.primary_brand.h));
    }
}
