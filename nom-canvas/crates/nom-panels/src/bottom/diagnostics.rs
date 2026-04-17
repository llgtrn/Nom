#![deny(unsafe_code)]
use crate::dock::{fill_quad, DockPosition, Panel};
use nom_gpui::scene::Scene;
use nom_theme::tokens;

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

    pub fn paint_scene(&self, width: f32, height: f32, scene: &mut Scene) {
        scene.push_quad(fill_quad(0.0, 0.0, width, height, tokens::BG));
        for (i, diag) in self.visible().iter().enumerate() {
            let color = match diag.severity {
                DiagnosticSeverity::Error   => tokens::EDGE_LOW,
                DiagnosticSeverity::Warning => tokens::EDGE_MED,
                DiagnosticSeverity::Info    => tokens::EDGE_HIGH,
                DiagnosticSeverity::Hint    => tokens::BORDER,
            };
            scene.push_quad(fill_quad(0.0, i as f32 * 20.0 + 2.0, 4.0, 16.0, color));
        }
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
    fn diagnostics_panel_paint_by_severity() {
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
        let mut scene = Scene::new();
        panel.paint_scene(800.0, 400.0, &mut scene);
        // bg + 3 severity accents = 4 quads.
        assert_eq!(scene.quads.len(), 4);
    }
}
