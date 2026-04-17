#![deny(unsafe_code)]
use crate::dock::{DockPosition, Panel};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileNodeKind { Directory, NomFile, NomtuFile, Asset }

#[derive(Debug, Clone)]
pub struct FileNode {
    pub id: String,
    pub name: String,
    pub kind: FileNodeKind,
    pub depth: u32,
    pub is_expanded: bool,
    pub children: Vec<FileNode>,
    pub entity_id: Option<String>,  // NomtuRef.id when this is a .nomtu entry
}

impl FileNode {
    pub fn dir(name: impl Into<String>, depth: u32) -> Self {
        let name = name.into();
        Self { id: name.clone(), name, kind: FileNodeKind::Directory, depth, is_expanded: false, children: vec![], entity_id: None }
    }

    pub fn file(name: impl Into<String>, depth: u32, kind: FileNodeKind) -> Self {
        let name = name.into();
        Self { id: name.clone(), name, kind, depth, is_expanded: false, children: vec![], entity_id: None }
    }

    pub fn toggle_expand(&mut self) { self.is_expanded = !self.is_expanded; }

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
        Self { id: id.into(), title: title.into(), is_open: true, nodes: vec![] }
    }

    pub fn toggle(&mut self) { self.is_open = !self.is_open; }

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
        Self { sections: vec![workspace], selected_id: None }
    }

    pub fn select(&mut self, id: &str) { self.selected_id = Some(id.to_string()); }
}

impl Default for FileTreePanel { fn default() -> Self { Self::new() } }

impl Panel for FileTreePanel {
    fn id(&self) -> &str { "file-tree" }
    fn title(&self) -> &str { "Explorer" }
    fn default_size(&self) -> f32 { 248.0 }
    fn position(&self) -> DockPosition { DockPosition::Left }
    fn activation_priority(&self) -> u32 { 10 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapsible_section_count() {
        let mut s = CollapsibleSection::new("s", "Section");
        let mut dir = FileNode::dir("src", 0);
        dir.children.push(FileNode::file("main.nom", 1, FileNodeKind::NomFile));
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
}
