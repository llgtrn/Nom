/// Reranker and postprocessor for RAG pipelines.
/// A candidate document to be reranked.
#[derive(Debug, Clone)]
pub struct RerankCandidate {
    pub doc_id: u64,
    pub text: String,
    pub initial_score: f32,
    pub rerank_score: f32,
}

impl RerankCandidate {
    pub fn new(doc_id: u64, text: impl Into<String>, initial_score: f32) -> Self {
        Self {
            doc_id,
            text: text.into(),
            initial_score,
            rerank_score: initial_score,
        }
    }

    pub fn apply_rerank(&mut self, new_score: f32) {
        self.rerank_score = new_score;
    }
}

/// Reranking strategy determining the scoring model used.
#[derive(Debug, Clone, PartialEq)]
pub enum RerankStrategy {
    CrossEncoder,
    MonoT5,
    SentenceTransformer,
}

impl RerankStrategy {
    pub fn strategy_name(&self) -> &str {
        match self {
            RerankStrategy::CrossEncoder => "CrossEncoder",
            RerankStrategy::MonoT5 => "MonoT5",
            RerankStrategy::SentenceTransformer => "SentenceTransformer",
        }
    }

    pub fn score_boost(&self) -> f32 {
        match self {
            RerankStrategy::CrossEncoder => 1.2,
            RerankStrategy::MonoT5 => 1.1,
            RerankStrategy::SentenceTransformer => 1.05,
        }
    }
}

/// Reranker applies a scoring strategy to a list of candidates.
pub struct Reranker {
    pub strategy: RerankStrategy,
}

impl Reranker {
    pub fn new(strategy: RerankStrategy) -> Self {
        Self { strategy }
    }

    pub fn rerank(&self, mut candidates: Vec<RerankCandidate>, query: &str) -> Vec<RerankCandidate> {
        let query_factor = 1.0 + 0.1 * query.len().min(10) as f32 / 10.0;
        let boost = self.strategy.score_boost();
        for candidate in &mut candidates {
            let new_score = candidate.initial_score * boost * query_factor;
            candidate.apply_rerank(new_score);
        }
        candidates.sort_by(|a, b| b.rerank_score.partial_cmp(&a.rerank_score).unwrap());
        candidates
    }

    pub fn top_n(&self, candidates: Vec<RerankCandidate>, n: usize) -> Vec<RerankCandidate> {
        let reranked = self.rerank(candidates, "");
        reranked.into_iter().take(n).collect()
    }
}

/// PostProcessor filters and deduplicates reranked candidates.
pub struct PostProcessor {
    pub threshold: f32,
}

impl PostProcessor {
    pub fn new(threshold: f32) -> Self {
        Self { threshold }
    }

    pub fn filter(&self, candidates: Vec<RerankCandidate>) -> Vec<RerankCandidate> {
        candidates
            .into_iter()
            .filter(|c| c.rerank_score >= self.threshold)
            .collect()
    }

    pub fn deduplicate(candidates: Vec<RerankCandidate>) -> Vec<RerankCandidate> {
        let mut seen: std::collections::HashMap<u64, RerankCandidate> =
            std::collections::HashMap::new();
        for candidate in candidates {
            seen.entry(candidate.doc_id)
                .and_modify(|existing| {
                    if candidate.rerank_score > existing.rerank_score {
                        *existing = candidate.clone();
                    }
                })
                .or_insert(candidate);
        }
        let mut result: Vec<RerankCandidate> = seen.into_values().collect();
        result.sort_by(|a, b| b.rerank_score.partial_cmp(&a.rerank_score).unwrap());
        result
    }
}

#[cfg(test)]
mod reranker_tests {
    use super::*;

    #[test]
    fn test_new_initial_equals_rerank_score() {
        let c = RerankCandidate::new(1, "hello", 0.5);
        assert_eq!(c.initial_score, c.rerank_score);
        assert_eq!(c.rerank_score, 0.5);
    }

    #[test]
    fn test_apply_rerank_changes_score() {
        let mut c = RerankCandidate::new(1, "hello", 0.5);
        c.apply_rerank(0.9);
        assert_eq!(c.rerank_score, 0.9);
        assert_eq!(c.initial_score, 0.5);
    }

    #[test]
    fn test_strategy_score_boost() {
        assert_eq!(RerankStrategy::CrossEncoder.score_boost(), 1.2);
        assert_eq!(RerankStrategy::MonoT5.score_boost(), 1.1);
        assert_eq!(RerankStrategy::SentenceTransformer.score_boost(), 1.05);
    }

    #[test]
    fn test_rerank_sorts_by_score_desc() {
        let candidates = vec![
            RerankCandidate::new(1, "low", 0.1),
            RerankCandidate::new(2, "high", 0.9),
            RerankCandidate::new(3, "mid", 0.5),
        ];
        let reranker = Reranker::new(RerankStrategy::CrossEncoder);
        let result = reranker.rerank(candidates, "q");
        assert!(result[0].rerank_score >= result[1].rerank_score);
        assert!(result[1].rerank_score >= result[2].rerank_score);
    }

    #[test]
    fn test_rerank_applies_boost() {
        let candidates = vec![RerankCandidate::new(1, "doc", 1.0)];
        let reranker = Reranker::new(RerankStrategy::CrossEncoder);
        // query length = 0, query_factor = 1.0 + 0.1 * 0 / 10 = 1.0
        let result = reranker.rerank(candidates, "");
        // new_score = 1.0 * 1.2 * 1.0 = 1.2
        assert!((result[0].rerank_score - 1.2).abs() < 1e-5);
    }

    #[test]
    fn test_top_n_returns_correct_count() {
        let candidates = vec![
            RerankCandidate::new(1, "a", 0.1),
            RerankCandidate::new(2, "b", 0.5),
            RerankCandidate::new(3, "c", 0.9),
            RerankCandidate::new(4, "d", 0.3),
        ];
        let reranker = Reranker::new(RerankStrategy::MonoT5);
        let result = reranker.top_n(candidates, 2);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_filter_removes_below_threshold() {
        let candidates = vec![
            RerankCandidate::new(1, "low", 0.3),
            RerankCandidate::new(2, "high", 0.8),
            RerankCandidate::new(3, "mid", 0.5),
        ];
        // set rerank_score manually via new (initial == rerank)
        let pp = PostProcessor::new(0.5);
        let result = pp.filter(candidates);
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|c| c.rerank_score >= 0.5));
    }

    #[test]
    fn test_deduplicate_keeps_highest_score_per_doc_id() {
        let mut c1 = RerankCandidate::new(1, "first", 0.3);
        let mut c2 = RerankCandidate::new(1, "second", 0.9);
        c1.apply_rerank(0.3);
        c2.apply_rerank(0.9);
        let candidates = vec![c1, c2];
        let result = PostProcessor::deduplicate(candidates);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].doc_id, 1);
        assert!((result[0].rerank_score - 0.9).abs() < 1e-5);
    }

    #[test]
    fn test_deduplicate_no_duplicates_unchanged() {
        let candidates = vec![
            RerankCandidate::new(1, "a", 0.5),
            RerankCandidate::new(2, "b", 0.7),
            RerankCandidate::new(3, "c", 0.3),
        ];
        let result = PostProcessor::deduplicate(candidates);
        assert_eq!(result.len(), 3);
    }
}
