#![deny(unsafe_code)]
use std::collections::HashMap;
use crate::node::{ExecNode, NodeId, IsChanged};
use crate::dag::Dag;
use crate::cache::ExecutionCache;

/// Result of executing a single node
#[derive(Clone, Debug)]
pub struct NodeOutput {
    pub node_id: NodeId,
    pub outputs: HashMap<String, Vec<u8>>,  // port_name -> serialized output bytes
    pub cache_key: u64,
    pub was_cached: bool,
}

/// Execution engine: runs a DAG in topological order with caching
pub struct ExecutionEngine {
    pub cache: Box<dyn ExecutionCache>,
}

impl ExecutionEngine {
    pub fn new(cache: impl ExecutionCache + 'static) -> Self {
        Self { cache: Box::new(cache) }
    }

    /// Compute a SipHash-like key from node kind + serialized inputs
    pub fn compute_cache_key(node_kind: &str, input_hash: u64) -> u64 {
        // Simple deterministic hash: xor with kind hash
        let kind_hash = node_kind.bytes().fold(0u64, |acc, b| {
            acc.wrapping_mul(31).wrapping_add(b as u64)
        });
        kind_hash ^ input_hash
    }

    /// Check if a node should re-execute based on IS_CHANGED hierarchy
    pub fn should_execute(&self, node: &ExecNode, input_hash: u64) -> bool {
        match node.is_changed {
            IsChanged::Always => true,
            IsChanged::Never => false,
            IsChanged::HashInput => {
                let key = Self::compute_cache_key(&node.kind, input_hash);
                self.cache.get(key).is_none()
            }
        }
    }

    /// Execute DAG: sort -> filter by cache -> dispatch in order
    /// Returns execution plan (list of nodes to actually run)
    pub fn plan_execution(&self, dag: &Dag) -> Result<Vec<NodeId>, Vec<NodeId>> {
        let sorted = dag.topological_sort()?;
        // Filter: only nodes that need re-execution
        let to_run: Vec<NodeId> = sorted.into_iter().filter(|id| {
            if let Some(node) = dag.nodes.get(id) {
                self.should_execute(node, 0) // input_hash=0 for planning phase
            } else {
                false
            }
        }).collect();
        Ok(to_run)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::BasicCache;

    #[test]
    fn cache_key_deterministic() {
        let k1 = ExecutionEngine::compute_cache_key("verb", 42);
        let k2 = ExecutionEngine::compute_cache_key("verb", 42);
        assert_eq!(k1, k2);
    }

    #[test]
    fn cache_key_differs_by_kind() {
        let k1 = ExecutionEngine::compute_cache_key("verb", 42);
        let k2 = ExecutionEngine::compute_cache_key("concept", 42);
        assert_ne!(k1, k2);
    }

    #[test]
    fn should_execute_always() {
        let engine = ExecutionEngine::new(BasicCache::new());
        let mut node = ExecNode::new("n1", "verb");
        node.is_changed = IsChanged::Always;
        assert!(engine.should_execute(&node, 0));
        assert!(engine.should_execute(&node, 0)); // always re-runs
    }

    #[test]
    fn plan_execution_linear() {
        let engine = ExecutionEngine::new(BasicCache::new());
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("a", "verb"));
        dag.add_node(ExecNode::new("b", "verb"));
        dag.add_edge("a", "out", "b", "in");
        let plan = engine.plan_execution(&dag).unwrap();
        assert_eq!(plan, vec!["a", "b"]);
    }
}
