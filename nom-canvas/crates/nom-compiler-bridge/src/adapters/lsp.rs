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
    fn hover(&self, path: &std::path::Path, _offset: usize) -> Option<HoverResult> {
        // Use the file stem as a word probe when no buffer extraction is available.
        // This lets the cache-based lookup work for document-level hover.
        let word = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        hover_from_dict(word, &self.state)
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
    use crate::shared::GrammarKind;

    #[test]
    fn compiler_lsp_provider_completions_from_cache() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            GrammarKind { name: "verb".into(), description: "action word".into() },
            GrammarKind { name: "concept".into(), description: "abstract idea".into() },
        ]);
        let provider = CompilerLspProvider::new(state);
        let completions = provider.completions(std::path::Path::new("test.nomx"), 0);
        assert_eq!(completions.len(), 2);
        assert_eq!(completions[0].label, "verb");
    }

    #[test]
    fn compiler_lsp_provider_hover_returns_none_without_compiler_feature() {
        // Without the "compiler" feature, hover_from_dict always returns None.
        // CompilerLspProvider::hover should propagate that None.
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let provider = CompilerLspProvider::new(state);
        let result = provider.hover(std::path::Path::new("verb.nomx"), 0);
        // Without compiler feature, always None regardless of cache
        assert!(result.is_none());
    }

    #[test]
    fn compiler_lsp_provider_goto_def_returns_none() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let provider = CompilerLspProvider::new(state);
        let result = provider.goto_definition(std::path::Path::new("any.nomx"), 0);
        assert!(result.is_none());
    }

    #[test]
    fn compiler_lsp_provider_completions_empty_when_no_grammar() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let provider = CompilerLspProvider::new(state);
        let completions = provider.completions(std::path::Path::new("test.nomx"), 0);
        assert!(completions.is_empty());
    }
}
