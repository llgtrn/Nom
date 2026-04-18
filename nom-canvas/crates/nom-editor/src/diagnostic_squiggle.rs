/// Severity level for a diagnostic message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

impl DiagnosticSeverity {
    /// Returns true only for `Error`.
    pub fn is_error(&self) -> bool {
        matches!(self, DiagnosticSeverity::Error)
    }

    /// Returns the CSS class string for this severity.
    pub fn css_class(&self) -> &'static str {
        match self {
            DiagnosticSeverity::Error => "error",
            DiagnosticSeverity::Warning => "warning",
            DiagnosticSeverity::Info => "info",
            DiagnosticSeverity::Hint => "hint",
        }
    }
}

/// A byte-range span carrying a diagnostic message and severity.
#[derive(Debug, Clone)]
pub struct DiagnosticSpan {
    pub start_byte: usize,
    pub end_byte: usize,
    pub message: String,
    pub severity: DiagnosticSeverity,
}

impl DiagnosticSpan {
    /// Number of bytes covered by this span.
    pub fn len(&self) -> usize {
        self.end_byte.saturating_sub(self.start_byte)
    }

    /// Returns true if this span overlaps with `other`.
    /// Two spans overlap when `self.start < other.end && self.end > other.start`.
    pub fn overlaps(&self, other: &DiagnosticSpan) -> bool {
        self.start_byte < other.end_byte && self.end_byte > other.start_byte
    }
}

/// Visual style for rendering a squiggle underline.
#[derive(Debug, Clone)]
pub struct SquiggleStyle {
    pub color: String,
    pub thickness: f32,
    pub dash_pattern: Vec<f32>,
}

impl SquiggleStyle {
    /// Returns the canonical squiggle style for the given severity.
    pub fn for_severity(s: &DiagnosticSeverity) -> SquiggleStyle {
        match s {
            DiagnosticSeverity::Error => SquiggleStyle {
                color: "#FF0000".to_string(),
                thickness: 2.0,
                dash_pattern: vec![],
            },
            DiagnosticSeverity::Warning => SquiggleStyle {
                color: "#FFA500".to_string(),
                thickness: 1.5,
                dash_pattern: vec![4.0, 2.0],
            },
            DiagnosticSeverity::Info => SquiggleStyle {
                color: "#0080FF".to_string(),
                thickness: 1.0,
                dash_pattern: vec![2.0, 2.0],
            },
            DiagnosticSeverity::Hint => SquiggleStyle {
                color: "#808080".to_string(),
                thickness: 1.0,
                dash_pattern: vec![1.0, 3.0],
            },
        }
    }
}

/// Overlay that accumulates diagnostic spans for rendering.
#[derive(Debug, Default)]
pub struct DiagnosticOverlay {
    pub diagnostics: Vec<DiagnosticSpan>,
}

impl DiagnosticOverlay {
    /// Appends a span to the overlay.
    pub fn add(&mut self, span: DiagnosticSpan) {
        self.diagnostics.push(span);
    }

    /// Returns references to all spans whose severity is `Error`.
    pub fn errors(&self) -> Vec<&DiagnosticSpan> {
        self.diagnostics
            .iter()
            .filter(|s| s.severity.is_error())
            .collect()
    }

    /// Removes all spans.
    pub fn clear(&mut self) {
        self.diagnostics.clear();
    }

    /// Pairs each span with its computed squiggle style.
    pub fn squiggle_styles(&self) -> Vec<(&DiagnosticSpan, SquiggleStyle)> {
        self.diagnostics
            .iter()
            .map(|s| (s, SquiggleStyle::for_severity(&s.severity)))
            .collect()
    }
}

#[cfg(test)]
mod diagnostic_squiggle_tests {
    use super::*;

    fn make_span(start: usize, end: usize, severity: DiagnosticSeverity) -> DiagnosticSpan {
        DiagnosticSpan {
            start_byte: start,
            end_byte: end,
            message: "test".to_string(),
            severity,
        }
    }

    #[test]
    fn severity_is_error() {
        assert!(DiagnosticSeverity::Error.is_error());
        assert!(!DiagnosticSeverity::Warning.is_error());
        assert!(!DiagnosticSeverity::Info.is_error());
        assert!(!DiagnosticSeverity::Hint.is_error());
    }

    #[test]
    fn css_class_for_each_severity() {
        assert_eq!(DiagnosticSeverity::Error.css_class(), "error");
        assert_eq!(DiagnosticSeverity::Warning.css_class(), "warning");
        assert_eq!(DiagnosticSeverity::Info.css_class(), "info");
        assert_eq!(DiagnosticSeverity::Hint.css_class(), "hint");
    }

    #[test]
    fn span_len() {
        let span = make_span(10, 20, DiagnosticSeverity::Error);
        assert_eq!(span.len(), 10);
    }

    #[test]
    fn span_overlaps_true() {
        let a = make_span(5, 15, DiagnosticSeverity::Error);
        let b = make_span(10, 20, DiagnosticSeverity::Warning);
        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));
    }

    #[test]
    fn span_overlaps_false_adjacent() {
        // Adjacent spans: a ends exactly where b begins — no overlap.
        let a = make_span(0, 10, DiagnosticSeverity::Error);
        let b = make_span(10, 20, DiagnosticSeverity::Warning);
        assert!(!a.overlaps(&b));
        assert!(!b.overlaps(&a));
    }

    #[test]
    fn squiggle_style_colors() {
        assert_eq!(SquiggleStyle::for_severity(&DiagnosticSeverity::Error).color, "#FF0000");
        assert_eq!(SquiggleStyle::for_severity(&DiagnosticSeverity::Warning).color, "#FFA500");
        assert_eq!(SquiggleStyle::for_severity(&DiagnosticSeverity::Info).color, "#0080FF");
        assert_eq!(SquiggleStyle::for_severity(&DiagnosticSeverity::Hint).color, "#808080");
    }

    #[test]
    fn overlay_add_and_errors_count() {
        let mut overlay = DiagnosticOverlay::default();
        overlay.add(make_span(0, 5, DiagnosticSeverity::Error));
        overlay.add(make_span(5, 10, DiagnosticSeverity::Warning));
        overlay.add(make_span(10, 15, DiagnosticSeverity::Error));
        assert_eq!(overlay.diagnostics.len(), 3);
        assert_eq!(overlay.errors().len(), 2);
    }

    #[test]
    fn overlay_clear() {
        let mut overlay = DiagnosticOverlay::default();
        overlay.add(make_span(0, 5, DiagnosticSeverity::Error));
        overlay.clear();
        assert!(overlay.diagnostics.is_empty());
    }

    #[test]
    fn squiggle_styles_pairs_count() {
        let mut overlay = DiagnosticOverlay::default();
        overlay.add(make_span(0, 5, DiagnosticSeverity::Error));
        overlay.add(make_span(5, 10, DiagnosticSeverity::Info));
        overlay.add(make_span(10, 15, DiagnosticSeverity::Hint));
        let pairs = overlay.squiggle_styles();
        assert_eq!(pairs.len(), 3);
        assert_eq!(pairs[0].1.color, "#FF0000");
        assert_eq!(pairs[1].1.color, "#0080FF");
        assert_eq!(pairs[2].1.color, "#808080");
    }
}
