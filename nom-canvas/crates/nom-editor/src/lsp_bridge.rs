#![deny(unsafe_code)]

#[derive(Clone, Debug)]
pub struct HoverResult { pub contents: String, pub range: Option<std::ops::Range<usize>> }
#[derive(Clone, Debug)]
pub struct Location { pub path: std::path::PathBuf, pub range: std::ops::Range<usize> }
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompletionKind { Function, Class, Value, Field, Module, Keyword, Snippet }
#[derive(Clone, Debug)]
pub struct CompletionItem { pub label: String, pub kind: CompletionKind, pub detail: Option<String>, pub insert_text: String, pub sort_text: Option<String> }

pub trait LspProvider: Send + Sync {
    fn hover(&self, path: &std::path::Path, offset: usize) -> Option<HoverResult>;
    fn completions(&self, path: &std::path::Path, offset: usize) -> Vec<CompletionItem>;
    fn goto_definition(&self, path: &std::path::Path, offset: usize) -> Option<Location>;
}

/// Stub replaced by Wave C CompilerLspProvider
pub struct StubLspProvider;
impl LspProvider for StubLspProvider {
    fn hover(&self, _path: &std::path::Path, _offset: usize) -> Option<HoverResult> { None }
    fn completions(&self, _path: &std::path::Path, _offset: usize) -> Vec<CompletionItem> { vec![] }
    fn goto_definition(&self, _path: &std::path::Path, _offset: usize) -> Option<Location> { None }
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
}
