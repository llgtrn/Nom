# Nom — Task Execution Checklist

**Date:** 2026-04-19 | **HEAD:** `5a525e5` | **Tests:** 9315 | **Workspace:** clean — Waves AX+AY+AZ+ABA+ABB+ABC+ABD+ABE+ABF complete. A-axis ~67%, B-axis ~82%, C-axis ~90%, D-axis ~99%.

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

## Wave AX (2026-04-19) — COMMITTED ✅ (HEAD partial, merged into 761c3eb)
- ✅ C5-V10 — VideoEncoder + FrameBuffer two-stage pipeline in nom-compose
- ✅ AH-UI — IntentPreviewCard + AiReviewCard in nom-panels/right
- ✅ D3 golden paths — 5 end-to-end integration tests in nom-canvas-tests crate
- ✅ D2 audit — ThemeTokenAudit + 3 audit tests
- ✅ AN-TEST-DEDUP — -20 duplicate tests nom-intent, -4 nom-compose

## Wave AY (2026-04-19) — COMMITTED ✅ (HEAD 761c3eb, 8785 tests, 0 warnings)
- ✅ D8 AF-TITLEBAR — TitleBarPanel + with_traffic_lights + title truncation
- ✅ D8 AF-HEADER — HeaderPanel + HeaderAction enum
- ✅ D8 AF-STATUS — StatusBar + StatusItem + StatusKind
- ✅ D8 AF-LEFT — IconRail + LeftPanelLayout
- ✅ D8 AF-CENTER — TabManager + CenterLayout + SplitDirection
- ✅ D8 AF-RIGHT — ChatPanel + HypothesisTree + PropertiesPanel
- ✅ D2 visual — FrostedGlassToken + BezierCurve Newton + ThemeMode/Registry
- ✅ D10 UC-POLARS — DataFrame query abstraction (QueryDataFrame)
- ✅ D10 UC-API-TESTS — API integration tests for serve.rs endpoints
- ✅ C7 — AnimatedReasoningCard FSM + HypothesisTreeNav DFS
- ✅ C8 — WebGpuRenderer stub + wasm feature gate + .cargo/config.toml + build_wasm.sh
- ✅ A3 nom-ux — UxPattern/Screen/UserFlow crate (7 tests)
- ✅ A3 nom-media — MediaUnit/Codec/Container crate (6 tests)
- ✅ D5 docs — docs/user-manual.md + docs/api-reference.md + CONTRIBUTING.md

## Wave AZ (2026-04-19) — COMMITTED ✅ (HEAD 07ab271, 8827 tests, 0 warnings)
- ✅ C4-LSP visual — DiagnosticSquiggle + HoverTooltip + CompletionPopup + LspOverlay (+10)
- ✅ A6 LSP real — LspTransport JSON-RPC framing + AuthoringProtocol event stream (+8)
- ✅ B6 MECE — MeceValidator + DreamScore EPIC_SCORE_THRESHOLD=95 (+8)
- ✅ B1 full parse — ConceptNode + parse_concept_source pipeline (+6)
- ✅ C5 audio — AudioSource + AudioPlayback + AudioMixer (+8)
- ✅ C5 image — ImageLayer + ImageComposite + BlendMode (+11)
- ✅ C5 storyboard — StoryboardPanel + Storyboard (+storyboard)
- ✅ D3 demo — DemoRunner + DemoKind + DemoResult golden sequences (+5)
- ✅ D2 render — FrostedPassConfig + FrostedRenderPass state machine (+7)
- ✅ A10 corpus — CorpusStats + report_stats() (+4)
- ✅ AN-TEST-DEDUP — -9 duplicates across nom-gpui/nom-lint/nom-memoize

## Wave ABA (2026-04-19) — COMMITTED ✅ (HEAD 6a41b2b, 8891 tests)

- ✅ D2 render wired — FrostedRenderPass integrated into WebGpuRenderer.begin_frame() (+4)
- ✅ A6 LSP loop — LspLoopState + LspServerLoop state machine wired (+6)
- ✅ B2 migration tool — ConvertDirection/Options/Result + convert_source/convert_file (+5)
- ✅ A7 bootstrap stubs — BootstrapStage/StageBuild/BootstrapProof + check_fixpoint() (+7)
- ✅ A11 LLVM IR — IrType/IrValue/IrInstr/IrFunction/IrModule typed IR (+8)
- ✅ B2 corpus — 10 .nomx golden examples in examples/ (define-that syntax)
- ✅ C5 DataLoader — DataSourceKind/LoadStrategy/DataBatch/DataLoader stub (+7)
- ✅ Telemetry — TraceSpan + TraceCollector (+4)
- ✅ Collab — VectorClock + happened_before (+4)
- ✅ Graph dispatch — NodeHandler trait + PassThroughHandler + NodeHandlerRegistry (+6)
- ✅ Editor — SyntaxHighlight spans + LineFoldRegion/LineDisplayMap (+8)

## Wave ABB (2026-04-19) — COMMITTED ✅ (HEAD 8b11241, 8957 tests)

- ✅ A11 codegen — NomParser + AstToIr + IrPrinter in nom-concept (+8)
- ✅ C5-V10 — FrameCapture + TwoStagePipeline video pipeline (+6)
- ✅ B2 corpus — 40/100 .nomx golden examples (+30 files)
- ✅ C7/C4 visual — DeepThinkRenderer + EditorView in nom-panels (+8)
- ✅ D5 README — README.md rewritten, fmt clean
- ✅ D3 golden paths — 14 golden path tests in nom-canvas-tests (+9)
- ✅ D1 Haystack — ComponentPipeline + TextSplitter + DocumentRetriever (+14)
- ✅ C6 RAG — BM25Retriever + CosineSimilarityRetriever (+8)
- ✅ B1 parse — FullParser + BlockExpr implicit return (+8)
- ✅ nom-blocks — deeper workspace tests (+6)
- ✅ nom-graph — real dispatch integration tests (+5)

## Wave ABC (2026-04-19) — COMMITTED ✅ (HEAD ed86222, 9045 tests)

- ✅ D2 visual tokens — ColorSet dark/light/oled + ThemeTokens (nom-theme +16)
- ✅ D2 typography — FontFamily/FontSize/TypographyScale (nom-theme shared)
- ✅ D1 WidgetRegistry — 35 WidgetKind variants, 6 categories (+8)
- ✅ C7 graphify charts — ChartType/DataSeries/ChartConfig/Chart (+8)
- ✅ B3 ingestion — IngestionPipeline + LifecycleManager (+8)
- ✅ A6 kind query — KindQueryClient + KindPromotion/KindStatus (+8)
- ✅ C1 scene builder — SceneLayer + SceneBuilder (+8, nom-gpui)
- ✅ C9 eviction — EvictionPolicy (Lru/Lfu/Fifo) + PolicyConfig (+6)
- ✅ D4 naming lint — NamingLinter check_snake_case/no_foreign_brand (+8)
- ✅ D4 merge CRDT — MergeStrategy + MergeRecord (+6, nom-collab)
- ✅ D5 telemetry OTel — Counter/Histogram/MetricsRegistry (+13)
- ✅ D3 golden +4 — 18 golden path tests total
- ✅ B2 corpus 70/100 — 30 more .nomx files (archive_entry→websocket_connect)

## Wave ABD (2026-04-19) — COMMITTED ✅ (HEAD d0d56df, 9121 tests)

- ✅ B2 corpus 100/100 — COMPLETE (30 more .nomx, all topics covered)
- ✅ AM-SPATIAL-WIRE — ViewportSnap + SnapGrid + AabbIndex (+10)
- ✅ AL-COSMIC text — TextLayoutEngine + GlyphRun + TextAlign (+8)
- ✅ D4 collab session — CollabSession + SessionRole (+6)
- ✅ NOM-GRAPH-ANCESTRY — AncestryCache depth cache (+6)
- ✅ UC-GRAPH-TRAVERSE — GraphTraversal dfs/bfs (+6)
- ✅ AM-INTENT-STRUCT — IntentClassifier + MecePartition (+8)
- ✅ D1 ToolJet 51 — WidgetRegistry 51 kinds (+4)
- ✅ A11 bitcode — BitcodeModule + IrToBitcode (+8, nom-concept)
- ✅ B7 editor cursor — EditorCursor + BufferHistory (+8)
- ✅ C5 reverse — ReverseOrchestrator media→nomx pipeline (+8)
- ✅ D3 golden +4 — 22 total golden path tests

## Wave ABE (2026-04-19) — COMMITTED ✅ (HEAD 7dc8dd3, 9217 tests)

- ✅ Chat AI dispatch — ChatAttachment + CanvasMode + AiChatSession (+10)
- ✅ UC-SERVE POST — ComposeRequest/Response + compose_logic (+8)
- ✅ A6 LSP I/O — LspFrame + LspIoBuffer + LspAsyncLoop (+8)
- ✅ D2 RenderPipeline — DrawCommand/FrameGraph/Coordinator (+10)
- ✅ A7 bootstrap depth — FixpointAttempt + BootstrapRunner (+4)
- ✅ A3 ingest — IngestSource + IngestPipeline events (+4)
- ✅ D3 BlockDiffer — BlockDiffKind + BlockDiffer (+6)
- ✅ C7 WeightGraph — WeightedGraph + WeightGraph (+8 total)
- ✅ D5 CacheStats + StructureLinter (+12)
- ✅ D5 MetricsExporter + AnimationRegistry (+12)
- ✅ D4 PresenceMap + AnimationClip (+14)
- ✅ D3 golden 26 — 26 total golden path tests

## Wave ABF (2026-04-19) — COMMITTED ✅ (HEAD 5a525e5, 9315 tests)

- ✅ NomInspector — 7 InspectTarget kinds + InspectFinding/InspectReport + detect_target/inspect_url (+10)
- ✅ Sherlock adapter — SherlockStatus/SherlockSite/SherlockResult + parse_json_output + to_inspect_findings (+8)
- ✅ StrategyExtractor — BusinessModel/StrategySignal/StrategyReport keyword extraction (+8)
- ✅ RepoInspector — RepoLanguage/RepoFile/RepoProfile (nom-compiler-bridge) (+8)
- ✅ ContentHash/ContentDag — FNV-1a ContentHash + ContentStore + DagNode/DagEdge/ContentDag (+12)
- ✅ NativeCodegen stub — TargetArch/TargetOs/NativeBinary/NativeCodegen lower_to_native() (+6)
- ✅ FixpointVerifier — verify_fixpoint() in nom-concept bootstrap (+4)
- ✅ InspectPanel (internal) — InspectKind/InspectRequest/InspectResult routing logic (+10)
- ✅ EventQueue — KeyModifiers + InputEvent + EventQueue (nom-gpui) (+8)
- ✅ SelectionManager — SelectionAnchor + SelectionRange + SelectionManager (nom-editor) (+8)
- ✅ OpLog — OpKind + Op + OpLog CRDT log (nom-collab) (+6)
- ✅ ActiveSpan — SpanKind + SpanEvent + duration_ns (nom-telemetry) (+6)
- ✅ D3 golden 30 — 30 total golden path tests in nom-canvas-tests
- ✅ NomInspector design spec — docs/superpowers/specs/2026-04-19-nom-inspector-design.md

## Wave ABG+ABH (2026-04-19) — COMMITTED ✅ (HEAD de66f18, ~9415 tests)

- ✅ ChatDispatch → InspectDispatch wired (WebUrl/FilePath routing, 8 tests)
- ✅ LlmQualityGate + inspect_with_quality() DreamScore≥95 (6 tests)
- ✅ LspSyncDriver std::io Content-Length framing (5 tests)
- ✅ CorpusOrchestrator 4-ecosystem ingestion planner (6 tests)
- ✅ CompilePipeline parse→IR→codegen (8 tests)
- ✅ B8 +20 paradigm translations (actor, CSP, lenses, free monad, session types…)
- ✅ AudioRenderer PlaybackEntry + rodio-pattern (8 tests)
- ✅ D3 golden 35 total tests
- ✅ ContentDag + ContentHash 16 integration tests
- ✅ SherlockNative — SiteEntry + ErrorDetect + CheckStatus native Rust (8 tests)
- ✅ VisionProvider — UiComponentType + ScreenshotAnalyzer (10 tests)
- ✅ C9 build --all-features passes cleanly (7,258 nom-canvas tests)
- ✅ 8 vision repos cloned (SAM, YOLOv8, unilm, screenshot-to-code, gpt-engineer, donut, AnimateDiff, stable-video-diffusion)

## Open Items — Wave ABI targets (vision pipeline native Rust)

- ❌ **BBoxDetector** — YOLOv8 LetterBox + NMS + BBox pipeline in nom-compose/src/detection.rs
- ❌ **SegmentPipeline** — SAM sparse/dense prompt encoding + mask decoder stubs in nom-compose/src/segmentation.rs
- ❌ **LayoutAnalyzer** — LayoutLMv3 spatial bbox embedding pattern in nom-compose/src/layout.rs
- ❌ **AnimationPipeline** — AnimateDiff UNet3D + temporal attention stubs in nom-compose/src/diffusion.rs
- ❌ **VisionOrchestrator** — chains BBox→Segment→Layout→nomx generation
- ❌ **C5 video GPU→FFmpeg** — real FrameCapture → FFmpeg encode in TwoStagePipeline
- ❌ **A6 LSP real tokio** — real tokio stdin/stdout in LspAsyncLoop
- ❌ **D3 golden 40** — 40 total golden path tests

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
