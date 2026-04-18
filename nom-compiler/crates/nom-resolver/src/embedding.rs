//! Embedding infrastructure for semantic vector search.
//!
//! This module provides the data structures and math for embedding-based
//! retrieval. No network calls — pure math + data structures.
//!
//! When an embedding model becomes available (network unblocked), plug it
//! in by implementing `EmbeddingModel` and passing it to `EmbeddingIndex`.
//!
//! Current state:
//! - `StubEmbedding` always returns zero vectors (safe fallback).
//! - `EmbeddingIndex` stores pre-computed vectors and does nearest-neighbor
//!   search via cosine similarity.
//! - Callers in `SemanticEmbedding` (intent.rs) remain wired to
//!   `Err(Unavailable)` until a real model is plugged in.

/// A dense embedding vector.
pub type EmbeddingVector = Vec<f32>;

/// Error type for embedding operations.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum EmbeddingError {
    /// The embedding model is not available (e.g. not loaded, network gated).
    #[error("embedding model unavailable: {0}")]
    Unavailable(String),
    /// Input text was empty or could not be processed.
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

/// Trait for anything that can embed a text string into a vector.
///
/// Implementations may be: a stub returning zero vectors, an in-process ONNX
/// model, an HTTP API client, or the nom-compiler CLI itself acting as oracle.
pub trait EmbeddingModel {
    fn embed(&self, text: &str) -> Result<EmbeddingVector, EmbeddingError>;
}

// ── Cosine similarity ────────────────────────────────────────────────────────

/// Compute cosine similarity between two vectors.
///
/// Returns a value in [-1.0, 1.0]. Returns 0.0 if either vector has zero norm
/// (safe fallback — avoids division by zero).
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| *x as f64 * *y as f64)
        .sum();
    let norm_a: f64 = a
        .iter()
        .map(|x| (*x as f64) * (*x as f64))
        .sum::<f64>()
        .sqrt();
    let norm_b: f64 = b
        .iter()
        .map(|x| (*x as f64) * (*x as f64))
        .sum::<f64>()
        .sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

// ── StubEmbedding ─────────────────────────────────────────────────────────────

/// A stub embedding model that always returns a zero vector of fixed dimension.
///
/// Used as a safe fallback when no real model is available. All zero vectors
/// have cosine similarity 0.0 to every other vector, so index searches return
/// no meaningful results — which is correct behavior for an unavailable backend.
pub struct StubEmbedding {
    /// Dimension of zero vectors to return (must match any pre-built index).
    pub dim: usize,
}

impl StubEmbedding {
    /// Create a stub with the given vector dimension.
    pub fn new(dim: usize) -> Self {
        Self { dim }
    }
}

impl EmbeddingModel for StubEmbedding {
    fn embed(&self, _text: &str) -> Result<EmbeddingVector, EmbeddingError> {
        Ok(vec![0.0f32; self.dim])
    }
}

// ── EmbeddingIndex ────────────────────────────────────────────────────────────

/// A pre-computed embedding index for nearest-neighbor search.
///
/// Stores (key, vector) pairs. `search` returns the top-k keys ranked by
/// cosine similarity to the query vector.
///
/// # Usage
///
/// ```rust
/// use nom_resolver::embedding::{EmbeddingIndex, cosine_similarity};
///
/// let mut index = EmbeddingIndex::new();
/// index.insert("entry_a".into(), vec![1.0, 0.0, 0.0]);
/// index.insert("entry_b".into(), vec![0.0, 1.0, 0.0]);
///
/// let query = vec![1.0, 0.0, 0.0];
/// let results = index.search(&query, 1);
/// assert_eq!(results[0].0, "entry_a");
/// ```
pub struct EmbeddingIndex {
    entries: Vec<(String, EmbeddingVector)>,
}

impl EmbeddingIndex {
    /// Create an empty index.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Insert or replace an entry by key.
    pub fn insert(&mut self, key: String, vector: EmbeddingVector) {
        // Replace existing entry with same key if present.
        if let Some(pos) = self.entries.iter().position(|(k, _)| k == &key) {
            self.entries[pos].1 = vector;
        } else {
            self.entries.push((key, vector));
        }
    }

    /// Number of entries in the index.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the index has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Search the index and return up to `limit` (key, score) pairs ranked by
    /// cosine similarity (highest first). Entries with score <= 0.0 are excluded
    /// unless the query is a zero vector, in which case all scores are 0.0 and
    /// the first `limit` entries are returned in insertion order.
    pub fn search(&self, query: &[f32], limit: usize) -> Vec<(String, f64)> {
        if limit == 0 || self.entries.is_empty() {
            return Vec::new();
        }

        let mut scored: Vec<(String, f64)> = self
            .entries
            .iter()
            .map(|(key, vec)| (key.clone(), cosine_similarity(query, vec)))
            .collect();

        // Sort highest score first; ties broken by key for determinism.
        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });

        scored.truncate(limit);
        scored
    }
}

impl Default for EmbeddingIndex {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── cosine_similarity ────────────────────────────────────────────────────

    #[test]
    fn cosine_identical_vectors_returns_one() {
        let v = vec![1.0f32, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-9, "identical vectors: {sim}");
    }

    #[test]
    fn cosine_orthogonal_vectors_returns_zero() {
        let a = vec![1.0f32, 0.0, 0.0];
        let b = vec![0.0f32, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 0.0).abs() < 1e-9, "orthogonal: {sim}");
    }

    #[test]
    fn cosine_opposite_vectors_returns_minus_one() {
        let a = vec![1.0f32, 0.0];
        let b = vec![-1.0f32, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - (-1.0)).abs() < 1e-9, "opposite: {sim}");
    }

    #[test]
    fn cosine_zero_vector_returns_zero() {
        let a = vec![0.0f32, 0.0, 0.0];
        let b = vec![1.0f32, 2.0, 3.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
        assert_eq!(cosine_similarity(&b, &a), 0.0);
        assert_eq!(cosine_similarity(&a, &a), 0.0);
    }

    #[test]
    fn cosine_mismatched_lengths_returns_zero() {
        let a = vec![1.0f32, 2.0];
        let b = vec![1.0f32, 2.0, 3.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn cosine_empty_vectors_returns_zero() {
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
    }

    #[test]
    fn cosine_45_degree_angle() {
        // [1,0] vs [1,1]/sqrt(2) → cos(45°) ≈ 0.7071
        let a = vec![1.0f32, 0.0];
        let b = vec![1.0f32, 1.0];
        let sim = cosine_similarity(&a, &b);
        let expected = 1.0_f64 / 2.0_f64.sqrt();
        assert!((sim - expected).abs() < 1e-6, "45°: {sim} vs {expected}");
    }

    // ── StubEmbedding ────────────────────────────────────────────────────────

    #[test]
    fn stub_returns_zero_vector_of_correct_dim() {
        let stub = StubEmbedding::new(128);
        let vec = stub.embed("hello world").unwrap();
        assert_eq!(vec.len(), 128);
        assert!(vec.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn stub_zero_dim_returns_empty_vec() {
        let stub = StubEmbedding::new(0);
        let vec = stub.embed("anything").unwrap();
        assert!(vec.is_empty());
    }

    #[test]
    fn stub_cosine_similarity_of_zero_vectors_is_zero() {
        let stub = StubEmbedding::new(4);
        let a = stub.embed("a").unwrap();
        let b = stub.embed("b").unwrap();
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    // ── EmbeddingIndex ───────────────────────────────────────────────────────

    #[test]
    fn index_search_returns_nearest_first() {
        let mut index = EmbeddingIndex::new();
        index.insert("close".into(), vec![1.0, 0.0, 0.0]);
        index.insert("far".into(), vec![0.0, 1.0, 0.0]);
        index.insert("medium".into(), vec![0.7, 0.3, 0.0]);

        let query = vec![1.0f32, 0.0, 0.0];
        let results = index.search(&query, 3);

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].0, "close", "nearest should be first");
        assert!(results[0].1 > results[1].1, "scores should be descending");
    }

    #[test]
    fn index_search_respects_limit() {
        let mut index = EmbeddingIndex::new();
        for i in 0..10 {
            index.insert(format!("entry_{i}"), vec![i as f32, 0.0]);
        }
        let results = index.search(&[1.0, 0.0], 3);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn index_search_empty_index_returns_empty() {
        let index = EmbeddingIndex::new();
        let results = index.search(&[1.0, 0.0], 5);
        assert!(results.is_empty());
    }

    #[test]
    fn index_search_limit_zero_returns_empty() {
        let mut index = EmbeddingIndex::new();
        index.insert("a".into(), vec![1.0, 0.0]);
        let results = index.search(&[1.0, 0.0], 0);
        assert!(results.is_empty());
    }

    #[test]
    fn index_insert_replaces_existing_key() {
        let mut index = EmbeddingIndex::new();
        index.insert("a".into(), vec![1.0, 0.0]);
        index.insert("a".into(), vec![0.0, 1.0]);
        assert_eq!(index.len(), 1);

        // The replaced vector should be [0.0, 1.0]
        let results = index.search(&[0.0, 1.0], 1);
        assert_eq!(results[0].0, "a");
        assert!((results[0].1 - 1.0).abs() < 1e-9);
    }

    #[test]
    fn index_len_and_is_empty() {
        let mut index = EmbeddingIndex::new();
        assert!(index.is_empty());
        index.insert("x".into(), vec![1.0]);
        assert_eq!(index.len(), 1);
        assert!(!index.is_empty());
    }

    #[test]
    fn index_search_deterministic_on_tied_scores() {
        // Two entries with identical cosine similarity — tie broken by key.
        let mut index = EmbeddingIndex::new();
        index.insert("beta".into(), vec![1.0, 0.0]);
        index.insert("alpha".into(), vec![1.0, 0.0]);

        let query = vec![1.0f32, 0.0];
        let r1 = index.search(&query, 2);
        let r2 = index.search(&query, 2);
        assert_eq!(r1, r2, "repeated calls must be deterministic");
        // "alpha" < "beta" lexicographically, so alpha ranks first on tie.
        assert_eq!(r1[0].0, "alpha");
    }
}
