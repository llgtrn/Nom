# Nom Universal Composer — Platform Leap Design

**Date:** 2026-04-18 | **Status:** Approved | **Wave:** AI-Composer

## 1. Context & Goal

Nom is evolving from a single-user IDE into a composable platform. The highest-ROAS trajectory is AI-native automation — the grammar DB becomes a compounding moat where every AI-invoked composition call trains the next. This spec defines the Platform Leap: wiring 10 upstream patterns into the existing 14-crate workspace so the `POST /compose` endpoint becomes the monetization surface.

Foundation stays Nom Language. No foreign identities in public API. All additions are additive to existing interfaces.

## 2. Architecture

Three layers above nom-compiler core:

```
+-----------------------------------------------------+
|  POST /compose  (nom-cli HTTP server, tokio-axum)   |
+-----------------------------------------------------+
|  IntentResolver v2                                   |
|  lexical -> Qdrant HNSW semantic -> classify_react() |
+-----------------------------------------------------+
|  HybridResolver (Tier 1 -> 2 -> 3)                  |
|  DB-driven | Provider-driven | AI-Glue              |
+--------------+--------------+-----------------------+
| BackendReg.  | UnifiedDisp. | AiGlueOrchestrator    |
| (existing)   | MediaVendor  | Candle + Wasmtime      |
|              | credentials  | DeerFlow middleware    |
+--------------+--------------+-----------------------+
         nom-compiler (29 crates, UNCHANGED)
                  nomdict.db (truth)
```

## 3. Ten Pattern Integrations

| Pattern | Source repo | Replaces / Augments | Impact |
|---|---|---|---|
| Candle in-process ML | `upstreams/candle/` | `NomCliAdapter` stub → real Phi-3/Gemma-2B inference, no subprocess | `ReActLlmFn` runs inside the binary |
| Qdrant HNSW semantic | `upstreams/qdrant/` | BM25 in `IntentResolver` → HNSW vector search over grammar.kinds embeddings | Sub-10ms semantic kind match |
| Wasmtime WASM sandbox | `upstreams/wasmtime/` | JS AST `eval_expr` stub → compiled WASM module per glue | Safe, fast, zero subprocess |
| DeerFlow StepMiddleware | `services/deer-flow-main/` | Bare `dispatch` → before/after hooks per compose step | Telemetry, retry, cost tracking free |
| Refly FlowNode graph | `upstreams/refly-main/` | Linear `ComposeOrchestrator` → typed node graph with version control | Multi-step compositions as first-class entities |
| AgentScope critique loop | `services/agentscope-main/` | Single-pass AI glue → propose → critique → refine cycle | Quality gate before sandbox execution |
| ToolJet widget registry | `services/ToolJet-develop/` | Hardcoded panel types → declarative 72+ widget kinds in DB | Node palette fully DB-driven |
| Polars lazy frames | `upstreams/polars/` | `data_query` backend returns rows → columnar Arrow lazy plan | 10-100x data transform speed |
| Open-Higgsfield registry | `services/Open-Higgsfield-AI/` | Empty `MediaVendor` list → 200+ model polymorphic registry | First real video/image provider |
| Bolt.new SwitchableStream | `upstreams/bolt.new-main/` | Blocking glue synthesis → streaming token-by-token `.nomx` output | Live preview as AI writes glue |

## 4. Data Flow

```
User / AI agent: POST /compose {"kind": "video", "input": "sunset timelapse"}
        |
        v
IntentResolver v2
  1. lexical scan grammar.kinds (exact) -> miss
  2. Qdrant HNSW k=5 over kinds embeddings -> "video" 0.97 sim
  3. kind = BackendKind::Video, tier = Db or Provider
        |
        v
HybridResolver
  Tier1: grammar.kinds WHERE word='video' AND status='Complete' -> found
  -> BackendRegistry::dispatch_with_context(ComposeContext)
  -> DeerFlow StepMiddleware wraps: before_step() -> dispatch -> after_step()
        |
        +- Tier1 hit: BackendRegistry calls video backend
        |         Open-Higgsfield MediaVendor -> generation request
        |         SwitchableStream -> streaming artifact bytes
        |
        +- Tier1 miss -> Tier2: UnifiedDispatcher ProviderRouter
              +- Tier2 miss -> Tier3: AiGlueOrchestrator
                    AgentScope: propose .nomx -> critique -> refine
                    Candle Phi-3: ReActLlmFn inference (in-process)
                    Wasmtime: compile + execute refined .nomx
                    GlueCache: store Transient entry
                    "intended to <purpose>" detected -> promote to Partial
        |
        v
ComposeResult { artifact, tier_used, glue_hash, streaming_rx }
        |
        v
DeerFlow after_step(): record latency, cost, token count -> nomdict.db
```

## 5. New Modules (additive only)

```
nom-compose/src/
  intent_v2.rs          <- Qdrant HNSW replaces BM25 path in IntentResolver
  middleware.rs          <- DeerFlow StepMiddleware trait + registry
  flow_graph.rs          <- Refly FlowNode typed node graph with version control
  critique.rs            <- AgentScope propose/critique/refine loop
  streaming.rs           <- Bolt.new SwitchableStream for live .nomx preview

nom-compiler-bridge/src/
  candle_adapter.rs      <- Candle BackendDevice -> ReActLlmFn impl (in-process Phi-3)
  wasm_sandbox.rs        <- Wasmtime Store<T> + Linker -- replaces JS AST eval_expr

nom-cli/src/
  serve.rs               <- tokio-axum POST /compose endpoint
```

nom-gpui node palette driven by DB SELECT on grammar.kinds (no hardcoded enums).
No changes to nom-compiler 29 crates.

## 6. Grammar Flywheel

Every compose call feeds back to nomdict.db:

- Tier1 hit: increment grammar.kinds.use_count; use_count >= 10 -> auto-promote to Complete
- Tier3 hit: GlueCache Transient entry created; user Accept -> DictWriter.insert_partial_entry() immediately; 3 uses + 0.7 confidence -> background ticker promotes to Partial
- DeerFlow after_step() writes latency/cost/token rows; Polars lazy frame aggregates daily stats; cost-per-kind visible in settings panel

Grammar DB grows automatically. No human curation required for the common path.

## 7. API Surface

```
POST /compose
{
  "input": "sunset timelapse 30s",
  "kind_hint": "video",          // optional - IntentResolver used if absent
  "vendor_hint": "higgsfield",   // optional - ProviderRouter used if absent
  "stream": true                 // Bolt.new SwitchableStream
}

Streaming response (text/event-stream):
  data: {"tier": "provider", "kind": "video", "progress": 0.12}
  data: {"artifact_url": "...", "glue_hash": null, "cost_tokens": 0}

Non-streaming response:
  {
    "artifact": "<bytes|url>",
    "tier": "ai_glue",
    "glue_hash": "abc123",
    "purpose_clause": "intended to generate a timelapse video from scene description",
    "promote_action": "POST /promote/abc123"
  }
```

## 8. Implementation Order (9 steps)

1. `nom-cli/src/serve.rs` — tokio-axum skeleton, POST /compose -> placeholder 200
2. `nom-compiler-bridge/src/candle_adapter.rs` — Candle BackendDevice::Cpu + ReActLlmFn impl
3. `nom-compiler-bridge/src/wasm_sandbox.rs` — Wasmtime Store + Linker, replace eval_expr call site
4. `nom-compose/src/intent_v2.rs` — Qdrant client + HNSW search over kinds embeddings
5. `nom-compose/src/middleware.rs` — StepMiddleware trait + MiddlewareRegistry
6. `nom-compose/src/critique.rs` — AgentScope propose/critique/refine (3-round cap)
7. `nom-compose/src/streaming.rs` — SwitchableStream wrapping AiGlueOrchestrator
8. `nom-compose/src/flow_graph.rs` — FlowNode + FlowEdge + version control
9. Wire all into HybridResolver::resolve() + ComposeOrchestrator::run()

## 9. Success Criteria

- `POST /compose {"input": "sunset timelapse 30s", "kind_hint": "video"}` returns artifact or streaming progress
- Tier3 AI glue path produces .nomx with `intended to` clause, GlueCache stores it
- User Accept -> grammar.kinds gains Partial entry, visible in node palette on next load
- DeerFlow after_step records latency + cost rows in nomdict.db
- All 10 upstream patterns have corresponding tests (>= 2 tests each = 20+ new tests)
