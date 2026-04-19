/// DAG workflow runner with topological execution.

// ---------------------------------------------------------------------------
// NodeStatus
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeStatus {
    Pending,
    Running,
    Success,
    Failed,
}

impl NodeStatus {
    /// Returns `true` if the status is terminal (Success or Failed).
    pub fn is_terminal(&self) -> bool {
        matches!(self, NodeStatus::Success | NodeStatus::Failed)
    }
}

// ---------------------------------------------------------------------------
// WorkflowNode
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct WorkflowNode {
    pub id: u64,
    pub name: String,
    pub status: NodeStatus,
}

impl WorkflowNode {
    pub fn new(id: u64, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            status: NodeStatus::Pending,
        }
    }

    pub fn mark_success(&mut self) {
        self.status = NodeStatus::Success;
    }

    pub fn mark_failed(&mut self) {
        self.status = NodeStatus::Failed;
    }

    /// Returns `true` when the node has not yet been executed.
    pub fn is_ready(&self) -> bool {
        self.status == NodeStatus::Pending
    }
}

// ---------------------------------------------------------------------------
// WorkflowGraph
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct WorkflowGraph {
    pub nodes: Vec<WorkflowNode>,
    /// Directed edges: (from_id, to_id) means `from` must execute before `to`.
    pub edges: Vec<(u64, u64)>,
}

impl WorkflowGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: WorkflowNode) {
        self.nodes.push(node);
    }

    pub fn add_edge(&mut self, from: u64, to: u64) {
        self.edges.push((from, to));
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// All predecessor node IDs for `node_id` (i.e. edges where to_id == node_id).
    pub fn dependencies_of(&self, node_id: u64) -> Vec<u64> {
        self.edges
            .iter()
            .filter(|(_, to)| *to == node_id)
            .map(|(from, _)| *from)
            .collect()
    }

    /// Topological order via Kahn's algorithm.
    /// Ties broken by insertion order (index in `self.nodes`).
    pub fn execution_order(&self) -> Vec<u64> {
        use std::collections::{HashMap, VecDeque};

        // Build in-degree map keyed by node id.
        let mut in_degree: HashMap<u64, usize> = HashMap::new();
        for node in &self.nodes {
            in_degree.entry(node.id).or_insert(0);
        }
        for (_, to) in &self.edges {
            *in_degree.entry(*to).or_insert(0) += 1;
        }

        // Adjacency list: from_id → [to_id, ...]
        let mut adj: HashMap<u64, Vec<u64>> = HashMap::new();
        for node in &self.nodes {
            adj.entry(node.id).or_default();
        }
        for (from, to) in &self.edges {
            adj.entry(*from).or_default().push(*to);
        }

        // Seed queue with zero-in-degree nodes in insertion order.
        let mut queue: VecDeque<u64> = self
            .nodes
            .iter()
            .filter(|n| in_degree[&n.id] == 0)
            .map(|n| n.id)
            .collect();

        let mut order = Vec::with_capacity(self.nodes.len());
        while let Some(id) = queue.pop_front() {
            order.push(id);
            // Visit successors in insertion order.
            if let Some(succs) = adj.get(&id) {
                // Collect and sort by insertion index to maintain determinism.
                let mut succs_ordered: Vec<u64> = succs.clone();
                succs_ordered.sort_by_key(|sid| {
                    self.nodes.iter().position(|n| n.id == *sid).unwrap_or(usize::MAX)
                });
                for succ in succs_ordered {
                    let deg = in_degree.get_mut(&succ).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(succ);
                    }
                }
            }
        }

        order
    }
}

// ---------------------------------------------------------------------------
// WorkflowRunner
// ---------------------------------------------------------------------------

pub struct WorkflowRunner {
    pub graph: WorkflowGraph,
    pub execution_log: Vec<String>,
}

impl WorkflowRunner {
    pub fn new(graph: WorkflowGraph) -> Self {
        Self {
            graph,
            execution_log: Vec::new(),
        }
    }

    /// Execute a single node by id: log it and mark success.
    pub fn run_node(&mut self, node_id: u64) {
        if let Some(node) = self.graph.nodes.iter_mut().find(|n| n.id == node_id) {
            let msg = format!("executed: {}", node.name);
            self.execution_log.push(msg);
            node.mark_success();
        }
    }

    /// Execute all nodes in topological order.
    pub fn run_all(&mut self) {
        let order = self.graph.execution_order();
        for id in order {
            self.run_node(id);
        }
    }

    /// Count of nodes whose status is Success.
    pub fn success_count(&self) -> usize {
        self.graph
            .nodes
            .iter()
            .filter(|n| n.status == NodeStatus::Success)
            .count()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod workflow_runner_tests {
    use super::*;

    #[test]
    fn node_status_is_terminal_success() {
        assert!(NodeStatus::Success.is_terminal());
        assert!(NodeStatus::Failed.is_terminal());
    }

    #[test]
    fn node_status_is_terminal_pending_false() {
        assert!(!NodeStatus::Pending.is_terminal());
        assert!(!NodeStatus::Running.is_terminal());
    }

    #[test]
    fn workflow_node_mark_success() {
        let mut node = WorkflowNode::new(1, "alpha");
        assert!(node.is_ready());
        node.mark_success();
        assert_eq!(node.status, NodeStatus::Success);
        assert!(!node.is_ready());
    }

    #[test]
    fn workflow_graph_add_node_and_edge() {
        let mut g = WorkflowGraph::new();
        g.add_node(WorkflowNode::new(1, "a"));
        g.add_node(WorkflowNode::new(2, "b"));
        g.add_edge(1, 2);
        assert_eq!(g.node_count(), 2);
        assert_eq!(g.edge_count(), 1);
    }

    #[test]
    fn workflow_graph_dependencies_of() {
        let mut g = WorkflowGraph::new();
        g.add_node(WorkflowNode::new(1, "root"));
        g.add_node(WorkflowNode::new(2, "child"));
        g.add_node(WorkflowNode::new(3, "child2"));
        g.add_edge(1, 2);
        g.add_edge(1, 3);
        // node 2 depends on node 1
        let deps = g.dependencies_of(2);
        assert_eq!(deps, vec![1]);
        // node 1 has no dependencies
        let deps_root = g.dependencies_of(1);
        assert!(deps_root.is_empty());
    }

    #[test]
    fn workflow_graph_execution_order_respects_deps() {
        // Graph: 1 → 2 → 3, plus 1 → 4
        let mut g = WorkflowGraph::new();
        g.add_node(WorkflowNode::new(1, "start"));
        g.add_node(WorkflowNode::new(2, "middle"));
        g.add_node(WorkflowNode::new(3, "end"));
        g.add_node(WorkflowNode::new(4, "branch"));
        g.add_edge(1, 2);
        g.add_edge(2, 3);
        g.add_edge(1, 4);

        let order = g.execution_order();
        assert_eq!(order.len(), 4);

        let pos = |id: u64| order.iter().position(|&x| x == id).unwrap();
        // 1 must come before 2, 2 before 3, 1 before 4
        assert!(pos(1) < pos(2));
        assert!(pos(2) < pos(3));
        assert!(pos(1) < pos(4));
    }

    #[test]
    fn workflow_runner_run_node_logs() {
        let mut g = WorkflowGraph::new();
        g.add_node(WorkflowNode::new(10, "fetch-data"));
        let mut runner = WorkflowRunner::new(g);
        runner.run_node(10);
        assert_eq!(runner.execution_log, vec!["executed: fetch-data"]);
        assert_eq!(runner.graph.nodes[0].status, NodeStatus::Success);
    }

    #[test]
    fn workflow_runner_run_all_success_count() {
        let mut g = WorkflowGraph::new();
        g.add_node(WorkflowNode::new(1, "a"));
        g.add_node(WorkflowNode::new(2, "b"));
        g.add_node(WorkflowNode::new(3, "c"));
        g.add_edge(1, 2);
        g.add_edge(2, 3);
        let mut runner = WorkflowRunner::new(g);
        runner.run_all();
        assert_eq!(runner.success_count(), 3);
    }

    #[test]
    fn workflow_runner_run_all_order() {
        // Ensure log reflects topological order: 1 → 2 → 3
        let mut g = WorkflowGraph::new();
        g.add_node(WorkflowNode::new(1, "first"));
        g.add_node(WorkflowNode::new(2, "second"));
        g.add_node(WorkflowNode::new(3, "third"));
        g.add_edge(1, 2);
        g.add_edge(2, 3);
        let mut runner = WorkflowRunner::new(g);
        runner.run_all();
        assert_eq!(
            runner.execution_log,
            vec!["executed: first", "executed: second", "executed: third"]
        );
    }
}
