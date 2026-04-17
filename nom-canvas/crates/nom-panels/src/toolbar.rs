#![deny(unsafe_code)]
use crate::dock::fill_quad;
use nom_gpui::scene::Scene;
use nom_theme::tokens;

#[derive(Debug, Clone)]
pub struct ToolbarButton {
    pub label: String,
    pub action: String,
    pub active: bool,
}

impl ToolbarButton {
    pub fn new(label: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            action: action.into(),
            active: false,
        }
    }
}

pub struct Toolbar {
    pub buttons: Vec<ToolbarButton>,
    pub height: f32,
}

impl Toolbar {
    pub fn new() -> Self {
        Self {
            buttons: vec![],
            height: 48.0,
        }
    }

    pub fn add_button(&mut self, label: impl Into<String>, action: impl Into<String>) {
        self.buttons.push(ToolbarButton::new(label, action));
    }

    pub fn set_active(&mut self, action: &str) {
        for btn in &mut self.buttons {
            btn.active = btn.action == action;
        }
    }

    /// Paint the toolbar into the shared GPU scene.
    ///
    /// Layout: full-width background quad (BG), one button quad per button
    /// (active→CTA, inactive→BG), and a 1px border bottom (EDGE_LOW).
    pub fn paint_scene(&self, width: f32, _height: f32, scene: &mut Scene) {
        // Background
        scene.push_quad(fill_quad(0.0, 0.0, width, self.height, tokens::BG));

        // Button quads
        let btn_w = 40.0;
        let btn_h = 32.0;
        let btn_y = (self.height - btn_h) / 2.0;
        for (i, btn) in self.buttons.iter().enumerate() {
            let btn_x = 8.0 + i as f32 * (btn_w + 4.0);
            let color = if btn.active { tokens::CTA } else { tokens::BG };
            scene.push_quad(fill_quad(btn_x, btn_y, btn_w, btn_h, color));
        }

        // Bottom border
        scene.push_quad(fill_quad(
            0.0,
            self.height - 1.0,
            width,
            1.0,
            tokens::EDGE_LOW,
        ));
    }
}

impl Default for Toolbar {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolbar_add_button_and_count() {
        let mut toolbar = Toolbar::new();
        assert_eq!(toolbar.buttons.len(), 0);
        toolbar.add_button("Run", "run");
        toolbar.add_button("Debug", "debug");
        toolbar.add_button("Test", "test");
        assert_eq!(toolbar.buttons.len(), 3);
        assert_eq!(toolbar.height, 48.0);
    }

    #[test]
    fn toolbar_set_active_changes_state() {
        let mut toolbar = Toolbar::new();
        toolbar.add_button("Run", "run");
        toolbar.add_button("Debug", "debug");
        toolbar.add_button("Test", "test");

        // None active initially
        assert!(!toolbar.buttons[0].active);
        assert!(!toolbar.buttons[1].active);

        toolbar.set_active("debug");
        assert!(!toolbar.buttons[0].active);
        assert!(toolbar.buttons[1].active);
        assert!(!toolbar.buttons[2].active);

        // Switch active to another
        toolbar.set_active("run");
        assert!(toolbar.buttons[0].active);
        assert!(!toolbar.buttons[1].active);
    }

    #[test]
    fn toolbar_height_is_48() {
        let toolbar = Toolbar::new();
        assert_eq!(toolbar.height, 48.0);
    }

    #[test]
    fn toolbar_buttons_count() {
        let mut toolbar = Toolbar::new();
        toolbar.add_button("A", "action_a");
        toolbar.add_button("B", "action_b");
        toolbar.add_button("C", "action_c");
        assert_eq!(toolbar.buttons.len(), 3);
    }

    #[test]
    fn toolbar_active_button_toggle() {
        let mut toolbar = Toolbar::new();
        toolbar.add_button("Run", "run");
        toolbar.add_button("Stop", "stop");
        // Initially none active
        assert!(!toolbar.buttons[0].active);
        assert!(!toolbar.buttons[1].active);
        // Activate run
        toolbar.set_active("run");
        assert!(toolbar.buttons[0].active);
        assert!(!toolbar.buttons[1].active);
        // Switch to stop — run becomes inactive
        toolbar.set_active("stop");
        assert!(!toolbar.buttons[0].active);
        assert!(toolbar.buttons[1].active);
        // Activate unknown — all become inactive
        toolbar.set_active("unknown");
        assert!(!toolbar.buttons[0].active);
        assert!(!toolbar.buttons[1].active);
    }

    #[test]
    fn toolbar_button_action() {
        let btn = ToolbarButton::new("Run", "run_action");
        assert_eq!(btn.action, "run_action");
        assert_eq!(btn.label, "Run");
    }

    #[test]
    fn toolbar_button_active_state() {
        let mut toolbar = Toolbar::new();
        toolbar.add_button("Build", "build");
        toolbar.set_active("build");
        assert!(toolbar.buttons[0].active);
        // Toggle off by activating a different action
        toolbar.set_active("other");
        assert!(!toolbar.buttons[0].active);
    }

    #[test]
    fn toolbar_separator() {
        // A separator is modeled as a button with empty action string
        let mut toolbar = Toolbar::new();
        toolbar.add_button("", "");
        let sep = &toolbar.buttons[0];
        assert!(sep.action.is_empty());
    }

    #[test]
    fn toolbar_paint_scene_emits_background() {
        let mut toolbar = Toolbar::new();
        toolbar.add_button("Run", "run");
        toolbar.add_button("Debug", "debug");
        toolbar.set_active("run");

        let mut scene = Scene::new();
        toolbar.paint_scene(1440.0, 48.0, &mut scene);

        // background + 2 button quads + bottom border = 4 quads minimum
        assert!(
            scene.quads.len() >= 4,
            "expected >=4 quads, got {}",
            scene.quads.len()
        );

        // First quad is the background spanning full width
        let bg = &scene.quads[0];
        assert_eq!(bg.bounds.origin.x, nom_gpui::types::Pixels(0.0));
        assert_eq!(bg.bounds.origin.y, nom_gpui::types::Pixels(0.0));
        assert_eq!(bg.bounds.size.width, nom_gpui::types::Pixels(1440.0));
        assert_eq!(bg.bounds.size.height, nom_gpui::types::Pixels(48.0));
        assert!(bg.background.is_some());
    }
}
