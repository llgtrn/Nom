# Graph-RAG + Agentic-RAG + ReAct for Nom — Design Spec

**Date:** 2026-04-14
**Scope:** extend `nom-intent` with a ReAct loop + 5 grouped capability-tools covering all 30 crates
**Brainstorm shape:** β (extend M8 slice-1 rather than introducing a new `nom-rag` crate)
**Research inputs:** [research/language-analysis/11-graph-rag-agentic-rag-research.md](../../../research/language-analysis/11-graph-rag-agentic-rag-research.md)
**Pre-authorized** per [feedback memory](../../../../..) to proceed with recommended β shape.

## Problem

Nom has 30 workspace crates with rich capabilities (typed-slot resolver, concept-graph closure, MECE validator, glass-box report, community detection) but **no unified retrieval surface** that an LLM agent can drive. Any integration today would force the agent to call 30 tools, which the 2026 ReAct literature unanimously flags as anti-pattern ("don't provide 20+ tools — confuses the agent").

We also already ship **three structural pieces** of the Agentic-RAG pattern without having connected them:
- `NomIntent::Reject(Reason)` from M8 slice-1 = the CRAG weak-retrieval signal
- MECE validator = the Self-RAG critique step
- Glass-box report = the "show your work" artifact

Connecting these into a ReAct-shaped loop is the work.

## Solution shape (β)

**Extend `nom-intent`** (do not create a `nom-rag` crate) with:
1. **5 grouped capability-tools** each wrapping 3-6 underlying crates
2. **`ReActStep` enum** (Thought/Action/Observation/Answer/Reject) as the loop transcript
3. **`classify_with_react()` driver** that runs a bounded loop (≤4 iterations by default, caller-overridable) and returns the full `Vec<ReActStep>` for glass-box surfacing

## Architecture

```
  prose input
       │
       ▼
  ┌─────────────────────────────────────┐
  │ classify_with_react(prose, ctx,     │
  │                     max_iters, llm) │
  └─────────────┬───────────────────────┘
                │
     ┌──────────┼──────────┐
     ▼          ▼          ▼
  Thought     Action     Observation
  (LLM)    (grouped tool) (tool result)
     │          │          │
     └──────────┴──────────┘
                │
                ▼
          Answer(NomIntent::...)
          or Reject(Reason::IterationBudgetExhausted)
```

### The 5 grouped tools

Each tool wraps multiple underlying crates; agent sees only the tool name + its docstring.

1. **`query(subject, kind, depth) -> Vec<Uid>`**
   Covers: `nom-dict::{find_word_v2, find_words_v2_by_kind}`, `nom-concept::resolve_closure`, `nom-graph::detect_communities`, `nom-search`.
   Purpose: retrieve candidate UIDs for a subject, optionally filtered by kind, optionally expanding to depth hops.

2. **`compose(prose, context) -> NomIntent`**
   Covers: `nom-intent::classify` (the existing slice-1), `nom-extract::parse_and_extract`, `nom-concept` MECE pre-check.
   Purpose: propose a Nom (Kind/Symbol/Flow) from prose + retrieved context. Returns existing `NomIntent` enum.

3. **`verify(uid_or_draft) -> VerifyReport`**
   Covers: `nom-verifier`, `nom-security::audit`, MECE validator (ME + CE checks).
   Purpose: judge a proposed Nom. Returns structured `VerifyReport { passed: bool, failures: Vec<Failure>, warnings: Vec<Warning> }`.

4. **`render(uid, target) -> RenderResult`**
   Covers: `nom-codegen`, `nom-llvm`, `nom-app::cmd_app_build`, `nom-media::render`.
   Purpose: emit artifact for a verified Nom. `target` is a string tag (`"llvm-bc"`, `"rust-src"`, `"app-manifest"`, `"avif"`, etc.).

5. **`explain(uid, depth) -> ExplanationReport`**
   Covers: `cmd_build_report`, `LayeredDreamReport`, glass-box outputs.
   Purpose: show-your-work. For "why this Nom?" drill-through from editor to dream report.

### `ReActStep` enum

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReActStep {
    /// LLM-generated reasoning text. Bounded to `max_thought_words` (default 20)
    /// per "cheap reasoning" ReAct tactic.
    Thought(String),
    /// Tool invocation. Each variant is one of the 5 grouped tools.
    Action(AgentAction),
    /// Result of the preceding Action; fed back into the LLM context on the
    /// next iteration.
    Observation(Observation),
    /// Terminal: the loop resolved to a NomIntent.
    Answer(NomIntent),
    /// Terminal: loop exhausted budget or hit a structural error.
    Reject(Reason),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentAction {
    Query { subject: String, kind: Option<String>, depth: usize },
    Compose { prose: String, context: Vec<String> },
    Verify { target: String },
    Render { uid: String, target: String },
    Explain { uid: String, depth: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Observation {
    Candidates(Vec<String>),         // query result
    Proposal(NomIntent),             // compose result
    Verdict { passed: bool, failures: Vec<String>, warnings: Vec<String> },
    Rendered { target: String, bytes_hash: String },
    Explanation { summary: String },
    Error(String),
}
```

### `classify_with_react` driver

```rust
pub struct ReActBudget {
    pub max_iterations: usize,       // default 4 (ReAct literature baseline)
    pub max_thought_words: usize,    // default 20
    pub confidence_threshold: f32,   // inherited from IntentCtx
}

impl Default for ReActBudget {
    fn default() -> Self {
        Self { max_iterations: 4, max_thought_words: 20, confidence_threshold: 0.7 }
    }
}

pub type ReActLlmFn = Box<
    dyn Fn(&str, &[ReActStep]) -> Result<ReActStep, IntentError>
>;

pub fn classify_with_react(
    prose: &str,
    ctx: &IntentCtx,
    budget: &ReActBudget,
    llm: &ReActLlmFn,
    tools: &dyn AgentTools,
) -> Result<Vec<ReActStep>, IntentError>;
```

The driver loop:
1. Invoke `llm(prose, &transcript)` → expects a `Thought` then an `Action` (in 2 consecutive calls) or a terminal `Answer`/`Reject`.
2. Dispatch the `Action` to the matching `tools.query/compose/verify/render/explain` method.
3. Append `Thought`, `Action`, `Observation` to the transcript.
4. If the last step is `Answer` or `Reject`, return transcript. Otherwise continue until `budget.max_iterations` is hit, then return `Reject(Reason::IterationBudgetExhausted)`.

### `AgentTools` trait

Abstracts the 5 tools so tests can swap in stubs deterministically (mirrors the `LlmFn` closure discipline from M8 slice-1):

```rust
pub trait AgentTools {
    fn query(&self, subject: &str, kind: Option<&str>, depth: usize) -> Observation;
    fn compose(&self, prose: &str, context: &[String]) -> Observation;
    fn verify(&self, target: &str) -> Observation;
    fn render(&self, uid: &str, target: &str) -> Observation;
    fn explain(&self, uid: &str, depth: usize) -> Observation;
}
```

Three concrete impls planned (in slice order):
- `StubTools` (slice-2 ship) — deterministic canned responses for tests
- `DictTools` (slice-3) — wires the real 30-crate surfaces behind the 5-tool API
- `InstrumentedTools` (slice-4) — wraps `DictTools` with glass-box logging for the "why this Nom?" editor drill-through

## LazyGraphRAG discipline

**Critical**: community summaries are NOT precomputed at `nom store sync` time. The `query` tool computes them at query time when a community-scoped query hits a cold subtree. Aligns with doc 04 §10.3.1 "no LLM in build path" — summarization is a **query-time agent operation**, not a build step.

Cache: summaries cached in-memory per session, invalidated on `NomDict::is_stale()` == true (integrates with Phase 1 freshness shipped in `60534e4`).

## CRAG loop integration

When `compose` returns `NomIntent::Reject(Reason)` from M8 slice-1, the driver:
1. Records it as an `Observation::Proposal(Reject)`
2. Does NOT terminate (Reject from `compose` is a tool-level weak signal, not a loop terminal)
3. Lets the LLM generate the next `Thought` reasoning about which tool to call instead (typically a broader `query` with different kind filter)
4. Only a `ReActStep::Reject(...)` at the driver level terminates

This is exactly CRAG: retrieve → evaluate → retry differently.

## Self-RAG critique step

The `verify` tool IS the Self-RAG critique. When the LLM is about to emit `Answer(NomIntent)`, it's strongly hinted (via prompt engineering in later slices) to `verify` first. If `verify` returns `failures`, the LLM uses the structured failure list to refine — bounded by iteration budget.

## Components to ship

| Slice | Module | LOC estimate | Tests |
|---|---|---|---|
| **slice-2 (this cycle)** | `nom-intent/src/react.rs` — ReActStep + AgentAction + Observation + AgentTools trait + StubTools + classify_with_react driver | ~300 | ≥6 |
| slice-3 | `nom-intent/src/react_tools.rs` — DictTools real wiring | ~400 | ≥8 |
| slice-4 | `nom-intent/src/react_instrumented.rs` — glass-box logging | ~150 | ≥4 |
| slice-5 | `nom-cli/src/main.rs` — `nom agent classify <prose>` CLI | ~80 | 2 integration |
| slice-6 | `nom-lsp/src/lib.rs` — editor `why-this-Nom?` command | ~120 | 3 integration |

## Testing strategy

**slice-2 tests** (this cycle, all with `StubTools` + closure-based `ReActLlmFn`):

1. `classify_with_react_terminates_on_answer` — LLM returns Answer immediately, transcript has 1 step.
2. `classify_with_react_terminates_on_reject` — LLM returns Reject immediately.
3. `classify_with_react_dispatches_query_action` — LLM emits Thought+Action(Query{..}), driver calls tools.query, Observation appended.
4. `classify_with_react_exhausts_budget_returns_reject` — LLM emits infinite Thought+Action, driver caps at max_iterations and returns Reject(IterationBudgetExhausted).
5. `transcript_round_trips_through_json` — Vec<ReActStep> serde integrity.
6. `all_agent_action_variants_dispatch_to_correct_tool_method` — each AgentAction variant hits the right AgentTools method.

## Sequencing

- **slice-2 ships this cycle** (spec + enum + driver + 6 tests in one commit). Wedge time estimate: ≈1hr.
- slices 3-6 land per subsequent cycles following the 1-wedge-per-cycle pattern.

## Out of scope (deliberate, per research note's "Don't adopt")

- **External vector DB integration** — stick with LadybugDB + existing `entry_embeddings` stub
- **Custom retriever training** — statistical ML stays in M8 slice-1's narrowing path only
- **Full GraphRAG precomputation** — LazyGraphRAG discipline enforces query-time summarization
- **>5 grouped tools** — cap is 5 per ReAct "don't-exceed-20 tools" best practice; strict ceiling
- **LLM in the build path** — ReAct loop is a **pre-build authoring aid**, not a compile-step (doc 04 §10.3.1 fixpoint unchanged)

## Slice-5b clarification (2026-04-14 user directive)

The "real LLM adapter" is NOT limited to OpenAI/Anthropic. Three concrete backends qualify, ordered by preference:

1. **`NomCliAdapter`** — **the nom-compiler itself as oracle**. Uses the shipped `DictTools` + `classify_with_react` + `compose` token-overlap + MECE validator to produce `ReActStep`s deterministically. Zero external dependencies; completely offline. **Default adapter.**
2. **`McpAdapter`** — connects to an MCP stdio server (same protocol as `scripts/gitnexus-mcp.js`), routes the prose + transcript through the MCP tool-call surface. Works with any MCP-compatible client (Claude Code, Codex CLI, Gemini CLI per doc 10 §D). Ships **before** external API adapters.
3. **`RealLlmAdapter`** (optional) — OpenAI/Anthropic structured-output wrapper. Only shipped when external API access is configured; **not required** for the loop to work.

The existing `ReActLlmFn` closure type (`Box<dyn Fn(...) -> Result<ReActStep, _>>`) becomes the **blanket impl** of a new `trait ReActAdapter { fn next_step(...) -> Result<ReActStep, _>; }`. Every concrete adapter implements this trait. `cmd_agent_classify` CLI gains `--adapter {nom-cli, mcp, openai, anthropic, stub}` selecting among them (default `nom-cli`).

**Invariant preserved across all adapters:** output MUST resolve to a registered `NomIntent` variant. Reject-on-invalid ensures no adapter can break slice-1's bounded-output discipline. Tests assert this per-adapter.

**Rationale** (per memory `project_react_llm_adapter_polymorphism.md`): Nom-as-its-own-oracle is the Agentic-RAG win. External LLMs are a nice-to-have, not a prerequisite. Doc 04 §10.3.1 fixpoint discipline holds — the "LLM" can be literally Nom's own deterministic compose+verify, which never enters the build path.

## Spec self-review (2026-04-14)

- ✅ No TBDs / placeholders / vague "TODO" blocks remain
- ✅ Approach β cleanly extends M8 slice-1 without breaking the shipped `NomIntent` enum — ReActStep::Answer wraps NomIntent, doesn't replace it
- ✅ The 5-tool grouping is derived from research literature (§3 of doc 11), not invented
- ✅ LazyGraphRAG discipline + CRAG integration points are explicit
- ✅ Each slice has concrete LOC + test counts; ambiguity absent
- ✅ Out-of-scope list explicitly references doc 04 §10.3.1 fixpoint boundary

Next step: ship slice-2 (`react.rs` module + tests) in the same cycle as this spec commit.
