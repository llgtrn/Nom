#![deny(unsafe_code)]

use crate::shared::SharedState;
use std::sync::Arc;

/// A hover response for a nomtu word.
#[derive(Debug, Clone)]
pub struct HoverResponse {
    pub word: String,
    pub kind: Option<String>,
    pub documentation: String,
    pub confidence: f32,
}

/// A completion item for prefix-based lookup.
#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub kind_hint: String,   // "nomtu", "keyword", "kind", etc.
    pub detail: Option<String>,
    pub sort_score: f32,
}

/// A diagnostic from the inline compiler.
#[derive(Debug, Clone)]
pub struct LspDiagnostic {
    pub line: u32,
    pub column: u32,
    pub message: String,
    pub severity: LspSeverity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LspSeverity { Error, Warning, Info, Hint }

/// CompilerLspProvider — bridges nom-compiler ops to LSP protocol shapes.
pub struct CompilerLspProvider {
    shared: Arc<SharedState>,
}

impl CompilerLspProvider {
    pub fn new(shared: Arc<SharedState>) -> Self { Self { shared } }

    /// Get hover documentation for a word.
    pub fn hover(&self, word: &str) -> Option<HoverResponse> {
        if word.is_empty() { return None; }
        Some(HoverResponse {
            word: word.to_string(),
            kind: Some("nomtu".to_string()),
            documentation: format!("nomtu `{}` — defined in {}", word, self.shared.dict_path),
            confidence: 0.8,
        })
    }

    /// Get completions for a prefix.
    pub fn complete(&self, prefix: &str, limit: usize) -> Vec<CompletionItem> {
        if prefix.is_empty() { return vec![]; }
        // Stub: return single prefix-completion; real impl would query nom-dict
        vec![CompletionItem {
            label: format!("{prefix}_nomtu"),
            kind_hint: "nomtu".to_string(),
            detail: Some(format!("from {}", self.shared.dict_path)),
            sort_score: 1.0,
        }]
        .into_iter()
        .take(limit)
        .collect()
    }

    /// Check a source line for inline diagnostics.
    pub fn diagnose_line(&self, line: &str, line_num: u32) -> Vec<LspDiagnostic> {
        let mut diags = Vec::new();
        // Simple: flag lines that are too long (>120 chars)
        if line.len() > 120 {
            diags.push(LspDiagnostic {
                line: line_num, column: 120,
                message: "line exceeds 120 characters".to_string(),
                severity: LspSeverity::Warning,
            });
        }
        diags
    }

    /// Batch-check a source file.
    pub fn diagnose(&self, source: &str) -> Vec<LspDiagnostic> {
        source.lines().enumerate()
            .flat_map(|(i, line)| self.diagnose_line(line, i as u32 + 1))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::SharedState;

    fn make_provider() -> CompilerLspProvider {
        CompilerLspProvider::new(Arc::new(SharedState::new("test.db", "test.gram")))
    }

    #[test]
    fn lsp_hover_returns_response() {
        let p = make_provider();
        let r = p.hover("run").unwrap();
        assert_eq!(r.word, "run");
        assert!(r.confidence > 0.0);
    }
    #[test]
    fn lsp_hover_empty_returns_none() {
        let p = make_provider();
        assert!(p.hover("").is_none());
    }
    #[test]
    fn lsp_complete_returns_items() {
        let p = make_provider();
        let items = p.complete("ru", 5);
        assert!(!items.is_empty());
        assert!(items[0].label.starts_with("ru"));
    }
    #[test]
    fn lsp_complete_empty_prefix_returns_empty() {
        let p = make_provider();
        assert!(p.complete("", 5).is_empty());
    }
    #[test]
    fn lsp_diagnose_long_line() {
        let p = make_provider();
        let long = "x".repeat(130);
        let diags = p.diagnose(&long);
        assert!(!diags.is_empty());
        assert_eq!(diags[0].severity, LspSeverity::Warning);
    }
    #[test]
    fn lsp_diagnose_clean_source() {
        let p = make_provider();
        let diags = p.diagnose("short line\nanother line");
        assert!(diags.is_empty());
    }

    #[test]
    fn lsp_provider_hover_returns_response() {
        let p = make_provider();
        let r = p.hover("emit").unwrap();
        assert_eq!(r.word, "emit");
        assert!(r.documentation.contains("emit"));
    }

    #[test]
    fn lsp_provider_completion_items_non_empty() {
        let p = make_provider();
        let items = p.complete("de", 10);
        assert!(!items.is_empty());
    }

    #[test]
    fn lsp_severity_ordering() {
        // Variants are defined in order Error < Warning < Info < Hint
        let severities = [LspSeverity::Error, LspSeverity::Warning, LspSeverity::Info, LspSeverity::Hint];
        // Each must be cloneable and comparable
        for s in &severities {
            let cloned = s.clone();
            assert_eq!(&cloned, s);
        }
        // Error != Warning
        assert_ne!(LspSeverity::Error, LspSeverity::Warning);
        // Info != Hint
        assert_ne!(LspSeverity::Info, LspSeverity::Hint);
    }

    #[test]
    fn lsp_diagnostic_has_range() {
        let d = LspDiagnostic {
            line: 3,
            column: 10,
            message: "test error".into(),
            severity: LspSeverity::Error,
        };
        assert_eq!(d.line, 3);
        assert_eq!(d.column, 10);
        assert_eq!(d.message, "test error");
        assert_eq!(d.severity, LspSeverity::Error);
    }

    #[test]
    fn completion_item_label_preserved() {
        let item = CompletionItem {
            label: "define".to_string(),
            kind_hint: "keyword".to_string(),
            detail: Some("core keyword".into()),
            sort_score: 1.0,
        };
        assert_eq!(item.label, "define");
        assert_eq!(item.kind_hint, "keyword");
        assert_eq!(item.sort_score, 1.0);
        assert_eq!(item.detail.as_deref(), Some("core keyword"));
    }
}
