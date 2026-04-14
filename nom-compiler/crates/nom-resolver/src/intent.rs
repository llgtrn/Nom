//! T3.1 — `IntentResolver` trait + two impls.
//!
//! Per the approved plan T3.1 (mighty-jumping-snowglobe.md):
//!
//!   pub trait IntentResolver {
//!       fn rank(&self, query: &str, kind: Kind, limit: usize)
//!         -> Vec<(f64, EntityHash)>;
//!   }
//!
//! Two implementations:
//!
//!   1. [`JaccardOverIntents`] — deterministic Jaccard token-overlap
//!      against the dict's word + intent text fields. Ships now and
//!      backs the resolver's "semantic re-rank" stage with no external
//!      models, no embeddings, no nondeterminism. Same backend as the
//!      `nom-grammar::search_patterns` helper used by CLI + LSP + CI
//!      tests, so retrieval ranking stays consistent across surfaces.
//!   2. [`SemanticEmbedding`] — stub that always returns
//!      `Err(ResolverErr::Unavailable)`. Kept as a placeholder so the
//!      production wiring for "the real embedding-driven backend lands
//!      one cycle from now" already has a typed slot to plug into.
//!
//! Discipline: the trait operates over abstract `EntityHash`es so it
//! does NOT depend on the legacy schema OR `NomDict` directly. The
//! `JaccardOverIntents` adapter takes a borrowed slice of (hash, text)
//! pairs — callers pre-fetch from whichever DB tier is cheapest.

use std::collections::HashSet;

/// 64-char hex SHA-256 of the entry body. Owned so impls can return
/// detached results.
pub type EntityHash = String;

/// Rank result: (similarity_score in [0.0, 1.0], hash).
pub type RankedHit = (f64, EntityHash);

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ResolverErr {
    /// Backend is structurally unavailable (e.g. embedding model not
    /// loaded, network gated). Callers should fall back to a sibling
    /// impl, NOT bubble the error to the user.
    #[error("intent-resolver backend unavailable: {0}")]
    Unavailable(String),
}

/// The single-method ranking trait. Impls fan out into different
/// retrieval strategies; the resolver picks one (or chains them) at
/// runtime.
pub trait IntentResolver {
    fn rank(
        &self,
        query: &str,
        kind: Option<&str>,
        limit: usize,
    ) -> Result<Vec<RankedHit>, ResolverErr>;
}

// ── Jaccard impl ───────────────────────────────────────────────────────

/// (hash, kind, intent_text) tuple — what `JaccardOverIntents` consumes
/// per entry. `kind` is `None` for cross-kind candidates.
pub struct IntentRow<'a> {
    pub hash: &'a str,
    pub kind: Option<&'a str>,
    pub intent: &'a str,
}

/// Deterministic Jaccard re-rank over a borrowed slice of intent rows.
/// No hidden state; same input → same output. Backs the production
/// resolver's first-pass re-rank.
pub struct JaccardOverIntents<'a> {
    pub rows: &'a [IntentRow<'a>],
}

impl<'a> IntentResolver for JaccardOverIntents<'a> {
    fn rank(
        &self,
        query: &str,
        kind: Option<&str>,
        limit: usize,
    ) -> Result<Vec<RankedHit>, ResolverErr> {
        let q_tokens = tokens(query);
        let mut scored: Vec<RankedHit> = self
            .rows
            .iter()
            .filter(|r| match (kind, r.kind) {
                (None, _) => true,
                (Some(want), Some(got)) => want == got,
                (Some(_), None) => false,
            })
            .map(|r| (jaccard(&q_tokens, &tokens(r.intent)), r.hash.to_string()))
            .filter(|(score, _)| *score > 0.0)
            .collect();
        // Highest score first; ties broken by hash for determinism.
        scored.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.cmp(&b.1))
        });
        scored.truncate(limit);
        Ok(scored)
    }
}

// ── Semantic-embedding stub ────────────────────────────────────────────

/// Placeholder for the future embedding backend. Always errors
/// `Unavailable` so callers fall back to `JaccardOverIntents`. Replaced
/// in T4.2 (gated on the deterministic-build / pinned-model decision).
pub struct SemanticEmbedding;

impl IntentResolver for SemanticEmbedding {
    fn rank(
        &self,
        _query: &str,
        _kind: Option<&str>,
        _limit: usize,
    ) -> Result<Vec<RankedHit>, ResolverErr> {
        Err(ResolverErr::Unavailable(
            "semantic embedding backend not wired (T4.2 gate)".into(),
        ))
    }
}

// ── Tokenization helpers (private, mirrors nom-grammar::fuzzy_tokens) ──
//
// Kept private and small. Once nom-resolver's deps include nom-grammar,
// these collapse to direct calls into the canonical helpers. For now
// nom-resolver doesn't depend on nom-grammar — adding the dep would
// pull SQLite-init code into a crate that doesn't need it. The token
// set + Jaccard formula are deliberately the same shape as the
// canonical implementation so results stay aligned.

fn tokens(text: &str) -> HashSet<String> {
    text.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty() && t.len() > 1)
        .map(|t| t.to_string())
        .collect()
}

fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count() as f64;
    let union = a.union(b).count() as f64;
    if union == 0.0 { 0.0 } else { inter / union }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<IntentRow<'static>> {
        vec![
            IntentRow {
                hash: "h_add",
                kind: Some("function"),
                intent: "add two numbers and return the sum",
            },
            IntentRow {
                hash: "h_mul",
                kind: Some("function"),
                intent: "multiply two numbers and return the product",
            },
            IntentRow {
                hash: "h_logger",
                kind: Some("module"),
                intent: "structured logging facade with severity levels",
            },
            IntentRow {
                hash: "h_unrelated",
                kind: Some("function"),
                intent: "render a Bezier curve to a pixel buffer",
            },
        ]
    }

    #[test]
    fn jaccard_ranks_close_match_first() {
        let rs = rows();
        let r = JaccardOverIntents { rows: &rs };
        let hits = r.rank("add numbers", Some("function"), 3).unwrap();
        assert!(!hits.is_empty(), "expected at least one match");
        assert_eq!(hits[0].1, "h_add", "best match should be add: got {hits:?}");
        assert!(hits[0].0 > hits.last().unwrap().0);
    }

    #[test]
    fn kind_filter_is_strict() {
        let rs = rows();
        let r = JaccardOverIntents { rows: &rs };
        let hits = r.rank("logging facade", Some("function"), 5).unwrap();
        // h_logger is module; kind=function filter removes it entirely.
        assert!(hits.iter().all(|(_, h)| h != "h_logger"));
    }

    #[test]
    fn no_token_overlap_yields_empty_result() {
        let rs = rows();
        let r = JaccardOverIntents { rows: &rs };
        // "xyzqq" doesn't intersect any intent text.
        let hits = r.rank("xyzqq", Some("function"), 5).unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn rank_is_deterministic_across_calls() {
        let rs = rows();
        let r = JaccardOverIntents { rows: &rs };
        let a = r.rank("multiply numbers", Some("function"), 2).unwrap();
        let b = r.rank("multiply numbers", Some("function"), 2).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn semantic_embedding_stub_always_errors_unavailable() {
        let s = SemanticEmbedding;
        let result = s.rank("anything", None, 1);
        assert!(matches!(result, Err(ResolverErr::Unavailable(_))));
    }

    #[test]
    fn limit_caps_result_size() {
        let rs = rows();
        let r = JaccardOverIntents { rows: &rs };
        // Query that hits multiple intents.
        let hits = r
            .rank("two numbers and return", Some("function"), 1)
            .unwrap();
        assert!(hits.len() <= 1, "limit must cap result vec length");
    }
}
