#![deny(unsafe_code)]
use serde::{Deserialize, Serialize};
use crate::graph_node::NodeId;

pub type ConnectorId = String;
pub type SlotName = String;

/// A wire between two graph nodes. can_wire_result is NON-OPTIONAL.
/// In Wave B: populated with stub. In Wave C: real can_wire() from grammar.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Connector {
    pub id: ConnectorId,
    pub src: (NodeId, SlotName),
    pub dst: (NodeId, SlotName),
    pub confidence: f32,
    pub reason: String,
    pub route: Vec<[f32; 2]>,
    /// (is_valid, confidence, reason) — NON-OPTIONAL, stub in Wave B
    pub can_wire_result: (bool, f32, String),
}

impl Connector {
    pub fn new_stub(
        id: impl Into<String>,
        src_node: impl Into<String>,
        src_slot: impl Into<String>,
        dst_node: impl Into<String>,
        dst_slot: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            src: (src_node.into(), src_slot.into()),
            dst: (dst_node.into(), dst_slot.into()),
            confidence: 0.0,
            reason: String::new(),
            route: Vec::new(),
            can_wire_result: (true, 0.0, "stub - pending Wave C validation".into()),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.can_wire_result.0
    }

    /// Auto-route: straight line from src to dst with 2 bezier control points
    pub fn auto_route(&mut self, src_pos: [f32; 2], dst_pos: [f32; 2]) {
        let mid_x = (src_pos[0] + dst_pos[0]) / 2.0;
        self.route = vec![
            src_pos,
            [mid_x, src_pos[1]],
            [mid_x, dst_pos[1]],
            dst_pos,
        ];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connector_stub_is_valid() {
        let c = Connector::new_stub("c1", "n1", "output", "n2", "input");
        assert!(c.is_valid());
        assert!(c.can_wire_result.2.contains("stub"));
    }

    #[test]
    fn connector_auto_route() {
        let mut c = Connector::new_stub("c1", "n1", "out", "n2", "in");
        c.auto_route([0.0, 50.0], [200.0, 100.0]);
        assert_eq!(c.route.len(), 4);
        assert_eq!(c.route[0], [0.0, 50.0]);
        assert_eq!(c.route[3], [200.0, 100.0]);
    }
}
