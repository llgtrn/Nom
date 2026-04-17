#![deny(unsafe_code)]
use crate::dock::{DockPosition, Panel};
use crate::RenderPrimitive;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalLineKind { Stdout, Stderr, Command, Info }

#[derive(Debug, Clone)]
pub struct TerminalLine {
    pub text: String,
    pub kind: TerminalLineKind,
}

impl TerminalLine {
    pub fn stdout(text: impl Into<String>) -> Self { Self { text: text.into(), kind: TerminalLineKind::Stdout } }
    pub fn stderr(text: impl Into<String>) -> Self { Self { text: text.into(), kind: TerminalLineKind::Stderr } }
    pub fn command(text: impl Into<String>) -> Self { Self { text: text.into(), kind: TerminalLineKind::Command } }
}

pub struct TerminalPanel {
    pub lines: Vec<TerminalLine>,
    pub current_input: String,
    pub max_lines: usize,
    pub cursor_row: usize,
    pub cursor_col: usize,
}

impl TerminalPanel {
    pub fn new() -> Self { Self { lines: vec![], current_input: String::new(), max_lines: 5000, cursor_row: 0, cursor_col: 0 } }

    pub fn push_line(&mut self, line: TerminalLine) {
        self.lines.push(line);
        if self.lines.len() > self.max_lines {
            self.lines.drain(0..self.lines.len() - self.max_lines);
        }
    }

    pub fn set_input(&mut self, s: impl Into<String>) { self.current_input = s.into(); }

    pub fn submit_command(&mut self) -> String {
        let cmd = std::mem::take(&mut self.current_input);
        if !cmd.is_empty() {
            self.push_line(TerminalLine::command(format!("$ {}", cmd)));
        }
        cmd
    }

    pub fn clear(&mut self) { self.lines.clear(); }

    pub fn render_bounds(&self, width: f32, height: f32) -> Vec<RenderPrimitive> {
        let mut out = Vec::new();
        out.push(RenderPrimitive::Rect { x: 0.0, y: 0.0, w: width, h: height, color: 0x181825 });
        for (i, line) in self.lines.iter().enumerate() {
            let color = match line.kind {
                TerminalLineKind::Stderr  => 0xf38ba8,
                TerminalLineKind::Command => 0xa6e3a1,
                _                        => 0xcdd6f4,
            };
            out.push(RenderPrimitive::Text {
                x: 4.0,
                y: i as f32 * 16.0,
                text: line.text.clone(),
                size: 13.0,
                color,
            });
        }
        out.push(RenderPrimitive::Rect {
            x: self.cursor_col as f32 * 8.0 + 4.0,
            y: self.cursor_row as f32 * 16.0,
            w: 8.0,
            h: 16.0,
            color: 0xcdd6f4,
        });
        out
    }
}

impl Default for TerminalPanel { fn default() -> Self { Self::new() } }

impl Panel for TerminalPanel {
    fn id(&self) -> &str { "terminal" }
    fn title(&self) -> &str { "Terminal" }
    fn default_size(&self) -> f32 { 220.0 }
    fn position(&self) -> DockPosition { DockPosition::Bottom }
    fn activation_priority(&self) -> u32 { 10 }
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
    fn terminal_max_lines_eviction() {
        let mut t = TerminalPanel { lines: vec![], current_input: String::new(), max_lines: 3, cursor_row: 0, cursor_col: 0 };
        for i in 0..5 {
            t.push_line(TerminalLine::stdout(format!("line {}", i)));
        }
        assert_eq!(t.lines.len(), 3);
        assert_eq!(t.lines[0].text, "line 2");
    }

    #[test]
    fn terminal_panel_render_lines() {
        let mut t = TerminalPanel::new();
        t.push_line(TerminalLine::stdout("hello"));
        t.push_line(TerminalLine::stderr("error msg"));
        t.push_line(TerminalLine::command("cargo build"));
        t.cursor_row = 1;
        t.cursor_col = 2;
        let prims = t.render_bounds(800.0, 300.0);
        // first primitive is background rect
        assert!(matches!(prims[0], RenderPrimitive::Rect { color: 0x181825, .. }));
        // stdout line: default output color
        assert!(matches!(prims[1], RenderPrimitive::Text { color: 0xcdd6f4, .. }));
        // stderr line: error color
        assert!(matches!(prims[2], RenderPrimitive::Text { color: 0xf38ba8, .. }));
        // command line: green
        assert!(matches!(prims[3], RenderPrimitive::Text { color: 0xa6e3a1, .. }));
        // cursor rect
        let cursor = &prims[4];
        assert!(matches!(cursor, RenderPrimitive::Rect { x, y, w: 8.0, h: 16.0, color: 0xcdd6f4 }
            if (*x - (2.0 * 8.0 + 4.0)).abs() < f32::EPSILON && (*y - 16.0).abs() < f32::EPSILON));
    }
}
