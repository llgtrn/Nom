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
}
