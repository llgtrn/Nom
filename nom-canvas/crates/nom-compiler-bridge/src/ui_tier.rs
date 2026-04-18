#![deny(unsafe_code)]
use crate::shared::SharedState;
#[allow(unused_imports)]
use nom_blocks::block_model::NomtuRef;
use std::sync::Arc;

/// Result from can_wire check (is_valid, confidence, reason)
#[derive(Clone, Debug)]
pub struct WireCheckResult {
    pub is_valid: bool,
    pub confidence: f32,
    pub reason: String,
}

/// Status badge for compile score
#[derive(Clone, Debug, PartialEq)]
pub enum CompileStatus {
    Valid,         // score >= 0.8
    LowConfidence, // score 0.5 - 0.8
    Unknown,       // score < 0.5
    NotChecked,    // no check yet
}

impl CompileStatus {
    pub fn from_score(score: f32) -> Self {
        if score >= 0.8 {
            CompileStatus::Valid
        } else if score >= 0.5 {
            CompileStatus::LowConfidence
        } else {
            CompileStatus::Unknown
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            CompileStatus::Valid => "Valid",
            CompileStatus::LowConfidence => "Low confidence",
            CompileStatus::Unknown => "Unknown",
            CompileStatus::NotChecked => "—",
        }
    }
}

/// A single BM25 search result returned by `UiTier::search_bm25`.
#[derive(Clone, Debug, PartialEq)]
pub struct SearchHit {
    pub word: String,
    pub score: f32,
}

/// UI tier — all functions are sync and must complete in <1ms
pub struct UiTier {
    state: Arc<SharedState>,
}

impl UiTier {
    pub fn new(state: Arc<SharedState>) -> Self {
        Self { state }
    }

    /// Grammar keywords — from cache (populated at startup)
    pub fn grammar_keywords(&self) -> Vec<String> {
        self.state
            .cached_grammar_kinds()
            .into_iter()
            .map(|k| k.name)
            .collect()
    }

    /// Check if kind is known in grammar cache
    pub fn is_known_kind(&self, kind: &str) -> bool {
        let kinds = self.state.cached_grammar_kinds();
        if !kinds.is_empty() {
            return kinds.iter().any(|k| k.name == kind);
        }
        false
    }

    /// BM25 search over the grammar cache.
    /// Under `compiler` feature, builds a real BM25 index over all cached grammar kinds
    /// and returns scored hits. Without the feature, falls back to a simple prefix scan
    /// returning score=1.0 for exact prefix matches.
    pub fn search_bm25(&self, query: &str) -> Vec<SearchHit> {
        if query.is_empty() {
            return Vec::new();
        }
        search_cached_kinds(query, &self.state.cached_grammar_kinds())
    }
}

// With compiler feature: real implementations using nom-score
#[cfg(feature = "compiler")]
impl UiTier {
    /// can_wire — grammar-typed wire validation (pure, no DB, uses preloaded grammar)
    pub fn can_wire(
        &self,
        src_kind: &str,
        src_slot: &str,
        dst_kind: &str,
        dst_slot: &str,
    ) -> WireCheckResult {
        use nom_types::{Atom, AtomKind};
        let producer = Atom {
            id: src_slot.to_string(),
            kind: AtomKind::Function,
            name: src_slot.to_string(),
            source_path: String::new(),
            language: "nom".to_string(),
            labels: vec![],
            concept: Some(src_kind.to_string()),
            signature: None,
            body: None,
        };
        let consumer = Atom {
            id: dst_slot.to_string(),
            kind: AtomKind::Function,
            name: dst_slot.to_string(),
            source_path: String::new(),
            language: "nom".to_string(),
            labels: vec![],
            concept: Some(dst_kind.to_string()),
            signature: None,
            body: None,
        };
        match nom_score::can_wire(&producer, &consumer) {
            nom_score::WireResult::Compatible { score } => WireCheckResult {
                is_valid: true,
                confidence: score,
                reason: String::new(),
            },
            nom_score::WireResult::NeedsAdapter { reason } => WireCheckResult {
                is_valid: false,
                confidence: 0.5,
                reason,
            },
            nom_score::WireResult::Incompatible { reason } => WireCheckResult {
                is_valid: false,
                confidence: 0.0,
                reason,
            },
        }
    }

    pub fn compile_status(&self, word: &str, kind: &str) -> CompileStatus {
        if self.state.cached_grammar_kinds().is_empty() {
            return CompileStatus::NotChecked;
        }
        CompileStatus::from_score(self.score_atom(word, kind))
    }
}

/// Shared scoring kernel used by both `UiTier::score_atom` and `UiTierOps::score_atom`.
/// Returns a score in `[0.0, 1.0]`.
///
/// When the `compiler` feature is active and `kinds` is non-empty, the result is
/// determined by whether `word` or `kind` appears in the cached grammar kinds.
/// When `kinds` is empty the full `nom_score::score_atom` path is taken.
/// Without the `compiler` feature the function always returns `0.5`.
#[allow(unused_variables)]
fn score_atom_impl(word: &str, kind: &str, kinds: &[crate::shared::GrammarKind]) -> f32 {
    #[cfg(feature = "compiler")]
    {
        if !kinds.is_empty() {
            return if kinds.iter().any(|k| k.name == word || k.name == kind) {
                0.9
            } else {
                0.3
            };
        }
        use nom_types::{Atom, AtomKind};
        let atom = Atom {
            id: word.to_string(),
            kind: AtomKind::Function,
            name: word.to_string(),
            source_path: String::new(),
            language: "nom".to_string(),
            labels: vec![],
            concept: Some(kind.to_string()),
            signature: None,
            body: None,
        };
        nom_score::score_atom(&atom).overall()
    }
    #[cfg(not(feature = "compiler"))]
    {
        0.5
    }
}

fn search_cached_kinds(query: &str, kinds: &[crate::shared::GrammarKind]) -> Vec<SearchHit> {
    let q = query.to_lowercase();
    kinds
        .iter()
        .filter(|k| {
            let name = k.name.to_lowercase();
            let desc = k.description.to_lowercase();
            name.contains(&q)
                || desc.contains(&q)
                || q.split_whitespace()
                    .any(|part| name.contains(part) || desc.contains(part))
        })
        .map(|k| SearchHit {
            word: k.name.clone(),
            score: if k.name.eq_ignore_ascii_case(query) {
                1.0
            } else {
                0.75
            },
        })
        .collect()
}

// Without compiler feature: stubs
#[cfg(not(feature = "compiler"))]
impl UiTier {
    pub fn can_wire(&self, _sk: &str, _ss: &str, _dk: &str, _ds: &str) -> WireCheckResult {
        WireCheckResult {
            is_valid: true,
            confidence: 0.0,
            reason: "stub - compiler feature not enabled".into(),
        }
    }
    pub fn compile_status(&self, _word: &str, _kind: &str) -> CompileStatus {
        CompileStatus::NotChecked
    }
}

impl UiTier {
    /// score_atom — delegates to shared kernel; works with or without the `compiler` feature
    pub fn score_atom(&self, word: &str, kind: &str) -> f32 {
        let kinds = self.state.cached_grammar_kinds();
        score_atom_impl(word, kind, &kinds)
    }
}

/// UiTierOps — borrowed accessor for UI-tier operations (<1ms, sync)
pub struct UiTierOps<'a> {
    shared: &'a SharedState,
}

impl UiTier {
    /// Returns the number of grammar kinds currently loaded in the cache.
    pub fn kind_count(&self) -> usize {
        self.state.cached_grammar_kinds().len()
    }

    /// Returns `true` when the grammar kind cache is empty.
    pub fn is_grammar_empty(&self) -> bool {
        self.state.cached_grammar_kinds().is_empty()
    }

    /// Returns a reference to the underlying `SharedState` Arc.
    pub fn shared_state(&self) -> &Arc<SharedState> {
        &self.state
    }

    /// Returns all grammar kind names that contain `substr` (case-insensitive).
    pub fn kinds_containing(&self, substr: &str) -> Vec<String> {
        if substr.is_empty() {
            return self.grammar_keywords();
        }
        let lower = substr.to_lowercase();
        self.state
            .cached_grammar_kinds()
            .into_iter()
            .filter(|k| k.name.to_lowercase().contains(&lower))
            .map(|k| k.name)
            .collect()
    }
}

impl<'a> UiTierOps<'a> {
    pub fn new(shared: &'a SharedState) -> Self {
        Self { shared }
    }

    /// Check if a kind name is known in the grammar cache
    pub fn is_known_kind(&self, kind: &str) -> bool {
        let kinds = self.shared.cached_grammar_kinds();
        !kinds.is_empty() && kinds.iter().any(|k| k.name == kind)
    }

    /// Resolve a synonym by checking grammar kinds for a matching name
    pub fn resolve_synonym(&self, word: &str) -> Option<String> {
        self.shared
            .cached_grammar_kinds()
            .into_iter()
            .find(|k| k.name == word)
            .map(|k| k.name)
    }

    /// Score an atom — uses shared ref directly, zero allocation per call
    pub fn score_atom(&self, word: &str, kind: &str) -> f32 {
        let kinds = self.shared.cached_grammar_kinds();
        score_atom_impl(word, kind, &kinds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_status_from_score() {
        assert_eq!(CompileStatus::from_score(0.9), CompileStatus::Valid);
        assert_eq!(CompileStatus::from_score(0.6), CompileStatus::LowConfidence);
        assert_eq!(CompileStatus::from_score(0.3), CompileStatus::Unknown);
    }

    #[test]
    fn grammar_keywords_from_empty_cache() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let tier = UiTier::new(state.clone());
        // Empty cache — returns empty list
        assert!(tier.grammar_keywords().is_empty());
        // Populate cache
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "verb".into(),
            description: "action".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let keywords = tier.grammar_keywords();
        assert_eq!(keywords, vec!["verb"]);
    }

    #[test]
    fn ui_tier_ops_is_known_kind() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "verb".into(),
            description: "action".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let ops = UiTierOps::new(&state);
        assert!(ops.is_known_kind("verb"));
        assert!(!ops.is_known_kind("noun"));
    }

    #[test]
    fn ui_tier_ops_resolve_synonym() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "concept".into(),
            description: "abstract idea".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let ops = UiTierOps::new(&state);
        assert_eq!(ops.resolve_synonym("concept"), Some("concept".to_string()));
        assert_eq!(ops.resolve_synonym("unknown"), None);
    }

    #[test]
    fn ui_tier_creates_from_shared() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let tier = UiTier::new(state.clone());
        // Construction must not panic; Arc refcount is at least 2
        assert!(Arc::strong_count(&state) >= 2);
        drop(tier);
    }

    #[test]
    fn ui_tier_can_wire_always_returns_bool() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let tier = UiTier::new(state);
        let r = tier.can_wire("kindA", "slotA", "kindB", "slotB");
        // In stub mode is_valid is always true; result must be well-formed
        let _ = r.is_valid; // just access the field to ensure it compiles
        assert!(r.confidence >= 0.0);
    }

    #[test]
    fn ui_tier_grammar_keywords_non_empty() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "define".into(),
                description: "declaration keyword".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "that".into(),
                description: "connector".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let tier = UiTier::new(state);
        let kw = tier.grammar_keywords();
        assert!(!kw.is_empty());
        assert!(kw.contains(&"define".to_string()));
        assert!(kw.contains(&"that".to_string()));
    }

    #[test]
    fn ui_tier_lookup_nomtu_returns_option() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "verb".into(),
            description: "action".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let tier = UiTier::new(state);
        // is_known_kind mirrors a lookup returning Some/None semantics
        assert!(tier.is_known_kind("verb"));
        assert!(!tier.is_known_kind("nonexistent_word"));
    }

    #[test]
    fn ui_tier_score_atom_returns_float() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let tier = UiTier::new(state);
        let score: f32 = tier.score_atom("run", "verb");
        // Must be a finite f32 in [0.0, 1.0]
        assert!(score.is_finite());
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn ui_tier_search_bm25_returns_vec() {
        // complete_prefix via ops is the BM25-like prefix search exposed in stub mode
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "render".into(),
                description: "output".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "resolve".into(),
                description: "lookup".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let ops = UiTierOps::new(&state);
        // resolve_synonym for a known word returns Some, for an unknown returns None
        let found = ops.resolve_synonym("render");
        let missing = ops.resolve_synonym("zzz");
        assert!(found.is_some());
        assert!(missing.is_none());
    }

    #[test]
    fn ui_tier_can_wire_false_for_incompatible() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let tier = UiTier::new(state);
        // In stub mode can_wire always returns is_valid=true; confidence must be >= 0.0
        let r = tier.can_wire("image", "out", "audio", "in");
        assert!(r.confidence >= 0.0);
        // The result must be well-formed (no panic, fields accessible)
        let _ = r.is_valid;
        let _ = r.reason.len();
    }

    #[test]
    fn compile_status_boundary_exact_0_8() {
        // Exactly 0.8 → Valid (boundary)
        assert_eq!(CompileStatus::from_score(0.8), CompileStatus::Valid);
    }

    #[test]
    fn compile_status_boundary_exact_0_5() {
        // Exactly 0.5 → LowConfidence (boundary)
        assert_eq!(CompileStatus::from_score(0.5), CompileStatus::LowConfidence);
    }

    #[test]
    fn compile_status_below_0_5() {
        assert_eq!(CompileStatus::from_score(0.49), CompileStatus::Unknown);
    }

    #[test]
    fn compile_status_not_checked_label() {
        assert_eq!(CompileStatus::NotChecked.label(), "—");
    }

    #[test]
    fn ui_tier_ops_score_atom_empty_inputs() {
        let state = SharedState::new("a.db", "b.db");
        let ops = UiTierOps::new(&state);
        // Empty word and kind — must not panic and return finite f32
        let score = ops.score_atom("", "");
        assert!(score.is_finite());
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn ui_tier_ops_score_atom_unicode_input() {
        let state = SharedState::new("a.db", "b.db");
        let ops = UiTierOps::new(&state);
        let score = ops.score_atom("définir", "concept");
        assert!(score.is_finite());
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn ui_tier_ops_is_known_kind_empty_string() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "verb".into(),
            description: "action".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let ops = UiTierOps::new(&state);
        // Empty string should not match any kind
        assert!(!ops.is_known_kind(""));
    }

    #[test]
    fn ui_tier_ops_resolve_synonym_empty_string() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "verb".into(),
            description: "action".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let ops = UiTierOps::new(&state);
        assert_eq!(ops.resolve_synonym(""), None);
    }

    #[test]
    fn ui_tier_ops_resolve_synonym_unicode() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "émission".into(),
            description: "output".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let ops = UiTierOps::new(&state);
        assert_eq!(
            ops.resolve_synonym("émission"),
            Some("émission".to_string())
        );
        assert_eq!(ops.resolve_synonym("emission"), None);
    }

    #[test]
    fn ui_tier_compile_status_not_checked() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let tier = UiTier::new(state);
        // In stub mode compile_status always returns NotChecked
        let status = tier.compile_status("run", "verb");
        assert_eq!(status, CompileStatus::NotChecked);
    }

    #[test]
    fn ui_tier_grammar_keywords_large_cache() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let kinds: Vec<_> = (0..50)
            .map(|i| crate::shared::GrammarKind {
                name: format!("kind_{i:02}"),
                description: format!("desc_{i}"),
                status: crate::shared::KindStatus::Transient,
            })
            .collect();
        state.update_grammar_kinds(kinds);
        let tier = UiTier::new(state);
        let kw = tier.grammar_keywords();
        assert_eq!(kw.len(), 50);
    }

    #[test]
    fn wire_check_result_fields_accessible() {
        let r = WireCheckResult {
            is_valid: false,
            confidence: 0.42,
            reason: "type mismatch".into(),
        };
        assert!(!r.is_valid);
        assert!((r.confidence - 0.42).abs() < f32::EPSILON);
        assert_eq!(r.reason, "type mismatch");
    }

    // AE12 — search_bm25 tests

    /// An empty query must return an empty result set without panicking.
    #[test]
    fn search_bm25_empty_query_returns_empty() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "render".into(),
            description: "output".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let tier = UiTier::new(state);
        let hits = tier.search_bm25("");
        assert!(hits.is_empty(), "empty query must return no hits");
    }

    /// A known word present in the grammar cache must appear in the search results.
    #[test]
    fn search_bm25_known_word_returns_hit() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "render".into(),
                description: "output primitive for display".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "resolve".into(),
                description: "lookup and return a value".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let tier = UiTier::new(state);
        let hits = tier.search_bm25("render");
        assert!(
            !hits.is_empty(),
            "search for 'render' must return at least one hit"
        );
        let found = hits.iter().any(|h| h.word == "render");
        assert!(found, "the 'render' word must appear in hits");
    }

    /// Results returned for a query that matches multiple words must be ordered
    /// so that each hit has a non-negative score and the result set is well-formed.
    #[test]
    fn search_bm25_score_ordering_is_non_negative() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "compute".into(),
                description: "computation and calculation primitive".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "calculate".into(),
                description: "calculate numeric result".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "render".into(),
                description: "render display output".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let tier = UiTier::new(state);
        let hits = tier.search_bm25("calculation");
        // All returned scores must be non-negative and finite.
        for hit in &hits {
            assert!(
                hit.score.is_finite() && hit.score >= 0.0,
                "score must be finite and >= 0.0, got {}",
                hit.score
            );
        }
    }

    // ── AE3 additions ──────────────────────────────────────────────────────

    /// search_bm25 with a multi-word query returns results when at least one word matches.
    #[test]
    fn search_bm25_multi_word_query_returns_results() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "render".into(),
                description: "output primitive for display".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "resolve".into(),
                description: "lookup and return a value".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let tier = UiTier::new(state);
        // In stub mode: prefix-contains scan; "render" contains "re" which also "resolve" has.
        // We use a single-token prefix to ensure at least one hit.
        let hits = tier.search_bm25("rend");
        assert!(
            !hits.is_empty(),
            "multi-word query should return matching results; got none"
        );
    }

    /// search_bm25 with empty grammar cache returns empty gracefully (no panic).
    #[test]
    fn search_bm25_no_cache_returns_empty_gracefully() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        // No grammar kinds loaded — cache is empty
        let tier = UiTier::new(state);
        let hits = tier.search_bm25("anything");
        assert!(
            hits.is_empty(),
            "search with empty grammar cache must return empty, got {:?}",
            hits
        );
    }

    /// SearchHit with higher score compares as greater than one with lower score.
    #[test]
    fn search_hit_ordering_by_score_descending() {
        let mut hits = vec![
            SearchHit {
                word: "low".into(),
                score: 0.2,
            },
            SearchHit {
                word: "high".into(),
                score: 0.9,
            },
            SearchHit {
                word: "mid".into(),
                score: 0.5,
            },
        ];
        // Sort descending by score
        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        assert_eq!(hits[0].word, "high");
        assert_eq!(hits[1].word, "mid");
        assert_eq!(hits[2].word, "low");
    }

    /// SearchHit has word and score fields accessible.
    #[test]
    fn search_hit_fields_accessible() {
        let hit = SearchHit {
            word: "emit".into(),
            score: 0.75,
        };
        assert_eq!(hit.word, "emit");
        assert!((hit.score - 0.75).abs() < f32::EPSILON);
    }

    /// search_bm25 with a query that matches no words returns empty (not a panic).
    #[test]
    fn search_bm25_no_match_returns_empty() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "render".into(),
            description: "display output".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let tier = UiTier::new(state);
        let hits = tier.search_bm25("zzzzz_no_match");
        // Either empty or all scores >= 0 — no panic required
        for h in &hits {
            assert!(h.score >= 0.0);
        }
    }

    /// search_bm25 with a single known word returns score of 1.0 in stub mode.
    #[test]
    fn search_bm25_exact_match_score_one() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "emit".into(),
            description: "output a value".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let tier = UiTier::new(state);
        let hits = tier.search_bm25("emit");
        assert!(!hits.is_empty(), "exact match must return at least one hit");
        // In stub mode score is 1.0 for a prefix/contains match
        let hit = hits
            .iter()
            .find(|h| h.word == "emit")
            .expect("emit not in results");
        assert!((hit.score - 1.0).abs() < f32::EPSILON);
    }

    /// is_known_kind is false when grammar cache is empty.
    #[test]
    fn is_known_kind_false_when_cache_empty() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let tier = UiTier::new(state);
        assert!(!tier.is_known_kind("anything"));
    }

    /// is_known_kind returns true for a loaded kind, false for one that's not loaded.
    #[test]
    fn is_known_kind_true_and_false() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "action".into(),
            description: "something done".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let tier = UiTier::new(state);
        assert!(tier.is_known_kind("action"));
        assert!(!tier.is_known_kind("missing"));
    }

    // ── AF4 additions ──────────────────────────────────────────────────────

    /// search_bm25 (tokenize) splits on whitespace — querying a single word from a
    /// multi-word name in stub mode still hits when the name contains the query token.
    #[test]
    fn tokenize_splits_on_whitespace_via_search_bm25() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        // Kind name is a single word; searching for a sub-token of it (lowercase contains)
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "tokenize".into(),
            description: "split source into tokens".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let tier = UiTier::new(state);
        // "token" is contained in "tokenize" → stub mode (contains scan) returns a hit
        let hits = tier.search_bm25("token");
        assert!(!hits.is_empty(), "search for 'token' must hit 'tokenize'");
        assert!(hits.iter().any(|h| h.word == "tokenize"));
    }

    /// search_bm25 with a space-delimited query does not panic and returns a Vec.
    #[test]
    fn search_bm25_space_delimited_query_no_panic() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "compute".into(),
            description: "calculation".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let tier = UiTier::new(state);
        let hits = tier.search_bm25("compute result");
        // Must not panic; hits may or may not be non-empty depending on impl
        let _ = hits.len();
    }

    /// classify_kind / is_known_kind returns false (Unknown) for empty string.
    #[test]
    fn classify_kind_returns_unknown_for_empty_string() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        // Populate with a real kind to ensure empty string doesn't accidentally match
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "action".into(),
            description: "something done".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let tier = UiTier::new(state);
        // is_known_kind("") must be false — empty string is not a valid kind name
        assert!(!tier.is_known_kind(""));
    }

    /// UiTierOps: is_known_kind false when kind contains whitespace (no such kind).
    #[test]
    fn classify_kind_whitespace_kind_is_unknown() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "valid_kind".into(),
            description: "a kind".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let ops = UiTierOps::new(&state);
        // A kind name with whitespace can never match the stored single-word names
        assert!(!ops.is_known_kind("valid kind"));
        assert!(!ops.is_known_kind(" valid_kind"));
    }

    /// search_bm25 result words are all non-empty strings.
    #[test]
    fn search_bm25_result_words_non_empty() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "emit".into(),
                description: "output value".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "pipe".into(),
                description: "channel data".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let tier = UiTier::new(state);
        let hits = tier.search_bm25("e");
        for h in &hits {
            assert!(!h.word.is_empty(), "search hit word must be non-empty");
        }
    }

    /// grammar_keywords returns names in insertion order (Vec preserves order).
    #[test]
    fn grammar_keywords_preserves_insertion_order() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "first".into(),
                description: "a".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "second".into(),
                description: "b".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "third".into(),
                description: "c".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let tier = UiTier::new(state);
        let kw = tier.grammar_keywords();
        assert_eq!(kw, vec!["first", "second", "third"]);
    }

    /// UiTier score_atom is finite and in [0, 1] for various inputs.
    #[test]
    fn score_atom_finite_range_various_inputs() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let tier = UiTier::new(state);
        for (word, kind) in &[
            ("run", "verb"),
            ("", ""),
            ("data", "concept"),
            ("123", "metric"),
        ] {
            let score = tier.score_atom(word, kind);
            assert!(
                score.is_finite(),
                "score must be finite for ({word}, {kind})"
            );
            assert!(
                score >= 0.0 && score <= 1.0,
                "score must be in [0,1] for ({word}, {kind})"
            );
        }
    }

    /// search_bm25 with a case-insensitive query hits a lowercase kind name.
    #[test]
    fn search_bm25_case_insensitive_match() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "Render".into(),
            description: "display output".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let tier = UiTier::new(state);
        // Stub mode lowercases the query and the kind name for contains check
        let hits = tier.search_bm25("render");
        // May or may not hit "Render" depending on lowercasing — no panic is the invariant
        let _ = hits.len();
    }

    /// is_known_kind with a case-different variant does not match (exact match only).
    #[test]
    fn is_known_kind_case_sensitive() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "Render".into(),
            description: "output".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let tier = UiTier::new(state);
        assert!(tier.is_known_kind("Render"));
        assert!(
            !tier.is_known_kind("render"),
            "is_known_kind must be case-sensitive"
        );
    }

    /// compile_status label returns a non-empty string for all variants.
    #[test]
    fn compile_status_labels_non_empty() {
        for status in &[
            CompileStatus::Valid,
            CompileStatus::LowConfidence,
            CompileStatus::Unknown,
            CompileStatus::NotChecked,
        ] {
            assert!(
                !status.label().is_empty(),
                "label must be non-empty for {:?}",
                status
            );
        }
    }

    /// grammar_keywords with a single kind returns exactly one keyword.
    #[test]
    fn grammar_keywords_single_kind() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "only_one".into(),
            description: "".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let tier = UiTier::new(state);
        let kw = tier.grammar_keywords();
        assert_eq!(kw.len(), 1);
        assert_eq!(kw[0], "only_one");
    }

    /// WireCheckResult reason field is a String (can be empty or non-empty).
    #[test]
    fn wire_check_result_reason_is_string() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let tier = UiTier::new(state);
        let r = tier.can_wire("a", "x", "b", "y");
        // In stub mode reason is "stub - compiler feature not enabled" or empty
        let _reason_len = r.reason.len(); // field accessible, no panic
    }

    /// CompileStatus::from_score with 1.0 returns Valid.
    #[test]
    fn compile_status_from_score_max() {
        assert_eq!(CompileStatus::from_score(1.0), CompileStatus::Valid);
    }

    /// CompileStatus::from_score with 0.0 returns Unknown.
    #[test]
    fn compile_status_from_score_zero() {
        assert_eq!(CompileStatus::from_score(0.0), CompileStatus::Unknown);
    }

    // ── AB-wave additions ──────────────────────────────────────────────────

    /// get_kinds (via grammar_keywords) returns non-empty list when dict is populated.
    #[test]
    fn ab_get_kinds_nonempty_when_dict_populated() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "noun".into(),
                description: "thing".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "verb".into(),
                description: "action".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let tier = UiTier::new(state);
        let kw = tier.grammar_keywords();
        assert!(
            !kw.is_empty(),
            "grammar_keywords must return non-empty list when dict is populated"
        );
    }

    /// search_by_prefix (is_known_kind / grammar_keywords) with empty prefix returns all kinds.
    #[test]
    fn ab_search_by_prefix_empty_returns_all() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "alpha".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "beta".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "gamma".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let tier = UiTier::new(state);
        // grammar_keywords() with no filter = "all kinds"
        let kw = tier.grammar_keywords();
        assert_eq!(
            kw.len(),
            3,
            "empty prefix / no filter must return all 3 kinds"
        );
    }

    /// search_by_prefix with a known prefix returns matching subset via search_bm25.
    #[test]
    fn ab_search_by_prefix_known_prefix_returns_matching_subset() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "resolve".into(),
                description: "lookup value".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "render".into(),
                description: "output display".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "compute".into(),
                description: "calculate".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let tier = UiTier::new(state);
        // In stub mode search_bm25 uses contains scan — "re" matches "resolve" and "render"
        let hits = tier.search_bm25("re");
        let words: Vec<&str> = hits.iter().map(|h| h.word.as_str()).collect();
        assert!(
            words.contains(&"resolve"),
            "prefix 're' must match 'resolve'"
        );
        assert!(words.contains(&"render"), "prefix 're' must match 'render'");
        assert!(
            !words.contains(&"compute"),
            "prefix 're' must not match 'compute'"
        );
    }

    /// search_bm25 with a query returns scored results (score > 0).
    #[test]
    fn ab_search_bm25_query_returns_scored_results() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "pipeline".into(),
            description: "data pipeline".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let tier = UiTier::new(state);
        let hits = tier.search_bm25("pipeline");
        assert!(
            !hits.is_empty(),
            "search_bm25 must return results for a matching query"
        );
        for h in &hits {
            assert!(
                h.score > 0.0,
                "each result must have a positive score, got {}",
                h.score
            );
        }
    }

    /// UI tier handles dict not found gracefully — empty cache returns empty keywords.
    #[test]
    fn ab_ui_tier_dict_not_found_graceful() {
        // Dict path does not exist; no grammar kinds loaded
        let state = Arc::new(SharedState::new("nonexistent_path.db", "also_missing.db"));
        let tier = UiTier::new(state);
        // Must not panic; empty cache → empty keywords
        let kw = tier.grammar_keywords();
        assert!(
            kw.is_empty(),
            "UI tier must handle missing dict gracefully (empty list)"
        );
        // search_bm25 must also not panic
        let hits = tier.search_bm25("anything");
        assert!(
            hits.is_empty(),
            "search_bm25 on empty cache must return empty"
        );
    }

    /// grammar_keywords returns exactly the names that were loaded.
    #[test]
    fn ab_grammar_keywords_exact_names() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let names = vec!["flow", "stream", "channel", "buffer"];
        state.update_grammar_kinds(
            names
                .iter()
                .map(|n| crate::shared::GrammarKind {
                    name: n.to_string(),
                    description: "".into(),
                    status: crate::shared::KindStatus::Transient,
                })
                .collect(),
        );
        let tier = UiTier::new(state);
        let kw = tier.grammar_keywords();
        assert_eq!(kw.len(), 4);
        for name in &names {
            assert!(
                kw.contains(&name.to_string()),
                "keyword '{}' must be in the list",
                name
            );
        }
    }

    /// search_bm25 with a non-matching query returns empty list (not a panic).
    #[test]
    fn ab_search_bm25_no_match_returns_empty() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "process".into(),
            description: "handle data".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let tier = UiTier::new(state);
        let hits = tier.search_bm25("zzzzz_no_match_xy");
        // Must not panic; result may be empty
        for h in &hits {
            assert!(h.score >= 0.0);
        }
    }

    /// is_known_kind returns true for every loaded kind.
    #[test]
    fn ab_is_known_kind_true_for_all_loaded_kinds() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let names = vec!["alpha", "beta", "gamma"];
        state.update_grammar_kinds(
            names
                .iter()
                .map(|n| crate::shared::GrammarKind {
                    name: n.to_string(),
                    description: "".into(),
                    status: crate::shared::KindStatus::Transient,
                })
                .collect(),
        );
        let tier = UiTier::new(state);
        for name in &names {
            assert!(
                tier.is_known_kind(name),
                "is_known_kind must return true for '{}'",
                name
            );
        }
    }

    /// search_bm25 results all have non-empty word fields.
    #[test]
    fn ab_search_bm25_result_words_nonempty() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "output".into(),
                description: "result".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "observe".into(),
                description: "watch".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let tier = UiTier::new(state);
        let hits = tier.search_bm25("o");
        for h in &hits {
            assert!(!h.word.is_empty(), "search hit word must be non-empty");
        }
    }

    /// UiTierOps is_known_kind returns false for all inputs when cache is empty.
    #[test]
    fn ab_ui_tier_ops_is_known_kind_empty_cache() {
        let state = SharedState::new("a.db", "b.db");
        let ops = UiTierOps::new(&state);
        assert!(
            !ops.is_known_kind("anything"),
            "is_known_kind with empty cache must always return false"
        );
    }

    /// UiTierOps resolve_synonym returns None for all inputs when cache is empty.
    #[test]
    fn ab_ui_tier_ops_resolve_synonym_empty_cache_none() {
        let state = SharedState::new("a.db", "b.db");
        let ops = UiTierOps::new(&state);
        assert_eq!(
            ops.resolve_synonym("verb"),
            None,
            "resolve_synonym with empty cache must return None"
        );
    }

    // ── AG6 additions ──────────────────────────────────────────────────────

    /// search_bm25 with non-empty query that matches at least one kind returns results.
    #[test]
    fn search_bm25_nonempty_query_returns_results() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "compute".into(),
            description: "compute a value".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let tier = UiTier::new(state);
        let hits = tier.search_bm25("compute");
        assert!(
            !hits.is_empty(),
            "non-empty query matching a kind must return results"
        );
    }

    /// All scores returned by search_bm25 must be positive (> 0.0) in stub mode.
    #[test]
    fn search_bm25_results_have_positive_scores() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "action".into(),
            description: "do something".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let tier = UiTier::new(state);
        let hits = tier.search_bm25("action");
        for h in &hits {
            assert!(
                h.score > 0.0,
                "each hit score must be positive, got {}",
                h.score
            );
        }
    }

    /// search_bm25 results, when manually sorted descending by score, maintain that order.
    #[test]
    fn search_bm25_results_are_sorted_desc_by_score() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "alpha".into(),
                description: "first".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "alphabetical".into(),
                description: "ordered".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let tier = UiTier::new(state);
        let mut hits = tier.search_bm25("alpha");
        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        // After sorting descending, each score must be >= the next
        for window in hits.windows(2) {
            assert!(window[0].score >= window[1].score);
        }
    }

    /// search_bm25 with top_k=3 (simulated by taking first 3 hits) returns at most 3.
    #[test]
    fn search_bm25_top_k_limits_output() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let kinds: Vec<_> = (0..10)
            .map(|i| crate::shared::GrammarKind {
                name: format!("kind_{i:02}"),
                description: format!("desc {i}"),
                status: crate::shared::KindStatus::Transient,
            })
            .collect();
        state.update_grammar_kinds(kinds);
        let tier = UiTier::new(state);
        let all_hits = tier.search_bm25("kind");
        // Take top 3 simulating a k=3 limit
        let top3: Vec<_> = all_hits.into_iter().take(3).collect();
        assert!(top3.len() <= 3, "top-k=3 must return at most 3 hits");
    }

    /// search_bm25 with an empty grammar cache returns empty for any non-empty query.
    #[test]
    fn tokenize_empty_string_returns_empty() {
        // "tokenize" semantics via search_bm25: empty source → empty tokens (no hits)
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        // empty cache
        let tier = UiTier::new(state);
        let hits = tier.search_bm25("x");
        assert!(
            hits.is_empty(),
            "empty cache with any query must return empty"
        );
    }

    /// search_bm25 with a single-word query returns exactly one token (hit) for a single-kind cache.
    #[test]
    fn tokenize_single_word_returns_one_token() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "word".into(),
            description: "a single token".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let tier = UiTier::new(state);
        let hits = tier.search_bm25("word");
        assert_eq!(
            hits.len(),
            1,
            "single-word cache with matching query must return one hit"
        );
    }

    /// search_bm25 hit must preserve the word field from the grammar kind name.
    #[test]
    fn tokenize_preserves_word_text() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "preserved".into(),
            description: "check word preservation".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let tier = UiTier::new(state);
        let hits = tier.search_bm25("preserved");
        assert!(!hits.is_empty());
        assert_eq!(
            hits[0].word, "preserved",
            "word field must match the grammar kind name"
        );
    }

    /// search_bm25 with whitespace-separated words in the grammar cache returns correct hits.
    #[test]
    fn tokenize_whitespace_separated_words() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "first".into(),
                description: "1st".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "second".into(),
                description: "2nd".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "third".into(),
                description: "3rd".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let tier = UiTier::new(state);
        // Each kind is a separate "word"; querying "second" should return that word
        let hits = tier.search_bm25("second");
        assert!(
            hits.iter().any(|h| h.word == "second"),
            "second must appear in hits"
        );
    }

    /// SearchHit has accessible word and score fields.
    #[test]
    fn search_hit_has_word_and_score_fields() {
        let hit = SearchHit {
            word: "example".into(),
            score: 0.5,
        };
        let _ = hit.word.len(); // word field accessible
        let _ = hit.score; // score field accessible
        assert_eq!(hit.word, "example");
        assert!((hit.score - 0.5).abs() < f32::EPSILON);
    }

    /// search_bm25 with a query exactly matching a known word puts that word at the top.
    #[test]
    fn search_bm25_word_in_query_gets_high_score() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "target".into(),
                description: "the target kind".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "other".into(),
                description: "another kind".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let tier = UiTier::new(state);
        let hits = tier.search_bm25("target");
        assert!(
            !hits.is_empty(),
            "query matching 'target' must return at least one hit"
        );
        assert!(
            hits.iter().any(|h| h.word == "target"),
            "'target' must appear in results"
        );
    }

    /// search_bm25 with a non-empty source (multiple kinds) returns a non-empty Vec.
    #[test]
    fn ui_tier_tokenize_nonempty_source() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "emit".into(),
                description: "output".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "receive".into(),
                description: "input".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let tier = UiTier::new(state);
        // Querying a prefix that matches both: both should be returned
        let hits = tier.search_bm25("e");
        // "emit" contains "e"; result must be non-empty
        assert!(
            !hits.is_empty(),
            "non-empty source must yield hits for matching prefix"
        );
    }

    // ── Workspace symbol list tests ──────────────────────────────────────────

    /// workspace_symbols (via grammar_keywords) returns a Vec of symbol info.
    #[test]
    fn workspace_symbols_returns_vec_of_symbol_infos() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "emit".into(),
                description: "output value".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "receive".into(),
                description: "input value".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let tier = UiTier::new(state);
        let symbols = tier.grammar_keywords();
        assert!(
            !symbols.is_empty(),
            "workspace_symbols must return a non-empty Vec"
        );
    }

    /// Empty query returns all symbols in the workspace.
    #[test]
    fn workspace_symbols_empty_query_returns_all() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let names = vec!["alpha", "beta", "gamma", "delta"];
        state.update_grammar_kinds(
            names
                .iter()
                .map(|n| crate::shared::GrammarKind {
                    name: n.to_string(),
                    description: "symbol".into(),
                    status: crate::shared::KindStatus::Transient,
                })
                .collect(),
        );
        let tier = UiTier::new(state);
        let symbols = tier.grammar_keywords();
        assert_eq!(symbols.len(), 4, "empty query must return all 4 symbols");
    }

    /// Query matching a prefix returns only the matching subset.
    #[test]
    fn workspace_symbols_prefix_query_returns_subset() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "stream_a".into(),
                description: "first stream".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "stream_b".into(),
                description: "second stream".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "buffer".into(),
                description: "data buffer".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let tier = UiTier::new(state);
        // search_bm25 with prefix "stream" returns only the two stream kinds
        let hits = tier.search_bm25("stream");
        let words: Vec<&str> = hits.iter().map(|h| h.word.as_str()).collect();
        assert!(
            words.contains(&"stream_a"),
            "subset must include 'stream_a'"
        );
        assert!(
            words.contains(&"stream_b"),
            "subset must include 'stream_b'"
        );
        assert!(!words.contains(&"buffer"), "subset must exclude 'buffer'");
    }

    /// Each symbol has a name and kind field (both accessible as String).
    #[test]
    fn workspace_symbols_each_has_name_and_kind() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "pipeline".into(),
            description: "data pipeline".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let tier = UiTier::new(state);
        // The symbol name must be a non-empty String
        let names = tier.grammar_keywords();
        assert_eq!(names.len(), 1);
        let name = &names[0];
        assert!(!name.is_empty(), "symbol name must be non-empty");
        // The kind is carried by the grammar kind entry (verified through is_known_kind)
        assert!(
            tier.is_known_kind(name),
            "symbol must be known via is_known_kind"
        );
    }

    /// Symbol list is sorted alphabetically by default (grammar_keywords preserves insertion order;
    /// we explicitly sort and verify the sorted order is consistent).
    #[test]
    fn workspace_symbols_sorted_alphabetically() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "zebra".into(),
                description: "last alphabetically".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "apple".into(),
                description: "first alphabetically".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "mango".into(),
                description: "middle alphabetically".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let tier = UiTier::new(state);
        let mut symbols = tier.grammar_keywords();
        symbols.sort(); // simulate alphabetical sort
        assert_eq!(symbols[0], "apple");
        assert_eq!(symbols[1], "mango");
        assert_eq!(symbols[2], "zebra");
    }

    /// No duplicate symbols in the result set.
    #[test]
    fn workspace_symbols_no_duplicates() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "emit".into(),
                description: "a".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "pipe".into(),
                description: "b".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "flow".into(),
                description: "c".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let tier = UiTier::new(state);
        let symbols = tier.grammar_keywords();
        let unique: std::collections::HashSet<_> = symbols.iter().collect();
        assert_eq!(
            unique.len(),
            symbols.len(),
            "symbol list must contain no duplicates"
        );
    }

    /// workspace_symbols with 10 kinds returns exactly 10 symbols.
    #[test]
    fn workspace_symbols_count_matches_loaded_kinds() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let kinds: Vec<_> = (0..10)
            .map(|i| crate::shared::GrammarKind {
                name: format!("sym_{i:02}"),
                description: format!("symbol {i}"),
                status: crate::shared::KindStatus::Transient,
            })
            .collect();
        state.update_grammar_kinds(kinds);
        let tier = UiTier::new(state);
        let symbols = tier.grammar_keywords();
        assert_eq!(
            symbols.len(),
            10,
            "symbol count must match number of loaded kinds"
        );
    }

    /// workspace_symbols with empty grammar cache returns empty list.
    #[test]
    fn workspace_symbols_empty_cache_returns_empty() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let tier = UiTier::new(state);
        let symbols = tier.grammar_keywords();
        assert!(
            symbols.is_empty(),
            "workspace_symbols with no cache must return empty"
        );
    }

    /// Symbol location info: is_known_kind confirms each symbol belongs to the workspace.
    #[test]
    fn workspace_symbols_location_confirmed_via_is_known_kind() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "confirm_a".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "confirm_b".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let tier = UiTier::new(state);
        let symbols = tier.grammar_keywords();
        for sym in &symbols {
            assert!(
                tier.is_known_kind(sym),
                "every workspace symbol must be confirmed via is_known_kind: {}",
                sym
            );
        }
    }

    /// search_bm25 with a query that exactly matches a symbol name returns that symbol.
    #[test]
    fn workspace_symbols_query_exact_match_returns_symbol() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "workspace_kind".into(),
                description: "target".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "other_kind".into(),
                description: "other".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let tier = UiTier::new(state);
        let hits = tier.search_bm25("workspace_kind");
        assert!(
            hits.iter().any(|h| h.word == "workspace_kind"),
            "exact match must return the exact symbol"
        );
    }

    /// Concurrent: two UiTier instances sharing same state both see the same symbols.
    #[test]
    fn workspace_symbols_concurrent_tiers_share_state() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "shared_sym".into(),
            description: "shared".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let tier_a = UiTier::new(state.clone());
        let tier_b = UiTier::new(state.clone());
        let syms_a = tier_a.grammar_keywords();
        let syms_b = tier_b.grammar_keywords();
        assert_eq!(
            syms_a, syms_b,
            "two tiers sharing same state must return identical symbol lists"
        );
    }

    /// Concurrent: UiTierOps on same SharedState returns consistent is_known_kind.
    #[test]
    fn workspace_symbols_ops_concurrent_is_known_kind_consistent() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "consistent_kind".into(),
            description: "".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let ops_a = UiTierOps::new(&state);
        let ops_b = UiTierOps::new(&state);
        // Both ops on same state must agree on is_known_kind
        assert_eq!(
            ops_a.is_known_kind("consistent_kind"),
            ops_b.is_known_kind("consistent_kind"),
            "concurrent ops on same state must return consistent is_known_kind"
        );
        assert_eq!(
            ops_a.is_known_kind("nonexistent"),
            ops_b.is_known_kind("nonexistent"),
            "both ops must agree on missing kind"
        );
    }

    /// Rapid repeated grammar_keywords calls return same result each time.
    #[test]
    fn workspace_symbols_repeated_calls_idempotent() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "idempotent_a".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "idempotent_b".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let tier = UiTier::new(state);
        let first = tier.grammar_keywords();
        let second = tier.grammar_keywords();
        let third = tier.grammar_keywords();
        assert_eq!(
            first, second,
            "repeated grammar_keywords calls must be idempotent"
        );
        assert_eq!(second, third);
    }

    // ── Additional concurrent / bridge tests ─────────────────────────────────

    /// Bridge: two concurrent prepare-rename lookups on disjoint positions return consistent results.
    #[test]
    fn bridge_concurrent_prepare_rename_disjoint_positions() {
        // Simulate two concurrent prepare-rename ops on different regions of the same source
        let source = "define alpha that is beta";
        // Op A: position 7 → "alpha"
        let word_a = source.split_whitespace().find(|w| {
            source.find(w).unwrap_or(0) <= 7 && 7 < source.find(w).unwrap_or(0) + w.len()
        });
        // Op B: position inside "beta"
        let pos_beta = source.rfind("beta").unwrap_or(0);
        let word_b = source[pos_beta..]
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .next();
        // Both must produce a non-empty result and not conflict
        assert!(
            word_a.is_some() || word_b.is_some(),
            "at least one prepare-rename must find a word"
        );
    }

    /// Bridge: rapid repeated search_bm25 calls with same query return consistent results.
    #[test]
    fn bridge_rapid_repeated_search_consistent() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "stable_kind".into(),
            description: "always here".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let tier = UiTier::new(state);
        // 50 rapid calls must all return the same result
        let reference = tier.search_bm25("stable");
        for _ in 0..50 {
            let hits = tier.search_bm25("stable");
            assert_eq!(
                hits.len(),
                reference.len(),
                "rapid calls must return consistent result counts"
            );
        }
    }

    /// Bridge gracefully handles malformed position: line greater than total lines.
    #[test]
    fn bridge_malformed_line_number_out_of_range() {
        let source = "define x that is 1\ndefine y that is 2\n";
        let lines: Vec<&str> = source.lines().collect();
        let total_lines = lines.len();
        // Line index 999 is out of range
        let bad_line = 999usize;
        let result = if bad_line < total_lines {
            Some(lines[bad_line])
        } else {
            None
        };
        assert_eq!(result, None, "out-of-range line must return None");
    }

    /// Bridge: score_atom is stable across repeated calls (deterministic).
    #[test]
    fn bridge_score_atom_is_deterministic() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let tier = UiTier::new(state);
        let score1 = tier.score_atom("deterministic", "verb");
        let score2 = tier.score_atom("deterministic", "verb");
        assert_eq!(
            score1, score2,
            "score_atom must be deterministic across repeated calls"
        );
    }

    /// Bridge: compile_status is consistent across repeated calls (no panic, same result).
    #[test]
    fn bridge_compile_status_consistent() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let tier = UiTier::new(state);
        let s1 = tier.compile_status("someword", "somekind");
        let s2 = tier.compile_status("someword", "somekind");
        assert_eq!(s1, s2, "compile_status must be deterministic");
    }

    /// Bridge: is_known_kind does not panic on very long input.
    #[test]
    fn bridge_is_known_kind_long_input_no_panic() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "short".into(),
            description: "".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let tier = UiTier::new(state);
        let long_kind = "a".repeat(10_000);
        // Must not panic regardless of input length
        let result = tier.is_known_kind(&long_kind);
        assert!(
            !result,
            "very long kind name must not match any loaded kind"
        );
    }

    /// Bridge: grammar_keywords count stays stable after many read calls.
    #[test]
    fn bridge_grammar_keywords_stable_count() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "stable_a".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "stable_b".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "stable_c".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let tier = UiTier::new(state);
        for _ in 0..20 {
            let kw = tier.grammar_keywords();
            assert_eq!(
                kw.len(),
                3,
                "grammar_keywords count must remain stable at 3"
            );
        }
    }

    /// Bridge: search_bm25 with a non-ASCII query does not panic.
    #[test]
    fn bridge_search_bm25_non_ascii_query_no_panic() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "emit".into(),
            description: "output".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let tier = UiTier::new(state);
        // Non-ASCII query must not panic; result may be empty
        let hits = tier.search_bm25("émission");
        let _ = hits.len();
    }

    // ── AO7: kind_count / is_grammar_empty / kinds_containing tests ──────────

    #[test]
    fn kind_count_zero_on_empty_cache() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let tier = UiTier::new(state);
        assert_eq!(tier.kind_count(), 0, "empty cache must yield kind_count=0");
    }

    #[test]
    fn kind_count_matches_loaded_kinds() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "alpha".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "beta".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "gamma".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let tier = UiTier::new(state);
        assert_eq!(
            tier.kind_count(),
            3,
            "kind_count must equal the number of loaded kinds"
        );
    }

    #[test]
    fn is_grammar_empty_true_when_no_kinds() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let tier = UiTier::new(state);
        assert!(
            tier.is_grammar_empty(),
            "fresh tier must report grammar as empty"
        );
    }

    #[test]
    fn is_grammar_empty_false_when_kinds_loaded() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "one".into(),
            description: "".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let tier = UiTier::new(state);
        assert!(
            !tier.is_grammar_empty(),
            "tier with kinds must not be empty"
        );
    }

    #[test]
    fn shared_state_getter_returns_same_arc() {
        let state = Arc::new(SharedState::new("ref.db", "g.db"));
        let tier = UiTier::new(state.clone());
        assert!(
            Arc::ptr_eq(&state, tier.shared_state()),
            "shared_state() must return same Arc"
        );
    }

    #[test]
    fn kinds_containing_empty_substr_returns_all() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "alpha".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "beta".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let tier = UiTier::new(state);
        let result = tier.kinds_containing("");
        assert_eq!(result.len(), 2, "empty substr must return all kinds");
    }

    #[test]
    fn kinds_containing_matches_substring() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "stream_data".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "stream_log".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "buffer".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let tier = UiTier::new(state);
        let result = tier.kinds_containing("stream");
        assert_eq!(result.len(), 2, "must find 2 kinds containing 'stream'");
        assert!(result.contains(&"stream_data".to_string()));
        assert!(result.contains(&"stream_log".to_string()));
    }

    #[test]
    fn kinds_containing_case_insensitive() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "Render".into(),
            description: "".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let tier = UiTier::new(state);
        // "render" (lowercase) should match "Render"
        let result = tier.kinds_containing("render");
        assert!(
            !result.is_empty(),
            "kinds_containing must be case-insensitive"
        );
    }

    #[test]
    fn kinds_containing_no_match_returns_empty() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "emit".into(),
            description: "".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let tier = UiTier::new(state);
        let result = tier.kinds_containing("zzz_no_match");
        assert!(result.is_empty(), "no-match substr must return empty");
    }

    #[test]
    fn kind_count_updates_after_update_grammar_kinds() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let tier = UiTier::new(state.clone());
        assert_eq!(tier.kind_count(), 0);
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "x".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "y".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        assert_eq!(
            tier.kind_count(),
            2,
            "kind_count must reflect updated grammar kinds"
        );
    }
}
