//! Scenario/workflow composition backend — pull-based stack execution.
//!
//! Models workflow execution as a typed data structure: a stack
//! of node activations, each with retry policy + continue-on-fail behaviour.
//! Actual execution engine lives in a runtime crate; this module is pure
//! data + validation + execution-state transitions.
#![deny(unsafe_code)]

use crate::backend_trait::{CompositionBackend, ComposeSpec, ComposeOutput, ComposeError, InterruptFlag, ProgressSink};
use crate::kind::NomKind;

pub type NodeKey = String;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnError { ContinueRegularOutput, ContinueErrorOutput, StopWorkflow }

#[derive(Clone, Debug, PartialEq)]
pub struct RetryPolicy {
    pub retry_on_fail: bool,
    pub max_tries: u8,           // 0..=5
    pub wait_between_tries_ms: u32,  // 0..=5000
}

impl Default for RetryPolicy {
    fn default() -> Self { Self { retry_on_fail: false, max_tries: 1, wait_between_tries_ms: 0 } }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorkflowNode {
    pub key: NodeKey,
    pub node_type: String,       // abstract — e.g. "http_request", "code_block", "branch"
    pub params: Vec<(String, String)>,
    pub retry: RetryPolicy,
    pub on_error: OnError,
    pub continue_on_fail: bool,
}

impl WorkflowNode {
    pub fn new(key: impl Into<NodeKey>, node_type: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            node_type: node_type.into(),
            params: vec![],
            retry: RetryPolicy::default(),
            on_error: OnError::StopWorkflow,
            continue_on_fail: false,
        }
    }
    pub fn with_retry(mut self, retry: RetryPolicy) -> Self { self.retry = retry; self }
    pub fn with_on_error(mut self, on_error: OnError) -> Self { self.on_error = on_error; self }
    pub fn continue_on_fail(mut self) -> Self { self.continue_on_fail = true; self }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeState { Pending, Running, Succeeded, Failed, Skipped }

#[derive(Clone, Debug, PartialEq)]
pub struct NodeActivation {
    pub key: NodeKey,
    pub state: NodeState,
    pub attempt: u8,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct WorkflowSpec {
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<(NodeKey, NodeKey)>,
    pub entry: Option<NodeKey>,
}

impl WorkflowSpec {
    pub fn new() -> Self { Self::default() }

    pub fn add_node(&mut self, node: WorkflowNode) -> Result<(), WorkflowError> {
        if self.nodes.iter().any(|n| n.key == node.key) {
            return Err(WorkflowError::DuplicateKey(node.key));
        }
        self.nodes.push(node);
        Ok(())
    }

    pub fn connect(&mut self, from: impl Into<NodeKey>, to: impl Into<NodeKey>) -> Result<(), WorkflowError> {
        let from = from.into();
        let to = to.into();
        if !self.nodes.iter().any(|n| n.key == from) { return Err(WorkflowError::UnknownNode(from)); }
        if !self.nodes.iter().any(|n| n.key == to) { return Err(WorkflowError::UnknownNode(to)); }
        self.edges.push((from, to));
        Ok(())
    }

    pub fn set_entry(&mut self, key: impl Into<NodeKey>) -> Result<(), WorkflowError> {
        let key = key.into();
        if !self.nodes.iter().any(|n| n.key == key) { return Err(WorkflowError::UnknownNode(key)); }
        self.entry = Some(key);
        Ok(())
    }

    /// Outgoing edges from a given node.
    pub fn successors(&self, node: &str) -> Vec<&NodeKey> {
        self.edges.iter().filter(|(from, _)| from == node).map(|(_, to)| to).collect()
    }

    pub fn node_count(&self) -> usize { self.nodes.len() }
    pub fn edge_count(&self) -> usize { self.edges.len() }
}

#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("duplicate node key '{0}'")]
    DuplicateKey(String),
    #[error("unknown node '{0}'")]
    UnknownNode(String),
    #[error("entry node not set")]
    NoEntry,
    #[error("max_tries {0} exceeds limit 5")]
    InvalidMaxTries(u8),
    #[error("wait_between_tries_ms {0} exceeds limit 5000")]
    InvalidWaitMs(u32),
}

pub fn validate(spec: &WorkflowSpec) -> Result<(), WorkflowError> {
    if spec.entry.is_none() { return Err(WorkflowError::NoEntry); }
    for n in &spec.nodes {
        if n.retry.max_tries > 5 { return Err(WorkflowError::InvalidMaxTries(n.retry.max_tries)); }
        if n.retry.wait_between_tries_ms > 5000 { return Err(WorkflowError::InvalidWaitMs(n.retry.wait_between_tries_ms)); }
    }
    Ok(())
}

/// Compute which node should run next given the current activation stack.
/// Returns the key of the next Pending node whose predecessors are all
/// Succeeded (or Skipped — continue-on-fail flows), or None when done.
pub fn next_ready<'a>(spec: &'a WorkflowSpec, activations: &[NodeActivation]) -> Option<&'a NodeKey> {
    for node in &spec.nodes {
        let act = activations.iter().find(|a| a.key == node.key);
        let state = act.map(|a| a.state).unwrap_or(NodeState::Pending);
        if state != NodeState::Pending { continue; }
        let preds: Vec<&NodeKey> = spec.edges.iter()
            .filter(|(_, to)| to == &node.key)
            .map(|(from, _)| from)
            .collect();
        let all_ready = preds.iter().all(|p| {
            activations.iter().find(|a| a.key == **p)
                .map(|a| matches!(a.state, NodeState::Succeeded | NodeState::Skipped))
                .unwrap_or(false)
        });
        if all_ready { return Some(&node.key); }
    }
    None
}

pub struct StubScenarioWorkflowBackend;

impl CompositionBackend for StubScenarioWorkflowBackend {
    fn kind(&self) -> NomKind { NomKind::ScenarioWorkflow }
    fn name(&self) -> &str { "stub-scenario-workflow" }
    fn compose(&self, _spec: &ComposeSpec, _progress: &dyn ProgressSink, _interrupt: &InterruptFlag) -> Result<ComposeOutput, ComposeError> {
        Ok(ComposeOutput { bytes: b"{}".to_vec(), mime_type: "application/json".to_string(), cost_cents: 0 })
    }
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // RetryPolicy::default
    #[test]
    fn retry_policy_default_values() {
        let r = RetryPolicy::default();
        assert!(!r.retry_on_fail);
        assert_eq!(r.max_tries, 1);
        assert_eq!(r.wait_between_tries_ms, 0);
    }

    // WorkflowNode::new defaults
    #[test]
    fn workflow_node_new_defaults() {
        let n = WorkflowNode::new("fetch", "http_request");
        assert_eq!(n.key, "fetch");
        assert_eq!(n.node_type, "http_request");
        assert!(n.params.is_empty());
        assert_eq!(n.retry, RetryPolicy::default());
        assert_eq!(n.on_error, OnError::StopWorkflow);
        assert!(!n.continue_on_fail);
    }

    // Builder: with_retry
    #[test]
    fn workflow_node_with_retry() {
        let policy = RetryPolicy { retry_on_fail: true, max_tries: 3, wait_between_tries_ms: 500 };
        let n = WorkflowNode::new("a", "code_block").with_retry(policy.clone());
        assert_eq!(n.retry, policy);
    }

    // Builder: with_on_error
    #[test]
    fn workflow_node_with_on_error() {
        let n = WorkflowNode::new("b", "branch").with_on_error(OnError::ContinueErrorOutput);
        assert_eq!(n.on_error, OnError::ContinueErrorOutput);
    }

    // Builder: continue_on_fail
    #[test]
    fn workflow_node_continue_on_fail() {
        let n = WorkflowNode::new("c", "transform").continue_on_fail();
        assert!(n.continue_on_fail);
    }

    // WorkflowSpec::new is empty
    #[test]
    fn workflow_spec_new_empty() {
        let s = WorkflowSpec::new();
        assert_eq!(s.node_count(), 0);
        assert_eq!(s.edge_count(), 0);
        assert!(s.entry.is_none());
    }

    // add_node success
    #[test]
    fn add_node_success() {
        let mut s = WorkflowSpec::new();
        s.add_node(WorkflowNode::new("x", "http_request")).unwrap();
        assert_eq!(s.node_count(), 1);
    }

    // add_node duplicate → DuplicateKey
    #[test]
    fn add_node_duplicate_error() {
        let mut s = WorkflowSpec::new();
        s.add_node(WorkflowNode::new("dup", "code_block")).unwrap();
        let err = s.add_node(WorkflowNode::new("dup", "code_block")).unwrap_err();
        assert!(matches!(err, WorkflowError::DuplicateKey(ref k) if k == "dup"));
    }

    // connect success
    #[test]
    fn connect_success() {
        let mut s = WorkflowSpec::new();
        s.add_node(WorkflowNode::new("a", "t")).unwrap();
        s.add_node(WorkflowNode::new("b", "t")).unwrap();
        s.connect("a", "b").unwrap();
        assert_eq!(s.edge_count(), 1);
    }

    // connect unknown from → UnknownNode
    #[test]
    fn connect_unknown_from_error() {
        let mut s = WorkflowSpec::new();
        s.add_node(WorkflowNode::new("b", "t")).unwrap();
        let err = s.connect("missing", "b").unwrap_err();
        assert!(matches!(err, WorkflowError::UnknownNode(ref k) if k == "missing"));
    }

    // connect unknown to → UnknownNode
    #[test]
    fn connect_unknown_to_error() {
        let mut s = WorkflowSpec::new();
        s.add_node(WorkflowNode::new("a", "t")).unwrap();
        let err = s.connect("a", "ghost").unwrap_err();
        assert!(matches!(err, WorkflowError::UnknownNode(ref k) if k == "ghost"));
    }

    // set_entry success
    #[test]
    fn set_entry_success() {
        let mut s = WorkflowSpec::new();
        s.add_node(WorkflowNode::new("start", "trigger")).unwrap();
        s.set_entry("start").unwrap();
        assert_eq!(s.entry.as_deref(), Some("start"));
    }

    // set_entry unknown → UnknownNode
    #[test]
    fn set_entry_unknown_error() {
        let mut s = WorkflowSpec::new();
        let err = s.set_entry("nowhere").unwrap_err();
        assert!(matches!(err, WorkflowError::UnknownNode(ref k) if k == "nowhere"));
    }

    // successors filters correctly + sink node has 0 successors
    #[test]
    fn successors_filters_and_sink() {
        let mut s = WorkflowSpec::new();
        s.add_node(WorkflowNode::new("a", "t")).unwrap();
        s.add_node(WorkflowNode::new("b", "t")).unwrap();
        s.add_node(WorkflowNode::new("c", "t")).unwrap();
        s.connect("a", "b").unwrap();
        s.connect("a", "c").unwrap();
        let succs = s.successors("a");
        assert_eq!(succs.len(), 2);
        assert!(succs.contains(&&"b".to_string()));
        assert!(succs.contains(&&"c".to_string()));
        assert_eq!(s.successors("c").len(), 0);
    }

    // node_count + edge_count
    #[test]
    fn node_and_edge_count() {
        let mut s = WorkflowSpec::new();
        s.add_node(WorkflowNode::new("p", "t")).unwrap();
        s.add_node(WorkflowNode::new("q", "t")).unwrap();
        s.connect("p", "q").unwrap();
        assert_eq!(s.node_count(), 2);
        assert_eq!(s.edge_count(), 1);
    }

    // validate Ok with entry set
    #[test]
    fn validate_ok() {
        let mut s = WorkflowSpec::new();
        s.add_node(WorkflowNode::new("start", "trigger")).unwrap();
        s.set_entry("start").unwrap();
        validate(&s).unwrap();
    }

    // validate NoEntry
    #[test]
    fn validate_no_entry_error() {
        let s = WorkflowSpec::new();
        let err = validate(&s).unwrap_err();
        assert!(matches!(err, WorkflowError::NoEntry));
    }

    // validate max_tries=6 → InvalidMaxTries
    #[test]
    fn validate_max_tries_exceeded() {
        let mut s = WorkflowSpec::new();
        let node = WorkflowNode::new("n", "t")
            .with_retry(RetryPolicy { retry_on_fail: true, max_tries: 6, wait_between_tries_ms: 0 });
        s.add_node(node).unwrap();
        s.set_entry("n").unwrap();
        let err = validate(&s).unwrap_err();
        assert!(matches!(err, WorkflowError::InvalidMaxTries(6)));
    }

    // validate wait_ms=10000 → InvalidWaitMs
    #[test]
    fn validate_wait_ms_exceeded() {
        let mut s = WorkflowSpec::new();
        let node = WorkflowNode::new("n", "t")
            .with_retry(RetryPolicy { retry_on_fail: true, max_tries: 1, wait_between_tries_ms: 10000 });
        s.add_node(node).unwrap();
        s.set_entry("n").unwrap();
        let err = validate(&s).unwrap_err();
        assert!(matches!(err, WorkflowError::InvalidWaitMs(10000)));
    }

    // next_ready picks entry node when no predecessors
    #[test]
    fn next_ready_picks_entry_no_preds() {
        let mut s = WorkflowSpec::new();
        s.add_node(WorkflowNode::new("entry", "trigger")).unwrap();
        s.set_entry("entry").unwrap();
        let activations = vec![];
        let ready = next_ready(&s, &activations);
        assert_eq!(ready.map(|k| k.as_str()), Some("entry"));
    }

    // next_ready skips non-ready (predecessor not yet succeeded)
    #[test]
    fn next_ready_skips_blocked_node() {
        let mut s = WorkflowSpec::new();
        s.add_node(WorkflowNode::new("a", "t")).unwrap();
        s.add_node(WorkflowNode::new("b", "t")).unwrap();
        s.connect("a", "b").unwrap();
        s.set_entry("a").unwrap();
        // a is Running, b should not be ready
        let activations = vec![
            NodeActivation { key: "a".to_string(), state: NodeState::Running, attempt: 1 },
        ];
        let ready = next_ready(&s, &activations);
        // a is not Pending, b's predecessor (a) is not Succeeded/Skipped → None
        assert!(ready.is_none());
    }

    // next_ready returns b once a is Succeeded
    #[test]
    fn next_ready_returns_after_predecessor_succeeds() {
        let mut s = WorkflowSpec::new();
        s.add_node(WorkflowNode::new("a", "t")).unwrap();
        s.add_node(WorkflowNode::new("b", "t")).unwrap();
        s.connect("a", "b").unwrap();
        s.set_entry("a").unwrap();
        let activations = vec![
            NodeActivation { key: "a".to_string(), state: NodeState::Succeeded, attempt: 1 },
        ];
        let ready = next_ready(&s, &activations);
        assert_eq!(ready.map(|k| k.as_str()), Some("b"));
    }

    // StubScenarioWorkflowBackend kind = ScenarioWorkflow + name
    #[test]
    fn stub_backend_kind_and_name() {
        let b = StubScenarioWorkflowBackend;
        assert_eq!(b.kind(), NomKind::ScenarioWorkflow);
        assert_eq!(b.name(), "stub-scenario-workflow");
    }

    // StubScenarioWorkflowBackend compose returns JSON stub
    #[test]
    fn stub_backend_compose_returns_json() {
        use std::sync::{Arc, atomic::AtomicBool};
        struct NoopSink;
        impl ProgressSink for NoopSink { fn notify(&self, _: u32, _: &str) {} }
        let b = StubScenarioWorkflowBackend;
        let spec = ComposeSpec { kind: NomKind::ScenarioWorkflow, params: vec![] };
        let flag = InterruptFlag(Arc::new(AtomicBool::new(false)));
        let out = b.compose(&spec, &NoopSink, &flag).unwrap();
        assert_eq!(out.bytes, b"{}");
        assert_eq!(out.mime_type, "application/json");
        assert_eq!(out.cost_cents, 0);
    }
}
