/// Font composition primitives for nom-compose.

// ---------------------------------------------------------------------------
// FontStyle
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

impl FontStyle {
    /// Returns the CSS keyword for this style.
    pub fn css_value(&self) -> &'static str {
        match self {
            FontStyle::Normal => "normal",
            FontStyle::Italic => "italic",
            FontStyle::Oblique => "oblique",
        }
    }

    /// Returns `true` only for the upright (Normal) variant.
    pub fn is_upright(&self) -> bool {
        matches!(self, FontStyle::Normal)
    }
}

// ---------------------------------------------------------------------------
// FontWeight
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FontWeight(pub u16);

impl FontWeight {
    /// Returns `true` when the weight is bold (>= 600).
    pub fn is_bold(&self) -> bool {
        self.0 >= 600
    }

    /// Returns `true` when the weight is thin (<= 300).
    pub fn is_thin(&self) -> bool {
        self.0 <= 300
    }

    /// Returns a new `FontWeight` clamped to the valid range 100..=900.
    pub fn clamped(&self) -> FontWeight {
        FontWeight(self.0.max(100).min(900))
    }
}

// ---------------------------------------------------------------------------
// FontSpec
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct FontSpec {
    pub family: String,
    pub size_px: f32,
    pub weight: FontWeight,
    pub style: FontStyle,
}

impl FontSpec {
    /// Returns a CSS font shorthand string: `"{style} {weight} {size}px {family}"`.
    pub fn css_shorthand(&self) -> String {
        format!(
            "{} {} {}px {}",
            self.style.css_value(),
            self.weight.0,
            self.size_px,
            self.family
        )
    }

    /// Returns `true` when `size_px` is considered display-scale (>= 24.0).
    pub fn is_display_size(&self) -> bool {
        self.size_px >= 24.0
    }
}

// ---------------------------------------------------------------------------
// FontFamily
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct FontFamily {
    pub name: String,
    pub fallbacks: Vec<String>,
}

impl FontFamily {
    /// Builds the full CSS font-family stack: primary name followed by fallbacks,
    /// separated by `", "`.
    pub fn stack(&self) -> String {
        if self.fallbacks.is_empty() {
            return self.name.clone();
        }
        let mut parts = Vec::with_capacity(1 + self.fallbacks.len());
        parts.push(self.name.clone());
        parts.extend(self.fallbacks.iter().cloned());
        parts.join(", ")
    }

    /// Returns `true` when `name` is present in the fallback list.
    pub fn has_fallback(&self, name: &str) -> bool {
        self.fallbacks.iter().any(|f| f == name)
    }
}

// ---------------------------------------------------------------------------
// FontComposer
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct FontComposer {
    pub specs: Vec<FontSpec>,
}

impl FontComposer {
    pub fn new() -> Self {
        FontComposer { specs: Vec::new() }
    }

    /// Appends a `FontSpec` to the composer.
    pub fn add(&mut self, spec: FontSpec) {
        self.specs.push(spec);
    }

    /// Returns references to every spec whose `size_px` qualifies as display size.
    pub fn display_specs(&self) -> Vec<&FontSpec> {
        self.specs.iter().filter(|s| s.is_display_size()).collect()
    }

    /// Returns the total number of registered specs.
    pub fn spec_count(&self) -> usize {
        self.specs.len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // 1. FontStyle::css_value returns correct CSS keywords for all variants.
    #[test]
    fn font_style_css_value() {
        assert_eq!(FontStyle::Normal.css_value(), "normal");
        assert_eq!(FontStyle::Italic.css_value(), "italic");
        assert_eq!(FontStyle::Oblique.css_value(), "oblique");
    }

    // 2. FontStyle::is_upright is true only for Normal.
    #[test]
    fn font_style_is_upright() {
        assert!(FontStyle::Normal.is_upright());
        assert!(!FontStyle::Italic.is_upright());
        assert!(!FontStyle::Oblique.is_upright());
    }

    // 3. FontWeight::is_bold threshold at 600.
    #[test]
    fn font_weight_is_bold() {
        assert!(!FontWeight(599).is_bold());
        assert!(FontWeight(600).is_bold());
        assert!(FontWeight(700).is_bold());
        assert!(FontWeight(900).is_bold());
    }

    // 4. FontWeight::clamped keeps values inside 100..=900.
    #[test]
    fn font_weight_clamped_bounds() {
        assert_eq!(FontWeight(0).clamped(), FontWeight(100));
        assert_eq!(FontWeight(50).clamped(), FontWeight(100));
        assert_eq!(FontWeight(400).clamped(), FontWeight(400));
        assert_eq!(FontWeight(950).clamped(), FontWeight(900));
        assert_eq!(FontWeight(1200).clamped(), FontWeight(900));
    }

    // 5. FontSpec::css_shorthand produces the expected format string.
    #[test]
    fn font_spec_css_shorthand_format() {
        let spec = FontSpec {
            family: "Inter".to_string(),
            size_px: 16.0,
            weight: FontWeight(400),
            style: FontStyle::Normal,
        };
        let shorthand = spec.css_shorthand();
        assert_eq!(shorthand, "normal 400 16px Inter");
    }

    // 6. FontSpec::is_display_size boundary at 24.0 px.
    #[test]
    fn font_spec_is_display_size() {
        let make = |px: f32| FontSpec {
            family: "Test".to_string(),
            size_px: px,
            weight: FontWeight(400),
            style: FontStyle::Normal,
        };
        assert!(!make(23.9).is_display_size());
        assert!(make(24.0).is_display_size());
        assert!(make(48.0).is_display_size());
    }

    // 7. FontFamily::stack joins primary name and fallbacks correctly.
    #[test]
    fn font_family_stack() {
        let family = FontFamily {
            name: "Inter".to_string(),
            fallbacks: vec!["Helvetica Neue".to_string(), "sans-serif".to_string()],
        };
        assert_eq!(family.stack(), "Inter, Helvetica Neue, sans-serif");

        let solo = FontFamily {
            name: "Mono".to_string(),
            fallbacks: vec![],
        };
        assert_eq!(solo.stack(), "Mono");
    }

    // 8. FontFamily::has_fallback detects presence/absence correctly.
    #[test]
    fn font_family_has_fallback() {
        let family = FontFamily {
            name: "Inter".to_string(),
            fallbacks: vec!["Arial".to_string(), "sans-serif".to_string()],
        };
        assert!(family.has_fallback("Arial"));
        assert!(family.has_fallback("sans-serif"));
        assert!(!family.has_fallback("Inter")); // primary name is not a fallback
        assert!(!family.has_fallback("Comic Sans MS"));
    }

    // 9. FontComposer::display_specs filters correctly.
    #[test]
    fn font_composer_display_specs_filter() {
        let mut composer = FontComposer::new();
        composer.add(FontSpec {
            family: "Small".to_string(),
            size_px: 12.0,
            weight: FontWeight(400),
            style: FontStyle::Normal,
        });
        composer.add(FontSpec {
            family: "Heading".to_string(),
            size_px: 32.0,
            weight: FontWeight(700),
            style: FontStyle::Normal,
        });
        composer.add(FontSpec {
            family: "Display".to_string(),
            size_px: 24.0,
            weight: FontWeight(500),
            style: FontStyle::Italic,
        });

        assert_eq!(composer.spec_count(), 3);
        let display = composer.display_specs();
        assert_eq!(display.len(), 2);
        assert!(display.iter().any(|s| s.family == "Heading"));
        assert!(display.iter().any(|s| s.family == "Display"));
        assert!(!display.iter().any(|s| s.family == "Small"));
    }
}
