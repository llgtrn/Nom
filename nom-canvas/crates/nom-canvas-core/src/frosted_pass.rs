//! Frosted-glass render pass — two-pass blur for panel backgrounds.
//! Pass 1: Downsample scene to 1/4 resolution.
//! Pass 2: Gaussian blur + tint + composite.

/// Configuration parameters for the frosted-glass render pass.
#[derive(Debug, Clone)]
pub struct FrostedPassConfig {
    /// Gaussian blur radius in pixels applied during pass 2.
    pub blur_radius: f32,
    /// Opacity of the blurred background layer (0.0 – 1.0).
    pub background_opacity: f32,
    /// Opacity of the border overlay (0.0 – 1.0).
    pub border_opacity: f32,
    /// Tint colour red channel (0 – 255).
    pub tint_r: u8,
    /// Tint colour green channel (0 – 255).
    pub tint_g: u8,
    /// Tint colour blue channel (0 – 255).
    pub tint_b: u8,
    /// Tint colour alpha channel (0 – 255).
    pub tint_a: u8,
    /// Downsampling factor for pass 1 (render at 1/factor resolution). Default 4.
    pub downsample_factor: u32,
}

impl FrostedPassConfig {
    /// Create a new config with black tint and default downsample factor of 4.
    pub fn new(blur_radius: f32, bg_opacity: f32) -> Self {
        Self {
            blur_radius,
            background_opacity: bg_opacity,
            border_opacity: 0.0,
            tint_r: 0,
            tint_g: 0,
            tint_b: 0,
            tint_a: 255,
            downsample_factor: 4,
        }
    }

    /// Override the tint colour.
    pub fn with_tint(mut self, r: u8, g: u8, b: u8, a: u8) -> Self {
        self.tint_r = r;
        self.tint_g = g;
        self.tint_b = b;
        self.tint_a = a;
        self
    }

    /// Override the downsample factor, clamped to the range 1 – 8.
    pub fn with_downsample(mut self, factor: u32) -> Self {
        self.downsample_factor = factor.clamp(1, 8);
        self
    }

    /// Compute the downsampled texture size, with each dimension at least 1.
    pub fn downsampled_size(&self, width: u32, height: u32) -> (u32, u32) {
        let factor = self.downsample_factor.max(1);
        ((width / factor).max(1), (height / factor).max(1))
    }

    /// Returns `true` when the background is effectively fully transparent.
    pub fn is_transparent(&self) -> bool {
        self.background_opacity < 0.01
    }
}

/// Lifecycle state of a [`FrostedRenderPass`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrostedPassState {
    /// Pass is inactive and will not execute.
    Disabled,
    /// Pass has been scheduled but GPU resources are not yet allocated.
    Pending,
    /// Pass is fully initialised and executing each frame.
    Active,
    /// Pass encountered an error and cannot proceed.
    Error,
}

/// A frosted-glass render pass instance.
#[derive(Debug)]
pub struct FrostedRenderPass {
    /// Configuration snapshot for this pass.
    pub config: FrostedPassConfig,
    /// Current lifecycle state.
    pub state: FrostedPassState,
    /// Number of frames rendered while in the `Active` state.
    pub frame_count: u64,
}

impl FrostedRenderPass {
    /// Create a new render pass from `config`. Initial state is `Disabled`.
    pub fn new(config: FrostedPassConfig) -> Self {
        Self {
            config,
            state: FrostedPassState::Disabled,
            frame_count: 0,
        }
    }

    /// Transition to `Pending` state.
    pub fn enable(mut self) -> Self {
        self.state = FrostedPassState::Pending;
        self
    }

    /// Transition to `Active` state.
    pub fn activate(mut self) -> Self {
        self.state = FrostedPassState::Active;
        self
    }

    /// Transition to `Disabled` state.
    pub fn disable(mut self) -> Self {
        self.state = FrostedPassState::Disabled;
        self
    }

    /// If `Active`, increment `frame_count` by one; otherwise no-op.
    pub fn tick(mut self) -> Self {
        if self.state == FrostedPassState::Active {
            self.frame_count += 1;
        }
        self
    }

    /// Returns `true` when the pass is `Active`.
    pub fn is_rendering(&self) -> bool {
        self.state == FrostedPassState::Active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frosted_config_new() {
        let cfg = FrostedPassConfig::new(12.0, 0.85);
        assert_eq!(cfg.blur_radius, 12.0);
        assert_eq!(cfg.background_opacity, 0.85);
        assert_eq!(cfg.border_opacity, 0.0);
        assert_eq!(cfg.tint_r, 0);
        assert_eq!(cfg.tint_g, 0);
        assert_eq!(cfg.tint_b, 0);
        assert_eq!(cfg.tint_a, 255);
        assert_eq!(cfg.downsample_factor, 4);
    }

    #[test]
    fn frosted_config_with_tint_and_is_transparent() {
        let cfg = FrostedPassConfig::new(8.0, 0.0).with_tint(30, 60, 90, 200);
        assert_eq!(cfg.tint_r, 30);
        assert_eq!(cfg.tint_g, 60);
        assert_eq!(cfg.tint_b, 90);
        assert_eq!(cfg.tint_a, 200);
        assert!(cfg.is_transparent(), "background_opacity 0.0 must be transparent");

        let opaque = FrostedPassConfig::new(8.0, 0.5);
        assert!(!opaque.is_transparent(), "background_opacity 0.5 must not be transparent");
    }

    #[test]
    fn frosted_config_downsampled_size() {
        let cfg = FrostedPassConfig::new(12.0, 0.85);
        let (w, h) = cfg.downsampled_size(1920, 1080);
        assert_eq!(w, 480, "1920 / 4 = 480");
        assert_eq!(h, 270, "1080 / 4 = 270");
    }

    #[test]
    fn frosted_config_downsample_clamped() {
        let cfg = FrostedPassConfig::new(12.0, 0.85).with_downsample(100);
        assert_eq!(cfg.downsample_factor, 8, "factor > 8 must be clamped to 8");

        let low = FrostedPassConfig::new(12.0, 0.85).with_downsample(0);
        assert_eq!(low.downsample_factor, 1, "factor 0 must be clamped to 1");
    }

    #[test]
    fn frosted_pass_new_enable_activate() {
        let cfg = FrostedPassConfig::new(12.0, 0.85);
        let pass = FrostedRenderPass::new(cfg);
        assert_eq!(pass.state, FrostedPassState::Disabled);
        assert_eq!(pass.frame_count, 0);

        let pass = pass.enable();
        assert_eq!(pass.state, FrostedPassState::Pending);

        let pass = pass.activate();
        assert_eq!(pass.state, FrostedPassState::Active);
        assert!(pass.is_rendering());
    }

    #[test]
    fn frosted_pass_tick_increments_frame_count() {
        let cfg = FrostedPassConfig::new(12.0, 0.85);
        let pass = FrostedRenderPass::new(cfg).enable().activate();
        let pass = pass.tick().tick().tick();
        assert_eq!(pass.frame_count, 3, "three ticks while Active must yield frame_count = 3");

        // Ticking while not Active must not increment.
        let cfg2 = FrostedPassConfig::new(12.0, 0.85);
        let pending = FrostedRenderPass::new(cfg2).enable();
        let pending = pending.tick();
        assert_eq!(pending.frame_count, 0, "tick while Pending must not increment frame_count");
    }

    #[test]
    fn frosted_pass_disable() {
        let cfg = FrostedPassConfig::new(12.0, 0.85);
        let pass = FrostedRenderPass::new(cfg).enable().activate().disable();
        assert_eq!(pass.state, FrostedPassState::Disabled);
        assert!(!pass.is_rendering());
    }
}
