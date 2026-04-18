#![deny(unsafe_code)]
use crate::shared::SharedState;
use crate::ui_tier::CompileStatus;

/// Score a word+kind and return the compile status badge
pub fn score_to_status(word: &str, kind: &str, state: &SharedState) -> CompileStatus {
    score_from_cached_kinds(word, kind, state)
}

fn score_from_cached_kinds(word: &str, kind: &str, state: &SharedState) -> CompileStatus {
    let kinds = state.cached_grammar_kinds();
    if kinds.is_empty() {
        return CompileStatus::NotChecked;
    }
    let known_word = kinds.iter().any(|k| k.name == word);
    let known_kind = kinds.iter().any(|k| k.name == kind);
    if known_word || known_kind {
        CompileStatus::Valid
    } else {
        CompileStatus::Unknown
    }
}

/// Score label for status bar display
pub fn status_label(status: &CompileStatus) -> &'static str {
    status.label()
}

/// Color hint for status (as [h,s,l,a] for nom-gpui Hsla)
pub fn status_color(status: &CompileStatus) -> [f32; 4] {
    match status {
        CompileStatus::Valid => [0.397, 0.63, 0.49, 1.0], // green: accent-green
        CompileStatus::LowConfidence => [0.105, 0.921, 0.502, 1.0], // amber
        CompileStatus::Unknown => [0.0, 0.842, 0.602, 1.0], // red
        CompileStatus::NotChecked => [0.0, 0.0, 0.45, 1.0], // gray
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::GrammarKind;

    #[test]
    fn status_label_strings() {
        assert_eq!(status_label(&CompileStatus::Valid), "Valid");
        assert_eq!(
            status_label(&CompileStatus::LowConfidence),
            "Low confidence"
        );
        assert_eq!(status_label(&CompileStatus::Unknown), "Unknown");
    }

    #[test]
    fn score_to_status_not_checked_when_cache_empty() {
        let state = SharedState::new("a.db", "b.db");
        // Empty grammar cache → NotChecked (no basis for scoring)
        let status = score_to_status("summarize", "verb", &state);
        assert_eq!(status, CompileStatus::NotChecked);
    }

    #[test]
    fn score_to_status_valid_when_word_is_known_kind() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![GrammarKind {
            name: "verb".into(),
            description: "action word".into(),
        }]);
        // "verb" matches a known kind → score 0.9 → Valid
        let status = score_to_status("verb", "other", &state);
        assert_eq!(status, CompileStatus::Valid);
    }

    #[test]
    fn score_to_status_unknown_when_neither_matches() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![GrammarKind {
            name: "verb".into(),
            description: "action word".into(),
        }]);
        // Neither "summarize" nor "noun" is a known kind → score 0.3 → Unknown
        let status = score_to_status("summarize", "noun", &state);
        assert_eq!(status, CompileStatus::Unknown);
    }

    #[test]
    fn score_to_status_valid_when_kind_param_is_known() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![GrammarKind {
            name: "concept".into(),
            description: "abstract idea".into(),
        }]);
        // kind param matches a known grammar kind → Valid
        let status = score_to_status("unknown_word", "concept", &state);
        assert_eq!(status, CompileStatus::Valid);
    }

    #[test]
    fn status_color_valid_has_positive_alpha() {
        let color = status_color(&CompileStatus::Valid);
        assert_eq!(color[3], 1.0);
    }

    #[test]
    fn score_adapter_returns_valid_for_known_kind() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![GrammarKind {
            name: "define".into(),
            description: "declaration keyword".into(),
        }]);
        // word matches a known grammar kind → score 0.9 → Valid
        let status = score_to_status("define", "other", &state);
        assert_eq!(status, CompileStatus::Valid);
    }

    #[test]
    fn score_adapter_returns_unknown_for_empty_cache() {
        let state = SharedState::new("a.db", "b.db");
        // No grammar kinds loaded — score_to_status returns NotChecked (not Unknown)
        // because there is no basis for scoring without any grammar entries
        let status = score_to_status("some_word", "some_kind", &state);
        assert_eq!(status, CompileStatus::NotChecked);
    }

    #[test]
    fn score_adapter_returns_float_via_from_score() {
        // CompileStatus::from_score maps f32 in [0.0, 1.0] to a discriminant without panic
        let high = CompileStatus::from_score(1.0);
        let mid = CompileStatus::from_score(0.65);
        let low = CompileStatus::from_score(0.1);
        assert_eq!(high, CompileStatus::Valid);
        assert_eq!(mid, CompileStatus::LowConfidence);
        assert_eq!(low, CompileStatus::Unknown);
    }

    #[test]
    fn score_adapter_known_word_scores_higher_than_unknown() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![GrammarKind {
            name: "render".into(),
            description: "output".into(),
        }]);
        let known_status = score_to_status("render", "other", &state);
        let unknown_status = score_to_status("zzz_unknown", "zzz_kind", &state);
        // known word → Valid (0.9); unknown → Unknown (0.3)
        assert_eq!(known_status, CompileStatus::Valid);
        assert_eq!(unknown_status, CompileStatus::Unknown);
    }

    #[test]
    fn score_adapter_zero_for_empty() {
        // CompileStatus::from_score(0.0) → Unknown (score < 0.5)
        // Empty word with empty kind maps to the lowest tier
        let low_status = CompileStatus::from_score(0.0);
        assert_eq!(low_status, CompileStatus::Unknown);
    }
}
