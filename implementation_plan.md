# Nom — Implementation Plan

**Date:** 2026-04-18 | **HEAD:** `d6219b1` | **Tests:** 523 | **Workspace:** clean
**Canonical:** spec `docs/superpowers/specs/2026-04-17-nomcanvas-gpui-design.md` · state `nom_state_machine_report.md` · tasks `task.md` · entry `INIT.md`

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

### [x] Wave M — Infra (commit `ef9fc84`)
- [x] nom-lint sealed trait · compiler-bridge 3-tier · compose dispatch/plan/task_queue · graph 4-tier cache

## Open Missions (priority order)

- [ ] **E2 CRITICAL** — `nom-canvas-core/elements.rs:225-226` `paint()` = no-op; push Quad to Scene + port circles + bezier tessellation (~100-150 LOC)
- [ ] **TQ1** `task_queue.rs:48` missing `state == Running` guard (1-line)
- [ ] **E1** no `impl Element for` on panels — document or bind
- [ ] **F1** `find_replace.rs` regex stub — import `regex` crate
- [ ] **CK1** `cache.rs:66` LruCache.get() missing `touch()` (2-line)
- [ ] **DP1** `dispatch.rs` enum-lookup, not `Box<dyn Backend>`
- [ ] **DEEP1** deep_think stream unwired — nom-panels consumer missing
- [ ] **NI1** nom-intent 98 LOC — expand ReAct + scored hypothesis tree

## Vendoring

All 20 reference repos read end-to-end 2026-04-18. Patterns catalogued in `task.md` / spec §10.

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
