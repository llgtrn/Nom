#![deny(unsafe_code)]
use crate::dock::{fill_quad, focus_ring_quad, DockPosition, Panel};
use crate::entity_ref::PanelEntityRef;
use nom_blocks::NomtuRef;
use nom_gpui::scene::Scene;
use nom_theme::tokens;

#[derive(Debug, Clone)]
pub struct PropertyRow {
    pub key: String,
    pub value: String,
    pub editable: bool,
}

pub struct PropertiesPanel {
    pub entity: PanelEntityRef,
    pub rows: Vec<PropertyRow>,
}

impl PropertiesPanel {
    pub fn new() -> Self {
        Self {
            entity: PanelEntityRef::None,
            rows: vec![],
        }
    }

    pub fn load_entity(&mut self, id: &str, kind: &str) {
        self.load_entity_ref(NomtuRef::new(id, id, kind));
    }

    pub fn load_entity_ref(&mut self, entity: NomtuRef) {
        self.entity = PanelEntityRef::nomtu(entity);
        self.rows.clear();
    }

    pub fn set_row(&mut self, key: impl Into<String>, value: impl Into<String>, editable: bool) {
        let key = key.into();
        if let Some(row) = self.rows.iter_mut().find(|r| r.key == key) {
            row.value = value.into();
            row.editable = editable;
        } else {
            self.rows.push(PropertyRow {
                key,
                value: value.into(),
                editable,
            });
        }
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Paint a header quad followed by one quad per row.
    /// Editable rows get a CTA border via `focus_ring_quad`; all rows get an
    /// `EDGE_LOW` background fill.
    pub fn paint_scene(&self, width: f32, _height: f32, scene: &mut Scene) {
        // Header background.
        scene.push_quad(fill_quad(0.0, 0.0, width, 28.0, tokens::BG2));
        // Header bottom border.
        scene.push_quad(fill_quad(0.0, 27.0, width, 1.0, tokens::EDGE_LOW));

        for (i, row) in self.rows.iter().enumerate() {
            let y = 28.0 + i as f32 * 28.0;
            // Row background.
            scene.push_quad(fill_quad(0.0, y, width, 28.0, tokens::BG));
            // Bottom border.
            scene.push_quad(fill_quad(0.0, y + 27.0, width, 1.0, tokens::EDGE_LOW));
            // Editable rows get a focus/CTA ring.
            if row.editable {
                scene.push_quad(focus_ring_quad(0.0, y, width, 28.0));
            }
        }
    }
}

impl Default for PropertiesPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for PropertiesPanel {
    fn id(&self) -> &str {
        "properties"
    }
    fn title(&self) -> &str {
        "Properties"
    }
    fn default_size(&self) -> f32 {
        280.0
    }
    fn position(&self) -> DockPosition {
        DockPosition::Right
    }
    fn activation_priority(&self) -> u32 {
        20
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom_gpui::scene::Scene;

    #[test]
    fn properties_panel_load_entity() {
        let mut panel = PropertiesPanel::new();
        assert_eq!(panel.entity.id(), None);
        panel.set_row("name", "old", false);
        panel.load_entity("ent-42", "Function");
        assert_eq!(panel.entity.id(), Some("ent-42"));
        assert_eq!(panel.entity.kind(), Some("Function"));
        // rows cleared on load
        assert_eq!(panel.row_count(), 0);
    }

    #[test]
    fn properties_panel_set_row() {
        let mut panel = PropertiesPanel::new();
        panel.set_row("visibility", "public", false);
        panel.set_row("return_type", "i32", true);
        assert_eq!(panel.row_count(), 2);

        // Update existing row.
        panel.set_row("visibility", "private", true);
        assert_eq!(panel.row_count(), 2);
        assert_eq!(panel.rows[0].value, "private");
        assert!(panel.rows[0].editable);
        assert!(panel.rows[1].editable);
    }

    #[test]
    fn properties_row_count_empty() {
        let panel = PropertiesPanel::new();
        assert_eq!(panel.row_count(), 0);
    }

    #[test]
    fn properties_load_entity_populates() {
        let mut panel = PropertiesPanel::new();
        panel.load_entity("ent-99", "Concept");
        panel.set_row("name", "my_concept", true);
        panel.set_row("visibility", "public", false);
        assert_eq!(panel.row_count(), 2);
        assert_eq!(panel.entity.id(), Some("ent-99"));
        assert_eq!(panel.entity.kind(), Some("Concept"));
    }

    #[test]
    fn properties_panel_accepts_full_nomtu_ref() {
        let mut panel = PropertiesPanel::new();
        panel.load_entity_ref(NomtuRef::new("id-7", "word", "media"));
        assert_eq!(panel.entity.id(), Some("id-7"));
        assert_eq!(panel.entity.kind(), Some("media"));
    }

    #[test]
    fn properties_row_key_preserved() {
        let mut panel = PropertiesPanel::new();
        panel.set_row("return_type", "i32", true);
        assert_eq!(panel.rows[0].key, "return_type");
        assert_eq!(panel.rows[0].value, "i32");
        assert!(panel.rows[0].editable);
    }

    #[test]
    fn properties_panel_paint_scene_emits_per_row_quads() {
        let mut panel = PropertiesPanel::new();
        panel.load_entity("ent-1", "Concept");
        panel.set_row("name", "my_concept", true);
        panel.set_row("visibility", "public", false);

        let mut scene = Scene::new();
        panel.paint_scene(280.0, 400.0, &mut scene);

        // header bg + header border = 2
        // per row: bg + border = 2, plus 1 focus ring if editable
        // row 0 editable: 3 quads; row 1 non-editable: 2 quads
        // total: 2 + 3 + 2 = 7
        assert_eq!(scene.quads.len(), 7);
    }

    #[test]
    fn properties_panel_paint_scene_no_rows() {
        let mut panel = PropertiesPanel::new();
        panel.load_entity("ent-1", "Concept");
        let mut scene = Scene::new();
        panel.paint_scene(280.0, 400.0, &mut scene);
        // header bg + header border = 2
        assert_eq!(scene.quads.len(), 2);
    }

    #[test]
    fn properties_panel_paint_scene_all_non_editable() {
        let mut panel = PropertiesPanel::new();
        panel.load_entity("ent-1", "Concept");
        panel.set_row("a", "v1", false);
        panel.set_row("b", "v2", false);
        let mut scene = Scene::new();
        panel.paint_scene(280.0, 400.0, &mut scene);
        // header(2) + 2 rows × (bg+border=2) = 2 + 4 = 6
        assert_eq!(scene.quads.len(), 6);
    }

    #[test]
    fn properties_panel_paint_scene_all_editable() {
        let mut panel = PropertiesPanel::new();
        panel.load_entity("ent-1", "Concept");
        panel.set_row("a", "v1", true);
        panel.set_row("b", "v2", true);
        let mut scene = Scene::new();
        panel.paint_scene(280.0, 400.0, &mut scene);
        // header(2) + 2 rows × (bg+border+focus=3) = 2 + 6 = 8
        assert_eq!(scene.quads.len(), 8);
    }

    #[test]
    fn properties_row_update_keeps_count() {
        let mut panel = PropertiesPanel::new();
        panel.set_row("key1", "val1", false);
        panel.set_row("key2", "val2", true);
        assert_eq!(panel.row_count(), 2);
        // update existing
        panel.set_row("key1", "new_val", true);
        assert_eq!(panel.row_count(), 2);
        assert_eq!(panel.rows[0].value, "new_val");
        assert!(panel.rows[0].editable);
    }

    #[test]
    fn properties_load_entity_clears_rows() {
        let mut panel = PropertiesPanel::new();
        panel.set_row("x", "y", false);
        panel.set_row("a", "b", true);
        assert_eq!(panel.row_count(), 2);
        panel.load_entity("new-id", "NewKind");
        assert_eq!(panel.row_count(), 0);
        assert_eq!(panel.entity.id(), Some("new-id"));
    }

    #[test]
    fn properties_panel_id_and_title() {
        let panel = PropertiesPanel::new();
        assert_eq!(panel.id(), "properties");
        assert_eq!(panel.title(), "Properties");
        assert_eq!(panel.default_size(), 280.0);
    }

    #[test]
    fn properties_panel_position_is_right() {
        let panel = PropertiesPanel::new();
        assert_eq!(panel.position(), crate::dock::DockPosition::Right);
    }

    #[test]
    fn properties_panel_activation_priority() {
        let panel = PropertiesPanel::new();
        assert_eq!(panel.activation_priority(), 20);
    }

    #[test]
    fn properties_panel_default_is_empty() {
        let panel = PropertiesPanel::default();
        assert_eq!(panel.row_count(), 0);
        assert_eq!(panel.entity, crate::entity_ref::PanelEntityRef::None);
    }

    #[test]
    fn properties_row_readonly_detection() {
        let mut panel = PropertiesPanel::new();
        panel.set_row("locked", "value", false);
        panel.set_row("editable", "value", true);
        assert!(!panel.rows[0].editable);
        assert!(panel.rows[1].editable);
    }

    #[test]
    fn properties_multiple_loads_reset_each_time() {
        let mut panel = PropertiesPanel::new();
        panel.load_entity("ent-1", "A");
        panel.set_row("k", "v", false);
        panel.load_entity("ent-2", "B");
        assert_eq!(panel.row_count(), 0);
        assert_eq!(panel.entity.id(), Some("ent-2"));
        assert_eq!(panel.entity.kind(), Some("B"));
    }
}
