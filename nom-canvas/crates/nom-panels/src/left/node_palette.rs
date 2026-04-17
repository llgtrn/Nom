#![deny(unsafe_code)]
use crate::dock::{fill_quad, DockPosition, Panel};
use nom_gpui::scene::Scene;
use nom_theme::tokens;

/// A single entry in the node palette, populated from grammar.kinds DB rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteEntry {
    pub kind_name: String,
    pub display_name: String,
    pub description: String,
}

/// DB-driven node palette for Graph mode (spec §8).
///
/// Entries are loaded from a caller-supplied slice that simulates
/// `SELECT kind_name, display_name, description FROM grammar.kinds`.
pub struct NodePalette {
    pub entries: Vec<PaletteEntry>,
}

impl NodePalette {
    pub fn new() -> Self {
        Self { entries: vec![] }
    }

    /// Populate the palette from a DB-sourced slice of
    /// `(kind_name, display_name, description)` tuples.
    pub fn load_from_kinds(kinds: &[(&str, &str, &str)]) -> Self {
        let entries = kinds
            .iter()
            .map(|(kind_name, display_name, description)| PaletteEntry {
                kind_name: kind_name.to_string(),
                display_name: display_name.to_string(),
                description: description.to_string(),
            })
            .collect();
        Self { entries }
    }

    /// Substring search across `kind_name` and `display_name`.
    pub fn search<'a>(&'a self, query: &str) -> Vec<&'a PaletteEntry> {
        if query.is_empty() {
            return self.entries.iter().collect();
        }
        let q = query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| {
                e.kind_name.to_lowercase().contains(&q)
                    || e.display_name.to_lowercase().contains(&q)
            })
            .collect()
    }

    /// Total number of loaded entries.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Paint one stacked row per palette entry into the GPU scene.
    ///
    /// Each row is 24 px tall with a `tokens::BG` fill and a 1px
    /// `tokens::EDGE_LOW` border on the bottom edge.
    pub fn paint_scene(&self, width: f32, scene: &mut Scene) {
        for (i, _entry) in self.entries.iter().enumerate() {
            let y = i as f32 * 24.0;
            // Row background.
            scene.push_quad(fill_quad(0.0, y, width, 24.0, tokens::BG));
            // Bottom border using EDGE_LOW.
            // Render as a 1px-tall filled strip at the bottom of the row.
            scene.push_quad(fill_quad(0.0, y + 23.0, width, 1.0, tokens::EDGE_LOW));
        }
    }
}

impl Default for NodePalette {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for NodePalette {
    fn id(&self) -> &str {
        "node-palette"
    }
    fn title(&self) -> &str {
        "Node Palette"
    }
    fn default_size(&self) -> f32 {
        248.0
    }
    fn position(&self) -> DockPosition {
        DockPosition::Left
    }
    fn activation_priority(&self) -> u32 {
        30
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom_gpui::scene::Scene;
    use nom_gpui::types::Pixels;

    const SAMPLE_KINDS: &[(&str, &str, &str)] = &[
        ("Function", "Function", "A callable unit of work"),
        ("Concept", "Concept", "An abstract semantic unit"),
        ("Entity", "Entity", "A concrete named object"),
    ];

    #[test]
    fn node_palette_load_from_kinds() {
        let palette = NodePalette::load_from_kinds(SAMPLE_KINDS);
        assert_eq!(palette.entry_count(), 3);
        assert_eq!(palette.entries[0].kind_name, "Function");
        assert_eq!(palette.entries[1].display_name, "Concept");
        assert_eq!(palette.entries[2].description, "A concrete named object");
    }

    #[test]
    fn node_palette_search_substring() {
        let palette = NodePalette::load_from_kinds(SAMPLE_KINDS);

        // "unc" matches "Function"
        let results = palette.search("unc");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind_name, "Function");

        // "on" matches "Function", "Concept" (both contain "on" case-insensitively)
        let results = palette.search("on");
        assert!(results.len() >= 2);

        // Empty query returns all entries
        let all = palette.search("");
        assert_eq!(all.len(), 3);

        // No match
        let none = palette.search("zzz");
        assert!(none.is_empty());
    }

    #[test]
    fn node_palette_paint_scene_emits_two_quads_per_entry() {
        let palette = NodePalette::load_from_kinds(SAMPLE_KINDS);
        let mut scene = Scene::new();
        palette.paint_scene(248.0, &mut scene);

        // 2 quads per entry (bg + bottom border strip).
        assert_eq!(scene.quads.len(), SAMPLE_KINDS.len() * 2);

        // First row background starts at y=0.
        let first_bg = &scene.quads[0];
        assert_eq!(first_bg.bounds.origin.y, Pixels(0.0));
        assert_eq!(first_bg.bounds.size.width, Pixels(248.0));
        assert_eq!(first_bg.bounds.size.height, Pixels(24.0));
        assert!(first_bg.background.is_some());

        // Second row background starts at y=24.
        let second_bg = &scene.quads[2];
        assert_eq!(second_bg.bounds.origin.y, Pixels(24.0));
    }

    #[test]
    fn palette_empty_search_returns_all() {
        let palette = NodePalette::load_from_kinds(SAMPLE_KINDS);
        let results = palette.search("");
        assert_eq!(results.len(), SAMPLE_KINDS.len());
    }

    #[test]
    fn palette_search_filters_by_kind_name() {
        let kinds: &[(&str, &str, &str)] = &[
            ("VideoUnit", "Video Block", "A video media unit"),
            ("AudioUnit", "Audio Block", "An audio media unit"),
            ("TextBlock", "Text Block", "A prose text block"),
        ];
        let palette = NodePalette::load_from_kinds(kinds);
        let results = palette.search("video");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind_name, "VideoUnit");
    }

    #[test]
    fn palette_entry_fields_preserved() {
        let entry = PaletteEntry {
            kind_name: "Concept".to_string(),
            display_name: "Concept Node".to_string(),
            description: "An abstract idea".to_string(),
        };
        assert_eq!(entry.kind_name, "Concept");
        assert_eq!(entry.display_name, "Concept Node");
        assert_eq!(entry.description, "An abstract idea");
    }

    #[test]
    fn palette_entry_count_five() {
        let kinds: &[(&str, &str, &str)] = &[
            ("A", "Alpha", "desc a"),
            ("B", "Beta", "desc b"),
            ("C", "Gamma", "desc c"),
            ("D", "Delta", "desc d"),
            ("E", "Epsilon", "desc e"),
        ];
        let palette = NodePalette::load_from_kinds(kinds);
        assert_eq!(palette.entry_count(), 5);
    }
}
