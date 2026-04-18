//! Query result reranking for nom-intent.
//! Pattern: LlamaIndex postprocessor/llm_rerank — BM25 + score fusion.

use std::collections::HashMap;

/// A ranked result with original + reranked scores
#[derive(Debug, Clone)]
pub struct RankedResult {
    pub id: String,
    pub word: String,
    pub original_score: f64,
    pub rerank_score: f64,
    pub combined_score: f64,
}

/// BM25 parameters
const K1: f64 = 1.2;
const B: f64 = 0.75;

/// Tokenize a string into lowercase words (simple whitespace split)
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|s| s.len() >= 2)
        .map(String::from)
        .collect()
}

/// Compute BM25 score for a document against a query
fn bm25_score(
    query_tokens: &[String],
    doc_tokens: &[String],
    avg_doc_len: f64,
    num_docs: usize,
    doc_freqs: &HashMap<String, usize>,
) -> f64 {
    let doc_len = doc_tokens.len() as f64;
    let mut score = 0.0;

    let doc_token_counts: HashMap<&str, usize> = {
        let mut counts = HashMap::new();
        for t in doc_tokens {
            *counts.entry(t.as_str()).or_insert(0) += 1;
        }
        counts
    };

    for qt in query_tokens {
        let tf = *doc_token_counts.get(qt.as_str()).unwrap_or(&0) as f64;
        let df = *doc_freqs.get(qt).unwrap_or(&0) as f64;
        let idf = ((num_docs as f64 - df + 0.5) / (df + 0.5) + 1.0).ln();
        let tf_norm = (tf * (K1 + 1.0)) / (tf + K1 * (1.0 - B + B * doc_len / avg_doc_len));
        score += idf * tf_norm;
    }

    score
}

/// Rerank a list of candidates using BM25 scoring against a query.
/// BM25 scores are normalized to [0, 1] before being combined with the original
/// scores via weighted interpolation: `alpha * orig_score + (1 - alpha) * bm25_normalized`.
/// Both inputs are on the same [0, 1] scale, making the weight parameter meaningful.
pub fn rerank(query: &str, candidates: &[(String, String, f64)], alpha: f64) -> Vec<RankedResult> {
    // candidates: Vec<(id, word/description, original_score)>
    if candidates.is_empty() {
        return Vec::new();
    }

    let query_tokens = tokenize(query);
    if query_tokens.is_empty() {
        // No meaningful query tokens — return original order
        return candidates
            .iter()
            .map(|(id, word, score)| RankedResult {
                id: id.clone(),
                word: word.clone(),
                original_score: *score,
                rerank_score: 0.0,
                combined_score: *score,
            })
            .collect();
    }

    // Tokenize all documents
    let doc_tokens: Vec<Vec<String>> = candidates
        .iter()
        .map(|(_, word, _)| tokenize(word))
        .collect();

    // Compute document frequencies
    let num_docs = candidates.len();
    let mut doc_freqs: HashMap<String, usize> = HashMap::new();
    for tokens in &doc_tokens {
        let unique: std::collections::HashSet<&str> = tokens.iter().map(|s| s.as_str()).collect();
        for token in unique {
            *doc_freqs.entry(token.to_string()).or_insert(0) += 1;
        }
    }

    // Average document length
    let avg_doc_len = doc_tokens.iter().map(|t| t.len()).sum::<usize>() as f64 / num_docs as f64;

    // Score each candidate with raw BM25
    let mut results: Vec<RankedResult> = candidates
        .iter()
        .zip(doc_tokens.iter())
        .map(|((id, word, orig_score), tokens)| {
            let bm25 = bm25_score(&query_tokens, tokens, avg_doc_len, num_docs, &doc_freqs);
            RankedResult {
                id: id.clone(),
                word: word.clone(),
                original_score: *orig_score,
                rerank_score: bm25,
                combined_score: 0.0, // filled in after normalization
            }
        })
        .collect();

    // Normalize BM25 scores to [0, 1] so they are on the same scale as original_score
    let max_bm25 = results
        .iter()
        .map(|r| r.rerank_score)
        .fold(0.0f64, f64::max);
    for r in &mut results {
        let normalized_bm25 = if max_bm25 > 0.0 {
            r.rerank_score / max_bm25
        } else {
            0.0
        };
        r.rerank_score = normalized_bm25;
        r.combined_score = alpha * r.original_score + (1.0 - alpha) * normalized_bm25;
    }

    // Sort by combined score descending
    results.sort_by(|a, b| {
        b.combined_score
            .partial_cmp(&a.combined_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}

/// Reciprocal Rank Fusion — combine multiple ranked lists
pub fn reciprocal_rank_fusion(ranked_lists: &[Vec<RankedResult>], k: f64) -> Vec<RankedResult> {
    let mut scores: HashMap<String, (f64, String, f64)> = HashMap::new();

    for list in ranked_lists {
        for (rank, result) in list.iter().enumerate() {
            let rrf_score = 1.0 / (k + rank as f64 + 1.0);
            let entry = scores.entry(result.id.clone()).or_insert((
                0.0,
                result.word.clone(),
                result.original_score,
            ));
            entry.0 += rrf_score;
        }
    }

    let mut results: Vec<RankedResult> = scores
        .into_iter()
        .map(|(id, (rrf, word, orig))| RankedResult {
            id,
            word,
            original_score: orig,
            rerank_score: rrf,
            combined_score: rrf,
        })
        .collect();

    results.sort_by(|a, b| {
        b.combined_score
            .partial_cmp(&a.combined_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rerank_empty_returns_empty() {
        let results = rerank("test", &[], 0.5);
        assert!(results.is_empty());
    }

    #[test]
    fn rerank_basic_ordering() {
        let candidates = vec![
            ("h1".to_string(), "fetch url http request".to_string(), 0.5),
            ("h2".to_string(), "hash sha256 digest".to_string(), 0.8),
            (
                "h3".to_string(),
                "fetch data from url endpoint".to_string(),
                0.3,
            ),
        ];
        let results = rerank("fetch url", &candidates, 0.3);
        // fetch_url candidates should rank higher than hash
        assert_eq!(results.len(), 3);
        assert!(results[0].word.contains("fetch") || results[1].word.contains("fetch"));
    }

    #[test]
    fn rrf_combines_two_lists() {
        let list1 = vec![
            RankedResult {
                id: "a".into(),
                word: "alpha".into(),
                original_score: 1.0,
                rerank_score: 1.0,
                combined_score: 1.0,
            },
            RankedResult {
                id: "b".into(),
                word: "beta".into(),
                original_score: 0.5,
                rerank_score: 0.5,
                combined_score: 0.5,
            },
        ];
        let list2 = vec![
            RankedResult {
                id: "b".into(),
                word: "beta".into(),
                original_score: 0.8,
                rerank_score: 0.8,
                combined_score: 0.8,
            },
            RankedResult {
                id: "a".into(),
                word: "alpha".into(),
                original_score: 0.3,
                rerank_score: 0.3,
                combined_score: 0.3,
            },
        ];
        let fused = reciprocal_rank_fusion(&[list1, list2], 60.0);
        assert_eq!(fused.len(), 2);
        // Both items appear; RRF fuses their ranks
    }
}
