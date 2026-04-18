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
