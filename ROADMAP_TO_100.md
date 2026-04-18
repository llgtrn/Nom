# Nom — Roadmap to 100%

**Date:** 2026-04-18 | **Mandate:** reach 100% on all 4 axes. Every `[ ]` is a completable task.
**Last updated:** Wave AR COMPLETE — HEAD `fc67aa9`, **8391 tests**. Waves AP+AQ+AR closed 35+ items. C-axis 65%, D-axis 90%. Wave AS targets: C1 Click Run→LLVM, C3 default build, C4 LSP visual, C8 WASM, B1 syntax evolution, B9 remaining CLI, A6 LSP protocol.

## Current finalization snapshot

**Iteration 60 audit (8 agents, 2026-04-18) — Wave AN verified:**

| Axis | Today | Target | Gap | Notes |
|---|---|---|---|---|
| A · nom-compiler | 44% | 100% | 56pp | Lexer done; self-hosting not started; 22/29 crates never called from canvas |
| B · Nom language | 34% | 100% | 66pp | 9-kind foundation locked; C-like syntax; 30+ extended kinds unseeded |
| C · nom-canvas ↔ compiler integration | **65%** | 100% | **35pp** | +13pp Waves AQ+AR: NOM-GRAPH-ANCESTRY, SELF-DESCRIBE, BM25+classify_with_react, cosmic_text init, viewport SpatialIndex, POST /compose, InterruptSignal, compiler feature=default, multi-graph EdgeKind 22 variants. Open: C1 Click Run→LLVM, C4 LSP visual, C5 real backends, C6 RAG retrievers, C8 WASM |
| D · Overall platform | **90%** | 100% | **10pp** | +8pp Waves AQ+AR: 46 kinds seeded, entry_benchmarks+flow_steps schemas, nom author/corpus CLI, D4 clippy+fmt clean, ZERO foreign names. Open: D2 full visual verification, D3 golden paths, D4 CI/release pipeline, D5 docs |

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
- [ ] 21-edge multi-graph schema (Styles/Constrains/Recommends/InteractsWith/TransitionsTo/Specializes/BindsTo/Triggers/Reads/Writes/NavigatesTo/RunsOn/HasFlowArtifact/FlowsTo/Encodes/ContainedIn/UsesColor/UsesPalette/Derives/EmbeddedGlyph/Frame/RendersOn)
- [ ] Intent resolution pipeline (§5 body-only)
- [ ] Lifecycle transitions (merge/eliminate/evolve)
- [ ] UX extractor for Motion / Dioxus / ToolJet / DeerFlow corpus
- [ ] Skill routing via `EntryKind::Skill`
- [ ] `nom-ux` crate (peer to nom-extract)
- [ ] `nom-media` crate (peer to nom-extract)
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
- [ ] stdin/stdout handshake
- [ ] textDocument/hover
- [ ] textDocument/completion (streaming)
- [ ] textDocument/definition
- [ ] textDocument/references
- [ ] workspace/symbol
- [ ] AuthoringProtocol edit-is-compile event stream
- [ ] Partial-result streaming for long ops
- [ ] `workspace/rename` refactor

### A7. Phase 10 — Bootstrap fixpoint proof
- [ ] Stage0 (Rust rust-nomc) → Stage1 binary
- [ ] Stage1 → Stage2 binary
- [ ] Stage2 → Stage3 binary
- [ ] **`s2 == s3` byte-identical (THE proof)**
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
- [ ] AST → typed IR
- [ ] IR → LLVM bitcode for all S1-S6 stages
- [ ] Bitcode → native binary on Windows/Linux/macOS
- [ ] Cross-compile WASM target
- [ ] Cross-compile mobile (iOS/Android) targets
- [ ] Codegen benchmark suite (Google Benchmark pattern)

---

## AXIS B — Nom language → 100%

### B1. Syntax natural-language ≥95%
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
- [ ] v1 + v2 merge spec stabilized
- [ ] Migration tool `nom convert v1 v2`
- [ ] Golden corpus: 100 `.nomx` files in `examples/`
- [ ] Round-trip byte-identity tested

### B3. 9-kind foundation ✅
- [x] 9 core kinds

### B4. Extended kinds (each needs seed rows in `grammar.kinds`)
- [ ] UxPattern
- [ ] DesignRule
- [ ] Screen
- [ ] UserFlow
- [ ] Skill
- [ ] AppManifest
- [ ] DataSource
- [ ] Query
- [ ] AppAction
- [ ] AppVariable
- [ ] Page
- [ ] Benchmark
- [ ] BenchmarkRun
- [ ] FlowArtifact
- [ ] FlowStep
- [ ] FlowMiddleware
- [ ] MediaUnit
- [ ] PixelGrid
- [ ] AudioBuffer
- [ ] VideoStream
- [ ] VectorPath
- [ ] GlyphOutline
- [ ] MeshGeometry
- [ ] Color
- [ ] Palette
- [ ] Codec
- [ ] Container
- [ ] MediaMetadata
- [ ] RenderPipeline

### B5. Typed side-tables
- [ ] `entry_benchmarks` schema (run_id, platform, compiler_hash, workload_key, timing moments, counters)
- [ ] `flow_steps` schema (artifact_id, step_index, entry_id, start_ns, end_ns, input_hash, output_hash)
- [ ] Indexes + FKs declared
- [ ] Populated from real ingests

### B6. Dream-tree + MECE
- [ ] MECE-objectives validator firing on agent demos
- [ ] Feature-stack word IDs
- [ ] DreamReport score ≥95 gate active

### B7. Self-documenting Skills seeded in dict
- [ ] author_nom_app
- [ ] compose_from_dict
- [ ] debug_nom_closure
- [ ] extend_nom_compiler
- [ ] ingest_new_ecosystem
- [ ] use_ai_loop
- [ ] compose_brutalist_webpage
- [ ] compose_generative_art_piece
- [ ] compose_lofi_audio_loop

### B8. Corpus breadth
- [x] 84 translations baseline
- [ ] 100+ translations
- [ ] 100 paradigm families (71 today)
- [x] 20+ paradigm families (maintain)

### B9. Authoring CLI complete
- [ ] `nom author start`
- [ ] `nom author check`
- [ ] `nom corpus ingest pypi`
- [ ] `nom corpus ingest github`
- [ ] `nom corpus ingest repo`
- [ ] `nom corpus status/pause/resume/report`
- [ ] `nom bench run/compare/regress/curate`
- [ ] `nom flow record/show/diff/middleware`
- [ ] `nom media import/import-dir/render/transcode/diff/similar`
- [ ] `nom ux seed <path>`
- [ ] `nom app new/import/build/build-report/explain-selection`

### B10. Bootstrap proof (shared with A7)
- [ ] s2==s3 byte-identical attested in dict

---

## AXIS C — nom-canvas ↔ compiler integration → 100%

### C1. Spec §9 wire table complete
- [x] Type char → stage1_tokenize → highlight (Wave C)
- [x] Hover word → handle_hover → tooltip (Wave O)
- [x] Pause 500ms → run_pipeline → diagnostics
- [x] Drag wire → can_wire (Wave Q CW1)
- [ ] Click Run → compile → LLVM → execute output
- [x] Command-bar → classify_with_react
- [x] Deep Think → scored hypothesis chain (Wave R NI1)
- [ ] Open compose → dream_report → score + proposals

### C2. DB-driven palette actual wiring
- [x] `NodePalette::load_from_dict(&SqliteDictReader)` live SELECT
- [x] `LibraryPanel::load_from_dict()` same
- [ ] End-to-end test: real nomdict.db → palette renders
- [x] AC2 close: remove slice/static palette as production path; require live `grammar.kinds` source
- [x] AC3 close: library panel reads the same live grammar source as palette

### C3. Feature gate flip
- [ ] `compiler` feature = default ON in nom-compiler-bridge
- [ ] Default build links nom-compiler
- [ ] Bridge tests run without `--features compiler` flag
- [ ] CI matrix includes default+compiler

### C4. Full LSP stream visually verified
- [ ] Hover tooltip renders on canvas
- [ ] Completion popup visible with arrow-key navigation
- [ ] Diagnostic red-squiggle underline renders
- [ ] Go-to-definition navigates
- [ ] Rename-refactor preview works

### C5. Backend wiring beyond spec-and-stub
- [ ] Video backend: GPU scene → frame capture → FFmpeg parallel encode (Remotion)
- [ ] Audio backend: rodio/symphonia real encode
- [ ] Data-extract: opendataloader XY-Cut++ 0.015s/page
- [ ] Image backend: Open-Higgsfield model dispatch
- [ ] Storyboard: ArcReel 5-phase orchestration
- [ ] Native_screen: platform-specific codegen (AC10 added validation/error artifacts; capture still open)
- [ ] Mobile_screen: iOS/Android target (AC10 added validation/error artifacts; target integration still open)
- [ ] App_bundle: Cargo + wgpu signed bundle

### C6. RAG real retrievers
- [x] Graph RAG BFS + confidence weights (Wave Q)
- [ ] Vector retriever using `nom-search` BM25 + ANN
- [ ] LlamaIndex pipeline composition
- [ ] Refly skill-engine integration

### C7. Deep-think full round-trip
- [x] classify_with_react + react_chain_interruptible (Wave R)
- [x] DeepThinkPanel ingest_events/consume_stream (Wave P DEEP1)
- [ ] User interrupt button wired to InterruptSignal
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
- [ ] dify — full typed-Node + event-generator (currently partial)
- [x] n8n (4 AST sanitizers + credential store)
- [x] LlamaIndex (RRF + cosine + BFS)
- [ ] Haystack — full pipeline composition
- [ ] ToolJet — full 55-widget registry (16/55 today)
- [x] yara-x (Sealed + InternalRule)
- [x] typst (comemo Tracked/Constraint/hash128)
- [x] WrenAI (SemanticModel + MDL)
- [x] 9router (3-tier fallback + credential + compose_with_fallback)
- [ ] graphify (chart types + Redux slice) — NOT staged
- [ ] Refly — full skill-engine + LangGraph + BullMQ
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
- [ ] `cargo clippy --workspace -- -D warnings` clean
- [x] Targeted strict clippy clean for `nom-compose` + `nom-memoize`: `cargo clippy -p nom-compose -p nom-memoize --all-targets -- -D warnings`
- [ ] AD3 broad clippy: remove `nom-dict` deprecated compatibility warnings and remaining workspace lints
- [ ] `cargo fmt --check` clean
- [ ] GitHub Actions CI green on PR
- [ ] Release pipeline produces signed binaries
- [ ] Installer: MSI (Windows), AppImage (Linux), DMG (macOS)

### D5. Documentation
- [ ] README with install + quickstart
- [ ] User manual (`docs/user-guide/`)
- [ ] API reference (`cargo doc --no-deps`)
- [ ] Architecture deep-dive (spec extension)
- [ ] Video walkthrough / screencast
- [ ] `CONTRIBUTING.md` + `CODE_OF_CONDUCT.md`
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
- [ ] AF-HEADER: 36px top bar — workspace name · mode switcher (Code·Doc·Canvas·Graph·Draw·Compose) · search; 1px bottom border only
- [ ] AF-STATUS: 24px bottom bar — branch+lsp left, errors/position right; 1px top border only
- [ ] AF-TITLEBAR: platform-native frame integration (macOS traffic-light / Windows drag-region)

**Left sidebar:**
- [ ] AF-LEFT-ICONS: 48px icon rail, Lucide 20px icons, `text_secondary` tint, active = `accent` fill
- [ ] AF-LEFT-PANEL: 248px expandable panel; collapsible sections (Explorer · Outline · Library · RAG Context); frosted glass hover overlay
- [ ] AF-LEFT-PALETTE: DB-driven node palette (live `SELECT` from `grammar.kinds`); search box + category groups

**Center workspace:**
- [ ] AF-CENTER-EDITOR: Code mode — rope buffer, 40px gutter, compiler-bridge syntax highlighting, serif font for prose blocks
- [ ] AF-CENTER-CANVAS: Canvas mode — infinite viewport, frosted-glass AFFiNE cards (shadow-md), bezier edges with confidence-color tint
- [ ] AF-CENTER-TABS: 32px tab strip — 2px accent bottom for active tab, close icon on hover only

**Right sidebar (Rowboat pattern):**
- [ ] AF-RIGHT-CHAT: 320px panel — scrollable history cards top, sticky textarea + send + tool toggles bottom
- [ ] AF-RIGHT-DEEP: deep-think card stack — each card has 1px border-left colored by hypothesis confidence
- [ ] AF-RIGHT-PROPS: selected block/node metadata panel — NomtuRef word+kind+id, inline edit fields

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

- [ ] AH-CTX: `ComposeContext` / `ComposeResult` / `ComposeTier` in `nom-compose/src/context.rs`
- [ ] AH-DICTW: `DictWriter` write side: `insert_partial_entry()` + `promote_to_complete()` in `nom-compiler-bridge`
- [ ] AH-CACHE: `GlueCache` in `SharedState` + 60s promotion ticker
- [ ] AH-DISPATCH: `UnifiedDispatcher`: `ProviderRouter` <-> `BackendRegistry` bridge with credential injection
- [ ] AH-INTENT: `IntentResolver`: lexical scan + BM25 + `classify_with_react()`
- [ ] AH-GLUE: `AiGlueOrchestrator` + `GlueBlueprint` + `ReActLlmFn` trait (4 adapters: Stub/NomCli/Mcp/RealLlm)
- [ ] AH-HYBRID: `HybridResolver` orchestrating Tier1->Tier2->Tier3
- [ ] AH-ORCH: `ComposeOrchestrator` multi-kind parallel pipeline
- [ ] AH-DB-KINDS: 14 initial `grammar.kinds` seed rows (video/picture/audio/presentation/web_app/mobile_app/native_app/document/data_extract/data_query/workflow/ad_creative/3d_mesh/storyboard)
- [ ] AH-PURPOSE: `intended to <purpose>` clause required in every AI .nomx sentence; absent = orchestrator retries
- [ ] AH-EXPLICIT: user Accept in Review card -> `DictWriter::insert_partial_entry()` immediately (no usage count threshold)
- [ ] AH-UI: Intent Preview + AI Review cards in `nom-panels/src/right/`

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
