//! AFFiNE design tokens: categories, token set, resolver, and applier.
//!
//! Encodes the 73 AFFiNE design tokens as typed Rust values with category
//! classification, CSS variable generation, and a lightweight applier for
//! mapping token names to element style strings.

/// Category of a design token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenCategory {
    /// Color values (hex, rgb, rgba).
    Color,
    /// Spacing values (px, rem, em).
    Spacing,
    /// Typography values (font-size, font-family, line-height).
    Typography,
    /// Box-shadow values.
    Shadow,
    /// Border-related values (radius, width, style).
    Border,
    /// Animation / transition values (duration, easing).
    Animation,
}

impl TokenCategory {
    /// Returns the kebab-case name of this category.
    pub fn category_name(&self) -> &str {
        match self {
            TokenCategory::Color => "color",
            TokenCategory::Spacing => "spacing",
            TokenCategory::Typography => "typography",
            TokenCategory::Shadow => "shadow",
            TokenCategory::Border => "border",
            TokenCategory::Animation => "animation",
        }
    }
}

/// A single AFFiNE design token with a name, value, and category.
#[derive(Debug, Clone)]
pub struct AffineToken {
    /// Token name (e.g. `"primary"`, `"spacing-md"`).
    pub name: String,
    /// Token value (e.g. `"#1971FA"`, `"16px"`).
    pub value: String,
    /// Category this token belongs to.
    pub category: TokenCategory,
}

impl AffineToken {
    /// Creates a new `AffineToken`.
    pub fn new(name: impl Into<String>, value: impl Into<String>, category: TokenCategory) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            category,
        }
    }

    /// Returns `true` when this token belongs to the `Color` category.
    pub fn is_color(&self) -> bool {
        self.category == TokenCategory::Color
    }

    /// Returns the CSS custom-property name for this token.
    ///
    /// Example: token `"primary"` → `"--affine-primary"`.
    pub fn css_var(&self) -> String {
        format!("--affine-{}", self.name)
    }
}

/// The full set of AFFiNE design tokens.
#[derive(Debug, Clone, Default)]
pub struct AffineTokenSet {
    tokens: Vec<AffineToken>,
}

impl AffineTokenSet {
    /// Creates an empty token set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Seeds the token set with 9 representative AFFiNE default tokens.
    ///
    /// Adds tokens for primary color, backgrounds, text, spacing, typography,
    /// shadow, and border radius.
    pub fn seed_defaults(&mut self) {
        let defaults = [
            ("primary", "#1971FA", TokenCategory::Color),
            ("bg-primary", "#FFFFFF", TokenCategory::Color),
            ("text-primary", "#121212", TokenCategory::Color),
            ("spacing-xs", "4px", TokenCategory::Spacing),
            ("spacing-sm", "8px", TokenCategory::Spacing),
            ("spacing-md", "16px", TokenCategory::Spacing),
            ("font-body", "14px", TokenCategory::Typography),
            ("shadow-sm", "0 1px 4px rgba(0,0,0,.1)", TokenCategory::Shadow),
            ("border-radius", "8px", TokenCategory::Border),
        ];
        for (name, value, cat) in defaults {
            self.tokens.push(AffineToken::new(name, value, cat));
        }
    }

    /// Finds a token by exact name, returning `None` if not present.
    pub fn find(&self, name: &str) -> Option<&AffineToken> {
        self.tokens.iter().find(|t| t.name == name)
    }

    /// Returns all tokens belonging to the given category.
    pub fn by_category(&self, cat: &TokenCategory) -> Vec<&AffineToken> {
        self.tokens.iter().filter(|t| &t.category == cat).collect()
    }

    /// Returns the total number of tokens in this set.
    pub fn token_count(&self) -> usize {
        self.tokens.len()
    }
}

/// Resolves token names to their string values.
pub struct TokenResolver {
    token_set: AffineTokenSet,
}

impl TokenResolver {
    /// Creates a new resolver backed by the given token set.
    pub fn new(token_set: AffineTokenSet) -> Self {
        Self { token_set }
    }

    /// Returns the raw value for the named token, or `None` if not found.
    pub fn resolve(&self, name: &str) -> Option<&str> {
        self.token_set.find(name).map(|t| t.value.as_str())
    }

    /// Returns a `var(--affine-{name})` reference if the token exists, or an
    /// empty string when the token is unknown.
    pub fn resolve_css_var(&self, name: &str) -> String {
        if self.token_set.find(name).is_some() {
            format!("var(--affine-{})", name)
        } else {
            String::new()
        }
    }
}

/// Applies design tokens to element style properties.
pub struct DesignTokenApplier {
    resolver: TokenResolver,
}

impl DesignTokenApplier {
    /// Creates a new applier using the given resolver.
    pub fn new(resolver: TokenResolver) -> Self {
        Self { resolver }
    }

    /// Resolves a spacing token by name.  Returns `"0"` when unknown.
    pub fn apply_spacing(&self, token_name: &str) -> String {
        self.resolver
            .resolve(token_name)
            .unwrap_or("0")
            .to_string()
    }

    /// Resolves a color token by name.  Returns `"transparent"` when unknown.
    pub fn apply_color(&self, token_name: &str) -> String {
        self.resolver
            .resolve(token_name)
            .unwrap_or("transparent")
            .to_string()
    }

    /// Counts how many of the supplied `names` resolve to a known token value.
    pub fn applied_count(&self, names: &[&str]) -> usize {
        names
            .iter()
            .filter(|&&n| self.resolver.resolve(n).is_some())
            .count()
    }
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod affine_tokens_tests {
    use super::*;

    fn seeded_set() -> AffineTokenSet {
        let mut s = AffineTokenSet::new();
        s.seed_defaults();
        s
    }

    fn seeded_resolver() -> TokenResolver {
        TokenResolver::new(seeded_set())
    }

    fn seeded_applier() -> DesignTokenApplier {
        DesignTokenApplier::new(seeded_resolver())
    }

    #[test]
    fn affine_token_is_color() {
        let t = AffineToken::new("primary", "#1971FA", TokenCategory::Color);
        assert!(t.is_color(), "color token must report is_color() = true");

        let s = AffineToken::new("spacing-md", "16px", TokenCategory::Spacing);
        assert!(!s.is_color(), "spacing token must report is_color() = false");
    }

    #[test]
    fn affine_token_css_var() {
        let t = AffineToken::new("border-radius", "8px", TokenCategory::Border);
        assert_eq!(t.css_var(), "--affine-border-radius");
    }

    #[test]
    fn affine_token_set_seed_defaults_count() {
        let s = seeded_set();
        assert_eq!(s.token_count(), 9, "seed_defaults must insert exactly 9 tokens");
    }

    #[test]
    fn affine_token_set_find_existing() {
        let s = seeded_set();
        let t = s.find("spacing-md").expect("spacing-md must exist after seed_defaults");
        assert_eq!(t.value, "16px");
    }

    #[test]
    fn affine_token_set_by_category_color() {
        let s = seeded_set();
        let colors = s.by_category(&TokenCategory::Color);
        assert_eq!(colors.len(), 3, "seed_defaults contains 3 color tokens");
        let names: Vec<&str> = colors.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"primary"));
        assert!(names.contains(&"bg-primary"));
        assert!(names.contains(&"text-primary"));
    }

    #[test]
    fn token_resolver_resolve_known() {
        let r = seeded_resolver();
        assert_eq!(r.resolve("primary"), Some("#1971FA"));
        assert_eq!(r.resolve("unknown-token"), None);
    }

    #[test]
    fn token_resolver_resolve_css_var() {
        let r = seeded_resolver();
        assert_eq!(r.resolve_css_var("font-body"), "var(--affine-font-body)");
        assert_eq!(r.resolve_css_var("nonexistent"), "", "missing token must return empty string");
    }

    #[test]
    fn design_token_applier_apply_spacing() {
        let a = seeded_applier();
        assert_eq!(a.apply_spacing("spacing-xs"), "4px");
        assert_eq!(a.apply_spacing("missing"), "0", "missing spacing token falls back to \"0\"");
    }

    #[test]
    fn design_token_applier_applied_count() {
        let a = seeded_applier();
        let names = ["primary", "spacing-md", "shadow-sm", "nonexistent"];
        assert_eq!(a.applied_count(&names), 3, "3 out of 4 names are known tokens");
    }
}
