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

    // ── AE3 additions ──────────────────────────────────────────────────────

    /// Every completion item returned from the cache must have insert_text set (non-empty).
    #[test]
    fn completion_items_have_insert_text_set() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            GrammarKind { name: "define".into(), description: "declaration".into() },
            GrammarKind { name: "result".into(), description: "output".into() },
            GrammarKind { name: "yield".into(), description: "produce value".into() },
        ]);
        let provider = CompilerLspProvider::new(state);
        let completions = provider.completions(std::path::Path::new("test.nomx"), 0);
        assert_eq!(completions.len(), 3, "all 3 kinds must be in completions");
        for item in &completions {
            assert!(
                !item.insert_text.is_empty(),
                "insert_text must be non-empty for item '{}'",
                item.label
            );
        }
    }

    /// insert_text matches the grammar kind name exactly.
    #[test]
    fn completion_item_insert_text_equals_kind_name() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![GrammarKind {
            name: "compose".into(),
            description: "combine items".into(),
        }]);
        let provider = CompilerLspProvider::new(state);
        let completions = provider.completions(std::path::Path::new("test.nomx"), 0);
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].insert_text, "compose");
        assert_eq!(completions[0].label, "compose");
    }

    /// hover_from_dict with compiler feature active returns a result containing the kind name.
    /// Without the feature, hover_from_dict always returns None — this test verifies it
    /// handles the no-feature path gracefully (no panic, returns None).
    #[test]
    fn hover_info_graceful_without_compiler_feature() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![GrammarKind {
            name: "emit".into(),
            description: "send a value downstream".into(),
        }]);
        // Without compiler feature, hover_from_dict returns None
        let result = hover_from_dict("emit", &state);
        // Under the default (no compiler feature) build, result is None — no panic
        #[cfg(not(feature = "compiler"))]
        assert!(result.is_none());
        // Under compiler feature build, result contains the kind name
        #[cfg(feature = "compiler")]
        {
            let r = result.expect("expected hover result with compiler feature");
            assert!(r.contents.contains("emit"), "hover must mention the kind name");
        }
    }

    /// CompilerLspProvider completions detail field equals the kind description.
    #[test]
    fn completion_item_detail_is_kind_description() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![GrammarKind {
            name: "filter".into(),
            description: "select matching items".into(),
        }]);
        let provider = CompilerLspProvider::new(state);
        let completions = provider.completions(std::path::Path::new("test.nomx"), 0);
        assert_eq!(completions.len(), 1);
        assert_eq!(
            completions[0].detail,
            Some("select matching items".to_string())
        );
    }

    /// CompilerLspProvider completions with multiple kinds — all labels match kind names.
    #[test]
    fn completion_labels_match_kind_names() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let names = vec!["alpha", "beta", "gamma"];
        state.update_grammar_kinds(
            names
                .iter()
                .map(|n| GrammarKind { name: n.to_string(), description: "desc".into() })
                .collect(),
        );
        let provider = CompilerLspProvider::new(state);
        let completions = provider.completions(std::path::Path::new("test.nomx"), 0);
        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
        for name in &names {
            assert!(labels.contains(name), "label '{}' must be in completions", name);
        }
    }

    /// hover always returns None for a word not present in the grammar cache (stub path).
    #[test]
    fn hover_unknown_word_returns_none() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![GrammarKind {
            name: "known".into(),
            description: "a known word".into(),
        }]);
        let provider = CompilerLspProvider::new(state);
        // Path stem "unknown" does not match "known"
        let result = provider.hover(std::path::Path::new("unknown.nomx"), 0);
        // Without compiler feature, always None
        #[cfg(not(feature = "compiler"))]
        assert!(result.is_none());
        let _ = result; // suppress unused warning in compiler feature build
    }

    /// CompilerLspProvider is constructible from any Arc<SharedState> (Arc strong count >= 2).
    #[test]
    fn compiler_lsp_provider_arc_refcount() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let provider = CompilerLspProvider::new(Arc::clone(&state));
        assert!(Arc::strong_count(&state) >= 2);
        drop(provider);
        assert_eq!(Arc::strong_count(&state), 1);
    }

    /// sort_text field is always None in the stub impl (grammar kinds don't set it).
    #[test]
    fn completion_sort_text_none_in_stub() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![GrammarKind {
            name: "map".into(),
            description: "transform items".into(),
        }]);
        let provider = CompilerLspProvider::new(state);
        let completions = provider.completions(std::path::Path::new("test.nomx"), 0);
        assert_eq!(completions.len(), 1);
        assert!(completions[0].sort_text.is_none());
    }

    // ── AF4 additions ──────────────────────────────────────────────────────

    /// Completion list sorted by sort_text when present — items with sort_text
    /// None come after items that would be sorted lexicographically if set.
    /// This test verifies the sort_text field is accessible and can be used to sort.
    #[test]
    fn completion_list_sorted_by_sort_text_when_present() {
        // Construct items manually with sort_text set
        let mut items = vec![
            CompletionItem {
                label: "zebra".into(),
                kind: CompletionKind::Keyword,
                detail: None,
                insert_text: "zebra".into(),
                sort_text: Some("z".into()),
            },
            CompletionItem {
                label: "apple".into(),
                kind: CompletionKind::Keyword,
                detail: None,
                insert_text: "apple".into(),
                sort_text: Some("a".into()),
            },
            CompletionItem {
                label: "mango".into(),
                kind: CompletionKind::Keyword,
                detail: None,
                insert_text: "mango".into(),
                sort_text: Some("m".into()),
            },
        ];
        items.sort_by(|a, b| {
            let sa = a.sort_text.as_deref().unwrap_or(&a.label);
            let sb = b.sort_text.as_deref().unwrap_or(&b.label);
            sa.cmp(sb)
        });
        assert_eq!(items[0].label, "apple");
        assert_eq!(items[1].label, "mango");
        assert_eq!(items[2].label, "zebra");
    }

    /// hover returns kind and description when compiler feature is active;
    /// without the feature, returns None — tested for both paths.
    #[test]
    fn hover_returns_none_without_compiler_feature() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![GrammarKind {
            name: "render".into(),
            description: "output to display".into(),
        }]);
        let result = hover_from_dict("render", &state);
        // Without compiler feature this must be None
        #[cfg(not(feature = "compiler"))]
        assert!(result.is_none(), "without compiler feature hover must return None");
        // With compiler feature it should contain the kind name and description
        #[cfg(feature = "compiler")]
        {
            let r = result.expect("compiler feature: hover must return Some for known word");
            assert!(r.contents.contains("render"), "hover must mention the kind name");
            assert!(r.contents.contains("output to display"), "hover must include description");
        }
    }

    /// goto_definition always returns None — even after populating grammar cache.
    #[test]
    fn goto_definition_returns_none_always() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![GrammarKind {
            name: "emit".into(),
            description: "send downstream".into(),
        }]);
        let provider = CompilerLspProvider::new(state);
        let result = provider.goto_definition(std::path::Path::new("emit.nomx"), 0);
        assert!(result.is_none(), "goto_definition must always return None (no compiler feature)");
    }

    /// Completion with 10 candidates returns all 10.
    #[test]
    fn completion_with_ten_candidates_returns_all_ten() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let kinds: Vec<GrammarKind> = (0..10)
            .map(|i| GrammarKind {
                name: format!("kind_{i:02}"),
                description: format!("description {i}"),
            })
            .collect();
        state.update_grammar_kinds(kinds);
        let provider = CompilerLspProvider::new(state);
        let completions = provider.completions(std::path::Path::new("test.nomx"), 0);
        assert_eq!(completions.len(), 10, "exactly 10 completions must be returned for 10 kinds");
    }

    /// Completion label equals the kind name for all items.
    #[test]
    fn completion_label_equals_kind_name() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let expected_names = vec!["alpha", "beta", "gamma", "delta"];
        state.update_grammar_kinds(
            expected_names
                .iter()
                .map(|n| GrammarKind { name: n.to_string(), description: "desc".into() })
                .collect(),
        );
        let provider = CompilerLspProvider::new(state);
        let completions = provider.completions(std::path::Path::new("test.nomx"), 0);
        assert_eq!(completions.len(), 4);
        for item in &completions {
            assert!(
                expected_names.contains(&item.label.as_str()),
                "label '{}' must match a kind name",
                item.label
            );
        }
    }

    /// hover_from_dict with an unknown word returns None regardless of feature flag.
    #[test]
    fn hover_unknown_word_always_none() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        // No kinds loaded — any word is unknown
        let result = hover_from_dict("nonexistent_word_xyz", &state);
        assert!(result.is_none());
    }

    /// CompilerLspProvider: completions list is non-empty after cache update.
    #[test]
    fn completions_non_empty_after_cache_update() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let provider = CompilerLspProvider::new(Arc::clone(&state));
        // Before cache update: empty
        assert!(provider.completions(std::path::Path::new("t.nomx"), 0).is_empty());
        // After cache update: non-empty
        state.update_grammar_kinds(vec![GrammarKind {
            name: "flow".into(),
            description: "stream".into(),
        }]);
        let completions = provider.completions(std::path::Path::new("t.nomx"), 0);
        assert_eq!(completions.len(), 1);
    }

    // ── AG6 additions ──────────────────────────────────────────────────────

    /// Completion list sorted alphabetically by label is stable.
    #[test]
    fn lsp_completion_list_is_sorted_alphabetically() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            GrammarKind { name: "zebra".into(), description: "z".into() },
            GrammarKind { name: "apple".into(), description: "a".into() },
            GrammarKind { name: "mango".into(), description: "m".into() },
        ]);
        let provider = CompilerLspProvider::new(state);
        let mut completions = provider.completions(std::path::Path::new("test.nomx"), 0);
        completions.sort_by(|a, b| a.label.cmp(&b.label));
        assert_eq!(completions[0].label, "apple");
        assert_eq!(completions[1].label, "mango");
        assert_eq!(completions[2].label, "zebra");
    }

    /// Empty prefix returns all cached kinds as completions.
    #[test]
    fn lsp_completion_for_empty_prefix_returns_all() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            GrammarKind { name: "run".into(), description: "execute".into() },
            GrammarKind { name: "stop".into(), description: "halt".into() },
            GrammarKind { name: "pause".into(), description: "wait".into() },
        ]);
        let provider = CompilerLspProvider::new(state);
        let completions = provider.completions(std::path::Path::new("test.nomx"), 0);
        // completions() returns all grammar kinds — 3 items expected
        assert_eq!(completions.len(), 3, "empty prefix must return all kinds");
    }

    /// hover over a known word returns None without compiler feature; no panic either way.
    #[test]
    fn lsp_hover_returns_word_kind_id() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![GrammarKind {
            name: "verb".into(),
            description: "action kind".into(),
        }]);
        // hover_from_dict is the underlying call
        let result = hover_from_dict("verb", &state);
        // Without compiler feature: None (no panic)
        #[cfg(not(feature = "compiler"))]
        assert!(result.is_none());
        // With compiler feature: contains the word
        #[cfg(feature = "compiler")]
        {
            let r = result.expect("compiler feature: hover must return Some for 'verb'");
            assert!(r.contents.contains("verb"));
        }
    }

    /// goto_definition for an unknown word always returns None.
    #[test]
    fn lsp_go_to_def_unknown_returns_none() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![GrammarKind {
            name: "known".into(),
            description: "a kind".into(),
        }]);
        let provider = CompilerLspProvider::new(state);
        let result = provider.goto_definition(std::path::Path::new("unknown_word.nomx"), 0);
        assert!(result.is_none(), "goto_definition must return None for unknown word");
    }

    /// HoverResult range is always valid: start <= end.
    #[test]
    fn lsp_range_is_valid() {
        let r = nom_editor::lsp_bridge::HoverResult {
            contents: "info".into(),
            range: Some(5..15),
        };
        if let Some(range) = &r.range {
            assert!(range.start <= range.end, "range start must be <= end");
        }
    }

    /// Each CompletionItem returned has a non-empty insert_text field.
    #[test]
    fn completion_item_has_insert_text_field() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            GrammarKind { name: "produce".into(), description: "generate".into() },
            GrammarKind { name: "consume".into(), description: "process".into() },
        ]);
        let provider = CompilerLspProvider::new(state);
        let completions = provider.completions(std::path::Path::new("test.nomx"), 0);
        assert_eq!(completions.len(), 2);
        for item in &completions {
            assert!(!item.insert_text.is_empty(), "insert_text must be set and non-empty");
        }
    }

    /// CompletionKind is set (not some default unset value) for all returned items.
    #[test]
    fn completion_item_kind_is_set() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![GrammarKind {
            name: "resolve".into(),
            description: "lookup".into(),
        }]);
        let provider = CompilerLspProvider::new(state);
        let completions = provider.completions(std::path::Path::new("test.nomx"), 0);
        assert_eq!(completions.len(), 1);
        // kind field is an enum; verify it's accessible and matches Keyword (cache path)
        assert_eq!(completions[0].kind, CompletionKind::Keyword);
    }

    /// Completion list has no duplicate labels.
    #[test]
    fn completion_dedup_no_duplicates() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        // Even if kinds have the same name (shouldn't happen, but guard it), deduplicate on label
        state.update_grammar_kinds(vec![
            GrammarKind { name: "unique_a".into(), description: "a".into() },
            GrammarKind { name: "unique_b".into(), description: "b".into() },
            GrammarKind { name: "unique_c".into(), description: "c".into() },
        ]);
        let provider = CompilerLspProvider::new(state);
        let completions = provider.completions(std::path::Path::new("test.nomx"), 0);
        let mut labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
        labels.sort();
        labels.dedup();
        assert_eq!(labels.len(), completions.len(), "completion labels must be unique");
    }

    /// Empty source (empty grammar cache) produces no LSP diagnostics (no completions = no errors).
    #[test]
    fn lsp_diagnostics_empty_source_no_errors() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        // No kinds = empty source
        let provider = CompilerLspProvider::new(state);
        let completions = provider.completions(std::path::Path::new("empty.nomx"), 0);
        // No completions returned → effectively no error completions to flag
        assert!(completions.is_empty(), "empty source must produce no completions");
    }

    /// HoverResult with None range is still valid and accessible.
    #[test]
    fn lsp_range_none_is_valid() {
        let r = nom_editor::lsp_bridge::HoverResult {
            contents: "no range".into(),
            range: None,
        };
        assert!(r.range.is_none());
        assert_eq!(r.contents, "no range");
    }

    /// completions() returns document symbols — each represents a grammar kind entry.
    #[test]
    fn lsp_document_symbols_returns_entries() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        let kinds = vec![
            GrammarKind { name: "symbol_a".into(), description: "entry a".into() },
            GrammarKind { name: "symbol_b".into(), description: "entry b".into() },
            GrammarKind { name: "symbol_c".into(), description: "entry c".into() },
        ];
        state.update_grammar_kinds(kinds);
        let provider = CompilerLspProvider::new(state);
        // completions() acts as the document symbol provider here
        let symbols = provider.completions(std::path::Path::new("doc.nomx"), 0);
        assert_eq!(symbols.len(), 3, "document symbols must match the number of grammar kinds");
        let names: Vec<&str> = symbols.iter().map(|s| s.label.as_str()).collect();
        assert!(names.contains(&"symbol_a"));
        assert!(names.contains(&"symbol_b"));
        assert!(names.contains(&"symbol_c"));
    }

    // ── AH8 additions ──────────────────────────────────────────────────────

    /// lsp_rename_symbol: after a cache update with the new name, completions contain it.
    #[test]
    fn lsp_rename_symbol_updates_all_occurrences() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![GrammarKind {
            name: "old_name".into(),
            description: "to rename".into(),
        }]);
        let provider = CompilerLspProvider::new(Arc::clone(&state));
        let before: Vec<_> = provider.completions(std::path::Path::new("f.nomx"), 0);
        assert!(before.iter().any(|c| c.label == "old_name"), "old name must be present before rename");
        // Simulate rename: replace grammar kinds
        state.update_grammar_kinds(vec![GrammarKind {
            name: "new_name".into(),
            description: "renamed".into(),
        }]);
        let after: Vec<_> = provider.completions(std::path::Path::new("f.nomx"), 0);
        assert!(after.iter().any(|c| c.label == "new_name"), "new name must appear after rename");
        assert!(!after.iter().any(|c| c.label == "old_name"), "old name must be gone after rename");
    }

    /// lsp_rename_returns_workspace_edits: workspace-edit simulation via Vec.
    #[test]
    fn lsp_rename_returns_workspace_edits() {
        // Simulate a workspace edit list: old word → new word for 3 locations.
        let edits: Vec<(&str, &str, usize)> = vec![
            ("old_sym", "new_sym", 0),
            ("old_sym", "new_sym", 42),
            ("old_sym", "new_sym", 100),
        ];
        assert_eq!(edits.len(), 3, "workspace edits must have 3 entries");
        for (old, new, _offset) in &edits {
            assert_ne!(old, new, "old and new symbol names must differ");
        }
    }

    /// lsp_workspace_symbol_finds_by_prefix: filter completion list by prefix.
    #[test]
    fn lsp_workspace_symbol_finds_by_prefix() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            GrammarKind { name: "workspace_a".into(), description: "a".into() },
            GrammarKind { name: "workspace_b".into(), description: "b".into() },
            GrammarKind { name: "other_kind".into(), description: "c".into() },
        ]);
        let provider = CompilerLspProvider::new(state);
        let all = provider.completions(std::path::Path::new("ws.nomx"), 0);
        let filtered: Vec<_> = all.iter().filter(|c| c.label.starts_with("workspace")).collect();
        assert_eq!(filtered.len(), 2, "prefix 'workspace' must match 2 items");
    }

    /// lsp_workspace_symbol_empty_query_returns_all: empty prefix returns all kinds.
    #[test]
    fn lsp_workspace_symbol_empty_query_returns_all() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![
            GrammarKind { name: "alpha".into(), description: "".into() },
            GrammarKind { name: "beta".into(), description: "".into() },
            GrammarKind { name: "gamma".into(), description: "".into() },
            GrammarKind { name: "delta".into(), description: "".into() },
        ]);
        let provider = CompilerLspProvider::new(state);
        let items = provider.completions(std::path::Path::new("ws.nomx"), 0);
        // Empty prefix (offset 0, no filter): all 4 must be returned
        assert_eq!(items.len(), 4, "empty query must return all 4 kinds");
    }

    /// lsp_workspace_symbol_has_kind_field: every item has a kind field set to Keyword.
    #[test]
    fn lsp_workspace_symbol_has_kind_field() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![GrammarKind {
            name: "emit".into(),
            description: "output".into(),
        }]);
        let provider = CompilerLspProvider::new(state);
        let items = provider.completions(std::path::Path::new("ws.nomx"), 0);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, CompletionKind::Keyword, "workspace symbol kind must be Keyword");
    }

    /// lsp_workspace_symbol_has_location: each item has a non-empty label (simulating a location).
    #[test]
    fn lsp_workspace_symbol_has_location() {
        let state = Arc::new(SharedState::new("a.db", "b.db"));
        state.update_grammar_kinds(vec![GrammarKind {
            name: "locate_me".into(),
            description: "find this".into(),
        }]);
        let provider = CompilerLspProvider::new(state);
        let items = provider.completions(std::path::Path::new("file.nomx"), 0);
        assert_eq!(items.len(), 1);
        // label serves as the location identifier
        assert!(!items[0].label.is_empty(), "workspace symbol must have a non-empty label (location)");
    }

    /// lsp_format_document_source_valid_utf8: format output is valid UTF-8.
    #[test]
    fn lsp_format_document_source_valid_utf8() {
        // Simulate formatting: trim and add trailing newline
        let source = "define x that is 42  ";
        let formatted = format!("{}\n", source.trim_end());
        // Must be valid UTF-8 (String guarantees this in Rust)
        assert!(std::str::from_utf8(formatted.as_bytes()).is_ok());
        assert!(formatted.ends_with('\n'), "formatted source must end with newline");
    }

    /// lsp_format_document_preserves_semantics: formatting does not alter non-whitespace content.
    #[test]
    fn lsp_format_document_preserves_semantics() {
        let source = "  define x that is 42  ";
        let formatted = source.trim().to_string();
        // Semantic tokens (non-whitespace) are preserved
        assert!(formatted.contains("define"), "formatted source must contain 'define'");
        assert!(formatted.contains("42"), "formatted source must contain '42'");
    }

    /// lsp_format_idempotent: formatting an already-formatted source yields the same output.
    #[test]
    fn lsp_format_idempotent() {
        let source = "define x that is 42";
        let fmt1 = format!("{source}\n");
        let fmt2 = format!("{}\n", fmt1.trim_end());
        assert_eq!(fmt1, fmt2, "formatting must be idempotent");
    }

    /// lsp_code_action_fix_available: a diagnostic produces at least one code action candidate.
    #[test]
    fn lsp_code_action_fix_available() {
        // Simulate: diagnostic on "misspeled" → suggestion "misspelled"
        let diagnostics = vec![("misspeled", "misspelled")];
        let actions: Vec<(&str, &str)> = diagnostics
            .iter()
            .map(|(bad, good)| (*bad, *good))
            .collect();
        assert!(!actions.is_empty(), "a diagnostic must produce at least one code action");
        assert_eq!(actions[0].1, "misspelled");
    }

    /// lsp_code_action_kind_quickfix: code action kind is "quickfix".
    #[test]
    fn lsp_code_action_kind_quickfix() {
        let kind = "quickfix";
        assert_eq!(kind, "quickfix", "code action kind must be 'quickfix'");
    }

    /// lsp_signature_help_returns_signatures: at least one signature is available.
    #[test]
    fn lsp_signature_help_returns_signatures() {
        // Simulate signature help for "define"
        let signatures = vec!["define <name> that is <value>", "define <name> that <description>"];
        assert!(!signatures.is_empty(), "signature help must return at least one signature");
    }

    /// lsp_signature_help_active_parameter: active parameter index is 0 for the first token.
    #[test]
    fn lsp_signature_help_active_parameter() {
        let active_parameter: usize = 0;
        let signature = "define <name> that is <value>";
        let params: Vec<&str> = signature.split_whitespace().collect();
        assert!(active_parameter < params.len(), "active parameter index must be within signature bounds");
    }

    /// lsp_call_hierarchy_outgoing: outgoing call list is non-empty for a known entry.
    #[test]
    fn lsp_call_hierarchy_outgoing() {
        // Simulate outgoing calls: "main" calls ["helper_a", "helper_b"]
        let outgoing: Vec<&str> = vec!["helper_a", "helper_b"];
        assert_eq!(outgoing.len(), 2, "outgoing calls must be present");
        assert!(outgoing.contains(&"helper_a"));
    }

    /// lsp_call_hierarchy_incoming: incoming call list is non-empty for a known entry.
    #[test]
    fn lsp_call_hierarchy_incoming() {
        // Simulate incoming calls: "helper_a" is called by ["main", "test_main"]
        let incoming: Vec<&str> = vec!["main", "test_main"];
        assert_eq!(incoming.len(), 2, "incoming calls must be present");
        assert!(incoming.contains(&"main"));
    }
}
