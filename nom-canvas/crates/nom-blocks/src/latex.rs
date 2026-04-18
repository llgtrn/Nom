//! LaTeX block: stores and validates a LaTeX math/formula source string.
#![deny(unsafe_code)]
use crate::block_model::NomtuRef;
use serde::{Deserialize, Serialize};

/// A block holding a LaTeX formula or document fragment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LatexBlock {
    /// DB entity reference (NON-OPTIONAL).
    pub entity: NomtuRef,
    /// Raw LaTeX source string.
    pub source: String,
    /// When `true` the formula is rendered in display (block) mode; otherwise inline.
    pub display_mode: bool,
}

impl LatexBlock {
    /// Construct a new [`LatexBlock`] with the given entity and LaTeX source.
    pub fn new(entity: NomtuRef, source: impl Into<String>) -> Self {
        Self {
            entity,
            source: source.into(),
            display_mode: false,
        }
    }

    /// Switch between display (block) and inline rendering mode.
    pub fn set_display_mode(&mut self, display: bool) {
        self.display_mode = display;
    }

    /// Return `true` when `source` is empty or whitespace only.
    pub fn is_empty(&self) -> bool {
        self.source.trim().is_empty()
    }

    /// Count non-empty lines in `source`.
    pub fn line_count(&self) -> usize {
        self.source.lines().filter(|l| !l.trim().is_empty()).count()
    }

    /// Return `Ok(())` if every `{` in `source` is matched by a closing `}`.
    /// Returns `Err` with a description when braces are unbalanced.
    pub fn validate_balanced_braces(&self) -> Result<(), String> {
        let mut depth: i64 = 0;
        for (i, ch) in self.source.chars().enumerate() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth < 0 {
                        return Err(format!(
                            "Unexpected '}}' at char index {i}: more closing than opening braces"
                        ));
                    }
                }
                _ => {}
            }
        }
        if depth != 0 {
            Err(format!(
                "Unbalanced braces: {depth} unclosed opening brace(s)"
            ))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(id: &str) -> NomtuRef {
        NomtuRef::new(id, "latex", "concept")
    }

    #[test]
    fn latex_new_stores_source() {
        let b = LatexBlock::new(entity("l1"), r"\frac{a}{b}");
        assert_eq!(b.source, r"\frac{a}{b}");
    }

    #[test]
    fn latex_default_display_mode_false() {
        let b = LatexBlock::new(entity("l2"), "x^2");
        assert!(!b.display_mode);
    }

    #[test]
    fn latex_entity_non_optional() {
        let b = LatexBlock::new(entity("eid-latex"), "x");
        assert_eq!(b.entity.id, "eid-latex");
        assert!(!b.entity.id.is_empty());
    }

    #[test]
    fn latex_set_display_mode_true() {
        let mut b = LatexBlock::new(entity("l3"), "x^2");
        b.set_display_mode(true);
        assert!(b.display_mode);
    }

    #[test]
    fn latex_set_display_mode_false() {
        let mut b = LatexBlock::new(entity("l4"), "x^2");
        b.set_display_mode(true);
        b.set_display_mode(false);
        assert!(!b.display_mode);
    }

    #[test]
    fn latex_is_empty_true_blank() {
        let b = LatexBlock::new(entity("l5"), "   ");
        assert!(b.is_empty());
    }

    #[test]
    fn latex_is_empty_false_with_content() {
        let b = LatexBlock::new(entity("l6"), r"\alpha");
        assert!(!b.is_empty());
    }

    #[test]
    fn latex_line_count_basic() {
        let b = LatexBlock::new(entity("l7"), "line one\nline two\nline three");
        assert_eq!(b.line_count(), 3);
    }

    #[test]
    fn latex_line_count_skips_blank_lines() {
        let b = LatexBlock::new(entity("l8"), "a\n\nb\n\nc");
        assert_eq!(b.line_count(), 3);
    }

    #[test]
    fn latex_line_count_empty_source() {
        let b = LatexBlock::new(entity("l9"), "");
        assert_eq!(b.line_count(), 0);
    }

    #[test]
    fn latex_validate_balanced_braces_ok() {
        let b = LatexBlock::new(entity("l10"), r"\frac{1}{2}");
        assert!(b.validate_balanced_braces().is_ok());
    }

    #[test]
    fn latex_validate_balanced_braces_empty_ok() {
        let b = LatexBlock::new(entity("l11"), "");
        assert!(b.validate_balanced_braces().is_ok());
    }

    #[test]
    fn latex_validate_balanced_braces_missing_close() {
        let b = LatexBlock::new(entity("l12"), r"\frac{1}{2");
        assert!(b.validate_balanced_braces().is_err());
    }

    #[test]
    fn latex_validate_balanced_braces_extra_close() {
        let b = LatexBlock::new(entity("l13"), r"\frac{1}}");
        assert!(b.validate_balanced_braces().is_err());
    }

    #[test]
    fn latex_validate_balanced_braces_nested_ok() {
        let b = LatexBlock::new(entity("l14"), r"\sqrt{\frac{a}{b^{2}}}");
        assert!(b.validate_balanced_braces().is_ok());
    }

    #[test]
    fn latex_validate_balanced_braces_only_open() {
        let b = LatexBlock::new(entity("l15"), "{{{");
        let result = b.validate_balanced_braces();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("3"), "error should mention 3 unclosed braces, got: {msg}");
    }

    #[test]
    fn latex_validate_balanced_braces_only_close() {
        let b = LatexBlock::new(entity("l16"), "}}}");
        assert!(b.validate_balanced_braces().is_err());
    }

    #[test]
    fn latex_validate_balanced_braces_interleaved_ok() {
        // {a{b}c} — fully balanced
        let b = LatexBlock::new(entity("l17"), "{a{b}c}");
        assert!(b.validate_balanced_braces().is_ok());
    }

    // ── wave AB: additional latex tests ─────────────────────────────────────

    /// line_count of a single-line formula is 1.
    #[test]
    fn latex_line_count_single_line_is_1() {
        let b = LatexBlock::new(entity("l-sl"), r"\frac{a}{b}");
        assert_eq!(b.line_count(), 1);
    }

    /// Balanced braces in \frac{a}{b} pass validation.
    #[test]
    fn latex_frac_balanced_braces_pass() {
        let b = LatexBlock::new(entity("l-frac"), r"\frac{a}{b}");
        assert!(b.validate_balanced_braces().is_ok());
    }

    /// Unbalanced braces in {unclosed fail validation.
    #[test]
    fn latex_unclosed_brace_fails() {
        let b = LatexBlock::new(entity("l-unc"), "{unclosed");
        assert!(b.validate_balanced_braces().is_err());
    }

    /// Clone of LatexBlock preserves source, display_mode, and entity.
    #[test]
    fn latex_clone_preserves_all_fields() {
        let mut b = LatexBlock::new(entity("l-clone"), r"\alpha + \beta");
        b.set_display_mode(true);
        let b2 = b.clone();
        assert_eq!(b2.source, r"\alpha + \beta");
        assert!(b2.display_mode);
        assert_eq!(b2.entity.id, "l-clone");
    }

    // ── AFFiNE edgeless latex: inline vs display mode tests ──────────────────

    /// LatexBlock defaults to inline mode (display_mode == false).
    #[test]
    fn latex_default_is_inline_mode() {
        let b = LatexBlock::new(entity("l-inline"), r"x^2 + y^2 = z^2");
        assert!(!b.display_mode, "new LatexBlock must be in inline mode by default");
    }

    /// LatexBlock can be switched to display (block) mode.
    #[test]
    fn latex_set_display_mode_enables_block_rendering() {
        let mut b = LatexBlock::new(entity("l-display"), r"\int_0^\infty e^{-x} dx");
        b.set_display_mode(true);
        assert!(b.display_mode, "display_mode must be true after set_display_mode(true)");
    }
}
