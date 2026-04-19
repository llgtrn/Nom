//! WebGPU renderer variant — compiled only when wasm feature is enabled.
//! Provides the same surface as the wgpu 0.19 renderer but targets WebGPU API.

use crate::frosted_pass::{FrostedPassConfig, FrostedPassState, FrostedRenderPass};

/// Configuration for a WebGPU rendering context.
#[derive(Debug)]
pub struct WebGpuConfig {
    /// HTML canvas element id to attach the renderer to.
    pub canvas_id: String,
    /// Viewport width in pixels.
    pub width: u32,
    /// Viewport height in pixels.
    pub height: u32,
    /// GPU power preference hint.
    pub power_preference: WebGpuPowerPreference,
}

/// Power preference passed to the GPU adapter request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WebGpuPowerPreference {
    /// Let the platform choose.
    Default,
    /// Prefer integrated / low-power GPU.
    LowPower,
    /// Prefer discrete / high-performance GPU.
    HighPerformance,
}

impl WebGpuConfig {
    /// Create a new config with default power preference.
    pub fn new(canvas_id: &str, width: u32, height: u32) -> Self {
        Self {
            canvas_id: canvas_id.to_string(),
            width,
            height,
            power_preference: WebGpuPowerPreference::Default,
        }
    }

    /// Override the power preference.
    pub fn with_power(mut self, pref: WebGpuPowerPreference) -> Self {
        self.power_preference = pref;
        self
    }

    /// Returns `true` when all required fields are non-empty / non-zero.
    pub fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0 && !self.canvas_id.is_empty()
    }
}

/// Stub WebGPU renderer; real GPU calls are filled in during WASM integration.
#[derive(Debug)]
pub struct WebGpuRenderer {
    /// Active configuration snapshot.
    pub config: WebGpuConfig,
    /// Whether `initialize` has been called successfully.
    pub initialized: bool,
    /// Monotonically increasing frame counter, incremented by `begin_frame`.
    pub frame_count: u64,
    /// Optional frosted-glass render pass; ticked once per frame when `Active`.
    pub frosted_pass: Option<FrostedRenderPass>,
}

impl WebGpuRenderer {
    /// Construct a renderer from `config`. Not yet initialised.
    pub fn new(config: WebGpuConfig) -> Self {
        Self {
            config,
            initialized: false,
            frame_count: 0,
            frosted_pass: None,
        }
    }

    /// Attach a frosted-glass render pass. The pass starts in `Disabled` state;
    /// call [`FrostedRenderPass::enable`] and [`FrostedRenderPass::activate`] to
    /// progress it before the first frame.
    pub fn enable_frosted_pass(mut self, config: FrostedPassConfig) -> Self {
        self.frosted_pass = Some(FrostedRenderPass::new(config));
        self
    }

    /// Perform one-time initialisation. Sets `initialized = true`.
    pub fn initialize(&mut self) -> Result<(), String> {
        self.initialized = true;
        Ok(())
    }

    /// Mark the beginning of a new frame; increments `frame_count` and ticks the
    /// frosted-glass pass when it is in the `Active` state.
    pub fn begin_frame(&mut self) {
        self.frame_count += 1;
        if let Some(pass) = self.frosted_pass.take() {
            if pass.state == FrostedPassState::Active {
                self.frosted_pass = Some(pass.tick());
            } else {
                self.frosted_pass = Some(pass);
            }
        }
    }

    /// Mark the end of the current frame. No-op stub.
    pub fn end_frame(&mut self) {}

    /// Update the viewport dimensions.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_config_defaults() {
        let cfg = WebGpuConfig::new("canvas-main", 800, 600);
        assert_eq!(cfg.canvas_id, "canvas-main");
        assert_eq!(cfg.width, 800);
        assert_eq!(cfg.height, 600);
        assert_eq!(cfg.power_preference, WebGpuPowerPreference::Default);
    }

    #[test]
    fn with_power_overrides_preference() {
        let cfg =
            WebGpuConfig::new("c", 100, 100).with_power(WebGpuPowerPreference::HighPerformance);
        assert_eq!(cfg.power_preference, WebGpuPowerPreference::HighPerformance);
    }

    #[test]
    fn is_valid_rejects_empty_canvas_id() {
        let cfg = WebGpuConfig::new("", 800, 600);
        assert!(!cfg.is_valid(), "empty canvas_id must be invalid");
    }

    #[test]
    fn is_valid_rejects_zero_dimensions() {
        let zero_w = WebGpuConfig::new("c", 0, 600);
        assert!(!zero_w.is_valid(), "zero width must be invalid");
        let zero_h = WebGpuConfig::new("c", 800, 0);
        assert!(!zero_h.is_valid(), "zero height must be invalid");
    }

    #[test]
    fn new_renderer_is_not_initialized() {
        let cfg = WebGpuConfig::new("canvas", 1280, 720);
        let r = WebGpuRenderer::new(cfg);
        assert!(!r.initialized);
        assert_eq!(r.frame_count, 0);
    }

    #[test]
    fn initialize_sets_flag() {
        let cfg = WebGpuConfig::new("canvas", 1280, 720);
        let mut r = WebGpuRenderer::new(cfg);
        r.initialize().expect("initialize must succeed");
        assert!(r.initialized);
    }

    #[test]
    fn begin_frame_increments_frame_count() {
        let cfg = WebGpuConfig::new("canvas", 640, 480);
        let mut r = WebGpuRenderer::new(cfg);
        r.begin_frame();
        assert_eq!(r.frame_count, 1);
        r.begin_frame();
        assert_eq!(r.frame_count, 2);
    }

    #[test]
    fn renderer_frosted_pass_disabled_by_default() {
        let cfg = WebGpuConfig::new("canvas", 800, 600);
        let r = WebGpuRenderer::new(cfg);
        assert!(
            r.frosted_pass.is_none(),
            "frosted_pass must be None by default"
        );
    }

    #[test]
    fn renderer_enable_frosted_pass() {
        let cfg = WebGpuConfig::new("canvas", 800, 600);
        let r = WebGpuRenderer::new(cfg).enable_frosted_pass(FrostedPassConfig::new(8.0, 0.75));
        assert!(
            r.frosted_pass.is_some(),
            "frosted_pass must be Some after enable_frosted_pass"
        );
        let pass = r.frosted_pass.as_ref().unwrap();
        assert_eq!(
            pass.state,
            FrostedPassState::Disabled,
            "pass starts Disabled until activated"
        );
        assert_eq!(pass.frame_count, 0);
    }

    #[test]
    fn renderer_frosted_pass_ticks_on_frame() {
        
        let cfg = WebGpuConfig::new("canvas", 800, 600);
        let pass = FrostedRenderPass::new(FrostedPassConfig::new(8.0, 0.75))
            .enable()
            .activate();
        let mut r = WebGpuRenderer::new(cfg);
        r.frosted_pass = Some(pass);

        r.begin_frame();
        r.begin_frame();
        r.begin_frame();

        let pass = r.frosted_pass.as_ref().unwrap();
        assert_eq!(
            pass.frame_count, 3,
            "frosted pass must tick once per begin_frame while Active"
        );
        assert_eq!(r.frame_count, 3, "renderer frame_count must also be 3");
    }

    #[test]
    fn renderer_frosted_pass_inactive_no_tick() {
        let cfg = WebGpuConfig::new("canvas", 800, 600);
        let r = WebGpuRenderer::new(cfg).enable_frosted_pass(FrostedPassConfig::new(8.0, 0.75));
        // Pass is Disabled (not yet activated); ticking the renderer must not increment pass frame_count.
        let mut r = r;
        r.begin_frame();
        r.begin_frame();

        let pass = r.frosted_pass.as_ref().unwrap();
        assert_eq!(
            pass.frame_count, 0,
            "Disabled pass must not accumulate frame_count"
        );
        assert_eq!(
            r.frame_count, 2,
            "renderer frame_count must advance regardless"
        );
    }
}
