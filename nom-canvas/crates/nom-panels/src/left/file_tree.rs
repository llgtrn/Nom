#![deny(unsafe_code)]
use crate::dock::{DockPosition, Panel};
use crate::right::chat_sidebar::RenderPrimitive;

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

impl FileTreePanel {
    /// Render the panel into a flat list of primitives.
    ///
    /// Row layout: each visible file node occupies 20px of height.
    /// Indent = depth * 12px; directories get a directional arrow prefix.
    pub fn render_bounds(&self, width: f32, height: f32) -> Vec<RenderPrimitive> {
        let mut out = Vec::new();

        // Background rect for whole panel.
        out.push(RenderPrimitive::Rect {
            x: 0.0,
            y: 0.0,
            w: width,
            h: height,
            color: 0x1e1e2e,
        });

        let mut row: usize = 0;
        for section in &self.sections {
            for node in &section.nodes {
                for visible in node.visible_nodes() {
                    let y = row as f32 * 20.0 + 4.0;
                    let indent = visible.depth as f32 * 12.0;

                    // Selection highlight behind text.
                    if self.selected_id.as_deref() == Some(visible.id.as_str()) {
                        out.push(RenderPrimitive::Rect {
                            x: 0.0,
                            y: row as f32 * 20.0,
                            w: width,
                            h: 20.0,
                            color: 0x313244,
                        });
                    }

                    let prefix = match visible.kind {
                        FileNodeKind::Directory => {
                            if visible.is_expanded { "\u{25be} " } else { "\u{25b8} " }
                        }
                        _ => "",
                    };
                    let label = format!("{}{}", prefix, visible.name);

                    out.push(RenderPrimitive::Text {
                        x: indent,
                        y,
                        text: label,
                        size: 13.0,
                        color: 0xcdd6f4,
                    });

                    row += 1;
                }
            }
        }

        out
    }
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

    #[test]
    fn file_tree_render_returns_primitives() {
        let mut panel = FileTreePanel::new();
        // Add a child file so the section has visible nodes.
        let section = panel.sections.get_mut(0).unwrap();
        let mut dir = section.nodes.get_mut(0).unwrap();
        dir.is_expanded = true;
        dir.children.push(FileNode::file("main.nom", 1, FileNodeKind::NomFile));
        panel.selected_id = Some("src".to_string());

        let prims = panel.render_bounds(248.0, 600.0);

        // Must include at least: bg rect + selection rect + dir text + file text.
        assert!(prims.len() >= 4, "expected >=4 primitives, got {}", prims.len());

        // First primitive must be the background rect covering the full panel.
        match &prims[0] {
            RenderPrimitive::Rect { x, y, w, h, color } => {
                assert_eq!(*x, 0.0);
                assert_eq!(*y, 0.0);
                assert_eq!(*w, 248.0);
                assert_eq!(*h, 600.0);
                assert_eq!(*color, 0x1e1e2e);
            }
            other => panic!("expected bg Rect, got {:?}", other),
        }

        // Verify at least one Text primitive exists.
        let has_text = prims.iter().any(|p| matches!(p, RenderPrimitive::Text { .. }));
        assert!(has_text, "expected at least one Text primitive");

        // Verify selection highlight rect exists (color 0x313244).
        let has_highlight = prims.iter().any(|p| {
            matches!(p, RenderPrimitive::Rect { color: 0x313244, .. })
        });
        assert!(has_highlight, "expected selection highlight rect");
    }
}
