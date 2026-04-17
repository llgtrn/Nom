//! Bridge to the existing nom-compiler LSP service (hover, completion, inlay
//! hints, diagnostics).  This crate does NOT depend on nom-lsp directly;
//! instead it defines a trait the bridge implements so the editor can run
//! against a stub in tests and against the real LSP in production.
#![deny(unsafe_code)]

use std::collections::HashMap;
use std::ops::Range;

#[derive(Clone, Debug, PartialEq)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HoverInfo {
    pub contents: String,
    pub range: Option<Range<usize>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,
    pub insert_text: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CompletionKind {
    Text,
    Keyword,
    Function,
    Variable,
    Type,
    Module,
    Field,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub range: Range<usize>,
    pub message: String,
    pub code: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Information,
    Hint,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InlayHint {
    pub offset: usize,
    pub label: String,
    pub kind: InlayHintKind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InlayHintKind {
    Type,
    Parameter,
    Return,
}

#[derive(Debug, thiserror::Error)]
pub enum LspError {
    #[error("LSP server not initialized")]
    NotInitialized,
    #[error("request timed out after {0}ms")]
    Timeout(u64),
    #[error("invalid document URI: {0}")]
    InvalidUri(String),
}

/// Trait the editor consumes; the production impl wraps nom-compiler/nom-lsp.
pub trait LspProvider {
    fn hover(&self, uri: &str, offset: usize) -> Result<Option<HoverInfo>, LspError>;
    fn complete(&self, uri: &str, offset: usize) -> Result<Vec<CompletionItem>, LspError>;
    fn inlay_hints(&self, uri: &str, range: Range<usize>) -> Result<Vec<InlayHint>, LspError>;
    fn diagnostics(&self, uri: &str) -> Result<Vec<Diagnostic>, LspError>;
}

/// In-memory stub provider for testing + development.
pub struct StubLspProvider {
    hovers: HashMap<(String, usize), HoverInfo>,
    completions: HashMap<String, Vec<CompletionItem>>,
    hints: HashMap<String, Vec<InlayHint>>,
    diags: HashMap<String, Vec<Diagnostic>>,
}

impl StubLspProvider {
    pub fn new() -> Self {
        Self {
            hovers: HashMap::new(),
            completions: HashMap::new(),
            hints: HashMap::new(),
            diags: HashMap::new(),
        }
    }

    pub fn add_hover(&mut self, uri: impl Into<String>, offset: usize, info: HoverInfo) {
        self.hovers.insert((uri.into(), offset), info);
    }

    pub fn add_completion(&mut self, uri: impl Into<String>, items: Vec<CompletionItem>) {
        self.completions.insert(uri.into(), items);
    }

    pub fn add_hints(&mut self, uri: impl Into<String>, hints: Vec<InlayHint>) {
        self.hints.insert(uri.into(), hints);
    }

    pub fn add_diagnostics(&mut self, uri: impl Into<String>, diags: Vec<Diagnostic>) {
        self.diags.insert(uri.into(), diags);
    }
}

impl Default for StubLspProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl LspProvider for StubLspProvider {
    fn hover(&self, uri: &str, offset: usize) -> Result<Option<HoverInfo>, LspError> {
        Ok(self.hovers.get(&(uri.to_owned(), offset)).cloned())
    }

    fn complete(&self, uri: &str, _offset: usize) -> Result<Vec<CompletionItem>, LspError> {
        Ok(self.completions.get(uri).cloned().unwrap_or_default())
    }

    fn inlay_hints(&self, uri: &str, range: Range<usize>) -> Result<Vec<InlayHint>, LspError> {
        let all = self.hints.get(uri).cloned().unwrap_or_default();
        Ok(all.into_iter().filter(|h| range.contains(&h.offset)).collect())
    }

    fn diagnostics(&self, uri: &str) -> Result<Vec<Diagnostic>, LspError> {
        Ok(self.diags.get(uri).cloned().unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_new_is_empty() {
        let p = StubLspProvider::new();
        assert!(p.hover("file://a", 0).unwrap().is_none());
        assert!(p.complete("file://a", 0).unwrap().is_empty());
        assert!(p.inlay_hints("file://a", 0..100).unwrap().is_empty());
        assert!(p.diagnostics("file://a").unwrap().is_empty());
    }

    #[test]
    fn hover_hit_and_miss() {
        let mut p = StubLspProvider::new();
        let info = HoverInfo { contents: "hello".into(), range: Some(0..5) };
        p.add_hover("file://doc", 10, info.clone());
        assert_eq!(p.hover("file://doc", 10).unwrap(), Some(info));
        assert!(p.hover("file://doc", 99).unwrap().is_none());
        assert!(p.hover("file://other", 10).unwrap().is_none());
    }

    #[test]
    fn complete_hit_and_miss() {
        let mut p = StubLspProvider::new();
        let items = vec![
            CompletionItem {
                label: "foo".into(),
                kind: CompletionKind::Function,
                detail: None,
                insert_text: None,
            },
        ];
        p.add_completion("file://doc", items.clone());
        assert_eq!(p.complete("file://doc", 0).unwrap(), items);
        assert!(p.complete("file://other", 0).unwrap().is_empty());
    }

    #[test]
    fn inlay_hints_in_range() {
        let mut p = StubLspProvider::new();
        let hints = vec![
            InlayHint { offset: 5, label: ": i32".into(), kind: InlayHintKind::Type },
            InlayHint { offset: 50, label: "x:".into(), kind: InlayHintKind::Parameter },
        ];
        p.add_hints("file://doc", hints);
        let result = p.inlay_hints("file://doc", 0..20).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].offset, 5);
    }

    #[test]
    fn inlay_hints_out_of_range() {
        let mut p = StubLspProvider::new();
        let hints = vec![
            InlayHint { offset: 100, label: "-> T".into(), kind: InlayHintKind::Return },
        ];
        p.add_hints("file://doc", hints);
        let result = p.inlay_hints("file://doc", 0..50).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn diagnostics_hit_and_miss() {
        let mut p = StubLspProvider::new();
        let diags = vec![
            Diagnostic {
                severity: Severity::Error,
                range: 0..10,
                message: "undefined variable".into(),
                code: Some("E001".into()),
            },
        ];
        p.add_diagnostics("file://doc", diags.clone());
        assert_eq!(p.diagnostics("file://doc").unwrap(), diags);
        assert!(p.diagnostics("file://other").unwrap().is_empty());
    }

    #[test]
    fn severity_round_trip() {
        let severities = [
            Severity::Error,
            Severity::Warning,
            Severity::Information,
            Severity::Hint,
        ];
        for s in severities {
            let d = Diagnostic { severity: s, range: 0..1, message: "x".into(), code: None };
            assert_eq!(d.severity, s);
        }
    }

    #[test]
    fn completion_kind_round_trip() {
        let kinds = [
            CompletionKind::Text,
            CompletionKind::Keyword,
            CompletionKind::Function,
            CompletionKind::Variable,
            CompletionKind::Type,
            CompletionKind::Module,
            CompletionKind::Field,
        ];
        for k in kinds {
            let item = CompletionItem {
                label: "x".into(),
                kind: k,
                detail: None,
                insert_text: None,
            };
            assert_eq!(item.kind, k);
        }
    }

    #[test]
    fn inlay_hint_kind_round_trip() {
        let kinds = [InlayHintKind::Type, InlayHintKind::Parameter, InlayHintKind::Return];
        for k in kinds {
            let h = InlayHint { offset: 0, label: "x".into(), kind: k };
            assert_eq!(h.kind, k);
        }
    }

    #[test]
    fn lsp_error_timeout_display() {
        let err = LspError::Timeout(500);
        assert_eq!(err.to_string(), "request timed out after 500ms");
    }

    #[test]
    fn lsp_error_not_initialized_display() {
        let err = LspError::NotInitialized;
        assert_eq!(err.to_string(), "LSP server not initialized");
    }
}
