#![deny(unsafe_code)]
use serde::{Deserialize, Serialize};
use crate::block_model::NomtuRef;
use crate::slot::SlotBinding;

pub type NodeId = String;

/// Canvas graph node — production_kind validated against grammar.kinds at insert
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: NodeId,
    pub entity: NomtuRef,
    pub production_kind: String,   // validated via DictReader::is_known_kind, never a Rust enum
    pub slots: Vec<SlotBinding>,   // derived from clause_shapes query
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub collapsed: bool,
}

impl GraphNode {
    pub fn new(id: impl Into<String>, entity: NomtuRef, production_kind: impl Into<String>, position: [f32; 2]) -> Self {
        Self {
            id: id.into(),
            entity,
            production_kind: production_kind.into(),
            slots: Vec::new(),
            position,
            size: [200.0, 120.0],
            collapsed: false,
        }
    }

    /// Input slots: first half of slots list (left edge ports)
    pub fn input_slots(&self) -> Vec<&SlotBinding> {
        let mid = self.slots.len() / 2;
        self.slots[..mid].iter().collect()
    }

    /// Output slots: second half of slots list (right edge ports)
    pub fn output_slots(&self) -> Vec<&SlotBinding> {
        let mid = self.slots.len() / 2;
        self.slots[mid..].iter().collect()
    }

    /// Port y-position for slot at index i (evenly spaced within node height)
    pub fn port_y(&self, index: usize, total: usize) -> f32 {
        if total == 0 { return self.position[1] + self.size[1] / 2.0; }
        let spacing = self.size[1] / (total + 1) as f32;
        self.position[1] + spacing * (index + 1) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_node_port_y() {
        let node = GraphNode::new("n1", NomtuRef::new("id", "w", "verb"), "verb", [0.0, 0.0]);
        // With 3 slots total: spacing = 120/4 = 30, port at index 0 = 30
        assert!((node.port_y(0, 3) - 30.0).abs() < 0.01);
        assert!((node.port_y(1, 3) - 60.0).abs() < 0.01);
        assert!((node.port_y(2, 3) - 90.0).abs() < 0.01);
    }

    #[test]
    fn graph_node_id_unique() {
        let n1 = GraphNode::new("node-a", NomtuRef::new("e1", "fetch", "verb"), "verb", [0.0, 0.0]);
        let n2 = GraphNode::new("node-b", NomtuRef::new("e2", "store", "verb"), "verb", [100.0, 0.0]);
        assert_ne!(n1.id, n2.id);
    }

    #[test]
    fn graph_node_input_output_ports() {
        let mut node = GraphNode::new("n1", NomtuRef::new("e1", "transform", "verb"), "verb", [0.0, 0.0]);
        // Add 4 slots: first 2 = inputs, last 2 = outputs
        node.slots.push(SlotBinding::explicit("in1", "text", crate::slot::SlotValue::Bool(false)));
        node.slots.push(SlotBinding::explicit("in2", "number", crate::slot::SlotValue::Number(0.0)));
        node.slots.push(SlotBinding::explicit("out1", "text", crate::slot::SlotValue::Bool(false)));
        node.slots.push(SlotBinding::explicit("out2", "number", crate::slot::SlotValue::Number(1.0)));

        let inputs = node.input_slots();
        let outputs = node.output_slots();
        assert_eq!(inputs.len(), 2);
        assert_eq!(outputs.len(), 2);
        assert_eq!(inputs[0].clause_name, "in1");
        assert_eq!(outputs[0].clause_name, "out1");
    }

    #[test]
    fn graph_node_ports_from_clause_shapes() {
        let mut node = GraphNode::new("n2", NomtuRef::new("e2", "plan", "concept"), "concept", [0.0, 0.0]);
        node.slots.push(SlotBinding::inferred("slot-a", "prose", crate::slot::SlotValue::Text("x".into())));
        node.slots.push(SlotBinding::inferred("slot-b", "prose", crate::slot::SlotValue::Text("y".into())));
        // 2 clause shapes → 2 slots → 1 input + 1 output (half-split)
        assert_eq!(node.slots.len(), 2);
        assert_eq!(node.input_slots().len(), 1);
        assert_eq!(node.output_slots().len(), 1);
    }

    #[test]
    fn graph_node_port_y_zero_total_returns_midpoint() {
        let node = GraphNode::new("n3", NomtuRef::new("e3", "w", "verb"), "verb", [0.0, 10.0]);
        // total=0: returns position[1] + size[1]/2 = 10.0 + 60.0 = 70.0
        assert!((node.port_y(0, 0) - 70.0).abs() < 0.01);
    }

    #[test]
    fn graph_node_default_size() {
        let node = GraphNode::new("n4", NomtuRef::new("e4", "w", "verb"), "verb", [50.0, 50.0]);
        assert_eq!(node.size, [200.0, 120.0]);
        assert!(!node.collapsed);
    }
}
