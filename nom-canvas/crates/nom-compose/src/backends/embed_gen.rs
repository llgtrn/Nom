#![deny(unsafe_code)]
use nom_blocks::NomtuRef;
use crate::store::ArtifactStore;
use crate::progress::{ProgressSink, ComposeEvent};

pub struct EmbedGenInput {
    pub entity: NomtuRef,
    pub texts: Vec<String>,
    pub dim: usize,
}

pub struct EmbedGenOutput {
    pub artifact_hash: [u8; 32],
    pub vector_count: usize,
    pub dim: usize,
}

pub struct EmbedGenBackend;

impl EmbedGenBackend {
    pub fn compose(input: EmbedGenInput, store: &mut dyn ArtifactStore, sink: &dyn ProgressSink) -> EmbedGenOutput {
        sink.emit(ComposeEvent::Started { backend: "embed_gen".into(), entity_id: input.entity.id.clone() });
        let vectors: Vec<Vec<f32>> = input.texts.iter().enumerate().map(|(i, text)| {
            // Stub: deterministic embedding from text bytes, padded/truncated to dim
            let mut v = vec![0.0f32; input.dim];
            for (j, b) in text.bytes().enumerate().take(input.dim) {
                v[j] = (b as f32 + i as f32) / 256.0;
            }
            v
        }).collect();
        // Serialize as flat f32 LE bytes
        let mut bytes = Vec::with_capacity(vectors.len() * input.dim * 4);
        for vec in &vectors {
            for &val in vec {
                bytes.extend_from_slice(&val.to_le_bytes());
            }
        }
        let artifact_hash = store.write(&bytes);
        let vector_count = vectors.len();
        sink.emit(ComposeEvent::Completed { artifact_hash, byte_size: bytes.len() as u64 });
        EmbedGenOutput { artifact_hash, vector_count, dim: input.dim }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InMemoryStore;
    use crate::progress::LogProgressSink;

    #[test]
    fn embed_gen_produces_vectors() {
        let mut store = InMemoryStore::new();
        let out = EmbedGenBackend::compose(EmbedGenInput {
            entity: NomtuRef { id: "e1".into(), word: "embed".into(), kind: "concept".into() },
            texts: vec!["hello".into(), "world".into()],
            dim: 4,
        }, &mut store, &LogProgressSink);
        assert_eq!(out.vector_count, 2);
        assert_eq!(out.dim, 4);
        // 2 vectors * 4 dims * 4 bytes each = 32 bytes
        assert_eq!(store.byte_size(&out.artifact_hash).unwrap(), 32);
    }

    #[test]
    fn embed_gen_empty_texts() {
        let mut store = InMemoryStore::new();
        let out = EmbedGenBackend::compose(EmbedGenInput {
            entity: NomtuRef { id: "e2".into(), word: "embed".into(), kind: "concept".into() },
            texts: vec![],
            dim: 8,
        }, &mut store, &LogProgressSink);
        assert_eq!(out.vector_count, 0);
        assert!(store.exists(&out.artifact_hash));
    }
}
