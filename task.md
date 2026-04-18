# Nom — Task Execution Checklist

**Date:** 2026-04-18 | **HEAD:** `7a79e88` | **Tests listed:** 2439 default; `--all-features` PASS | **Workspace:** dirty (uncommitted code edits present)

## Wave AC Audit (2026-04-18) — DB-driven + UI/UX reliability gate

This section is the current reliability source until the code findings below are fixed and the docs are re-audited.

| Area | Verdict | Evidence |
|---|---|---|
| `NomtuRef` non-optional on block/node models | PASS | `nom-canvas/crates/nom-blocks/src/block_model.rs:46`, `nom-canvas/crates/nom-blocks/src/graph_node.rs:12` |
| Connector validation at every creation | PASS | Only `Connector::new_with_validation()` remains; no `Connector::new()` / `new_stub()` call sites found |
| Node palette live DB source | PASS | `nom-canvas/crates/nom-panels/src/left/node_palette.rs:26` uses `load_from_dict(&dyn DictReader)` over `list_kinds()` |
| Library panel live DB source | PASS | `nom-canvas/crates/nom-panels/src/left/library.rs:28` uses `load_from_dict(&dyn DictReader)` over `list_kinds()` |
| DictReader isolation | PASS | `nom-canvas/crates/nom-blocks/src/dict_reader.rs:22`; SQLite opens isolated to `nom-canvas/crates/nom-compiler-bridge/src/sqlite_dict.rs:23-27` |
| Cross-workspace compiler deps | PASS | `nom-canvas/crates/nom-compiler-bridge/Cargo.toml:6-23` feature-gates `../../../nom-compiler/crates/*` path deps |
| UI/UX runtime verification | PASS | `nom-panels` runtime scene test covers palette/library/file/properties/chat/deep-think surfaces; `nom-gpui` tests reduced-motion delta behavior |
| Source-repo parity docs | DRIFT | Several reference sources are not locally matched by path (`Remotion`, `Open-Higgsfield`, `opendataloader`), so parity claims require re-verification before `[x]` |

### Open Findings From Wave AC Audit

- [x] **AC1 CRITICAL — Connector constructors bypass grammar validation.** Closed by removing `Connector::new()` / `Connector::new_stub()` and keeping construction on `Connector::new_with_validation()`; verified by `rg` and `cargo test --workspace --all-features -q`.
- [x] **AC2 CRITICAL — Node palette is not a live `grammar.kinds` SELECT.** Closed by `NodePalette::load_from_dict(&dyn DictReader)` over `DictReader::list_kinds()`; verified by `cargo test --workspace --all-features -q`.
- [x] **AC3 HIGH — Library panel repeats the same static/slice kind source.** Closed by `LibraryPanel::load_from_dict(&dyn DictReader)` over `DictReader::list_kinds()`; verified by `cargo test --workspace --all-features -q`.
- [x] **AC4 HIGH — UI/UX needs real runtime verification, not token-only tests.** Closed by runtime scene coverage across palette/library/file/properties/chat/deep-think panels and reduced-motion animation tests; verified by `cargo test --workspace --all-features -q`.
- [x] **AC5 MEDIUM — Some UI surfaces still carry optional string entity IDs.** Closed by typed `PanelEntityRef` metadata boundary in `nom-panels`; no `entity_id: Option<String>` or `Option<NomtuRef>` remains in panel code.
- [ ] **AC6 MEDIUM — Document reliability is not 100%.** `implementation_plan.md` and `nom_state_machine_report.md` still carry stale HEAD/test snapshots; this file now records the current audit truth.
- [x] **AC7 HIGH — `--all-features` compiler bridge is not green.** Closed by cached grammar fallback fixes in completion/score/interactive paths; `cargo test --workspace --all-features -q` passes.

### Source-Repo Parity Pass (2026-04-18)

- [x] **AC8 — Dedicated crate-by-crate parity pass run after AC1/AC2 closure.** Scope covered all 15 `nom-canvas` crates with reference availability and pattern grep evidence.
- [x] **AC9 — Actual screenshot-style visual QA artifact lane.** `cargo run -p nom-panels --example visual_qa` writes `.omx/visual/nom-panels-runtime.ppm` and `.omx/visual/nom-panels-runtime.json`; latest report: 1200x720, 84 quads, 1 frosted rect, 200528 nonblank pixels, PASS.
- [x] **AC10 — nom-compose backend-depth tranche started.** Export backend now has a tested no-dependency base64 encoder; native/mobile screen backends reject invalid dimensions/platform/format and emit failure events.
- [x] **AC11 — warning cleanup before stricter clippy.** Removed `nom-compose`/`nom-memoize` warning sources and made `cargo clippy -p nom-compose -p nom-memoize --all-targets -- -D warnings` pass, including dependency lints it exposed in `nom-graph`/`nom-blocks`.
- [x] **AD1 — Native `nom-gpui::window::run_application` opens a winit window.** Native path now builds a real `winit` window/event loop; wasm keeps synthetic single-frame behavior. Verified with `cargo check -p nom-gpui` and all-features tests.
- [x] **AD2 — Real OS screenshot QA.** `nom-gpui` now has `window_first_paint` harness that opens a native window and exits deterministically; Windows screenshot tooling captured `.omx/visual/nom-gpui-window-first-paint.png` (542749 bytes).
- [ ] **AD3 — Broad workspace clippy.** Deprecated `nom-dict` compatibility warnings are contained, and targeted `nom-compose`/`nom-memoize` clippy passes; broad workspace clippy now reaches broader test lint debt in `nom-theme`, `nom-gpui`, `nom-collab`, and other crates.

| Crate | Parity verdict | Evidence / gap |
|---|---|---|
| nom-blocks | PASS | `NomtuRef`, `DictReader::list_kinds`, and grammar-backed connector validation are wired. |
| nom-canvas-core | PASS | Bezier hit-testing and element painting evidence present. |
| nom-cli | PASS | CLI command coverage present; no source-repo blocker found in this pass. |
| nom-collab | PASS | RGA/tombstone pattern present (`RgaPos`, `tombstoned`). |
| nom-compiler-bridge | PASS | Feature-gated compiler deps and all-features tests pass. |
| nom-compose | DRIFT | Backend-depth tranche started (export/native/mobile improved); several backend families remain partial. |
| nom-editor | PASS | Rope/editor maps and placeholder folds are implementation details, not parity blockers. |
| nom-gpui | DRIFT | Zed-like scene/renderer/atlas patterns present; real winit loop/shader bodies still stub-labelled. |
| nom-graph | PASS | Kahn DAG, sandbox, cache, and RRF evidence present. |
| nom-intent | PASS | ReAct/deep-think coverage present. |
| nom-lint | PASS | yara-x sealed/InternalRule pattern present. |
| nom-memoize | PASS | typst/comemo-style tracked/cache primitives present. |
| nom-panels | PASS | DB-backed palette/library fixed; runtime scene test and `.omx/visual/nom-panels-runtime.ppm` cover primary panel surfaces and typed entity metadata boundary. |
| nom-telemetry | PASS | W3C traceparent parse/format coverage present. |
| nom-theme | DRIFT | Token/math coverage present; real font asset loading remains open. |

Reference availability: Zed, AFFiNE, rowboat, ComfyUI, dify, n8n, LlamaIndex, Haystack, ToolJet, yara-x, typst, WrenAI, 9router, graphify, Refly, ArcReel, and waoowaoo paths were found under `C:\Users\trngh\Documents\APP`; Remotion, Open-Higgsfield, and opendataloader local paths remain missing.

**Master roadmap to 100%:** [`ROADMAP_TO_100.md`](ROADMAP_TO_100.md) — every remaining `[ ]` across 4 axes (compiler/language/integration/platform).

## Wave V (2026-04-18) — GPU library wiring + test coverage push ✅ (29cb808, 824 tests)
- [x] **ETAGERE-WIRE** — atlas.rs uses BucketedAtlasAllocator from etagere (not manual shelf)
- [x] **BYTEMUCK-WIRE** — QuadInstance/SpriteInstance/GlobalUniforms derive Pod+Zeroable
- [x] **CLI-EXPAND** — nom-cli: 5 new commands (version/help/run/format/rag--top-k) + 12 tests → 30
- [x] **TEST-CANVAS-CORE** — nom-canvas-core: already at 98 tests (no change needed)
- [x] **TEST-THEME** — nom-theme: +13 tests → 33
- [x] **TEST-EDITOR** — nom-editor: +13 tests → 80
- [x] **TEST-GRAPH** — nom-graph: +12 tests → 88
- [x] **TEST-COMPOSE** — nom-compose: +13 tests → 127
- [x] **TEST-INTENT-COLLAB-MEMO** — +16 tests (intent 20, collab 23, memoize 27)
- [x] **LINEAR-RGBA** — LinearRgba + ColorSpace enum added to nom-gpui renderer.rs

## Wave X (2026-04-18) — COMPLETE ✅ (d63bc35, 1266 tests)
- [x] nom-intent: 20→40 tests
- [x] nom-collab: 23→44 tests
- [x] nom-memoize: 27→56 tests
- [x] nom-theme: 33→59 tests
- [x] nom-gpui types.rs: +25 → 186 tests
- [x] nom-editor display/wrap/highlight: +25 → 105 tests
- [x] nom-compose cancel/credential/backends: +27 → 172 tests
- [x] nom-canvas-core spatial/snapping: +24 → 122 tests
- [x] nom-compiler-bridge adapters/sqlite: +25 → 95 tests

## Wave Y (2026-04-18) — COMPLETE ✅ (2b73744, 1462 tests)
- [x] nom-cli: 30→55 tests
- [x] nom-collab: 44→66 tests
- [x] nom-telemetry: 44→65 tests
- [x] nom-lint: 45→70 tests
- [x] nom-intent: 40→60 tests
- [x] nom-blocks: 70→93 tests
- [x] nom-memoize: 56→75 tests
- [x] nom-panels: 81→102 tests
- [x] nom-graph: 105→125 tests

## Wave Z (2026-04-18) — COMPLETE ✅ (59f3a30, 1679 tests)
- [x] nom-theme: 59→85 tests
- [x] nom-collab: 66→90 tests
- [x] nom-telemetry: 65→87 tests
- [x] nom-compiler-bridge: 95→121 tests
- [x] nom-compose: 172→197 tests
- [x] nom-canvas-core: 122→145 tests
- [x] nom-panels: 102→125 tests
- [x] nom-editor: 105→128 tests
- [x] nom-gpui: 186→211 tests

## Wave AA (2026-04-18) — COMPLETE ✅ (de12f4d, 2219 tests)
- [x] nom-cli: 55→105 tests (+50)
- [x] nom-intent: 60→115 tests (+55)
- [x] nom-lint: 70→125 tests (+55)
- [x] nom-memoize: 75→118 tests (+43)
- [x] nom-theme: 85→163 tests (+78)
- [x] nom-collab: 90→129 tests (+39)
- [x] nom-blocks: 93→150 tests (+57)
- [x] nom-telemetry: 87→145 tests (+58)
- [x] nom-graph: 125→175 tests (+50)
- [x] nom-compose: 197→252 tests (+55)

## Wave AB (2026-04-18) — in progress
- [ ] nom-canvas-core: 145→200+ tests (viewport edge cases, spatial index, rubber band)
- [ ] nom-compiler-bridge: 121→175+ tests (SharedState race, LSP completions, adapters)
- [ ] nom-editor: 128→185+ tests (multi-cursor, tab map, hints, find/replace regex)
- [ ] nom-gpui: 211→265+ tests (layout engine depth, styled chaining, atlas reuse)
- [ ] nom-panels: 125→180+ tests (deep-think card stream, palette search, properties)
- [ ] nom-collab: 129→175+ tests (3-way merge, op log replay, YJS-style convergence)
- [ ] nom-theme: 163→200+ tests (WCAG contrast ratios, animation spring physics)
- [ ] nom-graph: 175→220+ tests (sandbox AST sanitizers, execution engine cancel)
- [ ] nom-compose: 252→295+ tests (semantic MDL, credential store, provider router depth)
- [ ] nom-blocks: 150→195+ tests (DictReader integration, can_wire grammar backend)

## Wave W (2026-04-18) — COMPLETE (fc20fc8, 1044 tests)
- [x] nom-lint: +28 → 45 tests
- [x] nom-telemetry: +26 → 44 tests
- [x] nom-blocks: +32 → 70 tests
- [x] nom-compiler-bridge: +18 → 70 tests
- [x] nom-panels: +18 → 81 tests
- [x] nom-gpui platform/focus/animation/event: +51 → 161 tests
- [x] nom-graph dag/sandbox/graph_rag: +17 → 105 tests
- [x] nom-compose semantic/store/plan/task_queue: +18 → 145 tests
- [x] nom-canvas-core integration test suite (tests/integration.rs): +12
- [x] nom-gpui shaders.rs WGSL stubs: +8 shader tests; scene: +7

## Wave P (2026-04-18 Iter 45) — E2+11 fixes: paint bodies + all HIGHs + MEDIUMs
- [x] E2 CRITICAL: GraphNodeElement::paint() + WireElement::paint() push real Quads (5 body+port, 6 wire segments)
- [x] TQ1: task_queue complete() guards state==Running before transition
- [x] CK1: LruCache touch() on get() via Mutex interior mutability
- [x] DP1: Backend trait + BackendRegistry dispatch + NoopBackend stub
- [x] F1: find_replace.rs use_regex → real regex::Regex + find_iter match ranges
- [x] NR1: duplicate NomtuRef removed from graph_mode.rs; re-export canonical nom-blocks::NomtuRef
- [x] NI1: nom-intent expanded 98→240 LOC: ScoredHypothesis + InterruptSignal + rank_hypotheses + react_chain_interruptible
- [x] DEEP1: DeepThinkPanel ingest_events/consume_stream wired; paint_scene emits per-card Quads
- [x] FG1: FrostedRect primitive in nom-gpui::Scene; dock pushes frosted overlay (FROSTED_* tokens)
- [x] FR1: focus ring is 2px border-only Quad via focus_ring_quad() (3 sites: dock, file_tree, quick_search)
- [x] PAL1: NodePalette + PaletteEntry added to nom-panels/left — DB-driven load/search/paint
- [x] MEDIUMs: RRF_K=60.0 const; InternalRule 3rd trait; HierarchicalCache::len sums L1+L2; Panel trait 7 methods

**Wave S committed c4d6252 (686 tests). Wave T committed 0b0d48e (717 tests). All spec-mandated modules implemented.**
**Wave U: final test coverage push + element.rs WindowContext cleanup.**

## Wave N (2026-04-18 Iter 43) — router infra + sandbox + SHA-256 + semantic MDL
- [x] nom-compose vendor_trait.rs: MediaVendor + CostEstimate + StubVendor
- [x] nom-compose provider_router.rs: FallbackLevel 3-tier + retry_delay_ms (1000×2^level, max 120s)
- [x] nom-compose credential_store.rs: kind-keyed secret storage
- [x] nom-graph sandbox.rs: 4 AST sanitizers (DepthLimit + AllowedFunctions + NoSideEffects + TypeCoherence) + eval_expr
- [x] nom-compose store.rs: ContentHash now uses SHA-256 via sha2 (spec §14 compliance)
- [x] nom-compose semantic.rs: WrenAI MDL semantic layer (SemanticModel + SemanticRegistry + SQL generation)

**Sibling docs:** `implementation_plan.md` · `nom_state_machine_report.md` · `docs/superpowers/specs/2026-04-17-nomcanvas-gpui-design.md` · `INIT.md`

## Current Status

| Wave | Done | Evidence |
|------|------|---------|
| [x] 0 Bootstrap | ✅ | 14 crates, workspace clean |
| [x] A GPUI substrate | ✅ | nom-gpui 2938 + nom-canvas-core 2183 + nom-theme 1211 LOC |
| [x] B Editor + Blocks | ✅ | nom-blocks 1313 + nom-editor 1269 LOC |
| [x] C Compiler bridge | ✅ | 3-tier (ui/interactive/background), `--features compiler` clean |
| [x] D Shell | ✅ | nom-panels 1522 LOC, 9 files paint_scene → Quad |
| [x] E Compose backends | ✅ | 16 backends + dispatch + plan + task_queue |
| [x] F RAG + Deep-think | ✅ | classify_with_react + RRF 1/(60+rank) |
| [x] G Stubs populated | ✅ | nom-lint + collab + telemetry |
| [x] K (4 CRITICALs) | ✅ | dc6a025 — U1/W1/COL1/INT1 closed |
| [x] L (MEDIUM) | ✅ | d139644 — deep_think + W3C + RRF |
| [x] M (Infra) | ⚠️ DRIFT | ef9fc84 — 4 claims DRIFT, 1 HIGH bug (TQ1) |
| [x] N Infra+Vendor | ✅ | d6219b1 — vendor/router/cred/sandbox/SHA-256/semantic (523 tests) |
| [x] O Infra+LSP | ✅ | e61a93c — cancel/cache/LSP/sandbox/web_screen (537 tests) |
| [x] P Bug fixes | ✅ | 15a8366 — E2+11 fixes+MEDIUMs (558 tests) |
| [x] Q Quality | ✅ | f0ca908 — SB1+SC1+CW1+DOC1+CB1+E1+rag-confidence (581 tests) |
| [x] R Coverage | ✅ | 0949124 — NI1+SipHash13+coverage+57 (638 tests) |
| [x] S Spec align | ✅ | c4d6252 — 5 panels+10 backends+FrostedRect+hints+renderer (686 tests) |
| [x] T Cleanup | ✅ | 0b0d48e — scenario_workflow+renderer+integration+31 (717 tests) |
| [x] U Coverage | ✅ | ef5e058 — intent/telemetry/collab+16 (733 tests) |
| [x] V GPU wiring | ✅ | bytemuck+etagere+LinearRgba+CLI, 824 tests |
| [x] W Coverage | ✅ | fc20fc8 — lint+telemetry+blocks+gpui+graph, 1044 tests |
| [x] X Deep coverage | ✅ | d63bc35 — intent/collab/memoize/theme/types, 1266 tests |
| [x] Y Coverage | ✅ | 2b73744 — cli/collab/telemetry/lint/intent/blocks, 1462 tests |
| [x] Z Coverage | ✅ | 59f3a30 — theme/collab/telemetry/bridge/compose/panels/editor/gpui, 1679 tests |
| [x] AA Coverage | ✅ | de12f4d — +540 across 10 crates, 2219 tests |
| [ ] AB Coverage | ⏳ | canvas-core/bridge/editor/gpui/panels/collab/theme/graph/compose/blocks |

### Integrity Grep

| Check | Count | Expected |
|---|---|---|
| `use nom_gpui::scene` | 11 | ≥1 |
| `nom_intent` in deep_think.rs | 3 | ≥1 |
| `RgaPos`/`tombstoned` in nom-collab | 28 | ≥1 |
| `RenderPrimitive` custom enum | 0 | 0 |

## Open Missions (Wave T — COMPLETE)

All spec-mandated modules implemented. Wave U: final test coverage push + element.rs WindowContext cleanup.

### Wave T — Completed

- [x] **BE-SCENARIO-WORKFLOW** — scenario_workflow backend domain model + tests (0b0d48e)
- [x] **RENDERER-DRAW** — Renderer draw methods real FrameStats draw-call counting (0b0d48e)
- [x] **INTEG-PANELS-GPUI** — integration tests: panels↔gpui↔compose pipeline cross-crate (0b0d48e)
- [x] **INTEG-BLOCKS-CANVAS** — integration tests: blocks↔canvas-core↔editor cross-crate (0b0d48e)
- [x] **NOM-GRAPH-TESTS** — nom-graph test suite expansion: 68→75+ tests (0b0d48e)
- [x] **COMPILER-BRIDGE-TESTS** — nom-compiler-bridge test suite expansion: 44→52+ tests (0b0d48e)

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

## Wave Mission Archive

Detail checklists collapsed — retrieval via git log of canonical commits.

### [x] Wave 0 — Bootstrap
- [x] Cargo workspace + 14 crate stubs + rust-toolchain
- [x] `unsafe_code = deny` workspace lint
- [x] cross-workspace path deps to `../../../nom-compiler/crates/*` (feature-gated)

### [x] Wave A — GPUI Substrate (commit `8c7d32e`)
- [x] nom-gpui scene graph (Quad/Sprite/Path/Shadow/Underline) + 8 wgpu pipelines
- [x] cosmic-text + etagere glyph atlas
- [x] Element trait + Styled builder + winit + taffy + animation
- [x] nom-canvas-core viewport + hit-test + rubber-band + R-tree
- [x] nom-theme 73 AFFiNE tokens + Inter + Source Code Pro + 42 Lucide icons

### [x] Wave B — Editor + Blocks (commit `8c7d32e`)
- [x] nom-editor rope + multi-cursor + display/wrap/tab/line maps
- [x] Highlighter::color_runs consumer + LspProvider trait
- [x] nom-blocks with `NomtuRef` non-optional on every block
- [x] AFFiNE block types (heading/para/list/quote/divider/callout/code/db/linked)
- [x] Graph node ports derived from `clause_shapes` (DB-driven)
- [x] DictReader trait injection (no direct `Connection::open` in nom-blocks)
- [x] can_wire placeholder

### [x] Wave C — Compiler Bridge KEYSTONE (commit `fb66e01`, 17/17)
- [x] shared.rs Arc<RwLock<SharedState>> + dict_pool + grammar + LRU
- [x] ui_tier.rs sync: lookup_nomtu, score_atom, can_wire, grammar_keywords, search_bm25
- [x] interactive_tier.rs tokio mpsc: tokenize, highlight_spans, complete_prefix, hover
- [x] background_tier.rs crossbeam: compile, plan_flow, verify, deep_think
- [x] adapters: highlight (stage1 → color_runs) · lsp · completion · score
- [x] First wire: `.nomx` → stage1_tokenize → Highlighter live

### [x] Wave D — Shell (20/20 tests)
- [x] dock.rs (DockPosition/Dock/Panel) · pane.rs (PaneGroup split) · shell.rs
- [x] Left dock: CollapsibleSection + QuickSearchInput + ResizePanel + dict tree
- [x] Right dock: ChatSidebar + tool cards + deep-think stream + multi-agent
- [x] Bottom dock (terminal + diagnostics)
- [x] WindowPool multi-window + Cmd+K palette

### [x] Wave E — 16 Compose Backends (commit `a1ba5a1`, 26/26)
- [x] E-1 document (typst-memoize) · E-2 video (Remotion) · E-3 data_extract (XY-Cut++)
- [x] E-4 web_screen (ToolJet) · E-5 workflow (n8n + AST sandbox)
- [x] E-6 data_query (WrenAI) · E-7 image · E-8 storyboard · E-9 audio
- [x] E-10..16 data_frame/native/mobile/presentation/app_bundle/ad_creative/mesh
- [x] ArtifactStore + ProgressSink + InterruptFlag + MediaVendor dispatch

### [x] Wave F — Graph RAG + Deep Think (commit `be3b9a8`)
- [x] graph_rag.rs (retriever + BFS + cosine + RRF 1/(60+rank))
- [x] graph_mode.rs (force-directed + confidence edges + animate_to_layout)
- [x] deep_think.rs (stream + step chain + token budget + real ReAct)

### [x] Wave G — Stubs populated (commit `546e02d`)
- [x] nom-lint sealed trait (yara-x pattern)
- [x] nom-collab RGA CRDT (RgaPos anchor + tombstone + convergence)
- [x] nom-telemetry W3C traceparent

### [x] Wave K — 4 CRITICALs (commit `dc6a025`, 457 tests)
- [x] U1 — nom-panels paint_scene uses nom_gpui::scene::Quad + nom_theme::tokens (10 files)
- [x] W1 — deep_think imports `use nom_intent::{classify_with_react, react_chain}`
- [x] COL1 — RGA CRDT with RgaPos::{Head, After(OpId)} + tombstoned: bool
- [x] INT1 — 11+ cross-crate `use nom_gpui::scene` imports

### [x] Wave L — MEDIUMs (commit `d139644`, 504 tests)
- [x] with_deep_think wired end-to-end · W3C traceparent format · RRF 1/(60+rank)
- [x] impl Element for GraphNodeElement/WireElement (trait only — bodies empty → E2)

### [x] Wave M — Infra (commit `ef9fc84`, 498 tests)
- [x] nom-lint sealed (2-trait; MEDIUM: add 3rd InternalRule)
- [x] compiler-bridge 3-tier consolidated
- [x] compose dispatch + plan + task_queue (⚠️ TQ1 bug + DP1 drift)
- [x] graph 4-tier cache (⚠️ CK1 LruCache.get broken)

### [x] Wave N — Infra+Vendor (commit `d6219b1`, 523 tests)
- [x] nom-compose vendor_trait.rs: MediaVendor + CostEstimate + StubVendor
- [x] nom-compose provider_router.rs: FallbackLevel 3-tier + retry_delay_ms
- [x] nom-compose credential_store.rs: kind-keyed secret storage
- [x] nom-graph sandbox.rs: 4 AST sanitizers + eval_expr
- [x] nom-compose store.rs: SHA-256 ContentHash via sha2
- [x] nom-compose semantic.rs: WrenAI MDL SemanticModel + SemanticRegistry

### [x] Wave O — Infra+LSP (commit `e61a93c`, 537 tests)
- [x] nom-compiler-bridge: CompilerLspProvider (real completions/diagnostics from nom-compiler)
- [x] nom-compose backends: code_exec (n8n JsTaskRunner + sandbox wiring), web_screen (headless browser stub)
- [x] nom-graph: ExecutionEngine cancel/abort signal
- [x] nom-graph: HierarchicalCache L1+L2 promotion wiring in ExecutionEngine
- [x] Workspace-wide: 537 tests verified across all 14 canvas crates

### [x] Wave P — Bug fixes + MEDIUMs (commit `15a8366`, 558 tests)
- [x] E2 CRITICAL: GraphNodeElement::paint() + WireElement::paint() push real Quads (5 body+port, 6 wire segments)
- [x] TQ1: task_queue complete() guards state==Running before transition
- [x] CK1: LruCache touch() on get() via Mutex interior mutability
- [x] DP1: Backend trait + BackendRegistry dispatch + NoopBackend stub
- [x] F1: find_replace.rs use_regex → real regex::Regex + find_iter match ranges
- [x] NR1: duplicate NomtuRef removed from graph_mode.rs; re-export canonical nom-blocks::NomtuRef
- [x] NI1: nom-intent expanded 98→240 LOC: ScoredHypothesis + InterruptSignal + rank_hypotheses + react_chain_interruptible
- [x] DEEP1: DeepThinkPanel ingest_events/consume_stream wired; paint_scene emits per-card Quads
- [x] FG1: FrostedRect primitive in nom-gpui::Scene; dock pushes frosted overlay (FROSTED_* tokens)
- [x] FR1: focus ring is 2px border-only Quad via focus_ring_quad() (3 sites: dock, file_tree, quick_search)
- [x] PAL1: NodePalette + PaletteEntry added to nom-panels/left — DB-driven load/search/paint
- [x] MEDIUMs: RRF_K=60.0 const; InternalRule 3rd trait; HierarchicalCache::len sums L1+L2; Panel trait 7 methods

### [x] Wave Q — Quality (commit `f0ca908`, 581 tests)
- [x] SB1: nom-graph/src/sandbox.rs — added this_replace, prototype_block, dollar_validate sanitizers
- [x] SC1: score_atom in ui_tier.rs:167 — eliminated per-call UiTier/SharedState allocation
- [x] CW1: nom-blocks/src/connector.rs:62 can_wire — grammar-backed validation wired
- [x] DOC1: ui_tier.rs:40 docstring corrected to `<1ms` (spec §3 compliance)
- [x] CB1: compiler-bridge score.rs adapter dead-code stubs replaced with real implementation
- [x] E1: panels paint_scene divergence from impl Element documented; trait bindings added
- [x] rag-confidence: graph_rag edge-confidence weights (Refly-pattern per-edge scoring)

### [x] Wave R — Coverage (commit `0949124`, 638 tests)
- [x] NI1-REAPPLY: nom-intent ScoredHypothesis/InterruptSignal/rank_hypotheses re-landed (5→10 tests)
- [x] SH1: nom-memoize hash.rs FNV-1a replaced with SipHash13
- [x] COV-TELEMETRY: nom-telemetry coverage expanded to ≥15 tests
- [x] COV-COLLAB: nom-collab coverage expanded to ≥15 tests
- [x] COV-CLI: nom-cli coverage expanded to ≥15 tests
- [x] COV-LINT: nom-lint coverage expanded to ≥15 tests

### [x] Wave S — Spec align (commit `c4d6252`, 686 tests)
- [x] PANEL-CMD: command_palette.rs panel added to nom-panels
- [x] PANEL-TB: toolbar.rs panel added to nom-panels
- [x] PANEL-SB: statusbar.rs panel added to nom-panels
- [x] PANEL-PROPS: properties.rs panel added to nom-panels
- [x] PANEL-LIB: library.rs panel added to nom-panels
- [x] BE-MESH: mesh compose backend domain model + tests
- [x] BE-STORYBOARD: storyboard compose backend domain model + tests
- [x] BE-NATIVE-SCREEN: native_screen compose backend domain model + tests
- [x] BE-MOBILE-SCREEN: mobile_screen compose backend domain model + tests
- [x] BE-PRESENTATION: presentation compose backend domain model + tests
- [x] BE-APP-BUNDLE: app_bundle compose backend domain model + tests
- [x] BE-AD-CREATIVE: ad_creative compose backend domain model + tests
- [x] BE-DATA-EXTRACT: data_extract compose backend domain model + tests
- [x] BE-DATA-FRAME: data_frame compose backend domain model + tests
- [x] BE-DATA-QUERY: data_query compose backend domain model + tests
- [x] FROSTED-RENDERER: FrostedRect wired into Renderer::draw()
- [x] HINTS: nom-editor hints.rs inlay hints module
- [x] RENDERER-INFRA: Renderer FrameStats + WindowBuilder + LayoutRegistry improvements

## Compiler Parallel Track (nom-compiler — UNCHANGED as infra)

- [x] GAP-1c body_bytes · GAP-2 embeddings · GAP-3 corpus ingest
- [x] GAP-4 nom-intent 9router pattern · GAP-5 deep_think backing op
- [ ] Bootstrap fixpoint proof (Wave future)

## History

Iter log in `nom_state_machine_report.md`. Key pivots:
- Iter 30 HARD FREEZE — Executor added surface without closing blockers; lifted Iter 31
- Iter 36-41 — 9 consecutive iters U1 open; Iter 40 single-commit mandate issued
- Iter 42-43 — single-commit landed; Wave K closed 4 CRITICALs
- Iter 45 — Wave L; impl Element claim found SHAPE-ONLY
- Iter 46 — Wave M; 4 DRIFT + TQ1 correctness bug
- Iter 47 — whole-repo scan found E2 CRITICAL (paint body no-op)
- Iter 44 — Wave O closed: CompilerLspProvider + cancel + cache-promotion + sandbox wiring + web_screen (537 tests)
- Iter 45 — Wave P closed: E2 CRITICAL + 10 HIGH/MEDIUM + 1 CRITICAL + MEDIUMs (558 tests, commit 15a8366)
- Iter 46 — Wave Q closed: SB1+SC1+CW1+DOC1+CB1+E1+rag-confidence (581 tests, commit f0ca908)
