#![deny(unsafe_code)]
use nom_blocks::compose::video_block::VideoBlock;
use nom_blocks::NomtuRef;
use crate::store::ArtifactStore;
use crate::progress::{ProgressSink, ComposeEvent};

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
        let frame_count = input.frames.len() as u64;
        let raw: Vec<u8> = input.frames.into_iter().flatten().collect();
        sink.emit(ComposeEvent::Progress { percent: 0.5, stage: "encoding".into() });
        let artifact_hash = store.write(&raw);
        let byte_size = store.byte_size(&artifact_hash).unwrap_or(0);
        let duration_ms = if input.fps > 0 { frame_count * 1000 / input.fps as u64 } else { 0 };
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
    fn video_compose_basic() {
        let mut store = InMemoryStore::new();
        let input = VideoInput {
            entity: NomtuRef { id: "vid1".into(), word: "clip".into(), kind: "media".into() },
            frames: vec![vec![0u8; 4], vec![255u8; 4]],
            fps: 24,
            width: 1920,
            height: 1080,
        };
        let block = VideoBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.width, 1920);
        assert_eq!(block.height, 1080);
        assert!(store.exists(&block.artifact_hash));
    }
}
