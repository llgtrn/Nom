#![deny(unsafe_code)]
use crate::node::{ExecNode, NodeId};
use std::collections::{HashMap, HashSet, VecDeque};

pub struct Edge {
    pub src_node: NodeId,
    pub src_port: String,
    pub dst_node: NodeId,
    pub dst_port: String,
    /// Confidence weight for this edge in [0.0, 1.0].  Defaults to `1.0`.
    pub confidence: f32,
}

pub struct Dag {
    pub nodes: HashMap<NodeId, ExecNode>,
    pub edges: Vec<Edge>,
}

impl Dag {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: ExecNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn add_edge(
        &mut self,
        src_node: impl Into<String>,
        src_port: impl Into<String>,
        dst_node: impl Into<String>,
        dst_port: impl Into<String>,
    ) {
        self.edges.push(Edge {
            src_node: src_node.into(),
            src_port: src_port.into(),
            dst_node: dst_node.into(),
            dst_port: dst_port.into(),
            confidence: 1.0,
        });
    }

    /// Add an edge with an explicit confidence weight in [0.0, 1.0].
    pub fn add_edge_weighted(
        &mut self,
        src_node: impl Into<String>,
        src_port: impl Into<String>,
        dst_node: impl Into<String>,
        dst_port: impl Into<String>,
        confidence: f32,
    ) {
        self.edges.push(Edge {
            src_node: src_node.into(),
            src_port: src_port.into(),
            dst_node: dst_node.into(),
            dst_port: dst_port.into(),
            confidence: confidence.clamp(0.0, 1.0),
        });
    }

    /// Kahn topological sort (ComfyUI pattern: blockCount + blocking dicts)
    /// Returns Ok(sorted_ids) or Err(cycle_nodes)
    pub fn topological_sort(&self) -> Result<Vec<NodeId>, Vec<NodeId>> {
        // block_count[node] = number of unresolved input dependencies
        let mut block_count: HashMap<String, usize> = HashMap::new();
        // blocking[node] = list of nodes that this node blocks
        let mut blocking: HashMap<String, Vec<String>> = HashMap::new();

        for id in self.nodes.keys() {
            block_count.entry(id.clone()).or_insert(0);
            blocking.entry(id.clone()).or_default();
        }

        for edge in &self.edges {
            *block_count.entry(edge.dst_node.clone()).or_insert(0) += 1;
            blocking
                .entry(edge.src_node.clone())
                .or_default()
                .push(edge.dst_node.clone());
        }

        let mut queue: VecDeque<String> = block_count
            .iter()
            .filter(|(_, &count)| count == 0)
            .map(|(id, _)| id.clone())
            .collect();
        queue.make_contiguous().sort(); // deterministic order

        let mut result: Vec<NodeId> = Vec::new();
        while let Some(node_id) = queue.pop_front() {
            if let Some(dependents) = blocking.get(&node_id) {
                let deps: Vec<String> = dependents.clone();
                for dep in deps {
                    let count = block_count.entry(dep.clone()).or_insert(0);
                    *count -= 1;
                    if *count == 0 {
                        queue.push_back(dep);
                    }
                }
            }
            result.push(node_id);
        }

        if result.len() == self.nodes.len() {
            Ok(result)
        } else {
            // Cycle: return unresolved nodes
            let resolved: HashSet<&str> = result.iter().map(|s| s.as_ref()).collect();
            let cycle_nodes = self
                .nodes
                .keys()
                .filter(|id| !resolved.contains(id.as_str()))
                .cloned()
                .collect();
            Err(cycle_nodes)
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

impl Default for Dag {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::ExecNode;

    fn dag_linear() -> Dag {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("a", "verb"));
        dag.add_node(ExecNode::new("b", "verb"));
        dag.add_node(ExecNode::new("c", "verb"));
        dag.add_edge("a", "out", "b", "in");
        dag.add_edge("b", "out", "c", "in");
        dag
    }

    #[test]
    fn topological_sort_linear() {
        let dag = dag_linear();
        let sorted = dag.topological_sort().unwrap();
        assert_eq!(sorted, vec!["a", "b", "c"]);
    }

    #[test]
    fn topological_sort_detects_cycle() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("x", "verb"));
        dag.add_node(ExecNode::new("y", "verb"));
        dag.add_edge("x", "out", "y", "in");
        dag.add_edge("y", "out", "x", "in");
        let result = dag.topological_sort();
        assert!(result.is_err());
    }

    #[test]
    fn topological_sort_parallel() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("root", "verb"));
        dag.add_node(ExecNode::new("branch1", "verb"));
        dag.add_node(ExecNode::new("branch2", "verb"));
        dag.add_node(ExecNode::new("merge", "verb"));
        dag.add_edge("root", "out", "branch1", "in");
        dag.add_edge("root", "out", "branch2", "in");
        dag.add_edge("branch1", "out", "merge", "in");
        dag.add_edge("branch2", "out", "merge", "in");
        let sorted = dag.topological_sort().unwrap();
        assert_eq!(sorted.len(), 4);
        assert_eq!(sorted[0], "root");
        assert_eq!(sorted[3], "merge");
    }

    #[test]
    fn dag_add_edge_connects_nodes() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("src", "verb"));
        dag.add_node(ExecNode::new("dst", "verb"));
        dag.add_edge("src", "out", "dst", "in");
        assert_eq!(dag.edge_count(), 1);
        let edge = &dag.edges[0];
        assert_eq!(edge.src_node, "src");
        assert_eq!(edge.src_port, "out");
        assert_eq!(edge.dst_node, "dst");
        assert_eq!(edge.dst_port, "in");
    }

    #[test]
    fn dag_topological_order_respects_deps() {
        // Build a chain: p -> q -> r. Topological order must be [p, q, r].
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("p", "verb"));
        dag.add_node(ExecNode::new("q", "verb"));
        dag.add_node(ExecNode::new("r", "verb"));
        dag.add_edge("p", "o", "q", "i");
        dag.add_edge("q", "o", "r", "i");
        let order = dag.topological_sort().unwrap();
        assert_eq!(order, vec!["p", "q", "r"]);
        // p must come before q, q before r
        let pos = |id: &str| order.iter().position(|x| x == id).unwrap();
        assert!(pos("p") < pos("q"));
        assert!(pos("q") < pos("r"));
    }

    #[test]
    fn dag_remove_node_cleans_edges() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("alpha", "verb"));
        dag.add_node(ExecNode::new("beta", "verb"));
        dag.add_edge("alpha", "out", "beta", "in");
        assert_eq!(dag.node_count(), 2);
        assert_eq!(dag.edge_count(), 1);
        // Remove "alpha" and any edges that reference it.
        dag.nodes.remove("alpha");
        dag.edges
            .retain(|e| e.src_node != "alpha" && e.dst_node != "alpha");
        assert_eq!(dag.node_count(), 1);
        assert_eq!(dag.edge_count(), 0);
    }

    #[test]
    fn dag_weighted_edge_clamps_confidence_to_range() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("src", "verb"));
        dag.add_node(ExecNode::new("dst", "verb"));
        dag.add_edge_weighted("src", "out", "dst", "in", 1.5);
        assert_eq!(dag.edge_count(), 1);
        assert_eq!(
            dag.edges[0].confidence, 1.0,
            "confidence above 1.0 must be clamped to 1.0"
        );
    }

    #[test]
    fn dag_default_edge_confidence_is_one() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("a", "verb"));
        dag.add_node(ExecNode::new("b", "verb"));
        dag.add_edge("a", "out", "b", "in");
        assert_eq!(
            dag.edges[0].confidence, 1.0,
            "unweighted add_edge must set confidence to 1.0"
        );
    }

    #[test]
    fn dag_topology_respects_edge_confidence_prune() {
        // add_edge_weighted with confidence 0.0 stores a zero-confidence edge.
        // Callers filtering edges by minimum confidence (e.g. > 0.0) will skip it.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("x", "verb"));
        dag.add_node(ExecNode::new("y", "verb"));
        dag.add_edge_weighted("x", "out", "y", "in", 0.0);
        assert_eq!(dag.edge_count(), 1);
        assert_eq!(dag.edges[0].confidence, 0.0);
        // Filter by confidence > 0.0 — simulates BFS pruning low-confidence edges.
        let active_edges: Vec<_> = dag.edges.iter().filter(|e| e.confidence > 0.0).collect();
        assert!(
            active_edges.is_empty(),
            "zero-confidence edge must be pruned by confidence filter"
        );
    }

    #[test]
    fn dag_edge_confidence_default_one() {
        // add_edge (unweighted) must store confidence exactly 1.0.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("u", "verb"));
        dag.add_node(ExecNode::new("v", "verb"));
        dag.add_edge("u", "out", "v", "in");
        assert_eq!(
            dag.edges[0].confidence, 1.0,
            "default add_edge must produce confidence=1.0"
        );
    }

    #[test]
    fn dag_add_edge_weighted_clamps_above_one() {
        // Confidence values above 1.0 must be clamped to 1.0.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("a", "verb"));
        dag.add_node(ExecNode::new("b", "verb"));
        dag.add_edge_weighted("a", "out", "b", "in", 1.5);
        assert_eq!(
            dag.edges[0].confidence, 1.0,
            "confidence 1.5 must clamp to 1.0"
        );
    }

    #[test]
    fn dag_add_edge_weighted_clamps_below_zero() {
        // Confidence values below 0.0 must be clamped to 0.0.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("p", "verb"));
        dag.add_node(ExecNode::new("q", "verb"));
        dag.add_edge_weighted("p", "out", "q", "in", -0.1);
        assert_eq!(
            dag.edges[0].confidence, 0.0,
            "confidence -0.1 must clamp to 0.0"
        );
    }

    #[test]
    fn dag_edge_confidence_zero_point_five() {
        // add_edge_weighted with exactly 0.5 must store 0.5 (no clamping needed).
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("m", "verb"));
        dag.add_node(ExecNode::new("n", "verb"));
        dag.add_edge_weighted("m", "out", "n", "in", 0.5);
        assert_eq!(
            dag.edges[0].confidence, 0.5,
            "confidence 0.5 must be stored as-is"
        );
    }

    #[test]
    fn dag_topological_sort_simple() {
        // A → B → C must sort to [A, B, C].
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("A", "verb"));
        dag.add_node(ExecNode::new("B", "verb"));
        dag.add_node(ExecNode::new("C", "verb"));
        dag.add_edge("A", "out", "B", "in");
        dag.add_edge("B", "out", "C", "in");
        let sorted = dag.topological_sort().unwrap();
        let pos = |id: &str| sorted.iter().position(|x| x == id).unwrap();
        assert!(pos("A") < pos("B"), "A must precede B");
        assert!(pos("B") < pos("C"), "B must precede C");
    }

    #[test]
    fn dag_cycle_detection() {
        // A → B and B → A creates a cycle; topological_sort must return Err.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("A", "verb"));
        dag.add_node(ExecNode::new("B", "verb"));
        dag.add_edge("A", "out", "B", "in");
        dag.add_edge("B", "out", "A", "in");
        assert!(
            dag.topological_sort().is_err(),
            "cycle A→B→A must be detected"
        );
    }

    #[test]
    fn dag_node_count() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("n1", "verb"));
        dag.add_node(ExecNode::new("n2", "verb"));
        dag.add_node(ExecNode::new("n3", "verb"));
        assert_eq!(dag.node_count(), 3);
    }

    #[test]
    fn dag_edge_count() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("x", "verb"));
        dag.add_node(ExecNode::new("y", "verb"));
        dag.add_node(ExecNode::new("z", "verb"));
        dag.add_edge("x", "out", "y", "in");
        dag.add_edge("y", "out", "z", "in");
        assert_eq!(dag.edge_count(), 2);
    }

    #[test]
    fn dag_remove_node_also_removes_edges() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("src", "verb"));
        dag.add_node(ExecNode::new("mid", "verb"));
        dag.add_node(ExecNode::new("dst", "verb"));
        dag.add_edge("src", "out", "mid", "in");
        dag.add_edge("mid", "out", "dst", "in");
        assert_eq!(dag.node_count(), 3);
        assert_eq!(dag.edge_count(), 2);
        // Remove "mid" and all its incident edges.
        dag.nodes.remove("mid");
        dag.edges
            .retain(|e| e.src_node != "mid" && e.dst_node != "mid");
        assert_eq!(dag.node_count(), 2);
        assert_eq!(dag.edge_count(), 0);
    }

    #[test]
    fn dag_ancestors_of_leaf() {
        // Chain: root → parent → leaf. Ancestors of "leaf" (nodes that can reach it)
        // are root and parent; verified by checking topological positions.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("root", "verb"));
        dag.add_node(ExecNode::new("parent", "verb"));
        dag.add_node(ExecNode::new("leaf", "verb"));
        dag.add_edge("root", "out", "parent", "in");
        dag.add_edge("parent", "out", "leaf", "in");
        let sorted = dag.topological_sort().unwrap();
        let pos = |id: &str| sorted.iter().position(|x| x == id).unwrap();
        // root and parent both precede leaf in topological order.
        assert!(pos("root") < pos("leaf"), "root must precede leaf");
        assert!(pos("parent") < pos("leaf"), "parent must precede leaf");
    }

    #[test]
    fn dag_descendants_of_root() {
        // Star: root → a, root → b, root → c. All three must follow root in sort.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("root", "verb"));
        dag.add_node(ExecNode::new("a", "verb"));
        dag.add_node(ExecNode::new("b", "verb"));
        dag.add_node(ExecNode::new("c", "verb"));
        dag.add_edge("root", "out", "a", "in");
        dag.add_edge("root", "out", "b", "in");
        dag.add_edge("root", "out", "c", "in");
        let sorted = dag.topological_sort().unwrap();
        let root_pos = sorted.iter().position(|x| x == "root").unwrap();
        for child in &["a", "b", "c"] {
            let child_pos = sorted.iter().position(|x| x == *child).unwrap();
            assert!(root_pos < child_pos, "root must precede {child}");
        }
    }

    #[test]
    fn dag_empty_topological_sort() {
        // An empty DAG produces an empty sorted list.
        let dag = Dag::new();
        let sorted = dag.topological_sort().unwrap();
        assert!(sorted.is_empty(), "empty DAG must sort to []");
    }

    #[test]
    fn dag_linear_chain_topological() {
        // A→B→C→D must sort to [A, B, C, D] in that relative order.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("A", "verb"));
        dag.add_node(ExecNode::new("B", "verb"));
        dag.add_node(ExecNode::new("C", "verb"));
        dag.add_node(ExecNode::new("D", "verb"));
        dag.add_edge("A", "out", "B", "in");
        dag.add_edge("B", "out", "C", "in");
        dag.add_edge("C", "out", "D", "in");
        let sorted = dag.topological_sort().unwrap();
        assert_eq!(sorted.len(), 4);
        let pos = |id: &str| sorted.iter().position(|x| x == id).unwrap();
        assert!(pos("A") < pos("B"));
        assert!(pos("B") < pos("C"));
        assert!(pos("C") < pos("D"));
    }

    #[test]
    fn dag_diamond_topology() {
        // A→B, A→C, B→D, C→D — all 4 nodes must appear and A before D.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("A", "verb"));
        dag.add_node(ExecNode::new("B", "verb"));
        dag.add_node(ExecNode::new("C", "verb"));
        dag.add_node(ExecNode::new("D", "verb"));
        dag.add_edge("A", "out", "B", "in");
        dag.add_edge("A", "out", "C", "in");
        dag.add_edge("B", "out", "D", "in");
        dag.add_edge("C", "out", "D", "in");
        let sorted = dag.topological_sort().unwrap();
        assert_eq!(sorted.len(), 4, "diamond must include all 4 nodes");
        let pos = |id: &str| sorted.iter().position(|x| x == id).unwrap();
        assert!(pos("A") < pos("B"), "A before B");
        assert!(pos("A") < pos("C"), "A before C");
        assert!(pos("B") < pos("D"), "B before D");
        assert!(pos("C") < pos("D"), "C before D");
    }

    #[test]
    fn dag_self_loop_detection() {
        // A→A is a cycle; topological_sort must return Err.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("A", "verb"));
        dag.add_edge("A", "out", "A", "in");
        assert!(
            dag.topological_sort().is_err(),
            "self-loop A→A must be detected as cycle"
        );
    }

    #[test]
    fn dag_multiple_roots() {
        // Two independent nodes with no edges — both must appear in sort.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("A", "verb"));
        dag.add_node(ExecNode::new("B", "verb"));
        let sorted = dag.topological_sort().unwrap();
        assert_eq!(sorted.len(), 2, "both independent nodes must appear");
        assert!(sorted.contains(&"A".to_string()), "A must be in sort");
        assert!(sorted.contains(&"B".to_string()), "B must be in sort");
    }

    #[test]
    fn dag_edge_list() {
        // edges() field returns (src_node, dst_node) pairs as stored.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("src", "verb"));
        dag.add_node(ExecNode::new("dst", "verb"));
        dag.add_edge("src", "out", "dst", "in");
        assert_eq!(dag.edges.len(), 1);
        assert_eq!(dag.edges[0].src_node, "src");
        assert_eq!(dag.edges[0].dst_node, "dst");
    }

    #[test]
    fn dag_in_degree() {
        // B has two incoming edges → in-degree == 2.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("A", "verb"));
        dag.add_node(ExecNode::new("B", "verb"));
        dag.add_node(ExecNode::new("C", "verb"));
        dag.add_edge("A", "out", "B", "in");
        dag.add_edge("C", "out", "B", "in");
        let in_degree_b = dag.edges.iter().filter(|e| e.dst_node == "B").count();
        assert_eq!(in_degree_b, 2, "B must have in-degree 2");
    }

    #[test]
    fn dag_out_degree() {
        // A has two outgoing edges → out-degree == 2.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("A", "verb"));
        dag.add_node(ExecNode::new("B", "verb"));
        dag.add_node(ExecNode::new("C", "verb"));
        dag.add_edge("A", "out", "B", "in");
        dag.add_edge("A", "out", "C", "in");
        let out_degree_a = dag.edges.iter().filter(|e| e.src_node == "A").count();
        assert_eq!(out_degree_a, 2, "A must have out-degree 2");
    }

    #[test]
    fn dag_is_dag_true() {
        // A valid acyclic graph: topological_sort returns Ok.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("X", "verb"));
        dag.add_node(ExecNode::new("Y", "verb"));
        dag.add_edge("X", "out", "Y", "in");
        assert!(
            dag.topological_sort().is_ok(),
            "valid DAG must return Ok from topological_sort"
        );
    }

    #[test]
    fn dag_is_dag_false_on_cycle() {
        // A→B→C→A forms a cycle; topological_sort must return Err.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("A", "verb"));
        dag.add_node(ExecNode::new("B", "verb"));
        dag.add_node(ExecNode::new("C", "verb"));
        dag.add_edge("A", "out", "B", "in");
        dag.add_edge("B", "out", "C", "in");
        dag.add_edge("C", "out", "A", "in");
        assert!(
            dag.topological_sort().is_err(),
            "A→B→C→A cycle must cause topological_sort to return Err"
        );
    }

    // ------------------------------------------------------------------
    // Graph construction: single-node graph
    // ------------------------------------------------------------------
    #[test]
    fn dag_single_node_no_edges() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("solo", "verb"));
        assert_eq!(dag.node_count(), 1);
        assert_eq!(dag.edge_count(), 0);
        let sorted = dag.topological_sort().unwrap();
        assert_eq!(sorted, vec!["solo"]);
    }

    // ------------------------------------------------------------------
    // Graph construction: empty graph operations
    // ------------------------------------------------------------------
    #[test]
    fn dag_empty_node_count_zero() {
        let dag = Dag::new();
        assert_eq!(dag.node_count(), 0);
        assert_eq!(dag.edge_count(), 0);
    }

    #[test]
    fn dag_default_creates_empty_dag() {
        let dag = Dag::default();
        assert_eq!(dag.node_count(), 0);
        assert_eq!(dag.edge_count(), 0);
    }

    // ------------------------------------------------------------------
    // Graph construction: directed edges (src → dst, not dst → src)
    // ------------------------------------------------------------------
    #[test]
    fn dag_edge_is_directed() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("src", "verb"));
        dag.add_node(ExecNode::new("dst", "verb"));
        dag.add_edge("src", "out", "dst", "in");
        // Only one edge stored, and it points src→dst, not dst→src.
        assert_eq!(dag.edge_count(), 1);
        let e = &dag.edges[0];
        assert_eq!(e.src_node, "src");
        assert_eq!(e.dst_node, "dst");
        // No edge from dst→src.
        let reverse = dag
            .edges
            .iter()
            .any(|e| e.src_node == "dst" && e.dst_node == "src");
        assert!(!reverse, "directed DAG must not have implicit reverse edge");
    }

    // ------------------------------------------------------------------
    // Graph construction: multiple edges between same pair of nodes
    // ------------------------------------------------------------------
    #[test]
    fn dag_parallel_edges_same_pair() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("u", "verb"));
        dag.add_node(ExecNode::new("v", "verb"));
        dag.add_edge("u", "out1", "v", "in1");
        dag.add_edge("u", "out2", "v", "in2");
        assert_eq!(
            dag.edge_count(),
            2,
            "two parallel edges must both be stored"
        );
    }

    // ------------------------------------------------------------------
    // BFS traversal: reachability from a source node
    // ------------------------------------------------------------------
    #[test]
    fn dag_bfs_reachable_nodes() {
        // Graph: root → a, root → b, a → c
        let mut dag = Dag::new();
        for name in &["root", "a", "b", "c", "isolated"] {
            dag.add_node(ExecNode::new(*name, "verb"));
        }
        dag.add_edge("root", "out", "a", "in");
        dag.add_edge("root", "out", "b", "in");
        dag.add_edge("a", "out", "c", "in");

        // BFS from root using a manual walk over edges (DAG has directed edges).
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back("root".to_string());
        visited.insert("root".to_string());
        while let Some(current) = queue.pop_front() {
            for edge in &dag.edges {
                if edge.src_node == current && !visited.contains(&edge.dst_node) {
                    visited.insert(edge.dst_node.clone());
                    queue.push_back(edge.dst_node.clone());
                }
            }
        }
        assert!(visited.contains("a"), "BFS must reach a");
        assert!(visited.contains("b"), "BFS must reach b");
        assert!(visited.contains("c"), "BFS must reach c");
        assert!(
            !visited.contains("isolated"),
            "BFS must not reach isolated node"
        );
    }

    // ------------------------------------------------------------------
    // DFS traversal: post-order from a source node
    // ------------------------------------------------------------------
    #[test]
    fn dag_dfs_visits_all_reachable() {
        // Chain: start → mid → end
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("start", "verb"));
        dag.add_node(ExecNode::new("mid", "verb"));
        dag.add_node(ExecNode::new("end", "verb"));
        dag.add_edge("start", "out", "mid", "in");
        dag.add_edge("mid", "out", "end", "in");

        // Iterative DFS.
        let mut visited: Vec<String> = Vec::new();
        let mut stack = vec!["start".to_string()];
        let mut seen = std::collections::HashSet::new();
        while let Some(node) = stack.pop() {
            if seen.insert(node.clone()) {
                visited.push(node.clone());
                for edge in dag.edges.iter().rev() {
                    if edge.src_node == node && !seen.contains(&edge.dst_node) {
                        stack.push(edge.dst_node.clone());
                    }
                }
            }
        }
        assert!(visited.contains(&"start".to_string()));
        assert!(visited.contains(&"mid".to_string()));
        assert!(visited.contains(&"end".to_string()));
        assert_eq!(visited.len(), 3);
    }

    // ------------------------------------------------------------------
    // Subgraph extraction
    // ------------------------------------------------------------------
    #[test]
    fn dag_subgraph_extraction() {
        // Full graph: a → b → c → d; extract subgraph containing only {b, c}.
        let mut dag = Dag::new();
        for name in &["a", "b", "c", "d"] {
            dag.add_node(ExecNode::new(*name, "verb"));
        }
        dag.add_edge("a", "out", "b", "in");
        dag.add_edge("b", "out", "c", "in");
        dag.add_edge("c", "out", "d", "in");

        let keep: std::collections::HashSet<&str> = ["b", "c"].iter().copied().collect();
        let mut sub = Dag::new();
        for (id, node) in &dag.nodes {
            if keep.contains(id.as_str()) {
                sub.add_node(node.clone());
            }
        }
        for edge in &dag.edges {
            if keep.contains(edge.src_node.as_str()) && keep.contains(edge.dst_node.as_str()) {
                sub.add_edge(
                    edge.src_node.clone(),
                    edge.src_port.clone(),
                    edge.dst_node.clone(),
                    edge.dst_port.clone(),
                );
            }
        }

        assert_eq!(
            sub.node_count(),
            2,
            "subgraph must contain exactly {{b, c}}"
        );
        assert_eq!(sub.edge_count(), 1, "subgraph must contain only b→c edge");
        assert!(sub.nodes.contains_key("b"), "subgraph must contain b");
        assert!(sub.nodes.contains_key("c"), "subgraph must contain c");
        assert!(!sub.nodes.contains_key("a"), "subgraph must not contain a");
        assert!(!sub.nodes.contains_key("d"), "subgraph must not contain d");
    }

    // ------------------------------------------------------------------
    // Edge weight operations
    // ------------------------------------------------------------------
    #[test]
    fn dag_edge_weight_midpoint_stored_correctly() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("x", "verb"));
        dag.add_node(ExecNode::new("y", "verb"));
        dag.add_edge_weighted("x", "out", "y", "in", 0.75);
        assert!((dag.edges[0].confidence - 0.75).abs() < 1e-6);
    }

    #[test]
    fn dag_edge_weight_zero_is_valid() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("a", "verb"));
        dag.add_node(ExecNode::new("b", "verb"));
        dag.add_edge_weighted("a", "out", "b", "in", 0.0);
        assert_eq!(dag.edges[0].confidence, 0.0);
    }

    #[test]
    fn dag_edge_weight_one_is_valid() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("a", "verb"));
        dag.add_node(ExecNode::new("b", "verb"));
        dag.add_edge_weighted("a", "out", "b", "in", 1.0);
        assert_eq!(dag.edges[0].confidence, 1.0);
    }

    #[test]
    fn dag_filter_edges_by_weight_threshold() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("a", "verb"));
        dag.add_node(ExecNode::new("b", "verb"));
        dag.add_node(ExecNode::new("c", "verb"));
        dag.add_edge_weighted("a", "out", "b", "in", 0.9);
        dag.add_edge_weighted("a", "out", "c", "in", 0.1);
        // Only edges above 0.5 threshold.
        let high: Vec<_> = dag.edges.iter().filter(|e| e.confidence > 0.5).collect();
        assert_eq!(high.len(), 1);
        assert_eq!(high[0].dst_node, "b");
    }

    // ------------------------------------------------------------------
    // Node removal and dangling edge cleanup
    // ------------------------------------------------------------------
    #[test]
    fn dag_remove_middle_node_removes_both_incident_edges() {
        let mut dag = Dag::new();
        for name in &["a", "b", "c"] {
            dag.add_node(ExecNode::new(*name, "verb"));
        }
        dag.add_edge("a", "out", "b", "in");
        dag.add_edge("b", "out", "c", "in");
        assert_eq!(dag.edge_count(), 2);

        dag.nodes.remove("b");
        dag.edges.retain(|e| e.src_node != "b" && e.dst_node != "b");

        assert_eq!(dag.node_count(), 2);
        assert_eq!(
            dag.edge_count(),
            0,
            "both edges referencing b must be removed"
        );
    }

    #[test]
    fn dag_remove_leaf_node_removes_incoming_edge() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("root", "verb"));
        dag.add_node(ExecNode::new("leaf", "verb"));
        dag.add_edge("root", "out", "leaf", "in");

        dag.nodes.remove("leaf");
        dag.edges
            .retain(|e| e.src_node != "leaf" && e.dst_node != "leaf");

        assert_eq!(dag.node_count(), 1);
        assert_eq!(dag.edge_count(), 0);
    }

    #[test]
    fn dag_remove_nonexistent_node_is_noop() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("a", "verb"));
        let removed = dag.nodes.remove("does_not_exist");
        assert!(removed.is_none());
        assert_eq!(dag.node_count(), 1);
    }

    // ------------------------------------------------------------------
    // Graph merge / union
    // ------------------------------------------------------------------
    #[test]
    fn dag_merge_two_dags_combines_nodes_and_edges() {
        let mut dag1 = Dag::new();
        dag1.add_node(ExecNode::new("a", "verb"));
        dag1.add_node(ExecNode::new("b", "verb"));
        dag1.add_edge("a", "out", "b", "in");

        let mut dag2 = Dag::new();
        dag2.add_node(ExecNode::new("c", "verb"));
        dag2.add_node(ExecNode::new("d", "verb"));
        dag2.add_edge("c", "out", "d", "in");

        // Merge dag2 into dag1.
        for (id, node) in dag2.nodes {
            dag1.nodes.insert(id, node);
        }
        for edge in dag2.edges {
            dag1.edges.push(edge);
        }

        assert_eq!(dag1.node_count(), 4, "merged dag must have 4 nodes");
        assert_eq!(dag1.edge_count(), 2, "merged dag must have 2 edges");
        assert!(dag1.nodes.contains_key("a"));
        assert!(dag1.nodes.contains_key("c"));
    }

    #[test]
    fn dag_merge_into_empty_dag() {
        let mut empty = Dag::new();
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("x", "verb"));
        dag.add_edge("x", "out", "x", "in"); // self-loop

        for (id, node) in dag.nodes {
            empty.nodes.insert(id, node);
        }
        for edge in dag.edges {
            empty.edges.push(edge);
        }

        assert_eq!(empty.node_count(), 1);
        assert_eq!(empty.edge_count(), 1);
    }

    // ------------------------------------------------------------------
    // Serialization roundtrip (manual field-by-field check, no serde dep)
    // ------------------------------------------------------------------
    #[test]
    fn dag_serialization_roundtrip_via_fields() {
        // Construct a dag, capture its structural data, reconstruct, and verify equality.
        let mut original = Dag::new();
        original.add_node(ExecNode::new("p", "verb"));
        original.add_node(ExecNode::new("q", "verb"));
        original.add_edge_weighted("p", "out", "q", "in", 0.8);

        // Capture data.
        let node_ids: Vec<String> = {
            let mut ids: Vec<_> = original.nodes.keys().cloned().collect();
            ids.sort();
            ids
        };
        let edge_data: Vec<(String, String, String, String, f32)> = original
            .edges
            .iter()
            .map(|e| {
                (
                    e.src_node.clone(),
                    e.src_port.clone(),
                    e.dst_node.clone(),
                    e.dst_port.clone(),
                    e.confidence,
                )
            })
            .collect();

        // Reconstruct.
        let mut reconstructed = Dag::new();
        for id in &node_ids {
            reconstructed.add_node(ExecNode::new(id.clone(), "verb"));
        }
        for (src, sp, dst, dp, conf) in &edge_data {
            reconstructed.add_edge_weighted(
                src.clone(),
                sp.clone(),
                dst.clone(),
                dp.clone(),
                *conf,
            );
        }

        // Verify.
        assert_eq!(reconstructed.node_count(), original.node_count());
        assert_eq!(reconstructed.edge_count(), original.edge_count());
        let rec_ids: Vec<String> = {
            let mut ids: Vec<_> = reconstructed.nodes.keys().cloned().collect();
            ids.sort();
            ids
        };
        assert_eq!(rec_ids, node_ids);
        assert!((reconstructed.edges[0].confidence - 0.8).abs() < 1e-6);
    }

    // ------------------------------------------------------------------
    // Dense vs sparse graphs
    // ------------------------------------------------------------------
    #[test]
    fn dag_sparse_graph_few_edges() {
        // 10 nodes, only 2 edges → sparse
        let mut dag = Dag::new();
        for i in 0..10u32 {
            dag.add_node(ExecNode::new(format!("n{i}"), "verb"));
        }
        dag.add_edge("n0", "out", "n1", "in");
        dag.add_edge("n5", "out", "n6", "in");
        assert_eq!(dag.node_count(), 10);
        assert_eq!(dag.edge_count(), 2);
        // Each isolated node has no edges.
        let isolated_count = (0..10u32)
            .filter(|&i| {
                let name = format!("n{i}");
                !dag.edges
                    .iter()
                    .any(|e| e.src_node == name || e.dst_node == name)
            })
            .count();
        assert_eq!(
            isolated_count, 6,
            "6 nodes should have no edges in sparse graph"
        );
    }

    #[test]
    fn dag_dense_graph_many_edges() {
        // 4-node complete DAG (all forward edges in topological order): 6 edges
        let mut dag = Dag::new();
        let nodes = ["a", "b", "c", "d"];
        for name in &nodes {
            dag.add_node(ExecNode::new(*name, "verb"));
        }
        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                dag.add_edge(nodes[i], "out", nodes[j], "in");
            }
        }
        assert_eq!(dag.node_count(), 4);
        assert_eq!(
            dag.edge_count(),
            6,
            "complete 4-node DAG should have 6 edges"
        );
        // Topological sort should still succeed (all forward edges, no cycle).
        assert!(
            dag.topological_sort().is_ok(),
            "dense DAG with all forward edges must be acyclic"
        );
    }

    // ------------------------------------------------------------------
    // Graph clone/copy correctness
    // ------------------------------------------------------------------
    #[test]
    fn dag_clone_is_independent() {
        // Clone a node and verify modifying the clone doesn't affect the original.
        let original = ExecNode::new("node1", "kind_a");
        let mut cloned = original.clone();
        cloned.kind = "kind_b".to_string();
        assert_eq!(
            original.kind, "kind_a",
            "original kind must be unchanged after clone mutation"
        );
        assert_eq!(cloned.kind, "kind_b");
    }

    #[test]
    fn dag_node_ids_are_unique_in_dag() {
        // Adding a node with the same id twice overwrites the first (HashMap semantics).
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("dup", "verb"));
        dag.add_node(ExecNode::new("dup", "noun")); // same id, different kind
                                                    // HashMap insert overwrites; node_count stays at 1.
        assert_eq!(
            dag.node_count(),
            1,
            "duplicate node id must overwrite, not add"
        );
        assert_eq!(dag.nodes["dup"].kind, "noun", "second insert must win");
    }

    // ------------------------------------------------------------------
    // Topological sort: cycle returns error with the cycle nodes
    // ------------------------------------------------------------------
    #[test]
    fn dag_cycle_error_contains_cycle_nodes() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("X", "verb"));
        dag.add_node(ExecNode::new("Y", "verb"));
        dag.add_edge("X", "out", "Y", "in");
        dag.add_edge("Y", "out", "X", "in");
        match dag.topological_sort() {
            Err(cycle_nodes) => {
                assert!(
                    cycle_nodes.contains(&"X".to_string()),
                    "X must be in cycle nodes"
                );
                assert!(
                    cycle_nodes.contains(&"Y".to_string()),
                    "Y must be in cycle nodes"
                );
            }
            Ok(_) => panic!("expected Err from cycle X→Y→X"),
        }
    }

    // ------------------------------------------------------------------
    // Ports: edge carries src_port and dst_port
    // ------------------------------------------------------------------
    #[test]
    fn dag_edge_stores_port_names() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("producer", "verb"));
        dag.add_node(ExecNode::new("consumer", "verb"));
        dag.add_edge("producer", "result_port", "consumer", "input_port");
        let e = &dag.edges[0];
        assert_eq!(e.src_port, "result_port");
        assert_eq!(e.dst_port, "input_port");
    }

    // ------------------------------------------------------------------
    // Multi-root DAG: two independent source nodes both appear before sinks
    // ------------------------------------------------------------------
    #[test]
    fn dag_two_roots_both_precede_shared_sink() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("root1", "verb"));
        dag.add_node(ExecNode::new("root2", "verb"));
        dag.add_node(ExecNode::new("sink", "verb"));
        dag.add_edge("root1", "out", "sink", "in");
        dag.add_edge("root2", "out", "sink", "in");
        let sorted = dag.topological_sort().unwrap();
        assert_eq!(sorted.len(), 3);
        let pos = |id: &str| sorted.iter().position(|x| x == id).unwrap();
        assert!(pos("root1") < pos("sink"), "root1 must precede sink");
        assert!(pos("root2") < pos("sink"), "root2 must precede sink");
    }

    #[test]
    fn dag_three_roots_no_edges() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("r1", "verb"));
        dag.add_node(ExecNode::new("r2", "verb"));
        dag.add_node(ExecNode::new("r3", "verb"));
        let sorted = dag.topological_sort().unwrap();
        assert_eq!(sorted.len(), 3, "all three independent roots must appear");
        for r in &["r1", "r2", "r3"] {
            assert!(sorted.contains(&r.to_string()), "{r} must be in sorted list");
        }
    }

    // ------------------------------------------------------------------
    // Kahn edge cases: edge to non-existent node still counts
    // ------------------------------------------------------------------
    #[test]
    fn dag_edge_weight_exactly_half() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("x", "verb"));
        dag.add_node(ExecNode::new("y", "verb"));
        dag.add_edge_weighted("x", "out", "y", "in", 0.5);
        assert!((dag.edges[0].confidence - 0.5).abs() < 1e-7);
    }

    #[test]
    fn dag_kahn_single_chain_five_nodes() {
        // a→b→c→d→e must sort in exactly that order.
        let mut dag = Dag::new();
        for name in &["a", "b", "c", "d", "e"] {
            dag.add_node(ExecNode::new(*name, "verb"));
        }
        dag.add_edge("a", "out", "b", "in");
        dag.add_edge("b", "out", "c", "in");
        dag.add_edge("c", "out", "d", "in");
        dag.add_edge("d", "out", "e", "in");
        let sorted = dag.topological_sort().unwrap();
        assert_eq!(sorted.len(), 5);
        let pos = |id: &str| sorted.iter().position(|x| x == id).unwrap();
        assert!(pos("a") < pos("b"));
        assert!(pos("b") < pos("c"));
        assert!(pos("c") < pos("d"));
        assert!(pos("d") < pos("e"));
    }

    #[test]
    fn dag_kahn_three_node_cycle_detected() {
        // X→Y→Z→X is a three-cycle.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("X", "verb"));
        dag.add_node(ExecNode::new("Y", "verb"));
        dag.add_node(ExecNode::new("Z", "verb"));
        dag.add_edge("X", "out", "Y", "in");
        dag.add_edge("Y", "out", "Z", "in");
        dag.add_edge("Z", "out", "X", "in");
        assert!(dag.topological_sort().is_err(), "X→Y→Z→X must be detected as cycle");
    }

    #[test]
    fn dag_kahn_returns_all_cycle_nodes_in_error() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("P", "verb"));
        dag.add_node(ExecNode::new("Q", "verb"));
        dag.add_node(ExecNode::new("R", "verb"));
        dag.add_edge("P", "out", "Q", "in");
        dag.add_edge("Q", "out", "R", "in");
        dag.add_edge("R", "out", "P", "in");
        let err = dag.topological_sort().unwrap_err();
        assert!(err.contains(&"P".to_string()), "P must be in cycle error");
        assert!(err.contains(&"Q".to_string()), "Q must be in cycle error");
        assert!(err.contains(&"R".to_string()), "R must be in cycle error");
    }

    // ------------------------------------------------------------------
    // Kahn: node with no neighbors sorts correctly
    // ------------------------------------------------------------------
    #[test]
    fn dag_kahn_isolated_plus_chain() {
        // Chain a→b plus isolated node c — c can appear anywhere, a before b.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("a", "verb"));
        dag.add_node(ExecNode::new("b", "verb"));
        dag.add_node(ExecNode::new("c", "verb")); // isolated
        dag.add_edge("a", "out", "b", "in");
        let sorted = dag.topological_sort().unwrap();
        assert_eq!(sorted.len(), 3);
        let pos = |id: &str| sorted.iter().position(|x| x == id).unwrap();
        assert!(pos("a") < pos("b"), "a must precede b");
        assert!(sorted.contains(&"c".to_string()), "c must appear in sort");
    }

    #[test]
    fn dag_kahn_fan_in_two_to_one() {
        // Two parents both feed one child.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("p1", "verb"));
        dag.add_node(ExecNode::new("p2", "verb"));
        dag.add_node(ExecNode::new("child", "verb"));
        dag.add_edge("p1", "out", "child", "in");
        dag.add_edge("p2", "out", "child", "in");
        let sorted = dag.topological_sort().unwrap();
        let pos = |id: &str| sorted.iter().position(|x| x == id).unwrap();
        assert!(pos("p1") < pos("child"), "p1 must precede child");
        assert!(pos("p2") < pos("child"), "p2 must precede child");
    }

    #[test]
    fn dag_kahn_fan_out_one_to_three() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("src", "verb"));
        for c in ["c1", "c2", "c3"] {
            dag.add_node(ExecNode::new(c, "verb"));
            dag.add_edge("src", "out", c, "in");
        }
        let sorted = dag.topological_sort().unwrap();
        let src_pos = sorted.iter().position(|x| x == "src").unwrap();
        for c in ["c1", "c2", "c3"] {
            let child_pos = sorted.iter().position(|x| x == c).unwrap();
            assert!(src_pos < child_pos, "src must precede {c}");
        }
    }

    // ------------------------------------------------------------------
    // Edge: port strings are stored as-is
    // ------------------------------------------------------------------
    #[test]
    fn dag_edge_port_strings_preserved() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("a", "verb"));
        dag.add_node(ExecNode::new("b", "verb"));
        dag.add_edge("a", "custom_src_port", "b", "custom_dst_port");
        let e = &dag.edges[0];
        assert_eq!(e.src_port, "custom_src_port");
        assert_eq!(e.dst_port, "custom_dst_port");
    }

    // ------------------------------------------------------------------
    // Weighted edge: confidence 0.25 stored correctly
    // ------------------------------------------------------------------
    #[test]
    fn dag_weighted_edge_confidence_quarter() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("u", "verb"));
        dag.add_node(ExecNode::new("v", "verb"));
        dag.add_edge_weighted("u", "out", "v", "in", 0.25);
        assert!((dag.edges[0].confidence - 0.25).abs() < 1e-6);
    }

    // ------------------------------------------------------------------
    // Add multiple edges in sequence: edge_count increments correctly
    // ------------------------------------------------------------------
    #[test]
    fn dag_edge_count_increments_on_each_add() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("a", "verb"));
        dag.add_node(ExecNode::new("b", "verb"));
        dag.add_node(ExecNode::new("c", "verb"));
        assert_eq!(dag.edge_count(), 0);
        dag.add_edge("a", "out", "b", "in");
        assert_eq!(dag.edge_count(), 1);
        dag.add_edge("b", "out", "c", "in");
        assert_eq!(dag.edge_count(), 2);
    }

    // ------------------------------------------------------------------
    // Two-node DAG: topological order is src before dst
    // ------------------------------------------------------------------
    #[test]
    fn dag_two_node_topological_order() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("first", "verb"));
        dag.add_node(ExecNode::new("second", "verb"));
        dag.add_edge("first", "out", "second", "in");
        let sorted = dag.topological_sort().unwrap();
        assert_eq!(sorted[0], "first");
        assert_eq!(sorted[1], "second");
    }

    // ------------------------------------------------------------------
    // Deep chain: 10-node linear chain sorts correctly
    // ------------------------------------------------------------------
    #[test]
    fn dag_deep_chain_ten_nodes() {
        let mut dag = Dag::new();
        let names: Vec<String> = (0..10).map(|i| format!("n{i}")).collect();
        for name in &names {
            dag.add_node(ExecNode::new(name.clone(), "verb"));
        }
        for i in 0..9 {
            dag.add_edge(names[i].clone(), "out", names[i + 1].clone(), "in");
        }
        let sorted = dag.topological_sort().unwrap();
        assert_eq!(sorted.len(), 10);
        for i in 0..9 {
            let pi = sorted.iter().position(|x| x == &names[i]).unwrap();
            let pj = sorted.iter().position(|x| x == &names[i + 1]).unwrap();
            assert!(pi < pj, "n{i} must precede n{}", i + 1);
        }
    }

    // ------------------------------------------------------------------
    // 100-node DAG still toposorts correctly
    // ------------------------------------------------------------------
    #[test]
    fn dag_100_node_linear_chain_topological_sort() {
        // Build a linear chain: n0 → n1 → ... → n99.
        let mut dag = Dag::new();
        let names: Vec<String> = (0..100).map(|i| format!("n{i:03}")).collect();
        for name in &names {
            dag.add_node(ExecNode::new(name.clone(), "verb"));
        }
        for i in 0..99 {
            dag.add_edge(names[i].clone(), "out", names[i + 1].clone(), "in");
        }
        let sorted = dag.topological_sort().unwrap();
        assert_eq!(sorted.len(), 100, "all 100 nodes must appear in sort");
        // Verify every consecutive pair is in the correct relative order.
        for i in 0..99 {
            let pi = sorted.iter().position(|x| x == &names[i]).unwrap();
            let pj = sorted.iter().position(|x| x == &names[i + 1]).unwrap();
            assert!(
                pi < pj,
                "n{i:03} must precede n{:03} in topological sort",
                i + 1
            );
        }
    }

    // ------------------------------------------------------------------
    // Edge weight accumulation across a path
    // ------------------------------------------------------------------
    #[test]
    fn dag_edge_weight_accumulation_three_hop_path() {
        // Chain: a →(0.9)→ b →(0.8)→ c →(0.7)→ d
        // Accumulated product: 0.9 * 0.8 * 0.7 = 0.504
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("a", "verb"));
        dag.add_node(ExecNode::new("b", "verb"));
        dag.add_node(ExecNode::new("c", "verb"));
        dag.add_node(ExecNode::new("d", "verb"));
        dag.add_edge_weighted("a", "out", "b", "in", 0.9);
        dag.add_edge_weighted("b", "out", "c", "in", 0.8);
        dag.add_edge_weighted("c", "out", "d", "in", 0.7);

        // Manually accumulate the product along the path a→b→c→d.
        let path_edges: &[(&str, &str)] = &[("a", "b"), ("b", "c"), ("c", "d")];
        let product: f32 = path_edges.iter().map(|(src, dst)| {
            dag.edges
                .iter()
                .find(|e| e.src_node == *src && e.dst_node == *dst)
                .expect("edge must exist")
                .confidence
        }).product();
        let expected = 0.9f32 * 0.8 * 0.7;
        assert!(
            (product - expected).abs() < 1e-5,
            "accumulated weight along a→b→c→d must be {}, got {}",
            expected,
            product
        );
    }

    // ------------------------------------------------------------------
    // Edge weight accumulation: single-hop path
    // ------------------------------------------------------------------
    #[test]
    fn dag_edge_weight_accumulation_single_hop() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("x", "verb"));
        dag.add_node(ExecNode::new("y", "verb"));
        dag.add_edge_weighted("x", "out", "y", "in", 0.6);
        let conf = dag.edges[0].confidence;
        assert!((conf - 0.6).abs() < 1e-6, "single-hop weight must be 0.6, got {}", conf);
    }

    // ------------------------------------------------------------------
    // Edge weight accumulation: two independent paths, highest wins
    // ------------------------------------------------------------------
    #[test]
    fn dag_edge_weight_two_paths_highest_selected() {
        // a→c with weight 0.3, a→b→c where a→b=0.9, b→c=0.8 → product=0.72 > 0.3
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("a", "verb"));
        dag.add_node(ExecNode::new("b", "verb"));
        dag.add_node(ExecNode::new("c", "verb"));
        dag.add_edge_weighted("a", "out", "c", "direct_in", 0.3);
        dag.add_edge_weighted("a", "out", "b", "in", 0.9);
        dag.add_edge_weighted("b", "out", "c", "in", 0.8);

        let direct_conf = dag.edges.iter()
            .find(|e| e.src_node == "a" && e.dst_node == "c")
            .unwrap().confidence;
        let via_b_conf: f32 = dag.edges.iter()
            .find(|e| e.src_node == "a" && e.dst_node == "b")
            .unwrap().confidence
            * dag.edges.iter()
            .find(|e| e.src_node == "b" && e.dst_node == "c")
            .unwrap().confidence;

        assert!((direct_conf - 0.3).abs() < 1e-6, "direct path confidence must be 0.3");
        assert!((via_b_conf - 0.72).abs() < 1e-5, "two-hop path confidence must be 0.72");
        assert!(via_b_conf > direct_conf, "two-hop path must have higher accumulated weight");
    }

    // ------------------------------------------------------------------
    // 100 nodes: toposort is O(N+E) — returns Ok within time budget
    // ------------------------------------------------------------------
    #[test]
    fn dag_100_node_no_edges_toposort() {
        // 100 isolated nodes: no edges — each node is a root, sort must include all.
        let mut dag = Dag::new();
        let names: Vec<String> = (0..100).map(|i| format!("iso{i:03}")).collect();
        for name in &names {
            dag.add_node(ExecNode::new(name.clone(), "verb"));
        }
        let sorted = dag.topological_sort().unwrap();
        assert_eq!(sorted.len(), 100, "all 100 isolated nodes must appear in sort");
        for name in &names {
            assert!(sorted.contains(name), "{name} must be in sort");
        }
    }

    // ------------------------------------------------------------------
    // Diamond DAG toposort valid: A→B, A→C, B→D, C→D
    // A must precede both B and C; both B and C must precede D.
    // ------------------------------------------------------------------
    #[test]
    fn dag_diamond_toposort_all_constraints_satisfied() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("A", "verb"));
        dag.add_node(ExecNode::new("B", "verb"));
        dag.add_node(ExecNode::new("C", "verb"));
        dag.add_node(ExecNode::new("D", "verb"));
        dag.add_edge("A", "out", "B", "in");
        dag.add_edge("A", "out", "C", "in");
        dag.add_edge("B", "out", "D", "in");
        dag.add_edge("C", "out", "D", "in");

        let sorted = dag.topological_sort().expect("diamond DAG must not have a cycle");
        assert_eq!(sorted.len(), 4, "all 4 nodes must appear in topological sort");

        let pos = |id: &str| sorted.iter().position(|x| x == id).expect(id);
        // A must appear before B, C, and D.
        assert!(pos("A") < pos("B"), "A must precede B");
        assert!(pos("A") < pos("C"), "A must precede C");
        assert!(pos("A") < pos("D"), "A must precede D");
        // B and C must appear before D.
        assert!(pos("B") < pos("D"), "B must precede D");
        assert!(pos("C") < pos("D"), "C must precede D");
        // D must be last.
        assert_eq!(pos("D"), 3, "D must be last in diamond toposort");
    }

    // ------------------------------------------------------------------
    // Parallel execution sets: independent nodes (same depth in diamond)
    // share no ordering constraint between them.
    // ------------------------------------------------------------------
    #[test]
    fn dag_diamond_parallel_nodes_no_ordering_constraint() {
        // B and C are both depth-1 in the diamond: neither depends on the other.
        // Topological sort may place them in any relative order — we just verify
        // both appear, that A is before both, and D is after both.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("A", "verb"));
        dag.add_node(ExecNode::new("B", "verb"));
        dag.add_node(ExecNode::new("C", "verb"));
        dag.add_node(ExecNode::new("D", "verb"));
        dag.add_edge("A", "out", "B", "in");
        dag.add_edge("A", "out", "C", "in");
        dag.add_edge("B", "out", "D", "in");
        dag.add_edge("C", "out", "D", "in");

        let sorted = dag.topological_sort().expect("diamond must sort without error");
        let pos = |id: &str| sorted.iter().position(|x| x == id).expect(id);

        let pos_a = pos("A");
        let pos_b = pos("B");
        let pos_c = pos("C");
        let pos_d = pos("D");

        // A before both parallel nodes; both parallel nodes before D.
        assert!(pos_a < pos_b, "A must precede B");
        assert!(pos_a < pos_c, "A must precede C");
        assert!(pos_b < pos_d, "B must precede D");
        assert!(pos_c < pos_d, "C must precede D");

        // B and C can be in either order — the parallel execution set {B, C}
        // has no internal ordering constraint. Both appear in positions 1 and 2.
        let parallel_positions: std::collections::HashSet<usize> = [pos_b, pos_c].into();
        assert_eq!(
            parallel_positions,
            [1, 2].into(),
            "B and C must occupy the two middle positions (parallel execution set)"
        );
    }

    // ------------------------------------------------------------------
    // Parallel root nodes: multiple sources, no mutual dependency
    // ------------------------------------------------------------------
    #[test]
    fn dag_parallel_roots_all_before_sink() {
        // Four independent roots all feed into one sink.
        let mut dag = Dag::new();
        for r in &["r1", "r2", "r3", "r4"] {
            dag.add_node(ExecNode::new(*r, "verb"));
        }
        dag.add_node(ExecNode::new("sink", "verb"));
        for r in &["r1", "r2", "r3", "r4"] {
            dag.add_edge(*r, "out", "sink", "in");
        }

        let sorted = dag.topological_sort().unwrap();
        assert_eq!(sorted.len(), 5, "5 nodes must all appear");
        let sink_pos = sorted.iter().position(|x| x == "sink").unwrap();
        for r in &["r1", "r2", "r3", "r4"] {
            let rp = sorted.iter().position(|x| x == *r).unwrap();
            assert!(rp < sink_pos, "{r} must precede sink in topological order");
        }
    }

    // ------------------------------------------------------------------
    // Diamond DAG: exactly 4 edges stored
    // ------------------------------------------------------------------
    #[test]
    fn dag_diamond_edge_count_four() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("A", "verb"));
        dag.add_node(ExecNode::new("B", "verb"));
        dag.add_node(ExecNode::new("C", "verb"));
        dag.add_node(ExecNode::new("D", "verb"));
        dag.add_edge("A", "out", "B", "in");
        dag.add_edge("A", "out", "C", "in");
        dag.add_edge("B", "out", "D", "in");
        dag.add_edge("C", "out", "D", "in");
        assert_eq!(dag.edge_count(), 4, "diamond DAG must have exactly 4 edges");
    }

    // ------------------------------------------------------------------
    // Diamond DAG: no cycle (topological_sort returns Ok)
    // ------------------------------------------------------------------
    #[test]
    fn dag_diamond_is_acyclic() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("A", "verb"));
        dag.add_node(ExecNode::new("B", "verb"));
        dag.add_node(ExecNode::new("C", "verb"));
        dag.add_node(ExecNode::new("D", "verb"));
        dag.add_edge("A", "out", "B", "in");
        dag.add_edge("A", "out", "C", "in");
        dag.add_edge("B", "out", "D", "in");
        dag.add_edge("C", "out", "D", "in");
        assert!(
            dag.topological_sort().is_ok(),
            "diamond DAG A→B, A→C, B→D, C→D must be acyclic"
        );
    }

    // ------------------------------------------------------------------
    // Diamond DAG: A appears first, D appears last
    // ------------------------------------------------------------------
    #[test]
    fn dag_diamond_a_first_d_last() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("A", "verb"));
        dag.add_node(ExecNode::new("B", "verb"));
        dag.add_node(ExecNode::new("C", "verb"));
        dag.add_node(ExecNode::new("D", "verb"));
        dag.add_edge("A", "out", "B", "in");
        dag.add_edge("A", "out", "C", "in");
        dag.add_edge("B", "out", "D", "in");
        dag.add_edge("C", "out", "D", "in");
        let sorted = dag.topological_sort().unwrap();
        assert_eq!(sorted.first().map(|s| s.as_str()), Some("A"), "A must be first");
        assert_eq!(sorted.last().map(|s| s.as_str()), Some("D"), "D must be last");
    }

    // ------------------------------------------------------------------
    // Parallel execution set size: B and C form a set of 2 independents
    // ------------------------------------------------------------------
    #[test]
    fn dag_diamond_parallel_set_size_two() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("A", "verb"));
        dag.add_node(ExecNode::new("B", "verb"));
        dag.add_node(ExecNode::new("C", "verb"));
        dag.add_node(ExecNode::new("D", "verb"));
        dag.add_edge("A", "out", "B", "in");
        dag.add_edge("A", "out", "C", "in");
        dag.add_edge("B", "out", "D", "in");
        dag.add_edge("C", "out", "D", "in");

        // B and C have no edge between them — they are independent (parallel set).
        let bc_edge = dag.edges.iter().any(|e| {
            (e.src_node == "B" && e.dst_node == "C")
                || (e.src_node == "C" && e.dst_node == "B")
        });
        assert!(!bc_edge, "B and C must have no direct edge (independent parallel nodes)");
    }

    // ------------------------------------------------------------------
    // Parallel execution: three-way fan-out from one source
    // ------------------------------------------------------------------
    #[test]
    fn dag_three_way_fan_out_parallel_execution() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("src", "verb"));
        dag.add_node(ExecNode::new("w1", "verb"));
        dag.add_node(ExecNode::new("w2", "verb"));
        dag.add_node(ExecNode::new("w3", "verb"));
        dag.add_edge("src", "out", "w1", "in");
        dag.add_edge("src", "out", "w2", "in");
        dag.add_edge("src", "out", "w3", "in");

        let sorted = dag.topological_sort().unwrap();
        let src_pos = sorted.iter().position(|x| x == "src").unwrap();
        // All three workers must follow src.
        for w in &["w1", "w2", "w3"] {
            let wp = sorted.iter().position(|x| x == *w).unwrap();
            assert!(src_pos < wp, "src must precede {w}");
        }
        // None of the workers depends on any other (no edges among them).
        for &a in &["w1", "w2", "w3"] {
            for &b in &["w1", "w2", "w3"] {
                if a != b {
                    assert!(
                        !dag.edges.iter().any(|e| e.src_node == a && e.dst_node == b),
                        "parallel workers {a} and {b} must have no dependency edge"
                    );
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Nested diamonds: two consecutive diamond structures
    // ------------------------------------------------------------------
    #[test]
    fn dag_nested_diamonds_toposort_valid() {
        // First diamond: A→B, A→C, B→D, C→D
        // Second diamond: D→E, D→F, E→G, F→G
        let mut dag = Dag::new();
        for name in &["A", "B", "C", "D", "E", "F", "G"] {
            dag.add_node(ExecNode::new(*name, "verb"));
        }
        dag.add_edge("A", "out", "B", "in");
        dag.add_edge("A", "out", "C", "in");
        dag.add_edge("B", "out", "D", "in");
        dag.add_edge("C", "out", "D", "in");
        dag.add_edge("D", "out", "E", "in");
        dag.add_edge("D", "out", "F", "in");
        dag.add_edge("E", "out", "G", "in");
        dag.add_edge("F", "out", "G", "in");

        let sorted = dag.topological_sort().expect("nested diamonds must sort");
        assert_eq!(sorted.len(), 7, "all 7 nodes must appear");
        let pos = |id: &str| sorted.iter().position(|x| x == id).unwrap();
        assert!(pos("A") < pos("B") && pos("A") < pos("C"), "A before B and C");
        assert!(pos("B") < pos("D") && pos("C") < pos("D"), "B,C before D");
        assert!(pos("D") < pos("E") && pos("D") < pos("F"), "D before E and F");
        assert!(pos("E") < pos("G") && pos("F") < pos("G"), "E,F before G");
        assert_eq!(pos("G"), 6, "G must be last");
    }

    // ------------------------------------------------------------------
    // add_node overwrites existing node — node_count stays the same
    // ------------------------------------------------------------------
    #[test]
    fn dag_add_node_overwrite_stable_count() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("key", "kind_a"));
        dag.add_node(ExecNode::new("key", "kind_b")); // overwrites
        assert_eq!(dag.node_count(), 1, "overwrite must not increase node count");
        assert_eq!(dag.nodes["key"].kind, "kind_b", "second insert must win");
    }

    // ------------------------------------------------------------------
    // Diamond DAG: node_count is exactly 4
    // ------------------------------------------------------------------
    #[test]
    fn dag_diamond_node_count_four() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("A", "verb"));
        dag.add_node(ExecNode::new("B", "verb"));
        dag.add_node(ExecNode::new("C", "verb"));
        dag.add_node(ExecNode::new("D", "verb"));
        dag.add_edge("A", "out", "B", "in");
        dag.add_edge("A", "out", "C", "in");
        dag.add_edge("B", "out", "D", "in");
        dag.add_edge("C", "out", "D", "in");
        assert_eq!(dag.node_count(), 4, "diamond DAG must have exactly 4 nodes");
    }

    // ------------------------------------------------------------------
    // Multi-root: 3 roots all appear in topo result
    // ------------------------------------------------------------------
    #[test]
    fn dag_multi_root_topological_order_covers_all_nodes() {
        let mut dag = Dag::new();
        // 3 roots each feed their own leaf.
        for r in &["r1", "r2", "r3"] {
            dag.add_node(ExecNode::new(*r, "verb"));
        }
        for l in &["l1", "l2", "l3"] {
            dag.add_node(ExecNode::new(*l, "verb"));
        }
        dag.add_edge("r1", "out", "l1", "in");
        dag.add_edge("r2", "out", "l2", "in");
        dag.add_edge("r3", "out", "l3", "in");
        let sorted = dag.topological_sort().unwrap();
        assert_eq!(sorted.len(), 6, "all 6 nodes must appear in topo result");
        for name in &["r1", "r2", "r3", "l1", "l2", "l3"] {
            assert!(sorted.contains(&name.to_string()), "{name} must be in sorted output");
        }
    }

    // ------------------------------------------------------------------
    // Isolated node: is both a root and a leaf
    // ------------------------------------------------------------------
    #[test]
    fn dag_node_with_no_edges_is_own_root() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("isolated", "verb"));
        // No edges — it's a root (block_count=0) and a leaf (no outgoing).
        assert_eq!(dag.node_count(), 1);
        assert_eq!(dag.edge_count(), 0);
        let sorted = dag.topological_sort().unwrap();
        assert_eq!(sorted, vec!["isolated"]);
        // Verify it's a root: no incoming edges.
        let in_count = dag.edges.iter().filter(|e| e.dst_node == "isolated").count();
        assert_eq!(in_count, 0, "isolated node has no incoming edges");
        // Verify it's a leaf: no outgoing edges.
        let out_count = dag.edges.iter().filter(|e| e.src_node == "isolated").count();
        assert_eq!(out_count, 0, "isolated node has no outgoing edges");
    }

    // ------------------------------------------------------------------
    // Large chain (20 nodes): topo order preserves sequence
    // ------------------------------------------------------------------
    #[test]
    fn dag_large_chain_topo_order_preserves_order() {
        let mut dag = Dag::new();
        let names: Vec<String> = (0..20).map(|i| format!("n{i:02}")).collect();
        for n in &names {
            dag.add_node(ExecNode::new(n.clone(), "verb"));
        }
        for i in 0..19 {
            dag.add_edge(names[i].clone(), "out", names[i + 1].clone(), "in");
        }
        let sorted = dag.topological_sort().unwrap();
        assert_eq!(sorted.len(), 20);
        for i in 0..19 {
            let pi = sorted.iter().position(|x| x == &names[i]).unwrap();
            let pj = sorted.iter().position(|x| x == &names[i + 1]).unwrap();
            assert!(pi < pj, "n{i:02} must precede n{:02}", i + 1);
        }
    }

    // ------------------------------------------------------------------
    // Diamond: D executes last
    // ------------------------------------------------------------------
    #[test]
    fn dag_diamond_dependency_executes_join_last() {
        // A→B, A→C, B→D, C→D — D must be last in sort.
        let mut dag = Dag::new();
        for n in &["A", "B", "C", "D"] {
            dag.add_node(ExecNode::new(*n, "verb"));
        }
        dag.add_edge("A", "out", "B", "in");
        dag.add_edge("A", "out", "C", "in");
        dag.add_edge("B", "out", "D", "in");
        dag.add_edge("C", "out", "D", "in");
        let sorted = dag.topological_sort().unwrap();
        assert_eq!(sorted.last().map(|s| s.as_str()), Some("D"), "D must execute last in diamond");
    }

    // ------------------------------------------------------------------
    // node_count after adding 5 nodes
    // ------------------------------------------------------------------
    #[test]
    fn dag_node_count_after_five_additions() {
        let mut dag = Dag::new();
        for i in 0..5 {
            dag.add_node(ExecNode::new(format!("n{i}"), "verb"));
        }
        assert_eq!(dag.node_count(), 5, "node_count must be 5 after adding 5 nodes");
    }

    // ------------------------------------------------------------------
    // edge_count after adding edges one-by-one
    // ------------------------------------------------------------------
    #[test]
    fn dag_edge_count_after_sequential_additions() {
        let mut dag = Dag::new();
        for i in 0..4 {
            dag.add_node(ExecNode::new(format!("n{i}"), "verb"));
        }
        assert_eq!(dag.edge_count(), 0);
        dag.add_edge("n0", "out", "n1", "in");
        assert_eq!(dag.edge_count(), 1);
        dag.add_edge("n1", "out", "n2", "in");
        assert_eq!(dag.edge_count(), 2);
        dag.add_edge("n2", "out", "n3", "in");
        assert_eq!(dag.edge_count(), 3);
    }

    // ------------------------------------------------------------------
    // Cycle detection: adding back-edge returns error
    // ------------------------------------------------------------------
    #[test]
    fn dag_cycle_detection_returns_error() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("X", "verb"));
        dag.add_node(ExecNode::new("Y", "verb"));
        dag.add_node(ExecNode::new("Z", "verb"));
        // Forward edges X→Y→Z form a chain.
        dag.add_edge("X", "out", "Y", "in");
        dag.add_edge("Y", "out", "Z", "in");
        // Back-edge Z→X creates a cycle.
        dag.add_edge("Z", "out", "X", "in");
        assert!(
            dag.topological_sort().is_err(),
            "back-edge Z→X must cause cycle detection to return Err"
        );
    }

    // ------------------------------------------------------------------
    // All ancestors of a leaf node precede it in topo order
    // ------------------------------------------------------------------
    #[test]
    fn dag_all_ancestors_of_leaf_precede_it() {
        // Chain: n0 → n1 → n2 → n3 → n4 (leaf)
        // All of n0..n3 are ancestors of n4 and must precede it.
        let mut dag = Dag::new();
        let names: Vec<String> = (0..5).map(|i| format!("n{i}")).collect();
        for n in &names {
            dag.add_node(ExecNode::new(n.clone(), "verb"));
        }
        for i in 0..4 {
            dag.add_edge(names[i].clone(), "out", names[i + 1].clone(), "in");
        }
        let sorted = dag.topological_sort().unwrap();
        let leaf_pos = sorted.iter().position(|x| x == "n4").unwrap();
        for i in 0..4 {
            let ancestor_pos = sorted.iter().position(|x| x == &names[i]).unwrap();
            assert!(ancestor_pos < leaf_pos, "n{i} (ancestor) must precede n4 (leaf)");
        }
    }

    // ------------------------------------------------------------------
    // All descendants of a root follow it in topo order
    // ------------------------------------------------------------------
    #[test]
    fn dag_all_descendants_of_root_follow_it() {
        // Star: root → c0, root → c1, root → c2, root → c3, root → c4
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("root", "verb"));
        for i in 0..5 {
            dag.add_node(ExecNode::new(format!("c{i}"), "verb"));
            dag.add_edge("root", "out", format!("c{i}"), "in");
        }
        let sorted = dag.topological_sort().unwrap();
        let root_pos = sorted.iter().position(|x| x == "root").unwrap();
        for i in 0..5 {
            let child_pos = sorted.iter().position(|x| x == &format!("c{i}")).unwrap();
            assert!(root_pos < child_pos, "root must precede c{i} (descendant)");
        }
    }

    // ------------------------------------------------------------------
    // Empty DAG: topological sort is empty
    // ------------------------------------------------------------------
    #[test]
    fn dag_empty_dag_topo_order_is_empty_new() {
        let dag = Dag::new();
        let sorted = dag.topological_sort().unwrap();
        assert!(sorted.is_empty(), "empty DAG topo sort must be []");
        assert_eq!(dag.node_count(), 0);
        assert_eq!(dag.edge_count(), 0);
    }

    // ------------------------------------------------------------------
    // Remove edge from edge list fixes dangling references
    // ------------------------------------------------------------------
    #[test]
    fn dag_remove_edge_reduces_edge_count() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("a", "verb"));
        dag.add_node(ExecNode::new("b", "verb"));
        dag.add_node(ExecNode::new("c", "verb"));
        dag.add_edge("a", "out", "b", "in");
        dag.add_edge("b", "out", "c", "in");
        assert_eq!(dag.edge_count(), 2);
        // Remove the a→b edge by retaining only b→c.
        dag.edges.retain(|e| !(e.src_node == "a" && e.dst_node == "b"));
        assert_eq!(dag.edge_count(), 1, "removing a→b must reduce edge_count to 1");
        // After removal, topological sort still succeeds (no cycle in remaining edges).
        assert!(dag.topological_sort().is_ok());
    }

    // ------------------------------------------------------------------
    // Wide fan-in: 5 parents feed 1 child
    // ------------------------------------------------------------------
    #[test]
    fn dag_wide_fan_in_five_parents() {
        let mut dag = Dag::new();
        for i in 0..5 {
            dag.add_node(ExecNode::new(format!("p{i}"), "verb"));
        }
        dag.add_node(ExecNode::new("child", "verb"));
        for i in 0..5 {
            dag.add_edge(format!("p{i}"), "out", "child", "in");
        }
        let sorted = dag.topological_sort().unwrap();
        assert_eq!(sorted.len(), 6);
        let child_pos = sorted.iter().position(|x| x == "child").unwrap();
        for i in 0..5 {
            let pp = sorted.iter().position(|x| x == &format!("p{i}")).unwrap();
            assert!(pp < child_pos, "p{i} must precede child");
        }
    }
}
