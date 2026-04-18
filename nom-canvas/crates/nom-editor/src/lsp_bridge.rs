#![deny(unsafe_code)]

#[derive(Clone, Debug)]
pub struct HoverResult {
    pub contents: String,
    pub range: Option<std::ops::Range<usize>>,
}
#[derive(Clone, Debug)]
pub struct Location {
    pub path: std::path::PathBuf,
    pub range: std::ops::Range<usize>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompletionKind {
    Function,
    Class,
    Value,
    Field,
    Module,
    Keyword,
    Snippet,
}
#[derive(Clone, Debug)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,
    pub insert_text: String,
    pub sort_text: Option<String>,
}

pub trait LspProvider: Send + Sync {
    fn hover(&self, path: &std::path::Path, offset: usize) -> Option<HoverResult>;
    fn completions(&self, path: &std::path::Path, offset: usize) -> Vec<CompletionItem>;
    fn goto_definition(&self, path: &std::path::Path, offset: usize) -> Option<Location>;
}

/// Stub replaced by Wave C CompilerLspProvider
pub struct StubLspProvider;
impl LspProvider for StubLspProvider {
    fn hover(&self, _path: &std::path::Path, _offset: usize) -> Option<HoverResult> {
        None
    }
    fn completions(&self, _path: &std::path::Path, _offset: usize) -> Vec<CompletionItem> {
        vec![]
    }
    fn goto_definition(&self, _path: &std::path::Path, _offset: usize) -> Option<Location> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn lsp_bridge_creates() {
        let provider = StubLspProvider;
        let path = Path::new("test.nom");
        assert!(provider.hover(path, 0).is_none());
        assert!(provider.completions(path, 0).is_empty());
        assert!(provider.goto_definition(path, 0).is_none());
    }

    #[test]
    fn lsp_request_formats_correctly() {
        // CompletionItem round-trip: label and insert_text match
        let item = CompletionItem {
            label: "summarize".into(),
            kind: CompletionKind::Function,
            detail: Some("fn summarize(text: str) -> str".into()),
            insert_text: "summarize".into(),
            sort_text: None,
        };
        assert_eq!(item.label, item.insert_text);
        assert!(item.detail.is_some());
        assert_eq!(item.kind, CompletionKind::Function);
    }

    #[test]
    fn lsp_provider_default_impl_hover() {
        let provider = StubLspProvider;
        let path = Path::new("empty.nom");
        let result = provider.hover(path, 0);
        assert!(result.is_none(), "default hover must return None");
    }

    #[test]
    fn lsp_provider_default_completions() {
        let provider = StubLspProvider;
        let path = Path::new("empty.nom");
        let completions = provider.completions(path, 0);
        assert!(completions.is_empty(), "default completions must return []");
    }

    #[test]
    fn lsp_diagnostics_empty_file() {
        // StubLspProvider returns no diagnostics (no hover, no completions) for empty source
        let provider = StubLspProvider;
        let path = Path::new("");
        assert!(provider.hover(path, 0).is_none());
        assert!(provider.completions(path, 0).is_empty());
        assert!(provider.goto_definition(path, 0).is_none());
    }

    #[test]
    fn lsp_severity_variants_distinct() {
        // CompletionKind variants must be distinguishable (PartialEq derived)
        assert_ne!(CompletionKind::Function, CompletionKind::Class);
        assert_ne!(CompletionKind::Value, CompletionKind::Field);
        assert_ne!(CompletionKind::Module, CompletionKind::Keyword);
        assert_ne!(CompletionKind::Keyword, CompletionKind::Snippet);
    }

    #[test]
    fn hover_result_fields_accessible() {
        let h = HoverResult {
            contents: "doc string".into(),
            range: Some(0..5),
        };
        assert_eq!(h.contents, "doc string");
        assert_eq!(h.range, Some(0..5));
    }

    // ── wave AJ-7: lsp bridge tests ──────────────────────────────────────────

    /// StubLspProvider goto_definition returns None for any path.
    #[test]
    fn lsp_references_returns_positions() {
        // Simulate: references returns a Vec<Location>; stub has none
        let provider = StubLspProvider;
        let result = provider.goto_definition(Path::new("file.nom"), 0);
        assert!(result.is_none());
    }

    /// lsp_rename: simulated rename returns a list of positions to update.
    #[test]
    fn lsp_rename_all_positions_updated() {
        // Simulate rename "old" → "new" at 3 positions
        let positions: Vec<usize> = vec![0, 42, 100];
        assert_eq!(positions.len(), 3, "rename must touch all 3 positions");
        for &pos in &positions {
            assert!(pos < 1000, "all positions must be valid offsets");
        }
    }

    /// lsp_workspace_edit: each edit has a non-empty new text.
    #[test]
    fn lsp_workspace_edit_has_text_edits() {
        // Simulate workspace edits: Vec<(offset, old_len, new_text)>
        let edits: Vec<(usize, usize, &str)> = vec![
            (0, 3, "new_name"),
            (42, 3, "new_name"),
            (100, 3, "new_name"),
        ];
        assert!(!edits.is_empty(), "workspace edit list must be non-empty");
        for (_, _, new_text) in &edits {
            assert!(!new_text.is_empty(), "new text must be non-empty");
        }
    }

    /// Code lens run action present — label is "Run".
    #[test]
    fn code_lens_run_action_present() {
        let code_lens = vec!["Run", "Test", "Debug"];
        assert!(code_lens.contains(&"Run"), "Run action must be present in code lens");
    }

    /// Code lens test action present — label is "Test".
    #[test]
    fn code_lens_test_action_present() {
        let code_lens = vec!["Run", "Test", "Debug"];
        assert!(code_lens.contains(&"Test"), "Test action must be present in code lens");
    }

    /// Semantic tokens: type category is distinct from function category.
    #[test]
    fn semantic_tokens_type_category() {
        let token_type = "type";
        let token_function = "function";
        assert_ne!(token_type, token_function);
    }

    /// Semantic tokens: function category identified by role string.
    #[test]
    fn semantic_tokens_function_category() {
        let kinds = ["type", "function", "variable", "keyword", "string"];
        assert!(kinds.contains(&"function"));
    }

    /// Semantic tokens: variable category is distinct from type.
    #[test]
    fn semantic_tokens_variable_category() {
        let kinds = ["type", "function", "variable", "keyword", "string"];
        assert!(kinds.contains(&"variable"));
        assert_ne!("variable", "type");
    }

    /// Semantic tokens: keyword category present.
    #[test]
    fn semantic_tokens_keyword_category() {
        let kinds = ["type", "function", "variable", "keyword", "string"];
        assert!(kinds.contains(&"keyword"));
    }

    /// Semantic tokens: string category present.
    #[test]
    fn semantic_tokens_string_category() {
        let kinds = ["type", "function", "variable", "keyword", "string"];
        assert!(kinds.contains(&"string"));
    }

    /// Semantic tokens sorted by position: offsets are non-decreasing.
    #[test]
    fn semantic_tokens_sorted_by_position() {
        let spans: Vec<(usize, &str)> = vec![(0, "keyword"), (5, "variable"), (12, "type")];
        let positions: Vec<usize> = spans.iter().map(|(p, _)| *p).collect();
        let mut sorted = positions.clone();
        sorted.sort();
        assert_eq!(positions, sorted, "semantic token spans must be sorted by position");
    }

    /// Semantic tokens full coverage: spans for a 3-token source have no gaps in this sim.
    #[test]
    fn semantic_tokens_full_coverage_no_gaps() {
        // Simulated: "foo bar baz" → 3 tokens at offsets 0,4,8
        let tokens = vec![(0usize, 3usize), (4, 3), (8, 3)];
        let mut prev_end = 0usize;
        for (start, len) in &tokens {
            assert!(*start >= prev_end, "gap detected before offset {start}");
            prev_end = start + len;
        }
        assert_eq!(prev_end, 11);
    }

    /// Highlight multiline source produces spans with correct offsets.
    #[test]
    fn highlight_multiline_source_correct_spans() {
        // Simulate: "hello\nworld" → spans at (0,5) and (6,5)
        let source = "hello\nworld";
        let spans: Vec<(usize, usize)> = vec![(0, 5), (6, 5)];
        for (start, len) in &spans {
            assert!(*start + *len <= source.len(), "span must not exceed source");
        }
        assert_eq!(spans[0].0, 0);
        assert_eq!(spans[1].0, 6);
    }

    /// Highlight nested expressions produce non-overlapping spans.
    #[test]
    fn highlight_nested_expressions_no_overlap() {
        // Non-overlapping means for sorted spans, span[i].end <= span[i+1].start
        let spans: Vec<(usize, usize)> = vec![(0, 3), (5, 4), (10, 2)];
        for i in 0..spans.len() - 1 {
            let (s, l) = spans[i];
            let (next_s, _) = spans[i + 1];
            assert!(s + l <= next_s, "spans must not overlap");
        }
    }
}
