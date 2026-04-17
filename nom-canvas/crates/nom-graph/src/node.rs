#![deny(unsafe_code)]

pub type NodeId = String;
pub type PortId = String;

#[derive(Clone, Debug)]
pub enum PortDirection { Input, Output }

#[derive(Clone, Debug)]
pub struct Port {
    pub id: PortId,
    pub name: String,
    pub direction: PortDirection,
    pub required: bool,
    pub connected: bool,
}

#[derive(Clone, Debug)]
pub enum NodeState { Idle, Queued, Running, Completed, Error(String), Cached }

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
    Always,           // always re-execute (e.g., random seed nodes)
    HashInput,        // re-execute when inputs change (default)
    Never,            // never re-execute (pure functions with same inputs)
}

impl Default for IsChanged { fn default() -> Self { Self::HashInput } }

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
        self.ports.iter().filter(|p| matches!(p.direction, PortDirection::Input)).collect()
    }
    pub fn output_ports(&self) -> Vec<&Port> {
        self.ports.iter().filter(|p| matches!(p.direction, PortDirection::Output)).collect()
    }
    pub fn is_ready(&self) -> bool {
        self.input_ports().iter().all(|p| !p.required || p.connected)
    }
}
