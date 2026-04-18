use std::collections::HashMap;

// ---------------------------------------------------------------------------
// SceneNodeKind
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneNodeKind {
    Layer,
    Quad,
    Text,
    Image,
    Path,
}

impl SceneNodeKind {
    pub fn kind_name(&self) -> &str {
        match self {
            SceneNodeKind::Layer => "Layer",
            SceneNodeKind::Quad => "Quad",
            SceneNodeKind::Text => "Text",
            SceneNodeKind::Image => "Image",
            SceneNodeKind::Path => "Path",
        }
    }

    /// Returns true for leaf kinds (Quad, Text, Image, Path).
    /// Layer is not a leaf because it can contain children.
    pub fn is_leaf(&self) -> bool {
        !matches!(self, SceneNodeKind::Layer)
    }
}

// ---------------------------------------------------------------------------
// SceneNode
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SceneNode {
    pub id: u64,
    pub kind: SceneNodeKind,
    pub children: Vec<u64>,
    pub z_order: u32,
}

impl SceneNode {
    pub fn new(id: u64, kind: SceneNodeKind, z_order: u32) -> Self {
        Self {
            id,
            kind,
            children: Vec::new(),
            z_order,
        }
    }

    pub fn add_child(&mut self, child_id: u64) {
        self.children.push(child_id);
    }

    pub fn child_count(&self) -> usize {
        self.children.len()
    }
}

// ---------------------------------------------------------------------------
// SceneGraph
// ---------------------------------------------------------------------------

pub struct SceneGraph {
    pub nodes: HashMap<u64, SceneNode>,
    pub root_id: Option<u64>,
}

impl SceneGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            root_id: None,
        }
    }

    /// Insert a node. The first node inserted becomes the root.
    pub fn add_node(&mut self, node: SceneNode) {
        if self.root_id.is_none() {
            self.root_id = Some(node.id);
        }
        self.nodes.insert(node.id, node);
    }

    pub fn get(&self, id: u64) -> Option<&SceneNode> {
        self.nodes.get(&id)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Depth-first traversal starting from `start_id`.
    /// Returns node IDs in DFS visit order (pre-order).
    pub fn depth_first(&self, start_id: u64) -> Vec<u64> {
        let mut result = Vec::new();
        let mut stack = vec![start_id];
        while let Some(current_id) = stack.pop() {
            result.push(current_id);
            if let Some(node) = self.nodes.get(&current_id) {
                // Push children in reverse so the first child is visited first.
                for &child_id in node.children.iter().rev() {
                    stack.push(child_id);
                }
            }
        }
        result
    }

    /// All nodes whose kind reports is_leaf() == true.
    pub fn leaves(&self) -> Vec<u64> {
        let mut ids: Vec<u64> = self
            .nodes
            .values()
            .filter(|n| n.kind.is_leaf())
            .map(|n| n.id)
            .collect();
        ids.sort_unstable();
        ids
    }
}

impl Default for SceneGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// AtlasSlot
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AtlasSlot {
    pub slot_id: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub is_occupied: bool,
}

impl AtlasSlot {
    pub fn new(slot_id: u32, x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            slot_id,
            x,
            y,
            width,
            height,
            is_occupied: false,
        }
    }

    pub fn area(&self) -> u32 {
        self.width * self.height
    }

    /// Returns true when the requested dimensions fit and the slot is free.
    pub fn can_fit(&self, w: u32, h: u32) -> bool {
        w <= self.width && h <= self.height && !self.is_occupied
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod scene_traversal_tests {
    use super::*;

    // Test 1: SceneNodeKind::is_leaf()
    #[test]
    fn test_scene_node_kind_is_leaf() {
        assert!(!SceneNodeKind::Layer.is_leaf(), "Layer should not be a leaf");
        assert!(SceneNodeKind::Quad.is_leaf(), "Quad should be a leaf");
        assert!(SceneNodeKind::Text.is_leaf(), "Text should be a leaf");
        assert!(SceneNodeKind::Image.is_leaf(), "Image should be a leaf");
        assert!(SceneNodeKind::Path.is_leaf(), "Path should be a leaf");
    }

    // Test 2: SceneNode::add_child() + child_count()
    #[test]
    fn test_scene_node_add_child_and_child_count() {
        let mut node = SceneNode::new(1, SceneNodeKind::Layer, 0);
        assert_eq!(node.child_count(), 0);
        node.add_child(2);
        node.add_child(3);
        assert_eq!(node.child_count(), 2);
        assert_eq!(node.children, vec![2, 3]);
    }

    // Test 3: SceneGraph::add_node() sets root
    #[test]
    fn test_scene_graph_add_node_sets_root() {
        let mut graph = SceneGraph::new();
        assert!(graph.root_id.is_none());
        graph.add_node(SceneNode::new(10, SceneNodeKind::Layer, 0));
        assert_eq!(graph.root_id, Some(10));
        // Adding a second node does not change the root.
        graph.add_node(SceneNode::new(20, SceneNodeKind::Quad, 1));
        assert_eq!(graph.root_id, Some(10));
    }

    // Test 4: depth_first() returns all nodes
    #[test]
    fn test_depth_first_returns_all_nodes() {
        let mut graph = SceneGraph::new();
        let mut root = SceneNode::new(1, SceneNodeKind::Layer, 0);
        root.add_child(2);
        root.add_child(3);
        graph.add_node(root);
        graph.add_node(SceneNode::new(2, SceneNodeKind::Quad, 1));
        graph.add_node(SceneNode::new(3, SceneNodeKind::Text, 2));

        let mut visited = graph.depth_first(1);
        visited.sort_unstable();
        assert_eq!(visited, vec![1, 2, 3]);
    }

    // Test 5: depth_first() visits root first
    #[test]
    fn test_depth_first_visits_root_first() {
        let mut graph = SceneGraph::new();
        let mut root = SceneNode::new(1, SceneNodeKind::Layer, 0);
        root.add_child(2);
        root.add_child(3);
        graph.add_node(root);
        graph.add_node(SceneNode::new(2, SceneNodeKind::Quad, 1));
        graph.add_node(SceneNode::new(3, SceneNodeKind::Text, 2));

        let visited = graph.depth_first(1);
        assert_eq!(visited[0], 1, "Root must be visited first");
    }

    // Test 6: leaves() returns only leaf nodes
    #[test]
    fn test_leaves_returns_only_leaf_nodes() {
        let mut graph = SceneGraph::new();
        let mut root = SceneNode::new(1, SceneNodeKind::Layer, 0);
        root.add_child(2);
        root.add_child(3);
        graph.add_node(root);
        graph.add_node(SceneNode::new(2, SceneNodeKind::Quad, 1));
        graph.add_node(SceneNode::new(3, SceneNodeKind::Image, 2));

        let leaves = graph.leaves();
        assert_eq!(leaves, vec![2, 3], "Only Quad and Image should be leaves");
    }

    // Test 7: AtlasSlot::area() calculation
    #[test]
    fn test_atlas_slot_area() {
        let slot = AtlasSlot::new(0, 0, 0, 64, 32);
        assert_eq!(slot.area(), 64 * 32);
    }

    // Test 8: AtlasSlot::can_fit() true when fits
    #[test]
    fn test_atlas_slot_can_fit_true() {
        let slot = AtlasSlot::new(0, 0, 0, 128, 128);
        assert!(slot.can_fit(64, 64), "64x64 should fit in 128x128 free slot");
        assert!(slot.can_fit(128, 128), "exact size should fit");
    }

    // Test 9: AtlasSlot::can_fit() false when occupied
    #[test]
    fn test_atlas_slot_can_fit_false_when_occupied() {
        let mut slot = AtlasSlot::new(0, 0, 0, 128, 128);
        slot.is_occupied = true;
        assert!(
            !slot.can_fit(64, 64),
            "should not fit when slot is occupied"
        );
    }
}
