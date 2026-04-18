/// Block tree — hierarchical node structure for canvas block organization.
use std::collections::{HashMap, VecDeque};

/// Classifies the role of a node in the block tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockNodeKind {
    /// Tree root — owns the entire hierarchy.
    Root,
    /// Internal grouping node that may hold children.
    Container,
    /// Terminal content node.
    Leaf,
    /// Pointer to another node by id.
    Reference,
}

impl BlockNodeKind {
    /// Returns true if this kind is allowed to hold children.
    pub fn can_have_children(&self) -> bool {
        matches!(self, BlockNodeKind::Root | BlockNodeKind::Container)
    }

    /// Numeric code for this kind: Root=0, Container=1, Leaf=2, Reference=3.
    pub fn node_code(&self) -> u8 {
        match self {
            BlockNodeKind::Root => 0,
            BlockNodeKind::Container => 1,
            BlockNodeKind::Leaf => 2,
            BlockNodeKind::Reference => 3,
        }
    }
}

/// A single node in a `BlockTree`.
#[derive(Debug, Clone)]
pub struct BlockNode {
    /// Unique identifier for this node.
    pub id: u64,
    /// Structural role of this node.
    pub kind: BlockNodeKind,
    /// Human-readable label.
    pub label: String,
    /// Ordered child ids.
    pub children: Vec<u64>,
}

impl BlockNode {
    /// Append a child id.
    pub fn add_child(&mut self, child_id: u64) {
        self.children.push(child_id);
    }

    /// Number of direct children.
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// True when this node is a `Leaf` kind or has no children.
    pub fn is_leaf_node(&self) -> bool {
        self.kind == BlockNodeKind::Leaf || self.children.is_empty()
    }
}

/// A tree of `BlockNode`s indexed by their `u64` ids.
#[derive(Debug, Clone, Default)]
pub struct BlockTree {
    /// All nodes keyed by id.
    pub nodes: HashMap<u64, BlockNode>,
    /// Id of the designated root, if any.
    pub root_id: Option<u64>,
}

impl BlockTree {
    /// Insert a node. If its kind is `Root`, it also sets `root_id`.
    pub fn insert(&mut self, node: BlockNode) {
        if node.kind == BlockNodeKind::Root {
            self.root_id = Some(node.id);
        }
        self.nodes.insert(node.id, node);
    }

    /// Look up a node by id.
    pub fn get(&self, id: u64) -> Option<&BlockNode> {
        self.nodes.get(&id)
    }

    /// Return references to the direct children of `id` in child-order.
    pub fn children_of(&self, id: u64) -> Vec<&BlockNode> {
        match self.nodes.get(&id) {
            None => vec![],
            Some(node) => node
                .children
                .iter()
                .filter_map(|cid| self.nodes.get(cid))
                .collect(),
        }
    }

    /// Depth of `id` from root (root = 0). Returns 0 if id is the root or not found.
    pub fn depth_of(&self, id: u64) -> usize {
        let root = match self.root_id {
            None => return 0,
            Some(r) => r,
        };
        if id == root {
            return 0;
        }
        self.depth_recursive(root, id, 0).unwrap_or(0)
    }

    fn depth_recursive(&self, current: u64, target: u64, depth: usize) -> Option<usize> {
        let node = self.nodes.get(&current)?;
        for &child_id in &node.children {
            if child_id == target {
                return Some(depth + 1);
            }
            if let Some(d) = self.depth_recursive(child_id, target, depth + 1) {
                return Some(d);
            }
        }
        None
    }
}

/// BFS walker over a `BlockTree`.
pub struct BlockTreeWalker {
    /// The tree being walked.
    pub tree: BlockTree,
}

impl BlockTreeWalker {
    /// Wrap a tree.
    pub fn new(tree: BlockTree) -> Self {
        Self { tree }
    }

    /// BFS traversal from `root_id`; returns node ids in visit order.
    /// Returns an empty vec if there is no root.
    pub fn walk_bfs(&self) -> Vec<u64> {
        let root = match self.tree.root_id {
            None => return vec![],
            Some(r) => r,
        };
        let mut order = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(root);
        while let Some(id) = queue.pop_front() {
            order.push(id);
            if let Some(node) = self.tree.nodes.get(&id) {
                for &child in &node.children {
                    queue.push_back(child);
                }
            }
        }
        order
    }

    /// Return ids of all nodes that satisfy `is_leaf_node`.
    pub fn leaf_ids(&self) -> Vec<u64> {
        self.tree
            .nodes
            .values()
            .filter(|n| n.is_leaf_node())
            .map(|n| n.id)
            .collect()
    }
}

/// Diff between two tree snapshots.
#[derive(Debug, Clone, Default)]
pub struct TreeDiff {
    /// Ids present in the new tree but not the old.
    pub added_ids: Vec<u64>,
    /// Ids present in the old tree but not the new.
    pub removed_ids: Vec<u64>,
}

impl TreeDiff {
    /// True when there are no additions or removals.
    pub fn is_empty(&self) -> bool {
        self.added_ids.is_empty() && self.removed_ids.is_empty()
    }

    /// Sum of added and removed counts.
    pub fn total_changes(&self) -> usize {
        self.added_ids.len() + self.removed_ids.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: u64, kind: BlockNodeKind, label: &str) -> BlockNode {
        BlockNode { id, kind, label: label.into(), children: vec![] }
    }

    // 1. can_have_children returns true for Root and Container only
    #[test]
    fn kind_can_have_children() {
        assert!(BlockNodeKind::Root.can_have_children());
        assert!(BlockNodeKind::Container.can_have_children());
        assert!(!BlockNodeKind::Leaf.can_have_children());
        assert!(!BlockNodeKind::Reference.can_have_children());
    }

    // 2. node_code returns correct u8 per variant
    #[test]
    fn kind_node_code() {
        assert_eq!(BlockNodeKind::Root.node_code(), 0);
        assert_eq!(BlockNodeKind::Container.node_code(), 1);
        assert_eq!(BlockNodeKind::Leaf.node_code(), 2);
        assert_eq!(BlockNodeKind::Reference.node_code(), 3);
    }

    // 3. add_child and child_count
    #[test]
    fn node_add_child_and_count() {
        let mut node = make_node(1, BlockNodeKind::Container, "c");
        assert_eq!(node.child_count(), 0);
        node.add_child(10);
        node.add_child(11);
        assert_eq!(node.child_count(), 2);
        assert_eq!(node.children, vec![10, 11]);
    }

    // 4. is_leaf_node: Leaf kind => true, empty children => true, non-empty Container => false
    #[test]
    fn node_is_leaf_node() {
        let leaf = make_node(1, BlockNodeKind::Leaf, "l");
        assert!(leaf.is_leaf_node());

        let mut container = make_node(2, BlockNodeKind::Container, "c");
        // No children yet — counts as leaf
        assert!(container.is_leaf_node());
        container.add_child(99);
        assert!(!container.is_leaf_node());
    }

    // 5. tree insert and get
    #[test]
    fn tree_insert_and_get() {
        let mut tree = BlockTree::default();
        let node = make_node(42, BlockNodeKind::Root, "root");
        tree.insert(node);
        assert_eq!(tree.root_id, Some(42));
        let got = tree.get(42).unwrap();
        assert_eq!(got.id, 42);
        assert_eq!(got.label, "root");
        assert!(tree.get(99).is_none());
    }

    // 6. children_of returns correct child references
    #[test]
    fn tree_children_of() {
        let mut tree = BlockTree::default();
        let mut root = make_node(1, BlockNodeKind::Root, "r");
        root.add_child(2);
        root.add_child(3);
        tree.insert(root);
        tree.insert(make_node(2, BlockNodeKind::Leaf, "a"));
        tree.insert(make_node(3, BlockNodeKind::Leaf, "b"));

        let children = tree.children_of(1);
        let mut ids: Vec<u64> = children.iter().map(|n| n.id).collect();
        ids.sort();
        assert_eq!(ids, vec![2, 3]);
        assert!(tree.children_of(99).is_empty());
    }

    // 7. walk_bfs returns ids in breadth-first order
    #[test]
    fn walker_walk_bfs_order() {
        // Tree:   1 -> [2, 3],  2 -> [4, 5]
        let mut tree = BlockTree::default();
        let mut root = make_node(1, BlockNodeKind::Root, "r");
        root.add_child(2);
        root.add_child(3);
        let mut mid = make_node(2, BlockNodeKind::Container, "m");
        mid.add_child(4);
        mid.add_child(5);
        tree.insert(root);
        tree.insert(mid);
        tree.insert(make_node(3, BlockNodeKind::Leaf, "x"));
        tree.insert(make_node(4, BlockNodeKind::Leaf, "y"));
        tree.insert(make_node(5, BlockNodeKind::Leaf, "z"));

        let walker = BlockTreeWalker::new(tree);
        assert_eq!(walker.walk_bfs(), vec![1, 2, 3, 4, 5]);
    }

    // 8. leaf_ids returns all nodes satisfying is_leaf_node
    #[test]
    fn walker_leaf_ids() {
        let mut tree = BlockTree::default();
        let mut root = make_node(1, BlockNodeKind::Root, "r");
        root.add_child(2);
        root.add_child(3);
        tree.insert(root);
        tree.insert(make_node(2, BlockNodeKind::Leaf, "a"));
        tree.insert(make_node(3, BlockNodeKind::Container, "b")); // no children → leaf

        let walker = BlockTreeWalker::new(tree);
        let mut ids = walker.leaf_ids();
        ids.sort();
        // node 1 (Root) has children so NOT a leaf; nodes 2 and 3 qualify
        assert_eq!(ids, vec![2, 3]);
    }

    // 9. TreeDiff total_changes
    #[test]
    fn diff_total_changes() {
        let empty_diff = TreeDiff::default();
        assert!(empty_diff.is_empty());
        assert_eq!(empty_diff.total_changes(), 0);

        let diff = TreeDiff {
            added_ids: vec![10, 11, 12],
            removed_ids: vec![5],
        };
        assert!(!diff.is_empty());
        assert_eq!(diff.total_changes(), 4);
    }
}
