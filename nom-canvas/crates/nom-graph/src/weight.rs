#![deny(unsafe_code)]
//! Labeled, weighted directed edges and a simple weighted graph built from them.

/// A directed edge carrying an optional text label and an `f32` weight.
#[derive(Debug, Clone)]
pub struct EdgeWeight {
    /// Source node identifier.
    pub from: u32,
    /// Target node identifier.
    pub to: u32,
    /// Numeric weight of this edge.
    pub weight: f32,
    /// Optional human-readable label.
    pub label: Option<String>,
}

impl EdgeWeight {
    /// Create an unlabeled weighted edge.
    pub fn new(from: u32, to: u32, weight: f32) -> Self {
        Self {
            from,
            to,
            weight,
            label: None,
        }
    }

    /// Attach a label to this edge, returning `Self` for builder-style chaining.
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    /// Returns `true` when this edge carries a label.
    pub fn is_labeled(&self) -> bool {
        self.label.is_some()
    }
}

/// A directed graph of [`EdgeWeight`] edges keyed by `u32` node IDs.
#[derive(Debug, Default)]
pub struct WeightGraph {
    edges: Vec<EdgeWeight>,
}

impl WeightGraph {
    /// Create an empty graph.
    pub fn new() -> Self {
        Self { edges: Vec::new() }
    }

    /// Add an unlabeled directed edge and return `&mut Self` for chaining.
    pub fn add_edge(&mut self, from: u32, to: u32, weight: f32) -> &mut Self {
        self.edges.push(EdgeWeight::new(from, to, weight));
        self
    }

    /// Sum of all edge weights.
    pub fn total_weight(&self) -> f32 {
        self.edges.iter().map(|e| e.weight).sum()
    }

    /// Number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Maximum edge weight, or `None` if the graph is empty.
    pub fn max_weight(&self) -> Option<f32> {
        self.edges.iter().map(|e| e.weight).reduce(f32::max)
    }

    /// All edges whose `from` equals `node`.
    pub fn edges_from(&self, node: u32) -> Vec<&EdgeWeight> {
        self.edges.iter().filter(|e| e.from == node).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// EdgeWeight::new stores from/to/weight and has no label.
    #[test]
    fn edge_new() {
        let e = EdgeWeight::new(1, 2, 3.5);
        assert_eq!(e.from, 1);
        assert_eq!(e.to, 2);
        assert!((e.weight - 3.5).abs() < 1e-6);
        assert!(!e.is_labeled());
    }

    /// EdgeWeight::with_label attaches the label and is_labeled returns true.
    #[test]
    fn edge_with_label() {
        let e = EdgeWeight::new(0, 1, 1.0).with_label("connects");
        assert!(e.is_labeled());
        assert_eq!(e.label.as_deref(), Some("connects"));
    }

    /// WeightGraph::add_edge records the edge and edge_count reflects it.
    #[test]
    fn graph_add_edge() {
        let mut g = WeightGraph::new();
        g.add_edge(1, 2, 0.5).add_edge(1, 3, 1.5);
        assert_eq!(g.edge_count(), 2);
    }

    /// total_weight sums all edge weights correctly.
    #[test]
    fn total_weight() {
        let mut g = WeightGraph::new();
        g.add_edge(0, 1, 2.0).add_edge(1, 2, 3.0).add_edge(2, 3, 5.0);
        assert!((g.total_weight() - 10.0).abs() < 1e-5);
    }

    /// max_weight returns the heaviest edge, None for an empty graph.
    #[test]
    fn max_weight() {
        let g = WeightGraph::new();
        assert!(g.max_weight().is_none(), "empty graph has no max weight");

        let mut g2 = WeightGraph::new();
        g2.add_edge(0, 1, 1.0)
            .add_edge(1, 2, 9.0)
            .add_edge(2, 3, 4.0);
        let m = g2.max_weight().unwrap();
        assert!((m - 9.0).abs() < 1e-5, "max weight must be 9.0, got {m}");
    }

    /// edges_from returns only edges whose source matches the requested node.
    #[test]
    fn edges_from_node() {
        let mut g = WeightGraph::new();
        g.add_edge(1, 2, 1.0)
            .add_edge(1, 3, 2.0)
            .add_edge(2, 3, 3.0);
        let from1 = g.edges_from(1);
        assert_eq!(from1.len(), 2, "node 1 has 2 outgoing edges");
        assert!(from1.iter().all(|e| e.from == 1));
        let from2 = g.edges_from(2);
        assert_eq!(from2.len(), 1);
        let from99 = g.edges_from(99);
        assert!(from99.is_empty());
    }
}
