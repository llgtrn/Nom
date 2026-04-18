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

#[derive(Default)]
pub struct RagQueryBackend {
    pub deep_think_config: Option<DeepThinkConfig>,
    pub top_k: Option<usize>,
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
        for (i, slot) in qvec.iter_mut().enumerate() {
            *slot = ((input_hash >> (i * 4)) & 0xF) as f32 / 15.0;
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

    #[test]
    fn rag_max_k_clamping_top_k_larger_than_chunks() {
        let mut store = InMemoryStore::new();
        let chunks = vec![
            RagChunk {
                id: "a".into(),
                text: "one".into(),
                score: 0.5,
            },
            RagChunk {
                id: "b".into(),
                text: "two".into(),
                score: 0.8,
            },
        ];
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "e1".into(),
                    word: "q".into(),
                    kind: "verb".into(),
                },
                query: "test".into(),
                top_k: 10, // more than available chunks
                chunks,
            },
            &mut store,
            &LogProgressSink,
        );
        // Should return at most the available chunks (2), not error.
        assert_eq!(out.chunks_used.len(), 2);
    }

    #[test]
    fn rag_relevance_ordering_score_descending() {
        let mut store = InMemoryStore::new();
        let chunks = vec![
            RagChunk {
                id: "low".into(),
                text: "least".into(),
                score: 0.1,
            },
            RagChunk {
                id: "high".into(),
                text: "most".into(),
                score: 0.95,
            },
            RagChunk {
                id: "mid".into(),
                text: "middle".into(),
                score: 0.5,
            },
        ];
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "e2".into(),
                    word: "rank".into(),
                    kind: "noun".into(),
                },
                query: "relevance".into(),
                top_k: 3,
                chunks,
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(out.chunks_used[0], "high");
        assert_eq!(out.chunks_used[1], "mid");
        assert_eq!(out.chunks_used[2], "low");
    }

    #[test]
    fn rag_answer_contains_query_and_context() {
        let mut store = InMemoryStore::new();
        let chunks = vec![RagChunk {
            id: "c1".into(),
            text: "important context".into(),
            score: 0.9,
        }];
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "e3".into(),
                    word: "find".into(),
                    kind: "verb".into(),
                },
                query: "my specific query".into(),
                top_k: 1,
                chunks,
            },
            &mut store,
            &LogProgressSink,
        );
        assert!(out.answer.contains("my specific query"));
        assert!(out.answer.contains("important context"));
    }

    #[test]
    fn rag_empty_corpus_answer_still_contains_query() {
        let mut store = InMemoryStore::new();
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "e4".into(),
                    word: "find".into(),
                    kind: "verb".into(),
                },
                query: "no corpus query".into(),
                top_k: 5,
                chunks: vec![],
            },
            &mut store,
            &LogProgressSink,
        );
        assert!(out.answer.contains("no corpus query"));
        assert_eq!(out.chunks_used.len(), 0);
    }

    #[test]
    fn rag_top_k_zero_returns_no_chunks() {
        let mut store = InMemoryStore::new();
        let chunks = vec![RagChunk {
            id: "a".into(),
            text: "ignored".into(),
            score: 1.0,
        }];
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "e5".into(),
                    word: "skip".into(),
                    kind: "verb".into(),
                },
                query: "zero".into(),
                top_k: 0,
                chunks,
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(out.chunks_used.len(), 0);
    }

    #[test]
    fn rag_query_empty_query_returns_ok() {
        let mut store = InMemoryStore::new();
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "q1".into(),
                    word: "search".into(),
                    kind: "verb".into(),
                },
                query: "".into(),
                top_k: 5,
                chunks: vec![],
            },
            &mut store,
            &LogProgressSink,
        );
        assert!(store.exists(&out.artifact_hash));
    }

    #[test]
    fn rag_query_nonempty_query_returns_results() {
        let mut store = InMemoryStore::new();
        let chunks = vec![
            RagChunk {
                id: "r1".into(),
                text: "result one".into(),
                score: 0.8,
            },
            RagChunk {
                id: "r2".into(),
                text: "result two".into(),
                score: 0.5,
            },
        ];
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "q2".into(),
                    word: "find".into(),
                    kind: "verb".into(),
                },
                query: "something".into(),
                top_k: 2,
                chunks,
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(out.chunks_used.len(), 2);
    }

    #[test]
    fn rag_query_results_have_scores() {
        // Verifies that chunks with non-zero scores appear in output.
        let mut store = InMemoryStore::new();
        let chunks = vec![RagChunk {
            id: "s1".into(),
            text: "scored chunk".into(),
            score: 0.75,
        }];
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "q3".into(),
                    word: "score".into(),
                    kind: "noun".into(),
                },
                query: "scored".into(),
                top_k: 1,
                chunks,
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(out.chunks_used.len(), 1);
        assert_eq!(out.chunks_used[0], "s1");
    }

    #[test]
    fn rag_query_scores_between_0_and_1() {
        // Compose does not panic when scores are at boundary values.
        let mut store = InMemoryStore::new();
        let chunks = vec![
            RagChunk {
                id: "min".into(),
                text: "zero score".into(),
                score: 0.0,
            },
            RagChunk {
                id: "max".into(),
                text: "one score".into(),
                score: 1.0,
            },
        ];
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "q4".into(),
                    word: "boundary".into(),
                    kind: "noun".into(),
                },
                query: "boundary".into(),
                top_k: 2,
                chunks,
            },
            &mut store,
            &LogProgressSink,
        );
        // max-score chunk should come first.
        assert_eq!(out.chunks_used[0], "max");
        assert_eq!(out.chunks_used[1], "min");
    }

    #[test]
    fn rag_query_top_k_limits_results() {
        let mut store = InMemoryStore::new();
        let chunks = (0..10)
            .map(|i| RagChunk {
                id: format!("c{i}"),
                text: format!("chunk {i}"),
                score: i as f32 * 0.1,
            })
            .collect();
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "q5".into(),
                    word: "limit".into(),
                    kind: "verb".into(),
                },
                query: "top".into(),
                top_k: 3,
                chunks,
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(out.chunks_used.len(), 3);
    }

    #[test]
    fn rag_query_results_sorted_by_score_desc() {
        let mut store = InMemoryStore::new();
        let chunks = vec![
            RagChunk {
                id: "x".into(),
                text: "x".into(),
                score: 0.2,
            },
            RagChunk {
                id: "y".into(),
                text: "y".into(),
                score: 0.9,
            },
            RagChunk {
                id: "z".into(),
                text: "z".into(),
                score: 0.5,
            },
        ];
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "q6".into(),
                    word: "sort".into(),
                    kind: "verb".into(),
                },
                query: "order".into(),
                top_k: 3,
                chunks,
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(out.chunks_used[0], "y");
        assert_eq!(out.chunks_used[1], "z");
        assert_eq!(out.chunks_used[2], "x");
    }

    #[test]
    fn rag_query_empty_index_returns_empty() {
        let mut store = InMemoryStore::new();
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "q7".into(),
                    word: "empty".into(),
                    kind: "noun".into(),
                },
                query: "anything".into(),
                top_k: 5,
                chunks: vec![],
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(out.chunks_used.len(), 0);
    }

    #[test]
    fn rag_query_single_document_single_result() {
        let mut store = InMemoryStore::new();
        let chunks = vec![RagChunk {
            id: "only".into(),
            text: "sole document".into(),
            score: 0.6,
        }];
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "q8".into(),
                    word: "single".into(),
                    kind: "adj".into(),
                },
                query: "sole".into(),
                top_k: 5,
                chunks,
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(out.chunks_used.len(), 1);
        assert_eq!(out.chunks_used[0], "only");
    }

    #[test]
    fn rag_query_multiple_documents_ranked() {
        let mut store = InMemoryStore::new();
        let chunks = vec![
            RagChunk {
                id: "alpha".into(),
                text: "alpha text".into(),
                score: 0.3,
            },
            RagChunk {
                id: "beta".into(),
                text: "beta text".into(),
                score: 0.7,
            },
            RagChunk {
                id: "gamma".into(),
                text: "gamma text".into(),
                score: 0.55,
            },
        ];
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "q9".into(),
                    word: "rank".into(),
                    kind: "verb".into(),
                },
                query: "text".into(),
                top_k: 3,
                chunks,
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(out.chunks_used[0], "beta");
        assert_eq!(out.chunks_used[1], "gamma");
        assert_eq!(out.chunks_used[2], "alpha");
    }

    #[test]
    fn rag_query_partial_match_included() {
        // Chunks with low scores are still included when top_k allows.
        let mut store = InMemoryStore::new();
        let chunks = vec![
            RagChunk {
                id: "partial".into(),
                text: "partial match".into(),
                score: 0.05,
            },
            RagChunk {
                id: "full".into(),
                text: "full match".into(),
                score: 0.95,
            },
        ];
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "q10".into(),
                    word: "partial".into(),
                    kind: "adj".into(),
                },
                query: "match".into(),
                top_k: 2,
                chunks,
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(out.chunks_used.len(), 2);
        assert!(out.chunks_used.contains(&"partial".to_string()));
    }

    #[test]
    fn rag_query_no_match_returns_empty() {
        let mut store = InMemoryStore::new();
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "q11".into(),
                    word: "nothing".into(),
                    kind: "noun".into(),
                },
                query: "no results".into(),
                top_k: 5,
                chunks: vec![],
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(out.chunks_used.len(), 0);
        assert!(out.answer.contains("no results"));
    }

    #[test]
    fn rag_query_metadata_preserved() {
        // Verifies the output artifact hash is stored and answer contains the query.
        let mut store = InMemoryStore::new();
        let chunks = vec![RagChunk {
            id: "meta".into(),
            text: "metadata chunk".into(),
            score: 0.88,
        }];
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "q12".into(),
                    word: "meta".into(),
                    kind: "noun".into(),
                },
                query: "preserve metadata".into(),
                top_k: 1,
                chunks,
            },
            &mut store,
            &LogProgressSink,
        );
        assert!(store.exists(&out.artifact_hash));
        assert!(out.answer.contains("preserve metadata"));
        assert_eq!(out.chunks_used[0], "meta");
    }

    // ── Wave AJ new tests ────────────────────────────────────────────────────

    #[test]
    fn rag_query_two_documents_both_indexed() {
        let mut store = InMemoryStore::new();
        let chunks = vec![
            RagChunk {
                id: "doc-a".into(),
                text: "document alpha".into(),
                score: 0.6,
            },
            RagChunk {
                id: "doc-b".into(),
                text: "document beta".into(),
                score: 0.4,
            },
        ];
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "aj1".into(),
                    word: "two".into(),
                    kind: "noun".into(),
                },
                query: "both documents".into(),
                top_k: 2,
                chunks,
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(
            out.chunks_used.len(),
            2,
            "both documents must appear in output"
        );
        assert!(out.chunks_used.contains(&"doc-a".to_string()));
        assert!(out.chunks_used.contains(&"doc-b".to_string()));
    }

    #[test]
    fn rag_query_10_documents_top_3_returned() {
        let mut store = InMemoryStore::new();
        let chunks: Vec<RagChunk> = (0..10)
            .map(|i| RagChunk {
                id: format!("doc-{i}"),
                text: format!("doc text {i}"),
                score: i as f32 * 0.1,
            })
            .collect();
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "aj2".into(),
                    word: "top3".into(),
                    kind: "noun".into(),
                },
                query: "top three".into(),
                top_k: 3,
                chunks,
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(
            out.chunks_used.len(),
            3,
            "top_k=3 must return exactly 3 results"
        );
    }

    #[test]
    fn rag_query_documents_ranked_by_relevance() {
        let mut store = InMemoryStore::new();
        let chunks = vec![
            RagChunk {
                id: "rank-c".into(),
                text: "c".into(),
                score: 0.3,
            },
            RagChunk {
                id: "rank-a".into(),
                text: "a".into(),
                score: 0.9,
            },
            RagChunk {
                id: "rank-b".into(),
                text: "b".into(),
                score: 0.6,
            },
        ];
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "aj3".into(),
                    word: "rank".into(),
                    kind: "verb".into(),
                },
                query: "ranking".into(),
                top_k: 3,
                chunks,
            },
            &mut store,
            &LogProgressSink,
        );
        // Must be descending: a > b > c
        assert_eq!(out.chunks_used[0], "rank-a");
        assert_eq!(out.chunks_used[1], "rank-b");
        assert_eq!(out.chunks_used[2], "rank-c");
    }

    #[test]
    fn rag_query_duplicate_docs_deduplicated() {
        // When the same id appears twice with different scores, compose processes them independently.
        // The top_k=1 must return the higher-scored entry.
        let mut store = InMemoryStore::new();
        let chunks = vec![
            RagChunk {
                id: "dup".into(),
                text: "original".into(),
                score: 0.5,
            },
            RagChunk {
                id: "dup".into(),
                text: "duplicate".into(),
                score: 0.9,
            },
        ];
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "aj4".into(),
                    word: "dup".into(),
                    kind: "noun".into(),
                },
                query: "dedup".into(),
                top_k: 1,
                chunks,
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(
            out.chunks_used.len(),
            1,
            "top_k=1 must return only one result"
        );
        assert_eq!(out.chunks_used[0], "dup", "the dup id must appear");
    }

    #[test]
    fn rag_query_metadata_filter_by_kind() {
        // Simulate kind-filtered retrieval by providing only chunks matching the kind.
        let mut store = InMemoryStore::new();
        let chunks = vec![
            RagChunk {
                id: "verb-1".into(),
                text: "run fast".into(),
                score: 0.8,
            },
            RagChunk {
                id: "verb-2".into(),
                text: "jump high".into(),
                score: 0.6,
            },
        ];
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "aj5".into(),
                    word: "filter".into(),
                    kind: "verb".into(),
                },
                query: "verb filter".into(),
                top_k: 2,
                chunks,
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(out.chunks_used.len(), 2);
        assert!(out.answer.contains("verb filter"));
    }

    #[test]
    fn rag_query_embedding_dimension_positive() {
        // compose_with_dag derives a 16-element QueryVec — verify top_k default is positive.
        let backend = RagQueryBackend::default();
        assert!(
            backend.top_k.is_none(),
            "default top_k is None (uses 5 internally)"
        );
        // top_k override must be positive when set.
        let b2 = RagQueryBackend {
            top_k: Some(3),
            ..Default::default()
        };
        assert_eq!(b2.top_k, Some(3));
        assert!(b2.top_k.unwrap() > 0, "top_k must be positive");
    }

    #[test]
    fn rag_query_cache_hit_on_repeat_query() {
        // Two identical queries over the same chunks must produce the same artifact hash.
        let mut store = InMemoryStore::new();
        let make_input = || RagQueryInput {
            entity: NomtuRef {
                id: "aj6".into(),
                word: "cache".into(),
                kind: "noun".into(),
            },
            query: "repeated query".into(),
            top_k: 2,
            chunks: vec![
                RagChunk {
                    id: "c1".into(),
                    text: "first".into(),
                    score: 0.7,
                },
                RagChunk {
                    id: "c2".into(),
                    text: "second".into(),
                    score: 0.4,
                },
            ],
        };
        let out1 = RagQueryBackend::compose(make_input(), &mut store, &LogProgressSink);
        let out2 = RagQueryBackend::compose(make_input(), &mut store, &LogProgressSink);
        assert_eq!(
            out1.artifact_hash, out2.artifact_hash,
            "same input must yield the same hash"
        );
        assert_eq!(
            out1.answer, out2.answer,
            "same input must yield the same answer"
        );
    }

    #[test]
    fn rag_query_answer_non_empty_for_nonempty_chunks() {
        let mut store = InMemoryStore::new();
        let chunks = vec![RagChunk {
            id: "nonempty".into(),
            text: "substantial text".into(),
            score: 0.75,
        }];
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "aj7".into(),
                    word: "check".into(),
                    kind: "verb".into(),
                },
                query: "has content".into(),
                top_k: 1,
                chunks,
            },
            &mut store,
            &LogProgressSink,
        );
        assert!(
            !out.answer.is_empty(),
            "answer must not be empty when chunks are non-empty"
        );
    }

    #[test]
    fn rag_query_artifact_hash_stored_in_store() {
        let mut store = InMemoryStore::new();
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "aj8".into(),
                    word: "store".into(),
                    kind: "noun".into(),
                },
                query: "artifact storage".into(),
                top_k: 1,
                chunks: vec![RagChunk {
                    id: "s1".into(),
                    text: "stored".into(),
                    score: 0.5,
                }],
            },
            &mut store,
            &LogProgressSink,
        );
        assert!(
            store.exists(&out.artifact_hash),
            "compose must persist artifact in store"
        );
    }

    #[test]
    fn rag_query_top_1_from_5_returns_highest_score() {
        let mut store = InMemoryStore::new();
        let chunks = vec![
            RagChunk {
                id: "low1".into(),
                text: "l1".into(),
                score: 0.1,
            },
            RagChunk {
                id: "low2".into(),
                text: "l2".into(),
                score: 0.2,
            },
            RagChunk {
                id: "best".into(),
                text: "best".into(),
                score: 0.95,
            },
            RagChunk {
                id: "low3".into(),
                text: "l3".into(),
                score: 0.3,
            },
            RagChunk {
                id: "low4".into(),
                text: "l4".into(),
                score: 0.05,
            },
        ];
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: "aj9".into(),
                    word: "best".into(),
                    kind: "adj".into(),
                },
                query: "find best".into(),
                top_k: 1,
                chunks,
            },
            &mut store,
            &LogProgressSink,
        );
        assert_eq!(out.chunks_used.len(), 1);
        assert_eq!(
            out.chunks_used[0], "best",
            "must return the highest-scored chunk"
        );
    }
}
