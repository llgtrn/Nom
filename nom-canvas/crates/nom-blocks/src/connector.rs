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
    /// Clamped to [0.0, 1.0]. Represents confidence that this wire is valid.
    pub confidence: f32,
    pub reason: String,
    /// Ordered reasoning steps that led to this connection (spec: "reason chain").
    pub reason_chain: Vec<String>,
    pub route: Vec<[f32; 2]>,
    /// (is_valid, confidence, reason) — NON-OPTIONAL, stub in Wave B
    pub can_wire_result: (bool, f32, String),
}

impl Connector {
    /// Construct a connector from two node IDs with a confidence score.
    /// Confidence is clamped to [0.0, 1.0]. Slots default to empty strings.
    pub fn new(from: impl Into<NodeId>, to: impl Into<NodeId>, confidence: f32) -> Self {
        Self {
            id: String::new(),
            src: (from.into(), String::new()),
            dst: (to.into(), String::new()),
            confidence: confidence.clamp(0.0, 1.0),
            reason: String::new(),
            reason_chain: Vec::new(),
            route: Vec::new(),
            can_wire_result: (true, confidence.clamp(0.0, 1.0), String::new()),
        }
    }

    /// Append a reasoning step and return self (builder pattern).
    pub fn with_reason(mut self, reason: String) -> Self {
        self.reason_chain.push(reason);
        self
    }

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
            reason_chain: Vec::new(),
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

    #[test]
    fn connector_confidence_clamps_to_one() {
        let over = Connector::new("n1", "n2", 1.5);
        assert_eq!(over.confidence, 1.0);

        let under = Connector::new("n1", "n2", -0.3);
        assert_eq!(under.confidence, 0.0);

        let mid = Connector::new("n1", "n2", 0.75);
        assert!((mid.confidence - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn connector_reason_chain_accumulates() {
        let mut c = Connector::new("a", "b", 0.9);
        c.reason_chain.push("step one".into());
        c.reason_chain.push("step two".into());
        assert_eq!(c.reason_chain.len(), 2);
        assert_eq!(c.reason_chain[0], "step one");
        assert_eq!(c.reason_chain[1], "step two");
    }

    #[test]
    fn connector_with_reason_builder() {
        let c = Connector::new("x", "y", 0.6)
            .with_reason("grammar matched".into())
            .with_reason("type aligned".into());
        assert_eq!(c.reason_chain.len(), 2);
        assert_eq!(c.reason_chain[0], "grammar matched");
        assert_eq!(c.reason_chain[1], "type aligned");
    }
}
