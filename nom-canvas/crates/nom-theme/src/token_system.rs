/// A single color token in the design system.
#[derive(Debug, Clone)]
pub struct ColorToken {
    pub name: String,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl ColorToken {
    pub fn new(name: impl Into<String>, r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { name: name.into(), r, g, b, a }
    }

    /// Format as "#RRGGBB" (alpha ignored).
    pub fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// Perceived luminance in [0, 1].
    pub fn luminance(&self) -> f32 {
        0.299 * (self.r as f32 / 255.0)
            + 0.587 * (self.g as f32 / 255.0)
            + 0.114 * (self.b as f32 / 255.0)
    }

    /// Returns `true` when luminance < 0.5.
    pub fn is_dark(&self) -> bool {
        self.luminance() < 0.5
    }
}

/// A single spacing token in the design system.
#[derive(Debug, Clone)]
pub struct SpacingToken {
    pub name: String,
    pub value_px: u32,
}

impl SpacingToken {
    pub fn new(name: impl Into<String>, value_px: u32) -> Self {
        Self { name: name.into(), value_px }
    }

    /// Scale the spacing value by `factor`, rounding toward zero.
    pub fn scale(&self, factor: f32) -> u32 {
        (self.value_px as f32 * factor) as u32
    }
}

/// A collection of color and spacing tokens for the design system.
#[derive(Debug, Clone, Default)]
pub struct DesignSystemTokens {
    pub colors: Vec<ColorToken>,
    pub spacing: Vec<SpacingToken>,
}

impl DesignSystemTokens {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_color(&mut self, token: ColorToken) {
        self.colors.push(token);
    }

    pub fn add_spacing(&mut self, token: SpacingToken) {
        self.spacing.push(token);
    }

    pub fn find_color(&self, name: &str) -> Option<&ColorToken> {
        self.colors.iter().find(|t| t.name == name)
    }

    pub fn find_spacing(&self, name: &str) -> Option<&SpacingToken> {
        self.spacing.iter().find(|t| t.name == name)
    }

    /// Seed the Nom default token set (6 colors + 4 spacing).
    pub fn seed_nom_tokens() -> Self {
        let mut ds = Self::new();

        ds.add_color(ColorToken::new("background", 18, 18, 18, 255));
        ds.add_color(ColorToken::new("surface", 28, 28, 28, 255));
        ds.add_color(ColorToken::new("primary", 99, 102, 241, 255));
        ds.add_color(ColorToken::new("text", 240, 240, 240, 255));
        ds.add_color(ColorToken::new("muted", 120, 120, 120, 255));
        ds.add_color(ColorToken::new("accent", 56, 189, 248, 255));

        ds.add_spacing(SpacingToken::new("xs", 4));
        ds.add_spacing(SpacingToken::new("sm", 8));
        ds.add_spacing(SpacingToken::new("md", 16));
        ds.add_spacing(SpacingToken::new("lg", 32));

        ds
    }

    /// Returns `true` when the absolute luminance difference is >= 0.3.
    pub fn validate_contrast(bg: &ColorToken, fg: &ColorToken) -> bool {
        (bg.luminance() - fg.luminance()).abs() >= 0.3
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod token_system_tests {
    use super::*;

    #[test]
    fn color_token_to_hex_format() {
        let c = ColorToken::new("test", 0xFF, 0xA5, 0x00, 255);
        assert_eq!(c.to_hex(), "#FFA500");
    }

    #[test]
    fn color_token_is_dark_for_dark_color() {
        let c = ColorToken::new("background", 18, 18, 18, 255);
        assert!(c.is_dark());
    }

    #[test]
    fn spacing_token_scale_calculation() {
        let s = SpacingToken::new("md", 16);
        assert_eq!(s.scale(2.0), 32);
        assert_eq!(s.scale(0.5), 8);
    }

    #[test]
    fn design_system_find_color_found() {
        let ds = DesignSystemTokens::seed_nom_tokens();
        let c = ds.find_color("primary");
        assert!(c.is_some());
        assert_eq!(c.unwrap().name, "primary");
    }

    #[test]
    fn design_system_find_color_not_found() {
        let ds = DesignSystemTokens::seed_nom_tokens();
        assert!(ds.find_color("nonexistent").is_none());
    }

    #[test]
    fn seed_nom_tokens_has_6_colors() {
        let ds = DesignSystemTokens::seed_nom_tokens();
        assert_eq!(ds.colors.len(), 6);
    }

    #[test]
    fn seed_nom_tokens_has_4_spacing_tokens() {
        let ds = DesignSystemTokens::seed_nom_tokens();
        assert_eq!(ds.spacing.len(), 4);
    }

    #[test]
    fn validate_contrast_sufficient_contrast() {
        let bg = ColorToken::new("background", 18, 18, 18, 255);
        let fg = ColorToken::new("text", 240, 240, 240, 255);
        assert!(DesignSystemTokens::validate_contrast(&bg, &fg));
    }

    #[test]
    fn validate_contrast_insufficient_contrast() {
        // muted gray vs slightly lighter gray: luminance diff ~0.12, below 0.3 threshold
        let bg = ColorToken::new("muted", 120, 120, 120, 255);
        let fg = ColorToken::new("near-muted", 150, 150, 150, 255);
        assert!(!DesignSystemTokens::validate_contrast(&bg, &fg));
    }
}
