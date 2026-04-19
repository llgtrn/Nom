/// Workflow composition types: NodeType, WorkflowNode, WorkflowEdge, WorkflowGraph,
/// and WorkflowComposer with real logic for building and querying workflow graphs.

#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Trigger,
    Action,
    Condition,
    Loop,
    Output,
}

impl NodeType {
    /// Returns true for control-flow node types (Condition, Loop).
    pub fn is_control_flow(&self) -> bool {
        matches!(self, NodeType::Condition | NodeType::Loop)
    }

    /// Returns the display symbol for the node type.
    pub fn node_symbol(&self) -> &'static str {
        match self {
            NodeType::Trigger => "▶",
            NodeType::Action => "⚡",
            NodeType::Condition => "⟐",
            NodeType::Loop => "↺",
            NodeType::Output => "◀",
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkflowNode {
    pub id: u32,
    pub name: String,
    pub node_type: NodeType,
    pub enabled: bool,
}

impl WorkflowNode {
    /// Returns true when the node is enabled.
    pub fn is_active(&self) -> bool {
        self.enabled
    }

    /// Returns a display string: "{symbol} [{id}] {name}".
    pub fn display(&self) -> String {
        format!("{} [{}] {}", self.node_type.node_symbol(), self.id, self.name)
    }
}

#[derive(Debug, Clone)]
pub struct WorkflowEdge {
    pub from_id: u32,
    pub to_id: u32,
    pub label: String,
}

impl WorkflowEdge {
    /// Returns true when this edge goes from `a` to `b`.
    pub fn connects(&self, a: u32, b: u32) -> bool {
        self.from_id == a && self.to_id == b
    }

    /// Returns a string key formatted as "{from_id}→{to_id}".
    pub fn edge_key(&self) -> String {
        format!("{}→{}", self.from_id, self.to_id)
    }
}

#[derive(Debug, Default)]
pub struct WorkflowGraph {
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
}

impl WorkflowGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: WorkflowNode) {
        self.nodes.push(node);
    }

    pub fn add_edge(&mut self, edge: WorkflowEdge) {
        self.edges.push(edge);
    }

    /// Returns references to all enabled nodes.
    pub fn active_nodes(&self) -> Vec<&WorkflowNode> {
        self.nodes.iter().filter(|n| n.is_active()).collect()
    }

    /// Returns all edges whose `from_id` matches `node_id`.
    pub fn outgoing_edges(&self, node_id: u32) -> Vec<&WorkflowEdge> {
        self.edges.iter().filter(|e| e.from_id == node_id).collect()
    }
}

#[derive(Debug, Default)]
pub struct WorkflowComposer {
    pub graph: WorkflowGraph,
}

impl WorkflowComposer {
    pub fn new() -> Self {
        Self {
            graph: WorkflowGraph::new(),
        }
    }

    pub fn add_node(&mut self, node: WorkflowNode) {
        self.graph.add_node(node);
    }

    pub fn add_edge(&mut self, edge: WorkflowEdge) {
        self.graph.add_edge(edge);
    }

    /// Returns a summary string: "{total} nodes ({active} active), {edges} edges".
    pub fn summary(&self) -> String {
        let total = self.graph.nodes.len();
        let active = self.graph.active_nodes().len();
        let edges = self.graph.edges.len();
        format!("{} nodes ({} active), {} edges", total, active, edges)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_type_is_control_flow() {
        assert!(NodeType::Condition.is_control_flow());
        assert!(NodeType::Loop.is_control_flow());
        assert!(!NodeType::Trigger.is_control_flow());
        assert!(!NodeType::Action.is_control_flow());
        assert!(!NodeType::Output.is_control_flow());
    }

    #[test]
    fn node_type_node_symbol() {
        assert_eq!(NodeType::Trigger.node_symbol(), "▶");
        assert_eq!(NodeType::Action.node_symbol(), "⚡");
        assert_eq!(NodeType::Condition.node_symbol(), "⟐");
        assert_eq!(NodeType::Loop.node_symbol(), "↺");
        assert_eq!(NodeType::Output.node_symbol(), "◀");
    }

    #[test]
    fn workflow_node_is_active() {
        let active = WorkflowNode {
            id: 1,
            name: "start".to_string(),
            node_type: NodeType::Trigger,
            enabled: true,
        };
        let inactive = WorkflowNode {
            id: 2,
            name: "skipped".to_string(),
            node_type: NodeType::Action,
            enabled: false,
        };
        assert!(active.is_active());
        assert!(!inactive.is_active());
    }

    #[test]
    fn workflow_node_display_format() {
        let node = WorkflowNode {
            id: 42,
            name: "send-email".to_string(),
            node_type: NodeType::Action,
            enabled: true,
        };
        assert_eq!(node.display(), "⚡ [42] send-email");
    }

    #[test]
    fn workflow_edge_connects() {
        let edge = WorkflowEdge {
            from_id: 1,
            to_id: 2,
            label: "ok".to_string(),
        };
        assert!(edge.connects(1, 2));
        assert!(!edge.connects(2, 1));
        assert!(!edge.connects(1, 3));
    }

    #[test]
    fn workflow_edge_key_format() {
        let edge = WorkflowEdge {
            from_id: 5,
            to_id: 9,
            label: "branch".to_string(),
        };
        assert_eq!(edge.edge_key(), "5→9");
    }

    #[test]
    fn workflow_graph_active_nodes_filter() {
        let mut graph = WorkflowGraph::new();
        graph.add_node(WorkflowNode { id: 1, name: "a".to_string(), node_type: NodeType::Trigger, enabled: true });
        graph.add_node(WorkflowNode { id: 2, name: "b".to_string(), node_type: NodeType::Action, enabled: false });
        graph.add_node(WorkflowNode { id: 3, name: "c".to_string(), node_type: NodeType::Output, enabled: true });

        let active = graph.active_nodes();
        assert_eq!(active.len(), 2);
        assert!(active.iter().all(|n| n.enabled));
    }

    #[test]
    fn workflow_graph_outgoing_edges() {
        let mut graph = WorkflowGraph::new();
        graph.add_edge(WorkflowEdge { from_id: 1, to_id: 2, label: "next".to_string() });
        graph.add_edge(WorkflowEdge { from_id: 1, to_id: 3, label: "branch".to_string() });
        graph.add_edge(WorkflowEdge { from_id: 2, to_id: 3, label: "done".to_string() });

        let out = graph.outgoing_edges(1);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|e| e.from_id == 1));

        let out2 = graph.outgoing_edges(2);
        assert_eq!(out2.len(), 1);
        assert_eq!(out2[0].to_id, 3);
    }

    #[test]
    fn workflow_composer_summary_format() {
        let mut composer = WorkflowComposer::new();
        composer.add_node(WorkflowNode { id: 1, name: "trigger".to_string(), node_type: NodeType::Trigger, enabled: true });
        composer.add_node(WorkflowNode { id: 2, name: "action".to_string(), node_type: NodeType::Action, enabled: true });
        composer.add_node(WorkflowNode { id: 3, name: "disabled".to_string(), node_type: NodeType::Output, enabled: false });
        composer.add_edge(WorkflowEdge { from_id: 1, to_id: 2, label: "go".to_string() });
        composer.add_edge(WorkflowEdge { from_id: 2, to_id: 3, label: "finish".to_string() });

        assert_eq!(composer.summary(), "3 nodes (2 active), 2 edges");
    }
}
