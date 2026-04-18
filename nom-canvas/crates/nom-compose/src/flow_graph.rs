#![deny(unsafe_code)]

use std::collections::{HashMap, VecDeque};

/// Kind of node in the flow graph.
#[derive(Debug, Clone, PartialEq)]
pub enum FlowNodeKind {
    Input,
    Transform,
    Output,
    Conditional,
}

/// A node in the flow graph.
#[derive(Debug, Clone)]
pub struct FlowNode {
    pub id: String,
    pub kind: FlowNodeKind,
    /// Runtime string — no closed enum, new kinds from DB require no Rust change.
    pub backend_kind: String,
    pub version: u32,
}

/// A directed edge between two nodes in the flow graph.
#[derive(Debug, Clone)]
pub struct FlowEdge {
    pub from_id: String,
    pub to_id: String,
    pub label: String,
}

/// Typed directed flow graph replacing the linear ComposeOrchestrator approach.
#[derive(Debug, Clone)]
pub struct FlowGraph {
    pub nodes: HashMap<String, FlowNode>,
    pub edges: Vec<FlowEdge>,
    pub version: u32,
}

impl FlowGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            version: 0,
        }
    }

    /// Add a node, keyed by its id.
    pub fn add_node(&mut self, node: FlowNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    /// Add an edge. Returns Err if either endpoint is not a registered node.
    pub fn add_edge(&mut self, edge: FlowEdge) -> Result<(), String> {
        if !self.nodes.contains_key(&edge.from_id) {
            return Err(format!("add_edge: unknown from_id '{}'", edge.from_id));
        }
        if !self.nodes.contains_key(&edge.to_id) {
            return Err(format!("add_edge: unknown to_id '{}'", edge.to_id));
        }
        self.edges.push(edge);
        Ok(())
    }

    /// Kahn's algorithm: returns node IDs in topological execution order.
    /// Nodes with no incoming edges are processed first.
    /// Returns all reachable nodes; does not error on cycles (returns partial order).
    pub fn topological_order(&self) -> Vec<String> {
        // Build in-degree map and adjacency list
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

        for id in self.nodes.keys() {
            in_degree.entry(id.as_str()).or_insert(0);
            adj.entry(id.as_str()).or_default();
        }

        for edge in &self.edges {
            *in_degree.entry(edge.to_id.as_str()).or_insert(0) += 1;
            adj.entry(edge.from_id.as_str())
                .or_default()
                .push(edge.to_id.as_str());
        }

        // Seed queue with zero-in-degree nodes, sorted for determinism
        let mut queue: VecDeque<&str> = {
            let mut starts: Vec<&str> = in_degree
                .iter()
                .filter(|(_, &deg)| deg == 0)
                .map(|(&id, _)| id)
                .collect();
            starts.sort();
            starts.into()
        };

        let mut order: Vec<String> = Vec::new();
        while let Some(node) = queue.pop_front() {
            order.push(node.to_string());
            if let Some(neighbors) = adj.get(node) {
                let mut next: Vec<&str> = neighbors.clone();
                next.sort();
                for neighbor in next {
                    let deg = in_degree.entry(neighbor).or_insert(0);
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 {
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        order
    }

    /// Bump the graph version (call after any structural change).
    pub fn bump_version(&mut self) {
        self.version += 1;
    }
}

impl Default for FlowGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: &str, kind: FlowNodeKind, backend_kind: &str) -> FlowNode {
        FlowNode {
            id: id.to_string(),
            kind,
            backend_kind: backend_kind.to_string(),
            version: 1,
        }
    }

    #[test]
    fn test_flow_graph_add_node() {
        let mut g = FlowGraph::new();
        g.add_node(make_node("n1", FlowNodeKind::Input, "video"));
        g.add_node(make_node("n2", FlowNodeKind::Transform, "audio"));
        assert_eq!(g.nodes.len(), 2);
        assert!(g.nodes.contains_key("n1"));
        assert!(g.nodes.contains_key("n2"));
        assert_eq!(g.nodes["n1"].backend_kind, "video");
        assert_eq!(g.nodes["n2"].backend_kind, "audio");
    }

    #[test]
    fn test_flow_graph_add_edge_unknown_node_fails() {
        let mut g = FlowGraph::new();
        g.add_node(make_node("n1", FlowNodeKind::Input, "video"));
        // to_id "n2" not registered
        let err = g
            .add_edge(FlowEdge {
                from_id: "n1".to_string(),
                to_id: "n2".to_string(),
                label: "stream".to_string(),
            })
            .unwrap_err();
        assert!(err.contains("n2"), "error must name the missing node");

        // from_id missing
        let err2 = g
            .add_edge(FlowEdge {
                from_id: "ghost".to_string(),
                to_id: "n1".to_string(),
                label: "edge".to_string(),
            })
            .unwrap_err();
        assert!(
            err2.contains("ghost"),
            "error must name the missing from_id"
        );
    }

    #[test]
    fn test_flow_graph_topological_order_linear() {
        // n1 → n2 → n3
        let mut g = FlowGraph::new();
        g.add_node(make_node("n1", FlowNodeKind::Input, "video"));
        g.add_node(make_node("n2", FlowNodeKind::Transform, "transform"));
        g.add_node(make_node("n3", FlowNodeKind::Output, "export"));

        g.add_edge(FlowEdge {
            from_id: "n1".to_string(),
            to_id: "n2".to_string(),
            label: "a".to_string(),
        })
        .unwrap();
        g.add_edge(FlowEdge {
            from_id: "n2".to_string(),
            to_id: "n3".to_string(),
            label: "b".to_string(),
        })
        .unwrap();

        let order = g.topological_order();
        assert_eq!(order.len(), 3);
        let pos = |id: &str| order.iter().position(|x| x == id).unwrap();
        assert!(pos("n1") < pos("n2"), "n1 must come before n2");
        assert!(pos("n2") < pos("n3"), "n2 must come before n3");
    }

    #[test]
    fn test_flow_graph_version_bumps() {
        let mut g = FlowGraph::new();
        assert_eq!(g.version, 0, "initial version must be 0");
        g.bump_version();
        assert_eq!(g.version, 1);
        g.bump_version();
        assert_eq!(g.version, 2);
        g.add_node(make_node("x", FlowNodeKind::Conditional, "data"));
        g.bump_version();
        assert_eq!(g.version, 3);
    }

    #[test]
    fn test_flow_graph_edge_label_preserved() {
        let mut g = FlowGraph::new();
        g.add_node(make_node("a", FlowNodeKind::Input, "audio"));
        g.add_node(make_node("b", FlowNodeKind::Output, "render"));
        g.add_edge(FlowEdge {
            from_id: "a".to_string(),
            to_id: "b".to_string(),
            label: "raw_stream".to_string(),
        })
        .unwrap();
        assert_eq!(g.edges[0].label, "raw_stream");
    }

    #[test]
    fn test_flow_graph_topological_order_single_node() {
        let mut g = FlowGraph::new();
        g.add_node(make_node("solo", FlowNodeKind::Input, "video"));
        let order = g.topological_order();
        assert_eq!(order, vec!["solo"]);
    }

    #[test]
    fn test_flow_graph_topological_order_empty() {
        let g = FlowGraph::new();
        let order = g.topological_order();
        assert!(order.is_empty(), "empty graph must yield empty order");
    }

    #[test]
    fn test_flow_node_kind_equality() {
        assert_eq!(FlowNodeKind::Input, FlowNodeKind::Input);
        assert_ne!(FlowNodeKind::Input, FlowNodeKind::Output);
        assert_ne!(FlowNodeKind::Transform, FlowNodeKind::Conditional);
    }
}
