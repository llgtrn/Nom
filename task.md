# Nom — Task Execution Checklist

**Date:** 2026-04-18 | **HEAD:** `7086ff2` | **Tests:** 7652 | **Workspace:** clean

## DB-Driven Architecture (Wave AE/AC verified PASS)

| Check | Verdict | Evidence |
|---|---|---|
| `Connector::new_with_validation()` only constructor | PASS | `nom-blocks/src/connector.rs:88` |
| NodePalette live DB SELECT | PASS | `nom-panels/src/left/node_palette.rs:26` |
| LibraryPanel live DB SELECT | PASS | `nom-panels/src/left/library.rs:28` |
| `DictReader` isolation | PASS | `Connection::open` only in `sqlite_dict.rs:23,27` |
| `entity: NomtuRef` non-optional | PASS | `block_model.rs:46`, `graph_node.rs:12` |
| `production_kind: String` (not enum) | PASS | `graph_node.rs:13` |
| Cross-workspace path deps | PASS | Feature-gated optional deps in `Cargo.toml` |
| BackendKind enum | **CRITICAL VIOLATION** — Wave AM AL-BACKEND-KIND |

## Completion Percentages (Iteration 55 audit — 8 agents, 2026-04-18)

| Axis | % | Critical gap |
|---|---|---|
| A · nom-compiler | 44% | Self-hosting not started; 22/29 crates never called from canvas |
| B · Nom language | 34% | C-like syntax; 30+ extended kinds unseeded |
| C · nom-canvas ↔ compiler integration | **35%** | Renders 0 pixels; ComposeContext/HybridResolver/GlueCache MISSING; BackendKind closed enum |
| D · Overall platform | **72%** | Theme system stub; taffy stub; fonts stub; DB-driven automation 35% |

**DB-driven automation answer:** YES — grammar.kinds = workflow node library, .nomx = workflow definition, dispatch.rs = executor. Architecture correct. Gap: BackendKind closed enum + ComposeContext/UnifiedDispatcher missing.

---

## Wave AM (2026-04-18) — COMPLETE ✅ (7086ff2, 7652 tests)
- [x] nom-gpui: 701→743 (AdapterInfo, DeviceDescriptor, RenderTarget, format utils, extended FrameStats)
- [x] nom-blocks: 480→515 (BlockDiff/apply, hierarchical frame trees, edgeless positioning, workspace stats)
- [x] nom-canvas-core: 510→530 (GestureRecognizer tap/pan/pinch, CommandStack max-size, spatial bulk ops)
- [x] nom-compose: 625→625 (ComposeContext string-kind wired, UnifiedDispatcher register/dispatch)
- [x] nom-editor: 550→578 (SemanticToken/classify, DocumentSymbol, FoldingRange, cursor anchor/head)
- [x] nom-compiler-bridge: 470→505 (LSP rename/prepare-rename, workspace symbols, concurrent bridge)
- [x] nom-collab: 480→504 (multi-doc session, presence list, CRDT convergence)
- [x] nom-panels: 500→535 (panel persistence v2, floating panels, drag-between-panels)
- [x] nom-theme: 475→505 (animation curves, focus-visible tokens, forced-colors tokens)
- [x] nom-lint: 400→430; nom-intent: 380→410; nom-memoize: 385→415
- [x] nom-telemetry: 415→445; nom-cli: 340→370; nom-graph: 530 (connection errors — held)

## Wave AN (planned) — CRITICAL fixes from audit + test expansion + ~8100 target

### CRITICAL carry-forward items (from Iteration 54-55 audit)
- [ ] **AL-BACKEND-KIND** — Delete closed `BackendKind` enum; route via `UnifiedDispatcher` string keys
- [ ] **AL-SQL-INJECT** — Add `is_safe_identifier()` validator in data_query.rs + semantic.rs
- [ ] **AL-CRDT-OVERFLOW** — `checked_add` in `next_id()`; clamp remote counter in `apply()`
- [ ] **AM-ATLAS-LRU** — Fix `evict_lru()` to call `deallocate()` per entry, not `clear()`
- [ ] **AM-SPATIAL-WIRE** — Wire SpatialIndex R-tree into selection.rs + hit_test.rs O(n) scans
- [ ] **AM-INTENT-STRUCT** — Add `IntentResolver` struct with `resolve()` method

### Test expansion targets (~8100 total)
- [ ] nom-gpui: 743→780; nom-blocks: 515→550; nom-canvas-core: 530→565
- [ ] nom-compose: 625→660; nom-graph: 530→565; nom-collab: 504→540
- [ ] nom-editor: 578→615; nom-compiler-bridge: 505→540; nom-panels: 535→570
- [ ] nom-theme: 505→535; nom-lint: 430→460; nom-intent: 410→440
- [ ] nom-memoize: 415→445; nom-telemetry: 445→475; nom-cli: 370→400

## Wave AM (original planned) — wgpu device init + ComposeContext + DB-driven fixes + ~7750 target

### CRITICAL — Renderer (AE1 still never closed after 7+ waves)
- [ ] **AL-RENDER-2** — `nom-gpui/src/window.rs`: Window struct has ZERO GPU fields; add `surface: Option<wgpu::Surface<'static>>`, `device: Option<Arc<wgpu::Device>>`, `queue: Option<Arc<wgpu::Queue>>`, `surface_format: Option<wgpu::TextureFormat>`; add full wgpu 0.19 init chain in `run_native_application()` (Instance → create_surface → request_adapter → request_device → configure); add `pollster = "0.3"` to Cargo.toml
- [ ] **AL-RENDER-1** — `nom-gpui/src/renderer.rs:550-564`: replace `end_frame()` stub — add CommandEncoder creation, `begin_render_pass()` with clear, `set_pipeline(&quad_pipeline)`, `set_vertex_buffer(0, instance_buffer)`, `draw(0..6, 0..quad_count)`, drop pass, `queue.submit([encoder.finish()])`, `output.present()`; signature: `end_frame(&mut self, surface: Option<&wgpu::Surface<'_>>)`
- [ ] **AL-RENDER-3** — TWO changes required: (1) `renderer.rs:475` change `buffers: &[]` to VertexBufferLayout with 5×Float32x4 instance attrs (stride=80, step_mode=Instance, locations 0-4 = bounds/bg_color/border_color/border_widths/corner_radii); (2) `shaders.rs:4-9` replace degenerate WGSL with real QuadIn struct using @location(0-4), vertex position from bounds via pixel→NDC conversion using viewport uniform
- [ ] **AL-COSMIC** — `nom-theme/src/fonts.rs`: initialize `cosmic_text::FontSystem`, call `db_mut().load_font_data()` for Inter + Libre Baskerville + Berkeley Mono; replace placeholder integer IDs with real font handles

### CRITICAL — DB-driven mandate
- [ ] **AL-BACKEND-KIND** — `nom-compose/src/dispatch.rs`: DELETE lines 9-324 (`BackendKind` enum + `Backend` trait + `BackendRegistry` + 7 impl blocks); migrate all callers to existing `UnifiedDispatcher::dispatch(&ComposeContext)` (already string-keyed at lines 367-409); ~100 test sites: `BackendKind::Video` → `"video"` strings
- [ ] **AL-GRAMMAR-STATUS** — `nom-compiler-bridge/src/shared.rs`: add `pub enum KindStatus { Transient, Partial, Complete }` with `from_str()` before GrammarKind; add `pub status: KindStatus` field to struct; update `list_kinds()` SQL to `SELECT name, description, COALESCE(status, 'transient') FROM kinds`; update ~25 test construction sites
- [ ] **AL-COMPOSE-BRIDGE** — `ComposeContext` already exists at `dispatch.rs:332-359`; `UnifiedDispatcher` already exists at lines 367-409; task is to DELETE the closed BackendKind path (AL-BACKEND-KIND above) so UnifiedDispatcher becomes the ONLY dispatch route — no new file needed

### HIGH — Security
- [ ] **AL-SQL-INJECT** — `nom-compose/src/backends/data_query.rs:27-29` + `semantic.rs:72-75`: add `fn is_safe_identifier(s: &str) -> bool { s.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.') }`; validate all table/column names before interpolation
- [ ] **AL-CRDT-OVERFLOW** — `nom-collab/src/lib.rs:95-100`: use `self.counter.checked_add(1).expect(...)` in `next_id()`; clamp remote counter in `apply()`: `self.counter = op.id.counter.min(u64::MAX - 1)`

### HIGH — UI/UX
- [ ] **AL-THEME-SYSTEM** — `nom-theme/src/tokens.rs`: add `pub struct Theme { ... }` + `Theme::dark()`, `Theme::light()`, `Theme::oled()` constructors; pass `&Theme` through render context
- [ ] **AL-FONTS** — `nom-theme/src/fonts.rs`: add `libre_baskerville_regular`, `eb_garamond_regular`, `berkeley_mono_regular` fields; update `TypeStyle::body()` to use prose fonts; `TypeStyle::code()` to use Berkeley Mono
- [ ] **AL-DEEPTHINK-CONFIDENCE** — `nom-panels/src/right/deep_think.rs:161-184`: use `tokens::edge_color_for_confidence(card.confidence)` for left border; set `border_widths.left = 2.0` only
- [ ] **AL-LAYOUT-TAFFY** — `nom-gpui/src/layout.rs`: replace HashMap stub with real `taffy::TaffyTree`; `request_layout` creates taffy nodes; `compute_layout` calls `taffy.compute_layout(root, available)` and propagates

### HIGH — Architecture
- [ ] **AL-SEMANTIC-RELOCATE** — `nom-compose/src/semantic.rs` is WrenAI MDL/BI layer, NOT workflow composition; move to `nom-compose/src/bi/semantic.rs` or `nom-compose/src/data/` namespace
- [ ] **AL-INTENT-RESOLVER** — `nom-intent/src/lib.rs`: add `IntentResolver::resolve(input: &str, grammar_kinds: &[GrammarKind]) -> ResolvedIntent`; lexical scan → BM25 scoring → `classify_with_react()` for ambiguous (delta < 0.15)
- [ ] **AL-UNIFIED-DISPATCHER** — `nom-compose/src/unified_dispatcher.rs`: wraps `BackendRegistry` + `ProviderRouter`; kind name → BackendKind → vendor → dispatch with ComposeContext + CredentialStore injection

### HIGH — Architecture (Iteration 55 new findings)
- [ ] **AM-ATLAS-LRU** — `nom-gpui/src/atlas.rs:139-152`: fix `evict_lru()` — call `self.allocator.deallocate(alloc)` per evicted entry instead of `self.allocator.clear()`; surviving entries must not retain stale UV coordinates
- [ ] **AM-SPATIAL-WIRE** — `nom-canvas-core/src/selection.rs:105-112` + `hit_test.rs`: replace O(n) linear scans with `SpatialIndex::query_region()` calls; R-tree exists in `spatial_index.rs` but is completely unwired
- [ ] **AM-INTENT-STRUCT** — `nom-intent/src/lib.rs`: create `pub struct IntentResolver { bm25_index: BM25Index, grammar_kinds: Vec<GrammarKind> }` + `fn resolve(&self, input: &str) -> ResolvedIntent` — struct is fully absent, only standalone functions exist

### MEDIUM (Iteration 55 new findings)
- [ ] **AM-CRDT-IDEMPOTENT** — `nom-collab/src/lib.rs:105-112`: add `if self.ops.contains(&op) { return; }` at top of `apply()` — `merge()` deduplicates but `apply()` does not, diverging behavior
- [ ] **AM-CONNECTOR-DESER** — `nom-blocks/src/connector.rs:34`: replace `#[derive(Deserialize)]` with custom `impl<'de> Deserialize<'de> for Connector` that calls `new_with_validation()` to prevent grammar bypass
- [ ] **AM-UITIER-DIVERGE** — `nom-compiler-bridge/src/ui_tier.rs`: unify `UiTier` and `UiTierOps` `score_atom` call paths — currently produce inconsistent scores for same input

### MEDIUM
- [ ] **AL-PALETTE-SEARCH-UI** — `nom-panels/src/left/node_palette.rs:77-86`: render 32px search box quad at top; category group header rows between kind groups
- [ ] **AL-TOOLBAR-HEIGHT** — `nom-theme/src/tokens.rs:232`: change `TOOLBAR_H` from 48.0 to 36.0 per mandate
- [ ] **AL-ATOMIC-ORDERING** — `nom-compiler-bridge/src/shared.rs:84,102`: change `grammar_version` load to `Ordering::Acquire`, store to `Ordering::Release`
- [ ] **AL-TEST-FRAUD** — `nom-compose/src/semantic.rs`: delete `artifact_diff_*` tests (testing #[cfg(test)]-only functions); replace with real SQL injection edge case tests
- [ ] **AL-FEATURE-TESTS** — `nom-compiler-bridge/src/ui_tier.rs`: add `#[cfg(feature = "compiler")]` test block testing real `nom_score::score_atom()`, `BM25Index::search()`, `can_wire()` paths

### Per-crate test targets (Wave AM ~7750)
| Crate | Current | Target |
|---|---|---|
| nom-blocks | 480 | 515 |
| nom-canvas-core | 510 | 545 |
| nom-cli | 340 | 370 |
| nom-collab | 480 | 515 |
| nom-compiler-bridge | 470 | 505 |
| nom-compose | 625 | 660 |
| nom-editor | 550 | 585 |
| nom-gpui | 701 | 740 |
| nom-graph | 530 | 565 |
| nom-intent | 380 | 410 |
| nom-lint | 400 | 430 |
| nom-memoize | 385 | 415 |
| nom-panels | 500 | 535 |
| nom-telemetry | 415 | 445 |
| nom-theme | 475 | 505 |
| **TOTAL** | **7241** | **~7750** |

---

## Wave AH (planned) — Hybrid Composition System
**Spec:** `docs/superpowers/specs/2026-04-18-hybrid-compose-design.md`
**Design:** DB-driven → Provider-driven → AI-leading, three-tier resolver with intent classification and grammar promotion lifecycle.

### Sub-project 1: ComposeContext + UnifiedDispatcher
- [ ] **AH-CTX** — `nom-compose/src/context.rs`: `ComposeContext`, `ComposeResult`, `ComposeTier`, `ComposeConstraints`
- [ ] **AH-DICTW** — `nom-compiler-bridge/src/dict_writer.rs`: `DictWriter::insert_partial_entry()` + `promote_to_complete()`
- [ ] **AH-CACHE** — `SharedState` in `shared.rs`: add `glue_cache: RwLock<HashMap<String, GlueCacheEntry>>` + 60s promotion ticker
- [ ] **AH-DISPATCH** — `nom-compose/src/unified_dispatcher.rs`: `UnifiedDispatcher` bridges `ProviderRouter` ↔ `BackendRegistry` with credential injection
- [ ] **AH-ROUTER** — `ProviderRouter::route_with_context(&ComposeContext)` + `BackendRegistry::dispatch_with_context(&ComposeContext)`
- [ ] **AH-VENDOR** — `MediaVendor` trait: add `credential: Option<&str>` + `ctx: &ComposeContext` to `compose()` signature

### Sub-project 2: IntentResolver
- [ ] **AH-INTENT** — `nom-compose/src/intent_resolver.rs`: 3-step pipeline (lexical scan → BM25 → `classify_with_react()`)
- [ ] **AH-BM25** — BM25 index built over `grammar.kinds.description` + `grammar.kinds.word` at startup
- [ ] **AH-MULTI** — multi-kind detection: `Vec<(BackendKind, f32)>` candidates above 0.65 threshold
- [ ] **AH-TRAIN** — training signal: user correction feeds back into BM25 index

### Sub-project 3: AiGlueOrchestrator + HybridResolver
- [ ] **AH-GLUE** — `nom-compose/src/ai_glue.rs`: `AiGlueOrchestrator`, `GlueBlueprint`, `ReActLlmFn` trait + 4 adapters (Stub/NomCli/Mcp/RealLlm)
- [ ] **AH-HYBRID** — `nom-compose/src/hybrid_resolver.rs`: `HybridResolver` orchestrates Tier1→Tier2→Tier3
- [ ] **AH-ORCH** — `nom-compose/src/orchestrator.rs`: `ComposeOrchestrator` multi-kind parallel pipeline via `TaskQueue`
- [ ] **AH-PURPOSE** — `intended to <purpose>` clause required in every AI `.nomx` sentence; orchestrator rejects+retries if absent; purpose text → `grammar.kinds.description`
- [ ] **AH-EXPLICIT** — explicit promotion: Accept or Edit+Save in Review card → `DictWriter::insert_partial_entry()` immediately
- [ ] **AH-PROMOTE** — `glue_promotion_config` DB table: auto-path thresholds (auto_promote_count=3, auto_promote_confidence=0.7, complete_use_count=10)
- [ ] **AH-DB-KINDS** — seed 14 initial `grammar.kinds` rows (video/picture/audio/presentation/web_app/mobile_app/native_app/document/data_extract/data_query/workflow/ad_creative/3d_mesh/storyboard)

### Sub-project 4: UI Surfaces
- [ ] **AH-PREVIEW** — `nom-panels/src/right/intent_preview.rs`: Intent Preview card (kind confidence bars + compose/change/all-3 buttons)
- [ ] **AH-REVIEW** — `nom-panels/src/right/glue_review.rs`: AI Review card — purpose clause highlighted inline; Accept/Edit+Save → immediate Partial; Skip → Transient
- [ ] **AH-GUTTER** — Doc mode gutter `⚡` badge for AI-generated entities (Partial status)
- [ ] **AH-NODE** — Graph mode: amber tint + `⚡` badge on AI-generated node cards, removed on Complete
- [ ] **AH-STATUS** — Status bar: `⚡ N AI entities pending review` counter

---

## Wave AI-Composer — Universal Composer Platform Leap (2026-04-18, planned)
**Spec:** `docs/superpowers/specs/2026-04-18-nom-universal-composer-design.md`

**Goal:** Wire 10 upstream patterns into 14-crate workspace. All additive. Expose `POST /compose` as AI-native monetization surface. Grammar DB compounds as moat.

- [ ] **UC-CANDLE** — `nom-compiler-bridge/src/candle_adapter.rs`: `BackendDevice::Cpu` + `ReActLlmFn` impl; Phi-3/Gemma-2B in-process, zero subprocess
- [ ] **UC-QDRANT** — `nom-compose/src/intent_v2.rs`: Qdrant HNSW client replacing BM25 in `IntentResolver`; embeddings per grammar.kinds entry; sub-10ms kind match
- [ ] **UC-WASM** — `nom-compiler-bridge/src/wasm_sandbox.rs`: Wasmtime `Store<T>` + `Linker::func_wrap()` replacing JS AST `eval_expr`; glue .nomx compiles to WASM module
- [ ] **UC-MIDDLEWARE** — `nom-compose/src/middleware.rs`: `StepMiddleware` trait + `MiddlewareRegistry` wrapping every `BackendRegistry::dispatch` call
- [ ] **UC-TELEMETRY-MW** — latency/cost/token rows to nomdict.db via `after_step()`; Polars lazy frame daily aggregation; cost-per-kind in settings panel
- [ ] **UC-FLOWGRAPH** — `nom-compose/src/flow_graph.rs`: `FlowNode` + `FlowEdge` typed graph replacing linear `ComposeOrchestrator`; version control on composition graphs
- [ ] **UC-CRITIQUE** — `nom-compose/src/critique.rs`: propose → critique → refine (3-round cap) before Wasmtime execution
- [ ] **UC-TOOLJET** — grammar.kinds DB rows drive node palette (72+ kinds); `SELECT kind, label, icon FROM grammar.kinds ORDER BY use_count DESC`; zero hardcoded enums
- [ ] **UC-POLARS** — `data_query` backend: Polars lazy `LazyFrame` + Arrow columnar; 10-100x speed
- [ ] **UC-HIGGSFIELD** — `nom-compose/src/vendors/higgsfield.rs`: Open-Higgsfield `MediaVendor` impl; 200+ model registry; generation history as few-shot cache in nomdict.db
- [ ] **UC-STREAM** — `nom-compose/src/streaming.rs`: Bolt.new `SwitchableStream` wrapping `AiGlueOrchestrator`; token-by-token .nomx to AI Review card
- [ ] **UC-SERVE** — `nom-cli/src/serve.rs`: tokio-axum `POST /compose`; streaming + non-streaming response modes
- [ ] **UC-PROMOTE** — `POST /promote/:glue_hash` → `DictWriter::insert_partial_entry()` for headless AI callers
- [ ] **UC-API-TESTS** — ≥2 integration tests per pattern (20+ new tests total)

---

## Compiler Parallel Track (nom-compiler — UNCHANGED as infra)

- [x] GAP-1c body_bytes · GAP-2 embeddings · GAP-3 corpus ingest
- [x] GAP-4 nom-intent 9router pattern · GAP-5 deep_think backing op
- [ ] Bootstrap fixpoint proof (Wave future)

---

## Non-Negotiable Rules

1. Read source repos end-to-end before any code borrowing the pattern
2. Always use `ui-ux-pro-max` skill for UI work
3. Zero foreign identities in public API
4. nom-compiler is CORE — direct workspace path deps, zero IPC
5. DB IS the workflow engine — no external orchestrator
6. Every canvas object = DB entry — `entity: NomtuRef` non-optional
7. Canvas = AFFiNE-for-RAG (frosted glass + confidence edges)
8. Doc mode = Zed + Rowboat + AFFiNE
9. Deep thinking = compiler op streamed to right dock
10. GPUI fully Rust — one binary, no webview
11. Spawn parallel subagents for multi-file work
12. Run `gitnexus_impact` before editing any symbol

**Sibling docs:** `implementation_plan.md` · `nom_state_machine_report.md` · `docs/superpowers/specs/2026-04-17-nomcanvas-gpui-design.md` · `INIT.md`
