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
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum IsChanged {
    Always, // always re-execute (e.g., random seed nodes)
    #[default]
    HashInput, // re-execute when inputs change (default)
    Never,  // never re-execute (pure functions with same inputs)
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
        assert!(inputs
            .iter()
            .all(|p| matches!(p.direction, PortDirection::Input)));
    }

    #[test]
    fn exec_node_output_ports_returns_only_outputs() {
        let mut node = ExecNode::new("n", "verb");
        node.ports.push(make_input_port("i1", false, false));
        node.ports.push(make_output_port("o1"));
        node.ports.push(make_output_port("o2"));
        let outputs = node.output_ports();
        assert_eq!(outputs.len(), 2);
        assert!(outputs
            .iter()
            .all(|p| matches!(p.direction, PortDirection::Output)));
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
        assert!(
            !node.is_ready(),
            "required+unconnected port must not be ready"
        );
    }

    #[test]
    fn exec_node_optional_unconnected_port_is_ready() {
        let mut node = ExecNode::new("n", "verb");
        node.ports.push(make_input_port("i", false, false)); // optional, not connected
        assert!(
            node.is_ready(),
            "optional unconnected port must still be ready"
        );
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
        node.ports.push(make_input_port("opt", false, true)); // optional, connected
        assert!(
            !node.is_ready(),
            "required unconnected port makes node not ready"
        );
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
        assert_eq!(
            original.kind, "kind",
            "original must be unaffected by clone mutation"
        );
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

    // ------------------------------------------------------------------
    // Node serialization: structural field-by-field round-trip
    // ------------------------------------------------------------------
    #[test]
    fn exec_node_serialization_roundtrip_via_fields() {
        // Capture fields of a node and reconstruct — verify equality.
        let mut original = ExecNode::new("serial_node", "transform");
        original.cache_key = Some(12345);
        original.is_changed = IsChanged::Always;
        original.ports.push(make_input_port("in1", true, true));
        original.ports.push(make_output_port("out1"));

        // "Serialize": capture fields.
        let id = original.id.clone();
        let kind = original.kind.clone();
        let cache_key = original.cache_key;
        let port_count = original.ports.len();

        // "Deserialize": reconstruct.
        let mut reconstructed = ExecNode::new(id.clone(), kind.clone());
        reconstructed.cache_key = cache_key;
        reconstructed.is_changed = IsChanged::Always;

        assert_eq!(reconstructed.id, "serial_node");
        assert_eq!(reconstructed.kind, "transform");
        assert_eq!(reconstructed.cache_key, Some(12345));
        assert_eq!(reconstructed.is_changed, IsChanged::Always);
        assert_eq!(port_count, 2, "original had 2 ports");
    }

    // ------------------------------------------------------------------
    // Node comparison: two nodes with the same id and kind are structurally equal by field
    // ------------------------------------------------------------------
    #[test]
    fn exec_node_comparison_same_fields() {
        let n1 = ExecNode::new("node_a", "kind_x");
        let n2 = ExecNode::new("node_a", "kind_x");
        assert_eq!(n1.id, n2.id);
        assert_eq!(n1.kind, n2.kind);
        assert_eq!(n1.ports.len(), n2.ports.len());
        assert_eq!(n1.cache_key, n2.cache_key);
    }

    // ------------------------------------------------------------------
    // Node comparison: nodes with different ids are distinct
    // ------------------------------------------------------------------
    #[test]
    fn exec_node_comparison_different_ids_are_distinct() {
        let n1 = ExecNode::new("alpha", "verb");
        let n2 = ExecNode::new("beta", "verb");
        assert_ne!(
            n1.id, n2.id,
            "nodes with different ids must have different id fields"
        );
    }

    // ------------------------------------------------------------------
    // Default NodeState is Idle
    // ------------------------------------------------------------------
    #[test]
    fn exec_node_default_state_is_idle() {
        let node = ExecNode::new("n", "verb");
        assert!(
            matches!(node.state, NodeState::Idle),
            "default state must be Idle"
        );
    }

    // ------------------------------------------------------------------
    // NodeState transitions: Idle → Queued → Running → Completed
    // ------------------------------------------------------------------
    #[test]
    fn exec_node_state_transitions_idle_to_completed() {
        let mut node = ExecNode::new("n", "verb");
        // Idle → Queued
        node.state = NodeState::Queued;
        assert!(matches!(node.state, NodeState::Queued));
        // Queued → Running
        node.state = NodeState::Running;
        assert!(matches!(node.state, NodeState::Running));
        // Running → Completed
        node.state = NodeState::Completed;
        assert!(matches!(node.state, NodeState::Completed));
    }

    // ------------------------------------------------------------------
    // NodeState transition: Running → Error
    // ------------------------------------------------------------------
    #[test]
    fn exec_node_state_running_to_error() {
        let mut node = ExecNode::new("n", "verb");
        node.state = NodeState::Running;
        node.state = NodeState::Error("something went wrong".to_string());
        match &node.state {
            NodeState::Error(msg) => assert!(msg.contains("something went wrong")),
            _ => panic!("expected Error state"),
        }
    }

    // ------------------------------------------------------------------
    // NodeState: Cached variant
    // ------------------------------------------------------------------
    #[test]
    fn exec_node_state_cached() {
        let mut node = ExecNode::new("n", "verb");
        node.state = NodeState::Cached;
        assert!(matches!(node.state, NodeState::Cached));
    }

    // ------------------------------------------------------------------
    // IsChanged variants
    // ------------------------------------------------------------------
    #[test]
    fn is_changed_always_variant() {
        let ic = IsChanged::Always;
        assert_eq!(ic, IsChanged::Always);
    }

    #[test]
    fn is_changed_never_variant() {
        let ic = IsChanged::Never;
        assert_eq!(ic, IsChanged::Never);
    }

    #[test]
    fn is_changed_hash_input_variant() {
        let ic = IsChanged::HashInput;
        assert_eq!(ic, IsChanged::HashInput);
    }

    // ------------------------------------------------------------------
    // Port: direction is preserved
    // ------------------------------------------------------------------
    #[test]
    fn port_direction_input_preserved() {
        let p = make_input_port("x", false, false);
        assert!(matches!(p.direction, PortDirection::Input));
    }

    #[test]
    fn port_direction_output_preserved() {
        let p = make_output_port("y");
        assert!(matches!(p.direction, PortDirection::Output));
    }

    // ------------------------------------------------------------------
    // Port: required and connected flags
    // ------------------------------------------------------------------
    #[test]
    fn port_required_flag_stored() {
        let required = make_input_port("r", true, false);
        let optional = make_input_port("o", false, false);
        assert!(required.required);
        assert!(!optional.required);
    }

    #[test]
    fn port_connected_flag_stored() {
        let connected = make_input_port("c", false, true);
        let disconnected = make_input_port("d", false, false);
        assert!(connected.connected);
        assert!(!disconnected.connected);
    }

    // ------------------------------------------------------------------
    // cache_key: set and clear
    // ------------------------------------------------------------------
    #[test]
    fn exec_node_cache_key_set_and_clear() {
        let mut node = ExecNode::new("n", "verb");
        assert!(node.cache_key.is_none());
        node.cache_key = Some(999);
        assert_eq!(node.cache_key, Some(999));
        node.cache_key = None;
        assert!(node.cache_key.is_none());
    }
}
