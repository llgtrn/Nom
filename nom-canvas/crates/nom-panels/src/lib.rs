#![deny(unsafe_code)]
pub mod bottom;
pub mod command_palette;
pub mod dock;
pub mod left;
pub mod pane;
pub mod right;
pub mod shell;
pub mod statusbar;
pub mod toolbar;

pub use bottom::{
    Diagnostic, DiagnosticSeverity, DiagnosticsPanel, TerminalLine, TerminalLineKind, TerminalPanel,
};
pub use command_palette::{CommandPalette, CommandPaletteItem};
pub use dock::{
    fill_quad, focus_ring_quad, rgba_to_hsla, Dock, DockPosition, Panel, PanelEntry, PanelSizeState,
};
pub use left::{
    FileNode, FileNodeKind, FileTreePanel, LibraryKind, LibraryPanel, NodePalette, PaletteEntry,
    QuickSearchPanel, SearchResult, SearchResultKind,
};
pub use pane::{Member, Pane, PaneAxis, PaneGroup, PaneTab, SplitDirection};
pub use right::{
    ChatMessage, ChatRole, ChatSidebarPanel, DeepThinkPanel, PropertiesPanel, PropertyRow,
    ThinkingStep, ToolCard,
};
pub use shell::{Shell, ShellLayout, ShellMode};
pub use statusbar::{StatusBar, StatusSlot};
pub use toolbar::{Toolbar, ToolbarButton};

#[cfg(test)]
mod integration_tests {
    use nom_gpui::scene::Scene;
    use nom_theme::tokens;

    use crate::command_palette::{CommandPalette, CommandPaletteItem};
    use crate::dock::{rgba_to_hsla, Dock, DockPosition};
    use crate::left::library::LibraryPanel;
    use crate::left::node_palette::NodePalette;
    use crate::right::deep_think::{DeepThinkPanel, ThinkingStep};

    // -------------------------------------------------------------------------
    // Test 1: panels_use_nom_theme_tokens
    // Verifies that Dock.paint_scene emits quads whose color values derive
    // from nom_theme::tokens constants, not from hardcoded arbitrary values.
    // -------------------------------------------------------------------------
    #[test]
    fn panels_use_nom_theme_tokens() {
        let mut dock = Dock::new(DockPosition::Left);
        dock.add_panel("node-palette", 248.0);
        let mut scene = Scene::new();
        dock.paint_scene(1440.0, 900.0, &mut scene);

        // The background quad must use tokens::BG as its fill color.
        let bg_quad = &scene.quads[0];
        let expected_bg = rgba_to_hsla(tokens::BG);
        let actual_bg = bg_quad
            .background
            .expect("background quad must have a fill");
        assert!(
            (actual_bg.h - expected_bg.h).abs() < 1e-3
                && (actual_bg.l - expected_bg.l).abs() < 1e-3
                && (actual_bg.a - expected_bg.a).abs() < 1e-3,
            "background color must match tokens::BG exactly"
        );

        // The border color must match tokens::BORDER.
        let expected_border = rgba_to_hsla(tokens::BORDER);
        let actual_border = bg_quad
            .border_color
            .expect("background quad must have a border");
        assert!(
            (actual_border.h - expected_border.h).abs() < 1e-3
                && (actual_border.l - expected_border.l).abs() < 1e-3
                && (actual_border.a - expected_border.a).abs() < 1e-3,
            "border color must match tokens::BORDER exactly"
        );

        // The active-tab ring must use tokens::FOCUS as border color.
        let ring_quad = &scene.quads[1];
        let expected_focus = rgba_to_hsla(tokens::FOCUS);
        let actual_focus = ring_quad
            .border_color
            .expect("focus ring must have a border");
        assert!(
            (actual_focus.h - expected_focus.h).abs() < 1e-3
                && (actual_focus.l - expected_focus.l).abs() < 1e-3
                && (actual_focus.a - expected_focus.a).abs() < 1e-3,
            "focus ring color must match tokens::FOCUS exactly"
        );
    }

    // -------------------------------------------------------------------------
    // Test 2: command_palette_with_deep_think_panel
    // Paint CommandPalette (3 items) + DeepThinkPanel to the same scene;
    // combined quad count must be >= 4.
    // -------------------------------------------------------------------------
    #[test]
    fn command_palette_with_deep_think_panel() {
        let mut palette = CommandPalette::new();
        palette
            .items
            .push(CommandPaletteItem::new("Open Graph", "Open graph view"));
        palette
            .items
            .push(CommandPaletteItem::new("Save All", "Save all files"));
        palette.items.push(CommandPaletteItem::new(
            "Run Build",
            "Execute build pipeline",
        ));

        let mut deep_think = DeepThinkPanel::new();
        deep_think.begin("analyze cross-panel layout");
        deep_think.push_step(ThinkingStep::new("evaluate panel positions", 0.8));
        deep_think.complete();

        let mut scene = Scene::new();
        palette.paint_scene(1440.0, 900.0, &mut scene);
        let quads_after_palette = scene.quads.len();

        deep_think.paint_scene(320.0, 600.0, &mut scene);

        let total_quads = scene.quads.len();
        assert!(
            total_quads >= 4,
            "combined quad count must be >= 4, got {}",
            total_quads
        );
        assert!(
            quads_after_palette >= 1,
            "palette must contribute at least 1 quad"
        );
        assert!(
            total_quads > quads_after_palette,
            "deep-think panel must add quads to the shared scene"
        );
    }

    // -------------------------------------------------------------------------
    // Test 3: node_palette_and_library_panel_coexist
    // Load 3 kinds into NodePalette and 3 into LibraryPanel;
    // verify total entry counts.
    // -------------------------------------------------------------------------
    #[test]
    fn node_palette_and_library_panel_coexist() {
        let node_kinds: &[(&str, &str, &str)] = &[
            ("Function", "Function", "A callable unit"),
            ("Concept", "Concept", "An abstract unit"),
            ("Entity", "Entity", "A concrete object"),
        ];
        let palette = NodePalette::load_from_kinds(node_kinds);

        let mut library = LibraryPanel::new();
        library.load_kinds(&[
            ("Function", "Callable units of work", 10),
            ("Concept", "Abstract semantic units", 5),
            ("Entity", "Concrete named objects", 3),
        ]);

        assert_eq!(palette.entry_count(), 3, "NodePalette must have 3 entries");
        assert_eq!(library.kind_count(), 3, "LibraryPanel must have 3 kinds");

        let total = palette.entry_count() + library.kind_count();
        assert_eq!(total, 6, "combined entry count must be 6");

        // Paint both into the same scene and verify they each emit quads.
        let mut scene = Scene::new();
        palette.paint_scene(248.0, &mut scene);
        let quads_after_palette = scene.quads.len();
        library.paint_scene(248.0, 500.0, &mut scene);
        let total_quads = scene.quads.len();

        // NodePalette: 2 quads per entry = 6; LibraryPanel: 1 header + 2*3 = 7
        assert!(quads_after_palette >= 6, "palette must emit >= 6 quads");
        assert!(
            total_quads >= quads_after_palette + 7,
            "library must add >= 7 quads"
        );
    }
}
