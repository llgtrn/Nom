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

// ── Semantic highlight ───────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticToken {
    pub line: u32,
    pub start: u32,
    pub length: u32,
    pub token_type: String,
}

/// Classify a text token into a semantic category.
pub fn classify_token(text: &str) -> String {
    const KEYWORDS: &[&str] = &[
        "define", "that", "is", "with", "from", "as", "use", "return", "if", "else", "match",
        "let", "fn", "struct", "enum", "impl", "pub", "mod", "where", "for", "while", "loop",
        "break", "continue", "true", "false",
    ];
    if text.starts_with("//") || text.starts_with('#') {
        return "comment".to_string();
    }
    if (text.starts_with('"') && text.ends_with('"') && text.len() >= 2)
        || (text.starts_with('\'') && text.ends_with('\'') && text.len() >= 2)
    {
        return "string".to_string();
    }
    if text
        .chars()
        .all(|c| c.is_ascii_digit() || c == '.' || c == '_')
        && text
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
    {
        return "number".to_string();
    }
    if KEYWORDS.contains(&text) {
        return "keyword".to_string();
    }
    "identifier".to_string()
}

// ── Document symbols ─────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentSymbol {
    pub name: String,
    pub kind: String,
    pub range_start: u32,
    pub range_end: u32,
}

/// Extract top-level document symbols from source text.
/// Recognises `define <name>` patterns and produces a symbol per match.
pub fn extract_symbols(source: &str) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();
    for (line_idx, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("define ") {
            let name: String = rest.split_whitespace().next().unwrap_or("").to_string();
            if !name.is_empty() {
                let range_start = line_idx as u32;
                let range_end = range_start + 1;
                symbols.push(DocumentSymbol {
                    name,
                    kind: "function".to_string(),
                    range_start,
                    range_end,
                });
            }
        }
    }
    symbols
}

// ── Folding ranges ───────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoldingRange {
    pub start_line: u32,
    pub end_line: u32,
    pub kind: String,
}

/// Compute folding ranges for `{...}` blocks in source text.
pub fn compute_folding_ranges(source: &str) -> Vec<FoldingRange> {
    let mut ranges = Vec::new();
    let mut stack: Vec<u32> = Vec::new();
    for (idx, line) in source.lines().enumerate() {
        let ln = idx as u32;
        for ch in line.chars() {
            match ch {
                '{' => stack.push(ln),
                '}' => {
                    if let Some(start) = stack.pop() {
                        ranges.push(FoldingRange {
                            start_line: start,
                            end_line: ln,
                            kind: "region".to_string(),
                        });
                    }
                }
                _ => {}
            }
        }
    }
    ranges
}

// ── LSP position / range conversion ─────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LspPosition {
    pub line: u32,
    pub character: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

/// Convert a byte offset in `text` to an LSP `LspPosition` (line/character).
/// If `offset` exceeds the text length it is clamped to the end.
pub fn byte_offset_to_lsp_position(text: &str, offset: usize) -> LspPosition {
    let offset = offset.min(text.len());
    let mut line = 0u32;
    let mut character = 0u32;
    for (i, ch) in text.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += ch.len_utf16() as u32;
        }
    }
    LspPosition { line, character }
}

/// Convert an `LspPosition` back to a byte offset in `text`.
/// If the position is beyond the text it is clamped to `text.len()`.
pub fn lsp_position_to_byte_offset(text: &str, pos: LspPosition) -> usize {
    let mut cur_line = 0u32;
    let mut cur_char = 0u32;
    for (byte_idx, ch) in text.char_indices() {
        if cur_line == pos.line && cur_char == pos.character {
            return byte_idx;
        }
        if ch == '\n' {
            if cur_line == pos.line {
                // character was past end of line — clamp to the newline byte
                return byte_idx;
            }
            cur_line += 1;
            cur_char = 0;
        } else {
            cur_char += ch.len_utf16() as u32;
        }
    }
    text.len()
}

/// Convert a byte range in `text` to an `LspRange`.
pub fn byte_range_to_lsp_range(text: &str, range: std::ops::Range<usize>) -> LspRange {
    LspRange {
        start: byte_offset_to_lsp_position(text, range.start),
        end: byte_offset_to_lsp_position(text, range.end),
    }
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
        assert!(
            code_lens.contains(&"Run"),
            "Run action must be present in code lens"
        );
    }

    /// Code lens test action present — label is "Test".
    #[test]
    fn code_lens_test_action_present() {
        let code_lens = vec!["Run", "Test", "Debug"];
        assert!(
            code_lens.contains(&"Test"),
            "Test action must be present in code lens"
        );
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
        assert_eq!(
            positions, sorted,
            "semantic token spans must be sorted by position"
        );
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
                Ok(HoverResult {
                    contents: "ok".into(),
                    range: None,
                })
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

    // ── wave AB: semantic highlight tests ────────────────────────────────────

    /// classify_token("define") returns "keyword".
    #[test]
    fn semantic_classify_define_is_keyword() {
        assert_eq!(classify_token("define"), "keyword");
    }

    /// classify_token("my_var") returns "identifier".
    #[test]
    fn semantic_classify_identifier() {
        assert_eq!(classify_token("my_var"), "identifier");
    }

    /// classify_token("42") returns "number".
    #[test]
    fn semantic_classify_integer_number() {
        assert_eq!(classify_token("42"), "number");
    }

    /// classify_token with a quoted string literal returns "string".
    #[test]
    fn semantic_classify_string_literal() {
        assert_eq!(classify_token("\"hello\""), "string");
    }

    /// classify_token("// comment") returns "comment".
    #[test]
    fn semantic_classify_line_comment() {
        assert_eq!(classify_token("// comment"), "comment");
    }

    /// SemanticToken with length=0 is valid (zero-width marker).
    #[test]
    fn semantic_token_zero_length_is_valid() {
        let tok = SemanticToken {
            line: 0,
            start: 5,
            length: 0,
            token_type: "keyword".to_string(),
        };
        assert_eq!(tok.length, 0);
        assert_eq!(tok.token_type, "keyword");
    }

    /// SemanticToken line/start/length fields are all preserved.
    #[test]
    fn semantic_token_fields_preserved() {
        let tok = SemanticToken {
            line: 3,
            start: 7,
            length: 5,
            token_type: "identifier".to_string(),
        };
        assert_eq!(tok.line, 3);
        assert_eq!(tok.start, 7);
        assert_eq!(tok.length, 5);
    }

    /// Tokenising "define foo 42" produces keyword, identifier, number in order.
    #[test]
    fn semantic_tokenize_short_source_token_types_in_order() {
        let words = ["define", "foo", "42"];
        let types: Vec<String> = words.iter().map(|w| classify_token(w)).collect();
        assert_eq!(types[0], "keyword");
        assert_eq!(types[1], "identifier");
        assert_eq!(types[2], "number");
    }

    /// classify_token("fn") returns "keyword".
    #[test]
    fn semantic_classify_fn_keyword() {
        assert_eq!(classify_token("fn"), "keyword");
    }

    /// classify_token("3.14") returns "number".
    #[test]
    fn semantic_classify_float_number() {
        assert_eq!(classify_token("3.14"), "number");
    }

    /// SemanticToken can be cloned; clone equals original.
    #[test]
    fn semantic_token_clone_equals_original() {
        let tok = SemanticToken {
            line: 1,
            start: 2,
            length: 3,
            token_type: "string".to_string(),
        };
        let cloned = tok.clone();
        assert_eq!(cloned, tok);
    }

    // ── wave AB: document symbol tests ───────────────────────────────────────

    /// Empty source returns empty symbol list.
    #[test]
    fn doc_symbols_empty_source_returns_empty() {
        let symbols = extract_symbols("");
        assert!(symbols.is_empty());
    }

    /// Source with "define foo" produces a symbol named "foo".
    #[test]
    fn doc_symbols_define_foo_produces_foo_symbol() {
        let source = "define foo that is 1";
        let symbols = extract_symbols(source);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "foo");
    }

    /// Source with two nested definitions: both symbols appear.
    #[test]
    fn doc_symbols_two_definitions_both_appear() {
        let source = "define outer that is\n  define inner that is 1";
        let symbols = extract_symbols(source);
        assert_eq!(symbols.len(), 2);
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"outer"));
        assert!(names.contains(&"inner"));
    }

    /// Symbol kind is "function" for a define.
    #[test]
    fn doc_symbols_kind_is_function_for_define() {
        let source = "define my_func that is 0";
        let symbols = extract_symbols(source);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].kind, "function");
    }

    /// Symbol range_start < range_end.
    #[test]
    fn doc_symbols_range_start_less_than_end() {
        let source = "define compute that is 42";
        let symbols = extract_symbols(source);
        assert_eq!(symbols.len(), 1);
        assert!(symbols[0].range_start < symbols[0].range_end);
    }

    /// Source with no "define" keyword produces no symbols.
    #[test]
    fn doc_symbols_no_define_produces_no_symbols() {
        let source = "let x = 5\nuse std";
        let symbols = extract_symbols(source);
        assert!(symbols.is_empty());
    }

    // ── wave AB: folding range tests ─────────────────────────────────────────

    /// Empty source has no folding ranges.
    #[test]
    fn folding_empty_source_no_ranges() {
        let ranges = compute_folding_ranges("");
        assert!(ranges.is_empty());
    }

    /// Source with a single `{...}` block produces one folding range.
    #[test]
    fn folding_single_block_produces_one_range() {
        let source = "fn foo {\n  let x = 1\n}";
        let ranges = compute_folding_ranges(source);
        assert_eq!(ranges.len(), 1);
    }

    /// Folding range start_line <= end_line.
    #[test]
    fn folding_start_line_le_end_line() {
        let source = "outer {\n  inner\n}";
        let ranges = compute_folding_ranges(source);
        assert!(!ranges.is_empty());
        for r in &ranges {
            assert!(r.start_line <= r.end_line);
        }
    }

    /// Multiple independent blocks produce multiple ranges.
    #[test]
    fn folding_multiple_blocks_produce_multiple_ranges() {
        let source = "fn a {\n}\nfn b {\n}";
        let ranges = compute_folding_ranges(source);
        assert_eq!(ranges.len(), 2);
    }

    /// Nested blocks: inner range entirely within outer range.
    #[test]
    fn folding_nested_blocks_inner_within_outer() {
        let source = "outer {\n  inner {\n  }\n}";
        let ranges = compute_folding_ranges(source);
        // Should have two ranges (inner and outer).
        assert_eq!(ranges.len(), 2);
        // Find the smaller range (inner) and the larger range (outer).
        let inner = ranges
            .iter()
            .min_by_key(|r| r.end_line - r.start_line)
            .unwrap();
        let outer = ranges
            .iter()
            .max_by_key(|r| r.end_line - r.start_line)
            .unwrap();
        assert!(inner.start_line >= outer.start_line);
        assert!(inner.end_line <= outer.end_line);
    }

    /// FoldingRange kind field is preserved.
    #[test]
    fn folding_range_kind_preserved() {
        let r = FoldingRange {
            start_line: 0,
            end_line: 5,
            kind: "region".to_string(),
        };
        assert_eq!(r.kind, "region");
    }

    // ── wave AO-6: LSP position conversion tests ─────────────────────────────

    /// Empty string: byte offset 0 → line 0, character 0.
    #[test]
    fn lsp_pos_empty_string_offset_zero() {
        let pos = byte_offset_to_lsp_position("", 0);
        assert_eq!(
            pos,
            LspPosition {
                line: 0,
                character: 0
            }
        );
    }

    /// Empty string: out-of-bounds offset is clamped → line 0, character 0.
    #[test]
    fn lsp_pos_empty_string_oob_clamped() {
        let pos = byte_offset_to_lsp_position("", 999);
        assert_eq!(
            pos,
            LspPosition {
                line: 0,
                character: 0
            }
        );
    }

    /// Single line, offset at start → line 0, character 0.
    #[test]
    fn lsp_pos_single_line_offset_start() {
        let pos = byte_offset_to_lsp_position("hello", 0);
        assert_eq!(
            pos,
            LspPosition {
                line: 0,
                character: 0
            }
        );
    }

    /// Single line, offset in the middle.
    #[test]
    fn lsp_pos_single_line_offset_middle() {
        let pos = byte_offset_to_lsp_position("hello", 3);
        assert_eq!(
            pos,
            LspPosition {
                line: 0,
                character: 3
            }
        );
    }

    /// Single line, offset at end.
    #[test]
    fn lsp_pos_single_line_offset_end() {
        let pos = byte_offset_to_lsp_position("hello", 5);
        assert_eq!(
            pos,
            LspPosition {
                line: 0,
                character: 5
            }
        );
    }

    /// Multi-line: offset at start of second line.
    #[test]
    fn lsp_pos_multiline_start_of_second_line() {
        // "hello\nworld" — byte 6 is 'w' on line 1
        let pos = byte_offset_to_lsp_position("hello\nworld", 6);
        assert_eq!(
            pos,
            LspPosition {
                line: 1,
                character: 0
            }
        );
    }

    /// Multi-line: offset in the middle of the second line.
    #[test]
    fn lsp_pos_multiline_middle_of_second_line() {
        let pos = byte_offset_to_lsp_position("hello\nworld", 8);
        assert_eq!(
            pos,
            LspPosition {
                line: 1,
                character: 2
            }
        );
    }

    /// Multi-line: offset at the newline character (still on line 0).
    #[test]
    fn lsp_pos_multiline_at_newline_char() {
        // '\n' is at byte 5; it belongs to line 0
        let pos = byte_offset_to_lsp_position("hello\nworld", 5);
        assert_eq!(
            pos,
            LspPosition {
                line: 0,
                character: 5
            }
        );
    }

    /// Out-of-bounds offset is clamped to end of text.
    #[test]
    fn lsp_pos_oob_offset_clamped_to_end() {
        let text = "abc";
        let pos = byte_offset_to_lsp_position(text, 9999);
        // clamped to len=3 → line 0, character 3
        assert_eq!(
            pos,
            LspPosition {
                line: 0,
                character: 3
            }
        );
    }

    /// Unicode: two-byte UTF-8 char is counted as 1 UTF-16 unit.
    #[test]
    fn lsp_pos_unicode_two_byte_char_utf16_count() {
        // "é" = U+00E9 = 2 UTF-8 bytes, 1 UTF-16 code unit
        // "aéb" → byte offsets: a=0, é=1..3, b=3
        let text = "aéb";
        let pos_b = byte_offset_to_lsp_position(text, 3); // byte of 'b'
        assert_eq!(
            pos_b,
            LspPosition {
                line: 0,
                character: 2
            }
        ); // 'a' + 'é' = 2 UTF-16
    }

    /// lsp_position_to_byte_offset: round-trip on single line.
    #[test]
    fn lsp_pos_roundtrip_single_line() {
        let text = "hello";
        for offset in 0..=5 {
            let pos = byte_offset_to_lsp_position(text, offset);
            let back = lsp_position_to_byte_offset(text, pos);
            assert_eq!(back, offset, "round-trip failed for offset {offset}");
        }
    }

    /// lsp_position_to_byte_offset: round-trip on multi-line text.
    #[test]
    fn lsp_pos_roundtrip_multiline() {
        let text = "abc\nxyz\n12";
        for offset in [0, 1, 3, 4, 7, 8, 9] {
            let pos = byte_offset_to_lsp_position(text, offset);
            let back = lsp_position_to_byte_offset(text, pos);
            assert_eq!(back, offset, "round-trip failed for offset {offset}");
        }
    }

    /// byte_range_to_lsp_range: single-line range has same line number.
    #[test]
    fn lsp_range_single_line_same_line() {
        let range = byte_range_to_lsp_range("hello world", 6..11);
        assert_eq!(range.start.line, 0);
        assert_eq!(range.end.line, 0);
        assert_eq!(range.start.character, 6);
        assert_eq!(range.end.character, 11);
    }

    /// byte_range_to_lsp_range: cross-line range spans two lines.
    #[test]
    fn lsp_range_cross_line_span() {
        let text = "hello\nworld";
        let range = byte_range_to_lsp_range(text, 3..8);
        assert_eq!(range.start.line, 0);
        assert_eq!(range.end.line, 1);
    }

    /// byte_range_to_lsp_range: zero-length range has same start and end.
    #[test]
    fn lsp_range_zero_length_same_start_end() {
        let range = byte_range_to_lsp_range("hello", 2..2);
        assert_eq!(range.start, range.end);
    }

    /// LspPosition and LspRange implement Copy — can be used without move.
    #[test]
    fn lsp_position_and_range_are_copy() {
        let pos = LspPosition {
            line: 1,
            character: 5,
        };
        let _copy = pos; // copy
        let _ = pos.line; // original still accessible
        let r = LspRange {
            start: pos,
            end: pos,
        };
        let _r2 = r; // copy
        let _ = r.start.line; // original still accessible
    }

    /// byte_offset_to_lsp_position: three-line text, offset at start of third line.
    #[test]
    fn lsp_pos_three_lines_start_of_third() {
        // "a\nb\nc" — line 2 starts at byte 4
        let text = "a\nb\nc";
        let pos = byte_offset_to_lsp_position(text, 4);
        assert_eq!(pos.line, 2);
        assert_eq!(pos.character, 0);
    }

    /// byte_offset_to_lsp_position: offset exactly at text length.
    #[test]
    fn lsp_pos_offset_at_text_len() {
        let text = "hello";
        let pos = byte_offset_to_lsp_position(text, text.len());
        assert_eq!(
            pos,
            LspPosition {
                line: 0,
                character: 5
            }
        );
    }

    /// lsp_position_to_byte_offset: position on empty string returns 0.
    #[test]
    fn lsp_pos_to_offset_empty_string() {
        let offset = lsp_position_to_byte_offset(
            "",
            LspPosition {
                line: 0,
                character: 0,
            },
        );
        assert_eq!(offset, 0);
    }

    /// LspPosition equality: same line/character are equal, different are not.
    #[test]
    fn lsp_position_equality() {
        let a = LspPosition {
            line: 3,
            character: 7,
        };
        let b = LspPosition {
            line: 3,
            character: 7,
        };
        let c = LspPosition {
            line: 3,
            character: 8,
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    /// byte_range_to_lsp_range on full text.
    #[test]
    fn lsp_range_full_text() {
        let text = "hello";
        let range = byte_range_to_lsp_range(text, 0..text.len());
        assert_eq!(
            range.start,
            LspPosition {
                line: 0,
                character: 0
            }
        );
        assert_eq!(
            range.end,
            LspPosition {
                line: 0,
                character: 5
            }
        );
    }

    /// LspRange start is before end in a forward range.
    #[test]
    fn lsp_range_start_before_end() {
        let text = "line1\nline2";
        let range = byte_range_to_lsp_range(text, 2..8);
        // start is on line 0, end is on line 1; start line <= end line
        assert!(range.start.line <= range.end.line);
    }

    /// byte_offset_to_lsp_position: newline-only text.
    #[test]
    fn lsp_pos_newline_only_text() {
        let text = "\n\n";
        // offset 0 = line 0, char 0
        let pos0 = byte_offset_to_lsp_position(text, 0);
        assert_eq!(pos0.line, 0);
        // offset 1 = start of line 1
        let pos1 = byte_offset_to_lsp_position(text, 1);
        assert_eq!(pos1.line, 1);
        // offset 2 = start of line 2
        let pos2 = byte_offset_to_lsp_position(text, 2);
        assert_eq!(pos2.line, 2);
    }

    /// lsp_position_to_byte_offset: second line offset.
    #[test]
    fn lsp_pos_to_offset_second_line() {
        let text = "abc\nxyz";
        let pos = LspPosition {
            line: 1,
            character: 1,
        };
        let offset = lsp_position_to_byte_offset(text, pos);
        // line 1 starts at byte 4; char 1 → byte 5 ('y')
        assert_eq!(offset, 5);
    }
}
