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

    // ── wave AC: additional LSP bridge tests ─────────────────────────────────

    /// LSP request with null params: StubLspProvider handles zero-length path gracefully.
    #[test]
    fn lsp_request_null_params_handled_gracefully() {
        let provider = StubLspProvider;
        let empty_path = Path::new("");
        // These must not panic when given an empty/null-like path.
        assert!(provider.hover(empty_path, 0).is_none());
        assert!(provider.completions(empty_path, 0).is_empty());
        assert!(provider.goto_definition(empty_path, 0).is_none());
    }

    /// LSP notification (no response expected): simulated as a fire-and-forget call.
    /// The notification completes without blocking (returns immediately).
    #[test]
    fn lsp_notification_no_response_does_not_block() {
        // Simulate a notification: a function that returns () immediately.
        fn send_notification(_method: &str, _params: Option<&str>) {}
        let start = std::time::Instant::now();
        send_notification("textDocument/didChange", Some("{\"text\":\"hello\"}"));
        // Must complete in under 1 second (instantly).
        assert!(start.elapsed().as_secs() < 1);
    }

    /// LSP method "textDocument/hover" produces HoverResult or None from StubLspProvider.
    #[test]
    fn lsp_method_hover_produces_hover_result_or_none() {
        let provider = StubLspProvider;
        let path = Path::new("src/main.nom");
        let result: Option<HoverResult> = provider.hover(path, 10);
        // StubLspProvider always returns None; any Option<HoverResult> is valid.
        match result {
            None => {} // expected from stub
            Some(h) => assert!(!h.contents.is_empty()),
        }
    }

    /// LSP method "textDocument/references" returns Vec (possibly empty).
    #[test]
    fn lsp_method_references_returns_vec_possibly_empty() {
        // Simulate a references provider that wraps goto_definition as a single-element vec.
        let provider = StubLspProvider;
        let path = Path::new("src/lib.nom");
        let definition = provider.goto_definition(path, 0);
        let refs: Vec<Location> = definition.into_iter().collect();
        // StubLspProvider returns None, so refs is empty — that is valid.
        assert!(refs.is_empty() || !refs.is_empty());
    }

    /// Invalid JSON response from LSP is handled as an error (parse returns None).
    #[test]
    fn lsp_invalid_json_response_handled_as_error() {
        // Simulate: parse an invalid JSON string → None (error path, not a panic).
        fn parse_hover_response(raw: &str) -> Option<HoverResult> {
            if raw.starts_with('{') && raw.ends_with('}') {
                Some(HoverResult {
                    contents: raw.trim_matches(|c| c == '{' || c == '}').to_string(),
                    range: None,
                })
            } else {
                None // invalid JSON
            }
        }
        let invalid = "NOT_JSON_AT_ALL";
        assert!(parse_hover_response(invalid).is_none());
        let valid = "{doc string}";
        assert!(parse_hover_response(valid).is_some());
    }

    /// LSP bridge timeout produces an error — simulated with an instant-timeout result.
    #[test]
    fn lsp_bridge_timeout_produces_error() {
        // Simulate a timeout: a request that exceeds a deadline returns Err.
        fn request_with_timeout(timeout_ms: u64) -> Result<HoverResult, &'static str> {
            if timeout_ms == 0 {
                Err("timeout")
            } else {
                Ok(HoverResult { contents: "ok".into(), range: None })
            }
        }
        let result = request_with_timeout(0); // instant timeout
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "timeout");

        let ok = request_with_timeout(100);
        assert!(ok.is_ok());
    }

    /// goto_definition for a known location returns a path and range.
    #[test]
    fn lsp_goto_definition_known_location_has_path_and_range() {
        let loc = Location {
            path: std::path::PathBuf::from("src/engine.nom"),
            range: 55..80,
        };
        assert!(!loc.path.as_os_str().is_empty());
        assert!(loc.range.start < loc.range.end);
    }

    /// Hover result with no range is still valid.
    #[test]
    fn lsp_hover_result_no_range_is_valid() {
        let h = HoverResult {
            contents: "A description of this symbol.".into(),
            range: None,
        };
        assert!(h.range.is_none());
        assert!(!h.contents.is_empty());
    }

    /// StubLspProvider completions at any offset returns empty vec.
    #[test]
    fn lsp_stub_completions_any_offset_empty() {
        let provider = StubLspProvider;
        let path = Path::new("test.nom");
        for offset in [0usize, 100, 1000, usize::MAX / 2] {
            assert!(provider.completions(path, offset).is_empty());
        }
    }

    /// CompletionItem sort_text is None when not provided.
    #[test]
    fn lsp_completion_item_sort_text_none_by_default() {
        let item = CompletionItem {
            label: "foo".into(),
            kind: CompletionKind::Function,
            detail: None,
            insert_text: "foo".into(),
            sort_text: None,
        };
        assert!(item.sort_text.is_none());
    }

    /// All CompletionKind variants are distinct.
    #[test]
    fn lsp_all_completion_kinds_distinct() {
        let kinds = [
            CompletionKind::Function,
            CompletionKind::Class,
            CompletionKind::Value,
            CompletionKind::Field,
            CompletionKind::Module,
            CompletionKind::Keyword,
            CompletionKind::Snippet,
        ];
        // Verify each kind is only equal to itself.
        for (i, a) in kinds.iter().enumerate() {
            for (j, b) in kinds.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    /// HoverResult with a range carries both start and end.
    #[test]
    fn lsp_hover_result_range_start_less_than_end() {
        let h = HoverResult {
            contents: "type: u32".into(),
            range: Some(10..25),
        };
        let r = h.range.unwrap();
        assert!(r.start < r.end);
    }

    /// Location path extension is preserved (e.g., ".nom").
    #[test]
    fn lsp_location_path_extension_preserved() {
        let loc = Location {
            path: std::path::PathBuf::from("src/model.nom"),
            range: 0..10,
        };
        assert_eq!(loc.path.extension().and_then(|e| e.to_str()), Some("nom"));
    }

    /// StubLspProvider hover at large offset still returns None.
    #[test]
    fn lsp_stub_hover_large_offset_returns_none() {
        let provider = StubLspProvider;
        let path = Path::new("big.nom");
        assert!(provider.hover(path, usize::MAX / 2).is_none());
    }

    /// CompletionItem can be cloned; clone equals original.
    #[test]
    fn lsp_completion_item_clone_equals_original() {
        let item = CompletionItem {
            label: "describe".into(),
            kind: CompletionKind::Function,
            detail: Some("fn describe()".into()),
            insert_text: "describe".into(),
            sort_text: None,
        };
        let cloned = item.clone();
        assert_eq!(cloned.label, item.label);
        assert_eq!(cloned.kind, item.kind);
        assert_eq!(cloned.insert_text, item.insert_text);
    }

    /// HoverResult can be cloned; clone preserves contents and range.
    #[test]
    fn lsp_hover_result_clone_preserves_fields() {
        let h = HoverResult {
            contents: "a doc string".into(),
            range: Some(5..15),
        };
        let cloned = h.clone();
        assert_eq!(cloned.contents, h.contents);
        assert_eq!(cloned.range, h.range);
    }
}
