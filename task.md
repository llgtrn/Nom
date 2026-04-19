# Nom — Executor Task Checklist

**Date:** 2026-04-19 | **HEAD:** `2da0748` | **Tests:** ~10,630 (canvas:~10,548 + compiler:~82)

**Current:** Wave AD ✅ COMPLETE (4/5 demos) | **Next:** Wave AE 🔲 OPEN | **Wave AF** 🆕 LAUNCHED (16-repo parallel audit)

---

## Per-Crate Test Counts

| Crate | Tests |
|---|---|
| nom-gpui | 864 |
| nom-canvas-core | 795 |
| nom-compose | 1,351 |
| nom-canvas-intent | 591 |
| nom-canvas-graph | 570 |
| nom-lint | 519 |
| nom-blocks | 560 |
| nom-collab | 546 |
| nom-editor | 620 |
| nom-compiler-bridge | 553 |
| nom-panels | 601 |
| nom-theme | 556 |
| nom-memoize | 468 |
| nom-telemetry | 500 |
| nom-canvas-cli | 482 |
| **Canvas Total** | **~10,619** |
| nom-concept | 60+ |
| nom-llvm | 15+ |
| nom-grammar | 7+ |
| **Compiler Total** | **~82** |
| **GRAND TOTAL** | **~10,701** |

---

## Open Items — Wave AD (End-to-End Demos)

- [x] **AD-RENDER-DEMO** — `cargo run --example window_first_paint` shows red quad on GPU ✅
- [x] **AD-VIDEO-DEMO** — `nom compose video hello.nomx` produces real `.mp4` with valid `ftyp` header ✅
- [x] **AD-DB-DEMO** — `nom dict list-kinds` returns real rows from `nomdict.db` ✅
- [x] **AD-CHAIN-DEMO** — `nom compose intent "make a logo"` runs full Chain → `.nomx` output ✅
- [ ] **AD-NOMX-EXEC** — CB3: `.nomx` files execute via AST bridge to `nom-ast::Expr` (deferred)

## Open Items — Wave AE (Polish)

- [ ] **AE-LSP-REAL** — Replace hardcoded stubs: hover, completion, definition
- [ ] **AE-GOLDEN-E2E** — 4 real end-to-end tests (render pixel, video encode, DB query, chain output)
- [ ] **AE-CLIPPING** — `cargo clippy --workspace --all-targets` 0 warnings

## Medium-Term Open Items

- [ ] CB3 — `.nomx` evaluator: bridge `parse_define_that()` to `nom-ast::Expr`
- [ ] Bootstrap fixpoint proof
- [ ] Real LLVM parser/resolver/codegen.nom path
- [ ] Parser-backed Nomx validation
- [ ] `cargo build --workspace --release` Windows/Linux/macOS
- [ ] CI green on PR
- [ ] Settings panel full-screen overlay
- [ ] Theme toggle `Cmd/Ctrl+K T`
- [ ] API reference (`cargo doc --no-deps`)

---

## Recently Completed (Last 5 Waves)

- **Wave AD** — 4 end-to-end demos (render, video, DB, chain) + P0 renames + workspace compiles clean
- **Wave Critical** — CB1/2/4/5 fixed + AI orchestration + data engine + media pipeline + brand scrub
- **Wave ABAO** — TableBlock, ConflictResolver, TypeMap, WorkflowComposer, LayerStack
- **Wave ABJ+ABK** — DonutPipeline, CodeGenPipeline, TypeInferencer, ReAct+BM25
- **Wave ABI** — Vision pipeline (YOLOv8, SAM, LayoutLMv3, AnimateDiff)
- **Wave ABG+ABH** — ChatDispatch, LlmQualityGate, LspSyncDriver, CorpusOrchestrator, CompilePipeline

---

## Future Waves

- [x] **Wave AD** — End-to-End Demos (4/5 complete, CB3 deferred)
- [ ] **Wave AE** — Polish: real LSP, golden E2E, clippy clean
- [ ] **Wave AF** — Massive Parallel Repo Pattern Audit & Adoption (16 repos)
- [ ] **Wave AG** — UI hardening: settings, theme toggle, font wiring
- [ ] **Wave AH+** — Bootstrap fixpoint, LLVM path, release builds

---

## Wave AF — Massive Parallel Repo Pattern Audit & Adoption

**Objective:** Extract actionable patterns from 12 untapped high-value reference repos + 4 future-wave repos. Each repo analyzed by a dedicated subagent. Findings written to `.archive/audits/2026-04-19-{repo}-pattern.md`.

### P0 Critical (adopt immediately)
- [ ] **AF-FFMPEG** — Filter graph DSL + format negotiation → `nom-compose/src/video_encode.rs` | Analysis ✅ | Effort: 3–5 weeks | Blocker: build-system integration, hwaccel context
- [x] **AF-POLARS** — LazyFrame + pushdown optimizer → `nom-compose/src/data_query.rs` | Analysis ✅ | Effort: 1–2 weeks (Level B) | Rec: real `PolarsEngine` behind `QueryEngine` trait
- [x] **AF-LANGCHAIN** — Runnable composition + tool use → `nom-compose/src/chain.rs` | Analysis ✅ | Effort: tiny–large | Gap: untyped, sync-only, string-based vs LangChain's typed async streaming

### P1 High (adopt in this wave)
- [x] **AF-CREWAI** — Multi-agent Flow/Crew orchestration → `nom-compose/src/crew.rs` | Analysis ✅ | Effort: 3–4 wk MVP, 13 wk full | Gap: no Flow DAG, no unified memory, no schema-first tools
- [ ] **AF-SPIDER** — Builder-pattern API client + retry + streaming → `nom-compose/src/vendors/*.rs` | Analysis ✅ | Effort: ~24–41 hr | Note: `website.rs` is 13k lines, highly `#[cfg]` gated
- [ ] **AF-OPENHARNESS** — 43+ tool skill library + memory → `nom-compose/src/harness.rs` | Analysis ✅ | Effort: 15–25 dev-days | Gap: 11 stub tools with raw `&str` closures, no schemas/async/permissions/memory
- [x] **AF-OLLAMA** — Zero-config local LLM serving → `nom-compose/src/ollama.rs` | Analysis ✅ | Effort: 1–2 wk HTTP wrapper, 8–12 wk native port | Rec: start with HTTP client against external binary
- [x] **AF-OPENDAL** — Universal storage (S3/GCS/Azure/HDFS) → `nom-compose/src/storage.rs` | Analysis ✅ | Effort: 0.5–1 day (add layers) | Win: add `RetryLayer` + `LoggingLayer` to existing `UniversalStorage`

### P2 Medium (adopt in this wave)
- [x] **AF-TEMPORAL** — Durable workflow execution → `nom-compose/src/durable.rs` | Analysis ✅ | Effort: 28–40 eng-days | Gap: `durable.rs` does not exist; needs history-driven replay FSMs
- [ ] **AF-VELLO** — GPU vector renderer (Rust) → `nom-canvas-core/src/vector.rs` | Analysis ✅ | Effort: 3–4 weeks full, 1–2 weeks incremental | Rec: full integration (Option A)
- [x] **AF-MEMPALACE** — Long-context verbatim memory → `nom-compose/src/memory.rs` | Analysis ✅ | Effort: 1 week baseline (96.6%), 4–6 weeks full | Gap: lacks embeddings, layering, hybrid retrieval
- [x] **AF-VOXCPM** — Controllable voice cloning/TTS → `nom-compose/src/voice.rs` | Analysis ✅ | Effort: Medium–High | Rec: Python inference server (FastAPI/gRPC) called from Rust short-term

### Future-Wave Analysis (scheduled for Wave AG+)
- [ ] **AF-VLLM** — PagedAttention high-throughput inference analysis | Analysis ✅ | Effort: 5–7 mo single-GPU, 9–12 mo multi-GPU | Rec: treat as external executor, do not port GPU kernels
- [ ] **AF-DENO** — Secure JS/TS sandbox analysis | Analysis ✅ | Effort: Low–Very High (4 options) | Alt: `rquickjs` for lighter sandbox
- [ ] **AF-WASMTIME** — WASM sandbox for polyglot plugins analysis | Analysis ✅ | Effort: 1.5–2 wk minimal, 3–4 wk production | Gap: `wasm.rs` absent; `wasm_bridge.rs` is compile-time only
- [ ] **AF-NATS** — Event streaming for distributed agents analysis | Analysis ✅ | Effort: 1–2 wk subject router, 6–12 mo full Raft | Gap: `stream.rs` absent; `streaming.rs` is AI-token streaming

---

## Wave AF-2 — Already Referenced Repo Gap Analysis & Closure

**Objective:** Deep-dive the "already referenced" repos that are actually partial/stubbed, plus the not-yet-adopted D1 repos. 10 subagents dispatched.

### Partial References (gap analysis complete)
- [ ] **AF2-ZED** — GPUI integration gaps | Analysis ✅ | Effort: 2–3 days for CB1/CB2 closure, 8–10 weeks full parity | Top gaps: layout engine stub, 6 GPU pipelines no-ops, no atlas wiring, no uniform buffer upload
- [ ] **AF2-AFFiNE** — Frosted glass + block editor + theming | Analysis ✅ | Effort: 2 weeks frosted GPU pass, 8–10 weeks full | Top gaps: frosted glass draws plain quads (zero blur), tokens compile-time only, no CRDT block schema
- [ ] **AF2-ROWBOAT** — Real LLM integration + tool cards + deep-think | Analysis ✅ | Effort: 3–4 weeks minimal, 6–7 weeks full | Top gaps: zero real LLM adapters, ToolCard 20px strip, deep-think hypotheses are synthetic strings
- [ ] **AF2-N8N** — Sandbox wiring + credential encryption + execution context | Analysis ✅ | Effort: 2.5–3 weeks P0, 4–5 weeks full | Top gaps: no parser bridge, plaintext credentials, no VM isolation, no runtime injection
- [x] **AF2-WRENAI** — SemanticModel + MDL semantic layer | Analysis ✅ | Effort: 5–6 weeks single eng, 2–3 weeks with 2-person team | Top gaps: no manifest, relationships, calculated fields, views, metrics, DDL builders

### Not Yet Adopted (pattern extraction complete)
- [ ] **AF2-REMOTION** — FFmpeg image2pipe + frame sequencer + progress parser | Analysis ✅ | Effort: 3–5 days pipe/args/progress, 2–3 weeks E2E | ⚠️ **License alert:** custom two-tier (not MIT/Apache) — clean-room only
- [ ] **AF2-HAYSTACK** — Graph pipeline + typed sockets + priority scheduler | Analysis ✅ | Effort: 9–13 dev-days MVP, 17–25 full | Nom gap: sequential Vec<Box<dyn>> with untyped String I/O vs Haystack's networkx graph + typed sockets
- [ ] **AF2-TOOLJET** — Widget schema registry + dependency graph + event dispatcher | Analysis ✅ | Effort: 6–8 weeks MVP, 12–16 weeks full | ⚠️ **License alert:** AGPL-3.0 — study patterns only
- [x] **AF2-CANDLE** — Real CandleAdapter with Device::Cpu + model loading | Analysis ✅ | Effort: 1–2 dev-days | Rec: feature-gate `candle-cpu` / `candle-cuda` / `candle-metal`
- [x] **AF2-ARCREEL** — 5-phase video orchestration | Analysis ✅ | Effort: 3–5 weeks | Gap: `media_pipeline.rs` does not exist; closest analog is `author_session.rs` (5-phase `AuthorPhase`)

---

## Structural Debt (from 2026-04-19 audit)

### P0 — Duplicate crate names (blocks workspace unification) ✅ COMPLETE
- [x] Rename `nom-cli` (canvas) → `nom-canvas-cli`
- [x] Rename `nom-graph` (canvas) → `nom-canvas-graph`
- [x] Rename `nom-intent` (canvas) → `nom-canvas-intent`
- [x] Delete `nom-media` (canvas stub) — 351 lines removed
- [x] Delete `nom-ux` (canvas stub) — 625 lines removed
- [x] Delete `nom-canvas/nom-canvas/` stale build artifacts
- [x] Delete `nom-canvas/tests/` superseded by `nom-canvas-tests`

### P1 — God crate splits
- [ ] Split `nom-compose` (28,607 lines) → orchestrator + backends + output
- [ ] Split `nom-canvas-core` (16,902 lines) → render + input + viewport
- [ ] Split `nom-blocks` (11,672 lines) → core + tree + registry
- [ ] Split `nom-graph` (canvas, 15,542 lines) → execution + rag + infra
- [ ] Merge `nom-memoize` (5,926 lines) into `nom-graph`

### P2 — Compiler cleanup
- [ ] Split `nom-concept` (~13K lines) → lexer + parser + ir + validate + bootstrap
- [ ] Modularize `nom-cli` (compiler) subcommands into per-group modules

---

## Single Source of Truth

| Data | Canonical Location |
|---|---|
| Test counts | **This file** — updated per-wave |
| Axis percentages | `ROADMAP_TO_100.md` |
| Wave history | `nom_state_machine_report.md` |
