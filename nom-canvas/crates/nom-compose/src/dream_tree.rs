use std::collections::HashMap;

/// A node in the dream tree, representing a candidate app design scored 0.0–100.0.
pub struct DreamNode {
    pub id: u64,
    pub label: String,
    pub score: f32,
    pub children: Vec<u64>,
}

impl DreamNode {
    pub fn new(id: u64, label: impl Into<String>, score: f32) -> Self {
        Self {
            id,
            label: label.into(),
            score,
            children: Vec::new(),
        }
    }

    /// Returns true when score meets the EPIC threshold (>= 95.0).
    pub fn is_epic(&self) -> bool {
        self.score >= 95.0
    }

    pub fn add_child(&mut self, child_id: u64) {
        self.children.push(child_id);
    }
}

/// Tree of dream nodes keyed by id.
pub struct DreamTree {
    pub nodes: HashMap<u64, DreamNode>,
    pub root_id: Option<u64>,
}

impl DreamTree {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            root_id: None,
        }
    }

    /// Insert a node; sets root_id on the first insertion. Returns the node id.
    pub fn add_node(&mut self, node: DreamNode) -> u64 {
        let id = node.id;
        if self.root_id.is_none() {
            self.root_id = Some(id);
        }
        self.nodes.insert(id, node);
        id
    }

    pub fn get(&self, id: u64) -> Option<&DreamNode> {
        self.nodes.get(&id)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the node with the highest score, or None for an empty tree.
    pub fn find_best(&self) -> Option<&DreamNode> {
        self.nodes
            .values()
            .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal))
    }
}

impl Default for DreamTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Pareto-optimal front: a node is dominated if another node has score >= its
/// score AND children.len() >= its children.len() (strictly greater on at least
/// one dimension).  Only non-dominated nodes appear on the front.
pub struct ParetoFront;

impl ParetoFront {
    pub fn compute(nodes: &[&DreamNode]) -> Vec<u64> {
        let mut front = Vec::new();
        for candidate in nodes {
            let dominated = nodes.iter().any(|other| {
                other.id != candidate.id
                    && other.score >= candidate.score
                    && other.children.len() >= candidate.children.len()
                    && (other.score > candidate.score
                        || other.children.len() > candidate.children.len())
            });
            if !dominated {
                front.push(candidate.id);
            }
        }
        front
    }

    pub fn front_size(nodes: &[&DreamNode]) -> usize {
        Self::compute(nodes).len()
    }
}

#[cfg(test)]
mod dream_tree_tests {
    use super::*;

    #[test]
    fn test_dream_node_fields() {
        let node = DreamNode::new(1, "root", 80.0);
        assert_eq!(node.id, 1);
        assert_eq!(node.label, "root");
        assert!((node.score - 80.0).abs() < f32::EPSILON);
        assert!(node.children.is_empty());
    }

    #[test]
    fn test_is_epic_true_for_95() {
        let node = DreamNode::new(2, "epic", 95.0);
        assert!(node.is_epic());
        let node2 = DreamNode::new(3, "almost", 94.9);
        assert!(!node2.is_epic());
    }

    #[test]
    fn test_add_child_grows_children() {
        let mut node = DreamNode::new(4, "parent", 50.0);
        assert_eq!(node.children.len(), 0);
        node.add_child(10);
        node.add_child(11);
        assert_eq!(node.children.len(), 2);
        assert_eq!(node.children[0], 10);
        assert_eq!(node.children[1], 11);
    }

    #[test]
    fn test_dream_tree_new_empty() {
        let tree = DreamTree::new();
        assert_eq!(tree.node_count(), 0);
        assert!(tree.root_id.is_none());
    }

    #[test]
    fn test_add_node_increments_count() {
        let mut tree = DreamTree::new();
        let n1 = DreamNode::new(1, "a", 10.0);
        let n2 = DreamNode::new(2, "b", 20.0);
        tree.add_node(n1);
        assert_eq!(tree.node_count(), 1);
        tree.add_node(n2);
        assert_eq!(tree.node_count(), 2);
    }

    #[test]
    fn test_find_best_returns_highest_score() {
        let mut tree = DreamTree::new();
        tree.add_node(DreamNode::new(1, "low", 30.0));
        tree.add_node(DreamNode::new(2, "high", 90.0));
        tree.add_node(DreamNode::new(3, "mid", 60.0));
        let best = tree.find_best().expect("should have a best node");
        assert_eq!(best.id, 2);
        assert!((best.score - 90.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_find_best_empty_returns_none() {
        let tree = DreamTree::new();
        assert!(tree.find_best().is_none());
    }

    #[test]
    fn test_pareto_compute_single_node() {
        let node = DreamNode::new(99, "solo", 70.0);
        let nodes: Vec<&DreamNode> = vec![&node];
        let front = ParetoFront::compute(&nodes);
        assert_eq!(front.len(), 1);
        assert_eq!(front[0], 99);
    }

    #[test]
    fn test_pareto_front_size_nonempty() {
        let n1 = DreamNode::new(1, "a", 80.0);
        let n2 = DreamNode::new(2, "b", 60.0);
        let nodes: Vec<&DreamNode> = vec![&n1, &n2];
        assert!(ParetoFront::front_size(&nodes) >= 1);
    }
}
