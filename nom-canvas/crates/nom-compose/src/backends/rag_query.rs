#![deny(unsafe_code)]
use nom_blocks::NomtuRef;
use crate::store::ArtifactStore;
use crate::progress::{ProgressSink, ComposeEvent};
use crate::deep_think::DeepThinkConfig;

pub struct RagChunk {
    pub id: String,
    pub text: String,
    pub score: f32,
}

pub struct RagQueryInput {
    pub entity: NomtuRef,
    pub query: String,
    pub top_k: usize,
    pub chunks: Vec<RagChunk>,
}

pub struct RagQueryOutput {
    pub artifact_hash: [u8; 32],
    pub answer: String,
    pub chunks_used: Vec<String>,
}

pub struct RagQueryBackend {
    pub deep_think_config: Option<DeepThinkConfig>,
}

impl Default for RagQueryBackend {
    fn default() -> Self {
        Self { deep_think_config: None }
    }
}

impl RagQueryBackend {
    /// Attach a `DeepThinkConfig` to this backend; compose signature is unchanged.
    pub fn with_deep_think(mut self, config: DeepThinkConfig) -> Self {
        self.deep_think_config = Some(config);
        self
    }

    pub fn compose(input: RagQueryInput, store: &mut dyn ArtifactStore, sink: &dyn ProgressSink) -> RagQueryOutput {
        sink.emit(ComposeEvent::Started { backend: "rag_query".into(), entity_id: input.entity.id.clone() });
        let mut top: Vec<&RagChunk> = input.chunks.iter().collect();
        top.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        top.truncate(input.top_k);
        let context = top.iter().map(|c| c.text.as_str()).collect::<Vec<_>>().join("\n---\n");
        let answer = format!("Query: {}\n\nContext:\n{}", input.query, context);
        let hash = store.write(answer.as_bytes());
        let used_ids: Vec<String> = top.iter().map(|c| c.id.clone()).collect();
        sink.emit(ComposeEvent::Completed { artifact_hash: hash, byte_size: answer.len() as u64 });
        RagQueryOutput { artifact_hash: hash, answer, chunks_used: used_ids }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InMemoryStore;
    use crate::progress::LogProgressSink;

    #[test]
    fn rag_top_k_selection() {
        let mut store = InMemoryStore::new();
        let chunks = vec![
            RagChunk { id: "a".into(), text: "low".into(), score: 0.3 },
            RagChunk { id: "b".into(), text: "high".into(), score: 0.9 },
            RagChunk { id: "c".into(), text: "mid".into(), score: 0.6 },
        ];
        let out = RagQueryBackend::compose(RagQueryInput {
            entity: NomtuRef { id: "r1".into(), word: "search".into(), kind: "verb".into() },
            query: "what is high?".into(),
            top_k: 2,
            chunks,
        }, &mut store, &LogProgressSink);
        assert_eq!(out.chunks_used.len(), 2);
        assert_eq!(out.chunks_used[0], "b");
    }

    #[test]
    fn rag_empty_chunks() {
        let mut store = InMemoryStore::new();
        let out = RagQueryBackend::compose(RagQueryInput {
            entity: NomtuRef { id: "r2".into(), word: "search".into(), kind: "verb".into() },
            query: "anything".into(),
            top_k: 3,
            chunks: vec![],
        }, &mut store, &LogProgressSink);
        assert_eq!(out.chunks_used.len(), 0);
        assert!(store.exists(&out.artifact_hash));
    }
}
