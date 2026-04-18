//! Frosted-glass pipeline coordinator: pass inputs, outputs, and aggregate statistics.

/// Configuration for a single frosted-glass pipeline layer.
#[derive(Debug, Clone, PartialEq)]
pub struct FrostedLayerConfig {
    /// Blur radius in pixels.
    pub blur_radius: f32,
    /// Tint alpha (0.0–1.0).
    pub tint_alpha: f32,
    /// Saturation boost factor.
    pub saturation_boost: f32,
}

impl FrostedLayerConfig {
    /// Returns `true` when `blur_radius` exceeds 20.0.
    pub fn is_heavy(&self) -> bool {
        self.blur_radius > 20.0
    }

    /// Returns `tint_alpha` clamped to `0.0..=1.0`.
    pub fn clamped_tint(&self) -> f32 {
        self.tint_alpha.clamp(0.0, 1.0)
    }
}

/// Input descriptor for one frosted-glass pipeline pass.
#[derive(Debug, Clone)]
pub struct FrostedPassInput {
    /// Layer identifier.
    pub layer_id: u32,
    /// Render target width in pixels.
    pub width: u32,
    /// Render target height in pixels.
    pub height: u32,
    /// Layer configuration.
    pub config: FrostedLayerConfig,
}

impl FrostedPassInput {
    /// Returns the total pixel count as `width as u64 * height as u64`.
    pub fn pixel_count(&self) -> u64 {
        self.width as u64 * self.height as u64
    }

    /// Returns `true` when `width > 0`, `height > 0`, and `blur_radius > 0.0`.
    pub fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0 && self.config.blur_radius > 0.0
    }
}

/// Output record produced after executing one frosted-glass pipeline pass.
#[derive(Debug, Clone)]
pub struct FrostedPassOutput {
    /// Layer identifier matching the corresponding `FrostedPassInput`.
    pub layer_id: u32,
    /// Number of blur passes executed.
    pub blur_passes: u32,
    /// Elapsed time in nanoseconds.
    pub elapsed_ns: u64,
}

impl FrostedPassOutput {
    /// Returns passes per millisecond. Returns `0.0` when `elapsed_ns == 0`.
    pub fn passes_per_ms(&self) -> f64 {
        if self.elapsed_ns == 0 {
            return 0.0;
        }
        self.blur_passes as f64 / (self.elapsed_ns as f64 / 1_000_000.0)
    }
}

/// Coordinates execution of multiple frosted-glass pipeline passes.
#[derive(Debug, Default)]
pub struct FrostedPipelineRunner {
    /// Registered pass inputs.
    pub inputs: Vec<FrostedPassInput>,
}

impl FrostedPipelineRunner {
    /// Creates an empty runner.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a pass input to the runner.
    pub fn add_input(&mut self, i: FrostedPassInput) {
        self.inputs.push(i);
    }

    /// Returns references to all inputs that satisfy `is_valid()`.
    pub fn valid_inputs(&self) -> Vec<&FrostedPassInput> {
        self.inputs.iter().filter(|i| i.is_valid()).collect()
    }

    /// Executes all valid inputs and returns one `FrostedPassOutput` per valid input.
    ///
    /// `blur_passes = ceil(config.blur_radius / 5.0) as u32`
    /// `elapsed_ns  = pixel_count * 10`
    pub fn run_all(&self) -> Vec<FrostedPassOutput> {
        self.valid_inputs()
            .into_iter()
            .map(|input| {
                let blur_passes = (input.config.blur_radius / 5.0).ceil() as u32;
                let elapsed_ns = input.pixel_count() * 10;
                FrostedPassOutput {
                    layer_id: input.layer_id,
                    blur_passes,
                    elapsed_ns,
                }
            })
            .collect()
    }
}

/// Aggregate statistics for a completed pipeline run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineStats {
    /// Sum of `blur_passes` across all outputs.
    pub total_passes: u32,
    /// Sum of `pixel_count` across all inputs.
    pub total_pixels: u64,
}

impl PipelineStats {
    /// Computes aggregate statistics from a set of pipeline inputs and outputs.
    pub fn from_outputs(inputs: &[FrostedPassInput], outputs: &[FrostedPassOutput]) -> PipelineStats {
        let total_passes = outputs.iter().map(|o| o.blur_passes).sum();
        let total_pixels = inputs.iter().map(|i| i.pixel_count()).sum();
        PipelineStats { total_passes, total_pixels }
    }
}

#[cfg(test)]
mod frosted_pipeline_tests {
    use super::*;

    fn make_config(blur_radius: f32, tint_alpha: f32) -> FrostedLayerConfig {
        FrostedLayerConfig { blur_radius, tint_alpha, saturation_boost: 1.0 }
    }

    fn make_input(layer_id: u32, width: u32, height: u32, blur_radius: f32) -> FrostedPassInput {
        FrostedPassInput {
            layer_id,
            width,
            height,
            config: make_config(blur_radius, 0.5),
        }
    }

    #[test]
    fn config_is_heavy_true() {
        let cfg = make_config(25.0, 0.5);
        assert!(cfg.is_heavy(), "blur_radius 25.0 > 20.0 should be heavy");
    }

    #[test]
    fn config_is_heavy_false() {
        let cfg = make_config(20.0, 0.5);
        assert!(!cfg.is_heavy(), "blur_radius 20.0 is not > 20.0, should not be heavy");
    }

    #[test]
    fn config_clamped_tint_clamps() {
        let over = make_config(5.0, 1.8);
        assert_eq!(over.clamped_tint(), 1.0, "tint_alpha 1.8 should clamp to 1.0");

        let under = make_config(5.0, -0.3);
        assert_eq!(under.clamped_tint(), 0.0, "tint_alpha -0.3 should clamp to 0.0");

        let mid = make_config(5.0, 0.6);
        assert!((mid.clamped_tint() - 0.6).abs() < f32::EPSILON, "tint_alpha 0.6 should pass through");
    }

    #[test]
    fn input_pixel_count() {
        let input = make_input(1, 1920, 1080, 8.0);
        assert_eq!(input.pixel_count(), 1920u64 * 1080);
    }

    #[test]
    fn input_is_valid_false_zero_width() {
        let input = make_input(2, 0, 1080, 8.0);
        assert!(!input.is_valid(), "zero width should make input invalid");
    }

    #[test]
    fn runner_valid_inputs_filter() {
        let mut runner = FrostedPipelineRunner::new();
        runner.add_input(make_input(1, 800, 600, 8.0));   // valid
        runner.add_input(make_input(2, 0, 600, 8.0));     // invalid: zero width
        runner.add_input(make_input(3, 800, 0, 8.0));     // invalid: zero height
        runner.add_input(make_input(4, 800, 600, 0.0));   // invalid: zero blur_radius
        runner.add_input(make_input(5, 1280, 720, 15.0)); // valid

        let valid = runner.valid_inputs();
        assert_eq!(valid.len(), 2, "only 2 of 5 inputs should be valid");
        assert_eq!(valid[0].layer_id, 1);
        assert_eq!(valid[1].layer_id, 5);
    }

    #[test]
    fn runner_run_all_pass_count_radius_10() {
        let mut runner = FrostedPipelineRunner::new();
        // blur_radius=10 → ceil(10/5)=2 passes
        runner.add_input(make_input(1, 100, 100, 10.0));
        let outputs = runner.run_all();
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].blur_passes, 2, "radius=10 should yield 2 passes");
    }

    #[test]
    fn pass_output_passes_per_ms() {
        let output = FrostedPassOutput { layer_id: 1, blur_passes: 4, elapsed_ns: 2_000_000 };
        // 4 passes / (2_000_000 ns / 1_000_000) = 4 / 2.0 = 2.0
        let ppm = output.passes_per_ms();
        assert!((ppm - 2.0).abs() < 1e-9, "expected 2.0 passes/ms, got {ppm}");

        let zero_elapsed = FrostedPassOutput { layer_id: 2, blur_passes: 3, elapsed_ns: 0 };
        assert_eq!(zero_elapsed.passes_per_ms(), 0.0, "zero elapsed_ns should return 0.0");
    }

    #[test]
    fn stats_total_passes_sum() {
        let inputs = vec![
            make_input(1, 100, 100, 10.0),
            make_input(2, 200, 200, 15.0),
        ];
        let outputs = vec![
            FrostedPassOutput { layer_id: 1, blur_passes: 2, elapsed_ns: 100 },
            FrostedPassOutput { layer_id: 2, blur_passes: 3, elapsed_ns: 200 },
        ];
        let stats = PipelineStats::from_outputs(&inputs, &outputs);
        assert_eq!(stats.total_passes, 5, "2+3=5 total passes");
    }

    #[test]
    fn stats_total_pixels_sum() {
        let inputs = vec![
            make_input(1, 100, 200, 8.0),   // 20_000 pixels
            make_input(2, 50, 50, 8.0),     // 2_500 pixels
        ];
        let outputs = vec![
            FrostedPassOutput { layer_id: 1, blur_passes: 2, elapsed_ns: 100 },
            FrostedPassOutput { layer_id: 2, blur_passes: 2, elapsed_ns: 50 },
        ];
        let stats = PipelineStats::from_outputs(&inputs, &outputs);
        assert_eq!(stats.total_pixels, 22_500, "20000+2500=22500 total pixels");
    }
}
