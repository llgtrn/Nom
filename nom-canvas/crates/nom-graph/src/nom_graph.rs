#![deny(unsafe_code)]
//! Nom-graph: a directed graph over [`NomtuRef`] nodes.
//!
//! Provides graph primitives (node/edge inspection, cycle detection, topological
//! sort, connected-components, neighbor queries) using `NomtuRef` as the vertex key.

use std::collections::{HashMap, HashSet, VecDeque};

// ---------------------------------------------------------------------------
// NomtuRef
// ---------------------------------------------------------------------------

/// A reference to a nomtu dictionary entry, identified by a 64-bit hash and a
/// human-readable word string.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NomtuRef {
    pub hash: u64,
    pub word: String,
}

impl NomtuRef {
    pub fn new(hash: u64, word: impl Into<String>) -> Self {
        Self {
            hash,
            word: word.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// NomGraph
// ---------------------------------------------------------------------------

/// Directed graph over [`NomtuRef`] vertices.
///
/// Edges are stored as a simple adjacency list.  Parallel edges (same src → dst)
/// are allowed; the caller is responsible for deduplication when needed.
#[derive(Clone, Debug, Default)]
pub struct NomGraph {
    /// Adjacency list: src → set of dst.
    adj: HashMap<NomtuRef, Vec<NomtuRef>>,
    /// Reverse adjacency list: dst → set of src (for in-degree / in-neighbors).
    radj: HashMap<NomtuRef, Vec<NomtuRef>>,
}

impl NomGraph {
    /// Create an empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a node.  No-op if the node already exists.
    pub fn add_node(&mut self, id: NomtuRef) {
        self.adj.entry(id.clone()).or_default();
        self.radj.entry(id).or_default();
    }

    /// Insert a directed edge from `from` to `to`.
    /// Both endpoints are automatically added as nodes if absent.
    pub fn add_edge(&mut self, from: NomtuRef, to: NomtuRef) {
        self.add_node(from.clone());
        self.add_node(to.clone());
        self.adj.entry(from.clone()).or_default().push(to.clone());
        self.radj.entry(to).or_default().push(from);
    }

    // ------------------------------------------------------------------
    // Basic queries
    // ------------------------------------------------------------------

    /// Number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.adj.len()
    }

    /// Number of directed edges in the graph (counting parallel edges).
    pub fn edge_count(&self) -> usize {
        self.adj.values().map(|v| v.len()).sum()
    }

    /// Returns `true` if the graph contains a node with the given id.
    pub fn has_node(&self, id: &NomtuRef) -> bool {
        self.adj.contains_key(id)
    }

    /// Returns `true` if there is at least one directed edge from `from` to `to`.
    pub fn has_edge(&self, from: &NomtuRef, to: &NomtuRef) -> bool {
        self.adj
            .get(from)
            .map_or(false, |neighbors| neighbors.contains(to))
    }

    // ------------------------------------------------------------------
    // Neighbor / degree queries
    // ------------------------------------------------------------------

    /// Returns the out-neighbors of `id` (nodes reachable via a single forward edge).
    /// Returns an empty vec if the node does not exist.
    pub fn neighbors(&self, id: &NomtuRef) -> Vec<NomtuRef> {
        self.adj.get(id).cloned().unwrap_or_default()
    }

    /// In-degree of `id`: number of edges directed *into* the node.
    pub fn in_degree(&self, id: &NomtuRef) -> usize {
        self.radj.get(id).map_or(0, |v| v.len())
    }

    /// Out-degree of `id`: number of edges directed *out of* the node.
    pub fn out_degree(&self, id: &NomtuRef) -> usize {
        self.adj.get(id).map_or(0, |v| v.len())
    }

    // ------------------------------------------------------------------
    // Cycle detection & topological sort
    // ------------------------------------------------------------------

    /// Returns `true` if the directed graph contains at least one cycle.
    ///
    /// Uses iterative DFS with a three-colour marking scheme
    /// (white = unvisited, gray = in-stack, black = done).
    pub fn is_cyclic(&self) -> bool {
        // 0 = white, 1 = gray (in stack), 2 = black (done)
        let mut color: HashMap<&NomtuRef, u8> = HashMap::new();

        for start in self.adj.keys() {
            if color.get(start).copied().unwrap_or(0) == 0 && self.dfs_has_cycle(start, &mut color)
            {
                return true;
            }
        }
        false
    }

    fn dfs_has_cycle<'a>(
        &'a self,
        start: &'a NomtuRef,
        color: &mut HashMap<&'a NomtuRef, u8>,
    ) -> bool {
        // Iterative DFS using an explicit stack of (node, iterator-index).
        // We push an entry for `start`, mark it gray, then explore neighbors.
        let mut stack: Vec<(&NomtuRef, usize)> = vec![(start, 0)];
        color.insert(start, 1);

        while let Some((node, idx)) = stack.last_mut() {
            let neighbors = self.adj.get(*node).map(|v| v.as_slice()).unwrap_or(&[]);
            if *idx < neighbors.len() {
                let next = &neighbors[*idx];
                *idx += 1;
                match color.get(next).copied().unwrap_or(0) {
                    1 => return true, // back edge → cycle
                    0 => {
                        color.insert(next, 1);
                        stack.push((next, 0));
                    }
                    _ => {} // black — already fully explored
                }
            } else {
                // All neighbors explored; mark black and pop.
                color.insert(*node, 2);
                stack.pop();
            }
        }
        false
    }

    /// Topological sort of the graph.
    ///
    /// Returns `Some(order)` if the graph is acyclic, `None` if it contains a cycle.
    /// The returned vector lists nodes from least to most dependent (sources first).
    pub fn topological_sort(&self) -> Option<Vec<NomtuRef>> {
        // Kahn's algorithm.
        let mut in_deg: HashMap<&NomtuRef, usize> = HashMap::new();
        for node in self.adj.keys() {
            in_deg.entry(node).or_insert(0);
        }
        for neighbors in self.adj.values() {
            for nb in neighbors {
                *in_deg.entry(nb).or_insert(0) += 1;
            }
        }

        // Seed queue with all zero-in-degree nodes, sorted for determinism.
        let mut queue: VecDeque<&NomtuRef> = {
            let mut seeds: Vec<&NomtuRef> = in_deg
                .iter()
                .filter(|(_, &d)| d == 0)
                .map(|(n, _)| *n)
                .collect();
            seeds.sort_by_key(|n| (n.hash, n.word.as_str()));
            VecDeque::from(seeds)
        };

        let mut order: Vec<NomtuRef> = Vec::with_capacity(self.adj.len());
        while let Some(node) = queue.pop_front() {
            order.push(node.clone());
            if let Some(neighbors) = self.adj.get(node) {
                let mut next_batch: Vec<&NomtuRef> = Vec::new();
                for nb in neighbors {
                    let d = in_deg.get_mut(nb).unwrap();
                    *d -= 1;
                    if *d == 0 {
                        next_batch.push(nb);
                    }
                }
                next_batch.sort_by_key(|n| (n.hash, n.word.as_str()));
                for nb in next_batch {
                    queue.push_back(nb);
                }
            }
        }

        if order.len() == self.adj.len() {
            Some(order)
        } else {
            None // cycle detected
        }
    }

    // ------------------------------------------------------------------
    // Connected components (treating edges as undirected)
    // ------------------------------------------------------------------

    /// Returns the number of weakly-connected components in the graph.
    ///
    /// Each node that has no edges (and is not part of any component) counts
    /// as its own component.
    pub fn connected_components(&self) -> usize {
        let mut visited: HashSet<&NomtuRef> = HashSet::new();
        let mut count = 0;

        for start in self.adj.keys() {
            if visited.contains(start) {
                continue;
            }
            // BFS treating edges as undirected.
            let mut queue: VecDeque<&NomtuRef> = VecDeque::new();
            queue.push_back(start);
            visited.insert(start);

            while let Some(node) = queue.pop_front() {
                // Forward edges.
                if let Some(fwd) = self.adj.get(node) {
                    for nb in fwd {
                        if visited.insert(nb) {
                            queue.push_back(nb);
                        }
                    }
                }
                // Reverse edges (so we treat the graph as undirected).
                if let Some(rev) = self.radj.get(node) {
                    for nb in rev {
                        if visited.insert(nb) {
                            queue.push_back(nb);
                        }
                    }
                }
            }
            count += 1;
        }
        count
    }

    // ------------------------------------------------------------------
    // Execution order
    // ------------------------------------------------------------------

    /// Returns a valid execution order for all nodes.
    ///
    /// Equivalent to `topological_sort()` but returns an empty vec on cycles
    /// rather than `None`, making it a convenient wrapper for callers that
    /// want a best-effort order.
    pub fn execution_order(&self) -> Vec<NomtuRef> {
        self.topological_sort().unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn r(hash: u64, word: &str) -> NomtuRef {
        NomtuRef::new(hash, word)
    }

    // ------------------------------------------------------------------
    // node_count / edge_count / has_node / has_edge
    // ------------------------------------------------------------------

    #[test]
    fn nom_graph_empty_counts_zero() {
        let g = NomGraph::new();
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.edge_count(), 0);
    }

    #[test]
    fn nom_graph_node_count_after_add() {
        let mut g = NomGraph::new();
        g.add_node(r(1, "alpha"));
        g.add_node(r(2, "beta"));
        assert_eq!(g.node_count(), 2);
    }

    #[test]
    fn nom_graph_edge_count_after_add() {
        let mut g = NomGraph::new();
        g.add_edge(r(1, "a"), r(2, "b"));
        g.add_edge(r(2, "b"), r(3, "c"));
        assert_eq!(g.edge_count(), 2);
    }

    #[test]
    fn nom_graph_add_edge_also_adds_nodes() {
        let mut g = NomGraph::new();
        g.add_edge(r(10, "src"), r(20, "dst"));
        assert_eq!(g.node_count(), 2);
        assert!(g.has_node(&r(10, "src")));
        assert!(g.has_node(&r(20, "dst")));
    }

    #[test]
    fn nom_graph_has_node_true_for_existing() {
        let mut g = NomGraph::new();
        g.add_node(r(7, "seven"));
        assert!(g.has_node(&r(7, "seven")));
    }

    #[test]
    fn nom_graph_has_node_false_for_missing() {
        let g = NomGraph::new();
        assert!(!g.has_node(&r(99, "missing")));
    }

    #[test]
    fn nom_graph_has_edge_true_for_existing() {
        let mut g = NomGraph::new();
        g.add_edge(r(1, "x"), r(2, "y"));
        assert!(g.has_edge(&r(1, "x"), &r(2, "y")));
    }

    #[test]
    fn nom_graph_has_edge_false_for_reverse() {
        let mut g = NomGraph::new();
        g.add_edge(r(1, "x"), r(2, "y"));
        assert!(!g.has_edge(&r(2, "y"), &r(1, "x")));
    }

    #[test]
    fn nom_graph_has_edge_false_for_missing() {
        let mut g = NomGraph::new();
        g.add_node(r(1, "a"));
        g.add_node(r(2, "b"));
        assert!(!g.has_edge(&r(1, "a"), &r(2, "b")));
    }

    #[test]
    fn nom_graph_duplicate_node_add_is_idempotent() {
        let mut g = NomGraph::new();
        g.add_node(r(5, "dup"));
        g.add_node(r(5, "dup"));
        assert_eq!(g.node_count(), 1);
    }

    // ------------------------------------------------------------------
    // neighbors / in_degree / out_degree
    // ------------------------------------------------------------------

    #[test]
    fn nom_graph_neighbors_returns_out_neighbors() {
        let mut g = NomGraph::new();
        g.add_edge(r(1, "a"), r(2, "b"));
        g.add_edge(r(1, "a"), r(3, "c"));
        let mut nb = g.neighbors(&r(1, "a"));
        nb.sort_by_key(|n| n.hash);
        assert_eq!(nb.len(), 2);
        assert!(nb.contains(&r(2, "b")));
        assert!(nb.contains(&r(3, "c")));
    }

    #[test]
    fn nom_graph_neighbors_empty_for_sink() {
        let mut g = NomGraph::new();
        g.add_edge(r(1, "src"), r(2, "sink"));
        assert!(g.neighbors(&r(2, "sink")).is_empty());
    }

    #[test]
    fn nom_graph_neighbors_empty_for_missing_node() {
        let g = NomGraph::new();
        assert!(g.neighbors(&r(99, "ghost")).is_empty());
    }

    #[test]
    fn nom_graph_out_degree_correct() {
        let mut g = NomGraph::new();
        g.add_edge(r(1, "a"), r(2, "b"));
        g.add_edge(r(1, "a"), r(3, "c"));
        assert_eq!(g.out_degree(&r(1, "a")), 2);
    }

    #[test]
    fn nom_graph_in_degree_correct() {
        let mut g = NomGraph::new();
        g.add_edge(r(1, "a"), r(3, "c"));
        g.add_edge(r(2, "b"), r(3, "c"));
        assert_eq!(g.in_degree(&r(3, "c")), 2);
    }

    #[test]
    fn nom_graph_out_degree_zero_for_isolated() {
        let mut g = NomGraph::new();
        g.add_node(r(5, "iso"));
        assert_eq!(g.out_degree(&r(5, "iso")), 0);
    }

    #[test]
    fn nom_graph_in_degree_zero_for_source() {
        let mut g = NomGraph::new();
        g.add_edge(r(1, "src"), r(2, "dst"));
        assert_eq!(g.in_degree(&r(1, "src")), 0);
    }

    #[test]
    fn nom_graph_degree_zero_for_missing_node() {
        let g = NomGraph::new();
        assert_eq!(g.in_degree(&r(99, "x")), 0);
        assert_eq!(g.out_degree(&r(99, "x")), 0);
    }

    #[test]
    fn nom_graph_neighbors_not_affected_by_incoming_edges() {
        // Node "b" receives edges from "a" but has no outgoing edges.
        let mut g = NomGraph::new();
        g.add_edge(r(1, "a"), r(2, "b"));
        assert!(g.neighbors(&r(2, "b")).is_empty());
    }

    // ------------------------------------------------------------------
    // is_cyclic / topological_sort
    // ------------------------------------------------------------------

    #[test]
    fn nom_graph_acyclic_not_cyclic() {
        let mut g = NomGraph::new();
        g.add_edge(r(1, "a"), r(2, "b"));
        g.add_edge(r(2, "b"), r(3, "c"));
        assert!(!g.is_cyclic());
    }

    #[test]
    fn nom_graph_self_loop_is_cyclic() {
        let mut g = NomGraph::new();
        g.add_node(r(1, "a"));
        g.add_edge(r(1, "a"), r(1, "a"));
        assert!(g.is_cyclic());
    }

    #[test]
    fn nom_graph_two_node_cycle_detected() {
        let mut g = NomGraph::new();
        g.add_edge(r(1, "x"), r(2, "y"));
        g.add_edge(r(2, "y"), r(1, "x"));
        assert!(g.is_cyclic());
    }

    #[test]
    fn nom_graph_three_node_cycle_detected() {
        let mut g = NomGraph::new();
        g.add_edge(r(1, "a"), r(2, "b"));
        g.add_edge(r(2, "b"), r(3, "c"));
        g.add_edge(r(3, "c"), r(1, "a"));
        assert!(g.is_cyclic());
    }

    #[test]
    fn nom_graph_empty_graph_not_cyclic() {
        let g = NomGraph::new();
        assert!(!g.is_cyclic());
    }

    #[test]
    fn nom_graph_single_node_no_edge_not_cyclic() {
        let mut g = NomGraph::new();
        g.add_node(r(1, "solo"));
        assert!(!g.is_cyclic());
    }

    #[test]
    fn nom_graph_topo_sort_linear() {
        let mut g = NomGraph::new();
        g.add_edge(r(1, "a"), r(2, "b"));
        g.add_edge(r(2, "b"), r(3, "c"));
        let order = g.topological_sort().unwrap();
        let pos = |hash: u64| order.iter().position(|n| n.hash == hash).unwrap();
        assert!(pos(1) < pos(2));
        assert!(pos(2) < pos(3));
    }

    #[test]
    fn nom_graph_topo_sort_returns_none_on_cycle() {
        let mut g = NomGraph::new();
        g.add_edge(r(1, "a"), r(2, "b"));
        g.add_edge(r(2, "b"), r(1, "a"));
        assert!(g.topological_sort().is_none());
    }

    #[test]
    fn nom_graph_topo_sort_empty() {
        let g = NomGraph::new();
        assert_eq!(g.topological_sort(), Some(vec![]));
    }

    #[test]
    fn nom_graph_topo_sort_diamond() {
        // a→b, a→c, b→d, c→d
        let mut g = NomGraph::new();
        g.add_edge(r(1, "a"), r(2, "b"));
        g.add_edge(r(1, "a"), r(3, "c"));
        g.add_edge(r(2, "b"), r(4, "d"));
        g.add_edge(r(3, "c"), r(4, "d"));
        let order = g.topological_sort().unwrap();
        assert_eq!(order.len(), 4);
        let pos = |h: u64| order.iter().position(|n| n.hash == h).unwrap();
        assert!(pos(1) < pos(2));
        assert!(pos(1) < pos(3));
        assert!(pos(2) < pos(4));
        assert!(pos(3) < pos(4));
    }

    // ------------------------------------------------------------------
    // connected_components
    // ------------------------------------------------------------------

    #[test]
    fn nom_graph_single_component() {
        let mut g = NomGraph::new();
        g.add_edge(r(1, "a"), r(2, "b"));
        g.add_edge(r(2, "b"), r(3, "c"));
        assert_eq!(g.connected_components(), 1);
    }

    #[test]
    fn nom_graph_two_isolated_nodes_two_components() {
        let mut g = NomGraph::new();
        g.add_node(r(1, "alpha"));
        g.add_node(r(2, "beta"));
        assert_eq!(g.connected_components(), 2);
    }

    #[test]
    fn nom_graph_three_components() {
        let mut g = NomGraph::new();
        g.add_edge(r(1, "a"), r(2, "b")); // component 1
        g.add_node(r(3, "c")); // component 2
        g.add_node(r(4, "d")); // component 3
        assert_eq!(g.connected_components(), 3);
    }

    #[test]
    fn nom_graph_empty_graph_zero_components() {
        let g = NomGraph::new();
        assert_eq!(g.connected_components(), 0);
    }

    #[test]
    fn nom_graph_bidirectional_edges_one_component() {
        let mut g = NomGraph::new();
        g.add_edge(r(1, "x"), r(2, "y"));
        g.add_edge(r(2, "y"), r(1, "x"));
        assert_eq!(g.connected_components(), 1);
    }

    // ------------------------------------------------------------------
    // execution_order
    // ------------------------------------------------------------------

    #[test]
    fn nom_graph_execution_order_acyclic() {
        let mut g = NomGraph::new();
        g.add_edge(r(10, "fetch"), r(20, "transform"));
        g.add_edge(r(20, "transform"), r(30, "store"));
        let order = g.execution_order();
        assert_eq!(order.len(), 3);
        let pos = |h: u64| order.iter().position(|n| n.hash == h).unwrap();
        assert!(pos(10) < pos(20));
        assert!(pos(20) < pos(30));
    }

    #[test]
    fn nom_graph_execution_order_cyclic_returns_empty() {
        let mut g = NomGraph::new();
        g.add_edge(r(1, "a"), r(2, "b"));
        g.add_edge(r(2, "b"), r(1, "a"));
        assert!(g.execution_order().is_empty());
    }

    #[test]
    fn nom_graph_execution_order_empty_graph() {
        let g = NomGraph::new();
        assert!(g.execution_order().is_empty());
    }

    #[test]
    fn nom_graph_execution_order_single_node() {
        let mut g = NomGraph::new();
        g.add_node(r(42, "solo"));
        let order = g.execution_order();
        assert_eq!(order.len(), 1);
        assert_eq!(order[0], r(42, "solo"));
    }

    #[test]
    fn nom_graph_execution_order_parallel_independent_nodes() {
        // Two independent nodes: a and b with no edges between them.
        let mut g = NomGraph::new();
        g.add_node(r(1, "a"));
        g.add_node(r(2, "b"));
        let order = g.execution_order();
        assert_eq!(order.len(), 2);
        assert!(order.contains(&r(1, "a")));
        assert!(order.contains(&r(2, "b")));
    }

    #[test]
    fn nom_graph_node_count_matches_unique_keys() {
        // Adding the same node twice must not increment node_count.
        let mut g = NomGraph::new();
        g.add_node(r(1, "x"));
        g.add_node(r(2, "y"));
        g.add_node(r(1, "x")); // duplicate — must be ignored
        assert_eq!(g.node_count(), 2);
        assert_eq!(g.edge_count(), 0);
    }
}
