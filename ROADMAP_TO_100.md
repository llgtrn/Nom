# Nom — Roadmap to 100%

**Date:** 2026-04-18 | **Mandate:** reach 100% on all 4 axes. Every `[ ]` is a completable task.
**Last updated:** Wave AH complete — HEAD `59d58c4`, 5196 tests. Wave AI planned: wgpu real draw calls + missing_docs + ~5600 target.

## Current finalization snapshot

| Axis | Today | Target | Gap | Tests |
|---|---|---|---|---|
| A · nom-compiler | 44% | 100% | 56pp | (upstream, unchanged) |
| B · Nom language | 34% | 100% | 66pp | (upstream, unchanged) |
| C · nom-canvas ↔ compiler integration | 95% | 100% | 5pp | 5196 tests; Wave AH complete |
| D · Overall platform | 81% | 100% | 19pp | 15/15 crates; Wave AI next |

**Per-crate test counts (Wave AH actuals → Wave AI targets):**
| Crate | Wave AH actual | Wave AI target |
|---|---|---|
| nom-blocks | 330 | 365 |
| nom-canvas-core | 375 | 410 |
| nom-cli | 226 | 255 |
| nom-collab | 340 | 375 |
| nom-compiler-bridge | 330 | 365 |
| nom-compose | 485 | 520 |
| nom-editor | 390 | 425 |
| nom-gpui | 530 | 570 |
| nom-graph | 405 | 440 |
| nom-intent | 260 | 290 |
| nom-lint | 280 | 310 |
| nom-memoize | 265 | 295 |
| nom-panels | 360 | 395 |
| nom-telemetry | 295 | 325 |
| nom-theme | 325 | 360 |
| **TOTAL** | **5196** | **~5600** |

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

### D9. Hybrid Composition System (Wave AH — spec 2026-04-18)
**Spec:** `docs/superpowers/specs/2026-04-18-hybrid-compose-design.md`
**Architecture:** Three-tier resolver — DB-driven (grammar.kinds Complete) → Provider-driven (registered MediaVendor) → AI-leading (AiGlueOrchestrator generates .nomx glue). Intent classification at front. Grammar promotion lifecycle at back (Transient → Partial → Complete).

**ComposeContext + UnifiedDispatcher:**
- [ ] `ComposeContext` / `ComposeResult` / `ComposeTier` envelope in `nom-compose/src/context.rs`
- [ ] `DictWriter::insert_partial_entry()` + `promote_to_complete()` in `nom-compiler-bridge/src/dict_writer.rs`
- [ ] `GlueCache` in `SharedState` + 60s promotion background ticker
- [ ] `UnifiedDispatcher` bridging `ProviderRouter` ↔ `BackendRegistry` with credential injection
- [ ] `ProviderRouter::route_with_context()` + `BackendRegistry::dispatch_with_context()`
- [ ] `MediaVendor::compose(input, credential, ctx)` signature update

**IntentResolver — kind detection before routing:**
- [ ] Lexical scan: `SELECT word FROM grammar.kinds` → exact token match → confidence 1.0
- [ ] BM25 + cosine over `grammar.kinds.description` → semantic ranking when no exact match
- [ ] `classify_with_react()` fires when top-2 candidates within 0.15 delta
- [ ] Multi-kind detection: return all candidates above 0.65 threshold
- [ ] Low-confidence (below 0.6): show disambiguation card, user picks from DB-driven kind list
- [ ] Training signal: user correction feeds back into BM25 index weights

**AiGlueOrchestrator + HybridResolver:**
- [ ] `AiGlueOrchestrator::synthesize()`: GraphRagRetriever + clause_shapes query + ReActLlmFn → .nomx GlueBlueprint
- [ ] `ReActLlmFn` trait + 4 adapters: Stub / NomCli (offline) / Mcp / RealLlm (optional credential)
- [ ] `HybridResolver`: Tier1 (DB Complete) → Tier2 (vendor) → Tier3 (AI glue)
- [ ] `ComposeOrchestrator`: multi-kind parallel pipeline via existing `TaskQueue`
- [ ] `glue_promotion_config` DB table: PROMOTE_AFTER count + confidence threshold as data rows
- [ ] 14 initial `grammar.kinds` seed rows: video/picture/audio/presentation/web_app/mobile_app/native_app/document/data_extract/data_query/workflow/ad_creative/3d_mesh/storyboard

**Grammar promotion lifecycle:**
- [ ] `intended to <purpose>` clause required in every AI `.nomx` sentence — orchestrator rejects + retries if absent; purpose text → `grammar.kinds.description`
- [ ] Explicit path: user Accept or Edit+Save in Review card → `DictWriter::insert_partial_entry()` immediately; no usage count; `NomtuRef` assigned at promotion
- [ ] Auto path (user never reviews): usage_count >= 3 AND confidence >= 0.7 → Partial (background ticker, 60s poll)
- [ ] Partial → Complete (used 10+ times AND compiler validation passes): `DictWriter::promote_to_complete()`
- [ ] `glue_promotion_config` DB table: thresholds as data rows (auto_promote_count, auto_promote_confidence, complete_use_count)
- [ ] On Complete: entity indistinguishable from human-authored in palette and canvas

**UI surfaces:**
- [ ] Intent Preview card (right dock): kind confidence bars + Compose/Change/All-3 actions
- [ ] AI Review card (right dock): Accept/Edit/Skip with inline .nomx editor
- [ ] Doc mode: `⚡` gutter badge for Partial AI-generated entities; removed on Complete
- [ ] Graph mode: amber frosted-glass tint + `⚡` badge; removed on Complete
- [ ] Status bar: `⚡ N AI entities pending review` counter

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

**Current velocity:** ~270 tests/wave (Waves V–AA average, accelerating). At this rate:
- nom-canvas test suite hits 3000 tests in ~3 more waves
- Wave AB targets ~2600 (+161 from current 2439 listed tests)
- Axis C functional completeness needs real backend wiring (C5) + golden demos (D3)

**Critical path:** A7 (fixpoint proof) + C5 (real backends) + D3 (golden demos).
