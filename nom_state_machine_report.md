# Nom State Machine Report

**Date:** 2026-04-18 | **HEAD:** `ced46fc` | **Tests:** 8428 | **Workspace:** clean — Waves AP+AQ+AR+AS+AT complete

---

## Iteration 65 — Wave AT COMPLETE (HEAD ced46fc, 8428 tests)

| Gap ID | Fix | Crate |
|--------|-----|-------|
| AL-PALETTE-SEARCH-UI | 32px search box + category group headers; filtered_entries/grouped_items | nom-panels |
| AL-TEST-FRAUD | ArtifactDiff + artifact_diff() moved out of cfg(test); 5 real SQL injection edge-case tests | nom-compose |
| AL-FEATURE-TESTS | 3 #[cfg(all(test, feature="compiler"))] tests for nom_score/bm25_index/can_wire | nom-compiler-bridge |
| AH-CTX | ComposeContext / ComposeResult / ComposeTier in nom-compose/src/context.rs | nom-compose |
| AH-DICTW | DictWriter insert_partial_entry() + promote_to_complete() | nom-compiler-bridge |
| AH-GLUE | ReActLlmFn trait + 4 adapters (Stub/NomCli/Mcp/RealLlm) + AiGlueOrchestrator + GlueBlueprint | nom-compose |
| AH-HYBRID | HybridResolver 3-tier orchestration (DbDriven→Provider→AiLeading) | nom-compose |
| UC-FLOWGRAPH | FlowGraph + FlowNode + FlowEdge + Kahn topological sort | nom-compose |

Previously complete (Waves AQ/AR/AS): NOM-GRAPH-ANCESTRY, NOM-BACKEND-SELF-DESCRIBE, AM-INTENT-STRUCT, AL-COSMIC, AM-SPATIAL-WIRE viewport.rs, UC-SERVE, B4 46 kinds, B5 side-tables, B7 9 skills, C3 feature flag, C7 interrupt, A3 EdgeKind, B1 define/that tokens, B2 NomxFormat, B9 bench/flow/media CLI, CI matrix, C1 run_composition.

---

## Open Items (Wave AU targets)

- ❌ **AN-TEST-DEDUP** — ~85% duplication ratio across 14 crates; target ≤20%
- ❌ **AH-CACHE** — GlueCache in SharedState + 60s promotion ticker
- ❌ **AH-ORCH** — ComposeOrchestrator multi-kind parallel pipeline
- ❌ **AH-DB-KINDS** — 14 initial grammar.kinds seed rows (video/picture/audio/…)
- ❌ **UC-CANDLE** — candle_adapter.rs BackendDevice::Cpu + ReActLlmFn impl
- ❌ **UC-MIDDLEWARE** — StepMiddleware trait + MiddlewareRegistry wrapping every dispatch
- ❌ **UC-PROMOTE** — POST /promote/:glue_hash endpoint → DictWriter::insert_partial_entry()
- ❌ **B9 remaining** — nom corpus ingest pypi/github, nom ux seed, nom app new/...
- ❌ **C1 full LLVM** — Click Run → native binary (run_composition wired; LLVM codegen not yet)
- ❌ **C4 LSP visual** — hover tooltip/completion popup/diagnostic squiggle render verified

---

## Per-crate Test Counts (Wave AT actuals)

| Crate | Tests |
|---|---|
| nom-gpui | 790 |
| nom-blocks | 560 |
| nom-canvas-core | 575 (+12 integration) |
| nom-cli | 411 |
| nom-collab | 546 |
| nom-compiler-bridge | 550 |
| nom-compose | 703 |
| nom-editor | 620 |
| nom-graph | 572 |
| nom-intent | 472 |
| nom-lint | 485 |
| nom-memoize | 468 |
| nom-panels | 608 |
| nom-telemetry | 500 |
| nom-theme | 556 |
| **TOTAL** | **8428** |

---

**Detailed commit history:** `git log --oneline`. This file keeps only latest state + open missions.
