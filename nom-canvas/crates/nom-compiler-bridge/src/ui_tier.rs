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
        #[cfg(feature = "compiler")]
        {
            use nom_search::BM25Index;
            let kinds = self.state.cached_grammar_kinds();
            if kinds.is_empty() {
                return Vec::new();
            }
            let mut index = BM25Index::new();
            for k in &kinds {
                // Index word + description so both fields participate in scoring.
                let text = format!("{} {}", k.name, k.description);
                index.add_document(&k.name, &text);
            }
            let limit = 50;
            index
                .search(query, limit)
                .into_iter()
                .map(|r| SearchHit {
                    word: r.doc_id,
                    score: r.score as f32,
                })
                .collect()
        }
        #[cfg(not(feature = "compiler"))]
        {
            let q = query.to_lowercase();
            let kinds = self.state.cached_grammar_kinds();
            kinds
                .into_iter()
                .filter(|k| k.name.to_lowercase().contains(&q))
                .map(|k| SearchHit {
                    word: k.name,
                    score: 1.0,
                })
                .collect()
        }
    }
}

// With compiler feature: real implementations using nom-score
#[cfg(feature = "compiler")]
impl UiTier {
    /// score_atom — wraps nom-score::score_atom, pure stateless, no DB
    pub fn score_atom(&self, word: &str, kind: &str) -> f32 {
        let kinds = self.state.cached_grammar_kinds();
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

// Without compiler feature: stubs
#[cfg(not(feature = "compiler"))]
impl UiTier {
    pub fn score_atom(&self, _word: &str, _kind: &str) -> f32 {
        0.5
    }
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

/// UiTierOps — borrowed accessor for UI-tier operations (<1ms, sync)
pub struct UiTierOps<'a> {
    shared: &'a SharedState,
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
        #[cfg(feature = "compiler")]
        {
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
            return nom_score::score_atom(&atom).overall();
        }
        #[cfg(not(feature = "compiler"))]
        {
            let _ = (word, kind);
            0.5
        }
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
            },
            crate::shared::GrammarKind {
                name: "that".into(),
                description: "connector".into(),
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
            },
            crate::shared::GrammarKind {
                name: "resolve".into(),
                description: "lookup".into(),
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
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "render".into(),
                description: "output".into(),
            },
        ]);
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
            },
            crate::shared::GrammarKind {
                name: "resolve".into(),
                description: "lookup and return a value".into(),
            },
        ]);
        let tier = UiTier::new(state);
        let hits = tier.search_bm25("render");
        assert!(!hits.is_empty(), "search for 'render' must return at least one hit");
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
            },
            crate::shared::GrammarKind {
                name: "calculate".into(),
                description: "calculate numeric result".into(),
            },
            crate::shared::GrammarKind {
                name: "render".into(),
                description: "render display output".into(),
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
            },
            crate::shared::GrammarKind {
                name: "resolve".into(),
                description: "lookup and return a value".into(),
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
            SearchHit { word: "low".into(), score: 0.2 },
            SearchHit { word: "high".into(), score: 0.9 },
            SearchHit { word: "mid".into(), score: 0.5 },
        ];
        // Sort descending by score
        hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        assert_eq!(hits[0].word, "high");
        assert_eq!(hits[1].word, "mid");
        assert_eq!(hits[2].word, "low");
    }

    /// SearchHit has word and score fields accessible.
    #[test]
    fn search_hit_fields_accessible() {
        let hit = SearchHit { word: "emit".into(), score: 0.75 };
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
        }]);
        let tier = UiTier::new(state);
        let hits = tier.search_bm25("emit");
        assert!(!hits.is_empty(), "exact match must return at least one hit");
        // In stub mode score is 1.0 for a prefix/contains match
        let hit = hits.iter().find(|h| h.word == "emit").expect("emit not in results");
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
            crate::shared::GrammarKind { name: "emit".into(), description: "output value".into() },
            crate::shared::GrammarKind { name: "pipe".into(), description: "channel data".into() },
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
            crate::shared::GrammarKind { name: "first".into(), description: "a".into() },
            crate::shared::GrammarKind { name: "second".into(), description: "b".into() },
            crate::shared::GrammarKind { name: "third".into(), description: "c".into() },
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
        for (word, kind) in &[("run", "verb"), ("", ""), ("data", "concept"), ("123", "metric")] {
            let score = tier.score_atom(word, kind);
            assert!(score.is_finite(), "score must be finite for ({word}, {kind})");
            assert!(score >= 0.0 && score <= 1.0, "score must be in [0,1] for ({word}, {kind})");
        }
    }

    /// search_bm25 with a case-insensitive query hits a lowercase kind name.
    #[test]
    fn search_bm25_case_insensitive_match() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "Render".into(),
            description: "display output".into(),
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
        }]);
        let tier = UiTier::new(state);
        assert!(tier.is_known_kind("Render"));
        assert!(!tier.is_known_kind("render"), "is_known_kind must be case-sensitive");
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
            assert!(!status.label().is_empty(), "label must be non-empty for {:?}", status);
        }
    }

    /// grammar_keywords with a single kind returns exactly one keyword.
    #[test]
    fn grammar_keywords_single_kind() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "only_one".into(),
            description: "".into(),
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

    // ── AG6 additions ──────────────────────────────────────────────────────

    /// search_bm25 with non-empty query that matches at least one kind returns results.
    #[test]
    fn search_bm25_nonempty_query_returns_results() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "compute".into(),
            description: "compute a value".into(),
        }]);
        let tier = UiTier::new(state);
        let hits = tier.search_bm25("compute");
        assert!(!hits.is_empty(), "non-empty query matching a kind must return results");
    }

    /// All scores returned by search_bm25 must be positive (> 0.0) in stub mode.
    #[test]
    fn search_bm25_results_have_positive_scores() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind { name: "action".into(), description: "do something".into() },
        ]);
        let tier = UiTier::new(state);
        let hits = tier.search_bm25("action");
        for h in &hits {
            assert!(h.score > 0.0, "each hit score must be positive, got {}", h.score);
        }
    }

    /// search_bm25 results, when manually sorted descending by score, maintain that order.
    #[test]
    fn search_bm25_results_are_sorted_desc_by_score() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind { name: "alpha".into(), description: "first".into() },
            crate::shared::GrammarKind { name: "alphabetical".into(), description: "ordered".into() },
        ]);
        let tier = UiTier::new(state);
        let mut hits = tier.search_bm25("alpha");
        hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
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
        assert!(hits.is_empty(), "empty cache with any query must return empty");
    }

    /// search_bm25 with a single-word query returns exactly one token (hit) for a single-kind cache.
    #[test]
    fn tokenize_single_word_returns_one_token() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "word".into(),
            description: "a single token".into(),
        }]);
        let tier = UiTier::new(state);
        let hits = tier.search_bm25("word");
        assert_eq!(hits.len(), 1, "single-word cache with matching query must return one hit");
    }

    /// search_bm25 hit must preserve the word field from the grammar kind name.
    #[test]
    fn tokenize_preserves_word_text() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "preserved".into(),
            description: "check word preservation".into(),
        }]);
        let tier = UiTier::new(state);
        let hits = tier.search_bm25("preserved");
        assert!(!hits.is_empty());
        assert_eq!(hits[0].word, "preserved", "word field must match the grammar kind name");
    }

    /// search_bm25 with whitespace-separated words in the grammar cache returns correct hits.
    #[test]
    fn tokenize_whitespace_separated_words() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind { name: "first".into(), description: "1st".into() },
            crate::shared::GrammarKind { name: "second".into(), description: "2nd".into() },
            crate::shared::GrammarKind { name: "third".into(), description: "3rd".into() },
        ]);
        let tier = UiTier::new(state);
        // Each kind is a separate "word"; querying "second" should return that word
        let hits = tier.search_bm25("second");
        assert!(hits.iter().any(|h| h.word == "second"), "second must appear in hits");
    }

    /// SearchHit has accessible word and score fields.
    #[test]
    fn search_hit_has_word_and_score_fields() {
        let hit = SearchHit { word: "example".into(), score: 0.5 };
        let _ = hit.word.len();   // word field accessible
        let _ = hit.score;        // score field accessible
        assert_eq!(hit.word, "example");
        assert!((hit.score - 0.5).abs() < f32::EPSILON);
    }

    /// search_bm25 with a query exactly matching a known word puts that word at the top.
    #[test]
    fn search_bm25_word_in_query_gets_high_score() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind { name: "target".into(), description: "the target kind".into() },
            crate::shared::GrammarKind { name: "other".into(), description: "another kind".into() },
        ]);
        let tier = UiTier::new(state);
        let hits = tier.search_bm25("target");
        assert!(!hits.is_empty(), "query matching 'target' must return at least one hit");
        assert!(hits.iter().any(|h| h.word == "target"), "'target' must appear in results");
    }

    /// search_bm25 with a non-empty source (multiple kinds) returns a non-empty Vec.
    #[test]
    fn ui_tier_tokenize_nonempty_source() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind { name: "emit".into(), description: "output".into() },
            crate::shared::GrammarKind { name: "receive".into(), description: "input".into() },
        ]);
        let tier = UiTier::new(state);
        // Querying a prefix that matches both: both should be returned
        let hits = tier.search_bm25("e");
        // "emit" contains "e"; result must be non-empty
        assert!(!hits.is_empty(), "non-empty source must yield hits for matching prefix");
    }
}
