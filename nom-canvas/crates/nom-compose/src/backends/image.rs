//! Image composition backend (data-only stub).
//!
//! Selects between on-device diffusion and cloud dispatch, and supports
//! tile-based upscaling.  Actual model inference + tile scheduling lives
//! in runtime crates; this module is pure data + validation.
#![deny(unsafe_code)]

use crate::backend_trait::{
    CompositionBackend, ComposeError, ComposeOutput, ComposeSpec, InterruptFlag, ProgressSink,
};
use crate::kind::NomKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Webp,
    Avif,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InferenceLocation {
    OnDevice,
    Cloud,
    Auto,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImageSpec {
    pub prompt: String,
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
    pub location: InferenceLocation,
    pub seed: Option<u64>,
    pub steps: u32,
    pub cfg_scale: f32,
    /// Optional upscale factor (1, 2, 4).  Above 1 triggers tile-based upscale.
    pub upscale_factor: u32,
}

impl ImageSpec {
    pub fn new(prompt: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            prompt: prompt.into(),
            width,
            height,
            format: ImageFormat::Png,
            location: InferenceLocation::Auto,
            seed: None,
            steps: 30,
            cfg_scale: 7.5,
            upscale_factor: 1,
        }
    }

    pub fn with_format(mut self, format: ImageFormat) -> Self {
        self.format = format;
        self
    }

    pub fn with_location(mut self, loc: InferenceLocation) -> Self {
        self.location = loc;
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    pub fn with_upscale(mut self, factor: u32) -> Self {
        self.upscale_factor = factor;
        self
    }

    pub fn final_dimensions(&self) -> (u32, u32) {
        (self.width * self.upscale_factor, self.height * self.upscale_factor)
    }

    /// Number of 512x512 tiles needed for tile-upscale.
    pub fn tile_count_512(&self) -> u32 {
        if self.upscale_factor <= 1 {
            return 1;
        }
        let (w, h) = self.final_dimensions();
        let wt = (w + 511) / 512;
        let ht = (h + 511) / 512;
        wt * ht
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error("prompt must not be empty")]
    EmptyPrompt,
    #[error("dimensions must be > 0")]
    InvalidDimensions,
    #[error("upscale factor must be 1, 2, or 4; got {0}")]
    InvalidUpscale(u32),
    #[error("steps must be in 1..=150; got {0}")]
    InvalidSteps(u32),
}

pub fn validate(spec: &ImageSpec) -> Result<(), ImageError> {
    if spec.prompt.trim().is_empty() {
        return Err(ImageError::EmptyPrompt);
    }
    if spec.width == 0 || spec.height == 0 {
        return Err(ImageError::InvalidDimensions);
    }
    if !matches!(spec.upscale_factor, 1 | 2 | 4) {
        return Err(ImageError::InvalidUpscale(spec.upscale_factor));
    }
    if spec.steps == 0 || spec.steps > 150 {
        return Err(ImageError::InvalidSteps(spec.steps));
    }
    Ok(())
}

pub struct StubImageBackend;

impl CompositionBackend for StubImageBackend {
    fn kind(&self) -> NomKind {
        NomKind::MediaImage
    }

    fn name(&self) -> &str {
        "stub-image"
    }

    fn compose(
        &self,
        _spec: &ComposeSpec,
        _progress: &dyn ProgressSink,
        _interrupt: &InterruptFlag,
    ) -> Result<ComposeOutput, ComposeError> {
        Ok(ComposeOutput {
            bytes: Vec::new(),
            mime_type: "image/png".to_string(),
            cost_cents: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_defaults() {
        let s = ImageSpec::new("a cat", 512, 512);
        assert_eq!(s.format, ImageFormat::Png);
        assert_eq!(s.location, InferenceLocation::Auto);
        assert_eq!(s.seed, None);
        assert_eq!(s.steps, 30);
        assert!((s.cfg_scale - 7.5).abs() < f32::EPSILON);
        assert_eq!(s.upscale_factor, 1);
    }

    #[test]
    fn builder_chain() {
        let s = ImageSpec::new("sky", 256, 256)
            .with_format(ImageFormat::Jpeg)
            .with_location(InferenceLocation::Cloud)
            .with_seed(42)
            .with_upscale(2);
        assert_eq!(s.format, ImageFormat::Jpeg);
        assert_eq!(s.location, InferenceLocation::Cloud);
        assert_eq!(s.seed, Some(42));
        assert_eq!(s.upscale_factor, 2);
    }

    #[test]
    fn final_dimensions_upscale2() {
        let s = ImageSpec::new("x", 100, 200).with_upscale(2);
        assert_eq!(s.final_dimensions(), (200, 400));
    }

    #[test]
    fn final_dimensions_upscale1() {
        let s = ImageSpec::new("x", 100, 200);
        assert_eq!(s.final_dimensions(), (100, 200));
    }

    #[test]
    fn tile_count_512_no_upscale() {
        let s = ImageSpec::new("x", 512, 512);
        assert_eq!(s.tile_count_512(), 1);
    }

    #[test]
    fn tile_count_512_1024x1024_upscale2() {
        // final = 2048x2048; 2048/512 = 4 tiles per axis → 16 total
        let s = ImageSpec::new("x", 1024, 1024).with_upscale(2);
        assert_eq!(s.tile_count_512(), 16);
    }

    #[test]
    fn validate_ok() {
        let s = ImageSpec::new("a dog", 512, 512);
        assert!(validate(&s).is_ok());
    }

    #[test]
    fn validate_empty_prompt() {
        let s = ImageSpec::new("", 512, 512);
        assert!(matches!(validate(&s), Err(ImageError::EmptyPrompt)));
    }

    #[test]
    fn validate_whitespace_prompt() {
        let s = ImageSpec::new("   ", 512, 512);
        assert!(matches!(validate(&s), Err(ImageError::EmptyPrompt)));
    }

    #[test]
    fn validate_zero_dimension() {
        let s = ImageSpec::new("cat", 0, 512);
        assert!(matches!(validate(&s), Err(ImageError::InvalidDimensions)));
    }

    #[test]
    fn validate_invalid_upscale() {
        let s = ImageSpec::new("cat", 512, 512).with_upscale(3);
        assert!(matches!(validate(&s), Err(ImageError::InvalidUpscale(3))));
    }

    #[test]
    fn validate_steps_zero() {
        let mut s = ImageSpec::new("cat", 512, 512);
        s.steps = 0;
        assert!(matches!(validate(&s), Err(ImageError::InvalidSteps(0))));
    }

    #[test]
    fn validate_steps_over_limit() {
        let mut s = ImageSpec::new("cat", 512, 512);
        s.steps = 200;
        assert!(matches!(validate(&s), Err(ImageError::InvalidSteps(200))));
    }

    #[test]
    fn stub_backend_kind_and_name() {
        let b = StubImageBackend;
        assert_eq!(b.kind(), NomKind::MediaImage);
        assert_eq!(b.name(), "stub-image");
    }
}
