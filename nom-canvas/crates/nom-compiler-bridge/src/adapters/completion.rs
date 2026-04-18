#![deny(unsafe_code)]
use crate::shared::SharedState;
use nom_editor::lsp_bridge::{CompletionItem, CompletionKind};

/// CompletionKind mapping from grammar kind string
#[cfg_attr(not(feature = "compiler"), allow(dead_code))]
fn kind_to_completion_kind(kind: &str) -> CompletionKind {
    match kind {
        "verb" => CompletionKind::Function,
        "concept" => CompletionKind::Class,
        "metric" => CompletionKind::Value,
        "attribute" => CompletionKind::Field,
        "constraint" => CompletionKind::Snippet,
        _ => CompletionKind::Keyword,
    }
}

// With compiler feature: real completion from nom-dict word search
#[cfg(feature = "compiler")]
pub fn complete_from_dict(
    prefix: &str,
    kind_filter: Option<&str>,
    state: &SharedState,
) -> Vec<CompletionItem> {
    use nom_dict::NomDict;
    use std::path::Path;
    let cached = complete_from_cached_kinds(prefix, kind_filter, state);
    let Ok(dict) = NomDict::open_in_place(Path::new(&state.dict_path)) else {
        return cached;
    };
    let Ok(entries) = dict.find_entities_by_word(prefix) else {
        return cached;
    };
    let items: Vec<_> = entries
        .into_iter()
        .filter(|e| kind_filter.is_none_or(|f| e.kind == f))
        .take(20)
        .map(|entry| {
            let ck = kind_to_completion_kind(&entry.kind);
            CompletionItem {
                label: entry.word.clone(),
                kind: ck,
                detail: Some(format!("[{}]", entry.kind)),
                insert_text: entry.word,
                sort_text: None,
            }
        })
        .collect();
    if items.is_empty() {
        cached
    } else {
        items
    }
}

// Without compiler feature: use cached grammar kinds as completions
#[cfg(not(feature = "compiler"))]
pub fn complete_from_dict(
    prefix: &str,
    kind_filter: Option<&str>,
    state: &SharedState,
) -> Vec<CompletionItem> {
    complete_from_cached_kinds(prefix, kind_filter, state)
}

fn complete_from_cached_kinds(
    prefix: &str,
    kind_filter: Option<&str>,
    state: &SharedState,
) -> Vec<CompletionItem> {
    state
        .cached_grammar_kinds()
        .into_iter()
        .filter(|k| k.name.starts_with(prefix) && kind_filter.is_none_or(|f| k.name.contains(f)))
        .take(20)
        .map(|k| CompletionItem {
            label: k.name.clone(),
            kind: CompletionKind::Keyword,
            detail: Some(k.description),
            insert_text: k.name,
            sort_text: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_from_dict_stub_prefix_filter() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "verb".into(),
                description: "action".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "concept".into(),
                description: "idea".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let items = complete_from_dict("ve", None, &state);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "verb");
    }

    #[test]
    fn kind_mapping() {
        assert_eq!(kind_to_completion_kind("verb"), CompletionKind::Function);
        assert_eq!(kind_to_completion_kind("concept"), CompletionKind::Class);
        assert_eq!(kind_to_completion_kind("metric"), CompletionKind::Value);
    }

    #[test]
    fn completion_adapter_prefix_filter() {
        // Only items whose name starts with the prefix are returned
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "alpha".into(),
                description: "first".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "beta".into(),
                description: "second".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "aleph".into(),
                description: "letter".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let items = complete_from_dict("al", None, &state);
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|i| i.label.starts_with("al")));
    }

    #[test]
    fn completion_adapter_returns_items_for_non_empty_cache() {
        // Non-empty grammar cache produces non-empty items when prefix matches
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "flow".into(),
            description: "movement".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let items = complete_from_dict("", None, &state);
        assert!(!items.is_empty());
    }

    #[test]
    fn completion_adapter_empty_prefix_returns_all() {
        let state = SharedState::new("a.db", "b.db");
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
        let items = complete_from_dict("", None, &state);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn completion_adapter_no_match_returns_empty() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "zeta".into(),
            description: "".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let items = complete_from_dict("abc", None, &state);
        assert!(items.is_empty());
    }

    #[test]
    fn completion_adapter_max_items_respects_take_limit() {
        // complete_from_dict uses .take(20) — loading 25 matching entries yields at most 20
        let state = SharedState::new("a.db", "b.db");
        let kinds: Vec<_> = (0..25)
            .map(|i| crate::shared::GrammarKind {
                name: format!("aa_kind_{i:02}"),
                description: "test".into(),
                status: crate::shared::KindStatus::Transient,
            })
            .collect();
        state.update_grammar_kinds(kinds);
        let items = complete_from_dict("aa", None, &state);
        assert!(items.len() <= 20, "complete_from_dict must cap at 20 items");
    }

    #[test]
    fn kind_mapping_attribute_and_constraint() {
        assert_eq!(kind_to_completion_kind("attribute"), CompletionKind::Field);
        assert_eq!(
            kind_to_completion_kind("constraint"),
            CompletionKind::Snippet
        );
    }

    #[test]
    fn kind_mapping_unknown_falls_to_keyword() {
        assert_eq!(
            kind_to_completion_kind("unknown_kind"),
            CompletionKind::Keyword
        );
        assert_eq!(kind_to_completion_kind(""), CompletionKind::Keyword);
    }

    #[test]
    fn completion_insert_text_matches_label() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "stream".into(),
            description: "data flow".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let items = complete_from_dict("st", None, &state);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, items[0].insert_text);
    }

    #[test]
    fn completion_detail_contains_description() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "render".into(),
            description: "output action".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let items = complete_from_dict("ren", None, &state);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].detail.as_deref(), Some("output action"));
    }

    #[test]
    fn completion_sort_text_is_none_in_stub() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "emit".into(),
            description: "send".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let items = complete_from_dict("em", None, &state);
        assert!(!items.is_empty());
        assert!(items[0].sort_text.is_none());
    }

    #[test]
    fn completion_kind_filter_no_match_returns_empty() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "verb".into(),
            description: "action".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        // kind_filter "zz" doesn't appear in name "verb" → empty
        let items = complete_from_dict("", Some("zz"), &state);
        assert!(items.is_empty());
    }

    #[test]
    fn completion_kind_filter_match_returns_items() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "verb_run".into(),
                description: "run action".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "noun_entity".into(),
                description: "entity".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        // kind_filter "verb" matches name "verb_run" (contains check)
        let items = complete_from_dict("", Some("verb"), &state);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "verb_run");
    }

    #[test]
    fn completion_empty_cache_returns_empty() {
        let state = SharedState::new("a.db", "b.db");
        let items = complete_from_dict("anything", None, &state);
        assert!(items.is_empty());
    }

    #[test]
    fn completion_unicode_prefix_matches() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "définir".into(),
            description: "declare".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let items = complete_from_dict("dé", None, &state);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "définir");
    }

    // ── AF4 additions ──────────────────────────────────────────────────────

    /// Completion with exactly 10 candidates returns all 10 items.
    #[test]
    fn completion_ten_candidates_returns_all_ten() {
        let state = SharedState::new("a.db", "b.db");
        let kinds: Vec<_> = (0..10)
            .map(|i| crate::shared::GrammarKind {
                name: format!("item_{i:02}"),
                description: format!("desc {i}"),
                status: crate::shared::KindStatus::Transient,
            })
            .collect();
        state.update_grammar_kinds(kinds);
        let items = complete_from_dict("item", None, &state);
        assert_eq!(items.len(), 10, "all 10 matching items must be returned");
    }

    /// Completion label equals kind name for each returned item.
    #[test]
    fn completion_label_equals_kind_name() {
        let state = SharedState::new("a.db", "b.db");
        let names = vec!["run", "jump", "fly"];
        state.update_grammar_kinds(
            names
                .iter()
                .map(|n| crate::shared::GrammarKind {
                    name: n.to_string(),
                    description: "action".into(),
                    status: crate::shared::KindStatus::Transient,
                })
                .collect(),
        );
        let items = complete_from_dict("", None, &state);
        assert_eq!(items.len(), 3);
        for item in &items {
            assert!(
                names.contains(&item.label.as_str()),
                "label '{}' must match a kind name",
                item.label
            );
            // label and insert_text must be equal
            assert_eq!(item.label, item.insert_text);
        }
    }

    /// Completion kind for cached kinds is always Keyword.
    #[test]
    fn completion_kind_is_keyword_for_cached_kinds() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "resolve".into(),
                description: "lookup".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "compose".into(),
                description: "combine".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let items = complete_from_dict("", None, &state);
        for item in &items {
            assert_eq!(
                item.kind,
                CompletionKind::Keyword,
                "cached-kind completions must have Keyword kind"
            );
        }
    }

    /// Complete from dict with kind_filter matching a substring selects correctly.
    #[test]
    fn completion_kind_filter_substring_match() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "verb_run".into(),
                description: "run".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "verb_jump".into(),
                description: "jump".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "noun_thing".into(),
                description: "thing".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let items = complete_from_dict("", Some("verb"), &state);
        assert_eq!(items.len(), 2, "only verb_* kinds should match");
        for item in &items {
            assert!(item.label.contains("verb"));
        }
    }

    /// Completions with prefix filter returns subset of matching items.
    #[test]
    fn completion_prefix_filter_subset() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "stream".into(),
                description: "flow".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "string".into(),
                description: "text".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "select".into(),
                description: "choose".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let items = complete_from_dict("str", None, &state);
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|i| i.label.starts_with("str")));
    }

    /// Complete from dict returns items with non-empty label and insert_text.
    #[test]
    fn completion_non_empty_label_and_insert_text() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "produce".into(),
            description: "generate output".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let items = complete_from_dict("p", None, &state);
        assert!(!items.is_empty());
        for item in &items {
            assert!(!item.label.is_empty());
            assert!(!item.insert_text.is_empty());
        }
    }

    // ── AB-wave additions ──────────────────────────────────────────────────

    /// Completion with empty source (empty prefix, empty cache) returns empty list.
    #[test]
    fn completion_empty_source_returns_empty() {
        let state = SharedState::new("a.db", "b.db");
        // no grammar kinds loaded — empty cache
        let items = complete_from_dict("", None, &state);
        assert!(
            items.is_empty(),
            "empty source with empty cache must return empty list"
        );
    }

    /// Completion with a valid prefix (non-empty cache, prefix matches) returns non-empty list.
    #[test]
    fn completion_valid_prefix_returns_non_empty() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "transform".into(),
            description: "map data".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let items = complete_from_dict("tr", None, &state);
        assert!(
            !items.is_empty(),
            "valid prefix 'tr' must return at least one completion"
        );
    }

    /// All completion result items have non-empty label fields.
    #[test]
    fn completion_results_have_non_empty_labels() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "action".into(),
                description: "do".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "aspect".into(),
                description: "view".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let items = complete_from_dict("a", None, &state);
        assert!(!items.is_empty());
        for item in &items {
            assert!(
                !item.label.is_empty(),
                "every completion must have non-empty label"
            );
        }
    }

    /// Completion kind field is CompletionKind::Keyword for cached completions.
    #[test]
    fn completion_kind_field_is_keyword() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "validate".into(),
            description: "check constraint".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let items = complete_from_dict("v", None, &state);
        assert!(!items.is_empty());
        assert_eq!(
            items[0].kind,
            CompletionKind::Keyword,
            "cached completions must have Keyword kind"
        );
    }

    /// Max results limit: 25 matching entries returns at most 20.
    #[test]
    fn completion_max_results_limit_capped_at_20() {
        let state = SharedState::new("a.db", "b.db");
        let kinds: Vec<_> = (0..25)
            .map(|i| crate::shared::GrammarKind {
                name: format!("zz_item_{i:02}"),
                description: "test".into(),
                status: crate::shared::KindStatus::Transient,
            })
            .collect();
        state.update_grammar_kinds(kinds);
        let items = complete_from_dict("zz", None, &state);
        assert!(
            items.len() <= 20,
            "complete_from_dict must return at most 20 items, got {}",
            items.len()
        );
    }

    /// Completion at EOF position (prefix is the full kind name) still returns that kind.
    #[test]
    fn completion_at_eof_position_handled_gracefully() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "finalize".into(),
            description: "end the pipeline".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        // "EOF" position = prefix equals the full word
        let items = complete_from_dict("finalize", None, &state);
        assert!(
            !items.is_empty(),
            "prefix matching full word must still return the item"
        );
        assert_eq!(items[0].label, "finalize");
    }

    /// Completion with single-character prefix returns all items starting with that char.
    #[test]
    fn completion_single_char_prefix_returns_matching() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "flow".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "filter".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "generate".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let items = complete_from_dict("f", None, &state);
        assert_eq!(
            items.len(),
            2,
            "single char 'f' must match 'flow' and 'filter'"
        );
        for item in &items {
            assert!(item.label.starts_with('f'));
        }
    }

    /// Completion kind_filter with None returns all matching items regardless of kind.
    #[test]
    fn completion_no_kind_filter_returns_all_matching() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "noun_x".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "noun_y".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "verb_x".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        // No kind_filter — all items with empty prefix are returned
        let items = complete_from_dict("", None, &state);
        assert_eq!(items.len(), 3, "no kind filter must return all 3 items");
    }

    /// Completion with prefix longer than any kind name returns empty.
    #[test]
    fn completion_prefix_longer_than_any_kind_returns_empty() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "run".into(),
            description: "execute".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        // "running" is longer than "run" — "run".starts_with("running") is false
        let items = complete_from_dict("running", None, &state);
        assert!(
            items.is_empty(),
            "prefix longer than any kind must return empty"
        );
    }

    /// Completion with multiple kinds that all share the same prefix returns all of them.
    #[test]
    fn completion_all_items_share_prefix_returns_all() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind {
                name: "stream".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "stride".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
            crate::shared::GrammarKind {
                name: "strip".into(),
                description: "".into(),
                status: crate::shared::KindStatus::Transient,
            },
        ]);
        let items = complete_from_dict("str", None, &state);
        assert_eq!(items.len(), 3, "all 3 'str*' items must be returned");
    }

    /// kind_to_completion_kind maps "metric" to Value.
    #[test]
    fn kind_mapping_metric_to_value() {
        assert_eq!(kind_to_completion_kind("metric"), CompletionKind::Value);
    }

    /// Completion items returned have detail field set (from description).
    #[test]
    fn completion_detail_is_set_from_description() {
        let state = SharedState::new("a.db", "b.db");
        state.update_grammar_kinds(vec![crate::shared::GrammarKind {
            name: "ingest".into(),
            description: "consume input".into(),
            status: crate::shared::KindStatus::Transient,
        }]);
        let items = complete_from_dict("in", None, &state);
        assert!(!items.is_empty());
        // detail is Some and contains the description text
        assert!(items[0].detail.is_some(), "detail must be Some");
        assert_eq!(items[0].detail.as_deref(), Some("consume input"));
    }
}
