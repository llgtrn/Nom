#![deny(unsafe_code)]
use nom_gpui::FontId;

/// Registry of font IDs loaded into the cosmic-text FontSystem.
///
/// In full window initialization these IDs map to actual loaded font faces.
/// `placeholder()` returns sequential sentinel IDs for compile-time wiring;
/// replace with the results of `FontSystem::db_mut().load_font_data(...)` at
/// window creation time.
#[derive(Clone)]
pub struct FontRegistry {
    pub inter_regular: FontId,
    pub inter_medium: FontId,
    pub inter_semibold: FontId,
    pub inter_bold: FontId,
    pub source_code_pro_regular: FontId,
    pub source_code_pro_semibold: FontId,
}

impl FontRegistry {
    /// Placeholder registry — sequential IDs, no font data loaded.
    /// Real loading happens in Window init via cosmic_text FontSystem.
    pub fn placeholder() -> Self {
        Self {
            inter_regular: 0,
            inter_medium: 1,
            inter_semibold: 2,
            inter_bold: 3,
            source_code_pro_regular: 4,
            source_code_pro_semibold: 5,
        }
    }
}

/// Resolved typographic style for a text run.
pub struct TypeStyle {
    pub font_id: FontId,
    pub size: f32,
    pub line_height: f32,
    pub letter_spacing: f32,
}

impl TypeStyle {
    /// Body text — Inter Regular 14px / 1.5 lh.
    pub fn body(fonts: &FontRegistry) -> Self {
        Self {
            font_id: fonts.inter_regular,
            size: crate::tokens::FONT_SIZE_BODY,
            line_height: crate::tokens::LINE_HEIGHT_BODY,
            letter_spacing: 0.0,
        }
    }

    /// Caption / label text — Inter Regular 12px / 1.4 lh.
    pub fn caption(fonts: &FontRegistry) -> Self {
        Self {
            font_id: fonts.inter_regular,
            size: crate::tokens::FONT_SIZE_CAPTION,
            line_height: crate::tokens::LINE_HEIGHT_CAPTION,
            letter_spacing: 0.0,
        }
    }

    /// H3 heading — Inter Medium 18px / 1.2 lh.
    pub fn heading3(fonts: &FontRegistry) -> Self {
        Self {
            font_id: fonts.inter_medium,
            size: crate::tokens::FONT_SIZE_H3,
            line_height: crate::tokens::LINE_HEIGHT_HEADING,
            letter_spacing: -0.3,
        }
    }

    /// H2 heading — Inter SemiBold 20px / 1.2 lh.
    pub fn heading2(fonts: &FontRegistry) -> Self {
        Self {
            font_id: fonts.inter_semibold,
            size: crate::tokens::FONT_SIZE_H2,
            line_height: crate::tokens::LINE_HEIGHT_HEADING,
            letter_spacing: -0.4,
        }
    }

    /// H1 heading — Inter Bold 24px / 1.2 lh.
    pub fn heading1(fonts: &FontRegistry) -> Self {
        Self {
            font_id: fonts.inter_bold,
            size: crate::tokens::FONT_SIZE_H1,
            line_height: crate::tokens::LINE_HEIGHT_HEADING,
            letter_spacing: -0.5,
        }
    }

    /// Monospace code — SourceCodePro Regular 13px / 1.6 lh.
    pub fn code(fonts: &FontRegistry) -> Self {
        Self {
            font_id: fonts.source_code_pro_regular,
            size: crate::tokens::FONT_SIZE_CODE,
            line_height: crate::tokens::LINE_HEIGHT_CODE,
            letter_spacing: 0.0,
        }
    }

    /// Monospace code bold — SourceCodePro SemiBold 13px / 1.6 lh.
    pub fn code_semibold(fonts: &FontRegistry) -> Self {
        Self {
            font_id: fonts.source_code_pro_semibold,
            size: crate::tokens::FONT_SIZE_CODE,
            line_height: crate::tokens::LINE_HEIGHT_CODE,
            letter_spacing: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens;

    #[test]
    fn font_registry_has_default() {
        let reg = FontRegistry::placeholder();
        // Placeholder IDs are sequential 0..5; all must be valid u32 values.
        let ids = [
            reg.inter_regular,
            reg.inter_medium,
            reg.inter_semibold,
            reg.inter_bold,
            reg.source_code_pro_regular,
            reg.source_code_pro_semibold,
        ];
        // All distinct.
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                assert_ne!(ids[i], ids[j], "placeholder font IDs must be distinct");
            }
        }
    }

    #[test]
    fn font_size_scale_monotonic() {
        let reg = FontRegistry::placeholder();
        let caption = TypeStyle::caption(&reg);
        let body = TypeStyle::body(&reg);
        let h3 = TypeStyle::heading3(&reg);
        let h2 = TypeStyle::heading2(&reg);
        let h1 = TypeStyle::heading1(&reg);
        assert!(caption.size < body.size);
        assert!(body.size < h3.size);
        assert!(h3.size < h2.size);
        assert!(h2.size < h1.size);
    }

    #[test]
    fn type_style_body_matches_tokens() {
        let reg = FontRegistry::placeholder();
        let body = TypeStyle::body(&reg);
        assert_eq!(body.size, tokens::FONT_SIZE_BODY);
        assert_eq!(body.line_height, tokens::LINE_HEIGHT_BODY);
        assert_eq!(body.letter_spacing, 0.0);
    }

    #[test]
    fn type_style_code_uses_mono_font() {
        let reg = FontRegistry::placeholder();
        let code = TypeStyle::code(&reg);
        let code_bold = TypeStyle::code_semibold(&reg);
        // Both must reference a SourceCodePro font ID.
        assert_eq!(code.font_id, reg.source_code_pro_regular);
        assert_eq!(code_bold.font_id, reg.source_code_pro_semibold);
        // Same size, same line height.
        assert_eq!(code.size, tokens::FONT_SIZE_CODE);
        assert_eq!(code.size, code_bold.size);
        assert_eq!(code.line_height, code_bold.line_height);
    }

    #[test]
    fn font_registry_placeholder_ids_start_at_zero() {
        let reg = FontRegistry::placeholder();
        assert_eq!(
            reg.inter_regular, 0,
            "inter_regular placeholder ID must be 0"
        );
    }

    #[test]
    fn font_type_style_headings_have_negative_tracking() {
        let reg = FontRegistry::placeholder();
        let h1 = TypeStyle::heading1(&reg);
        let h2 = TypeStyle::heading2(&reg);
        let h3 = TypeStyle::heading3(&reg);
        // Headings use tight letter-spacing (negative values).
        assert!(
            h1.letter_spacing < 0.0,
            "H1 letter_spacing ({}) must be negative",
            h1.letter_spacing
        );
        assert!(
            h2.letter_spacing < 0.0,
            "H2 letter_spacing ({}) must be negative",
            h2.letter_spacing
        );
        assert!(
            h3.letter_spacing < 0.0,
            "H3 letter_spacing ({}) must be negative",
            h3.letter_spacing
        );
    }

    #[test]
    fn font_type_style_caption_line_height_positive() {
        let reg = FontRegistry::placeholder();
        let caption = TypeStyle::caption(&reg);
        assert!(
            caption.line_height > 0.0,
            "caption line_height must be positive"
        );
    }

    #[test]
    fn font_type_style_code_line_height_wider_than_body() {
        let reg = FontRegistry::placeholder();
        let body = TypeStyle::body(&reg);
        let code = TypeStyle::code(&reg);
        // Code blocks need more vertical breathing room than prose.
        assert!(
            code.line_height > body.line_height,
            "code line_height ({}) should be wider than body line_height ({})",
            code.line_height,
            body.line_height
        );
    }

    #[test]
    fn font_type_style_all_sizes_positive() {
        let reg = FontRegistry::placeholder();
        let styles = [
            TypeStyle::body(&reg),
            TypeStyle::caption(&reg),
            TypeStyle::heading1(&reg),
            TypeStyle::heading2(&reg),
            TypeStyle::heading3(&reg),
            TypeStyle::code(&reg),
            TypeStyle::code_semibold(&reg),
        ];
        for (i, s) in styles.iter().enumerate() {
            assert!(
                s.size > 0.0,
                "TypeStyle[{i}].size must be positive, got {}",
                s.size
            );
            assert!(
                s.line_height > 0.0,
                "TypeStyle[{i}].line_height must be positive, got {}",
                s.line_height
            );
        }
    }

    #[test]
    fn font_display_larger_than_body() {
        // H1 is the display-level heading; must exceed body size.
        let reg = FontRegistry::placeholder();
        let body = TypeStyle::body(&reg);
        let h1 = TypeStyle::heading1(&reg);
        assert!(
            h1.size > body.size,
            "H1 ({}) must be larger than body ({})",
            h1.size,
            body.size
        );
    }

    #[test]
    fn font_body_size_range() {
        let reg = FontRegistry::placeholder();
        let body = TypeStyle::body(&reg);
        assert!(
            body.size >= 12.0 && body.size <= 18.0,
            "body font size ({}) must be between 12 and 18pt",
            body.size
        );
    }

    #[test]
    fn font_code_is_monospace() {
        // SourceCodePro IDs are sequential 4 and 5 in the placeholder registry.
        // Verify code TypeStyle uses the SourceCodePro slot (ID >= 4).
        let reg = FontRegistry::placeholder();
        let code = TypeStyle::code(&reg);
        assert_eq!(
            code.font_id, reg.source_code_pro_regular,
            "code TypeStyle must use source_code_pro_regular font"
        );
    }

    #[test]
    fn font_heading_1_largest() {
        let reg = FontRegistry::placeholder();
        let h1 = TypeStyle::heading1(&reg);
        let h2 = TypeStyle::heading2(&reg);
        assert!(
            h1.size >= h2.size,
            "H1 ({}) must be >= H2 ({})",
            h1.size,
            h2.size
        );
    }

    #[test]
    fn font_heading_2_larger_than_body() {
        let reg = FontRegistry::placeholder();
        let h2 = TypeStyle::heading2(&reg);
        let body = TypeStyle::body(&reg);
        assert!(
            h2.size > body.size,
            "H2 ({}) must be > body ({})",
            h2.size,
            body.size
        );
    }

    #[test]
    fn font_caption_smallest() {
        let reg = FontRegistry::placeholder();
        let caption = TypeStyle::caption(&reg);
        let body = TypeStyle::body(&reg);
        assert!(
            caption.size <= body.size,
            "caption ({}) must be <= body ({})",
            caption.size,
            body.size
        );
    }

    #[test]
    fn font_line_height_not_zero() {
        let reg = FontRegistry::placeholder();
        let styles = [
            TypeStyle::body(&reg),
            TypeStyle::caption(&reg),
            TypeStyle::heading1(&reg),
            TypeStyle::heading2(&reg),
            TypeStyle::heading3(&reg),
            TypeStyle::code(&reg),
            TypeStyle::code_semibold(&reg),
        ];
        for (i, s) in styles.iter().enumerate() {
            assert!(
                s.line_height > 0.0,
                "TypeStyle[{i}].line_height must not be zero, got {}",
                s.line_height
            );
        }
    }

    // -----------------------------------------------------------------------
    // Extended font tests
    // -----------------------------------------------------------------------

    #[test]
    fn font_registry_has_six_entries() {
        // The placeholder registry must expose exactly 6 font IDs.
        let reg = FontRegistry::placeholder();
        let ids = [
            reg.inter_regular,
            reg.inter_medium,
            reg.inter_semibold,
            reg.inter_bold,
            reg.source_code_pro_regular,
            reg.source_code_pro_semibold,
        ];
        assert_eq!(ids.len(), 6, "FontRegistry must have 6 font slots");
    }

    #[test]
    fn font_registry_clone_matches_original() {
        // Clone must produce equal IDs.
        let reg = FontRegistry::placeholder();
        let cloned = reg.clone();
        assert_eq!(reg.inter_regular, cloned.inter_regular);
        assert_eq!(reg.inter_bold, cloned.inter_bold);
        assert_eq!(reg.source_code_pro_regular, cloned.source_code_pro_regular);
        assert_eq!(reg.source_code_pro_semibold, cloned.source_code_pro_semibold);
    }

    #[test]
    fn font_type_style_heading1_uses_bold() {
        let reg = FontRegistry::placeholder();
        let h1 = TypeStyle::heading1(&reg);
        assert_eq!(
            h1.font_id, reg.inter_bold,
            "H1 must use inter_bold font"
        );
    }

    #[test]
    fn font_type_style_heading2_uses_semibold() {
        let reg = FontRegistry::placeholder();
        let h2 = TypeStyle::heading2(&reg);
        assert_eq!(
            h2.font_id, reg.inter_semibold,
            "H2 must use inter_semibold font"
        );
    }

    #[test]
    fn font_type_style_heading3_uses_medium() {
        let reg = FontRegistry::placeholder();
        let h3 = TypeStyle::heading3(&reg);
        assert_eq!(
            h3.font_id, reg.inter_medium,
            "H3 must use inter_medium font"
        );
    }

    #[test]
    fn font_type_style_body_uses_regular() {
        let reg = FontRegistry::placeholder();
        let body = TypeStyle::body(&reg);
        assert_eq!(
            body.font_id, reg.inter_regular,
            "body must use inter_regular font"
        );
    }

    #[test]
    fn font_type_style_caption_uses_regular() {
        let reg = FontRegistry::placeholder();
        let caption = TypeStyle::caption(&reg);
        assert_eq!(
            caption.font_id, reg.inter_regular,
            "caption must use inter_regular font"
        );
    }

    #[test]
    fn font_type_style_code_zero_letter_spacing() {
        // Monospace code must not have letter-spacing adjustments.
        let reg = FontRegistry::placeholder();
        let code = TypeStyle::code(&reg);
        let code_bold = TypeStyle::code_semibold(&reg);
        assert_eq!(code.letter_spacing, 0.0, "code letter_spacing must be 0.0");
        assert_eq!(
            code_bold.letter_spacing, 0.0,
            "code_semibold letter_spacing must be 0.0"
        );
    }

    #[test]
    fn font_type_style_body_zero_letter_spacing() {
        let reg = FontRegistry::placeholder();
        let body = TypeStyle::body(&reg);
        assert_eq!(body.letter_spacing, 0.0, "body letter_spacing must be 0.0");
    }

    #[test]
    fn font_caption_zero_letter_spacing() {
        let reg = FontRegistry::placeholder();
        let caption = TypeStyle::caption(&reg);
        assert_eq!(
            caption.letter_spacing, 0.0,
            "caption letter_spacing must be 0.0"
        );
    }

    #[test]
    fn font_heading_letter_spacing_tighter_at_larger_size() {
        // H1 (24px) must have tighter tracking than H3 (18px).
        let reg = FontRegistry::placeholder();
        let h1 = TypeStyle::heading1(&reg);
        let h3 = TypeStyle::heading3(&reg);
        assert!(
            h1.letter_spacing <= h3.letter_spacing,
            "H1 letter_spacing ({}) must be <= H3 letter_spacing ({}) — larger headings track tighter",
            h1.letter_spacing,
            h3.letter_spacing
        );
    }

    #[test]
    fn font_code_semibold_larger_font_id_than_regular() {
        // In placeholder ordering, semibold has a higher ID than regular for same family.
        let reg = FontRegistry::placeholder();
        assert!(
            reg.source_code_pro_semibold > reg.source_code_pro_regular,
            "source_code_pro_semibold ID must be > source_code_pro_regular ID"
        );
    }

    #[test]
    fn font_inter_ids_before_source_code_pro_ids() {
        // Placeholder ordering: Inter IDs 0-3, SourceCodePro IDs 4-5.
        let reg = FontRegistry::placeholder();
        assert!(reg.inter_bold < reg.source_code_pro_regular);
        assert!(reg.inter_semibold < reg.source_code_pro_regular);
    }

    #[test]
    fn font_type_style_heading_sizes_match_tokens() {
        let reg = FontRegistry::placeholder();
        let h1 = TypeStyle::heading1(&reg);
        let h2 = TypeStyle::heading2(&reg);
        let h3 = TypeStyle::heading3(&reg);
        assert_eq!(h1.size, tokens::FONT_SIZE_H1);
        assert_eq!(h2.size, tokens::FONT_SIZE_H2);
        assert_eq!(h3.size, tokens::FONT_SIZE_H3);
    }

    #[test]
    fn font_type_style_heading_line_heights_match_token() {
        let reg = FontRegistry::placeholder();
        let h1 = TypeStyle::heading1(&reg);
        let h2 = TypeStyle::heading2(&reg);
        let h3 = TypeStyle::heading3(&reg);
        assert_eq!(h1.line_height, tokens::LINE_HEIGHT_HEADING);
        assert_eq!(h2.line_height, tokens::LINE_HEIGHT_HEADING);
        assert_eq!(h3.line_height, tokens::LINE_HEIGHT_HEADING);
    }

    #[test]
    fn font_caption_matches_token_size() {
        let reg = FontRegistry::placeholder();
        let caption = TypeStyle::caption(&reg);
        assert_eq!(caption.size, tokens::FONT_SIZE_CAPTION);
        assert_eq!(caption.line_height, tokens::LINE_HEIGHT_CAPTION);
    }

    #[test]
    fn font_code_matches_token_size() {
        let reg = FontRegistry::placeholder();
        let code = TypeStyle::code(&reg);
        assert_eq!(code.size, tokens::FONT_SIZE_CODE);
        assert_eq!(code.line_height, tokens::LINE_HEIGHT_CODE);
    }
}
