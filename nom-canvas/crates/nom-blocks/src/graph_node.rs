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
}
