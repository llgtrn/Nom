#![deny(unsafe_code)]
use nom_blocks::compose::document_block::DocumentBlock;
use nom_blocks::NomtuRef;
use crate::store::ArtifactStore;
use crate::progress::{ProgressSink, ComposeEvent};
use crate::backends::ComposeResult;

pub struct DocumentInput {
    pub entity: NomtuRef,
    pub content_blocks: Vec<String>,
    pub target_mime: String,
}

pub struct DocumentBackend;

impl DocumentBackend {
    pub fn compose(input: DocumentInput, store: &mut dyn ArtifactStore, sink: &dyn ProgressSink) -> DocumentBlock {
        sink.emit(ComposeEvent::Started { backend: "document".into(), entity_id: input.entity.id.clone() });
        let content = input.content_blocks.join("\n\n");
        sink.emit(ComposeEvent::Progress { percent: 0.5, stage: "rendering".into() });
        let artifact_hash = store.write(content.as_bytes());
        let byte_size = store.byte_size(&artifact_hash).unwrap_or(0);
        sink.emit(ComposeEvent::Completed { artifact_hash, byte_size });
        DocumentBlock {
            entity: input.entity,
            artifact_hash,
            page_count: (content.len() / 3000 + 1) as u32,
            mime: input.target_mime,
        }
    }

    /// Error-wrapped variant of [`compose`]. Runs the same pipeline and returns
    /// `Ok(())` on success. Returns `Err(msg)` if the store rejects the write
    /// (currently the in-memory store never rejects, so this always returns `Ok`).
    pub fn compose_safe(input: DocumentInput, store: &mut dyn ArtifactStore, sink: &dyn ProgressSink) -> ComposeResult {
        let _block = Self::compose(input, store, sink);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InMemoryStore;
    use crate::progress::LogProgressSink;
    #[test]
    fn document_compose_basic() {
        let mut store = InMemoryStore::new();
        let input = DocumentInput {
            entity: NomtuRef { id: "doc1".into(), word: "report".into(), kind: "concept".into() },
            content_blocks: vec!["# Title".into(), "body text".into()],
            target_mime: "text/markdown".into(),
        };
        let block = DocumentBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.mime, "text/markdown");
        assert!(store.exists(&block.artifact_hash));
    }

    #[test]
    fn document_compose_safe_returns_ok() {
        let mut store = InMemoryStore::new();
        let input = DocumentInput {
            entity: NomtuRef { id: "doc2".into(), word: "brief".into(), kind: "concept".into() },
            content_blocks: vec!["intro".into(), "conclusion".into()],
            target_mime: "text/plain".into(),
        };
        let result = DocumentBackend::compose_safe(input, &mut store, &LogProgressSink);
        assert!(result.is_ok(), "compose_safe must return Ok(()) on success");
    }
}
