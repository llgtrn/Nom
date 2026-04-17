# Nom Programming Language — Context

> **HEAD:** `56604c4` on main (wave-10 landed — 1272 workspace tests, 13 crates) | **Date:** 2026-04-17
> **Compiler:** 29 crates, 1067 tests (unchanged) | **Canvas v1:** ARCHIVED (`.archive/nom-canvas-v1-typescript/`)
> **NomCanvas (render + edit substrate):** 13 crates shipped, **1272 tests** green under `RUSTFLAGS=-D warnings`. Phase 1 + Phase 2 (100%) + Phase 3 (~100%, line_layout.rs landed in wave-6) + Phase 4 scaffolds (nom-compose) + Phase 5 scaffolds (nom-lint, nom-memoize, nom-telemetry, nom-collab) shipped.
> **⚠️ "Compose by natural language on canvas" promise: 0% delivered.** Despite 12 crates + 1028 tests + 11 compose-backend stubs: (a) input path DEAD — user keystrokes reach `Buffer` then die as `String`; no `use nom_concept::`/`nom_grammar::`/`nom_lsp::` anywhere; no syntax highlighting producer; (b) output path DEAD — `ComposeDispatcher::dispatch()` only called in tests, two disconnected `CompositionPlan` structs (canvas + compiler), backends return empty bytes, `ArtifactStore::put` has 0 non-test callers, preview blocks don't read from store. **Keystone wire (architect pick):** `nom_concept::stage1_tokenize` → `Highlighter::color_runs` adapter. Both endpoints exist + compatible + pure. Est. ~200 LOC + cross-workspace Cargo path-dep. Unlocks first user-visible compiler behavior (keywords highlight as user types). All subsequent integrations chain from here.
> **19-repo vendoring: 58% integrated** (+5pp this cycle). **8 DEEP**: Zed/AFFiNE/yara-x/typst/WrenAI/9router/**n8n** (new: AST sandbox in nom-graph-v2/sandbox.rs)/**Remotion** (promoted: VideoComposition concrete in nom-compose/video_composition.rs). 3 PATTERN: ComfyUI/waoowaoo/ArcReel. 6 REFERENCED-ONLY. 2 NOT-USED. Next-tick target: LlamaIndex REF→PATTERN.
> **Foundation:** Everything built around Nom language. Compiler IS the IDE *(aspirational — currently 0% wired)*. Dictionary IS the knowledge base.

## NomCanvas — Full Rust GPU-Native Universal Composition Engine

**Spec:** `docs/superpowers/specs/2026-04-17-nomcanvas-gpui-design.md` (719 lines)

**Architecture:** Custom GPUI (NOT Dioxus — Dioxus desktop = webview). wgpu + winit + taffy + cosmic-text.
**Core:** Compiler IS the IDE. 10 crates → 3 thread tiers (UI <1ms, Interactive <100ms, Background >100ms).
**Modes:** Code + Doc + Canvas + Graph + Draw — all on one infinite GPU surface.
**Composition:** Universal — write Nom → get video/image/PDF/data/app/audio/3D.
**Video:** Remotion pattern in Rust — GPU scene graph renders frames → FFmpeg pipe → MP4 (no browser).
**Targets:** Desktop (Vulkan/Metal/DX12) + Browser (WebGPU) from same binary.

**19 repos read end-to-end:** Zed (GPUI), AFFiNE (design), ComfyUI (DAG), Refly (skill+MCP), LlamaIndex (RAG), Haystack (pipeline), ToolJet (components), n8n (workflow), yara-x (linter), typst (incremental compile), Dioxus (confirmed webview), ArcReel (video agents), waoowaoo (storyboard), Open-Higgsfield (200+ models), opendataloader-pdf (data extraction), WrenAI (semantic layer), Huly (collaboration), 9router (provider routing), Remotion (programmatic video via DeepWiki).

## Compiler (29 crates, unchanged)

GAP-4/5a/5b/6/7/8/12 shipped. 1067 tests. GAP-1c in progress. GAP-2/3 blocked. GAP-9/10 planned. bootstrap.rs (GAP-10 real impl) landed.

## NON-NEGOTIABLE Rules

1. Everything built around Nom language foundation (9 kinds compose everything)
2. Executing agents MUST read source repos end-to-end before writing ANY code
3. Always use `ui-ux-pro-max` skill for ALL UI design
4. Zero foreign identities in Nom codebase
5. MACRO point of view every iteration
6. Spawn subagents to plan/structure/create tasks
7. Strict external comparison (study from source paths)

## Session 2026-04-17 — Waves 4→10 (+1068 tests, +10 crates)

**8 commits on main:** `c2d7090` → `24f7e05` → `4592b85` → `9f3df57` → `2e47d5d` → `365db9b` → `4096db9` → `56604c4`.

**Test growth:** 204 → 376 → 519 → 751 → 870 → 1028 → 1155 → **1272 workspace tests** across 13 crates, all green under `NOM_SKIP_GPU_TESTS=1 RUSTFLAGS="-D warnings -A deprecated"`.

**10 new crates** added to `nom-canvas/crates/`: `nom-theme`, `nom-panels`, `nom-blocks`, `nom-graph-v2`, `nom-compose`, `nom-lint`, `nom-memoize`, `nom-telemetry`, `nom-collab` (Phase 3-5 scaffolds).

**Phase coverage:**
- Phase 1 (`nom-gpui`) — added `animation.rs`, `cursor.rs`, `transition.rs`, `hit_test_tests` module, 3 pixel-diff tests for shadow/polychrome/subpixel.
- Phase 2 (`nom-canvas-core` + `nom-editor`) — COMPLETE. Added: selection/marquee/transform_handles/snapping/history/rendering_hints (canvas-core); anchor/selection/selections_collection/movement/editing/syntax_map/display_map/lsp_bridge/inlay_hints/wrap_map/tab_map/line_layout/commands (editor).
- Phase 3 (blocks/panels/theme) — shipped: block_schema/model/transformer/selection/config + 7 block types (prose, nomx, media, graph_node, drawing, table, embed) + 6 compose-preview blocks + tree_query + validators; 8 panels + shortcuts + command_history + layout; 73 tokens + color + mode + fonts + icons + typography + motion.
- Phase 4 (compose) — all 11 `NomKind` variants have stub backends (video, image, web_screen, native_screen, data_extract, data_query, storyboard_narrative, audio, data_frame, mesh, scenario_workflow) + `register_all_stubs()` + dispatcher + task_queue + provider_router + credential_store + artifact_store + vendor_trait + video_composition + format_translator + semantic + plugin_registry.
- Phase 5 (production) — scaffolds for: `nom-graph-v2` (Kahn topology + 4 caches + sandbox + execution), `nom-lint` (sealed-trait linter + 2 concrete rules + watcher), `nom-memoize` (thread-local cache), `nom-telemetry` (W3C traceparent + rayon bridge), `nom-collab` (CRDT protocol + presence).

**Integration tests added:** pixel-diff (gpu_integration.rs) + diamond DAG end-to-end (nom-graph-v2/tests/end_to_end.rs) + dispatcher-all-kinds (nom-compose/tests/dispatch_all_kinds.rs) + commands (nom-editor/tests/commands_integration.rs).

**Audit findings resolved (2 HIGH, 5 MEDIUM, 1 LOW):**
- HIGH animation.rs `Duration::ZERO` → NaN propagation (wave-6)
- HIGH `NarrativeResult::completed_phase()` skipped `Storyboard` phase → added `video_output_hash` field (wave-9)
- MEDIUM `EmbedKind::Youtube/Figma` brand-name identifiers → `VideoStream/DesignFile` (wave-6)
- MEDIUM three colliding `Rgba` types → `drawing::Rgba` renamed `SrgbColor` (wave-9)
- MEDIUM `FractionalIndex` duplicated across 3 files → hoisted to `block_model.rs` (wave-9)
- MEDIUM CI canvas job missing `NOM_SKIP_GPU_TESTS=1` env → added (wave-6)
- MEDIUM `RUSTFLAGS=-D warnings` turned dead_code/unused_import into errors → 5 sites fixed (24f7e05)
- LOW `CommandError::Failed` variant unconstructed → `#[allow(dead_code)]` (56604c4)

**CI status:** Canvas job went from consistently-failing → consistently-green starting at 24f7e05. Waves 5/6/7 all verified green on main. Waves 8/9/10 pending at session end.

**Still 0%:** Compose-by-natural-language user promise (iter-17 audit) — render substrate + 1272 tests + 11 backend stubs exist, but input path (prose → compiler) and output path (artifact → canvas preview) are fully disconnected. Keystone wire identified: `nom_concept::stage1_tokenize → Highlighter::color_runs` adapter (~200 LOC), blocked on cross-workspace cargo path-dep decision.
