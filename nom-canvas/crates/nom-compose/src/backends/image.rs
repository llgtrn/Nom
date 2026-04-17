#![deny(unsafe_code)]
use nom_blocks::compose::image_block::ImageBlock;
use nom_blocks::NomtuRef;
use crate::store::ArtifactStore;
use crate::progress::{ProgressSink, ComposeEvent};

pub struct ImageInput {
    pub entity: NomtuRef,
    pub pixel_data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub prompt_used: String,
}

pub struct ImageBackend;

impl ImageBackend {
    pub fn compose(input: ImageInput, store: &mut dyn ArtifactStore, sink: &dyn ProgressSink) -> ImageBlock {
        sink.emit(ComposeEvent::Started { backend: "image".into(), entity_id: input.entity.id.clone() });
        sink.emit(ComposeEvent::Progress { percent: 0.5, stage: "rasterizing".into() });
        let artifact_hash = store.write(&input.pixel_data);
        let byte_size = store.byte_size(&artifact_hash).unwrap_or(0);
        sink.emit(ComposeEvent::Completed { artifact_hash, byte_size });
        ImageBlock {
            entity: input.entity,
            artifact_hash,
            width: input.width,
            height: input.height,
            prompt_used: input.prompt_used,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InMemoryStore;
    use crate::progress::LogProgressSink;

    #[test]
    fn image_compose_basic() {
        let mut store = InMemoryStore::new();
        let input = ImageInput {
            entity: NomtuRef { id: "img1".into(), word: "banner".into(), kind: "media".into() },
            pixel_data: vec![255u8; 64],
            width: 8,
            height: 8,
            prompt_used: "a white square".into(),
        };
        let block = ImageBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.width, 8);
        assert_eq!(block.height, 8);
        assert!(store.exists(&block.artifact_hash));
    }

    #[test]
    fn image_compose_stores_pixel_data() {
        let mut store = InMemoryStore::new();
        let pixel_data: Vec<u8> = (0u8..=255).collect();
        let input = ImageInput {
            entity: NomtuRef { id: "img2".into(), word: "gradient".into(), kind: "media".into() },
            pixel_data: pixel_data.clone(),
            width: 16,
            height: 16,
            prompt_used: "gradient test".into(),
        };
        let block = ImageBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.width, 16);
        assert_eq!(block.height, 16);
        assert_eq!(block.prompt_used, "gradient test");
        let stored = store.read(&block.artifact_hash).unwrap();
        assert_eq!(stored, pixel_data);
    }

    #[test]
    fn image_compose_entity_propagated() {
        let mut store = InMemoryStore::new();
        let input = ImageInput {
            entity: NomtuRef { id: "img3".into(), word: "thumbnail".into(), kind: "media".into() },
            pixel_data: vec![0u8; 16],
            width: 4,
            height: 4,
            prompt_used: "black thumbnail".into(),
        };
        let block = ImageBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.entity.id, "img3");
        assert_eq!(block.entity.word, "thumbnail");
    }
}
