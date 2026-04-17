//! Sidebar panel — document tree, search, and recent files.

use smallvec::SmallVec;

use crate::DocumentId;

/// A node in the document tree displayed by the sidebar.
#[derive(Debug, Clone)]
pub struct DocumentNode {
    pub id: DocumentId,
    pub title: String,
    /// Children use Vec for heap allocation — SmallVec cannot hold recursive types inline.
    pub children: Vec<DocumentNode>,
    pub expanded: bool,
}

impl DocumentNode {
    pub fn new(id: DocumentId, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            children: Vec::new(),
            expanded: false,
        }
    }
}

/// Sidebar panel state.
#[derive(Debug)]
pub struct Sidebar {
    pub width_px: f32,
    pub is_collapsed: bool,
    pub tree_roots: SmallVec<[DocumentNode; 8]>,
    pub search_query: String,
    pub recent: SmallVec<[DocumentId; 8]>,
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            width_px: 248.0,
            is_collapsed: false,
            tree_roots: SmallVec::new(),
            search_query: String::new(),
            recent: SmallVec::new(),
        }
    }

    /// Toggle the collapsed state of the sidebar.
    pub fn toggle_collapse(&mut self) {
        self.is_collapsed = !self.is_collapsed;
    }

    /// Return tree roots whose title contains `search_query` (case-insensitive).
    /// If `search_query` is empty, returns all roots.
    pub fn filter_tree(&self) -> Vec<&DocumentNode> {
        if self.search_query.is_empty() {
            return self.tree_roots.iter().collect();
        }
        let q = self.search_query.to_lowercase();
        self.tree_roots
            .iter()
            .filter(|n| n.title.to_lowercase().contains(&q))
            .collect()
    }

    /// Stub paint method — rendering lives in the GPU layer.
    pub fn paint(&self) {}
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_width_is_248() {
        let s = Sidebar::new();
        assert_eq!(s.width_px, 248.0);
        assert!(!s.is_collapsed);
    }

    #[test]
    fn toggle_collapse_flips_state() {
        let mut s = Sidebar::new();
        s.toggle_collapse();
        assert!(s.is_collapsed);
        s.toggle_collapse();
        assert!(!s.is_collapsed);
    }

    #[test]
    fn filter_empty_query_returns_all() {
        let mut s = Sidebar::new();
        s.tree_roots.push(DocumentNode::new(1, "Alpha"));
        s.tree_roots.push(DocumentNode::new(2, "Beta"));
        assert_eq!(s.filter_tree().len(), 2);
    }

    #[test]
    fn filter_query_narrows_results() {
        let mut s = Sidebar::new();
        s.tree_roots.push(DocumentNode::new(1, "Alpha"));
        s.tree_roots.push(DocumentNode::new(2, "Beta"));
        s.search_query = "alp".to_string();
        let results = s.filter_tree();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Alpha");
    }
}
