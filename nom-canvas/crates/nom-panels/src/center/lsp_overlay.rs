//! LSP visual overlay — hover tooltips, completion popups, diagnostic squiggles.

// ---------------------------------------------------------------------------
// Diagnostics
// ---------------------------------------------------------------------------

/// Severity level for a diagnostic message.
#[derive(Debug, Clone, PartialEq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

/// A squiggle annotation that underlines a source range with a diagnostic message.
#[derive(Debug, Clone)]
pub struct DiagnosticSquiggle {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub message: String,
    pub severity: DiagnosticSeverity,
    pub source: Option<String>,
}

impl DiagnosticSquiggle {
    /// Construct a new squiggle. `source` defaults to `None`.
    pub fn new(
        start_line: u32,
        start_col: u32,
        end_line: u32,
        end_col: u32,
        message: &str,
        severity: DiagnosticSeverity,
    ) -> Self {
        Self {
            start_line,
            start_col,
            end_line,
            end_col,
            message: message.to_owned(),
            severity,
            source: None,
        }
    }

    /// Returns `true` when the severity is [`DiagnosticSeverity::Error`].
    pub fn is_error(&self) -> bool {
        self.severity == DiagnosticSeverity::Error
    }

    /// Returns `true` when the squiggle spans more than one line.
    pub fn spans_multiple_lines(&self) -> bool {
        self.end_line > self.start_line
    }
}

// ---------------------------------------------------------------------------
// Hover tooltip
// ---------------------------------------------------------------------------

/// A floating tooltip anchored to a source position.
#[derive(Debug, Clone)]
pub struct HoverTooltip {
    pub anchor_line: u32,
    pub anchor_col: u32,
    pub content: String,
    /// Maximum display width in logical pixels. Defaults to 400.
    pub max_width: u32,
    pub visible: bool,
}

impl HoverTooltip {
    /// Create a hidden tooltip with default `max_width` of 400.
    pub fn new(anchor_line: u32, anchor_col: u32, content: &str) -> Self {
        Self {
            anchor_line,
            anchor_col,
            content: content.to_owned(),
            max_width: 400,
            visible: false,
        }
    }

    /// Make the tooltip visible (builder-style).
    pub fn show(mut self) -> Self {
        self.visible = true;
        self
    }

    /// Hide the tooltip (builder-style).
    pub fn hide(mut self) -> Self {
        self.visible = false;
        self
    }

    /// Split the content on newlines and return each line as a `&str` slice.
    pub fn content_lines(&self) -> Vec<&str> {
        self.content.split('\n').collect()
    }
}

// ---------------------------------------------------------------------------
// Completion popup
// ---------------------------------------------------------------------------

/// Category of a completion entry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompletionItemKind {
    Function,
    Variable,
    Keyword,
    Kind,
    Skill,
    Module,
}

/// A single entry in the completion list.
#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub detail: Option<String>,
    pub kind: CompletionItemKind,
    pub insert_text: String,
    /// Relevance score in the range `0.0..=1.0`.
    pub score: f32,
}

/// A popup list of completion items anchored to a source position.
#[derive(Debug, Clone)]
pub struct CompletionPopup {
    pub items: Vec<CompletionItem>,
    pub selected_index: usize,
    pub anchor_line: u32,
    pub anchor_col: u32,
    pub visible: bool,
    /// Maximum number of items shown at once. Defaults to 10.
    pub max_items: usize,
}

impl CompletionPopup {
    /// Create a hidden, empty popup anchored at the given position.
    pub fn new(anchor_line: u32, anchor_col: u32) -> Self {
        Self {
            items: Vec::new(),
            selected_index: 0,
            anchor_line,
            anchor_col,
            visible: false,
            max_items: 10,
        }
    }

    /// Append a completion item (builder-style).
    pub fn push_item(mut self, item: CompletionItem) -> Self {
        self.items.push(item);
        self
    }

    /// Move selection down by one, wrapping around to the first item.
    pub fn select_next(mut self) -> Self {
        if !self.items.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.items.len();
        }
        self
    }

    /// Move selection up by one, wrapping around to the last item.
    pub fn select_prev(mut self) -> Self {
        if !self.items.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.items.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
        self
    }

    /// Return the currently selected item, if any.
    pub fn selected_item(&self) -> Option<&CompletionItem> {
        self.items.get(self.selected_index)
    }

    /// Make the popup visible (builder-style).
    pub fn show(mut self) -> Self {
        self.visible = true;
        self
    }

    /// Hide the popup (builder-style).
    pub fn hide(mut self) -> Self {
        self.visible = false;
        self
    }

    /// Return up to `max_items` items for display.
    pub fn visible_items(&self) -> &[CompletionItem] {
        let end = self.max_items.min(self.items.len());
        &self.items[..end]
    }
}

// ---------------------------------------------------------------------------
// Overlay root
// ---------------------------------------------------------------------------

/// Aggregates all LSP visual decorations for a single editor view.
#[derive(Debug, Default)]
pub struct LspOverlay {
    pub diagnostics: Vec<DiagnosticSquiggle>,
    pub tooltip: Option<HoverTooltip>,
    pub completion: Option<CompletionPopup>,
}

impl LspOverlay {
    /// Create an empty overlay.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a diagnostic squiggle (builder-style).
    pub fn push_diagnostic(mut self, d: DiagnosticSquiggle) -> Self {
        self.diagnostics.push(d);
        self
    }

    /// Replace the current tooltip (builder-style).
    pub fn set_tooltip(mut self, tooltip: HoverTooltip) -> Self {
        self.tooltip = Some(tooltip);
        self
    }

    /// Remove the current tooltip (builder-style).
    pub fn clear_tooltip(mut self) -> Self {
        self.tooltip = None;
        self
    }

    /// Replace the current completion popup (builder-style).
    pub fn set_completion(mut self, popup: CompletionPopup) -> Self {
        self.completion = Some(popup);
        self
    }

    /// Count diagnostics with [`DiagnosticSeverity::Error`].
    pub fn error_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.is_error()).count()
    }

    /// Count diagnostics with [`DiagnosticSeverity::Warning`].
    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Warning)
            .count()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // 1. diagnostic_new + is_error
    #[test]
    fn test_diagnostic_new_is_error() {
        let d = DiagnosticSquiggle::new(1, 0, 1, 5, "undefined variable", DiagnosticSeverity::Error);
        assert_eq!(d.start_line, 1);
        assert_eq!(d.end_col, 5);
        assert_eq!(d.message, "undefined variable");
        assert!(d.is_error());
        assert!(!DiagnosticSquiggle::new(0, 0, 0, 1, "warn", DiagnosticSeverity::Warning).is_error());
    }

    // 2. diagnostic_spans_multiple_lines
    #[test]
    fn test_diagnostic_spans_multiple_lines() {
        let single = DiagnosticSquiggle::new(3, 0, 3, 10, "x", DiagnosticSeverity::Hint);
        let multi = DiagnosticSquiggle::new(3, 0, 5, 2, "x", DiagnosticSeverity::Information);
        assert!(!single.spans_multiple_lines());
        assert!(multi.spans_multiple_lines());
    }

    // 3. hover_tooltip_new + content_lines
    #[test]
    fn test_hover_tooltip_new_content_lines() {
        let t = HoverTooltip::new(10, 4, "line one\nline two\nline three");
        assert_eq!(t.anchor_line, 10);
        assert_eq!(t.anchor_col, 4);
        assert_eq!(t.max_width, 400);
        assert!(!t.visible);
        assert_eq!(t.content_lines(), vec!["line one", "line two", "line three"]);
    }

    // 4. hover_show_hide
    #[test]
    fn test_hover_show_hide() {
        let t = HoverTooltip::new(0, 0, "hello");
        let shown = t.show();
        assert!(shown.visible);
        let hidden = shown.hide();
        assert!(!hidden.visible);
    }

    // 5. completion_popup_new
    #[test]
    fn test_completion_popup_new() {
        let p = CompletionPopup::new(7, 3);
        assert_eq!(p.anchor_line, 7);
        assert_eq!(p.anchor_col, 3);
        assert_eq!(p.max_items, 10);
        assert!(!p.visible);
        assert!(p.items.is_empty());
        assert_eq!(p.selected_index, 0);
    }

    // 6. completion_push_item
    #[test]
    fn test_completion_push_item() {
        let item = CompletionItem {
            label: "my_fn".to_owned(),
            detail: Some("fn() -> ()".to_owned()),
            kind: CompletionItemKind::Function,
            insert_text: "my_fn()".to_owned(),
            score: 0.9,
        };
        let p = CompletionPopup::new(0, 0).push_item(item);
        assert_eq!(p.items.len(), 1);
        assert_eq!(p.items[0].label, "my_fn");
    }

    // 7. completion_select_next_wraps
    #[test]
    fn test_completion_select_next_wraps() {
        let make_item = |label: &str| CompletionItem {
            label: label.to_owned(),
            detail: None,
            kind: CompletionItemKind::Variable,
            insert_text: label.to_owned(),
            score: 0.5,
        };
        let p = CompletionPopup::new(0, 0)
            .push_item(make_item("a"))
            .push_item(make_item("b"))
            .push_item(make_item("c"));
        // index starts at 0
        let p = p.select_next(); // → 1
        assert_eq!(p.selected_index, 1);
        let p = p.select_next(); // → 2
        assert_eq!(p.selected_index, 2);
        let p = p.select_next(); // → wraps to 0
        assert_eq!(p.selected_index, 0);
    }

    // 8. completion_selected_item
    #[test]
    fn test_completion_selected_item() {
        let item = CompletionItem {
            label: "kw".to_owned(),
            detail: None,
            kind: CompletionItemKind::Keyword,
            insert_text: "kw".to_owned(),
            score: 1.0,
        };
        let p = CompletionPopup::new(0, 0).push_item(item);
        let sel = p.selected_item().expect("should have selection");
        assert_eq!(sel.label, "kw");

        let empty = CompletionPopup::new(0, 0);
        assert!(empty.selected_item().is_none());
    }

    // 9. lsp_overlay_push_diagnostic_error_count
    #[test]
    fn test_lsp_overlay_push_diagnostic_error_count() {
        let overlay = LspOverlay::new()
            .push_diagnostic(DiagnosticSquiggle::new(0, 0, 0, 1, "e1", DiagnosticSeverity::Error))
            .push_diagnostic(DiagnosticSquiggle::new(1, 0, 1, 1, "w1", DiagnosticSeverity::Warning))
            .push_diagnostic(DiagnosticSquiggle::new(2, 0, 2, 1, "e2", DiagnosticSeverity::Error));
        assert_eq!(overlay.error_count(), 2);
        assert_eq!(overlay.warning_count(), 1);
        assert_eq!(overlay.diagnostics.len(), 3);
    }

    // 10. lsp_overlay_set_clear_tooltip
    #[test]
    fn test_lsp_overlay_set_clear_tooltip() {
        let tooltip = HoverTooltip::new(5, 2, "doc comment").show();
        let overlay = LspOverlay::new().set_tooltip(tooltip);
        assert!(overlay.tooltip.is_some());
        let overlay = overlay.clear_tooltip();
        assert!(overlay.tooltip.is_none());
    }
}
