#![deny(unsafe_code)]
use crate::dock::{fill_quad, Dock, DockPosition};
use crate::pane::PaneGroup;
use nom_gpui::scene::Scene;
use nom_theme::tokens::{self, PANEL_RIGHT_WIDTH, SIDEBAR_W, STATUSBAR_H, TOOLBAR_H};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellMode {
    Normal,
    Insert,
}

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

    pub fn left_visible(&self) -> bool {
        self.left.is_open
    }
    pub fn right_visible(&self) -> bool {
        self.right.is_open
    }
    pub fn bottom_visible(&self) -> bool {
        self.bottom.is_open
    }

    /// Paint the shell chrome (status bar + optional mode/file strip) into the
    /// shared GPU scene.
    pub fn paint_scene(&self, width: f32, height: f32, scene: &mut Scene) {
        // Root background.
        scene.push_quad(fill_quad(0.0, 0.0, width, height, tokens::BG));

        // Status bar background strip at the bottom.
        let sb_h = self.layout.statusbar_h;
        let sb_y = height - sb_h;
        scene.push_quad(fill_quad(0.0, sb_y, width, sb_h, tokens::BG2));

        // Mode indicator accent rect (left edge of status bar).
        let accent_color = match self.mode {
            ShellMode::Normal => tokens::CTA,
            ShellMode::Insert => tokens::EDGE_HIGH,
        };
        scene.push_quad(fill_quad(0.0, sb_y, 4.0, sb_h, accent_color));

        // Active-file strip (right side of status bar).
        if self.active_file.is_some() {
            scene.push_quad(fill_quad(width - 240.0, sb_y, 240.0, sb_h, tokens::BORDER));
        }
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

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
    fn shell_paint_status_bar() {
        let mut shell = Shell::new();
        shell.active_file = Some("src/main.nom".to_string());
        let mut scene = Scene::new();
        shell.paint_scene(1440.0, 900.0, &mut scene);

        // bg + status bar + mode accent + active-file strip = 4 quads.
        assert!(
            scene.quads.len() >= 4,
            "expected >=4 quads, got {}",
            scene.quads.len()
        );
    }

    #[test]
    fn shell_paint_insert_mode_has_accent() {
        let mut shell = Shell::new();
        shell.mode = ShellMode::Insert;
        let mut scene = Scene::new();
        shell.paint_scene(800.0, 600.0, &mut scene);
        // bg + status bar + mode accent = 3 (no active_file).
        assert_eq!(scene.quads.len(), 3);
    }

    #[test]
    fn shell_new_has_no_active_file() {
        let shell = Shell::new();
        assert!(shell.active_file.is_none());
    }

    #[test]
    fn shell_left_visible_by_default() {
        let shell = Shell::new();
        assert!(shell.left_visible());
        assert!(shell.right_visible());
        assert!(shell.bottom_visible());
    }

    #[test]
    fn shell_toggle_left_dock() {
        let mut shell = Shell::new();
        assert!(shell.left_visible());
        shell.left.toggle();
        assert!(!shell.left_visible());
        shell.left.toggle();
        assert!(shell.left_visible());
    }

    #[test]
    fn shell_mode_default_is_normal() {
        let shell = Shell::new();
        assert_eq!(shell.mode, ShellMode::Normal);
    }

    #[test]
    fn shell_active_file_set() {
        let mut shell = Shell::new();
        shell.active_file = Some("main.nom".to_string());
        assert_eq!(shell.active_file.as_deref(), Some("main.nom"));
    }
}
