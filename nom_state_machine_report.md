# Nom State Machine Report

**Date:** 2026-04-19 | **HEAD:** `07ab271` | **Tests:** 8827 | **Workspace:** clean — Waves AX+AY+AZ complete. 0 clippy warnings.

---

## Iteration 71 — Wave AZ COMPLETE (HEAD 07ab271, 8827 tests, 0 warnings)

**10 parallel agents. LSP visual overlay + audio/image/storyboard backends + MECE + define-that pipeline.**

| Gap ID | Fix | Crate |
|--------|-----|-------|
| C4-LSP visual | DiagnosticSquiggle + HoverTooltip + CompletionPopup + LspOverlay | nom-panels |
| A6 LSP real | LspTransport JSON-RPC framing + AuthoringProtocol event stream | nom-compiler-bridge |
| B6 MECE | MeceValidator + MeceCategory + DreamScore (EPIC_SCORE_THRESHOLD=95) | nom-concept |
| B1 full | ConceptNode + define_that_to_concept_node + parse_concept_source | nom-concept |
| C5 audio | AudioSource + AudioPlayback + AudioMixer (rodio-pattern stub) | nom-compose |
| C5 image | ImageLayer + ImageComposite + BlendMode | nom-compose |
| C5 storyboard | StoryboardPanel + Storyboard + estimated_frames() | nom-compose |
| D3 demo | DemoRunner + DemoKind + DemoResult golden-path sequences | nom-cli |
| D2 render | FrostedPassConfig + FrostedRenderPass state machine | nom-canvas-core |
| A10 corpus | CorpusStats + report_stats() | nom-cli |
| AN-TEST-DEDUP | -9 duplicates across nom-gpui/nom-lint/nom-memoize | 3 crates |

## Iteration 70 — Wave AY COMPLETE (HEAD 761c3eb, 8785 tests, 0 warnings)

**13 parallel agents. Shell chrome + visual tokens + nom-ux/nom-media + WASM + docs.**

| Gap ID | Fix | Crate |
|--------|-----|-------|
| C5-V10 | VideoEncoder + FrameBuffer two-stage capture→encode pipeline | nom-compose |
| AH-UI | IntentPreviewCard + AiReviewCard in nom-panels/right | nom-panels |
| D3 golden | 5 end-to-end integration tests (nom-canvas-tests crate) | nom-canvas-tests |
| D2 audit | ThemeTokenAudit struct + 3 audit tests | nom-theme |
| AN-TEST-DEDUP | -20 duplicate tests nom-intent, -4 nom-compose | nom-intent/compose |
| D8 AF-TITLEBAR | TitleBarPanel + traffic lights + title truncation | nom-panels |
| D8 AF-HEADER | HeaderPanel + HeaderAction enum | nom-panels |
| D8 AF-STATUS | StatusBar + StatusItem + StatusKind | nom-panels |
| D8 AF-LEFT | IconRail + LeftPanelLayout (collapse/expand) | nom-panels |
| D8 AF-CENTER | TabManager + CenterLayout + SplitDirection | nom-panels |
| D8 AF-RIGHT | ChatPanel + HypothesisTree + PropertiesPanel | nom-panels |
| D2 visual | FrostedGlassToken + BezierCurve + ThemeMode/Registry | nom-theme |
| D10 UC-POLARS | DataFrame query abstraction (QueryDataFrame) | nom-compose |
| D10 UC-API-TESTS | API integration tests for serve.rs endpoints | nom-cli |
| C7 reasoning | AnimatedReasoningCard FSM + HypothesisTreeNav DFS | nom-panels |
| C8 WASM | WebGpuRenderer stub + wasm feature gate + build_wasm.sh | nom-canvas-core |
| A3 nom-ux | UxPattern/Screen/UserFlow crate (7 tests) | nom-ux (new) |
| A3 nom-media | MediaUnit/Codec/Container crate (6 tests) | nom-media (new) |
| D5 docs | user-manual.md + api-reference.md + CONTRIBUTING.md | docs/ |

Previously complete (Waves AT+AU+AV+AW+AX): AL-PALETTE-SEARCH-UI, AH-CTX/DICTW/GLUE/HYBRID, UC-FLOWGRAPH, AH-CACHE/ORCH/DB-KINDS, C5-V1..V9, UC-MIDDLEWARE/STREAM/PROMOTE/CANDLE, A6-LSP, B1/B2/B8, D1/D4/D5.

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

## Open Items (Wave ABA targets)

- ❌ **D2 render wired** — FrostedRenderPass integrated into actual wgpu draw loop (pass exists, not called)
- ❌ **A11 LLVM** — Parser/Resolver/TypeChecker/Codegen .nom compiles via rust-nomc
- ❌ **C5 real wiring** — GPU→FFmpeg real encode, actual rodio playback, opendataloader real load
- ❌ **A6 LSP async** — tokio stdin/stdout I/O loop (transport framing done, full loop not started)
- ❌ **B2 migration tool** — `nom convert v1 v2` + 100 .nomx golden corpus
- ❌ **A7 bootstrap** — Stage0→Stage1→Stage2→Stage3 fixpoint proof
- ❌ **A10 100-repo corpus** — 100-repo ingestion pipeline + 100M+ nomtu entries

---

## Per-crate Test Counts (Wave AY actuals)

| Crate | Tests |
|---|---|
| nom-gpui | 790 |
| nom-blocks | 560 |
| nom-canvas-core | 582 (+7 WebGPU) |
| nom-canvas-tests | 17 (5 golden + 12 integration) |
| nom-cli | 425 (+4 api_integration) |
| nom-collab | 546 |
| nom-compiler-bridge | 558 |
| nom-compose | 748 |
| nom-editor | 620 |
| nom-graph | 575 |
| nom-intent | 460 (-20 dedup) |
| nom-lint | 485 |
| nom-media | 6 (new crate) |
| nom-memoize | 468 |
| nom-panels | 661 (+52 shell chrome + right dock + center) |
| nom-telemetry | 500 |
| nom-theme | 563 (+7 FrostedGlass/Bezier/ThemeMode) |
| nom-ux | 7 (new crate) |
| **nom-canvas TOTAL** | **~8571** |
| nom-concept (+B8) | ~178 (162 lib + 16 translation_b8) |
| nom-grammar | 36 |
| **GRAND TOTAL** | **~8785** |

---

**Detailed commit history:** `git log --oneline`. This file keeps only latest state + open missions.
