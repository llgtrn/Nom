#![deny(unsafe_code)]
use nom_editor::lsp_bridge::{CompletionItem, CompletionKind};
use crate::shared::SharedState;

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
    let Ok(dict) = NomDict::open_in_place(Path::new(&state.dict_path)) else {
        return vec![];
    };
    let Ok(entries) = dict.find_entities_by_word(prefix) else {
        return vec![];
    };
    entries.into_iter()
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
        .collect()
}

// Without compiler feature: use cached grammar kinds as completions
#[cfg(not(feature = "compiler"))]
pub fn complete_from_dict(
    prefix: &str,
    kind_filter: Option<&str>,
    state: &SharedState,
) -> Vec<CompletionItem> {
    state.cached_grammar_kinds()
        .into_iter()
        .filter(|k| {
            k.name.starts_with(prefix) &&
            kind_filter.map_or(true, |f| k.name.contains(f))
        })
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
            crate::shared::GrammarKind { name: "verb".into(), description: "action".into() },
            crate::shared::GrammarKind { name: "concept".into(), description: "idea".into() },
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
}
