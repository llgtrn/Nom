/// Configuration for one animation generation.
#[derive(Debug, Clone)]
pub struct AnimationConfig {
    pub prompt: String,
    pub negative_prompt: String,
    pub video_length: usize,     // number of frames
    pub height: u32,             // pixel height (multiple of 8)
    pub width: u32,              // pixel width (multiple of 8)
    pub num_inference_steps: u32,
    pub guidance_scale: f32,     // classifier-free guidance strength
    pub seed: Option<u64>,
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            negative_prompt: String::new(),
            video_length: 16,
            height: 512,
            width: 512,
            num_inference_steps: 25,
            guidance_scale: 7.5,
            seed: None,
        }
    }
}

impl AnimationConfig {
    pub fn latent_h(&self) -> u32 { self.height / 8 }  // VAE 8x downscale
    pub fn latent_w(&self) -> u32 { self.width / 8 }
    pub fn latent_channels(&self) -> u32 { 4 }         // standard SD latent channels
    pub fn latent_size(&self) -> usize {
        (self.latent_channels() * self.latent_h() * self.latent_w()) as usize * self.video_length
    }
}

/// Latent state for one denoising step.
#[derive(Debug, Clone)]
pub struct LatentState {
    pub data: Vec<f32>,          // flattened (channels, frames, h, w)
    pub timestep: u32,
    pub noise_pred_cond: Vec<f32>,
    pub noise_pred_uncond: Vec<f32>,
}

impl LatentState {
    pub fn new_zeros(size: usize, timestep: u32) -> Self {
        Self {
            data: vec![0.0f32; size],
            timestep,
            noise_pred_cond: vec![0.0f32; size],
            noise_pred_uncond: vec![0.0f32; size],
        }
    }

    pub fn new_noise(size: usize, timestep: u32, seed: u64) -> Self {
        // Deterministic LCG pseudo-noise (no external crate)
        let mut state = seed;
        let data: Vec<f32> = (0..size).map(|_| {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (state >> 33) as f32 / u32::MAX as f32 * 2.0 - 1.0
        }).collect();
        Self { data, timestep, noise_pred_cond: vec![0.0; size], noise_pred_uncond: vec![0.0; size] }
    }

    /// Apply classifier-free guidance: combine cond + uncond predictions.
    pub fn apply_guidance(&self, guidance_scale: f32) -> Vec<f32> {
        self.noise_pred_uncond.iter().zip(self.noise_pred_cond.iter())
            .map(|(u, c)| u + guidance_scale * (c - u))
            .collect()
    }
}

/// Temporal attention configuration for UNet3D.
#[derive(Debug, Clone)]
pub struct TemporalAttentionConfig {
    pub num_heads: u32,
    pub head_dim: u32,
    pub num_layers: u32,
    pub max_seq_len: u32,     // max frames for temporal attention
    pub use_position_encoding: bool,
}

impl Default for TemporalAttentionConfig {
    fn default() -> Self {
        Self { num_heads: 8, head_dim: 64, num_layers: 3, max_seq_len: 32, use_position_encoding: true }
    }
}

/// One frame in the output video.
#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub index: usize,
    pub data: Vec<f32>,     // HWC, values in [0, 1]
    pub width: u32,
    pub height: u32,
}

impl VideoFrame {
    pub fn new_blank(index: usize, width: u32, height: u32) -> Self {
        Self { index, data: vec![0.5f32; (width * height * 3) as usize], width, height }
    }

    pub fn pixel_count(&self) -> usize { (self.width * self.height) as usize }
}

/// Result of one animation generation.
#[derive(Debug, Clone)]
pub struct AnimationResult {
    pub frames: Vec<VideoFrame>,
    pub config: AnimationConfig,
    pub steps_taken: u32,
}

impl AnimationResult {
    pub fn frame_count(&self) -> usize { self.frames.len() }
    pub fn duration_at_fps(&self, fps: f32) -> f32 { self.frames.len() as f32 / fps }
}

/// AnimateDiff pipeline stub — models the denoising loop without tensor ops.
pub struct AnimationPipeline {
    pub temporal_config: TemporalAttentionConfig,
}

impl AnimationPipeline {
    pub fn new() -> Self { Self { temporal_config: TemporalAttentionConfig::default() } }

    /// Generate timestep schedule (linear from T to 0).
    pub fn timestep_schedule(&self, steps: u32, max_t: u32) -> Vec<u32> {
        (0..steps).rev().map(|i| (i as f32 / steps as f32 * max_t as f32) as u32).collect()
    }

    /// Run the full denoising loop (stub — returns synthetic frames).
    pub fn generate(&self, config: &AnimationConfig) -> AnimationResult {
        let latent_size = config.latent_size();
        let seed = config.seed.unwrap_or(42);
        let schedule = self.timestep_schedule(config.num_inference_steps, 1000);

        let mut latent = LatentState::new_noise(latent_size, schedule[0], seed);

        for &t in &schedule {
            latent.timestep = t;
            // Stub denoising: slightly reduce noise each step
            latent.data.iter_mut().for_each(|v| *v *= 0.95);
        }

        // VAE decode: latents → pixel frames (stub: map latent to [0,1] range)
        let frames: Vec<VideoFrame> = (0..config.video_length).map(|i| {
            let mut frame = VideoFrame::new_blank(i, config.width, config.height);
            // Stub decode: use first latent values
            let offset = i * (latent_size / config.video_length);
            for (j, px) in frame.data.iter_mut().enumerate() {
                let lv = latent.data.get(offset + j % 64).cloned().unwrap_or(0.0);
                *px = (lv * 0.5 + 0.5).clamp(0.0, 1.0);
            }
            frame
        }).collect();

        AnimationResult { frames, config: config.clone(), steps_taken: config.num_inference_steps }
    }
}

impl Default for AnimationPipeline {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod diffusion_tests {
    use super::*;

    #[test]
    fn test_animation_config_defaults() {
        let c = AnimationConfig::default();
        assert_eq!(c.video_length, 16);
        assert_eq!(c.height, 512);
        assert_eq!(c.guidance_scale, 7.5);
    }

    #[test]
    fn test_latent_dimensions() {
        let c = AnimationConfig::default();
        assert_eq!(c.latent_h(), 64);
        assert_eq!(c.latent_w(), 64);
        assert_eq!(c.latent_channels(), 4);
    }

    #[test]
    fn test_latent_state_noise_deterministic() {
        let s1 = LatentState::new_noise(100, 999, 42);
        let s2 = LatentState::new_noise(100, 999, 42);
        assert_eq!(s1.data, s2.data);
    }

    #[test]
    fn test_latent_state_noise_different_seeds() {
        let s1 = LatentState::new_noise(100, 999, 1);
        let s2 = LatentState::new_noise(100, 999, 2);
        assert_ne!(s1.data, s2.data);
    }

    #[test]
    fn test_apply_guidance() {
        let mut s = LatentState::new_zeros(4, 500);
        s.noise_pred_uncond = vec![1.0, 1.0, 1.0, 1.0];
        s.noise_pred_cond = vec![2.0, 2.0, 2.0, 2.0];
        let guided = s.apply_guidance(7.5);
        // u + 7.5*(c-u) = 1 + 7.5 = 8.5
        assert!((guided[0] - 8.5).abs() < 0.001);
    }

    #[test]
    fn test_timestep_schedule_length() {
        let p = AnimationPipeline::new();
        let sched = p.timestep_schedule(25, 1000);
        assert_eq!(sched.len(), 25);
    }

    #[test]
    fn test_generate_frame_count() {
        let p = AnimationPipeline::new();
        let config = AnimationConfig { video_length: 8, ..Default::default() };
        let result = p.generate(&config);
        assert_eq!(result.frame_count(), 8);
    }

    #[test]
    fn test_generate_pixel_range() {
        let p = AnimationPipeline::new();
        let config = AnimationConfig::default();
        let result = p.generate(&config);
        for frame in &result.frames {
            for &px in &frame.data {
                assert!((0.0..=1.0).contains(&px), "pixel out of range: {}", px);
            }
        }
    }

    #[test]
    fn test_animation_duration() {
        let p = AnimationPipeline::new();
        let config = AnimationConfig { video_length: 24, ..Default::default() };
        let result = p.generate(&config);
        let dur = result.duration_at_fps(24.0);
        assert!((dur - 1.0).abs() < 0.01);
    }
}
