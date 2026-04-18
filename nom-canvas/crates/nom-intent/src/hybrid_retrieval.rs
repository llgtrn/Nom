#![deny(unsafe_code)]

use std::collections::HashMap;

/// Combined BM25 + vector retrieval score for a single document.
#[derive(Debug, Clone)]
pub struct RetrievalScore {
    pub doc_id: u64,
    pub bm25_score: f32,
    pub vector_score: f32,
}

impl RetrievalScore {
    pub fn new(doc_id: u64, bm25_score: f32, vector_score: f32) -> Self {
        Self { doc_id, bm25_score, vector_score }
    }

    /// Weighted combination: bm25_weight * bm25 + (1 - bm25_weight) * vector.
    pub fn hybrid_score(&self, bm25_weight: f32) -> f32 {
        bm25_weight * self.bm25_score + (1.0 - bm25_weight) * self.vector_score
    }
}

/// Retriever that merges BM25 and vector scores into a single ranked list.
pub struct HybridRetriever {
    pub bm25_weight: f32,
}

impl HybridRetriever {
    /// Create with a custom BM25 weight (default 0.6).
    pub fn new(bm25_weight: f32) -> Self {
        Self { bm25_weight }
    }

    /// Merge BM25 and vector score lists by doc_id.
    /// Documents present in only one list receive 0.0 for the missing score.
    pub fn merge_scores(bm25: &[(u64, f32)], vector: &[(u64, f32)]) -> Vec<RetrievalScore> {
        let mut map: HashMap<u64, (f32, f32)> = HashMap::new();

        for &(id, score) in bm25 {
            map.entry(id).or_insert((0.0, 0.0)).0 = score;
        }
        for &(id, score) in vector {
            map.entry(id).or_insert((0.0, 0.0)).1 = score;
        }

        let mut scores: Vec<RetrievalScore> = map
            .into_iter()
            .map(|(id, (b, v))| RetrievalScore::new(id, b, v))
            .collect();

        // stable sort by doc_id for deterministic ordering
        scores.sort_by_key(|s| s.doc_id);
        scores
    }

    /// Return the top-k doc IDs sorted by hybrid score descending.
    pub fn top_k(scores: &[RetrievalScore], k: usize, bm25_weight: f32) -> Vec<u64> {
        let mut indexed: Vec<(usize, f32)> = scores
            .iter()
            .enumerate()
            .map(|(i, s)| (i, s.hybrid_score(bm25_weight)))
            .collect();

        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        indexed.iter().take(k).map(|&(i, _)| scores[i].doc_id).collect()
    }

    /// Reciprocal Rank Fusion over two ranked lists.
    /// Score = sum of 1/(k + rank) across lists (1-based rank).
    /// Returns doc IDs sorted by fused score descending.
    pub fn reciprocal_rank_fusion(
        bm25_ranks: &[u64],
        vector_ranks: &[u64],
        k: u32,
    ) -> Vec<(u64, f32)> {
        let mut scores: HashMap<u64, f32> = HashMap::new();

        for (rank, &id) in bm25_ranks.iter().enumerate() {
            *scores.entry(id).or_insert(0.0) += 1.0 / (k as f32 + (rank as f32 + 1.0));
        }
        for (rank, &id) in vector_ranks.iter().enumerate() {
            *scores.entry(id).or_insert(0.0) += 1.0 / (k as f32 + (rank as f32 + 1.0));
        }

        let mut result: Vec<(u64, f32)> = scores.into_iter().collect();
        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        result
    }
}

impl Default for HybridRetriever {
    fn default() -> Self {
        Self::new(0.6)
    }
}

#[cfg(test)]
mod hybrid_retrieval_tests {
    use super::*;

    // Test 1: hybrid_score() basic calculation
    #[test]
    fn test_hybrid_score_calculation() {
        let s = RetrievalScore::new(1, 0.8, 0.4);
        let result = s.hybrid_score(0.5);
        let expected = 0.5 * 0.8 + 0.5 * 0.4;
        assert!((result - expected).abs() < 1e-6);
    }

    // Test 2: weight=1.0 returns bm25 only
    #[test]
    fn test_hybrid_score_weight_one_returns_bm25_only() {
        let s = RetrievalScore::new(2, 0.9, 0.3);
        let result = s.hybrid_score(1.0);
        assert!((result - 0.9).abs() < 1e-6);
    }

    // Test 3: merge_scores handles docs only in bm25
    #[test]
    fn test_merge_scores_bm25_only_docs() {
        let bm25 = vec![(10u64, 0.7)];
        let vector: Vec<(u64, f32)> = vec![];
        let scores = HybridRetriever::merge_scores(&bm25, &vector);
        assert_eq!(scores.len(), 1);
        assert_eq!(scores[0].doc_id, 10);
        assert!((scores[0].bm25_score - 0.7).abs() < 1e-6);
        assert!((scores[0].vector_score - 0.0).abs() < 1e-6);
    }

    // Test 4: merge_scores handles docs only in vector
    #[test]
    fn test_merge_scores_vector_only_docs() {
        let bm25: Vec<(u64, f32)> = vec![];
        let vector = vec![(20u64, 0.5)];
        let scores = HybridRetriever::merge_scores(&bm25, &vector);
        assert_eq!(scores.len(), 1);
        assert_eq!(scores[0].doc_id, 20);
        assert!((scores[0].bm25_score - 0.0).abs() < 1e-6);
        assert!((scores[0].vector_score - 0.5).abs() < 1e-6);
    }

    // Test 5: merge_scores combines scores for shared docs
    #[test]
    fn test_merge_scores_shared_docs() {
        let bm25 = vec![(5u64, 0.6)];
        let vector = vec![(5u64, 0.8)];
        let scores = HybridRetriever::merge_scores(&bm25, &vector);
        assert_eq!(scores.len(), 1);
        assert!((scores[0].bm25_score - 0.6).abs() < 1e-6);
        assert!((scores[0].vector_score - 0.8).abs() < 1e-6);
    }

    // Test 6: top_k returns correct count
    #[test]
    fn test_top_k_returns_correct_count() {
        let scores = vec![
            RetrievalScore::new(1, 0.9, 0.1),
            RetrievalScore::new(2, 0.5, 0.5),
            RetrievalScore::new(3, 0.1, 0.9),
            RetrievalScore::new(4, 0.3, 0.3),
        ];
        let result = HybridRetriever::top_k(&scores, 2, 0.6);
        assert_eq!(result.len(), 2);
    }

    // Test 7: top_k sorts by hybrid score descending
    #[test]
    fn test_top_k_sorts_descending() {
        let scores = vec![
            RetrievalScore::new(1, 0.1, 0.1), // hybrid = 0.1
            RetrievalScore::new(2, 1.0, 1.0), // hybrid = 1.0
            RetrievalScore::new(3, 0.5, 0.5), // hybrid = 0.5
        ];
        let result = HybridRetriever::top_k(&scores, 3, 0.6);
        assert_eq!(result[0], 2);
        assert_eq!(result[1], 3);
        assert_eq!(result[2], 1);
    }

    // Test 8: reciprocal_rank_fusion ranks shared docs higher
    #[test]
    fn test_rrf_shared_docs_rank_higher() {
        // doc 100 appears in both lists at rank 1 — should score highest
        let bm25_ranks = vec![100u64, 200];
        let vector_ranks = vec![100u64, 300];
        let result = HybridRetriever::reciprocal_rank_fusion(&bm25_ranks, &vector_ranks, 60);
        assert!(!result.is_empty());
        assert_eq!(result[0].0, 100, "shared doc should rank first");
    }

    // Test 9: reciprocal_rank_fusion handles empty list
    #[test]
    fn test_rrf_empty_list() {
        let bm25_ranks: Vec<u64> = vec![];
        let vector_ranks = vec![1u64, 2, 3];
        let result = HybridRetriever::reciprocal_rank_fusion(&bm25_ranks, &vector_ranks, 60);
        assert_eq!(result.len(), 3);
    }
}
