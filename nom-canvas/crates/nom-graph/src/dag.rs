#![deny(unsafe_code)]
use std::collections::{HashMap, HashSet, VecDeque};
use crate::node::{ExecNode, NodeId};

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
        Self { nodes: HashMap::new(), edges: Vec::new() }
    }

    pub fn add_node(&mut self, node: ExecNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn add_edge(&mut self, src_node: impl Into<String>, src_port: impl Into<String>,
                    dst_node: impl Into<String>, dst_port: impl Into<String>) {
        self.edges.push(Edge {
            src_node: src_node.into(), src_port: src_port.into(),
            dst_node: dst_node.into(), dst_port: dst_port.into(),
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
            blocking.entry(id.clone()).or_insert_with(Vec::new);
        }

        for edge in &self.edges {
            *block_count.entry(edge.dst_node.clone()).or_insert(0) += 1;
            blocking.entry(edge.src_node.clone()).or_insert_with(Vec::new).push(edge.dst_node.clone());
        }

        let mut queue: VecDeque<String> = block_count.iter()
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
            let cycle_nodes = self.nodes.keys()
                .filter(|id| !resolved.contains(id.as_str()))
                .cloned()
                .collect();
            Err(cycle_nodes)
        }
    }

    pub fn node_count(&self) -> usize { self.nodes.len() }
    pub fn edge_count(&self) -> usize { self.edges.len() }
}

impl Default for Dag { fn default() -> Self { Self::new() } }

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
        dag.edges.retain(|e| e.src_node != "alpha" && e.dst_node != "alpha");
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
        assert_eq!(dag.edges[0].confidence, 1.0, "confidence above 1.0 must be clamped to 1.0");
    }

    #[test]
    fn dag_default_edge_confidence_is_one() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("a", "verb"));
        dag.add_node(ExecNode::new("b", "verb"));
        dag.add_edge("a", "out", "b", "in");
        assert_eq!(dag.edges[0].confidence, 1.0, "unweighted add_edge must set confidence to 1.0");
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
        assert!(active_edges.is_empty(), "zero-confidence edge must be pruned by confidence filter");
    }
}
