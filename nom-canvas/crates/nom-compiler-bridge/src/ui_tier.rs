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

/// UI tier — all functions are sync and must complete in <2ms
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
}
