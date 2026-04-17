#![deny(unsafe_code)]
use crate::dock::{fill_quad, focus_ring_quad, DockPosition, Panel};
use nom_gpui::scene::Scene;
use nom_theme::tokens;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryKind {
    pub name: String,
    pub description: String,
    pub entry_count: usize,
}

pub struct LibraryPanel {
    pub kinds: Vec<LibraryKind>,
    pub selected_kind: Option<String>,
}

impl LibraryPanel {
    pub fn new() -> Self {
        Self { kinds: vec![], selected_kind: None }
    }

    /// Populate from a slice of `(name, description, entry_count)` tuples.
    pub fn load_kinds(&mut self, kinds: &[(&str, &str, usize)]) {
        self.kinds = kinds
            .iter()
            .map(|(name, description, entry_count)| LibraryKind {
                name: name.to_string(),
                description: description.to_string(),
                entry_count: *entry_count,
            })
            .collect();
    }

    pub fn select_kind(&mut self, name: &str) {
        if self.kinds.iter().any(|k| k.name == name) {
            self.selected_kind = Some(name.to_string());
        }
    }

    pub fn kind_count(&self) -> usize {
        self.kinds.len()
    }

    /// Paint a header background followed by one quad per kind.
    /// The selected kind receives an additional `focus_ring_quad` (CTA border).
    pub fn paint_scene(&self, width: f32, _height: f32, scene: &mut Scene) {
        // Header background.
        scene.push_quad(fill_quad(0.0, 0.0, width, 28.0, tokens::BG2));

        for (i, kind) in self.kinds.iter().enumerate() {
            let y = 28.0 + i as f32 * 28.0;
            scene.push_quad(fill_quad(0.0, y, width, 28.0, tokens::BG));
            // Bottom separator.
            scene.push_quad(fill_quad(0.0, y + 27.0, width, 1.0, tokens::EDGE_LOW));
            // Selected kind gets a focus ring.
            if self.selected_kind.as_deref() == Some(&kind.name) {
                scene.push_quad(focus_ring_quad(0.0, y, width, 28.0));
            }
        }
    }
}

impl Default for LibraryPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for LibraryPanel {
    fn id(&self) -> &str { "library" }
    fn title(&self) -> &str { "Library" }
    fn default_size(&self) -> f32 { 248.0 }
    fn position(&self) -> DockPosition { DockPosition::Left }
    fn activation_priority(&self) -> u32 { 20 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom_gpui::scene::Scene;

    const SAMPLE_KINDS: &[(&str, &str, usize)] = &[
        ("Function", "Callable units of work", 42),
        ("Concept", "Abstract semantic units", 17),
        ("Entity", "Concrete named objects", 8),
    ];

    #[test]
    fn library_panel_load_kinds() {
        let mut panel = LibraryPanel::new();
        panel.load_kinds(SAMPLE_KINDS);
        assert_eq!(panel.kind_count(), 3);
        assert_eq!(panel.kinds[0].name, "Function");
        assert_eq!(panel.kinds[1].description, "Abstract semantic units");
        assert_eq!(panel.kinds[2].entry_count, 8);
    }

    #[test]
    fn library_panel_select_kind() {
        let mut panel = LibraryPanel::new();
        panel.load_kinds(SAMPLE_KINDS);
        assert!(panel.selected_kind.is_none());

        panel.select_kind("Concept");
        assert_eq!(panel.selected_kind.as_deref(), Some("Concept"));

        // Selecting a non-existent name is a no-op.
        panel.select_kind("NonExistent");
        assert_eq!(panel.selected_kind.as_deref(), Some("Concept"));
    }

    #[test]
    fn library_kind_count_empty() {
        let panel = LibraryPanel::new();
        assert_eq!(panel.kind_count(), 0);
        assert!(panel.selected_kind.is_none());
    }

    #[test]
    fn library_load_kinds_populates() {
        let mut panel = LibraryPanel::new();
        panel.load_kinds(SAMPLE_KINDS);
        assert_eq!(panel.kind_count(), 3);
        assert_eq!(panel.kinds[0].name, "Function");
        assert_eq!(panel.kinds[1].name, "Concept");
        assert_eq!(panel.kinds[2].name, "Entity");
    }

    #[test]
    fn library_select_kind_sets_selected() {
        let mut panel = LibraryPanel::new();
        panel.load_kinds(SAMPLE_KINDS);
        panel.select_kind("Function");
        assert_eq!(panel.selected_kind.as_deref(), Some("Function"));
        // Selecting another replaces the selection
        panel.select_kind("Entity");
        assert_eq!(panel.selected_kind.as_deref(), Some("Entity"));
    }

    #[test]
    fn library_panel_paint_scene_quads() {
        let mut panel = LibraryPanel::new();
        panel.load_kinds(SAMPLE_KINDS);
        panel.select_kind("Function");

        let mut scene = Scene::new();
        panel.paint_scene(248.0, 500.0, &mut scene);

        // header = 1 quad
        // per kind: bg + border = 2, selected adds 1 focus ring
        // "Function" selected: 3, "Concept": 2, "Entity": 2  → total: 1 + 3 + 2 + 2 = 8
        assert_eq!(scene.quads.len(), 8);
    }
}
