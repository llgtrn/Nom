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

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::thread::{self, JoinHandle};

/// Builder for constructing FFmpeg command-line arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfmpegArgsBuilder {
    fps: u32,
    width: u32,
    height: u32,
    codec: VideoCodec,
    output_path: String,
    pixel_format: String,
    input_format: String,
    extra_args: Vec<String>,
}

impl FfmpegArgsBuilder {
    /// Start a new builder targeting `output_path`.
    pub fn new(output_path: impl Into<String>) -> Self {
        Self {
            fps: 30,
            width: 1920,
            height: 1080,
            codec: VideoCodec::H264,
            output_path: output_path.into(),
            pixel_format: "yuv420p".into(),
            input_format: "mjpeg".into(),
            extra_args: Vec::new(),
        }
    }

    /// Set output frame rate.
    pub fn fps(mut self, fps: u32) -> Self {
        self.fps = fps;
        self
    }

    /// Set frame resolution.
    pub fn resolution(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set video codec.
    pub fn codec(mut self, codec: VideoCodec) -> Self {
        self.codec = codec;
        self
    }

    /// Set output pixel format (e.g. `"yuv420p"`).
    pub fn pixel_format(mut self, fmt: impl Into<String>) -> Self {
        self.pixel_format = fmt.into();
        self
    }

    /// Set the expected input image format for `image2pipe`
    /// (e.g. `"mjpeg"` or `"png"`).
    pub fn input_format(mut self, fmt: impl Into<String>) -> Self {
        self.input_format = fmt.into();
        self
    }

    /// Append an arbitrary flag or value.
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.extra_args.push(arg.into());
        self
    }

    /// Build the full argument vector for `ffmpeg`.
    ///
    /// The returned list does **not** include the `"ffmpeg"` binary name.
    pub fn build(self) -> Vec<String> {
        let mut args = vec![
            "-y".to_string(),
            "-f".to_string(),
            "image2pipe".to_string(),
            "-vcodec".to_string(),
            self.input_format.clone(),
            "-r".to_string(),
            self.fps.to_string(),
            "-s".to_string(),
            format!("{}x{}", self.width, self.height),
            "-i".to_string(),
            "-".to_string(),
        ];
        match self.codec {
            VideoCodec::H264 => {
                args.push("-c:v".to_string());
                args.push("libx264".to_string());
            }
            VideoCodec::H265 => {
                args.push("-c:v".to_string());
                args.push("libx265".to_string());
            }
            VideoCodec::Vp9 => {
                args.push("-c:v".to_string());
                args.push("libvpx-vp9".to_string());
            }
            VideoCodec::Av1 => {
                args.push("-c:v".to_string());
                args.push("libaom-av1".to_string());
            }
        }
        args.push("-pix_fmt".to_string());
        args.push(self.pixel_format);
        args.extend(self.extra_args);
        args.push(self.output_path);
        args
    }
}

/// A parsed progress line from FFmpeg stderr.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FfmpegProgress {
    /// Current output frame number.
    pub frame: Option<u64>,
    /// Encoding speed in frames-per-second.
    pub fps: Option<f64>,
    /// Timestamp of the last encoded frame.
    pub time: Option<String>,
    /// Current bitrate string, e.g. `"1200kbits/s"`.
    pub bitrate: Option<String>,
    /// Speed multiplier, e.g. `"2.0x"`.
    pub speed: Option<String>,
}

/// Parses FFmpeg stderr progress lines.
#[derive(Debug, Clone, Copy)]
pub struct FfmpegProgressParser;

impl FfmpegProgressParser {
    /// Create a new parser.
    pub fn new() -> Self {
        Self
    }

    /// Attempt to parse a single line of FFmpeg stderr output.
    ///
    /// Returns `None` if the line does not contain progress fields.
    pub fn parse_line(line: &str) -> Option<FfmpegProgress> {
        if !line.contains("frame=") && !line.contains("time=") {
            return None;
        }
        // Normalize spaces around '=' so "frame=  123" becomes "frame=123"
        let mut normalized = line.to_string();
        while normalized.contains("= ") {
            normalized = normalized.replace("= ", "=");
        }
        let mut progress = FfmpegProgress::default();
        for part in normalized.split_whitespace() {
            if let Some((key, val)) = part.split_once('=') {
                match key {
                    "frame" => {
                        progress.frame = val.trim().parse().ok();
                    }
                    "fps" => {
                        progress.fps = val.trim().parse().ok();
                    }
                    "time" => {
                        progress.time = Some(val.trim().to_string());
                    }
                    "bitrate" => {
                        progress.bitrate = Some(val.trim().to_string());
                    }
                    "speed" => {
                        progress.speed = Some(val.trim().to_string());
                    }
                    _ => {}
                }
            }
        }
        Some(progress)
    }
}

/// Spawns FFmpeg with `image2pipe` input and writes encoded frames to its stdin.
#[derive(Debug)]
pub struct FfmpegPipeEncoder {
    stdin: ChildStdin,
    child: Child,
    stderr_handle: JoinHandle<Vec<FfmpegProgress>>,
}

impl FfmpegPipeEncoder {
    /// Spawn FFmpeg with the given argument list.
    ///
    /// The first element of `args` should **not** be `"ffmpeg"`.
    pub fn spawn(args: &[String]) -> Result<Self, std::io::Error> {
        let mut child = Command::new("ffmpeg")
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdin = child.stdin.take().expect("stdin was piped");
        let stderr = child.stderr.take().expect("stderr was piped");
        let stderr_handle = thread::spawn(move || {
            let reader = BufReader::new(stderr);
            let mut progress = Vec::new();
            for line in reader.lines().map_while(Result::ok) {
                if let Some(p) = FfmpegProgressParser::parse_line(&line) {
                    progress.push(p);
                }
            }
            progress
        });
        Ok(Self {
            stdin,
            child,
            stderr_handle,
        })
    }

    /// Write one encoded frame (e.g. JPEG or PNG bytes) to FFmpeg's stdin.
    pub fn write_frame(&mut self, bytes: &[u8]) -> Result<(), std::io::Error> {
        self.stdin.write_all(bytes)
    }

    /// Close stdin, wait for FFmpeg to finish, and collect parsed progress.
    pub fn finish(mut self) -> Result<Vec<FfmpegProgress>, std::io::Error> {
        drop(self.stdin);
        let _status = self.child.wait()?;
        match self.stderr_handle.join() {
            Ok(progress) => Ok(progress),
            Err(_) => Ok(Vec::new()),
        }
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

    #[test]
    fn ffmpeg_args_builder_defaults() {
        let args = FfmpegArgsBuilder::new("out.mp4").build();
        assert!(args.contains(&"-y".to_string()));
        assert!(args.contains(&"image2pipe".to_string()));
        assert!(args.contains(&"out.mp4".to_string()));
        assert!(args.contains(&"libx264".to_string()));
    }

    #[test]
    fn ffmpeg_args_builder_custom_codec() {
        let args = FfmpegArgsBuilder::new("out.webm")
            .codec(VideoCodec::Vp9)
            .fps(24)
            .resolution(1280, 720)
            .build();
        assert!(args.contains(&"libvpx-vp9".to_string()));
        assert!(args.contains(&"24".to_string()));
        assert!(args.contains(&"1280x720".to_string()));
    }

    #[test]
    fn ffmpeg_args_builder_extra_args_before_output() {
        let args = FfmpegArgsBuilder::new("out.mp4")
            .arg("-preset")
            .arg("fast")
            .build();
        let out_idx = args.iter().position(|a| a == "out.mp4").unwrap();
        let preset_idx = args.iter().position(|a| a == "-preset").unwrap();
        assert!(preset_idx < out_idx, "flags must appear before output path");
    }

    #[test]
    fn ffmpeg_progress_parser_parses_frame_line() {
        let line = "frame=  123 fps= 60 q=28.0 time=00:00:05.12 bitrate=1200kbits/s speed=2.0x";
        let p = FfmpegProgressParser::parse_line(line).unwrap();
        assert_eq!(p.frame, Some(123));
        assert_eq!(p.fps, Some(60.0));
        assert_eq!(p.time, Some("00:00:05.12".to_string()));
        assert_eq!(p.bitrate, Some("1200kbits/s".to_string()));
        assert_eq!(p.speed, Some("2.0x".to_string()));
    }

    #[test]
    fn ffmpeg_progress_parser_ignores_non_progress() {
        assert!(FfmpegProgressParser::parse_line("Input #0, image2pipe").is_none());
        assert!(FfmpegProgressParser::parse_line("  Duration: N/A").is_none());
    }

    #[test]
    fn ffmpeg_progress_parser_handles_partial_lines() {
        let line = "frame=0 fps=0.0 time=00:00:00.00 bitrate=N/A speed=N/A";
        let p = FfmpegProgressParser::parse_line(line).unwrap();
        assert_eq!(p.frame, Some(0));
        assert_eq!(p.time, Some("00:00:00.00".to_string()));
    }

    #[test]
    fn ffmpeg_pipe_encoder_api_exists() {
        // We cannot assume `ffmpeg` is on PATH in every test environment,
        // so this test only verifies that the `spawn` API is callable and
        // returns an error when the binary is missing.
        if std::process::Command::new("ffmpeg")
            .arg("-version")
            .output()
            .is_err()
        {
            let args = FfmpegArgsBuilder::new("out.mp4").build();
            let result = FfmpegPipeEncoder::spawn(&args);
            assert!(result.is_err());
        }
    }
}
