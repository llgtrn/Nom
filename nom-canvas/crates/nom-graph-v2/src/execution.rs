use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::cache::CacheBackend;
use crate::error::GraphError;
use crate::fingerprint::fingerprint_inputs;
use crate::node_schema::{NodeId, NodeSchema};
use crate::progress::ProgressHandler;

pub struct Executor<B: CacheBackend, H: ProgressHandler> {
    nodes: HashMap<NodeId, NodeSchema>,
    cache: B,
    progress: H,
    interrupt: Arc<AtomicBool>,
}

impl<B: CacheBackend, H: ProgressHandler> Executor<B, H> {
    pub fn new(cache: B, progress: H) -> Self {
        Self {
            nodes: HashMap::new(),
            cache,
            progress,
            interrupt: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn with_interrupt(mut self, flag: Arc<AtomicBool>) -> Self {
        self.interrupt = flag;
        self
    }

    pub fn register(&mut self, schema: NodeSchema) {
        self.nodes.insert(schema.id, schema);
    }

    pub fn execute(
        &mut self,
        order: &[NodeId],
        inputs: HashMap<NodeId, Vec<(String, Vec<u8>)>>,
    ) -> Result<HashMap<NodeId, Vec<Vec<u8>>>, GraphError> {
        let mut outputs: HashMap<NodeId, Vec<Vec<u8>>> = HashMap::new();

        for (step_idx, &node_id) in order.iter().enumerate() {
            // Check interrupt between nodes
            if self.interrupt.load(Ordering::Relaxed) {
                return Err(GraphError::Interrupted);
            }

            let schema = self.nodes.get(&node_id).ok_or_else(|| GraphError::NodeExecution {
                node: node_id,
                reason: "schema not registered".to_string(),
            })?;

            // Gather string-typed inputs for fingerprinting
            let node_inputs = inputs.get(&node_id).cloned().unwrap_or_default();
            let string_inputs: Vec<(String, String)> = node_inputs
                .iter()
                .map(|(k, v)| (k.clone(), String::from_utf8_lossy(v).into_owned()))
                .collect();

            // Collect ancestor fingerprints from upstream outputs
            let ancestors: Vec<u64> = order[..step_idx]
                .iter()
                .filter(|&&anc| outputs.contains_key(&anc))
                .map(|&anc| anc)
                .collect();

            let fp = fingerprint_inputs(&schema.class_type, &string_inputs, &ancestors);

            // Cache check
            if let Some(cached) = self.cache.get(fp) {
                // Deserialize: each output is length-prefixed (8 bytes LE)
                let restored = deserialize_outputs(&cached);
                self.progress.update(node_id, 1, 1);
                outputs.insert(node_id, restored);
                continue;
            }

            // Run the node (placeholder: concatenates all input bytes)
            let result = run_node(schema, &node_inputs);
            self.progress.update(node_id, 1, 1);

            // Cache the result
            let serialized = serialize_outputs(&result);
            self.cache.set(fp, serialized);

            outputs.insert(node_id, result);
        }

        Ok(outputs)
    }
}

/// Placeholder node executor — concatenates all input byte slices into a single output.
fn run_node(schema: &NodeSchema, inputs: &[(String, Vec<u8>)]) -> Vec<Vec<u8>> {
    let _ = schema;
    let combined: Vec<u8> = inputs.iter().flat_map(|(_, v)| v.iter().copied()).collect();
    vec![combined]
}

fn serialize_outputs(outputs: &[Vec<u8>]) -> Vec<u8> {
    let mut buf = Vec::new();
    // Store count as 8-byte LE
    buf.extend_from_slice(&(outputs.len() as u64).to_le_bytes());
    for out in outputs {
        buf.extend_from_slice(&(out.len() as u64).to_le_bytes());
        buf.extend_from_slice(out);
    }
    buf
}

fn deserialize_outputs(data: &[u8]) -> Vec<Vec<u8>> {
    if data.len() < 8 {
        return Vec::new();
    }
    let count = u64::from_le_bytes(data[..8].try_into().unwrap_or([0; 8])) as usize;
    let mut pos = 8;
    let mut result = Vec::with_capacity(count);
    for _ in 0..count {
        if pos + 8 > data.len() {
            break;
        }
        let len = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap_or([0; 8])) as usize;
        pos += 8;
        if pos + len > data.len() {
            break;
        }
        result.push(data[pos..pos + len].to_vec());
        pos += len;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::ClassicCache;
    use crate::progress::NoProgress;

    fn make_executor() -> Executor<ClassicCache, NoProgress> {
        Executor::new(ClassicCache::new(), NoProgress)
    }

    #[test]
    fn empty_dag_returns_empty_outputs() {
        let mut ex = make_executor();
        let result = ex.execute(&[], HashMap::new()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn three_node_chain_executes_in_order() {
        let mut ex = make_executor();
        ex.register(NodeSchema::new(1, "A").with_output("BYTES"));
        ex.register(NodeSchema::new(2, "B").with_output("BYTES"));
        ex.register(NodeSchema::new(3, "C").with_output("BYTES"));

        let mut inputs = HashMap::new();
        inputs.insert(1, vec![("data".to_string(), b"hello".to_vec())]);
        inputs.insert(2, vec![("data".to_string(), b"world".to_vec())]);
        inputs.insert(3, vec![]);

        let result = ex.execute(&[1, 2, 3], inputs).unwrap();
        assert!(result.contains_key(&1));
        assert!(result.contains_key(&2));
        assert!(result.contains_key(&3));
        // Node 1 output = "hello"
        assert_eq!(result[&1][0], b"hello");
    }

    #[test]
    fn cache_hit_skips_re_execution() {
        let cache = ClassicCache::new();
        let mut ex = Executor::new(cache, NoProgress);
        ex.register(NodeSchema::new(1, "Node").with_output("BYTES"));

        let mut inputs = HashMap::new();
        inputs.insert(1, vec![("x".to_string(), b"data".to_vec())]);

        // First run — miss
        let r1 = ex.execute(&[1], inputs.clone()).unwrap();
        // Second run — hit (same inputs → same fingerprint)
        let r2 = ex.execute(&[1], inputs).unwrap();
        // Outputs should be identical
        assert_eq!(r1[&1], r2[&1]);
    }

    #[test]
    fn interrupt_mid_run_returns_interrupted() {
        let flag = Arc::new(AtomicBool::new(false));
        let mut ex = Executor::new(ClassicCache::new(), NoProgress)
            .with_interrupt(Arc::clone(&flag));
        ex.register(NodeSchema::new(1, "A"));
        ex.register(NodeSchema::new(2, "B"));

        // Trigger interrupt before execution
        flag.store(true, Ordering::Relaxed);

        let result = ex.execute(&[1, 2], HashMap::new());
        assert!(matches!(result, Err(GraphError::Interrupted)));
    }

    #[test]
    fn missing_schema_returns_error() {
        let mut ex = make_executor();
        // Node 99 not registered
        let result = ex.execute(&[99], HashMap::new());
        assert!(matches!(result, Err(GraphError::NodeExecution { node: 99, .. })));
    }

    #[test]
    fn single_node_no_inputs() {
        let mut ex = make_executor();
        ex.register(NodeSchema::new(10, "Empty"));
        let result = ex.execute(&[10], HashMap::new()).unwrap();
        assert!(result.contains_key(&10));
        // No inputs → empty combined output
        assert_eq!(result[&10][0], b"");
    }
}
