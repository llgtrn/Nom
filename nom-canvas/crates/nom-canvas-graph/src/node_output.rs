#![deny(unsafe_code)]

/// The data type carried on a node output port.
///
/// Mirrors the typed output contract used in graph workflow engines
/// (Text, structured data, scalars, collections, raw binary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeOutputType {
    Text,
    Json,
    Number,
    Boolean,
    Array,
    Binary,
}

/// A named, typed output port on a node.
#[derive(Debug, Clone)]
pub struct NodeOutputPort {
    pub name: String,
    pub output_type: NodeOutputType,
    pub description: String,
}

/// Events emitted by a node as it executes.
///
/// Pattern derived from event-generator nodes in typed workflow graphs:
/// nodes produce a stream of lifecycle events that downstream consumers
/// or UI layers can observe without polling.
#[derive(Debug, Clone)]
pub enum NodeEvent {
    Started { node_id: String },
    Progress { node_id: String, percent: f32 },
    Completed { node_id: String, output: String },
    Failed { node_id: String, error: String },
}

/// A node that declares typed output ports and emits lifecycle events.
pub trait TypedNode: Send + Sync {
    fn node_id(&self) -> &str;
    fn output_ports(&self) -> Vec<NodeOutputPort>;
    fn emit_events(&self, input: &str) -> Vec<NodeEvent>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoNode {
        id: String,
    }

    impl TypedNode for EchoNode {
        fn node_id(&self) -> &str {
            &self.id
        }

        fn output_ports(&self) -> Vec<NodeOutputPort> {
            vec![
                NodeOutputPort {
                    name: "result".to_string(),
                    output_type: NodeOutputType::Text,
                    description: "echoed input text".to_string(),
                },
                NodeOutputPort {
                    name: "length".to_string(),
                    output_type: NodeOutputType::Number,
                    description: "character count of input".to_string(),
                },
            ]
        }

        fn emit_events(&self, input: &str) -> Vec<NodeEvent> {
            vec![
                NodeEvent::Started {
                    node_id: self.id.clone(),
                },
                NodeEvent::Progress {
                    node_id: self.id.clone(),
                    percent: 50.0,
                },
                NodeEvent::Completed {
                    node_id: self.id.clone(),
                    output: input.to_string(),
                },
            ]
        }
    }

    #[test]
    fn typed_node_output_ports_declared() {
        let node = EchoNode {
            id: "echo-1".to_string(),
        };
        let ports = node.output_ports();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].name, "result");
        assert_eq!(ports[0].output_type, NodeOutputType::Text);
        assert_eq!(ports[1].name, "length");
        assert_eq!(ports[1].output_type, NodeOutputType::Number);
    }

    #[test]
    fn typed_node_emits_lifecycle_events() {
        let node = EchoNode {
            id: "echo-2".to_string(),
        };
        let events = node.emit_events("hello");
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], NodeEvent::Started { .. }));
        assert!(matches!(events[1], NodeEvent::Progress { percent, .. } if percent == 50.0));
        assert!(matches!(events[2], NodeEvent::Completed { ref output, .. } if output == "hello"));
    }

    #[test]
    fn typed_node_failed_event_carries_error_message() {
        let event = NodeEvent::Failed {
            node_id: "n1".to_string(),
            error: "out of memory".to_string(),
        };
        match event {
            NodeEvent::Failed { node_id, error } => {
                assert_eq!(node_id, "n1");
                assert_eq!(error, "out of memory");
            }
            _ => panic!("expected Failed variant"),
        }
    }
}
