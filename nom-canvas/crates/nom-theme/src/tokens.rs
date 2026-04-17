#![deny(unsafe_code)]
use nom_gpui::Hsla;

// ---------------------------------------------------------------------------
// Spacing scale (4px base grid)
// ---------------------------------------------------------------------------

pub const SPACING_1: f32 = 4.0;
pub const SPACING_2: f32 = 8.0;
pub const SPACING_3: f32 = 12.0;
pub const SPACING_4: f32 = 16.0;
pub const SPACING_6: f32 = 24.0;
pub const SPACING_8: f32 = 32.0;
pub const SPACING_12: f32 = 48.0;

// ---------------------------------------------------------------------------
// Radius scale
// ---------------------------------------------------------------------------

pub const RADIUS_NONE: f32 = 0.0;
pub const RADIUS_SM: f32 = 4.0;
pub const RADIUS_MD: f32 = 8.0;
pub const RADIUS_LG: f32 = 12.0;
pub const RADIUS_XL: f32 = 16.0;
pub const RADIUS_FULL: f32 = 9999.0;

// ---------------------------------------------------------------------------
// Typography (size in px)
// ---------------------------------------------------------------------------

pub const FONT_SIZE_CAPTION: f32 = 12.0;
pub const FONT_SIZE_BODY: f32 = 14.0;
pub const FONT_SIZE_H3: f32 = 18.0;
pub const FONT_SIZE_H2: f32 = 20.0;
pub const FONT_SIZE_H1: f32 = 24.0;
pub const FONT_SIZE_CODE: f32 = 13.0;
pub const LINE_HEIGHT_CAPTION: f32 = 1.4;
pub const LINE_HEIGHT_BODY: f32 = 1.5;
pub const LINE_HEIGHT_HEADING: f32 = 1.2;
pub const LINE_HEIGHT_CODE: f32 = 1.6;

// ---------------------------------------------------------------------------
// Frosted glass
// ---------------------------------------------------------------------------

pub const FROSTED_BLUR_RADIUS: f32 = 12.0;
pub const FROSTED_BG_ALPHA: f32 = 0.85;
pub const FROSTED_BORDER_ALPHA: f32 = 0.12;

// ---------------------------------------------------------------------------
// Motion tokens (AFFiNE spring motion)
// ---------------------------------------------------------------------------

pub const MOTION_SPRING_STIFFNESS: f32 = 400.0;
pub const MOTION_SPRING_DAMPING: f32 = 28.0;
pub const MOTION_HOVER_DURATION_MS: u64 = 120;
pub const MOTION_PANEL_RESIZE_DURATION_MS: u64 = 200;

// ---------------------------------------------------------------------------
// Panel sizes
// ---------------------------------------------------------------------------

pub const PANEL_LEFT_WIDTH: f32 = 248.0;
pub const PANEL_RIGHT_WIDTH: f32 = 320.0;
pub const PANEL_BOTTOM_HEIGHT: f32 = 200.0;
pub const PANEL_MIN_WIDTH: f32 = 160.0;
pub const PANEL_MAX_WIDTH: f32 = 480.0;

// ---------------------------------------------------------------------------
// Shadow tokens
// ---------------------------------------------------------------------------

pub struct ShadowToken {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: fn() -> Hsla,
}

pub const SHADOW_SM: ShadowToken = ShadowToken {
    offset_x: 0.0,
    offset_y: 1.0,
    blur: 2.0,
    spread: 0.0,
    color: || Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.15 },
};

pub const SHADOW_MD: ShadowToken = ShadowToken {
    offset_x: 0.0,
    offset_y: 4.0,
    blur: 8.0,
    spread: 0.0,
    color: || Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.20 },
};

pub const SHADOW_LG: ShadowToken = ShadowToken {
    offset_x: 0.0,
    offset_y: 8.0,
    blur: 24.0,
    spread: 0.0,
    color: || Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.25 },
};

pub const SHADOW_XL: ShadowToken = ShadowToken {
    offset_x: 0.0,
    offset_y: 16.0,
    blur: 48.0,
    spread: 0.0,
    color: || Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.30 },
};

// ---------------------------------------------------------------------------
// Dark theme colors (AFFiNE dark palette) — runtime color functions
// ---------------------------------------------------------------------------

/// Primary background: hsl(220°, 13%, 11%)
pub fn color_bg_primary() -> Hsla {
    Hsla::new(220.0 / 360.0, 0.13, 0.11, 1.0)
}

/// Secondary background: hsl(220°, 11%, 14%)
pub fn color_bg_secondary() -> Hsla {
    Hsla::new(220.0 / 360.0, 0.11, 0.14, 1.0)
}

/// Tertiary background: hsl(220°, 10%, 17%)
pub fn color_bg_tertiary() -> Hsla {
    Hsla::new(220.0 / 360.0, 0.10, 0.17, 1.0)
}

/// Primary text: near-white
pub fn color_text_primary() -> Hsla {
    Hsla::new(0.0, 0.0, 0.98, 1.0)
}

/// Secondary text: hsl(220°, 9%, 65%)
pub fn color_text_secondary() -> Hsla {
    Hsla::new(220.0 / 360.0, 0.09, 0.65, 1.0)
}

/// Tertiary / muted text: hsl(220°, 7%, 45%)
pub fn color_text_tertiary() -> Hsla {
    Hsla::new(220.0 / 360.0, 0.07, 0.45, 1.0)
}

/// Subtle border: hsl(220°, 13%, 22%)
pub fn color_border_subtle() -> Hsla {
    Hsla::new(220.0 / 360.0, 0.13, 0.22, 1.0)
}

/// Normal border: hsl(220°, 11%, 30%)
pub fn color_border_normal() -> Hsla {
    Hsla::new(220.0 / 360.0, 0.11, 0.30, 1.0)
}

/// Accent blue (~#1E90FF): hsl(211°, 100%, 60%)
pub fn color_accent_blue() -> Hsla {
    Hsla::new(211.0 / 360.0, 1.0, 0.60, 1.0)
}

/// Accent purple (nomtu references): hsl(270°, 91%, 70%)
pub fn color_accent_purple() -> Hsla {
    Hsla::new(270.0 / 360.0, 0.91, 0.70, 1.0)
}

/// Accent green (literals, #22C55E): hsl(145°, 63%, 49%)
pub fn color_accent_green() -> Hsla {
    Hsla::new(145.0 / 360.0, 0.63, 0.49, 1.0)
}

/// Surface overlay (panel backgrounds): hsl(220°, 14%, 8%, 85%)
pub fn color_surface_overlay() -> Hsla {
    Hsla::new(220.0 / 360.0, 0.14, 0.08, 0.85)
}

// ---------------------------------------------------------------------------
// Graph edge confidence colors (exact from spec)
// ---------------------------------------------------------------------------

/// High confidence >= 0.8: #22C55E — hsl(142.1°, 70.6%, 45.3%)
pub fn edge_color_high_confidence() -> Hsla {
    Hsla::new(142.1 / 360.0, 0.706, 0.453, 1.0)
}

/// Medium confidence 0.5–0.8: #F59E0B — hsl(37.7°, 92.1%, 50.2%)
pub fn edge_color_medium_confidence() -> Hsla {
    Hsla::new(37.7 / 360.0, 0.921, 0.502, 1.0)
}

/// Low confidence < 0.5: #EF4444 — hsl(0°, 84.2%, 60.2%)
pub fn edge_color_low_confidence() -> Hsla {
    Hsla::new(0.0, 0.842, 0.602, 1.0)
}

/// Select the correct edge color for a given confidence score.
pub fn edge_color_for_confidence(confidence: f32) -> Hsla {
    if confidence >= 0.8 {
        edge_color_high_confidence()
    } else if confidence >= 0.5 {
        edge_color_medium_confidence()
    } else {
        edge_color_low_confidence()
    }
}

// ---------------------------------------------------------------------------
// Spec-required named constants (layout, typography, color, animation)
// ---------------------------------------------------------------------------

pub const SIDEBAR_W: f32 = 248.0;
pub const TOOLBAR_H: f32 = 48.0;
pub const STATUSBAR_H: f32 = 24.0;
pub const BLOCK_RADIUS: f32 = 4.0;
pub const MODAL_RADIUS: f32 = 22.0;
pub const POPOVER_RADIUS: f32 = 12.0;
pub const BTN_H: f32 = 28.0;
pub const BTN_H_LG: f32 = 32.0;
pub const BTN_H_XL: f32 = 40.0;
pub const ICON_SIZE: f32 = 24.0;
pub const H1_WEIGHT: u16 = 700;
pub const H1_LETTER_SPACING: f32 = -0.02;
pub const H2_WEIGHT: u16 = 600;
pub const BODY_WEIGHT: u16 = 400;
pub const BG: [f32; 4] = [0.059, 0.090, 0.165, 1.0];
pub const BG2: [f32; 4] = [0.118, 0.161, 0.251, 1.0];
pub const TEXT: [f32; 4] = [0.973, 0.980, 0.988, 1.0];
pub const CTA: [f32; 4] = [0.133, 0.773, 0.369, 1.0];
pub const BORDER: [f32; 4] = [0.200, 0.255, 0.333, 1.0];
pub const FOCUS: [f32; 4] = [0.118, 0.588, 0.922, 0.3];
pub const EDGE_HIGH: [f32; 4] = [0.133, 0.773, 0.369, 0.9];
pub const EDGE_MED: [f32; 4] = [0.957, 0.702, 0.078, 0.7];
pub const EDGE_LOW: [f32; 4] = [0.937, 0.267, 0.267, 0.5];
pub const ANIM_DEFAULT_MS: f32 = 300.0;
pub const ANIM_FAST_MS: f32 = 200.0;
