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
