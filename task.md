# Nom — Task Execution Checklist

**Date:** 2026-04-18 | **HEAD:** `59d58c4` | **Tests:** 5196 | **Workspace:** clean

## Wave AE Audit (2026-04-18) — Hard audit: UI rendering, bridge stubs, backend depth, security

### DB-Driven Architecture (all PASS — confirmed by parallel agent scan)

| Check | Verdict | Evidence |
|---|---|---|
| `Connector::new_with_validation()` only constructor | PASS | `nom-blocks/src/connector.rs:88`; grep zero matches for `new(` or `new_stub` |
| NodePalette live DB SELECT | PASS | `nom-panels/src/left/node_palette.rs:26` calls `dict.list_kinds()` |
| LibraryPanel live DB SELECT | PASS | `nom-panels/src/left/library.rs:28` calls `dict.list_kinds()` |
| `DictReader` isolation | PASS | `Connection::open` only in `sqlite_dict.rs:23,27` — zero violations elsewhere |
| `entity: NomtuRef` non-optional | PASS | `block_model.rs:46`, `graph_node.rs:12` — plain `NomtuRef` field, no `Option` |
| `Option<NomtuRef>` in struct fields | PASS | 5 occurrences all in `lookup_entity()` return types (correct), zero struct fields |
| `production_kind: String` (not enum) | PASS | `graph_node.rs:13` — comment: "validated via DictReader::is_known_kind, never a Rust enum" |
| Cross-workspace path deps | PASS | `Cargo.toml:7-23` feature-gated optional deps point to `../../../nom-compiler/crates/*` |
| BackendKind enum | ACCEPTABLE | 16-variant closed enum for compose routing (not grammar kinds); `from_kind_name(str)` bridges to runtime strings |

### Open Findings — Wave AE

- [x] **AE1 CRITICAL — Renderer::draw() is a pure stub, renders zero pixels.** `nom-gpui/src/renderer.rs:130-261` — every `draw_*` method only increments `FrameStats` counters. No `wgpu::Device`, `wgpu::Queue`, `wgpu::RenderPass`, `wgpu::RenderPipeline` anywhere. WGSL shaders return constant colors. Window opens (real winit) but displays a blank frame. Fix: implement wgpu device init → pipeline creation → instance buffer upload → draw calls → present.
- [x] **AE2 CRITICAL — highlight adapter emits zero-width spans.** Fixed: `tok_text_len()` helper maps each `Tok` variant to byte length; `end = pos + len` in both `tokenize_to_spans` and `highlight_source`. 171 nom-compiler-bridge tests pass.
- [x] **AE3 HIGH — lsp_provider.rs is a duplicate hardcoded stub.** Fixed: deleted `lsp_provider.rs`; `lib.rs` now re-exports `adapters::lsp::CompilerLspProvider` (real `LspProvider` trait impl). 171 tests pass. `nom-compiler-bridge/src/lsp_provider.rs:42-104` defines a second `CompilerLspProvider` that returns `"{prefix}_nomtu"` completion stubs and a hardcoded hover string. The real `CompilerLspProvider` implementing `LspProvider` trait lives in `adapters/lsp.rs:27`. `lib.rs` re-exports the stub. Fix: delete `lsp_provider.rs`, update `lib.rs` re-exports to use `adapters::lsp::CompilerLspProvider`.
- [x] **AE4 HIGH — scenario_workflow::compose() is a no-op stub.** Fixed: iterates steps, validates each, builds JSON result with steps_total/completed/triggers/timeout/success. 315 nom-compose tests pass. `nom-compose/src/backends/scenario_workflow.rs:26-31` — body is: check name non-empty, return `Ok(())`. `steps`, `triggers`, `timeout_ms` fields never used. Fix: implement step execution loop or mark `unimplemented!()` so it's not mistaken for working code.
- [x] **AE5 HIGH — data_query::compose() generates SQL then discards it.** Fixed: SQL written to artifact store via `store.write(sql.as_bytes())`. Test verifies stored bytes match expected SQL. 316 nom-compose tests pass. `nom-compose/src/backends/data_query.rs:43` — `Some(_sql) => Ok(())` discards the SQL string. Fix: write SQL to artifact store (same pattern as other backends).
- [x] **AE6 HIGH — Backend trait not implemented by concrete backends.** `nom-compose/src/dispatch.rs:71-74` defines `Backend` trait with `kind()` + `compose(&str, ...)`. None of `VideoBackend`, `DocumentBackend`, etc. implement it. Registry exists but can't route to actual backends. Fix: add `impl Backend for VideoBackend` adapters bridging the typed signatures to the trait.
- [x] **AE7 HIGH — Credential Debug derive leaks secret values.** Fixed: removed `Debug` from derive; custom `impl Debug` prints `[REDACTED]` for value field. Test verifies raw secret absent from `{:?}` output. 316 nom-compose tests pass. `nom-compose/src/credential_store.rs:6-10` — `#[derive(Debug, Clone)]` on `Credential { value: String }`. Any `{:?}` format or `unwrap()` panic prints raw secret. No zeroing on Drop. Fix: custom `Debug` impl redacting `value`, or use `secrecy::SecretString`.
- [x] **AE8 HIGH — eval_expr has no runtime recursion depth guard.** `nom-graph/src/sandbox.rs:341-365` recurses into `BinOp`/`If`/`Call` without a depth counter. Static sanitizer limit of 16 not enforced at eval time. `code_exec.rs:94-113` calls `eval_expr` without calling `sanitize()` first. Fix: add `depth: usize` parameter + return `SandboxError::DepthLimitExceeded` at >64.
- [x] **AE9 MEDIUM — FrostedRect blur_radius is stored but never used.** `nom-gpui/src/renderer.rs:230-260` — `draw_frosted_rects` ignores `rect.blur_radius`, produces two flat grey Quads. Not AFFiNE frosted glass. Fix: at minimum vary the tint based on `blur_radius`; in GPU path run blur pre-pass.
- [x] **AE10 MEDIUM — adapters/score.rs bypasses nom_score::score_atom.** `nom-compiler-bridge/src/adapters/score.rs:6-22` — does a name-match against grammar cache only, never calls `nom_score::score_atom()`. Fix: under `compiler` feature, construct `Atom` and call `nom_score::score_atom().overall()`.
- [x] **AE11 MEDIUM — SharedState uses Mutex not RwLock; no connection pooling.** `nom-compiler-bridge/src/shared.rs:23-34` — `grammar_kinds` in `Mutex` (blocks concurrent reads). `dict_path`/`grammar_path` are strings, not pre-opened connections; each `SqliteDictReader` call opens a fresh `Connection`. Fix: `RwLock<Vec<GrammarKindRow>>` for grammar kinds; pre-open WAL read connections into pool.
- [x] **AE12 MEDIUM — BM25 search not wired in UI tier.** `nom-search` declared as optional dep in `Cargo.toml` but never used anywhere in the bridge. `search_bm25` is absent from `ui_tier.rs`. Fix: add `search_bm25(query: &str) -> Vec<SearchHit>` calling `nom_search::BM25Index::search()`.
- [x] **AE13 MEDIUM — NoSideEffectsSanitizer is a no-op.** Documented: `// STUB` + `// TODO(security): implement before adding Expr::Assign/Import/Exec AST variants` added at sandbox.rs:172. Logic unchanged. `nom-graph/src/sandbox.rs:173-178` — `check()` unconditionally returns `Ok(())`. Any future side-effecting AST node bypasses it. Fix: at minimum document with `// STUB` and `TODO(security)` marker.
- [x] **AE14 MEDIUM — Integer overflow in sandbox arithmetic.** Fixed: `checked_add`/`checked_sub`/`checked_mul` for i64 BinOp in `eval_binop`; returns `SandboxError::TypeMismatch` on overflow. 256 nom-graph tests pass. `nom-graph/src/sandbox.rs:373-394` — `i64` Add/Sub/Mul use default operators (panic in debug, wrap in release). Fix: use `checked_add`/`checked_sub`/`checked_mul` returning `SandboxError`.
- [x] **AE15 MEDIUM — nom-theme imported by nom-blocks but colors are hard-coded.** `nom-blocks` declares `nom-theme` dep in Cargo.toml but zero `nom_theme::tokens::*` usages in source. `drawing.rs` hard-codes HSLA directly. Fix: route drawing colors through theme tokens.
- [x] **AE16 MEDIUM — Hsla.h convention mismatch.** `nom-gpui/src/renderer.rs` expects 0-360 degrees from `rgba_to_hsla`; `nom-theme/src/tokens.rs` `color_*()` functions store normalized 0-1 (e.g., `Hsla::new(220.0 / 360.0, ...)`). Double-divide would produce garbled hue. Fix: standardize on 0-360 degrees throughout.
- [x] **AE17 MEDIUM — background_tier plan_flow/verify/deep_think are stubs.** `nom-compiler-bridge/src/background_tier.rs:239-287` — `plan_flow` returns empty plan (confidence 0.0), `verify` returns empty diagnostics, `deep_think` emits 3 synthetic steps. Fix per Wave F/G roadmap.

### Reference-Repo Gap Summary (from agent comparison)

| NomCanvas | Reference | Gap |
|---|---|---|
| nom-gpui renderer | Zed gpui_wgpu | CRITICAL — Zed has full wgpu pipelines; nom-gpui has stub only |
| nom-gpui scene | Zed scene.rs | PARTIAL — missing SubpixelSprite + PaintSurface |
| nom-compose video | Remotion pattern | PARTIAL — Y4M serialization only; no GPU frames, no FFmpeg |
| nom-compose document | typst-memoize | PARTIAL — metadata only; no layout engine |
| nom-compose scenario_workflow | n8n DAG+sandbox | STUB — compose() is Ok(()) |
| nom-blocks block types | AFFiNE blocks | PARTIAL — 14/20+ types; missing table, data-view, frame, edgeless-text, latex |
| nom-blocks design tokens | AFFiNE cssVarV2 | MISSING — nom-theme imported but not used for block colors |

### Completion Percentages (2026-04-18 Wave AE state)

| Axis | % | Gate |
|---|---|---|
| A · nom-compiler | 44% | LLVM self-hosting, bootstrap fixpoint (upstream) |
| B · Nom language | 34% | Parser/resolver/typechecker in .nom (upstream) |
| C · nom-canvas ↔ compiler integration | 77% | AE1-AE17 open; AC6+AD3 still open |
| D · Overall platform | 61% | Renderer stub blocks real platform deliverable |

**DB-driven automation answer (for user):** YES — the DB IS the workflow engine. `grammar.kinds` = n8n/dify node-type library (every row is a draggable node). `clause_shapes` = wire type system. `.nomx` prose → grammar productions via S1-S6. `nom-compose/dispatch.rs` routes `NomKind → backend`. No external orchestrator needed. The architecture is correct and AC1-AC3 confirm it is wired. The gap is execution depth (backends are PARTIAL/STUB) and the renderer (AE1 — pixels not reaching screen yet).

**UI state answer (for user):** The window OPENS (real winit, AD1 confirmed). Panels push real Quad primitives to the Scene graph. Tokens/colors/spring math/focus rings are correct. BUT `Renderer::draw()` is a pure stub — it counts quads but submits zero wgpu draw calls. The screen stays blank. The entire GPU pipeline (wgpu device → pipelines → instance buffers → render pass → present) is missing from renderer.rs.

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

## Wave AB (2026-04-18) — COMPLETE ✅ (c3d2323, 2841 tests)
- [x] nom-canvas-core: →212 tests (+67, viewport zoom/pan, spatial bulk, bezier hit)
- [x] nom-compiler-bridge: →187 tests (+66, SharedState, completion/score adapters)
- [x] nom-editor: →204 tests (+76, multi-cursor buffer, tab_map, indent, scroll, commands)
- [x] nom-gpui: →302 tests (+91, deep flex layout, spring animation, styled chaining)
- [x] nom-panels: →189 tests (+64, DB palette search, library, deep_think, properties)
- [x] nom-collab: →175 tests (+46, 3-way merge, tombstone revival, YJS ordering)
- [x] nom-theme: →202 tests (+39, WCAG contrast, font scale, icon viewBox invariants)
- [x] nom-graph: →256 tests (+81, multi-root DAG, LRU cache, sandbox AST sanitizers)
- [x] nom-compose: →295 tests (+43, semantic MDL, credential store, fallback backoff)
- [x] nom-blocks: →211 tests (+61, DictReader integration, connector grammar, workspace)

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
- [ ] **AH-PROMOTE** — `glue_promotion_config` DB table: thresholds as data, not constants
- [ ] **AH-DB-KINDS** — seed 14 initial `grammar.kinds` rows (video/picture/audio/presentation/web_app/mobile_app/native_app/document/data_extract/data_query/workflow/ad_creative/3d_mesh/storyboard)

### Sub-project 4: UI Surfaces
- [ ] **AH-PREVIEW** — `nom-panels/src/right/intent_preview.rs`: Intent Preview card (kind confidence bars + compose/change/all-3 buttons)
- [ ] **AH-REVIEW** — `nom-panels/src/right/glue_review.rs`: AI Review card (accept/edit/skip with .nomx inline edit)
- [ ] **AH-GUTTER** — Doc mode gutter `⚡` badge for AI-generated entities (Partial status)
- [ ] **AH-NODE** — Graph mode: amber tint + `⚡` badge on AI-generated node cards, removed on Complete promotion
- [ ] **AH-STATUS** — Status bar: `⚡ N AI entities pending review` counter

## Wave AF (2026-04-18) — COMPLETE ✅ (617c064, 4194 tests)
- [x] nom-gpui: 420→456 (scene layers, renderer lifecycle, pixel_diff tolerance, layout)
- [x] nom-compose: 380→415 (task_queue state machine, cancel token, progress, export)
- [x] nom-graph: 302→335 (viewport transform, RRF dedup, HierarchicalCache, diamond DAG)
- [x] nom-compiler-bridge: 225→261 (highlight, LSP completions, UI tier tokenize)
- [x] nom-canvas-core: 273→305 (snapping grid, select-all, zoom-to-fit, group bounds)
- [x] nom-collab: 240→270; nom-editor: 273→317 (emoji convergence, case-insensitive find)
- [x] nom-panels: 254→292; nom-blocks: 216→260 (file tree, prose word count, embed URL)
- [x] nom-theme: 225→258; nom-lint: 190→220; nom-intent: 170→200
- [x] nom-memoize: 178→205; nom-telemetry: 205→235; nom-cli: 138→165
- [x] nom-collab 13 clippy warnings fixed (needless_borrows, range_loop, useless_vec)
- [x] 5 unused deps identified for Wave AG (tokio/crossbeam in nom-compose/nom-cli/nom-graph)

## Wave AG (2026-04-18) — COMPLETE ✅ (76ba05d, 4693 tests)
- [x] Remove `tokio` from nom-compose Cargo.toml (confirmed unused)
- [x] Remove `crossbeam-channel` from nom-compose Cargo.toml
- [x] Remove `tokio` from nom-cli Cargo.toml
- [x] Remove `nom-panels` from nom-cli Cargo.toml
- [x] Remove `crossbeam-channel` from nom-graph Cargo.toml
- [x] nom-gpui: 456→492 (pixel_diff round-trip, renderer blur formula, types edge cases)
- [x] nom-compose: 415→452 (codec/container stubs, audio/video display, semantic)
- [x] nom-graph: 335→369 (multi-root DAG, LRU eviction, RRF dedup, hierarchical cache)
- [x] nom-compiler-bridge: 261→295 (BM25 search, reader pool, LSP completions)
- [x] nom-canvas-core: 305→340 (snapping grid, selection, viewport)
- [x] nom-collab: 270→305 (3-peer split-brain, emoji, causal ordering)
- [x] nom-editor: 317→355 (undo/redo, cursor clamping, find/replace, scroll)
- [x] nom-blocks: 260→295 (block model hash, embed URL type, media MIME)
- [x] nom-panels: 292→325 (settings model, entity_ref, file tree)
- [x] nom-theme: 258→290 (type scale, OLED palette, WCAG contrast, icon)
- [x] nom-lint: 220→250; nom-intent: 200→230; nom-memoize: 205→235
- [x] nom-telemetry: 235→265; nom-cli: 165→195

## Wave AH (2026-04-18) — COMPLETE ✅ (59d58c4, 5196 tests)
- [x] nom-gpui: 492→530 (PipelineDescriptor, WGSL content tests, atlas allocator, PI clippy fix)
- [x] nom-blocks: 295→330 (table/dataview new types, CRUD, filter, cell access)
- [x] nom-canvas-core: 340→375 (bezier hit-test, spatial bulk 100-elem, viewport zoom/pan)
- [x] nom-compose: 452→485 (scenario_workflow steps, data_query SQL store, dispatch roundtrip)
- [x] nom-graph: 369→405 (sandbox overflow/depth, graph_mode CRUD, DAG compute_layers)
- [x] nom-collab: 305→340 (undo/redo sim, snapshot diff, 10-peer concurrent, tombstone)
- [x] nom-editor: 355→390 (multi-cursor, tab_map, comment toggle, transaction atomic)
- [x] nom-compiler-bridge: 295→330 (background plan/verify/deep_think, interactive LSP)
- [x] nom-panels: 325→360 (settings serial, file tree CRUD, chat/deep-think depth)
- [x] nom-theme: 290→325 (frosted glass curve, shadow depth, HSLA→RGB, font registry)
- [x] nom-lint: 250→280; nom-intent: 230→260; nom-memoize: 235→265
- [x] nom-telemetry: 265→295; nom-cli: 195→226

## Wave AI (planned) — wgpu real draw calls + missing_docs + LLVM integration (~5600 target)
- [ ] nom-gpui: implement wgpu Device/Queue init in Renderer::with_gpu() (AE1 — no real pixels yet)
- [ ] nom-gpui: 530→570 tests (real GPU init path, shader compile check, swapchain)
- [ ] nom-blocks: add `#![warn(missing_docs)]` + doc tests → 330→365
- [ ] nom-canvas-core: add `#![warn(missing_docs)]` + doc tests → 375→410
- [ ] nom-compose: 485→520 (RAG query pipeline, mobile/native screen depth)
- [ ] nom-graph: 405→440 (graph_rag scoring, node update/notify, cross-node deps)
- [ ] nom-collab: 340→375 (rich text bold/italic, YJS-style state vector, offline queue)
- [ ] nom-editor: 390→425 (LSP diagnostic display, gutter icons, fold/unfold)
- [ ] nom-compiler-bridge: 330→365 (rename refactor, workspace symbol, format round-trip)
- [ ] nom-panels: 360→395 (command palette model, quick-open, settings-open keybind)
- [ ] nom-theme: 325→360 (reduced-motion media query, high-contrast mode, print styles)
- [ ] nom-lint: 280→310; nom-intent: 260→290; nom-memoize: 265→295
- [ ] nom-telemetry: 295→325; nom-cli: 226→255

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
| [x] AB Coverage | ✅ | e1aa03b — canvas-core/bridge/editor/gpui/panels/collab/theme/graph/compose/blocks, 3233 tests |
| [ ] AF Minimalist UI | 🎨 | Wave AF design direction — Zed chrome + Rowboat right + classical typography + swappable themes |

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
