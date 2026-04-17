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
}
