//! Design tokens extracted from the AFFiNE / blocksuite design system.
//!
//! All color tokens are HSLA tuples `(hue, saturation, lightness, alpha)`
//! where all values are normalised to `[0.0, 1.0]`. Hue = degrees / 360.
//!
//! Light-mode values are the defaults. `mode.rs` provides the mode-aware
//! `Theme` type that swaps to dark-mode variants.
//!
//! Naming rules: SCREAMING_SNAKE_CASE, Nom-native identifiers only.

// ─── Primary / brand family ──────────────────────────────────────────────────

/// Core brand hue — mid-blue used for interactive elements and selection.
pub const PRIMARY_BRAND: (f32, f32, f32, f32) = (0.597, 0.832, 0.582, 1.0);
/// Slightly lighter brand variant used for hover-state fills.
pub const PRIMARY_BRAND_LIGHT: (f32, f32, f32, f32) = (0.597, 0.832, 0.650, 1.0);
/// Secondary emphasis — violet / indigo for secondary actions.
pub const SECONDARY_BRAND: (f32, f32, f32, f32) = (0.700, 0.700, 0.580, 1.0);
/// Tertiary muted hue for supporting chrome.
pub const TERTIARY_BRAND: (f32, f32, f32, f32) = (0.583, 0.400, 0.700, 1.0);
/// Quaternary neutral-blue for decorative regions.
pub const QUATERNARY_BRAND: (f32, f32, f32, f32) = (0.583, 0.200, 0.850, 1.0);

// ─── Text tones ──────────────────────────────────────────────────────────────

/// Primary text — near-black, highest contrast.
pub const TEXT_PRIMARY: (f32, f32, f32, f32) = (0.0, 0.0, 0.102, 1.0);
/// Secondary text — medium-dark, used for labels and metadata.
pub const TEXT_SECONDARY: (f32, f32, f32, f32) = (0.0, 0.0, 0.400, 1.0);
/// Tertiary / placeholder text — lighter weight hints.
pub const TEXT_TERTIARY: (f32, f32, f32, f32) = (0.0, 0.0, 0.600, 1.0);
/// Disabled text — barely-legible intentionally.
pub const TEXT_DISABLED: (f32, f32, f32, f32) = (0.0, 0.0, 0.749, 1.0);
/// Emphasis / highlight text colour used on selected items.
pub const TEXT_EMPHASIS: (f32, f32, f32, f32) = (0.597, 0.832, 0.420, 1.0);

// ─── Surface / background levels ─────────────────────────────────────────────

/// Page background — pure white in light mode.
pub const SURFACE_BACKGROUND: (f32, f32, f32, f32) = (0.0, 0.0, 1.0, 1.0);
/// Primary panel background (sidebar, toolbars).
pub const SURFACE_PRIMARY: (f32, f32, f32, f32) = (0.0, 0.0, 0.980, 1.0);
/// Secondary card / container surface.
pub const SURFACE_SECONDARY: (f32, f32, f32, f32) = (0.0, 0.0, 0.961, 1.0);
/// Tertiary inset surface (code blocks, quoted regions).
pub const SURFACE_TERTIARY: (f32, f32, f32, f32) = (0.0, 0.0, 0.941, 1.0);
/// Modal / overlay backdrop surface.
pub const SURFACE_MODAL: (f32, f32, f32, f32) = (0.0, 0.0, 1.0, 0.90);
/// Floating panel (popovers, tooltips, context menus).
pub const SURFACE_OVERLAY: (f32, f32, f32, f32) = (0.0, 0.0, 1.0, 0.95);
/// Processing / loading state tint.
pub const SURFACE_PROCESSING: (f32, f32, f32, f32) = (0.583, 0.700, 0.960, 1.0);
/// Code-block background — slightly warm grey.
pub const SURFACE_CODE_BLOCK: (f32, f32, f32, f32) = (0.0, 0.0, 0.949, 1.0);

// ─── Border levels ────────────────────────────────────────────────────────────

/// Primary divider / hairline border.
pub const BORDER_PRIMARY: (f32, f32, f32, f32) = (0.0, 0.0, 0.878, 1.0);
/// Secondary lighter separation line.
pub const BORDER_SECONDARY: (f32, f32, f32, f32) = (0.0, 0.0, 0.918, 1.0);
/// Hover-state border brightening.
pub const BORDER_HOVER: (f32, f32, f32, f32) = (0.597, 0.600, 0.800, 1.0);
/// Focus ring border for accessibility.
pub const BORDER_FOCUS: (f32, f32, f32, f32) = (0.597, 0.832, 0.582, 1.0);
/// Divider between logical sections (same as border primary, kept distinct for semantics).
pub const BORDER_DIVIDER: (f32, f32, f32, f32) = (0.0, 0.0, 0.898, 1.0);

// ─── Semantic colours ────────────────────────────────────────────────────────

/// Positive / success green.
pub const SEMANTIC_SUCCESS: (f32, f32, f32, f32) = (0.361, 0.690, 0.502, 1.0);
/// Warning amber.
pub const SEMANTIC_WARNING: (f32, f32, f32, f32) = (0.097, 0.850, 0.600, 1.0);
/// Error / destructive red.
pub const SEMANTIC_ERROR: (f32, f32, f32, f32) = (0.0, 0.832, 0.582, 1.0);
/// Informational blue.
pub const SEMANTIC_INFO: (f32, f32, f32, f32) = (0.597, 0.700, 0.600, 1.0);
/// Positive sentiment / upvote tint.
pub const SEMANTIC_POSITIVE: (f32, f32, f32, f32) = (0.361, 0.600, 0.600, 1.0);
/// Negative sentiment / downvote tint.
pub const SEMANTIC_NEGATIVE: (f32, f32, f32, f32) = (0.0, 0.700, 0.600, 1.0);

// ─── Semantic surface backgrounds ────────────────────────────────────────────

/// Success-state background tint.
pub const SURFACE_SUCCESS: (f32, f32, f32, f32) = (0.361, 0.600, 0.941, 1.0);
/// Warning-state background tint.
pub const SURFACE_WARNING: (f32, f32, f32, f32) = (0.097, 0.850, 0.961, 1.0);
/// Error-state background tint.
pub const SURFACE_ERROR: (f32, f32, f32, f32) = (0.0, 0.832, 0.961, 1.0);
/// Info-state background tint.
pub const SURFACE_INFO_BG: (f32, f32, f32, f32) = (0.597, 0.700, 0.961, 1.0);

// ─── Brand accent hues ────────────────────────────────────────────────────────

/// Tag: vivid blue accent.
pub const ACCENT_BLUE: (f32, f32, f32, f32) = (0.597, 0.800, 0.600, 1.0);
/// Tag: teal / cyan accent.
pub const ACCENT_TEAL: (f32, f32, f32, f32) = (0.500, 0.700, 0.600, 1.0);
/// Tag: green accent.
pub const ACCENT_GREEN: (f32, f32, f32, f32) = (0.361, 0.700, 0.580, 1.0);
/// Tag: amber / orange accent.
pub const ACCENT_AMBER: (f32, f32, f32, f32) = (0.083, 0.850, 0.580, 1.0);
/// Tag: rose / red accent.
pub const ACCENT_ROSE: (f32, f32, f32, f32) = (0.0, 0.760, 0.600, 1.0);
/// Tag: violet / purple accent.
pub const ACCENT_VIOLET: (f32, f32, f32, f32) = (0.736, 0.700, 0.600, 1.0);

// ─── Selection tones ─────────────────────────────────────────────────────────

/// Selection highlight background (text selection, node selection).
pub const SELECTION_BG: (f32, f32, f32, f32) = (0.597, 0.832, 0.800, 0.3);
/// Selection outline ring.
pub const SELECTION_RING: (f32, f32, f32, f32) = (0.597, 0.832, 0.582, 1.0);
/// Hover preview selection tint.
pub const SELECTION_HOVER: (f32, f32, f32, f32) = (0.597, 0.600, 0.900, 0.15);
/// Active pressed state during a drag-select.
pub const SELECTION_ACTIVE: (f32, f32, f32, f32) = (0.597, 0.832, 0.700, 0.4);

// ─── Hover / active state modifiers ─────────────────────────────────────────

/// Default hover fill tint (laid over any surface).
pub const STATE_HOVER: (f32, f32, f32, f32) = (0.0, 0.0, 0.0, 0.04);
/// Pressed / active fill tint.
pub const STATE_ACTIVE: (f32, f32, f32, f32) = (0.0, 0.0, 0.0, 0.08);
/// Focused keyboard-navigation highlight.
pub const STATE_FOCUS: (f32, f32, f32, f32) = (0.597, 0.832, 0.582, 0.2);
/// Selected-row fill tint.
pub const STATE_SELECTED: (f32, f32, f32, f32) = (0.597, 0.700, 0.900, 0.12);

// ─── Shadow depths (HSLA approximations for box-shadow colours) ──────────────

/// Level 0 — no visible shadow (transparent).
pub const SHADOW_0: (f32, f32, f32, f32) = (0.0, 0.0, 0.0, 0.0);
/// Level 1 — card lift shadow.
pub const SHADOW_1: (f32, f32, f32, f32) = (0.583, 0.120, 0.420, 0.08);
/// Level 2 — popover / dropdown shadow.
pub const SHADOW_2: (f32, f32, f32, f32) = (0.583, 0.150, 0.380, 0.12);
/// Level 3 — modal / dialog shadow.
pub const SHADOW_3: (f32, f32, f32, f32) = (0.583, 0.180, 0.340, 0.18);
/// Level 4 — tooltip above shadow.
pub const SHADOW_4: (f32, f32, f32, f32) = (0.583, 0.200, 0.300, 0.22);
/// Float button shadow.
pub const SHADOW_FLOAT: (f32, f32, f32, f32) = (0.583, 0.220, 0.260, 0.28);

// ─── Spacing multiples (pixels, represented as f32) ──────────────────────────

/// 4 px — base spacing atom.
pub const SPACING_1: f32 = 4.0;
/// 8 px.
pub const SPACING_2: f32 = 8.0;
/// 12 px.
pub const SPACING_3: f32 = 12.0;
/// 16 px.
pub const SPACING_4: f32 = 16.0;
/// 20 px.
pub const SPACING_5: f32 = 20.0;
/// 24 px.
pub const SPACING_6: f32 = 24.0;
/// 32 px.
pub const SPACING_8: f32 = 32.0;
/// 48 px.
pub const SPACING_12: f32 = 48.0;

// ─── Font sizes (pixels, f32) ─────────────────────────────────────────────────

/// xs — 12 px.
pub const FONT_SIZE_XS: f32 = 12.0;
/// sm — 14 px.
pub const FONT_SIZE_SM: f32 = 14.0;
/// base — 15 px (AFFiNE default body size).
pub const FONT_SIZE_BASE: f32 = 15.0;
/// lg — 18 px.
pub const FONT_SIZE_LG: f32 = 18.0;
/// xl — 24 px (section headings).
pub const FONT_SIZE_XL: f32 = 24.0;

// ─── Font weights (CSS numeric, stored as f32) ───────────────────────────────

/// Light weight — 300.
pub const FONT_WEIGHT_LIGHT: f32 = 300.0;
/// Regular body weight — 400.
pub const FONT_WEIGHT_REGULAR: f32 = 400.0;
/// Medium / label weight — 500.
pub const FONT_WEIGHT_MEDIUM: f32 = 500.0;
/// Bold / heading weight — 600.
pub const FONT_WEIGHT_BOLD: f32 = 600.0;

// ─── Corner radii (pixels, f32) ───────────────────────────────────────────────

/// Extra-small radius — 2 px (hairline chips).
pub const RADIUS_XS: f32 = 2.0;
/// Small radius — 4 px (buttons, inputs).
pub const RADIUS_SM: f32 = 4.0;
/// Medium radius — 8 px (cards, panels).
pub const RADIUS_MD: f32 = 8.0;
/// Large radius — 16 px (modals, dialogs).
pub const RADIUS_LG: f32 = 16.0;

// ─── Dark-mode overrides ─────────────────────────────────────────────────────
// Provide dark equivalents for the most frequently mode-switched tokens.
// The `mode.rs` layer uses these to populate `ThemeTokens`.

/// Dark mode — primary text (near-white).
pub const DARK_TEXT_PRIMARY: (f32, f32, f32, f32) = (0.0, 0.0, 0.898, 1.0);
/// Dark mode — secondary text.
pub const DARK_TEXT_SECONDARY: (f32, f32, f32, f32) = (0.0, 0.0, 0.600, 1.0);
/// Dark mode — page background.
pub const DARK_SURFACE_BACKGROUND: (f32, f32, f32, f32) = (0.0, 0.0, 0.110, 1.0);
/// Dark mode — primary panel background.
pub const DARK_SURFACE_PRIMARY: (f32, f32, f32, f32) = (0.0, 0.0, 0.137, 1.0);
/// Dark mode — secondary surface.
pub const DARK_SURFACE_SECONDARY: (f32, f32, f32, f32) = (0.0, 0.0, 0.157, 1.0);
/// Dark mode — primary border.
pub const DARK_BORDER_PRIMARY: (f32, f32, f32, f32) = (0.0, 0.0, 0.220, 1.0);
/// Dark mode — brand primary (unchanged hue, slightly brighter).
pub const DARK_PRIMARY_BRAND: (f32, f32, f32, f32) = (0.597, 0.832, 0.620, 1.0);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spacing_values_are_multiples_of_four() {
        let spacings = [
            SPACING_1, SPACING_2, SPACING_3, SPACING_4, SPACING_5, SPACING_6, SPACING_8,
            SPACING_12,
        ];
        for s in spacings {
            assert_eq!(s % 4.0, 0.0, "spacing {s} is not a multiple of 4");
        }
    }

    #[test]
    fn corner_radii_are_in_ascending_order() {
        assert!(RADIUS_XS < RADIUS_SM);
        assert!(RADIUS_SM < RADIUS_MD);
        assert!(RADIUS_MD < RADIUS_LG);
    }

    #[test]
    fn font_weights_are_in_ascending_order() {
        assert!(FONT_WEIGHT_LIGHT < FONT_WEIGHT_REGULAR);
        assert!(FONT_WEIGHT_REGULAR < FONT_WEIGHT_MEDIUM);
        assert!(FONT_WEIGHT_MEDIUM < FONT_WEIGHT_BOLD);
    }

    #[test]
    fn font_sizes_are_in_ascending_order() {
        assert!(FONT_SIZE_XS < FONT_SIZE_SM);
        assert!(FONT_SIZE_SM < FONT_SIZE_BASE);
        assert!(FONT_SIZE_BASE < FONT_SIZE_LG);
        assert!(FONT_SIZE_LG < FONT_SIZE_XL);
    }

    #[test]
    fn color_tokens_alpha_in_range() {
        let tokens: &[(f32, f32, f32, f32)] = &[
            PRIMARY_BRAND,
            TEXT_PRIMARY,
            TEXT_DISABLED,
            SURFACE_BACKGROUND,
            BORDER_PRIMARY,
            SEMANTIC_SUCCESS,
            SEMANTIC_ERROR,
            ACCENT_BLUE,
            SELECTION_BG,
            STATE_HOVER,
            SHADOW_1,
        ];
        for &(h, s, l, a) in tokens {
            assert!((0.0..=1.0).contains(&h), "hue {h} out of range");
            assert!((0.0..=1.0).contains(&s), "saturation {s} out of range");
            assert!((0.0..=1.0).contains(&l), "lightness {l} out of range");
            assert!((0.0..=1.0).contains(&a), "alpha {a} out of range");
        }
    }

    #[test]
    fn dark_background_darker_than_light() {
        // Dark surface background lightness < light surface background lightness
        assert!(DARK_SURFACE_BACKGROUND.2 < SURFACE_BACKGROUND.2);
        assert!(DARK_SURFACE_PRIMARY.2 < SURFACE_PRIMARY.2);
    }

    #[test]
    fn selection_bg_is_semi_transparent() {
        assert!(SELECTION_BG.3 < 1.0, "selection background should be semi-transparent");
    }
}
