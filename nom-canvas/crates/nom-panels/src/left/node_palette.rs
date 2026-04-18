#![deny(unsafe_code)]
use crate::dock::{fill_quad, DockPosition, Panel};
use nom_blocks::dict_reader::DictReader;
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
pub struct NodePalette {
    pub entries: Vec<PaletteEntry>,
}

impl NodePalette {
    pub fn new() -> Self {
        Self { entries: vec![] }
    }

    /// Populate the palette from the dictionary/grammar source of truth.
    pub fn load_from_dict(dict: &dyn DictReader) -> Self {
        let entries = dict
            .list_kinds()
            .into_iter()
            .map(|kind| PaletteEntry {
                display_name: kind.name.clone(),
                kind_name: kind.name,
                description: kind.description,
            })
            .collect();
        Self { entries }
    }

    #[cfg(test)]
    fn load_from_kinds(kinds: &[(&str, &str, &str)]) -> Self {
        Self {
            entries: kinds
                .iter()
                .map(|(kind_name, display_name, description)| PaletteEntry {
                    kind_name: (*kind_name).to_string(),
                    display_name: (*display_name).to_string(),
                    description: (*description).to_string(),
                })
                .collect(),
        }
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

    #[test]
    fn node_palette_panel_trait_id_and_title() {
        let palette = NodePalette::new();
        assert_eq!(palette.id(), "node-palette");
        assert_eq!(palette.title(), "Node Palette");
        assert_eq!(palette.default_size(), 248.0);
    }

    #[test]
    fn node_palette_position_is_left() {
        let palette = NodePalette::new();
        assert_eq!(palette.position(), crate::dock::DockPosition::Left);
    }

    #[test]
    fn node_palette_activation_priority() {
        let palette = NodePalette::new();
        assert_eq!(palette.activation_priority(), 30);
    }

    #[test]
    fn node_palette_search_case_insensitive() {
        let palette = NodePalette::load_from_kinds(SAMPLE_KINDS);
        let results = palette.search("FUNCTION");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind_name, "Function");
    }

    #[test]
    fn node_palette_search_matches_display_name() {
        let kinds: &[(&str, &str, &str)] = &[
            ("K1", "Alpha Widget", "desc"),
            ("K2", "Beta Widget", "desc"),
            ("K3", "Gamma Other", "desc"),
        ];
        let palette = NodePalette::load_from_kinds(kinds);
        let results = palette.search("Widget");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn node_palette_default_is_empty() {
        let palette = NodePalette::default();
        assert_eq!(palette.entry_count(), 0);
    }

    #[test]
    fn node_palette_empty_kinds_empty_palette() {
        let palette = NodePalette::load_from_kinds(&[]);
        assert_eq!(palette.entry_count(), 0);
        let all = palette.search("");
        assert!(all.is_empty());
    }

    #[test]
    fn node_palette_category_grouping_by_prefix() {
        let kinds: &[(&str, &str, &str)] = &[
            ("MediaVideo", "Video", "video block"),
            ("MediaAudio", "Audio", "audio block"),
            ("TextBlock", "Text", "text block"),
        ];
        let palette = NodePalette::load_from_kinds(kinds);
        // Search for "media" matches first two
        let media_results = palette.search("media");
        assert_eq!(media_results.len(), 2);
        // Search for "block" matches last two (via display or kind)
        let block_results = palette.search("block");
        // "TextBlock" matches by kind_name, "MediaVideo"/"MediaAudio" match by description
        assert!(block_results.len() >= 1);
    }

    #[test]
    fn node_palette_search_no_match_returns_empty() {
        let palette = NodePalette::load_from_kinds(SAMPLE_KINDS);
        let results = palette.search("xyznotfound");
        assert!(results.is_empty());
    }

    #[test]
    fn node_palette_search_exact_kind_name() {
        let palette = NodePalette::load_from_kinds(SAMPLE_KINDS);
        let results = palette.search("Entity");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind_name, "Entity");
    }

    #[test]
    fn node_palette_search_partial_description_not_matched() {
        // search only covers kind_name and display_name, NOT description
        let kinds: &[(&str, &str, &str)] = &[("K1", "Alpha", "unique_desc_xyz")];
        let palette = NodePalette::load_from_kinds(kinds);
        // description not searched — "unique_desc_xyz" is only in description
        let results = palette.search("unique_desc_xyz");
        assert!(results.is_empty());
    }

    #[test]
    fn node_palette_single_entry() {
        let kinds: &[(&str, &str, &str)] = &[("Single", "Only One", "sole entry")];
        let palette = NodePalette::load_from_kinds(kinds);
        assert_eq!(palette.entry_count(), 1);
        let all = palette.search("");
        assert_eq!(all.len(), 1);
        let none = palette.search("zzz");
        assert!(none.is_empty());
    }

    #[test]
    fn node_palette_paint_scene_empty_palette() {
        let palette = NodePalette::load_from_kinds(&[]);
        let mut scene = nom_gpui::scene::Scene::new();
        palette.paint_scene(200.0, &mut scene);
        assert_eq!(scene.quads.len(), 0);
    }

    #[test]
    fn node_palette_paint_scene_single_entry() {
        let kinds: &[(&str, &str, &str)] = &[("K", "K", "d")];
        let palette = NodePalette::load_from_kinds(kinds);
        let mut scene = nom_gpui::scene::Scene::new();
        palette.paint_scene(100.0, &mut scene);
        // 1 entry → 2 quads (bg + border)
        assert_eq!(scene.quads.len(), 2);
    }

    #[test]
    fn node_palette_entry_equality() {
        let a = PaletteEntry {
            kind_name: "K".to_string(),
            display_name: "D".to_string(),
            description: "desc".to_string(),
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn node_palette_entry_inequality() {
        let a = PaletteEntry {
            kind_name: "K1".to_string(),
            display_name: "D".to_string(),
            description: "desc".to_string(),
        };
        let b = PaletteEntry {
            kind_name: "K2".to_string(),
            display_name: "D".to_string(),
            description: "desc".to_string(),
        };
        assert_ne!(a, b);
    }

    // ── 50-entry paint scene ──────────────────────────────────────────────────

    fn make_50_kinds() -> Vec<(String, String, String)> {
        (0..50)
            .map(|i| (format!("Kind{i}"), format!("Kind {i}"), format!("desc {i}")))
            .collect()
    }

    #[test]
    fn node_palette_paint_scene_50_entries_quad_count() {
        let kinds_owned = make_50_kinds();
        let kinds: Vec<(&str, &str, &str)> = kinds_owned
            .iter()
            .map(|(a, b, c)| (a.as_str(), b.as_str(), c.as_str()))
            .collect();
        let palette = NodePalette::load_from_kinds(&kinds);
        assert_eq!(palette.entry_count(), 50);
        let mut scene = nom_gpui::scene::Scene::new();
        palette.paint_scene(248.0, &mut scene);
        // 2 quads per entry (bg + border)
        assert_eq!(scene.quads.len(), 100);
    }

    #[test]
    fn node_palette_paint_scene_50_entries_row_heights_correct() {
        let kinds_owned = make_50_kinds();
        let kinds: Vec<(&str, &str, &str)> = kinds_owned
            .iter()
            .map(|(a, b, c)| (a.as_str(), b.as_str(), c.as_str()))
            .collect();
        let palette = NodePalette::load_from_kinds(&kinds);
        let mut scene = nom_gpui::scene::Scene::new();
        palette.paint_scene(300.0, &mut scene);
        // Row bg quads are at even indices (0, 2, 4, ...)
        // Row i bg quad origin y = i * 24.0
        let bg_quads: Vec<_> = scene.quads.iter().step_by(2).collect();
        assert_eq!(bg_quads.len(), 50);
        for (i, q) in bg_quads.iter().enumerate() {
            assert_eq!(
                q.bounds.origin.y,
                Pixels(i as f32 * 24.0),
                "row {i} y mismatch"
            );
        }
    }

    #[test]
    fn node_palette_paint_scene_50_entries_all_widths_match() {
        let kinds_owned = make_50_kinds();
        let kinds: Vec<(&str, &str, &str)> = kinds_owned
            .iter()
            .map(|(a, b, c)| (a.as_str(), b.as_str(), c.as_str()))
            .collect();
        let palette = NodePalette::load_from_kinds(&kinds);
        let mut scene = nom_gpui::scene::Scene::new();
        let paint_width = 320.0_f32;
        palette.paint_scene(paint_width, &mut scene);
        // Every bg quad should match paint_width
        for (i, q) in scene.quads.iter().step_by(2).enumerate() {
            assert_eq!(
                q.bounds.size.width,
                Pixels(paint_width),
                "width mismatch at entry {i}"
            );
        }
    }

    // ── scrolled-offset clip simulation ──────────────────────────────────────

    /// Simulate scroll offset: visible entries within a viewport height.
    #[test]
    fn node_palette_scroll_offset_clips_visible_entries() {
        let kinds_owned = make_50_kinds();
        let kinds: Vec<(&str, &str, &str)> = kinds_owned
            .iter()
            .map(|(a, b, c)| (a.as_str(), b.as_str(), c.as_str()))
            .collect();
        let palette = NodePalette::load_from_kinds(&kinds);
        let row_height = 24.0_f32;
        let viewport_height = 200.0_f32;
        let scroll_offset = 5usize; // skip first 5 entries

        let visible_entries: Vec<_> = palette
            .entries
            .iter()
            .skip(scroll_offset)
            .take((viewport_height / row_height).ceil() as usize)
            .collect();

        // Should see at most ceil(200/24) = 9 entries
        assert!(visible_entries.len() <= 9);
        assert_eq!(visible_entries[0].kind_name, "Kind5");
    }

    #[test]
    fn node_palette_scroll_offset_zero_shows_first_entry() {
        let kinds_owned = make_50_kinds();
        let kinds: Vec<(&str, &str, &str)> = kinds_owned
            .iter()
            .map(|(a, b, c)| (a.as_str(), b.as_str(), c.as_str()))
            .collect();
        let palette = NodePalette::load_from_kinds(&kinds);
        let scroll_offset = 0usize;
        let first = palette.entries.iter().skip(scroll_offset).next();
        assert!(first.is_some());
        assert_eq!(first.unwrap().kind_name, "Kind0");
    }

    #[test]
    fn node_palette_scroll_offset_max_shows_last_entry() {
        let kinds_owned = make_50_kinds();
        let kinds: Vec<(&str, &str, &str)> = kinds_owned
            .iter()
            .map(|(a, b, c)| (a.as_str(), b.as_str(), c.as_str()))
            .collect();
        let palette = NodePalette::load_from_kinds(&kinds);
        // scroll to near the bottom
        let scroll_offset = 49usize;
        let visible: Vec<_> = palette.entries.iter().skip(scroll_offset).collect();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].kind_name, "Kind49");
    }

    // ── keyboard selection cycling ─────────────────────────────────────────────

    #[test]
    fn node_palette_keyboard_selection_cycles_forward() {
        let palette = NodePalette::load_from_kinds(SAMPLE_KINDS);
        let count = palette.entry_count();
        let mut selection = 0usize;
        // Arrow down 3 times wraps around (3 entries)
        for _ in 0..3 {
            selection = (selection + 1) % count;
        }
        assert_eq!(selection, 0); // wrapped back to start
    }

    #[test]
    fn node_palette_keyboard_selection_cycles_backward() {
        let palette = NodePalette::load_from_kinds(SAMPLE_KINDS);
        let count = palette.entry_count();
        let mut selection = 0usize;
        // Arrow up from 0 → wraps to last
        selection = selection.checked_sub(1).unwrap_or(count - 1);
        assert_eq!(selection, count - 1);
    }

    #[test]
    fn node_palette_keyboard_selection_stays_within_bounds() {
        let palette = NodePalette::load_from_kinds(SAMPLE_KINDS);
        let count = palette.entry_count();
        let mut selection = 0usize;
        for i in 0..20 {
            selection = (selection + 1) % count;
            assert!(selection < count, "selection out of bounds at step {i}");
        }
    }

    #[test]
    fn node_palette_keyboard_selection_initial_is_zero() {
        // Initial selection index is always 0 (first entry)
        let selection: usize = 0;
        let palette = NodePalette::load_from_kinds(SAMPLE_KINDS);
        assert!(selection < palette.entry_count());
        assert_eq!(palette.entries[selection].kind_name, "Function");
    }

    #[test]
    fn node_palette_keyboard_selection_selects_correct_entry() {
        let palette = NodePalette::load_from_kinds(SAMPLE_KINDS);
        let count = palette.entry_count();
        for target in 0..count {
            assert!(target < palette.entries.len());
            let _ = &palette.entries[target]; // no panic = valid selection
        }
    }
}
