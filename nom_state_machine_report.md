# Nom Compiler + NomCanvas IDE — State Machine Report

> **CANONICAL TRACKING DOC — MAIN** (Planner/Auditor refreshes every cycle)
> **HEAD:** `e2b7ecb` on main | **CI:** canvas job GREEN (cargo check + test 23s on ubuntu-latest); compiler matrix still running | **Date:** 2026-04-17
> **Sibling docs:** `implementation_plan.md`, `task.md`, `docs/superpowers/specs/2026-04-17-nomcanvas-gpui-design.md` (all 4 MUST stay in sync)
> **Compiler:** 29 crates, 1067 tests | **Canvas v1:** ARCHIVED to `.archive/nom-canvas-v1-typescript/`
> **NomCanvas:** Custom GPUI (wgpu+winit+taffy+cosmic-text). Full Rust. GPU-native. Phase 1 foundation landed.
> **Foundation:** Everything built around Nom language. 9 kinds compose everything in the world.

---

## Current State

**Compiler:** 29 crates, 1067 tests. GAP-4/5a/5b/6/7/8/12 shipped. bootstrap.rs (GAP-10) landed. nom-intent has 7 modules (ReAct, prompt, tools, rerank).

**Canvas v1:** ARCHIVED to `.archive/nom-canvas-v1-typescript/`.

**NomCanvas Phase 1 batch-1 (NEW):** 1 crate (nom-gpui), 9 modules, 31/31 tests passing. Scene graph (6 primitive types, z-ordered via BoundsTree R-tree), Element trait (3-phase lifecycle), taffy layout wrapper, Styled fluent builder, PlatformAtlas trait + in-memory impl, geometry + color primitives. Zero foreign identities; zero wrappers — every type is native-implemented. Deps: `wgpu 22`, `taffy 0.6`, `cosmic-text 0.12`, `winit 0.30`, `etagere 0.2`, `bytemuck 1`, `parking_lot`, `smallvec`. Remaining in Phase 1: wgpu renderer, cosmic-text/etagere atlas wiring, winit window loop, browser/desktop platform abstraction.

**NomCanvas Design:** `docs/superpowers/specs/2026-04-17-nomcanvas-gpui-design.md` (719 lines). Custom GPUI. Compiler-as-core. Universal composition. 5 unified modes. 19+ repos read end-to-end.

**nom-workflow skill:** Upgraded with AUDIT lane, full Superpowers list (17 skills), graphify integration, 21 reference repos.

## NomCanvas Key Decisions

| Decision | Choice | Why |
|----------|--------|-----|
| Framework | Custom GPUI | Dioxus desktop = webview (confirmed by end-to-end reading) |
| GPU | wgpu | Cross-platform: Vulkan/Metal/DX12 + WebGPU for browser |
| Layout | taffy | Flexbox/grid in Rust (same as Zed) |
| Text | cosmic-text | Font shaping without platform dependency |
| Compiler | Direct function calls | No IPC, no JSON, no Tauri — compiler crates linked as deps |
| Video | Remotion pattern in Rust | GPU scene graph → frame capture → FFmpeg pipe |
| Design | AFFiNE tokens | Inter + Source Code Pro, 73 variables, pixel-perfect |

## 19 End-to-End Repo Readings

Zed (GPUI rendering), AFFiNE (design system), ComfyUI (DAG + 4 caches), Refly (46 modules + MCP), LlamaIndex (50+ stores), Haystack (component pipelines), ToolJet (55 widgets), n8n (304 nodes + AST sandbox), yara-x (sealed trait linter), typst (comemo incremental), Dioxus (webview confirmed), ArcReel (video agents), waoowaoo (4-phase storyboard), Open-Higgsfield (200+ models), opendataloader-pdf (XY-Cut++), WrenAI (semantic MDL), Huly (30 services + CRDT), 9router (3-tier fallback routing), Remotion (programmatic video via DeepWiki).

## Session Summary

- Brainstormed → designed → built v1 (37 commits, 46 modules)
- 8-agent audit found 59 issues → fixed 8 CRITICAL
- 2nd audit: 3 CRITICAL + 5 HIGH remaining in v1
- 6-agent deep-dive extracted 60 patterns from 48 repos
- v2 design: Custom GPUI + compiler-as-core + universal composition
- 19 repos read fully end-to-end (not README — actual source)
- Remotion video pattern adapted for GPU-native Rust

## Plan Completeness Tracker (loop goal → 100%)

| Scope | Detailed % | Notes |
|-------|-----------|-------|
| Phase 1 batch-1 | **100% landed + all 3 CRITICAL + 4 HIGH audit fixes VERIFIED** (44/44 tests) | 7 MED + 6 LOW + 5 test gaps still open, none block batch-2 start |
| Phase 1 batch-2 | **95%** | iter-5 added concrete WGSL skeletons (bind groups, unit-quad unpack, NDC normalize, SDF quad, helper fn list), 3 waves (shaders+buffers+ctx / atlas+text+window / full 8 pipelines). Zed shaders.wgsl 1335-line single file + wgpu_renderer.rs pipeline/surface patterns cited throughout. Still need: winit 0.30 ApplicationHandler scaffold + wgpu_context device-lost recovery detail |
| Phase 2 (canvas+editor) | 85% | 29 subtasks across 2 crates + 10 test targets + 6 explicit non-goals + Excalidraw/Zed/AFFiNE file:line citations |
| Phase 3 (blocks+panels) | **85%** | iter-5 decomposed into ~45 subtasks: shared infra (5) + 7 block types × (schema + render + events + transformer) + nom-panels (7) + nom-theme (4) + 4 ADOPT patterns + 3 SKIP patterns + 8 test targets. AFFiNE defineBlockSchema pattern ported as `define_block_schema!` macro |
| Phase 4 (composition) | **100%** | iter-9 backfilled 5 missing backends (media/storyboard waoowaoo 4-phase, media/novel→video ArcReel adapted via nom-intent, media/audio, data/transform Polars MVP new `nom-data` crate, media/3D glTF) + `MediaVendor` trait (blueprint §12) + content-addressed artifact store `~/.nom/store/<hash>/` + 6 compose-output preview block types + WrenAI 5-stage pipeline detail + 11 iter-9-specific SKIP items. Total: ~95 Phase 4 subtasks. **Zero foreign identities mandate honored**: ArcReel Claude-SDK-direct → adapted to nom-intent agents + MediaVendor facade; polars crate → write own nom-data MVP |
| Phase 5 (production) | **90%** | iter-8 decomposed into ~35 subtasks across 5 new crates: nom-lint (6 modules, yara-x sealed pattern + Nom-improvement visitor + incremental cache), nom-collab (7 modules, Huly-minimal Rust port of Hocuspocus w/ Yrs), nom-memoize (5 modules, comemo pattern port WITHOUT the crate dep), nom-telemetry (6 modules, 4-tier OTel + W3C propagation), file-watcher (2 modules). 13 test targets including compile-fail sealed-trait check + CRDT convergence property test |
| **Overall plan** | **100%** | Blueprint fully decomposed across all 18 sections. Remaining work is EXECUTION, not planning. Executor fix-wave stalled 5 iterations blocks wave-2 start |

**Blueprint-gap backfill (iter-8):** caught 6 modules named in spec §8 but missed in earlier decompositions — `nom-gpui/animation.rs`, `nom-editor/input.rs` (IME!) + `completion.rs`, `nom-panels/properties.rs`, `nom-theme/fonts.rs` + `icons.rs`. Added as Phase 1/2/3 addenda.

**Iteration 13 delta:** Commit `cb40522` landed (batch-2 wave-3 — renderer + hit-test + integration, 99→113 tests). 6 parallel audit agents. Key findings:

**✅ Wave-3 shipped real code, not stubs:**
- `Renderer::draw()` ([renderer.rs:189-279](nom-canvas/crates/nom-gpui/src/renderer.rs#L189-L279)) iterates `scene.batches()` with proper bind groups (globals + instances), `begin_render_pass` with Clear→Store, `TriangleStrip` `pass.draw(0..4, 0..count)` per pipeline, `FrameGlobals` uniform written pre-frame. 3 of 4 primitive types wired (Quad, MonoSprite, Underline); SubpixelSprite + Shadow + Path + Poly are empty match arms (silent drop)
- `Scene::hit_test(point) -> Option<HitResult>` ([scene.rs:206](nom-canvas/crates/nom-gpui/src/scene.rs#L206)) works; 3 tests pass (basic hit, miss, topmost-by-order)
- Headless `gpu_integration.rs` test: full offscreen wgpu Device + RenderPass + `buffer.map_async` readback + pixel assertion on clear color
- Security posture preserved: **1 unsafe block unchanged**, 0 new deps, all Pod `#[repr(C)]` correct, device-lost properly propagated

**⚠️ iter-10 fix-wave: 0 of 7 items addressed. But severity re-classification warranted:**
- `order: u32` on Scene primitives EXISTS ([scene.rs:15](nom-canvas/crates/nom-gpui/src/scene.rs#L15)); `Scene::finish()` sorts by it; renderer paints in batch order = **painter's algorithm works**. Z-sort is NOT broken at runtime — CPU-side sort handles it. Missing `order` field on **GPU-side Pod structs** (QuadInstance WGSL, GpuQuad Rust) matters only if we add depth-buffer-based sorting later. **Re-classify iter-10 items 1-2 from CRITICAL → MEDIUM.**
- `bytes_per_row` alignment: agent-disagreement; Agent 1 says MISSING at [wgpu_atlas.rs:287, :311](nom-canvas/crates/nom-gpui/src/wgpu_atlas.rs#L287) (raw `width * bpp`, no 256-align); Agent 6 claimed LANDED but referenced `using_alignment(adapter.limits())` which is a Limits builder method, not bytes_per_row. **Ground truth: STILL MISSING**. First glyph with width non-multiple-of-256/bpp will trigger wgpu validation panic on Vulkan/DX12. Still HIGH.
- 20-frame overflow guard: MISSING. Silent VRAM exhaustion risk remains. Still MEDIUM (was CRITICAL; no evidence of exhaustion at test load).
- GammaParams: MISSING. Still HIGH — blocks subpixel text.
- SubpixelSprite pipeline: MISSING. Still MEDIUM — silent drop of subpixel sprites (empty match arm at [renderer.rs:272](nom-canvas/crates/nom-gpui/src/renderer.rs#L272))
- SubpixelVariant 4×4: MISSING. LOW — cosmetic AA quality.

**⚠️ Integration is "plumbed, not done":**
- `Renderer` struct is NOT `pub` in [lib.rs:21-39](nom-canvas/crates/nom-gpui/src/lib.rs#L21-L39) — external code can't construct it. Crate-private API.
- Only `FrameHandler` impl is `NoopHandler` (test-only) at [frame_loop.rs:274](nom-canvas/crates/nom-gpui/src/frame_loop.rs#L274). No `ExampleFrameHandler` / `CanvasFrameHandler` that actually calls Renderer
- `LayoutEngine::compute_layout` never called from frame path. Layout disconnected from render.
- `Element::paint` never invoked from frame path. Tests populate Scene manually via `insert_quad()`
- Hit-test uses O(N) brute-force, NOT BoundsTree (explicitly deferred). `HitResult` returns primitive index, NOT `ElementId`. No DrawOrder→ElementId reverse map. No pointer event routing (zero `CursorMoved`/`MouseInput` handlers). `ElementStateMap` scaffolding exists but not wired.
- No `examples/` dir. Can't produce on-screen pixel from scratch without writing custom FrameHandler that manually instantiates (currently private) Renderer.

**❌ Test quality: STILL D+ (unchanged since iter-6, 3 iterations flat)**
- +14 tests; ~3 meaningful (batch-iterator + texture-break). 3 tautological (`renderer_constructs`, `pipelines_construct_on_bgra_and_rgba`, size-assertion checks)
- Silent-skips WORSENED: 15+ → **23 total** (`let Some(...) = gpu_pair() else { return }` pattern), now in 6 files
- The 1 pixel-readback test only verifies clear color (empty scene), not drawn primitives
- Still NO: BoundsTree proptest, bytes_per_row alignment test, subpixel variant diff test, multi-frame test, pipeline selection test, DUAL_SOURCE_BLENDING fallback test

**Wave-4 priority list (ordered by blocking severity):**
1. **Make `Renderer` `pub`** (1-line fix in lib.rs) — or nothing outside the crate can render
2. **Fix `bytes_per_row` 256-alignment** in wgpu_atlas — will crash on real hardware
3. **Write concrete FrameHandler** that wires Layout+Element+Scene+Renderer end-to-end
4. **Wire winit pointer events → hit_test → ElementId dispatch**
5. **Add pixel-readback test for drawn primitive** (not just clear color)
6. **Add BoundsTree proptest** (iter-4 gap still open after 10 iterations)
7. Then GammaParams + SubpixelSprite pipeline before subpixel text renders

**Iteration 12 delta:** +0.5 pp (loop-edge). Added **cross-phase integration test matrix** — a dimension the plan didn't have (per-phase tests exist, but no end-to-end tests spanning phases). 10 integration scenarios defined:

| # | Scenario | Phases spanned | Key assertion |
|---|----------|----------------|---------------|
| I1 | User opens empty canvas → drags prose block → types → saves | 1→2→3 | Block persists to nom-dict; reloads identical |
| I2 | Compile .nomx file → emit error → error decoration appears at exact char offset | 1→2+compiler | Error span == tree-sitter token span |
| I3 | Drag connector between 2 graph nodes → wire scores green | 3+compiler | `nom-score::can_wire()` called sub-1ms |
| I4 | Write prose describing video → press "compose" → MP4 artifact appears in preview block | 3→4 | `~/.nom/store/<hash>/body.mp4` exists + plays |
| I5 | 2 clients edit same canvas via WebSocket → changes converge | 1→2→3→5-collab | CRDT property: both see identical Y.Doc state |
| I6 | Linter catches bug → Fix applied via keyboard shortcut | 2+5-lint | Diagnostic.fix.apply() produces expected text edit |
| I7 | Edit 1-char in large file → tree-sitter incremental re-parse only affected region | 2+5-memoize | Instrumentation counter: ≤ 1 layer re-parsed |
| I8 | Open 500-element canvas → zoom 0.1→10 pivot at center | 1→2 | 60fps maintained; zoom-to-point invariant holds |
| I9 | User prose "extract tables from PDF" → data block with 100-row table | 3→4 | opendataloader-pdf XY-Cut++ output matches golden |
| I10 | Run composition of 50 scenes → task queue shows progress → cancel at frame 25 | 3→4 | Frame 26+ not rendered; artifact_store has partial + cancel marker |

These are not yet in task.md — recording here as a planning artifact. Promoting to task.md contingent on wave-3 landing (integration tests depend on working render path).

**Iteration 11 delta:** 0 pp. No new commits (HEAD still `910f29a`). **Direct source-code verification** via grep confirms all 6 iter-10 defects remain in working tree:
- `order: u32` — zero hits in `nom-canvas/crates/nom-gpui/src/` for instance struct order field
- `overflow_frames` / `OVERFLOW_FRAMES` — zero hits (guard unimplemented)
- `bytes_per_row` alignment — only `align_up` helper in `buffers.rs`; `wgpu_atlas.rs` doesn't use it (still misaligned)
- `GammaParams` — zero hits (uniform absent)
- SubpixelSprite **pipeline** — zero hits for `subpixel_sprites` pipeline (the Scene Kind + BatchIterator exist but no `RenderPipeline` in pipelines.rs)
- SubpixelVariant 4×4 — `y\.0 / 4.0` pattern not found (still `y/2.0` = 4×2)

**Loop has reached its useful planning limit.** Plan is 100%. 6 critical defects block wave-3. Next productive action is Executor landing the 6-item fix commit per iter-10 architectural verdict (which already lists them actionably). Further 1-minute planning iterations at this point generate no new signal; recommend either (a) pause cron until Executor advances, or (b) shift loop goal from "expand plan" to "audit after each commit" (cron wakes, sees no commit, exits in <5s; wakes on commit, dispatches 6 audit agents).

**Iteration 10 delta:** Commit `910f29a` landed (batch-2 wave-2 — atlas + text + window + frame_loop, 59→99 tests). 6 parallel agents audited (5 full reports, 1 API-overloaded). **Mixed outcome:**

- ✅ **text.rs, window.rs, frame_loop.rs, wgpu_atlas.rs STRUCTURE** all MATCH iter-5 spec (cosmic-text ShapeLine+Shaping::Advanced, swash ScaleContext+Render::new, Bgra8→Rgba8 fallback, Fifo, desired_maximum_frame_latency:2, BucketedAtlasAllocator per slab, parking_lot Mutex Arc-wrapped). ApplicationHandler trait, no thread::sleep, device-loss recovery wired.
- ✅ **Security posture preserved** — 1 pre-existing unsafe in window.rs:89 (documented SAFETY), 0 new unsafe, all Pod `#[repr(C)]`, font loading via system DB only, resize clamped, zero foreign identifiers in public API, zero wrappers (GpuAtlas + WindowSurface + App add real logic).
- ❌ **iter-6 fix-wave STILL 3/4 UNRESOLVED (5 loop iterations stalled)**: `QuadInstance` + `MonoSpriteInstance` still have NO `order: u32` (Z-sort broken in rendering), `MonoSpriteInstance` missing `transformation` (rotated glyphs impossible), 20-frame overflow guard STILL unimplemented (silent VRAM exhaustion). `pub mod context;` was already fixed in wave-1 `205aea9`.
- ❌ **NEW wave-2 HIGH defects**: (a) `bytes_per_row` NOT aligned to `COPY_BYTES_PER_ROW_ALIGNMENT=256` in [wgpu_atlas.rs:309-311](nom-canvas/crates/nom-gpui/src/wgpu_atlas.rs#L309-L311) — wgpu validation error on first non-256-aligned glyph; (b) `GammaParams` STILL missing from globals BGL ([pipelines.rs:72-84](nom-canvas/crates/nom-gpui/src/pipelines.rs#L72-L84)) — subpixel text will render with wrong gamma; (c) SubpixelSprite render pipeline ABSENT ([renderer.rs:259-260](nom-canvas/crates/nom-gpui/src/renderer.rs#L259-L260) logs "batch skipped (batch-3)") — subpixel glyphs silently dropped; (d) 4×4 subpixel variants reduced to 4×2 ([text.rs:33-38](nom-canvas/crates/nom-gpui/src/text.rs#L33-L38)) — divergence from blueprint §4.
- **Test quality: D+ (unchanged from iter-6)**. +40 tests dominated by happy-path + silent-skip GPU tests. iter-4 gaps: 4 of 5 filled. New gaps: bytes_per_row alignment, subpixel differentiation, window lifecycle, redraw-request dispatch, DUAL_SOURCE_BLENDING fallback. 3 tautological tests flagged.
- **Architectural verdict**: wave-3 CAN proceed at architecture level, but Executor MUST land a single fix commit with: `order` fields + `transformation` + 20-frame guard + bytes_per_row alignment + GammaParams + SubpixelSprite pipeline + 4×4 variants — otherwise first real text rendering will panic (bytes_per_row) or render visibly wrong (Z-sort + gamma + missing subpixel path).

**Iteration 9 delta:** +4 pp (95→99% overall). Read blueprint §12-15; backfilled 5 missing Phase 4 backends (media/storyboard, media/novel→video, media/audio, data/transform, media/3D) + MediaVendor trait + artifact_store + 6 compose-output preview blocks + WrenAI 5-stage pipeline. 2 parallel agents read waoowaoo+ArcReel + Polars.

**Iteration 8 delta:** +5 pp (82→95%). Blueprint §8-11 re-read surfaced 6 missed modules (animation/input/completion/properties/fonts/icons). 3 agents read yara-x + Huly + typst-comemo + OpenTelemetry. Phase 5 decomposed 15→90% across 5 new crates (nom-lint, nom-collab, nom-memoize, nom-telemetry, file-watcher).

**Iteration 7 delta:** +8 pp. 3 agents ComfyUI + n8n + typst/Remotion. Phase 4 20→90%.

**Iteration 6 delta:** 0 pp. Commit `205aea9` (wave-1 59/59 tests) audited; 4 CRITICAL + 7 HIGH flagged.

**Iteration 1 delta:** +5 pp (Phase 1 batch-2 decomposed to 12 tasks with Zed citations)
**Iteration 2 delta:** +7 pp — Commit `e2b7ecb` landed; 6 agents found 3 CRITICAL + 4 HIGH + 10 MED + 6 LOW; archive clean, 0 unsafe/CVEs
**Iteration 3 delta:** +13 pp — 3 agents decomposed Phase 2 into 29 tasks from Excalidraw/Zed-editor/AFFiNE-GFX end-to-end reads
**Iteration 4 delta:** +6 pp — Commit `1daa80e` (Executor audit-fix, 31→44 tests). 6 parallel agents verified **all 3 CRITICAL + 4 HIGH landed cleanly**. 1 bonus MED (`max_leaf` fast-path) opportunistically done. Security posture preserved. Test quality grade **C+**: strong on z-interleave + bounds overlap + styled borrow-lifecycle; weak on sprite ABA, HSL boundaries, LayoutError path. One tautological test flagged for trybuild rewrite. Two intentional Zed-divergences noted (Styled `&mut self`, Hsla `[0,360)`) — functional, documented, not regressions. **Architectural verdict: batch-2 GPU work can proceed.**
**Iteration 5 delta:** +11 pp — 3 parallel Explore agents deep-read: (1) Zed `shaders.wgsl` 1335-line single file → all WGSL skeletons + helper functions (quad_sdf_impl, gaussian, erf, oklab, enhance_contrast) extracted verbatim; (2) Zed `wgpu_renderer.rs` + `wgpu_context.rs` → 4 bind-group-layout patterns + surface config + DUAL_SOURCE_BLENDING optional feature + MSAA adapter gating; (3) AFFiNE blocks/ → schema-first `defineBlockSchema()` pattern, universal block model (id+flavour+props+children), 7-Nom-block mapping to AFFiNE refs, Lit/CSS/floating-ui flagged as SKIP. Phase 1 batch-2 bumped 85→95% with 3-wave decomposition (shaders+buffers+ctx / atlas+text+window / full 8 pipelines). Phase 3 bumped 25→85% with ~45 subtasks covering shared infra + 7 block types × 4 aspects (schema/render/events/transformer) + nom-panels + nom-theme + 8 test targets.

**New standing rule (2026-04-17):** Every Planner/Auditor iteration MUST read the blueprint `docs/superpowers/specs/2026-04-17-nomcanvas-gpui-design.md` FIRST — before git log, before agents, before commits. Added to `nom-planner-auditor` skill as NON-NEGOTIABLE section, saved as persistent memory `feedback_always_read_blueprint.md`.

**Iteration 10 delta:** +1 pp (99→**100%**). Final blueprint pass: §15-18 read; backfilled `FallbackStrategy` 3-variant enum (Fallback/RoundRobin/FillFirst), `nom-collab::transactor` (missing 4th field in `CollaborationEngine` per §16 — immutable event log separate from state snapshots), and Remotion-pattern concrete `VideoComposition` / `SceneEntry` struct from §18 with content-addressing advantage over Remotion (per-scene render cache via artifact_store hash). **Plan is complete.** Every section of the 719-line blueprint is decomposed into actionable subtasks with file:line citations across 5 phases (Phase 1 batch-1+2 GPU framework, Phase 2 canvas+editor, Phase 3 blocks+panels, Phase 4 universal composition with 12 backends, Phase 5 production quality with 5 new crates).

**⚠️ EXECUTION STALL (5 iterations and counting):** iter-6 fix-wave at wave-1 commit `205aea9` remains unlanded. Blocks wave-2 (atlas+text+window) regardless of how detailed the plan gets. The 4 CRITICAL defects (missing `order` field on both instance structs, missing `transformation` on sprite, unimplemented 20-frame overflow guard, dead-code `context.rs` not re-exported in lib.rs) must ship before wave-2 can begin. Cron loop's marginal value is now near-zero until Executor advances — future iterations should be audit-focused on whatever the Executor finally commits, not further expansion. Recommend either (a) pause cron and let Executor catch up, or (b) keep cron running but expect 0-pp iterations until a new commit lands.

**Iteration 9 delta:** +4 pp (95→99% overall). Read blueprint §12-15 first (standing rule); surfaced 5 backend gaps in iter-7 decomposition: media/storyboard (waoowaoo 4-phase), media/novel→video (ArcReel agent workflow), media/audio (synthesis+codec), data/transform (Polars MVP new `nom-data` crate), media/3D (glTF). Also surfaced `MediaVendor` trait + `artifact_store` + 6 compose-output preview blocks + WrenAI 5-stage pipeline detail. 2 parallel Explore agents read waoowaoo+ArcReel (combined) and Polars. **Key zero-foreign-identities adaptation**: ArcReel uses Claude SDK directly; Nom MUST NOT — instead use `nom-intent` ReAct agents + `MediaVendor` facades with format translation at boundary. Phase 4 100%. **iter-6 fix-wave STILL PENDING EXECUTOR** (4 iterations stalled: missing `order` field, missing `transformation`, missing 20-frame overflow guard, dead-code `context.rs`). This blocks wave-2 regardless of how detailed the plan gets — planning has now meaningfully exceeded execution and the loop's marginal value is diminishing until Executor advances.

**Iteration 8 delta:** +5 pp (82→95% overall) — No new commits (HEAD still `205aea9`; iter-6 fix-wave pending). Re-read blueprint §8 + §9 + §10 + §11 first (standing rule); surfaced 6 module gaps in my earlier decompositions: `nom-gpui/animation.rs`, `nom-editor/{input,completion}.rs`, `nom-panels/properties.rs`, `nom-theme/{fonts,icons}.rs` — added as Phase 1/2/3 addenda. Then 3 parallel Explore agents deep-read: (1) yara-x → sealed trait via supertrait binding, runtime `Vec<Box<dyn Rule>>` registration, byte-offset `Span`, Fix struct for auto-fix; (2) Huly → Hocuspocus+Yjs architecture, ~22-service inventory with 4-service minimal-collab core (collaborator + presence + account + server); (3) typst comemo + OpenTelemetry Rust SDK → `Tracked<T>`+`Constraint::validate()` reference-equality memoization, W3C traceparent + ParentBased(TraceIdRatioBased(0.01)) sampling. Phase 5 decomposed 15→90% with ~35 subtasks across 5 new crates (nom-lint + nom-collab + nom-memoize + nom-telemetry + file-watcher) including port-comemo-without-the-crate-dep + reimplement-Hocuspocus-in-Rust as explicit SKIP-dependencies directives.

**Iteration 7 delta:** +8 pp — No new commits (HEAD still `205aea9`; iter-6 fix-wave pending). 3 parallel Explore agents deep-read: (1) ComfyUI execution.py + graph.py + caching.py → Kahn topo-sort with lazy cycle detection + IS_CHANGED contract + 4 cache strategies (None/Lru/RamPressure/Classic) + hierarchical subcache + cooperative cancellation; (2) n8n workflow-execute.ts + expression-sandboxing.ts → pull-based stack exec + retry/continueOnFail + isolated-vm sandbox with ThisSanitizer + PrototypeSanitizer + DollarSignValidator (critical for `.nom` script safety, all 3 ported verbatim); (3) typst + Remotion → Tracked<dyn World> memoization + Frame/FrameItem Arc<LazyHash> + rayon-parallel layout + GPU scene→frame→FFmpeg pipe pattern. Phase 4 decomposed 20→90% (~60 subtasks across nom-graph-v2 + nom-compose + 7 backends + shared AST sandbox + 14 test targets including security-focused sandbox-escape tests). Plan overall: 82→90%.

**Iteration 6 delta:** 0 pp (held at 82%) — Commit `205aea9` (Executor batch-2 wave-1, 44→59 tests). 6 parallel agents audited against iter-5 spec + blueprint. **4 CRITICAL defects block wave-2 start**: (1) `QuadInstance`+`MonoSpriteInstance` missing `order` field (breaks Z-sorted rendering); (2) `MonoSpriteInstance` missing `transformation` field (no rotated glyphs); (3) 20-frame overflow guard completely UNIMPLEMENTED (spec-mandated safety); (4) `context.rs` dead code — `pub mod context;` missing from lib.rs. 7 HIGH items: 4-file shader split vs 1-file spec, missing GammaParams binding, `recover()` Arc-staleness, min_binding_size None, FRAGMENT-only texture visibility, missing hsla_to_rgba, clip_bounds vs content_mask naming. **Security + blueprint conformance: CLEAN** (0 unsafe, 0 wrappers, 0 foreign identifiers, thread-model + wasm + compiler-linkage all READY). **Test quality grade: D+** — strong on pure-math buffer helpers, zero coverage on shader compat / SDF boundaries / NDC / unit-quad / 20-frame guard; 1 tautological test; 3 silent-skip tests. Plan stays at 82% because wave-1 didn't ADD plan detail — it generated a fix-wave requirement list that the Executor must address before wave-2 can start.
