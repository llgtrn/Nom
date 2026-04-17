#![deny(unsafe_code)]
use crate::dock::{DockPosition, Panel};

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
}

impl TerminalPanel {
    pub fn new() -> Self { Self { lines: vec![], current_input: String::new(), max_lines: 5000 } }

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
        let mut t = TerminalPanel { lines: vec![], current_input: String::new(), max_lines: 3 };
        for i in 0..5 {
            t.push_line(TerminalLine::stdout(format!("line {}", i)));
        }
        assert_eq!(t.lines.len(), 3);
        assert_eq!(t.lines[0].text, "line 2");
    }
}
