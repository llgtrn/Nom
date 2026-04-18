# Nom — Implementation Plan

**Date:** 2026-04-18 | **HEAD:** `679ce6b` (dirty Wave AP) | **Tests:** 8391 | **Workspace:** dirty — Wave AP complete (Iteration 61 audited), Wave AQ planned
**Canonical:** spec `docs/superpowers/specs/2026-04-17-nomcanvas-gpui-design.md` · state `nom_state_machine_report.md` · tasks `task.md` · entry `INIT.md`

## Current State (Wave AP complete, Iteration 61 audited, 2026-04-18)

DB-driven architecture CONFIRMED PASS. **ALL 4 CRITICAL blockers FIXED in Wave AP:**
1. **RENDER ✅**: `window.rs` now has real wgpu::Surface/Device/Queue/surface_format fields. `end_frame_render()` has full CommandEncoder+RenderPass+draw+submit+present. `build_quad_pipeline` has real VertexBufferLayout. Shaders have real QuadIn @location(0-4) + GlobalUniforms. Renders real pixels.
2. **DB-ENUM ✅**: `BackendKind` closed enum DELETED. All dispatch uses runtime `&str`/`String` keys. `UnifiedDispatcher` + `ComposeContext` are re-exported from lib.rs and are the ONLY dispatch route.
3. **COMPOSE-BRIDGE ✅**: `UnifiedDispatcher` is the primary dispatch. `BackendRegistry` and all callers migrated to string keys.
4. **GRAMMAR-STATUS ✅**: `KindStatus` enum exists. `GrammarKind` has `pub status: KindStatus` field. `list_kinds()` + `promote_kind()` SQL helpers added. Transient/Partial/Complete lifecycle active.

Per-crate test counts Wave AP actuals: nom-gpui 790, nom-blocks 560, nom-canvas-core 575, nom-compose 685, nom-graph 570, nom-collab 546, nom-editor 620, nom-compiler-bridge 553, nom-panels 601, nom-theme 556, nom-lint 485, nom-intent 470, nom-memoize 468, nom-telemetry 500, nom-cli 400.
**TOTAL: 8391 tests, 0 failed.**

## Wave AM — Open Targets (wgpu device init + ComposeContext + DB-driven fixes)

### CRITICAL — Renderer
- [ ] **AL-RENDER-1** — `renderer.rs`: `CommandEncoder` + `begin_render_pass` + `set_pipeline` + `set_vertex_buffer` + `draw_instanced` + `queue.submit` + `surface_texture.present`
- [ ] **AL-RENDER-2** — `window.rs`: wgpu init chain after winit window — `Instance::new` → `create_surface` → `request_adapter` → `request_device` → `Renderer::with_gpu`
- [ ] **AL-RENDER-3** — `shaders.rs`: real WGSL — `@group(0) @binding(0)` GlobalUniforms, instance attributes from QuadInstance (position/size/color/border/corners)
- [ ] **AL-COSMIC** — `fonts.rs`: `cosmic_text::FontSystem::new()` + `load_font_data` for Inter + Libre Baskerville + Berkeley Mono

### CRITICAL — DB-driven mandate
- [ ] **AL-BACKEND-KIND** — `dispatch.rs`: replace closed enum with `pub struct BackendKind(pub String)` runtime newtype
- [ ] **AL-GRAMMAR-STATUS** — `shared.rs`: `KindStatus { Transient, Partial, Complete }` on `GrammarKind`
- [ ] **AL-COMPOSE-BRIDGE** — `nom-compose/src/context.rs`: `ComposeContext` envelope + `dispatch_with_context` on BackendRegistry

### HIGH — Security + UI + Architecture
- [ ] **AL-SQL-INJECT** — `data_query.rs` + `semantic.rs`: `is_safe_identifier()` allowlist validation before SQL interpolation
- [ ] **AL-CRDT-OVERFLOW** — `nom-collab`: `checked_add` in `next_id()`, clamp remote counter in `apply()`
- [ ] **AL-THEME-SYSTEM** — `tokens.rs`: `Theme` struct + `Theme::dark()/light()/oled()` constructors; pass through render context
- [ ] **AL-FONTS** — `fonts.rs`: Libre Baskerville + EB Garamond + Berkeley Mono font handles
- [ ] **AL-DEEPTHINK-CONFIDENCE** — `deep_think.rs`: `edge_color_for_confidence(card.confidence)` for left border tint
- [ ] **AL-LAYOUT-TAFFY** — `layout.rs`: real `taffy::TaffyTree` replacing HashMap stub
- [ ] **AL-INTENT-RESOLVER** — `nom-intent`: `IntentResolver` struct + `resolve()` method — struct is fully absent, only standalone functions exist
- [ ] **AL-UNIFIED-DISPATCHER** — `nom-compose`: `UnifiedDispatcher` wrapping BackendRegistry + ProviderRouter + CredentialStore
- [ ] **AL-SEMANTIC-RELOCATE** — move `semantic.rs` (WrenAI MDL BI) to `nom-compose/src/bi/` or `nom-compose/src/data/`
- [ ] **AM-ATLAS-LRU** — `atlas.rs`: fix `evict_lru()` to call `deallocate()` per entry instead of `allocator.clear()` — partial eviction corrupts surviving UV coordinates
- [ ] **AM-SPATIAL-WIRE** — `selection.rs` + `hit_test.rs`: wire `SpatialIndex::query_region()` — R-tree exists but O(n) linear scans used throughout

## Wave AH — Hybrid Composition System (2026-04-18, planned)
**Spec:** `docs/superpowers/specs/2026-04-18-hybrid-compose-design.md`

Three tiers per request, resolved in order:
1. **DB-driven** — `grammar.kinds` Complete entity → `BackendRegistry::dispatch_with_context()`
2. **Provider-driven** — registered `MediaVendor` → `UnifiedDispatcher` with credential injection
3. **AI-leading** — neither found → `AiGlueOrchestrator` generates `.nomx` glue; sandbox executes; `GlueCache` tracks; Transient→Partial→Complete promotion lifecycle

Intent classification at front (`IntentResolver`: lexical scan → BM25 → `classify_with_react()`).
Multi-kind requests route to parallel `TaskQueue` pipeline via `ComposeOrchestrator`.

### Wave AH Targets
- [ ] **AH-CTX** — `ComposeContext` / `ComposeResult` / `ComposeTier` in `nom-compose/src/context.rs`
- [ ] **AH-DICTW** — `DictWriter` write side: `insert_partial_entry()` + `promote_to_complete()` in `nom-compiler-bridge`
- [ ] **AH-CACHE** — `GlueCache` in `SharedState` + 60s promotion ticker
- [ ] **AH-DISPATCH** — `UnifiedDispatcher`: `ProviderRouter` ↔ `BackendRegistry` bridge with credential injection
- [ ] **AH-INTENT** — `IntentResolver`: lexical scan + BM25 + `classify_with_react()`
- [ ] **AH-GLUE** — `AiGlueOrchestrator` + `GlueBlueprint` + `ReActLlmFn` trait (4 adapters: Stub/NomCli/Mcp/RealLlm)
- [ ] **AH-HYBRID** — `HybridResolver` orchestrating Tier1→Tier2→Tier3
- [ ] **AH-ORCH** — `ComposeOrchestrator` multi-kind parallel pipeline
- [ ] **AH-DB-KINDS** — 14 initial `grammar.kinds` seed rows
- [ ] **AH-PURPOSE** — `intended to <purpose>` clause + orchestrator rejection on absent clause
- [ ] **AH-EXPLICIT** — user Accept → `DictWriter::insert_partial_entry()` immediately
- [ ] **AH-PROMOTE** — `glue_promotion_config` DB table with auto-path thresholds
- [ ] **AH-UI** — Intent Preview + AI Review cards in `nom-panels/src/right/`

## Wave AI-Composer — Universal Composer Platform Leap (2026-04-18, planned)
**Spec:** `docs/superpowers/specs/2026-04-18-nom-universal-composer-design.md`

10 upstream patterns wired into 14-crate workspace. All additive. Exposes `POST /compose` as AI-native monetization surface. Grammar DB compounds as moat.

### Wave AI-Composer Targets
- [ ] **UC-CANDLE** — `nom-compiler-bridge/src/candle_adapter.rs`: `BackendDevice::Cpu` + `ReActLlmFn` impl; Phi-3/Gemma-2B in-process inference
- [ ] **UC-QDRANT** — `nom-compose/src/intent_v2.rs`: Qdrant HNSW client; embeddings per grammar.kinds entry; replaces BM25 in `IntentResolver`
- [ ] **UC-WASM** — `nom-compiler-bridge/src/wasm_sandbox.rs`: Wasmtime `Store<T>` + `Linker`; replaces JS AST `eval_expr`; glue .nomx compiles to WASM module
- [ ] **UC-MIDDLEWARE** — `nom-compose/src/middleware.rs`: `StepMiddleware` trait + `MiddlewareRegistry` wrapping every dispatch call
- [ ] **UC-TELEMETRY-MW** — latency/cost/token rows to nomdict.db via after_step(); Polars lazy aggregation
- [ ] **UC-FLOWGRAPH** — `nom-compose/src/flow_graph.rs`: `FlowNode` + `FlowEdge` typed graph + version control; replaces linear `ComposeOrchestrator`
- [ ] **UC-CRITIQUE** — `nom-compose/src/critique.rs`: AgentScope propose→critique→refine (3-round cap) before Wasmtime execution
- [ ] **UC-TOOLJET** — grammar.kinds DB drives `NodePalette` (72+ kinds); `SELECT kind, label, icon FROM grammar.kinds ORDER BY use_count DESC`; zero hardcoded enums
- [ ] **UC-POLARS** — `data_query` backend: Polars lazy `LazyFrame` + Arrow columnar format; 10-100x speed
- [ ] **UC-HIGGSFIELD** — `nom-compose/src/vendors/higgsfield.rs`: Open-Higgsfield `MediaVendor` impl; 200+ model registry; generation history as few-shot cache
- [ ] **UC-STREAM** — `nom-compose/src/streaming.rs`: Bolt.new `SwitchableStream` wrapping `AiGlueOrchestrator`; token-by-token .nomx to AI Review card
- [ ] **UC-SERVE** — `nom-cli/src/serve.rs`: tokio-axum `POST /compose`; streaming + non-streaming response modes
- [ ] **UC-PROMOTE** — `POST /promote/:glue_hash` → `DictWriter::insert_partial_entry()` for headless AI callers
- [ ] **UC-API-TESTS** — ≥2 integration tests per pattern (20+ new tests)

## Architecture

- **Foundation:** nom-compiler (29 crates) — UNCHANGED, direct workspace deps
- **Shell:** Zed 3-column — Left AFFiNE 248px · Center PaneGroup (6 modes) · Right Rowboat 320px · Bottom · Status
- **Modes:** Code · Doc · Canvas · Graph · Draw · Compose (spatial, no switching)
- **GPUI:** wgpu + winit + taffy + cosmic-text — one binary, no webview

## Compose Targets

| Category | Outputs |
|---|---|
| Media | video · picture · audio · 3D mesh · storyboard · novel→video |
| Screen | web · native · mobile · presentation |
| App | full bundle · ad creative |
| Data | extract (PDF→JSON) · transform · query (WrenAI MDL) |
| Concept | document (PDF/DOCX) |
| Scenario | workflow (n8n + AST sandbox) |

## Wave History (all ✅ complete)

| Wave | Commit | Tests | Highlights |
|---|---|---|---|
| Wave 0 Bootstrap | — | — | 14 crates, workspace clean |
| Wave A GPUI substrate | `8c7d32e` | — | nom-gpui scene graph, 8 wgpu pipelines, etagere atlas, winit |
| Wave B Editor+Blocks | `8c7d32e` | — | nom-editor rope, nom-blocks NomtuRef non-optional, AFFiNE block types |
| Wave C Compiler bridge | `fb66e01` | 17 | shared.rs 3-tier, adapters, first .nomx → highlight wire |
| Wave D Shell | — | 20 | dock, pane, shell, left/right/bottom panels |
| Wave E 16 backends | `a1ba5a1` | 26 | ArtifactStore + ProgressSink + 16 compose backends |
| Wave F Graph RAG | `be3b9a8` | — | graph_rag.rs, graph_mode.rs, deep_think.rs |
| Wave G Stubs | `546e02d` | — | nom-lint sealed, nom-collab RGA CRDT, nom-telemetry W3C |
| Wave K 4 CRITICALs | `dc6a025` | 457 | U1/W1/COL1/INT1 |
| Wave L MEDIUMs | `d139644` | 504 | deep_think + W3C + RRF + impl Element |
| Wave M Infra | `ef9fc84` | 498 | sealed + 3-tier + dispatch/plan/task_queue + 4-tier cache |
| Wave N Infra+Vendor | `d6219b1` | 523 | MediaVendor + ProviderRouter + CredentialStore + SHA-256 + MDL |
| Wave O Infra+LSP | `e61a93c` | 537 | CompilerLspProvider + cancel + cache-promotion + sandbox |
| Wave P Bug fixes | `15a8366` | 558 | E2 CRITICAL + 10 HIGH/MEDIUM |
| Wave Q Quality | `f0ca908` | 581 | sandbox sanitizers + score adapter + can_wire + rag-confidence |
| Wave R Coverage | `0949124` | 638 | NI1 + SipHash13 + coverage expansion |
| Wave S Spec align | `c4d6252` | 686 | 5 panels + 10 backends + FrostedRect + hints |
| Wave T Cleanup | `0b0d48e` | 717 | scenario_workflow + integration tests |
| Waves V–AB | `c3d2323` | 2841 | GPU library wiring + 9 coverage waves (+2124 tests) |
| Waves AF–AK | `8088889` | 6743 | Minimalist UI + 5 coverage waves (+3902 tests) |
| Wave AL | `778b085` | 7241 | CommandStack + CRDT GC + panel serialization (+498 tests) |
| Waves AM–AO | `679ce6b` | 8384 | DB-driven palette/library/SQL inject/CRDT fix/spatial/frame/theme (+1143 tests) |
| Wave AP | uncommitted | 8391 | **ALL 4 CRITICALS FIXED**: renderer renders pixels, BackendKind deleted, GrammarKind.status, TaffyTree, atlas LRU, 21 items closed (+7 tests) |

## Non-Negotiable Rules

1. Read source repos end-to-end before writing code
2. Always use `ui-ux-pro-max` skill for UI work
3. Zero foreign identities in public API
4. nom-compiler is CORE — direct workspace deps, zero IPC
5. DB IS the workflow engine — no external orchestrator
6. Every canvas object = DB entry — `entity: NomtuRef` non-optional
7. Canvas = AFFiNE-for-RAG
8. Doc mode = Zed + Rowboat + AFFiNE
9. Deep thinking = compiler op streamed right dock
10. GPUI fully Rust — no webview
11. Spawn parallel subagents for multi-file work
12. Run `gitnexus_impact` before editing any symbol
