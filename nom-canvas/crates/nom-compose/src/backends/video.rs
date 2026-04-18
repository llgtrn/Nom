#![deny(unsafe_code)]
use crate::progress::{ComposeEvent, ProgressSink};
use crate::store::ArtifactStore;
use nom_blocks::compose::video_block::VideoBlock;
use nom_blocks::NomtuRef;
use std::fmt;

/// Output container format for video composition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContainerFormat {
    /// Raw YCbCr stream (YUV4MPEG2). Default — no external encoder needed.
    #[default]
    Y4m,
    /// MP4 stub — writes a header marker; a real encoder is required.
    Mp4Stub,
    /// WebM stub — writes a header marker; a real encoder is required.
    WebMStub,
}

impl fmt::Display for ContainerFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContainerFormat::Y4m => write!(f, "video/x-yuv4mpeg"),
            ContainerFormat::Mp4Stub => write!(f, "video/mp4"),
            ContainerFormat::WebMStub => write!(f, "video/webm"),
        }
    }
}

/// Video codec for composition output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VideoCodec {
    /// Uncompressed raw frames. Default.
    #[default]
    Raw,
    /// H.264 stub — writes a marker; external ffmpeg/libx264 required.
    H264Stub,
    /// VP9 stub — writes a marker; external libvpx required.
    Vp9Stub,
}

impl fmt::Display for VideoCodec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VideoCodec::Raw => write!(f, "rawvideo"),
            VideoCodec::H264Stub => write!(f, "h264"),
            VideoCodec::Vp9Stub => write!(f, "vp9"),
        }
    }
}

/// A single frame descriptor in the video timeline.
#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub frame_index: u32,
    pub duration_ms: u32,
    pub scene_hash: String,
}

/// Video composition spec — Remotion-pattern.
#[derive(Debug, Clone)]
pub struct VideoSpec {
    pub fps: u32,
    pub width: u32,
    pub height: u32,
    pub duration_frames: u32,
    pub frames: Vec<VideoFrame>,
}

impl VideoSpec {
    pub fn new(fps: u32, width: u32, height: u32, duration_secs: f32) -> Self {
        let duration_frames = (fps as f32 * duration_secs) as u32;
        Self {
            fps,
            width,
            height,
            duration_frames,
            frames: Vec::new(),
        }
    }

    pub fn duration_ms(&self) -> u32 {
        (self.duration_frames * 1000) / self.fps.max(1)
    }

    pub fn add_frame(&mut self, frame: VideoFrame) {
        self.frames.push(frame);
    }
}

pub struct VideoInput {
    pub entity: NomtuRef,
    pub frames: Vec<Vec<u8>>,
    pub fps: u32,
    pub width: u32,
    pub height: u32,
    /// Container format for the output artifact. Defaults to `Y4m`.
    pub container_format: ContainerFormat,
    /// Codec used to encode frames. Defaults to `Raw`.
    pub codec: VideoCodec,
}

pub struct VideoBackend;

impl VideoBackend {
    pub fn compose(
        input: VideoInput,
        store: &mut dyn ArtifactStore,
        sink: &dyn ProgressSink,
    ) -> VideoBlock {
        sink.emit(ComposeEvent::Started {
            backend: "video".into(),
            entity_id: input.entity.id.clone(),
        });

        let frame_count = input.frames.len() as u32;
        let fps = input.fps.max(1);
        let duration_secs = frame_count as f32 / fps as f32;

        // Build a VideoSpec from the input frames.
        let mut spec = VideoSpec::new(fps, input.width, input.height, duration_secs);
        for (i, raw_frame) in input.frames.iter().enumerate() {
            // Use FNV-1a over the raw frame bytes as the scene hash.
            let mut h: u64 = 14695981039346656037;
            for &b in raw_frame {
                h ^= b as u64;
                h = h.wrapping_mul(1099511628211);
            }
            spec.add_frame(VideoFrame {
                frame_index: i as u32,
                duration_ms: if fps > 0 { 1000 / fps } else { 0 },
                scene_hash: format!("{:016x}", h),
            });
        }

        // Emit progress events in batches of ~10% of frames (at least once per 10% mark).
        let batch_size = (frame_count / 10).max(1);
        for batch_start in (0..frame_count).step_by(batch_size as usize) {
            let pct = (batch_start as f32 + batch_size as f32).min(frame_count as f32)
                / frame_count.max(1) as f32;
            sink.emit(ComposeEvent::Progress {
                percent: pct,
                stage: format!(
                    "encoding frames {}-{}",
                    batch_start,
                    (batch_start + batch_size).min(frame_count)
                ),
            });
        }

        let payload = match (input.container_format, input.codec) {
            (ContainerFormat::Y4m, _) => encode_y4m_manifest(&spec, &input.frames),
            (ContainerFormat::Mp4Stub, codec) => {
                encode_stub_container("MP4", &codec.to_string(), &spec)
            }
            (ContainerFormat::WebMStub, codec) => {
                encode_stub_container("WebM", &codec.to_string(), &spec)
            }
        };
        let artifact_hash = store.write(&payload);
        let byte_size = store.byte_size(&artifact_hash).unwrap_or(0);

        let duration_ms = spec.duration_ms() as u64;
        sink.emit(ComposeEvent::Completed {
            artifact_hash,
            byte_size,
        });

        VideoBlock {
            entity: input.entity,
            artifact_hash,
            duration_ms,
            width: input.width,
            height: input.height,
            progress: None,
        }
    }
}

/// Produce a stub container payload with a machine-readable header marker.
/// The marker indicates that an external encoder is needed to produce a real file.
fn encode_stub_container(container: &str, codec: &str, spec: &VideoSpec) -> Vec<u8> {
    format!(
        "# NOM-STUB-CONTAINER: {} codec={} W={} H={} fps={} frames={}\n\
         # External encoder required to produce a real {} file.\n",
        container, codec, spec.width, spec.height, spec.fps, spec.duration_frames, container,
    )
    .into_bytes()
}

fn encode_y4m_manifest(spec: &VideoSpec, frames: &[Vec<u8>]) -> Vec<u8> {
    let mut out = format!(
        "YUV4MPEG2 W{} H{} F{}:1 Ip A1:1 Cmono\n",
        spec.width, spec.height, spec.fps
    )
    .into_bytes();
    for frame in frames {
        out.extend_from_slice(b"FRAME\n");
        out.extend_from_slice(frame);
        out.push(b'\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::LogProgressSink;
    use crate::store::InMemoryStore;

    fn default_video_input(
        id: &str,
        word: &str,
        frames: Vec<Vec<u8>>,
        fps: u32,
        w: u32,
        h: u32,
    ) -> VideoInput {
        VideoInput {
            entity: NomtuRef {
                id: id.into(),
                word: word.into(),
                kind: "media".into(),
            },
            frames,
            fps,
            width: w,
            height: h,
            container_format: ContainerFormat::default(),
            codec: VideoCodec::default(),
        }
    }

    // --- existing tests (backward-compat) ---

    #[test]
    fn video_spec_creation() {
        let spec = VideoSpec::new(30, 1920, 1080, 2.0);
        assert_eq!(spec.fps, 30);
        assert_eq!(spec.duration_frames, 60);
        assert_eq!(spec.width, 1920);
        assert_eq!(spec.height, 1080);
        assert!(spec.frames.is_empty());
    }

    #[test]
    fn video_spec_duration_ms() {
        let spec = VideoSpec::new(24, 1280, 720, 5.0);
        assert_eq!(spec.duration_ms(), 5000);
        let spec_zero = VideoSpec::new(0, 1280, 720, 5.0);
        assert_eq!(spec_zero.duration_ms(), 0);
    }

    #[test]
    fn video_compose_returns_artifact() {
        let mut store = InMemoryStore::new();
        let input = default_video_input(
            "vid1",
            "clip",
            vec![vec![0u8; 4], vec![128u8; 4], vec![255u8; 4]],
            24,
            1920,
            1080,
        );
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.width, 1920);
        assert_eq!(block.height, 1080);
        assert!(store.exists(&block.artifact_hash));
        let payload = store.read(&block.artifact_hash).unwrap();
        assert!(payload.starts_with(b"YUV4MPEG2"));
    }

    #[test]
    fn video_compose_entity_propagated() {
        let mut store = InMemoryStore::new();
        let input = default_video_input("vid2", "intro", vec![vec![0u8; 4]], 30, 1280, 720);
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.entity.id, "vid2");
        assert_eq!(block.entity.word, "intro");
    }

    #[test]
    fn video_compose_duration_ms() {
        let mut store = InMemoryStore::new();
        let frames: Vec<Vec<u8>> = (0..30).map(|_| vec![0u8; 4]).collect();
        let input = default_video_input("vid3", "second", frames, 30, 640, 480);
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.duration_ms, 1000);
    }

    #[test]
    fn video_spec_add_frame() {
        let mut spec = VideoSpec::new(24, 1920, 1080, 1.0);
        spec.add_frame(VideoFrame {
            frame_index: 0,
            duration_ms: 41,
            scene_hash: "abc".into(),
        });
        assert_eq!(spec.frames.len(), 1);
        assert_eq!(spec.frames[0].frame_index, 0);
    }

    #[test]
    fn video_y4m_manifest_has_frame_markers() {
        let mut spec = VideoSpec::new(24, 2, 2, 2.0 / 24.0);
        spec.add_frame(VideoFrame {
            frame_index: 0,
            duration_ms: 41,
            scene_hash: "a".into(),
        });
        spec.add_frame(VideoFrame {
            frame_index: 1,
            duration_ms: 41,
            scene_hash: "b".into(),
        });
        let bytes = encode_y4m_manifest(&spec, &[vec![0], vec![1]]);
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.starts_with("YUV4MPEG2 W2 H2 F24:1"));
        assert_eq!(text.matches("FRAME").count(), 2);
    }

    // --- new codec/container tests ---

    #[test]
    fn container_format_default_is_y4m() {
        assert_eq!(ContainerFormat::default(), ContainerFormat::Y4m);
    }

    #[test]
    fn video_codec_default_is_raw() {
        assert_eq!(VideoCodec::default(), VideoCodec::Raw);
    }

    #[test]
    fn container_format_display_mime_types() {
        assert_eq!(ContainerFormat::Y4m.to_string(), "video/x-yuv4mpeg");
        assert_eq!(ContainerFormat::Mp4Stub.to_string(), "video/mp4");
        assert_eq!(ContainerFormat::WebMStub.to_string(), "video/webm");
    }

    #[test]
    fn video_codec_display_names() {
        assert_eq!(VideoCodec::Raw.to_string(), "rawvideo");
        assert_eq!(VideoCodec::H264Stub.to_string(), "h264");
        assert_eq!(VideoCodec::Vp9Stub.to_string(), "vp9");
    }

    #[test]
    fn mp4_stub_produces_header_marker() {
        let mut store = InMemoryStore::new();
        let input = VideoInput {
            entity: NomtuRef {
                id: "v4".into(),
                word: "reel".into(),
                kind: "media".into(),
            },
            frames: vec![vec![0u8; 4]],
            fps: 24,
            width: 1280,
            height: 720,
            container_format: ContainerFormat::Mp4Stub,
            codec: VideoCodec::H264Stub,
        };
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        let text = String::from_utf8_lossy(&payload);
        assert!(text.contains("NOM-STUB-CONTAINER: MP4"));
        assert!(text.contains("codec=h264"));
        assert!(text.contains("External encoder required"));
    }

    #[test]
    fn webm_stub_produces_header_marker() {
        let mut store = InMemoryStore::new();
        let input = VideoInput {
            entity: NomtuRef {
                id: "v5".into(),
                word: "clip".into(),
                kind: "media".into(),
            },
            frames: vec![vec![0u8; 4]],
            fps: 30,
            width: 854,
            height: 480,
            container_format: ContainerFormat::WebMStub,
            codec: VideoCodec::Vp9Stub,
        };
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        let text = String::from_utf8_lossy(&payload);
        assert!(text.contains("NOM-STUB-CONTAINER: WebM"));
        assert!(text.contains("codec=vp9"));
    }

    #[test]
    fn stub_container_dimensions_in_header() {
        let spec = VideoSpec::new(25, 1920, 1080, 1.0);
        let bytes = encode_stub_container("MP4", "h264", &spec);
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("W=1920"));
        assert!(text.contains("H=1080"));
        assert!(text.contains("fps=25"));
    }

    #[test]
    fn y4m_default_backward_compat_round_trip() {
        // Default VideoInput uses Y4m + Raw — output must start with YUV4MPEG2.
        let mut store = InMemoryStore::new();
        let input = VideoInput {
            entity: NomtuRef {
                id: "v6".into(),
                word: "test".into(),
                kind: "media".into(),
            },
            frames: vec![vec![10u8, 20u8]],
            fps: 30,
            width: 320,
            height: 240,
            container_format: ContainerFormat::Y4m,
            codec: VideoCodec::Raw,
        };
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        assert!(payload.starts_with(b"YUV4MPEG2"));
    }

    #[test]
    fn mp4_stub_frame_count_in_header() {
        let spec = VideoSpec::new(24, 640, 360, 2.0); // 48 frames
        let bytes = encode_stub_container("MP4", "h264", &spec);
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("frames=48"));
    }

    #[test]
    fn video_input_new_fields_propagate_to_dispatch() {
        let mut store = InMemoryStore::new();
        let input = VideoInput {
            entity: NomtuRef {
                id: "v7".into(),
                word: "promo".into(),
                kind: "media".into(),
            },
            frames: vec![vec![0u8; 4]],
            fps: 60,
            width: 3840,
            height: 2160,
            container_format: ContainerFormat::Mp4Stub,
            codec: VideoCodec::H264Stub,
        };
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        // dimensions preserved
        assert_eq!(block.width, 3840);
        assert_eq!(block.height, 2160);
        // stub artifact exists
        assert!(store.exists(&block.artifact_hash));
    }

    // ── Wave AE new tests ────────────────────────────────────────────────────

    #[test]
    fn mp4_stub_header_contains_width_and_height() {
        let spec = VideoSpec::new(30, 1280, 720, 1.0);
        let bytes = encode_stub_container("MP4", "h264", &spec);
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("W=1280"), "MP4 stub must contain width");
        assert!(text.contains("H=720"), "MP4 stub must contain height");
    }

    #[test]
    fn webm_stub_header_contains_codec_field() {
        let spec = VideoSpec::new(25, 854, 480, 1.0);
        let bytes = encode_stub_container("WebM", "vp9", &spec);
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            text.contains("codec=vp9"),
            "WebM stub must contain codec field"
        );
        assert!(text.contains("NOM-STUB-CONTAINER: WebM"));
    }

    #[test]
    fn mp4_stub_container_format_display() {
        assert_eq!(ContainerFormat::Mp4Stub.to_string(), "video/mp4");
    }

    #[test]
    fn webm_stub_container_format_display() {
        assert_eq!(ContainerFormat::WebMStub.to_string(), "video/webm");
    }

    #[test]
    fn video_codec_h264_stub_display() {
        assert_eq!(VideoCodec::H264Stub.to_string(), "h264");
    }

    #[test]
    fn video_codec_vp9_stub_display() {
        assert_eq!(VideoCodec::Vp9Stub.to_string(), "vp9");
    }

    #[test]
    fn mp4_stub_compose_stores_artifact_in_store() {
        let mut store = InMemoryStore::new();
        let input = VideoInput {
            entity: NomtuRef {
                id: "mp4-test".into(),
                word: "film".into(),
                kind: "media".into(),
            },
            frames: vec![vec![1u8, 2u8, 3u8]],
            fps: 24,
            width: 640,
            height: 480,
            container_format: ContainerFormat::Mp4Stub,
            codec: VideoCodec::H264Stub,
        };
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        assert!(store.exists(&block.artifact_hash));
        let payload = store.read(&block.artifact_hash).unwrap();
        assert!(!payload.is_empty());
    }

    #[test]
    fn webm_stub_compose_stores_artifact_in_store() {
        let mut store = InMemoryStore::new();
        let input = VideoInput {
            entity: NomtuRef {
                id: "webm-test".into(),
                word: "loop".into(),
                kind: "media".into(),
            },
            frames: vec![vec![0u8; 8]],
            fps: 30,
            width: 1920,
            height: 1080,
            container_format: ContainerFormat::WebMStub,
            codec: VideoCodec::Vp9Stub,
        };
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        assert!(store.exists(&block.artifact_hash));
    }

    #[test]
    fn video_spec_zero_fps_duration_ms_is_zero() {
        let spec = VideoSpec::new(0, 640, 480, 1.0);
        // fps=0 — duration_ms must not panic and returns 0
        assert_eq!(spec.duration_ms(), 0);
    }

    #[test]
    fn video_compose_single_frame_duration_ms() {
        let mut store = InMemoryStore::new();
        // 1 frame at 1 fps => 1000 ms
        let input = default_video_input("v-single", "frame", vec![vec![0u8; 4]], 1, 8, 8);
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.duration_ms, 1000);
    }

    #[test]
    fn video_codec_roundtrip_via_display() {
        // Each codec's Display should produce a non-empty string
        let codecs = [VideoCodec::Raw, VideoCodec::H264Stub, VideoCodec::Vp9Stub];
        for codec in &codecs {
            let s = codec.to_string();
            assert!(
                !s.is_empty(),
                "codec display must be non-empty for {:?}",
                codec
            );
        }
    }

    #[test]
    fn container_format_all_three_display_different() {
        let y4m = ContainerFormat::Y4m.to_string();
        let mp4 = ContainerFormat::Mp4Stub.to_string();
        let webm = ContainerFormat::WebMStub.to_string();
        assert_ne!(y4m, mp4);
        assert_ne!(mp4, webm);
        assert_ne!(y4m, webm);
    }

    // ── Wave AG new tests ────────────────────────────────────────────────────

    #[test]
    fn video_codec_raw_produces_y4m_uncompressed() {
        // Raw codec with Y4m container must emit YUV4MPEG2 header.
        let mut store = InMemoryStore::new();
        let input = VideoInput {
            entity: NomtuRef {
                id: "raw-test".into(),
                word: "raw".into(),
                kind: "media".into(),
            },
            frames: vec![vec![0xAAu8; 4], vec![0xBBu8; 4]],
            fps: 24,
            width: 2,
            height: 2,
            container_format: ContainerFormat::Y4m,
            codec: VideoCodec::Raw,
        };
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        let text = String::from_utf8_lossy(&payload);
        assert!(
            text.starts_with("YUV4MPEG2"),
            "Raw codec must produce Y4m header"
        );
        assert_eq!(text.matches("FRAME").count(), 2);
    }

    #[test]
    fn video_backend_compose_with_mp4stub_returns_ok() {
        // Mp4Stub compose must succeed and store a non-empty artifact.
        let mut store = InMemoryStore::new();
        let input = VideoInput {
            entity: NomtuRef {
                id: "mp4ok".into(),
                word: "ok".into(),
                kind: "media".into(),
            },
            frames: vec![vec![0u8; 4]],
            fps: 30,
            width: 320,
            height: 240,
            container_format: ContainerFormat::Mp4Stub,
            codec: VideoCodec::H264Stub,
        };
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        assert!(store.exists(&block.artifact_hash));
        let payload = store.read(&block.artifact_hash).unwrap();
        assert!(!payload.is_empty());
    }

    #[test]
    fn video_backend_compose_with_webmstub_returns_ok() {
        let mut store = InMemoryStore::new();
        let input = VideoInput {
            entity: NomtuRef {
                id: "webmok".into(),
                word: "ok".into(),
                kind: "media".into(),
            },
            frames: vec![vec![1u8; 4]],
            fps: 25,
            width: 640,
            height: 360,
            container_format: ContainerFormat::WebMStub,
            codec: VideoCodec::Vp9Stub,
        };
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        assert!(store.exists(&block.artifact_hash));
        let payload = store.read(&block.artifact_hash).unwrap();
        let text = String::from_utf8_lossy(&payload);
        assert!(text.contains("NOM-STUB-CONTAINER: WebM"));
    }

    #[test]
    fn video_backend_tracks_frame_count_in_spec() {
        // 5 frames → duration_frames == 5
        let mut store = InMemoryStore::new();
        let frames: Vec<Vec<u8>> = (0..5).map(|i| vec![i as u8; 4]).collect();
        let input = default_video_input("fc-test", "frames", frames, 5, 4, 4);
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        // 5 frames at 5fps => 1000 ms
        assert_eq!(block.duration_ms, 1000);
    }

    #[test]
    fn video_backend_zero_frames_produces_empty_y4m() {
        // Zero frames: no FRAME markers, just the header.
        let mut store = InMemoryStore::new();
        let input = default_video_input("zero-frames", "empty", vec![], 24, 8, 8);
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        let text = String::from_utf8_lossy(&payload);
        assert!(
            text.starts_with("YUV4MPEG2"),
            "zero frames still emits Y4m header"
        );
        assert_eq!(
            text.matches("FRAME").count(),
            0,
            "zero frames = no FRAME markers"
        );
    }

    #[test]
    fn video_spec_fps_preserved_in_y4m_header() {
        let spec = VideoSpec::new(60, 1920, 1080, 0.0);
        let bytes = encode_y4m_manifest(&spec, &[]);
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            text.contains("F60:1"),
            "fps=60 must appear in YUV4MPEG2 header"
        );
    }

    #[test]
    fn mp4_stub_container_display_starts_with_video() {
        assert!(ContainerFormat::Mp4Stub.to_string().starts_with("video/"));
    }

    #[test]
    fn webm_stub_container_display_starts_with_video() {
        assert!(ContainerFormat::WebMStub.to_string().starts_with("video/"));
    }

    #[test]
    fn video_codec_h264stub_display_is_h264() {
        assert_eq!(VideoCodec::H264Stub.to_string(), "h264");
    }

    #[test]
    fn video_codec_vp9stub_display_is_vp9() {
        assert_eq!(VideoCodec::Vp9Stub.to_string(), "vp9");
    }

    #[test]
    fn stub_container_fps_appears_in_header() {
        let spec = VideoSpec::new(120, 1920, 1080, 1.0);
        let bytes = encode_stub_container("MP4", "h264", &spec);
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("fps=120"));
    }

    #[test]
    fn video_y4m_frame_hashes_are_hex_strings() {
        let mut spec = VideoSpec::new(24, 2, 2, 1.0 / 24.0);
        let frames = vec![vec![0xFFu8; 4]];
        // Build the spec with frame added manually then verify through encode
        spec.add_frame(VideoFrame {
            frame_index: 0,
            duration_ms: 41,
            scene_hash: format!("{:016x}", 0xDEADBEEFu64),
        });
        // scene_hash must be a valid lowercase hex string of length 16
        assert_eq!(spec.frames[0].scene_hash.len(), 16);
        assert!(spec.frames[0]
            .scene_hash
            .chars()
            .all(|c| c.is_ascii_hexdigit()));
        let _ = encode_y4m_manifest(&spec, &frames);
    }

    // ── Wave AJ new tests ────────────────────────────────────────────────────

    #[test]
    fn video_backend_frame_header_includes_dimensions() {
        // Y4m header must carry the width and height from VideoInput.
        let mut store = InMemoryStore::new();
        let input = default_video_input("hdr-dim", "scene", vec![vec![0u8; 4]], 24, 1280, 720);
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        let text = String::from_utf8_lossy(&payload);
        assert!(text.contains("W1280"), "header must contain width");
        assert!(text.contains("H720"), "header must contain height");
    }

    #[test]
    fn video_backend_frame_count_tracked() {
        // Compose with 7 frames; the resulting Y4m payload must have exactly 7 FRAME markers.
        let mut store = InMemoryStore::new();
        let frames: Vec<Vec<u8>> = (0..7).map(|i| vec![i as u8; 4]).collect();
        let input = default_video_input("fc7", "clip", frames, 7, 4, 4);
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        let text = String::from_utf8_lossy(&payload);
        assert_eq!(
            text.matches("FRAME").count(),
            7,
            "must have 7 FRAME markers"
        );
        let _ = block;
    }

    #[test]
    fn video_backend_fps_positive() {
        // fps=1 must produce a non-panic compose with duration_ms == frame_count * 1000.
        let mut store = InMemoryStore::new();
        let frames: Vec<Vec<u8>> = vec![vec![0u8; 4], vec![0u8; 4]];
        let input = default_video_input("fps-pos", "slow", frames, 1, 8, 8);
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.duration_ms, 2000, "2 frames at 1fps => 2000ms");
    }

    #[test]
    fn video_backend_artifact_size_grows_with_frames() {
        // More frames → larger artifact in the store.
        let mut store = InMemoryStore::new();
        let small_input = default_video_input("size-s", "s", vec![vec![0u8; 4]; 1], 24, 4, 4);
        let large_input = default_video_input("size-l", "l", vec![vec![0u8; 4]; 10], 24, 4, 4);
        let s_block = VideoBackend::compose(small_input, &mut store, &LogProgressSink);
        let l_block = VideoBackend::compose(large_input, &mut store, &LogProgressSink);
        let s_size = store.byte_size(&s_block.artifact_hash).unwrap();
        let l_size = store.byte_size(&l_block.artifact_hash).unwrap();
        assert!(
            l_size > s_size,
            "10 frames must produce larger artifact than 1 frame"
        );
    }

    #[test]
    fn video_backend_color_space_in_output() {
        // Y4m header must contain color-space marker (default is "Cmono" in the encoder).
        let mut store = InMemoryStore::new();
        let input = default_video_input("cs-test", "c", vec![vec![0u8; 4]], 24, 4, 4);
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        let text = String::from_utf8_lossy(&payload);
        assert!(
            text.contains("C"),
            "Y4m header must contain a color-space field starting with C"
        );
    }

    #[test]
    fn video_backend_h264_stub_output_labeled() {
        // MP4Stub + H264Stub must produce a payload containing "h264".
        let mut store = InMemoryStore::new();
        let input = VideoInput {
            entity: nom_blocks::NomtuRef {
                id: "h264-lbl".into(),
                word: "h264".into(),
                kind: "media".into(),
            },
            frames: vec![vec![0u8; 4]],
            fps: 24,
            width: 640,
            height: 360,
            container_format: ContainerFormat::Mp4Stub,
            codec: VideoCodec::H264Stub,
        };
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        let text = String::from_utf8_lossy(&payload);
        assert!(
            text.contains("h264"),
            "MP4/H264 stub must label codec as h264"
        );
    }

    #[test]
    fn video_backend_vp9_stub_output_labeled() {
        // WebMStub + Vp9Stub must produce a payload containing "vp9".
        let mut store = InMemoryStore::new();
        let input = VideoInput {
            entity: nom_blocks::NomtuRef {
                id: "vp9-lbl".into(),
                word: "vp9".into(),
                kind: "media".into(),
            },
            frames: vec![vec![0u8; 4]],
            fps: 30,
            width: 1280,
            height: 720,
            container_format: ContainerFormat::WebMStub,
            codec: VideoCodec::Vp9Stub,
        };
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        let text = String::from_utf8_lossy(&payload);
        assert!(
            text.contains("vp9"),
            "WebM/VP9 stub must label codec as vp9"
        );
    }

    #[test]
    fn video_backend_codec_name_in_output() {
        // Raw codec in Y4m: codec name should not appear (format is implicit), but Y4m header exists.
        let mut store = InMemoryStore::new();
        let input = default_video_input("codec-raw", "raw", vec![vec![0u8; 4]], 24, 4, 4);
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        let text = String::from_utf8_lossy(&payload);
        assert!(
            text.starts_with("YUV4MPEG2"),
            "raw codec Y4m output starts with YUV4MPEG2"
        );
    }

    #[test]
    fn video_backend_container_name_in_output() {
        // MP4Stub must include "MP4" in the stub header.
        let spec = VideoSpec::new(24, 640, 480, 1.0);
        let bytes = encode_stub_container("MP4", "h264", &spec);
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            text.contains("MP4"),
            "stub header must contain container name 'MP4'"
        );
    }

    #[test]
    fn video_backend_zero_duration_no_panic() {
        // 0-second duration (0 frames) must compose without panicking.
        let mut store = InMemoryStore::new();
        let input = default_video_input("zero-dur", "empty", vec![], 30, 4, 4);
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.duration_ms, 0, "zero frames => zero duration");
        assert!(store.exists(&block.artifact_hash));
    }

    // ── Wave AK new tests ────────────────────────────────────────────────────

    #[test]
    fn video_backend_h264stub_produces_nonempty_artifact() {
        // H264Stub codec with Mp4Stub container must produce a non-empty stored artifact.
        let mut store = InMemoryStore::new();
        let input = VideoInput {
            entity: NomtuRef {
                id: "h264-ne".into(),
                word: "film".into(),
                kind: "media".into(),
            },
            frames: vec![vec![0xAAu8; 8]],
            fps: 24,
            width: 640,
            height: 480,
            container_format: ContainerFormat::Mp4Stub,
            codec: VideoCodec::H264Stub,
        };
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        assert!(
            !payload.is_empty(),
            "H264Stub compose must produce non-empty artifact"
        );
    }

    #[test]
    fn video_backend_vp9stub_different_mime_than_h264stub() {
        // Vp9Stub in WebMStub yields "video/webm"; H264Stub in Mp4Stub yields "video/mp4".
        assert_eq!(
            ContainerFormat::WebMStub.to_string(),
            "video/webm",
            "Vp9Stub container must be video/webm"
        );
        assert_eq!(
            ContainerFormat::Mp4Stub.to_string(),
            "video/mp4",
            "H264Stub container must be video/mp4"
        );
        assert_ne!(
            ContainerFormat::WebMStub.to_string(),
            ContainerFormat::Mp4Stub.to_string(),
            "VP9 and H264 stub mime types must differ"
        );
    }

    #[test]
    fn video_backend_three_frames_larger_than_one_frame() {
        // 3 frames → combined artifact size must exceed single-frame artifact.
        let mut store = InMemoryStore::new();
        let single = default_video_input("sz1", "s", vec![vec![0u8; 16]], 24, 4, 4);
        let triple = default_video_input(
            "sz3",
            "t",
            vec![vec![1u8; 16], vec![2u8; 16], vec![3u8; 16]],
            24,
            4,
            4,
        );
        let s_block = VideoBackend::compose(single, &mut store, &LogProgressSink);
        let t_block = VideoBackend::compose(triple, &mut store, &LogProgressSink);
        let s_size = store.byte_size(&s_block.artifact_hash).unwrap();
        let t_size = store.byte_size(&t_block.artifact_hash).unwrap();
        assert!(
            t_size > s_size,
            "3 GPU frames must produce larger artifact than 1 frame"
        );
    }

    #[test]
    fn video_backend_zero_frames_produces_y4m_header_only() {
        // Zero frames → Y4m output has the header but zero FRAME markers (not an error).
        let mut store = InMemoryStore::new();
        let input = default_video_input("zf-ak", "empty", vec![], 30, 8, 8);
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        let payload = store.read(&block.artifact_hash).unwrap();
        let text = String::from_utf8_lossy(&payload);
        assert!(
            text.starts_with("YUV4MPEG2"),
            "zero frames must still produce Y4m header"
        );
        assert_eq!(
            text.matches("FRAME").count(),
            0,
            "zero frames must have 0 FRAME markers"
        );
        assert_eq!(block.duration_ms, 0, "zero frames must have zero duration");
    }

    #[test]
    fn video_codec_h264stub_name_round_trip() {
        // VideoCodec::H264Stub.to_string() -> "h264" and the Display impl is deterministic.
        let name = VideoCodec::H264Stub.to_string();
        assert_eq!(name, "h264", "H264Stub codec name must be 'h264'");
        // Round-trip: same variant produces the same string on repeated calls.
        assert_eq!(
            VideoCodec::H264Stub.to_string(),
            name,
            "codec name must be stable"
        );
    }

    #[test]
    fn video_backend_artifact_stored_for_h264stub() {
        // After compose, the artifact hash must be retrievable from the store.
        let mut store = InMemoryStore::new();
        let input = VideoInput {
            entity: NomtuRef {
                id: "h264-store".into(),
                word: "scene".into(),
                kind: "media".into(),
            },
            frames: vec![vec![0xFFu8; 4]],
            fps: 25,
            width: 320,
            height: 240,
            container_format: ContainerFormat::Mp4Stub,
            codec: VideoCodec::H264Stub,
        };
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        assert!(
            store.exists(&block.artifact_hash),
            "H264Stub artifact must be in store"
        );
    }

    #[test]
    fn video_backend_vp9stub_artifact_stored() {
        let mut store = InMemoryStore::new();
        let input = VideoInput {
            entity: NomtuRef {
                id: "vp9-store".into(),
                word: "clip".into(),
                kind: "media".into(),
            },
            frames: vec![vec![0x11u8; 4]],
            fps: 30,
            width: 640,
            height: 360,
            container_format: ContainerFormat::WebMStub,
            codec: VideoCodec::Vp9Stub,
        };
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        assert!(
            store.exists(&block.artifact_hash),
            "Vp9Stub artifact must be in store"
        );
    }

    #[test]
    fn video_backend_entity_word_preserved_mp4stub() {
        let mut store = InMemoryStore::new();
        let input = VideoInput {
            entity: NomtuRef {
                id: "word-mp4".into(),
                word: "my-movie".into(),
                kind: "media".into(),
            },
            frames: vec![vec![0u8; 4]],
            fps: 24,
            width: 1920,
            height: 1080,
            container_format: ContainerFormat::Mp4Stub,
            codec: VideoCodec::H264Stub,
        };
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.entity.word, "my-movie");
    }

    #[test]
    fn video_backend_h264_stub_payload_contains_stub_marker() {
        let spec = VideoSpec::new(24, 1280, 720, 1.0);
        let bytes = encode_stub_container("MP4", "h264", &spec);
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            text.contains("NOM-STUB-CONTAINER"),
            "MP4/h264 payload must contain stub marker"
        );
    }

    #[test]
    fn video_backend_vp9_stub_payload_contains_stub_marker() {
        let spec = VideoSpec::new(30, 854, 480, 1.0);
        let bytes = encode_stub_container("WebM", "vp9", &spec);
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            text.contains("NOM-STUB-CONTAINER"),
            "WebM/vp9 payload must contain stub marker"
        );
    }
}
