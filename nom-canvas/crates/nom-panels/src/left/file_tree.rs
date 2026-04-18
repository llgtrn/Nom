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
}
