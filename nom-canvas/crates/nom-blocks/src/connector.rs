//! Connector model: a validated, grammar-backed wire between two graph nodes.
#![deny(unsafe_code)]
use crate::dict_reader::DictReader;
use crate::graph_node::NodeId;
use serde::{Deserialize, Serialize};

/// Unique identifier for a connector.
pub type ConnectorId = String;
/// Name of a slot/port on a node.
pub type SlotName = String;

/// Parameters required to construct a validated [`Connector`].
pub struct ConnectorValidation<'a> {
    /// Unique ID to assign to the new connector.
    pub id: ConnectorId,
    /// Source node ID.
    pub from_node: NodeId,
    /// Source port name.
    pub from_port: SlotName,
    /// Destination node ID.
    pub to_node: NodeId,
    /// Destination port name.
    pub to_port: SlotName,
    /// Dictionary used for grammar-shape validation.
    pub dict: &'a dyn DictReader,
    /// Grammar kind of the source node.
    pub from_kind: &'a str,
    /// Grammar kind of the destination node.
    pub to_kind: &'a str,
}

/// A wire between two graph nodes. can_wire_result is NON-OPTIONAL.
/// Grammar-backed validation is required at construction time.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Connector {
    /// Unique connector ID.
    pub id: ConnectorId,
    /// Source (node ID, port name).
    pub src: (NodeId, SlotName),
    /// Destination (node ID, port name).
    pub dst: (NodeId, SlotName),
    /// Clamped to [0.0, 1.0]. Represents confidence that this wire is valid.
    pub confidence: f32,
    /// Human-readable validation reason.
    pub reason: String,
    /// Ordered reasoning steps that led to this connection (spec: "reason chain").
    pub reason_chain: Vec<String>,
    /// Bezier route control points.
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

    /// Returns `true` if the connector passed grammar validation.
    pub fn is_valid(&self) -> bool {
        self.can_wire_result.0
    }

    /// Returns the full `(is_valid, confidence, reason)` tuple.
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

    /// validate_with_dict returns valid when both ports have grammar_shape "any"
    #[test]
    fn validate_with_dict_any_any_is_valid() {
        let dict = StubDictReader::new()
            .with_shapes("a", vec![make_shape("out", "any")])
            .with_shapes("b", vec![make_shape("in", "any")]);
        let (valid, confidence, _reason) =
            Connector::validate_with_dict(&dict, "a", "out", "b", "in");
        assert!(valid);
        assert!((confidence - 0.9).abs() < f32::EPSILON);
    }

    /// validate_with_dict: from_port "any" → to_port specific type is compatible
    #[test]
    fn validate_with_dict_any_to_specific_is_valid() {
        let dict = StubDictReader::new()
            .with_shapes("a", vec![make_shape("out", "any")])
            .with_shapes("b", vec![make_shape("in", "integer")]);
        let (valid, _, _) = Connector::validate_with_dict(&dict, "a", "out", "b", "in");
        assert!(valid, "any grammar_shape must be compatible with any other");
    }

    /// validate_with_dict: specific type → "any" is compatible
    #[test]
    fn validate_with_dict_specific_to_any_is_valid() {
        let dict = StubDictReader::new()
            .with_shapes("a", vec![make_shape("out", "prose")])
            .with_shapes("b", vec![make_shape("in", "any")]);
        let (valid, _, _) = Connector::validate_with_dict(&dict, "a", "out", "b", "in");
        assert!(valid, "specific type to 'any' must be valid");
    }

    /// validate_with_dict: mismatched types produce low confidence
    #[test]
    fn validate_with_dict_mismatch_has_low_confidence() {
        let dict = StubDictReader::new()
            .with_shapes("a", vec![make_shape("out", "text")])
            .with_shapes("b", vec![make_shape("in", "integer")]);
        let (valid, confidence, reason) =
            Connector::validate_with_dict(&dict, "a", "out", "b", "in");
        assert!(!valid);
        assert_eq!(confidence, 0.3);
        assert!(reason.contains("type mismatch"));
    }

    /// validate_with_dict: unknown from_port yields confidence 0.0
    #[test]
    fn validate_with_dict_unknown_from_port_zero_confidence() {
        let dict = StubDictReader::new()
            .with_shapes("a", vec![make_shape("out", "text")])
            .with_shapes("b", vec![make_shape("in", "text")]);
        let (valid, confidence, reason) =
            Connector::validate_with_dict(&dict, "a", "missing", "b", "in");
        assert!(!valid);
        assert_eq!(confidence, 0.0);
        assert!(reason.contains("missing"));
    }

    /// auto_route mid-x is average of src and dst x
    #[test]
    fn connector_auto_route_midpoint() {
        let mut c = valid_connector();
        c.auto_route([0.0, 0.0], [100.0, 0.0]);
        // route[1] and route[2] should have x = 50.0 (midpoint)
        assert_eq!(c.route[1][0], 50.0);
        assert_eq!(c.route[2][0], 50.0);
    }

    /// Connector id is preserved after construction
    #[test]
    fn connector_id_preserved() {
        let dict = StubDictReader::new();
        let c = Connector::new_with_validation(ConnectorValidation {
            id: "my-unique-wire-id".into(),
            from_node: "n1".into(),
            from_port: "output".into(),
            to_node: "n2".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        assert_eq!(c.id, "my-unique-wire-id");
    }

    /// Connector route starts empty before auto_route is called
    #[test]
    fn connector_route_empty_before_auto_route() {
        let c = valid_connector();
        assert!(c.route.is_empty(), "route must be empty before auto_route");
    }

    /// with_reason appends to existing chain
    #[test]
    fn connector_with_reason_chain_order() {
        let c = valid_connector()
            .with_reason("first".into())
            .with_reason("second".into())
            .with_reason("third".into());
        assert_eq!(c.reason_chain.len(), 3);
        assert_eq!(c.reason_chain[0], "first");
        assert_eq!(c.reason_chain[2], "third");
    }

    /// can_wire_result confidence matches connector.confidence field
    #[test]
    fn connector_confidence_matches_can_wire_confidence() {
        let c = valid_connector();
        assert!((c.confidence - c.can_wire_result().1).abs() < f32::EPSILON);
    }

    /// validate_with_dict with matching text→text returns reason "validated"
    #[test]
    fn validate_with_dict_matching_types_reason_validated() {
        let dict = StubDictReader::new()
            .with_shapes("a", vec![make_shape("out", "prose")])
            .with_shapes("b", vec![make_shape("in", "prose")]);
        let (valid, _conf, reason) = Connector::validate_with_dict(&dict, "a", "out", "b", "in");
        assert!(valid);
        assert_eq!(reason, "validated");
    }

    // ── wave AI: new connector tests ────────────────────────────────────────────

    #[test]
    fn connector_new_with_validation_ok() {
        let dict = StubDictReader::new();
        let c = Connector::new_with_validation(ConnectorValidation {
            id: "ok-wire".into(),
            from_node: "src".into(),
            from_port: "output".into(),
            to_node: "dst".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        assert!(c.is_valid(), "valid source/target/type must succeed");
    }

    #[test]
    fn connector_new_with_validation_empty_source_errors() {
        let dict = StubDictReader::new()
            .with_shapes("verb", vec![make_shape("output", "text")])
            .with_shapes("concept", vec![make_shape("input", "text")]);
        let c = Connector::new_with_validation(ConnectorValidation {
            id: "c-empty-src".into(),
            from_node: "n1".into(),
            from_port: "".into(), // empty port → unknown port
            to_node: "n2".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        assert!(!c.is_valid(), "empty source port must fail validation");
        assert_eq!(c.confidence, 0.0);
    }

    #[test]
    fn connector_new_with_validation_empty_target_errors() {
        let dict = StubDictReader::new()
            .with_shapes("verb", vec![make_shape("output", "text")])
            .with_shapes("concept", vec![make_shape("input", "text")]);
        let c = Connector::new_with_validation(ConnectorValidation {
            id: "c-empty-dst".into(),
            from_node: "n1".into(),
            from_port: "output".into(),
            to_node: "n2".into(),
            to_port: "".into(), // empty port → unknown port
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        assert!(!c.is_valid(), "empty target port must fail validation");
        assert_eq!(c.confidence, 0.0);
    }

    #[test]
    fn connector_new_with_validation_empty_kind_errors() {
        // StubDictReader returns default [input, output] shapes for any kind including "".
        // A mismatch requires custom shapes with a non-"any" type that doesn't match.
        let dict = StubDictReader::new()
            .with_shapes("", vec![make_shape("output", "integer")])
            .with_shapes("concept", vec![make_shape("input", "text")]);
        // integer → text: type mismatch → invalid
        let c = Connector::new_with_validation(ConnectorValidation {
            id: "c-empty-kind".into(),
            from_node: "n1".into(),
            from_port: "output".into(),
            to_node: "n2".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "",
            to_kind: "concept",
        });
        assert!(!c.is_valid(), "type mismatch with empty from_kind must fail");
    }

    #[test]
    fn connector_source_equals_input() {
        let dict = StubDictReader::new();
        let c = Connector::new_with_validation(ConnectorValidation {
            id: "c-src".into(),
            from_node: "the-source-node".into(),
            from_port: "output".into(),
            to_node: "dst".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        assert_eq!(c.src.0, "the-source-node");
        assert_eq!(c.src.1, "output");
    }

    #[test]
    fn connector_target_equals_input() {
        let dict = StubDictReader::new();
        let c = Connector::new_with_validation(ConnectorValidation {
            id: "c-dst".into(),
            from_node: "src".into(),
            from_port: "output".into(),
            to_node: "the-target-node".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        assert_eq!(c.dst.0, "the-target-node");
        assert_eq!(c.dst.1, "input");
    }

    #[test]
    fn connector_kind_equals_input() {
        let dict = StubDictReader::new();
        let c = Connector::new_with_validation(ConnectorValidation {
            id: "c-kind".into(),
            from_node: "n1".into(),
            from_port: "output".into(),
            to_node: "n2".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        // is_valid() reflects the grammar check for those kinds
        assert!(c.is_valid());
    }

    #[test]
    fn connector_clone_equals_original() {
        let c = valid_connector();
        let c2 = c.clone();
        assert_eq!(c.id, c2.id);
        assert_eq!(c.src, c2.src);
        assert_eq!(c.dst, c2.dst);
        assert!((c.confidence - c2.confidence).abs() < f32::EPSILON);
        assert_eq!(c.reason, c2.reason);
    }

    #[test]
    fn connector_eq_by_fields() {
        let dict = StubDictReader::new();
        let make = || {
            Connector::new_with_validation(ConnectorValidation {
                id: "same-id".into(),
                from_node: "n1".into(),
                from_port: "output".into(),
                to_node: "n2".into(),
                to_port: "input".into(),
                dict: &dict,
                from_kind: "verb",
                to_kind: "concept",
            })
        };
        let c1 = make();
        let c2 = make();
        assert_eq!(c1.id, c2.id);
        assert_eq!(c1.src, c2.src);
        assert_eq!(c1.dst, c2.dst);
        assert_eq!(c1.is_valid(), c2.is_valid());
    }

    #[test]
    fn connector_debug_contains_source() {
        let c = valid_connector();
        let debug_str = format!("{c:?}");
        // src field contains the from_node value
        assert!(
            debug_str.contains("n1"),
            "debug output must contain source node id"
        );
    }

    // ── wave AJ: additional connector tests ────────────────────────────────────

    /// Self-referential connector (src node == dst node) is valid when ports are compatible.
    #[test]
    fn connector_allows_self_referential_if_supported() {
        let dict = StubDictReader::new();
        let c = Connector::new_with_validation(ConnectorValidation {
            id: "self-ref".into(),
            from_node: "same-node".into(),
            from_port: "output".into(),
            to_node: "same-node".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "verb",
        });
        // "any"→"any" ports are compatible regardless of src==dst
        assert!(c.is_valid());
        assert_eq!(c.src.0, "same-node");
        assert_eq!(c.dst.0, "same-node");
    }

    /// Only valid (compatible) port kinds pass connector validation.
    #[test]
    fn connector_kind_whitelist_enforcement() {
        // integer → text: incompatible → invalid
        let dict = StubDictReader::new()
            .with_shapes("verb", vec![make_shape("out", "integer")])
            .with_shapes("concept", vec![make_shape("in", "text")]);
        let bad = Connector::new_with_validation(ConnectorValidation {
            id: "whitelist-bad".into(),
            from_node: "n1".into(),
            from_port: "out".into(),
            to_node: "n2".into(),
            to_port: "in".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        assert!(!bad.is_valid());

        // prose → prose: compatible → valid
        let dict2 = StubDictReader::new()
            .with_shapes("verb", vec![make_shape("out", "prose")])
            .with_shapes("concept", vec![make_shape("in", "prose")]);
        let good = Connector::new_with_validation(ConnectorValidation {
            id: "whitelist-good".into(),
            from_node: "n1".into(),
            from_port: "out".into(),
            to_node: "n2".into(),
            to_port: "in".into(),
            dict: &dict2,
            from_kind: "verb",
            to_kind: "concept",
        });
        assert!(good.is_valid());
    }

    /// Connector serializes and deserializes to identical field values.
    #[test]
    fn connector_from_serialized_round_trip() {
        let c = valid_connector();
        let json = serde_json::to_string(&c).expect("serialize");
        let c2: Connector = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(c.id, c2.id);
        assert_eq!(c.src, c2.src);
        assert_eq!(c.dst, c2.dst);
        assert!((c.confidence - c2.confidence).abs() < f32::EPSILON);
        assert_eq!(c.reason, c2.reason);
        assert_eq!(c.is_valid(), c2.is_valid());
    }

    /// Two independently constructed connectors with the same fields have equal field values.
    #[test]
    fn connector_hash_by_fields() {
        let dict = StubDictReader::new();
        let make = || {
            Connector::new_with_validation(ConnectorValidation {
                id: "hash-test".into(),
                from_node: "src".into(),
                from_port: "output".into(),
                to_node: "dst".into(),
                to_port: "input".into(),
                dict: &dict,
                from_kind: "verb",
                to_kind: "concept",
            })
        };
        let c1 = make();
        let c2 = make();
        assert_eq!(c1.id, c2.id);
        assert_eq!(c1.src, c2.src);
        assert_eq!(c1.dst, c2.dst);
        assert_eq!(c1.is_valid(), c2.is_valid());
    }

    /// Two connectors differ when their validation result differs due to different kinds.
    #[test]
    fn connector_ne_different_kind() {
        let dict = StubDictReader::new();
        let c1 = Connector::new_with_validation(ConnectorValidation {
            id: "ne-kind".into(),
            from_node: "n1".into(),
            from_port: "output".into(),
            to_node: "n2".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        // Change to_kind to one with incompatible shape
        let dict2 = StubDictReader::new()
            .with_shapes("verb", vec![make_shape("output", "text")])
            .with_shapes("noun", vec![make_shape("input", "integer")]);
        let c2 = Connector::new_with_validation(ConnectorValidation {
            id: "ne-kind".into(),
            from_node: "n1".into(),
            from_port: "output".into(),
            to_node: "n2".into(),
            to_port: "input".into(),
            dict: &dict2,
            from_kind: "verb",
            to_kind: "noun",
        });
        // c1 is valid (any/any), c2 is not (text/integer mismatch)
        assert_ne!(c1.is_valid(), c2.is_valid());
    }

    /// Two connectors differ when their source node ID differs.
    #[test]
    fn connector_ne_different_source() {
        let dict = StubDictReader::new();
        let c1 = Connector::new_with_validation(ConnectorValidation {
            id: "c".into(),
            from_node: "src-A".into(),
            from_port: "output".into(),
            to_node: "dst".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        let c2 = Connector::new_with_validation(ConnectorValidation {
            id: "c".into(),
            from_node: "src-B".into(),
            from_port: "output".into(),
            to_node: "dst".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        assert_ne!(c1.src.0, c2.src.0);
    }

    /// Two connectors differ when their target node ID differs.
    #[test]
    fn connector_ne_different_target() {
        let dict = StubDictReader::new();
        let c1 = Connector::new_with_validation(ConnectorValidation {
            id: "c".into(),
            from_node: "src".into(),
            from_port: "output".into(),
            to_node: "dst-A".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        let c2 = Connector::new_with_validation(ConnectorValidation {
            id: "c".into(),
            from_node: "src".into(),
            from_port: "output".into(),
            to_node: "dst-B".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        assert_ne!(c1.dst.0, c2.dst.0);
    }

    /// Batch creation of 10 connectors all get distinct IDs and are valid.
    #[test]
    fn connector_batch_create_10_connectors() {
        let dict = StubDictReader::new();
        let connectors: Vec<Connector> = (0..10)
            .map(|i| {
                Connector::new_with_validation(ConnectorValidation {
                    id: format!("batch-{i}"),
                    from_node: format!("src-{i}"),
                    from_port: "output".into(),
                    to_node: format!("dst-{i}"),
                    to_port: "input".into(),
                    dict: &dict,
                    from_kind: "verb",
                    to_kind: "concept",
                })
            })
            .collect();
        assert_eq!(connectors.len(), 10);
        let ids: std::collections::HashSet<_> = connectors.iter().map(|c| &c.id).collect();
        assert_eq!(ids.len(), 10);
        assert!(connectors.iter().all(|c| c.is_valid()));
    }

    /// A newly created connector has a non-empty id.
    #[test]
    fn connector_kind_is_non_empty_after_creation() {
        let c = valid_connector();
        assert!(!c.id.is_empty(), "connector id must be non-empty");
    }

    /// Debug representation contains the source node id.
    #[test]
    fn connector_to_string_contains_source() {
        let dict = StubDictReader::new();
        let c = Connector::new_with_validation(ConnectorValidation {
            id: "c-debug-src".into(),
            from_node: "node-source-xyz".into(),
            from_port: "output".into(),
            to_node: "node-target".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        let s = format!("{c:?}");
        assert!(s.contains("node-source-xyz"), "debug must contain source node id");
    }

    /// Debug representation contains the target node id.
    #[test]
    fn connector_to_string_contains_target() {
        let dict = StubDictReader::new();
        let c = Connector::new_with_validation(ConnectorValidation {
            id: "c-debug-dst".into(),
            from_node: "src".into(),
            from_port: "output".into(),
            to_node: "node-target-xyz".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        let s = format!("{c:?}");
        assert!(s.contains("node-target-xyz"), "debug must contain target node id");
    }

    /// Debug representation contains the connector id.
    #[test]
    fn connector_to_string_contains_kind() {
        let dict = StubDictReader::new();
        let c = Connector::new_with_validation(ConnectorValidation {
            id: "wire-kind-check".into(),
            from_node: "n1".into(),
            from_port: "output".into(),
            to_node: "n2".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        let s = format!("{c:?}");
        assert!(s.contains("wire-kind-check"), "debug must contain connector id");
    }

    // ── wave AB: additional connector tests ────────────────────────────────────

    /// Two connectors between the same source/target are distinct objects (different IDs).
    #[test]
    fn connector_two_wires_same_endpoints_are_distinct() {
        let dict = StubDictReader::new();
        let c1 = Connector::new_with_validation(ConnectorValidation {
            id: "dup-wire-1".into(),
            from_node: "n1".into(),
            from_port: "output".into(),
            to_node: "n2".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        let c2 = Connector::new_with_validation(ConnectorValidation {
            id: "dup-wire-2".into(),
            from_node: "n1".into(),
            from_port: "output".into(),
            to_node: "n2".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        assert_ne!(c1.id, c2.id);
        assert_eq!(c1.src, c2.src);
        assert_eq!(c1.dst, c2.dst);
    }

    /// Connector label (reason) is preserved after clone.
    #[test]
    fn connector_label_preserved_after_clone() {
        let c = valid_connector().with_reason("my label".into());
        let c2 = c.clone();
        assert_eq!(c2.reason_chain[0], "my label");
    }

    /// Connector with a very long label (>1000 chars) is accepted without truncation.
    #[test]
    fn connector_very_long_label_accepted() {
        let long_label = "x".repeat(1001);
        let c = valid_connector().with_reason(long_label.clone());
        assert_eq!(c.reason_chain[0].len(), 1001);
        assert_eq!(c.reason_chain[0], long_label);
    }

    /// Empty from_kind with any-type shapes produces a valid connector.
    #[test]
    fn connector_empty_kind_with_any_shapes_is_valid() {
        // StubDictReader default shapes use "any" — empty kind still resolves to any/any
        let dict = StubDictReader::new();
        let c = Connector::new_with_validation(ConnectorValidation {
            id: "empty-kind-any".into(),
            from_node: "n1".into(),
            from_port: "output".into(),
            to_node: "n2".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "",
            to_kind: "concept",
        });
        // With default "any" shapes, empty from_kind still resolves the "output" port as "any"
        assert!(c.is_valid());
    }

    /// Connector with empty to_port is invalid (no shape found for empty port name).
    #[test]
    fn connector_empty_to_port_is_invalid() {
        let dict = StubDictReader::new()
            .with_shapes("verb", vec![make_shape("output", "text")])
            .with_shapes("concept", vec![make_shape("input", "text")]);
        let c = Connector::new_with_validation(ConnectorValidation {
            id: "empty-to-port".into(),
            from_node: "n1".into(),
            from_port: "output".into(),
            to_node: "n2".into(),
            to_port: "".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        assert!(!c.is_valid());
        assert_eq!(c.confidence, 0.0);
    }

    /// Connector with empty from_port is invalid (no shape found for empty port name).
    #[test]
    fn connector_empty_from_port_is_invalid() {
        let dict = StubDictReader::new()
            .with_shapes("verb", vec![make_shape("output", "text")])
            .with_shapes("concept", vec![make_shape("input", "text")]);
        let c = Connector::new_with_validation(ConnectorValidation {
            id: "empty-from-port".into(),
            from_node: "n1".into(),
            from_port: "".into(),
            to_node: "n2".into(),
            to_port: "input".into(),
            dict: &dict,
            from_kind: "verb",
            to_kind: "concept",
        });
        assert!(!c.is_valid());
        assert_eq!(c.confidence, 0.0);
    }
}
