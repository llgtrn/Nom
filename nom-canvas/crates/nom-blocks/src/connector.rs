#![deny(unsafe_code)]
use serde::{Deserialize, Serialize};
use crate::graph_node::NodeId;
use crate::dict_reader::DictReader;

pub type ConnectorId = String;
pub type SlotName = String;

/// A wire between two graph nodes. can_wire_result is NON-OPTIONAL.
/// Grammar-backed validation added in Wave Q — use new_with_validation() for real port/type checking.
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
            can_wire_result: (true, 0.0, "stub — use new_with_validation() for grammar-backed checking".into()),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.can_wire_result.0
    }

    /// Validate whether two ports can be wired using grammar shapes from the dict.
    /// Returns (is_valid, confidence, reason).
    pub fn validate_with_dict(
        dict: &dyn DictReader,
        from_kind: &str,
        from_port: &str,
        to_kind: &str,
        to_port: &str,
    ) -> (bool, f32, String) {
        let from_shapes = dict.clause_shapes_for(from_kind);
        let to_shapes = dict.clause_shapes_for(to_kind);

        let from_shape = from_shapes.iter().find(|s| s.name == from_port);
        let to_shape = to_shapes.iter().find(|s| s.name == to_port);

        match (from_shape, to_shape) {
            (None, _) => (false, 0.0, format!("unknown port: {}", from_port)),
            (_, None) => (false, 0.0, format!("unknown port: {}", to_port)),
            (Some(fs), Some(ts)) => {
                let compatible = fs.grammar_shape == "any"
                    || ts.grammar_shape == "any"
                    || fs.grammar_shape == ts.grammar_shape;
                if compatible {
                    (true, 0.9, "validated".into())
                } else {
                    (
                        false,
                        0.3,
                        format!("type mismatch: {} → {}", fs.grammar_shape, ts.grammar_shape),
                    )
                }
            }
        }
    }

    /// Construct a connector with real grammar validation from the dict.
    pub fn new_with_validation(
        id: impl Into<String>,
        from_node: impl Into<NodeId>,
        from_port: impl Into<String>,
        to_node: impl Into<NodeId>,
        to_port: impl Into<String>,
        dict: &dyn DictReader,
        from_kind: &str,
        to_kind: &str,
    ) -> Self {
        let from_port = from_port.into();
        let to_port = to_port.into();
        let result =
            Self::validate_with_dict(dict, from_kind, &from_port, to_kind, &to_port);
        let confidence = result.1;
        Self {
            id: id.into(),
            src: (from_node.into(), from_port),
            dst: (to_node.into(), to_port),
            confidence,
            reason: result.2.clone(),
            reason_chain: Vec::new(),
            route: Vec::new(),
            can_wire_result: result,
        }
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
    use crate::dict_reader::ClauseShape;
    use crate::stub_dict::StubDictReader;

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

    fn make_shape(name: &str, grammar_shape: &str) -> ClauseShape {
        ClauseShape {
            name: name.into(),
            grammar_shape: grammar_shape.into(),
            is_required: false,
            description: String::new(),
        }
    }

    #[test]
    fn connector_validates_known_ports() {
        let dict = StubDictReader::new()
            .with_shapes("verb", vec![make_shape("output", "text")])
            .with_shapes("concept", vec![make_shape("input", "text")]);
        let c = Connector::new_with_validation(
            "c1", "n1", "output", "n2", "input", &dict, "verb", "concept",
        );
        assert!(c.is_valid());
        assert!((c.confidence - 0.9).abs() < f32::EPSILON);
        assert_eq!(c.can_wire_result.2, "validated");
    }

    #[test]
    fn connector_rejects_unknown_port() {
        let dict = StubDictReader::new()
            .with_shapes("verb", vec![make_shape("output", "text")])
            .with_shapes("concept", vec![make_shape("input", "text")]);
        let c = Connector::new_with_validation(
            "c2", "n1", "nonexistent", "n2", "input", &dict, "verb", "concept",
        );
        assert!(!c.is_valid());
        assert_eq!(c.confidence, 0.0);
        assert!(c.can_wire_result.2.contains("unknown port: nonexistent"));
    }

    #[test]
    fn connector_validates_any_type_port() {
        // StubDictReader default shapes use grammar_shape "any" — should always be compatible
        let dict = StubDictReader::new();
        let c = Connector::new_with_validation(
            "c3", "n1", "output", "n2", "input", &dict, "verb", "concept",
        );
        assert!(c.is_valid());
        assert!((c.confidence - 0.9).abs() < f32::EPSILON);
        assert_eq!(c.can_wire_result.2, "validated");
    }
}
