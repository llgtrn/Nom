#![deny(unsafe_code)]
use crate::deep_think::{DeepThinkConfig, DeepThinkStream};
use crate::progress::{ComposeEvent, ProgressSink};
use crate::store::ArtifactStore;
use nom_blocks::NomtuRef;
use nom_graph::{Dag, GraphRagRetriever, QueryVec, RetrievedNode};

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

/// Pipeline configuration for a single RAG retrieval pass over a [`Dag`].
pub struct RagPipeline {
    /// Hash of the query input, used to derive the query embedding.
    pub input_hash: u64,
    /// How many nodes to retrieve (default 5).
    pub top_k: usize,
    /// Maximum BFS hops during graph traversal (default 3).
    pub max_hops: usize,
}

impl RagPipeline {
    pub fn new(input_hash: u64) -> Self {
        Self {
            input_hash,
            top_k: 5,
            max_hops: 3,
        }
    }
}

pub struct RagQueryBackend {
    pub deep_think_config: Option<DeepThinkConfig>,
    pub top_k: Option<usize>,
}

impl Default for RagQueryBackend {
    fn default() -> Self {
        Self {
            deep_think_config: None,
            top_k: None,
        }
    }
}

impl RagQueryBackend {
    /// Attach a `DeepThinkConfig` to this backend; compose signature is unchanged.
    pub fn with_deep_think(mut self, config: DeepThinkConfig) -> Self {
        self.deep_think_config = Some(config);
        self
    }

    pub fn compose(
        input: RagQueryInput,
        store: &mut dyn ArtifactStore,
        sink: &dyn ProgressSink,
    ) -> RagQueryOutput {
        sink.emit(ComposeEvent::Started {
            backend: "rag_query".into(),
            entity_id: input.entity.id.clone(),
        });
        let mut top: Vec<&RagChunk> = input.chunks.iter().collect();
        top.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        top.truncate(input.top_k);
        let context = top
            .iter()
            .map(|c| c.text.as_str())
            .collect::<Vec<_>>()
            .join("\n---\n");
        let answer = format!("Query: {}\n\nContext:\n{}", input.query, context);
        let hash = store.write(answer.as_bytes());
        let used_ids: Vec<String> = top.iter().map(|c| c.id.clone()).collect();
        sink.emit(ComposeEvent::Completed {
            artifact_hash: hash,
            byte_size: answer.len() as u64,
        });
        RagQueryOutput {
            artifact_hash: hash,
            answer,
            chunks_used: used_ids,
        }
    }

    /// Retrieve graph nodes from `dag` most relevant to `input_hash`.
    ///
    /// Derives a [`QueryVec`] from `input_hash` by spreading its bits across
    /// 16 slots (`v[i] = ((input_hash >> (i * 4)) & 0xF) as f32 / 15.0`),
    /// then runs [`GraphRagRetriever::retrieve`] with the configured `top_k`
    /// and a fixed `max_hops` of 3.  Emits a midpoint progress event and a
    /// completion event via `sink`.
    pub fn compose_with_dag(
        &self,
        dag: &Dag,
        input_hash: u64,
        sink: &dyn ProgressSink,
    ) -> Vec<RetrievedNode> {
        // If deep-think config is set, run the reasoning chain first; steps are
        // emitted to sink as progress events before the RAG retrieval begins.
        if let Some(ref config) = self.deep_think_config {
            let stream = DeepThinkStream::new(config.clone());
            let _steps = stream.think(input_hash, sink);
        }

        let retriever = GraphRagRetriever::new(dag);

        let mut qvec: QueryVec = [0.0f32; 16];
        for i in 0..16 {
            qvec[i] = ((input_hash >> (i * 4)) & 0xF) as f32 / 15.0;
        }

        sink.emit(ComposeEvent::Progress {
            percent: 50.0,
            stage: "graph_rag_retrieve".into(),
        });

        let results = retriever.retrieve(&qvec, self.top_k.unwrap_or(5), 3);

        sink.emit(ComposeEvent::Progress {
            percent: 100.0,
            stage: "graph_rag_done".into(),
        });

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::{LogProgressSink, VecProgressSink};
    use crate::store::InMemoryStore;
    use nom_graph::{Dag, ExecNode};

    #[test]
    fn rag_top_k_selection() {
        let mut store = InMemoryStore::new();
        let chunks = vec![
            RagChunk {
                id: "a".into(),
                text: "low".into(),
                score: 0.3,
            },
            RagChunk {
                id: "b".into(),
                text: "high".into(),
                score: 0.9,
            },
            RagChunk {
                id: "c".into(),
                text: "mid".into(),
                score: 0.6,
            },
        ];
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "r1".into(),
                    word: "search".into(),
                    kind: "verb".into(),
                },
                query: "what is high?".into(),
                top_k: 2,
                chunks,
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(out.chunks_used.len(), 2);
        assert_eq!(out.chunks_used[0], "b");
    }

    #[test]
    fn rag_empty_chunks() {
        let mut store = InMemoryStore::new();
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "r2".into(),
                    word: "search".into(),
                    kind: "verb".into(),
                },
                query: "anything".into(),
                top_k: 3,
                chunks: vec![],
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(out.chunks_used.len(), 0);
        assert!(store.exists(&out.artifact_hash));
    }

    // -----------------------------------------------------------------------
    // compose_with_dag integration tests
    // -----------------------------------------------------------------------

    #[test]
    fn rag_pipeline_compose_with_empty_dag() {
        let dag = Dag::new();
        let backend = RagQueryBackend::default();
        let results = backend.compose_with_dag(&dag, 0xdeadbeef_cafebabe, &LogProgressSink);
        assert!(results.is_empty(), "empty DAG must return no results");
    }

    #[test]
    fn rag_pipeline_compose_returns_nodes() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("node_a", "verb"));
        dag.add_node(ExecNode::new("node_b", "verb"));
        dag.add_node(ExecNode::new("node_c", "verb"));
        dag.add_edge("node_a", "out", "node_b", "in");
        dag.add_edge("node_b", "out", "node_c", "in");

        let backend = RagQueryBackend {
            top_k: Some(2),
            ..Default::default()
        };
        let results = backend.compose_with_dag(&dag, 0x1234567890abcdef, &LogProgressSink);

        assert_eq!(results.len(), 2, "top_k=2 must return exactly 2 nodes");
        // Results must be sorted by score descending.
        assert!(
            results[0].score >= results[1].score,
            "results must be sorted by score descending"
        );
    }

    #[test]
    fn rag_query_with_deep_think_emits_progress() {
        use crate::deep_think::DeepThinkConfig;

        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("dt_node", "verb"));

        let config = DeepThinkConfig {
            max_steps: 2,
            beam_width: 1,
            token_budget: 128,
        };
        let backend = RagQueryBackend::default().with_deep_think(config);
        let sink = VecProgressSink::new();
        let _ = backend.compose_with_dag(&dag, 0xaabb_ccdd_eeff_0011, &sink);

        let events = sink.take();
        // Deep-think emits max_steps Progress events + 1 Completed, then RAG
        // emits 2 more Progress events.  We just verify at least 2 Progress
        // events from the deep-think phase are present.
        let progress_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, ComposeEvent::Progress { .. }))
            .collect();
        assert!(
            progress_events.len() >= 2,
            "deep-think (max_steps=2) must emit at least 2 Progress events, got {}",
            progress_events.len()
        );
        // Verify at least one deep-think stage label is present.
        assert!(
            progress_events.iter().any(|e| {
                if let ComposeEvent::Progress { stage, .. } = e {
                    stage.starts_with("think_step_")
                } else {
                    false
                }
            }),
            "expected at least one think_step_* Progress event"
        );
    }

    #[test]
    fn rag_pipeline_emits_progress_events() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("x", "verb"));

        let backend = RagQueryBackend::default();
        let sink = VecProgressSink::new();
        let _ = backend.compose_with_dag(&dag, 0xffffffff00000000, &sink);

        let events = sink.take();
        let progress_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, ComposeEvent::Progress { .. }))
            .collect();
        assert!(
            progress_events.len() >= 2,
            "must emit at least 2 Progress events, got {}",
            progress_events.len()
        );
        // First event must be at 50%, last at 100%.
        if let ComposeEvent::Progress { percent, .. } = &progress_events[0] {
            assert!(
                (*percent - 50.0).abs() < 0.01,
                "first progress must be 50%, got {percent}"
            );
        }
        if let ComposeEvent::Progress { percent, .. } = &progress_events[progress_events.len() - 1]
        {
            assert!(
                (*percent - 100.0).abs() < 0.01,
                "last progress must be 100%, got {percent}"
            );
        }
    }
}
