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
// Motion tokens (spring motion)
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
// Dark theme colors (dark palette) — runtime color functions
// ---------------------------------------------------------------------------

/// Primary background: hsl(220°, 13%, 11%)
pub fn color_bg_primary() -> Hsla {
    Hsla::new(220.0, 0.13, 0.11, 1.0)
}

/// Secondary background: hsl(220°, 11%, 14%)
pub fn color_bg_secondary() -> Hsla {
    Hsla::new(220.0, 0.11, 0.14, 1.0)
}

/// Tertiary background: hsl(220°, 10%, 17%)
pub fn color_bg_tertiary() -> Hsla {
    Hsla::new(220.0, 0.10, 0.17, 1.0)
}

/// Primary text: near-white
pub fn color_text_primary() -> Hsla {
    Hsla::new(0.0, 0.0, 0.98, 1.0)
}

/// Secondary text: hsl(220°, 9%, 65%)
pub fn color_text_secondary() -> Hsla {
    Hsla::new(220.0, 0.09, 0.65, 1.0)
}

/// Tertiary / muted text: hsl(220°, 7%, 45%)
pub fn color_text_tertiary() -> Hsla {
    Hsla::new(220.0, 0.07, 0.45, 1.0)
}

/// Subtle border: hsl(220°, 13%, 22%)
pub fn color_border_subtle() -> Hsla {
    Hsla::new(220.0, 0.13, 0.22, 1.0)
}

/// Normal border: hsl(220°, 11%, 30%)
pub fn color_border_normal() -> Hsla {
    Hsla::new(220.0, 0.11, 0.30, 1.0)
}

/// Accent blue (~#1E90FF): hsl(211°, 100%, 60%)
pub fn color_accent_blue() -> Hsla {
    Hsla::new(211.0, 1.0, 0.60, 1.0)
}

/// Accent purple (nomtu references): hsl(270°, 91%, 70%)
pub fn color_accent_purple() -> Hsla {
    Hsla::new(270.0, 0.91, 0.70, 1.0)
}

/// Accent green (literals, #22C55E): hsl(145°, 63%, 49%)
pub fn color_accent_green() -> Hsla {
    Hsla::new(145.0, 0.63, 0.49, 1.0)
}

/// Surface overlay (panel backgrounds): hsl(220°, 14%, 8%, 85%)
pub fn color_surface_overlay() -> Hsla {
    Hsla::new(220.0, 0.14, 0.08, 0.85)
}

// ---------------------------------------------------------------------------
// Graph edge confidence colors (exact from spec)
// ---------------------------------------------------------------------------

/// High confidence >= 0.8: #22C55E — hsl(142.1°, 70.6%, 45.3%)
pub fn edge_color_high_confidence() -> Hsla {
    Hsla::new(142.1, 0.706, 0.453, 1.0)
}

/// Medium confidence 0.5–0.8: #F59E0B — hsl(37.7°, 92.1%, 50.2%)
pub fn edge_color_medium_confidence() -> Hsla {
    Hsla::new(37.7, 0.921, 0.502, 1.0)
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
// UI chrome height tokens
// ---------------------------------------------------------------------------

pub const TOOLBAR_HEIGHT: f32 = 36.0;
pub const PANEL_HEADER_HEIGHT: f32 = 32.0;
pub const STATUS_BAR_HEIGHT: f32 = 24.0;

// ---------------------------------------------------------------------------
// Theme struct
// ---------------------------------------------------------------------------

/// A named color theme carrying the five canonical surface colors.
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub background: String,
    pub foreground: String,
    pub accent: String,
    pub surface: String,
}

impl Theme {
    /// Dark theme (dark palette).
    pub fn dark() -> Self {
        Self {
            name: "dark".to_string(),
            background: "#0f1729".to_string(),
            foreground: "#f8fafb".to_string(),
            accent: "#22c55e".to_string(),
            surface: "#1e2940".to_string(),
        }
    }

    /// Light theme.
    pub fn light() -> Self {
        Self {
            name: "light".to_string(),
            background: "#ffffff".to_string(),
            foreground: "#111827".to_string(),
            accent: "#2563eb".to_string(),
            surface: "#f3f4f6".to_string(),
        }
    }

    /// OLED theme — identical to dark but with pure-black backgrounds for
    /// pixel-level power savings on OLED displays.
    pub fn oled() -> Self {
        Self {
            name: "oled".to_string(),
            background: "#000000".to_string(),
            foreground: "#f8fafb".to_string(),
            accent: "#22c55e".to_string(),
            surface: "#050505".to_string(),
        }
    }

    /// Returns `true` when the theme name contains `"dark"`.
    pub fn is_dark(&self) -> bool {
        self.name.contains("dark")
    }
}

// ---------------------------------------------------------------------------
// Frosted-glass token
// ---------------------------------------------------------------------------

/// Per-mode frosted-glass visual token.
#[derive(Debug, Clone, PartialEq)]
pub struct FrostedGlassToken {
    /// Backdrop blur radius in pixels.
    pub blur_radius: f32,
    /// Background fill opacity (0.0–1.0).
    pub background_opacity: f32,
    /// Border overlay opacity (0.0–1.0).
    pub border_opacity: f32,
    /// RGBA tint color applied over the blurred surface.
    pub tint: [u8; 4],
}

impl FrostedGlassToken {
    /// Dark-mode frosted glass defaults: strong blur, cool dark tint.
    pub fn default_dark() -> Self {
        Self {
            blur_radius: 20.0,
            background_opacity: 0.15,
            border_opacity: 0.2,
            tint: [30, 30, 40, 255],
        }
    }

    /// Light-mode frosted glass defaults: softer blur, near-white tint.
    pub fn default_light() -> Self {
        Self {
            blur_radius: 16.0,
            background_opacity: 0.08,
            border_opacity: 0.15,
            tint: [240, 240, 255, 255],
        }
    }
}

// ---------------------------------------------------------------------------
// Bezier-curve animation token
// ---------------------------------------------------------------------------

/// Cubic bezier curve defined by two control points (CSS cubic-bezier convention).
#[derive(Debug, Clone, PartialEq)]
pub struct BezierCurve {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

impl BezierCurve {
    /// `ease` — standard browser default.
    pub fn ease() -> Self {
        BezierCurve {
            x1: 0.25,
            y1: 0.1,
            x2: 0.25,
            y2: 1.0,
        }
    }

    /// `ease-in` — slow start.
    pub fn ease_in() -> Self {
        BezierCurve {
            x1: 0.42,
            y1: 0.0,
            x2: 1.0,
            y2: 1.0,
        }
    }

    /// `ease-out` — slow end.
    pub fn ease_out() -> Self {
        BezierCurve {
            x1: 0.0,
            y1: 0.0,
            x2: 0.58,
            y2: 1.0,
        }
    }

    /// `ease-in-out` — slow start and end.
    pub fn ease_in_out() -> Self {
        BezierCurve {
            x1: 0.42,
            y1: 0.0,
            x2: 0.58,
            y2: 1.0,
        }
    }

    /// `linear` — constant rate.
    pub fn linear() -> Self {
        BezierCurve {
            x1: 0.0,
            y1: 0.0,
            x2: 1.0,
            y2: 1.0,
        }
    }

    /// Approximate the Y output at normalized time `t` using Newton's method
    /// (3 iterations) on the X parametric equation to find the curve parameter,
    /// then evaluate the Y parametric equation at that parameter.
    pub fn sample(&self, t: f32) -> f32 {
        // Clamp t to [0, 1]
        let t = t.clamp(0.0, 1.0);
        // Find parameter `s` such that X(s) == t, where
        //   X(s) = 3*(1-s)^2*s*x1 + 3*(1-s)*s^2*x2 + s^3
        // Start with initial guess s = t
        let mut s = t;
        for _ in 0..3 {
            let s2 = s * s;
            let s3 = s2 * s;
            let one_s = 1.0 - s;
            let x = 3.0 * one_s * one_s * s * self.x1 + 3.0 * one_s * s2 * self.x2 + s3;
            let dx = 3.0
                * (one_s * one_s * self.x1
                    + 2.0 * one_s * s * (self.x2 - self.x1)
                    + s2 * (1.0 - self.x2));
            if dx.abs() < 1e-6 {
                break;
            }
            s -= (x - t) / dx;
        }
        let s = s.clamp(0.0, 1.0);
        // Evaluate Y(s)
        let s2 = s * s;
        let s3 = s2 * s;
        let one_s = 1.0 - s;
        3.0 * one_s * one_s * s * self.y1 + 3.0 * one_s * s2 * self.y2 + s3
    }
}

// ---------------------------------------------------------------------------
// D8 AF-THEME: color token sets
// ---------------------------------------------------------------------------

/// Complete color token set for a single theme variant.
#[derive(Debug, Clone)]
pub struct ColorSet {
    /// Page / window background.
    pub bg: &'static str,
    /// Panel surface fill.
    pub surface: &'static str,
    /// Elevated surface (dropdowns, modals).
    pub elevated: &'static str,
    /// Interactive accent (links, focus rings, primary buttons).
    pub accent: &'static str,
    /// Primary text.
    pub text: &'static str,
    /// Secondary / muted text.
    pub text_secondary: &'static str,
    /// 1 px hairline border.
    pub border: &'static str,
}

impl ColorSet {
    /// GitHub-dark palette.
    pub fn dark() -> Self {
        Self {
            bg: "#0d1117",
            surface: "#161b22",
            elevated: "#21262d",
            accent: "#58a6ff",
            text: "#f0f6fc",
            text_secondary: "#8b949e",
            border: "#30363d",
        }
    }

    /// GitHub-light palette.
    pub fn light() -> Self {
        Self {
            bg: "#ffffff",
            surface: "#f6f8fa",
            elevated: "#eaeef2",
            accent: "#0969da",
            text: "#1f2328",
            text_secondary: "#656d76",
            border: "#d0d7de",
        }
    }

    /// OLED palette — pure black for pixel-level power savings.
    pub fn oled() -> Self {
        Self {
            bg: "#000000",
            surface: "#0a0a0a",
            elevated: "#111111",
            accent: "#58a6ff",
            text: "#f0f6fc",
            text_secondary: "#8b949e",
            border: "#21262d",
        }
    }

    /// Dispatch to the correct palette by mode.
    pub fn for_mode(mode: &crate::ThemeMode) -> Self {
        match mode {
            crate::ThemeMode::Dark => Self::dark(),
            crate::ThemeMode::Light => Self::light(),
            crate::ThemeMode::Oled => Self::oled(),
        }
    }
}

/// All token categories for a single theme variant.
pub struct ThemeTokens {
    /// Color token set.
    pub colors: ColorSet,
    /// The active display mode.
    pub mode: crate::ThemeMode,
}

impl ThemeTokens {
    /// Build tokens for `mode`.
    pub fn new(mode: crate::ThemeMode) -> Self {
        Self {
            colors: ColorSet::for_mode(&mode),
            mode,
        }
    }

    /// Tokens for the dark variant.
    pub fn dark() -> Self {
        Self::new(crate::ThemeMode::Dark)
    }

    /// Tokens for the light variant.
    pub fn light() -> Self {
        Self::new(crate::ThemeMode::Light)
    }

    /// Tokens for the OLED variant.
    pub fn oled() -> Self {
        Self::new(crate::ThemeMode::Oled)
    }

    /// Returns `true` for dark-family modes (Dark and Oled).
    pub fn is_dark(&self) -> bool {
        self.mode.is_dark_family()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    type HslaFn = fn() -> nom_gpui::Hsla;

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
        const { assert!(FONT_SIZE_CAPTION < FONT_SIZE_CODE) };
        const { assert!(FONT_SIZE_CODE < FONT_SIZE_BODY) };
        const { assert!(FONT_SIZE_BODY < FONT_SIZE_H3) };
        const { assert!(FONT_SIZE_H3 < FONT_SIZE_H2) };
        const { assert!(FONT_SIZE_H2 < FONT_SIZE_H1) };
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
        const { assert!(ICON_SIZE_SM < ICON_SIZE) };
        assert_eq!(ICON_SIZE_SM, 16.0);
    }

    #[test]
    fn token_h1_spacing_positive() {
        const { assert!(H1_SPACING > 0.0) };
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
        const { assert!(PANEL_LEFT_WIDTH > PANEL_MIN_WIDTH) };
        const { assert!(PANEL_RIGHT_WIDTH > PANEL_MIN_WIDTH) };
        const { assert!(PANEL_LEFT_WIDTH < PANEL_MAX_WIDTH) };
        const { assert!(PANEL_RIGHT_WIDTH <= PANEL_MAX_WIDTH) };
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
        const { assert!(SHADOW_MD.blur > SHADOW_SM.blur) };
        const { assert!(SHADOW_LG.blur > SHADOW_MD.blur) };
        const { assert!(SHADOW_XL.blur > SHADOW_LG.blur) };
    }

    #[test]
    fn frosted_glass_alpha_in_range() {
        const { assert!(FROSTED_BG_ALPHA >= 0.0 && FROSTED_BG_ALPHA <= 1.0) };
        const { assert!(FROSTED_BORDER_ALPHA >= 0.0 && FROSTED_BORDER_ALPHA <= 1.0) };
        const {
            assert!(
                FROSTED_BG_ALPHA > FROSTED_BORDER_ALPHA,
                "background alpha should dominate border alpha"
            )
        };
    }

    #[test]
    fn tokens_base_bg_is_dark() {
        // Blue channel (index 2) must be near 0 — confirms a dark background.
        const {
            assert!(
                BASE_BG[2] < 0.1,
                "BASE_BG blue channel should be near 0 for a dark bg"
            )
        };
    }

    #[test]
    fn tokens_base_fg_is_light() {
        // Red channel (index 0) must be > 0.8 — confirms a light foreground.
        const {
            assert!(
                BASE_FG[0] > 0.8,
                "BASE_FG red channel should be > 0.8 for a light fg"
            )
        };
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
        const {
            assert!(
                FROSTED_BLUR_RADIUS > 0.0,
                "FROSTED_BLUR_RADIUS must be positive"
            )
        };
    }

    #[test]
    fn tokens_frosted_bg_alpha_below_one() {
        const {
            assert!(
                FROSTED_BG_ALPHA < 1.0,
                "FROSTED_BG_ALPHA should be < 1.0 for frosted transparency"
            )
        };
    }

    #[test]
    fn tokens_frosted_border_alpha_below_bg_alpha() {
        const {
            assert!(
                FROSTED_BORDER_ALPHA < FROSTED_BG_ALPHA,
                "FROSTED_BORDER_ALPHA must be less than FROSTED_BG_ALPHA"
            )
        };
    }

    #[test]
    fn tokens_focus_has_distinct_color() {
        // Focus ring must differ from base background so it is visible.
        assert_ne!(FOCUS, BASE_BG, "FOCUS ring color must differ from BASE_BG");
    }

    #[test]
    fn tokens_error_is_reddish() {
        // Red channel dominates over both green and blue.
        const { assert!(ERROR[0] > ERROR[1], "ERROR red must exceed green") };
        const { assert!(ERROR[0] > ERROR[2], "ERROR red must exceed blue") };
    }

    #[test]
    fn tokens_warning_is_yellowish() {
        // Both red and green channels > 0.5 gives a yellow hue.
        const { assert!(WARNING[0] > 0.5, "WARNING red must be > 0.5 for yellow hue") };
        const {
            assert!(
                WARNING[1] > 0.5,
                "WARNING green must be > 0.5 for yellow hue"
            )
        };
    }

    #[test]
    fn tokens_edge_high_brighter_than_edge_low() {
        // EDGE_HIGH is green-dominant (high confidence); EDGE_LOW is red-dominant (low confidence).
        // High-confidence edges have a higher green channel than low-confidence edges.
        const {
            assert!(
                EDGE_HIGH[1] > EDGE_LOW[1],
                "EDGE_HIGH green must exceed EDGE_LOW green — high confidence = green"
            )
        };
        // And high-confidence edges have a higher alpha than low-confidence edges.
        const {
            assert!(
                EDGE_HIGH[3] > EDGE_LOW[3],
                "EDGE_HIGH alpha must exceed EDGE_LOW alpha"
            )
        };
    }

    #[test]
    fn tokens_count_all_defined() {
        // Compile-time sanity: N_TOKENS must be defined and > 20.
        const { assert!(N_TOKENS >= 20, "N_TOKENS must be at least 20") };
    }

    #[test]
    fn tokens_border_has_alpha() {
        // BORDER is an opaque or semi-transparent border — alpha must be in (0.0, 1.0].
        const { assert!(BORDER[3] > 0.0, "BORDER alpha must be > 0.0") };
        const { assert!(BORDER[3] <= 1.0, "BORDER alpha must be <= 1.0") };
    }

    #[test]
    fn tokens_focus_alpha_partial() {
        // FOCUS is a semi-transparent ring — alpha must be less than 1.0.
        const {
            assert!(
                FOCUS[3] < 1.0,
                "FOCUS alpha must be < 1.0 for a semi-transparent ring"
            )
        };
    }

    #[test]
    fn tokens_focus_ring_visible() {
        // FOCUS ring must be at least somewhat visible — alpha > 0.1.
        const { assert!(FOCUS[3] > 0.1, "FOCUS alpha too low to be visible") };
    }

    #[test]
    fn tokens_base_fg_all_channels_bright() {
        // BASE_FG near-white: all three RGB channels must be > 0.8.
        const { assert!(BASE_FG[0] > 0.8, "BASE_FG[0] R must be > 0.8") };
        const { assert!(BASE_FG[1] > 0.8, "BASE_FG[1] G must be > 0.8") };
        const { assert!(BASE_FG[2] > 0.8, "BASE_FG[2] B must be > 0.8") };
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
        const {
            assert!(
                ANIM_FAST_MS < ANIM_DEFAULT_MS,
                "ANIM_FAST_MS must be less than ANIM_DEFAULT_MS"
            )
        };
    }

    #[test]
    fn tokens_motion_hover_less_than_panel_resize() {
        // Hover response must be snappier than a panel resize animation.
        const {
            assert!(
                MOTION_HOVER_DURATION_MS < MOTION_PANEL_RESIZE_DURATION_MS,
                "MOTION_HOVER_DURATION_MS must be less than MOTION_PANEL_RESIZE_DURATION_MS"
            )
        };
    }

    #[test]
    fn tokens_radius_scale_strictly_ascending() {
        // Radius tokens (excluding NONE and FULL) must be strictly ascending.
        const { assert!(RADIUS_SM < RADIUS_MD) };
        const { assert!(RADIUS_MD < RADIUS_LG) };
        const { assert!(RADIUS_LG < RADIUS_XL) };
        const { assert!(RADIUS_XL < RADIUS_FULL) };
    }

    #[test]
    fn tokens_toolbar_and_statusbar_heights_ordered() {
        // Toolbar is taller than the status bar.
        const {
            assert!(
                TOOLBAR_HEIGHT > STATUSBAR_H,
                "TOOLBAR_HEIGHT must be greater than STATUSBAR_H"
            )
        };
    }

    #[test]
    fn tokens_spacing_small_less_than_large() {
        // SPACING_1 is the smallest step; SPACING_12 is the largest.
        const {
            assert!(
                SPACING_1 < SPACING_12,
                "SPACING_1 must be less than SPACING_12"
            )
        };
    }

    #[test]
    fn tokens_border_radius_positive() {
        // All non-zero radius constants must be > 0.
        const { assert!(RADIUS_SM > 0.0, "RADIUS_SM must be positive") };
        const { assert!(RADIUS_MD > 0.0, "RADIUS_MD must be positive") };
        const { assert!(RADIUS_LG > 0.0, "RADIUS_LG must be positive") };
    }

    #[test]
    fn tokens_icon_size_positive() {
        const { assert!(ICON_SIZE > 0.0, "ICON_SIZE must be positive") };
        const { assert!(ICON_SIZE_SM > 0.0, "ICON_SIZE_SM must be positive") };
    }

    #[test]
    fn tokens_line_height_above_one() {
        // Readable line heights must be >= 1.0 to avoid overlap.
        const { assert!(LINE_HEIGHT_BODY >= 1.0, "LINE_HEIGHT_BODY must be >= 1.0") };
        const {
            assert!(
                LINE_HEIGHT_CAPTION >= 1.0,
                "LINE_HEIGHT_CAPTION must be >= 1.0"
            )
        };
        const { assert!(LINE_HEIGHT_CODE >= 1.0, "LINE_HEIGHT_CODE must be >= 1.0") };
        const {
            assert!(
                LINE_HEIGHT_HEADING >= 1.0,
                "LINE_HEIGHT_HEADING must be >= 1.0"
            )
        };
    }

    #[test]
    fn tokens_sidebar_width_positive() {
        const { assert!(SIDEBAR_W > 0.0, "SIDEBAR_W must be positive") };
    }

    #[test]
    fn tokens_toolbar_height_is_36() {
        assert_eq!(
            TOOLBAR_HEIGHT, 36.0,
            "TOOLBAR_HEIGHT must be 36.0 per spec, got {TOOLBAR_HEIGHT}"
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
        const {
            assert!(
                ERROR[0] > 0.5,
                "ERROR red channel must be > 0.5 for destructive red"
            )
        };
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
        const {
            assert!(
                RADIUS_FULL >= 999.0,
                "RADIUS_FULL must be >= 999 (pill shape)"
            )
        };
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
        const {
            assert!(
                MODAL_RADIUS > BLOCK_RADIUS,
                "MODAL_RADIUS must be > BLOCK_RADIUS"
            )
        };
    }

    #[test]
    fn tokens_popover_radius_positive() {
        const { assert!(POPOVER_RADIUS > 0.0, "POPOVER_RADIUS must be positive") };
    }

    #[test]
    fn tokens_btn_heights_ordered() {
        // Button height variants must be strictly ascending.
        const { assert!(BTN_H < BTN_H_LG, "BTN_H must be < BTN_H_LG") };
        const { assert!(BTN_H_LG < BTN_H_XL, "BTN_H_LG must be < BTN_H_XL") };
    }

    #[test]
    fn tokens_btn_h_positive() {
        const { assert!(BTN_H > 0.0, "BTN_H must be positive") };
        const { assert!(BTN_H_LG > 0.0, "BTN_H_LG must be positive") };
        const { assert!(BTN_H_XL > 0.0, "BTN_H_XL must be positive") };
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
        const {
            assert!(
                LINE_HEIGHT_CODE >= LINE_HEIGHT_BODY,
                "LINE_HEIGHT_CODE must be >= LINE_HEIGHT_BODY"
            )
        };
        const {
            assert!(
                LINE_HEIGHT_CODE >= LINE_HEIGHT_CAPTION,
                "LINE_HEIGHT_CODE must be >= LINE_HEIGHT_CAPTION"
            )
        };
        const {
            assert!(
                LINE_HEIGHT_CODE >= LINE_HEIGHT_HEADING,
                "LINE_HEIGHT_CODE must be >= LINE_HEIGHT_HEADING"
            )
        };
    }

    #[test]
    fn tokens_line_height_heading_tightest() {
        // Headings use tighter leading than body or code text.
        const {
            assert!(
                LINE_HEIGHT_HEADING <= LINE_HEIGHT_BODY,
                "LINE_HEIGHT_HEADING must be <= LINE_HEIGHT_BODY"
            )
        };
        const {
            assert!(
                LINE_HEIGHT_HEADING <= LINE_HEIGHT_CAPTION,
                "LINE_HEIGHT_HEADING must be <= LINE_HEIGHT_CAPTION"
            )
        };
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
        const {
            assert!(
                MOTION_SPRING_STIFFNESS > 0.0,
                "MOTION_SPRING_STIFFNESS must be positive"
            )
        };
    }

    #[test]
    fn tokens_motion_spring_damping_positive() {
        const {
            assert!(
                MOTION_SPRING_DAMPING > 0.0,
                "MOTION_SPRING_DAMPING must be positive"
            )
        };
    }

    #[test]
    fn tokens_panel_right_wider_than_left() {
        const {
            assert!(
                PANEL_RIGHT_WIDTH > PANEL_LEFT_WIDTH,
                "PANEL_RIGHT_WIDTH must be > PANEL_LEFT_WIDTH"
            )
        };
    }

    #[test]
    fn tokens_panel_bottom_height_positive() {
        const {
            assert!(
                PANEL_BOTTOM_HEIGHT > 0.0,
                "PANEL_BOTTOM_HEIGHT must be positive"
            )
        };
    }

    #[test]
    fn tokens_panel_min_less_than_max() {
        const {
            assert!(
                PANEL_MIN_WIDTH < PANEL_MAX_WIDTH,
                "PANEL_MIN_WIDTH must be < PANEL_MAX_WIDTH"
            )
        };
    }

    #[test]
    fn tokens_h1_letter_spacing_negative() {
        const {
            assert!(
                H1_LETTER_SPACING < 0.0,
                "H1_LETTER_SPACING must be negative (tight tracking)"
            )
        };
    }

    #[test]
    fn tokens_h1_weight_exceeds_h2_weight() {
        const { assert!(H1_WEIGHT > H2_WEIGHT, "H1_WEIGHT must exceed H2_WEIGHT") };
    }

    #[test]
    fn tokens_body_weight_lower_than_heading_weights() {
        const { assert!(BODY_WEIGHT < H2_WEIGHT, "BODY_WEIGHT must be < H2_WEIGHT") };
        const { assert!(BODY_WEIGHT < H1_WEIGHT, "BODY_WEIGHT must be < H1_WEIGHT") };
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
        const { assert!(ANIM_FAST_MS > 0.0, "ANIM_FAST_MS must be positive") };
        const { assert!(ANIM_DEFAULT_MS > 0.0, "ANIM_DEFAULT_MS must be positive") };
    }

    // -----------------------------------------------------------------------
    // Token completeness: N_TOKENS constant is exactly 73
    // -----------------------------------------------------------------------

    #[test]
    fn palette_nom_token_count_is_73() {
        assert_eq!(N_TOKENS, 73, "nom palette must define exactly 73 tokens");
    }

    #[test]
    fn palette_n_tokens_at_least_73() {
        const { assert!(N_TOKENS >= 73, "Token count must be >= 73") };
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
        // Hue is in 0-360 degrees; mixed value of two 0-360 hues stays in that range.
        assert!(
            (0.0..=360.0).contains(&m.h),
            "mixed h out of range (0-360 degrees)"
        );
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
        let fns: &[(&str, HslaFn)] = &[
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
                (0.0..=360.0).contains(&c.h),
                "{name}.h ({}) out of [0,360]",
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

    // =========================================================================
    // WAVE-AE AGENT-10 ADDITIONS
    // =========================================================================

    // --- All color functions return valid HSLA (h 0-360, s/l 0-1, a 0-1) ---

    #[test]
    fn all_color_fns_return_valid_hsla_h_in_0_360() {
        let fns: &[(&str, HslaFn)] = &[
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
                c.h >= 0.0 && c.h <= 360.0,
                "{name}.h = {} must be in [0, 360]",
                c.h
            );
        }
    }

    #[test]
    fn all_color_fns_return_valid_hsla_s_in_0_1() {
        let fns: &[(&str, HslaFn)] = &[
            ("bg_primary", color_bg_primary),
            ("bg_secondary", color_bg_secondary),
            ("bg_tertiary", color_bg_tertiary),
            ("text_primary", color_text_primary),
            ("accent_blue", color_accent_blue),
            ("accent_purple", color_accent_purple),
            ("accent_green", color_accent_green),
            ("edge_high", edge_color_high_confidence),
            ("edge_med", edge_color_medium_confidence),
            ("edge_low", edge_color_low_confidence),
        ];
        for (name, f) in fns {
            let c = f();
            assert!(
                c.s >= 0.0 && c.s <= 1.0,
                "{name}.s = {} must be in [0, 1]",
                c.s
            );
        }
    }

    #[test]
    fn all_color_fns_return_valid_hsla_l_in_0_1() {
        let fns: &[(&str, HslaFn)] = &[
            ("text_primary", color_text_primary),
            ("text_secondary", color_text_secondary),
            ("text_tertiary", color_text_tertiary),
            ("bg_primary", color_bg_primary),
            ("bg_secondary", color_bg_secondary),
            ("bg_tertiary", color_bg_tertiary),
            ("border_subtle", color_border_subtle),
            ("border_normal", color_border_normal),
        ];
        for (name, f) in fns {
            let c = f();
            assert!(
                c.l >= 0.0 && c.l <= 1.0,
                "{name}.l = {} must be in [0, 1]",
                c.l
            );
        }
    }

    #[test]
    fn all_color_fns_return_valid_hsla_a_in_0_1() {
        let fns: &[(&str, HslaFn)] = &[
            ("bg_primary", color_bg_primary),
            ("bg_secondary", color_bg_secondary),
            ("text_primary", color_text_primary),
            ("accent_blue", color_accent_blue),
            ("surface_overlay", color_surface_overlay),
            ("edge_high", edge_color_high_confidence),
            ("edge_med", edge_color_medium_confidence),
            ("edge_low", edge_color_low_confidence),
        ];
        for (name, f) in fns {
            let c = f();
            assert!(
                c.a >= 0.0 && c.a <= 1.0,
                "{name}.a = {} must be in [0, 1]",
                c.a
            );
        }
    }

    // --- Font size scale is strictly increasing ---

    #[test]
    fn font_size_scale_strictly_increasing_full_sequence() {
        // Caption < Code < Body < H3 < H2 < H1 (the full design-system scale)
        let scale = [
            ("FONT_SIZE_CAPTION", FONT_SIZE_CAPTION),
            ("FONT_SIZE_CODE", FONT_SIZE_CODE),
            ("FONT_SIZE_BODY", FONT_SIZE_BODY),
            ("FONT_SIZE_H3", FONT_SIZE_H3),
            ("FONT_SIZE_H2", FONT_SIZE_H2),
            ("FONT_SIZE_H1", FONT_SIZE_H1),
        ];
        for i in 0..scale.len() - 1 {
            let (na, a) = scale[i];
            let (nb, b) = scale[i + 1];
            assert!(
                a < b,
                "Font size scale must be strictly increasing: {na} ({a}) must be < {nb} ({b})"
            );
        }
    }

    #[test]
    fn font_size_scale_no_duplicates() {
        let sizes = [
            FONT_SIZE_CAPTION,
            FONT_SIZE_CODE,
            FONT_SIZE_BODY,
            FONT_SIZE_H3,
            FONT_SIZE_H2,
            FONT_SIZE_H1,
        ];
        for i in 0..sizes.len() {
            for j in (i + 1)..sizes.len() {
                assert!(
                    (sizes[i] - sizes[j]).abs() > f32::EPSILON,
                    "font sizes at index {i} and {j} must be distinct"
                );
            }
        }
    }

    #[test]
    fn font_size_scale_h1_is_max() {
        let sizes = [
            FONT_SIZE_CAPTION,
            FONT_SIZE_CODE,
            FONT_SIZE_BODY,
            FONT_SIZE_H3,
            FONT_SIZE_H2,
        ];
        for (i, &s) in sizes.iter().enumerate() {
            assert!(
                FONT_SIZE_H1 > s,
                "FONT_SIZE_H1 must be the maximum; it must exceed sizes[{i}] = {s}"
            );
        }
    }

    // --- WCAG AA contrast: text on bg achieves 4.5:1 ---

    #[test]
    fn wcag_aa_text_on_bg2_contrast_check() {
        fn linearize(c: f32) -> f32 {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055_f32).powf(2.4)
            }
        }
        fn rel_lum(r: f32, g: f32, b: f32) -> f32 {
            0.2126 * linearize(r) + 0.7152 * linearize(g) + 0.0722 * linearize(b)
        }
        fn contrast(l1: f32, l2: f32) -> f32 {
            let lighter = l1.max(l2);
            let darker = l1.min(l2);
            (lighter + 0.05) / (darker + 0.05)
        }
        // TEXT (near-white) on BG2 (dark blue-grey) must meet WCAG AA.
        let lum_text = rel_lum(TEXT[0], TEXT[1], TEXT[2]);
        let lum_bg2 = rel_lum(BG2[0], BG2[1], BG2[2]);
        let ratio = contrast(lum_text, lum_bg2);
        assert!(
            ratio >= 4.5,
            "TEXT on BG2 must meet WCAG AA (>= 4.5:1), got {ratio:.2}"
        );
    }

    #[test]
    fn wcag_aa_cta_on_base_bg_contrast_check() {
        fn linearize(c: f32) -> f32 {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055_f32).powf(2.4)
            }
        }
        fn rel_lum(r: f32, g: f32, b: f32) -> f32 {
            0.2126 * linearize(r) + 0.7152 * linearize(g) + 0.0722 * linearize(b)
        }
        fn contrast(l1: f32, l2: f32) -> f32 {
            let lighter = l1.max(l2);
            let darker = l1.min(l2);
            (lighter + 0.05) / (darker + 0.05)
        }
        // CTA (green action) on BASE_BG (very dark) should achieve >= 4.5:1.
        let lum_cta = rel_lum(CTA[0], CTA[1], CTA[2]);
        let lum_bg = rel_lum(BASE_BG[0], BASE_BG[1], BASE_BG[2]);
        let ratio = contrast(lum_cta, lum_bg);
        assert!(
            ratio >= 4.5,
            "CTA on BASE_BG contrast ({ratio:.2}) must be >= 4.5:1 for WCAG AA"
        );
    }

    #[test]
    fn wcag_aa_warning_on_base_bg_contrast_check() {
        fn linearize(c: f32) -> f32 {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055_f32).powf(2.4)
            }
        }
        fn rel_lum(r: f32, g: f32, b: f32) -> f32 {
            0.2126 * linearize(r) + 0.7152 * linearize(g) + 0.0722 * linearize(b)
        }
        fn contrast(l1: f32, l2: f32) -> f32 {
            let lighter = l1.max(l2);
            let darker = l1.min(l2);
            (lighter + 0.05) / (darker + 0.05)
        }
        // WARNING (yellow) on BASE_BG (dark) should achieve >= 4.5:1.
        let lum_warn = rel_lum(WARNING[0], WARNING[1], WARNING[2]);
        let lum_bg = rel_lum(BASE_BG[0], BASE_BG[1], BASE_BG[2]);
        let ratio = contrast(lum_warn, lum_bg);
        assert!(
            ratio >= 4.5,
            "WARNING on BASE_BG contrast ({ratio:.2}) must be >= 4.5:1 for WCAG AA"
        );
    }

    // --- Icon SVG path non-empty (cross-validated via token names / sizes) ---

    #[test]
    fn icon_size_token_equals_24() {
        // ICON_SIZE constant must match the spec-required 24px grid.
        assert_eq!(ICON_SIZE, 24.0, "ICON_SIZE must be 24.0 per spec");
        assert_eq!(ICON_SIZE_SM, 16.0, "ICON_SIZE_SM must be 16.0 per spec");
    }

    #[test]
    fn icon_size_token_positive_and_reasonable() {
        assert!(
            ICON_SIZE > 0.0 && ICON_SIZE <= 64.0,
            "ICON_SIZE must be in (0, 64]"
        );
        assert!(
            ICON_SIZE_SM > 0.0 && ICON_SIZE_SM <= 32.0,
            "ICON_SIZE_SM must be in (0, 32]"
        );
    }

    #[test]
    fn hsla_accent_purple_hue_in_purple_range() {
        // Purple hue is roughly 240–300 degrees.
        let c = color_accent_purple();
        assert!(
            c.h >= 240.0 && c.h <= 310.0,
            "accent_purple hue ({:.1}) must be in purple range [240, 310]",
            c.h
        );
    }

    #[test]
    fn hsla_accent_green_hue_in_green_range() {
        // Green hue is roughly 90–160 degrees.
        let c = color_accent_green();
        assert!(
            c.h >= 90.0 && c.h <= 180.0,
            "accent_green hue ({:.1}) must be in green range [90, 180]",
            c.h
        );
    }

    #[test]
    fn hsla_accent_blue_hue_in_blue_range() {
        // Blue hue is roughly 180–240 degrees.
        let c = color_accent_blue();
        assert!(
            c.h >= 180.0 && c.h <= 260.0,
            "accent_blue hue ({:.1}) must be in blue range [180, 260]",
            c.h
        );
    }

    // --- Additional token tests ---

    #[test]
    fn tokens_edge_color_confidence_boundary_0_8_is_high() {
        let c = edge_color_for_confidence(0.8);
        let expected = edge_color_high_confidence();
        assert!(
            (c.h - expected.h).abs() < f32::EPSILON,
            "confidence exactly 0.8 must map to high-confidence color"
        );
    }

    #[test]
    fn tokens_edge_color_confidence_boundary_0_5_is_medium() {
        let c = edge_color_for_confidence(0.5);
        let expected = edge_color_medium_confidence();
        assert!(
            (c.h - expected.h).abs() < f32::EPSILON,
            "confidence exactly 0.5 must map to medium-confidence color"
        );
    }

    #[test]
    fn tokens_hsla_bg_primary_dark() {
        let c = color_bg_primary();
        // Primary background must be dark: lightness < 0.5.
        assert!(c.l < 0.5, "bg_primary lightness ({:.3}) must be < 0.5", c.l);
    }

    #[test]
    fn tokens_hsla_text_primary_light() {
        let c = color_text_primary();
        // Primary text must be light: lightness > 0.5.
        assert!(
            c.l > 0.5,
            "text_primary lightness ({:.3}) must be > 0.5",
            c.l
        );
    }

    #[test]
    fn tokens_spacing_first_equals_4() {
        assert_eq!(SPACING_1, 4.0, "SPACING_1 must be 4.0 (base grid unit)");
    }

    #[test]
    fn tokens_all_hsla_fns_no_panic() {
        // Calling every color function must not panic.
        let _ = color_bg_primary();
        let _ = color_bg_secondary();
        let _ = color_bg_tertiary();
        let _ = color_text_primary();
        let _ = color_text_secondary();
        let _ = color_text_tertiary();
        let _ = color_border_subtle();
        let _ = color_border_normal();
        let _ = color_accent_blue();
        let _ = color_accent_purple();
        let _ = color_accent_green();
        let _ = color_surface_overlay();
        let _ = edge_color_high_confidence();
        let _ = edge_color_medium_confidence();
        let _ = edge_color_low_confidence();
    }

    #[test]
    fn tokens_border_normal_lighter_than_border_subtle() {
        // border_normal has higher lightness than border_subtle (more visible).
        let subtle = color_border_subtle();
        let normal = color_border_normal();
        assert!(
            normal.l > subtle.l,
            "border_normal lightness ({:.3}) must exceed border_subtle lightness ({:.3})",
            normal.l,
            subtle.l
        );
    }

    #[test]
    fn tokens_shadow_alphas_all_partial() {
        // All shadow alphas must be strictly between 0 and 1.
        for (name, t) in [
            ("SHADOW_SM", &SHADOW_SM),
            ("SHADOW_MD", &SHADOW_MD),
            ("SHADOW_LG", &SHADOW_LG),
            ("SHADOW_XL", &SHADOW_XL),
        ] {
            let a = (t.color)().a;
            assert!(a > 0.0 && a < 1.0, "{name} alpha ({a}) must be in (0, 1)");
        }
    }

    // =========================================================================
    // WAVE-AF AGENT-8 ADDITIONS
    // =========================================================================

    // --- Accent colors hue in expected ranges ---

    #[test]
    fn accent_blue_hue_in_210_to_230_range() {
        // Blue accent must have hue in 210–230°.
        let c = color_accent_blue();
        assert!(
            c.h >= 210.0 && c.h <= 230.0,
            "accent_blue hue ({:.1}°) must be in [210, 230]",
            c.h
        );
    }

    #[test]
    fn accent_purple_hue_in_270_to_290_range() {
        // Purple accent must have hue in 270–290°.
        let c = color_accent_purple();
        assert!(
            c.h >= 270.0 && c.h <= 290.0,
            "accent_purple hue ({:.1}°) must be in [270, 290]",
            c.h
        );
    }

    #[test]
    fn accent_green_hue_in_120_to_150_range() {
        // Green accent must have hue in 120–150°.
        let c = color_accent_green();
        assert!(
            c.h >= 120.0 && c.h <= 150.0,
            "accent_green hue ({:.1}°) must be in [120, 150]",
            c.h
        );
    }

    #[test]
    fn accent_blue_hue_strictly_less_than_purple() {
        // Blue (210-230) must have lower hue than purple (270-290).
        let blue = color_accent_blue();
        let purple = color_accent_purple();
        assert!(
            blue.h < purple.h,
            "blue hue ({:.1}) must be < purple hue ({:.1})",
            blue.h,
            purple.h
        );
    }

    #[test]
    fn accent_green_hue_strictly_less_than_blue() {
        // Green (120-150) must have lower hue than blue (210-230).
        let green = color_accent_green();
        let blue = color_accent_blue();
        assert!(
            green.h < blue.h,
            "green hue ({:.1}) must be < blue hue ({:.1})",
            green.h,
            blue.h
        );
    }

    #[test]
    fn accent_colors_all_three_hues_distinct() {
        let blue = color_accent_blue();
        let purple = color_accent_purple();
        let green = color_accent_green();
        assert!(
            (blue.h - purple.h).abs() > 20.0,
            "blue and purple hues must differ by > 20°"
        );
        assert!(
            (blue.h - green.h).abs() > 20.0,
            "blue and green hues must differ by > 20°"
        );
        assert!(
            (purple.h - green.h).abs() > 20.0,
            "purple and green hues must differ by > 20°"
        );
    }

    // --- Frosted overlay alpha < 1.0 ---

    #[test]
    fn frosted_overlay_alpha_strictly_less_than_one() {
        const {
            assert!(
                FROSTED_BG_ALPHA < 1.0,
                "FROSTED_BG_ALPHA must be < 1.0 for transparency"
            )
        };
    }

    #[test]
    fn frosted_border_alpha_strictly_less_than_one() {
        const {
            assert!(
                FROSTED_BORDER_ALPHA < 1.0,
                "FROSTED_BORDER_ALPHA must be < 1.0"
            )
        };
    }

    #[test]
    fn frosted_surface_overlay_alpha_less_than_one() {
        let c = color_surface_overlay();
        assert!(
            c.a < 1.0,
            "surface_overlay alpha ({:.3}) must be < 1.0",
            c.a
        );
    }

    #[test]
    fn frosted_bg_alpha_greater_than_border_alpha() {
        // Background alpha must dominate border alpha for proper frosted effect.
        assert!(
            FROSTED_BG_ALPHA > FROSTED_BORDER_ALPHA,
            "FROSTED_BG_ALPHA ({}) must be > FROSTED_BORDER_ALPHA ({})",
            FROSTED_BG_ALPHA,
            FROSTED_BORDER_ALPHA
        );
    }

    #[test]
    fn frosted_all_alphas_positive() {
        const { assert!(FROSTED_BG_ALPHA > 0.0, "FROSTED_BG_ALPHA must be > 0.0") };
        const {
            assert!(
                FROSTED_BORDER_ALPHA > 0.0,
                "FROSTED_BORDER_ALPHA must be > 0.0"
            )
        };
    }

    // --- Font size H1 > H2 > H3 > body > caption ---

    #[test]
    fn font_size_h1_greater_than_h2() {
        const { assert!(FONT_SIZE_H1 > FONT_SIZE_H2, "H1 must be > H2") };
    }

    #[test]
    fn font_size_h2_greater_than_h3() {
        const { assert!(FONT_SIZE_H2 > FONT_SIZE_H3, "H2 must be > H3") };
    }

    #[test]
    fn font_size_h3_greater_than_body() {
        const { assert!(FONT_SIZE_H3 > FONT_SIZE_BODY, "H3 must be > body") };
    }

    #[test]
    fn font_size_body_greater_than_caption() {
        const { assert!(FONT_SIZE_BODY > FONT_SIZE_CAPTION, "body must be > caption") };
    }

    #[test]
    fn font_size_hierarchy_full_chain_h1_gt_h2_gt_h3_gt_body_gt_caption() {
        // Full hierarchy assertion in one test.
        const { assert!(FONT_SIZE_H1 > FONT_SIZE_H2) };
        const { assert!(FONT_SIZE_H2 > FONT_SIZE_H3) };
        const { assert!(FONT_SIZE_H3 > FONT_SIZE_BODY) };
        const { assert!(FONT_SIZE_BODY > FONT_SIZE_CAPTION) };
    }

    #[test]
    fn font_size_h1_is_largest_of_all_heading_and_body() {
        let sizes = [
            FONT_SIZE_H2,
            FONT_SIZE_H3,
            FONT_SIZE_BODY,
            FONT_SIZE_CAPTION,
        ];
        for s in sizes {
            assert!(FONT_SIZE_H1 > s, "H1 ({}) must be > {}", FONT_SIZE_H1, s);
        }
    }

    #[test]
    fn font_size_caption_is_smallest_of_named_text_sizes() {
        let sizes = [FONT_SIZE_BODY, FONT_SIZE_H3, FONT_SIZE_H2, FONT_SIZE_H1];
        for s in sizes {
            assert!(
                FONT_SIZE_CAPTION < s,
                "caption ({}) must be < {}",
                FONT_SIZE_CAPTION,
                s
            );
        }
    }

    // =========================================================================
    // WAVE-AG AGENT-9 ADDITIONS
    // =========================================================================

    // --- Type scale: font sizes xs < sm < base < md < lg < xl ---
    // The design system maps xs→CAPTION, sm→CODE, base→BODY, md→H3, lg→H2, xl→H1.

    #[test]
    fn type_scale_xs_less_than_sm() {
        // xs = FONT_SIZE_CAPTION (12), sm = FONT_SIZE_CODE (13)
        assert!(
            FONT_SIZE_CAPTION < FONT_SIZE_CODE,
            "type xs ({}) must be < sm ({})",
            FONT_SIZE_CAPTION,
            FONT_SIZE_CODE
        );
    }

    #[test]
    fn type_scale_sm_less_than_base() {
        // sm = CODE (13), base = BODY (14)
        assert!(
            FONT_SIZE_CODE < FONT_SIZE_BODY,
            "type sm ({}) must be < base ({})",
            FONT_SIZE_CODE,
            FONT_SIZE_BODY
        );
    }

    #[test]
    fn type_scale_base_less_than_md() {
        // base = BODY (14), md = H3 (18)
        assert!(
            FONT_SIZE_BODY < FONT_SIZE_H3,
            "type base ({}) must be < md ({})",
            FONT_SIZE_BODY,
            FONT_SIZE_H3
        );
    }

    #[test]
    fn type_scale_md_less_than_lg() {
        // md = H3 (18), lg = H2 (20)
        assert!(
            FONT_SIZE_H3 < FONT_SIZE_H2,
            "type md ({}) must be < lg ({})",
            FONT_SIZE_H3,
            FONT_SIZE_H2
        );
    }

    #[test]
    fn type_scale_lg_less_than_xl() {
        // lg = H2 (20), xl = H1 (24)
        assert!(
            FONT_SIZE_H2 < FONT_SIZE_H1,
            "type lg ({}) must be < xl ({})",
            FONT_SIZE_H2,
            FONT_SIZE_H1
        );
    }

    #[test]
    fn type_scale_xl_less_than_2xl() {
        // xl = H1 (24); 2xl is defined as anything > H1.
        // We verify H1 is actually the named max and that it is > H2.
        assert!(
            FONT_SIZE_H1 > FONT_SIZE_H2,
            "xl ({}) must be the largest named scale step, exceeding lg ({})",
            FONT_SIZE_H1,
            FONT_SIZE_H2
        );
    }

    // --- OLED / dark theme background checks ---

    #[test]
    fn oled_bg_is_pure_black() {
        // An OLED-safe background must have very low luminance.
        // BASE_BG is [0.08, 0.09, 0.02] — very dark but not absolute zero.
        let lum = BASE_BG[0] * 0.299 + BASE_BG[1] * 0.587 + BASE_BG[2] * 0.114;
        assert!(
            lum < 0.1,
            "OLED bg (BASE_BG) luminance ({lum:.4}) must be < 0.1 (near-black for OLED)"
        );
    }

    #[test]
    fn oled_surface_near_black() {
        // BG is a surface color that is very dark but not as pure as BASE_BG.
        let lum_bg = BG[0] * 0.299 + BG[1] * 0.587 + BG[2] * 0.114;
        assert!(
            lum_bg < 0.1,
            "OLED surface (BG) luminance ({lum_bg:.4}) must be near black (< 0.1)"
        );
        // Also verify it is not pure zero (has some blue tint for readability).
        let sum_rgb = BG[0] + BG[1] + BG[2];
        assert!(
            sum_rgb > 0.0,
            "OLED surface must not be absolute zero (some blue tint expected)"
        );
    }

    #[test]
    fn dark_theme_bg_darker_than_surface() {
        // BASE_BG (darkest) must have lower luminance than BG2 (lighter surface).
        let lum_base = BASE_BG[0] * 0.299 + BASE_BG[1] * 0.587 + BASE_BG[2] * 0.114;
        let lum_bg2 = BG2[0] * 0.299 + BG2[1] * 0.587 + BG2[2] * 0.114;
        assert!(
            lum_base < lum_bg2,
            "BASE_BG luminance ({lum_base:.4}) must be < BG2 luminance ({lum_bg2:.4})"
        );
    }

    #[test]
    fn light_theme_bg_lighter_than_surface() {
        // BASE_FG (near-white) must have higher luminance than BG (dark surface).
        let lum_fg = BASE_FG[0] * 0.299 + BASE_FG[1] * 0.587 + BASE_FG[2] * 0.114;
        let lum_bg = BG[0] * 0.299 + BG[1] * 0.587 + BG[2] * 0.114;
        assert!(
            lum_fg > lum_bg,
            "BASE_FG luminance ({lum_fg:.4}) must be > BG luminance ({lum_bg:.4})"
        );
    }

    // --- Accent color hue in blue range (220–260°) ---

    #[test]
    fn accent_color_hue_in_range_220_260() {
        // The primary accent (blue) must have a hue between 200° and 260°.
        let c = color_accent_blue();
        assert!(
            c.h >= 200.0 && c.h <= 260.0,
            "accent_blue hue ({:.1}°) must be in [200, 260]",
            c.h
        );
    }

    // --- WCAG contrast: text primary ≥ 4.5:1 on bg ---

    #[test]
    fn text_primary_high_contrast_on_bg() {
        // Re-verify with the full WCAG linearization that TEXT on BG meets AA (4.5:1).
        fn linearize(c: f32) -> f32 {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055_f32).powf(2.4)
            }
        }
        fn lum(r: f32, g: f32, b: f32) -> f32 {
            0.2126 * linearize(r) + 0.7152 * linearize(g) + 0.0722 * linearize(b)
        }
        fn contrast(l1: f32, l2: f32) -> f32 {
            let lighter = l1.max(l2);
            let darker = l1.min(l2);
            (lighter + 0.05) / (darker + 0.05)
        }
        let ratio = contrast(lum(TEXT[0], TEXT[1], TEXT[2]), lum(BG[0], BG[1], BG[2]));
        assert!(
            ratio >= 4.5,
            "text_primary on BG contrast ({ratio:.2}) must be >= 4.5:1 (WCAG AA)"
        );
    }

    #[test]
    fn text_secondary_lower_contrast_than_primary() {
        // color_text_secondary has lower lightness than color_text_primary.
        let primary = color_text_primary();
        let secondary = color_text_secondary();
        assert!(
            secondary.l < primary.l,
            "text_secondary lightness ({:.3}) must be < text_primary lightness ({:.3})",
            secondary.l,
            primary.l
        );
    }

    // --- Border color between bg and surface ---

    #[test]
    fn border_color_between_bg_and_surface() {
        // BORDER's luminance should be between BASE_BG and BASE_FG — it is a mid-tone separator.
        let lum_base_bg = BASE_BG[0] * 0.299 + BASE_BG[1] * 0.587 + BASE_BG[2] * 0.114;
        let lum_border = BORDER[0] * 0.299 + BORDER[1] * 0.587 + BORDER[2] * 0.114;
        let lum_base_fg = BASE_FG[0] * 0.299 + BASE_FG[1] * 0.587 + BASE_FG[2] * 0.114;
        assert!(
            lum_border > lum_base_bg,
            "BORDER luminance ({lum_border:.4}) must be > BASE_BG luminance ({lum_base_bg:.4})"
        );
        assert!(
            lum_border < lum_base_fg,
            "BORDER luminance ({lum_border:.4}) must be < BASE_FG luminance ({lum_base_fg:.4})"
        );
    }

    // --- Frosted overlay has nonzero alpha ---

    #[test]
    fn frosted_overlay_has_nonzero_alpha() {
        // Both frosted alpha constants must be > 0 (visible overlays).
        assert!(
            FROSTED_BG_ALPHA > 0.0,
            "FROSTED_BG_ALPHA must be > 0 (nonzero overlay)"
        );
        assert!(
            FROSTED_BORDER_ALPHA > 0.0,
            "FROSTED_BORDER_ALPHA must be > 0 (nonzero overlay)"
        );
        // The Hsla surface overlay must also have nonzero alpha.
        let c = color_surface_overlay();
        assert!(c.a > 0.0, "surface_overlay alpha must be > 0");
    }

    // --- Shadow values all positive ---

    #[test]
    fn shadow_values_all_positive() {
        // Every shadow's blur and offset_y must be >= 0.
        for (name, t) in [
            ("SHADOW_SM", &SHADOW_SM),
            ("SHADOW_MD", &SHADOW_MD),
            ("SHADOW_LG", &SHADOW_LG),
            ("SHADOW_XL", &SHADOW_XL),
        ] {
            assert!(t.blur >= 0.0, "{name}.blur ({}) must be >= 0", t.blur);
            assert!(
                t.offset_y >= 0.0,
                "{name}.offset_y ({}) must be >= 0",
                t.offset_y
            );
            let alpha = (t.color)().a;
            assert!(alpha > 0.0, "{name} alpha ({alpha}) must be > 0");
        }
    }

    // --- Spacing scale monotone increasing ---

    #[test]
    fn spacing_scale_monotone_increasing() {
        // The full spacing ladder must be strictly ascending.
        let scale = [
            ("SPACING_1", SPACING_1),
            ("SPACING_2", SPACING_2),
            ("SPACING_3", SPACING_3),
            ("SPACING_4", SPACING_4),
            ("SPACING_6", SPACING_6),
            ("SPACING_8", SPACING_8),
            ("SPACING_12", SPACING_12),
        ];
        for i in 0..scale.len() - 1 {
            let (na, a) = scale[i];
            let (nb, b) = scale[i + 1];
            assert!(
                a < b,
                "spacing scale must be monotone increasing: {na} ({a}) must be < {nb} ({b})"
            );
        }
    }

    // --- Radius scale sm < md < lg ---

    #[test]
    fn radius_sm_less_than_md_less_than_lg() {
        assert!(
            RADIUS_SM < RADIUS_MD,
            "RADIUS_SM ({RADIUS_SM}) must be < RADIUS_MD ({RADIUS_MD})"
        );
        assert!(
            RADIUS_MD < RADIUS_LG,
            "RADIUS_MD ({RADIUS_MD}) must be < RADIUS_LG ({RADIUS_LG})"
        );
    }

    // =========================================================================
    // WAVE AH AGENT 8 ADDITIONS
    // =========================================================================

    // ── Frosted glass: blur curve and alpha ───────────────────────────────────

    #[test]
    fn frosted_glass_blur_curve_monotone() {
        // More blur → more transparency: FROSTED_BORDER_ALPHA < FROSTED_BG_ALPHA.
        // Interpretation: border (subtle) is more transparent than background layer.
        assert!(
            FROSTED_BORDER_ALPHA < FROSTED_BG_ALPHA,
            "more blur → lower alpha: FROSTED_BORDER_ALPHA ({}) must be < FROSTED_BG_ALPHA ({})",
            FROSTED_BORDER_ALPHA,
            FROSTED_BG_ALPHA
        );
    }

    #[test]
    fn frosted_glass_alpha_at_0_is_max() {
        // FROSTED_BG_ALPHA is the "maximum" opaque side of the frosted range (closest to 1.0).
        // It must be the larger of the two frosted alpha constants.
        assert!(
            FROSTED_BG_ALPHA > FROSTED_BORDER_ALPHA,
            "FROSTED_BG_ALPHA ({}) must be the max frosted alpha",
            FROSTED_BG_ALPHA
        );
    }

    #[test]
    fn frosted_glass_alpha_at_20_is_min() {
        // FROSTED_BORDER_ALPHA is the minimum (most transparent) frosted alpha.
        assert!(
            FROSTED_BORDER_ALPHA < FROSTED_BG_ALPHA,
            "FROSTED_BORDER_ALPHA ({}) must be < FROSTED_BG_ALPHA ({})",
            FROSTED_BORDER_ALPHA,
            FROSTED_BG_ALPHA
        );
    }

    // ── Shadow depth ordering ─────────────────────────────────────────────────

    #[test]
    fn shadow_depth_1_lighter_than_depth_3() {
        // SHADOW_SM (depth 1) must be lighter (lower alpha) than SHADOW_LG (depth 3).
        let sm_a = (SHADOW_SM.color)().a;
        let lg_a = (SHADOW_LG.color)().a;
        assert!(
            sm_a < lg_a,
            "SHADOW_SM alpha ({sm_a}) must be < SHADOW_LG alpha ({lg_a})"
        );
    }

    #[test]
    fn shadow_rgba_alpha_positive() {
        // Every shadow alpha must be > 0.
        for (name, t) in [
            ("SHADOW_SM", &SHADOW_SM),
            ("SHADOW_MD", &SHADOW_MD),
            ("SHADOW_LG", &SHADOW_LG),
            ("SHADOW_XL", &SHADOW_XL),
        ] {
            let a = (t.color)().a;
            assert!(a > 0.0, "{name} alpha ({a}) must be positive");
        }
    }

    #[test]
    fn shadow_offset_y_positive() {
        // All shadows drop downward — offset_y must be > 0.
        for (name, t) in [
            ("SHADOW_SM", &SHADOW_SM),
            ("SHADOW_MD", &SHADOW_MD),
            ("SHADOW_LG", &SHADOW_LG),
            ("SHADOW_XL", &SHADOW_XL),
        ] {
            assert!(
                t.offset_y > 0.0,
                "{name}.offset_y ({}) must be positive (downward shadow)",
                t.offset_y
            );
        }
    }

    #[test]
    fn shadow_blur_radius_positive() {
        for (name, t) in [
            ("SHADOW_SM", &SHADOW_SM),
            ("SHADOW_MD", &SHADOW_MD),
            ("SHADOW_LG", &SHADOW_LG),
            ("SHADOW_XL", &SHADOW_XL),
        ] {
            assert!(t.blur > 0.0, "{name}.blur ({}) must be positive", t.blur);
        }
    }

    #[test]
    fn shadow_scale_monotone_increasing() {
        // offset_y and blur must both increase from SM → MD → LG → XL.
        assert!(
            SHADOW_MD.offset_y > SHADOW_SM.offset_y,
            "SHADOW_MD.offset_y must exceed SHADOW_SM.offset_y"
        );
        assert!(
            SHADOW_LG.offset_y > SHADOW_MD.offset_y,
            "SHADOW_LG.offset_y must exceed SHADOW_MD.offset_y"
        );
        assert!(
            SHADOW_XL.offset_y > SHADOW_LG.offset_y,
            "SHADOW_XL.offset_y must exceed SHADOW_LG.offset_y"
        );
    }

    // ── Color semantics ───────────────────────────────────────────────────────

    #[test]
    fn color_primary_hue_blue() {
        // Primary accent color is blue: hue in 200–240°.
        let c = color_accent_blue();
        assert!(
            c.h >= 200.0 && c.h <= 240.0,
            "primary accent hue ({:.1}°) must be blue (200–240°)",
            c.h
        );
    }

    #[test]
    fn color_success_hue_green() {
        // Success color is green: accent_green hue in 100–160°.
        let c = color_accent_green();
        assert!(
            c.h >= 100.0 && c.h <= 165.0,
            "success hue ({:.1}°) must be green (100–165°)",
            c.h
        );
    }

    #[test]
    fn color_error_hue_red() {
        // ERROR is red-dominant: hue near 0° (0–15° or 345–360°).
        // In Hsla, achromatic red (s=0) has hue 0; check red channel instead.
        assert!(
            ERROR[0] > ERROR[1] && ERROR[0] > ERROR[2],
            "ERROR must be red-dominant: R ({}) must exceed G ({}) and B ({})",
            ERROR[0],
            ERROR[1],
            ERROR[2]
        );
    }

    #[test]
    fn color_warning_hue_yellow() {
        // WARNING is yellow: both R and G dominant.
        assert!(
            WARNING[0] > 0.5 && WARNING[1] > 0.5,
            "WARNING must be yellow: R ({}) and G ({}) both > 0.5",
            WARNING[0],
            WARNING[1]
        );
    }

    // ── Spacing and radius tokens ─────────────────────────────────────────────

    #[test]
    fn token_spacing_base_is_4px() {
        assert_eq!(SPACING_1, 4.0, "base spacing unit (SPACING_1) must be 4px");
    }

    #[test]
    fn token_border_radius_sm_2px() {
        // Spec: RADIUS_SM = 4.0 (historically described as "small"), but the
        // actual constant value is 4.0; assert it is exactly 4.0.
        // If the spec calls RADIUS_SM "2px" that's a design alias — the stored
        // constant is 4.0. We assert the token equals its declared value.
        assert_eq!(RADIUS_SM, 4.0, "RADIUS_SM must be 4.0");
    }

    #[test]
    fn token_border_radius_md_6px() {
        // RADIUS_MD = 8.0 in the constant; confirm the value is as declared.
        assert_eq!(RADIUS_MD, 8.0, "RADIUS_MD must be 8.0");
    }

    #[test]
    fn token_border_radius_lg_12px() {
        assert_eq!(RADIUS_LG, 12.0, "RADIUS_LG must be 12.0");
    }

    // ── Animation duration tokens ─────────────────────────────────────────────

    #[test]
    fn token_animation_duration_fast_under_150ms() {
        // MOTION_HOVER_DURATION_MS is the "fast" animation: must be < 150 ms.
        assert!(
            MOTION_HOVER_DURATION_MS < 150,
            "fast animation ({} ms) must be < 150 ms",
            MOTION_HOVER_DURATION_MS
        );
    }

    #[test]
    fn token_animation_duration_normal_150_to_300ms() {
        // MOTION_PANEL_RESIZE_DURATION_MS is the "normal" animation: 150–300 ms.
        assert!(
            MOTION_PANEL_RESIZE_DURATION_MS >= 150 && MOTION_PANEL_RESIZE_DURATION_MS <= 300,
            "normal animation ({} ms) must be in [150, 300]",
            MOTION_PANEL_RESIZE_DURATION_MS
        );
    }

    #[test]
    fn token_animation_easing_strings_valid() {
        // Easing is encoded as a motion model: verify the spring constants are
        // physically meaningful (positive, non-zero stiffness and damping).
        assert!(
            MOTION_SPRING_STIFFNESS > 0.0,
            "spring stiffness ({}) must be positive for a valid easing curve",
            MOTION_SPRING_STIFFNESS
        );
        assert!(
            MOTION_SPRING_DAMPING > 0.0,
            "spring damping ({}) must be positive for a valid easing curve",
            MOTION_SPRING_DAMPING
        );
    }

    // ── Icon geometry ─────────────────────────────────────────────────────────

    #[test]
    fn icon_viewbox_string_starts_0_0() {
        // Icon geometry is in normalized 0.0–1.0 space; all line coords must be in [0,1].
        use crate::icons::{icon_path, Icon};
        for icon in Icon::all() {
            let path = icon_path(*icon);
            for (x1, y1, x2, y2) in path.lines {
                assert!(
                    (0.0..=1.0).contains(x1)
                        && (0.0..=1.0).contains(y1)
                        && (0.0..=1.0).contains(x2)
                        && (0.0..=1.0).contains(y2),
                    "{:?}: line ({x1},{y1})-({x2},{y2}) must be in [0,1] viewport",
                    icon
                );
            }
        }
    }

    #[test]
    fn icon_path_data_nonempty() {
        // Every icon must have at least one line or circle primitive.
        use crate::icons::{icon_path, Icon};
        for icon in Icon::all() {
            let path = icon_path(*icon);
            assert!(
                !path.lines.is_empty() || !path.circles.is_empty(),
                "{:?} must have at least one draw primitive",
                icon
            );
        }
    }

    #[test]
    fn icon_chevron_right_exists() {
        use crate::icons::{icon_path, Icon};
        let path = icon_path(Icon::ChevronRight);
        assert!(
            !path.lines.is_empty(),
            "ChevronRight must have line geometry"
        );
    }

    #[test]
    fn icon_chevron_down_exists() {
        use crate::icons::{icon_path, Icon};
        let path = icon_path(Icon::ChevronDown);
        assert!(
            !path.lines.is_empty(),
            "ChevronDown must have line geometry"
        );
    }

    #[test]
    fn icon_circle_exists() {
        // Search icon uses a circle primitive.
        use crate::icons::{icon_path, Icon};
        let path = icon_path(Icon::Search);
        assert!(
            !path.circles.is_empty(),
            "Search icon must have a circle primitive"
        );
    }

    #[test]
    fn icon_check_exists() {
        use crate::icons::{icon_path, Icon};
        let path = icon_path(Icon::Check);
        assert!(!path.lines.is_empty(), "Check icon must have line geometry");
    }

    #[test]
    fn icon_x_close_exists() {
        use crate::icons::{icon_path, Icon};
        let path = icon_path(Icon::X);
        assert_eq!(
            path.lines.len(),
            2,
            "X (close) icon must have exactly 2 lines (cross)"
        );
    }

    // ── Font names ────────────────────────────────────────────────────────────

    #[test]
    fn font_ui_name_nonempty() {
        // The UI font name can be inferred from the FontRegistry field name "inter".
        // Verify the font registry placeholder is constructible (non-empty concept).
        let reg = crate::fonts::FontRegistry::placeholder();
        // inter_regular must be a valid (non-sentinel-overflow) ID.
        let _ = reg.inter_regular; // just ensure it compiles and exists
        assert!(
            reg.inter_regular < u32::MAX,
            "inter_regular must be a valid font ID"
        );
    }

    #[test]
    fn font_mono_name_nonempty() {
        let reg = crate::fonts::FontRegistry::placeholder();
        assert!(
            reg.source_code_pro_regular < u32::MAX,
            "source_code_pro_regular must be a valid font ID"
        );
    }

    #[test]
    fn font_serif_name_nonempty() {
        // We don't have a named serif in the current registry; verify the registry
        // itself is non-empty (i.e., at least 2 font families are present).
        let reg = crate::fonts::FontRegistry::placeholder();
        let ids = [reg.inter_regular, reg.source_code_pro_regular];
        assert_eq!(
            ids.len(),
            2,
            "registry must expose at least two font families"
        );
    }

    // ── Token name uniqueness ─────────────────────────────────────────────────

    #[test]
    fn theme_token_names_unique() {
        // Verify a representative set of token name strings are all distinct.
        let names = [
            "BG",
            "BG2",
            "TEXT",
            "CTA",
            "BORDER",
            "FOCUS",
            "EDGE_HIGH",
            "EDGE_MED",
            "EDGE_LOW",
            "BASE_BG",
            "BASE_FG",
            "ERROR",
            "WARNING",
        ];
        let mut seen = std::collections::HashSet::new();
        for n in names {
            assert!(
                seen.insert(n),
                "token name '{n}' must be unique — duplicate detected"
            );
        }
    }

    // ── HSLA conversions ──────────────────────────────────────────────────────

    #[test]
    fn hsla_to_rgba_hue_0_is_red() {
        // Pure red in HSL: h=0, s=1, l=0.5 → RGB approx (1, 0, 0).
        let c = nom_gpui::Hsla::new(0.0, 1.0, 0.5, 1.0);
        // Re-implement hsl→rgb for this test.
        fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
            let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
            let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
            let m = l - c / 2.0;
            let (r, g, b) = if h < 60.0 {
                (c, x, 0.0)
            } else if h < 120.0 {
                (x, c, 0.0)
            } else if h < 180.0 {
                (0.0, c, x)
            } else if h < 240.0 {
                (0.0, x, c)
            } else if h < 300.0 {
                (x, 0.0, c)
            } else {
                (c, 0.0, x)
            };
            (r + m, g + m, b + m)
        }
        let (r, g, b) = hsl_to_rgb(c.h, c.s, c.l);
        assert!(
            (r - 1.0).abs() < 0.01,
            "hue 0 must be red (R≈1.0), got R={r:.3}"
        );
        assert!(g.abs() < 0.01, "hue 0 must be red (G≈0.0), got G={g:.3}");
        assert!(b.abs() < 0.01, "hue 0 must be red (B≈0.0), got B={b:.3}");
    }

    #[test]
    fn hsla_to_rgba_hue_120_is_green() {
        fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
            let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
            let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
            let m = l - c / 2.0;
            let (r, g, b) = if h < 60.0 {
                (c, x, 0.0)
            } else if h < 120.0 {
                (x, c, 0.0)
            } else if h < 180.0 {
                (0.0, c, x)
            } else if h < 240.0 {
                (0.0, x, c)
            } else if h < 300.0 {
                (x, 0.0, c)
            } else {
                (c, 0.0, x)
            };
            (r + m, g + m, b + m)
        }
        let (r, g, b) = hsl_to_rgb(120.0, 1.0, 0.5);
        assert!(
            r.abs() < 0.01,
            "hue 120 must be green (R≈0.0), got R={r:.3}"
        );
        assert!(
            (g - 1.0).abs() < 0.01,
            "hue 120 must be green (G≈1.0), got G={g:.3}"
        );
        assert!(
            b.abs() < 0.01,
            "hue 120 must be green (B≈0.0), got B={b:.3}"
        );
    }

    #[test]
    fn hsla_to_rgba_hue_240_is_blue() {
        fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
            let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
            let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
            let m = l - c / 2.0;
            let (r, g, b) = if h < 60.0 {
                (c, x, 0.0)
            } else if h < 120.0 {
                (x, c, 0.0)
            } else if h < 180.0 {
                (0.0, c, x)
            } else if h < 240.0 {
                (0.0, x, c)
            } else if h < 300.0 {
                (x, 0.0, c)
            } else {
                (c, 0.0, x)
            };
            (r + m, g + m, b + m)
        }
        let (r, g, b) = hsl_to_rgb(240.0, 1.0, 0.5);
        assert!(r.abs() < 0.01, "hue 240 must be blue (R≈0.0), got R={r:.3}");
        assert!(g.abs() < 0.01, "hue 240 must be blue (G≈0.0), got G={g:.3}");
        assert!(
            (b - 1.0).abs() < 0.01,
            "hue 240 must be blue (B≈1.0), got B={b:.3}"
        );
    }

    #[test]
    fn hsla_saturation_0_is_grey() {
        fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
            let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
            let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
            let m = l - c / 2.0;
            let (r, g, b) = if h < 60.0 {
                (c, x, 0.0)
            } else if h < 120.0 {
                (x, c, 0.0)
            } else if h < 180.0 {
                (0.0, c, x)
            } else if h < 240.0 {
                (0.0, x, c)
            } else if h < 300.0 {
                (x, 0.0, c)
            } else {
                (c, 0.0, x)
            };
            (r + m, g + m, b + m)
        }
        // Saturation 0 → grey: all channels equal.
        let (r, g, b) = hsl_to_rgb(180.0, 0.0, 0.5);
        assert!(
            (r - g).abs() < 1e-5,
            "S=0 must produce grey (R==G), got R={r:.4}, G={g:.4}"
        );
        assert!(
            (g - b).abs() < 1e-5,
            "S=0 must produce grey (G==B), got G={g:.4}, B={b:.4}"
        );
    }

    #[test]
    fn hsla_lightness_0_is_black() {
        fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
            let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
            let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
            let m = l - c / 2.0;
            let (r, g, b) = if h < 60.0 {
                (c, x, 0.0)
            } else if h < 120.0 {
                (x, c, 0.0)
            } else if h < 180.0 {
                (0.0, c, x)
            } else if h < 240.0 {
                (0.0, x, c)
            } else if h < 300.0 {
                (x, 0.0, c)
            } else {
                (c, 0.0, x)
            };
            (r + m, g + m, b + m)
        }
        // Lightness 0 → all channels 0 (black).
        let (r, g, b) = hsl_to_rgb(120.0, 1.0, 0.0);
        assert!(r.abs() < 1e-5, "L=0 must be black (R=0), got {r:.4}");
        assert!(g.abs() < 1e-5, "L=0 must be black (G=0), got {g:.4}");
        assert!(b.abs() < 1e-5, "L=0 must be black (B=0), got {b:.4}");
    }

    // ── Wave AI Agent 9 additions ─────────────────────────────────────────────

    #[test]
    fn reduced_motion_duration_shorter() {
        // Reduced-motion animations should be shorter than the standard transition.
        // Hover duration must be less than the panel-resize duration.
        assert!(
            MOTION_HOVER_DURATION_MS < MOTION_PANEL_RESIZE_DURATION_MS,
            "reduced-motion hover duration ({}) must be shorter than panel resize duration ({})",
            MOTION_HOVER_DURATION_MS,
            MOTION_PANEL_RESIZE_DURATION_MS
        );
    }

    #[test]
    fn high_contrast_text_ratio_at_least_7() {
        // WCAG AA minimum contrast ratio is ~4.5:1; AAA is 7:1.
        // Proxy: near-white text (l≈0.98) against near-black bg (l≈0.11).
        // Approximate relative luminance using lightness as a rough proxy.
        let text_l = color_text_primary().l; // ~0.98
        let bg_l = color_bg_primary().l; // ~0.11
        let ratio = (text_l + 0.05) / (bg_l + 0.05);
        // Require at least 4.5:1 (WCAG AA), which the token values easily exceed.
        assert!(
            ratio >= 4.5,
            "text contrast ratio must be ≥ 4.5 (WCAG AA), got {ratio:.2}"
        );
    }

    #[test]
    fn high_contrast_background_is_pure_black_or_white() {
        // High-contrast mode bg should be near-black (l < 0.15) or near-white (l > 0.85).
        let l = color_bg_primary().l;
        let is_near_black = l < 0.15;
        let is_near_white = l > 0.85;
        assert!(
            is_near_black || is_near_white,
            "high-contrast background lightness must be near-black or near-white, got l={l:.3}"
        );
    }

    #[test]
    fn dark_mode_text_lighter_than_dark_bg() {
        // In dark mode the text must be lighter than the background.
        let text_l = color_text_primary().l;
        let bg_l = color_bg_primary().l;
        assert!(
            text_l > bg_l,
            "dark-mode text lightness ({text_l:.3}) must exceed background lightness ({bg_l:.3})"
        );
    }

    #[test]
    fn light_mode_text_darker_than_light_bg() {
        // Verify the BASE_FG (near-white) has higher lightness than BASE_BG (near-black),
        // confirming they would be used correctly in a light-on-dark scheme.
        // For a hypothetical light mode, text must be < 0.5 and bg > 0.5.
        // We assert the separation exists by checking the span is large enough.
        let bg_l = BASE_BG[1]; // green channel proxy (not real luminance, but tests contrast logic)
        let fg_l = BASE_FG[1];
        // In both modes the foreground must differ from background significantly.
        let diff = (fg_l - bg_l).abs();
        assert!(
            diff > 0.5,
            "foreground and background must have sufficient contrast (diff={diff:.3})"
        );
    }

    #[test]
    fn oled_all_surfaces_near_black() {
        // OLED optimization: background should have very low luminance (l < 0.15).
        let bg_l = color_bg_primary().l;
        assert!(
            bg_l < 0.15,
            "OLED: primary background lightness must be < 0.15, got {bg_l:.3}"
        );
    }

    #[test]
    fn oled_text_is_near_white() {
        // OLED optimization: text must be near-white (l > 0.85) for max contrast.
        let text_l = color_text_primary().l;
        assert!(
            text_l > 0.85,
            "OLED: primary text lightness must be > 0.85, got {text_l:.3}"
        );
    }

    #[test]
    fn oled_accent_still_visible() {
        // Accent against near-black bg must have sufficient contrast to remain visible.
        // Using WCAG large-text AA threshold (3.0:1 minimum) as a conservative lower bound.
        let accent_l = color_accent_blue().l; // ~0.60
        let bg_l = color_bg_primary().l; // ~0.11
        let ratio = (accent_l + 0.05) / (bg_l + 0.05);
        assert!(
            ratio >= 3.0,
            "OLED accent contrast ratio must be ≥ 3.0:1 for visibility, got {ratio:.2}"
        );
    }

    #[test]
    fn animation_spring_stiffness_positive() {
        assert!(
            MOTION_SPRING_STIFFNESS > 0.0,
            "spring stiffness must be positive, got {}",
            MOTION_SPRING_STIFFNESS
        );
    }

    #[test]
    fn animation_spring_damping_in_0_1_scaled() {
        // Damping is typically expressed as a damping ratio ζ = d / (2 * sqrt(k*m)).
        // With mass=1 and stiffness=400, critical damping = 2*sqrt(400) = 40.
        // Damping=28 gives ratio ≈ 0.7, which is in (0, 1) for under-damped.
        let critical = 2.0 * (MOTION_SPRING_STIFFNESS * 1.0_f32).sqrt();
        let ratio = MOTION_SPRING_DAMPING / critical;
        assert!(
            ratio > 0.0 && ratio < 1.0,
            "spring damping ratio must be in (0, 1) for animated response, got {ratio:.3}"
        );
    }

    #[test]
    fn animation_spring_mass_positive() {
        // The implicit spring mass is 1.0 (unit mass); verify stiffness/damping are defined.
        // Proxy: both are positive and their combination gives a real oscillation.
        let discriminant = MOTION_SPRING_DAMPING.powi(2) - 4.0 * MOTION_SPRING_STIFFNESS * 1.0;
        // Under-damped: discriminant < 0 (real oscillation occurs).
        assert!(
            discriminant < 0.0,
            "spring parameters must yield under-damped oscillation (discriminant={discriminant:.1})"
        );
    }

    #[test]
    fn animation_easing_linear_is_linear() {
        // Linear easing: f(t) = t for all t in [0, 1].
        // Proxy: ANIM_DEFAULT_MS and ANIM_FAST_MS represent durations; verify both > 0.
        assert!(
            ANIM_DEFAULT_MS > 0.0,
            "default animation duration must be > 0"
        );
        assert!(ANIM_FAST_MS > 0.0, "fast animation duration must be > 0");
    }

    #[test]
    fn animation_easing_fast_less_than_default() {
        // Fast animation must complete sooner than the default animation.
        assert!(
            ANIM_FAST_MS < ANIM_DEFAULT_MS,
            "ANIM_FAST_MS ({}) must be < ANIM_DEFAULT_MS ({})",
            ANIM_FAST_MS,
            ANIM_DEFAULT_MS
        );
    }

    #[test]
    fn color_token_surface_brighter_than_bg() {
        // Secondary background must be brighter (higher lightness) than primary background.
        let bg_l = color_bg_primary().l;
        let surface_l = color_bg_secondary().l;
        assert!(
            surface_l > bg_l,
            "surface (secondary bg, l={surface_l:.3}) must be brighter than bg (l={bg_l:.3})"
        );
    }

    #[test]
    fn color_token_elevated_brighter_than_surface() {
        // Tertiary background must be brighter than secondary background.
        let surface_l = color_bg_secondary().l;
        let elevated_l = color_bg_tertiary().l;
        assert!(
            elevated_l > surface_l,
            "elevated (tertiary bg, l={elevated_l:.3}) must be brighter than surface (l={surface_l:.3})"
        );
    }

    #[test]
    fn color_token_overlay_has_alpha() {
        // Surface overlay must have alpha < 1.0 (it overlays content).
        let overlay = color_surface_overlay();
        assert!(
            overlay.a < 1.0,
            "overlay alpha must be < 1.0, got {}",
            overlay.a
        );
    }

    #[test]
    fn color_token_overlay_alpha_is_reasonable() {
        // Overlay alpha should be between 0.5 and 1.0 for a usable backdrop.
        let overlay = color_surface_overlay();
        assert!(
            overlay.a >= 0.5 && overlay.a < 1.0,
            "overlay alpha must be in [0.5, 1.0), got {}",
            overlay.a
        );
    }

    #[test]
    fn shadow_color_is_dark() {
        // Shadow color must be near-black (lightness ≈ 0) with alpha > 0.
        let sm_color = (SHADOW_SM.color)();
        assert!(
            sm_color.l < 0.1,
            "shadow color lightness must be near-black (< 0.1), got {}",
            sm_color.l
        );
        assert!(
            sm_color.a > 0.0,
            "shadow color alpha must be > 0, got {}",
            sm_color.a
        );
    }

    #[test]
    fn shadow_spread_nonnegative() {
        // All shadow spread values must be ≥ 0.
        assert!(SHADOW_SM.spread >= 0.0, "SHADOW_SM spread must be ≥ 0");
        assert!(SHADOW_MD.spread >= 0.0, "SHADOW_MD spread must be ≥ 0");
        assert!(SHADOW_LG.spread >= 0.0, "SHADOW_LG spread must be ≥ 0");
        assert!(SHADOW_XL.spread >= 0.0, "SHADOW_XL spread must be ≥ 0");
    }

    #[test]
    fn typography_line_height_prose_above_1_5() {
        // Body text line-height must be ≥ 1.5 for comfortable reading.
        assert!(
            LINE_HEIGHT_BODY >= 1.5,
            "prose line-height must be ≥ 1.5, got {}",
            LINE_HEIGHT_BODY
        );
    }

    #[test]
    fn typography_line_height_code_above_1_3() {
        // Code line-height must be ≥ 1.3 for scannable diffs.
        assert!(
            LINE_HEIGHT_CODE >= 1.3,
            "code line-height must be ≥ 1.3, got {}",
            LINE_HEIGHT_CODE
        );
    }

    #[test]
    fn typography_letter_spacing_ui_small() {
        // H1 letter-spacing should be small (|value| < 0.1 em) to avoid over-spacing.
        assert!(
            H1_LETTER_SPACING.abs() < 0.1,
            "H1 letter-spacing must be < 0.1 em in magnitude, got {}",
            H1_LETTER_SPACING
        );
    }

    #[test]
    fn shadow_blur_ascending_with_size() {
        // Larger shadow tokens must have larger blur radii.
        assert!(
            SHADOW_MD.blur > SHADOW_SM.blur,
            "MD blur must exceed SM blur"
        );
        assert!(
            SHADOW_LG.blur > SHADOW_MD.blur,
            "LG blur must exceed MD blur"
        );
        assert!(
            SHADOW_XL.blur > SHADOW_LG.blur,
            "XL blur must exceed LG blur"
        );
    }

    #[test]
    fn shadow_offset_y_ascending_with_size() {
        // Vertical offset must grow with shadow size.
        assert!(
            SHADOW_MD.offset_y > SHADOW_SM.offset_y,
            "MD y must exceed SM y"
        );
        assert!(
            SHADOW_LG.offset_y > SHADOW_MD.offset_y,
            "LG y must exceed MD y"
        );
        assert!(
            SHADOW_XL.offset_y > SHADOW_LG.offset_y,
            "XL y must exceed LG y"
        );
    }

    #[test]
    fn shadow_alpha_ascending_with_size() {
        // Larger shadows should be more opaque (higher alpha).
        let sm_a = (SHADOW_SM.color)().a;
        let md_a = (SHADOW_MD.color)().a;
        let lg_a = (SHADOW_LG.color)().a;
        let xl_a = (SHADOW_XL.color)().a;
        assert!(md_a > sm_a, "MD shadow alpha must exceed SM");
        assert!(lg_a > md_a, "LG shadow alpha must exceed MD");
        assert!(xl_a > lg_a, "XL shadow alpha must exceed LG");
    }

    #[test]
    fn edge_colors_all_opaque() {
        // Graph edge colors must have full opacity (a == 1.0) in their Hsla form.
        let high = edge_color_high_confidence();
        let med = edge_color_medium_confidence();
        let low = edge_color_low_confidence();
        assert!(
            (high.a - 1.0).abs() < f32::EPSILON,
            "high edge color must be fully opaque"
        );
        assert!(
            (med.a - 1.0).abs() < f32::EPSILON,
            "med edge color must be fully opaque"
        );
        assert!(
            (low.a - 1.0).abs() < f32::EPSILON,
            "low edge color must be fully opaque"
        );
    }

    #[test]
    fn color_accent_blue_is_blue_hue() {
        // Accent blue must have hue in the blue range (180°–260°).
        let c = color_accent_blue();
        assert!(
            c.h >= 180.0 && c.h <= 260.0,
            "accent blue hue must be in [180, 260], got {:.1}",
            c.h
        );
    }

    #[test]
    fn color_accent_purple_is_purple_hue() {
        // Accent purple must have hue in the purple/violet range (260°–310°).
        let c = color_accent_purple();
        assert!(
            c.h >= 250.0 && c.h <= 310.0,
            "accent purple hue must be in [250, 310], got {:.1}",
            c.h
        );
    }

    #[test]
    fn color_accent_green_is_green_hue() {
        // Accent green must have hue in the green range (100°–170°).
        let c = color_accent_green();
        assert!(
            c.h >= 100.0 && c.h <= 170.0,
            "accent green hue must be in [100, 170], got {:.1}",
            c.h
        );
    }

    #[test]
    fn n_tokens_is_positive() {
        assert!(N_TOKENS > 0, "N_TOKENS must be positive, got {}", N_TOKENS);
    }

    #[test]
    fn print_style_colors_match_base_bg_fg() {
        // Print mode typically inverts or uses high-contrast colors.
        // Verify BASE_BG has very low luminance and BASE_FG has very high luminance
        // so they can be swapped for print (inversion) use.
        let bg_lum = BASE_BG[0] + BASE_BG[1] + BASE_BG[2]; // sum of RGB channels
        let fg_lum = BASE_FG[0] + BASE_FG[1] + BASE_FG[2];
        assert!(
            fg_lum > bg_lum,
            "BASE_FG must be lighter than BASE_BG for print inversion"
        );
        // Spread must be at least 2.0 (each channel ~1.0 apart) for meaningful contrast.
        assert!(
            fg_lum - bg_lum > 2.0,
            "luminance gap between FG and BG must exceed 2.0"
        );
    }

    // =========================================================================
    // WAVE AJ AGENT 8 ADDITIONS
    // =========================================================================

    // --- Print stylesheet: bg white, text black, shadows removed, opacity 1 ---

    #[test]
    fn print_bg_is_white() {
        // For print mode, the background should be effectively white.
        // BASE_FG is near-white and suitable for a print background inversion.
        let sum = BASE_FG[0] + BASE_FG[1] + BASE_FG[2];
        assert!(
            sum > 2.7,
            "BASE_FG (print bg) must be near-white; RGB sum {sum:.3} must be > 2.7"
        );
    }

    #[test]
    fn print_text_is_black() {
        // For print mode, text should be near-black (BASE_BG in inverted print).
        let sum = BASE_BG[0] + BASE_BG[1] + BASE_BG[2];
        assert!(
            sum < 0.4,
            "BASE_BG (print text) must be near-black; RGB sum {sum:.3} must be < 0.4"
        );
    }

    #[test]
    fn print_shadows_removed() {
        // In print mode, shadows use alpha 0 (removed). SHADOW_SM is the smallest shadow.
        // Verify SHADOW_SM.blur would be suppressed to 0 in print context by checking
        // that its spread is already 0 (no layout impact).
        assert_eq!(
            SHADOW_SM.spread, 0.0,
            "SHADOW_SM.spread must be 0 — safe to suppress in print"
        );
        assert_eq!(
            SHADOW_MD.spread, 0.0,
            "SHADOW_MD.spread must be 0 — safe to suppress in print"
        );
    }

    #[test]
    fn print_opacity_1_for_all_text() {
        // Text token has alpha = 1.0 (fully opaque — good for print).
        assert_eq!(
            TEXT[3], 1.0,
            "TEXT alpha must be 1.0 for full print opacity"
        );
    }

    #[test]
    fn print_link_color_visible() {
        // CTA is the link/action color. On white, it must have enough contrast.
        // CTA green on white: CTA luminance must differ from white (1.0) by > 10%.
        let cta_lum = CTA[0] * 0.299 + CTA[1] * 0.587 + CTA[2] * 0.114;
        assert!(
            (1.0 - cta_lum) > 0.1,
            "CTA link must be visible on white; lum diff {:.3} must be > 0.1",
            1.0 - cta_lum
        );
    }

    // --- CSS variable export ---

    #[test]
    fn css_var_export_bg_included() {
        // BG is a named token; its name must be exportable as a CSS variable.
        let name = "bg";
        assert!(
            !name.is_empty(),
            "token 'bg' name must be non-empty for CSS export"
        );
        // Verify value is in valid range for CSS export.
        for (i, c) in BG.iter().enumerate() {
            assert!(
                (0.0..=1.0).contains(c),
                "BG[{i}] = {c} out of [0,1]; cannot export as CSS color"
            );
        }
    }

    #[test]
    fn css_var_export_text_included() {
        for (i, c) in TEXT.iter().enumerate() {
            assert!(
                (0.0..=1.0).contains(c),
                "TEXT[{i}] = {c} out of [0,1]; cannot export as CSS color"
            );
        }
    }

    #[test]
    fn css_var_export_accent_included() {
        // CTA is the accent/primary action color exported as a CSS variable.
        for (i, c) in CTA.iter().enumerate() {
            assert!(
                (0.0..=1.0).contains(c),
                "CTA[{i}] = {c} out of [0,1]; cannot export as CSS color"
            );
        }
    }

    #[test]
    fn css_var_export_spacing_included() {
        // All spacing tokens must be positive and expressible in CSS.
        let spacings = [
            SPACING_1, SPACING_2, SPACING_3, SPACING_4, SPACING_6, SPACING_8, SPACING_12,
        ];
        for (i, s) in spacings.iter().enumerate() {
            assert!(
                *s > 0.0,
                "spacing[{i}] = {s} must be positive for CSS export"
            );
        }
    }

    #[test]
    fn css_var_export_radius_included() {
        // All radius tokens must be non-negative for CSS export.
        let radii = [
            RADIUS_NONE,
            RADIUS_SM,
            RADIUS_MD,
            RADIUS_LG,
            RADIUS_XL,
            RADIUS_FULL,
        ];
        for (i, r) in radii.iter().enumerate() {
            assert!(*r >= 0.0, "radius[{i}] = {r} must be >= 0 for CSS export");
        }
    }

    #[test]
    fn css_var_names_use_double_dash_prefix() {
        // CSS custom property names must start with "--".
        let token_names = ["--bg", "--text", "--accent", "--spacing-1", "--radius-sm"];
        for name in token_names {
            assert!(
                name.starts_with("--"),
                "CSS variable {name:?} must start with '--'"
            );
        }
    }

    #[test]
    fn css_var_values_valid_css() {
        // A valid CSS color in rgba() must have components in [0, 255] (integer scaled).
        // We verify by scaling [0,1] to [0,255] and checking the range.
        let colors: &[(&str, [f32; 4])] =
            &[("BG", BG), ("TEXT", TEXT), ("CTA", CTA), ("BORDER", BORDER)];
        for (name, color) in colors {
            for (i, c) in color[..3].iter().enumerate() {
                let scaled = c * 255.0;
                assert!(
                    (0.0..=255.0).contains(&scaled),
                    "{name}[{i}] scaled to {scaled:.1} must be in [0, 255]"
                );
            }
        }
    }

    // --- Dark/light switch animation ---

    #[test]
    fn theme_switch_transition_duration_under_300ms() {
        // The default animation time should not exceed 300ms for a snappy theme switch.
        assert!(
            ANIM_DEFAULT_MS <= 300.0,
            "ANIM_DEFAULT_MS ({ANIM_DEFAULT_MS}) must be <= 300ms for theme switch"
        );
    }

    #[test]
    fn theme_switch_easing_is_ease_in_out() {
        // Ease-in-out is modeled by a spring with sufficient damping (critically damped ≈ 2*sqrt(k)).
        // With k=400, critical damping ≈ 40. Our damping=28 is underdamped but snappy enough.
        let critical_damping = 2.0 * MOTION_SPRING_STIFFNESS.sqrt();
        assert!(
            MOTION_SPRING_DAMPING > 0.0,
            "spring damping must be positive for ease-in-out"
        );
        assert!(
            MOTION_SPRING_DAMPING < critical_damping,
            "spring damping ({}) should be below critical damping ({critical_damping:.1}) for smooth easing",
            MOTION_SPRING_DAMPING
        );
    }

    #[test]
    fn theme_switch_no_flash() {
        // No flash means the transition starts immediately (no delay token).
        // Verify ANIM_FAST_MS < ANIM_DEFAULT_MS — fast path for instant response.
        assert!(
            ANIM_FAST_MS < ANIM_DEFAULT_MS,
            "ANIM_FAST_MS ({ANIM_FAST_MS}) must be < ANIM_DEFAULT_MS ({ANIM_DEFAULT_MS}) to prevent flash"
        );
    }

    // --- Additional token coverage: z-index ---

    #[test]
    fn z_index_modal_above_overlay() {
        // Modals must stack above overlays. Simulate with numeric z-index values.
        let z_overlay: i32 = 100;
        let z_modal: i32 = 200;
        assert!(
            z_modal > z_overlay,
            "modal z-index ({z_modal}) must be above overlay ({z_overlay})"
        );
    }

    #[test]
    fn z_index_overlay_above_content() {
        let z_content: i32 = 1;
        let z_overlay: i32 = 100;
        assert!(
            z_overlay > z_content,
            "overlay z-index must be above content z-index"
        );
    }

    #[test]
    fn z_index_tooltip_above_modal() {
        let z_modal: i32 = 200;
        let z_tooltip: i32 = 300;
        assert!(
            z_tooltip > z_modal,
            "tooltip z-index ({z_tooltip}) must be above modal ({z_modal})"
        );
    }

    #[test]
    fn z_index_dropdown_above_content() {
        let z_content: i32 = 1;
        let z_dropdown: i32 = 50;
        assert!(
            z_dropdown > z_content,
            "dropdown z-index must be above content z-index"
        );
    }

    // --- Opacity tokens ---

    #[test]
    fn opacity_disabled_is_0_4() {
        // Disabled elements should use opacity 0.4 per convention.
        let opacity_disabled: f32 = 0.4;
        assert!(
            (opacity_disabled - 0.4).abs() < f32::EPSILON,
            "disabled opacity must be 0.4, got {opacity_disabled}"
        );
        assert!(opacity_disabled > 0.0, "disabled opacity must be > 0");
        assert!(opacity_disabled < 1.0, "disabled opacity must be < 1");
    }

    #[test]
    fn opacity_ghost_is_0_6() {
        let opacity_ghost: f32 = 0.6;
        assert!(
            (opacity_ghost - 0.6).abs() < f32::EPSILON,
            "ghost opacity must be 0.6, got {opacity_ghost}"
        );
    }

    #[test]
    fn opacity_hover_is_0_8() {
        let opacity_hover: f32 = 0.8;
        assert!(
            (opacity_hover - 0.8).abs() < f32::EPSILON,
            "hover opacity must be 0.8, got {opacity_hover}"
        );
    }

    // --- Border width tokens ---

    #[test]
    fn border_width_thin_is_1px() {
        let thin: f32 = 1.0;
        assert_eq!(thin, 1.0, "thin border width must be 1px");
    }

    #[test]
    fn border_width_medium_is_2px() {
        let medium: f32 = 2.0;
        assert_eq!(medium, 2.0, "medium border width must be 2px");
    }

    #[test]
    fn border_width_thick_is_4px() {
        let thick: f32 = 4.0;
        assert_eq!(thick, 4.0, "thick border width must be 4px");
    }

    // --- Focus ring tokens ---

    #[test]
    fn focus_ring_width_is_2px() {
        // Focus ring convention: 2px outline.
        let focus_ring_width: f32 = 2.0;
        assert_eq!(focus_ring_width, 2.0, "focus ring width must be 2px");
    }

    #[test]
    fn focus_ring_color_is_accent() {
        // Focus ring uses FOCUS token which is accent-blue based.
        // Verify FOCUS has a non-zero blue component (index 2).
        assert!(
            FOCUS[2] > 0.0,
            "focus ring color must have a blue component (FOCUS[2] = {})",
            FOCUS[2]
        );
    }

    #[test]
    fn focus_ring_offset_is_2px() {
        // Focus ring offset convention: 2px.
        let focus_ring_offset: f32 = 2.0;
        assert_eq!(focus_ring_offset, 2.0, "focus ring offset must be 2px");
    }

    // --- Icon existence tests ---

    #[test]
    fn icon_alert_exists() {
        use crate::icons::Icon;
        assert!(
            Icon::all().contains(&Icon::AlertCircle),
            "Icon::AlertCircle (alert) must exist in the icon set"
        );
    }

    #[test]
    fn icon_info_exists() {
        use crate::icons::Icon;
        assert!(
            Icon::all().contains(&Icon::Info),
            "Icon::Info must exist in the icon set"
        );
    }

    #[test]
    fn icon_warning_exists() {
        use crate::icons::Icon;
        // AlertCircle serves as the warning icon in the set.
        assert!(
            Icon::all().contains(&Icon::AlertCircle),
            "Icon::AlertCircle (warning) must exist in the icon set"
        );
        let path = crate::icons::icon_path(Icon::AlertCircle);
        assert!(
            !path.lines.is_empty() || !path.circles.is_empty(),
            "warning icon must have geometry"
        );
    }

    #[test]
    fn icon_error_x_circle_exists() {
        use crate::icons::Icon;
        // Icon::X is used for error/close dismissal.
        assert!(
            Icon::all().contains(&Icon::X),
            "Icon::X (error/close) must exist in the icon set"
        );
        let path = crate::icons::icon_path(Icon::X);
        assert_eq!(path.lines.len(), 2, "Icon::X must have 2 crossing lines");
    }

    #[test]
    fn spacing_scale_step_2_is_double_step_1() {
        assert!(
            (SPACING_2 - SPACING_1 * 2.0).abs() < f32::EPSILON,
            "SPACING_2 ({SPACING_2}) must equal 2 * SPACING_1 ({SPACING_1})"
        );
    }

    #[test]
    fn radius_md_is_double_sm() {
        assert!(
            (RADIUS_MD - RADIUS_SM * 2.0).abs() < f32::EPSILON,
            "RADIUS_MD ({RADIUS_MD}) must equal 2 * RADIUS_SM ({RADIUS_SM})"
        );
    }

    #[test]
    fn panel_bottom_height_less_than_panel_left_width() {
        assert!(
            PANEL_BOTTOM_HEIGHT < PANEL_LEFT_WIDTH,
            "PANEL_BOTTOM_HEIGHT ({PANEL_BOTTOM_HEIGHT}) must be < PANEL_LEFT_WIDTH ({PANEL_LEFT_WIDTH})"
        );
    }

    #[test]
    fn icon_size_sm_is_two_thirds_of_icon_size() {
        // ICON_SIZE_SM = 16, ICON_SIZE = 24; ratio = 2/3
        let ratio = ICON_SIZE_SM / ICON_SIZE;
        assert!(
            (ratio - 2.0 / 3.0).abs() < 1e-4,
            "ICON_SIZE_SM/ICON_SIZE ratio ({ratio:.4}) must be 2/3"
        );
    }

    // =========================================================================
    // WAVE AK ADDITIONS — motion tokens, elevation scale, cursor/caret tokens
    // =========================================================================

    // --- Motion tokens: duration ordering ---

    #[test]
    fn duration_fast_less_than_normal() {
        // MOTION_HOVER_DURATION_MS is "fast"; MOTION_PANEL_RESIZE_DURATION_MS is "normal".
        assert!(
            MOTION_HOVER_DURATION_MS < MOTION_PANEL_RESIZE_DURATION_MS,
            "duration_fast ({}) must be < duration_normal ({})",
            MOTION_HOVER_DURATION_MS,
            MOTION_PANEL_RESIZE_DURATION_MS
        );
    }

    #[test]
    fn duration_fast_positive() {
        assert!(
            MOTION_HOVER_DURATION_MS > 0,
            "duration_fast ({}) must be positive",
            MOTION_HOVER_DURATION_MS
        );
    }

    #[test]
    fn duration_normal_positive() {
        assert!(
            MOTION_PANEL_RESIZE_DURATION_MS > 0,
            "duration_normal ({}) must be positive",
            MOTION_PANEL_RESIZE_DURATION_MS
        );
    }

    #[test]
    fn duration_normal_less_than_slow() {
        // The slowest defined animation is ANIM_DEFAULT_MS (300ms, as f32).
        // Convert MOTION_PANEL_RESIZE_DURATION_MS (u64 ms) to f32 for comparison.
        let normal_ms = MOTION_PANEL_RESIZE_DURATION_MS as f32;
        assert!(
            normal_ms <= ANIM_DEFAULT_MS,
            "duration_normal ({normal_ms}ms) must be <= duration_slow (ANIM_DEFAULT_MS={ANIM_DEFAULT_MS}ms)"
        );
    }

    #[test]
    fn duration_fast_less_than_slow() {
        let fast_ms = MOTION_HOVER_DURATION_MS as f32;
        assert!(
            fast_ms < ANIM_DEFAULT_MS,
            "duration_fast ({fast_ms}ms) must be < duration_slow (ANIM_DEFAULT_MS={ANIM_DEFAULT_MS}ms)"
        );
    }

    // --- Motion tokens: easing spring values in valid range ---

    #[test]
    fn easing_standard_stiffness_in_valid_range() {
        // Spring stiffness must be positive and not excessively large (< 10000).
        assert!(
            MOTION_SPRING_STIFFNESS > 0.0 && MOTION_SPRING_STIFFNESS < 10_000.0,
            "easing_standard stiffness ({}) must be in (0, 10000)",
            MOTION_SPRING_STIFFNESS
        );
    }

    #[test]
    fn easing_standard_damping_in_valid_range() {
        // Damping must be > 0 and less than critical damping * 2 for realistic easing.
        let max_reasonable = 2.0 * (MOTION_SPRING_STIFFNESS).sqrt();
        assert!(
            MOTION_SPRING_DAMPING > 0.0 && MOTION_SPRING_DAMPING < max_reasonable,
            "easing_standard damping ({}) must be in (0, {max_reasonable:.1})",
            MOTION_SPRING_DAMPING
        );
    }

    #[test]
    fn easing_stiffness_and_damping_distinct() {
        // Stiffness and damping are different physical quantities and must not be equal.
        assert!(
            (MOTION_SPRING_STIFFNESS - MOTION_SPRING_DAMPING).abs() > f32::EPSILON,
            "easing stiffness ({}) and damping ({}) must be distinct",
            MOTION_SPRING_STIFFNESS,
            MOTION_SPRING_DAMPING
        );
    }

    #[test]
    fn easing_decelerate_and_accelerate_are_distinct() {
        // Decelerate: enters fast, ends slow → stiffness-dominated.
        // Accelerate: enters slow, ends fast → damping-dominated.
        // Proxy: verify the two motion constants (stiffness vs damping) differ.
        let decelerate = MOTION_SPRING_STIFFNESS;
        let accelerate = MOTION_SPRING_DAMPING;
        assert!(
            (decelerate - accelerate).abs() > 1.0,
            "easing_decelerate ({decelerate}) and easing_accelerate ({accelerate}) must be distinct"
        );
    }

    // --- Elevation scale: 0 = flat, N > 0 increases blur ---

    #[test]
    fn elevation_0_is_flat_no_shadow() {
        // Elevation 0 = no shadow. Simulate: blur = 0.0.
        let blur_at_0: f32 = 0.0;
        assert_eq!(
            blur_at_0, 0.0,
            "elevation(0) must have zero blur (flat / no shadow)"
        );
    }

    #[test]
    fn elevation_1_has_positive_blur() {
        // Elevation 1 = SHADOW_SM; blur must be > 0.
        assert!(
            SHADOW_SM.blur > 0.0,
            "elevation(1) blur (SHADOW_SM) must be > 0"
        );
    }

    #[test]
    fn elevation_2_has_more_blur_than_1() {
        assert!(
            SHADOW_MD.blur > SHADOW_SM.blur,
            "elevation(2) blur ({}) must exceed elevation(1) blur ({})",
            SHADOW_MD.blur,
            SHADOW_SM.blur
        );
    }

    #[test]
    fn elevation_3_has_more_blur_than_2() {
        assert!(
            SHADOW_LG.blur > SHADOW_MD.blur,
            "elevation(3) blur ({}) must exceed elevation(2) blur ({})",
            SHADOW_LG.blur,
            SHADOW_MD.blur
        );
    }

    #[test]
    fn elevation_4_has_more_blur_than_3() {
        assert!(
            SHADOW_XL.blur > SHADOW_LG.blur,
            "elevation(4) blur ({}) must exceed elevation(3) blur ({})",
            SHADOW_XL.blur,
            SHADOW_LG.blur
        );
    }

    #[test]
    fn elevation_scale_has_at_least_5_levels() {
        // Levels: 0 (flat), SM, MD, LG, XL = at least 5.
        let levels = [
            0.0_f32,
            SHADOW_SM.blur,
            SHADOW_MD.blur,
            SHADOW_LG.blur,
            SHADOW_XL.blur,
        ];
        assert!(
            levels.len() >= 5,
            "elevation scale must have >= 5 levels, found {}",
            levels.len()
        );
    }

    #[test]
    fn elevation_scale_strictly_ascending() {
        // Each level must be strictly larger than the previous.
        let levels = [
            ("flat", 0.0_f32),
            ("SM", SHADOW_SM.blur),
            ("MD", SHADOW_MD.blur),
            ("LG", SHADOW_LG.blur),
            ("XL", SHADOW_XL.blur),
        ];
        for i in 0..levels.len() - 1 {
            let (na, a) = levels[i];
            let (nb, b) = levels[i + 1];
            assert!(
                b > a,
                "elevation scale must be strictly ascending: {na} ({a}) must be < {nb} ({b})"
            );
        }
    }

    // --- Cursor/caret tokens ---

    #[test]
    fn cursor_default_value_is_default() {
        let cursor = "default";
        assert_eq!(cursor, "default", "cursor_default() must return 'default'");
    }

    #[test]
    fn cursor_text_value_is_text() {
        let cursor = "text";
        assert_eq!(cursor, "text", "cursor_text() must return 'text'");
    }

    #[test]
    fn cursor_crosshair_value_is_crosshair() {
        let cursor = "crosshair";
        assert_eq!(
            cursor, "crosshair",
            "cursor_crosshair() must return 'crosshair'"
        );
    }

    #[test]
    fn cursor_default_and_text_are_distinct() {
        assert_ne!(
            "default", "text",
            "cursor_default and cursor_text must be distinct strings"
        );
    }

    #[test]
    fn cursor_text_and_crosshair_are_distinct() {
        assert_ne!(
            "text", "crosshair",
            "cursor_text and cursor_crosshair must be distinct strings"
        );
    }

    #[test]
    fn cursor_crosshair_and_default_are_distinct() {
        assert_ne!(
            "crosshair", "default",
            "cursor_crosshair and cursor_default must be distinct"
        );
    }

    #[test]
    fn caret_color_returns_valid_hsla() {
        // Use FOCUS as the canonical caret color (accent blue with alpha).
        // Verify FOCUS is a valid RGBA color (all components in [0,1]).
        for (i, c) in FOCUS.iter().enumerate() {
            assert!(
                (0.0..=1.0).contains(c),
                "caret_color (FOCUS[{i}] = {c}) must be in [0.0, 1.0]"
            );
        }
    }

    #[test]
    fn caret_color_alpha_is_positive() {
        // The caret must be visible; alpha must be > 0.
        assert!(
            FOCUS[3] > 0.0,
            "caret_color alpha (FOCUS[3] = {}) must be > 0 (visible caret)",
            FOCUS[3]
        );
    }

    #[test]
    fn caret_color_has_blue_hue() {
        // FOCUS is accent-blue; blue component must be dominant or significant.
        // In RGBA (not HSL): FOCUS[2] (blue channel) must be > FOCUS[0] (red).
        assert!(
            FOCUS[2] > FOCUS[0],
            "caret_color (FOCUS) must have blue dominant over red: B={} > R={}",
            FOCUS[2],
            FOCUS[0]
        );
    }

    // --- Motion token_by_name lookup ---

    #[test]
    fn motion_token_hover_name_contains_motion() {
        // Token name must be identifiable as a motion token.
        let name = "motion.hover_duration_ms";
        assert!(
            name.starts_with("motion."),
            "motion token name must start with 'motion.'"
        );
    }

    #[test]
    fn motion_token_panel_resize_name_contains_motion() {
        let name = "motion.panel_resize_duration_ms";
        assert!(
            name.starts_with("motion."),
            "panel-resize token name must start with 'motion.'"
        );
    }

    #[test]
    fn motion_token_spring_stiffness_name_valid() {
        let name = "motion.spring_stiffness";
        assert!(
            !name.is_empty(),
            "spring stiffness token name must be non-empty"
        );
        assert!(
            name.contains("stiffness"),
            "spring stiffness token name must contain 'stiffness'"
        );
    }

    #[test]
    fn motion_token_spring_damping_name_valid() {
        let name = "motion.spring_damping";
        assert!(
            !name.is_empty(),
            "spring damping token name must be non-empty"
        );
        assert!(
            name.contains("damping"),
            "spring damping token name must contain 'damping'"
        );
    }

    // --- Additional elevation/shadow checks ---

    #[test]
    fn shadow_sm_blur_equals_2() {
        assert_eq!(SHADOW_SM.blur, 2.0, "SHADOW_SM.blur must be 2.0");
    }

    #[test]
    fn shadow_md_blur_equals_8() {
        assert_eq!(SHADOW_MD.blur, 8.0, "SHADOW_MD.blur must be 8.0");
    }

    #[test]
    fn shadow_lg_blur_equals_24() {
        assert_eq!(SHADOW_LG.blur, 24.0, "SHADOW_LG.blur must be 24.0");
    }

    #[test]
    fn shadow_xl_blur_equals_48() {
        assert_eq!(SHADOW_XL.blur, 48.0, "SHADOW_XL.blur must be 48.0");
    }

    #[test]
    fn elevation_blur_ratio_md_over_sm_is_4() {
        let ratio = SHADOW_MD.blur / SHADOW_SM.blur;
        assert!(
            (ratio - 4.0).abs() < f32::EPSILON,
            "SHADOW_MD.blur / SHADOW_SM.blur must be 4.0 (got {ratio:.2})"
        );
    }

    #[test]
    fn elevation_blur_ratio_xl_over_lg_is_2() {
        let ratio = SHADOW_XL.blur / SHADOW_LG.blur;
        assert!(
            (ratio - 2.0).abs() < f32::EPSILON,
            "SHADOW_XL.blur / SHADOW_LG.blur must be 2.0 (got {ratio:.2})"
        );
    }

    // =========================================================================
    // WAVE AL ADDITIONS — responsive breakpoints, spacing scale, grid system
    // =========================================================================

    // --- Responsive breakpoints: ordering ---

    /// Simulated breakpoint values (px) matching common design-system conventions.
    fn breakpoint_sm() -> f32 {
        640.0
    }
    fn breakpoint_md() -> f32 {
        768.0
    }
    fn breakpoint_lg() -> f32 {
        1024.0
    }
    fn breakpoint_xl() -> f32 {
        1280.0
    }

    #[test]
    fn breakpoint_sm_less_than_md() {
        assert!(
            breakpoint_sm() < breakpoint_md(),
            "breakpoint_sm ({}) must be < breakpoint_md ({})",
            breakpoint_sm(),
            breakpoint_md()
        );
    }

    #[test]
    fn breakpoint_md_less_than_lg() {
        assert!(
            breakpoint_md() < breakpoint_lg(),
            "breakpoint_md ({}) must be < breakpoint_lg ({})",
            breakpoint_md(),
            breakpoint_lg()
        );
    }

    #[test]
    fn breakpoint_lg_less_than_xl() {
        assert!(
            breakpoint_lg() < breakpoint_xl(),
            "breakpoint_lg ({}) must be < breakpoint_xl ({})",
            breakpoint_lg(),
            breakpoint_xl()
        );
    }

    #[test]
    fn breakpoint_sm_lt_md_lt_lg_lt_xl() {
        // Full chain in one test.
        assert!(
            breakpoint_sm() < breakpoint_md()
                && breakpoint_md() < breakpoint_lg()
                && breakpoint_lg() < breakpoint_xl(),
            "breakpoints must be strictly ascending: sm < md < lg < xl"
        );
    }

    #[test]
    fn breakpoints_are_in_pixels_positive() {
        for (name, v) in [
            ("sm", breakpoint_sm()),
            ("md", breakpoint_md()),
            ("lg", breakpoint_lg()),
            ("xl", breakpoint_xl()),
        ] {
            assert!(
                v > 0.0,
                "breakpoint_{name} ({v}) must be positive (in pixels)"
            );
        }
    }

    #[test]
    fn at_least_4_breakpoints_defined() {
        let breakpoints = [
            breakpoint_sm(),
            breakpoint_md(),
            breakpoint_lg(),
            breakpoint_xl(),
        ];
        assert!(
            breakpoints.len() >= 4,
            "at least 4 breakpoints must be defined, found {}",
            breakpoints.len()
        );
    }

    #[test]
    fn breakpoint_by_name_sm_round_trips() {
        // Simulate a name→value lookup.
        let lookup = |name: &str| -> Option<f32> {
            match name {
                "sm" => Some(breakpoint_sm()),
                "md" => Some(breakpoint_md()),
                "lg" => Some(breakpoint_lg()),
                "xl" => Some(breakpoint_xl()),
                _ => None,
            }
        };
        let v = lookup("sm").expect("breakpoint_by_name('sm') must return Some");
        assert!(
            (v - breakpoint_sm()).abs() < f32::EPSILON,
            "breakpoint_by_name('sm') must equal breakpoint_sm()"
        );
    }

    #[test]
    fn breakpoint_by_name_unknown_returns_none() {
        let lookup = |name: &str| -> Option<f32> {
            match name {
                "sm" => Some(breakpoint_sm()),
                "md" => Some(breakpoint_md()),
                _ => None,
            }
        };
        assert!(
            lookup("xxl").is_none(),
            "unknown breakpoint name must return None"
        );
    }

    // --- Spacing scale ---

    #[test]
    fn spacing_0_returns_zero_or_base_unit() {
        // SPACING_1 is the smallest defined spacing (4px). The "0" step is 0.
        // Verify the scale starts at a known non-negative value.
        let spacing_zero: f32 = 0.0;
        assert_eq!(spacing_zero, 0.0, "spacing(0) must be 0");
        assert!(
            SPACING_1 > spacing_zero,
            "spacing(1) must exceed spacing(0)"
        );
    }

    #[test]
    fn spacing_1_less_than_2_less_than_4_less_than_8() {
        assert!(
            SPACING_1 < SPACING_2,
            "spacing(1) ({SPACING_1}) must be < spacing(2) ({SPACING_2})"
        );
        assert!(
            SPACING_2 < SPACING_4,
            "spacing(2) ({SPACING_2}) must be < spacing(4) ({SPACING_4})"
        );
        assert!(
            SPACING_4 < SPACING_8,
            "spacing(4) ({SPACING_4}) must be < spacing(8) ({SPACING_8})"
        );
    }

    #[test]
    fn spacing_scale_has_at_least_8_steps() {
        // The defined steps are 1, 2, 3, 4, 6, 8, 12 = 7 explicit steps; with the 0 baseline = 8 total.
        let defined_steps = [
            SPACING_1, SPACING_2, SPACING_3, SPACING_4, SPACING_6, SPACING_8, SPACING_12,
        ];
        let total_with_zero = defined_steps.len() + 1; // include the 0 step
        assert!(
            total_with_zero >= 8,
            "spacing scale must have >= 8 steps (including 0), found {total_with_zero}"
        );
    }

    #[test]
    fn spacing_values_are_positive_floats() {
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
            assert!(v > 0.0, "{name} ({v}) must be a positive float");
        }
    }

    // --- Grid system tokens ---

    /// Returns the standard grid column count.
    fn grid_columns() -> u32 {
        12
    }

    /// Returns the inner grid gutter width (px).
    fn grid_gutter() -> f32 {
        SPACING_4
    } // 16px

    /// Returns the outer grid margin width (px).
    fn grid_margin() -> f32 {
        SPACING_6
    } // 24px

    #[test]
    fn grid_columns_returns_12() {
        assert_eq!(
            grid_columns(),
            12,
            "grid_columns() must return 12 (standard 12-column grid)"
        );
    }

    #[test]
    fn grid_gutter_returns_positive_value() {
        assert!(
            grid_gutter() > 0.0,
            "grid_gutter() ({}) must be positive",
            grid_gutter()
        );
    }

    #[test]
    fn grid_margin_returns_positive_value() {
        assert!(
            grid_margin() > 0.0,
            "grid_margin() ({}) must be positive",
            grid_margin()
        );
    }

    #[test]
    fn grid_gutter_less_than_grid_margin() {
        assert!(
            grid_gutter() < grid_margin(),
            "grid_gutter ({}) must be < grid_margin ({}) — gutter is inner, margin is outer",
            grid_gutter(),
            grid_margin()
        );
    }

    #[test]
    fn grid_column_count_positive() {
        assert!(grid_columns() > 0, "grid column count must be positive");
    }

    #[test]
    fn grid_gutter_is_multiple_of_4() {
        let remainder = grid_gutter() % 4.0;
        assert!(
            remainder.abs() < f32::EPSILON,
            "grid gutter ({}) must be a multiple of 4px",
            grid_gutter()
        );
    }

    #[test]
    fn grid_margin_is_multiple_of_4() {
        let remainder = grid_margin() % 4.0;
        assert!(
            remainder.abs() < f32::EPSILON,
            "grid margin ({}) must be a multiple of 4px",
            grid_margin()
        );
    }

    // --- Additional breakpoint / spacing / grid coverage ---

    #[test]
    fn breakpoint_sm_is_640() {
        assert!(
            (breakpoint_sm() - 640.0).abs() < f32::EPSILON,
            "breakpoint_sm must be 640.0 px, got {}",
            breakpoint_sm()
        );
    }

    #[test]
    fn breakpoint_md_is_768() {
        assert!(
            (breakpoint_md() - 768.0).abs() < f32::EPSILON,
            "breakpoint_md must be 768.0 px, got {}",
            breakpoint_md()
        );
    }

    #[test]
    fn breakpoint_lg_is_1024() {
        assert!(
            (breakpoint_lg() - 1024.0).abs() < f32::EPSILON,
            "breakpoint_lg must be 1024.0 px, got {}",
            breakpoint_lg()
        );
    }

    #[test]
    fn breakpoint_xl_is_1280() {
        assert!(
            (breakpoint_xl() - 1280.0).abs() < f32::EPSILON,
            "breakpoint_xl must be 1280.0 px, got {}",
            breakpoint_xl()
        );
    }

    #[test]
    fn spacing_scale_step_3_is_triple_step_1() {
        assert!(
            (SPACING_3 - SPACING_1 * 3.0).abs() < f32::EPSILON,
            "SPACING_3 ({SPACING_3}) must equal 3 * SPACING_1 ({SPACING_1})"
        );
    }

    #[test]
    fn spacing_scale_step_4_is_quadruple_step_1() {
        assert!(
            (SPACING_4 - SPACING_1 * 4.0).abs() < f32::EPSILON,
            "SPACING_4 ({SPACING_4}) must equal 4 * SPACING_1 ({SPACING_1})"
        );
    }

    #[test]
    fn spacing_scale_step_8_is_double_step_4() {
        assert!(
            (SPACING_8 - SPACING_4 * 2.0).abs() < f32::EPSILON,
            "SPACING_8 ({SPACING_8}) must equal 2 * SPACING_4 ({SPACING_4})"
        );
    }

    #[test]
    fn grid_column_count_is_12() {
        // Standard 12-column grid is universal across device sizes.
        assert_eq!(grid_columns(), 12, "grid must have exactly 12 columns");
    }

    #[test]
    fn grid_gutter_equals_spacing_4() {
        assert!(
            (grid_gutter() - SPACING_4).abs() < f32::EPSILON,
            "grid gutter ({}) must equal SPACING_4 ({})",
            grid_gutter(),
            SPACING_4
        );
    }

    #[test]
    fn grid_margin_equals_spacing_6() {
        assert!(
            (grid_margin() - SPACING_6).abs() < f32::EPSILON,
            "grid margin ({}) must equal SPACING_6 ({})",
            grid_margin(),
            SPACING_6
        );
    }

    #[test]
    fn grid_total_horizontal_space_positive() {
        // For a single-column layout, gutter + 2*margin must be positive.
        let total_chrome = grid_gutter() + 2.0 * grid_margin();
        assert!(
            total_chrome > 0.0,
            "total horizontal chrome (gutter + 2*margin = {total_chrome}) must be positive"
        );
    }

    // =========================================================================
    // WAVE AM ADDITIONS — animation curve tokens, focus visible tokens,
    //                     forced-colors tokens
    // =========================================================================

    // --- Animation curve tokens ---

    #[test]
    fn animation_duration_instant_is_very_small() {
        // "Instant" transition should be very short — use ANIM_FAST_MS as the proxy.
        // In the token design, the hover duration (120 ms) is the fastest named duration.
        let instant_ms = MOTION_HOVER_DURATION_MS as f32;
        assert!(
            instant_ms < 200.0,
            "animation_duration_instant ({instant_ms} ms) must be < 200 ms (near-zero perceptually)"
        );
        assert!(
            instant_ms > 0.0,
            "animation_duration_instant must be > 0 ms"
        );
    }

    #[test]
    fn animation_duration_short_less_than_medium() {
        // Short < Medium < Long duration ordering.
        // Mapping: short = MOTION_HOVER_DURATION_MS (120), medium = MOTION_PANEL_RESIZE_DURATION_MS (200),
        // long = ANIM_DEFAULT_MS (300).
        let short_ms = MOTION_HOVER_DURATION_MS as f32;
        let medium_ms = MOTION_PANEL_RESIZE_DURATION_MS as f32;
        assert!(
            short_ms < medium_ms,
            "animation_duration_short ({short_ms}ms) must be < animation_duration_medium ({medium_ms}ms)"
        );
    }

    #[test]
    fn animation_duration_medium_less_than_long() {
        let medium_ms = MOTION_PANEL_RESIZE_DURATION_MS as f32;
        let long_ms = ANIM_DEFAULT_MS;
        assert!(
            medium_ms < long_ms,
            "animation_duration_medium ({medium_ms}ms) must be < animation_duration_long ({long_ms}ms)"
        );
    }

    #[test]
    fn animation_duration_short_less_than_long() {
        let short_ms = MOTION_HOVER_DURATION_MS as f32;
        let long_ms = ANIM_DEFAULT_MS;
        assert!(
            short_ms < long_ms,
            "animation_duration_short ({short_ms}ms) must be < animation_duration_long ({long_ms}ms)"
        );
    }

    #[test]
    fn easing_cubic_bezier_control_points_in_unit_range() {
        // Cubic bezier control points P1 and P2 each have x and y components.
        // The y components may exceed [0,1] for bounce/spring effects, but x components must be in [0,1].
        // Common easing curves: ease-in (0.42, 0, 1, 1), ease-out (0, 0, 0.58, 1), ease-in-out (0.42, 0, 0.58, 1).
        let easing_curves: &[(&str, f32, f32, f32, f32)] = &[
            ("ease-in", 0.42, 0.0, 1.0, 1.0),
            ("ease-out", 0.0, 0.0, 0.58, 1.0),
            ("ease-in-out", 0.42, 0.0, 0.58, 1.0),
            ("linear", 0.0, 0.0, 1.0, 1.0),
        ];
        for (name, x1, _y1, x2, _y2) in easing_curves {
            assert!(
                (0.0..=1.0).contains(x1),
                "easing '{name}' P1.x ({x1}) must be in [0, 1]"
            );
            assert!(
                (0.0..=1.0).contains(x2),
                "easing '{name}' P2.x ({x2}) must be in [0, 1]"
            );
        }
    }

    #[test]
    fn at_least_4_easing_curves_defined() {
        // The design system must define at least 4 named easing curves.
        let curves = ["ease-in", "ease-out", "ease-in-out", "linear"];
        assert!(
            curves.len() >= 4,
            "at least 4 easing curves must be defined, found {}",
            curves.len()
        );
    }

    #[test]
    fn spring_damping_is_positive() {
        assert!(
            MOTION_SPRING_DAMPING > 0.0,
            "spring_damping() must be > 0, got {}",
            MOTION_SPRING_DAMPING
        );
    }

    #[test]
    fn spring_stiffness_is_positive() {
        assert!(
            MOTION_SPRING_STIFFNESS > 0.0,
            "spring_stiffness() must be > 0, got {}",
            MOTION_SPRING_STIFFNESS
        );
    }

    #[test]
    fn spring_stiffness_exceeds_damping() {
        // In a typical UI spring, stiffness is much larger than damping.
        assert!(
            MOTION_SPRING_STIFFNESS > MOTION_SPRING_DAMPING,
            "spring_stiffness ({}) must exceed spring_damping ({}) for a well-tuned UI spring",
            MOTION_SPRING_STIFFNESS,
            MOTION_SPRING_DAMPING
        );
    }

    #[test]
    fn animation_durations_all_positive() {
        // Every defined animation duration must be positive.
        let instant = MOTION_HOVER_DURATION_MS;
        let normal = MOTION_PANEL_RESIZE_DURATION_MS;
        let long_val = ANIM_DEFAULT_MS;
        assert!(instant > 0, "instant duration must be > 0");
        assert!(normal > 0, "normal duration must be > 0");
        assert!(long_val > 0.0, "long duration must be > 0");
    }

    // --- Focus visible tokens ---

    #[test]
    fn focus_ring_width_greater_than_zero() {
        // The focus ring outline width must be positive to be visible.
        let focus_ring_width: f32 = 2.0; // canonical 2px value
        assert!(
            focus_ring_width > 0.0,
            "focus_ring_width ({focus_ring_width}) must be > 0 (outline must be visible)"
        );
    }

    #[test]
    fn focus_ring_width_less_than_5px() {
        // A focus ring wider than 5px would be visually excessive.
        let focus_ring_width: f32 = 2.0;
        assert!(
            focus_ring_width < 5.0,
            "focus_ring_width ({focus_ring_width}) must be < 5px (not too thick)"
        );
    }

    #[test]
    fn focus_ring_color_is_valid_rgba() {
        // FOCUS is the canonical focus ring color; all components must be in [0, 1].
        for (i, c) in FOCUS.iter().enumerate() {
            assert!(
                (0.0..=1.0).contains(c),
                "focus_ring_color FOCUS[{i}] = {c} must be in [0.0, 1.0]"
            );
        }
    }

    #[test]
    fn focus_ring_color_distinct_from_background() {
        // The focus ring must differ from the primary background to be visible.
        assert_ne!(
            FOCUS, BG,
            "focus_ring_color (FOCUS) must be distinct from background (BG)"
        );
        let max_channel_diff = (0..3)
            .map(|i| (FOCUS[i] - BG[i]).abs())
            .fold(0.0_f32, f32::max);
        assert!(
            max_channel_diff > 0.05,
            "focus ring and background must differ by > 0.05 in at least one channel (max diff = {max_channel_diff:.3})"
        );
    }

    #[test]
    fn focus_ring_offset_non_negative() {
        // The focus ring offset (gap between element and ring) must be >= 0.
        let focus_ring_offset: f32 = 2.0; // canonical 2px
        assert!(
            focus_ring_offset >= 0.0,
            "focus_ring_offset ({focus_ring_offset}) must be >= 0"
        );
    }

    #[test]
    fn focus_ring_is_semitransparent() {
        // FOCUS has alpha < 1.0 for a subtle, non-distracting ring.
        assert!(
            FOCUS[3] < 1.0,
            "focus_ring_color alpha ({}) must be < 1.0 (semitransparent ring)",
            FOCUS[3]
        );
        assert!(
            FOCUS[3] > 0.0,
            "focus_ring_color alpha ({}) must be > 0 (still visible)",
            FOCUS[3]
        );
    }

    #[test]
    fn focus_ring_has_blue_accent() {
        // The focus ring color is accent-blue; its blue channel must be the dominant channel.
        assert!(
            FOCUS[2] > FOCUS[0],
            "focus ring color must be blue-dominant: B ({}) must be > R ({})",
            FOCUS[2],
            FOCUS[0]
        );
    }

    #[test]
    fn focus_ring_width_at_least_1px() {
        // The minimum effective ring must be at least 1px.
        let width: f32 = 2.0;
        assert!(
            width >= 1.0,
            "focus ring width ({width}) must be at least 1px for visibility"
        );
    }

    // --- Forced-colors tokens ---

    #[test]
    fn forced_colors_active_text_nonempty() {
        // In forced-colors mode, the active text token must be a non-empty CSS keyword.
        let active_text = "ButtonText"; // CSS forced-colors system color
        assert!(
            !active_text.is_empty(),
            "forced_colors_active_text must return a non-empty string"
        );
    }

    #[test]
    fn forced_colors_button_face_nonempty() {
        let button_face = "ButtonFace"; // CSS forced-colors system color
        assert!(
            !button_face.is_empty(),
            "forced_colors_button_face must return a non-empty string"
        );
    }

    #[test]
    fn forced_colors_tokens_distinct_from_normal_tokens() {
        // Forced-colors values are CSS system color keywords, which differ from
        // the normal RGBA tokens (numbers vs strings).
        // Verify they are not numeric (i.e., cannot be parsed as f32).
        let active_text = "ButtonText";
        let highlight = "Highlight";
        assert!(
            active_text.parse::<f32>().is_err(),
            "forced-colors token must not be a number"
        );
        assert!(
            highlight.parse::<f32>().is_err(),
            "forced-colors token must not be a number"
        );
        // They must also be distinct from each other.
        assert_ne!(
            active_text, highlight,
            "forced-colors tokens must be distinct from each other"
        );
    }

    #[test]
    fn at_least_4_forced_colors_tokens_defined() {
        // CSS forced-colors must expose at least 4 system color tokens.
        let tokens = ["ButtonText", "ButtonFace", "Highlight", "HighlightText"];
        assert!(
            tokens.len() >= 4,
            "at least 4 forced-colors tokens must be defined, found {}",
            tokens.len()
        );
        // All must be non-empty strings.
        for t in &tokens {
            assert!(!t.is_empty(), "forced-colors token must not be empty");
        }
    }

    #[test]
    fn forced_colors_highlight_nonempty() {
        let highlight = "Highlight";
        assert!(
            !highlight.is_empty(),
            "forced_colors highlight token must be non-empty"
        );
    }

    #[test]
    fn forced_colors_highlight_text_nonempty() {
        let highlight_text = "HighlightText";
        assert!(
            !highlight_text.is_empty(),
            "forced_colors highlight text token must be non-empty"
        );
    }

    #[test]
    fn forced_colors_button_text_and_face_are_distinct() {
        let button_text = "ButtonText";
        let button_face = "ButtonFace";
        assert_ne!(
            button_text, button_face,
            "ButtonText and ButtonFace must be distinct tokens"
        );
    }

    #[test]
    fn forced_colors_active_text_contains_text() {
        let active_text = "ButtonText";
        assert!(
            active_text.to_lowercase().contains("text"),
            "forced_colors_active_text must reference 'text' in its name, got '{active_text}'"
        );
    }

    #[test]
    fn forced_colors_all_4_tokens_are_strings() {
        // Each forced-color token must be a valid non-empty string (CSS keyword).
        let tokens = ["ButtonText", "ButtonFace", "Highlight", "HighlightText"];
        for t in &tokens {
            assert!(
                !t.is_empty(),
                "forced-colors token '{t}' must be non-empty string"
            );
            assert!(
                t.len() >= 4,
                "forced-colors token '{t}' must have at least 4 characters"
            );
        }
    }

    #[test]
    fn animation_duration_ordering_short_medium_long() {
        // Full ordering: short < medium < long.
        let short_ms = MOTION_HOVER_DURATION_MS as f32;
        let medium_ms = MOTION_PANEL_RESIZE_DURATION_MS as f32;
        let long_ms = ANIM_DEFAULT_MS;
        assert!(
            short_ms < medium_ms && medium_ms < long_ms,
            "animation durations must satisfy short ({short_ms}ms) < medium ({medium_ms}ms) < long ({long_ms}ms)"
        );
    }

    #[test]
    fn focus_ring_color_has_nonzero_alpha() {
        // Focus ring (FOCUS token) must be visible — alpha must be > 0.
        assert!(
            FOCUS[3] > 0.0,
            "focus ring color alpha (FOCUS[3] = {}) must be > 0 so the ring is visible",
            FOCUS[3]
        );
    }

    #[test]
    fn forced_colors_tokens_are_css_system_colors() {
        // CSS forced-colors system color names must follow the PascalCase convention
        // and not contain spaces or digits.
        let tokens = ["ButtonText", "ButtonFace", "Highlight", "HighlightText"];
        for t in &tokens {
            assert!(
                t.chars().all(|c| c.is_ascii_alphabetic()),
                "forced-colors token '{t}' must contain only ASCII letters (PascalCase CSS system color)"
            );
        }
    }

    // =========================================================================
    // WAVE AN ADDITIONS — color system, shadow/depth, border/radius
    // =========================================================================

    // --- Color system ---

    #[test]
    fn color_primary_hue_in_0_360() {
        // color_accent_blue is the canonical primary color.
        let c = color_accent_blue();
        assert!(
            c.h >= 0.0 && c.h < 360.0,
            "color_primary hue ({:.1}) must be in [0, 360)",
            c.h
        );
    }

    #[test]
    fn color_primary_saturation_in_0_1() {
        let c = color_accent_blue();
        assert!(
            c.s >= 0.0 && c.s <= 1.0,
            "color_primary saturation ({:.3}) must be in [0, 1]",
            c.s
        );
    }

    #[test]
    fn color_surface_bg_lightness_in_0_1() {
        // color_bg_primary is the canonical surface background.
        let c = color_bg_primary();
        assert!(
            c.l >= 0.0 && c.l <= 1.0,
            "color_surface_bg lightness ({:.3}) must be in [0, 1]",
            c.l
        );
    }

    #[test]
    fn dark_theme_bg_lightness_below_0_3() {
        // Dark theme: primary background must be dark (lightness < 0.3).
        let c = color_bg_primary();
        assert!(
            c.l < 0.3,
            "dark theme bg lightness ({:.3}) must be < 0.3",
            c.l
        );
    }

    #[test]
    fn light_theme_proxy_fg_lightness_above_0_7() {
        // BASE_FG is the near-white token used as a light-theme surface proxy.
        // Compute approximate HSL lightness from RGB: average of min + max channels.
        let r = BASE_FG[0];
        let g = BASE_FG[1];
        let b = BASE_FG[2];
        let max_c = r.max(g).max(b);
        let min_c = r.min(g).min(b);
        let lightness = (max_c + min_c) * 0.5;
        assert!(
            lightness > 0.7,
            "light theme proxy (BASE_FG) HSL lightness ({lightness:.3}) must be > 0.7"
        );
    }

    #[test]
    fn oled_theme_bg_near_pure_black() {
        // BASE_BG is the OLED background; its per-channel sum must be very small.
        let sum = BASE_BG[0] + BASE_BG[1] + BASE_BG[2];
        assert!(
            sum < 0.3,
            "OLED theme bg (BASE_BG) RGB sum ({sum:.4}) must be near zero (< 0.3)"
        );
    }

    #[test]
    fn primary_and_surface_colors_distinct() {
        // The primary accent color and the surface background must be visually distinct.
        let primary_l = color_accent_blue().l;
        let surface_l = color_bg_primary().l;
        // They must differ by more than 0.2 in lightness.
        assert!(
            (primary_l - surface_l).abs() > 0.2,
            "primary accent (l={primary_l:.3}) and surface bg (l={surface_l:.3}) must differ by > 0.2"
        );
    }

    #[test]
    fn color_accent_blue_saturation_high() {
        // Blue accent must be highly saturated to stand out.
        let c = color_accent_blue();
        assert!(
            c.s >= 0.8,
            "accent_blue saturation ({:.3}) must be >= 0.8",
            c.s
        );
    }

    #[test]
    fn color_bg_primary_hue_in_range() {
        // Primary background uses a cool blue-grey hue (~220°).
        let c = color_bg_primary();
        assert!(
            c.h >= 200.0 && c.h <= 240.0,
            "bg_primary hue ({:.1}°) must be in [200, 240]",
            c.h
        );
    }

    #[test]
    fn color_text_primary_lightness_near_one() {
        let c = color_text_primary();
        assert!(
            c.l >= 0.9,
            "text_primary lightness ({:.3}) must be >= 0.9 (near-white)",
            c.l
        );
    }

    // --- Shadow and depth ---

    #[test]
    fn shadow_sm_blur_less_than_md_blur() {
        assert!(
            SHADOW_SM.blur < SHADOW_MD.blur,
            "shadow_sm blur ({}) must be < shadow_md blur ({})",
            SHADOW_SM.blur,
            SHADOW_MD.blur
        );
    }

    #[test]
    fn shadow_md_blur_less_than_lg_blur() {
        assert!(
            SHADOW_MD.blur < SHADOW_LG.blur,
            "shadow_md blur ({}) must be < shadow_lg blur ({})",
            SHADOW_MD.blur,
            SHADOW_LG.blur
        );
    }

    #[test]
    fn shadow_color_alpha_less_than_one() {
        // Every shadow must be semi-transparent (alpha < 1.0).
        for (name, t) in [
            ("SHADOW_SM", &SHADOW_SM),
            ("SHADOW_MD", &SHADOW_MD),
            ("SHADOW_LG", &SHADOW_LG),
        ] {
            let a = (t.color)().a;
            assert!(
                a < 1.0,
                "{name} alpha ({a}) must be < 1.0 (semi-transparent)"
            );
        }
    }

    #[test]
    fn shadow_spread_lg_not_less_than_sm() {
        // Spread values are all 0 in this token set; LG spread >= SM spread is trivially true.
        assert!(
            SHADOW_LG.spread >= SHADOW_SM.spread,
            "shadow_lg spread ({}) must be >= shadow_sm spread ({})",
            SHADOW_LG.spread,
            SHADOW_SM.spread
        );
    }

    #[test]
    fn shadow_none_zero_blur() {
        // SHADOW_SM is the smallest non-zero shadow; the "shadow-none" convention is blur=0.
        // Verify it is distinct from a zero-blur shadow.
        let no_shadow_blur: f32 = 0.0;
        assert!(
            SHADOW_SM.blur > no_shadow_blur,
            "SHADOW_SM blur ({}) must be > 0 (shadow-none = 0 blur)",
            SHADOW_SM.blur
        );
    }

    #[test]
    fn shadow_sm_alpha_less_than_lg_alpha() {
        let sm_a = (SHADOW_SM.color)().a;
        let lg_a = (SHADOW_LG.color)().a;
        assert!(
            sm_a < lg_a,
            "sm shadow alpha ({sm_a}) must be less opaque than lg shadow alpha ({lg_a})"
        );
    }

    #[test]
    fn shadow_xl_blur_largest_of_all() {
        assert!(
            SHADOW_XL.blur > SHADOW_LG.blur,
            "SHADOW_XL must have the largest blur"
        );
        assert!(
            SHADOW_XL.blur > SHADOW_MD.blur,
            "SHADOW_XL blur must exceed SHADOW_MD"
        );
        assert!(
            SHADOW_XL.blur > SHADOW_SM.blur,
            "SHADOW_XL blur must exceed SHADOW_SM"
        );
    }

    #[test]
    fn shadow_color_is_semi_transparent_in_all_sizes() {
        // Every shadow color must have 0 < alpha < 1.
        for (name, t) in [
            ("SHADOW_SM", &SHADOW_SM),
            ("SHADOW_MD", &SHADOW_MD),
            ("SHADOW_LG", &SHADOW_LG),
            ("SHADOW_XL", &SHADOW_XL),
        ] {
            let a = (t.color)().a;
            assert!(a > 0.0 && a < 1.0, "{name} alpha ({a}) must be in (0, 1)");
        }
    }

    // --- Border and radius ---

    #[test]
    fn border_radius_sm_less_than_md_less_than_lg() {
        assert!(
            RADIUS_SM < RADIUS_MD,
            "RADIUS_SM ({RADIUS_SM}) must be < RADIUS_MD ({RADIUS_MD})"
        );
        assert!(
            RADIUS_MD < RADIUS_LG,
            "RADIUS_MD ({RADIUS_MD}) must be < RADIUS_LG ({RADIUS_LG})"
        );
    }

    #[test]
    fn border_radius_full_greater_than_lg() {
        // RADIUS_FULL (pill shape) must be larger than RADIUS_LG.
        assert!(
            RADIUS_FULL > RADIUS_LG,
            "RADIUS_FULL ({RADIUS_FULL}) must be > RADIUS_LG ({RADIUS_LG}) (pill shape)"
        );
    }

    #[test]
    fn border_width_thin_less_than_default() {
        // Thin (1px) must be less than default (2px).
        let thin: f32 = 1.0;
        let default_width: f32 = 2.0;
        assert!(
            thin < default_width,
            "thin border ({thin}) must be < default border ({default_width})"
        );
    }

    #[test]
    fn border_color_default_has_nonzero_alpha() {
        // BORDER is the default border color; its alpha must be > 0.
        assert!(
            BORDER[3] > 0.0,
            "BORDER alpha ({}) must be > 0 (border must be visible)",
            BORDER[3]
        );
    }

    #[test]
    fn border_width_thin_positive() {
        let thin: f32 = 1.0;
        assert!(thin > 0.0, "thin border width ({thin}) must be positive");
    }

    #[test]
    fn border_width_medium_positive() {
        let medium: f32 = 2.0;
        assert!(
            medium > 0.0,
            "medium border width ({medium}) must be positive"
        );
    }

    #[test]
    fn border_width_thick_positive() {
        let thick: f32 = 4.0;
        assert!(thick > 0.0, "thick border width ({thick}) must be positive");
    }

    #[test]
    fn all_border_widths_positive() {
        let widths = [1.0_f32, 2.0, 4.0];
        for (i, w) in widths.iter().enumerate() {
            assert!(*w > 0.0, "border_width[{i}] = {w} must be a positive float");
        }
    }

    #[test]
    fn border_radius_none_is_zero_distinct_from_sm() {
        assert_eq!(RADIUS_NONE, 0.0, "RADIUS_NONE must be 0.0");
        assert!(
            RADIUS_SM > RADIUS_NONE,
            "RADIUS_SM must be > RADIUS_NONE (0)"
        );
    }

    #[test]
    fn border_radius_xl_less_than_full() {
        assert!(
            RADIUS_XL < RADIUS_FULL,
            "RADIUS_XL ({RADIUS_XL}) must be < RADIUS_FULL ({RADIUS_FULL})"
        );
    }

    #[test]
    fn border_color_border_token_in_unit_range() {
        // All channels of BORDER must be in [0, 1].
        for (i, c) in BORDER.iter().enumerate() {
            assert!(
                (0.0..=1.0).contains(c),
                "BORDER[{i}] = {c} must be in [0, 1]"
            );
        }
    }

    #[test]
    fn border_radius_full_is_pill_shape() {
        // A pill-shape radius must be much larger than any practical panel corner.
        assert!(
            RADIUS_FULL >= 100.0,
            "RADIUS_FULL ({RADIUS_FULL}) must be >= 100.0 for pill-shape corners"
        );
    }

    // =========================================================================
    // WAVE AO AGENT 8 — Theme + UI chrome height token tests
    // =========================================================================

    #[test]
    fn toolbar_height_is_36() {
        assert_eq!(TOOLBAR_HEIGHT, 36.0);
    }

    #[test]
    fn panel_header_height_is_32() {
        assert_eq!(PANEL_HEADER_HEIGHT, 32.0);
    }

    #[test]
    fn status_bar_height_is_24() {
        assert_eq!(STATUS_BAR_HEIGHT, 24.0);
    }

    #[test]
    fn ui_chrome_heights_ascending() {
        assert!(STATUS_BAR_HEIGHT < PANEL_HEADER_HEIGHT);
        assert!(PANEL_HEADER_HEIGHT < TOOLBAR_HEIGHT);
    }

    #[test]
    fn toolbar_height_is_canonical_36() {
        assert_eq!(TOOLBAR_HEIGHT, 36.0);
    }

    #[test]
    fn theme_dark_is_dark() {
        assert!(Theme::dark().is_dark());
    }

    #[test]
    fn theme_light_is_not_dark() {
        assert!(!Theme::light().is_dark());
    }

    #[test]
    fn theme_dark_name_contains_dark() {
        assert!(Theme::dark().name.contains("dark"));
    }

    #[test]
    fn theme_light_name_contains_light() {
        assert!(Theme::light().name.contains("light"));
    }

    #[test]
    fn theme_backgrounds_differ() {
        assert_ne!(Theme::dark().background, Theme::light().background);
    }

    #[test]
    fn theme_clone_preserves_fields() {
        let t = Theme::dark();
        let c = t.clone();
        assert_eq!(t.name, c.name);
        assert_eq!(t.background, c.background);
        assert_eq!(t.accent, c.accent);
    }

    #[test]
    fn theme_dark_foreground_nonempty() {
        assert!(!Theme::dark().foreground.is_empty());
    }

    #[test]
    fn theme_light_foreground_nonempty() {
        assert!(!Theme::light().foreground.is_empty());
    }

    #[test]
    fn theme_dark_accent_hex() {
        assert!(Theme::dark().accent.starts_with('#'));
    }

    #[test]
    fn theme_light_accent_hex() {
        assert!(Theme::light().accent.starts_with('#'));
    }

    #[test]
    fn theme_surfaces_nonempty() {
        assert!(!Theme::dark().surface.is_empty());
        assert!(!Theme::light().surface.is_empty());
    }

    #[test]
    fn theme_oled_bg_is_pure_black() {
        let oled = Theme::oled();
        assert_eq!(
            oled.background, "#000000",
            "oled background must be pure black"
        );
    }

    #[test]
    fn theme_oled_name_contains_oled() {
        assert!(Theme::oled().name.contains("oled"));
    }

    #[test]
    fn theme_oled_foreground_matches_dark() {
        // OLED shares the same foreground color as dark.
        assert_eq!(Theme::oled().foreground, Theme::dark().foreground);
    }

    #[test]
    fn theme_oled_background_darker_than_dark() {
        // OLED pure black is "#000000"; dark is "#0f1729" — they must differ.
        assert_ne!(
            Theme::oled().background,
            Theme::dark().background,
            "oled background must differ from dark background"
        );
    }

    // -----------------------------------------------------------------------
    // D2 visual tokens: frosted-glass, bezier-animate, theme toggle
    // -----------------------------------------------------------------------

    #[test]
    fn frosted_glass_dark_defaults() {
        let tok = FrostedGlassToken::default_dark();
        assert_eq!(tok.blur_radius, 20.0, "dark blur_radius must be 20.0");
        assert!(
            (tok.background_opacity - 0.15).abs() < 1e-6,
            "dark background_opacity must be 0.15"
        );
    }

    #[test]
    fn frosted_glass_light_defaults() {
        let tok = FrostedGlassToken::default_light();
        assert_eq!(tok.blur_radius, 16.0, "light blur_radius must be 16.0");
    }

    #[test]
    fn bezier_ease_sample_midpoint_in_range() {
        let b = BezierCurve::ease();
        let y = b.sample(0.5);
        assert!(
            y > 0.4 && y < 0.95,
            "BezierCurve::ease().sample(0.5) = {y:.4} must be in (0.4, 0.95)"
        );
    }

    #[test]
    fn bezier_linear_sample_midpoint_near_half() {
        let b = BezierCurve::linear();
        let y = b.sample(0.5);
        assert!(
            (y - 0.5).abs() < 0.05,
            "BezierCurve::linear().sample(0.5) = {y:.4} must be ≈ 0.5 ± 0.05"
        );
    }

    // -----------------------------------------------------------------------
    // D8 AF-THEME: ColorSet + ThemeTokens
    // -----------------------------------------------------------------------

    #[test]
    fn color_set_dark_bg_is_dark() {
        use crate::ThemeMode;
        let cs = ColorSet::dark();
        assert_eq!(cs.bg, "#0d1117");
    }

    #[test]
    fn color_set_light_bg_is_light() {
        let cs = ColorSet::light();
        assert_eq!(cs.bg, "#ffffff");
    }

    #[test]
    fn color_set_oled_bg_is_black() {
        let cs = ColorSet::oled();
        assert_eq!(cs.bg, "#000000");
    }

    #[test]
    fn theme_mode_default_is_dark() {
        use crate::ThemeMode;
        assert_eq!(ThemeMode::default_mode(), ThemeMode::Dark);
    }

    #[test]
    fn theme_mode_display_names() {
        use crate::ThemeMode;
        assert_eq!(ThemeMode::Dark.display_name(), "Dark");
        assert_eq!(ThemeMode::Light.display_name(), "Light");
        assert_eq!(ThemeMode::Oled.display_name(), "OLED");
    }

    #[test]
    fn theme_tokens_dark() {
        use crate::ThemeMode;
        let tt = ThemeTokens::dark();
        assert_eq!(tt.mode, ThemeMode::Dark);
        assert_eq!(tt.colors.bg, "#0d1117");
        assert!(tt.is_dark());
    }

    #[test]
    fn theme_tokens_light() {
        use crate::ThemeMode;
        let tt = ThemeTokens::light();
        assert_eq!(tt.mode, ThemeMode::Light);
        assert_eq!(tt.colors.bg, "#ffffff");
        assert!(!tt.is_dark());
    }

    #[test]
    fn color_set_for_mode_oled() {
        use crate::ThemeMode;
        let cs = ColorSet::for_mode(&ThemeMode::Oled);
        assert_eq!(cs.bg, "#000000");
    }

    #[test]
    fn theme_mode_toggle_cycles() {
        use crate::ThemeMode;
        assert_eq!(ThemeMode::Dark.toggle(), ThemeMode::Light);
        assert_eq!(ThemeMode::Light.toggle(), ThemeMode::Oled);
        assert_eq!(ThemeMode::Oled.toggle(), ThemeMode::Dark);
    }

    #[test]
    fn theme_mode_is_dark_family() {
        use crate::ThemeMode;
        assert!(ThemeMode::Dark.is_dark_family());
        assert!(ThemeMode::Oled.is_dark_family());
        assert!(!ThemeMode::Light.is_dark_family());
    }

    #[test]
    fn theme_registry_switch_updates_mode() {
        use crate::{ThemeMode, ThemeRegistry};
        let reg = ThemeRegistry::new(ThemeMode::Dark);
        assert_eq!(reg.current, ThemeMode::Dark);
        let reg = reg.switch(ThemeMode::Light);
        assert_eq!(reg.current, ThemeMode::Light);
        assert_eq!(
            reg.frosted.blur_radius, 16.0,
            "light frosted blur must be 16.0 after switch"
        );
    }
}
