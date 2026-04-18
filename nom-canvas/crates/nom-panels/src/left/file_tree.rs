#![deny(unsafe_code)]
use crate::dock::{fill_quad, focus_ring_quad, DockPosition, Panel};
use crate::entity_ref::PanelEntityRef;
use nom_gpui::scene::Scene;
use nom_theme::tokens;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileNodeKind {
    Directory,
    NomFile,
    NomtuFile,
    Asset,
}

#[derive(Debug, Clone)]
pub struct FileNode {
    pub id: String,
    pub name: String,
    pub kind: FileNodeKind,
    pub depth: u32,
    pub is_expanded: bool,
    pub children: Vec<FileNode>,
    pub entity: PanelEntityRef,
}

impl FileNode {
    pub fn dir(name: impl Into<String>, depth: u32) -> Self {
        let name = name.into();
        Self {
            id: name.clone(),
            name,
            kind: FileNodeKind::Directory,
            depth,
            is_expanded: false,
            children: vec![],
            entity: PanelEntityRef::None,
        }
    }

    pub fn file(name: impl Into<String>, depth: u32, kind: FileNodeKind) -> Self {
        let name = name.into();
        Self {
            id: name.clone(),
            name,
            kind,
            depth,
            is_expanded: false,
            children: vec![],
            entity: PanelEntityRef::None,
        }
    }

    pub fn with_entity(mut self, entity: PanelEntityRef) -> Self {
        self.entity = entity;
        self
    }

    pub fn toggle_expand(&mut self) {
        self.is_expanded = !self.is_expanded;
    }

    pub fn visible_nodes(&self) -> Vec<&FileNode> {
        let mut out = vec![self as &FileNode];
        if self.is_expanded {
            for child in &self.children {
                out.extend(child.visible_nodes());
            }
        }
        out
    }
}

pub struct CollapsibleSection {
    pub id: String,
    pub title: String,
    pub is_open: bool,
    pub nodes: Vec<FileNode>,
}

impl CollapsibleSection {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            is_open: true,
            nodes: vec![],
        }
    }

    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
    }

    pub fn visible_count(&self) -> usize {
        if self.is_open {
            self.nodes.iter().map(|n| n.visible_nodes().len()).sum()
        } else {
            0
        }
    }
}

pub struct FileTreePanel {
    pub sections: Vec<CollapsibleSection>,
    pub selected_id: Option<String>,
}

impl FileTreePanel {
    pub fn new() -> Self {
        let mut workspace = CollapsibleSection::new("workspace", "WORKSPACE");
        workspace.nodes.push(FileNode::dir("src", 0));
        Self {
            sections: vec![workspace],
            selected_id: None,
        }
    }

    pub fn select(&mut self, id: &str) {
        self.selected_id = Some(id.to_string());
    }
}

impl FileTreePanel {
    /// Paint the file tree into the shared GPU scene.
    ///
    /// Row height = 20 px. Each visible file node gets a quad entry; the
    /// currently selected node gets a CTA-coloured highlight behind it.
    pub fn paint_scene(&self, width: f32, height: f32, scene: &mut Scene) {
        // Background for the whole panel.
        scene.push_quad(fill_quad(0.0, 0.0, width, height, tokens::BG));

        let mut row: usize = 0;
        for section in &self.sections {
            for node in &section.nodes {
                for visible in node.visible_nodes() {
                    let y = row as f32 * 20.0;

                    // Row background alternates subtly via BG2 so the tree is
                    // legible even without text rendering.
                    if row % 2 == 1 {
                        scene.push_quad(fill_quad(0.0, y, width, 20.0, tokens::BG2));
                    }

                    // Selection focus ring: 2px border-only outline (no fill).
                    if self.selected_id.as_deref() == Some(visible.id.as_str()) {
                        scene.push_quad(focus_ring_quad(0.0, y, width, 20.0));
                    }

                    row += 1;
                }
            }
        }
    }
}

impl Default for FileTreePanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for FileTreePanel {
    fn id(&self) -> &str {
        "file-tree"
    }
    fn title(&self) -> &str {
        "Explorer"
    }
    fn default_size(&self) -> f32 {
        248.0
    }
    fn position(&self) -> DockPosition {
        DockPosition::Left
    }
    fn activation_priority(&self) -> u32 {
        10
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapsible_section_count() {
        let mut s = CollapsibleSection::new("s", "Section");
        let mut dir = FileNode::dir("src", 0);
        dir.children
            .push(FileNode::file("main.nom", 1, FileNodeKind::NomFile));
        dir.is_expanded = true;
        s.nodes.push(dir);
        assert_eq!(s.visible_count(), 2);
        s.toggle();
        assert_eq!(s.visible_count(), 0);
    }

    #[test]
    fn file_tree_panel_implements_panel() {
        let p = FileTreePanel::new();
        assert_eq!(p.id(), "file-tree");
        assert_eq!(p.position(), DockPosition::Left);
    }

    #[test]
    fn file_tree_empty() {
        let panel = FileTreePanel {
            sections: vec![],
            selected_id: None,
        };
        let total: usize = panel.sections.iter().map(|s| s.nodes.len()).sum();
        assert_eq!(total, 0);
    }

    #[test]
    fn file_tree_add_file() {
        let mut section = CollapsibleSection::new("ws", "Workspace");
        section
            .nodes
            .push(FileNode::file("main.nom", 0, FileNodeKind::NomFile));
        let panel = FileTreePanel {
            sections: vec![section],
            selected_id: None,
        };
        let total: usize = panel.sections.iter().map(|s| s.nodes.len()).sum();
        assert!(total > 0);
    }

    #[test]
    fn file_tree_add_directory() {
        let node = FileNode::dir("src", 0);
        assert_eq!(node.kind, FileNodeKind::Directory);
    }

    #[test]
    fn file_node_entity_metadata_is_typed_boundary() {
        let node = FileNode::file("entry.nomtu", 0, FileNodeKind::NomtuFile).with_entity(
            PanelEntityRef::nomtu(nom_blocks::NomtuRef::new("e1", "entry", "concept")),
        );
        assert_eq!(node.entity.id(), Some("e1"));
        assert_eq!(node.entity.kind(), Some("concept"));
    }

    #[test]
    fn file_tree_expand() {
        let mut node = FileNode::dir("src", 0);
        node.children
            .push(FileNode::file("main.nom", 1, FileNodeKind::NomFile));
        node.toggle_expand();
        assert!(node.is_expanded);
        let visible = node.visible_nodes();
        assert_eq!(visible.len(), 2); // dir + child
    }

    #[test]
    fn file_tree_collapse() {
        let mut node = FileNode::dir("src", 0);
        node.children
            .push(FileNode::file("main.nom", 1, FileNodeKind::NomFile));
        node.is_expanded = true;
        node.toggle_expand();
        assert!(!node.is_expanded);
        let visible = node.visible_nodes();
        assert_eq!(visible.len(), 1); // only dir itself
    }

    #[test]
    fn file_tree_paint_has_quads() {
        let mut panel = FileTreePanel::new();
        let section = panel.sections.get_mut(0).unwrap();
        let dir = section.nodes.get_mut(0).unwrap();
        dir.is_expanded = true;
        dir.children
            .push(FileNode::file("main.nom", 1, FileNodeKind::NomFile));
        panel.selected_id = Some("src".to_string());

        let mut scene = Scene::new();
        panel.paint_scene(248.0, 600.0, &mut scene);

        // bg + selection highlight for "src" at least.
        assert!(
            scene.quads.len() >= 2,
            "expected >=2 quads, got {}",
            scene.quads.len()
        );

        // First quad is the background.
        let bg = &scene.quads[0];
        assert_eq!(bg.bounds.size.width, nom_gpui::types::Pixels(248.0));
        assert_eq!(bg.bounds.size.height, nom_gpui::types::Pixels(600.0));
    }

    // ── expand/collapse state persistence ────────────────────────────────────

    #[test]
    fn file_node_expand_collapse_toggle() {
        let mut node = FileNode::dir("src", 0);
        assert!(!node.is_expanded, "initially collapsed");
        node.toggle_expand();
        assert!(node.is_expanded);
        node.toggle_expand();
        assert!(!node.is_expanded);
    }

    #[test]
    fn file_node_collapsed_shows_only_self() {
        let mut node = FileNode::dir("src", 0);
        node.children
            .push(FileNode::file("a.nom", 1, FileNodeKind::NomFile));
        node.children
            .push(FileNode::file("b.nom", 1, FileNodeKind::NomFile));
        // collapsed: visible_nodes returns only the dir itself
        let visible = node.visible_nodes();
        assert_eq!(visible.len(), 1, "collapsed dir shows only itself");
    }

    #[test]
    fn file_node_expanded_shows_children() {
        let mut node = FileNode::dir("src", 0);
        node.children
            .push(FileNode::file("a.nom", 1, FileNodeKind::NomFile));
        node.children
            .push(FileNode::file("b.nom", 1, FileNodeKind::NomFile));
        node.is_expanded = true;
        let visible = node.visible_nodes();
        assert_eq!(visible.len(), 3, "expanded dir shows self + 2 children");
    }

    #[test]
    fn collapsible_section_toggle_open_close() {
        let mut section = CollapsibleSection::new("ws", "Workspace");
        assert!(section.is_open, "sections start open");
        section.toggle();
        assert!(!section.is_open);
        section.toggle();
        assert!(section.is_open);
    }

    #[test]
    fn collapsible_section_closed_visible_count_is_zero() {
        let mut section = CollapsibleSection::new("ws", "Workspace");
        section
            .nodes
            .push(FileNode::file("a.nom", 0, FileNodeKind::NomFile));
        section
            .nodes
            .push(FileNode::file("b.nom", 0, FileNodeKind::NomFile));
        section.is_open = false;
        assert_eq!(section.visible_count(), 0);
    }

    #[test]
    fn collapsible_section_open_shows_all_nodes() {
        let mut section = CollapsibleSection::new("ws", "Workspace");
        for i in 0..5 {
            section.nodes.push(FileNode::file(
                format!("f{i}.nom"),
                0,
                FileNodeKind::NomFile,
            ));
        }
        assert!(section.is_open);
        assert_eq!(section.visible_count(), 5);
    }

    #[test]
    fn file_tree_select_updates_selected_id() {
        let mut panel = FileTreePanel::new();
        panel.select("src");
        assert_eq!(panel.selected_id.as_deref(), Some("src"));
    }

    #[test]
    fn file_node_nested_expansion() {
        // src/
        //   lib/
        //     types.nom
        let mut lib = FileNode::dir("lib", 1);
        lib.children
            .push(FileNode::file("types.nom", 2, FileNodeKind::NomFile));
        lib.is_expanded = true;
        let mut src = FileNode::dir("src", 0);
        src.children.push(lib);
        src.is_expanded = true;
        // src expanded + lib expanded → visible = [src, lib, types.nom]
        let visible = src.visible_nodes();
        assert_eq!(visible.len(), 3);
        assert_eq!(visible[0].name, "src");
        assert_eq!(visible[1].name, "lib");
        assert_eq!(visible[2].name, "types.nom");
    }

    #[test]
    fn file_node_nested_parent_collapsed() {
        // When parent is collapsed, children are hidden even if children are expanded.
        let mut lib = FileNode::dir("lib", 1);
        lib.children
            .push(FileNode::file("types.nom", 2, FileNodeKind::NomFile));
        lib.is_expanded = true; // lib would show types.nom if expanded
        let mut src = FileNode::dir("src", 0);
        src.children.push(lib);
        // src is NOT expanded
        src.is_expanded = false;
        let visible = src.visible_nodes();
        assert_eq!(visible.len(), 1, "collapsed parent hides all descendants");
    }

    #[test]
    fn file_node_kind_asset() {
        let node = FileNode::file("logo.png", 0, FileNodeKind::Asset);
        assert_eq!(node.kind, FileNodeKind::Asset);
    }

    // ── deep nesting (5 levels) ───────────────────────────────────────────────

    #[test]
    fn file_tree_five_levels_deep_expanded() {
        // a/b/c/d/leaf.nom — all expanded
        let leaf = FileNode::file("leaf.nom", 4, FileNodeKind::NomFile);
        let mut d = FileNode::dir("d", 3);
        d.children.push(leaf);
        d.is_expanded = true;
        let mut c = FileNode::dir("c", 2);
        c.children.push(d);
        c.is_expanded = true;
        let mut b = FileNode::dir("b", 1);
        b.children.push(c);
        b.is_expanded = true;
        let mut a = FileNode::dir("a", 0);
        a.children.push(b);
        a.is_expanded = true;
        let visible = a.visible_nodes();
        // a + b + c + d + leaf = 5
        assert_eq!(
            visible.len(),
            5,
            "5-level fully expanded tree should show 5 nodes"
        );
    }

    #[test]
    fn file_tree_five_levels_deep_middle_collapsed() {
        // a/b/c/d/leaf.nom — c is collapsed, d and leaf are hidden
        let leaf = FileNode::file("leaf.nom", 4, FileNodeKind::NomFile);
        let mut d = FileNode::dir("d", 3);
        d.children.push(leaf);
        d.is_expanded = true;
        let mut c = FileNode::dir("c", 2);
        c.children.push(d);
        c.is_expanded = false; // collapsed here
        let mut b = FileNode::dir("b", 1);
        b.children.push(c);
        b.is_expanded = true;
        let mut a = FileNode::dir("a", 0);
        a.children.push(b);
        a.is_expanded = true;
        let visible = a.visible_nodes();
        // a + b + c = 3 (d and leaf hidden under collapsed c)
        assert_eq!(visible.len(), 3, "collapsing level 2 hides levels 3 and 4");
    }

    #[test]
    fn file_tree_five_levels_depth_values_correct() {
        let mut d3 = FileNode::dir("d3", 3);
        d3.is_expanded = true;
        d3.children
            .push(FileNode::file("f4.nom", 4, FileNodeKind::NomFile));
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
        // d0, d1, d2, d3, f4 = 5 nodes
        assert_eq!(visible.len(), 5);
        assert_eq!(visible[0].depth, 0);
        assert_eq!(visible[1].depth, 1);
        assert_eq!(visible[2].depth, 2);
        assert_eq!(visible[3].depth, 3);
        assert_eq!(visible[4].depth, 4);
    }

    // ── file rename in tree ───────────────────────────────────────────────────

    #[test]
    fn file_node_rename_updates_name() {
        let mut node = FileNode::file("old.nom", 0, FileNodeKind::NomFile);
        node.name = "new.nom".to_string();
        assert_eq!(node.name, "new.nom");
    }

    #[test]
    fn file_node_rename_id_independent_of_name() {
        let mut node = FileNode::file("original.nom", 0, FileNodeKind::NomFile);
        let original_id = node.id.clone();
        node.name = "renamed.nom".to_string();
        // id is set at construction time; renaming name does not auto-change id
        assert_eq!(node.id, original_id);
        assert_eq!(node.name, "renamed.nom");
    }

    #[test]
    fn file_node_rename_via_id_update() {
        let mut node = FileNode::file("alpha.nom", 0, FileNodeKind::NomFile);
        node.id = "beta.nom".to_string();
        node.name = "beta.nom".to_string();
        assert_eq!(node.id, "beta.nom");
        assert_eq!(node.name, "beta.nom");
    }

    #[test]
    fn file_tree_select_renamed_node() {
        let mut panel = FileTreePanel::new();
        let section = &mut panel.sections[0];
        let mut node = FileNode::file("old.nom", 0, FileNodeKind::NomFile);
        node.id = "new.nom".to_string();
        node.name = "new.nom".to_string();
        section.nodes.push(node);
        panel.select("new.nom");
        assert_eq!(panel.selected_id.as_deref(), Some("new.nom"));
    }

    // ── sort order: directories before files ──────────────────────────────────

    #[test]
    fn sort_order_dirs_before_files_manual() {
        let mut nodes = vec![
            FileNode::file("zebra.nom", 0, FileNodeKind::NomFile),
            FileNode::dir("alpha", 0),
            FileNode::file("main.nom", 0, FileNodeKind::NomFile),
            FileNode::dir("src", 0),
        ];
        // Sort: directories first, then files
        nodes.sort_by_key(|n| {
            if n.kind == FileNodeKind::Directory {
                0u8
            } else {
                1u8
            }
        });
        assert_eq!(
            nodes[0].kind,
            FileNodeKind::Directory,
            "first entry must be a directory"
        );
        assert_eq!(
            nodes[1].kind,
            FileNodeKind::Directory,
            "second entry must be a directory"
        );
        assert_eq!(
            nodes[2].kind,
            FileNodeKind::NomFile,
            "third entry must be a file"
        );
    }

    #[test]
    fn sort_order_dirs_before_files_within_section() {
        let mut section = CollapsibleSection::new("ws", "Workspace");
        section
            .nodes
            .push(FileNode::file("main.nom", 0, FileNodeKind::NomFile));
        section.nodes.push(FileNode::dir("src", 0));
        section
            .nodes
            .push(FileNode::file("config.nom", 0, FileNodeKind::NomFile));
        section.nodes.push(FileNode::dir("tests", 0));
        section.nodes.sort_by_key(|n| {
            if n.kind == FileNodeKind::Directory {
                0u8
            } else {
                1u8
            }
        });
        assert_eq!(section.nodes[0].name, "src");
        assert_eq!(section.nodes[1].name, "tests");
    }

    #[test]
    fn sort_order_alphabetical_within_dirs() {
        let mut dirs = vec![
            FileNode::dir("zoo", 0),
            FileNode::dir("alpha", 0),
            FileNode::dir("beta", 0),
        ];
        dirs.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(dirs[0].name, "alpha");
        assert_eq!(dirs[1].name, "beta");
        assert_eq!(dirs[2].name, "zoo");
    }

    #[test]
    fn sort_order_files_alphabetical() {
        let mut files = vec![
            FileNode::file("z.nom", 0, FileNodeKind::NomFile),
            FileNode::file("a.nom", 0, FileNodeKind::NomFile),
            FileNode::file("m.nom", 0, FileNodeKind::NomFile),
        ];
        files.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(files[0].name, "a.nom");
        assert_eq!(files[1].name, "m.nom");
        assert_eq!(files[2].name, "z.nom");
    }

    #[test]
    fn file_tree_paint_with_five_level_depth() {
        let leaf = FileNode::file("leaf.nom", 4, FileNodeKind::NomFile);
        let mut d = FileNode::dir("d", 3);
        d.children.push(leaf);
        d.is_expanded = true;
        let mut c = FileNode::dir("c", 2);
        c.children.push(d);
        c.is_expanded = true;
        let mut b = FileNode::dir("b", 1);
        b.children.push(c);
        b.is_expanded = true;
        let mut root = FileNode::dir("root", 0);
        root.children.push(b);
        root.is_expanded = true;

        let mut section = CollapsibleSection::new("ws", "WS");
        section.nodes.push(root);

        let panel = FileTreePanel {
            sections: vec![section],
            selected_id: None,
        };
        let mut scene = nom_gpui::scene::Scene::new();
        panel.paint_scene(248.0, 600.0, &mut scene);
        // background + alternating BG2 rows (5 nodes → rows 1,3 get BG2 = 2 extra)
        assert!(
            scene.quads.len() >= 3,
            "deep tree should emit background + row quads"
        );
    }
}
