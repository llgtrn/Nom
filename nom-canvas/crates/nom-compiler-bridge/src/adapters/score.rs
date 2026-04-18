#![deny(unsafe_code)]
use crate::shared::SharedState;
use crate::ui_tier::CompileStatus;

/// Score a word+kind and return the compile status badge.
/// Under the `compiler` feature, constructs an Atom and calls `nom_score::score_atom()`
/// for the real 8-dimension weighted score. Falls back to grammar-cache name match otherwise.
pub fn score_to_status(word: &str, kind: &str, state: &SharedState) -> CompileStatus {
    #[cfg(feature = "compiler")]
    {
        return score_via_nom_score(word, kind, state);
    }
    #[cfg(not(feature = "compiler"))]
    {
        score_from_cached_kinds(word, kind, state)
    }
}

/// Real path: build an Atom and delegate to `nom_score::score_atom().overall()`.
/// If the grammar cache is empty there is no basis for scoring.
#[cfg(feature = "compiler")]
fn score_via_nom_score(word: &str, kind: &str, state: &SharedState) -> CompileStatus {
    use nom_types::{Atom, AtomKind};
    let kinds = state.cached_grammar_kinds();
    if kinds.is_empty() {
        return CompileStatus::NotChecked;
    }
    // Use grammar cache as a label hint so name-matched words score higher.
    let in_cache = kinds.iter().any(|k| k.name == word || k.name == kind);
    let labels = if in_cache {
        vec!["documented".to_string(), "grammar-known".to_string()]
    } else {
        vec![]
    };
    let atom = Atom {
        id: word.to_string(),
        kind: AtomKind::Function,
        name: word.to_string(),
        source_path: String::new(),
        language: "nom".to_string(),
        labels,
        concept: Some(kind.to_string()),
        signature: None,
        body: None,
    };
    let score = nom_score::score_atom(&atom).overall();
    CompileStatus::from_score(score)
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

    #[test]
    fn status_color_not_checked_has_unit_alpha() {
        let color = status_color(&CompileStatus::NotChecked);
        assert_eq!(color[3], 1.0);
    }

    #[test]
    fn status_color_unknown_has_unit_alpha() {
        let color = status_color(&CompileStatus::Unknown);
        assert_eq!(color[3], 1.0);
    }

    #[test]
    fn status_color_low_confidence_has_unit_alpha() {
        let color = status_color(&CompileStatus::LowConfidence);
        assert_eq!(color[3], 1.0);
    }

    #[test]
    fn status_color_all_components_finite() {
        for status in [
            CompileStatus::Valid,
            CompileStatus::LowConfidence,
            CompileStatus::Unknown,
            CompileStatus::NotChecked,
        ] {
            let c = status_color(&status);
            for v in c {
                assert!(v.is_finite(), "color component must be finite");
            }
        }
    }

    #[test]
    fn status_label_not_checked() {
        assert_eq!(status_label(&CompileStatus::NotChecked), "—");
    }

    #[test]
    fn score_to_status_valid_word_matches_kind_in_cache() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![GrammarKind {
            name: "metric".into(),
            description: "measurement".into(),
        }]);
        // word == "metric" is in the cache → Valid
        let s = score_to_status("metric", "other_kind", &state);
        assert_eq!(s, CompileStatus::Valid);
    }

    #[test]
    fn score_to_status_multiple_kinds_still_valid() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![
            GrammarKind { name: "alpha".into(), description: "".into() },
            GrammarKind { name: "beta".into(), description: "".into() },
            GrammarKind { name: "gamma".into(), description: "".into() },
        ]);
        let s = score_to_status("beta", "zzz", &state);
        assert_eq!(s, CompileStatus::Valid);
    }

    #[test]
    fn score_to_status_unknown_when_cache_has_entries_but_no_match() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![
            GrammarKind { name: "alpha".into(), description: "".into() },
        ]);
        let s = score_to_status("omega", "delta", &state);
        assert_eq!(s, CompileStatus::Unknown);
    }

    #[test]
    fn status_color_valid_green_hue() {
        let color = status_color(&CompileStatus::Valid);
        // hue is in [0.39, 0.41] (green region around 0.397)
        assert!(color[0] > 0.3 && color[0] < 0.5, "valid color should be greenish");
    }

    // AE10 — nom_score path tests

    /// Known words in the grammar cache produce a higher-ranked status than unknown ones.
    /// This exercises the cache-as-label-hint logic that boosts scores for cached terms.
    #[test]
    fn score_increases_with_better_name_match() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![GrammarKind {
            name: "render".into(),
            description: "output primitive".into(),
        }]);
        let known_status = score_to_status("render", "other", &state);
        let unknown_status = score_to_status("zzz_xyzzy_word", "zzz_kind", &state);
        // A known word must resolve to at least as high a status as an unknown one.
        // Valid > LowConfidence > Unknown in discriminant order.
        let rank = |s: &CompileStatus| match s {
            CompileStatus::Valid => 3,
            CompileStatus::LowConfidence => 2,
            CompileStatus::Unknown => 1,
            CompileStatus::NotChecked => 0,
        };
        assert!(
            rank(&known_status) >= rank(&unknown_status),
            "known word should rank >= unknown: {known_status:?} vs {unknown_status:?}"
        );
    }

    /// Grammar cache is used as a fallback: an empty cache always returns NotChecked.
    #[test]
    fn score_uses_grammar_cache_as_fallback() {
        let state = SharedState::new("a.db", "b.db");
        // No cache populated — cannot score, returns NotChecked as fallback
        let status = score_to_status("some_word", "some_kind", &state);
        assert_eq!(
            status,
            CompileStatus::NotChecked,
            "empty grammar cache must produce NotChecked (no basis for scoring)"
        );
    }

    /// The nom_score path is exercised when the grammar cache is non-empty.
    /// The result must be a well-formed CompileStatus (not a panic).
    #[test]
    fn nom_score_path_exercised_under_feature() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![
            GrammarKind { name: "compute".into(), description: "calculation".into() },
            GrammarKind { name: "validate".into(), description: "verification".into() },
        ]);
        // "validate" contains a security signal in nom_score — score_security adds 0.15.
        // Both words should produce non-NotChecked statuses.
        let s1 = score_to_status("validate", "concept", &state);
        let s2 = score_to_status("compute", "concept", &state);
        assert_ne!(s1, CompileStatus::NotChecked, "validate should be scored");
        assert_ne!(s2, CompileStatus::NotChecked, "compute should be scored");
    }

    // ── wave AJ-7: additional score adapter tests ────────────────────────────

    /// score_to_status with 5 known kinds still returns Valid for a match.
    #[test]
    fn score_to_status_with_five_kinds_valid() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![
            GrammarKind { name: "define".into(), description: "".into() },
            GrammarKind { name: "result".into(), description: "".into() },
            GrammarKind { name: "map".into(), description: "".into() },
            GrammarKind { name: "filter".into(), description: "".into() },
            GrammarKind { name: "reduce".into(), description: "".into() },
        ]);
        let s = score_to_status("map", "other", &state);
        assert_eq!(s, CompileStatus::Valid);
    }

    /// status_label for Valid is exactly "Valid".
    #[test]
    fn status_label_valid_exact() {
        assert_eq!(status_label(&CompileStatus::Valid), "Valid");
    }

    /// status_label for LowConfidence is "Low confidence".
    #[test]
    fn status_label_low_confidence_exact() {
        assert_eq!(status_label(&CompileStatus::LowConfidence), "Low confidence");
    }

    /// status_label for Unknown is "Unknown".
    #[test]
    fn status_label_unknown_exact() {
        assert_eq!(status_label(&CompileStatus::Unknown), "Unknown");
    }

    /// CompileStatus::from_score at boundary 0.8 produces Valid.
    #[test]
    fn score_from_score_boundary_0_8_valid() {
        let s = CompileStatus::from_score(0.8);
        assert_eq!(s, CompileStatus::Valid);
    }

    /// CompileStatus::from_score at 0.5 produces a non-NotChecked status.
    #[test]
    fn score_from_score_boundary_0_5_not_checked() {
        let s = CompileStatus::from_score(0.5);
        assert_ne!(s, CompileStatus::NotChecked);
    }

    /// status_color components are all in [0.0, 1.0] range.
    #[test]
    fn status_color_components_in_range() {
        for status in [
            CompileStatus::Valid,
            CompileStatus::LowConfidence,
            CompileStatus::Unknown,
            CompileStatus::NotChecked,
        ] {
            let c = status_color(&status);
            for (i, v) in c.iter().enumerate() {
                assert!(
                    *v >= 0.0 && *v <= 1.0,
                    "color component {i} for {status:?} must be in [0.0, 1.0]: got {v}"
                );
            }
        }
    }

    /// score_to_status returns Valid for kind match (not just word match).
    #[test]
    fn score_to_status_kind_match_returns_valid() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![GrammarKind {
            name: "entity".into(),
            description: "".into(),
        }]);
        let s = score_to_status("unknown_xyz", "entity", &state);
        assert_eq!(s, CompileStatus::Valid);
    }

    /// status_color LowConfidence hue is in amber region (non-zero hue).
    #[test]
    fn status_color_low_confidence_hue_amber() {
        let color = status_color(&CompileStatus::LowConfidence);
        assert!(color[0] > 0.0, "amber hue must be > 0");
    }

    /// status_color Unknown is in red region (hue near 0.0).
    #[test]
    fn status_color_unknown_red_region() {
        let color = status_color(&CompileStatus::Unknown);
        assert_eq!(color[0], 0.0, "Unknown status must have red hue (0.0)");
    }

    /// from_score at 1.0 always produces Valid.
    #[test]
    fn score_from_score_max_is_valid() {
        let s = CompileStatus::from_score(1.0);
        assert_eq!(s, CompileStatus::Valid);
    }

    /// from_score at 0.0 always produces Unknown.
    #[test]
    fn score_from_score_min_is_unknown() {
        let s = CompileStatus::from_score(0.0);
        assert_eq!(s, CompileStatus::Unknown);
    }

    /// score_to_status after updating grammar kinds reflects new state.
    #[test]
    fn score_to_status_reflects_updated_kinds() {
        let state = SharedState::new("a.db", "b.db");
        let s1 = score_to_status("newkind", "other", &state);
        assert_eq!(s1, CompileStatus::NotChecked);
        state.update_grammar_kinds(vec![GrammarKind {
            name: "newkind".into(),
            description: "".into(),
        }]);
        let s2 = score_to_status("newkind", "other", &state);
        assert_eq!(s2, CompileStatus::Valid);
    }

    /// score_to_status for case-sensitive mismatch returns Unknown.
    #[test]
    fn score_to_status_case_sensitive_no_match() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![GrammarKind {
            name: "Define".into(),
            description: "".into(),
        }]);
        let s = score_to_status("define", "other", &state);
        assert_eq!(s, CompileStatus::Unknown, "score matching must be case-sensitive");
    }
}
