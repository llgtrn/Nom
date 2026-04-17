#![deny(unsafe_code)]
use crate::dock::{DockPosition, Panel};
use crate::RenderPrimitive;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticSeverity { Hint, Info, Warning, Error }

impl DiagnosticSeverity {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Hint => "hint",
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub id: String,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub source_path: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub code: Option<String>,
}

impl Diagnostic {
    pub fn error(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self { id: id.into(), severity: DiagnosticSeverity::Error, message: message.into(), source_path: None, line: None, column: None, code: None }
    }

    pub fn warning(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self { id: id.into(), severity: DiagnosticSeverity::Warning, message: message.into(), source_path: None, line: None, column: None, code: None }
    }

    pub fn with_location(mut self, path: impl Into<String>, line: u32, col: u32) -> Self {
        self.source_path = Some(path.into());
        self.line = Some(line);
        self.column = Some(col);
        self
    }
}

pub struct DiagnosticsPanel {
    pub diagnostics: Vec<Diagnostic>,
    pub filter_severity: Option<DiagnosticSeverity>,
    pub selected: Option<usize>,
}

impl DiagnosticsPanel {
    pub fn new() -> Self { Self { diagnostics: vec![], filter_severity: None, selected: None } }

    pub fn push(&mut self, d: Diagnostic) { self.diagnostics.push(d); }

    pub fn clear(&mut self) { self.diagnostics.clear(); self.selected = None; }

    pub fn error_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.severity == DiagnosticSeverity::Error).count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.severity == DiagnosticSeverity::Warning).count()
    }

    pub fn visible(&self) -> Vec<&Diagnostic> {
        match &self.filter_severity {
            None => self.diagnostics.iter().collect(),
            Some(sev) => self.diagnostics.iter().filter(|d| &d.severity == sev).collect(),
        }
    }

    pub fn render_bounds(&self, width: f32, height: f32) -> Vec<RenderPrimitive> {
        let mut out = Vec::new();
        out.push(RenderPrimitive::Rect { x: 0.0, y: 0.0, w: width, h: height, color: 0x1e1e2e });
        for (i, diag) in self.visible().iter().enumerate() {
            let (icon, color) = match diag.severity {
                DiagnosticSeverity::Error   => ("\u{2715} ", 0xf38ba8u32),
                DiagnosticSeverity::Warning => ("\u{26a0} ", 0xf9e2afu32),
                DiagnosticSeverity::Info    => ("\u{2139} ", 0x89dcebu32),
                DiagnosticSeverity::Hint    => ("\u{2139} ", 0x89dcebu32),
            };
            let location = match (&diag.source_path, diag.line) {
                (Some(path), Some(ln)) => format!("{}:{}", path, ln),
                (Some(path), None)     => path.clone(),
                _                     => String::from("?"),
            };
            let text = format!("{icon}[{location}] {}", diag.message);
            out.push(RenderPrimitive::Text {
                x: 4.0,
                y: i as f32 * 20.0 + 2.0,
                text,
                size: 13.0,
                color,
            });
        }
        out
    }
}

impl Default for DiagnosticsPanel { fn default() -> Self { Self::new() } }

impl Panel for DiagnosticsPanel {
    fn id(&self) -> &str { "diagnostics" }
    fn title(&self) -> &str { "Diagnostics" }
    fn default_size(&self) -> f32 { 220.0 }
    fn position(&self) -> DockPosition { DockPosition::Bottom }
    fn activation_priority(&self) -> u32 { 20 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_count() {
        let mut panel = DiagnosticsPanel::new();
        panel.push(Diagnostic::error("e1", "type mismatch"));
        panel.push(Diagnostic::warning("w1", "unused variable"));
        panel.push(Diagnostic::error("e2", "missing semicolon"));
        assert_eq!(panel.error_count(), 2);
        assert_eq!(panel.warning_count(), 1);
    }

    #[test]
    fn diagnostics_filter() {
        let mut panel = DiagnosticsPanel::new();
        panel.push(Diagnostic::error("e1", "err"));
        panel.push(Diagnostic::warning("w1", "warn"));
        panel.filter_severity = Some(DiagnosticSeverity::Error);
        assert_eq!(panel.visible().len(), 1);
    }

    #[test]
    fn diagnostics_panel_render_by_severity() {
        let mut panel = DiagnosticsPanel::new();
        panel.push(
            Diagnostic::error("e1", "type mismatch")
                .with_location("src/main.rs", 10, 1),
        );
        panel.push(Diagnostic::warning("w1", "unused variable"));
        panel.push(
            Diagnostic {
                id: "i1".into(),
                severity: DiagnosticSeverity::Info,
                message: "consider this".into(),
                source_path: None,
                line: None,
                column: None,
                code: None,
            },
        );
        let prims = panel.render_bounds(800.0, 400.0);
        // first is background
        assert!(matches!(prims[0], RenderPrimitive::Rect { color: 0x1e1e2e, .. }));
        // error row: red color
        assert!(matches!(prims[1], RenderPrimitive::Text { color: 0xf38ba8, .. }));
        // warning row: yellow color
        assert!(matches!(prims[2], RenderPrimitive::Text { color: 0xf9e2af, .. }));
        // info row: cyan color
        assert!(matches!(prims[3], RenderPrimitive::Text { color: 0x89dceb, .. }));
        // error text contains file and line
        if let RenderPrimitive::Text { text, .. } = &prims[1] {
            assert!(text.contains("src/main.rs:10"), "expected location in '{text}'");
            assert!(text.contains("type mismatch"), "expected message in '{text}'");
        }
        // y positions
        if let RenderPrimitive::Text { y, .. } = &prims[1] { assert!((*y - 2.0).abs() < f32::EPSILON); }
        if let RenderPrimitive::Text { y, .. } = &prims[2] { assert!((*y - 22.0).abs() < f32::EPSILON); }
    }
}
