# Nom — Implementation Plan

**Date:** 2026-04-18 | **HEAD:** `c3d2323` | **Tests:** 2841 | **Workspace:** clean — Wave AE audit findings recorded
**Canonical:** spec `docs/superpowers/specs/2026-04-17-nomcanvas-gpui-design.md` · state `nom_state_machine_report.md` · tasks `task.md` · entry `INIT.md`

## Current State (Wave AE, 2026-04-18)

DB-driven architecture CONFIRMED PASS (Wave AC/AD closed). Wave AE audit revealed:
- CRITICAL AE1: `renderer.rs:130` — draw methods are stubs, window opens but renders zero pixels
- CRITICAL AE2: `adapters/highlight.rs:23` — zero-width spans, syntax highlighting broken
- HIGH AE3-AE8: lsp_provider.rs duplicate stub, scenario_workflow no-op, data_query discards SQL, Backend trait disconnected, Credential Debug leaks, eval_expr no depth guard
- MEDIUM AE9-AE17: FrostedRect blur ignored, score adapter bypasses nom_score, SharedState Mutex/no-pool, BM25 unwired, NoSideEffects stub, int overflow, nom-theme unused in blocks, Hsla convention, background_tier stubs

Per-crate test counts (agent-verified ~2,614 #[test] fns in source, task.md shows 2841 run).

### Wave AE Targets

- [ ] **AE1** — `renderer.rs`: implement wgpu device → pipeline → instance buffer → draw → present
- [ ] **AE2** — `adapters/highlight.rs:23`: fix `end` offset (+ token length)
- [ ] **AE3** — delete `lsp_provider.rs`; update `lib.rs` re-exports
- [ ] **AE4** — `scenario_workflow.rs`: implement compose step loop
- [ ] **AE5** — `data_query.rs`: write SQL to artifact store
- [ ] **AE6** — add `impl Backend for` each concrete backend type
- [ ] **AE7** — custom `Debug` for `Credential` redacting `value`
- [ ] **AE8** — `eval_expr` depth guard + `sanitize()` before eval in code_exec.rs
- [ ] **AE9** — `FrostedRect` use blur_radius in tint calculation
- [ ] **AE10** — `adapters/score.rs`: call `nom_score::score_atom()` under compiler feature
- [ ] **AE11** — `SharedState`: `RwLock` for grammar_kinds; pre-open WAL read connections
- [ ] **AE12** — wire `nom_search::BM25Index::search()` as `search_bm25` in ui_tier.rs
- [ ] **AE13** — document `NoSideEffectsSanitizer` as STUB with TODO(security)
- [ ] **AE14** — checked arithmetic in sandbox BinOp eval (checked_add/sub/mul)
- [ ] **AE15** — route nom-blocks drawing colors through nom_theme::tokens
- [ ] **AE16** — standardize Hsla.h on 0-360 degrees throughout

## Architecture

- **Foundation:** nom-compiler (29 crates) — UNCHANGED, direct workspace deps
- **Shell:** Zed 3-column — Left AFFiNE 248px · Center PaneGroup (6 modes) · Right Rowboat 320px · Bottom · Status
- **Modes:** Code · Doc · Canvas · Graph · Draw · Compose (spatial, no switching)
- **GPUI:** wgpu + winit + taffy + cosmic-text — one binary, no webview

## Compose Targets

| Category | Outputs |
|---|---|
| Media | video · picture · audio · 3D mesh · storyboard · novel→video |
| Screen | web · native · mobile · presentation |
| App | full bundle · ad creative |
| Data | extract (PDF→JSON) · transform · query (WrenAI MDL) |
| Concept | document (PDF/DOCX) |
| Scenario | workflow (n8n + AST sandbox) |

## Wave Missions

### [x] Wave 0 — Bootstrap
- [x] 14 crates, workspace `Cargo.toml`, `cargo check` clean

### [x] Wave A — GPUI substrate (commit `8c7d32e`)
- [x] nom-gpui scene graph (Quad · Sprite · Path · Shadow · Underline)
- [x] 8 wgpu pipelines
- [x] cosmic-text + etagere glyph atlas
- [x] Element trait (request_layout/prepaint/paint)
- [x] Styled builder + winit `ApplicationHandler` + frame_loop
- [x] taffy wrapper + animation easing
- [x] nom-canvas-core viewport + shape + hit-test + rubber-band + R-tree
- [x] nom-theme 73 AFFiNE tokens + Inter + 42 Lucide icons

### [x] Wave B — Editor + Blocks (commit `8c7d32e`)
- [x] nom-editor rope + multi-cursor + display_map + wrap_map + tab_map
- [x] `Highlighter::color_runs` consumer + `LspProvider` trait
- [x] nom-blocks `NomtuRef` non-optional on all blocks
- [x] AFFiNE block types (heading/para/list/quote/divider/callout/code/db/linked)
- [x] Graph node ports derived from `clause_shapes` (DB-driven)
- [x] can_wire() placeholder

### [x] Wave C — nom-compiler-bridge KEYSTONE (commit `fb66e01`, 17/17 tests)
- [x] shared.rs `Arc<RwLock<SharedState>>` + dict_pool + grammar + LRU
- [x] ui_tier / interactive_tier / background_tier
- [x] adapters: highlight · lsp · completion · score
- [x] First wire: `.nomx` → stage1_tokenize → Highlighter

### [x] Wave D — 3-column shell (20/20 tests, 9 panel modules)
- [x] dock.rs (DockPosition/Dock/Panel) · pane.rs (PaneGroup) · shell.rs
- [x] Left: CollapsibleSection + QuickSearchInput + ResizePanel + dict tree
- [x] Right: ChatSidebar + tool cards + deep-think stream + multi-agent
- [x] WindowPool multi-window

### [x] Wave E — 16 compose backends (commit `a1ba5a1`, 26/26 tests)
- [x] document · video · image · audio · data_extract · data_frame · data_query
- [x] web_screen · native · mobile · presentation · app_bundle · ad_creative
- [x] workflow · scenario · rag_query · transform · embed_gen · render · export · pipeline
- [x] ArtifactStore + ProgressSink

### [x] Wave F — Graph RAG + Deep thinking
- [x] graph_rag.rs (GraphRagRetriever + BFS + cosine + RRF)
- [x] graph_mode.rs (GraphModeState + force-directed + confidence edges)
- [x] deep_think.rs (DeepThinkStream + DeepThinkStep chain)

### [x] Wave G — Stubs populated (commit `546e02d`)
- [x] nom-lint · nom-collab (RGA CRDT) · nom-telemetry (W3C traceparent)

### [x] Wave K — 4 CRITICAL closures (commit `dc6a025`, 457 tests)
- [x] U1 — nom-panels `paint_scene` uses nom_gpui::scene::Quad + nom_theme::tokens
- [x] W1 — deep_think imports `nom_intent::classify_with_react` real ReAct
- [x] COL1 — RGA CRDT (RgaPos anchor + tombstone + convergence tests)
- [x] INT1 — 11+ cross-crate `use nom_gpui::scene` imports

### [x] Wave L — MEDIUM closures (commit `d139644`)
- [x] with_deep_think wired · W3C traceparent · RRF `1/(60+rank)` · impl Element on elements

### [x] Wave M — Infra (commit `ef9fc84`, 498 tests)
- [x] nom-lint sealed trait · compiler-bridge 3-tier · compose dispatch/plan/task_queue · graph 4-tier cache

### [x] Wave N — Infra+Vendor (commit `d6219b1`, 523 tests)
- [x] nom-compose: MediaVendor trait + ProviderRouter 3-tier fallback + CredentialStore kind-keyed
- [x] nom-graph: 4 AST sanitizers + eval_expr
- [x] nom-compose: SHA-256 ContentHash via sha2 + SemanticModel/SemanticRegistry MDL

### [x] Wave O — Infra+LSP (commit `e61a93c`, 537 tests)
- [x] nom-compiler-bridge: CompilerLspProvider (real completions/diagnostics from nom-compiler)
- [x] nom-compose: code_exec (n8n JsTaskRunner + sandbox wiring) + web_screen (headless browser stub)
- [x] nom-graph: ExecutionEngine cancel/abort signal + HierarchicalCache L1→L2 promotion wiring

### [x] Wave P — Bug fixes + MEDIUMs (commit `15a8366`, 558 tests)
- [x] E2 CRITICAL: GraphNodeElement::paint() + WireElement::paint() push real Quads + port circles + bezier
- [x] TQ1/CK1/DP1/F1/NR1/NI1/DEEP1/FG1/FR1/PAL1 HIGHs closed
- [x] MEDIUMs: RRF_K const + InternalRule 3rd trait + HierarchicalCache::len L1+L2 + Panel trait 7 methods

### [ ] Wave V — GPU library wiring + test coverage push (2026-04-18)
- [ ] **ETAGERE-WIRE** — atlas.rs uses `BucketedAtlasAllocator` from etagere (not manual shelf)
- [ ] **BYTEMUCK-WIRE** — `QuadInstance`/`SpriteInstance`/`GlobalUniforms` derive `Pod+Zeroable`
- [ ] **CLI-EXPAND** — nom-cli: 5 new commands (version/help/run/format/rag--top-k) + 10 tests → 28+
- [ ] **TEST-CANVAS-CORE** — nom-canvas-core: +12 tests → 50+
- [ ] **TEST-THEME** — nom-theme: +12 tests → 32+
- [ ] **TEST-EDITOR** — nom-editor: +12 tests → 64+
- [ ] **TEST-GRAPH** — nom-graph: +12 tests → 79+
- [ ] **TEST-COMPOSE** — nom-compose: +12 tests → 106+
- [ ] **TEST-INTENT-COLLAB-MEMO** — +15 tests across intent/collab/memoize
- [ ] **LINEAR-RGBA** — `LinearRgba` type + `ColorSpace` enum added to nom-gpui

**Goal:** wire real GPU library types (etagere atlas allocation, bytemuck GPU buffer safety) and push total tests from 733 → 800+.

## Vendoring

Reference-repo claims require a fresh parity audit before being used as pass evidence. Local source paths exist for many references, but Remotion, Open-Higgsfield, and opendataloader were not found under the claimed local patterns during Wave AC; mark those as unverified until paths or evidence are supplied.

## Non-Negotiable Rules

1. Read source repos end-to-end before writing code
2. Always use `ui-ux-pro-max` skill for UI work
3. Zero foreign identities in public API
4. nom-compiler is CORE — direct workspace deps, zero IPC
5. DB IS the workflow engine — no external orchestrator
6. Every canvas object = DB entry — `entity: NomtuRef` non-optional
7. Canvas = AFFiNE-for-RAG
8. Doc mode = Zed + Rowboat + AFFiNE
9. Deep thinking = compiler op streamed right dock
10. GPUI fully Rust — no webview
11. Spawn parallel subagents for multi-file work
12. Run `gitnexus_impact` before editing any symbol
