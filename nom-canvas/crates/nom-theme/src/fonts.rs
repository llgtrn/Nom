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

    /// H1 heading — Inter SemiBold 24px / 1.2 lh.
    pub fn heading1(fonts: &FontRegistry) -> Self {
        Self {
            font_id: fonts.inter_semibold,
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
