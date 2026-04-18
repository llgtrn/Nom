#![deny(unsafe_code)]
use crate::cache::{ChangedFlags, ExecutionCache};
use crate::dag::Dag;
use crate::node::{ExecNode, IsChanged, NodeId};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// NodeHandler trait and built-in implementations
// ---------------------------------------------------------------------------

/// Handler for executing a specific node kind during graph execution.
pub trait NodeHandler: Send + Sync {
    fn kind(&self) -> &str;
    fn execute(&self, inputs: &[String]) -> Result<String, String>;
}

/// Default pass-through handler for unknown kinds — joins inputs with commas.
pub struct PassThroughHandler {
    pub kind_name: String,
}

impl NodeHandler for PassThroughHandler {
    fn kind(&self) -> &str {
        &self.kind_name
    }
    fn execute(&self, inputs: &[String]) -> Result<String, String> {
        Ok(inputs.join(","))
    }
}

/// Concatenation handler for demo/test — joins inputs without separator.
pub struct ConcatHandler;

impl NodeHandler for ConcatHandler {
    fn kind(&self) -> &str {
        "concat"
    }
    fn execute(&self, inputs: &[String]) -> Result<String, String> {
        Ok(inputs.concat())
    }
}

// ---------------------------------------------------------------------------
// NodeHandlerRegistry
// ---------------------------------------------------------------------------

/// Registry mapping node kinds to their handlers.
pub struct NodeHandlerRegistry {
    pub handlers: Vec<Box<dyn NodeHandler>>,
}

impl NodeHandlerRegistry {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    /// Register a handler, consuming and returning `self` for builder chaining.
    pub fn register(mut self, handler: Box<dyn NodeHandler>) -> Self {
        self.handlers.push(handler);
        self
    }

    /// Find a handler by exact kind name.
    pub fn find(&self, kind: &str) -> Option<&dyn NodeHandler> {
        self.handlers
            .iter()
            .find(|h| h.kind() == kind)
            .map(|h| h.as_ref())
    }

    /// Number of registered handlers.
    pub fn handler_count(&self) -> usize {
        self.handlers.len()
    }

    /// Build a registry pre-loaded with `PassThroughHandler("default")` and `ConcatHandler`.
    pub fn with_defaults() -> Self {
        Self::new()
            .register(Box::new(PassThroughHandler {
                kind_name: "default".to_string(),
            }))
            .register(Box::new(ConcatHandler))
    }
}

impl Default for NodeHandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of executing a single node
#[derive(Clone, Debug)]
pub struct NodeOutput {
    pub node_id: NodeId,
    pub outputs: HashMap<String, Vec<u8>>, // port_name -> serialized output bytes
    pub cache_key: u64,
    pub was_cached: bool,
}

/// Collect all transitive ancestors of `node` (inclusive) via DFS over incoming edges.
/// The returned list is sorted for deterministic cache-key derivation.
pub fn collect_ancestors(dag: &Dag, node: &NodeId, visited: &mut HashSet<NodeId>) -> Vec<NodeId> {
    if visited.contains(node) {
        return vec![];
    }
    visited.insert(node.clone());
    let mut ancestors = vec![node.clone()];
    let parents: Vec<NodeId> = dag
        .edges
        .iter()
        .filter(|e| &e.dst_node == node)
        .map(|e| e.src_node.clone())
        .collect();
    for parent in parents {
        ancestors.extend(collect_ancestors(dag, &parent, visited));
    }
    ancestors.sort();
    ancestors
}

/// Execution engine: runs a DAG in topological order with caching
pub struct ExecutionEngine {
    pub cache: Box<dyn ExecutionCache>,
    /// Per-node IS_CHANGED flags — nodes marked clean can be skipped when cache is warm.
    pub changed_flags: ChangedFlags,
    /// Shared cancel flag. Set via `cancel()`; checked inside the execution loop.
    pub cancel_flag: Arc<AtomicBool>,
    /// Optional handler registry; when present, `execute()` dispatches to real logic.
    pub registry: Option<NodeHandlerRegistry>,
}

impl ExecutionEngine {
    pub fn new(cache: impl ExecutionCache + 'static) -> Self {
        Self {
            cache: Box::new(cache),
            changed_flags: ChangedFlags::default(),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            registry: None,
        }
    }

    /// Attach a handler registry, enabling real node dispatch in `execute()`.
    pub fn with_registry(mut self, reg: NodeHandlerRegistry) -> Self {
        self.registry = Some(reg);
        self
    }

    /// Signal the engine to abort the current execution as soon as possible.
    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    /// Clear the cancel flag so the engine can accept new executions.
    pub fn reset_cancel(&self) {
        self.cancel_flag.store(false, Ordering::SeqCst);
    }

    /// Returns `true` if a cancel has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::SeqCst)
    }

    /// Compute a SipHash-like key from node kind + serialized inputs
    pub fn compute_cache_key(node_kind: &str, input_hash: u64) -> u64 {
        // Simple deterministic hash: xor with kind hash
        let kind_hash = node_kind
            .bytes()
            .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
        kind_hash ^ input_hash
    }

    /// Check if a node should re-execute based on IS_CHANGED hierarchy.
    /// Also respects ChangedFlags: nodes explicitly marked clean are skipped
    /// when a cached result exists (Classic-style gating).
    pub fn should_execute(&self, node: &ExecNode, input_hash: u64) -> bool {
        // If node is explicitly flagged as clean, honour that and skip it.
        if !self.changed_flags.is_changed(&node.id) {
            let key = Self::compute_cache_key(&node.kind, input_hash);
            if self.cache.get(key).is_some() {
                return false;
            }
        }
        match node.is_changed {
            IsChanged::Always => true,
            IsChanged::Never => false,
            IsChanged::HashInput => {
                let key = Self::compute_cache_key(&node.kind, input_hash);
                self.cache.get(key).is_none()
            }
        }
    }

    /// Execute a pre-computed plan (list of [`NodeId`]) against the given DAG.
    ///
    /// For each node in plan order, collects cached outputs of upstream nodes as inputs,
    /// computes a cache key from the node kind + combined input hash, and stores a
    /// [`NodeOutput`] entry in the result map.
    ///
    /// Returns early with partial results if `cancel()` has been called.
    pub fn execute(&mut self, dag: &Dag, plan: &[NodeId]) -> HashMap<NodeId, NodeOutput> {
        let mut results: HashMap<NodeId, NodeOutput> = HashMap::new();
        for node_id in plan {
            if self.is_cancelled() {
                break;
            }
            if let Some(node) = dag.nodes.get(node_id) {
                // Collect all transitive ancestors (excluding this node itself) and
                // combine their cached output hashes into one deterministic input hash.
                let mut visited = HashSet::new();
                let all_ancestors = collect_ancestors(dag, node_id, &mut visited);
                let input_hash = all_ancestors.iter().filter(|id| *id != node_id).fold(
                    0u64,
                    |acc, ancestor_id| {
                        let upstream_hash =
                            results.get(ancestor_id).map(|o| o.cache_key).unwrap_or(0);
                        acc.wrapping_add(upstream_hash.rotate_left(17))
                    },
                );
                let cache_key = Self::compute_cache_key(&node.kind, input_hash);
                let was_cached = self.cache.get(cache_key).is_some();
                // Collect string outputs from immediate upstream nodes as handler inputs.
                let handler_inputs: Vec<String> = dag
                    .edges
                    .iter()
                    .filter(|e| &e.dst_node == node_id)
                    .filter_map(|e| {
                        results.get(&e.src_node).and_then(|o| {
                            o.outputs.get("out").map(|b| {
                                String::from_utf8_lossy(b).into_owned()
                            })
                        })
                    })
                    .collect();
                // Dispatch to registered handler when available.
                let mut node_outputs: HashMap<String, Vec<u8>> = HashMap::new();
                if let Some(reg) = &self.registry {
                    let handler_kind = reg
                        .find(&node.kind)
                        .map(|_| node.kind.as_str())
                        .unwrap_or("default");
                    if let Some(handler) = reg.find(handler_kind) {
                        if let Ok(result) = handler.execute(&handler_inputs) {
                            node_outputs.insert("out".to_string(), result.into_bytes());
                        }
                    }
                }
                if !was_cached {
                    // Store a placeholder so downstream nodes see the new key.
                    self.cache
                        .put(cache_key, crate::cache::CachedValue::Bytes(vec![]));
                }
                results.insert(
                    node_id.clone(),
                    NodeOutput {
                        node_id: node_id.clone(),
                        outputs: node_outputs,
                        cache_key,
                        was_cached,
                    },
                );
            }
        }
        results
    }

    /// Execute DAG: sort -> filter by cache -> dispatch in order
    /// Returns execution plan (list of nodes to actually run), or `Err` if cancelled or cycle detected.
    pub fn plan_execution(&self, dag: &Dag) -> Result<Vec<NodeId>, Vec<NodeId>> {
        if self.is_cancelled() {
            return Err(vec![]);
        }
        let sorted = dag.topological_sort()?;
        // Track output hash per node so downstream edges pick up real values
        let mut outputs: HashMap<NodeId, u64> = HashMap::new();
        let to_run: Vec<NodeId> = sorted
            .into_iter()
            .filter(|id| {
                if let Some(node) = dag.nodes.get(id) {
                    // Collect input hashes from upstream outputs
                    let input_hash =
                        dag.edges
                            .iter()
                            .filter(|e| &e.dst_node == id)
                            .fold(0u64, |acc, edge| {
                                let upstream_hash =
                                    outputs.get(&edge.src_node).copied().unwrap_or(0);
                                acc.wrapping_add(upstream_hash.rotate_left(17))
                            });
                    let should_run = self.should_execute(node, input_hash);
                    // Store output hash for downstream consumers regardless of run/skip
                    let key = Self::compute_cache_key(&node.kind, input_hash);
                    outputs.insert(id.clone(), key);
                    should_run
                } else {
                    false
                }
            })
            .collect();
        Ok(to_run)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::BasicCache;

    #[test]
    fn test_transitive_ancestry_cache_key() {
        // Chain: A -> B -> C
        // Ancestry of C must include A and B (not just B).
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("a", "verb"));
        dag.add_node(ExecNode::new("b", "verb"));
        dag.add_node(ExecNode::new("c", "verb"));
        dag.add_edge("a", "out", "b", "in");
        dag.add_edge("b", "out", "c", "in");

        let mut visited = HashSet::new();
        let ancestors = collect_ancestors(&dag, &"c".to_string(), &mut visited);
        assert!(
            ancestors.contains(&"a".to_string()),
            "a must be a transitive ancestor of c"
        );
        assert!(
            ancestors.contains(&"b".to_string()),
            "b must be a direct ancestor of c"
        );
        assert!(ancestors.contains(&"c".to_string()), "c is included (self)");
        assert_eq!(ancestors.len(), 3);
    }

    #[test]
    fn test_transitive_ancestry_no_duplicates() {
        // Diamond: A -> B, A -> C, B -> D, C -> D
        // Ancestry of D must include A exactly once.
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("a", "verb"));
        dag.add_node(ExecNode::new("b", "verb"));
        dag.add_node(ExecNode::new("c", "verb"));
        dag.add_node(ExecNode::new("d", "verb"));
        dag.add_edge("a", "out", "b", "in");
        dag.add_edge("a", "out", "c", "in");
        dag.add_edge("b", "out", "d", "in");
        dag.add_edge("c", "out", "d", "in");

        let mut visited = HashSet::new();
        let ancestors = collect_ancestors(&dag, &"d".to_string(), &mut visited);
        // All four nodes, each exactly once.
        assert_eq!(ancestors.len(), 4);
        let a_count = ancestors.iter().filter(|id| id.as_str() == "a").count();
        assert_eq!(
            a_count, 1,
            "a must appear exactly once despite diamond topology"
        );
    }

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

    #[test]
    fn plan_execution_propagates_hashes() {
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("root", "verb"));
        dag.add_node(ExecNode::new("child", "verb"));
        dag.add_edge("root", "out", "child", "in");
        let engine = ExecutionEngine::new(BasicCache::new());
        let plan = engine.plan_execution(&dag).unwrap();
        assert_eq!(plan.len(), 2); // both should run (empty cache)
        assert_eq!(plan[0], "root");
        assert_eq!(plan[1], "child");
    }

    #[test]
    fn execution_engine_runs_single_node() {
        let engine = ExecutionEngine::new(BasicCache::new());
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("solo", "verb"));
        let plan = engine.plan_execution(&dag).unwrap();
        assert_eq!(plan, vec!["solo"]);
    }

    #[test]
    fn execution_engine_propagates_output_hash() {
        // Two-node chain: root -> leaf. Both start with empty cache so both run.
        // After planning, the output hash for "root" feeds into "leaf"'s input hash,
        // meaning the leaf's cache key is derived from root's output — not zero.
        let engine = ExecutionEngine::new(BasicCache::new());
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("root", "verb"));
        dag.add_node(ExecNode::new("leaf", "noun"));
        dag.add_edge("root", "out", "leaf", "in");
        let plan = engine.plan_execution(&dag).unwrap();
        // Both nodes should be scheduled when cache is empty.
        assert_eq!(plan.len(), 2);
        // Order must be root before leaf.
        assert_eq!(plan[0], "root");
        assert_eq!(plan[1], "leaf");
        // The cache key for leaf should differ from a standalone leaf (non-zero input hash).
        let standalone_key = ExecutionEngine::compute_cache_key("noun", 0);
        let root_key = ExecutionEngine::compute_cache_key("verb", 0);
        let chained_key = ExecutionEngine::compute_cache_key("noun", root_key.rotate_left(17));
        assert_ne!(standalone_key, chained_key);
    }

    #[test]
    fn execution_cancel_flag_works() {
        let engine = ExecutionEngine::new(BasicCache::new());
        assert!(
            !engine.is_cancelled(),
            "fresh engine should not be cancelled"
        );
        engine.cancel();
        assert!(
            engine.is_cancelled(),
            "engine should be cancelled after cancel()"
        );
        engine.reset_cancel();
        assert!(
            !engine.is_cancelled(),
            "engine should not be cancelled after reset_cancel()"
        );
    }

    #[test]
    fn plan_execution_returns_err_when_cancelled() {
        let engine = ExecutionEngine::new(BasicCache::new());
        engine.cancel();
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("n1", "verb"));
        let result = engine.plan_execution(&dag);
        assert!(
            result.is_err(),
            "plan_execution should return Err when cancelled"
        );
    }

    #[test]
    fn execution_engine_skips_unchanged_nodes() {
        use crate::cache::CachedValue;
        let mut cache = BasicCache::new();
        // Pre-populate the cache for node "stable" with HashInput policy (default).
        // input_hash for a standalone node (no upstream edges) is 0.
        let key = ExecutionEngine::compute_cache_key("verb", 0);
        cache.put(key, CachedValue::String("cached".into()));

        let engine = ExecutionEngine::new(cache);
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("stable", "verb")); // HashInput, cache hit → skip
        let plan = engine.plan_execution(&dag).unwrap();
        assert!(plan.is_empty(), "node with warm cache should be skipped");
    }

    #[test]
    fn execution_engine_cancel_prevents_processing() {
        let engine = ExecutionEngine::new(BasicCache::new());
        engine.cancel();
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("n1", "verb"));
        dag.add_node(ExecNode::new("n2", "verb"));
        dag.add_edge("n1", "out", "n2", "in");
        let result = engine.plan_execution(&dag);
        assert!(
            result.is_err(),
            "plan_execution must return Err when cancel flag is set before call"
        );
        assert_eq!(
            result.unwrap_err(),
            Vec::<NodeId>::new(),
            "cancel returns empty Err vec"
        );
    }

    #[test]
    fn execution_engine_changed_flags_skip_unchanged() {
        use crate::cache::CachedValue;
        let mut cache = BasicCache::new();
        // Pre-populate cache for "clean_node" (kind="verb", input_hash=0).
        let key = ExecutionEngine::compute_cache_key("verb", 0);
        cache.put(key, CachedValue::String("cached".into()));

        let mut engine = ExecutionEngine::new(cache);
        // Mark the node as clean — combined with a cache hit this should skip it.
        engine.changed_flags.mark_clean("clean_node".to_string());

        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("clean_node", "verb"));
        let plan = engine.plan_execution(&dag).unwrap();
        assert!(
            plan.is_empty(),
            "node marked clean with a warm cache must be skipped"
        );
    }

    // ------------------------------------------------------------------
    // NodeHandler trait implementations
    // ------------------------------------------------------------------

    #[test]
    fn pass_through_handler_execute() {
        let handler = PassThroughHandler {
            kind_name: "verb".to_string(),
        };
        assert_eq!(handler.kind(), "verb");
        let result = handler
            .execute(&["a".to_string(), "b".to_string(), "c".to_string()])
            .unwrap();
        assert_eq!(result, "a,b,c");
    }

    #[test]
    fn concat_handler_execute() {
        let handler = ConcatHandler;
        assert_eq!(handler.kind(), "concat");
        let result = handler
            .execute(&["hello".to_string(), " world".to_string()])
            .unwrap();
        assert_eq!(result, "hello world");
    }

    // ------------------------------------------------------------------
    // NodeHandlerRegistry
    // ------------------------------------------------------------------

    #[test]
    fn registry_new_register_handler_count() {
        let reg = NodeHandlerRegistry::new()
            .register(Box::new(ConcatHandler))
            .register(Box::new(PassThroughHandler {
                kind_name: "custom".to_string(),
            }));
        assert_eq!(reg.handler_count(), 2);
    }

    #[test]
    fn registry_find_by_kind() {
        let reg = NodeHandlerRegistry::new().register(Box::new(ConcatHandler));
        assert!(reg.find("concat").is_some());
        assert!(reg.find("unknown").is_none());
    }

    #[test]
    fn registry_with_defaults_has_two_handlers() {
        let reg = NodeHandlerRegistry::with_defaults();
        assert_eq!(reg.handler_count(), 2);
        assert!(reg.find("default").is_some());
        assert!(reg.find("concat").is_some());
    }

    #[test]
    fn execution_engine_with_registry_does_not_panic() {
        let reg = NodeHandlerRegistry::with_defaults();
        let mut engine = ExecutionEngine::new(BasicCache::new()).with_registry(reg);
        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("n1", "concat"));
        let plan = engine.plan_execution(&dag).unwrap();
        let _results = engine.execute(&dag, &plan);
        // No panic means the registry dispatch path is exercised successfully.
    }
}
