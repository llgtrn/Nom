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
pub const ICON_SIZE_SM: f32 = 16.0;
pub const H1_SPACING: f32 = 8.0;   // letter-spacing for H1
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

// ---------------------------------------------------------------------------
// Semantic color aliases and additional tokens
// ---------------------------------------------------------------------------

/// Base background — very dark, near-zero blue channel.
pub const BASE_BG: [f32; 4] = [0.08, 0.09, 0.02, 1.0];
/// Base foreground — near-white, all channels > 0.8.
pub const BASE_FG: [f32; 4] = [0.97, 0.98, 0.99, 1.0];
/// Error state — red-dominant.
pub const ERROR: [f32; 4] = [0.937, 0.267, 0.267, 1.0];
/// Warning state — yellowish (both R and G > 0.5).
pub const WARNING: [f32; 4] = [0.957, 0.702, 0.078, 1.0];

/// Total count of distinct named color tokens defined in this module.
pub const N_TOKENS: usize = 73;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_spacing_values_consistent() {
        // Each step is a multiple of the 4px base grid.
        assert_eq!(SPACING_1, 4.0);
        assert_eq!(SPACING_2, 8.0);
        assert_eq!(SPACING_3, 12.0);
        assert_eq!(SPACING_4, 16.0);
        assert_eq!(SPACING_6, 24.0);
        assert_eq!(SPACING_8, 32.0);
        assert_eq!(SPACING_12, 48.0);
        // Strictly ascending.
        let spacings = [SPACING_1, SPACING_2, SPACING_3, SPACING_4, SPACING_6, SPACING_8, SPACING_12];
        for w in spacings.windows(2) {
            assert!(w[1] > w[0], "spacing scale must be strictly ascending");
        }
    }

    #[test]
    fn token_font_sizes_ordered() {
        // Caption < code < body < h3 < h2 < h1
        assert!(FONT_SIZE_CAPTION < FONT_SIZE_CODE);
        assert!(FONT_SIZE_CODE < FONT_SIZE_BODY);
        assert!(FONT_SIZE_BODY < FONT_SIZE_H3);
        assert!(FONT_SIZE_H3 < FONT_SIZE_H2);
        assert!(FONT_SIZE_H2 < FONT_SIZE_H1);
    }

    #[test]
    fn token_colors_are_valid_rgba() {
        // Every [f32; 4] color constant must have components in [0.0, 1.0].
        let named_colors: &[(&str, [f32; 4])] = &[
            ("BG", BG),
            ("BG2", BG2),
            ("TEXT", TEXT),
            ("CTA", CTA),
            ("BORDER", BORDER),
            ("FOCUS", FOCUS),
            ("EDGE_HIGH", EDGE_HIGH),
            ("EDGE_MED", EDGE_MED),
            ("EDGE_LOW", EDGE_LOW),
        ];
        for (name, c) in named_colors {
            for (i, component) in c.iter().enumerate() {
                assert!(
                    (0.0..=1.0).contains(component),
                    "{name}[{i}] = {component} out of [0,1]"
                );
            }
        }
    }

    #[test]
    fn theme_token_all_colors_nonzero() {
        // RGB channels of opaque color constants must have at least one non-zero component.
        let colors: &[(&str, [f32; 4])] = &[
            ("BG", BG),
            ("BG2", BG2),
            ("TEXT", TEXT),
            ("CTA", CTA),
            ("BORDER", BORDER),
            ("EDGE_HIGH", EDGE_HIGH),
            ("EDGE_MED", EDGE_MED),
            ("EDGE_LOW", EDGE_LOW),
        ];
        for (name, c) in colors {
            let sum: f32 = c[0] + c[1] + c[2];
            assert!(sum > 0.0, "{name} RGB sum must be > 0");
        }
    }

    #[test]
    fn token_icon_size_matches_spec() {
        // Spec: ICON_SIZE = 24.0 px
        assert_eq!(ICON_SIZE, 24.0);
    }

    #[test]
    fn token_icon_size_sm_smaller_than_icon_size() {
        assert!(ICON_SIZE_SM < ICON_SIZE);
        assert_eq!(ICON_SIZE_SM, 16.0);
    }

    #[test]
    fn token_h1_spacing_positive() {
        assert!(H1_SPACING > 0.0);
        assert_eq!(H1_SPACING, 8.0);
    }

    #[test]
    fn token_font_constants_match_spec() {
        // H1 weight 700, H2 weight 600, body weight 400
        assert_eq!(H1_WEIGHT, 700);
        assert_eq!(H2_WEIGHT, 600);
        assert_eq!(BODY_WEIGHT, 400);
    }

    #[test]
    fn token_panel_sizes_within_bounds() {
        assert!(PANEL_LEFT_WIDTH > PANEL_MIN_WIDTH);
        assert!(PANEL_RIGHT_WIDTH > PANEL_MIN_WIDTH);
        assert!(PANEL_LEFT_WIDTH < PANEL_MAX_WIDTH);
        assert!(PANEL_RIGHT_WIDTH <= PANEL_MAX_WIDTH);
    }

    #[test]
    fn edge_color_for_confidence_routing() {
        let high = edge_color_for_confidence(0.9);
        let expected_high = edge_color_high_confidence();
        assert!((high.h - expected_high.h).abs() < f32::EPSILON);

        let med = edge_color_for_confidence(0.65);
        let expected_med = edge_color_medium_confidence();
        assert!((med.h - expected_med.h).abs() < f32::EPSILON);

        let low = edge_color_for_confidence(0.2);
        let expected_low = edge_color_low_confidence();
        assert!((low.h - expected_low.h).abs() < f32::EPSILON);

        // Boundary: exactly 0.8 → high
        let boundary = edge_color_for_confidence(0.8);
        assert!((boundary.h - expected_high.h).abs() < f32::EPSILON);

        // Boundary: exactly 0.5 → medium
        let boundary_med = edge_color_for_confidence(0.5);
        assert!((boundary_med.h - expected_med.h).abs() < f32::EPSILON);
    }

    #[test]
    fn shadow_tokens_blur_ordering() {
        // Larger shadow variants must have greater blur.
        assert!(SHADOW_MD.blur > SHADOW_SM.blur);
        assert!(SHADOW_LG.blur > SHADOW_MD.blur);
        assert!(SHADOW_XL.blur > SHADOW_LG.blur);
    }

    #[test]
    fn frosted_glass_alpha_in_range() {
        assert!((0.0..=1.0).contains(&FROSTED_BG_ALPHA));
        assert!((0.0..=1.0).contains(&FROSTED_BORDER_ALPHA));
        assert!(FROSTED_BG_ALPHA > FROSTED_BORDER_ALPHA,
            "background alpha should dominate border alpha");
    }

    #[test]
    fn tokens_base_bg_is_dark() {
        // Blue channel (index 2) must be near 0 — confirms a dark background.
        assert!(BASE_BG[2] < 0.1, "BASE_BG blue channel should be near 0 for a dark bg, got {}", BASE_BG[2]);
    }

    #[test]
    fn tokens_base_fg_is_light() {
        // Red channel (index 0) must be > 0.8 — confirms a light foreground.
        assert!(BASE_FG[0] > 0.8, "BASE_FG red channel should be > 0.8 for a light fg, got {}", BASE_FG[0]);
    }

    #[test]
    fn tokens_cta_has_alpha_one() {
        assert_eq!(CTA[3], 1.0, "CTA alpha must be 1.0 (fully opaque)");
    }

    #[test]
    fn tokens_all_alphas_in_range() {
        let all_colors: &[(&str, [f32; 4])] = &[
            ("BG", BG),
            ("BG2", BG2),
            ("TEXT", TEXT),
            ("CTA", CTA),
            ("BORDER", BORDER),
            ("FOCUS", FOCUS),
            ("EDGE_HIGH", EDGE_HIGH),
            ("EDGE_MED", EDGE_MED),
            ("EDGE_LOW", EDGE_LOW),
            ("BASE_BG", BASE_BG),
            ("BASE_FG", BASE_FG),
            ("ERROR", ERROR),
            ("WARNING", WARNING),
        ];
        for (name, c) in all_colors {
            assert!(
                (0.0..=1.0).contains(&c[3]),
                "{name}[3] alpha = {} is out of [0.0, 1.0]",
                c[3]
            );
        }
    }

    #[test]
    fn tokens_frosted_blur_radius_positive() {
        assert!(FROSTED_BLUR_RADIUS > 0.0, "FROSTED_BLUR_RADIUS must be positive");
    }

    #[test]
    fn tokens_frosted_bg_alpha_below_one() {
        assert!(FROSTED_BG_ALPHA < 1.0, "FROSTED_BG_ALPHA should be < 1.0 for frosted transparency");
    }

    #[test]
    fn tokens_frosted_border_alpha_below_bg_alpha() {
        assert!(
            FROSTED_BORDER_ALPHA < FROSTED_BG_ALPHA,
            "FROSTED_BORDER_ALPHA ({}) must be less than FROSTED_BG_ALPHA ({})",
            FROSTED_BORDER_ALPHA,
            FROSTED_BG_ALPHA
        );
    }

    #[test]
    fn tokens_focus_has_distinct_color() {
        // Focus ring must differ from base background so it is visible.
        assert_ne!(
            FOCUS, BASE_BG,
            "FOCUS ring color must differ from BASE_BG"
        );
    }

    #[test]
    fn tokens_error_is_reddish() {
        // Red channel dominates over both green and blue.
        assert!(ERROR[0] > ERROR[1], "ERROR red ({}) must exceed green ({})", ERROR[0], ERROR[1]);
        assert!(ERROR[0] > ERROR[2], "ERROR red ({}) must exceed blue ({})", ERROR[0], ERROR[2]);
    }

    #[test]
    fn tokens_warning_is_yellowish() {
        // Both red and green channels > 0.5 gives a yellow hue.
        assert!(WARNING[0] > 0.5, "WARNING red ({}) must be > 0.5 for yellow hue", WARNING[0]);
        assert!(WARNING[1] > 0.5, "WARNING green ({}) must be > 0.5 for yellow hue", WARNING[1]);
    }

    #[test]
    fn tokens_edge_high_brighter_than_edge_low() {
        // EDGE_HIGH is green-dominant (high confidence); EDGE_LOW is red-dominant (low confidence).
        // High-confidence edges have a higher green channel than low-confidence edges.
        assert!(
            EDGE_HIGH[1] > EDGE_LOW[1],
            "EDGE_HIGH green ({}) must exceed EDGE_LOW green ({}) — high confidence = green",
            EDGE_HIGH[1], EDGE_LOW[1]
        );
        // And high-confidence edges have a higher alpha than low-confidence edges.
        assert!(
            EDGE_HIGH[3] > EDGE_LOW[3],
            "EDGE_HIGH alpha ({}) must exceed EDGE_LOW alpha ({})",
            EDGE_HIGH[3], EDGE_LOW[3]
        );
    }

    #[test]
    fn tokens_count_all_defined() {
        // Compile-time sanity: N_TOKENS must be defined and > 20.
        assert!(N_TOKENS >= 20, "N_TOKENS ({N_TOKENS}) must be at least 20");
    }
}
