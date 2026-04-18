# Nom State Machine Report

**Date:** 2026-04-18 | **HEAD:** `7716377` | **Tests:** 8947 | **Workspace:** clean — Waves AT+AU+AV+AW complete. 0 clippy warnings.

---

## Iteration 69 — Wave AW COMPLETE (HEAD 7716377, 8947 tests, 0 warnings)

**10 parallel agents. Remotion video pipeline + hybrid compose + Dify/ToolJet/Refly patterns.**

| Gap ID | Fix | Crate |
|--------|-----|-------|
| C5-V5 | VideoRenderConfig + RenderProgress{rendered_frames,encoded_frames,stage,elapsed_ms} | nom-compose |
| C5-V6 | ComposeEvent::Progress extended; all 49 construction sites updated | nom-compose |
| C5-V7 | CancelSignal + make_cancel_signal() via AtomicBool | nom-compose |
| C5-V8 | VideoConfigContext + thread-local push_video_config/pop/get | nom-compose |
| C5-V9 | validate_codec_pixel_format() — even-dims + ProRes/VP9 matrix | nom-compose |
| UC-MIDDLEWARE | StepMiddleware + MiddlewareRegistry + LoggingMiddleware + LatencyMiddleware | nom-compose |
| UC-STREAM | SwitchableStream + StreamToken (streaming/batch via AiGlueOrchestrator) | nom-compose |
| UC-PROMOTE | POST /promote/:glue_hash axum endpoint | nom-cli |
| UC-CANDLE | CandleAdapter + BackendDevice{Cpu,Cuda} + InferenceFn trait | nom-compiler-bridge |
| A6-LSP | LspRequest/LspResponse + dispatch_lsp_request (6 method stubs) | nom-compiler-bridge |
| B1 parse | DefineThatExpr + parse_define_that() using Tok::Define+Word+That | nom-concept |
| B2 migrate | migrate_typed_to_natural() fn→define, ->→that | nom-concept |
| B8 100 | +16 translation tests (lazy eval, tail-call, monadic bind, etc.) | nom-concept |
| D1 Dify | TypedNode + NodeOutputPort + NodeEvent (Started/Progress/Completed/Failed) | nom-graph |
| D1 ToolJet | palette_kind_count() reflecting 46+ seeded kinds | nom-panels |
| D1 Refly | SkillRouter + SkillDefinition (register/find_by_id/find_by_query) | nom-intent |
| B9 ux/app | nom ux seed, nom app new/import/build/build-report/explain-selection | nom-cli |
| B9 corpus | nom corpus ingest-pypi/ingest-github/pause/resume/report stubs | nom-cli |
| D4 clippy | 0 warnings, 0 errors workspace-wide; unknown lint removed | nom-canvas |
| D5 README | Wave history, Composition API + Video Pipeline sections | README |

Previously complete (Waves AT+AU+AV): AL-PALETTE-SEARCH-UI, AL-TEST-FRAUD, AL-FEATURE-TESTS, AH-CTX, AH-DICTW, AH-GLUE, AH-HYBRID, UC-FLOWGRAPH, AH-CACHE, AH-ORCH, AH-DB-KINDS, C5-V1..V4 Remotion composition/timeline/animate.

---

## Open Items (Wave AX targets)

- ❌ **AN-TEST-DEDUP** — ~85% duplication ratio across 14 crates; target ≤20%
- ❌ **C5-V10** — Two-stage video pipeline (parallel frame capture → FFmpeg stdin streaming)
- ❌ **C4-LSP visual** — hover tooltip/completion popup/diagnostic squiggle rendered on canvas
- ❌ **AH-UI** — Intent Preview + AI Review cards in nom-panels/src/right/
- ❌ **D3 golden paths** — end-to-end demos: Type .nomx → highlight; drag node; compose → artifact
- ❌ **D2 visual** — frosted-glass blur, bezier animate, all 73 tokens visible
- ❌ **A11 LLVM** — Parser/Resolver/TypeChecker/Codegen .nom compiles via rust-nomc
- ❌ **C5 real backends** — GPU→FFmpeg, rodio, opendataloader beyond stubs

---

## Per-crate Test Counts (Wave AW actuals)

| Crate | Tests |
|---|---|
| nom-gpui | 790 |
| nom-blocks | 560 |
| nom-canvas-core | 575 (+12 integration) |
| nom-cli | 424 |
| nom-collab | 546 |
| nom-compiler-bridge | 558 |
| nom-compose | 748 |
| nom-editor | 620 |
| nom-graph | 575 |
| nom-intent | 480 |
| nom-lint | 485 |
| nom-memoize | 468 |
| nom-panels | 609 |
| nom-telemetry | 500 |
| nom-theme | 556 |
| **nom-canvas TOTAL** | **~8506** |
| nom-concept (+B8) | +21 (162 lib + 16 translation_b8) |
| nom-grammar (+AH-DB-KINDS) | 36 |
| **GRAND TOTAL** | **~8947** |

---

**Detailed commit history:** `git log --oneline`. This file keeps only latest state + open missions.
