# Nom — Task Execution Checklist

**Date:** 2026-04-18 | **HEAD:** `7716377` | **Tests:** 8947 | **Workspace:** clean — Waves AT+AU+AV+AW complete. B-axis ~62%, C-axis ~72%, D-axis ~95%.

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
| BackendKind enum | PASS — Wave AP deleted closed enum; all dispatch via runtime `&str` |

---

## Wave AP (2026-04-18) — COMMITTED ✅ (HEAD ~679ce6b, 8391 tests) — ALL CRITICALS FIXED

### Fixed (21 items)
- ✅ AL-RENDER-2 — real wgpu::Surface/Device/Queue fields + full wgpu 0.19 init chain; pollster=0.3 added
- ✅ AL-RENDER-1 — end_frame_render(): CommandEncoder + begin_render_pass + set_pipeline + draw + submit + present
- ✅ AL-RENDER-3 — VertexBufferLayout (stride=80, 5×Float32x4, Instance); real WGSL QuadIn + GlobalUniforms + NDC transform
- ✅ AL-BACKEND-KIND — BackendKind closed enum DELETED; UnifiedDispatcher+ComposeContext re-exported as primary dispatch
- ✅ AL-GRAMMAR-STATUS — `pub status: KindStatus` added to GrammarKind; list_kinds() + promote_kind() SQL helpers
- ✅ AM-ATLAS-LRU — evict_lru() calls allocator.deallocate(alloc) per entry; no more allocator.clear()
- ✅ AL-LAYOUT-TAFFY — LayoutEngine replaced with real taffy::TaffyTree + node_map
- ✅ AM-UITIER-DIVERGE — score_atom_impl() extracted; UiTier + UiTierOps both delegate
- ✅ AL-DEEPTHINK-CONFIDENCE — edge_color_for_confidence(card.confidence) wired
- ✅ AL-TOOLBAR-HEIGHT — TOOLBAR_H=48.0 deleted; all callers use TOOLBAR_HEIGHT=36.0
- ✅ AL-FONTS — libre_baskerville_regular, eb_garamond_regular, berkeley_mono_regular added to FontRegistry
- ✅ AL-THEME-SYSTEM oled — Theme::oled() constructor added
- ✅ AN-WORKSPACE-DUP — insert_block() dedup guard; entity() returns Option; remove_node()+remove_connector()
- ✅ AN-FRAME-SPATIAL rotation+cycle — rotation: f32 field; add_child() returns Result with cycle guard
- ✅ AN-BLOCKDIFF-WORD — diff_blocks() emits Modified{field:"word"} diffs
- ✅ AM-SPATIAL-WIRE hit_test — CanvasHitTester with R-tree broadphase in production
- ✅ NOM-EDITOR-POINT — Point{row,column} type + Buffer::point_at() + Buffer::offset_from_point()
- ✅ NOM-GRAPH-EXEC — ExecutionEngine::execute() runs plan, calls node logic, stores results
- ✅ AL-ATOMIC-ORDERING — fixed in Wave AN
- ✅ AL-SQL-INJECT — fixed in Wave AO
- ✅ AM-CRDT-IDEMPOTENT — fixed in Wave AN

### Per-crate actuals (Wave AP)
| Crate | Tests |
|---|---|
| nom-gpui | 790 |
| nom-blocks | 560 |
| nom-canvas-core | 575 |
| nom-cli | 400 |
| nom-collab | 546 |
| nom-compiler-bridge | 553 |
| nom-compose | 685 |
| nom-editor | 620 |
| nom-graph | 570 |
| nom-intent | 470 |
| nom-lint | 485 |
| nom-memoize | 468 |
| nom-panels | 601 |
| nom-telemetry | 500 |
| nom-theme | 556 + 12 integration |
| **TOTAL** | **8391** |

---

## Wave AQ (2026-04-18) — COMMITTED ✅ (HEAD c30f2a0)
- ✅ NOM-GRAPH-ANCESTRY — collect_ancestors() DFS transitive cache key walk in ExecutionEngine
- ✅ NOM-BACKEND-SELF-DESCRIBE — BackendDescriptor struct + describe() on Backend trait + list_backends() on UnifiedDispatcher
- ✅ AM-INTENT-STRUCT — bm25_index field; bm25_score() helper; 3-pass resolve() (substring→BM25→classify_with_react)
- ✅ AL-COSMIC — cosmic_text::FontSystem initialized; load_font_data() for each font in FontRegistry
- ✅ AM-SPATIAL-WIRE viewport.rs — SpatialIndex field on Viewport; insert_element() + elements_in_view()
- ✅ UC-SERVE — POST /compose axum endpoint in nom-cli/src/serve.rs; streaming + non-streaming modes; 2 integration tests

## Wave AR (2026-04-18) — COMMITTED ✅ (HEAD fc67aa9)
- ✅ B4 46 kinds — 28 extended kinds + 9 skills seeded in grammar.kinds baseline.sql; no_foreign_brand_names test
- ✅ B5 side-tables — entry_benchmarks + flow_steps schemas in nom-dict; insert_benchmark() + insert_flow_step() helpers
- ✅ B7 9 skills — author_nom_app/compose_from_dict/debug_nom_closure/extend_nom_compiler/ingest_new_ecosystem/use_ai_loop/compose_brutalist_webpage/compose_generative_art_piece/compose_lofi_audio_loop
- ✅ C3 compiler feature=default — `default = ["compiler"]` in nom-compiler-bridge/Cargo.toml
- ✅ C7 interrupt — InterruptSignal field + trigger_interrupt() in DeepThinkPanel
- ✅ A3 EdgeKind — 22 variants with display_name() + is_structural() in nom-types
- ✅ D4 clippy+fmt — workspace lints section; cargo fmt clean
- ✅ D6 foreign names — all "affine:*" → "nom:*" across nom-blocks; brand names in comments → neutral pattern descriptions

## Wave AS (2026-04-18) — COMMITTED ✅ (HEAD 050b1e9)
- ✅ C1 run_composition — run_composition(&self, input) on BackgroundTierOps; run_composition_command() in terminal panel
- ✅ B9 bench CLI — nom bench run/compare/regress/curate in nom-cli/src/bench.rs
- ✅ B9 flow CLI — nom flow record/show/diff/middleware in nom-cli/src/flow.rs
- ✅ B9 media CLI — nom media import/import-dir/render/transcode/diff/similar in nom-cli/src/media.rs
- ✅ CI matrix — .github/workflows/ci.yml 3-OS (ubuntu+windows+macos)
- ✅ B1 define/that — Tok::Define + Tok::That variants + lexer arms in nom-concept; highlight arms
- ✅ B2 NomxFormat — NomxFormat{Typed,Natural,Standard} enum + detect_format() + 5 B2 tests

## Wave AT (2026-04-18) — COMMITTED ✅ (HEAD ced46fc, +37 tests)
- ✅ AL-PALETTE-SEARCH-UI — 32px search box + category headers; filtered_entries/grouped_items; 3 tests
- ✅ AL-TEST-FRAUD — ArtifactDiff out of cfg(test); 5 real SQL injection edge-case tests
- ✅ AL-FEATURE-TESTS — 3 #[cfg(all(test,feature="compiler"))] tests for nom_score/bm25/can_wire
- ✅ AH-CTX — ComposeContext/ComposeResult/ComposeTier in nom-compose/src/context.rs
- ✅ AH-DICTW — DictWriter insert_partial_entry() + promote_to_complete()
- ✅ AH-GLUE — ReActLlmFn trait + 4 adapters + AiGlueOrchestrator + GlueBlueprint
- ✅ AH-HYBRID — HybridResolver 3-tier (DbDriven→Provider→AiLeading)
- ✅ UC-FLOWGRAPH — FlowGraph + FlowNode + FlowEdge + Kahn sort

## Wave AU (2026-04-18) — COMMITTED ✅ (HEAD f38224c)
- ✅ AH-CACHE — GlueCache + GlueStatus Transient/Partial/Complete lifecycle
- ✅ AH-ORCH — ComposeOrchestrator wrapping HybridResolver; run/run_parallel
- ✅ AH-DB-KINDS — 14 composition grammar.kinds seed rows (video_compose/picture_compose/…)

## Wave AV (2026-04-18) — COMMITTED ✅ (HEAD 34f222e)
- ✅ C5-V1 — CompositionConfig + CompositionRegistry (Remotion composition metadata pattern)
- ✅ C5-V2 — SequenceContext + current_frame_in_sequence + is_frame_active
- ✅ C5-V3 — interpolate() with ExtrapolateMode (Clamp/Extend/Identity/Wrap)
- ✅ C5-V4 — spring() underdamped+overdamped physics + SpringConfig

## Wave AW (2026-04-18) — COMMITTED ✅ (HEAD 7716377, 8947 tests, 0 clippy warnings)
- ✅ C5-V5 — VideoRenderConfig + RenderProgress{rendered_frames,encoded_frames,stage,elapsed_ms}
- ✅ C5-V6 — ComposeEvent::Progress extended + all 49 construction sites updated
- ✅ C5-V7 — CancelSignal + make_cancel_signal() via AtomicBool
- ✅ C5-V8 — VideoConfigContext + thread-local push/pop/get_video_config() stack
- ✅ C5-V9 — validate_codec_pixel_format(codec, format, w, h)
- ✅ UC-MIDDLEWARE — StepMiddleware + MiddlewareRegistry + LoggingMiddleware + LatencyMiddleware
- ✅ UC-STREAM — SwitchableStream + StreamToken (word-by-word streaming via AiGlueOrchestrator)
- ✅ UC-PROMOTE — POST /promote/:glue_hash axum endpoint
- ✅ UC-CANDLE — CandleAdapter + BackendDevice{Cpu,Cuda} + InferenceFn trait
- ✅ A6-LSP — LspRequest/LspResponse + dispatch_lsp_request (6 methods)
- ✅ B1 parse — DefineThatExpr + parse_define_that() using Tok::Define+Word+That
- ✅ B2 migrate — migrate_typed_to_natural() fn→define, ->→that
- ✅ B8 100 translations — +16 tests (lazy eval, tail-call, monadic bind, dependent types, etc.)
- ✅ D1 Dify — TypedNode trait + NodeOutputPort + NodeEvent (Started/Progress/Completed/Failed)
- ✅ D1 ToolJet — palette_kind_count() reflecting 46+ seeded kinds
- ✅ D1 Refly — SkillRouter + SkillDefinition (register/find_by_id/find_by_query)
- ✅ B9 ux/app CLI — nom ux seed, nom app new/import/build/build-report/explain-selection
- ✅ B9 corpus — nom corpus ingest-pypi/ingest-github/pause/resume/report
- ✅ D4 clippy — 0 warnings, 0 errors workspace-wide
- ✅ D5 README — Wave history, Composition API + Video Pipeline sections

---

## Open Items — Wave AX targets

- ❌ **AN-TEST-DEDUP** — ~85% duplication ratio; target ≤20%
- ❌ **C5-V10** — Two-stage video pipeline (parallel frame capture → FFmpeg stdin streaming)
- ❌ **C4-LSP** — hover tooltip/completion popup/diagnostic squiggle visually rendered on canvas
- ❌ **AH-INTENT** — classify_with_react 3-pass fully wired (✅ done), AH-PROMOTE UI cards
- ❌ **AH-UI** — Intent Preview + AI Review cards in nom-panels/src/right/
- ❌ **D3 golden paths** — Type .nomx → highlight; drag node → canvas render; wire → confidence edge
- ❌ **A11 LLVM** — Parser/Resolver/TypeChecker/Codegen .nom compiles via rust-nomc
- ❌ **D2 visual** — frosted-glass blur, bezier animate, all 73 tokens used
- ❌ **C5 real backends** — GPU→FFmpeg, rodio, opendataloader real wiring (not stubs)

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
