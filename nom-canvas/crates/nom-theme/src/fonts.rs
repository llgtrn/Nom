//! Font stack registry — metadata only.
//!
//! Actual byte loading + rasterization happens in nom-gpui via cosmic-text.
//! This module is the canonical list of families to try and in what order.
#![deny(unsafe_code)]

/// Font family identifier (CSS-style name).
pub type FamilyName = &'static str;

/// Semantic role for a font stack — used for lookup rather than brand names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontRole {
    UiSans,
    Monospace,
    UiSerif,
    ContentSans,
    ContentSerif,
    Emoji,
}

/// Ordered list of font families for a given role.
/// The first entry is the preferred primary; remaining entries are fallbacks.
#[derive(Clone, Debug, PartialEq)]
pub struct FontStack {
    pub role: FontRole,
    /// Ordered: [primary, fallback1, fallback2, ..., generic]
    pub families: Vec<FamilyName>,
}

impl FontStack {
    pub fn ui_sans() -> Self {
        FontStack {
            role: FontRole::UiSans,
            families: vec!["Inter", "Segoe UI", "San Francisco", "Roboto", "sans-serif"],
        }
    }

    pub fn monospace() -> Self {
        FontStack {
            role: FontRole::Monospace,
            families: vec![
                "Source Code Pro",
                "Cascadia Code",
                "Menlo",
                "Consolas",
                "monospace",
            ],
        }
    }

    pub fn ui_serif() -> Self {
        FontStack {
            role: FontRole::UiSerif,
            families: vec!["Iowan Old Style", "Georgia", "Times New Roman", "serif"],
        }
    }

    pub fn content_sans() -> Self {
        FontStack {
            role: FontRole::ContentSans,
            families: vec!["Inter", "Segoe UI", "San Francisco", "Roboto", "sans-serif"],
        }
    }

    pub fn content_serif() -> Self {
        FontStack {
            role: FontRole::ContentSerif,
            families: vec!["Iowan Old Style", "Georgia", "Times New Roman", "serif"],
        }
    }

    pub fn emoji() -> Self {
        FontStack {
            role: FontRole::Emoji,
            families: vec!["Segoe UI Emoji", "Apple Color Emoji", "Noto Color Emoji"],
        }
    }

    /// Returns the primary (most preferred) family name.
    pub fn primary(&self) -> FamilyName {
        self.families[0]
    }

    /// Returns the fallback families, excluding the primary.
    pub fn fallback_chain(&self) -> &[FamilyName] {
        &self.families[1..]
    }
}

/// Registry of default font stacks, keyed by role.
pub struct FontRegistry {
    stacks: Vec<FontStack>,
}

impl FontRegistry {
    pub fn new() -> Self {
        Self {
            stacks: vec![
                FontStack::ui_sans(),
                FontStack::monospace(),
                FontStack::ui_serif(),
                FontStack::content_sans(),
                FontStack::content_serif(),
                FontStack::emoji(),
            ],
        }
    }

    /// Look up a stack by role.
    pub fn get(&self, role: FontRole) -> Option<&FontStack> {
        self.stacks.iter().find(|s| s.role == role)
    }

    /// Replace the stack for a given role.
    pub fn override_role(&mut self, role: FontRole, stack: FontStack) {
        self.stacks.retain(|s| s.role != role);
        self.stacks.push(stack);
    }
}

impl Default for FontRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// CSS-compatible numeric font weight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontWeight {
    Thin = 100,
    ExtraLight = 200,
    Light = 300,
    Regular = 400,
    Medium = 500,
    SemiBold = 600,
    Bold = 700,
    ExtraBold = 800,
    Black = 900,
}

impl FontWeight {
    /// Returns the numeric weight value.
    pub fn as_u16(self) -> u16 {
        self as u16
    }

    /// Parse a CSS font-weight keyword or numeric string.
    ///
    /// Handles `"normal"` → `Regular`, `"bold"` → `Bold`,
    /// and `"100"` through `"900"` in multiples of 100.
    pub fn from_css_keyword(s: &str) -> Option<Self> {
        match s {
            "normal" => Some(FontWeight::Regular),
            "bold" => Some(FontWeight::Bold),
            _ => match s.parse::<u16>().ok()? {
                100 => Some(FontWeight::Thin),
                200 => Some(FontWeight::ExtraLight),
                300 => Some(FontWeight::Light),
                400 => Some(FontWeight::Regular),
                500 => Some(FontWeight::Medium),
                600 => Some(FontWeight::SemiBold),
                700 => Some(FontWeight::Bold),
                800 => Some(FontWeight::ExtraBold),
                900 => Some(FontWeight::Black),
                _ => None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_sans_primary_is_inter() {
        assert_eq!(FontStack::ui_sans().primary(), "Inter");
    }

    #[test]
    fn monospace_primary_is_source_code_pro() {
        assert_eq!(FontStack::monospace().primary(), "Source Code Pro");
    }

    #[test]
    fn emoji_has_at_least_three_entries() {
        assert!(FontStack::emoji().families.len() >= 3);
    }

    #[test]
    fn fallback_chain_excludes_primary() {
        let stack = FontStack::ui_sans();
        let chain = stack.fallback_chain();
        assert!(!chain.contains(&stack.primary()));
        assert_eq!(chain.len(), stack.families.len() - 1);
    }

    #[test]
    fn registry_has_all_six_roles() {
        let reg = FontRegistry::new();
        for role in [
            FontRole::UiSans,
            FontRole::Monospace,
            FontRole::UiSerif,
            FontRole::ContentSans,
            FontRole::ContentSerif,
            FontRole::Emoji,
        ] {
            assert!(reg.get(role).is_some(), "missing role {role:?}");
        }
    }

    #[test]
    fn registry_get_returns_some() {
        let reg = FontRegistry::new();
        assert!(reg.get(FontRole::Monospace).is_some());
    }

    #[test]
    fn registry_override_role_replaces() {
        let mut reg = FontRegistry::new();
        let custom = FontStack {
            role: FontRole::UiSans,
            families: vec!["Custom Font", "sans-serif"],
        };
        reg.override_role(FontRole::UiSans, custom);
        let found = reg.get(FontRole::UiSans).unwrap();
        assert_eq!(found.primary(), "Custom Font");
        // Only one entry for the role after override
        let count = reg.stacks.iter().filter(|s| s.role == FontRole::UiSans).count();
        assert_eq!(count, 1);
    }

    #[test]
    fn font_weight_regular_is_400() {
        assert_eq!(FontWeight::Regular.as_u16(), 400);
    }

    #[test]
    fn from_css_keyword_normal_is_regular() {
        assert_eq!(
            FontWeight::from_css_keyword("normal"),
            Some(FontWeight::Regular)
        );
    }

    #[test]
    fn from_css_keyword_bold_is_bold() {
        assert_eq!(
            FontWeight::from_css_keyword("bold"),
            Some(FontWeight::Bold)
        );
    }

    #[test]
    fn from_css_keyword_numeric_500_is_medium() {
        assert_eq!(
            FontWeight::from_css_keyword("500"),
            Some(FontWeight::Medium)
        );
    }

    #[test]
    fn from_css_keyword_bogus_is_none() {
        assert_eq!(FontWeight::from_css_keyword("bogus"), None);
    }

    #[test]
    fn content_sans_and_ui_sans_same_primary() {
        assert_eq!(
            FontStack::content_sans().primary(),
            FontStack::ui_sans().primary()
        );
    }

    #[test]
    fn content_serif_and_ui_serif_same_primary() {
        assert_eq!(
            FontStack::content_serif().primary(),
            FontStack::ui_serif().primary()
        );
    }
}
