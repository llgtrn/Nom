#![deny(unsafe_code)]
use nom_blocks::compose::video_block::VideoBlock;
use nom_blocks::NomtuRef;
use crate::store::ArtifactStore;
use crate::progress::{ProgressSink, ComposeEvent};

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
        Self { fps, width, height, duration_frames, frames: Vec::new() }
    }

    pub fn duration_ms(&self) -> u32 {
        (self.duration_frames * 1000) / self.fps.max(1)
    }

    pub fn add_frame(&mut self, frame: VideoFrame) { self.frames.push(frame); }
}

pub struct VideoInput {
    pub entity: NomtuRef,
    pub frames: Vec<Vec<u8>>,
    pub fps: u32,
    pub width: u32,
    pub height: u32,
}

pub struct VideoBackend;

impl VideoBackend {
    pub fn compose(input: VideoInput, store: &mut dyn ArtifactStore, sink: &dyn ProgressSink) -> VideoBlock {
        sink.emit(ComposeEvent::Started { backend: "video".into(), entity_id: input.entity.id.clone() });

        let frame_count = input.frames.len() as u32;
        let fps = input.fps.max(1);
        let duration_secs = frame_count as f32 / fps as f32;

        // Build a VideoSpec from the input frames.
        let mut spec = VideoSpec::new(fps, input.width, input.height, duration_secs);
        for (i, raw_frame) in input.frames.iter().enumerate() {
            // Use FNV-1a over the raw frame bytes as the scene hash.
            let mut h: u64 = 14695981039346656037;
            for &b in raw_frame { h ^= b as u64; h = h.wrapping_mul(1099511628211); }
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
                stage: format!("encoding frames {}-{}", batch_start, (batch_start + batch_size).min(frame_count)),
            });
        }

        // Serialize spec to JSON and write to artifact store.
        let spec_json = serde_json::json!({
            "fps": spec.fps,
            "width": spec.width,
            "height": spec.height,
            "duration_frames": spec.duration_frames,
            "frame_count": spec.frames.len(),
        });
        let spec_bytes = spec_json.to_string().into_bytes();
        let artifact_hash = store.write(&spec_bytes);
        let byte_size = store.byte_size(&artifact_hash).unwrap_or(0);

        let duration_ms = spec.duration_ms() as u64;
        sink.emit(ComposeEvent::Completed { artifact_hash, byte_size });

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InMemoryStore;
    use crate::progress::LogProgressSink;

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
        // 120 frames / 24 fps * 1000 = 5000 ms
        assert_eq!(spec.duration_ms(), 5000);

        let spec_zero = VideoSpec::new(0, 1280, 720, 5.0);
        // fps.max(1) prevents divide-by-zero; duration_frames = 0
        assert_eq!(spec_zero.duration_ms(), 0);
    }

    #[test]
    fn video_compose_returns_artifact() {
        let mut store = InMemoryStore::new();
        let input = VideoInput {
            entity: NomtuRef { id: "vid1".into(), word: "clip".into(), kind: "media".into() },
            frames: vec![vec![0u8; 4], vec![128u8; 4], vec![255u8; 4]],
            fps: 24,
            width: 1920,
            height: 1080,
        };
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.width, 1920);
        assert_eq!(block.height, 1080);
        assert!(store.exists(&block.artifact_hash));
    }

    #[test]
    fn video_compose_entity_propagated() {
        let mut store = InMemoryStore::new();
        let input = VideoInput {
            entity: NomtuRef { id: "vid2".into(), word: "intro".into(), kind: "media".into() },
            frames: vec![vec![0u8; 4]],
            fps: 30,
            width: 1280,
            height: 720,
        };
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.entity.id, "vid2");
        assert_eq!(block.entity.word, "intro");
    }

    #[test]
    fn video_compose_duration_ms() {
        let mut store = InMemoryStore::new();
        // 30 frames at 30 fps = 1000 ms
        let frames: Vec<Vec<u8>> = (0..30).map(|_| vec![0u8; 4]).collect();
        let input = VideoInput {
            entity: NomtuRef { id: "vid3".into(), word: "second".into(), kind: "media".into() },
            frames,
            fps: 30,
            width: 640,
            height: 480,
        };
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.duration_ms, 1000);
    }

    #[test]
    fn video_spec_add_frame() {
        let mut spec = VideoSpec::new(24, 1920, 1080, 1.0);
        spec.add_frame(VideoFrame { frame_index: 0, duration_ms: 41, scene_hash: "abc".into() });
        assert_eq!(spec.frames.len(), 1);
        assert_eq!(spec.frames[0].frame_index, 0);
    }
}
