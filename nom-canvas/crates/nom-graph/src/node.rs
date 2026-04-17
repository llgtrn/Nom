#![deny(unsafe_code)]

pub type NodeId = String;
pub type PortId = String;

#[derive(Clone, Debug)]
pub enum PortDirection {
    Input,
    Output,
}

#[derive(Clone, Debug)]
pub struct Port {
    pub id: PortId,
    pub name: String,
    pub direction: PortDirection,
    pub required: bool,
    pub connected: bool,
}

#[derive(Clone, Debug)]
pub enum NodeState {
    Idle,
    Queued,
    Running,
    Completed,
    Error(String),
    Cached,
}

/// Execution node in the DAG
#[derive(Clone, Debug)]
pub struct ExecNode {
    pub id: NodeId,
    pub kind: String,
    pub ports: Vec<Port>,
    pub state: NodeState,
    pub cache_key: Option<u64>,
    pub is_changed: IsChanged,
}

/// ComfyUI IS_CHANGED hierarchy
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IsChanged {
    Always,    // always re-execute (e.g., random seed nodes)
    HashInput, // re-execute when inputs change (default)
    Never,     // never re-execute (pure functions with same inputs)
}

impl Default for IsChanged {
    fn default() -> Self {
        Self::HashInput
    }
}

impl ExecNode {
    pub fn new(id: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            ports: Vec::new(),
            state: NodeState::Idle,
            cache_key: None,
            is_changed: IsChanged::default(),
        }
    }
    pub fn input_ports(&self) -> Vec<&Port> {
        self.ports
            .iter()
            .filter(|p| matches!(p.direction, PortDirection::Input))
            .collect()
    }
    pub fn output_ports(&self) -> Vec<&Port> {
        self.ports
            .iter()
            .filter(|p| matches!(p.direction, PortDirection::Output))
            .collect()
    }
    pub fn is_ready(&self) -> bool {
        self.input_ports()
            .iter()
            .all(|p| !p.required || p.connected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_input_port(id: &str, required: bool, connected: bool) -> Port {
        Port {
            id: id.to_string(),
            name: id.to_string(),
            direction: PortDirection::Input,
            required,
            connected,
        }
    }

    fn make_output_port(id: &str) -> Port {
        Port {
            id: id.to_string(),
            name: id.to_string(),
            direction: PortDirection::Output,
            required: false,
            connected: false,
        }
    }

    // ------------------------------------------------------------------
    // ExecNode::new defaults
    // ------------------------------------------------------------------
    #[test]
    fn exec_node_new_defaults() {
        let node = ExecNode::new("n1", "verb");
        assert_eq!(node.id, "n1");
        assert_eq!(node.kind, "verb");
        assert!(node.ports.is_empty());
        assert!(node.cache_key.is_none());
        assert_eq!(node.is_changed, IsChanged::HashInput);
    }

    // ------------------------------------------------------------------
    // Port filtering
    // ------------------------------------------------------------------
    #[test]
    fn exec_node_input_ports_returns_only_inputs() {
        let mut node = ExecNode::new("n", "verb");
        node.ports.push(make_input_port("i1", true, false));
        node.ports.push(make_output_port("o1"));
        node.ports.push(make_input_port("i2", false, true));
        let inputs = node.input_ports();
        assert_eq!(inputs.len(), 2);
        assert!(inputs.iter().all(|p| matches!(p.direction, PortDirection::Input)));
    }

    #[test]
    fn exec_node_output_ports_returns_only_outputs() {
        let mut node = ExecNode::new("n", "verb");
        node.ports.push(make_input_port("i1", false, false));
        node.ports.push(make_output_port("o1"));
        node.ports.push(make_output_port("o2"));
        let outputs = node.output_ports();
        assert_eq!(outputs.len(), 2);
        assert!(outputs.iter().all(|p| matches!(p.direction, PortDirection::Output)));
    }

    #[test]
    fn exec_node_no_ports_is_ready() {
        let node = ExecNode::new("n", "verb");
        assert!(node.is_ready(), "node with no ports must be ready");
    }

    #[test]
    fn exec_node_required_connected_port_is_ready() {
        let mut node = ExecNode::new("n", "verb");
        node.ports.push(make_input_port("i", true, true));
        assert!(node.is_ready(), "required+connected port must be ready");
    }

    #[test]
    fn exec_node_required_unconnected_port_not_ready() {
        let mut node = ExecNode::new("n", "verb");
        node.ports.push(make_input_port("i", true, false));
        assert!(!node.is_ready(), "required+unconnected port must not be ready");
    }

    #[test]
    fn exec_node_optional_unconnected_port_is_ready() {
        let mut node = ExecNode::new("n", "verb");
        node.ports.push(make_input_port("i", false, false)); // optional, not connected
        assert!(node.is_ready(), "optional unconnected port must still be ready");
    }

    #[test]
    fn exec_node_mixed_ports_ready_when_all_required_connected() {
        let mut node = ExecNode::new("n", "verb");
        node.ports.push(make_input_port("req", true, true)); // required, connected
        node.ports.push(make_input_port("opt", false, false)); // optional, not connected
        node.ports.push(make_output_port("out"));
        assert!(node.is_ready(), "all required ports connected means ready");
    }

    #[test]
    fn exec_node_mixed_ports_not_ready_when_required_disconnected() {
        let mut node = ExecNode::new("n", "verb");
        node.ports.push(make_input_port("req", true, false)); // required, NOT connected
        node.ports.push(make_input_port("opt", false, true));  // optional, connected
        assert!(!node.is_ready(), "required unconnected port makes node not ready");
    }

    // ------------------------------------------------------------------
    // IsChanged default
    // ------------------------------------------------------------------
    #[test]
    fn is_changed_default_is_hash_input() {
        assert_eq!(IsChanged::default(), IsChanged::HashInput);
    }

    // ------------------------------------------------------------------
    // NodeState variants
    // ------------------------------------------------------------------
    #[test]
    fn node_state_error_carries_message() {
        let state = NodeState::Error("disk full".to_string());
        match state {
            NodeState::Error(msg) => assert_eq!(msg, "disk full"),
            _ => panic!("expected Error variant"),
        }
    }

    // ------------------------------------------------------------------
    // ExecNode clone correctness
    // ------------------------------------------------------------------
    #[test]
    fn exec_node_clone_is_independent() {
        let original = ExecNode::new("orig", "kind");
        let mut cloned = original.clone();
        cloned.kind = "mutated".to_string();
        assert_eq!(original.kind, "kind", "original must be unaffected by clone mutation");
        assert_eq!(cloned.kind, "mutated");
    }

    #[test]
    fn exec_node_clone_preserves_ports() {
        let mut original = ExecNode::new("n", "verb");
        original.ports.push(make_input_port("p1", true, false));
        let cloned = original.clone();
        assert_eq!(cloned.ports.len(), 1);
        assert_eq!(cloned.ports[0].id, "p1");
    }
}
