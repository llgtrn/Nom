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
