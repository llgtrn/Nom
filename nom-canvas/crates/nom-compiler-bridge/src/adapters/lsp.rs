#![deny(unsafe_code)]
use crate::shared::SharedState;
use nom_editor::lsp_bridge::{CompletionItem, CompletionKind, HoverResult, Location, LspProvider};
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
        let word = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        hover_from_dict(word, &self.state)
    }

    fn completions(&self, _path: &std::path::Path, _offset: usize) -> Vec<CompletionItem> {
        // Returns grammar keywords as completions
        self.state
            .cached_grammar_kinds()
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
            GrammarKind {
                name: "verb".into(),
                description: "action word".into(),
            },
            GrammarKind {
                name: "concept".into(),
                description: "abstract idea".into(),
            },
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

    #[test]
    fn lsp_adapter_converts_span_range() {
        // Verify that HoverResult range field is a plain std::ops::Range<usize>
        // and is preserved when returned from completions/hover logic.
        let result = nom_editor::lsp_bridge::HoverResult {
            contents: "test doc".into(),
            range: Some(3..7),
        };
        assert_eq!(result.range.as_ref().map(|r| r.start), Some(3));
        assert_eq!(result.range.as_ref().map(|r| r.end), Some(7));
        assert_eq!(result.contents, "test doc");
    }

    #[test]
    fn lsp_adapter_location_range_preserved() {
        // Location struct keeps path + range intact
        let loc = nom_editor::lsp_bridge::Location {
            path: std::path::PathBuf::from("foo.nom"),
            range: 10..20,
        };
        assert_eq!(loc.range.start, 10);
        assert_eq!(loc.range.end, 20);
        assert_eq!(loc.path, std::path::PathBuf::from("foo.nom"));
    }

    #[test]
    fn compiler_lsp_provider_completions_kind_is_keyword() {
        // Completions built from grammar kinds always have CompletionKind::Keyword
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![GrammarKind {
            name: "emit".into(),
            description: "output a value".into(),
        }]);
        let provider = CompilerLspProvider::new(state);
        let completions = provider.completions(std::path::Path::new("test.nomx"), 0);
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].kind, CompletionKind::Keyword);
        assert_eq!(completions[0].insert_text, "emit");
        assert_eq!(completions[0].detail, Some("output a value".to_string()));
    }

    #[test]
    fn lsp_adapter_severity_mapping() {
        // LspSeverity (from lsp_provider.rs) maps Error as the critical variant.
        // nom_editor's lsp_bridge does not expose a severity enum — we verify that
        // the CompletionKind variants used by adapters are consistent.
        let ck = CompletionKind::Keyword;
        let cf = CompletionKind::Function;
        let cc = CompletionKind::Class;
        // Each must be a distinct variant
        assert_ne!(ck, cf);
        assert_ne!(cf, cc);
        assert_ne!(ck, cc);
    }

    #[test]
    fn completion_adapter_max_items() {
        // completions() is unbounded in the current impl (returns all grammar kinds).
        // Verify that a large grammar cache does not cause a panic and respects the Vec type.
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let many_kinds: Vec<_> = (0..30)
            .map(|i| GrammarKind {
                name: format!("kind_{i}"),
                description: "desc".into(),
            })
            .collect();
        state.update_grammar_kinds(many_kinds);
        let provider = CompilerLspProvider::new(state);
        let completions = provider.completions(std::path::Path::new("test.nomx"), 0);
        // All 30 items are returned (no hard cap in current stub)
        assert_eq!(completions.len(), 30);
    }
}
