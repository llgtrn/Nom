//! Video composition backend (data-only stub).
//!
//! Describes the spec + output contract for video generation.  Actual
//! frame rasterisation + container muxing lives in a separate runtime
//! crate; this module ships the schema + validate rules + a stub impl
//! of `CompositionBackend` for tests.
#![deny(unsafe_code)]

use crate::backend_trait::{
    CompositionBackend, ComposeError, ComposeOutput, ComposeSpec, InterruptFlag, ProgressSink,
};
use crate::kind::NomKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameFormat {
    Png,
    Rgba8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoCodec {
    H264,
    H265,
    Vp9,
    Av1,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VideoSpec {
    pub fps: u16,
    pub duration_frames: u32,
    pub width: u32,
    pub height: u32,
    pub frame_format: FrameFormat,
    pub codec: VideoCodec,
    pub bitrate_kbps: Option<u32>,
}

impl VideoSpec {
    pub fn new(fps: u16, duration_frames: u32, width: u32, height: u32) -> Self {
        Self {
            fps,
            duration_frames,
            width,
            height,
            frame_format: FrameFormat::Png,
            codec: VideoCodec::H264,
            bitrate_kbps: None,
        }
    }

    pub fn with_codec(mut self, codec: VideoCodec) -> Self {
        self.codec = codec;
        self
    }

    pub fn with_bitrate(mut self, kbps: u32) -> Self {
        self.bitrate_kbps = Some(kbps);
        self
    }

    pub fn aspect_ratio(&self) -> f32 {
        if self.height == 0 {
            1.0
        } else {
            self.width as f32 / self.height as f32
        }
    }

    pub fn duration_ms(&self) -> u64 {
        if self.fps == 0 {
            0
        } else {
            (self.duration_frames as u64 * 1000) / self.fps as u64
        }
    }

    pub fn total_pixels(&self) -> u64 {
        self.width as u64 * self.height as u64 * self.duration_frames as u64
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VideoError {
    #[error("fps must be > 0")]
    InvalidFps,
    #[error("dimensions must be > 0")]
    InvalidDimensions,
    #[error("duration must be > 0")]
    InvalidDuration,
}

pub fn validate(spec: &VideoSpec) -> Result<(), VideoError> {
    if spec.fps == 0 {
        return Err(VideoError::InvalidFps);
    }
    if spec.width == 0 || spec.height == 0 {
        return Err(VideoError::InvalidDimensions);
    }
    if spec.duration_frames == 0 {
        return Err(VideoError::InvalidDuration);
    }
    Ok(())
}

/// Stub backend that emits an empty byte vector + metadata.  Real impl
/// spawns an ffmpeg process and pipes per-frame rasterisation via a
/// bounded `mpsc::channel`.
pub struct StubVideoBackend;

impl CompositionBackend for StubVideoBackend {
    fn kind(&self) -> NomKind {
        NomKind::MediaVideo
    }

    fn name(&self) -> &str {
        "stub-video"
    }

    fn compose(
        &self,
        _spec: &ComposeSpec,
        _progress: &dyn ProgressSink,
        _interrupt: &InterruptFlag,
    ) -> Result<ComposeOutput, ComposeError> {
        Ok(ComposeOutput {
            bytes: Vec::new(),
            mime_type: "video/mp4".to_string(),
            cost_cents: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend_trait::{ComposeSpec, InterruptFlag};
    use crate::kind::NomKind;

    struct NoopProgress;
    impl ProgressSink for NoopProgress {
        fn notify(&self, _percent: u32, _message: &str) {}
    }

    #[test]
    fn new_defaults() {
        let s = VideoSpec::new(30, 90, 1920, 1080);
        assert_eq!(s.frame_format, FrameFormat::Png);
        assert_eq!(s.codec, VideoCodec::H264);
        assert!(s.bitrate_kbps.is_none());
    }

    #[test]
    fn with_codec_chains() {
        let s = VideoSpec::new(30, 90, 1920, 1080).with_codec(VideoCodec::Av1);
        assert_eq!(s.codec, VideoCodec::Av1);
    }

    #[test]
    fn with_bitrate_chains() {
        let s = VideoSpec::new(30, 90, 1920, 1080).with_bitrate(4000);
        assert_eq!(s.bitrate_kbps, Some(4000));
    }

    #[test]
    fn aspect_ratio_1920x1080() {
        let s = VideoSpec::new(30, 90, 1920, 1080);
        let ratio = s.aspect_ratio();
        assert!((ratio - 1.7777_f32).abs() < 0.001, "got {ratio}");
    }

    #[test]
    fn aspect_ratio_zero_height_returns_one() {
        let s = VideoSpec::new(30, 90, 1920, 0);
        assert_eq!(s.aspect_ratio(), 1.0);
    }

    #[test]
    fn duration_ms_30fps_90frames() {
        let s = VideoSpec::new(30, 90, 1920, 1080);
        assert_eq!(s.duration_ms(), 3000);
    }

    #[test]
    fn duration_ms_zero_fps_returns_zero() {
        let s = VideoSpec::new(0, 90, 1920, 1080);
        assert_eq!(s.duration_ms(), 0);
    }

    #[test]
    fn total_pixels_arithmetic() {
        let s = VideoSpec::new(30, 10, 4, 3);
        assert_eq!(s.total_pixels(), 4 * 3 * 10);
    }

    #[test]
    fn validate_ok_for_valid_spec() {
        let s = VideoSpec::new(30, 90, 1920, 1080);
        assert!(validate(&s).is_ok());
    }

    #[test]
    fn validate_rejects_fps_zero() {
        let s = VideoSpec::new(0, 90, 1920, 1080);
        assert!(matches!(validate(&s), Err(VideoError::InvalidFps)));
    }

    #[test]
    fn validate_rejects_zero_width() {
        let s = VideoSpec::new(30, 90, 0, 1080);
        assert!(matches!(validate(&s), Err(VideoError::InvalidDimensions)));
    }

    #[test]
    fn validate_rejects_zero_duration() {
        let s = VideoSpec::new(30, 0, 1920, 1080);
        assert!(matches!(validate(&s), Err(VideoError::InvalidDuration)));
    }

    #[test]
    fn stub_backend_kind() {
        assert_eq!(StubVideoBackend.kind(), NomKind::MediaVideo);
    }

    #[test]
    fn stub_backend_name() {
        assert_eq!(StubVideoBackend.name(), "stub-video");
    }

    #[test]
    fn stub_backend_compose_returns_empty_mp4() {
        let spec = ComposeSpec {
            kind: NomKind::MediaVideo,
            params: vec![],
        };
        let out = StubVideoBackend
            .compose(&spec, &NoopProgress, &InterruptFlag::new())
            .unwrap();
        assert!(out.bytes.is_empty());
        assert_eq!(out.mime_type, "video/mp4");
    }
}
