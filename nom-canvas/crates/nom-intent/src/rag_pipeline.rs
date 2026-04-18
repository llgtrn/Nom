/// RAG pipeline — LlamaIndex/Refly pattern.
///
/// Components:
///   RagQuery      — a query with embedding parameters and filters
///   RagDocument   — a retrieved document with a relevance score
///   RagRetriever  — retrieves candidate documents from an in-memory store
///   RagSynthesis  — the synthesised answer produced from retrieved documents
///   RagPipeline   — full pipeline: retrieve → synthesize

/// A query issued against the RAG retriever.
#[derive(Debug, Clone)]
pub struct RagQuery {
    pub text: String,
    pub top_k: usize,
    pub min_score: f32,
}

impl RagQuery {
    /// Create a new query.  `min_score` defaults to 0.0 (no filtering).
    pub fn new(text: impl Into<String>, top_k: usize) -> Self {
        Self {
            text: text.into(),
            top_k,
            min_score: 0.0,
        }
    }

    /// Builder: set a minimum score threshold.
    pub fn with_min_score(mut self, min_score: f32) -> Self {
        self.min_score = min_score;
        self
    }

    /// Returns `true` when a score filter is active (min_score > 0.0).
    pub fn is_filtered(&self) -> bool {
        self.min_score > 0.0
    }
}

/// A single document in the retrieval store, carrying a relevance score.
#[derive(Debug, Clone)]
pub struct RagDocument {
    pub id: u64,
    pub content: String,
    pub score: f32,
}

impl RagDocument {
    pub fn new(id: u64, content: impl Into<String>, score: f32) -> Self {
        Self {
            id,
            content: content.into(),
            score,
        }
    }

    /// Returns `true` when this document's score meets the threshold.
    pub fn passes_threshold(&self, min_score: f32) -> bool {
        self.score >= min_score
    }
}

/// In-memory document store with retrieval support.
#[derive(Debug, Default)]
pub struct RagRetriever {
    documents: Vec<RagDocument>,
}

impl RagRetriever {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_doc(&mut self, doc: RagDocument) {
        self.documents.push(doc);
    }

    /// Retrieve up to `query.top_k` documents whose score >= `query.min_score`,
    /// returned in descending score order.
    pub fn retrieve<'a>(&'a self, query: &RagQuery) -> Vec<&'a RagDocument> {
        let mut candidates: Vec<&RagDocument> = self
            .documents
            .iter()
            .filter(|d| d.score >= query.min_score)
            .collect();

        // Sort descending by score.
        candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(query.top_k);
        candidates
    }

    pub fn doc_count(&self) -> usize {
        self.documents.len()
    }
}

/// The synthesised answer produced from a set of retrieved documents.
#[derive(Debug, Clone)]
pub struct RagSynthesis {
    pub query_text: String,
    pub source_count: usize,
    pub answer: String,
}

impl RagSynthesis {
    pub fn new(query_text: impl Into<String>, sources: &[&RagDocument]) -> Self {
        let source_count = sources.len();
        Self {
            query_text: query_text.into(),
            source_count,
            answer: format!("synthesized from {} sources", source_count),
        }
    }
}

/// Full RAG pipeline: retrieve documents then synthesise an answer.
pub struct RagPipeline {
    retriever: RagRetriever,
}

impl RagPipeline {
    pub fn new(retriever: RagRetriever) -> Self {
        Self { retriever }
    }

    /// Run the pipeline for the given query.
    pub fn run(&self, query: RagQuery) -> RagSynthesis {
        let query_text = query.text.clone();
        let docs = self.retriever.retrieve(&query);
        RagSynthesis::new(query_text, &docs)
    }

    pub fn retriever_doc_count(&self) -> usize {
        self.retriever.doc_count()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod rag_pipeline_tests {
    use super::*;

    #[test]
    fn rag_query_is_filtered_false() {
        let q = RagQuery::new("what is Nom?", 5);
        assert!(!q.is_filtered());
    }

    #[test]
    fn rag_query_is_filtered_true() {
        let q = RagQuery::new("what is Nom?", 5).with_min_score(0.5);
        assert!(q.is_filtered());
    }

    #[test]
    fn rag_document_passes_threshold() {
        let doc = RagDocument::new(1, "hello world", 0.8);
        assert!(doc.passes_threshold(0.5));
        assert!(doc.passes_threshold(0.8));
        assert!(!doc.passes_threshold(0.9));
    }

    #[test]
    fn rag_retriever_retrieve_top_k() {
        let mut r = RagRetriever::new();
        for i in 0..10 {
            r.add_doc(RagDocument::new(i, format!("doc {i}"), i as f32 * 0.1));
        }
        let q = RagQuery::new("query", 3);
        let results = r.retrieve(&q);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn rag_retriever_retrieve_filters_by_score() {
        let mut r = RagRetriever::new();
        r.add_doc(RagDocument::new(1, "low", 0.2));
        r.add_doc(RagDocument::new(2, "mid", 0.5));
        r.add_doc(RagDocument::new(3, "high", 0.9));
        let q = RagQuery::new("query", 10).with_min_score(0.5);
        let results = r.retrieve(&q);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|d| d.score >= 0.5));
    }

    #[test]
    fn rag_retriever_retrieve_sorted_by_score() {
        let mut r = RagRetriever::new();
        r.add_doc(RagDocument::new(1, "a", 0.3));
        r.add_doc(RagDocument::new(2, "b", 0.9));
        r.add_doc(RagDocument::new(3, "c", 0.6));
        let q = RagQuery::new("query", 10);
        let results = r.retrieve(&q);
        let scores: Vec<f32> = results.iter().map(|d| d.score).collect();
        assert!(scores.windows(2).all(|w| w[0] >= w[1]), "expected descending order");
    }

    #[test]
    fn rag_synthesis_source_count() {
        let d1 = RagDocument::new(1, "x", 0.8);
        let d2 = RagDocument::new(2, "y", 0.7);
        let sources = vec![&d1, &d2];
        let s = RagSynthesis::new("q", &sources);
        assert_eq!(s.source_count, 2);
        assert_eq!(s.answer, "synthesized from 2 sources");
    }

    #[test]
    fn rag_pipeline_run_returns_synthesis() {
        let mut r = RagRetriever::new();
        r.add_doc(RagDocument::new(1, "doc a", 0.9));
        r.add_doc(RagDocument::new(2, "doc b", 0.7));
        let p = RagPipeline::new(r);
        let result = p.run(RagQuery::new("test", 5));
        assert_eq!(result.source_count, 2);
        assert_eq!(result.query_text, "test");
    }

    #[test]
    fn rag_pipeline_run_empty_retriever() {
        let r = RagRetriever::new();
        let p = RagPipeline::new(r);
        let result = p.run(RagQuery::new("empty", 5));
        assert_eq!(result.source_count, 0);
        assert_eq!(result.answer, "synthesized from 0 sources");
    }
}
