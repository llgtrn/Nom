#![deny(unsafe_code)]
use nom_blocks::compose::app_block::AppBlock;
use nom_blocks::NomtuRef;
use crate::store::ArtifactStore;
use crate::progress::{ProgressSink, ComposeEvent};

pub struct AppInput {
    pub entity: NomtuRef,
    pub source_hash: [u8; 32],
    pub target_platform: String,
}

pub struct AppBackend;

impl AppBackend {
    pub fn compose(input: AppInput, store: &mut dyn ArtifactStore, sink: &dyn ProgressSink) -> AppBlock {
        sink.emit(ComposeEvent::Started { backend: "app".into(), entity_id: input.entity.id.clone() });
        // Stub: build artifact is source_hash bytes tagged with platform
        let mut artifact_data = input.source_hash.to_vec();
        artifact_data.extend_from_slice(input.target_platform.as_bytes());
        sink.emit(ComposeEvent::Progress { percent: 0.5, stage: "linking".into() });
        let artifact_hash = store.write(&artifact_data);
        let byte_size = store.byte_size(&artifact_hash).unwrap_or(0);
        sink.emit(ComposeEvent::Completed { artifact_hash, byte_size });
        AppBlock {
            entity: input.entity,
            artifact_hash,
            target_platform: input.target_platform,
            deploy_url: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InMemoryStore;
    use crate::progress::LogProgressSink;
    #[test]
    fn app_compose_basic() {
        let mut store = InMemoryStore::new();
        let source_hash = [42u8; 32];
        let input = AppInput {
            entity: NomtuRef { id: "app1".into(), word: "dashboard".into(), kind: "app".into() },
            source_hash,
            target_platform: "web".into(),
        };
        let block = AppBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.target_platform, "web");
        assert!(store.exists(&block.artifact_hash));
    }
}
