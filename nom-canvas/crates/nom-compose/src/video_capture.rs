use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

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

/// Configuration for the video encoding process.
pub struct VideoEncoderConfig {
    pub output_path: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate_kbps: u32,
}

impl VideoEncoderConfig {
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

/// Run an external command with optional stdin data and a timeout.
/// Returns the captured stdout on success, or an error string on failure.
pub(crate) fn run_command_with_timeout(
    command: &str,
    args: &[String],
    stdin_data: Option<&[u8]>,
    timeout_secs: u64,
) -> Result<Vec<u8>, String> {
    let timeout = Duration::from_secs(timeout_secs);
    let mut cmd = Command::new(command);
    cmd.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if stdin_data.is_some() {
        cmd.stdin(Stdio::piped());
    }

    let mut child = cmd.spawn().map_err(|e| format!("{} spawn failed: {}", command, e))?;

    if let Some(data) = stdin_data {
        let mut stdin = child.stdin.take().unwrap();
        let data = data.to_vec();
        thread::spawn(move || {
            let _ = stdin.write_all(&data);
        });
    }

    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stdout = Vec::new();
                if let Some(mut out) = child.stdout.take() {
                    let _ = out.read_to_end(&mut stdout);
                }
                let mut stderr = Vec::new();
                if let Some(mut err) = child.stderr.take() {
                    let _ = err.read_to_end(&mut stderr);
                }
                if !status.success() {
                    return Err(format!(
                        "{} failed: {}",
                        command,
                        String::from_utf8_lossy(&stderr)
                    ));
                }
                return Ok(stdout);
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return Err(format!("{} timed out after {}s", command, timeout_secs));
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(format!("{} wait error: {}", command, e)),
        }
    }
}

/// Buffers frames and builds the external encoder CLI argument list for encoding.
pub struct ExternalEncoder {
    pub frames: Vec<FrameCapture>,
    config: VideoEncoderConfig,
}

impl ExternalEncoder {
    pub fn new(config: VideoEncoderConfig) -> Self {
        Self { frames: Vec::new(), config }
    }

    /// Appends a frame to the internal buffer.
    pub fn add_frame(&mut self, frame: FrameCapture) {
        self.frames.push(frame);
    }

    /// Returns the external encoder CLI arguments that would encode all buffered frames.
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

    /// Spawn ffmpeg to encode `input_path` to `output_path`.
    /// Uses a 30-second timeout by default.
    pub fn encode(&self, input_path: &str, output_path: &str) -> Result<(), String> {
        let args = vec![
            "-y".to_string(),
            "-i".to_string(), input_path.to_string(),
            "-c:v".to_string(), "libx264".to_string(),
            "-b:v".to_string(), format!("{}k", self.config.bitrate_kbps),
            output_path.to_string(),
        ];
        run_command_with_timeout("ffmpeg", &args, None, 30)?;
        Ok(())
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

/// Top-level pipeline that validates incoming frames and delegates to ExternalEncoder.
pub struct VideoCapturePipeline {
    pub encoder: ExternalEncoder,
    pub config: VideoEncoderConfig,
}

impl VideoCapturePipeline {
    pub fn new(config: VideoEncoderConfig) -> Self {
        let encoder_config = VideoEncoderConfig {
            output_path: config.output_path.clone(),
            width: config.width,
            height: config.height,
            fps: config.fps,
            bitrate_kbps: config.bitrate_kbps,
        };
        Self { encoder: ExternalEncoder::new(encoder_config), config }
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

    /// Delegates to ExternalEncoder::encode_command.
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

    fn make_config() -> VideoEncoderConfig {
        VideoEncoderConfig::new("/tmp/out.mp4", 1280, 720, 30, 4000)
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

    // Test 2: VideoEncoderConfig construction stores all fields.
    #[test]
    fn video_encoder_config_construction() {
        let cfg = VideoEncoderConfig::new("/output/video.mp4", 1920, 1080, 60, 8000);
        assert_eq!(cfg.output_path, "/output/video.mp4");
        assert_eq!(cfg.width, 1920);
        assert_eq!(cfg.height, 1080);
        assert_eq!(cfg.fps, 60);
        assert_eq!(cfg.bitrate_kbps, 8000);
    }

    // Test 3: encode_command() has "ffmpeg" as the first element.
    #[test]
    fn encode_command_first_element_is_ffmpeg() {
        let encoder = ExternalEncoder::new(make_config());
        let cmd = encoder.encode_command();
        assert!(!cmd.is_empty(), "command must not be empty");
        assert_eq!(cmd[0], "ffmpeg");
    }

    // Test 4: encode_command() contains the output path.
    #[test]
    fn encode_command_contains_output_path() {
        let encoder = ExternalEncoder::new(make_config());
        let cmd = encoder.encode_command();
        assert!(
            cmd.contains(&"/tmp/out.mp4".to_string()),
            "command must contain output path, got: {:?}", cmd
        );
    }

    // Test 5: encode_command() contains the frame dimensions.
    #[test]
    fn encode_command_contains_dimensions() {
        let encoder = ExternalEncoder::new(make_config());
        let cmd = encoder.encode_command();
        assert!(
            cmd.contains(&"1280x720".to_string()),
            "command must contain WxH, got: {:?}", cmd
        );
    }

    // Test 6: encode_command() contains the fps value.
    #[test]
    fn encode_command_contains_fps() {
        let encoder = ExternalEncoder::new(make_config());
        let cmd = encoder.encode_command();
        let fps_idx = cmd.iter().position(|s| s == "-r");
        assert!(fps_idx.is_some(), "command must have -r flag");
        let fps_val = &cmd[fps_idx.unwrap() + 1];
        assert_eq!(fps_val, "30", "fps value must match config");
    }

    // Test 7: add_frame() increments frame_count.
    #[test]
    fn add_frame_increments_count() {
        let mut encoder = ExternalEncoder::new(make_config());
        assert_eq!(encoder.frame_count(), 0);
        encoder.add_frame(make_frame(0, 0, 1280, 720));
        assert_eq!(encoder.frame_count(), 1);
        encoder.add_frame(make_frame(1, 33, 1280, 720));
        assert_eq!(encoder.frame_count(), 2);
    }

    // Test 8: total_duration_ms() returns the maximum timestamp across frames.
    #[test]
    fn total_duration_ms_calculation() {
        let mut encoder = ExternalEncoder::new(make_config());
        assert_eq!(encoder.total_duration_ms(), 0);
        encoder.add_frame(make_frame(0, 0, 1280, 720));
        encoder.add_frame(make_frame(1, 33, 1280, 720));
        encoder.add_frame(make_frame(2, 66, 1280, 720));
        assert_eq!(encoder.total_duration_ms(), 66);
    }

    // Test 9: VideoCapturePipeline::is_ready_to_encode() returns true after capture.
    #[test]
    fn pipeline_is_ready_after_capture() {
        let cfg = VideoEncoderConfig::new("/tmp/out.mp4", 1280, 720, 30, 4000);
        let mut pipeline = VideoCapturePipeline::new(cfg);
        assert!(!pipeline.is_ready_to_encode());
        pipeline.capture(make_frame(0, 0, 1280, 720));
        assert!(pipeline.is_ready_to_encode());
    }

    // Test 10: run_command_with_timeout reports spawn failure for missing binary.
    #[test]
    fn command_runner_spawn_failure() {
        let result = run_command_with_timeout("this_does_not_exist_ffmpeg", &[], None, 1);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("spawn failed"), "expected spawn failed, got: {}", err);
    }

    // Test 11: run_command_with_timeout kills process that exceeds timeout.
    #[test]
    fn command_runner_timeout() {
        // "cmd /c ping -n 6 127.0.0.1" takes ~5s; our timeout is 1s.
        let result = run_command_with_timeout(
            "cmd",
            &["/c".into(), "ping".into(), "-n".into(), "6".into(), "127.0.0.1".into()],
            None,
            1,
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("timed out"), "expected timeout, got: {}", err);
    }

    // Test 12: ExternalEncoder::encode() returns error when ffmpeg is missing.
    #[test]
    fn encoder_encode_returns_error_for_missing_ffmpeg() {
        let encoder = ExternalEncoder::new(make_config());
        let result = encoder.encode("/tmp/in.y4m", "/tmp/out.mp4");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("spawn failed") || err.contains("failed"), "expected ffmpeg error, got: {}", err);
    }
}
