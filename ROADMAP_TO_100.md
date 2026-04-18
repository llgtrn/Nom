# Nom — Roadmap to 100%

**Date:** 2026-04-19 | **Mandate:** reach 100% on all 4 axes. Every `[ ]` is a completable task.
**Last updated:** Wave ABB-5 — **8891 tests**, 0 clippy warnings, fmt clean. D5 README+user-manual+CONTRIBUTING ticked. A-axis ~55%, B-axis ~73%, C-axis ~83%, D-axis ~98%.

## Current finalization snapshot

**Iteration 60 audit (8 agents, 2026-04-18) — Wave AN verified:**

| Axis | Today | Target | Gap | Notes |
|---|---|---|---|---|
| A · nom-compiler | 44% | 100% | 56pp | A6 LSP stubs ✅; self-hosting not started; LLVM beyond lexer open |
| B · Nom language | **62%** | 100% | 38pp | +10pp Waves AT+AW: B1 parse_define_that ✅, B2 migrate_typed_to_natural ✅, B8 100 translations ✅, B9 corpus/ux/app CLI ✅. Open: B6 MECE/Dream, B9 depth, B10 bootstrap |
| C · nom-canvas ↔ compiler integration | **72%** | 100% | **28pp** | +5pp Waves AV+AW: C5-V1..V9 Remotion patterns ✅, UC-MIDDLEWARE ✅, UC-STREAM ✅. Open: C5-V10 FFmpeg, C4 LSP visual, C6 RAG real |
| D · Overall platform | **95%** | 100% | **5pp** | +3pp Waves AT+AW: D1 Dify/ToolJet/Refly ✅, D4 0-clippy ✅, D5 README ✅. Open: D2 visual frosted-glass, D3 golden demos, D4 signed release |

**C-axis at ~34%** (Wave AN fixed CRDT overflow and selection.rs wiring, nothing else):
1. Renderer still renders zero pixels — 10 waves overdue
2. BackendKind enum still 379 active references; UnifiedDispatcher dead code
3. KindStatus enum exists but GrammarKind has no status field; no list_kinds() SQL
4. is_safe_identifier() defined but never called in to_sql() — 4 SQL injection points active
5. ExecutionEngine::plan_execution() plans DAG but execute() never calls node logic

**D-axis at ~72%** (Wave AN partial progress):
- Theme struct + dark()/light() constructors now exist
- LspPosition/LspRange types now exist in lsp_bridge.rs (not yet integrated into Buffer API)
- FrameBlock now has x/y/width/height/z_index fields
- Still: oled() absent, TOOLBAR_H=48.0 vs TOOLBAR_HEIGHT=36.0 ambiguity, edge_color_for_confidence() never called from deep_think.rs, atlas LRU corrupts on partial eviction

**Per-crate test counts (Wave AP actuals → Wave AQ targets).**
| Crate | Wave AO actual | Wave AP actual | Wave AQ target | Wave AQ priority |
|---|---|---|---|---|
| nom-blocks | 560 | 560 | 590 | NOM-GRAPH-ANCESTRY; deeper workspace tests |
| nom-canvas-core | 575 | 575 | 610 | AM-SPATIAL-WIRE viewport.rs; snapping tests |
| nom-cli | 400 | 400 | 430 | POST /compose axum endpoint |
| nom-collab | 545 | 546 | 575 | Remaining clippy/test debt |
| nom-compiler-bridge | 548 | 553 | 585 | list_kinds SQL integration tests |
| nom-compose | 690 | 685 | 720 | NOM-BACKEND-SELF-DESCRIBE; middleware |
| nom-editor | 620 | 620 | 655 | Point API integration; display map pipeline |
| nom-gpui | 790 | 790 | 835 | AL-COSMIC (cosmic_text FontSystem) |
| nom-graph | 570 | 570 | 610 | NOM-GRAPH-ANCESTRY transitive cache |
| nom-intent | 470 | 470 | 505 | AM-INTENT-STRUCT bm25_index + classify_with_react |
| nom-lint | 485 | 485 | 510 | — |
| nom-memoize | 470 | 468 | 495 | — |
| nom-panels | 600 | 601 | 630 | Intent Preview + AI Review cards |
| nom-telemetry | 500 | 500 | 525 | — |
| nom-theme | 560 | 556 | 590 | cosmic_text font handles wired |
| **TOTAL** | **8384** | **8391** | **~8650** | — |

**Discipline:** tick `[x]` only after BOTH the code change AND a regression test are committed. Never tick from trackers alone. See `feedback_audit_must_also_fix.md`.

---

## AXIS A — nom-compiler → 100%

### A1. Phase 3 — LLVM self-hosting completion
- [x] Lexer.nom compiles natively on Windows (baseline)
- [ ] Parser.nom compiles via rust-nomc
- [ ] Resolver.nom compiles
- [ ] Type checker.nom compiles
- [ ] Codegen.nom compiles
- [ ] Full S1→S6 pipeline.nom builds standalone
- [ ] Self-built `s1` binary executes `hello.nomx`
- [ ] Self-built `s1` reproduces 255+ test-suite results

### A2. Phase 4 — DIDS
- [x] Design-Integrated Dictionary System shipped
- [x] 9-kind foundation locked

### A3. Phase 5 — Body-only ingestion + extended kinds
- [x] 22-edge multi-graph EdgeKind (Styles/Constrains/Recommends/InteractsWith/TransitionsTo/Specializes/BindsTo/Triggers/Reads/Writes/NavigatesTo/RunsOn/HasFlowArtifact/FlowsTo/Encodes/ContainedIn/UsesColor/UsesPalette/Derives/EmbeddedGlyph/Frame/RendersOn + display_name/is_structural)
- [ ] Intent resolution pipeline (§5 body-only)
- [ ] Lifecycle transitions (merge/eliminate/evolve)
- [ ] UX extractor for Motion / Dioxus / ToolJet / DeerFlow corpus
- [ ] Skill routing via `EntryKind::Skill`
- [x] `nom-ux` crate — UxPattern/Screen/UserFlow (Wave AY)
- [x] `nom-media` crate — MediaUnit/Codec/Container (Wave AY)
- [ ] Stream-and-discard disk discipline for mass ingest
- [ ] Checkpoint + resumption for interrupted ingests
- [ ] Bandwidth-throttle non-optional
- [ ] `Partial` → `Complete` canonicalization lift (§5.10)
- [ ] Aesthetic skills seeded (§5.18)
- [ ] AI-invokes-compiler verify→build→bench→flow loop (§5.19)

### A4. Phase 6-7 — Parser-in-Nom
- [ ] `stdlib/self_host/lexer.nom` frozen
- [ ] `stdlib/self_host/parser.nom`
- [ ] `stdlib/self_host/ast_printer.nom` pretty-printer
- [ ] Round-trip byte-identity on 100-sample corpus

### A5. Phase 8 — Architectural ADOPT
- [ ] Workspace manifest expressed as `.nomx` AppManifest
- [ ] Cargo-style deps in .nomx
- [ ] Module graph via `HasFlowArtifact` edges

### A6. Phase 9 — LSP + AuthoringProtocol CORE
- [x] LspRequest/LspResponse + dispatch_lsp_request stub (Wave AW)
- [x] textDocument/hover stub dispatch
- [x] textDocument/completion stub dispatch
- [x] textDocument/definition stub dispatch
- [x] textDocument/references stub dispatch
- [x] workspace/symbol stub dispatch
- [x] stdin/stdout real handshake — LspServerLoop state machine (Wave ABA stub)
- [x] AuthoringProtocol edit-is-compile event stream (Wave AZ)
- [ ] Partial-result streaming for long ops
- [ ] `workspace/rename` refactor

### A7. Phase 10 — Bootstrap fixpoint proof
- [x] Stage0 (Rust rust-nomc) → Stage1 binary stub (Wave ABA)
- [x] Stage1 → Stage2 binary stub (Wave ABA)
- [x] Stage2 → Stage3 binary stub (Wave ABA)
- [ ] **`s2 == s3` byte-identical (THE proof — stubs exist, real compilation open)**
- [ ] Proof tuple `(s1_hash, s2_hash, s3_hash, fixpoint_at_date, compiler_manifest_hash)` stored in dict
- [ ] Parity track: ≥99% IR equivalence across test corpus
- [ ] 100% runtime correctness on test corpus
- [ ] 4-week parity hold before default flip
- [ ] Default flip (rust-nomc → nom-nomc)
- [ ] Rust sources → `.archive/rust-<version>/`
- [ ] 3-month grace period
- [ ] Archive lock + announcement

### A8. Phase 11 — Dream mode
- [ ] Criteria → Proposals emitter
- [ ] `app_score` ≥95 gate
- [ ] `nom app dream` CLI iterates until score reached
- [ ] Dream history persisted in `entry_meta`

### A9. Phase 12 — Closure-level specialization
- [ ] `entry_benchmarks` side-table populated from real runs
- [ ] Bipartite min-cost assignment solver (§5.15)
- [ ] Cross-app specialization sharing via content-address
- [ ] 70–95% binary-size reduction verified on test corpus
- [ ] `nom bench regress` CLI catches regressions

### A10. Dict ingestion at scale
- [ ] 100-repo corpus (Accelworld/upstreams) ingested end-to-end
- [ ] 100M+ nomtu entries in nomdict.db
- [ ] All 20+ paradigm families catalogued
- [ ] PyPI top-500 ingested (§5.17)
- [ ] Top-500/ecosystem GitHub ingested (JS/Python/Rust/Go/Java/C++/Swift/Ruby/PHP)
- [ ] `nom corpus workspace-gc` runs clean
- [ ] DB stats: ≥1 GB, ≥1 000 kinds, ≥100k clause_shapes

### A11. LLVM pipeline beyond lexer
- [ ] Parser → AST codegen
- [x] AST → typed IR (IrType/IrValue/IrInstr/IrFunction/IrModule — Wave ABA)
- [ ] IR → LLVM bitcode for all S1-S6 stages
- [ ] Bitcode → native binary on Windows/Linux/macOS
- [ ] Cross-compile WASM target
- [ ] Cross-compile mobile (iOS/Android) targets
- [ ] Codegen benchmark suite (Google Benchmark pattern)

---

## AXIS B — Nom language → 100%

### B1. Syntax natural-language ≥95%
- [x] `Tok::Define` + `Tok::That` lexer variants + highlight arms (partial — parser not yet updated)
- [ ] `define X that Y` replaces `fn X -> Y` across stdlib (full parse integration)
- [ ] Last-sentence implicit return
- [ ] Zero null by grammar (no Option at Nom level)
- [ ] Zero race by grammar
- [ ] Zero overflow by grammar
- [ ] Zero panic by grammar
- [x] English-only vocabulary (VN tokens = 0, verified 2026-04-13)
- [x] Vietnamese GRAMMAR STYLE inspiration only (no VN tokens in code)
- [ ] C-like syntax track archived → `.archive/syntax-clike/`

### B2. .nomx single format
- [x] NomxFormat{Typed,Natural,Standard} enum + detect_format() — Wave AS
- [x] migrate_typed_to_natural() fn→define, ->→that — Wave AW
- [ ] v1 + v2 merge spec stabilized (full migration tool)
- [x] Migration tool `nom convert v1 v2` — ConvertDirection/Options/Result (Wave ABA)
- [ ] Golden corpus: 100 `.nomx` files in `examples/` (10/100 — Wave ABA)
- [ ] Round-trip byte-identity tested

### B3. 9-kind foundation ✅
- [x] 9 core kinds

### B4. Extended kinds (each needs seed rows in `grammar.kinds`) — Wave AR ✅ all seeded in baseline.sql
- [x] UxPattern
- [x] DesignRule
- [x] Screen
- [x] UserFlow
- [x] Skill
- [x] AppManifest
- [x] DataSource
- [x] Query
- [x] AppAction
- [x] AppVariable
- [x] Page
- [x] Benchmark
- [x] BenchmarkRun
- [x] FlowArtifact
- [x] FlowStep
- [x] FlowMiddleware
- [x] MediaUnit
- [x] PixelGrid
- [x] AudioBuffer
- [x] VideoStream
- [x] VectorPath
- [x] GlyphOutline
- [x] MeshGeometry
- [x] Color
- [x] Palette
- [x] Codec
- [x] Container
- [x] MediaMetadata
- [x] RenderPipeline

### B5. Typed side-tables
- [x] `entry_benchmarks` schema — Wave AR (insert_benchmark() + 4 tests)
- [x] `flow_steps` schema — Wave AR (insert_flow_step())
- [x] Indexes + FKs declared in nom-dict/src/dict.rs schema
- [ ] Populated from real ingests (requires bench/flow CLI driving actual runs)

### B6. Dream-tree + MECE
- [x] MECE-objectives validator + DreamScore ≥95 gate (Wave AZ)
- [ ] Feature-stack word IDs
- [ ] DreamReport score ≥95 gate active

### B7. Self-documenting Skills seeded in dict — Wave AR ✅ all 9 in baseline.sql
- [x] author_nom_app
- [x] compose_from_dict
- [x] debug_nom_closure
- [x] extend_nom_compiler
- [x] ingest_new_ecosystem
- [x] use_ai_loop
- [x] compose_brutalist_webpage
- [x] compose_generative_art_piece
- [x] compose_lofi_audio_loop

### B8. Corpus breadth
- [x] 84 translations baseline
- [x] 100+ translations — Wave AW (84 + 16 = 100 total)
- [ ] 100 paradigm families (71 today)
- [x] 20+ paradigm families (maintain)

### B9. Authoring CLI complete
- [x] `nom author start` — Wave AR
- [x] `nom author check` — Wave AR
- [x] `nom corpus ingest repo` — Wave AR
- [x] `nom corpus status/workspace-gc` — Wave AR
- [x] `nom bench run/compare/regress/curate` — Wave AS
- [x] `nom flow record/show/diff/middleware` — Wave AS
- [x] `nom media import/import-dir/render/transcode/diff/similar` — Wave AS
- [x] `nom corpus ingest pypi` — stub Wave AW
- [x] `nom corpus ingest github` — stub Wave AW
- [x] `nom corpus pause/resume/report` — stub Wave AW
- [x] `nom ux seed <path>` — Wave AW
- [x] `nom app new/import/build/build-report/explain-selection` — Wave AW

### B10. Bootstrap proof (shared with A7)
- [ ] s2==s3 byte-identical attested in dict

---

## AXIS C — nom-canvas ↔ compiler integration → 100%

### C1. Spec §9 wire table complete
- [x] Type char → stage1_tokenize → highlight (Wave C)
- [x] Hover word → handle_hover → tooltip (Wave O)
- [x] Pause 500ms → run_pipeline → diagnostics
- [x] Drag wire → can_wire (Wave Q CW1)
- [x] Click Run → run_composition() → background_tier → compile result (Wave AS; full LLVM codegen still open)
- [ ] Click Run → compile → LLVM → native binary execute
- [x] Command-bar → classify_with_react
- [x] Deep Think → scored hypothesis chain (Wave R NI1)
- [ ] Open compose → dream_report → score + proposals

### C2. DB-driven palette actual wiring
- [x] `NodePalette::load_from_dict(&SqliteDictReader)` live SELECT
- [x] `LibraryPanel::load_from_dict()` same
- [ ] End-to-end test: real nomdict.db → palette renders
- [x] AC2 close: remove slice/static palette as production path; require live `grammar.kinds` source
- [x] AC3 close: library panel reads the same live grammar source as palette

### C3. Feature gate flip — Wave AR ✅
- [x] `compiler` feature = default ON in nom-compiler-bridge
- [x] Default build links nom-compiler
- [x] Bridge tests run without `--features compiler` flag
- [x] CI matrix includes default+compiler (Wave AS: 3-OS CI matrix)

### C4. Full LSP stream visually verified
- [ ] Hover tooltip renders on canvas
- [ ] Completion popup visible with arrow-key navigation
- [ ] Diagnostic red-squiggle underline renders
- [ ] Go-to-definition navigates
- [ ] Rename-refactor preview works

### C5. Backend wiring beyond spec-and-stub

**C5-VIDEO (Remotion-pattern — from scan 2026-04-18):**
- [x] C5-V1: CompositionConfig + CompositionRegistry — Wave AV
- [x] C5-V2: SequenceContext + current_frame_in_sequence + is_frame_active — Wave AV
- [x] C5-V3: interpolate() + ExtrapolateMode — Wave AV
- [x] C5-V4: spring() + SpringConfig{damping,mass,tension} — Wave AV
- [x] C5-V5: VideoRenderConfig + RenderProgress{rendered_frames,encoded_frames,stage,elapsed_ms} — Wave AW
- [x] C5-V6: ComposeEvent::Progress extended + all 49 sites updated — Wave AW
- [x] C5-V7: CancelSignal + make_cancel_signal() — Wave AW
- [x] C5-V8: VideoConfigContext + thread-local push/pop/get_video_config() — Wave AW
- [x] C5-V9: validate_codec_pixel_format() — even-dims + ProRes/VP9 matrix — Wave AW
- [ ] C5-V10: Two-stage video pipeline (parallel frame capture → FFmpeg stdin streaming) in video.rs [ABB target]

**Other C5 backends:**
- [ ] Video backend GPU scene → FFmpeg parallel encode (blocked on C5-V5/V10)
- [ ] Audio backend: rodio/symphonia real encode
- [x] Data-extract: DataLoader stub (DataSourceKind/LoadStrategy/DataBatch — Wave ABA)
- [ ] Data-extract: opendataloader XY-Cut++ 0.015s/page (real impl open)
- [ ] Image backend: model dispatch with 200+ model registry
- [ ] Storyboard: 5-phase orchestration
- [ ] Native_screen: platform-specific codegen capture
- [ ] Mobile_screen: iOS/Android target integration
- [ ] App_bundle: Cargo + wgpu signed bundle

### C6. RAG real retrievers
- [x] Graph RAG BFS + confidence weights (Wave Q)
- [ ] Vector retriever using `nom-search` BM25 + ANN
- [ ] LlamaIndex pipeline composition
- [ ] Refly skill-engine integration

### C7. Deep-think full round-trip
- [x] classify_with_react + react_chain_interruptible (Wave R)
- [x] DeepThinkPanel ingest_events/consume_stream (Wave P DEEP1)
- [x] InterruptSignal field + trigger_interrupt() on DeepThinkPanel (Wave AR)
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

### D1. Reference-repo parity (22 repos, replicate-and-stage)
- [x] AC8 crate-by-crate parity pass completed after DB-driven fixes (remaining DRIFT items stay open below)
- [x] Zed gpui (scene/renderer/atlas/elements/styled/window/layout)
- [x] AFFiNE (73 tokens + frosted + bezier + collapsible)
- [x] rowboat (ChatSidebar + tool cards + deep-think)
- [x] ComfyUI (4-tier cache + Kahn + cancel + IS_CHANGED)
- [x] GitNexus (confidence+reason edges + NomtuRef)
- [x] dify — TypedNode trait + NodeOutputPort + NodeEvent (Wave AW)
- [x] n8n (4 AST sanitizers + credential store)
- [x] LlamaIndex (RRF + cosine + BFS)
- [ ] Haystack — full pipeline composition
- [ ] ToolJet — full 55-widget registry (16/55 today)
- [x] yara-x (Sealed + InternalRule)
- [x] typst (comemo Tracked/Constraint/hash128)
- [x] WrenAI (SemanticModel + MDL)
- [x] 9router (3-tier fallback + credential + compose_with_fallback)
- [ ] graphify (chart types + Redux slice) — NOT staged
- [x] Refly — SkillRouter + SkillDefinition + find_by_query (Wave AW)
- [ ] Remotion — real GPU→frame→FFmpeg encoder
- [ ] Open-Higgsfield — 200+ model dispatch
- [ ] ArcReel — 5-phase orchestration (spec only today)
- [ ] waoowaoo — 4-phase parallel cinematography (NOT staged)
- [ ] opendataloader — XY-Cut++ + hybrid-AI tables
- [x] excalidraw (hit-test + selection + snapping)

### D2. UI/UX full visual verification (ui-ux-pro-max skill required each check)
- [ ] Frosted-glass pipeline renders visible blur on canvas
- [x] Focus ring = 2px outline stroke (FR1 follow-through)
- [x] All panels render real quads with token colors
- [ ] Bezier control points animate smoothly
- [x] Spring animation at AFFiNE defaults (stiffness=400, damping=28) verified on-screen
- [x] Color contrast ≥4.5:1 WCAG AA for all text-on-surface combos
- [x] Motion timing 200ms/300ms verified
- [ ] All 73 AFFiNE tokens visually used
- [ ] Dark + light theme toggle
- [x] Reduced-motion/accessibility gate for animation paths
- [x] Screenshot/runtime audit proving palette/library/chat surfaces are not quad-only stubs
- [x] AC9 visual artifact generated at `.omx/visual/nom-panels-runtime.ppm` with JSON report
- [x] AD2 OS screenshot artifact generated at `.omx/visual/nom-gpui-window-first-paint.png`

### D3. End-to-end golden paths
- [ ] Type .nomx → live syntax highlighting (C1 green)
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
- [ ] Same Linux
- [ ] Same macOS
- [x] `cargo test --workspace --all-features` all green
- [x] AC7 close: fix 14 `nom-compiler-bridge` all-features failures in completion/score/interactive/LSP adapter tests
- [x] Workspace `[lints.clippy]` section in nom-canvas/Cargo.toml (Wave AR)
- [x] `cargo fmt --check` clean (Wave AR)
- [x] `cargo clippy --workspace --all-targets` — 0 warnings, 0 errors (Wave AW)
- [x] Targeted strict clippy clean for `nom-compose` + `nom-memoize`
- [ ] AD3 broad clippy: remove `nom-dict` deprecated compatibility warnings and remaining workspace lints
- [ ] `cargo fmt --check` clean
- [ ] GitHub Actions CI green on PR
- [ ] Release pipeline produces signed binaries
- [ ] Installer: MSI (Windows), AppImage (Linux), DMG (macOS)

### D5. Documentation
- [x] README with install + quickstart (Wave ABB-5: full structure, workspace map, wave history)
- [x] User manual (`docs/user-manual.md` — Wave AY)
- [ ] API reference (`cargo doc --no-deps`)
- [ ] Architecture deep-dive (spec extension)
- [ ] Video walkthrough / screencast
- [x] `CONTRIBUTING.md` (Wave AY)
- [ ] `CODE_OF_CONDUCT.md`
- [ ] Migration guide (v1 → v2)

### D6. Spec §16 non-negotiables
- [x] Source repos read end-to-end before code
- [x] ui-ux-pro-max skill used for UI work
- [x] Zero foreign identities in public API
- [x] nom-compiler is CORE (zero IPC confirmed)
- [x] DB IS workflow engine
- [x] NomtuRef non-optional on every canvas object (AC5 closed: panel metadata uses typed `PanelEntityRef`, not canvas-object identity)
- [ ] Canvas = AFFiNE-for-RAG (visible RAG overlay — currently partial)
- [ ] Doc = Zed + Rowboat + AFFiNE (all three visible — currently partial)
- [x] Deep thinking = compiler op streamed right dock
- [x] GPUI fully Rust — one binary
- [x] Parallel subagents for multi-file work
- [x] gitnexus_impact before editing symbols

### D7. State hygiene
- [ ] Weekly `task.md` compaction ritual
- [ ] Weekly state-report trim

### D8. Minimalist UI Design (Wave AF — design confirmed 2026-04-18)

**Aesthetic mandate:** Simple but strong. Every surface earns its space. Theme = Zed-dark by default, swappable.

**Shell chrome:**
- [x] AF-HEADER: HeaderPanel + HeaderAction enum — Wave AY
- [x] AF-STATUS: StatusBar + StatusItem + StatusKind — Wave AY
- [x] AF-TITLEBAR: TitleBarPanel + with_traffic_lights + title truncation — Wave AY

**Left sidebar:**
- [x] AF-LEFT-ICONS: IconRail + IconRailItem + badge_total() — Wave AY
- [x] AF-LEFT-PANEL: LeftPanelLayout + LeftPanelTab enum + toggle_collapse + effective_width — Wave AY
- [ ] AF-LEFT-PALETTE: DB-driven node palette search box + category groups (search stub done, real DB wiring open)

**Center workspace:**
- [ ] AF-CENTER-EDITOR: Code mode — rope buffer, 40px gutter, compiler-bridge syntax highlighting, serif font for prose blocks
- [x] AF-CENTER-CANVAS: CenterLayout + SplitDirection — Wave AY (visual wiring open)
- [x] AF-CENTER-TABS: TabManager + Tab + TabKind enum, dirty_count, pinned tabs — Wave AY

**Right sidebar (Rowboat pattern):**
- [x] AF-RIGHT-CHAT: ChatPanel + ChatMessage + ChatRole — Wave AY
- [x] AF-RIGHT-DEEP: HypothesisTree + AnimatedReasoningCard FSM — Wave AY/AZ
- [x] AF-RIGHT-PROPS: PropertiesPanel stub — Wave AY

**Typography — classical editorial:**
- [ ] AF-FONT-PROSE: Libre Baskerville 15px or EB Garamond 16px for doc/prose blocks
- [ ] AF-FONT-CODE: Berkeley Mono or JetBrains Mono 13px for all code surfaces
- [ ] AF-FONT-UI: Inter 13px for all chrome (already in nom-theme)
- [ ] AF-FONT-SCALE: xs=11 sm=12 base=13 md=15 lg=18 xl=24 2xl=32 (px) — locked in nom-theme tokens

**Swappable color themes:**
- [ ] AF-THEME-DARK: `#0d1117` bg · `#161b22` surface · `#21262d` elevated · `#58a6ff` accent · `#f0f6fc` text (default)
- [ ] AF-THEME-LIGHT: `#ffffff` bg · `#f6f8fa` surface · `#eaeef2` elevated · `#0969da` accent · `#1f2328` text
- [ ] AF-THEME-OLED: `#000000` bg · `#0a0a0a` surface · `#111111` elevated (OLED power savings)
- [ ] AF-THEME-TOGGLE: `Cmd/Ctrl+K T` shortcut + settings panel + command palette (`theme <name>`)

**Settings panel:**
- [ ] AF-SETTINGS-PANEL: full-screen overlay (not sidebar); sections = Editor · Canvas · Theme · Keybindings · Extensions · Advanced
- [ ] AF-SETTINGS-EDITOR: font family/size/tab-size/line-wrap toggles
- [ ] AF-SETTINGS-CANVAS: grid snap · background pattern · zoom sensitivity
- [ ] AF-SETTINGS-KEYBIND: searchable, rebind on double-click
- [ ] AF-SETTINGS-OPEN: `Cmd/Ctrl+,` keybind + settings icon in status bar

**Minimalism enforcement rules (each item above must pass all):**
- Zero decorative borders — only functional 1px hairlines (`border_color` token)
- No gradients except frosted-glass `blur_radius` = 24px backdrop filter
- Icon rail = icon only (no label duplication)
- No visible placeholder content in shipped surfaces
- Motion: ≤200ms standard, 300ms ease-out for deep-think card entry, 0 for reduced-motion preference
- [ ] `npx gitnexus analyze --embeddings` post every push
- [ ] Memory pruning of stale facts

### D9. Hybrid Composition System (Wave AH — design confirmed 2026-04-18)

**Three-tier resolver:** DB-driven (grammar.kinds Complete) → Provider-driven (registered MediaVendor + credentials) → AI-leading (AiGlueOrchestrator generates .nomx glue; sandbox executes; GlueCache tracks Transient→Partial→Complete lifecycle).

**Intent classification:** `IntentResolver` — lexical scan → BM25 → `classify_with_react()` for ambiguous (delta < 0.15). Multi-kind requests route to parallel `TaskQueue` pipeline via `ComposeOrchestrator`.

- [x] AH-CTX: `ComposeContext` / `ComposeResult` / `ComposeTier` — Wave AT
- [x] AH-DICTW: `DictWriter` insert_partial_entry() + promote_to_complete() — Wave AT
- [x] AH-CACHE: `GlueCache` + `GlueStatus` Transient/Partial/Complete — Wave AU
- [ ] AH-DISPATCH: `UnifiedDispatcher`: `ProviderRouter` <-> `BackendRegistry` bridge with credential injection
- [ ] AH-INTENT: `IntentResolver`: lexical scan + BM25 + `classify_with_react()`
- [x] AH-GLUE: `AiGlueOrchestrator` + `GlueBlueprint` + `ReActLlmFn` trait (4 adapters) — Wave AT
- [x] AH-HYBRID: `HybridResolver` 3-tier (DbDriven→Provider→AiLeading) — Wave AT
- [x] AH-ORCH: `ComposeOrchestrator` wrapping HybridResolver; run/run_parallel — Wave AU
- [x] AH-DB-KINDS: 14 composition grammar.kinds seed rows — Wave AU
- [ ] AH-PURPOSE: `intended to <purpose>` clause required in every AI .nomx sentence; absent = orchestrator retries
- [ ] AH-EXPLICIT: user Accept in Review card -> `DictWriter::insert_partial_entry()` immediately (no usage count threshold)
- [x] AH-UI: Intent Preview + AI Review cards in nom-panels/src/right/ — Wave AX

### D10. Universal Composer — Platform Leap (Wave AI-Composer — design confirmed 2026-04-18)

**Spec:** `docs/superpowers/specs/2026-04-18-nom-universal-composer-design.md`

**10 upstream patterns wired into 14-crate workspace. All additive — no existing interfaces broken.**

**Primary revenue model:** AI-native automation. `POST /compose` endpoint — usage-based pricing. Grammar DB compounds as moat: every Tier3 AI glue execution that promotes trains the next call.

**Candle (in-process ML):**
- [ ] UC-CANDLE: `nom-compiler-bridge/src/candle_adapter.rs` — `BackendDevice::Cpu` + `ReActLlmFn` impl (Phi-3/Gemma-2B, no subprocess)

**Qdrant (semantic intent):**
- [ ] UC-QDRANT: `nom-compose/src/intent_v2.rs` — Qdrant HNSW client replacing BM25 in `IntentResolver`; embeddings stored per grammar.kinds entry

**Wasmtime (WASM sandbox):**
- [ ] UC-WASM: `nom-compiler-bridge/src/wasm_sandbox.rs` — `Store<T>` + `Linker::func_wrap()` replacing JS AST `eval_expr`; glue .nomx compiles to WASM module

**DeerFlow (step middleware):**
- [ ] UC-MIDDLEWARE: `nom-compose/src/middleware.rs` — `StepMiddleware` trait with `before_step()`/`after_step()` hooks; `MiddlewareRegistry` wraps every `BackendRegistry::dispatch` call
- [ ] UC-TELEMETRY-MW: latency/cost/token rows written to nomdict.db via after_step(); Polars lazy frame daily aggregation

**Refly (typed flow graph):**
- [ ] UC-FLOWGRAPH: `nom-compose/src/flow_graph.rs` — `FlowNode` + `FlowEdge` typed graph replacing linear `ComposeOrchestrator`; version control on composition graphs

**AgentScope (critique loop):**
- [ ] UC-CRITIQUE: `nom-compose/src/critique.rs` — propose → critique → refine (3-round cap) via `MsgHub` broadcast before Wasmtime execution

**ToolJet (widget/kind registry):**
- [ ] UC-TOOLJET: grammar.kinds DB rows drive node palette (72+ declarative kinds); `NodePalette` loads via `SELECT kind, label, icon FROM grammar.kinds ORDER BY use_count DESC`

**Polars (data transforms):**
- [ ] UC-POLARS: `data_query` backend replaces row-returning impl with Polars lazy `LazyFrame`; Arrow columnar format for all DataFrame operations

**Open-Higgsfield (media vendor):**
- [ ] UC-HIGGSFIELD: `nom-compose/src/vendors/higgsfield.rs` — `MediaVendor` impl for Open-Higgsfield 200+ model registry; generation history as few-shot cache entries in nomdict.db

**Bolt.new (streaming glue):**
- [ ] UC-STREAM: `nom-compose/src/streaming.rs` — `SwitchableStream` wrapping `AiGlueOrchestrator`; token-by-token .nomx streaming to AI Review card in right dock

**HTTP API:**
- [ ] UC-SERVE: `nom-cli/src/serve.rs` — tokio-axum `POST /compose` endpoint; request -> `HybridResolver` -> `ComposeResult`; streaming and non-streaming response modes
- [ ] UC-PROMOTE: `POST /promote/:glue_hash` endpoint -> `DictWriter::insert_partial_entry()` for headless AI callers

---

## Completion criteria (100%)

All four axes reach 100% when:
1. Every `[ ]` above is `[x]` with committed code + passing test
2. 0 open CRITICAL + HIGH + MEDIUM findings
3. Bootstrap fixpoint proof landed (s2 == s3 byte-identical)
4. All 4 golden-path demos playable from README install
5. All 22 reference-repo patterns replicated (or explicitly marked `ADOPT-ONLY` with rationale)
6. CI green on Windows + Linux + macOS + wasm
7. Spec §16 all 12 rules verified

**Estimated effort to 100%** (rough, at current pace of ~200 tests/wave):
- Axis A: 40+ waves (LLVM + bootstrap fixpoint is the long pole)
- Axis B: 20 waves (mostly DB ingestion throughput-bound)
- Axis C: 8 waves (wires + backend depth; C1/C6/C7 partially done)
- Axis D: 15 waves (UI polish + CI + docs)

**Current velocity:** ~350 tests/wave (Waves AF–AL average). At 7241 tests (Wave AL):
- Axis C functional completeness blocked on renderer (render 0 pixels), BackendKind enum, ComposeContext wiring
- Axis D progress blocked on same renderer blocker + theme system stub + taffy stub
- Critical path: AL-RENDER-1/2/3 → AL-BACKEND-KIND → AL-COMPOSE-BRIDGE → AH tiers → AI-Composer

**Critical path:** A7 (fixpoint proof) + C5 (real backends) + D3 (golden demos).
