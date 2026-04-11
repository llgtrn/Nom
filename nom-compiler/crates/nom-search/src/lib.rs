//! Hybrid search (BM25 + semantic) for .nomtu resolution.
//!
//! Provides BM25 keyword scoring against .nomtu entries and
//! Reciprocal Rank Fusion (RRF) for merging multiple ranked lists.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ── BM25 Index ───────────────────────────────────────────────────────

/// BM25 scoring for keyword search against .nomtu entries.
pub struct BM25Index {
    /// term -> document frequency
    df: HashMap<String, usize>,
    /// total documents
    doc_count: usize,
    /// average document length
    avg_dl: f64,
    /// document term frequencies: doc_id -> term -> count
    tf: HashMap<String, HashMap<String, usize>>,
    /// document lengths
    doc_len: HashMap<String, usize>,
}

/// A single BM25 search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BM25Result {
    pub doc_id: String,
    pub score: f64,
}

/// A hybrid search result from RRF fusion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridResult {
    pub doc_id: String,
    pub score: f64,
    pub sources: Vec<String>,
}

impl BM25Index {
    pub fn new() -> Self {
        Self {
            df: HashMap::new(),
            doc_count: 0,
            avg_dl: 0.0,
            tf: HashMap::new(),
            doc_len: HashMap::new(),
        }
    }

    /// Index a .nomtu entry. `doc_id` is typically "word::variant" or just "word".
    /// `text` should combine word, variant, describe, kind for full-text indexing.
    pub fn add_document(&mut self, doc_id: &str, text: &str) {
        let tokens = tokenize(text);
        let doc_len = tokens.len();

        // Update total doc length for average calculation
        let total_len: usize = self.doc_len.values().sum::<usize>() + doc_len;
        self.doc_count += 1;
        self.avg_dl = total_len as f64 / self.doc_count as f64;

        self.doc_len.insert(doc_id.to_string(), doc_len);

        // Count term frequencies for this document
        let mut term_freq: HashMap<String, usize> = HashMap::new();
        for token in &tokens {
            *term_freq.entry(token.clone()).or_default() += 1;
        }

        // Update document frequency (each term counted once per doc)
        for term in term_freq.keys() {
            *self.df.entry(term.clone()).or_default() += 1;
        }

        self.tf.insert(doc_id.to_string(), term_freq);
    }

    /// Search for a query string, return scored results.
    pub fn search(&self, query: &str, limit: usize) -> Vec<BM25Result> {
        let query_terms = tokenize(query);
        if query_terms.is_empty() {
            return Vec::new();
        }

        let mut scores: Vec<BM25Result> = self
            .tf
            .keys()
            .map(|doc_id| BM25Result {
                score: self.bm25_score(&query_terms, doc_id),
                doc_id: doc_id.clone(),
            })
            .filter(|r| r.score > 0.0)
            .collect();

        scores.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(limit);
        scores
    }

    fn bm25_score(&self, query_terms: &[String], doc_id: &str) -> f64 {
        let k1 = 1.2;
        let b = 0.75;
        let dl = *self.doc_len.get(doc_id).unwrap_or(&1) as f64;

        let mut score = 0.0;
        for term in query_terms {
            let df = *self.df.get(term).unwrap_or(&0) as f64;
            let tf = self
                .tf
                .get(doc_id)
                .and_then(|m| m.get(term))
                .map(|&c| c as f64)
                .unwrap_or(0.0);

            if df == 0.0 || tf == 0.0 {
                continue;
            }

            let idf = ((self.doc_count as f64 - df + 0.5) / (df + 0.5) + 1.0).ln();
            let tf_norm = (tf * (k1 + 1.0)) / (tf + k1 * (1.0 - b + b * dl / self.avg_dl));
            score += idf * tf_norm;
        }
        score
    }
}

impl Default for BM25Index {
    fn default() -> Self {
        Self::new()
    }
}

// ── Reciprocal Rank Fusion ───────────────────────────────────────────

/// Reciprocal Rank Fusion -- merge multiple ranked lists.
/// Same algorithm used by Elasticsearch, Pinecone, GitNexus.
///
/// `k` is the RRF constant (default: 60.0).
pub fn reciprocal_rank_fusion(
    ranked_lists: &[Vec<(String, f64)>],
    k: f64,
    limit: usize,
) -> Vec<HybridResult> {
    let mut scores: HashMap<String, (f64, Vec<String>)> = HashMap::new();

    for (list_idx, list) in ranked_lists.iter().enumerate() {
        let source = format!("source-{list_idx}");
        for (rank, (doc_id, _score)) in list.iter().enumerate() {
            let rrf_score = 1.0 / (k + rank as f64 + 1.0);
            let entry = scores
                .entry(doc_id.clone())
                .or_insert_with(|| (0.0, Vec::new()));
            entry.0 += rrf_score;
            if !entry.1.contains(&source) {
                entry.1.push(source.clone());
            }
        }
    }

    let mut results: Vec<HybridResult> = scores
        .into_iter()
        .map(|(doc_id, (score, sources))| HybridResult {
            doc_id,
            score,
            sources,
        })
        .collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit);
    results
}

// ── Tokenization ─────────────────────────────────────────────────────

/// Tokenize text for BM25 indexing.
fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|s| s.to_lowercase())
        .filter(|s| s.len() > 1 && !is_stopword(s))
        .collect()
}

fn is_stopword(word: &str) -> bool {
    matches!(
        word,
        "the" | "is" | "at" | "in" | "of" | "on" | "to" | "for" | "and" | "or" | "an" | "a"
    )
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bm25_search_ranks_relevant_documents() {
        let mut index = BM25Index::new();
        index.add_document("hash::sha256", "hash sha256 cryptographic hashing function security");
        index.add_document("hash::md5", "hash md5 legacy hashing function deprecated");
        index.add_document("sort::quicksort", "sort quicksort sorting algorithm performance");
        index.add_document("auth::jwt", "auth jwt token authentication security bearer");

        let results = index.search("hashing function", 10);
        assert!(!results.is_empty(), "expected results for 'hashing function'");
        // hash entries should rank highest
        assert!(
            results[0].doc_id.starts_with("hash"),
            "expected hash entry first, got {}",
            results[0].doc_id
        );
    }

    #[test]
    fn bm25_empty_query_returns_nothing() {
        let mut index = BM25Index::new();
        index.add_document("foo", "some function");
        let results = index.search("", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn rrf_merges_ranked_lists() {
        let list_a = vec![
            ("doc1".to_string(), 10.0),
            ("doc2".to_string(), 8.0),
            ("doc3".to_string(), 5.0),
        ];
        let list_b = vec![
            ("doc2".to_string(), 9.0),
            ("doc1".to_string(), 7.0),
            ("doc4".to_string(), 3.0),
        ];

        let results = reciprocal_rank_fusion(&[list_a, list_b], 60.0, 10);
        assert!(!results.is_empty());

        // doc1 and doc2 appear in both lists, should rank highest
        let top_ids: Vec<&str> = results.iter().take(2).map(|r| r.doc_id.as_str()).collect();
        assert!(top_ids.contains(&"doc1") || top_ids.contains(&"doc2"));

        // doc2 should have 2 sources
        let doc2 = results.iter().find(|r| r.doc_id == "doc2").unwrap();
        assert_eq!(doc2.sources.len(), 2, "doc2 should appear in both lists");
    }

    #[test]
    fn tokenize_splits_and_lowercases() {
        let tokens = tokenize("Hello_World foo-bar BAZ");
        assert!(tokens.contains(&"hello_world".to_string()));
        assert!(tokens.contains(&"foo".to_string()));
        assert!(tokens.contains(&"bar".to_string()));
        assert!(tokens.contains(&"baz".to_string()));
    }

    #[test]
    fn stopwords_are_filtered() {
        let tokens = tokenize("the quick brown fox is a test");
        assert!(!tokens.contains(&"the".to_string()));
        assert!(!tokens.contains(&"is".to_string()));
        assert!(!tokens.contains(&"a".to_string()));
        assert!(tokens.contains(&"quick".to_string()));
    }
}
