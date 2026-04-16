use std::collections::BTreeSet;
use wasm_bindgen::prelude::*;
use serde::Serialize;

/// Stopwords stripped before tokenization. Exact copy of nom-grammar's
/// FUZZY_STOPWORDS constant — closed list, never reordered, so the same
/// input always produces the same token set across environments.
const FUZZY_STOPWORDS: &[&str] = &[
    "a", "the", "of", "to", "and", "or", "with", "for", "in", "on", "as", "an", "is", "into",
    "from", "by", "that", "this", "its", "at", "be", "are", "it", "one", "two", "each", "every",
    "any", "all", "no", "not", "then", "than", "only", "also", "same",
];

/// Tokenize a free-form intent string into a normalized set of domain
/// words for Jaccard-similarity comparison. Lowercase, alphabetic-only,
/// length >= 3, not in FUZZY_STOPWORDS. Deterministic — the same input
/// always produces the same set in the same order.
fn fuzzy_tokens(intent: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut cur = String::new();
    let lower = intent.to_lowercase();
    for ch in lower.chars().chain(std::iter::once(' ')) {
        if ch.is_ascii_alphabetic() {
            cur.push(ch);
        } else {
            if cur.len() >= 3 && !FUZZY_STOPWORDS.contains(&cur.as_str()) {
                out.insert(std::mem::take(&mut cur));
            } else {
                cur.clear();
            }
        }
    }
    out
}

/// Jaccard similarity between two token sets — |a ∩ b| / |a ∪ b|.
/// Returns 0.0 when either set is empty (avoids div-by-zero).
fn jaccard(a: &BTreeSet<String>, b: &BTreeSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count();
    let union_count = a.union(b).count();
    if union_count == 0 {
        return 0.0;
    }
    inter as f64 / union_count as f64
}

#[derive(Serialize)]
struct MatchResult {
    pattern_id: String,
    intent: String,
    score: f64,
}

// In-memory pattern catalog — loaded once from Tauri backend JSON.
// thread_local! + RefCell is safe for WASM which is single-threaded by
// specification.
thread_local! {
    static PATTERNS: std::cell::RefCell<Option<Vec<(String, String, BTreeSet<String>)>>> =
        std::cell::RefCell::new(None);
}

/// Load the pattern catalog from a JSON string produced by the Tauri backend.
/// Expected format: [{"pattern_id": "...", "intent": "..."}, ...]
/// Returns true on success, false on parse error.
#[wasm_bindgen]
pub fn load_patterns(json: &str) -> bool {
    let parsed: Result<Vec<serde_json::Value>, _> = serde_json::from_str(json);
    match parsed {
        Ok(items) => {
            let mut catalog = Vec::with_capacity(items.len());
            for item in items {
                let pid = item
                    .get("pattern_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let intent = item
                    .get("intent")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let tokens = fuzzy_tokens(&intent);
                catalog.push((pid, intent, tokens));
            }
            PATTERNS.with(|p| *p.borrow_mut() = Some(catalog));
            true
        }
        Err(_) => false,
    }
}

/// Match free-form input against the loaded pattern catalog using Jaccard
/// similarity. Returns a JSON array of {pattern_id, intent, score} objects
/// sorted by score descending, capped at `limit` results, filtered to
/// scores >= threshold.
#[wasm_bindgen]
pub fn match_input(input: &str, threshold: f64, limit: usize) -> JsValue {
    let q = fuzzy_tokens(input);
    if q.is_empty() {
        return serde_wasm_bindgen::to_value(&Vec::<MatchResult>::new())
            .unwrap_or(JsValue::NULL);
    }

    PATTERNS.with(|p| {
        let borrow = p.borrow();
        let Some(catalog) = borrow.as_ref() else {
            return serde_wasm_bindgen::to_value(&Vec::<MatchResult>::new())
                .unwrap_or(JsValue::NULL);
        };

        let mut scored: Vec<MatchResult> = catalog
            .iter()
            .filter_map(|(pid, intent, tokens)| {
                let score = jaccard(&q, tokens);
                if score >= threshold {
                    Some(MatchResult {
                        pattern_id: pid.clone(),
                        intent: intent.clone(),
                        score,
                    })
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(limit);

        serde_wasm_bindgen::to_value(&scored).unwrap_or(JsValue::NULL)
    })
}

/// Tokenize input and return the resulting token array as a JsValue.
/// Useful for debugging tokenization in the browser console.
#[wasm_bindgen]
pub fn tokenize(input: &str) -> JsValue {
    let tokens: Vec<String> = fuzzy_tokens(input).into_iter().collect();
    serde_wasm_bindgen::to_value(&tokens).unwrap_or(JsValue::NULL)
}

/// Return the number of patterns currently loaded in the catalog.
#[wasm_bindgen]
pub fn pattern_count() -> usize {
    PATTERNS.with(|p| p.borrow().as_ref().map(|c| c.len()).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_tokens_strips_stopwords_and_normalizes() {
        let toks = fuzzy_tokens("The Quick brown-fox JUMPS over the lazy dog");
        assert!(toks.contains("quick"));
        assert!(toks.contains("brown"));
        assert!(toks.contains("fox"));
        assert!(toks.contains("jumps"));
        assert!(toks.contains("over"));
        assert!(toks.contains("lazy"));
        assert!(toks.contains("dog"));
        assert!(!toks.contains("the"));
    }

    #[test]
    fn jaccard_identical_sets_return_one() {
        let a = fuzzy_tokens("cache pure function results");
        let b = fuzzy_tokens("cache pure function results");
        assert!((jaccard(&a, &b) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn jaccard_empty_sets_return_zero() {
        let empty: BTreeSet<String> = BTreeSet::new();
        let a = fuzzy_tokens("cache pure function results");
        assert_eq!(jaccard(&empty, &a), 0.0);
        assert_eq!(jaccard(&a, &empty), 0.0);
    }

    #[test]
    fn fuzzy_tokens_all_stopwords_returns_empty() {
        let toks = fuzzy_tokens("the of a to and or");
        assert!(toks.is_empty());
    }
}
