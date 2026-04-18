/// Traversal strategy for graph walks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraversalOrder {
    /// Visit nodes deepest-first.
    DepthFirst,
    /// Visit nodes level-by-level.
    BreadthFirst,
    /// Visit nodes in topological order.
    TopologicalSort,
}

impl TraversalOrder {
    /// Human-readable name for this traversal order.
    pub fn name(&self) -> &str {
        match self {
            TraversalOrder::DepthFirst => "depth_first",
            TraversalOrder::BreadthFirst => "breadth_first",
            TraversalOrder::TopologicalSort => "topological_sort",
        }
    }
}

/// Result produced by a graph traversal.
#[derive(Debug)]
pub struct TraversalResult {
    /// Node ids in the order they were visited.
    pub visited: Vec<u32>,
    /// Strategy that produced this result.
    pub order: TraversalOrder,
    /// Whether at least one cycle was detected during the walk.
    pub cycles_detected: bool,
}

impl TraversalResult {
    /// Create an empty result for the given `order`.
    pub fn new(order: TraversalOrder) -> Self {
        Self {
            visited: Vec::new(),
            order,
            cycles_detected: false,
        }
    }

    /// Append `id` to the visited list.
    pub fn add_visited(&mut self, id: u32) {
        self.visited.push(id);
    }

    /// Number of nodes visited so far.
    pub fn visit_count(&self) -> usize {
        self.visited.len()
    }

    /// Record that a cycle was detected.
    pub fn mark_cycle(&mut self) {
        self.cycles_detected = true;
    }
}

/// A lightweight directed graph represented as an edge list.
#[derive(Debug, Default)]
pub struct GraphTraversal {
    adjacency: Vec<(u32, u32)>,
}

impl GraphTraversal {
    /// Create an empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a directed edge from `from` to `to`.
    pub fn add_edge(&mut self, from: u32, to: u32) {
        self.adjacency.push((from, to));
    }

    /// Number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.adjacency.len()
    }

    /// Depth-first traversal starting at `start`.
    /// Only nodes reachable from `start` are visited.
    pub fn dfs(&self, start: u32) -> TraversalResult {
        let mut result = TraversalResult::new(TraversalOrder::DepthFirst);
        let mut visited: std::collections::HashSet<u32> = std::collections::HashSet::new();
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            if visited.contains(&node) {
                result.mark_cycle();
                continue;
            }
            visited.insert(node);
            result.add_visited(node);
            // Push neighbours in reverse order so left-most is explored first.
            let mut neighbours: Vec<u32> = self
                .adjacency
                .iter()
                .filter(|(f, _)| *f == node)
                .map(|(_, t)| *t)
                .collect();
            neighbours.reverse();
            for n in neighbours {
                stack.push(n);
            }
        }
        result
    }

    /// Breadth-first traversal starting at `start`.
    pub fn bfs(&self, start: u32) -> TraversalResult {
        let mut result = TraversalResult::new(TraversalOrder::BreadthFirst);
        let mut visited: std::collections::HashSet<u32> = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(start);
        visited.insert(start);
        while let Some(node) = queue.pop_front() {
            result.add_visited(node);
            let neighbours: Vec<u32> = self
                .adjacency
                .iter()
                .filter(|(f, _)| *f == node)
                .map(|(_, t)| *t)
                .collect();
            for n in neighbours {
                if !visited.contains(&n) {
                    visited.insert(n);
                    queue.push_back(n);
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_empty() {
        let g = GraphTraversal::new();
        assert_eq!(g.edge_count(), 0);
    }

    #[test]
    fn add_edge() {
        let mut g = GraphTraversal::new();
        g.add_edge(1, 2);
        g.add_edge(2, 3);
        assert_eq!(g.edge_count(), 2);
    }

    #[test]
    fn dfs_single() {
        let mut g = GraphTraversal::new();
        g.add_edge(1, 2);
        g.add_edge(2, 3);
        let r = g.dfs(1);
        assert_eq!(r.visited, vec![1, 2, 3]);
        assert_eq!(r.order, TraversalOrder::DepthFirst);
    }

    #[test]
    fn bfs_single() {
        let mut g = GraphTraversal::new();
        g.add_edge(1, 2);
        g.add_edge(1, 3);
        g.add_edge(2, 4);
        let r = g.bfs(1);
        // Level 0: 1, Level 1: 2, 3, Level 2: 4
        assert_eq!(r.visited[0], 1);
        assert!(r.visited.contains(&2));
        assert!(r.visited.contains(&3));
        assert!(r.visited.contains(&4));
        assert_eq!(r.order, TraversalOrder::BreadthFirst);
    }

    #[test]
    fn edge_count() {
        let mut g = GraphTraversal::new();
        g.add_edge(0, 1);
        g.add_edge(0, 2);
        g.add_edge(1, 3);
        assert_eq!(g.edge_count(), 3);
    }

    #[test]
    fn traversal_visit_count() {
        let mut g = GraphTraversal::new();
        g.add_edge(10, 20);
        g.add_edge(20, 30);
        g.add_edge(30, 40);
        let r = g.dfs(10);
        assert_eq!(r.visit_count(), 4);
    }
}

// ---------------------------------------------------------------------------
// GraphEdge + SimpleGraph
// ---------------------------------------------------------------------------

/// A directed edge in a weighted graph.
#[derive(Debug, Clone)]
pub struct GraphEdge {
    /// Source node id.
    pub from: u64,
    /// Destination node id.
    pub to: u64,
    /// Edge weight.
    pub weight: f32,
    /// Optional label.
    pub label: String,
}

impl GraphEdge {
    /// Create a new edge.
    pub fn new(from: u64, to: u64, weight: f32, label: impl Into<String>) -> Self {
        Self { from, to, weight, label: label.into() }
    }

    /// Returns `true` when the edge weight exceeds 1.0.
    pub fn is_heavy(&self) -> bool {
        self.weight > 1.0
    }
}

/// A simple directed graph backed by a list of [`GraphEdge`]s.
#[derive(Debug, Default)]
pub struct SimpleGraph {
    /// All edges in the graph.
    pub edges: Vec<GraphEdge>,
}

impl SimpleGraph {
    /// Create an empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an edge.
    pub fn add_edge(&mut self, edge: GraphEdge) {
        self.edges.push(edge);
    }

    /// All destination node ids reachable from `node` in one hop.
    pub fn neighbors(&self, node: u64) -> Vec<u64> {
        self.edges.iter().filter(|e| e.from == node).map(|e| e.to).collect()
    }

    /// Total number of edges stored.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// All edges whose weight exceeds 1.0.
    pub fn heavy_edges(&self) -> Vec<&GraphEdge> {
        self.edges.iter().filter(|e| e.is_heavy()).collect()
    }

    /// BFS traversal from `start`; returns visited node ids in visit order (no duplicates).
    pub fn bfs(&self, start: u64) -> Vec<u64> {
        let mut visited: std::collections::HashSet<u64> = std::collections::HashSet::new();
        let mut order: Vec<u64> = Vec::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(start);
        visited.insert(start);
        while let Some(node) = queue.pop_front() {
            order.push(node);
            for nb in self.neighbors(node) {
                if visited.insert(nb) {
                    queue.push_back(nb);
                }
            }
        }
        order
    }

    /// DFS traversal from `start` (iterative stack); returns visited node ids (no duplicates).
    pub fn dfs(&self, start: u64) -> Vec<u64> {
        let mut visited: std::collections::HashSet<u64> = std::collections::HashSet::new();
        let mut order: Vec<u64> = Vec::new();
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            if !visited.insert(node) {
                continue;
            }
            order.push(node);
            let mut nbs = self.neighbors(node);
            nbs.reverse();
            stack.extend(nbs);
        }
        order
    }

    /// All edges originating from `node`.
    pub fn edges_from(&self, node: u64) -> Vec<&GraphEdge> {
        self.edges.iter().filter(|e| e.from == node).collect()
    }
}

#[cfg(test)]
mod traversal_tests {
    use super::*;

    // Test 1 — is_heavy() true and false
    #[test]
    fn graph_edge_is_heavy() {
        let heavy = GraphEdge::new(1, 2, 2.5, "heavy");
        let light = GraphEdge::new(1, 2, 0.5, "light");
        let boundary = GraphEdge::new(1, 2, 1.0, "boundary");
        assert!(heavy.is_heavy());
        assert!(!light.is_heavy());
        assert!(!boundary.is_heavy());
    }

    // Test 2 — add_edge() and edge_count()
    #[test]
    fn add_edge_and_count() {
        let mut g = SimpleGraph::new();
        assert_eq!(g.edge_count(), 0);
        g.add_edge(GraphEdge::new(1, 2, 1.0, "a"));
        g.add_edge(GraphEdge::new(2, 3, 0.5, "b"));
        assert_eq!(g.edge_count(), 2);
    }

    // Test 3 — neighbors() returns correct destination nodes
    #[test]
    fn neighbors_correct() {
        let mut g = SimpleGraph::new();
        g.add_edge(GraphEdge::new(10, 20, 1.0, ""));
        g.add_edge(GraphEdge::new(10, 30, 1.0, ""));
        g.add_edge(GraphEdge::new(20, 40, 1.0, ""));
        let mut nb = g.neighbors(10);
        nb.sort();
        assert_eq!(nb, vec![20, 30]);
        assert_eq!(g.neighbors(20), vec![40]);
        assert!(g.neighbors(99).is_empty());
    }

    // Test 4 — bfs() visits all reachable nodes
    #[test]
    fn bfs_visits_all_reachable() {
        let mut g = SimpleGraph::new();
        g.add_edge(GraphEdge::new(1, 2, 1.0, ""));
        g.add_edge(GraphEdge::new(1, 3, 1.0, ""));
        g.add_edge(GraphEdge::new(2, 4, 1.0, ""));
        g.add_edge(GraphEdge::new(3, 5, 1.0, ""));
        let visited = g.bfs(1);
        let mut sorted = visited.clone();
        sorted.sort();
        assert_eq!(sorted, vec![1, 2, 3, 4, 5]);
    }

    // Test 5 — bfs() starts at root (first element is start)
    #[test]
    fn bfs_starts_from_root() {
        let mut g = SimpleGraph::new();
        g.add_edge(GraphEdge::new(7, 8, 1.0, ""));
        g.add_edge(GraphEdge::new(8, 9, 1.0, ""));
        let visited = g.bfs(7);
        assert_eq!(visited[0], 7);
    }

    // Test 6 — dfs() visits all reachable nodes
    #[test]
    fn dfs_visits_all_reachable() {
        let mut g = SimpleGraph::new();
        g.add_edge(GraphEdge::new(1, 2, 1.0, ""));
        g.add_edge(GraphEdge::new(2, 3, 1.0, ""));
        g.add_edge(GraphEdge::new(3, 4, 1.0, ""));
        let visited = g.dfs(1);
        let mut sorted = visited.clone();
        sorted.sort();
        assert_eq!(sorted, vec![1, 2, 3, 4]);
    }

    // Test 7 — heavy_edges() filter
    #[test]
    fn heavy_edges_filter() {
        let mut g = SimpleGraph::new();
        g.add_edge(GraphEdge::new(1, 2, 0.5, "light"));
        g.add_edge(GraphEdge::new(2, 3, 3.0, "heavy"));
        g.add_edge(GraphEdge::new(3, 4, 1.0, "boundary"));
        g.add_edge(GraphEdge::new(4, 5, 2.0, "also_heavy"));
        let heavy = g.heavy_edges();
        assert_eq!(heavy.len(), 2);
        assert!(heavy.iter().all(|e| e.is_heavy()));
    }

    // Test 8 — edges_from() filter
    #[test]
    fn edges_from_filter() {
        let mut g = SimpleGraph::new();
        g.add_edge(GraphEdge::new(1, 2, 1.0, "a"));
        g.add_edge(GraphEdge::new(1, 3, 1.0, "b"));
        g.add_edge(GraphEdge::new(2, 4, 1.0, "c"));
        let from1 = g.edges_from(1);
        assert_eq!(from1.len(), 2);
        assert!(from1.iter().all(|e| e.from == 1));
        assert_eq!(g.edges_from(2).len(), 1);
        assert!(g.edges_from(99).is_empty());
    }

    // Test 9 — bfs/dfs on disconnected graph visits only reachable nodes
    #[test]
    fn disconnected_graph_visits_only_reachable() {
        let mut g = SimpleGraph::new();
        // Component A: 1 -> 2 -> 3
        g.add_edge(GraphEdge::new(1, 2, 1.0, ""));
        g.add_edge(GraphEdge::new(2, 3, 1.0, ""));
        // Component B: 10 -> 11 (disconnected)
        g.add_edge(GraphEdge::new(10, 11, 1.0, ""));

        let bfs_visited = g.bfs(1);
        let mut bfs_sorted = bfs_visited.clone();
        bfs_sorted.sort();
        assert_eq!(bfs_sorted, vec![1, 2, 3]);
        assert!(!bfs_visited.contains(&10));
        assert!(!bfs_visited.contains(&11));

        let dfs_visited = g.dfs(1);
        let mut dfs_sorted = dfs_visited.clone();
        dfs_sorted.sort();
        assert_eq!(dfs_sorted, vec![1, 2, 3]);
        assert!(!dfs_visited.contains(&10));
    }
}
