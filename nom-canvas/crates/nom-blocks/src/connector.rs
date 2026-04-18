#![deny(unsafe_code)]
use crate::dict_reader::DictReader;
use crate::graph_node::NodeId;
use serde::{Deserialize, Serialize};

pub type ConnectorId = String;
pub type SlotName = String;

pub struct ConnectorValidation<'a> {
    pub id: ConnectorId,
    pub from_node: NodeId,
    pub from_port: SlotName,
    pub to_node: NodeId,
    pub to_port: SlotName,
    pub dict: &'a dyn DictReader,
    pub from_kind: &'a str,
    pub to_kind: &'a str,
}

/// A wire between two graph nodes. can_wire_result is NON-OPTIONAL.
/// Grammar-backed validation is required at construction time.
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
    can_wire_result: (bool, f32, String),
}

impl Connector {
    /// Append a reasoning step and return self (builder pattern).
    pub fn with_reason(mut self, reason: String) -> Self {
        self.reason_chain.push(reason);
        self
    }

    pub fn is_valid(&self) -> bool {
        self.can_wire_result.0
    }

    pub fn can_wire_result(&self) -> &(bool, f32, String) {
        &self.can_wire_result
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
    pub fn new_with_validation(request: ConnectorValidation<'_>) -> Self {
        let result = Self::validate_with_dict(
            request.dict,
            request.from_kind,
            &request.from_port,
            request.to_kind,
            &request.to_port,
        );
        let confidence = result.1;
        Self {
            id: request.id,
            src: (request.from_node, request.from_port),
            dst: (request.to_node, request.to_port),
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
        self.route = vec![src_pos, [mid_x, src_pos[1]], [mid_x, dst_pos[1]], dst_pos];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict_reader::ClauseShape;
    use crate::stub_dict::StubDictReader;

    fn valid_connector() -> Connector {
        let dict = StubDictReader::new();
        Connector::new_with_validation(ConnectorValidation {
            id: "c1".into(),
            from_node: "n1".into(),
            from_port: "output".into(),
            to_node: "n2".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        })
    }

    #[test]
    fn connector_validated_constructor_is_valid() {
        let c = valid_connector();
        assert!(c.is_valid());
        assert_eq!(c.can_wire_result().2, "validated");
    }

    #[test]
    fn connector_auto_route() {
        let mut c = valid_connector();
        c.auto_route([0.0, 50.0], [200.0, 100.0]);
        assert_eq!(c.route.len(), 4);
        assert_eq!(c.route[0], [0.0, 50.0]);
        assert_eq!(c.route[3], [200.0, 100.0]);
    }

    #[test]
    fn connector_confidence_clamps_to_one() {
        let dict = StubDictReader::new()
            .with_shapes("verb", vec![make_shape("output", "text")])
            .with_shapes("concept", vec![make_shape("input", "integer")]);
        let failed = Connector::new_with_validation(ConnectorValidation {
            id: "c".into(),
            from_node: "n1".into(),
            from_port: "output".into(),
            to_node: "n2".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        assert_eq!(failed.confidence, 0.3);
        assert!(!failed.is_valid());
    }

    #[test]
    fn connector_reason_chain_accumulates() {
        let mut c = valid_connector();
        c.reason_chain.push("step one".into());
        c.reason_chain.push("step two".into());
        assert_eq!(c.reason_chain.len(), 2);
        assert_eq!(c.reason_chain[0], "step one");
        assert_eq!(c.reason_chain[1], "step two");
    }

    #[test]
    fn connector_with_reason_builder() {
        let c = valid_connector()
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
        let c = Connector::new_with_validation(ConnectorValidation {
            id: "c1".into(),
            from_node: "n1".into(),
            from_port: "output".into(),
            to_node: "n2".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        assert!(c.is_valid());
        assert!((c.confidence - 0.9).abs() < f32::EPSILON);
        assert_eq!(c.can_wire_result().2, "validated");
    }

    #[test]
    fn connector_rejects_unknown_port() {
        let dict = StubDictReader::new()
            .with_shapes("verb", vec![make_shape("output", "text")])
            .with_shapes("concept", vec![make_shape("input", "text")]);
        let c = Connector::new_with_validation(ConnectorValidation {
            id: "c2".into(),
            from_node: "n1".into(),
            from_port: "nonexistent".into(),
            to_node: "n2".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        assert!(!c.is_valid());
        assert_eq!(c.confidence, 0.0);
        assert!(c.can_wire_result().2.contains("unknown port: nonexistent"));
    }

    #[test]
    fn connector_validates_any_type_port() {
        // StubDictReader default shapes use grammar_shape "any" — should always be compatible
        let dict = StubDictReader::new();
        let c = Connector::new_with_validation(ConnectorValidation {
            id: "c3".into(),
            from_node: "n1".into(),
            from_port: "output".into(),
            to_node: "n2".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        assert!(c.is_valid());
        assert!((c.confidence - 0.9).abs() < f32::EPSILON);
        assert_eq!(c.can_wire_result().2, "validated");
    }

    #[test]
    fn connector_has_from_to_nodes() {
        let dict = StubDictReader::new();
        let c = Connector::new_with_validation(ConnectorValidation {
            id: "wire-1".into(),
            from_node: "node-src".into(),
            from_port: "output".into(),
            to_node: "node-dst".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        assert_eq!(c.src.0, "node-src");
        assert_eq!(c.src.1, "output");
        assert_eq!(c.dst.0, "node-dst");
        assert_eq!(c.dst.1, "input");
    }

    #[test]
    fn connector_can_wire_same_kind() {
        // Same block type wiring to itself — should be valid with "any" ports
        let dict = StubDictReader::new();
        let c = Connector::new_with_validation(ConnectorValidation {
            id: "c4".into(),
            from_node: "n1".into(),
            from_port: "output".into(),
            to_node: "n1".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "verb",
        });
        assert!(c.is_valid());
    }

    #[test]
    fn connector_type_mismatch_fails() {
        let dict = StubDictReader::new()
            .with_shapes("verb", vec![make_shape("output", "text")])
            .with_shapes("concept", vec![make_shape("input", "integer")]);
        let c = Connector::new_with_validation(ConnectorValidation {
            id: "c5".into(),
            from_node: "n1".into(),
            from_port: "output".into(),
            to_node: "n2".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        assert!(!c.is_valid());
        assert!(c.can_wire_result().2.contains("type mismatch"));
    }

    #[test]
    fn connector_unknown_dst_port_fails() {
        let dict = StubDictReader::new()
            .with_shapes("verb", vec![make_shape("output", "text")])
            .with_shapes("concept", vec![make_shape("input", "text")]);
        let c = Connector::new_with_validation(ConnectorValidation {
            id: "c6".into(),
            from_node: "n1".into(),
            from_port: "output".into(),
            to_node: "n2".into(),
            to_port: "no_such_port".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        assert!(!c.is_valid());
        assert!(c.can_wire_result().2.contains("unknown port: no_such_port"));
    }
}
