use thiserror::Error;
use crate::node_schema::NodeId;

#[derive(Debug, Error)]
pub enum GraphError {
    #[error("node {node} execution failed: {reason}")]
    NodeExecution { node: NodeId, reason: String },

    #[error("cycle detected among nodes: {participants:?}")]
    Cycle { participants: Vec<NodeId> },

    #[error("execution was interrupted")]
    Interrupted,

    #[error("node {node} is missing input '{input}'")]
    MissingInput { node: NodeId, input: String },

    #[error("node {node} produced unexpected output count")]
    OutputMismatch { node: NodeId },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_execution_display() {
        let e = GraphError::NodeExecution { node: 5, reason: "OOM".to_string() };
        let s = e.to_string();
        assert!(s.contains("5"), "should mention node id");
        assert!(s.contains("OOM"), "should include reason");
    }

    #[test]
    fn missing_input_display() {
        let e = GraphError::MissingInput { node: 3, input: "image".to_string() };
        let s = e.to_string();
        assert!(s.contains("3"));
        assert!(s.contains("image"));
    }

    #[test]
    fn interrupted_display() {
        let e = GraphError::Interrupted;
        assert!(e.to_string().contains("interrupted"));
    }
}
