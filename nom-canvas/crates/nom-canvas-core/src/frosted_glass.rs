//! Frosted-glass pipeline: blur layers, tinted glass effects, and layer compositing.

/// Defines blur parameters for a single render region.
#[derive(Debug, Clone, PartialEq)]
pub struct BlurLayer {
    /// Blur radius in pixels.
    pub radius: f32,
    /// Gaussian sigma derived from radius.
    pub sigma: f32,
    /// Quality level 1–8 (higher = more samples).
    pub quality: u8,
}

impl BlurLayer {
    /// Constructs a new `BlurLayer` with `sigma = radius / 3.0` and `quality = 4`.
    pub fn new(radius: f32) -> Self {
        Self {
            radius,
            sigma: radius / 3.0,
            quality: 4,
        }
    }

    /// Overrides quality level, clamped to `1..=8`.
    pub fn with_quality(mut self, q: u8) -> Self {
        self.quality = q.clamp(1, 8);
        self
    }

    /// Returns `true` when quality is 6 or above.
    pub fn is_high_quality(&self) -> bool {
        self.quality >= 6
    }

    /// Returns the effective sigma scaled by the quality factor.
    ///
    /// `effective_sigma = sigma * quality / 4.0`
    pub fn effective_sigma(&self) -> f32 {
        self.sigma * self.quality as f32 / 4.0
    }
}

/// Combines a `BlurLayer` with a tint colour and overall opacity to produce a
/// frosted-glass visual effect.
#[derive(Debug, Clone, PartialEq)]
pub struct FrostedGlassEffect {
    /// The underlying blur configuration.
    pub blur: BlurLayer,
    /// Red channel of the tint (0.0–1.0).
    pub tint_r: f32,
    /// Green channel of the tint (0.0–1.0).
    pub tint_g: f32,
    /// Blue channel of the tint (0.0–1.0).
    pub tint_b: f32,
    /// Overall layer opacity (0.0–1.0).
    pub opacity: f32,
}

impl FrostedGlassEffect {
    /// Constructs a default frosted-glass effect: white tint (0.9, 0.9, 0.9), opacity 0.7.
    pub fn new(blur: BlurLayer) -> Self {
        Self {
            blur,
            tint_r: 0.9,
            tint_g: 0.9,
            tint_b: 0.9,
            opacity: 0.7,
        }
    }

    /// Overrides the tint colour.
    pub fn with_tint(mut self, r: f32, g: f32, b: f32) -> Self {
        self.tint_r = r;
        self.tint_g = g;
        self.tint_b = b;
        self
    }

    /// Overrides the opacity, clamped to `0.0..=1.0`.
    pub fn with_opacity(mut self, o: f32) -> Self {
        self.opacity = o.clamp(0.0, 1.0);
        self
    }

    /// Returns `true` when opacity is greater than zero.
    pub fn is_visible(&self) -> bool {
        self.opacity > 0.0
    }
}

/// Blend mode for compositing a `FrostedGlassEffect` layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerBlend {
    /// Porter-Duff over (standard alpha compositing).
    Normal,
    /// Multiply blend mode.
    Multiply,
    /// Screen blend mode.
    Screen,
}

impl LayerBlend {
    /// Returns a human-readable name for the blend mode.
    pub fn blend_name(&self) -> &str {
        match self {
            LayerBlend::Normal => "normal",
            LayerBlend::Multiply => "multiply",
            LayerBlend::Screen => "screen",
        }
    }
}

/// Composites an ordered stack of `FrostedGlassEffect` layers.
#[derive(Debug, Default)]
pub struct LayerCompositor {
    layers: Vec<(FrostedGlassEffect, LayerBlend)>,
}

impl LayerCompositor {
    /// Creates an empty compositor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a layer to the top of the stack.
    pub fn push_layer(&mut self, effect: FrostedGlassEffect, blend: LayerBlend) {
        self.layers.push((effect, blend));
    }

    /// Returns the total number of layers.
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Returns the number of layers whose `is_visible()` returns `true`.
    pub fn visible_layers(&self) -> usize {
        self.layers.iter().filter(|(e, _)| e.is_visible()).count()
    }

    /// Returns the product of all layer opacities, or `1.0` if there are no layers.
    pub fn flatten_opacity(&self) -> f32 {
        self.layers
            .iter()
            .map(|(e, _)| e.opacity)
            .fold(1.0, |acc, o| acc * o)
    }
}

#[cfg(test)]
mod frosted_glass_tests {
    use super::*;

    #[test]
    fn sigma_from_radius() {
        let layer = BlurLayer::new(12.0);
        assert_eq!(layer.radius, 12.0);
        assert!((layer.sigma - 4.0).abs() < f32::EPSILON, "sigma should be radius/3 = 4.0");
    }

    #[test]
    fn with_quality_clamp() {
        let low = BlurLayer::new(6.0).with_quality(0); // below min → 1
        assert_eq!(low.quality, 1);
        let high = BlurLayer::new(6.0).with_quality(255); // above max → 8
        assert_eq!(high.quality, 8);
        let mid = BlurLayer::new(6.0).with_quality(5);
        assert_eq!(mid.quality, 5);
    }

    #[test]
    fn is_high_quality() {
        assert!(!BlurLayer::new(6.0).with_quality(5).is_high_quality());
        assert!(BlurLayer::new(6.0).with_quality(6).is_high_quality());
        assert!(BlurLayer::new(6.0).with_quality(8).is_high_quality());
    }

    #[test]
    fn effective_sigma() {
        // radius=12, sigma=4, quality=8 → 4 * 8/4 = 8.0
        let layer = BlurLayer::new(12.0).with_quality(8);
        assert!((layer.effective_sigma() - 8.0).abs() < f32::EPSILON);
        // quality=4 (default) → 4 * 4/4 = 4.0
        let default_layer = BlurLayer::new(12.0);
        assert!((default_layer.effective_sigma() - 4.0).abs() < f32::EPSILON);
    }

    #[test]
    fn frosted_glass_effect_is_visible() {
        let effect = FrostedGlassEffect::new(BlurLayer::new(8.0));
        assert!(effect.is_visible(), "default opacity 0.7 should be visible");
        let invisible = effect.with_opacity(0.0);
        assert!(!invisible.is_visible(), "opacity 0.0 should not be visible");
    }

    #[test]
    fn frosted_glass_with_opacity_clamp() {
        let over = FrostedGlassEffect::new(BlurLayer::new(4.0)).with_opacity(2.5);
        assert_eq!(over.opacity, 1.0, "opacity clamped to 1.0");
        let under = FrostedGlassEffect::new(BlurLayer::new(4.0)).with_opacity(-0.5);
        assert_eq!(under.opacity, 0.0, "opacity clamped to 0.0");
    }

    #[test]
    fn layer_compositor_push_and_count() {
        let mut comp = LayerCompositor::new();
        assert_eq!(comp.layer_count(), 0);
        comp.push_layer(
            FrostedGlassEffect::new(BlurLayer::new(4.0)),
            LayerBlend::Normal,
        );
        comp.push_layer(
            FrostedGlassEffect::new(BlurLayer::new(8.0)),
            LayerBlend::Multiply,
        );
        assert_eq!(comp.layer_count(), 2);
    }

    #[test]
    fn layer_compositor_visible_layers() {
        let mut comp = LayerCompositor::new();
        comp.push_layer(
            FrostedGlassEffect::new(BlurLayer::new(4.0)).with_opacity(0.0),
            LayerBlend::Normal,
        );
        comp.push_layer(
            FrostedGlassEffect::new(BlurLayer::new(4.0)).with_opacity(0.5),
            LayerBlend::Screen,
        );
        comp.push_layer(
            FrostedGlassEffect::new(BlurLayer::new(4.0)).with_opacity(1.0),
            LayerBlend::Normal,
        );
        assert_eq!(comp.visible_layers(), 2, "two layers with opacity > 0");
    }

    #[test]
    fn layer_compositor_flatten_opacity() {
        // Empty compositor → 1.0
        let empty = LayerCompositor::new();
        assert_eq!(empty.flatten_opacity(), 1.0);

        let mut comp = LayerCompositor::new();
        comp.push_layer(
            FrostedGlassEffect::new(BlurLayer::new(4.0)).with_opacity(0.5),
            LayerBlend::Normal,
        );
        comp.push_layer(
            FrostedGlassEffect::new(BlurLayer::new(4.0)).with_opacity(0.4),
            LayerBlend::Normal,
        );
        // 0.5 * 0.4 = 0.2
        assert!((comp.flatten_opacity() - 0.2).abs() < 1e-6);
    }
}
