//! Animated reasoning-card hypothesis tree.
//!
//! Provides [`HypothesisNodeState`], [`ReasoningNode`], [`HypothesisTree`], and
//! [`BeliefPropagator`] for building and reasoning over tree-structured hypotheses.

#![deny(unsafe_code)]

// ---------------------------------------------------------------------------
// NodeState
// ---------------------------------------------------------------------------

/// State of a single reasoning node in the hypothesis tree.
#[derive(Clone, Debug, PartialEq)]
pub enum HypothesisNodeState {
    /// Node represents an open hypothesis that has not yet been resolved.
    Hypothesis,
    /// Node has been confirmed by evidence.
    Confirmed,
    /// Node has been refuted by evidence.
    Refuted,
}

impl HypothesisNodeState {
    /// Human-readable name for the state.
    pub fn state_name(&self) -> &str {
        match self {
            HypothesisNodeState::Hypothesis => "Hypothesis",
            HypothesisNodeState::Confirmed => "Confirmed",
            HypothesisNodeState::Refuted => "Refuted",
        }
    }

    /// Returns `true` when the state is a terminal one (Confirmed or Refuted).
    pub fn is_terminal(&self) -> bool {
        matches!(self, HypothesisNodeState::Confirmed | HypothesisNodeState::Refuted)
    }
}

// ---------------------------------------------------------------------------
// ReasoningNode
// ---------------------------------------------------------------------------

/// A single node in the hypothesis tree.
#[derive(Clone, Debug)]
pub struct ReasoningNode {
    /// Unique identifier for this node.
    pub id: u64,
    /// The hypothesis text.
    pub hypothesis: String,
    /// Confidence score in the range `[0.0, 1.0]`.
    pub confidence: f32,
    /// Current state of this node.
    pub state: HypothesisNodeState,
    /// Evidence strings that support or refute this node.
    pub evidence: Vec<String>,
}

impl ReasoningNode {
    /// Create a new node in the [`HypothesisNodeState::Hypothesis`] state.
    pub fn new(id: u64, hypothesis: impl Into<String>, confidence: f32) -> Self {
        Self {
            id,
            hypothesis: hypothesis.into(),
            confidence: confidence.clamp(0.0, 1.0),
            state: HypothesisNodeState::Hypothesis,
            evidence: Vec::new(),
        }
    }

    /// Append a piece of evidence to this node.
    pub fn add_evidence(&mut self, e: impl Into<String>) {
        self.evidence.push(e.into());
    }

    /// Transition the node to the [`HypothesisNodeState::Confirmed`] state.
    pub fn confirm(&mut self) {
        self.state = HypothesisNodeState::Confirmed;
    }

    /// Transition the node to the [`HypothesisNodeState::Refuted`] state.
    pub fn refute(&mut self) {
        self.state = HypothesisNodeState::Refuted;
    }

    /// Returns the number of evidence strings attached to this node.
    pub fn evidence_count(&self) -> usize {
        self.evidence.len()
    }
}

// ---------------------------------------------------------------------------
// HypothesisTree
// ---------------------------------------------------------------------------

/// A tree of [`ReasoningNode`]s connected by parent→child edges.
#[derive(Default)]
pub struct HypothesisTree {
    /// All nodes stored in the tree.
    pub nodes: Vec<ReasoningNode>,
    /// Directed edges as `(parent_id, child_id)` pairs.
    pub edges: Vec<(u64, u64)>,
}

impl HypothesisTree {
    /// Create an empty hypothesis tree.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node to the tree.
    pub fn add_node(&mut self, node: ReasoningNode) {
        self.nodes.push(node);
    }

    /// Record a parent→child relationship between two nodes already in the tree.
    pub fn add_child_of(&mut self, parent_id: u64, child_id: u64) {
        self.edges.push((parent_id, child_id));
    }

    /// Look up a node by its `id`.
    pub fn get_node(&self, id: u64) -> Option<&ReasoningNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Return the ids of all direct children of `id`.
    pub fn children_of(&self, id: u64) -> Vec<u64> {
        self.edges
            .iter()
            .filter(|(parent, _)| *parent == id)
            .map(|(_, child)| *child)
            .collect()
    }

    /// Total number of nodes currently in the tree.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

// ---------------------------------------------------------------------------
// BeliefPropagator
// ---------------------------------------------------------------------------

/// Propagates belief (confidence) scores through a [`HypothesisTree`].
#[derive(Default)]
pub struct BeliefPropagator;

impl BeliefPropagator {
    /// Create a new propagator.
    pub fn new() -> Self {
        Self
    }

    /// Compute the average confidence of `node_id` and all of its descendants.
    ///
    /// Uses a breadth-first traversal. Returns `0.0` if the node is not found.
    pub fn propagate_confidence(&self, tree: &HypothesisTree, node_id: u64) -> f32 {
        let mut total = 0.0f32;
        let mut count = 0u32;
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(node_id);

        while let Some(current) = queue.pop_front() {
            if let Some(node) = tree.get_node(current) {
                total += node.confidence;
                count += 1;
                for child_id in tree.children_of(current) {
                    queue.push_back(child_id);
                }
            }
        }

        if count == 0 {
            0.0
        } else {
            total / count as f32
        }
    }

    /// Return the `id` of the node with the highest confidence, or `None` if the tree is empty.
    pub fn highest_confidence_node(&self, tree: &HypothesisTree) -> Option<u64> {
        tree.nodes
            .iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal))
            .map(|n| n.id)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod hypothesis_tree_tests {
    use super::*;

    #[test]
    fn node_state_is_terminal_confirmed() {
        let state = HypothesisNodeState::Confirmed;
        assert!(state.is_terminal());
    }

    #[test]
    fn node_state_is_terminal_hypothesis_false() {
        let state = HypothesisNodeState::Hypothesis;
        assert!(!state.is_terminal());
    }

    #[test]
    fn reasoning_node_add_evidence() {
        let mut node = ReasoningNode::new(1, "the sky is blue", 0.8);
        assert_eq!(node.evidence_count(), 0);
        node.add_evidence("observation at noon");
        node.add_evidence("spectral analysis");
        assert_eq!(node.evidence_count(), 2);
    }

    #[test]
    fn reasoning_node_confirm_changes_state() {
        let mut node = ReasoningNode::new(2, "water boils at 100°C", 0.9);
        assert_eq!(node.state, HypothesisNodeState::Hypothesis);
        node.confirm();
        assert_eq!(node.state, HypothesisNodeState::Confirmed);
        assert!(node.state.is_terminal());
    }

    #[test]
    fn hypothesis_tree_add_node_and_children() {
        let mut tree = HypothesisTree::new();
        tree.add_node(ReasoningNode::new(10, "root", 0.7));
        tree.add_node(ReasoningNode::new(11, "child-a", 0.5));
        tree.add_node(ReasoningNode::new(12, "child-b", 0.6));
        tree.add_child_of(10, 11);
        tree.add_child_of(10, 12);
        assert_eq!(tree.node_count(), 3);
        assert_eq!(tree.edges.len(), 2);
    }

    #[test]
    fn hypothesis_tree_children_of() {
        let mut tree = HypothesisTree::new();
        tree.add_node(ReasoningNode::new(20, "parent", 0.5));
        tree.add_node(ReasoningNode::new(21, "child1", 0.4));
        tree.add_node(ReasoningNode::new(22, "child2", 0.6));
        tree.add_child_of(20, 21);
        tree.add_child_of(20, 22);
        let mut children = tree.children_of(20);
        children.sort_unstable();
        assert_eq!(children, vec![21, 22]);
    }

    #[test]
    fn hypothesis_tree_get_node() {
        let mut tree = HypothesisTree::new();
        tree.add_node(ReasoningNode::new(30, "lookup-me", 0.55));
        let node = tree.get_node(30).expect("node should exist");
        assert_eq!(node.id, 30);
        assert_eq!(node.hypothesis, "lookup-me");
        assert!(tree.get_node(99).is_none());
    }

    #[test]
    fn belief_propagator_propagate_confidence() {
        let mut tree = HypothesisTree::new();
        // root confidence = 0.8, child = 0.4  → avg = 0.6
        tree.add_node(ReasoningNode::new(40, "root", 0.8));
        tree.add_node(ReasoningNode::new(41, "child", 0.4));
        tree.add_child_of(40, 41);

        let bp = BeliefPropagator::new();
        let avg = bp.propagate_confidence(&tree, 40);
        let expected = (0.8 + 0.4) / 2.0;
        assert!((avg - expected).abs() < 1e-5, "expected {expected}, got {avg}");
    }

    #[test]
    fn belief_propagator_highest_confidence() {
        let mut tree = HypothesisTree::new();
        tree.add_node(ReasoningNode::new(50, "low", 0.2));
        tree.add_node(ReasoningNode::new(51, "high", 0.95));
        tree.add_node(ReasoningNode::new(52, "mid", 0.6));

        let bp = BeliefPropagator::new();
        let best = bp.highest_confidence_node(&tree);
        assert_eq!(best, Some(51));
    }
}
