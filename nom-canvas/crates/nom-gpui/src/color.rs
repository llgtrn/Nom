//! RGBA and HSLA color types with alpha compositing.

/// Linear-space RGBA color with components in [0, 1].
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn opaque(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Parse a 24-bit hex literal (#RRGGBB) with full alpha.
    pub const fn hex(rgb: u32) -> Self {
        let r = ((rgb >> 16) & 0xFF) as f32 / 255.0;
        let g = ((rgb >> 8) & 0xFF) as f32 / 255.0;
        let b = (rgb & 0xFF) as f32 / 255.0;
        Self { r, g, b, a: 1.0 }
    }

    /// Source-over alpha blend: result = src + dst * (1 - src.a).
    pub fn blend(self, below: Self) -> Self {
        let a = self.a + below.a * (1.0 - self.a);
        if a <= f32::EPSILON {
            return Self::TRANSPARENT;
        }
        let comp = |s: f32, b: f32| (s * self.a + b * below.a * (1.0 - self.a)) / a;
        Self {
            r: comp(self.r, below.r),
            g: comp(self.g, below.g),
            b: comp(self.b, below.b),
            a,
        }
    }

    /// Convert to sRGB-encoded u8 tuple (for CPU-side display only).
    pub fn to_u8(self) -> [u8; 4] {
        let s = |c: f32| (c.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        [s(self.r), s(self.g), s(self.b), s(self.a)]
    }
}

/// HSLA color (hue in degrees [0,360), saturation/lightness/alpha in [0,1]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsla {
    pub h: f32,
    pub s: f32,
    pub l: f32,
    pub a: f32,
}

impl Hsla {
    /// Create an HSLA color. **`h` is in degrees [0, 360)**; s/l/a are in [0, 1].
    ///
    /// This matches standard CSS `hsl()` convention. Use [`Hsla::from_normalized`]
    /// if you have a hue in [0, 1] (Zed/GPUI convention).
    pub const fn new(h: f32, s: f32, l: f32, a: f32) -> Self {
        Self { h, s, l, a }
    }

    /// Explicit degrees constructor. `h_deg` is in [0, 360); s/l/a in [0, 1].
    /// Identical to [`Hsla::new`] but signals intent at the call site.
    pub fn from_degrees(h_deg: f32, s: f32, l: f32, a: f32) -> Self {
        Self { h: h_deg, s, l, a }
    }

    /// Normalized-hue constructor. `h_01` is in [0, 1] (Zed/GPUI convention);
    /// it is multiplied by 360 internally before storage. s/l/a are in [0, 1].
    pub fn from_normalized(h_01: f32, s: f32, l: f32, a: f32) -> Self {
        Self {
            h: h_01 * 360.0,
            s,
            l,
            a,
        }
    }

    pub fn to_rgba(self) -> Rgba {
        let c = (1.0 - (2.0 * self.l - 1.0).abs()) * self.s;
        let h_prime = (self.h.rem_euclid(360.0)) / 60.0;
        let x = c * (1.0 - (h_prime.rem_euclid(2.0) - 1.0).abs());
        let (r1, g1, b1) = match h_prime as i32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let m = self.l - c / 2.0;
        Rgba {
            r: r1 + m,
            g: g1 + m,
            b: b1 + m,
            a: self.a,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_parses_rgb() {
        let c = Rgba::hex(0xFF8040);
        let [r, g, b, a] = c.to_u8();
        assert_eq!(r, 255);
        assert_eq!(g, 128);
        assert_eq!(b, 64);
        assert_eq!(a, 255);
    }

    #[test]
    fn blend_over_opaque_keeps_alpha_one() {
        let top = Rgba::new(1.0, 0.0, 0.0, 0.5);
        let bot = Rgba::BLACK;
        let out = top.blend(bot);
        assert!((out.a - 1.0).abs() < 1e-6);
        assert!((out.r - 0.5).abs() < 1e-6);
    }

    #[test]
    fn blend_transparent_over_anything_returns_below() {
        let out = Rgba::TRANSPARENT.blend(Rgba::WHITE);
        assert_eq!(out, Rgba::WHITE);
    }

    #[test]
    fn hsla_red_converts_to_rgba() {
        let red = Hsla::new(0.0, 1.0, 0.5, 1.0).to_rgba();
        assert!((red.r - 1.0).abs() < 1e-4);
        assert!(red.g < 1e-4);
        assert!(red.b < 1e-4);
    }

    #[test]
    fn from_normalized_matches_from_degrees() {
        let a = Hsla::from_normalized(0.5, 0.8, 0.4, 1.0).to_rgba();
        let b = Hsla::from_degrees(180.0, 0.8, 0.4, 1.0).to_rgba();
        assert!((a.r - b.r).abs() < 1e-5);
        assert!((a.g - b.g).abs() < 1e-5);
        assert!((a.b - b.b).abs() < 1e-5);
        assert!((a.a - b.a).abs() < 1e-5);
    }

    #[test]
    fn theme_style_normalized_hue_produces_correct_color() {
        // A theme using Zed-convention h=0.0833… (30°) should give an orange-ish tone.
        let c = Hsla::from_normalized(30.0 / 360.0, 1.0, 0.5, 1.0).to_rgba();
        // 30° HSL with s=1, l=0.5 → RGB (1.0, 0.5, 0.0)
        assert!((c.r - 1.0).abs() < 1e-4, "r={}", c.r);
        assert!((c.g - 0.5).abs() < 1e-4, "g={}", c.g);
        assert!(c.b < 1e-4, "b={}", c.b);
    }

    /// Hue 360° must produce the same color as hue 0° because `to_rgba` uses
    /// `rem_euclid(360.0)` — 360 wraps to 0 before the sector match.
    #[test]
    fn hsla_hue_wraps_at_360() {
        let a = Hsla::from_degrees(0.0, 1.0, 0.5, 1.0).to_rgba();
        let b = Hsla::from_degrees(360.0, 1.0, 0.5, 1.0).to_rgba();
        assert!(
            (a.r - b.r).abs() < 1e-4,
            "r mismatch: {:.6} vs {:.6}",
            a.r,
            b.r
        );
        assert!(
            (a.g - b.g).abs() < 1e-4,
            "g mismatch: {:.6} vs {:.6}",
            a.g,
            b.g
        );
        assert!(
            (a.b - b.b).abs() < 1e-4,
            "b mismatch: {:.6} vs {:.6}",
            a.b,
            b.b
        );
    }

    /// Saturation = 0 means the hue is irrelevant — all fully-desaturated colors
    /// are gray (r == g == b, modulated only by lightness).
    #[test]
    fn hsla_saturation_zero_produces_gray() {
        let gray = Hsla::from_degrees(180.0, 0.0, 0.5, 1.0).to_rgba();
        assert!(
            (gray.r - gray.g).abs() < 1e-4,
            "r={:.6} g={:.6} should be equal",
            gray.r,
            gray.g
        );
        assert!(
            (gray.g - gray.b).abs() < 1e-4,
            "g={:.6} b={:.6} should be equal",
            gray.g,
            gray.b
        );
    }

    /// Lightness 0 → black; lightness 1 → white (regardless of hue/saturation).
    #[test]
    fn hsla_lightness_extremes() {
        let black = Hsla::from_degrees(180.0, 1.0, 0.0, 1.0).to_rgba();
        assert!(
            black.r < 1e-4,
            "black.r should be ~0, got {:.6}",
            black.r
        );
        assert!(
            black.g < 1e-4,
            "black.g should be ~0, got {:.6}",
            black.g
        );
        assert!(
            black.b < 1e-4,
            "black.b should be ~0, got {:.6}",
            black.b
        );
        let white = Hsla::from_degrees(180.0, 1.0, 1.0, 1.0).to_rgba();
        assert!(
            (white.r - 1.0).abs() < 1e-4,
            "white.r should be ~1, got {:.6}",
            white.r
        );
        assert!(
            (white.g - 1.0).abs() < 1e-4,
            "white.g should be ~1, got {:.6}",
            white.g
        );
        assert!(
            (white.b - 1.0).abs() < 1e-4,
            "white.b should be ~1, got {:.6}",
            white.b
        );
    }
}
