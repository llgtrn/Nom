# Nom — Roadmap to 100%

**Date:** 2026-04-18 | **Mandate:** reach 100% on all 4 axes. Every `[ ]` is a completable task.

## Current finalization snapshot

| Axis | Today | Target | Gap |
|---|---|---|---|
| A · nom-compiler | 44% | 100% | 56pp |
| B · Nom language | 34% | 100% | 66pp |
| C · nom-canvas ↔ compiler integration | 68% | 100% | 32pp |
| D · Overall platform | 56% | 100% | 44pp |

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
- [ ] `NodePalette::load_from_dict(&SqliteDictReader)` live SELECT
- [ ] `LibraryPanel::load_from_dict()` same
- [ ] End-to-end test: real nomdict.db → palette renders

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
- [ ] Native_screen: platform-specific codegen
- [ ] Mobile_screen: iOS/Android target
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
- [ ] Focus ring = 2px outline stroke (FR1 follow-through)
- [ ] All panels render real quads with token colors
- [ ] Bezier control points animate smoothly
- [ ] Spring animation at AFFiNE defaults (stiffness=400, damping=28) verified on-screen
- [ ] Color contrast ≥4.5:1 WCAG AA for all text-on-surface combos
- [ ] Motion timing 200ms/300ms verified
- [ ] All 73 AFFiNE tokens visually used
- [ ] Dark + light theme toggle

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
- [ ] `cargo test --workspace --all-features` all green
- [ ] `cargo clippy --workspace -- -D warnings` clean
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
- [x] NomtuRef non-optional on every canvas object (NR1 closed)
- [ ] Canvas = AFFiNE-for-RAG (visible RAG overlay — currently partial)
- [ ] Doc = Zed + Rowboat + AFFiNE (all three visible — currently partial)
- [x] Deep thinking = compiler op streamed right dock
- [x] GPUI fully Rust — one binary
- [x] Parallel subagents for multi-file work
- [x] gitnexus_impact before editing symbols

### D7. State hygiene
- [ ] Weekly `task.md` compaction ritual
- [ ] Weekly state-report trim
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
- Axis C: 10 waves (wires + backend depth)
- Axis D: 15 waves (UI polish + CI + docs)

**Critical path:** A7 (fixpoint proof) + C5 (real backends) + D3 (golden demos).
