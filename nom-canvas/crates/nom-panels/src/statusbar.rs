//! Status bar panel — compile status, cursor position, git branch, diagnostics.

/// Compiler/build pipeline status.
#[derive(Debug, Clone, PartialEq)]
pub enum CompileStatus {
    Idle,
    Compiling,
    Ok,
    Error(String),
}

impl Default for CompileStatus {
    fn default() -> Self {
        CompileStatus::Idle
    }
}

/// Status bar panel state.
#[derive(Debug)]
pub struct StatusBar {
    pub height_px: f32,
    pub compile_status: CompileStatus,
    pub cursor_line: u32,
    pub cursor_col: u32,
    pub git_branch: Option<String>,
    pub diagnostics_count: u32,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            height_px: 24.0,
            compile_status: CompileStatus::Idle,
            cursor_line: 1,
            cursor_col: 1,
            git_branch: None,
            diagnostics_count: 0,
        }
    }

    /// Update the cursor position (1-based).
    pub fn set_cursor(&mut self, line: u32, col: u32) {
        self.cursor_line = line;
        self.cursor_col = col;
    }

    /// Stub paint method — rendering lives in the GPU layer.
    pub fn paint(&self) {}
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_height_is_24() {
        let sb = StatusBar::new();
        assert_eq!(sb.height_px, 24.0);
    }

    #[test]
    fn default_status_is_idle() {
        let sb = StatusBar::new();
        assert_eq!(sb.compile_status, CompileStatus::Idle);
    }

    #[test]
    fn set_cursor_updates_position() {
        let mut sb = StatusBar::new();
        sb.set_cursor(10, 5);
        assert_eq!(sb.cursor_line, 10);
        assert_eq!(sb.cursor_col, 5);
    }
}
