#![deny(unsafe_code)]
use crate::dock::{Dock, DockPosition};
use crate::pane::PaneGroup;
use nom_theme::tokens::{SIDEBAR_W, PANEL_RIGHT_WIDTH, STATUSBAR_H, TOOLBAR_H};

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
        }
    }

    pub fn left_visible(&self) -> bool { self.left.is_open }
    pub fn right_visible(&self) -> bool { self.right.is_open }
    pub fn bottom_visible(&self) -> bool { self.bottom.is_open }
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
}
