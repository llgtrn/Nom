/// A node in a content-addressed DAG.
#[derive(Debug, Clone)]
pub struct DagNode {
    /// Unique node identifier.
    pub id: u32,
    /// Simple byte-sum hash of the node's content string.
    pub content_hash: u64,
    /// Human-readable label.
    pub label: String,
}

impl DagNode {
    /// Create a new node. `content_hash` is the byte-sum of `content`.
    pub fn new(id: u32, content: &str, label: &str) -> Self {
        let content_hash = content.bytes().map(u64::from).sum();
        DagNode {
            id,
            content_hash,
            label: label.to_owned(),
        }
    }
}

/// A directed edge between two [`DagNode`]s.
#[derive(Debug, Clone)]
pub struct DagEdge {
    /// Source node id.
    pub from: u32,
    /// Destination node id.
    pub to: u32,
    /// Relationship kind label.
    pub kind: String,
}

impl DagEdge {
    /// Create a new directed edge.
    pub fn new(from: u32, to: u32, kind: &str) -> Self {
        DagEdge {
            from,
            to,
            kind: kind.to_owned(),
        }
    }
}

/// A content-addressed directed acyclic graph.
pub struct ContentDag {
    nodes: Vec<DagNode>,
    edges: Vec<DagEdge>,
}

impl ContentDag {
    /// Create an empty DAG.
    pub fn new() -> Self {
        ContentDag {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Add a node and return its id.
    pub fn add_node(&mut self, id: u32, content: &str, label: &str) -> u32 {
        self.nodes.push(DagNode::new(id, content, label));
        id
    }

    /// Add a directed edge.
    pub fn add_edge(&mut self, from: u32, to: u32, kind: &str) {
        self.edges.push(DagEdge::new(from, to, kind));
    }

    /// Number of nodes in the DAG.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of edges in the DAG.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Find the first node whose `content_hash` matches `hash`.
    pub fn find_by_hash(&self, hash: u64) -> Option<&DagNode> {
        self.nodes.iter().find(|n| n.content_hash == hash)
    }
}

impl Default for ContentDag {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dag_node_new() {
        let node = DagNode::new(1, "abc", "my-node");
        assert_eq!(node.id, 1);
        assert_eq!(node.label, "my-node");
        // byte-sum of "abc" = 97+98+99 = 294
        assert_eq!(node.content_hash, 294);
    }

    #[test]
    fn dag_edge_new() {
        let edge = DagEdge::new(1, 2, "depends");
        assert_eq!(edge.from, 1);
        assert_eq!(edge.to, 2);
        assert_eq!(edge.kind, "depends");
    }

    #[test]
    fn dag_add_node() {
        let mut dag = ContentDag::new();
        let id = dag.add_node(10, "content", "label");
        assert_eq!(id, 10);
        assert_eq!(dag.node_count(), 1);
    }

    #[test]
    fn dag_add_edge() {
        let mut dag = ContentDag::new();
        dag.add_node(1, "a", "node-a");
        dag.add_node(2, "b", "node-b");
        dag.add_edge(1, 2, "links");
        assert_eq!(dag.edge_count(), 1);
    }

    #[test]
    fn dag_find_by_hash() {
        let mut dag = ContentDag::new();
        dag.add_node(5, "abc", "target");
        // byte-sum of "abc" = 294
        let found = dag.find_by_hash(294);
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, 5);
        assert!(dag.find_by_hash(0).is_none());
    }

    #[test]
    fn dag_counts() {
        let mut dag = ContentDag::new();
        dag.add_node(1, "x", "n1");
        dag.add_node(2, "y", "n2");
        dag.add_edge(1, 2, "edge");
        assert_eq!(dag.node_count(), 2);
        assert_eq!(dag.edge_count(), 1);
    }
}

#[cfg(test)]
mod content_dag_integration_tests {
    use super::*;

    /// Build a multi-node, multi-edge DAG and verify counts are accurate.
    #[test]
    fn multi_node_multi_edge_counts() {
        let mut dag = ContentDag::new();
        dag.add_node(1, "root", "Root");
        dag.add_node(2, "child-a", "ChildA");
        dag.add_node(3, "child-b", "ChildB");
        dag.add_edge(1, 2, "has_child");
        dag.add_edge(1, 3, "has_child");
        assert_eq!(dag.node_count(), 3);
        assert_eq!(dag.edge_count(), 2);
    }

    /// find_by_hash returns the correct node when the hash is present.
    #[test]
    fn hash_lookup_returns_correct_node() {
        let mut dag = ContentDag::new();
        dag.add_node(10, "hello", "node-hello");
        dag.add_node(20, "world", "node-world");
        // byte-sum of "hello" = 104+101+108+108+111 = 532
        let found = dag.find_by_hash(532);
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, 10);
        assert_eq!(found.unwrap().label, "node-hello");
    }

    /// Inserting two nodes with identical content gives both the same content_hash,
    /// but find_by_hash returns the first one (dedup-by-hash semantics).
    #[test]
    fn duplicate_content_same_hash() {
        let mut dag = ContentDag::new();
        dag.add_node(1, "dup", "first");
        dag.add_node(2, "dup", "second");
        // Both have the same content_hash; find_by_hash returns the first.
        let found = dag.find_by_hash(dag.nodes[0].content_hash);
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, 1);
    }

    /// Edge traversal: verify that edges record from/to ids and kind correctly.
    #[test]
    fn edge_traversal_from_to_kind() {
        let mut dag = ContentDag::new();
        dag.add_node(5, "src", "Source");
        dag.add_node(6, "dst", "Dest");
        dag.add_edge(5, 6, "depends_on");
        let edge = &dag.edges[0];
        assert_eq!(edge.from, 5);
        assert_eq!(edge.to, 6);
        assert_eq!(edge.kind, "depends_on");
    }

    /// find_by_hash returns None when no node with that hash exists.
    #[test]
    fn find_by_hash_returns_none_for_missing() {
        let mut dag = ContentDag::new();
        dag.add_node(1, "present", "P");
        assert!(dag.find_by_hash(0).is_none());
        assert!(dag.find_by_hash(u64::MAX).is_none());
    }

    /// node_count and edge_count are both 0 for a freshly created DAG.
    #[test]
    fn empty_dag_has_zero_counts() {
        let dag = ContentDag::new();
        assert_eq!(dag.node_count(), 0);
        assert_eq!(dag.edge_count(), 0);
    }

    /// Adding the same node id twice still increments node_count (no structural dedup by id).
    #[test]
    fn adding_same_id_twice_increments_count() {
        let mut dag = ContentDag::new();
        dag.add_node(42, "data", "LabelA");
        dag.add_node(42, "data", "LabelB");
        // ContentDag does not deduplicate by id — both are stored.
        assert_eq!(dag.node_count(), 2);
    }

    /// Linear chain of 3 nodes: A→B→C — verify 3 nodes and 2 edges with correct topology.
    #[test]
    fn linear_chain_three_nodes() {
        let mut dag = ContentDag::new();
        dag.add_node(1, "alpha", "A");
        dag.add_node(2, "beta", "B");
        dag.add_node(3, "gamma", "C");
        dag.add_edge(1, 2, "next");
        dag.add_edge(2, 3, "next");
        assert_eq!(dag.node_count(), 3);
        assert_eq!(dag.edge_count(), 2);
        assert_eq!(dag.edges[0].from, 1);
        assert_eq!(dag.edges[0].to, 2);
        assert_eq!(dag.edges[1].from, 2);
        assert_eq!(dag.edges[1].to, 3);
    }
}
