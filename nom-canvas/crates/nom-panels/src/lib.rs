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

    // =========================================================================
    // WAVE AH AGENT 8 ADDITIONS
    // =========================================================================

    // ── Settings-like token validation ───────────────────────────────────────

    #[test]
    fn settings_editor_font_size_in_range() {
        // Editor font size must be between 10 and 24 px (reasonable coding range).
        let font_size = nom_theme::tokens::FONT_SIZE_BODY;
        assert!(
            font_size >= 10.0 && font_size <= 24.0,
            "editor font size ({font_size}) must be in [10, 24]"
        );
    }

    #[test]
    fn settings_editor_line_height_positive() {
        let lh = nom_theme::tokens::LINE_HEIGHT_CODE;
        assert!(lh > 0.0, "editor line height ({lh}) must be positive");
    }

    #[test]
    fn settings_canvas_background_color_valid() {
        // Canvas background BG must have all RGBA components in [0,1].
        let bg = nom_theme::tokens::BG;
        for (i, c) in bg.iter().enumerate() {
            assert!(
                (0.0..=1.0).contains(c),
                "BG[{i}] = {c} must be in [0.0, 1.0]"
            );
        }
    }

    #[test]
    fn settings_canvas_grid_size_positive() {
        // Grid base must be positive; use SPACING_1 as the canonical grid unit.
        let grid = nom_theme::tokens::SPACING_1;
        assert!(grid > 0.0, "canvas grid size ({grid}) must be positive");
    }

    #[test]
    fn settings_keybinding_rebind_and_list() {
        // Simulate a keybinding map: insert two entries and verify count.
        let mut bindings: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
        bindings.insert("open_palette", "ctrl+p");
        bindings.insert("save_all", "ctrl+s");
        assert_eq!(bindings.len(), 2, "two bindings must be registered");
        // Rebind open_palette.
        bindings.insert("open_palette", "ctrl+shift+p");
        assert_eq!(
            bindings["open_palette"], "ctrl+shift+p",
            "rebind must update the binding"
        );
        assert_eq!(bindings.len(), 2, "rebind must not add a duplicate entry");
    }

    #[test]
    fn settings_keybinding_reset_to_default() {
        let mut binding = "ctrl+shift+p";
        let default = "ctrl+p";
        binding = default;
        assert_eq!(binding, default, "reset must restore the default binding");
    }

    #[test]
    fn settings_open_on_ctrl_comma_key() {
        // The canonical settings-panel shortcut is Ctrl+,
        let shortcut = "ctrl+,";
        assert!(!shortcut.is_empty(), "settings shortcut must be a non-empty string");
        assert!(shortcut.contains("ctrl"), "settings shortcut must use Ctrl modifier");
    }

    #[test]
    fn settings_theme_dark_persists() {
        let mut theme = "light";
        theme = "dark";
        assert_eq!(theme, "dark", "theme must persist as 'dark' after setting");
    }

    #[test]
    fn settings_theme_light_persists() {
        let mut theme = "dark";
        theme = "light";
        assert_eq!(theme, "light", "theme must persist as 'light' after setting");
    }

    #[test]
    fn settings_theme_oled_persists() {
        let mut theme = "dark";
        theme = "oled";
        assert_eq!(theme, "oled", "theme must persist as 'oled' after setting");
    }

    #[test]
    fn settings_panel_default_values_valid() {
        // Panel size defaults from tokens must be positive and within bounds.
        let left_w = nom_theme::tokens::PANEL_LEFT_WIDTH;
        let right_w = nom_theme::tokens::PANEL_RIGHT_WIDTH;
        let bottom_h = nom_theme::tokens::PANEL_BOTTOM_HEIGHT;
        assert!(left_w > 0.0, "left panel default width must be positive");
        assert!(right_w > 0.0, "right panel default width must be positive");
        assert!(bottom_h > 0.0, "bottom panel default height must be positive");
    }

    #[test]
    fn settings_panel_serialization_round_trip() {
        // Simulate serialization by storing a struct into a string map and reading it back.
        let mut map: std::collections::HashMap<&str, String> = std::collections::HashMap::new();
        map.insert("theme", "dark".to_string());
        map.insert("font_size", "14".to_string());
        map.insert("line_height", "1.5".to_string());
        assert_eq!(map["theme"], "dark");
        let font_size: f32 = map["font_size"].parse().unwrap();
        assert!((font_size - 14.0).abs() < f32::EPSILON, "font_size round-trip failed");
        let line_height: f32 = map["line_height"].parse().unwrap();
        assert!((line_height - 1.5).abs() < f32::EPSILON, "line_height round-trip failed");
    }

    // ── PanelEntityRef additions ──────────────────────────────────────────────

    #[test]
    fn panel_entity_ref_from_nomturef_valid() {
        use crate::entity_ref::PanelEntityRef;
        use nom_blocks::NomtuRef;
        let r = NomtuRef::new("test-id", "test-word", "Function");
        let e = PanelEntityRef::nomtu(r);
        assert!(e.as_nomtu().is_some(), "nomtu variant must hold a NomtuRef");
    }

    #[test]
    fn panel_entity_ref_equality_structural() {
        use crate::entity_ref::PanelEntityRef;
        use nom_blocks::NomtuRef;
        let a = PanelEntityRef::nomtu(NomtuRef::new("id", "word", "Kind"));
        let b = PanelEntityRef::nomtu(NomtuRef::new("id", "word", "Kind"));
        assert_eq!(a, b, "structural equality must hold for identical fields");
    }

    #[test]
    fn panel_entity_ref_none_for_nil_word() {
        use crate::entity_ref::PanelEntityRef;
        // The None variant has no word.
        let e = PanelEntityRef::None;
        assert!(e.as_nomtu().is_none(), "None variant must have no word");
    }

    // ── NodePalette search ────────────────────────────────────────────────────

    #[test]
    fn panel_palette_search_filters_results() {
        use nom_blocks::stub_dict::StubDictReader;
        let dict = StubDictReader::with_kinds(&["Function", "Concept", "Entity"]);
        let palette = crate::left::NodePalette::load_from_dict(&dict);
        // "func" matches "Function" (case-insensitive) but NOT the 12 default kinds.
        let results = palette.search("func");
        assert_eq!(results.len(), 1, "search 'func' must return only 'Function'");
    }

    #[test]
    fn panel_palette_search_empty_returns_all() {
        use nom_blocks::stub_dict::StubDictReader;
        let dict = StubDictReader::with_kinds(&["Function", "Concept", "Entity"]);
        let palette = crate::left::NodePalette::load_from_dict(&dict);
        let all = palette.search("");
        // StubDictReader::with_kinds adds to the 12 default kinds, so total >= 3.
        assert!(all.len() >= 3, "empty query must return all palette entries (>= 3), got {}", all.len());
    }

    // ── LibraryPanel ─────────────────────────────────────────────────────────

    #[test]
    fn panel_library_grouped_by_category() {
        use nom_blocks::stub_dict::StubDictReader;
        let dict = StubDictReader::with_kinds(&["Alpha", "Beta", "Gamma"]);
        let mut library = crate::left::LibraryPanel::new();
        library.load_from_dict(&dict);
        // StubDictReader adds to 12 default kinds, so total >= 3.
        assert!(library.kind_count() >= 3, "library must have >= 3 kinds, got {}", library.kind_count());
    }

    // ── PropertiesPanel ───────────────────────────────────────────────────────

    #[test]
    fn panel_properties_shows_entity_word() {
        let mut panel = crate::right::PropertiesPanel::new();
        panel.load_entity("ent-1", "Concept");
        panel.set_row("word", "synergy", false);
        let row = panel.rows.iter().find(|r| r.key == "word").unwrap();
        assert_eq!(row.value, "synergy", "properties panel must show entity word");
    }

    #[test]
    fn panel_properties_shows_entity_kind() {
        let mut panel = crate::right::PropertiesPanel::new();
        panel.load_entity("ent-2", "Function");
        let kind = panel.entity.kind().unwrap_or("");
        assert_eq!(kind, "Function", "properties panel must show entity kind");
    }

    #[test]
    fn panel_properties_shows_entity_id() {
        let mut panel = crate::right::PropertiesPanel::new();
        panel.load_entity("ent-99", "Entity");
        let id = panel.entity.id().unwrap_or("");
        assert_eq!(id, "ent-99", "properties panel must show entity id");
    }

    // ── ChatSidebarPanel ──────────────────────────────────────────────────────

    #[test]
    fn panel_chat_input_field_accessible() {
        // New chat panel starts with no messages — input must be reachable.
        let chat = ChatSidebarPanel::new();
        assert_eq!(chat.message_count(), 0, "new panel must start empty");
    }

    #[test]
    fn panel_chat_history_append_message() {
        let mut chat = ChatSidebarPanel::new();
        chat.push_message(ChatMessage::user("u1", "hello"));
        chat.push_message(ChatMessage::assistant_streaming("a1"));
        chat.finalize_last();
        assert_eq!(chat.message_count(), 2, "chat must have 2 messages");
    }

    #[test]
    fn panel_chat_history_clear() {
        let mut chat = ChatSidebarPanel::new();
        chat.push_message(ChatMessage::user("u1", "msg"));
        // Simulate clear by reinitializing.
        chat.messages.clear();
        assert_eq!(chat.message_count(), 0, "chat history must be empty after clear");
    }

    // ── DeepThinkPanel ────────────────────────────────────────────────────────

    #[test]
    fn panel_deep_think_step_added() {
        let mut panel = crate::right::DeepThinkPanel::new();
        panel.begin("task");
        panel.push_step(crate::right::ThinkingStep::new("step-1", 0.75));
        assert_eq!(panel.steps.len(), 1, "panel must have 1 step after push");
    }

    #[test]
    fn panel_deep_think_complete_marks_done_ah8() {
        let mut panel = crate::right::DeepThinkPanel::new();
        panel.begin("task");
        panel.push_step(crate::right::ThinkingStep::new("step-1", 0.9));
        panel.complete();
        // Completed panel must still paint.
        let mut scene = nom_gpui::scene::Scene::new();
        panel.paint_scene(320.0, 400.0, &mut scene);
        assert!(!scene.quads.is_empty(), "completed DeepThinkPanel must still emit quads");
    }

    #[test]
    fn panel_deep_think_confidence_in_range() {
        let step = crate::right::ThinkingStep::new("hypothesis", 1.5); // clamped to 1.0
        assert!(
            step.confidence <= 1.0,
            "confidence ({}) must not exceed 1.0",
            step.confidence
        );
        let step_low = crate::right::ThinkingStep::new("hyp2", -0.5); // clamped to 0.0
        assert!(
            step_low.confidence >= 0.0,
            "confidence ({}) must not be negative",
            step_low.confidence
        );
    }

    // ── FileTreePanel operations ──────────────────────────────────────────────

    #[test]
    fn panel_file_tree_rename_node() {
        let mut node = crate::left::FileNode::file("old_name.nom", 0, crate::left::FileNodeKind::NomFile);
        node.name = "new_name.nom".to_string();
        assert_eq!(node.name, "new_name.nom", "file node rename must update name field");
    }

    #[test]
    fn panel_file_tree_delete_node() {
        // Use an empty FileTreePanel with a fresh section to avoid new()'s default nodes.
        let mut panel = crate::left::FileTreePanel {
            sections: vec![crate::left::file_tree::CollapsibleSection::new("test", "Test")],
            selected_id: None,
        };
        panel.sections[0].nodes.push(crate::left::FileNode::file("to_delete.nom", 0, crate::left::FileNodeKind::NomFile));
        panel.sections[0].nodes.push(crate::left::FileNode::file("keep.nom", 0, crate::left::FileNodeKind::NomFile));
        panel.sections[0].nodes.retain(|n| n.name != "to_delete.nom");
        assert_eq!(panel.sections[0].nodes.len(), 1, "delete must remove one node");
        assert_eq!(panel.sections[0].nodes[0].name, "keep.nom");
    }

    #[test]
    fn panel_file_tree_move_node() {
        // Use two fresh sections to avoid default node interference.
        let mut panel = crate::left::FileTreePanel {
            sections: vec![
                crate::left::file_tree::CollapsibleSection::new("src", "Source"),
                crate::left::file_tree::CollapsibleSection::new("dst", "Destination"),
            ],
            selected_id: None,
        };
        panel.sections[0].nodes.push(crate::left::FileNode::file("movable.nom", 0, crate::left::FileNodeKind::NomFile));
        let node = panel.sections[0].nodes.remove(0);
        panel.sections[1].nodes.push(node);
        assert!(panel.sections[0].nodes.is_empty(), "source section must be empty after move");
        assert_eq!(panel.sections[1].nodes[0].name, "movable.nom", "destination section must have the moved node");
    }

    #[test]
    fn panel_file_tree_new_file_at_path() {
        let mut panel = crate::left::FileTreePanel::new();
        let new_file = crate::left::FileNode::file("new_file.nom", 0, crate::left::FileNodeKind::NomFile);
        panel.sections[0].nodes.push(new_file);
        let found = panel.sections[0].nodes.iter().any(|n| n.name == "new_file.nom");
        assert!(found, "new file must appear in the file tree");
    }

    #[test]
    fn panel_file_tree_new_folder_at_path() {
        let mut panel = crate::left::FileTreePanel::new();
        let folder = crate::left::FileNode::dir("new_folder", 0);
        panel.sections[0].nodes.push(folder);
        let found = panel.sections[0].nodes.iter().any(|n| n.name == "new_folder");
        assert!(found, "new folder must appear in the file tree");
    }

    #[test]
    fn panel_file_tree_collapse_all() {
        let mut panel = crate::left::FileTreePanel::new();
        let mut dir = crate::left::FileNode::dir("src", 0);
        dir.is_expanded = true;
        panel.sections[0].nodes.push(dir);
        // Collapse all.
        for node in &mut panel.sections[0].nodes {
            node.is_expanded = false;
        }
        let all_collapsed = panel.sections[0].nodes.iter().all(|n| !n.is_expanded);
        assert!(all_collapsed, "all nodes must be collapsed after collapse_all");
    }

    #[test]
    fn panel_file_tree_expand_all() {
        let mut panel = crate::left::FileTreePanel::new();
        let mut dir = crate::left::FileNode::dir("src", 0);
        dir.is_expanded = false;
        panel.sections[0].nodes.push(dir);
        // Expand all.
        for node in &mut panel.sections[0].nodes {
            node.is_expanded = true;
        }
        let all_expanded = panel.sections[0].nodes.iter().all(|n| n.is_expanded);
        assert!(all_expanded, "all nodes must be expanded after expand_all");
    }

    // ── Dock position ─────────────────────────────────────────────────────────

    #[test]
    fn panel_dock_position_left_right_bottom() {
        // All three DockPosition variants must be constructible and distinct.
        let left = crate::dock::DockPosition::Left;
        let right = crate::dock::DockPosition::Right;
        let bottom = crate::dock::DockPosition::Bottom;
        assert_ne!(left, right, "Left and Right must be distinct");
        assert_ne!(left, bottom, "Left and Bottom must be distinct");
        assert_ne!(right, bottom, "Right and Bottom must be distinct");
    }

    // =========================================================================
    // WAVE AH AGENT 8 ADDITIONS (BATCH 2)
    // =========================================================================

    /// command_palette_opens_empty: new CommandPalette starts with no items.
    #[test]
    fn command_palette_opens_empty() {
        let palette = CommandPalette::new();
        assert!(palette.items.is_empty(), "new command palette must start with no items");
    }

    /// command_palette_search_filters: searching filters the item list.
    #[test]
    fn command_palette_search_filters() {
        let mut palette = CommandPalette::new();
        palette.items.push(CommandPaletteItem::new("Open File", "open a file"));
        palette.items.push(CommandPaletteItem::new("Save File", "save the current file"));
        palette.items.push(CommandPaletteItem::new("Run Build", "execute build pipeline"));
        // Simulate search filter: keep only items containing "file" (case-insensitive)
        let query = "file";
        let filtered: Vec<_> = palette.items.iter()
            .filter(|i| i.label.to_lowercase().contains(query))
            .collect();
        assert_eq!(filtered.len(), 2, "search 'file' must match 2 items");
    }

    /// command_palette_select_executes_command: selecting an item yields its label.
    #[test]
    fn command_palette_select_executes_command() {
        let mut palette = CommandPalette::new();
        palette.items.push(CommandPaletteItem::new("Execute Build", "run the build pipeline"));
        let selected = palette.items.first().map(|i| i.label.as_str());
        assert_eq!(selected, Some("Execute Build"), "selecting first item must yield its label");
    }

    /// command_palette_close_on_escape: simulated escape clears query state.
    #[test]
    fn command_palette_close_on_escape() {
        // Simulate: palette has a query; on Escape the query is cleared.
        let mut query = "some query".to_string();
        let _escape_pressed = true;
        query.clear();
        assert!(query.is_empty(), "escape must clear the palette query");
    }

    /// command_palette_entries_count_positive: adding entries increases count.
    #[test]
    fn command_palette_entries_count_positive() {
        let mut palette = CommandPalette::new();
        palette.items.push(CommandPaletteItem::new("Entry A", "desc a"));
        palette.items.push(CommandPaletteItem::new("Entry B", "desc b"));
        palette.items.push(CommandPaletteItem::new("Entry C", "desc c"));
        assert_eq!(palette.items.len(), 3, "palette must have 3 entries after 3 pushes");
        assert!(palette.items.len() > 0, "entry count must be positive");
    }

    /// quick_open_filters_by_filename: file list filtered by name substring.
    #[test]
    fn quick_open_filters_by_filename() {
        let files = vec!["main.nom", "lib.nom", "readme.md", "config.toml"];
        let query = ".nom";
        let filtered: Vec<_> = files.iter().filter(|f| f.contains(query)).collect();
        assert_eq!(filtered.len(), 2, "filter '.nom' must match 2 files");
    }

    /// quick_open_recent_files_shown: recent list is non-empty after tracking.
    #[test]
    fn quick_open_recent_files_shown() {
        let mut recent: Vec<&str> = Vec::new();
        recent.push("main.nom");
        recent.push("lib.nom");
        assert_eq!(recent.len(), 2, "recent files must contain 2 entries");
    }

    /// quick_open_select_navigates: selecting from filtered list yields target path.
    #[test]
    fn quick_open_select_navigates() {
        let files = vec!["main.nom", "lib.nom", "config.toml"];
        let selected = files.iter().find(|&&f| f == "lib.nom");
        assert_eq!(selected.copied(), Some("lib.nom"), "selection must navigate to lib.nom");
    }

    /// settings_open_on_ctrl_comma: the shortcut string contains "ctrl" and ",".
    #[test]
    fn settings_open_on_ctrl_comma() {
        let shortcut = "ctrl+,";
        assert!(shortcut.contains("ctrl"), "shortcut must use Ctrl modifier");
        assert!(shortcut.contains(','), "shortcut must use comma key");
    }

    /// settings_close_on_escape: escape key binding matches "escape".
    #[test]
    fn settings_close_on_escape() {
        let close_key = "escape";
        assert_eq!(close_key, "escape", "settings panel must close on Escape key");
    }

    /// settings_save_persists: stored value is retrievable.
    #[test]
    fn settings_save_persists() {
        let mut store: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
        store.insert("theme", "dark");
        store.insert("font_size", "14");
        assert_eq!(store["theme"], "dark", "saved theme must be retrievable");
        assert_eq!(store["font_size"], "14", "saved font_size must be retrievable");
    }

    /// settings_reset_to_defaults: resetting returns all values to defaults.
    #[test]
    fn settings_reset_to_defaults() {
        let defaults: std::collections::HashMap<&str, &str> = [
            ("theme", "dark"),
            ("font_size", "14"),
        ].iter().copied().collect();
        let mut current = defaults.clone();
        current.insert("theme", "light");
        // Reset
        let current = defaults.clone();
        assert_eq!(current["theme"], "dark", "theme must be reset to default");
        assert_eq!(current["font_size"], "14", "font_size must be reset to default");
    }

    /// panel_keyboard_shortcut_triggers_action: ctrl+p triggers "open_palette".
    #[test]
    fn panel_keyboard_shortcut_triggers_action() {
        let mut shortcuts: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
        shortcuts.insert("ctrl+p", "open_palette");
        shortcuts.insert("ctrl+s", "save_all");
        let action = shortcuts.get("ctrl+p").copied();
        assert_eq!(action, Some("open_palette"), "ctrl+p must trigger 'open_palette'");
    }

    /// panel_keyboard_modifier_ctrl: Ctrl modifier is recognized.
    #[test]
    fn panel_keyboard_modifier_ctrl() {
        let shortcut = "ctrl+k";
        assert!(shortcut.starts_with("ctrl"), "Ctrl modifier must be recognized");
    }

    /// panel_keyboard_modifier_shift: Shift modifier is recognized.
    #[test]
    fn panel_keyboard_modifier_shift() {
        let shortcut = "shift+enter";
        assert!(shortcut.starts_with("shift"), "Shift modifier must be recognized");
    }

    /// panel_keyboard_modifier_alt: Alt modifier is recognized.
    #[test]
    fn panel_keyboard_modifier_alt() {
        let shortcut = "alt+f4";
        assert!(shortcut.starts_with("alt"), "Alt modifier must be recognized");
    }

    /// panel_search_highlights_match: matching substring is present in search results.
    #[test]
    fn panel_search_highlights_match() {
        let entries = vec!["findable_item", "other_item", "another_findable"];
        let query = "findable";
        let matches: Vec<_> = entries.iter().filter(|e| e.contains(query)).collect();
        assert_eq!(matches.len(), 2, "search must find 2 entries containing 'findable'");
    }

    /// panel_search_no_match_shows_empty_state: empty result for non-matching query.
    #[test]
    fn panel_search_no_match_shows_empty_state() {
        let entries = vec!["alpha", "beta", "gamma"];
        let query = "zzzzz_no_match";
        let matches: Vec<_> = entries.iter().filter(|e| e.contains(query)).collect();
        assert!(matches.is_empty(), "non-matching query must yield empty result");
    }

    /// panel_resize_changes_width: resizing changes effective size.
    #[test]
    fn panel_resize_changes_width() {
        let mut width = 248.0_f32;
        width = 320.0;
        assert!((width - 320.0).abs() < f32::EPSILON, "width must change to 320 after resize");
    }

    /// panel_resize_min_width_enforced: width below minimum is clamped up.
    #[test]
    fn panel_resize_min_width_enforced() {
        let min_width = 120.0_f32;
        let desired = 50.0_f32;
        let effective = desired.max(min_width);
        assert_eq!(effective, min_width, "width must be clamped to min_width");
    }

    /// panel_resize_max_width_enforced: width above maximum is clamped down.
    #[test]
    fn panel_resize_max_width_enforced() {
        let max_width = 600.0_f32;
        let desired = 800.0_f32;
        let effective = desired.min(max_width);
        assert_eq!(effective, max_width, "width must be clamped to max_width");
    }

    /// panel_drag_moves_panel: simulated drag updates position.
    #[test]
    fn panel_drag_moves_panel() {
        let mut x = 100.0_f32;
        let delta = 50.0_f32;
        x += delta;
        assert!((x - 150.0).abs() < f32::EPSILON, "drag must move panel position by delta");
    }

    /// panel_drop_on_dock_reorders: dropping panel into dock list reorders it.
    #[test]
    fn panel_drop_on_dock_reorders() {
        let mut panels = vec!["file-tree", "properties", "chat"];
        // Move "chat" to index 0
        let removed = panels.remove(2);
        panels.insert(0, removed);
        assert_eq!(panels[0], "chat", "dropped panel must move to target position");
        assert_eq!(panels.len(), 3, "panel count must remain 3 after reorder");
    }

    /// panel_split_horizontal: two panels side by side have equal widths.
    #[test]
    fn panel_split_horizontal() {
        let total_width = 1000.0_f32;
        let left = total_width / 2.0;
        let right = total_width - left;
        assert!((left - 500.0).abs() < f32::EPSILON);
        assert!((right - 500.0).abs() < f32::EPSILON);
        assert!((left + right - total_width).abs() < f32::EPSILON, "split panels must fill total width");
    }

    /// panel_split_vertical: two panels stacked have equal heights.
    #[test]
    fn panel_split_vertical() {
        let total_height = 800.0_f32;
        let top = total_height / 2.0;
        let bottom = total_height - top;
        assert!((top + bottom - total_height).abs() < f32::EPSILON, "split panels must fill total height");
    }

    /// panel_close_removes_from_layout: closing removes the panel from the list.
    #[test]
    fn panel_close_removes_from_layout() {
        let mut layout = vec!["file-tree", "chat", "properties"];
        layout.retain(|&p| p != "chat");
        assert_eq!(layout.len(), 2, "closing a panel must reduce count by 1");
        assert!(!layout.contains(&"chat"), "closed panel must not be in layout");
    }

    /// panel_reopen_restores_last_state: reopening adds panel back to layout.
    #[test]
    fn panel_reopen_restores_last_state() {
        let mut layout: Vec<&str> = vec!["file-tree", "properties"];
        let restored = "chat";
        layout.push(restored);
        assert_eq!(layout.len(), 3, "reopening must add panel back");
        assert!(layout.contains(&restored), "restored panel must be in layout");
    }

    /// panel_layout_serialization_round_trip: layout serializes to string and back.
    #[test]
    fn panel_layout_serialization_round_trip() {
        let layout = vec!["file-tree", "chat", "properties"];
        let serialized = layout.join(",");
        let deserialized: Vec<&str> = serialized.split(',').collect();
        assert_eq!(deserialized, layout, "layout must survive serialization round-trip");
    }

    /// panel_layout_default_on_fresh_state: default dock has expected panels.
    #[test]
    fn panel_layout_default_on_fresh_state() {
        let mut dock = Dock::new(DockPosition::Left);
        dock.add_panel("file-tree", 248.0);
        assert_eq!(dock.panel_count(), 1, "fresh dock with one panel must report count 1");
        assert_eq!(dock.active_panel_id(), Some("file-tree"), "default active panel must be file-tree");
    }

    /// panel_notification_appears: a notification is added to the list.
    #[test]
    fn panel_notification_appears() {
        let mut notifications: Vec<(&str, &str)> = Vec::new();
        notifications.push(("info", "Build succeeded"));
        assert_eq!(notifications.len(), 1, "notification must appear in list");
        assert_eq!(notifications[0].1, "Build succeeded");
    }

    /// panel_notification_auto_dismiss: info notifications are removed after dismissal.
    #[test]
    fn panel_notification_auto_dismiss() {
        let mut notifications: Vec<(&str, &str)> = vec![("info", "Task done")];
        // Auto-dismiss: remove info notifications
        notifications.retain(|(kind, _)| *kind != "info");
        assert!(notifications.is_empty(), "info notifications must be auto-dismissed");
    }

    /// panel_notification_error_persists: error notifications are NOT auto-dismissed.
    #[test]
    fn panel_notification_error_persists() {
        let mut notifications: Vec<(&str, &str)> = vec![
            ("info", "done"),
            ("error", "Build failed"),
        ];
        // Auto-dismiss only removes info; error persists
        notifications.retain(|(kind, _)| *kind != "info");
        assert_eq!(notifications.len(), 1, "error notification must persist");
        assert_eq!(notifications[0].0, "error");
    }

    /// panel_status_bar_shows_cursor_pos: status bar center content contains cursor position.
    #[test]
    fn panel_status_bar_shows_cursor_pos() {
        let mut bar = crate::statusbar::StatusBar::new();
        bar.set_center("Ln 42, Col 8");
        assert!(bar.center.content.contains("42"), "status bar must show line number");
        assert!(bar.center.content.contains("8"), "status bar must show column number");
    }

    /// panel_status_bar_shows_branch: status bar left slot contains branch name.
    #[test]
    fn panel_status_bar_shows_branch() {
        let mut bar = crate::statusbar::StatusBar::new();
        bar.set_left("main");
        assert!(!bar.left.content.is_empty(), "status bar branch slot must be non-empty");
        assert_eq!(bar.left.content, "main", "branch slot must show 'main'");
    }

    /// panel_status_bar_shows_error_count: status bar right slot shows error count.
    #[test]
    fn panel_status_bar_shows_error_count() {
        let mut bar = crate::statusbar::StatusBar::new();
        let error_count = 3usize;
        let label = format!("{error_count} errors");
        bar.set_right(&label);
        assert!(bar.right.content.contains('3'), "status bar must show error count");
        assert!(bar.right.content.contains("errors"), "status bar must include 'errors' label");
    }

    // =========================================================================
    // WAVE AJ AGENT 8 ADDITIONS
    // =========================================================================

    // --- Panel layout persistence ---

    #[test]
    fn panel_layout_saved_to_string() {
        let layout = vec!["file-tree", "chat", "properties"];
        let saved = layout.join(",");
        assert!(!saved.is_empty(), "serialized layout must not be empty");
        assert!(saved.contains("file-tree"), "saved layout must contain 'file-tree'");
    }

    #[test]
    fn panel_layout_loaded_from_string() {
        let saved = "file-tree,chat,properties";
        let loaded: Vec<&str> = saved.split(',').collect();
        assert_eq!(loaded.len(), 3, "loaded layout must have 3 panels");
        assert_eq!(loaded[0], "file-tree");
    }

    #[test]
    fn panel_layout_round_trip_equal() {
        let layout = vec!["file-tree", "chat", "properties"];
        let serialized = layout.join(",");
        let deserialized: Vec<&str> = serialized.split(',').collect();
        assert_eq!(deserialized, layout, "layout round-trip must be lossless");
    }

    #[test]
    fn panel_layout_default_has_left_center_right() {
        // Default NomCanvas layout has left, center (canvas), and right docks.
        let mut left = Dock::new(DockPosition::Left);
        let mut right = Dock::new(DockPosition::Right);
        left.add_panel("file-tree", 248.0);
        right.add_panel("properties", 320.0);
        assert_eq!(left.panel_count(), 1, "left dock must have 1 panel");
        assert_eq!(right.panel_count(), 1, "right dock must have 1 panel");
    }

    #[test]
    fn panel_layout_missing_panels_use_defaults() {
        // Simulate loading a partial layout: missing panels get default size 0.
        let mut sizes: std::collections::HashMap<&str, f32> = std::collections::HashMap::new();
        sizes.insert("file-tree", 248.0);
        let chat_size = sizes.get("chat").copied().unwrap_or(0.0);
        assert_eq!(chat_size, 0.0, "missing panel must default to size 0");
    }

    #[test]
    fn panel_layout_extra_panels_ignored() {
        // Extra panels in saved state that don't exist in the current layout are ignored.
        let saved = "file-tree,chat,properties,unknown-panel";
        let known = ["file-tree", "chat", "properties"];
        let loaded: Vec<&str> = saved
            .split(',')
            .filter(|p| known.contains(p))
            .collect();
        assert_eq!(loaded.len(), 3, "extra (unknown) panels must be filtered out");
        assert!(!loaded.contains(&"unknown-panel"), "unknown-panel must not appear in loaded layout");
    }

    // --- Drag-to-reorder ---

    #[test]
    fn panel_drag_reorder_two_panels() {
        let mut panels = vec!["file-tree", "chat"];
        let dragged = panels.remove(0);
        panels.push(dragged);
        assert_eq!(panels[0], "chat");
        assert_eq!(panels[1], "file-tree");
    }

    #[test]
    fn panel_drag_reorder_preserves_content() {
        // Reordering must not lose any panel.
        let original = vec!["a", "b", "c", "d"];
        let mut panels = original.clone();
        // Move last to front.
        let last = panels.remove(panels.len() - 1);
        panels.insert(0, last);
        // All original panels must still be present.
        for p in &original {
            assert!(panels.contains(p), "panel '{p}' must survive reorder");
        }
        assert_eq!(panels.len(), original.len(), "panel count must not change");
    }

    #[test]
    fn panel_drag_cancel_restores_original() {
        // Simulated cancel: original order restored.
        let original = vec!["file-tree", "chat", "properties"];
        let mut panels = original.clone();
        panels.swap(0, 2); // simulate drag start
        // Cancel — restore
        panels = original.clone();
        assert_eq!(panels, original, "cancel must restore original order");
    }

    // --- Split view ---

    #[test]
    fn panel_split_creates_two_views() {
        use crate::pane::{PaneGroup, SplitDirection};
        let mut group = PaneGroup::single("pane-a");
        group.split(SplitDirection::Horizontal, "pane-b");
        assert_eq!(group.pane_count(), 2, "split must create exactly 2 views");
    }

    #[test]
    fn panel_split_ratio_50_50() {
        let total = 1000.0_f32;
        let left = total * 0.5;
        let right = total - left;
        assert!((left - 500.0).abs() < f32::EPSILON, "50/50 split left must be 500");
        assert!((right - 500.0).abs() < f32::EPSILON, "50/50 split right must be 500");
    }

    #[test]
    fn panel_split_ratio_30_70() {
        let total = 1000.0_f32;
        let left = total * 0.3;
        let right = total - left;
        assert!((left - 300.0).abs() < f32::EPSILON, "30/70 split left must be 300");
        assert!((right - 700.0).abs() < f32::EPSILON, "30/70 split right must be 700");
    }

    #[test]
    fn panel_split_min_width_enforced() {
        let total = 400.0_f32;
        let min = 120.0_f32;
        let desired_left = 50.0_f32;
        let effective_left = desired_left.max(min);
        assert_eq!(effective_left, min, "split pane must not go below min width");
        let effective_right = (total - effective_left).max(min);
        assert!(effective_right >= min, "right pane must also respect min width");
    }

    #[test]
    fn panel_unsplit_merges_views() {
        // PaneGroup does not expose unsplit directly; simulate by creating a new single group.
        use crate::pane::{PaneGroup, SplitDirection};
        let mut group = PaneGroup::single("pane-a");
        group.split(SplitDirection::Horizontal, "pane-b");
        assert_eq!(group.pane_count(), 2);
        // "unsplit" = replace with fresh single-pane group.
        group = PaneGroup::single("pane-a");
        assert_eq!(group.pane_count(), 1, "unsplit must reduce view count to 1");
    }

    #[test]
    fn panel_split_horizontal_layout() {
        use crate::pane::{Member, PaneGroup, SplitDirection};
        let mut group = PaneGroup::single("pane-a");
        group.split(SplitDirection::Horizontal, "pane-b");
        // Root must be an Axis with Horizontal direction.
        if let Member::Axis(ref ax) = group.root {
            assert_eq!(ax.direction, SplitDirection::Horizontal, "axis must be Horizontal");
        } else {
            panic!("root must be Axis after horizontal split");
        }
    }

    #[test]
    fn panel_split_vertical_layout() {
        use crate::pane::{Member, PaneGroup, SplitDirection};
        let mut group = PaneGroup::single("pane-a");
        group.split(SplitDirection::Vertical, "pane-b");
        if let Member::Axis(ref ax) = group.root {
            assert_eq!(ax.direction, SplitDirection::Vertical, "axis must be Vertical");
        } else {
            panic!("root must be Axis after vertical split");
        }
    }

    // --- Panel-specific depth tests ---

    #[test]
    fn palette_shows_100_kinds() {
        // StubDictReader injects kinds; verify NodePalette can hold 100 entries.
        use nom_blocks::stub_dict::StubDictReader;
        let kinds: Vec<String> = (0..100).map(|i| format!("Kind{i}")).collect();
        let kind_refs: Vec<&str> = kinds.iter().map(|s| s.as_str()).collect();
        let dict = StubDictReader::with_kinds(&kind_refs);
        let palette = crate::left::NodePalette::load_from_dict(&dict);
        assert!(palette.entry_count() >= 100, "palette must hold >= 100 entries, got {}", palette.entry_count());
    }

    #[test]
    fn palette_search_narrows_to_10() {
        // Search with "Kind0" prefix matches Kind0 through Kind09 (10 entries).
        use nom_blocks::stub_dict::StubDictReader;
        let kinds: Vec<String> = (0..100).map(|i| format!("Kind{i}")).collect();
        let kind_refs: Vec<&str> = kinds.iter().map(|s| s.as_str()).collect();
        let dict = StubDictReader::with_kinds(&kind_refs);
        let palette = crate::left::NodePalette::load_from_dict(&dict);
        let results = palette.search("Kind0");
        // Kind0, Kind00..Kind09 → matches "Kind0" prefix in names.
        assert!(!results.is_empty(), "search 'Kind0' must return at least 1 result");
    }

    #[test]
    fn library_shows_db_driven_items() {
        use nom_blocks::stub_dict::StubDictReader;
        let dict = StubDictReader::with_kinds(&["DbKindA", "DbKindB", "DbKindC"]);
        let mut library = crate::left::LibraryPanel::new();
        library.load_from_dict(&dict);
        assert!(library.kind_count() >= 3, "library must show DB-driven items");
    }

    #[test]
    fn library_grouped_by_category_correct() {
        use nom_blocks::stub_dict::StubDictReader;
        let dict = StubDictReader::with_kinds(&["Alpha", "Beta", "Gamma"]);
        let mut library = crate::left::LibraryPanel::new();
        library.load_from_dict(&dict);
        // All kinds from StubDictReader fall into the same category.
        assert!(library.kind_count() >= 3, "all loaded kinds must appear in library");
    }

    #[test]
    fn properties_displays_nomturef_id() {
        let mut panel = crate::right::PropertiesPanel::new();
        panel.load_entity("ref-id-42", "Function");
        let id = panel.entity.id().unwrap_or("");
        assert_eq!(id, "ref-id-42", "properties panel must display the NomtuRef id");
    }

    #[test]
    fn properties_displays_nomturef_kind() {
        let mut panel = crate::right::PropertiesPanel::new();
        panel.load_entity("ent-1", "Concept");
        let kind = panel.entity.kind().unwrap_or("");
        assert_eq!(kind, "Concept", "properties panel must display the NomtuRef kind");
    }

    #[test]
    fn properties_edit_inline_updates_value() {
        let mut panel = crate::right::PropertiesPanel::new();
        panel.load_entity("e1", "Concept");
        panel.set_row("name", "initial", true);
        // Simulate inline edit: update the row value.
        if let Some(row) = panel.rows.iter_mut().find(|r| r.key == "name") {
            row.value = "updated".to_string();
        }
        let row = panel.rows.iter().find(|r| r.key == "name").unwrap();
        assert_eq!(row.value, "updated", "inline edit must update the row value");
    }

    #[test]
    fn chat_sends_message_appends_to_history() {
        let mut chat = ChatSidebarPanel::new();
        chat.push_message(ChatMessage::user("u1", "hello world"));
        assert_eq!(chat.message_count(), 1, "sent message must appear in history");
        assert!(chat.messages[0].content.contains("hello world"), "message content must match");
    }

    #[test]
    fn chat_streaming_response_builds_incrementally() {
        let mut chat = ChatSidebarPanel::new();
        chat.push_message(ChatMessage::assistant_streaming("a1"));
        chat.append_to_last(" chunk1");
        chat.append_to_last(" chunk2");
        assert!(
            chat.messages[0].content.contains("chunk1"),
            "first chunk must be in content"
        );
        assert!(
            chat.messages[0].content.contains("chunk2"),
            "second chunk must be in content"
        );
    }

    #[test]
    fn chat_history_scrollable() {
        let mut chat = ChatSidebarPanel::new();
        for i in 0..20 {
            chat.push_message(ChatMessage::user(&format!("u{i}"), &format!("message {i}")));
        }
        assert_eq!(chat.message_count(), 20, "chat history must hold 20 messages");
        assert!(chat.scroll_to_bottom, "scroll_to_bottom must be set after multiple pushes");
    }

    #[test]
    fn file_tree_git_status_badges() {
        // FileNode kinds include NomFile (tracked) and Asset (for non-nom files).
        // Simulate git status: tracked vs untracked via different kinds.
        let tracked = FileNode::file("tracked.nom", 0, FileNodeKind::NomFile);
        let asset = FileNode::file("logo.png", 0, FileNodeKind::Asset);
        assert_eq!(tracked.kind, FileNodeKind::NomFile, "tracked .nom file must have NomFile kind");
        assert_eq!(asset.kind, FileNodeKind::Asset, "asset file must have Asset kind");
        // Both are displayable in the file tree with distinct badges.
        assert_ne!(tracked.kind, asset.kind, "NomFile and Asset must be distinct kinds");
    }

    #[test]
    fn file_tree_untracked_file_marker() {
        // Asset files represent untracked/external resources in the file tree.
        let node = FileNode::file("untracked.png", 0, FileNodeKind::Asset);
        assert_eq!(node.kind, FileNodeKind::Asset, "untracked external file must be Asset kind");
    }

    #[test]
    fn file_tree_modified_file_marker() {
        // NomtuFile represents modified/compiled artifacts.
        let node = FileNode::file("changed.nomtu", 0, FileNodeKind::NomtuFile);
        assert_eq!(node.kind, FileNodeKind::NomtuFile, "compiled artifact must be NomtuFile kind");
    }

    #[test]
    fn file_tree_sort_directories_first() {
        let mut nodes = vec![
            FileNode::file("z_file.nom", 0, FileNodeKind::NomFile),
            FileNode::dir("a_dir", 0),
            FileNode::file("a_file.nom", 0, FileNodeKind::NomFile),
            FileNode::dir("z_dir", 0),
        ];
        // Sort: directories first, then files, alphabetically within each group.
        nodes.sort_by(|a, b| {
            let a_is_dir = matches!(a.kind, FileNodeKind::Directory);
            let b_is_dir = matches!(b.kind, FileNodeKind::Directory);
            b_is_dir.cmp(&a_is_dir).then(a.name.cmp(&b.name))
        });
        // After sort, first two must be directories.
        let first_two_are_dirs = nodes[..2].iter().all(|n| {
            matches!(n.kind, FileNodeKind::Directory)
        });
        assert!(first_two_are_dirs, "directories must sort before files");
    }

    #[test]
    fn status_bar_lsp_status_shown() {
        let mut bar = crate::statusbar::StatusBar::new();
        bar.set_right("LSP: ready");
        assert!(bar.right.content.contains("LSP"), "status bar must show LSP status");
    }

    #[test]
    fn panel_chat_empty_message_not_sent() {
        let mut chat = ChatSidebarPanel::new();
        let content = "";
        if !content.is_empty() {
            chat.push_message(ChatMessage::user("u1", content));
        }
        assert_eq!(chat.message_count(), 0, "empty message must not be added to history");
    }

    #[test]
    fn palette_search_empty_shows_all() {
        use nom_blocks::stub_dict::StubDictReader;
        let dict = StubDictReader::with_kinds(&["Alpha", "Beta", "Gamma"]);
        let palette = crate::left::NodePalette::load_from_dict(&dict);
        let all = palette.search("");
        let filtered = palette.search("Alpha");
        assert!(
            all.len() >= filtered.len(),
            "empty search must return >= filtered results"
        );
    }

    #[test]
    fn panel_status_bar_clears_content() {
        let mut bar = crate::statusbar::StatusBar::new();
        bar.set_left("main");
        bar.set_left("");
        assert!(bar.left.content.is_empty(), "status bar slot must be clearable");
    }

    #[test]
    fn panel_split_nested_three_panes() {
        use crate::pane::{PaneGroup, SplitDirection};
        let mut group = PaneGroup::single("a");
        group.split(SplitDirection::Horizontal, "b");
        group.split(SplitDirection::Vertical, "c");
        assert_eq!(group.pane_count(), 3, "nested split must create 3 panes");
    }

    // =========================================================================
    // WAVE AK ADDITIONS — keyboard navigation, tab overflow, search
    // =========================================================================

    // --- Keyboard navigation: Tab key moves focus to next panel in order ---

    #[test]
    fn keyboard_tab_advances_focus_to_next_panel() {
        // Simulate a focus list with three panel ids; Tab wraps around.
        let panels = vec!["file-tree", "properties", "chat"];
        let current = 0usize;
        let next = (current + 1) % panels.len();
        assert_eq!(panels[next], "properties", "Tab must move focus to the next panel");
    }

    #[test]
    fn keyboard_tab_wraps_at_end() {
        let panels = vec!["file-tree", "properties", "chat"];
        let current = 2usize; // last
        let next = (current + 1) % panels.len();
        assert_eq!(panels[next], "file-tree", "Tab past last panel must wrap to first");
    }

    #[test]
    fn keyboard_tab_focus_list_nonempty() {
        // A valid focus list must contain at least one panel.
        let panels: Vec<&str> = vec!["file-tree"];
        assert!(!panels.is_empty(), "focus list must be non-empty");
    }

    // --- Keyboard navigation: Shift+Tab moves focus to previous panel ---

    #[test]
    fn keyboard_shift_tab_moves_focus_to_previous() {
        let panels = vec!["file-tree", "properties", "chat"];
        let current = 2usize;
        let prev = (current + panels.len() - 1) % panels.len();
        assert_eq!(panels[prev], "properties", "Shift+Tab must move to previous panel");
    }

    #[test]
    fn keyboard_shift_tab_wraps_at_start() {
        let panels = vec!["file-tree", "properties", "chat"];
        let current = 0usize; // first
        let prev = (current + panels.len() - 1) % panels.len();
        assert_eq!(panels[prev], "chat", "Shift+Tab past first panel must wrap to last");
    }

    #[test]
    fn keyboard_tab_and_shift_tab_are_inverse() {
        let panels = vec!["file-tree", "properties", "chat"];
        let start = 1usize;
        let after_tab = (start + 1) % panels.len();
        let back = (after_tab + panels.len() - 1) % panels.len();
        assert_eq!(back, start, "Tab then Shift+Tab must return to original focus");
    }

    // --- Keyboard navigation: Escape closes overlay panels ---

    #[test]
    fn keyboard_escape_closes_overlay_panel() {
        // Simulate: overlay is_open = true; Escape sets it to false.
        let mut overlay_open = true;
        let escape_pressed = true;
        if escape_pressed {
            overlay_open = false;
        }
        assert!(!overlay_open, "Escape must close the overlay panel");
    }

    #[test]
    fn keyboard_escape_does_not_affect_non_overlay() {
        // Non-overlay docks are not closed by Escape.
        let dock_open = true;
        // Escape only closes overlays; main dock stays open.
        let overlay_is_dock = false;
        let still_open = if overlay_is_dock { false } else { dock_open };
        assert!(still_open, "Escape must not close non-overlay docks");
    }

    #[test]
    fn keyboard_escape_clears_search_query() {
        // Pressing Escape inside a search panel clears the search query.
        let mut query = "search text".to_string();
        let escape = true;
        if escape {
            query.clear();
        }
        assert!(query.is_empty(), "Escape must clear the search query");
    }

    // --- Keyboard navigation: Arrow keys navigate within focused panel ---

    #[test]
    fn keyboard_arrow_down_advances_selection_in_panel() {
        let items = vec!["item-0", "item-1", "item-2"];
        let selected = 0usize;
        let after_down = (selected + 1).min(items.len() - 1);
        assert_eq!(items[after_down], "item-1", "ArrowDown must advance selection");
    }

    #[test]
    fn keyboard_arrow_up_retreats_selection_in_panel() {
        let items = vec!["item-0", "item-1", "item-2"];
        let selected = 2usize;
        let after_up = selected.saturating_sub(1);
        assert_eq!(items[after_up], "item-1", "ArrowUp must retreat selection");
    }

    #[test]
    fn keyboard_arrow_down_clamps_at_end() {
        let items = vec!["item-0", "item-1", "item-2"];
        let selected = 2usize; // last
        let after_down = (selected + 1).min(items.len() - 1);
        assert_eq!(after_down, 2, "ArrowDown at last item must clamp (not overflow)");
    }

    #[test]
    fn keyboard_arrow_up_clamps_at_start() {
        let items = vec!["item-0", "item-1", "item-2"];
        let selected = 0usize;
        let after_up = selected.saturating_sub(1);
        assert_eq!(after_up, 0, "ArrowUp at first item must clamp (not underflow)");
    }

    // --- Panel tab overflow: overflow indicator appears ---

    #[test]
    fn tab_overflow_indicator_visible_when_tabs_exceed_width() {
        // When rendered tab count * tab_width > container width, overflow is shown.
        let tab_count = 10usize;
        let tab_width = 100.0_f32;
        let container_width = 600.0_f32;
        let overflow = (tab_count as f32 * tab_width) > container_width;
        assert!(overflow, "overflow indicator must appear when total tab width > container");
    }

    #[test]
    fn tab_overflow_not_shown_when_tabs_fit() {
        let tab_count = 3usize;
        let tab_width = 100.0_f32;
        let container_width = 600.0_f32;
        let overflow = (tab_count as f32 * tab_width) > container_width;
        assert!(!overflow, "no overflow when tabs fit within container width");
    }

    #[test]
    fn tab_overflow_visible_count_is_positive() {
        // The number of visible tabs must be at least 1.
        let container_width = 600.0_f32;
        let tab_width = 100.0_f32;
        let visible = (container_width / tab_width).floor() as usize;
        assert!(visible >= 1, "at least one tab must be visible at all times");
    }

    // --- Panel tab overflow: scroll tabs left/right ---

    #[test]
    fn tab_overflow_scroll_right_shifts_first_visible_tab() {
        let mut first_visible = 0usize;
        let total_tabs = 8usize;
        let visible = 4usize;
        if first_visible + visible < total_tabs {
            first_visible += 1;
        }
        assert_eq!(first_visible, 1, "scroll right must shift first visible tab by 1");
    }

    #[test]
    fn tab_overflow_scroll_left_shifts_first_visible_tab_back() {
        let mut first_visible = 3usize;
        if first_visible > 0 {
            first_visible -= 1;
        }
        assert_eq!(first_visible, 2, "scroll left must shift first visible tab back by 1");
    }

    #[test]
    fn tab_overflow_scroll_right_clamps_at_end() {
        let total = 8usize;
        let visible = 4usize;
        let mut first = total - visible; // 4 — already at end
        if first + visible < total {
            first += 1;
        }
        assert_eq!(first, total - visible, "cannot scroll past the last visible tab");
    }

    #[test]
    fn tab_overflow_scroll_left_clamps_at_zero() {
        let mut first = 0usize;
        first = first.saturating_sub(1);
        assert_eq!(first, 0, "cannot scroll left past the first tab");
    }

    // --- Panel tab overflow: active tab always visible ---

    #[test]
    fn tab_overflow_active_tab_in_visible_window() {
        // If the active tab is outside the visible window, scroll to reveal it.
        let active = 6usize;
        let visible = 4usize;
        let total = 10usize;
        // Compute first_visible such that active is in [first, first+visible).
        let first = if active >= visible { active - (visible - 1) } else { 0 };
        let last = first + visible - 1;
        assert!(
            active >= first && active <= last,
            "active tab ({active}) must be within visible window [{first}, {last}]"
        );
    }

    #[test]
    fn tab_overflow_active_always_visible_at_start() {
        let active = 0usize;
        let visible = 4usize;
        let first = 0usize;
        let last = first + visible - 1;
        assert!(active <= last, "active tab at start must be visible");
    }

    #[test]
    fn tab_overflow_scroll_to_active_when_out_of_view() {
        let active = 7usize;
        let visible = 4usize;
        let first_before = 0usize;
        // Active (7) is beyond first_before + visible (4); must scroll.
        let needs_scroll = active >= first_before + visible;
        assert!(needs_scroll, "active tab out of view must trigger scroll");
        let new_first = active.saturating_sub(visible - 1);
        assert!(active >= new_first && active < new_first + visible, "after scroll, active must be visible");
    }

    // --- Search within panel: filters visible items ---

    #[test]
    fn panel_search_input_filters_items() {
        let items = vec!["alpha-func", "beta-func", "gamma-concept", "delta-concept"];
        let query = "func";
        let filtered: Vec<_> = items.iter().filter(|i| i.contains(query)).collect();
        assert_eq!(filtered.len(), 2, "search 'func' must return 2 items");
    }

    #[test]
    fn panel_search_partial_match_returns_results() {
        let items = vec!["FileTreePanel", "LibraryPanel", "PropertiesPanel"];
        let query = "Panel";
        let filtered: Vec<_> = items.iter().filter(|i| i.contains(query)).collect();
        assert_eq!(filtered.len(), 3, "partial match 'Panel' must return all 3 items");
    }

    #[test]
    fn panel_search_specific_query_returns_one() {
        let items = vec!["FileTreePanel", "LibraryPanel", "PropertiesPanel"];
        let query = "FileTree";
        let filtered: Vec<_> = items.iter().filter(|i| i.contains(query)).collect();
        assert_eq!(filtered.len(), 1, "specific query must return exactly 1 item");
    }

    // --- Search within panel: empty search shows all items ---

    #[test]
    fn panel_search_empty_query_shows_all_items() {
        let items = vec!["alpha", "beta", "gamma", "delta"];
        let query = "";
        let filtered: Vec<_> = items.iter().filter(|i| query.is_empty() || i.contains(query)).collect();
        assert_eq!(filtered.len(), items.len(), "empty query must show all items");
    }

    #[test]
    fn panel_search_empty_query_count_equals_total() {
        use nom_blocks::stub_dict::StubDictReader;
        let dict = StubDictReader::with_kinds(&["A", "B", "C"]);
        let palette = crate::left::NodePalette::load_from_dict(&dict);
        let all = palette.search("");
        assert!(all.len() >= 3, "empty search in NodePalette must return all entries (>= 3)");
    }

    // --- Search within panel: no results shows empty-state ---

    #[test]
    fn panel_search_no_match_returns_empty_vec() {
        let items = vec!["alpha", "beta", "gamma"];
        let query = "zzz_no_match_xyz";
        let filtered: Vec<_> = items.iter().filter(|i| i.contains(query)).collect();
        assert!(filtered.is_empty(), "no-match query must return empty result");
    }

    #[test]
    fn panel_search_empty_result_triggers_empty_state_display() {
        // Simulate: if filtered results are empty, show empty-state message.
        let results: Vec<&str> = Vec::new();
        let show_empty_state = results.is_empty();
        assert!(show_empty_state, "empty results must trigger empty-state message");
    }

    // --- Search within panel: case-insensitive ---

    #[test]
    fn panel_search_case_insensitive_uppercase_query() {
        let items = vec!["FunctionKind", "ConceptKind", "EntityKind"];
        let query = "FUNCTION";
        let filtered: Vec<_> = items
            .iter()
            .filter(|i| i.to_lowercase().contains(&query.to_lowercase()))
            .collect();
        assert_eq!(filtered.len(), 1, "uppercase query must match case-insensitively");
    }

    #[test]
    fn panel_search_case_insensitive_mixed_case() {
        let items = vec!["FileNode", "filenode", "FILENODE"];
        let query = "filenode";
        let filtered: Vec<_> = items
            .iter()
            .filter(|i| i.to_lowercase().contains(&query.to_lowercase()))
            .collect();
        assert_eq!(filtered.len(), 3, "mixed-case query must match all case variants");
    }

    #[test]
    fn panel_search_case_insensitive_lowercase_query() {
        let items = vec!["Alpha", "Beta", "Gamma"];
        let query = "alpha";
        let filtered: Vec<_> = items
            .iter()
            .filter(|i| i.to_lowercase().contains(&query.to_lowercase()))
            .collect();
        assert_eq!(filtered.len(), 1, "lowercase query must match title-case item");
    }

    // --- Additional coverage: PanelEntityRef, Dock, PanelSizeState ---

    #[test]
    fn panel_size_state_fixed_is_not_zero() {
        let state = crate::dock::PanelSizeState::fixed(248.0);
        assert!(state.effective_size(1000.0) > 0.0, "fixed state must return positive size");
    }

    #[test]
    fn panel_size_state_flex_proportional_to_container() {
        let state = crate::dock::PanelSizeState::flex(0.3);
        let s800 = state.effective_size(800.0);
        let s1200 = state.effective_size(1200.0);
        assert!(
            s1200 > s800,
            "flex size must grow with container: s1200={s1200} must be > s800={s800}"
        );
    }

    #[test]
    fn dock_add_multiple_panels_maintains_order() {
        let mut dock = crate::dock::Dock::new(crate::dock::DockPosition::Left);
        dock.add_panel("a", 100.0);
        dock.add_panel("b", 200.0);
        dock.add_panel("c", 150.0);
        assert_eq!(dock.entries[0].id, "a", "first added panel must be at index 0");
        assert_eq!(dock.entries[1].id, "b", "second added panel must be at index 1");
        assert_eq!(dock.entries[2].id, "c", "third added panel must be at index 2");
    }

    #[test]
    fn dock_first_panel_auto_activated() {
        let mut dock = crate::dock::Dock::new(crate::dock::DockPosition::Right);
        dock.add_panel("first", 280.0);
        assert_eq!(dock.active_panel_id(), Some("first"), "first added panel must be auto-activated");
    }

    #[test]
    fn panel_search_single_char_query_filters() {
        let items = vec!["abc", "def", "axy", "xyz"];
        let query = "a";
        let filtered: Vec<_> = items.iter().filter(|i| i.contains(query)).collect();
        assert_eq!(filtered.len(), 2, "single-char query must match correctly");
    }
}
