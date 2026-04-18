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
        .filter(|e| kind_filter.map_or(true, |f| e.kind == f))
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
        .filter(|k| k.name.starts_with(prefix) && kind_filter.map_or(true, |f| k.name.contains(f)))
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
            },
            crate::shared::GrammarKind {
                name: "concept".into(),
                description: "idea".into(),
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
            },
            crate::shared::GrammarKind {
                name: "beta".into(),
                description: "second".into(),
            },
            crate::shared::GrammarKind {
                name: "aleph".into(),
                description: "letter".into(),
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
            },
            crate::shared::GrammarKind {
                name: "y".into(),
                description: "".into(),
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
            })
            .collect();
        state.update_grammar_kinds(kinds);
        let items = complete_from_dict("aa", None, &state);
        assert!(items.len() <= 20, "complete_from_dict must cap at 20 items");
    }
}
