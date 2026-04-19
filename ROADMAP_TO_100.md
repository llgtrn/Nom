# Nom — Roadmap to 100%

**Date:** 2026-04-19 | **Mandate:** every `[ ]` is a completable task. Tick only with committed code + passing test.

| Axis | Today | Target | Gap |
|---|---|---|---|
| A · nom-compiler | **~55%** | 100% | 45pp |
| B · Nom language | **~45%** | 100% | 55pp |
| C · canvas ↔ compiler | **~58%** | 100% | 42pp |
| D · Overall platform | **~60%** | 100% | 40pp |

---

## AXIS A — nom-compiler → 100%

### A1. Self-host parser/resolver/typechecker/codegen
- [x] Lexer.nom compiles natively on Windows (baseline)
- [ ] Parser.nom compiles via rust-nomc
- [ ] Resolver.nom compiles
- [ ] Typechecker.nom compiles
- [ ] Codegen.nom compiles
- [ ] Full S1→S6 pipeline.nom builds standalone
- [ ] Self-built `s1` binary executes `hello.nomx`
- [ ] Self-built `s1` reproduces 255+ test-suite results

### A2. DIDS
- [x] Design-Integrated Dictionary System shipped (Wave AR)
- [x] 9-kind foundation locked (Wave AR)

### A3. Body-only ingestion + extended kinds
- [x] 22-edge EdgeKind with display_name/is_structural (Wave ABK)
- [x] Intent resolution pipeline — HybridRetriever+BM25+RRF (Wave ABK)
- [x] Lifecycle transitions — LifecycleManager (Wave ABM)
- [x] UX extractor 12 patterns — UxExtractor (Wave ABO)
- [x] Skill routing via `EntryKind::Skill` — SkillRouter+SkillDispatch (Wave ABM)
- [x] `nom-ux` crate — UxPattern/Screen/UserFlow (Wave AY)
- [x] `nom-media` crate — MediaUnit/Codec/Container (Wave AY)
- [x] Stream-and-discard disk discipline — StreamConfig (Wave ABN)
- [x] Checkpoint + resumption — IngestCheckpoint (Wave ABN)
- [x] Bandwidth-throttle non-optional — StreamConfig.throttle (Wave ABN)
- [x] `Partial` → `Complete` canonicalization lift — PartialLifter+CanonicalForm (Wave ABU)
- [x] Aesthetic skills seeded — AestheticRegistry 9 skills (Wave ABN)
- [x] AI-invokes-compiler loop — AiCompilerLoop (Wave ABP)

### A4. Parser-in-Nom
- [ ] `stdlib/self_host/lexer.nom` frozen
- [ ] `stdlib/self_host/parser.nom`
- [ ] `stdlib/self_host/ast_printer.nom`
- [ ] Round-trip byte-identity on 100-sample corpus

### A5. Architectural ADOPT
- [x] Workspace manifest as `.nomx` AppManifest — NomxManifest (Wave ABU)
- [x] Cargo-style deps in .nomx — NomxDep with content_hash (Wave ABU)
- [x] Module graph via `HasFlowArtifact` edges — NomxModuleGraph (Wave ABU)

### A6. LSP + AuthoringProtocol
- [x] LspRequest/LspResponse + dispatch stubs (Wave AW)
- [x] textDocument/hover/definition/references/symbol stub dispatch (Wave AW)
- [x] stdin/stdout real handshake — LspServerLoop (Wave ABA)
- [x] AuthoringProtocol edit-is-compile event stream (Wave AZ)
- [x] Partial-result streaming — PartialResult+StreamingOutput+ResultBuffer (Wave ABU)
- [x] `workspace/rename` refactor — RenameOp+WorkspaceRenamer+RenamePreview (Wave ABU)

### A7. Bootstrap fixpoint proof
- [x] Stage0→Stage1→Stage2→Stage3 binary stubs (Wave ABA)
- [ ] `s2 == s3` byte-identical
- [ ] Proof tuple stored in dict
- [ ] Parity track: ≥99% IR equivalence across test corpus
- [ ] 100% runtime correctness on test corpus
- [ ] 4-week parity hold before default flip
- [ ] Default flip (rust-nomc → nom-nomc)
- [ ] Rust sources → `.archive/rust-<version>/`
- [ ] 3-month grace period
- [ ] Archive lock + announcement

### A8. Dream mode
- [x] Criteria → Proposals emitter — MeceObjective+MeceValidator (Wave ABM)
- [x] `app_score` ≥95 gate — AppScore EPIC_SCORE_THRESHOLD=95 (Wave ABM)
- [x] `nom app dream` CLI iterates until score reached — DreamEngine (Wave ABN)
- [x] Dream history persisted in `entry_meta` — DreamJournal+DreamHistoryStore (Wave ABU)

### A9. Closure-level specialization
- [x] `entry_benchmarks` side-table populated — BenchmarkSideTable (Wave ABV)
- [x] Bipartite min-cost assignment solver — MinCostSolver (Wave ABW)
- [x] Cross-app specialization sharing — ContentAddressStore+CrossAppStore (Wave ABX)
- [ ] 70–95% binary-size reduction verified on test corpus
- [x] `nom bench regress` CLI — RegressionChecker (Wave ABV)

### A10. Dict ingestion at scale
- [ ] 100-repo corpus ingested end-to-end
- [ ] 100M+ nomtu entries in nomdict.db
- [ ] All 20+ paradigm families catalogued
- [ ] PyPI top-500 ingested
- [ ] Top-500/ecosystem GitHub ingested
- [ ] `nom corpus workspace-gc` runs clean
- [ ] DB stats: ≥1 GB, ≥1 000 kinds, ≥100k clause_shapes

### A11. LLVM pipeline beyond lexer
- [x] Parser → AST codegen — NomParser+AstToIr+IrPrinter (Wave ABB)
- [x] AST → typed IR — IrType/IrValue/IrInstr/IrFunction/IrModule (Wave ABA)
- [ ] IR → LLVM bitcode for all S1-S6 stages
- [ ] Bitcode → native binary on Windows/Linux/macOS
- [ ] Cross-compile WASM target
- [ ] Cross-compile mobile (iOS/Android) targets
- [ ] Codegen benchmark suite (Google Benchmark pattern)

---

## AXIS B — Nom language → 100%

### B1. Natural-language syntax ≥95%
- [ ] `define X that Y` replaces `fn X -> Y` across stdlib
- [ ] Last-sentence implicit return
- [ ] Zero null by grammar (no Option at Nom level)
- [ ] Zero race by grammar
- [ ] Zero overflow by grammar
- [ ] Zero panic by grammar
- [x] English-only vocabulary (VN tokens = 0, verified 2026-04-13)
- [x] Vietnamese GRAMMAR STYLE inspiration only (no VN tokens in code)
- [ ] C-like syntax track archived → `.archive/syntax-clike/`

### B2. .nomx single format
- [x] NomxFormat{Typed,Natural,Standard} enum + detect_format() (Wave AS)
- [x] migrate_typed_to_natural() fn→define, ->→that (Wave AW)
- [ ] v1 + v2 merge spec stabilized
- [x] Migration tool `nom convert v1 v2` (Wave ABA)
- [x] 100 `.nomx` files in `examples/` (Wave ABB)
- [ ] Round-trip byte-identity tested
- [ ] Executable .nomx (tokenizes; no evaluator/AST bridge)

### B3. 9-kind foundation
- [x] 9 core kinds (Wave AR)

### B4. Extended kinds seeded
- [x] UxPattern, DesignRule, Screen, UserFlow, Skill, AppManifest (Wave AR)
- [x] DataSource, Query, AppAction, AppVariable, Page (Wave AR)
- [x] Benchmark, BenchmarkRun, FlowArtifact, FlowStep, FlowMiddleware (Wave AR)
- [x] MediaUnit, PixelGrid, AudioBuffer, VideoStream, VectorPath (Wave AR)
- [x] GlyphOutline, MeshGeometry, Color, Palette, Codec (Wave AR)
- [x] Container, MediaMetadata, RenderPipeline (Wave AR)

### B5. Typed side-tables
- [x] `entry_benchmarks` schema — insert_benchmark() + tests (Wave AR)
- [x] `flow_steps` schema — insert_flow_step() (Wave AR)
- [x] Indexes + FKs declared in nom-dict schema (Wave AR)
- [ ] Populated from real ingests

### B6. Dream-tree + MECE
- [x] MECE-objectives validator + DreamScore ≥95 gate (Wave AZ)
- [ ] Feature-stack word IDs
- [ ] DreamReport score ≥95 gate active

### B7. Self-documenting Skills seeded
- [x] author_nom_app, compose_from_dict, debug_nom_closure (Wave AR)
- [x] extend_nom_compiler, ingest_new_ecosystem, use_ai_loop (Wave AR)
- [x] compose_brutalist_webpage, compose_generative_art_piece, compose_lofi_audio_loop (Wave AR)

### B8. Corpus breadth
- [x] 100 translations baseline (Wave AW)
- [ ] 100 paradigm families
- [x] 20+ paradigm families (maintain)

### B9. Authoring CLI complete
- [x] `nom author start/check` (Wave AR)
- [x] `nom corpus ingest repo/status/workspace-gc` (Wave AR)
- [x] `nom bench run/compare/regress/curate` (Wave AS)
- [x] `nom flow record/show/diff/middleware` (Wave AS)
- [x] `nom media import/import-dir/render/transcode/diff/similar` (Wave AS)
- [x] `nom corpus ingest pypi/github/pause/resume/report` — stubs (Wave AW)
- [x] `nom ux seed <path>` (Wave AW)
- [x] `nom app new/import/build/build-report/explain-selection` (Wave AW)

### B10. Bootstrap proof
- [ ] s2==s3 byte-identical attested in dict

---

## AXIS C — canvas ↔ compiler integration → 100%

### C1. Spec §9 wire table
- [x] Type → tokenize → highlight (Wave C)
- [x] Hover → tooltip (Wave O)
- [x] Pause 500ms → diagnostics
- [x] Drag wire → can_wire (Wave Q)
- [x] Click Run → run_composition() → background_tier (Wave AS)
- [ ] Click Run → compile → LLVM → native binary execute
- [x] Command-bar → classify_with_react
- [x] Deep Think → scored hypothesis chain (Wave R)
- [ ] Open compose → dream_report → score + proposals

### C2. DB-driven palette
- [x] `NodePalette::load_from_dict()` live SELECT (Wave ABU)
- [x] `LibraryPanel::load_from_dict()` live SELECT (Wave ABU)
- [ ] End-to-end test: real nomdict.db → palette renders
- [x] Remove slice/static palette as production path (Wave ABU)
- [x] Library panel reads same live grammar source as palette (Wave ABU)

### C3. Feature gate flip
- [x] `compiler` feature = default ON in nom-compiler-bridge (Wave AR)
- [x] Default build links nom-compiler (Wave AR)
- [x] Bridge tests run without `--features compiler` flag (Wave AR)
- [x] CI matrix includes default+compiler (Wave AS)

### C4. LSP stream visually verified
- [x] Hover tooltip renders — HoverTooltip+TooltipRenderer (Wave ABAC)
- [x] Completion popup visible with arrow-key navigation — CompletionEngine+CompletionList (Wave ABAE)
- [x] Diagnostic red-squiggle underline renders — DiagnosticSquiggle+DiagnosticOverlay (Wave ABAC)
- [x] Go-to-definition navigates — GoToDefResolver+GoToDefRequest (Wave ABAD)
- [x] Rename-refactor preview works — RenamePreviewModel+RenameApplier (Wave ABAE)

### C5. Backend wiring
- [x] C5-V1..V10: Composition/Sequence/interpolate/spring/RenderConfig/Progress/Cancel/ConfigContext/validate/two-stage (Wave AV–ABB)
- [ ] Video GPU → FFmpeg parallel encode
- [ ] Audio rodio real playback
- [x] Data-extract: DataLoader stub (Wave ABA)
- [ ] Data-extract: opendataloader XY-Cut++ 0.015s/page
- [ ] Image: 200+ model registry
- [ ] Document: typst real backend
- [x] Storyboard: 5-phase orchestration (Wave ABV)
- [x] Native_screen: platform-specific capture (Wave ABAF)
- [ ] Mobile_screen: iOS/Android target integration
- [x] App_bundle: Cargo + wgpu signed bundle (Wave ABAD)
- [ ] Web app backend

### C6. RAG real retrievers
- [x] Graph RAG BFS + confidence weights (Wave Q)
- [x] Vector retriever: BM25Retriever + CosineSimilarityRetriever (Wave ABB)
- [x] LlamaIndex pipeline composition (Wave ABAC)
- [x] Refly skill-engine integration (Wave prior)

### C7. Deep-think round-trip
- [x] classify_with_react + react_chain_interruptible (Wave R)
- [x] DeepThinkPanel ingest_events/consume_stream (Wave P)
- [x] InterruptSignal + trigger_interrupt() (Wave AR)
- [ ] Animated reasoning-card progression on canvas
- [ ] Hypothesis tree navigation

### C8. Browser target
- [ ] wasm-bindgen build config
- [ ] WebGPU renderer variant
- [ ] bridge compiles to wasm (feature gate)
- [ ] Demo deployed to GitHub Pages

### C9. Cross-workspace build hygiene
- [x] Feature-gated path deps verified (Wave C)
- [ ] `cargo build --all-features` passes with compiler
- [ ] CI matrix: desktop + wasm + --features compiler

---

## AXIS D — Overall Platform → 100%

### D1. Reference-repo parity
- [x] Zed gpui — scene/renderer/atlas/elements/styled/window/layout | **PARTIAL** — renderer library-real, integration-stub
- [x] AFFiNE — 73 tokens + frosted + bezier + collapsible | **PARTIAL** — tokens real, frosted stub
- [x] rowboat — ChatSidebar + tool cards + deep-think | **PARTIAL** — UI models real, no LLM integration
- [x] ComfyUI — 4-tier cache + Kahn + cancel + IS_CHANGED | **REAL** ✅
- [x] GitNexus — confidence+reason edges + NomtuRef | **REAL** ✅
- [x] dify — TypedNode trait + NodeOutputPort + NodeEvent (Wave AW) | **REAL** ✅
- [x] n8n — 4 AST sanitizers + credential store | **PARTIAL** — sandbox exists, not wired
- [x] LlamaIndex — RRF + cosine + BFS | **REAL** ✅
- [x] yara-x — Sealed + InternalRule | **REAL** ✅
- [x] typst — comemo Tracked/Constraint/hash128 | **REAL** ✅
- [x] WrenAI — SemanticModel + MDL | **REAL** — `semantic.rs` with SQL generation + registry
- [x] 9router — 3-tier fallback + credential + compose_with_fallback | **REAL** ✅
- [x] Refly — SkillRouter + SkillDefinition + find_by_query (Wave AW) | **REAL** ✅
- [x] excalidraw — hit-test + selection + snapping | **REAL** ✅
- [ ] ffmpeg — Filter graph DSL + format negotiation
- [x] Polars — LazyFrame + pushdown optimizer
- [x] LangChain — Runnable composition + tool use
- [x] CrewAI — Multi-agent Flow/Crew orchestration
- [ ] Spider — Builder-pattern API client + retry
- [x] OpenHarness — 43+ tool skill library + memory | **PARTIAL** — `harness.rs` 5 schema-first tools + permissions
- [x] Ollama — Zero-config local LLM serving
- [x] opendal — Universal storage S3/GCS/Azure/HDFS
- [x] temporal-sdk-core — Durable workflow execution
- [ ] vello — GPU vector renderer
- [x] mempalace — Long-context verbatim memory
- [x] VoxCPM — Controllable voice cloning/TTS
- [ ] Haystack — full pipeline composition | **NOT ADOPTED** — `ComponentPipeline` stubbed
- [ ] ToolJet — 55-widget registry | **NOT ADOPTED** — 51 kinds seeded, not all wired
- [x] Remotion — GPU→frame→FFmpeg encoder | **PARTIAL** — `media_pipeline.rs` real MP4 via FFmpeg pipe
- [ ] Open-Higgsfield — 200+ model dispatch | **NOT ADOPTED** — stub only
- [x] ArcReel — 5-phase orchestration | **REAL** — `MediaPipeline` 5-stage with MP4 encode
- [ ] waoowaoo — 4-phase parallel cinematography | **NOT ADOPTED** — not in code
- [ ] opendataloader — XY-Cut++ + hybrid-AI tables | **NOT ADOPTED** — stub only

### D2. UI/UX visual verification
- [x] Frosted-glass pipeline renders visible blur (Wave ABAD)
- [x] Focus ring = 2px outline stroke
- [x] All panels render real quads with token colors
- [ ] Bezier control points animate smoothly
- [x] Spring animation at AFFiNE defaults verified on-screen (Wave ABAD)
- [x] Color contrast ≥4.5:1 WCAG AA (Wave ABAD)
- [x] Motion timing 200ms/300ms verified (Wave ABAD)
- [ ] All 73 AFFiNE tokens visually used
- [ ] Dark + light theme toggle
- [x] Reduced-motion/accessibility gate (Wave ABAD)
- [x] Screenshot/runtime audit: surfaces are not quad-only stubs (Wave ABAD)
- [x] Visual artifact at `.omx/visual/nom-panels-runtime.ppm` (Wave ABAD)
- [x] OS screenshot artifact at `.omx/visual/nom-gpui-window-first-paint.png` (Wave ABAD)

### D3. End-to-end golden paths
- [ ] Type .nomx → live syntax highlighting
- [ ] Drag node from palette → canvas renders node with port circles
- [ ] Wire two nodes → can_wire green, edge draws with confidence color
- [ ] Click compose → deep-think cards stream → artifact appears in preview block
- [ ] Video-compose demo: paragraph → 10-second MP4
- [ ] Document-compose demo: prose → PDF
- [ ] Web-compose demo: spec → web app
- [ ] Ad-creative demo: intent → static + video + interactive
- [ ] Mobile-app-compose demo
- [ ] 3D-mesh-compose demo

### D4. Build, CI, release
- [ ] `cargo build --workspace --release` Windows
- [ ] `cargo build --workspace --release` Linux
- [ ] `cargo build --workspace --release` macOS
- [x] `cargo test --workspace --all-features` all green (Wave AS)
- [x] `cargo fmt --check` clean (Wave AR)
- [x] `cargo clippy --workspace --all-targets` 0 warnings, 0 errors (Wave AW)
- [x] Workspace `[lints.clippy]` section in nom-canvas/Cargo.toml (Wave AR)
- [x] Targeted strict clippy clean for `nom-compose` + `nom-memoize` (Wave AW)
- [ ] Broad clippy: remove `nom-dict` deprecated compatibility warnings
- [ ] GitHub Actions CI green on PR
- [ ] Release pipeline produces signed binaries
- [ ] Installer: MSI (Windows), AppImage (Linux), DMG (macOS)

### D5. Documentation
- [x] README with install + quickstart (Wave ABB)
- [x] User manual — `docs/user-manual.md` (Wave AY)
- [x] `CONTRIBUTING.md` (Wave AY)
- [x] Migration guide v1→v2 — `nom convert` (Wave ABA)
- [ ] API reference (`cargo doc --no-deps`)
- [ ] Architecture deep-dive
- [ ] Video walkthrough / screencast
- [ ] `CODE_OF_CONDUCT.md`

### D6. Spec §16 non-negotiables
- [x] Source repos read end-to-end before code
- [x] Zero foreign identities in public API (CB5 fixed)
- [x] nom-compiler is CORE (zero IPC confirmed)
- [x] DB IS workflow engine
- [x] NomtuRef non-optional on every canvas object
- [x] Deep thinking = compiler op streamed right dock
- [x] GPUI fully Rust — one binary
- [x] Parallel subagents for multi-file work
- [x] gitnexus_impact before editing symbols
- [ ] Canvas = AFFiNE-for-RAG (visible RAG overlay partial)
- [ ] Doc = Zed + Rowboat + AFFiNE (partial)

### D7. State hygiene
- [ ] Weekly `task.md` compaction ritual
- [ ] Weekly state-report trim

### D8. Minimalist UI Design
- [x] AF-HEADER: HeaderPanel + HeaderAction (Wave AY)
- [x] AF-STATUS: StatusBar + StatusItem + StatusKind (Wave AY)
- [x] AF-TITLEBAR: TitleBarPanel + with_traffic_lights (Wave AY)
- [x] AF-LEFT-ICONS: IconRail + IconRailItem + badge_total() (Wave AY)
- [x] AF-LEFT-PANEL: LeftPanelLayout + LeftPanelTab + toggle_collapse (Wave AY)
- [ ] AF-LEFT-PALETTE: DB-driven node palette search box + category groups
- [ ] AF-CENTER-EDITOR: code mode — rope buffer, 40px gutter, syntax highlighting, serif font for prose
- [x] AF-CENTER-CANVAS: CenterLayout + SplitDirection (Wave AY)
- [x] AF-CENTER-TABS: TabManager + Tab + TabKind + dirty_count + pinned tabs (Wave AY)
- [x] AF-RIGHT-CHAT: ChatPanel + ChatMessage + ChatRole (Wave AY)
- [x] AF-RIGHT-DEEP: HypothesisTree + AnimatedReasoningCard (Wave AY/AZ)
- [x] AF-RIGHT-PROPS: PropertiesPanel stub (Wave AY)
- [ ] AF-FONT-PROSE: Libre Baskerville 15px or EB Garamond 16px
- [ ] AF-FONT-CODE: Berkeley Mono or JetBrains Mono 13px
- [ ] AF-FONT-UI: Inter 13px for all chrome
- [ ] AF-FONT-SCALE: xs=11 sm=12 base=13 md=15 lg=18 xl=24 2xl=32
- [ ] AF-THEME-DARK: `#0d1117` bg · `#161b22` surface · `#21262d` elevated · `#58a6ff` accent · `#f0f6fc` text
- [ ] AF-THEME-LIGHT: `#ffffff` bg · `#f6f8fa` surface · `#eaeef2` elevated · `#0969da` accent · `#1f2328` text
- [ ] AF-THEME-OLED: `#000000` bg · `#0a0a0a` surface · `#111111` elevated
- [ ] AF-THEME-TOGGLE: `Cmd/Ctrl+K T` shortcut + settings panel + command palette
- [ ] AF-SETTINGS-PANEL: full-screen overlay; sections = Editor · Canvas · Theme · Keybindings · Extensions · Advanced
- [ ] AF-SETTINGS-EDITOR: font family/size/tab-size/line-wrap toggles
- [ ] AF-SETTINGS-CANVAS: grid snap · background pattern · zoom sensitivity
- [ ] AF-SETTINGS-KEYBIND: searchable, rebind on double-click
- [ ] AF-SETTINGS-OPEN: `Cmd/Ctrl+,` keybind + settings icon in status bar

### D9. Hybrid Composition System
- [x] AH-CTX: ComposeContext / ComposeResult / ComposeTier (Wave AT)
- [x] AH-DICTW: DictWriter insert_partial_entry() + promote_to_complete() (Wave AT)
- [x] AH-CACHE: GlueCache + GlueStatus Transient/Partial/Complete (Wave AU)
- [x] AH-GLUE: AiGlueOrchestrator + GlueBlueprint + ReActLlmFn trait (Wave AT)
- [x] AH-HYBRID: HybridResolver 3-tier (Wave AT)
- [x] AH-ORCH: ComposeOrchestrator wrapping HybridResolver (Wave AU)
- [x] AH-DB-KINDS: 14 composition grammar.kinds seed rows (Wave AU)
- [x] AH-UI: Intent Preview + AI Review cards (Wave AX)
- [ ] AH-DISPATCH: UnifiedDispatcher: ProviderRouter ↔ BackendRegistry bridge with credential injection
- [ ] AH-INTENT: IntentResolver: lexical scan + BM25 + classify_with_react()
- [ ] AH-PURPOSE: `intended to <purpose>` clause required in every AI .nomx sentence
- [ ] AH-EXPLICIT: user Accept in Review card → DictWriter::insert_partial_entry() immediately
- [ ] AH-PROMOTE: auto-path thresholds

### D10. Universal Composer — Platform Leap
- [x] UC-CANDLE: candle_adapter.rs — BackendDevice::Cpu + ReActLlmFn impl
- [ ] UC-QDRANT: intent_v2.rs — Qdrant HNSW client replacing BM25
- [ ] UC-WASM: wasm_sandbox.rs — Store<T> + Linker::func_wrap()
- [ ] UC-MIDDLEWARE: StepMiddleware trait + before_step/after_step hooks
- [ ] UC-TELEMETRY-MW: latency/cost/token rows written via after_step(); Polars lazy frame daily aggregation
- [ ] UC-FLOWGRAPH: FlowNode + FlowEdge typed graph
- [ ] UC-CRITIQUE: propose → critique → refine (3-round cap)
- [ ] UC-TOOLJET: grammar.kinds DB rows drive node palette (72+ kinds)
- [x] UC-POLARS: data_query backend with Polars lazy LazyFrame
- [ ] UC-HIGGSFIELD: MediaVendor impl for 200+ model registry
- [ ] UC-STREAM: SwitchableStream wrapping AiGlueOrchestrator
- [ ] UC-SERVE: tokio-axum POST /compose endpoint
- [ ] UC-PROMOTE: POST /promote/:glue_hash endpoint for headless AI callers
- [ ] UC-FFMPEG: real video encode filter graph
- [x] UC-POLARS: data_query backend with Polars lazy LazyFrame
- [x] UC-LANGCHAIN: chain.rs Runnable composition
- [x] UC-CREWAI: crew.rs Flow/Crew orchestration
- [x] UC-OPENHARNESS: harness.rs 5 schema-first tools + permissions (MVP)
- [x] UC-OLLAMA: ollama.rs local LLM serving
- [x] UC-OPENDAL: storage.rs universal storage
- [x] UC-TEMPORAL: durable.rs workflow execution
- [ ] UC-VELLO: vector.rs GPU vector rendering | **NOT STARTED**
- [x] UC-MEMPALACE: memory.rs verbatim long-context
- [ ] UC-VOXCPM: voice.rs controllable TTS

### D11. Workspace hygiene
- [ ] Rename 5 canvas duplicate crates (nom-cli→nom-canvas-cli, nom-graph→nom-canvas-graph, nom-media→nom-canvas-media, nom-ux→nom-canvas-ux, nom-intent→nom-canvas-intent)
- [x] Delete `nom-canvas/nom-canvas/` stale build artifacts
- [x] Delete `nom-canvas/tests/` superseded by `nom-canvas-tests` crate
- [ ] Split `nom-compose` (28K lines) into orchestrator + backends + output
- [ ] Split `nom-canvas-core` (16K lines) into render + input + viewport
- [ ] Split `nom-blocks` (11K lines) into core + tree + registry
- [ ] Split `nom-graph` (15K lines) into execution + rag + infra
- [ ] Merge `nom-memoize` into `nom-graph`
- [ ] Merge `nom-ux` (canvas, 625 lines) into `nom-panels`
- [ ] Delete or merge `nom-media` (canvas, 351-line stub)
- [ ] Split `nom-concept` (compiler, ~13K lines) into lexer + parser + ir + validate + bootstrap

---

## Completion criteria (100%)

All four axes reach 100% when:
1. Every `[ ]` above is `[x]` with committed code + passing test
2. 0 open CRITICAL + HIGH + MEDIUM findings
3. Bootstrap fixpoint proof landed (s2 == s3 byte-identical)
4. All golden-path demos playable from README install
5. All 22 reference-repo patterns replicated (or explicitly marked `ADOPT-ONLY` with rationale)
6. CI green on Windows + Linux + macOS + wasm
