#![deny(unsafe_code)]

use super::lsp_overlay::LspOverlay;

/// View model for the code editor pane, wiring LSP decorations to toggle flags.
#[derive(Debug, Default)]
pub struct EditorView {
    pub lsp_overlay: LspOverlay,
    pub show_diagnostics: bool,
    pub show_completions: bool,
    pub show_hover: bool,
}

impl EditorView {
    /// Create a new `EditorView` with all toggles disabled.
    pub fn new() -> Self {
        Self {
            lsp_overlay: LspOverlay::new(),
            show_diagnostics: false,
            show_completions: false,
            show_hover: false,
        }
    }

    /// Toggle the diagnostics overlay on or off.
    pub fn toggle_diagnostics(&mut self) {
        self.show_diagnostics = !self.show_diagnostics;
    }

    /// Toggle the completion popup on or off.
    pub fn toggle_completions(&mut self) {
        self.show_completions = !self.show_completions;
    }

    /// Delegate to [`LspOverlay::error_count`].
    pub fn error_count(&self) -> usize {
        self.lsp_overlay.error_count()
    }

    /// Returns `true` when the LSP overlay has a completion popup attached.
    pub fn has_completions(&self) -> bool {
        self.lsp_overlay.completion.is_some()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::center::lsp_overlay::{
        CompletionItem, CompletionItemKind, CompletionPopup, DiagnosticSeverity, DiagnosticSquiggle,
    };

    #[test]
    fn new_defaults() {
        let v = EditorView::new();
        assert!(!v.show_diagnostics);
        assert!(!v.show_completions);
        assert!(!v.show_hover);
        assert_eq!(v.error_count(), 0);
        assert!(!v.has_completions());
    }

    #[test]
    fn toggle_diagnostics() {
        let mut v = EditorView::new();
        assert!(!v.show_diagnostics);
        v.toggle_diagnostics();
        assert!(v.show_diagnostics);
        v.toggle_diagnostics();
        assert!(!v.show_diagnostics);
    }

    #[test]
    fn error_count_delegates() {
        let mut v = EditorView::new();
        v.lsp_overlay = LspOverlay::new()
            .push_diagnostic(DiagnosticSquiggle::new(
                0,
                0,
                0,
                1,
                "e1",
                DiagnosticSeverity::Error,
            ))
            .push_diagnostic(DiagnosticSquiggle::new(
                1,
                0,
                1,
                1,
                "w1",
                DiagnosticSeverity::Warning,
            ))
            .push_diagnostic(DiagnosticSquiggle::new(
                2,
                0,
                2,
                1,
                "e2",
                DiagnosticSeverity::Error,
            ));
        assert_eq!(v.error_count(), 2);
    }

    #[test]
    fn has_completions() {
        let mut v = EditorView::new();
        assert!(!v.has_completions());
        let popup = CompletionPopup::new(5, 3).push_item(CompletionItem {
            label: "fn_a".to_owned(),
            detail: None,
            kind: CompletionItemKind::Function,
            insert_text: "fn_a()".to_owned(),
            score: 0.8,
        });
        v.lsp_overlay = LspOverlay::new().set_completion(popup);
        assert!(v.has_completions());
    }
}
