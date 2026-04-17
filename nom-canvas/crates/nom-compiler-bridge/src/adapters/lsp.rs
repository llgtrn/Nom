#![deny(unsafe_code)]
use nom_editor::lsp_bridge::{CompletionItem, CompletionKind, HoverResult, Location, LspProvider};
use crate::shared::SharedState;
use std::sync::Arc;

// With compiler feature: real LSP using nom-lsp
#[cfg(feature = "compiler")]
pub fn hover_from_dict(word: &str, state: &SharedState) -> Option<HoverResult> {
    // Check cached grammar kinds for description
    let kinds = state.cached_grammar_kinds();
    if let Some(kind) = kinds.iter().find(|k| k.name == word) {
        return Some(HoverResult {
            contents: format!("**{}** — {}", kind.name, kind.description),
            range: None,
        });
    }
    None
}

// Without compiler feature: stub
#[cfg(not(feature = "compiler"))]
pub fn hover_from_dict(_word: &str, _state: &SharedState) -> Option<HoverResult> {
    None
}

/// CompilerLspProvider — bridges nom-lsp to nom-editor's LspProvider trait
pub struct CompilerLspProvider {
    state: Arc<SharedState>,
}

impl CompilerLspProvider {
    pub fn new(state: Arc<SharedState>) -> Self {
        Self { state }
    }
}

impl LspProvider for CompilerLspProvider {
    fn hover(&self, _path: &std::path::Path, _offset: usize) -> Option<HoverResult> {
        // Real impl: extract word at offset from buffer, then hover_from_dict
        // Wave C stub: returns None until buffer word extraction is wired
        None
    }

    fn completions(&self, _path: &std::path::Path, _offset: usize) -> Vec<CompletionItem> {
        // Returns grammar keywords as completions
        self.state.cached_grammar_kinds()
            .into_iter()
            .map(|k| CompletionItem {
                label: k.name.clone(),
                kind: CompletionKind::Keyword,
                detail: Some(k.description),
                insert_text: k.name,
                sort_text: None,
            })
            .collect()
    }

    fn goto_definition(&self, _path: &std::path::Path, _offset: usize) -> Option<Location> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiler_lsp_provider_completions_from_cache() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            crate::shared::GrammarKind { name: "verb".into(), description: "action word".into() },
            crate::shared::GrammarKind { name: "concept".into(), description: "abstract idea".into() },
        ]);
        let provider = CompilerLspProvider::new(state);
        let completions = provider.completions(std::path::Path::new("test.nomx"), 0);
        assert_eq!(completions.len(), 2);
        assert_eq!(completions[0].label, "verb");
    }
}
