# Nom API Reference

All APIs listed here are implemented and passing tests. Crate paths use the
`nom-canvas` workspace layout (`nom-canvas/crates/<crate>/src/`).

---

## nom-compose

### `ComposeContext`

`nom_compose::context::ComposeContext`

Context for a hybrid composition request.

```rust
pub struct ComposeContext {
    pub kind: String,
    pub input: String,
    pub tier: ComposeTier,
    pub intent_query: String,
    pub session_id: Option<String>,
}
```

| Method | Description |
|--------|-------------|
| `new(kind, input) -> Self` | Create with `DbDriven` tier and empty intent query |
| `with_tier(tier) -> Self` | Override the dispatch tier |
| `with_intent(query) -> Self` | Attach a natural-language intent string |
| `with_session(id) -> Self` | Bind a session identifier |

---

### `ComposeResult`

`nom_compose::context::ComposeResult`

Result returned by the hybrid composition system.

```rust
pub struct ComposeResult {
    pub artifact: String,
    pub tier_used: ComposeTier,
    pub confidence: f32,
    pub glue_hash: Option<String>,
}
```

`glue_hash` is present only when `tier_used == ComposeTier::AiLeading` and
holds the hash of the generated `.nomx` glue file.

| Method | Description |
|--------|-------------|
| `new(artifact, tier, confidence) -> Self` | Construct with no glue hash |

---

### `ComposeTier`

`nom_compose::context::ComposeTier`

Enum controlling which dispatch path handles the request.

```rust
pub enum ComposeTier {
    DbDriven,   // grammar kinds with Complete status → BackendRegistry
    Provider,   // registered vendor backends → UnifiedDispatcher
    AiLeading,  // unknown kinds → AiGlueOrchestrator
}
```

---

### `CompositionConfig`

`nom_compose::composition::CompositionConfig`

Video render parameters for a named composition.

```rust
pub struct CompositionConfig {
    pub fps: u32,
    pub duration_frames: u32,
    pub width: u32,
    pub height: u32,
    pub default_codec: Option<VideoCodec>,
}
```

Default: 30 fps · 90 frames · 1920×1080 · H264 codec.

---

### `CompositionRegistry`

`nom_compose::composition::CompositionRegistry`

Thread-safe registry that maps string IDs to `CompositionConfig` factory functions.

```rust
pub struct CompositionRegistry { /* ... */ }
```

| Method | Description |
|--------|-------------|
| `new() -> Self` | Create an empty registry |
| `register(id, config_fn) -> Result<(), String>` | Register a config factory; returns `Err` if the ID is already taken |
| `get_config(id) -> Option<CompositionConfig>` | Invoke the factory for the given ID |
| `list_ids() -> Vec<String>` | Return all registered IDs |

---

### `VideoRenderConfig`

`nom_compose::backends::video::VideoRenderConfig`

Controls the video render pipeline.

```rust
pub struct VideoRenderConfig {
    pub concurrency: usize,
    pub ffmpeg_path: String,
    pub on_progress: Option<Box<dyn Fn(RenderProgress) + Send>>,
}
```

Default: `concurrency = 4`, `ffmpeg_path = "ffmpeg"`, no progress callback.

---

### `RenderProgress`

`nom_compose::backends::video::RenderProgress`

Progress snapshot emitted by the video render pipeline after each batch.

```rust
pub struct RenderProgress {
    pub rendered_frames: u32,
    pub encoded_frames: u32,
    pub total_frames: u32,
    pub stage: RenderStage,
    pub elapsed_ms: u64,
}
```

`RenderStage` variants: `Rendering`, `Encoding`, `Muxing`, `Complete`.

| Method | Description |
|--------|-------------|
| `percent() -> f32` | Rendered fraction in `[0.0, 1.0]`; returns 0.0 when `total_frames == 0` |

---

### `CancelSignal`

`nom_compose::cancellation::CancelSignal`

Read-only handle that queries whether cancellation has been requested.

```rust
pub struct CancelSignal(Arc<AtomicBool>);
```

| Method | Description |
|--------|-------------|
| `is_cancelled() -> bool` | Returns `true` after the paired cancel function is called |

---

### `make_cancel_signal()`

`nom_compose::cancellation::make_cancel_signal`

```rust
pub fn make_cancel_signal() -> (CancelSignal, Box<dyn Fn() + Send>)
```

Creates a paired `(CancelSignal, cancel_fn)`. Calling `cancel_fn()` sets the
shared atomic flag; all clones of the `CancelSignal` that share the same
`Arc` allocation see the change immediately.

---

### `HybridResolver`

`nom_compose::hybrid::HybridResolver`

Three-tier resolver: DB-driven -> Provider -> AiLeading.

```rust
pub struct HybridResolver {
    dispatcher: Arc<UnifiedDispatcher>,
    glue_orchestrator: AiGlueOrchestrator,
}
```

| Method | Description |
|--------|-------------|
| `new(dispatcher, glue) -> Self` | Construct from a dispatcher and glue orchestrator |
| `resolve(ctx) -> Result<ComposeResult, String>` | Try all three tiers in order; return the first success |

Tier 1 dispatches directly by kind string. Tier 2 prefixes the kind with
`provider_`. Tier 3 falls through to `AiGlueOrchestrator`.

---

### `ComposeOrchestrator`

`nom_compose::orchestrator::ComposeOrchestrator`

Runs one or more composition requests through a `HybridResolver`.

```rust
pub struct ComposeOrchestrator {
    resolver: Arc<HybridResolver>,
}
```

| Method | Description |
|--------|-------------|
| `new(resolver) -> Self` | Wrap a resolver |
| `run(ctx) -> Result<ComposeResult, String>` | Run a single request |
| `run_parallel(requests) -> Vec<Result<ComposeResult, String>>` | Run N requests; results are index-matched |

---

### `AiGlueOrchestrator`

`nom_compose::glue::AiGlueOrchestrator`

Generates `.nomx` glue code for unknown composition kinds via a `ReActLlmFn`
adapter.

```rust
pub struct AiGlueOrchestrator {
    llm: Box<dyn ReActLlmFn>,
}
```

| Method | Description |
|--------|-------------|
| `new(llm) -> Self` | Construct with any `ReActLlmFn` implementation |
| `generate_blueprint(ctx) -> Result<GlueBlueprint, String>` | Prompt the LLM and return a blueprint |
| `execute_blueprint(bp) -> Result<String, String>` | Execute a blueprint and return the artifact |

---

### `GlueBlueprint`

`nom_compose::glue::GlueBlueprint`

The `.nomx` glue produced by `AiGlueOrchestrator`.

```rust
pub struct GlueBlueprint {
    pub kind: String,
    pub nomx_source: String,
    pub confidence: f32,
    pub llm_name: String,
}
```

---

### `ReActLlmFn`

`nom_compose::glue::ReActLlmFn`

Trait for LLM adapters. Four concrete implementations are provided.

```rust
pub trait ReActLlmFn: Send + Sync {
    fn complete(&self, prompt: &str) -> Result<String, String>;
    fn name(&self) -> &str;
}
```

| Implementor | Description |
|-------------|-------------|
| `StubLlmFn` | Returns a hardcoded response; used in tests |
| `NomCliLlmFn` | Spawns the `nom` binary as a deterministic oracle |
| `McpLlmFn` | Delegates via an MCP tool call |
| `RealLlmFn` | External API endpoint |

---

### `FlowGraph`

`nom_compose::flow_graph::FlowGraph`

Directed flow graph where nodes are backend dispatch targets and edges carry
labels.

```rust
pub struct FlowGraph {
    pub nodes: HashMap<String, FlowNode>,
    pub edges: Vec<FlowEdge>,
    pub version: u32,
}
```

| Method | Description |
|--------|-------------|
| `new() -> Self` | Empty graph |
| `add_node(node)` | Insert a node keyed by `node.id` |
| `add_edge(edge) -> Result<(), String>` | Insert an edge; errors if either endpoint is not registered |
| `topological_order() -> Vec<String>` | Kahn's algorithm; returns node IDs in execution order |

---

### `FlowNode`

`nom_compose::flow_graph::FlowNode`

A node in the flow graph.

```rust
pub struct FlowNode {
    pub id: String,
    pub kind: FlowNodeKind,   // Input | Transform | Output | Conditional
    pub backend_kind: String, // runtime string — no closed enum
    pub version: u32,
}
```

---

### `FlowEdge`

`nom_compose::flow_graph::FlowEdge`

A directed, labelled edge between two `FlowNode` IDs.

```rust
pub struct FlowEdge {
    pub from_id: String,
    pub to_id: String,
    pub label: String,
}
```

---

## nom-intent

### `SkillRouter`

`nom_intent::skill_router::SkillRouter`

Registry of available skills supporting registration and fuzzy lookup.

```rust
pub struct SkillRouter {
    skills: Vec<SkillDefinition>,
}
```

| Method | Description |
|--------|-------------|
| `new() -> Self` | Empty registry |
| `register(skill)` | Append a skill definition |
| `find_by_id(id) -> Option<&SkillDefinition>` | Exact ID match, first registered |
| `find_by_query(query) -> Vec<&SkillDefinition>` | Case-insensitive substring match on name or description |
| `len() -> usize` | Number of registered skills |
| `is_empty() -> bool` | True when the registry is empty |

---

### `SkillDefinition`

`nom_intent::skill_router::SkillDefinition`

A skill that can be looked up and invoked by the intent layer.

```rust
pub struct SkillDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub input_schema: String,   // JSON Schema string
    pub output_schema: String,  // JSON Schema string
}
```

---

## nom-graph

### `ExecutionEngine`

`nom_graph::execution::ExecutionEngine`

Runs a DAG in topological order with optional caching and cancellation.

```rust
pub struct ExecutionEngine {
    pub cache: Box<dyn ExecutionCache>,
    pub changed_flags: ChangedFlags,
    pub cancel_flag: Arc<AtomicBool>,
}
```

| Method | Description |
|--------|-------------|
| `new(cache) -> Self` | Construct with any `ExecutionCache` implementation |
| `cancel()` | Set the cancel flag; the execution loop checks it each node |
| `execute(dag, nodes) -> Vec<NodeOutput>` | Run all nodes in topological order; returns per-node outputs |
| `collect_ancestors(dag, node, visited) -> Vec<NodeId>` | DFS transitive ancestor walk for cache-key derivation |

---

## nom-panels (right panel)

### `IntentPreviewCard`

`nom_panels::right::intent_preview::IntentPreviewCard`

Shows the classified kind and confidence score before a composition request
is dispatched.

```rust
pub struct IntentPreviewCard {
    pub query: String,
    pub classified_kind: String,
    pub confidence: f32,
    pub top_alternatives: Vec<(String, f32)>,
    pub purpose_clause: Option<String>,
}
```

| Method | Description |
|--------|-------------|
| `new(query, kind, confidence) -> Self` | Construct |
| `with_alternatives(alts) -> Self` | Attach ranked alternative kinds |
| `with_purpose(clause) -> Self` | Attach extracted purpose clause |
| `confidence_label() -> &str` | Returns `"high"` (≥ 0.8), `"medium"` (≥ 0.5), or `"low"` |

---

### `AiReviewCard`

`nom_panels::right::intent_preview::AiReviewCard`

Shows the generated `.nomx` glue for user accept/reject before it is committed.

```rust
pub struct AiReviewCard {
    pub glue_hash: String,
    pub kind: String,
    pub nomx_preview: String, // first 200 chars of generated .nomx
    pub tier: String,         // "ai_leading" | "provider" | "db_driven"
    pub accepted: bool,
}
```

| Method | Description |
|--------|-------------|
| `new(hash, kind, preview) -> Self` | Construct in unaccepted state with `tier = "ai_leading"` |
| `accept()` | Set `accepted = true` |
| `reject()` | Set `accepted = false` |
