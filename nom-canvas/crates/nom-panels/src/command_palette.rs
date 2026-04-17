#![deny(unsafe_code)]
use crate::dock::{fill_quad, focus_ring_quad};
use nom_gpui::scene::Scene;
use nom_theme::tokens;

#[derive(Debug, Clone)]
pub struct CommandPaletteItem {
    pub label: String,
    pub description: String,
    pub shortcut: Option<String>,
}

impl CommandPaletteItem {
    pub fn new(label: impl Into<String>, description: impl Into<String>) -> Self {
        Self { label: label.into(), description: description.into(), shortcut: None }
    }

    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }
}

pub struct CommandPalette {
    pub items: Vec<CommandPaletteItem>,
    pub query: String,
    pub selected_idx: usize,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self { items: vec![], query: String::new(), selected_idx: 0 }
    }

    pub fn set_query(&mut self, q: &str) {
        self.query = q.to_string();
        self.selected_idx = 0;
    }

    pub fn filtered_items(&self) -> Vec<&CommandPaletteItem> {
        if self.query.is_empty() {
            self.items.iter().collect()
        } else {
            let q = self.query.to_lowercase();
            self.items
                .iter()
                .filter(|item| {
                    item.label.to_lowercase().contains(&q)
                        || item.description.to_lowercase().contains(&q)
                })
                .collect()
        }
    }

    pub fn select_next(&mut self) {
        let count = self.filtered_items().len();
        if count == 0 { return; }
        self.selected_idx = (self.selected_idx + 1) % count;
    }

    pub fn select_prev(&mut self) {
        let count = self.filtered_items().len();
        if count == 0 { return; }
        self.selected_idx = if self.selected_idx == 0 {
            count - 1
        } else {
            self.selected_idx - 1
        };
    }

    /// Paint the command palette overlay into the shared GPU scene.
    ///
    /// Layout: centered modal with a background quad, a border quad (EDGE_MED),
    /// one row quad per filtered item (BG), and a focus ring on the selected item.
    pub fn paint_scene(&self, width: f32, height: f32, scene: &mut Scene) {
        let modal_w = (width * 0.5).min(640.0).max(320.0);
        let row_h = 32.0;
        let filtered = self.filtered_items();
        let modal_h = 48.0 + row_h * filtered.len() as f32;
        let modal_x = (width - modal_w) / 2.0;
        let modal_y = (height - modal_h) / 2.0;

        // Background fill
        scene.push_quad(fill_quad(modal_x, modal_y, modal_w, modal_h, tokens::BG));

        // Border quad using EDGE_MED as background (1px border simulation)
        scene.push_quad(fill_quad(modal_x, modal_y, modal_w, 1.0, tokens::EDGE_MED));
        scene.push_quad(fill_quad(modal_x, modal_y + modal_h - 1.0, modal_w, 1.0, tokens::EDGE_MED));
        scene.push_quad(fill_quad(modal_x, modal_y, 1.0, modal_h, tokens::EDGE_MED));
        scene.push_quad(fill_quad(modal_x + modal_w - 1.0, modal_y, 1.0, modal_h, tokens::EDGE_MED));

        // Input row
        let input_y = modal_y + 8.0;
        scene.push_quad(fill_quad(modal_x + 8.0, input_y, modal_w - 16.0, 32.0, tokens::BG));

        // Item rows
        for (i, _item) in filtered.iter().enumerate() {
            let row_y = modal_y + 48.0 + i as f32 * row_h;
            scene.push_quad(fill_quad(modal_x, row_y, modal_w, row_h, tokens::BG));

            if i == self.selected_idx {
                scene.push_quad(focus_ring_quad(modal_x + 2.0, row_y + 2.0, modal_w - 4.0, row_h - 4.0));
            }
        }
    }
}

impl Default for CommandPalette {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_palette_filter_by_query() {
        let mut palette = CommandPalette::new();
        palette.items.push(CommandPaletteItem::new("Open File", "Open a file in the editor"));
        palette.items.push(CommandPaletteItem::new("Save File", "Save current file"));
        palette.items.push(CommandPaletteItem::new("Run Tests", "Execute test suite"));

        assert_eq!(palette.filtered_items().len(), 3);

        palette.set_query("file");
        let filtered = palette.filtered_items();
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].label, "Open File");
        assert_eq!(filtered[1].label, "Save File");

        palette.set_query("xyz");
        assert_eq!(palette.filtered_items().len(), 0);
    }

    #[test]
    fn command_palette_select_next_wraps() {
        let mut palette = CommandPalette::new();
        palette.items.push(CommandPaletteItem::new("A", ""));
        palette.items.push(CommandPaletteItem::new("B", ""));
        palette.items.push(CommandPaletteItem::new("C", ""));

        assert_eq!(palette.selected_idx, 0);
        palette.select_next();
        assert_eq!(palette.selected_idx, 1);
        palette.select_next();
        assert_eq!(palette.selected_idx, 2);
        // Wraps around
        palette.select_next();
        assert_eq!(palette.selected_idx, 0);

        // select_prev wraps backward
        palette.select_prev();
        assert_eq!(palette.selected_idx, 2);
    }

    #[test]
    fn command_palette_paint_scene_emits_quads() {
        let mut palette = CommandPalette::new();
        palette.items.push(CommandPaletteItem::new("Open File", "").with_shortcut("Cmd+O"));
        palette.items.push(CommandPaletteItem::new("Save File", ""));
        palette.items.push(CommandPaletteItem::new("Run Tests", ""));

        let mut scene = Scene::new();
        palette.paint_scene(1440.0, 900.0, &mut scene);

        // background + 4 border edges + input row + 3 item rows + 1 focus ring = 10 quads minimum
        assert!(scene.quads.len() >= 10, "expected >=10 quads, got {}", scene.quads.len());

        // Background quad is the first one pushed
        let bg = &scene.quads[0];
        assert!(bg.background.is_some());
    }
}
