#![deny(unsafe_code)]
use crate::dock::{Dock, DockPosition, RenderPrimitive};
use crate::pane::PaneGroup;
use nom_theme::tokens::{SIDEBAR_W, PANEL_RIGHT_WIDTH, STATUSBAR_H, TOOLBAR_H};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellMode { Normal, Insert }

pub struct ShellLayout {
    pub toolbar_h: f32,
    pub statusbar_h: f32,
    pub left_w: f32,
    pub right_w: f32,
}

impl Default for ShellLayout {
    fn default() -> Self {
        Self {
            toolbar_h: TOOLBAR_H,
            statusbar_h: STATUSBAR_H,
            left_w: SIDEBAR_W,
            right_w: PANEL_RIGHT_WIDTH,
        }
    }
}

impl ShellLayout {
    pub fn center_w(&self, total_w: f32) -> f32 {
        (total_w - self.left_w - self.right_w).max(0.0)
    }

    pub fn center_h(&self, total_h: f32) -> f32 {
        (total_h - self.toolbar_h - self.statusbar_h).max(0.0)
    }
}

pub struct Shell {
    pub left: Dock,
    pub right: Dock,
    pub bottom: Dock,
    pub center: PaneGroup,
    pub layout: ShellLayout,
    pub mode: ShellMode,
    pub active_file: Option<String>,
}

impl Shell {
    pub fn new() -> Self {
        let mut left = Dock::new(DockPosition::Left);
        left.add_panel("file-tree", SIDEBAR_W);
        left.add_panel("search", SIDEBAR_W);

        let mut right = Dock::new(DockPosition::Right);
        right.add_panel("chat-sidebar", PANEL_RIGHT_WIDTH);
        right.add_panel("deep-think", PANEL_RIGHT_WIDTH);

        let mut bottom = Dock::new(DockPosition::Bottom);
        bottom.add_panel("terminal", 220.0);
        bottom.add_panel("diagnostics", 220.0);

        Self {
            left,
            right,
            bottom,
            center: PaneGroup::single("main-editor"),
            layout: ShellLayout::default(),
            mode: ShellMode::Normal,
            active_file: None,
        }
    }

    pub fn left_visible(&self) -> bool { self.left.is_open }
    pub fn right_visible(&self) -> bool { self.right.is_open }
    pub fn bottom_visible(&self) -> bool { self.bottom.is_open }

    pub fn render_bounds(&self, width: f32, height: f32) -> Vec<RenderPrimitive> {
        let mut out = Vec::new();

        // Status bar background at bottom
        let sb_y = height - 22.0;
        out.push(RenderPrimitive::Rect {
            x: 0.0,
            y: sb_y,
            w: width,
            h: 22.0,
            color: 0x181825,
        });

        // Mode indicator text (left side)
        let mode_text = match self.mode {
            ShellMode::Normal => "-- NORMAL --",
            ShellMode::Insert => "-- INSERT --",
        };
        out.push(RenderPrimitive::Text {
            x: 8.0,
            y: sb_y + 4.0,
            text: mode_text.to_string(),
            size: 12.0,
            color: 0xa6e3a1,
        });

        // File path text at center
        if let Some(ref path) = self.active_file {
            out.push(RenderPrimitive::Text {
                x: width / 2.0,
                y: sb_y + 4.0,
                text: path.clone(),
                size: 12.0,
                color: 0xcdd6f4,
            });
        }

        out
    }
}

impl Default for Shell { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_default_panels() {
        let shell = Shell::new();
        assert_eq!(shell.left.panel_count(), 2);
        assert_eq!(shell.right.panel_count(), 2);
        assert_eq!(shell.bottom.panel_count(), 2);
        assert_eq!(shell.center.pane_count(), 1);
    }

    #[test]
    fn shell_center_dimensions() {
        let shell = Shell::new();
        let w = shell.layout.center_w(1440.0);
        // 1440 - 248 - 320 = 872
        assert!((w - 872.0).abs() < 1.0);
        let h = shell.layout.center_h(900.0);
        // 900 - 48 - 24 = 828
        assert!((h - 828.0).abs() < 1.0);
    }

    #[test]
    fn shell_render_status_bar() {
        let mut shell = Shell::new();
        shell.active_file = Some("src/main.nom".to_string());
        let prims = shell.render_bounds(1440.0, 900.0);

        // Status bar rect at y=878 (900-22), h=22
        match &prims[0] {
            RenderPrimitive::Rect { x, y, w, h, color } => {
                assert!((x - 0.0).abs() < 0.01);
                assert!((y - 878.0).abs() < 0.01);
                assert!((w - 1440.0).abs() < 0.01);
                assert!((h - 22.0).abs() < 0.01);
                assert_eq!(*color, 0x181825);
            }
            _ => panic!("expected status bar Rect"),
        }

        // Mode indicator present with green color
        let mode_text = prims.iter().find(|p| matches!(p,
            RenderPrimitive::Text { color: 0xa6e3a1, .. }
        ));
        assert!(mode_text.is_some(), "mode indicator text missing");
        if let Some(RenderPrimitive::Text { text, .. }) = mode_text {
            assert_eq!(text, "-- NORMAL --");
        }

        // File path present with correct color
        let file_text = prims.iter().find(|p| matches!(p,
            RenderPrimitive::Text { color: 0xcdd6f4, .. }
        ));
        assert!(file_text.is_some(), "file path text missing");
        if let Some(RenderPrimitive::Text { text, .. }) = file_text {
            assert_eq!(text, "src/main.nom");
        }
    }

    #[test]
    fn shell_render_insert_mode() {
        let mut shell = Shell::new();
        shell.mode = ShellMode::Insert;
        let prims = shell.render_bounds(800.0, 600.0);
        let mode_text = prims.iter().find(|p| matches!(p,
            RenderPrimitive::Text { color: 0xa6e3a1, .. }
        ));
        assert!(mode_text.is_some());
        if let Some(RenderPrimitive::Text { text, .. }) = mode_text {
            assert_eq!(text, "-- INSERT --");
        }
    }
}
