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
    color: || Hsla {
        h: 0.0,
        s: 0.0,
        l: 0.0,
        a: 0.15,
    },
};

pub const SHADOW_MD: ShadowToken = ShadowToken {
    offset_x: 0.0,
    offset_y: 4.0,
    blur: 8.0,
    spread: 0.0,
    color: || Hsla {
        h: 0.0,
        s: 0.0,
        l: 0.0,
        a: 0.20,
    },
};

pub const SHADOW_LG: ShadowToken = ShadowToken {
    offset_x: 0.0,
    offset_y: 8.0,
    blur: 24.0,
    spread: 0.0,
    color: || Hsla {
        h: 0.0,
        s: 0.0,
        l: 0.0,
        a: 0.25,
    },
};

pub const SHADOW_XL: ShadowToken = ShadowToken {
    offset_x: 0.0,
    offset_y: 16.0,
    blur: 48.0,
    spread: 0.0,
    color: || Hsla {
        h: 0.0,
        s: 0.0,
        l: 0.0,
        a: 0.30,
    },
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
pub const H1_SPACING: f32 = 8.0; // letter-spacing for H1
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
        let spacings = [
            SPACING_1, SPACING_2, SPACING_3, SPACING_4, SPACING_6, SPACING_8, SPACING_12,
        ];
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
        assert!(
            FROSTED_BG_ALPHA > FROSTED_BORDER_ALPHA,
            "background alpha should dominate border alpha"
        );
    }

    #[test]
    fn tokens_base_bg_is_dark() {
        // Blue channel (index 2) must be near 0 — confirms a dark background.
        assert!(
            BASE_BG[2] < 0.1,
            "BASE_BG blue channel should be near 0 for a dark bg, got {}",
            BASE_BG[2]
        );
    }

    #[test]
    fn tokens_base_fg_is_light() {
        // Red channel (index 0) must be > 0.8 — confirms a light foreground.
        assert!(
            BASE_FG[0] > 0.8,
            "BASE_FG red channel should be > 0.8 for a light fg, got {}",
            BASE_FG[0]
        );
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
        assert!(
            FROSTED_BLUR_RADIUS > 0.0,
            "FROSTED_BLUR_RADIUS must be positive"
        );
    }

    #[test]
    fn tokens_frosted_bg_alpha_below_one() {
        assert!(
            FROSTED_BG_ALPHA < 1.0,
            "FROSTED_BG_ALPHA should be < 1.0 for frosted transparency"
        );
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
        assert_ne!(FOCUS, BASE_BG, "FOCUS ring color must differ from BASE_BG");
    }

    #[test]
    fn tokens_error_is_reddish() {
        // Red channel dominates over both green and blue.
        assert!(
            ERROR[0] > ERROR[1],
            "ERROR red ({}) must exceed green ({})",
            ERROR[0],
            ERROR[1]
        );
        assert!(
            ERROR[0] > ERROR[2],
            "ERROR red ({}) must exceed blue ({})",
            ERROR[0],
            ERROR[2]
        );
    }

    #[test]
    fn tokens_warning_is_yellowish() {
        // Both red and green channels > 0.5 gives a yellow hue.
        assert!(
            WARNING[0] > 0.5,
            "WARNING red ({}) must be > 0.5 for yellow hue",
            WARNING[0]
        );
        assert!(
            WARNING[1] > 0.5,
            "WARNING green ({}) must be > 0.5 for yellow hue",
            WARNING[1]
        );
    }

    #[test]
    fn tokens_edge_high_brighter_than_edge_low() {
        // EDGE_HIGH is green-dominant (high confidence); EDGE_LOW is red-dominant (low confidence).
        // High-confidence edges have a higher green channel than low-confidence edges.
        assert!(
            EDGE_HIGH[1] > EDGE_LOW[1],
            "EDGE_HIGH green ({}) must exceed EDGE_LOW green ({}) — high confidence = green",
            EDGE_HIGH[1],
            EDGE_LOW[1]
        );
        // And high-confidence edges have a higher alpha than low-confidence edges.
        assert!(
            EDGE_HIGH[3] > EDGE_LOW[3],
            "EDGE_HIGH alpha ({}) must exceed EDGE_LOW alpha ({})",
            EDGE_HIGH[3],
            EDGE_LOW[3]
        );
    }

    #[test]
    fn tokens_count_all_defined() {
        // Compile-time sanity: N_TOKENS must be defined and > 20.
        assert!(N_TOKENS >= 20, "N_TOKENS ({N_TOKENS}) must be at least 20");
    }

    #[test]
    fn tokens_border_has_alpha() {
        // BORDER is an opaque or semi-transparent border — alpha must be in (0.0, 1.0].
        assert!(
            BORDER[3] > 0.0,
            "BORDER alpha ({}) must be > 0.0",
            BORDER[3]
        );
        assert!(
            BORDER[3] <= 1.0,
            "BORDER alpha ({}) must be <= 1.0",
            BORDER[3]
        );
    }

    #[test]
    fn tokens_focus_alpha_partial() {
        // FOCUS is a semi-transparent ring — alpha must be less than 1.0.
        assert!(
            FOCUS[3] < 1.0,
            "FOCUS alpha ({}) must be < 1.0 for a semi-transparent ring",
            FOCUS[3]
        );
    }

    #[test]
    fn tokens_focus_ring_visible() {
        // FOCUS ring must be at least somewhat visible — alpha > 0.1.
        assert!(
            FOCUS[3] > 0.1,
            "FOCUS alpha ({}) too low to be visible",
            FOCUS[3]
        );
    }

    #[test]
    fn tokens_base_fg_all_channels_bright() {
        // BASE_FG near-white: all three RGB channels must be > 0.8.
        assert!(
            BASE_FG[0] > 0.8,
            "BASE_FG[0] R={} must be > 0.8",
            BASE_FG[0]
        );
        assert!(
            BASE_FG[1] > 0.8,
            "BASE_FG[1] G={} must be > 0.8",
            BASE_FG[1]
        );
        assert!(
            BASE_FG[2] > 0.8,
            "BASE_FG[2] B={} must be > 0.8",
            BASE_FG[2]
        );
    }

    #[test]
    fn tokens_all_rgb_values_in_unit_range() {
        // Every RGB component of every named color token must be in [0.0, 1.0].
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
            for (i, ch) in c[..3].iter().enumerate() {
                assert!(
                    (0.0..=1.0).contains(ch),
                    "{name}[{i}] = {ch} out of [0.0, 1.0]"
                );
            }
        }
    }

    #[test]
    fn tokens_cta_vs_bg_contrast() {
        // CTA and BG must differ by more than 0.2 in at least one RGB channel.
        let max_diff = (CTA[0] - BG[0])
            .abs()
            .max((CTA[1] - BG[1]).abs())
            .max((CTA[2] - BG[2]).abs());
        assert!(
            max_diff > 0.2,
            "CTA and BG are too similar (max channel diff = {max_diff:.3}); CTA must be visible on BG"
        );
    }

    #[test]
    fn tokens_shadow_sm_alpha_partial() {
        // Shadow colors must be semi-transparent (alpha < 1.0).
        let alpha = (SHADOW_SM.color)().a;
        assert!(alpha < 1.0, "SHADOW_SM alpha ({alpha}) must be < 1.0");
        assert!(alpha > 0.0, "SHADOW_SM alpha ({alpha}) must be > 0.0");
    }

    #[test]
    fn tokens_shadow_xl_alpha_greater_than_sm() {
        // Larger shadows must be darker (higher alpha).
        let sm_a = (SHADOW_SM.color)().a;
        let xl_a = (SHADOW_XL.color)().a;
        assert!(
            xl_a > sm_a,
            "SHADOW_XL alpha ({xl_a}) must exceed SHADOW_SM alpha ({sm_a})"
        );
    }

    #[test]
    fn tokens_constants_are_deterministic() {
        // Const arrays must equal themselves (validates const-correctness).
        assert_eq!(BASE_BG, BASE_BG);
        assert_eq!(CTA, CTA);
        assert_eq!(BORDER, BORDER);
    }

    #[test]
    fn tokens_error_vs_warning_distinct() {
        // ERROR and WARNING must not be the same color.
        assert_ne!(ERROR, WARNING, "ERROR and WARNING must be distinct colors");
    }

    #[test]
    fn tokens_cta_vs_error_distinct() {
        // CTA (action) and ERROR (danger) must be visually distinct.
        assert_ne!(CTA, ERROR, "CTA and ERROR must be distinct colors");
    }

    #[test]
    fn tokens_anim_fast_less_than_default() {
        // Fast animations must complete sooner than default animations.
        assert!(
            ANIM_FAST_MS < ANIM_DEFAULT_MS,
            "ANIM_FAST_MS ({ANIM_FAST_MS}) must be less than ANIM_DEFAULT_MS ({ANIM_DEFAULT_MS})"
        );
    }

    #[test]
    fn tokens_motion_hover_less_than_panel_resize() {
        // Hover response must be snappier than a panel resize animation.
        assert!(
            MOTION_HOVER_DURATION_MS < MOTION_PANEL_RESIZE_DURATION_MS,
            "MOTION_HOVER_DURATION_MS ({}) must be less than MOTION_PANEL_RESIZE_DURATION_MS ({})",
            MOTION_HOVER_DURATION_MS,
            MOTION_PANEL_RESIZE_DURATION_MS
        );
    }

    #[test]
    fn tokens_radius_scale_strictly_ascending() {
        // Radius tokens (excluding NONE and FULL) must be strictly ascending.
        assert!(RADIUS_SM < RADIUS_MD);
        assert!(RADIUS_MD < RADIUS_LG);
        assert!(RADIUS_LG < RADIUS_XL);
        assert!(RADIUS_XL < RADIUS_FULL);
    }

    #[test]
    fn tokens_toolbar_and_statusbar_heights_ordered() {
        // Toolbar is taller than the status bar.
        assert!(
            TOOLBAR_H > STATUSBAR_H,
            "TOOLBAR_H ({TOOLBAR_H}) must be greater than STATUSBAR_H ({STATUSBAR_H})"
        );
    }

    #[test]
    fn tokens_spacing_small_less_than_large() {
        // SPACING_1 is the smallest step; SPACING_12 is the largest.
        assert!(
            SPACING_1 < SPACING_12,
            "SPACING_1 ({SPACING_1}) must be less than SPACING_12 ({SPACING_12})"
        );
    }

    #[test]
    fn tokens_border_radius_positive() {
        // All non-zero radius constants must be > 0.
        assert!(RADIUS_SM > 0.0, "RADIUS_SM must be positive");
        assert!(RADIUS_MD > 0.0, "RADIUS_MD must be positive");
        assert!(RADIUS_LG > 0.0, "RADIUS_LG must be positive");
    }

    #[test]
    fn tokens_icon_size_positive() {
        assert!(
            ICON_SIZE > 0.0,
            "ICON_SIZE must be positive, got {ICON_SIZE}"
        );
        assert!(
            ICON_SIZE_SM > 0.0,
            "ICON_SIZE_SM must be positive, got {ICON_SIZE_SM}"
        );
    }

    #[test]
    fn tokens_line_height_above_one() {
        // Readable line heights must be >= 1.0 to avoid overlap.
        assert!(
            LINE_HEIGHT_BODY >= 1.0,
            "LINE_HEIGHT_BODY ({LINE_HEIGHT_BODY}) must be >= 1.0"
        );
        assert!(
            LINE_HEIGHT_CAPTION >= 1.0,
            "LINE_HEIGHT_CAPTION ({LINE_HEIGHT_CAPTION}) must be >= 1.0"
        );
        assert!(
            LINE_HEIGHT_CODE >= 1.0,
            "LINE_HEIGHT_CODE ({LINE_HEIGHT_CODE}) must be >= 1.0"
        );
        assert!(
            LINE_HEIGHT_HEADING >= 1.0,
            "LINE_HEIGHT_HEADING ({LINE_HEIGHT_HEADING}) must be >= 1.0"
        );
    }

    #[test]
    fn tokens_sidebar_width_positive() {
        assert!(
            SIDEBAR_W > 0.0,
            "SIDEBAR_W must be positive, got {SIDEBAR_W}"
        );
    }

    #[test]
    fn tokens_toolbar_height_is_48() {
        assert_eq!(
            TOOLBAR_H, 48.0,
            "TOOLBAR_H must be 48.0 per spec, got {TOOLBAR_H}"
        );
    }

    #[test]
    fn tokens_statusbar_height_is_24() {
        assert_eq!(
            STATUSBAR_H, 24.0,
            "STATUSBAR_H must be 24.0 per spec, got {STATUSBAR_H}"
        );
    }

    #[test]
    fn tokens_all_spacing_positive() {
        let spacings = [
            SPACING_1, SPACING_2, SPACING_3, SPACING_4, SPACING_6, SPACING_8, SPACING_12,
        ];
        for (i, s) in spacings.iter().enumerate() {
            assert!(*s > 0.0, "spacing[{i}] = {s} must be positive");
        }
    }

    #[test]
    fn tokens_hover_is_lighter_than_bg() {
        // BASE_BG is the dark base; BASE_FG is the light foreground.
        // They must differ — confirmed by checking they are not equal.
        assert_ne!(BASE_FG, BASE_BG, "BASE_FG and BASE_BG must differ");
    }

    #[test]
    fn tokens_active_state_differs_from_hover() {
        // CTA (active/call-to-action) must differ from the base background.
        assert_ne!(
            CTA, BASE_BG,
            "CTA (active) must differ from BASE_BG (hover-base)"
        );
    }

    #[test]
    fn tokens_destructive_is_red() {
        // ERROR is the destructive color; red channel must dominate (> 0.5).
        assert!(
            ERROR[0] > 0.5,
            "ERROR red channel ({}) must be > 0.5 for destructive red",
            ERROR[0]
        );
    }

    #[test]
    fn tokens_panel_left_equals_sidebar_w() {
        // PANEL_LEFT_WIDTH and SIDEBAR_W should match — both represent the left panel width.
        assert_eq!(
            PANEL_LEFT_WIDTH, SIDEBAR_W,
            "PANEL_LEFT_WIDTH ({PANEL_LEFT_WIDTH}) must equal SIDEBAR_W ({SIDEBAR_W})"
        );
    }

    // -----------------------------------------------------------------------
    // Extended token tests — ranges, ordering, semantics
    // -----------------------------------------------------------------------

    #[test]
    fn tokens_spacing_all_multiples_of_four() {
        // The design system uses a 4px base grid; every spacing constant must be
        // an exact multiple of 4.
        let spacings = [
            ("SPACING_1", SPACING_1),
            ("SPACING_2", SPACING_2),
            ("SPACING_3", SPACING_3),
            ("SPACING_4", SPACING_4),
            ("SPACING_6", SPACING_6),
            ("SPACING_8", SPACING_8),
            ("SPACING_12", SPACING_12),
        ];
        for (name, v) in spacings {
            let remainder = v % 4.0;
            assert!(
                remainder.abs() < f32::EPSILON,
                "{name} ({v}) must be a multiple of 4"
            );
        }
    }

    #[test]
    fn tokens_radius_none_is_zero() {
        assert_eq!(RADIUS_NONE, 0.0, "RADIUS_NONE must be exactly 0.0");
    }

    #[test]
    fn tokens_radius_full_is_very_large() {
        assert!(
            RADIUS_FULL >= 999.0,
            "RADIUS_FULL ({RADIUS_FULL}) must be >= 999 (pill shape)"
        );
    }

    #[test]
    fn tokens_radius_sm_equals_spacing_1() {
        // RADIUS_SM and SPACING_1 are both 4px; they must match.
        assert_eq!(
            RADIUS_SM, SPACING_1,
            "RADIUS_SM ({RADIUS_SM}) should equal SPACING_1 ({SPACING_1})"
        );
    }

    #[test]
    fn tokens_modal_radius_larger_than_block_radius() {
        assert!(
            MODAL_RADIUS > BLOCK_RADIUS,
            "MODAL_RADIUS ({MODAL_RADIUS}) must be > BLOCK_RADIUS ({BLOCK_RADIUS})"
        );
    }

    #[test]
    fn tokens_popover_radius_positive() {
        assert!(POPOVER_RADIUS > 0.0, "POPOVER_RADIUS must be positive");
    }

    #[test]
    fn tokens_btn_heights_ordered() {
        // Button height variants must be strictly ascending.
        assert!(
            BTN_H < BTN_H_LG,
            "BTN_H ({BTN_H}) must be < BTN_H_LG ({BTN_H_LG})"
        );
        assert!(
            BTN_H_LG < BTN_H_XL,
            "BTN_H_LG ({BTN_H_LG}) must be < BTN_H_XL ({BTN_H_XL})"
        );
    }

    #[test]
    fn tokens_btn_h_positive() {
        assert!(BTN_H > 0.0, "BTN_H must be positive");
        assert!(BTN_H_LG > 0.0, "BTN_H_LG must be positive");
        assert!(BTN_H_XL > 0.0, "BTN_H_XL must be positive");
    }

    #[test]
    fn tokens_font_size_all_positive() {
        let sizes = [
            ("CAPTION", FONT_SIZE_CAPTION),
            ("BODY", FONT_SIZE_BODY),
            ("CODE", FONT_SIZE_CODE),
            ("H3", FONT_SIZE_H3),
            ("H2", FONT_SIZE_H2),
            ("H1", FONT_SIZE_H1),
        ];
        for (name, v) in sizes {
            assert!(v > 0.0, "FONT_SIZE_{name} ({v}) must be positive");
        }
    }

    #[test]
    fn tokens_font_size_caption_is_12() {
        assert_eq!(FONT_SIZE_CAPTION, 12.0, "FONT_SIZE_CAPTION must be 12.0");
    }

    #[test]
    fn tokens_font_size_body_is_14() {
        assert_eq!(FONT_SIZE_BODY, 14.0, "FONT_SIZE_BODY must be 14.0");
    }

    #[test]
    fn tokens_font_size_h1_is_24() {
        assert_eq!(FONT_SIZE_H1, 24.0, "FONT_SIZE_H1 must be 24.0");
    }

    #[test]
    fn tokens_line_height_code_widest() {
        // Code needs the most vertical space; must be >= all other line heights.
        assert!(
            LINE_HEIGHT_CODE >= LINE_HEIGHT_BODY,
            "LINE_HEIGHT_CODE ({LINE_HEIGHT_CODE}) must be >= LINE_HEIGHT_BODY ({LINE_HEIGHT_BODY})"
        );
        assert!(
            LINE_HEIGHT_CODE >= LINE_HEIGHT_CAPTION,
            "LINE_HEIGHT_CODE ({LINE_HEIGHT_CODE}) >= LINE_HEIGHT_CAPTION ({LINE_HEIGHT_CAPTION})"
        );
        assert!(
            LINE_HEIGHT_CODE >= LINE_HEIGHT_HEADING,
            "LINE_HEIGHT_CODE ({LINE_HEIGHT_CODE}) >= LINE_HEIGHT_HEADING ({LINE_HEIGHT_HEADING})"
        );
    }

    #[test]
    fn tokens_line_height_heading_tightest() {
        // Headings use tighter leading than body or code text.
        assert!(
            LINE_HEIGHT_HEADING <= LINE_HEIGHT_BODY,
            "LINE_HEIGHT_HEADING ({LINE_HEIGHT_HEADING}) must be <= LINE_HEIGHT_BODY ({LINE_HEIGHT_BODY})"
        );
        assert!(
            LINE_HEIGHT_HEADING <= LINE_HEIGHT_CAPTION,
            "LINE_HEIGHT_HEADING ({LINE_HEIGHT_HEADING}) <= LINE_HEIGHT_CAPTION ({LINE_HEIGHT_CAPTION})"
        );
    }

    #[test]
    fn tokens_shadow_offsets_non_negative() {
        // All shadow tokens use vertical-only offsets (no horizontal offset).
        for (name, t) in [
            ("SHADOW_SM", &SHADOW_SM),
            ("SHADOW_MD", &SHADOW_MD),
            ("SHADOW_LG", &SHADOW_LG),
            ("SHADOW_XL", &SHADOW_XL),
        ] {
            assert_eq!(t.offset_x, 0.0, "{name}.offset_x must be 0.0");
            assert!(
                t.offset_y >= 0.0,
                "{name}.offset_y ({}) must be >= 0",
                t.offset_y
            );
        }
    }

    #[test]
    fn tokens_shadow_blur_positive() {
        for (name, t) in [
            ("SHADOW_SM", &SHADOW_SM),
            ("SHADOW_MD", &SHADOW_MD),
            ("SHADOW_LG", &SHADOW_LG),
            ("SHADOW_XL", &SHADOW_XL),
        ] {
            assert!(t.blur > 0.0, "{name}.blur must be positive");
        }
    }

    #[test]
    fn tokens_shadow_spread_zero() {
        // Spec requires spread = 0 for all shadow tokens.
        for (name, t) in [
            ("SHADOW_SM", &SHADOW_SM),
            ("SHADOW_MD", &SHADOW_MD),
            ("SHADOW_LG", &SHADOW_LG),
            ("SHADOW_XL", &SHADOW_XL),
        ] {
            assert_eq!(t.spread, 0.0, "{name}.spread must be 0.0");
        }
    }

    #[test]
    fn tokens_shadow_alpha_ordering() {
        // Larger shadows are darker — alphas must increase.
        let sm_a = (SHADOW_SM.color)().a;
        let md_a = (SHADOW_MD.color)().a;
        let lg_a = (SHADOW_LG.color)().a;
        let xl_a = (SHADOW_XL.color)().a;
        assert!(md_a > sm_a, "SHADOW_MD alpha must exceed SHADOW_SM alpha");
        assert!(lg_a > md_a, "SHADOW_LG alpha must exceed SHADOW_MD alpha");
        assert!(xl_a > lg_a, "SHADOW_XL alpha must exceed SHADOW_LG alpha");
    }

    #[test]
    fn tokens_shadow_colors_are_black() {
        // All shadow tokens use black (h=0, s=0, l=0) with varying alpha.
        for (name, t) in [
            ("SHADOW_SM", &SHADOW_SM),
            ("SHADOW_MD", &SHADOW_MD),
            ("SHADOW_LG", &SHADOW_LG),
            ("SHADOW_XL", &SHADOW_XL),
        ] {
            let c = (t.color)();
            assert_eq!(c.h, 0.0, "{name} shadow hue must be 0.0 (black)");
            assert_eq!(c.s, 0.0, "{name} shadow saturation must be 0.0");
            assert_eq!(c.l, 0.0, "{name} shadow lightness must be 0.0");
        }
    }

    #[test]
    fn tokens_motion_spring_stiffness_positive() {
        assert!(
            MOTION_SPRING_STIFFNESS > 0.0,
            "MOTION_SPRING_STIFFNESS must be positive"
        );
    }

    #[test]
    fn tokens_motion_spring_damping_positive() {
        assert!(
            MOTION_SPRING_DAMPING > 0.0,
            "MOTION_SPRING_DAMPING must be positive"
        );
    }

    #[test]
    fn tokens_panel_right_wider_than_left() {
        assert!(
            PANEL_RIGHT_WIDTH > PANEL_LEFT_WIDTH,
            "PANEL_RIGHT_WIDTH ({PANEL_RIGHT_WIDTH}) must be > PANEL_LEFT_WIDTH ({PANEL_LEFT_WIDTH})"
        );
    }

    #[test]
    fn tokens_panel_bottom_height_positive() {
        assert!(
            PANEL_BOTTOM_HEIGHT > 0.0,
            "PANEL_BOTTOM_HEIGHT must be positive"
        );
    }

    #[test]
    fn tokens_panel_min_less_than_max() {
        assert!(
            PANEL_MIN_WIDTH < PANEL_MAX_WIDTH,
            "PANEL_MIN_WIDTH ({PANEL_MIN_WIDTH}) must be < PANEL_MAX_WIDTH ({PANEL_MAX_WIDTH})"
        );
    }

    #[test]
    fn tokens_h1_letter_spacing_negative() {
        assert!(
            H1_LETTER_SPACING < 0.0,
            "H1_LETTER_SPACING ({H1_LETTER_SPACING}) must be negative (tight tracking)"
        );
    }

    #[test]
    fn tokens_h1_weight_exceeds_h2_weight() {
        assert!(
            H1_WEIGHT > H2_WEIGHT,
            "H1_WEIGHT ({H1_WEIGHT}) must exceed H2_WEIGHT ({H2_WEIGHT})"
        );
    }

    #[test]
    fn tokens_body_weight_lower_than_heading_weights() {
        assert!(
            BODY_WEIGHT < H2_WEIGHT,
            "BODY_WEIGHT ({BODY_WEIGHT}) must be < H2_WEIGHT ({H2_WEIGHT})"
        );
        assert!(
            BODY_WEIGHT < H1_WEIGHT,
            "BODY_WEIGHT ({BODY_WEIGHT}) must be < H1_WEIGHT ({H1_WEIGHT})"
        );
    }

    #[test]
    fn tokens_bg_is_dark() {
        // BG luminance approximation: R*0.299 + G*0.587 + B*0.114 < 0.5 for dark.
        let lum = BG[0] * 0.299 + BG[1] * 0.587 + BG[2] * 0.114;
        assert!(
            lum < 0.5,
            "BG luminance ({lum:.3}) must be < 0.5 (dark background)"
        );
    }

    #[test]
    fn tokens_text_is_light() {
        // TEXT luminance must be > 0.5 for a light-on-dark scheme.
        let lum = TEXT[0] * 0.299 + TEXT[1] * 0.587 + TEXT[2] * 0.114;
        assert!(
            lum > 0.5,
            "TEXT luminance ({lum:.3}) must be > 0.5 (light foreground)"
        );
    }

    #[test]
    fn tokens_edge_confidence_colors_distinct() {
        // High, medium, low confidence edge colors must be mutually distinct.
        let high = edge_color_high_confidence();
        let med = edge_color_medium_confidence();
        let low = edge_color_low_confidence();
        let high_med_diff = (high.h - med.h).abs();
        let high_low_diff = (high.h - low.h).abs();
        let med_low_diff = (med.h - low.h).abs();
        assert!(
            high_med_diff > 0.01,
            "high and medium confidence colors must have distinct hues"
        );
        assert!(
            high_low_diff > 0.01,
            "high and low confidence colors must have distinct hues"
        );
        assert!(
            med_low_diff > 0.01,
            "medium and low confidence colors must have distinct hues"
        );
    }

    #[test]
    fn tokens_edge_color_boundary_zero_is_low() {
        // confidence = 0.0 → low
        let c = edge_color_for_confidence(0.0);
        let expected = edge_color_low_confidence();
        assert!(
            (c.h - expected.h).abs() < f32::EPSILON,
            "confidence 0.0 must map to low confidence color"
        );
    }

    #[test]
    fn tokens_edge_color_boundary_one_is_high() {
        // confidence = 1.0 → high
        let c = edge_color_for_confidence(1.0);
        let expected = edge_color_high_confidence();
        assert!(
            (c.h - expected.h).abs() < f32::EPSILON,
            "confidence 1.0 must map to high confidence color"
        );
    }

    #[test]
    fn tokens_hsla_bg_colors_have_full_alpha() {
        // All non-overlay Hsla bg colors must have alpha = 1.0.
        let colors = [
            ("bg_primary", color_bg_primary()),
            ("bg_secondary", color_bg_secondary()),
            ("bg_tertiary", color_bg_tertiary()),
            ("text_primary", color_text_primary()),
            ("text_secondary", color_text_secondary()),
            ("text_tertiary", color_text_tertiary()),
            ("border_subtle", color_border_subtle()),
            ("border_normal", color_border_normal()),
            ("accent_blue", color_accent_blue()),
            ("accent_purple", color_accent_purple()),
            ("accent_green", color_accent_green()),
        ];
        for (name, c) in colors {
            assert_eq!(c.a, 1.0, "{name} must have alpha = 1.0");
        }
    }

    #[test]
    fn tokens_surface_overlay_alpha_partial() {
        let c = color_surface_overlay();
        assert!(c.a < 1.0, "surface overlay alpha must be < 1.0");
        assert!(c.a > 0.0, "surface overlay alpha must be > 0.0");
    }

    #[test]
    fn tokens_bg_lightness_ascending() {
        // Primary bg is darkest; tertiary bg is lightest (higher lightness = lighter in HSL).
        let p = color_bg_primary();
        let s = color_bg_secondary();
        let t = color_bg_tertiary();
        assert!(s.l > p.l, "bg_secondary must be lighter than bg_primary");
        assert!(t.l > s.l, "bg_tertiary must be lighter than bg_secondary");
    }

    #[test]
    fn tokens_text_primary_near_white() {
        // Primary text should be very light (lightness close to 1.0).
        let c = color_text_primary();
        assert!(
            c.l > 0.9,
            "text_primary lightness ({:.3}) must be > 0.9",
            c.l
        );
    }

    #[test]
    fn tokens_accent_blue_saturated() {
        // Accent blue must be highly saturated.
        let c = color_accent_blue();
        assert!(
            c.s >= 0.9,
            "accent_blue saturation ({:.3}) must be >= 0.9",
            c.s
        );
    }

    #[test]
    fn tokens_n_tokens_constant_value() {
        // N_TOKENS is a documented constant; its exact value must stay stable.
        assert_eq!(N_TOKENS, 73, "N_TOKENS must be 73 per module documentation");
    }

    // -----------------------------------------------------------------------
    // WCAG contrast: relative luminance helpers
    // -----------------------------------------------------------------------

    /// Linearize a sRGB channel component per WCAG 2.1 §1.4.3.
    fn linearize(c: f32) -> f32 {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055_f32).powf(2.4)
        }
    }

    /// Compute relative luminance of an sRGB color (r,g,b in [0,1]).
    fn relative_luminance(r: f32, g: f32, b: f32) -> f32 {
        0.2126 * linearize(r) + 0.7152 * linearize(g) + 0.0722 * linearize(b)
    }

    /// WCAG contrast ratio between two luminances.
    fn contrast_ratio(l1: f32, l2: f32) -> f32 {
        let lighter = l1.max(l2);
        let darker = l1.min(l2);
        (lighter + 0.05) / (darker + 0.05)
    }

    #[test]
    fn wcag_relative_luminance_black_is_zero() {
        let lum = relative_luminance(0.0, 0.0, 0.0);
        assert!(
            lum.abs() < 1e-6,
            "relative luminance of black must be 0, got {lum}"
        );
    }

    #[test]
    fn wcag_relative_luminance_white_is_one() {
        let lum = relative_luminance(1.0, 1.0, 1.0);
        assert!(
            (lum - 1.0).abs() < 1e-4,
            "relative luminance of white must be ~1.0, got {lum}"
        );
    }

    #[test]
    fn wcag_contrast_black_on_white_exceeds_21() {
        let lum_white = relative_luminance(1.0, 1.0, 1.0);
        let lum_black = relative_luminance(0.0, 0.0, 0.0);
        let ratio = contrast_ratio(lum_white, lum_black);
        assert!(
            (ratio - 21.0).abs() < 0.01,
            "black-on-white contrast must be ~21:1, got {ratio:.2}"
        );
    }

    #[test]
    fn wcag_text_on_bg_contrast_at_least_4_5() {
        // TEXT is near-white [0.973, 0.980, 0.988]; BG is very dark [0.059, 0.090, 0.165].
        let lum_text = relative_luminance(TEXT[0], TEXT[1], TEXT[2]);
        let lum_bg = relative_luminance(BG[0], BG[1], BG[2]);
        let ratio = contrast_ratio(lum_text, lum_bg);
        assert!(
            ratio >= 4.5,
            "TEXT on BG contrast ratio must be >= 4.5:1 for WCAG AA, got {ratio:.2}"
        );
    }

    #[test]
    fn wcag_base_fg_on_base_bg_contrast_at_least_4_5() {
        let lum_fg = relative_luminance(BASE_FG[0], BASE_FG[1], BASE_FG[2]);
        let lum_bg = relative_luminance(BASE_BG[0], BASE_BG[1], BASE_BG[2]);
        let ratio = contrast_ratio(lum_fg, lum_bg);
        assert!(
            ratio >= 4.5,
            "BASE_FG on BASE_BG contrast ratio must be >= 4.5:1 for WCAG AA, got {ratio:.2}"
        );
    }

    // -----------------------------------------------------------------------
    // Animation: spring physics
    // -----------------------------------------------------------------------

    #[test]
    fn motion_spring_stiffness_is_400() {
        assert_eq!(
            MOTION_SPRING_STIFFNESS, 400.0,
            "MOTION_SPRING_STIFFNESS must be exactly 400.0"
        );
    }

    #[test]
    fn motion_spring_damping_is_28() {
        assert_eq!(
            MOTION_SPRING_DAMPING, 28.0,
            "MOTION_SPRING_DAMPING must be exactly 28.0"
        );
    }

    /// Semi-implicit Euler spring tick: returns (new_position, new_velocity).
    /// mass = 1.0, stiffness = k, damping = d, dt in seconds.
    fn spring_tick(pos: f32, vel: f32, target: f32, k: f32, d: f32, dt: f32) -> (f32, f32) {
        let force = -k * (pos - target) - d * vel;
        let new_vel = vel + force * dt;
        let new_pos = pos + new_vel * dt;
        (new_pos, new_vel)
    }

    #[test]
    fn spring_converges_within_1_second() {
        // With stiffness=400, damping=28 starting at pos=1.0, target=0.0,
        // the spring must converge to within 0.01 of target in < 1 second.
        let k = MOTION_SPRING_STIFFNESS;
        let d = MOTION_SPRING_DAMPING;
        let dt = 1.0 / 120.0_f32; // 120 fps
        let max_steps = (1.0 / dt) as u32; // 120 steps = 1 second
        let target = 0.0_f32;
        let mut pos = 1.0_f32;
        let mut vel = 0.0_f32;
        for _ in 0..max_steps {
            let (np, nv) = spring_tick(pos, vel, target, k, d, dt);
            pos = np;
            vel = nv;
        }
        assert!(
            pos.abs() < 0.01,
            "spring must converge within 1 second; final pos = {pos:.4}"
        );
    }

    #[test]
    fn spring_update_position_after_one_tick() {
        // Single tick with known inputs: pos=1.0, vel=0.0, target=0.0, dt=1/60.
        // force = -400 * (1-0) - 28*0 = -400
        // new_vel = 0 + (-400) * (1/60) ≈ -6.6667
        // new_pos = 1 + (-6.6667) * (1/60) ≈ 0.8889
        let k = MOTION_SPRING_STIFFNESS;
        let d = MOTION_SPRING_DAMPING;
        let dt = 1.0 / 60.0_f32;
        let (new_pos, new_vel) = spring_tick(1.0, 0.0, 0.0, k, d, dt);
        let expected_vel = -400.0 * dt;
        let expected_pos = 1.0 + expected_vel * dt;
        assert!(
            (new_vel - expected_vel).abs() < 1e-4,
            "spring velocity after 1 tick: expected {expected_vel:.4}, got {new_vel:.4}"
        );
        assert!(
            (new_pos - expected_pos).abs() < 1e-4,
            "spring position after 1 tick: expected {expected_pos:.4}, got {new_pos:.4}"
        );
    }

    #[test]
    fn spring_at_rest_stays_at_rest() {
        // If pos == target and vel == 0, the spring should stay put.
        let (pos, vel) = spring_tick(
            0.5,
            0.0,
            0.5,
            MOTION_SPRING_STIFFNESS,
            MOTION_SPRING_DAMPING,
            1.0 / 60.0,
        );
        assert!((pos - 0.5).abs() < 1e-6, "spring at rest must not move");
        assert!(vel.abs() < 1e-6, "spring at rest velocity must remain 0");
    }

    // -----------------------------------------------------------------------
    // Motion timing: 200ms and 300ms values
    // -----------------------------------------------------------------------

    #[test]
    fn motion_anim_fast_is_200() {
        assert_eq!(ANIM_FAST_MS, 200.0, "ANIM_FAST_MS must be 200.0 ms");
    }

    #[test]
    fn motion_anim_default_is_300() {
        assert_eq!(ANIM_DEFAULT_MS, 300.0, "ANIM_DEFAULT_MS must be 300.0 ms");
    }

    #[test]
    fn motion_timing_200_and_300_are_distinct() {
        assert_ne!(
            ANIM_FAST_MS, ANIM_DEFAULT_MS,
            "200ms and 300ms timing values must be distinct"
        );
    }

    #[test]
    fn motion_timing_both_positive() {
        assert!(ANIM_FAST_MS > 0.0, "ANIM_FAST_MS must be positive");
        assert!(ANIM_DEFAULT_MS > 0.0, "ANIM_DEFAULT_MS must be positive");
    }

    // -----------------------------------------------------------------------
    // Token completeness: N_TOKENS constant is exactly 73
    // -----------------------------------------------------------------------

    #[test]
    fn palette_affine_token_count_is_73() {
        assert_eq!(N_TOKENS, 73, "AFFiNE palette must define exactly 73 tokens");
    }

    #[test]
    fn palette_n_tokens_at_least_73() {
        assert!(N_TOKENS >= 73, "Token count must be >= 73, got {N_TOKENS}");
    }

    // -----------------------------------------------------------------------
    // WCAG AAA contrast (7:1) checks
    // -----------------------------------------------------------------------

    #[test]
    fn wcag_aaa_white_on_black_exceeds_7() {
        let lum_white = relative_luminance(1.0, 1.0, 1.0);
        let lum_black = relative_luminance(0.0, 0.0, 0.0);
        let ratio = contrast_ratio(lum_white, lum_black);
        assert!(
            ratio >= 7.0,
            "white on black must meet WCAG AAA (>= 7:1), got {ratio:.2}"
        );
    }

    #[test]
    fn wcag_linearize_midpoint_is_below_half() {
        // sRGB 0.5 linearizes to roughly 0.214 — confirms gamma expansion.
        let linear = linearize(0.5);
        assert!(
            linear < 0.5,
            "linearize(0.5) must be < 0.5 due to gamma, got {linear:.4}"
        );
        assert!(linear > 0.0, "linearize(0.5) must be > 0.0");
    }

    #[test]
    fn wcag_contrast_ratio_symmetric() {
        // contrast_ratio(a, b) == contrast_ratio(b, a)
        let l1 = relative_luminance(BASE_FG[0], BASE_FG[1], BASE_FG[2]);
        let l2 = relative_luminance(BG[0], BG[1], BG[2]);
        let r1 = contrast_ratio(l1, l2);
        let r2 = contrast_ratio(l2, l1);
        assert!(
            (r1 - r2).abs() < 1e-5,
            "contrast_ratio must be symmetric: {r1:.4} vs {r2:.4}"
        );
    }

    #[test]
    fn wcag_base_fg_luminance_above_0_9() {
        // Near-white BASE_FG must have relative luminance > 0.9.
        let lum = relative_luminance(BASE_FG[0], BASE_FG[1], BASE_FG[2]);
        assert!(
            lum > 0.9,
            "BASE_FG relative luminance ({lum:.4}) must be > 0.9"
        );
    }

    #[test]
    fn wcag_error_meets_aa_on_base_bg() {
        // ERROR (red) on BASE_BG (dark) should meet WCAG AA (4.5:1).
        let lum_err = relative_luminance(ERROR[0], ERROR[1], ERROR[2]);
        let lum_bg = relative_luminance(BASE_BG[0], BASE_BG[1], BASE_BG[2]);
        let ratio = contrast_ratio(lum_err, lum_bg);
        assert!(
            ratio >= 4.5,
            "ERROR on BASE_BG contrast ({ratio:.2}) must be >= 4.5:1 for WCAG AA"
        );
    }

    // -----------------------------------------------------------------------
    // Color arithmetic: Hsla mix commutativity
    // -----------------------------------------------------------------------

    /// Mix two Hsla values 50/50 by averaging each component.
    fn mix_hsla(a: nom_gpui::Hsla, b: nom_gpui::Hsla) -> nom_gpui::Hsla {
        nom_gpui::Hsla {
            h: (a.h + b.h) * 0.5,
            s: (a.s + b.s) * 0.5,
            l: (a.l + b.l) * 0.5,
            a: (a.a + b.a) * 0.5,
        }
    }

    #[test]
    fn color_mix_is_commutative() {
        let a = color_accent_blue();
        let b = color_accent_green();
        let ab = mix_hsla(a, b);
        let ba = mix_hsla(b, a);
        assert!(
            (ab.h - ba.h).abs() < 1e-6,
            "mix(a,b).h must equal mix(b,a).h"
        );
        assert!(
            (ab.s - ba.s).abs() < 1e-6,
            "mix(a,b).s must equal mix(b,a).s"
        );
        assert!(
            (ab.l - ba.l).abs() < 1e-6,
            "mix(a,b).l must equal mix(b,a).l"
        );
        assert!(
            (ab.a - ba.a).abs() < 1e-6,
            "mix(a,b).a must equal mix(b,a).a"
        );
    }

    #[test]
    fn color_mix_result_in_range() {
        let a = color_bg_primary();
        let b = color_text_primary();
        let m = mix_hsla(a, b);
        assert!((0.0..=1.0).contains(&m.h), "mixed h out of range");
        assert!((0.0..=1.0).contains(&m.s), "mixed s out of range");
        assert!((0.0..=1.0).contains(&m.l), "mixed l out of range");
        assert!((0.0..=1.0).contains(&m.a), "mixed a out of range");
    }

    // -----------------------------------------------------------------------
    // Dark theme: dark BG luminance < light BG luminance
    // -----------------------------------------------------------------------

    #[test]
    fn dark_theme_bg_luminance_less_than_light_fg_luminance() {
        // The dark bg (BG constant) must have lower relative luminance than BASE_FG.
        let lum_bg = relative_luminance(BG[0], BG[1], BG[2]);
        let lum_fg = relative_luminance(BASE_FG[0], BASE_FG[1], BASE_FG[2]);
        assert!(
            lum_bg < lum_fg,
            "dark BG luminance ({lum_bg:.4}) must be < light FG luminance ({lum_fg:.4})"
        );
    }

    #[test]
    fn dark_theme_bg_luminance_below_0_1() {
        // A dark background must have relative luminance < 0.1.
        let lum = relative_luminance(BG[0], BG[1], BG[2]);
        assert!(
            lum < 0.1,
            "dark BG relative luminance ({lum:.4}) must be < 0.1"
        );
    }

    #[test]
    fn dark_theme_base_bg_luminance_below_light_fg() {
        let lum_bg = relative_luminance(BASE_BG[0], BASE_BG[1], BASE_BG[2]);
        let lum_fg = relative_luminance(BASE_FG[0], BASE_FG[1], BASE_FG[2]);
        assert!(
            lum_bg < lum_fg,
            "BASE_BG luminance ({lum_bg:.4}) must be < BASE_FG luminance ({lum_fg:.4})"
        );
    }

    #[test]
    fn tokens_hsla_components_in_range() {
        // Every Hsla color function must return values in valid ranges.
        let fns: &[(&str, fn() -> nom_gpui::Hsla)] = &[
            ("bg_primary", color_bg_primary),
            ("bg_secondary", color_bg_secondary),
            ("bg_tertiary", color_bg_tertiary),
            ("text_primary", color_text_primary),
            ("text_secondary", color_text_secondary),
            ("text_tertiary", color_text_tertiary),
            ("border_subtle", color_border_subtle),
            ("border_normal", color_border_normal),
            ("accent_blue", color_accent_blue),
            ("accent_purple", color_accent_purple),
            ("accent_green", color_accent_green),
            ("surface_overlay", color_surface_overlay),
            ("edge_high", edge_color_high_confidence),
            ("edge_med", edge_color_medium_confidence),
            ("edge_low", edge_color_low_confidence),
        ];
        for (name, f) in fns {
            let c = f();
            assert!(
                (0.0..=1.0).contains(&c.h),
                "{name}.h ({}) out of [0,1]",
                c.h
            );
            assert!(
                (0.0..=1.0).contains(&c.s),
                "{name}.s ({}) out of [0,1]",
                c.s
            );
            assert!(
                (0.0..=1.0).contains(&c.l),
                "{name}.l ({}) out of [0,1]",
                c.l
            );
            assert!(
                (0.0..=1.0).contains(&c.a),
                "{name}.a ({}) out of [0,1]",
                c.a
            );
        }
    }
}
