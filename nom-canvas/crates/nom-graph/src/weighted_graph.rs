#![deny(unsafe_code)]
//! A simple weighted directed graph keyed by `u32` node IDs.
//!
//! Designed for golden-path tests and lightweight scenarios where
//! the full [`Dag`] / [`ExecNode`] machinery is unnecessary.

/// A directed edge in a [`WeightedGraph`].
#[derive(Debug, Clone)]
pub struct WeightedEdge {
    pub from: u32,
    pub to: u32,
    pub weight: f64,
}

/// Lightweight directed weighted graph keyed by `u32` node IDs.
#[derive(Debug, Default)]
pub struct WeightedGraph {
    edges: Vec<WeightedEdge>,
}

impl WeightedGraph {
    /// Create an empty weighted graph.
    pub fn new() -> Self {
        Self { edges: Vec::new() }
    }

    /// Add a directed edge with the given weight.  Returns `&mut Self` for
    /// builder-style chaining.
    pub fn add_edge(&mut self, from: u32, to: u32, weight: f64) -> &mut Self {
        self.edges.push(WeightedEdge { from, to, weight });
        self
    }

    /// Number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Sum of all edge weights.
    pub fn total_weight(&self) -> f64 {
        self.edges.iter().map(|e| e.weight).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_graph_empty() {
        let g = WeightedGraph::new();
        assert_eq!(g.edge_count(), 0);
        assert!((g.total_weight()).abs() < 1e-9);
    }

    #[test]
    fn weighted_graph_add_two_edges() {
        let mut g = WeightedGraph::new();
        g.add_edge(1, 2, 0.5).add_edge(1, 3, 1.5);
        assert_eq!(g.edge_count(), 2);
        assert!((g.total_weight() - 2.0).abs() < 0.001);
    }
}
