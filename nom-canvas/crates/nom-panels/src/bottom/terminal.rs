#![deny(unsafe_code)]
use crate::dock::{fill_quad, DockPosition, Panel};
use nom_gpui::scene::Scene;
use nom_theme::tokens;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalLineKind {
    Stdout,
    Stderr,
    Command,
    Info,
}

#[derive(Debug, Clone)]
pub struct TerminalLine {
    pub text: String,
    pub kind: TerminalLineKind,
}

impl TerminalLine {
    pub fn stdout(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: TerminalLineKind::Stdout,
        }
    }
    pub fn stderr(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: TerminalLineKind::Stderr,
        }
    }
    pub fn command(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: TerminalLineKind::Command,
        }
    }
}

pub struct TerminalPanel {
    pub lines: Vec<TerminalLine>,
    pub current_input: String,
    pub max_lines: usize,
    pub cursor_row: usize,
    pub cursor_col: usize,
}

impl TerminalPanel {
    pub fn new() -> Self {
        Self {
            lines: vec![],
            current_input: String::new(),
            max_lines: 5000,
            cursor_row: 0,
            cursor_col: 0,
        }
    }

    pub fn push_line(&mut self, line: TerminalLine) {
        self.lines.push(line);
        if self.lines.len() > self.max_lines {
            self.lines.drain(0..self.lines.len() - self.max_lines);
        }
    }

    pub fn set_input(&mut self, s: impl Into<String>) {
        self.current_input = s.into();
    }

    pub fn submit_command(&mut self) -> String {
        let cmd = std::mem::take(&mut self.current_input);
        if !cmd.is_empty() {
            self.push_line(TerminalLine::command(format!("$ {}", cmd)));
        }
        cmd
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    pub fn paint_scene(&self, width: f32, height: f32, scene: &mut Scene) {
        // Panel background.
        scene.push_quad(fill_quad(0.0, 0.0, width, height, tokens::BG));

        // Severity accent strip per line (4 px wide on the left).
        for (i, line) in self.lines.iter().enumerate() {
            let color = match line.kind {
                TerminalLineKind::Stderr => tokens::EDGE_LOW,
                TerminalLineKind::Command => tokens::CTA,
                TerminalLineKind::Info => tokens::EDGE_MED,
                TerminalLineKind::Stdout => tokens::BORDER,
            };
            scene.push_quad(fill_quad(0.0, i as f32 * 16.0, 4.0, 16.0, color));
        }

        // Cursor quad.
        scene.push_quad(fill_quad(
            self.cursor_col as f32 * 8.0 + 4.0,
            self.cursor_row as f32 * 16.0,
            8.0,
            16.0,
            tokens::TEXT,
        ));
    }
}

impl Default for TerminalPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for TerminalPanel {
    fn id(&self) -> &str {
        "terminal"
    }
    fn title(&self) -> &str {
        "Terminal"
    }
    fn default_size(&self) -> f32 {
        220.0
    }
    fn position(&self) -> DockPosition {
        DockPosition::Bottom
    }
    fn activation_priority(&self) -> u32 {
        10
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_line_buffering() {
        let mut t = TerminalPanel::new();
        t.push_line(TerminalLine::stdout("hello"));
        t.set_input("cargo build");
        let cmd = t.submit_command();
        assert_eq!(cmd, "cargo build");
        assert_eq!(t.lines.len(), 2);
        assert!(t.current_input.is_empty());
    }

    #[test]
    fn terminal_new_empty() {
        let t = TerminalPanel::new();
        assert!(t.lines.is_empty());
        assert!(t.current_input.is_empty());
    }

    #[test]
    fn terminal_write() {
        let mut t = TerminalPanel::new();
        t.push_line(TerminalLine::stdout("build output"));
        assert_eq!(t.lines.len(), 1);
        assert_eq!(t.lines[0].text, "build output");
    }

    #[test]
    fn terminal_clear() {
        let mut t = TerminalPanel::new();
        t.push_line(TerminalLine::stdout("line 1"));
        t.push_line(TerminalLine::stdout("line 2"));
        t.clear();
        assert!(t.lines.is_empty());
    }

    #[test]
    fn terminal_command_history() {
        let mut t = TerminalPanel::new();
        t.set_input("cargo test");
        let cmd = t.submit_command();
        assert_eq!(cmd, "cargo test");
        // The submitted command is stored as a Command line with "$ " prefix
        assert_eq!(t.lines.len(), 1);
        assert_eq!(t.lines[0].kind, TerminalLineKind::Command);
        assert!(t.lines[0].text.contains("cargo test"));
    }

    #[test]
    fn terminal_max_lines_eviction() {
        let mut t = TerminalPanel {
            lines: vec![],
            current_input: String::new(),
            max_lines: 3,
            cursor_row: 0,
            cursor_col: 0,
        };
        for i in 0..5 {
            t.push_line(TerminalLine::stdout(format!("line {}", i)));
        }
        assert_eq!(t.lines.len(), 3);
        assert_eq!(t.lines[0].text, "line 2");
    }

    #[test]
    fn terminal_paint_scene_has_quads() {
        let mut t = TerminalPanel::new();
        t.push_line(TerminalLine::stdout("hello"));
        t.push_line(TerminalLine::stderr("error msg"));
        t.push_line(TerminalLine::command("cargo build"));
        t.cursor_row = 1;
        t.cursor_col = 2;
        let mut scene = Scene::new();
        t.paint_scene(800.0, 300.0, &mut scene);
        // bg + 3 line accents + cursor = 5 quads.
        assert_eq!(scene.quads.len(), 5);
    }
}
