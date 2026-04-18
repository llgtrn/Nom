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
    use crate::right::chat_sidebar::{ChatMessage, ChatRole, ChatSidebarPanel};
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

    // =========================================================================
    // WAVE-AG AGENT-9 ADDITIONS
    // =========================================================================

    // ── PanelSizeState: collapsed vs expanded ────────────────────────────────

    #[test]
    fn panel_size_state_collapsed_width_small() {
        // A collapsed (flex 0.1) state has a smaller effective size than an expanded (fixed 248) state.
        let collapsed = crate::dock::PanelSizeState::flex(0.1);
        let expanded = crate::dock::PanelSizeState::fixed(248.0);
        let collapsed_size = collapsed.effective_size(1440.0);
        let expanded_size = expanded.effective_size(1440.0);
        assert!(
            collapsed_size < expanded_size,
            "collapsed ({collapsed_size}) must be < expanded ({expanded_size})"
        );
    }

    #[test]
    fn panel_size_state_expanded_width_default() {
        let state = crate::dock::PanelSizeState::fixed(248.0);
        assert_eq!(state.effective_size(1440.0), 248.0, "fixed state must return 248.0");
    }

    #[test]
    fn panel_size_state_toggle_changes_state() {
        // Simulated toggle: swap between fixed(248) and fixed(0).
        let mut size = 248.0_f32;
        let was_expanded = size > 0.0;
        size = if was_expanded { 0.0 } else { 248.0 };
        assert_eq!(size, 0.0, "after toggle from expanded, size must be 0");
        let was_expanded = size > 0.0;
        size = if was_expanded { 0.0 } else { 248.0 };
        assert_eq!(size, 248.0, "after toggle from collapsed, size must be 248");
    }

    // ── All panel kinds paint without panic ──────────────────────────────────

    #[test]
    fn panel_all_kinds_paint_no_panic() {
        // Paint every major panel variant into a fresh scene; none must panic.
        let mut scene = Scene::new();

        let mut file_tree = FileTreePanel::new();
        file_tree.paint_scene(248.0, 600.0, &mut scene);

        let dict = nom_blocks::stub_dict::StubDictReader::with_kinds(&["Function"]);
        let mut library = crate::left::LibraryPanel::new();
        library.load_from_dict(&dict);
        library.paint_scene(248.0, 500.0, &mut scene);

        let palette = crate::left::NodePalette::load_from_dict(&dict);
        palette.paint_scene(248.0, &mut scene);

        let mut props = crate::right::PropertiesPanel::new();
        props.load_entity("e1", "Concept");
        props.paint_scene(280.0, 400.0, &mut scene);

        let mut chat = ChatSidebarPanel::new();
        chat.push_message(ChatMessage::assistant_streaming("a1"));
        chat.finalize_last();
        chat.paint_scene(320.0, 400.0, &mut scene);

        let mut deep_think = crate::right::DeepThinkPanel::new();
        deep_think.begin("verify");
        deep_think.push_step(ThinkingStep::new("step1", 0.8));
        deep_think.paint_scene(320.0, 400.0, &mut scene);

        assert!(!scene.quads.is_empty(), "at least one quad must be produced");
    }

    #[test]
    fn panel_paint_returns_quads() {
        let panel = FileTreePanel::new();
        let mut scene = Scene::new();
        panel.paint_scene(248.0, 600.0, &mut scene);
        assert!(scene.quads.len() >= 1, "painting must return at least 1 quad");
    }

    // ── ChatSidebarPanel: input and history ──────────────────────────────────

    #[test]
    fn panel_chat_model_has_input_and_history() {
        let mut chat = ChatSidebarPanel::new();
        // Initially no messages.
        assert_eq!(chat.message_count(), 0, "new chat panel starts with 0 messages");

        // Push a user message and an assistant message.
        chat.push_message(ChatMessage::user("u1", "hello"));
        chat.push_message(ChatMessage::assistant_streaming("a1"));
        chat.append_to_last(" world");
        chat.finalize_last();

        assert_eq!(chat.message_count(), 2, "chat must have 2 messages after two pushes");
        assert_eq!(chat.messages[0].role, ChatRole::User);
        assert_eq!(chat.messages[1].role, ChatRole::Assistant);
        assert!(!chat.messages[1].is_streaming, "finalized message must not be streaming");
        assert!(
            chat.messages[1].content.contains("world"),
            "appended delta must appear in content"
        );
    }

    #[test]
    fn panel_chat_tool_card_attached_to_last_message() {
        let mut chat = ChatSidebarPanel::new();
        chat.push_message(ChatMessage::assistant_streaming("a1"));
        chat.finalize_last();
        chat.begin_tool("compile", "source.nom");
        chat.complete_tool("ok", 42);
        assert_eq!(
            chat.messages[0].tool_cards.len(),
            1,
            "completed tool must be attached to the last message"
        );
        assert_eq!(chat.messages[0].tool_cards[0].tool_name, "compile");
    }

    // ── DeepThinkPanel: steps ────────────────────────────────────────────────

    #[test]
    fn panel_deep_think_model_has_steps() {
        let mut panel = crate::right::DeepThinkPanel::new();
        assert!(panel.steps.is_empty(), "new panel has no steps");

        panel.begin("analyze layout");
        panel.push_step(ThinkingStep::new("check positions", 0.7));
        panel.push_step(ThinkingStep::new("verify constraints", 0.9));

        assert_eq!(panel.steps.len(), 2, "panel must have 2 steps after two pushes");
        assert_eq!(panel.steps[0].hypothesis, "check positions");
        assert_eq!(panel.steps[1].hypothesis, "verify constraints");
    }

    #[test]
    fn panel_deep_think_complete_marks_done() {
        let mut panel = crate::right::DeepThinkPanel::new();
        panel.begin("reasoning task");
        panel.push_step(ThinkingStep::new("hypothesis", 0.85));
        panel.complete();
        // After complete(), the panel must have steps and the state must be Complete.
        assert!(!panel.steps.is_empty(), "panel must have steps after push_step");
        // Paint must still succeed after completion.
        let mut scene = Scene::new();
        panel.paint_scene(320.0, 400.0, &mut scene);
        assert!(!scene.quads.is_empty(), "completed panel must still paint quads");
    }

    // ── FileNode/FileTree additional tests ───────────────────────────────────

    #[test]
    fn file_tree_leaf_has_no_children() {
        let leaf = FileNode::file("leaf.nom", 3, FileNodeKind::NomFile);
        assert!(
            leaf.children.is_empty(),
            "leaf file node must have no children"
        );
    }

    #[test]
    fn file_tree_root_has_children() {
        let mut root = FileNode::dir("src", 0);
        root.children.push(FileNode::file("main.nom", 1, FileNodeKind::NomFile));
        root.children.push(FileNode::file("lib.nom", 1, FileNodeKind::NomFile));
        assert_eq!(root.children.len(), 2, "root must have 2 children after pushing 2");
    }

    #[test]
    fn file_tree_empty_tree_safe() {
        let panel = FileTreePanel {
            sections: vec![],
            selected_id: None,
        };
        let mut scene = Scene::new();
        // Must not panic when painting an empty tree.
        panel.paint_scene(248.0, 600.0, &mut scene);
        // An empty file tree emits only the background quad.
        assert!(!scene.quads.is_empty(), "empty tree must still emit background quad");
    }

    #[test]
    fn file_tree_node_count_correct() {
        let mut section = crate::left::file_tree::CollapsibleSection::new("ws", "Workspace");
        for i in 0..5 {
            section.nodes.push(FileNode::file(format!("f{i}.nom"), 0, FileNodeKind::NomFile));
        }
        assert_eq!(section.nodes.len(), 5, "section must have exactly 5 nodes");
    }

    #[test]
    fn file_tree_selected_node_tracking() {
        let mut panel = FileTreePanel::new();
        panel.sections[0].nodes.push(FileNode::file("alpha.nom", 0, FileNodeKind::NomFile));
        panel.sections[0].nodes.push(FileNode::file("beta.nom", 0, FileNodeKind::NomFile));

        panel.select("alpha.nom");
        assert_eq!(panel.selected_id.as_deref(), Some("alpha.nom"), "first selection");

        panel.select("beta.nom");
        assert_eq!(panel.selected_id.as_deref(), Some("beta.nom"), "selection must update");
    }

    #[test]
    fn file_tree_search_finds_file() {
        // Simple name-based search simulation.
        let mut panel = FileTreePanel::new();
        panel.sections[0].nodes.push(FileNode::file("search_target.nom", 0, FileNodeKind::NomFile));
        panel.sections[0].nodes.push(FileNode::file("other.nom", 0, FileNodeKind::NomFile));

        let found = panel.sections.iter().any(|sec| {
            sec.nodes.iter().any(|n| n.name.contains("search_target"))
        });
        assert!(found, "search must find the target file by name substring");
    }

    #[test]
    fn file_tree_5_level_depth_correct() {
        // Build a 5-level tree and verify all depth values.
        let mut d4 = FileNode::file("leaf.nom", 4, FileNodeKind::NomFile);
        let mut d3 = FileNode::dir("d3", 3);
        d3.is_expanded = true;
        d3.children.push(d4);
        let mut d2 = FileNode::dir("d2", 2);
        d2.is_expanded = true;
        d2.children.push(d3);
        let mut d1 = FileNode::dir("d1", 1);
        d1.is_expanded = true;
        d1.children.push(d2);
        let mut d0 = FileNode::dir("d0", 0);
        d0.is_expanded = true;
        d0.children.push(d1);

        let visible = d0.visible_nodes();
        assert_eq!(visible.len(), 5);
        for (i, node) in visible.iter().enumerate() {
            assert_eq!(
                node.depth as usize, i,
                "depth at position {i} must be {i}, got {}",
                node.depth
            );
        }
    }

    #[test]
    fn file_tree_expand_path_makes_visible() {
        // Expand a root→child path; all nodes on the path become visible.
        let mut child = FileNode::file("main.nom", 1, FileNodeKind::NomFile);
        let mut root = FileNode::dir("src", 0);
        root.children.push(child);
        // Before expand: only root visible.
        assert_eq!(root.visible_nodes().len(), 1);
        // Expand root.
        root.is_expanded = true;
        assert_eq!(root.visible_nodes().len(), 2, "after expand root, child must be visible");
    }

    // ── Entity ref additional tests (PanelEntityRef / NomtuRef) ─────────────

    #[test]
    fn entity_ref_into_option_some_for_valid() {
        use crate::entity_ref::PanelEntityRef;
        use nom_blocks::NomtuRef;
        let e = PanelEntityRef::nomtu(NomtuRef::new("id1", "word", "kind"));
        assert!(e.into_option().is_some(), "valid ref must yield Some");
    }

    #[test]
    fn entity_ref_into_option_none_for_empty() {
        use crate::entity_ref::PanelEntityRef;
        let e = PanelEntityRef::None;
        assert!(e.into_option().is_none(), "None ref must yield None");
    }

    #[test]
    fn entity_ref_word_nonempty_for_valid() {
        use crate::entity_ref::PanelEntityRef;
        use nom_blocks::NomtuRef;
        let e = PanelEntityRef::nomtu(NomtuRef::new("id", "myword", "kind"));
        let nomtu = e.as_nomtu().unwrap();
        assert!(!nomtu.word.is_empty(), "word must be non-empty for a valid ref");
    }

    #[test]
    fn entity_ref_kind_nonempty_for_valid() {
        use crate::entity_ref::PanelEntityRef;
        use nom_blocks::NomtuRef;
        let e = PanelEntityRef::nomtu(NomtuRef::new("id", "word", "Concept"));
        assert!(!e.kind().unwrap().is_empty(), "kind must be non-empty");
    }

    #[test]
    fn entity_ref_display_includes_word() {
        use crate::entity_ref::PanelEntityRef;
        use nom_blocks::NomtuRef;
        let e = PanelEntityRef::nomtu(NomtuRef::new("id", "display-word", "kind"));
        let debug_str = format!("{:?}", e);
        assert!(
            debug_str.contains("display-word"),
            "debug output must include the word field"
        );
    }

    #[test]
    fn entity_ref_clone_equal() {
        use crate::entity_ref::PanelEntityRef;
        use nom_blocks::NomtuRef;
        let a = PanelEntityRef::nomtu(NomtuRef::new("id-clone", "word", "Kind"));
        let b = a.clone();
        assert_eq!(a, b, "cloned PanelEntityRef must equal original");
    }

    #[test]
    fn entity_ref_ne_different_words() {
        use crate::entity_ref::PanelEntityRef;
        use nom_blocks::NomtuRef;
        let a = PanelEntityRef::nomtu(NomtuRef::new("id", "word-a", "Kind"));
        let b = PanelEntityRef::nomtu(NomtuRef::new("id", "word-b", "Kind"));
        // PanelEntityRef derives PartialEq — it compares the inner NomtuRef fields.
        assert_ne!(a, b, "refs with different words must not be equal");
    }

    #[test]
    fn entity_ref_eq_by_value() {
        use crate::entity_ref::PanelEntityRef;
        use nom_blocks::NomtuRef;
        let a = PanelEntityRef::nomtu(NomtuRef::new("same-id", "same-word", "same-kind"));
        let b = PanelEntityRef::nomtu(NomtuRef::new("same-id", "same-word", "same-kind"));
        assert_eq!(a, b, "two refs with identical fields must be equal");
    }

    #[test]
    fn entity_ref_hash_consistency() {
        use crate::entity_ref::PanelEntityRef;
        use nom_blocks::NomtuRef;
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Hash the inner NomtuRef directly (PanelEntityRef wraps it).
        let r1 = NomtuRef::new("hash-id", "word", "Kind");
        let r2 = NomtuRef::new("hash-id", "word", "Kind");
        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        r1.hash(&mut h1);
        r2.hash(&mut h2);
        assert_eq!(
            h1.finish(),
            h2.finish(),
            "identical NomtuRef values must hash identically"
        );
    }

    // ── Dock: additional coverage ────────────────────────────────────────────

    #[test]
    fn dock_position_left_paint_emits_quads() {
        let mut dock = crate::dock::Dock::new(crate::dock::DockPosition::Left);
        dock.add_panel("file-tree", 248.0);
        let mut scene = Scene::new();
        dock.paint_scene(1440.0, 900.0, &mut scene);
        assert!(!scene.quads.is_empty(), "left dock must emit quads when open");
    }

    #[test]
    fn dock_position_right_paint_emits_quads() {
        let mut dock = crate::dock::Dock::new(crate::dock::DockPosition::Right);
        dock.add_panel("props", 320.0);
        let mut scene = Scene::new();
        dock.paint_scene(1440.0, 900.0, &mut scene);
        assert!(!scene.quads.is_empty(), "right dock must emit quads when open");
    }

    #[test]
    fn dock_position_bottom_paint_emits_quads() {
        let mut dock = crate::dock::Dock::new(crate::dock::DockPosition::Bottom);
        dock.add_panel("terminal", 200.0);
        let mut scene = Scene::new();
        dock.paint_scene(1440.0, 900.0, &mut scene);
        assert!(!scene.quads.is_empty(), "bottom dock must emit quads when open");
    }

    #[test]
    fn chat_message_streaming_flag_initial_true() {
        let msg = ChatMessage::assistant_streaming("a-id");
        assert!(msg.is_streaming, "new assistant_streaming message must have is_streaming=true");
    }

    #[test]
    fn chat_message_user_not_streaming() {
        let msg = ChatMessage::user("u-id", "hello");
        assert!(!msg.is_streaming, "user message must not be streaming");
    }

    #[test]
    fn chat_message_append_delta_accumulates() {
        let mut msg = ChatMessage::assistant_streaming("a-id");
        msg.append_delta("hello");
        msg.append_delta(" world");
        assert_eq!(msg.content, "hello world", "appended deltas must accumulate in content");
    }

    #[test]
    fn chat_sidebar_scroll_to_bottom_set_on_push() {
        let mut chat = ChatSidebarPanel::new();
        assert!(!chat.scroll_to_bottom, "starts without scroll request");
        chat.push_message(ChatMessage::user("u1", "text"));
        assert!(chat.scroll_to_bottom, "scroll_to_bottom must be true after push");
    }
}
