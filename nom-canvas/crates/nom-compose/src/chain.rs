#![deny(unsafe_code)]

use std::ops::BitOr;
use std::sync::Arc;

use serde_json::Value;

use crate::context::ComposeContext;
use crate::dispatch::BackendRegistry;
use crate::glue::ReActLlmFn;

// ============================================================================
// NEW TYPED RUNNABLE TRAITS (LangChain-inspired)
// ============================================================================

/// A composable unit with typed input and output.
///
/// Inspired by LangChain's `Runnable[Input, Output]` protocol, this trait
/// turns every component—prompts, models, tools, parsers—into a composable,
/// streamable unit of work.
pub trait Runnable<I, O>: Send + Sync {
    /// Execute this unit with the given input.
    fn run(&self, input: I) -> O;

    /// Stream output as an iterator of chunks.
    ///
    /// MVP: most implementations return a single-item iterator.
    /// Full streaming passthrough requires every step to support transform.
    fn stream(&self, input: I) -> Box<dyn Iterator<Item = O> + Send + '_>;
}

/// A tool that can be bound to a [`Runnable`] for agentic use.
pub trait Tool: Send + Sync {
    /// Unique tool name.
    fn name(&self) -> &str;
    /// Human-readable description.
    fn description(&self) -> &str;
    /// JSON schema describing the tool's input parameters.
    fn schema(&self) -> Value;
    /// Execute the tool with the given input string.
    fn execute(&self, input: &str) -> Result<String, String>;
}

/// A runnable with one or more tools bound to it.
pub struct BoundRunnable<I, O> {
    inner: Box<dyn Runnable<I, O>>,
    tools: Vec<Box<dyn Tool>>,
}

impl<I, O> BoundRunnable<I, O> {
    /// Wrap an existing runnable with a tool.
    pub fn new(inner: Box<dyn Runnable<I, O>>, tool: Box<dyn Tool>) -> Self {
        Self {
            inner,
            tools: vec![tool],
        }
    }
}

impl<I, O> Runnable<I, O> for BoundRunnable<I, O> {
    fn run(&self, input: I) -> O {
        // MVP: delegate to inner runnable. Future agentic loop can inspect
        // tool schemas and route LLM tool-call requests here.
        self.inner.run(input)
    }

    fn stream(&self, input: I) -> Box<dyn Iterator<Item = O> + Send + '_> {
        self.inner.stream(input)
    }
}

/// Sequential composition of two runnables (`first | second`).
pub struct RunnableSequence<I, M, O> {
    first: Box<dyn Runnable<I, M>>,
    second: Box<dyn Runnable<M, O>>,
}

impl<I, M, O> RunnableSequence<I, M, O> {
    /// Create a new sequence from two runnables.
    pub fn new(first: Box<dyn Runnable<I, M>>, second: Box<dyn Runnable<M, O>>) -> Self {
        Self { first, second }
    }
}

impl<I, M, O: Send> Runnable<I, O> for RunnableSequence<I, M, O> {
    fn run(&self, input: I) -> O {
        let mid = self.first.run(input);
        self.second.run(mid)
    }

    fn stream(&self, input: I) -> Box<dyn Iterator<Item = O> + Send + '_> {
        // MVP: full streaming passthrough is a follow-up.
        Box::new(std::iter::once(self.run(input)))
    }
}

/// `a | b` syntax for chaining two boxed runnables.
impl<I: 'static, M: 'static, O: 'static + Send> BitOr<Box<dyn Runnable<M, O>>> for Box<dyn Runnable<I, M>> {
    type Output = Box<dyn Runnable<I, O>>;
    fn bitor(self, rhs: Box<dyn Runnable<M, O>>) -> Self::Output {
        Box::new(RunnableSequence::new(self, rhs))
    }
}

impl<I, O: Send> Runnable<I, O> for Box<dyn Runnable<I, O>> {
    fn run(&self, input: I) -> O {
        (**self).run(input)
    }
    fn stream(&self, input: I) -> Box<dyn Iterator<Item = O> + Send + '_> {
        (**self).stream(input)
    }
}

/// A pipeline of [`Runnable`] steps executed in order.
pub struct Chain {
    steps: Vec<Box<dyn Runnable<String, String>>>,
}

impl Chain {
    /// Create an empty chain.
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// Append a step to this chain.
    pub fn add_step(mut self, step: Box<dyn Runnable<String, String>>) -> Self {
        self.steps.push(step);
        self
    }
}

impl Default for Chain {
    fn default() -> Self {
        Self::new()
    }
}

impl Runnable<String, String> for Chain {
    fn run(&self, input: String) -> String {
        let mut output = input;
        for step in &self.steps {
            output = step.run(output);
        }
        output
    }

    fn stream(&self, input: String) -> Box<dyn Iterator<Item = String> + Send + '_> {
        Box::new(std::iter::once(self.run(input)))
    }
}

// ============================================================================
// LEGACY COMPOSE RUNNABLE (backward compatibility)
// ============================================================================

/// A composable unit that can be chained with other units into a pipeline.
///
/// This is the original Nom trait that carries a [`ComposeContext`] on every
/// call. New code should prefer the typed [`Runnable<I, O>`].
pub trait ComposeRunnable: Send + Sync {
    /// Execute this unit with the given input and context.
    fn run(&self, input: &str, ctx: &ComposeContext) -> Result<String, String>;

    /// Chain this runnable with another, producing a [`ComposeChain`].
    fn chain(self, next: Box<dyn ComposeRunnable>) -> ComposeChain
    where
        Self: Sized + 'static,
    {
        ComposeChain {
            steps: vec![Box::new(self), next],
        }
    }
}

/// A pipeline of [`ComposeRunnable`] steps executed in order.
pub struct ComposeChain {
    steps: Vec<Box<dyn ComposeRunnable>>,
}

impl ComposeChain {
    /// Create an empty chain.
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// Append a step to this chain.
    pub fn add_step(mut self, step: Box<dyn ComposeRunnable>) -> Self {
        self.steps.push(step);
        self
    }
}

impl Default for ComposeChain {
    fn default() -> Self {
        Self::new()
    }
}

impl ComposeRunnable for ComposeChain {
    fn run(&self, input: &str, ctx: &ComposeContext) -> Result<String, String> {
        let mut output = input.to_string();
        for step in &self.steps {
            output = step.run(&output, ctx)?;
        }
        Ok(output)
    }

    fn chain(self, next: Box<dyn ComposeRunnable>) -> ComposeChain {
        let mut steps = self.steps;
        steps.push(next);
        ComposeChain { steps }
    }
}

// ============================================================================
// Runnable implementations for existing structs
// ============================================================================

// ---------------------------------------------------------------------------
// IntentRunnable
// ---------------------------------------------------------------------------

/// Resolves free-text input to a grammar kind via [`nom_canvas_intent::IntentResolver`].
pub struct IntentRunnable {
    grammar_kinds: Vec<String>,
}

impl IntentRunnable {
    pub fn new(grammar_kinds: Vec<String>) -> Self {
        Self { grammar_kinds }
    }
}

impl ComposeRunnable for IntentRunnable {
    fn run(&self, input: &str, _ctx: &ComposeContext) -> Result<String, String> {
        let resolver = nom_canvas_intent::IntentResolver::new(self.grammar_kinds.clone());
        let resolved = resolver.resolve(input);
        match resolved.best_kind {
            Some(kind) => Ok(kind),
            None => Err("intent resolution failed: no matching kind".to_string()),
        }
    }
}

impl Runnable<String, String> for IntentRunnable {
    fn run(&self, input: String) -> String {
        let resolver = nom_canvas_intent::IntentResolver::new(self.grammar_kinds.clone());
        match resolver.resolve(&input).best_kind {
            Some(kind) => kind,
            None => "intent resolution failed: no matching kind".to_string(),
        }
    }

    fn stream(&self, input: String) -> Box<dyn Iterator<Item = String> + Send + '_> {
        Box::new(std::iter::once(Runnable::run(self, input)))
    }
}

// ---------------------------------------------------------------------------
// RagRunnable
// ---------------------------------------------------------------------------

/// Retrieves context from a graph via [`nom_canvas_graph::GraphRagRetriever`].
pub struct RagRunnable {
    dag: Arc<nom_canvas_graph::Dag>,
    top_k: usize,
    max_hops: usize,
}

impl RagRunnable {
    pub fn new(dag: Arc<nom_canvas_graph::Dag>, top_k: usize, max_hops: usize) -> Self {
        Self {
            dag,
            top_k,
            max_hops,
        }
    }
}

impl ComposeRunnable for RagRunnable {
    fn run(&self, input: &str, _ctx: &ComposeContext) -> Result<String, String> {
        let retriever = nom_canvas_graph::GraphRagRetriever::new(&self.dag);
        let query = nom_canvas_graph::node_vec(input);
        let results = retriever.retrieve(&query, self.top_k, self.max_hops);
        Ok(format!("rag:{}:{}", results.len(), input))
    }
}

impl Runnable<String, String> for RagRunnable {
    fn run(&self, input: String) -> String {
        let retriever = nom_canvas_graph::GraphRagRetriever::new(&self.dag);
        let query = nom_canvas_graph::node_vec(&input);
        let results = retriever.retrieve(&query, self.top_k, self.max_hops);
        format!("rag:{}:{}", results.len(), input)
    }

    fn stream(&self, input: String) -> Box<dyn Iterator<Item = String> + Send + '_> {
        Box::new(std::iter::once(Runnable::run(self, input)))
    }
}

// ---------------------------------------------------------------------------
// LlmRunnable
// ---------------------------------------------------------------------------

/// Wraps an LLM adapter to produce text completions.
pub struct LlmRunnable {
    llm: Arc<dyn ReActLlmFn>,
}

impl LlmRunnable {
    pub fn new(llm: Arc<dyn ReActLlmFn>) -> Self {
        Self { llm }
    }
}

impl ComposeRunnable for LlmRunnable {
    fn run(&self, input: &str, _ctx: &ComposeContext) -> Result<String, String> {
        self.llm.complete(input)
    }
}

impl Runnable<String, String> for LlmRunnable {
    fn run(&self, input: String) -> String {
        match self.llm.complete(&input) {
            Ok(response) => response,
            Err(e) => e,
        }
    }

    fn stream(&self, input: String) -> Box<dyn Iterator<Item = String> + Send + '_> {
        Box::new(std::iter::once(Runnable::run(self, input)))
    }
}

// ---------------------------------------------------------------------------
// ValidateRunnable
// ---------------------------------------------------------------------------

/// Validates input before downstream processing.
pub struct ValidateRunnable;

impl ValidateRunnable {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ValidateRunnable {
    fn default() -> Self {
        Self::new()
    }
}

impl ComposeRunnable for ValidateRunnable {
    fn run(&self, input: &str, _ctx: &ComposeContext) -> Result<String, String> {
        #[cfg(feature = "ai-orchestration")]
        {
            match nom_concept::stage1_tokenize(input) {
                Ok(stream) if !stream.toks.is_empty() => Ok(input.to_string()),
                Ok(_) => Err("validation failed: empty token stream".to_string()),
                Err(_) => Err("validation failed: tokenization error".to_string()),
            }
        }
        #[cfg(not(feature = "ai-orchestration"))]
        {
            if input.trim().is_empty() {
                return Err("validation failed: empty input".to_string());
            }
            Ok(input.to_string())
        }
    }
}

impl Runnable<String, String> for ValidateRunnable {
    fn run(&self, input: String) -> String {
        #[cfg(feature = "ai-orchestration")]
        {
            match nom_concept::stage1_tokenize(&input) {
                Ok(stream) if !stream.toks.is_empty() => input,
                Ok(_) => "validation failed: empty token stream".to_string(),
                Err(_) => "validation failed: tokenization error".to_string(),
            }
        }
        #[cfg(not(feature = "ai-orchestration"))]
        {
            if input.trim().is_empty() {
                return "validation failed: empty input".to_string();
            }
            input
        }
    }

    fn stream(&self, input: String) -> Box<dyn Iterator<Item = String> + Send + '_> {
        Box::new(std::iter::once(Runnable::run(self, input)))
    }
}

// ---------------------------------------------------------------------------
// DispatchRunnable
// ---------------------------------------------------------------------------

/// Dispatches to a [`BackendRegistry`] based on the compose context kind.
pub struct DispatchRunnable {
    registry: Arc<BackendRegistry>,
    kind: Option<String>,
}

impl DispatchRunnable {
    pub fn new(registry: Arc<BackendRegistry>) -> Self {
        Self { registry, kind: None }
    }

    /// Set the dispatch kind for the typed [`Runnable`] implementation.
    pub fn with_kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = Some(kind.into());
        self
    }
}

impl ComposeRunnable for DispatchRunnable {
    fn run(&self, input: &str, ctx: &ComposeContext) -> Result<String, String> {
        self.registry.dispatch(&ctx.kind, input, &|_| {})
    }
}

impl Runnable<String, String> for DispatchRunnable {
    fn run(&self, input: String) -> String {
        match &self.kind {
            Some(kind) => self
                .registry
                .dispatch(kind, &input, &|_| {})
                .unwrap_or_else(|e| e),
            None => "error: DispatchRunnable has no kind configured".to_string(),
        }
    }

    fn stream(&self, input: String) -> Box<dyn Iterator<Item = String> + Send + '_> {
        Box::new(std::iter::once(Runnable::run(self, input)))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::{BackendRegistry, NoopBackend};
    use crate::glue::StubLlmFn;

    // -----------------------------------------------------------------------
    // Legacy ComposeRunnable tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_compose_chain_runs_single_step() {
        let chain = ComposeChain::new().add_step(Box::new(ValidateRunnable::new()));
        let result = ComposeRunnable::run(&chain, "hello", &ComposeContext::new("test", "input"))
            .unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_compose_chain_runs_multiple_steps() {
        let chain = ComposeChain::new()
            .add_step(Box::new(ValidateRunnable::new()))
            .add_step(Box::new(LlmRunnable::new(Arc::new(StubLlmFn {
                response: "modified".to_string(),
            }))));
        let result = ComposeRunnable::run(&chain, "hello", &ComposeContext::new("test", "input"))
            .unwrap();
        assert_eq!(result, "modified");
    }

    #[test]
    fn test_intent_runnable_resolves() {
        let runnable = IntentRunnable::new(vec![
            "video".to_string(),
            "audio".to_string(),
            "image".to_string(),
        ]);
        let result = ComposeRunnable::run(
            &runnable,
            "render a video clip",
            &ComposeContext::new("test", "input"),
        )
        .unwrap();
        assert_eq!(result, "video");
    }

    #[test]
    fn test_llm_runnable_completes() {
        let llm = Arc::new(StubLlmFn {
            response: "completion".to_string(),
        });
        let runnable = LlmRunnable::new(llm);
        let result = ComposeRunnable::run(&runnable, "prompt", &ComposeContext::new("test", "input"))
            .unwrap();
        assert_eq!(result, "completion");
    }

    #[test]
    fn test_validate_runnable_passes() {
        let runnable = ValidateRunnable::new();
        let result = ComposeRunnable::run(
            &runnable,
            "valid input",
            &ComposeContext::new("test", "input"),
        )
        .unwrap();
        assert_eq!(result, "valid input");
    }

    #[test]
    fn test_validate_runnable_fails_empty() {
        let runnable = ValidateRunnable::new();
        let result = ComposeRunnable::run(&runnable, "   ", &ComposeContext::new("test", "input"));
        assert!(result.is_err());
    }

    #[test]
    fn test_dispatch_runnable_routes() {
        let mut registry = BackendRegistry::new();
        registry.register(Box::new(NoopBackend::new("image")));
        let runnable = DispatchRunnable::new(Arc::new(registry));
        let ctx = ComposeContext::new("image", "scene");
        let result = ComposeRunnable::run(&runnable, "scene", &ctx).unwrap();
        assert_eq!(result, "image:scene");
    }

    #[test]
    fn test_rag_runnable_retrieves() {
        let dag = nom_canvas_graph::Dag::new();
        let runnable = RagRunnable::new(Arc::new(dag), 3, 1);
        let result = ComposeRunnable::run(&runnable, "query", &ComposeContext::new("test", "input"))
            .unwrap();
        assert!(result.starts_with("rag:"));
    }

    #[test]
    fn test_full_ai_chain_end_to_end() {
        let llm = Arc::new(StubLlmFn {
            response: String::new(),
        });
        let dag = Arc::new(nom_canvas_graph::Dag::new());
        let mut registry = BackendRegistry::new();
        registry.register(Box::new(NoopBackend::new("image")));
        registry.register(Box::new(NoopBackend::new("logo")));

        let chain = ComposeChain::new()
            .add_step(Box::new(IntentRunnable::new(vec![
                "image".to_string(),
                "logo".to_string(),
            ])))
            .add_step(Box::new(RagRunnable::new(dag, 3, 2)))
            .add_step(Box::new(LlmRunnable::new(llm)))
            .add_step(Box::new(ValidateRunnable::new()))
            .add_step(Box::new(DispatchRunnable::new(Arc::new(registry))));

        let ctx = ComposeContext::new("image", "make a logo");
        let result = ComposeRunnable::run(&chain, "make a logo", &ctx).unwrap();

        assert!(!result.is_empty(), "output must not be empty");
        assert!(
            result.contains("define") || result.contains("->") || result.contains("compose"),
            "output must contain .nomx-like structure, got: {}",
            result
        );
    }

    // -----------------------------------------------------------------------
    // New typed Runnable tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_typed_chain_runs_single_step() {
        let chain = Chain::new().add_step(Box::new(ValidateRunnable::new()));
        let result = Runnable::run(&chain, "hello".to_string());
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_typed_chain_runs_multiple_steps() {
        let chain = Chain::new()
            .add_step(Box::new(ValidateRunnable::new()))
            .add_step(Box::new(LlmRunnable::new(Arc::new(StubLlmFn {
                response: "modified".to_string(),
            }))));
        let result = Runnable::run(&chain, "hello".to_string());
        assert_eq!(result, "modified");
    }

    #[test]
    fn test_runnable_sequence_via_bitor() {
        let first: Box<dyn Runnable<String, String>> = Box::new(ValidateRunnable::new());
        let second: Box<dyn Runnable<String, String>> = Box::new(LlmRunnable::new(Arc::new(
            StubLlmFn {
                response: "world".to_string(),
            },
        )));
        let seq = first | second;
        let result = Runnable::run(&seq, "hello".to_string());
        assert_eq!(result, "world");
    }

    #[test]
    fn test_runnable_stream_returns_single_item() {
        let runnable = LlmRunnable::new(Arc::new(StubLlmFn {
            response: "chunk".to_string(),
        }));
        let mut stream = runnable.stream("prompt".to_string());
        assert_eq!(stream.next(), Some("chunk".to_string()));
        assert_eq!(stream.next(), None);
    }

    #[test]
    fn test_typed_dispatch_runnable_with_kind() {
        let mut registry = BackendRegistry::new();
        registry.register(Box::new(NoopBackend::new("image")));
        let runnable = DispatchRunnable::new(Arc::new(registry)).with_kind("image");
        let result = Runnable::run(&runnable, "scene".to_string());
        assert_eq!(result, "image:scene");
    }

    #[test]
    fn test_typed_dispatch_runnable_without_kind_returns_error() {
        let registry = BackendRegistry::new();
        let runnable = DispatchRunnable::new(Arc::new(registry));
        let result = Runnable::run(&runnable, "scene".to_string());
        assert!(result.contains("no kind configured"));
    }

    #[test]
    fn test_bound_runnable_delegates() {
        let inner = Box::new(LlmRunnable::new(Arc::new(StubLlmFn {
            response: "delegated".to_string(),
        })));
        struct DummyTool;
        impl Tool for DummyTool {
            fn name(&self) -> &str {
                "dummy"
            }
            fn description(&self) -> &str {
                "dummy tool"
            }
            fn schema(&self) -> Value {
                Value::Object(Default::default())
            }
            fn execute(&self, _input: &str) -> Result<String, String> {
                Ok("ok".to_string())
            }
        }
        let bound = BoundRunnable::new(inner, Box::new(DummyTool));
        let result = Runnable::run(&bound, "test".to_string());
        assert_eq!(result, "delegated");
    }

    #[test]
    fn test_typed_intent_runnable() {
        let runnable = IntentRunnable::new(vec![
            "video".to_string(),
            "audio".to_string(),
            "image".to_string(),
        ]);
        let result = Runnable::run(&runnable, "render a video clip".to_string());
        assert_eq!(result, "video");
    }

    #[test]
    fn test_typed_validate_runnable_passes() {
        let runnable = ValidateRunnable::new();
        let result = Runnable::run(&runnable, "valid input".to_string());
        assert_eq!(result, "valid input");
    }

    #[test]
    fn test_typed_validate_runnable_fails_empty() {
        let runnable = ValidateRunnable::new();
        let result = Runnable::run(&runnable, "   ".to_string());
        assert!(result.contains("validation failed"));
    }

    #[test]
    fn test_typed_llm_runnable_completes() {
        let llm = Arc::new(StubLlmFn {
            response: "completion".to_string(),
        });
        let runnable = LlmRunnable::new(llm);
        let result = Runnable::run(&runnable, "prompt".to_string());
        assert_eq!(result, "completion");
    }

    #[test]
    fn test_typed_rag_runnable_retrieves() {
        let dag = nom_canvas_graph::Dag::new();
        let runnable = RagRunnable::new(Arc::new(dag), 3, 1);
        let result = Runnable::run(&runnable, "query".to_string());
        assert!(result.starts_with("rag:"));
    }

    #[test]
    fn test_typed_chain_bind_compiles() {
        let runnable = ValidateRunnable::new();
        struct DummyTool;
        impl Tool for DummyTool {
            fn name(&self) -> &str {
                "dummy"
            }
            fn description(&self) -> &str {
                "dummy tool"
            }
            fn schema(&self) -> Value {
                Value::Object(Default::default())
            }
            fn execute(&self, _input: &str) -> Result<String, String> {
                Ok("ok".to_string())
            }
        }
        let _bound = BoundRunnable::new(Box::new(runnable), Box::new(DummyTool));
    }
}
