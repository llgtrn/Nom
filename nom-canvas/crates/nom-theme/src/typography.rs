#![deny(unsafe_code)]

/// Logical font families in the Nom design system.
#[derive(Debug, Clone, PartialEq)]
pub enum FontFamily {
    /// Serif prose family — Libre Baskerville with EB Garamond fallback.
    Prose,
    /// Monospace code family — Berkeley Mono with JetBrains Mono fallback.
    Code,
    /// UI sans-serif family — Inter with system-ui fallback.
    Ui,
}

impl FontFamily {
    /// Primary font name for this family.
    pub fn primary_name(&self) -> &'static str {
        match self {
            FontFamily::Prose => "Libre Baskerville",
            FontFamily::Code => "Berkeley Mono",
            FontFamily::Ui => "Inter",
        }
    }

    /// Fallback font name for this family.
    pub fn fallback_name(&self) -> &'static str {
        match self {
            FontFamily::Prose => "EB Garamond",
            FontFamily::Code => "JetBrains Mono",
            FontFamily::Ui => "system-ui",
        }
    }

    /// Returns `true` if this family is a serif typeface.
    pub fn is_serif(&self) -> bool {
        matches!(self, FontFamily::Prose)
    }

    /// Returns `true` if this family is a monospace typeface.
    pub fn is_monospace(&self) -> bool {
        matches!(self, FontFamily::Code)
    }
}

/// Design system font size scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FontSize {
    /// 11 px
    Xs,
    /// 12 px
    Sm,
    /// 13 px — base (1.0 rem)
    Base,
    /// 15 px
    Md,
    /// 18 px
    Lg,
    /// 24 px
    Xl,
    /// 32 px
    Xl2,
}

impl FontSize {
    /// Pixel value for this size step.
    pub fn px(self) -> u32 {
        match self {
            FontSize::Xs => 11,
            FontSize::Sm => 12,
            FontSize::Base => 13,
            FontSize::Md => 15,
            FontSize::Lg => 18,
            FontSize::Xl => 24,
            FontSize::Xl2 => 32,
        }
    }

    /// Rem value relative to `Base` (13 px = 1.0 rem).
    pub fn rem(self) -> f32 {
        self.px() as f32 / 13.0
    }
}

/// Full typography configuration for the design system.
pub struct TypographyScale {
    /// Font family used for prose / long-form reading.
    pub prose: FontFamily,
    /// Font family used for code / monospace contexts.
    pub code: FontFamily,
    /// Font family used for UI chrome and labels.
    pub ui: FontFamily,
    /// Nominal base font size.
    pub base_size: FontSize,
}

impl TypographyScale {
    /// Create the default typography scale.
    pub fn new() -> Self {
        Self {
            prose: FontFamily::Prose,
            code: FontFamily::Code,
            ui: FontFamily::Ui,
            base_size: FontSize::Base,
        }
    }

    /// Pixel size used for prose text (Md = 15 px).
    pub fn prose_px(&self) -> u32 {
        FontSize::Md.px()
    }

    /// Pixel size used for code text (Base = 13 px).
    pub fn code_px(&self) -> u32 {
        FontSize::Base.px()
    }

    /// Pixel size used for UI text (Base = 13 px).
    pub fn ui_px(&self) -> u32 {
        FontSize::Base.px()
    }
}

impl Default for TypographyScale {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_family_prose_name() {
        assert_eq!(FontFamily::Prose.primary_name(), "Libre Baskerville");
        assert_eq!(FontFamily::Prose.fallback_name(), "EB Garamond");
    }

    #[test]
    fn font_family_code_is_monospace() {
        assert!(FontFamily::Code.is_monospace());
        assert!(!FontFamily::Prose.is_monospace());
        assert!(!FontFamily::Ui.is_monospace());
    }

    #[test]
    fn font_family_prose_is_serif() {
        assert!(FontFamily::Prose.is_serif());
        assert!(!FontFamily::Code.is_serif());
        assert!(!FontFamily::Ui.is_serif());
    }

    #[test]
    fn font_size_px_base_is_13() {
        assert_eq!(FontSize::Base.px(), 13);
    }

    #[test]
    fn font_size_rem_base_is_1() {
        let rem = FontSize::Base.rem();
        assert!((rem - 1.0).abs() < f32::EPSILON, "Base rem must be 1.0, got {rem}");
    }

    #[test]
    fn font_size_all_values() {
        assert_eq!(FontSize::Xs.px(), 11);
        assert_eq!(FontSize::Sm.px(), 12);
        assert_eq!(FontSize::Base.px(), 13);
        assert_eq!(FontSize::Md.px(), 15);
        assert_eq!(FontSize::Lg.px(), 18);
        assert_eq!(FontSize::Xl.px(), 24);
        assert_eq!(FontSize::Xl2.px(), 32);
    }

    #[test]
    fn typography_scale_default() {
        let scale = TypographyScale::default();
        assert_eq!(scale.prose, FontFamily::Prose);
        assert_eq!(scale.code, FontFamily::Code);
        assert_eq!(scale.ui, FontFamily::Ui);
        assert_eq!(scale.base_size, FontSize::Base);
    }

    #[test]
    fn typography_scale_prose_md_is_15() {
        let scale = TypographyScale::new();
        assert_eq!(scale.prose_px(), 15);
        assert_eq!(scale.code_px(), 13);
        assert_eq!(scale.ui_px(), 13);
    }
}
