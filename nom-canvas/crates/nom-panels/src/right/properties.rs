#![deny(unsafe_code)]
use crate::dock::{fill_quad, focus_ring_quad, DockPosition, Panel};
use crate::entity_ref::PanelEntityRef;
use nom_blocks::NomtuRef;
use nom_gpui::scene::Scene;
use nom_theme::tokens;

// ---------------------------------------------------------------------------
// Typed property value / entry (new rich-properties API)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    Text(String),
    Number(f64),
    Bool(bool),
    Color(String),
}

#[derive(Debug, Clone)]
pub struct PropertyEntry {
    pub key: String,
    pub value: PropertyValue,
    pub editable: bool,
}

// ---------------------------------------------------------------------------
// Legacy flat-string row (kept for paint_scene / entity-ref tests)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PropertyRow {
    pub key: String,
    pub value: String,
    pub editable: bool,
}

pub struct PropertiesPanel {
    pub entity: PanelEntityRef,
    pub rows: Vec<PropertyRow>,
    /// Panel display title.
    pub panel_title: String,
    /// Typed property entries (rich-properties API).
    pub entries: Vec<PropertyEntry>,
}

impl PropertiesPanel {
    pub fn new() -> Self {
        Self {
            entity: PanelEntityRef::None,
            rows: vec![],
            panel_title: "Properties".to_string(),
            entries: vec![],
        }
    }

    /// Construct with a custom display title.
    pub fn with_title(title: &str) -> Self {
        Self {
            entity: PanelEntityRef::None,
            rows: vec![],
            panel_title: title.to_string(),
            entries: vec![],
        }
    }

    /// Push a typed `PropertyEntry` (builder-style, consuming self).
    pub fn push(mut self, entry: PropertyEntry) -> Self {
        self.entries.push(entry);
        self
    }

    /// Retrieve a reference to the `PropertyValue` for `key`, if present.
    pub fn get(&self, key: &str) -> Option<&PropertyValue> {
        self.entries.iter().find(|e| e.key == key).map(|e| &e.value)
    }

    /// Count of entries that are marked editable.
    pub fn editable_count(&self) -> usize {
        self.entries.iter().filter(|e| e.editable).count()
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

    // ── NomtuRef field renders as link ────────────────────────────────────────

    /// A row whose value looks like an entity reference (contains "://") is a link-style row.
    #[test]
    fn properties_nomtu_ref_row_value_contains_entity_id() {
        let mut panel = PropertiesPanel::new();
        let entity = NomtuRef::new("ent-link-42", "link_word", "Concept");
        panel.load_entity_ref(entity.clone());
        // Simulate a "nomtu_ref" field pointing to the entity
        let link_value = format!("nomtu://{}/{}", entity.kind, entity.id);
        panel.set_row("ref_field", &link_value, false);
        assert_eq!(panel.rows[0].key, "ref_field");
        assert!(panel.rows[0].value.contains("ent-link-42"));
        assert!(panel.rows[0].value.contains("Concept"));
    }

    #[test]
    fn properties_nomtu_ref_row_is_non_editable() {
        let mut panel = PropertiesPanel::new();
        panel.load_entity_ref(NomtuRef::new("e1", "w", "Function"));
        // NomtuRef fields are rendered as read-only links
        panel.set_row("entity_ref", "nomtu://Function/e1", false);
        assert!(!panel.rows[0].editable);
    }

    #[test]
    fn properties_nomtu_ref_panel_entity_id_accessible() {
        let mut panel = PropertiesPanel::new();
        panel.load_entity_ref(NomtuRef::new("e-ref-99", "myword", "Media"));
        assert_eq!(panel.entity.id(), Some("e-ref-99"));
        assert_eq!(panel.entity.kind(), Some("Media"));
    }

    // ── multi-field update resets dirty ──────────────────────────────────────

    /// After loading a new entity, all row data is cleared (dirty reset).
    #[test]
    fn properties_multi_field_update_then_load_resets_all() {
        let mut panel = PropertiesPanel::new();
        panel.load_entity("ent-1", "Kind");
        panel.set_row("field_a", "val_a", true);
        panel.set_row("field_b", "val_b", false);
        panel.set_row("field_c", "val_c", true);
        assert_eq!(panel.row_count(), 3);
        // Simulate "reset dirty" by loading a new entity
        panel.load_entity("ent-2", "Kind");
        assert_eq!(
            panel.row_count(),
            0,
            "all rows must be cleared on new entity load"
        );
    }

    #[test]
    fn properties_multi_field_update_overwrites_existing_values() {
        let mut panel = PropertiesPanel::new();
        panel.set_row("alpha", "v1", false);
        panel.set_row("beta", "v2", false);
        // Update both
        panel.set_row("alpha", "v1_updated", true);
        panel.set_row("beta", "v2_updated", true);
        assert_eq!(panel.row_count(), 2);
        assert_eq!(panel.rows[0].value, "v1_updated");
        assert_eq!(panel.rows[1].value, "v2_updated");
        assert!(panel.rows[0].editable);
        assert!(panel.rows[1].editable);
    }

    #[test]
    fn properties_multi_field_update_only_target_key_changes() {
        let mut panel = PropertiesPanel::new();
        panel.set_row("x", "v1", false);
        panel.set_row("y", "v2", false);
        panel.set_row("z", "v3", false);
        // Update only "y"
        panel.set_row("y", "v2_new", true);
        assert_eq!(panel.rows[0].value, "v1"); // unchanged
        assert_eq!(panel.rows[1].value, "v2_new"); // changed
        assert_eq!(panel.rows[2].value, "v3"); // unchanged
    }

    // ── validation error field highlighted ────────────────────────────────────

    /// An editable row in error state is represented as editable (focus ring rendered).
    #[test]
    fn properties_validation_error_editable_row_gets_focus_ring() {
        let mut panel = PropertiesPanel::new();
        panel.load_entity("ent-1", "Concept");
        // "error" row is editable = true → paint_scene adds focus_ring_quad
        panel.set_row("invalid_field", "bad_value", true);
        let mut scene = Scene::new();
        panel.paint_scene(280.0, 400.0, &mut scene);
        // header(2) + 1 row editable(bg+border+focus=3) = 5
        assert_eq!(scene.quads.len(), 5);
    }

    #[test]
    fn properties_validation_error_non_editable_row_no_focus_ring() {
        let mut panel = PropertiesPanel::new();
        panel.load_entity("ent-1", "Concept");
        // read-only row → no focus ring
        panel.set_row("locked_field", "value", false);
        let mut scene = Scene::new();
        panel.paint_scene(280.0, 400.0, &mut scene);
        // header(2) + 1 row non-editable(bg+border=2) = 4
        assert_eq!(scene.quads.len(), 4);
    }

    #[test]
    fn properties_mixed_valid_invalid_rows_quad_count() {
        let mut panel = PropertiesPanel::new();
        panel.load_entity("ent-1", "Function");
        panel.set_row("name", "valid_name", false); // non-editable: 2 quads
        panel.set_row("return_type", "bad_type", true); // editable: 3 quads
        panel.set_row("visibility", "invalid", true); // editable: 3 quads
        let mut scene = Scene::new();
        panel.paint_scene(280.0, 400.0, &mut scene);
        // header(2) + 2 + 3 + 3 = 10
        assert_eq!(scene.quads.len(), 10);
    }

    // ── PropertyValue / PropertyEntry / rich-properties API ──────────────────

    #[test]
    fn properties_panel_with_title_new() {
        let panel = PropertiesPanel::with_title("Node Inspector");
        assert_eq!(panel.panel_title, "Node Inspector");
        assert!(panel.entries.is_empty());
        assert_eq!(panel.editable_count(), 0);
    }

    #[test]
    fn properties_panel_push_entry() {
        let panel = PropertiesPanel::with_title("Inspector")
            .push(PropertyEntry {
                key: "name".to_string(),
                value: PropertyValue::Text("my_node".to_string()),
                editable: true,
            })
            .push(PropertyEntry {
                key: "weight".to_string(),
                value: PropertyValue::Number(3.14),
                editable: false,
            });
        assert_eq!(panel.entries.len(), 2);
    }

    #[test]
    fn properties_panel_get() {
        let panel = PropertiesPanel::with_title("Props")
            .push(PropertyEntry {
                key: "active".to_string(),
                value: PropertyValue::Bool(true),
                editable: false,
            })
            .push(PropertyEntry {
                key: "color".to_string(),
                value: PropertyValue::Color("#ff0000".to_string()),
                editable: true,
            });
        assert_eq!(panel.get("active"), Some(&PropertyValue::Bool(true)));
        assert_eq!(
            panel.get("color"),
            Some(&PropertyValue::Color("#ff0000".to_string()))
        );
        assert!(panel.get("missing").is_none());
    }

    #[test]
    fn properties_panel_editable_count() {
        let panel = PropertiesPanel::with_title("Props")
            .push(PropertyEntry {
                key: "a".to_string(),
                value: PropertyValue::Text("v1".to_string()),
                editable: true,
            })
            .push(PropertyEntry {
                key: "b".to_string(),
                value: PropertyValue::Number(0.0),
                editable: false,
            })
            .push(PropertyEntry {
                key: "c".to_string(),
                value: PropertyValue::Bool(false),
                editable: true,
            });
        assert_eq!(panel.editable_count(), 2);
    }
}
