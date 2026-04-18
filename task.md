# Nom — Task Execution Checklist

**Date:** 2026-04-19 | **HEAD:** `0880564` | **Tests:** 9245 (canvas:9204 + compiler:41) | **Workspace:** clean — Waves AX→ABK complete. A-axis ~72%, B-axis ~85%, C-axis ~90%, D-axis ~100%.

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

## Wave ABI (2026-04-19) — COMMITTED ✅ (HEAD 2b453b0, ~9458 tests)

- ✅ BBoxDetector — LetterBox + IoU + NMS (YOLOv8 pattern, 9 tests)
- ✅ SegmentPipeline — SAM BinaryMask + PointGrid + SamPipeline (9 tests)
- ✅ LayoutAnalyzer — DocBBox + SpatialFeatures + reading-order (9 tests)
- ✅ AnimationPipeline — LatentState + VideoFrame + LCG noise + CFG guidance (9 tests)
- ✅ VisionOrchestrator — detect→segment→layout→.nomx (7 tests)

## Wave ABK (2026-04-19) — COMMITTED ✅ (HEAD 0880564, +38 tests)

- ✅ DonutPipeline — DonutToken markup, DocStructure, DocTask, 3 pipeline modes (9 tests)
- ✅ CodeGenPipeline — FilesDict, PrepromptHolder, GenerationMode, CodeGenPipeline (9 tests)
- ✅ TypeInferencer — TypeEnv + TypeConstraint + TypeInferencer using IrValue::type_of() (12 tests)
- ✅ ReAct+BM25 integration — 8 real integration tests (no stubs), BM25 top-k + ReAct loop

## Wave ABL (2026-04-19) — COMMITTED ✅ (HEAD 7341c05, +35 tests)

- ✅ VideoCapture real — FrameCapture + FfmpegEncoder + real ffmpeg arg construction (9 tests)
- ✅ LspAsyncLoop — LspAsyncConfig + LspAsyncMessage + parse/format/process_batch (8 tests)
- ✅ VisionBridge — VisionOutput→InspectFinding enrich_report() wired (8 tests)
- ✅ Pipeline+Ingest integration — 10 CompilePipeline + CorpusOrchestrator integration tests
- ✅ D-axis: 100% COMPLETE

## Wave ABM (2026-04-19) — COMMITTED ✅ (HEAD 828bd8e, +48 tests)

- ✅ SkillRouter — SkillEntry + SkillRouter case-insensitive + SkillDispatch (9 tests)
- ✅ LifecycleManager — EntryState + merge/eliminate/evolve transitions (11 tests)
- ✅ MeceValidator — MeceObjective + AppScore EPIC_SCORE_THRESHOLD≥95 (10 tests)
- ✅ DreamTree — DreamNode + DreamTree + ParetoFront (9 tests)
- ✅ LspPositionBridge — LspPositionBridge + BoundedLspBridge roundtrip (9 tests)

## Wave ABN (2026-04-19) — COMMITTED ✅ (HEAD 6584bde, +45 tests)

- ✅ UxExtractor — CorpusSource×4 Motion/Dioxus/ToolJet/DeerFlow + 12 seeded patterns (9 tests)
- ✅ IngestPartial — IngestQuality + IngestPromoter Partial→Complete (9 tests)
- ✅ DreamCLI — DreamEngine run_until_epic() score≥95 + DreamReport (9 tests)
- ✅ HybridRetrieval — BM25+vector merge + RRF + top_k (9 tests)
- ✅ SelfHostRegistry — SelfHostStage×5 + seed + SelfHostBootstrapProof fixpoint (9 tests)

## Wave ABO (2026-04-19) — COMMITTED ✅ (HEAD 092b161, +46 tests)

- ✅ AuthorSession — AuthorPhase×5 brainstorm→nomx motion pipeline (9 tests)
- ✅ SkillCLI — SkillCliRunner 8 seeds + case-insensitive route dispatch (10 tests)
- ✅ BootstrapCLI — 5-stage bootstrap report + fixpoint check (9 tests)
- ✅ StreamIngest — StreamConfig + IngestCheckpoint + StreamIngestor + SkipList (9 tests)
- ✅ AestheticRegistry — AestheticDomain×5 + 9 seeded aesthetic skills (9 tests)

## Wave ABP (2026-04-19) — COMMITTED ✅ (HEAD fd58825, +45 tests)

- ✅ AiCompilerLoop — verify→build→bench→flow 4-stage loop (9 tests)
- ✅ MediaComposePipeline — MediaKind×6 + ComposeOp×3 Nom operators (9 tests)
- ✅ AppManifest — FNV-1a dep hash + ManifestGraph (9 tests)
- ✅ Reranker — RerankStrategy×3 + PostProcessor dedup (9 tests)
- ✅ LlvmEmit — LlvmOp×10 dispatch + LlvmBlock + LlvmFunction IR (9 tests)

## Wave ABQ (2026-04-19) — COMMITTED ✅ (HEAD cd4484d, +46 tests)

- ✅ PlanCache — ExecutionCache LRU + Kahn topological sort + cycle detection (10 tests)
- ✅ CrdtHistory — CrdtOp + CrdtHistory + ConflictResolver last-write-wins (9 tests)
- ✅ MetricsDeep — Histogram p50 buckets + SpanTracer (9 tests)
- ✅ LintRules — LintSeverity×4 + LintRule pattern check + auto-fix (9 tests)
- ✅ LruCache — CacheStats hit-rate + CacheEntry TTL + capacity eviction (9 tests)

## Wave ABR (2026-04-19) — COMMITTED ✅ (HEAD 01b1c7e, +45 tests)

- ✅ MultiCursor — CursorAnchor + CursorRange + MultiCursor dedup/move (9 tests)
- ✅ SceneGraph — SceneNodeKind + DFS traversal + AtlasSlot fit (9 tests)
- ✅ LayoutState — PanelSide + ResizeHandle drag + LayoutSnapshot restore (9 tests)
- ✅ TokenSystem — ColorToken luminance/contrast + SpacingToken scale (9 tests)
- ✅ BlockHistory — BlockEvent inverse + HistoryStack undo/redo (9 tests)

## Wave ABS (2026-04-19) — COMMITTED ✅ (HEAD 5840d44, +45 tests)

- ✅ SnapGrid — GridConfig snap + BoundsRect union/intersect + BoundsUnion (9 tests)
- ✅ PipelineContext — PipelineStatus + StreamChunk streaming pipeline (9 tests)
- ✅ LspDiagnostics — DiagnosticSeverity + publishDiagnostics notification (9 tests)
- ✅ SSA — SsaVar + PhiNode + SsaBlock + SsaForm basic block stubs (9 tests)
- ✅ Accessibility — A11yRole + A11yAuditor violations + KeyboardNav (9 tests)

## Wave ABT — COMPLETE (2026-04-19, 9519 tests)

- ✅ SyntaxHighlighter — TokenKind×8 + SyntaxToken + HighlightRange (9 tests)
- ✅ GraphTraversal — GraphEdge + SimpleGraph BFS/DFS + edge filter (9 tests)
- ✅ WorkspaceSchema — SchemaVersion + SchemaMigration + MigrationPlan (9 tests)
- ✅ TypeChecker — CheckedType + TypeContext + constraint unification (9 tests)
- ✅ RenderFrame — DirtyRegion + DirtyTracker + dirty state tracking (9 tests)

## Wave ABU — COMPLETE (2026-04-19, 9555 tests)

- ✅ StreamingResult — PartialResult+StreamingOutput+ResultBuffer pipeline (9 tests)
- ✅ WorkspaceRename — RenameOp+WorkspaceRenamer+RenamePreview (nom-editor) (9 tests)
- ✅ Canonicalize — CanonicalForm+CanonicalizationChecker+PartialLifter §5.10 (9 tests)
- ✅ NomxManifest — NomxManifest+NomxDep+NomxModuleGraph .nomx workspace (9 tests)
- ✅ DreamHistory — DreamHistoryEntry+DreamHistoryStore+DreamJournal (9 tests)

## Wave ABV — COMPLETE (2026-04-19, 9582 tests)

- ✅ RegressionChecker — BenchmarkBaseline+RegressAlert, nom bench regress (9 tests)
- ✅ ViewportMap — ElementBounds+ViewportMap+VisibilityQuery spatial index (9 tests)
- ✅ BenchmarkSideTable — EntryBenchmark+BenchmarkAggregation side-table (9 tests)
- ✅ FeatureStack — WordIdMap+FeatureWeight+FeatureStack word IDs (9 tests)
- ✅ Storyboard — StoryboardPhase×5+StoryboardPlan+StoryboardExecutor (9 tests)

## Wave ABW — COMPLETE (2026-04-19, 9618 tests)

- ✅ BipartiteSolver — CostMatrix+BipartiteAssignment+MinCostSolver §5.15 (9 tests)
- ✅ ImageDispatch — ModelDescriptor+ModelRegistry+ImageDispatcher (9 tests)
- ✅ Ancestry — AncestryChain+ParentMap+AncestorQuery+DescendantIter (9 tests)
- ✅ CrdtMerge — VectorClock+CrdtMergeOp+MergeStrategy (9 tests)
- ✅ SpanAggregator — SpanSample+P95Calculator+SpanAggregator+TraceReport (9 tests)

## Wave ABX — COMPLETE (2026-04-19, 9663 tests)

- ✅ AudioEncode — AudioFormat+AudioBuffer+AudioEncoder+RodioBackend (9 tests)
- ✅ RagPipeline — RagQuery+RagDocument+RagRetriever+RagPipeline (9 tests)
- ✅ Bezier — BezierPoint+BezierCurve de Casteljau+AnimatedBezier (9 tests)
- ✅ ContentAddress — ContentHash FNV-1a+CrossAppStore specialization sharing (9 tests)
- ✅ BlockSchemaV2 — BlockSchemaV2+MigrationTool+RoundTripValidator (9 tests)

## Wave ABY — COMPLETE (2026-04-19, 9708 tests)

- ✅ N8nWorkflow — NodeStatus+WorkflowGraph+WorkflowRunner topo-sort (9 tests)
- ✅ Postproc — PostDoc FNV-1a+DeduplicateFilter+ScoreThresholdFilter+PostPipeline (9 tests)
- ✅ FrostedGlass — BlurLayer+FrostedGlassEffect+LayerCompositor (9 tests)
- ✅ TextureAtlas — AtlasRegion+TextureAtlas+AtlasShelf+AtlasAllocator (9 tests)
- ✅ PdfCompose — PdfElement+PdfPage+PdfDocument+PdfComposer prose→PDF (9 tests)

## Wave ABZ — COMPLETE (2026-04-19, 9753 tests)

- ✅ WebCompose — ComponentKind+WebAppSpec+WebComposer spec→web app (9 tests)
- ✅ HaystackPipeline — HaystackComponent+ComponentPipeline+PipelineRanker (9 tests)
- ✅ AffineTokens — AffineToken×9+TokenResolver+DesignTokenApplier (9 tests)
- ✅ TabBar — TabEntry+TabBar+TabBarState UI navigation (9 tests)
- ✅ VideoEncode — VideoCodec+VideoFrame+VideoEncoder+GpuVideoEncoder stub (9 tests)

## Wave ABAA — COMPLETE (2026-04-19, 9798 tests)

- ✅ AdCreative — AdFormat+AdDimension+AdCreativeSpec+AdComposer (9 tests)
- ✅ Theme — ThemeMode+ThemeTokenMap 4-token+ThemeToggle dark/light (9 tests)
- ✅ HypothesisTree — HypothesisNodeState+ReasoningNode+HypothesisTree+BeliefPropagator (9 tests)
- ✅ MobileCompose — MobilePlatform+MobileScreen+MobileAppSpec+MobileComposer (9 tests)
- ✅ TraceExport — SpanStatus+OpenTelemetrySpan+JaegerSpan+TraceExporter (9 tests)

## Wave ABAB — COMPLETE (2026-04-19, 9843 tests)

- ✅ MeshCompose — MeshVertex+MeshFace+Mesh+MeshComposer (9 tests)
- ✅ WasmBridge — WasmTarget+WasmModule+WasmFeatureGate+WasmBridge (9 tests)
- ✅ GraphifyChart — ChartType+ChartAxis+ChartSeries+ChartSpec+GraphifyComposer (9 tests)
- ✅ AnimationCard — CardState+AnimationCard+CardKeyframe+CardTimeline+CardAnimator (9 tests)
- ✅ Presence — PresenceUserStatus+PresenceUser+PresenceUserMap+PresenceBroadcast (9 tests)

## Wave ABAC — COMPLETE (2026-04-19, 9891 tests)

- ✅ DiagnosticSquiggle — DiagnosticSeverity+DiagnosticSpan+SquiggleStyle+DiagnosticOverlay (9 tests, C4)
- ✅ HoverTooltip — TooltipKind+TooltipContent+TooltipAnchor+HoverTooltip+TooltipRenderer (9 tests, C4)
- ✅ LlamaCompose — PipelineStage+LlamaPipelineNode+LlamaPipeline+PipelineCombinator+PipelineOutput (9 tests, C6)
- ✅ PropertyPanel — PropertyKind+PropertyValue+PropertyField+PropertyGroup+PropertyPanel (10 tests, UI)
- ✅ EventLog — EventKind+LoggedEvent+EventLog+EventLogStore (9 tests, telemetry)

## Wave ABAD — COMPLETE (2026-04-19, 9939 tests)

- ✅ FlowReplay — ReplaySpeed+FlowReplayEntry+FlowReplay+ReplayController+ReplaySnapshot (9 tests)
- ✅ AppBundle — BundleTarget×6+BundleManifest+BundleArtifact+BundleBuilder+BundleOutput (9 tests)
- ✅ FrostedPipeline — FrostedLayerConfig+PassInput+PassOutput+Runner+PipelineStats (9 tests)
- ✅ GoToDef — DefinitionKind+Location+Target+GoToDefRequest+GoToDefResolver (9 tests, C4)
- ✅ FlowStepTable — StepStatus+FlowStepRow+FlowStepTable+FlowStepQuery+StepTimeline (9 tests, A9)

## Wave ABAE — COMPLETE (2026-04-19, 9988 tests)

- ✅ RenamePreview — RenamePreviewKind+Change+Model+Conflict+Applier (9 tests, C4)
- ✅ CompletionEngine — CompletionKind+Item+List+Query+Engine (11 tests, C4)
- ✅ SemanticCache — SemanticKey+Entry+CacheEviction+Cache+CacheStats (9 tests)
- ✅ PixelDiff — PixelRegion+PixelDiff+DiffThreshold+DiffReport+RegionDiffer (11 tests)
- ✅ PerfCounter — CounterKind+PerfCounter+CounterSnapshot+Registry+RateCalc (9 tests)

## Wave ABAF — COMPLETE (2026-04-19, 10035 tests)

- ✅ DeltaCompress — DeltaKind+Frame+Encoder+Decoder+DeltaStream (9 tests)
- ✅ NativeScreen — ScreenTarget+CaptureResolution+CaptureBuffer+ScreenCapture+Backend (9 tests)
- ✅ SnapAlign — AlignAxis+AlignGuide+SnapTarget+AlignResult+AlignmentEngine (9 tests)
- ✅ MultiFileEdit — EditScope+MultiFileChange+Session+MultiFileDiff+SessionApplier (9 tests)
- ✅ EmbedRegistry — EmbedKind+Entry+EmbedRegistry+EmbedResolver (9 tests)

## Wave ABAG — COMPLETE (2026-04-19, 10080 tests)

- ✅ IntentGraph — IntentKind+Node+Edge+IntentGraph+IntentGraphQuery (9 tests)
- ✅ VideoTimeline — ClipKind+TimelineClip+VideoTimeline+ClipOverlap+TimelineRenderer (9 tests)
- ✅ LayoutGrid — TrackSize+GridTrack+GridCell+LayoutGrid+GridPlacement (9 tests)
- ✅ CodeLens — CodeLensKind+CodeLens+Provider+Overlay+LensResolver (9 tests)
- ✅ AuditLog — AuditCategory+AuditEvent+AuditFilter+AuditLog+AuditReporter (9 tests)

## Wave ABAH — COMPLETE (2026-04-19, 10125 tests)

- ✅ SchemaVersion — SchemaVersionId+VersionEdge+SchemaVersionGraph+VersionDiff+MigrationPlan (9 tests)
- ✅ ExportBundle — ExportFormat×6+Target+Job+Queue+ExportResult (9 tests)
- ✅ ViewportClip — ClipRect+ClipStack+ViewportClipper+ClipResult+ClipBatch (9 tests)
- ✅ Breadcrumb — BreadcrumbKind+Segment+Path+Nav+Renderer (9 tests)
- ✅ SyncProtocol — SyncMessageKind+Message+SyncState+SyncSession+SyncProtocol (9 tests)

## Wave ABAI — COMPLETE (2026-04-19, 10171 tests)

- ✅ RouteTable — RouteKind+Key+Entry+RouteTable+RouteResolver (9 tests)
- ✅ ImagePipeline — ImageStageKind+Stage+Pipeline+PipelineResult+Runner (9 tests)
- ✅ HitZone — HitZoneKind+HitZone+HitZoneMap+HitTestResult+ZoneHitTester (10 tests)
- ✅ OutlineView — OutlineItemKind+Item+Section+OutlineTree+Renderer (9 tests)
- ✅ BlockTree — BlockNodeKind+BlockNode+BlockTree+Walker+TreeDiff (9 tests)

## Open Items — Wave ABAJ targets

- ❌ **nom-graph memory_graph** — MemoryNode + MemoryEdge + MemoryGraph + MemoryQuery (graph-backed memory store)
- ❌ **nom-compose data_compose** — DataSource + DataQuery + DataComposer + DataResult (data composition pipeline)
- ❌ **nom-canvas-core transform_stack** — Transform2D + TransformStack + InverseTransform + TransformResult (2D transform math)
- ❌ **nom-editor search_index** — SearchToken + SearchIndex + SearchQuery + SearchResult (editor search)
- ❌ **nom-memoize memo_graph** — MemoKey + MemoEntry + MemoGraph + MemoInvalidator (graph-aware memoization)

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
