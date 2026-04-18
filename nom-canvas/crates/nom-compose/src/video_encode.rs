/// Video codec variants for GPU-accelerated encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoCodec {
    H264,
    H265,
    Vp9,
    Av1,
}

impl VideoCodec {
    pub fn codec_name(&self) -> &str {
        match self {
            VideoCodec::H264 => "h264",
            VideoCodec::H265 => "h265",
            VideoCodec::Vp9 => "vp9",
            VideoCodec::Av1 => "av1",
        }
    }

    /// Returns true for modern open codecs (Vp9 or Av1).
    pub fn is_modern(&self) -> bool {
        matches!(self, VideoCodec::Vp9 | VideoCodec::Av1)
    }
}

/// A single video frame carrying metadata and a pixel stub.
#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub frame_index: u64,
    pub pixel_count: u64,
}

impl VideoFrame {
    pub fn new(width: u32, height: u32, frame_index: u64) -> Self {
        Self {
            width,
            height,
            frame_index,
            pixel_count: width as u64 * height as u64,
        }
    }

    pub fn resolution(&self) -> String {
        format!("{}x{}", self.width, self.height)
    }

    pub fn is_hd(&self) -> bool {
        self.width >= 1280
    }
}

/// Sequential frame encoder backed by a chosen codec.
pub struct VideoEncoder {
    pub codec: VideoCodec,
    pub fps: u32,
    encoded_frames: u64,
}

impl VideoEncoder {
    pub fn new(codec: VideoCodec, fps: u32) -> Self {
        Self {
            codec,
            fps,
            encoded_frames: 0,
        }
    }

    pub fn encode_frame(&mut self, _frame: &VideoFrame) {
        self.encoded_frames += 1;
    }

    pub fn encoded_count(&self) -> u64 {
        self.encoded_frames
    }

    /// Rough output-size estimate in megabytes for a given duration.
    pub fn estimated_output_mb(&self, duration_secs: f32) -> f32 {
        self.fps as f32 * duration_secs * 0.5 / 1024.0 * 8.0
    }
}

/// GPU-accelerated encoder that dispatches to multiple parallel streams.
pub struct GpuVideoEncoder {
    pub encoder: VideoEncoder,
    pub parallel_streams: u8,
}

impl GpuVideoEncoder {
    pub fn new(encoder: VideoEncoder, parallel_streams: u8) -> Self {
        Self {
            encoder,
            parallel_streams,
        }
    }

    pub fn encode_batch(&mut self, frames: &[VideoFrame]) {
        for frame in frames {
            self.encoder.encode_frame(frame);
        }
    }

    pub fn stream_count(&self) -> u8 {
        self.parallel_streams
    }

    pub fn is_parallel(&self) -> bool {
        self.parallel_streams > 1
    }
}

#[cfg(test)]
mod video_encode_tests {
    use super::*;

    #[test]
    fn codec_name_returns_correct_string() {
        assert_eq!(VideoCodec::H264.codec_name(), "h264");
        assert_eq!(VideoCodec::H265.codec_name(), "h265");
        assert_eq!(VideoCodec::Vp9.codec_name(), "vp9");
        assert_eq!(VideoCodec::Av1.codec_name(), "av1");
    }

    #[test]
    fn codec_is_modern_only_for_vp9_and_av1() {
        assert!(!VideoCodec::H264.is_modern());
        assert!(!VideoCodec::H265.is_modern());
        assert!(VideoCodec::Vp9.is_modern());
        assert!(VideoCodec::Av1.is_modern());
    }

    #[test]
    fn video_frame_pixel_count_equals_width_times_height() {
        let frame = VideoFrame::new(1920, 1080, 0);
        assert_eq!(frame.pixel_count, 1920 * 1080);
    }

    #[test]
    fn video_frame_is_hd_threshold_at_1280() {
        let hd = VideoFrame::new(1280, 720, 0);
        let sd = VideoFrame::new(1279, 720, 0);
        assert!(hd.is_hd());
        assert!(!sd.is_hd());
    }

    #[test]
    fn video_frame_resolution_format() {
        let frame = VideoFrame::new(1920, 1080, 5);
        assert_eq!(frame.resolution(), "1920x1080");
    }

    #[test]
    fn video_encoder_encode_frame_increments_count() {
        let mut enc = VideoEncoder::new(VideoCodec::H264, 30);
        assert_eq!(enc.encoded_count(), 0);
        let f = VideoFrame::new(1280, 720, 0);
        enc.encode_frame(&f);
        enc.encode_frame(&f);
        assert_eq!(enc.encoded_count(), 2);
    }

    #[test]
    fn video_encoder_estimated_output_mb_is_nonzero() {
        let enc = VideoEncoder::new(VideoCodec::H265, 60);
        let mb = enc.estimated_output_mb(10.0);
        // 60 * 10 * 0.5 / 1024 * 8 ≈ 2.34 MB
        assert!(mb > 0.0, "estimated MB must be positive, got {mb}");
    }

    #[test]
    fn gpu_encoder_encode_batch_counts_all_frames() {
        let inner = VideoEncoder::new(VideoCodec::Av1, 24);
        let mut gpu = GpuVideoEncoder::new(inner, 4);
        let frames: Vec<VideoFrame> = (0..5).map(|i| VideoFrame::new(3840, 2160, i)).collect();
        gpu.encode_batch(&frames);
        assert_eq!(gpu.encoder.encoded_count(), 5);
    }

    #[test]
    fn gpu_encoder_is_parallel_requires_more_than_one_stream() {
        let single = GpuVideoEncoder::new(VideoEncoder::new(VideoCodec::H264, 30), 1);
        let multi = GpuVideoEncoder::new(VideoEncoder::new(VideoCodec::H264, 30), 2);
        assert!(!single.is_parallel());
        assert!(multi.is_parallel());
    }
}
