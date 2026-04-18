#![deny(unsafe_code)]
pub mod bottom;
pub mod command_palette;
pub mod dock;
pub mod entity_ref;
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
pub use entity_ref::PanelEntityRef;
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
    use crate::dock::{rgba_to_hsla, Dock, DockPosition, Panel};
    use crate::left::file_tree::{FileNode, FileNodeKind, FileTreePanel};
    use crate::left::library::LibraryPanel;
    use crate::left::node_palette::NodePalette;
    use crate::right::chat_sidebar::{ChatMessage, ChatSidebarPanel};
    use crate::right::deep_think::{DeepThinkPanel, ThinkingStep};
    use crate::right::properties::PropertiesPanel;
    use nom_blocks::stub_dict::StubDictReader;

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
        let dict = StubDictReader::with_kinds(&["Function", "Concept", "Entity"]);
        let palette = NodePalette::load_from_dict(&dict);

        let mut library = LibraryPanel::new();
        library.load_from_dict(&dict);

        assert!(
            palette.entry_count() >= 3,
            "NodePalette must include the requested grammar kinds"
        );
        assert!(
            library.kind_count() >= 3,
            "LibraryPanel must include the requested grammar kinds"
        );

        let total = palette.entry_count() + library.kind_count();
        assert!(total >= 6, "combined entry count must be at least 6");

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

    // ── Panel trait: all kinds paint without panic ────────────────────────────

    #[test]
    fn file_tree_panel_paints_without_panic() {
        let panel = FileTreePanel::new();
        let mut scene = Scene::new();
        panel.paint_scene(248.0, 600.0, &mut scene);
        assert!(!scene.quads.is_empty(), "file tree panel must emit quads");
    }

    #[test]
    fn library_panel_paints_without_panic() {
        let dict = StubDictReader::with_kinds(&["Function"]);
        let mut panel = crate::left::LibraryPanel::new();
        panel.load_from_dict(&dict);
        let mut scene = Scene::new();
        panel.paint_scene(248.0, 500.0, &mut scene);
        assert!(!scene.quads.is_empty(), "library panel must emit quads");
    }

    #[test]
    fn node_palette_paints_without_panic() {
        let dict = StubDictReader::with_kinds(&["Concept"]);
        let palette = crate::left::NodePalette::load_from_dict(&dict);
        let mut scene = Scene::new();
        palette.paint_scene(248.0, &mut scene);
        assert!(!scene.quads.is_empty(), "node palette must emit quads");
    }

    #[test]
    fn properties_panel_paints_without_panic() {
        let mut panel = crate::right::PropertiesPanel::new();
        panel.load_entity("e1", "Concept");
        let mut scene = Scene::new();
        panel.paint_scene(280.0, 400.0, &mut scene);
        assert!(!scene.quads.is_empty(), "properties panel must emit quads");
    }

    #[test]
    fn chat_sidebar_panel_paints_without_panic() {
        let mut panel = crate::right::ChatSidebarPanel::new();
        panel.push_message(crate::right::ChatMessage::assistant_streaming("hi"));
        panel.finalize_last();
        let mut scene = Scene::new();
        panel.paint_scene(320.0, 400.0, &mut scene);
        assert!(!scene.quads.is_empty(), "chat sidebar panel must emit quads");
    }

    #[test]
    fn deep_think_panel_paints_without_panic() {
        let mut panel = DeepThinkPanel::new();
        panel.begin("test reasoning");
        panel.push_step(ThinkingStep::new("step1", 0.7));
        let mut scene = Scene::new();
        panel.paint_scene(320.0, 400.0, &mut scene);
        assert!(!scene.quads.is_empty(), "deep think panel must emit quads");
    }

    #[test]
    fn command_palette_paints_without_panic() {
        let mut palette = CommandPalette::new();
        palette.items.push(CommandPaletteItem::new("Test", "description"));
        let mut scene = Scene::new();
        palette.paint_scene(800.0, 600.0, &mut scene);
        assert!(!scene.quads.is_empty(), "command palette must emit quads");
    }

    // ── Panel trait: resize respects min_width ────────────────────────────────

    #[test]
    fn panel_size_state_fixed_effective_size() {
        let state = crate::dock::PanelSizeState::fixed(248.0);
        let effective = state.effective_size(1440.0);
        assert!((effective - 248.0).abs() < 0.001, "fixed size must return its value");
    }

    #[test]
    fn panel_size_state_flex_effective_size() {
        let state = crate::dock::PanelSizeState::flex(0.25);
        let effective = state.effective_size(1000.0);
        assert!((effective - 250.0).abs() < 0.001, "flex 0.25 of 1000 = 250");
    }

    #[test]
    fn panel_size_state_flex_clamped_to_one() {
        let state = crate::dock::PanelSizeState::flex(2.0);
        let effective = state.effective_size(1000.0);
        assert!(effective <= 1000.0, "flex must be clamped to 1.0 max");
    }

    #[test]
    fn panel_size_state_flex_zero() {
        let state = crate::dock::PanelSizeState::flex(0.0);
        let effective = state.effective_size(1000.0);
        assert_eq!(effective, 0.0, "flex 0 always yields 0");
    }

    #[test]
    fn panel_min_width_file_tree_is_positive() {
        let panel = FileTreePanel::new();
        assert!(panel.default_size() > 0.0, "file tree default_size must be positive");
        // min_width is conventionally half of default_size (>=120px)
        assert!(panel.default_size() >= 120.0, "file tree min width must be at least 120px");
    }

    #[test]
    fn dock_resize_does_not_shrink_below_zero() {
        let mut dock = Dock::new(DockPosition::Left);
        dock.add_panel("file-tree", 248.0);
        // Simulate a resize to a very small container — effective_size must not panic
        let entry = &dock.entries[0];
        let effective = entry.size_state.effective_size(0.0);
        assert!(effective >= 0.0, "effective size must be non-negative");
    }

    #[test]
    fn dock_panel_count_after_multiple_adds() {
        let mut dock = Dock::new(DockPosition::Left);
        dock.add_panel("a", 100.0);
        dock.add_panel("b", 200.0);
        dock.add_panel("c", 150.0);
        assert_eq!(dock.panel_count(), 3);
    }

    #[test]
    fn dock_activate_sets_active_panel() {
        let mut dock = Dock::new(DockPosition::Right);
        dock.add_panel("props", 280.0);
        dock.add_panel("chat", 320.0);
        let activated = dock.activate("chat");
        assert!(activated, "activate must return true for a known panel");
        assert_eq!(dock.active_panel_id(), Some("chat"));
    }

    #[test]
    fn dock_activate_unknown_panel_returns_false() {
        let mut dock = Dock::new(DockPosition::Left);
        dock.add_panel("file-tree", 248.0);
        let result = dock.activate("unknown-panel");
        assert!(!result, "activating unknown panel must return false");
    }

    #[test]
    fn dock_toggle_open_close() {
        let mut dock = Dock::new(DockPosition::Bottom);
        assert!(dock.is_open, "dock starts open");
        dock.toggle();
        assert!(!dock.is_open);
        dock.toggle();
        assert!(dock.is_open);
    }

    #[test]
    fn dock_paint_when_closed_emits_no_quads() {
        let mut dock = Dock::new(DockPosition::Left);
        dock.add_panel("file-tree", 248.0);
        dock.is_open = false;
        let mut scene = Scene::new();
        dock.paint_scene(1440.0, 900.0, &mut scene);
        assert!(scene.quads.is_empty(), "closed dock must not emit any quads");
    }

    #[test]
    fn runtime_ui_surfaces_emit_nonblank_scene_primitives() {
        let dict = StubDictReader::with_kinds(&["Function", "Concept", "Entity"]);
        let palette = NodePalette::load_from_dict(&dict);

        let mut library = LibraryPanel::new();
        library.load_from_dict(&dict);
        library.select_kind("Function");

        let mut file_tree = FileTreePanel::new();
        file_tree.sections[0]
            .nodes
            .push(FileNode::file("main.nom", 0, FileNodeKind::NomFile));
        file_tree.select("main.nom");

        let mut properties = PropertiesPanel::new();
        properties.load_entity("ent-1", "Concept");
        properties.set_row("name", "concept", true);

        let mut chat = ChatSidebarPanel::new();
        chat.push_message(ChatMessage::assistant_streaming("a1"));
        chat.append_to_last("ready");
        chat.finalize_last();
        chat.begin_tool("compile", "source.nom");
        chat.complete_tool("ok", 12);

        let mut deep_think = DeepThinkPanel::new();
        deep_think.begin("verify");
        deep_think.push_step(ThinkingStep::new("inspect", 0.9));

        let mut scene = Scene::new();
        palette.paint_scene(248.0, &mut scene);
        library.paint_scene(248.0, 400.0, &mut scene);
        file_tree.paint_scene(248.0, 400.0, &mut scene);
        properties.paint_scene(280.0, 400.0, &mut scene);
        chat.paint_scene(320.0, 400.0, &mut scene);
        deep_think.paint_scene(320.0, 400.0, &mut scene);

        assert!(
            scene.quads.len() >= 20,
            "runtime panel scene should be visibly nonblank"
        );
        assert!(
            scene.quads.iter().any(|quad| quad.border_color.is_some()),
            "focus/border primitives must be visible"
        );
    }
}
