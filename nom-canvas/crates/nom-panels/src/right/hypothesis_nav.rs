#![deny(unsafe_code)]

use std::collections::HashMap;

use super::reasoning_card::AnimatedReasoningCard;

/// A node in the hypothesis tree.
#[derive(Debug, Clone)]
pub struct HypothesisNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub card: AnimatedReasoningCard,
    /// Ordered list of child node IDs.
    pub children: Vec<String>,
}

/// Navigation controller for a tree of hypothesis nodes.
#[derive(Debug)]
pub struct HypothesisTreeNav {
    pub nodes: HashMap<String, HypothesisNode>,
    pub root_ids: Vec<String>,
    pub selected_id: Option<String>,
}

impl HypothesisTreeNav {
    /// Create an empty navigation tree.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            root_ids: Vec::new(),
            selected_id: None,
        }
    }

    /// Add a root-level node to the tree.
    pub fn add_root(mut self, node: HypothesisNode) -> Self {
        self.root_ids.push(node.id.clone());
        self.nodes.insert(node.id.clone(), node);
        self
    }

    /// Add `node` as a child of the node identified by `parent_id`.
    ///
    /// If `parent_id` does not exist the node is still inserted but not linked.
    pub fn add_child(mut self, parent_id: &str, node: HypothesisNode) -> Self {
        let child_id = node.id.clone();
        self.nodes.insert(child_id.clone(), node);
        if let Some(parent) = self.nodes.get_mut(parent_id) {
            parent.children.push(child_id);
        }
        self
    }

    /// Select the node with the given ID.
    pub fn select(mut self, id: &str) -> Self {
        self.selected_id = Some(id.to_string());
        self
    }

    /// Return a reference to the currently selected node, if any.
    pub fn selected(&self) -> Option<&HypothesisNode> {
        self.selected_id
            .as_deref()
            .and_then(|id| self.nodes.get(id))
    }

    /// Return all node IDs in depth-first order starting from the roots.
    pub fn depth_first_ids(&self) -> Vec<String> {
        let mut result = Vec::with_capacity(self.nodes.len());
        for root_id in &self.root_ids {
            self.dfs(root_id, &mut result);
        }
        result
    }

    fn dfs(&self, id: &str, out: &mut Vec<String>) {
        out.push(id.to_string());
        if let Some(node) = self.nodes.get(id) {
            for child_id in &node.children {
                self.dfs(child_id, out);
            }
        }
    }

    /// Total number of nodes in the tree.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

impl Default for HypothesisTreeNav {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::right::reasoning_card::AnimatedReasoningCard;

    fn make_node(id: &str, parent_id: Option<&str>) -> HypothesisNode {
        HypothesisNode {
            id: id.to_string(),
            parent_id: parent_id.map(str::to_string),
            card: AnimatedReasoningCard::new(id, "hypothesis", 0.5),
            children: Vec::new(),
        }
    }

    #[test]
    fn new_tree_is_empty() {
        let nav = HypothesisTreeNav::new();
        assert_eq!(nav.node_count(), 0);
        assert!(nav.root_ids.is_empty());
        assert!(nav.selected_id.is_none());
    }

    #[test]
    fn add_root_inserts_node() {
        let nav = HypothesisTreeNav::new().add_root(make_node("root1", None));
        assert_eq!(nav.node_count(), 1);
        assert_eq!(nav.root_ids, vec!["root1"]);
    }

    #[test]
    fn add_child_links_to_parent() {
        let nav = HypothesisTreeNav::new()
            .add_root(make_node("r", None))
            .add_child("r", make_node("c1", Some("r")));
        assert_eq!(nav.node_count(), 2);
        let parent = nav.nodes.get("r").unwrap();
        assert_eq!(parent.children, vec!["c1"]);
    }

    #[test]
    fn select_returns_node() {
        let nav = HypothesisTreeNav::new()
            .add_root(make_node("r", None))
            .select("r");
        let selected = nav.selected().expect("should be selected");
        assert_eq!(selected.id, "r");
    }

    #[test]
    fn depth_first_ids_correct_order() {
        // Tree:  r -> a -> b
        //          -> c
        let nav = HypothesisTreeNav::new()
            .add_root(make_node("r", None))
            .add_child("r", make_node("a", Some("r")))
            .add_child("r", make_node("c", Some("r")))
            .add_child("a", make_node("b", Some("a")));
        let order = nav.depth_first_ids();
        assert_eq!(order, vec!["r", "a", "b", "c"]);
    }
}
