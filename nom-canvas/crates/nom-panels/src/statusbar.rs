#![deny(unsafe_code)]
use crate::dock::fill_quad;
use nom_gpui::scene::Scene;
use nom_theme::tokens;

#[derive(Debug, Clone, Default)]
pub struct StatusSlot {
    pub content: String,
}

impl StatusSlot {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }
}

pub struct StatusBar {
    pub left: StatusSlot,
    pub center: StatusSlot,
    pub right: StatusSlot,
    pub height: f32,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            left: StatusSlot::default(),
            center: StatusSlot::default(),
            right: StatusSlot::default(),
            height: 24.0,
        }
    }

    pub fn set_left(&mut self, s: &str) {
        self.left.content = s.to_string();
    }

    pub fn set_center(&mut self, s: &str) {
        self.center.content = s.to_string();
    }

    pub fn set_right(&mut self, s: &str) {
        self.right.content = s.to_string();
    }

    /// Paint the status bar into the shared GPU scene.
    ///
    /// Layout: full-width background quad (BG), then a 1px top border (EDGE_LOW).
    pub fn paint_scene(&self, width: f32, total_height: f32, scene: &mut Scene) {
        let bar_y = total_height - self.height;

        // Background
        scene.push_quad(fill_quad(0.0, bar_y, width, self.height, tokens::BG));

        // Top border
        scene.push_quad(fill_quad(0.0, bar_y, width, 1.0, tokens::EDGE_LOW));
    }
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
    fn statusbar_set_slots() {
        let mut bar = StatusBar::new();
        bar.set_left("main branch");
        bar.set_center("Ln 42, Col 7");
        bar.set_right("Rust");

        assert_eq!(bar.left.content, "main branch");
        assert_eq!(bar.center.content, "Ln 42, Col 7");
        assert_eq!(bar.right.content, "Rust");
    }

    #[test]
    fn statusbar_height_is_24() {
        let bar = StatusBar::new();
        assert_eq!(bar.height, 24.0);
    }

    #[test]
    fn statusbar_slot_set_left() {
        let mut bar = StatusBar::new();
        assert!(bar.left.content.is_empty());
        bar.set_left("main");
        assert_eq!(bar.left.content, "main");
    }

    #[test]
    fn statusbar_slot_set_right() {
        let mut bar = StatusBar::new();
        assert!(bar.right.content.is_empty());
        bar.set_right("UTF-8");
        assert_eq!(bar.right.content, "UTF-8");
    }

    #[test]
    fn statusbar_center_overwrite() {
        let mut bar = StatusBar::new();
        bar.set_center("Ln 1, Col 1");
        bar.set_center("Ln 42, Col 7");
        assert_eq!(bar.center.content, "Ln 42, Col 7");
    }

    #[test]
    fn statusbar_all_slots_independent() {
        let mut bar = StatusBar::new();
        bar.set_left("branch");
        bar.set_center("ready");
        bar.set_right("Rust");
        assert_eq!(bar.left.content, "branch");
        assert_eq!(bar.center.content, "ready");
        assert_eq!(bar.right.content, "Rust");
    }

    #[test]
    fn statusbar_paint_scene_emits_background() {
        let mut bar = StatusBar::new();
        bar.set_left("ready");
        bar.set_right("UTF-8");

        let mut scene = Scene::new();
        bar.paint_scene(1440.0, 900.0, &mut scene);

        // background + top border = 2 quads minimum
        assert!(
            scene.quads.len() >= 2,
            "expected >=2 quads, got {}",
            scene.quads.len()
        );

        // First quad is the background — positioned at bottom of the viewport
        let bg = &scene.quads[0];
        assert_eq!(bg.bounds.origin.x, nom_gpui::types::Pixels(0.0));
        // bar_y = 900.0 - 24.0 = 876.0
        assert_eq!(bg.bounds.origin.y, nom_gpui::types::Pixels(876.0));
        assert_eq!(bg.bounds.size.width, nom_gpui::types::Pixels(1440.0));
        assert_eq!(bg.bounds.size.height, nom_gpui::types::Pixels(24.0));
        assert!(bg.background.is_some());
    }
}
