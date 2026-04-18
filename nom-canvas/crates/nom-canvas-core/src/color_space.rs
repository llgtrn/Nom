//! Color space primitives: models, RGB/HSL colors, conversion utilities, and palette.

/// Color model descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorModel {
    /// Standard sRGB (gamma-encoded).
    Srgb,
    /// Linear light RGB (no gamma).
    LinearRgb,
    /// Hue-Saturation-Lightness.
    Hsl,
    /// Oklab perceptual color space.
    Oklab,
}

impl ColorModel {
    /// Returns `true` for perceptual color models (Hsl, Oklab).
    pub fn is_perceptual(&self) -> bool {
        matches!(self, ColorModel::Hsl | ColorModel::Oklab)
    }

    /// Short lowercase identifier for this model.
    pub fn model_name(&self) -> &'static str {
        match self {
            ColorModel::Srgb => "srgb",
            ColorModel::LinearRgb => "linear_rgb",
            ColorModel::Hsl => "hsl",
            ColorModel::Oklab => "oklab",
        }
    }
}

/// RGBA color with components in [0.0, 1.0].
#[derive(Debug, Clone, PartialEq)]
pub struct RgbColor {
    /// Red channel.
    pub r: f32,
    /// Green channel.
    pub g: f32,
    /// Blue channel.
    pub b: f32,
    /// Alpha channel (1.0 = fully opaque).
    pub a: f32,
}

impl RgbColor {
    /// Returns a new color with all components clamped to [0.0, 1.0].
    pub fn clamp(&self) -> RgbColor {
        RgbColor {
            r: self.r.clamp(0.0, 1.0),
            g: self.g.clamp(0.0, 1.0),
            b: self.b.clamp(0.0, 1.0),
            a: self.a.clamp(0.0, 1.0),
        }
    }

    /// Converts each channel to an 8-bit integer (r, g, b, a).
    pub fn to_u8(&self) -> (u8, u8, u8, u8) {
        (
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8,
        )
    }

    /// Returns `true` if alpha >= 1.0.
    pub fn is_opaque(&self) -> bool {
        self.a >= 1.0
    }

    /// Returns a CSS hex string like `#rrggbb`.
    pub fn hex_string(&self) -> String {
        let (r, g, b, _) = self.to_u8();
        format!("#{:02x}{:02x}{:02x}", r, g, b)
    }
}

/// HSL color: hue [0, 360), saturation [0, 1], lightness [0, 1].
#[derive(Debug, Clone, PartialEq)]
pub struct HslColor {
    /// Hue in degrees (0 ..= 360).
    pub h: f32,
    /// Saturation (0 ..= 1).
    pub s: f32,
    /// Lightness (0 ..= 1).
    pub l: f32,
}

impl HslColor {
    /// Returns `true` if the color is achromatic (saturation < 0.01).
    pub fn is_achromatic(&self) -> bool {
        self.s < 0.01
    }

    /// Returns `true` if lightness < 0.5 (dark half of the scale).
    pub fn is_dark(&self) -> bool {
        self.l < 0.5
    }

    /// Human-readable label: `hsl(h,s%,l%)`.
    pub fn label(&self) -> String {
        format!("hsl({:.0},{:.0}%,{:.0}%)", self.h, self.s * 100.0, self.l * 100.0)
    }
}

/// Stateless color-space conversion helpers.
pub struct ColorConvert;

impl ColorConvert {
    /// Converts an sRGB channel value to linear light.
    pub fn srgb_to_linear(c: f32) -> f32 {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    /// Converts a linear light channel value to sRGB.
    pub fn linear_to_srgb(c: f32) -> f32 {
        if c <= 0.0031308 {
            c * 12.92
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        }
    }

    /// Computes relative luminance from sRGB channel values.
    ///
    /// Each channel is first linearised before applying the ITU-R BT.709 weights.
    pub fn rgb_luminance(r: f32, g: f32, b: f32) -> f32 {
        let lr = Self::srgb_to_linear(r);
        let lg = Self::srgb_to_linear(g);
        let lb = Self::srgb_to_linear(b);
        0.2126 * lr + 0.7152 * lg + 0.0722 * lb
    }
}

/// An ordered collection of [`RgbColor`] values associated with a [`ColorModel`].
pub struct ColorPalette {
    /// Colors in insertion order.
    pub colors: Vec<RgbColor>,
    /// Color model for this palette.
    pub model: ColorModel,
}

impl ColorPalette {
    /// Creates an empty palette for the given model.
    pub fn new(model: ColorModel) -> Self {
        ColorPalette { colors: Vec::new(), model }
    }

    /// Appends a color to the palette.
    pub fn add(&mut self, color: RgbColor) {
        self.colors.push(color);
    }

    /// Returns the number of fully opaque colors in the palette.
    pub fn opaque_count(&self) -> usize {
        self.colors.iter().filter(|c| c.is_opaque()).count()
    }

    /// Returns the average relative luminance across all colors (0.0 if empty).
    pub fn average_luminance(&self) -> f32 {
        if self.colors.is_empty() {
            return 0.0;
        }
        let sum: f32 = self
            .colors
            .iter()
            .map(|c| ColorConvert::rgb_luminance(c.r, c.g, c.b))
            .sum();
        sum / self.colors.len() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_model_is_perceptual() {
        assert!(!ColorModel::Srgb.is_perceptual());
        assert!(!ColorModel::LinearRgb.is_perceptual());
        assert!(ColorModel::Hsl.is_perceptual());
        assert!(ColorModel::Oklab.is_perceptual());
    }

    #[test]
    fn color_model_name() {
        assert_eq!(ColorModel::Srgb.model_name(), "srgb");
        assert_eq!(ColorModel::LinearRgb.model_name(), "linear_rgb");
        assert_eq!(ColorModel::Hsl.model_name(), "hsl");
        assert_eq!(ColorModel::Oklab.model_name(), "oklab");
    }

    #[test]
    fn rgb_color_clamp_bounds() {
        let c = RgbColor { r: -0.5, g: 1.5, b: 0.5, a: 2.0 };
        let clamped = c.clamp();
        assert_eq!(clamped.r, 0.0);
        assert_eq!(clamped.g, 1.0);
        assert_eq!(clamped.b, 0.5);
        assert_eq!(clamped.a, 1.0);
    }

    #[test]
    fn rgb_color_to_u8() {
        let c = RgbColor { r: 1.0, g: 0.5, b: 0.0, a: 1.0 };
        let (r, g, b, a) = c.to_u8();
        assert_eq!(r, 255);
        assert_eq!(g, 127);
        assert_eq!(b, 0);
        assert_eq!(a, 255);
    }

    #[test]
    fn rgb_color_is_opaque() {
        let opaque = RgbColor { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
        let translucent = RgbColor { r: 1.0, g: 1.0, b: 1.0, a: 0.5 };
        assert!(opaque.is_opaque());
        assert!(!translucent.is_opaque());
    }

    #[test]
    fn rgb_color_hex_string_format() {
        let c = RgbColor { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
        assert_eq!(c.hex_string(), "#ff0000");

        let black = RgbColor { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
        assert_eq!(black.hex_string(), "#000000");

        let white = RgbColor { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
        assert_eq!(white.hex_string(), "#ffffff");
    }

    #[test]
    fn hsl_color_is_achromatic() {
        let grey = HslColor { h: 0.0, s: 0.005, l: 0.5 };
        assert!(grey.is_achromatic());

        let vivid = HslColor { h: 120.0, s: 0.8, l: 0.5 };
        assert!(!vivid.is_achromatic());
    }

    #[test]
    fn hsl_color_is_dark() {
        let dark = HslColor { h: 0.0, s: 0.5, l: 0.3 };
        assert!(dark.is_dark());

        let light = HslColor { h: 0.0, s: 0.5, l: 0.7 };
        assert!(!light.is_dark());
    }

    #[test]
    fn color_palette_opaque_count() {
        let mut palette = ColorPalette::new(ColorModel::Srgb);
        palette.add(RgbColor { r: 1.0, g: 0.0, b: 0.0, a: 1.0 });
        palette.add(RgbColor { r: 0.0, g: 1.0, b: 0.0, a: 0.5 });
        palette.add(RgbColor { r: 0.0, g: 0.0, b: 1.0, a: 1.0 });
        assert_eq!(palette.opaque_count(), 2);
    }
}
