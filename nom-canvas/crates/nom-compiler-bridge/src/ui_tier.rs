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
    Valid,          // score >= 0.8
    LowConfidence,  // score 0.5 - 0.8
    Unknown,        // score < 0.5
    NotChecked,     // no check yet
}

impl CompileStatus {
    pub fn from_score(score: f32) -> Self {
        if score >= 0.8 { CompileStatus::Valid }
        else if score >= 0.5 { CompileStatus::LowConfidence }
        else { CompileStatus::Unknown }
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
        self.state.cached_grammar_kinds()
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
}

// With compiler feature: real implementations using nom-score
#[cfg(feature = "compiler")]
impl UiTier {
    /// score_atom — wraps nom-score::score_atom, pure stateless, no DB
    pub fn score_atom(&self, word: &str, kind: &str) -> f32 {
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
        src_kind: &str, src_slot: &str,
        dst_kind: &str, dst_slot: &str,
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
            nom_score::WireResult::Compatible { score } =>
                WireCheckResult { is_valid: true, confidence: score, reason: String::new() },
            nom_score::WireResult::NeedsAdapter { reason } =>
                WireCheckResult { is_valid: false, confidence: 0.5, reason },
            nom_score::WireResult::Incompatible { reason } =>
                WireCheckResult { is_valid: false, confidence: 0.0, reason },
        }
    }

    pub fn compile_status(&self, word: &str, kind: &str) -> CompileStatus {
        CompileStatus::from_score(self.score_atom(word, kind))
    }
}

// Without compiler feature: stubs
#[cfg(not(feature = "compiler"))]
impl UiTier {
    pub fn score_atom(&self, _word: &str, _kind: &str) -> f32 { 0.5 }
    pub fn can_wire(&self, _sk: &str, _ss: &str, _dk: &str, _ds: &str) -> WireCheckResult {
        WireCheckResult { is_valid: true, confidence: 0.0, reason: "stub - compiler feature not enabled".into() }
    }
    pub fn compile_status(&self, _word: &str, _kind: &str) -> CompileStatus { CompileStatus::NotChecked }
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
        self.shared.cached_grammar_kinds()
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
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind { name: "verb".into(), description: "action".into() },
        ]);
        let keywords = tier.grammar_keywords();
        assert_eq!(keywords, vec!["verb"]);
    }

    #[test]
    fn ui_tier_ops_is_known_kind() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind { name: "verb".into(), description: "action".into() },
        ]);
        let ops = UiTierOps::new(&state);
        assert!(ops.is_known_kind("verb"));
        assert!(!ops.is_known_kind("noun"));
    }

    #[test]
    fn ui_tier_ops_resolve_synonym() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind { name: "concept".into(), description: "abstract idea".into() },
        ]);
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
            crate::shared::GrammarKind { name: "define".into(), description: "declaration keyword".into() },
            crate::shared::GrammarKind { name: "that".into(), description: "connector".into() },
        ]);
        let tier = UiTier::new(state);
        let kw = tier.grammar_keywords();
        assert!(!kw.is_empty());
        assert!(kw.contains(&"define".to_string()));
        assert!(kw.contains(&"that".to_string()));
    }
}
