#![deny(unsafe_code)]

/// A single document in the BM25 corpus.
#[derive(Debug, Clone)]
pub struct BM25Document {
    pub id: String,
    pub content: String,
    pub tokens: Vec<String>,
}

/// BM25 Okapi retriever over an in-memory corpus.
pub struct BM25Retriever {
    /// Term-frequency saturation parameter (default 1.5).
    pub k1: f32,
    /// Length normalisation parameter (default 0.75).
    pub b: f32,
    pub documents: Vec<BM25Document>,
}

impl BM25Retriever {
    /// Create a new retriever with k1=1.5, b=0.75 and an empty corpus.
    pub fn new() -> Self {
        Self {
            k1: 1.5,
            b: 0.75,
            documents: vec![],
        }
    }

    /// Tokenise `content` and add it to the corpus.
    pub fn add_document(&mut self, id: &str, content: &str) {
        let tokens = Self::tokenize(content);
        self.documents.push(BM25Document {
            id: id.to_string(),
            content: content.to_string(),
            tokens,
        });
    }

    /// Compute the BM25 score of `doc` against `query`.
    pub fn score(&self, query: &str, doc: &BM25Document) -> f32 {
        let query_tokens = Self::tokenize(query);
        let n = self.documents.len() as f32;
        let avg_dl = self.avg_doc_length();
        let dl = doc.tokens.len() as f32;

        query_tokens
            .iter()
            .map(|term| {
                // document frequency
                let df = self
                    .documents
                    .iter()
                    .filter(|d| d.tokens.contains(term))
                    .count() as f32;

                // term frequency in this document
                let tf = doc.tokens.iter().filter(|t| *t == term).count() as f32;

                if df == 0.0 || tf == 0.0 {
                    return 0.0;
                }

                // IDF (add 1 smoothing)
                let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();

                // Normalised TF
                let tf_norm = (tf * (self.k1 + 1.0))
                    / (tf + self.k1 * (1.0 - self.b + self.b * dl / avg_dl.max(1.0)));

                idf * tf_norm
            })
            .sum()
    }

    /// Return up to `top_k` documents sorted by BM25 score (descending).
    pub fn retrieve(&self, query: &str, top_k: usize) -> Vec<(&BM25Document, f32)> {
        let mut scored: Vec<(&BM25Document, f32)> = self
            .documents
            .iter()
            .map(|doc| (doc, self.score(query, doc)))
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored
    }

    /// Number of documents in the corpus.
    pub fn document_count(&self) -> usize {
        self.documents.len()
    }

    /// Split on whitespace and lowercase every token.
    fn tokenize(text: &str) -> Vec<String> {
        text.split_whitespace().map(|t| t.to_lowercase()).collect()
    }

    /// Average document length (in tokens) across the corpus.
    fn avg_doc_length(&self) -> f32 {
        if self.documents.is_empty() {
            return 0.0;
        }
        let total: usize = self.documents.iter().map(|d| d.tokens.len()).sum();
        total as f32 / self.documents.len() as f32
    }
}

impl Default for BM25Retriever {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Cosine-similarity retriever
// ---------------------------------------------------------------------------

/// A single document with a fixed-dimension embedding vector.
#[derive(Debug, Clone)]
pub struct VectorDocument {
    pub id: String,
    pub embedding: Vec<f32>,
}

/// Retriever that ranks documents by cosine similarity to a query embedding.
pub struct CosineSimilarityRetriever {
    pub documents: Vec<VectorDocument>,
}

impl CosineSimilarityRetriever {
    /// Create an empty retriever.
    pub fn new() -> Self {
        Self { documents: vec![] }
    }

    /// Add a document with a pre-computed embedding.
    pub fn add_document(&mut self, id: &str, embedding: Vec<f32>) {
        self.documents.push(VectorDocument {
            id: id.to_string(),
            embedding,
        });
    }

    /// Cosine similarity: dot(a, b) / (|a| * |b|).
    /// Returns 0.0 if either vector has zero norm.
    pub fn similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        dot / (norm_a * norm_b)
    }

    /// Return up to `top_k` documents sorted by cosine similarity (descending).
    pub fn retrieve(&self, query_embedding: &[f32], top_k: usize) -> Vec<(&VectorDocument, f32)> {
        let mut scored: Vec<(&VectorDocument, f32)> = self
            .documents
            .iter()
            .map(|doc| (doc, Self::similarity(query_embedding, &doc.embedding)))
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored
    }

    /// Number of documents in the index.
    pub fn document_count(&self) -> usize {
        self.documents.len()
    }
}

impl Default for CosineSimilarityRetriever {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // BM25 tests

    #[test]
    fn bm25_add_and_count() {
        let mut r = BM25Retriever::new();
        assert_eq!(r.document_count(), 0);
        r.add_document("d1", "hello world");
        r.add_document("d2", "foo bar baz");
        assert_eq!(r.document_count(), 2);
    }

    #[test]
    fn bm25_score_increases_with_term_freq() {
        let mut r = BM25Retriever::new();
        // doc_low has the query term once; doc_high has it three times
        r.add_document("doc_low", "rust is great");
        r.add_document("doc_high", "rust rust rust compiler");
        let score_low = r.score("rust", &r.documents[0].clone());
        let score_high = r.score("rust", &r.documents[1].clone());
        assert!(
            score_high > score_low,
            "expected score_high ({score_high}) > score_low ({score_low})"
        );
    }

    #[test]
    fn bm25_retrieve_top1_relevant() {
        let mut r = BM25Retriever::new();
        r.add_document("relevant", "the quick brown fox jumps over the lazy dog");
        r.add_document("irrelevant", "lorem ipsum dolor sit amet");
        let results = r.retrieve("fox jumps", 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.id, "relevant");
    }

    #[test]
    fn bm25_empty_corpus_retrieve() {
        let r = BM25Retriever::new();
        let results = r.retrieve("anything", 5);
        assert!(results.is_empty());
    }

    // Cosine similarity tests

    #[test]
    fn cosine_similarity_identical_vectors() {
        let v = vec![1.0_f32, 2.0, 3.0];
        let sim = CosineSimilarityRetriever::similarity(&v, &v);
        assert!(
            (sim - 1.0).abs() < 1e-6,
            "identical vectors should give 1.0, got {sim}"
        );
    }

    #[test]
    fn cosine_similarity_orthogonal_vectors() {
        let a = vec![1.0_f32, 0.0];
        let b = vec![0.0_f32, 1.0];
        let sim = CosineSimilarityRetriever::similarity(&a, &b);
        assert!(
            sim.abs() < 1e-6,
            "orthogonal vectors should give 0.0, got {sim}"
        );
    }

    #[test]
    fn cosine_retriever_add_and_count() {
        let mut r = CosineSimilarityRetriever::new();
        assert_eq!(r.document_count(), 0);
        r.add_document("v1", vec![1.0, 0.0]);
        r.add_document("v2", vec![0.0, 1.0]);
        assert_eq!(r.document_count(), 2);
    }

    #[test]
    fn cosine_retrieve_top1() {
        let mut r = CosineSimilarityRetriever::new();
        r.add_document("close", vec![1.0, 1.0]);
        r.add_document("far", vec![-1.0, -1.0]);
        let query = vec![1.0_f32, 1.0];
        let results = r.retrieve(&query, 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.id, "close");
    }
}
