/// A single captured frame with raw RGBA pixel data.
pub struct FrameCapture {
    pub frame_number: u64,
    pub timestamp_ms: u64,
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl FrameCapture {
    pub fn new(frame_number: u64, timestamp_ms: u64, pixels: Vec<u8>, width: u32, height: u32) -> Self {
        Self { frame_number, timestamp_ms, pixels, width, height }
    }
}

/// Configuration for the FFmpeg encoding process.
pub struct FfmpegConfig {
    pub output_path: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate_kbps: u32,
}

impl FfmpegConfig {
    pub fn new(output_path: impl Into<String>, width: u32, height: u32, fps: u32, bitrate_kbps: u32) -> Self {
        Self {
            output_path: output_path.into(),
            width,
            height,
            fps,
            bitrate_kbps,
        }
    }
}

/// Buffers frames and builds the FFmpeg CLI argument list for encoding.
pub struct FfmpegEncoder {
    pub frames: Vec<FrameCapture>,
    config: FfmpegConfig,
}

impl FfmpegEncoder {
    pub fn new(config: FfmpegConfig) -> Self {
        Self { frames: Vec::new(), config }
    }

    /// Appends a frame to the internal buffer.
    pub fn add_frame(&mut self, frame: FrameCapture) {
        self.frames.push(frame);
    }

    /// Returns the FFmpeg CLI arguments that would encode all buffered frames.
    /// The caller is responsible for piping raw RGBA data on stdin.
    pub fn encode_command(&self) -> Vec<String> {
        let size = format!("{}x{}", self.config.width, self.config.height);
        let fps_str = self.config.fps.to_string();
        let bitrate = format!("{}k", self.config.bitrate_kbps);
        vec![
            "ffmpeg".to_string(),
            "-f".to_string(), "rawvideo".to_string(),
            "-vcodec".to_string(), "rawvideo".to_string(),
            "-s".to_string(), size,
            "-r".to_string(), fps_str,
            "-pix_fmt".to_string(), "rgba".to_string(),
            "-i".to_string(), "pipe:0".to_string(),
            "-vcodec".to_string(), "libx264".to_string(),
            "-b:v".to_string(), bitrate,
            self.config.output_path.clone(),
        ]
    }

    /// Returns the number of buffered frames.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Returns the total duration in milliseconds based on frame timestamps.
    /// Returns 0 when no frames are buffered.
    pub fn total_duration_ms(&self) -> u64 {
        self.frames.iter().map(|f| f.timestamp_ms).max().unwrap_or(0)
    }
}

/// Top-level pipeline that validates incoming frames and delegates to FfmpegEncoder.
pub struct VideoCapturePipeline {
    pub encoder: FfmpegEncoder,
    pub config: FfmpegConfig,
}

impl VideoCapturePipeline {
    pub fn new(config: FfmpegConfig) -> Self {
        let encoder_config = FfmpegConfig {
            output_path: config.output_path.clone(),
            width: config.width,
            height: config.height,
            fps: config.fps,
            bitrate_kbps: config.bitrate_kbps,
        };
        Self { encoder: FfmpegEncoder::new(encoder_config), config }
    }

    /// Validates that the frame dimensions match the pipeline config, then buffers it.
    pub fn capture(&mut self, frame: FrameCapture) {
        assert_eq!(
            frame.width, self.config.width,
            "frame width {} does not match config width {}",
            frame.width, self.config.width
        );
        assert_eq!(
            frame.height, self.config.height,
            "frame height {} does not match config height {}",
            frame.height, self.config.height
        );
        self.encoder.add_frame(frame);
    }

    /// Delegates to FfmpegEncoder::encode_command.
    pub fn build_command(&self) -> Vec<String> {
        self.encoder.encode_command()
    }

    /// Returns true once at least one frame has been buffered.
    pub fn is_ready_to_encode(&self) -> bool {
        self.encoder.frame_count() > 0
    }
}

#[cfg(test)]
mod video_capture_tests {
    use super::*;

    fn make_frame(frame_number: u64, timestamp_ms: u64, width: u32, height: u32) -> FrameCapture {
        let pixels = vec![0u8; (width * height * 4) as usize];
        FrameCapture::new(frame_number, timestamp_ms, pixels, width, height)
    }

    fn make_config() -> FfmpegConfig {
        FfmpegConfig::new("/tmp/out.mp4", 1280, 720, 30, 4000)
    }

    // Test 1: FrameCapture fields are stored correctly.
    #[test]
    fn frame_capture_fields() {
        let pixels = vec![255u8; 4];
        let frame = FrameCapture::new(7, 233, pixels.clone(), 1, 1);
        assert_eq!(frame.frame_number, 7);
        assert_eq!(frame.timestamp_ms, 233);
        assert_eq!(frame.width, 1);
        assert_eq!(frame.height, 1);
        assert_eq!(frame.pixels, pixels);
    }

    // Test 2: FfmpegConfig construction stores all fields.
    #[test]
    fn ffmpeg_config_construction() {
        let cfg = FfmpegConfig::new("/output/video.mp4", 1920, 1080, 60, 8000);
        assert_eq!(cfg.output_path, "/output/video.mp4");
        assert_eq!(cfg.width, 1920);
        assert_eq!(cfg.height, 1080);
        assert_eq!(cfg.fps, 60);
        assert_eq!(cfg.bitrate_kbps, 8000);
    }

    // Test 3: encode_command() has "ffmpeg" as the first element.
    #[test]
    fn encode_command_first_element_is_ffmpeg() {
        let encoder = FfmpegEncoder::new(make_config());
        let cmd = encoder.encode_command();
        assert!(!cmd.is_empty(), "command must not be empty");
        assert_eq!(cmd[0], "ffmpeg");
    }

    // Test 4: encode_command() contains the output path.
    #[test]
    fn encode_command_contains_output_path() {
        let encoder = FfmpegEncoder::new(make_config());
        let cmd = encoder.encode_command();
        assert!(
            cmd.contains(&"/tmp/out.mp4".to_string()),
            "command must contain output path, got: {:?}", cmd
        );
    }

    // Test 5: encode_command() contains the frame dimensions.
    #[test]
    fn encode_command_contains_dimensions() {
        let encoder = FfmpegEncoder::new(make_config());
        let cmd = encoder.encode_command();
        assert!(
            cmd.contains(&"1280x720".to_string()),
            "command must contain WxH, got: {:?}", cmd
        );
    }

    // Test 6: encode_command() contains the fps value.
    #[test]
    fn encode_command_contains_fps() {
        let encoder = FfmpegEncoder::new(make_config());
        let cmd = encoder.encode_command();
        let fps_idx = cmd.iter().position(|s| s == "-r");
        assert!(fps_idx.is_some(), "command must have -r flag");
        let fps_val = &cmd[fps_idx.unwrap() + 1];
        assert_eq!(fps_val, "30", "fps value must match config");
    }

    // Test 7: add_frame() increments frame_count.
    #[test]
    fn add_frame_increments_count() {
        let mut encoder = FfmpegEncoder::new(make_config());
        assert_eq!(encoder.frame_count(), 0);
        encoder.add_frame(make_frame(0, 0, 1280, 720));
        assert_eq!(encoder.frame_count(), 1);
        encoder.add_frame(make_frame(1, 33, 1280, 720));
        assert_eq!(encoder.frame_count(), 2);
    }

    // Test 8: total_duration_ms() returns the maximum timestamp across frames.
    #[test]
    fn total_duration_ms_calculation() {
        let mut encoder = FfmpegEncoder::new(make_config());
        assert_eq!(encoder.total_duration_ms(), 0);
        encoder.add_frame(make_frame(0, 0, 1280, 720));
        encoder.add_frame(make_frame(1, 33, 1280, 720));
        encoder.add_frame(make_frame(2, 66, 1280, 720));
        assert_eq!(encoder.total_duration_ms(), 66);
    }

    // Test 9: VideoCapturePipeline::is_ready_to_encode() returns true after capture.
    #[test]
    fn pipeline_is_ready_after_capture() {
        let cfg = FfmpegConfig::new("/tmp/out.mp4", 1280, 720, 30, 4000);
        let mut pipeline = VideoCapturePipeline::new(cfg);
        assert!(!pipeline.is_ready_to_encode());
        pipeline.capture(make_frame(0, 0, 1280, 720));
        assert!(pipeline.is_ready_to_encode());
    }
}
