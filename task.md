# Nom — Task Execution Checklist

> **Date:** 2026-04-18 | **State:** fresh build — 2 of 14 crates implemented, 12 are 1-line stubs
> **Sibling docs:** `implementation_plan.md` · `nom_state_machine_report.md` · `docs/superpowers/specs/2026-04-17-nomcanvas-gpui-design.md` · `INIT.md`
> **nom-compiler:** 29 crates UNCHANGED — direct workspace deps for everything below
> **Architecture:** DB IS workflow engine · nom-compiler IS the IDE · Canvas = AFFiNE RAG · Doc = Zed+Rowboat+AFFiNE · GPUI fully Rust
> **Reference repos:** ALL read end-to-end. Exact patterns catalogued per wave below.

## Audit-Corrected Wave Status (2026-04-18 Iteration 31 — CRITICAL BACKLOG CLEARED ✅)

**Cron fire #7 / Audit #7.** Executor finally closed all 4 CRITICALs in a single surgical session (+72 LOC across 5 files):

- ✅ **C1 (Iter 25/26/28/29/30 unfixed) — nom-theme spec-named consts**: 22 of 25 present at exact spec values + `[f32;4]` format for colors. Remaining: `H1_SPACING` missing; `ICON`/`ANIM_DEFAULT`/`ANIM_FAST` need aliasing (impl has `ICON_SIZE`/`ANIM_*_MS`).
- ✅ **C2 — H1 weight**: `heading1()` now uses `fonts.inter_bold` (weight 700).
- ✅ **C3 — nom-gpui spring math**: replaced with exact underdamped formula `1 - e^(-zeta*omega*t) * (cos(omega_d*t) + (zeta*omega/omega_d)*sin(omega_d*t))` + critically-damped branch.
- ✅ **C4 — nom-editor display_map folds**: new `fold_text()` with U+2026 placeholder; `buffer_to_display` applies it.

**Iter 26 MEDIUMs also resolved:** AFFiNE flavours 15/15 (`affine:surface`+`affine:note` added); `#[allow(private_bounds)]` added to sealed validator trait.

**Remaining OPEN:** Wave C bridge still fails `cargo check -p nom-compiler-bridge --features compiler` with 21 errors (unchanged from Iter 29/30). Fix set: 4 adapter signature updates — 30 min work.

## Audit-Corrected Wave Status (2026-04-18 Iteration 30 — NO-FIX)

**Cron fire #6 / Audit #6.** No material code changes since Iter 29. `cargo check -p nom-compiler-bridge --features compiler` still fails with **21 errors** — verbatim error codes: E0432 (×2), E0433, E0425 (×2), E0061 (×2), E0308 (×4), E0282, E0609 (×8 for `TokenStream.tok/.span`). Priority list below is unchanged from Iter 29.

## Audit-Corrected Wave Status (2026-04-18 Iteration 29)

| Wave | Planned | Actual | Evidence |
|---|---|---|---|
| Wave 0 Bootstrap | 100% | 100% ✅ | |
| Wave A GPUI substrate | 100% | ~85% | nom-gpui DRIFT (spring math broken 4 iterations); nom-canvas-core PASS; nom-theme 25 spec constants still missing |
| Wave B Editor + Blocks | 100% | ~80% | All 16 editor modules present; display_map folds ignored (4 iter unfixed) |
| Wave C Compiler bridge | 100% | **~15% structural / ~5% real** | 10 files/587 LOC landed, but **3 of 4 adapters won't compile with `--features compiler`** — `stage1_tokenize` Result not unwrapped, `find_entities_by_prefix` doesn't exist, `score_atom` arg types wrong. Only `sqlite_dict.rs` actually works. `do_deep_think` is a canned 3-step stub. |
| Wave D Shell | 100% | 0% | nom-panels still 1 line |
| Wave E Compose backends | 100% | ~15% | nom-graph + nom-memoize scaffolded, nom-compose still 1 line |
| Wave F RAG + deep-think | 100% | 0% | blocked on real `deep_think` + Wave C bridge |

### ⛔ CRITICAL Iter 29 finding (new)

**Bridge has never been build-tested with `cargo check -p nom-compiler-bridge --features compiler`.** Three adapter files reference nonexistent or mis-typed nom-compiler functions. Suggest adding a CI step:

```bash
cd nom-canvas && cargo check --workspace --features compiler 2>&1
```

Today this fails with compile errors in highlight.rs, completion.rs, score.rs. See `nom_state_machine_report.md` Iteration 29 findings X1-X4.

### CRITICALs from Iter 25/26/28 now 5 iterations unfixed — HARD FREEZE recommended

The 4 items below are verified unfixed every audit cycle for 5 iterations. No new crate should land until these close.

## Audit-Corrected Wave Status (2026-04-18 Iteration 26)

| Wave | Planned | Actual | Evidence |
|---|---|---|---|
| Wave 0 Bootstrap | 100% | **100%** ✅ | Cargo.toml lists 14 crates, `cargo check` passes, path deps resolve |
| Wave A GPUI substrate | 100% | **~85%** | nom-gpui 2,411 LOC (DRIFT — spring math still broken, Iter 24/25/26 unfixed); nom-canvas-core 1,582 LOC (PASS); **nom-theme 884 LOC with naming drift — spec §7 names still missing (Iter 25/26 unfixed)** |
| Wave B Editor + Blocks | 100% | **~80%** | nom-blocks 1,194 LOC PASS mandates ✅; **nom-editor 818 LOC — ALL 16 spec modules now present** but 1 FAIL (display_map folds ignored, Iter 25/26 unfixed) + 3 DRIFT (find_replace dead flags, commands no context, scroll no anchor) |
| Wave C Compiler bridge | 100% | **0%** | `nom-compiler-bridge/src/lib.rs` still 1 line; path deps present; 3 tier modules + 4 adapters missing |
| Wave D Shell | 100% | **0%** | nom-panels still 1 line — no Dock, Panel, PaneGroup, Shell |
| Wave E Compose backends | 100% | **~15%** | **nom-graph 507 LOC NEW** (Kahn DAG + 4-tier cache + IS_CHANGED — all PASS, but cache key missing ancestry + planner hardcodes input_hash=0); **nom-memoize 324 LOC NEW** (HIGH drift — tracks version not per-method hash; uses FNV-1a not SipHash13); nom-compose still 1 line; 16 backends unstarted |
| Wave F RAG + deep-think | 100% | **0%** | depends on Wave C bridge + GAP-5 deep_think() |

**Core mandates (Iter 25):** NomtuRef non-optional ✅ · can_wire non-optional ✅ · No hardcoded node enum ✅ · DictReader trait isolation ✅ · Cross-workspace path deps ✅ · DB-driven invariant ✅

**⚠️ STOP adding new code. Fix these CRITICALs from prior iterations FIRST (Iter 25 + Iter 26 flagged):**

1. **`nom-theme/src/tokens.rs`** — add spec §7 named constants (25 names): `SIDEBAR_W=248.0`, `TOOLBAR_H=48.0`, `STATUSBAR_H=24.0`, `BLOCK_RADIUS=4.0`, `MODAL_RADIUS=22.0`, `POPOVER_RADIUS=12.0`, `BTN_H=28.0`, `BTN_H_LG=32.0`, `BTN_H_XL=40.0`, `ICON=24.0`, `H1_WEIGHT=700`, `H1_SPACING=-0.02`, `H2_WEIGHT=600`, `BODY_WEIGHT=400`, `BG=[0.059,0.090,0.165,1.0]`, `BG2=[0.118,0.161,0.251,1.0]`, `TEXT=[0.973,0.980,0.988,1.0]`, `CTA=[0.133,0.773,0.369,1.0]`, `BORDER=[0.200,0.255,0.333,1.0]`, `FOCUS=[0.118,0.588,0.922,0.3]`, `EDGE_HIGH=[0.133,0.773,0.369,0.9]`, `EDGE_MED=[0.957,0.702,0.078,0.7]`, `EDGE_LOW=[0.937,0.267,0.267,0.5]`, `ANIM_DEFAULT=300.0`, `ANIM_FAST=200.0`. Keep existing semantic names but ADD these as exports. **[ITER 25 + ITER 26 BOTH FLAGGED — still unfixed]**
2. **`nom-theme/src/fonts.rs:84-92`** — change `fonts.inter_semibold` → `fonts.inter_bold` for H1 (spec requires weight 700, not 600). **[ITER 25 + ITER 26 BOTH FLAGGED — still unfixed]**
3. **`nom-gpui/src/animation.rs:96-102`** — replace spring math with proper underdamped form: `y(t) = 1 - e^(-zeta*omega*t) * (cos(omega_d*t) + (zeta*omega/omega_d)*sin(omega_d*t))` where `omega_d = omega*sqrt(1-zeta^2)`. **[ITER 24 + 25 + 26 ALL FLAGGED — 3 iterations unfixed]**
4. **`nom-editor/src/display_map.rs:32-43` `buffer_to_display`** — currently iterates chars ignoring stored `FoldRegion` list. Apply folds by sorting fold vec by start, emitting placeholder chars when offset enters a fold range, and skipping to `fold.buffer_range.end`. **[ITER 25 + 26 BOTH FLAGGED — still unfixed]**
5. ~~`nom-editor/src/line_layout.rs`~~ — ✅ DONE (Iter 26). PASS.
6. ~~`nom-editor` missing modules~~ — ✅ DONE (Iter 26). All 8 modules landed; 3 have DRIFT (find_replace dead flags, commands no context, scroll no anchor).

### New HIGH findings from Iter 26 (fix after CRITICAL 1-4):

- **`nom-editor/src/find_replace.rs:7-21`** — `use_regex` and `whole_word` flags stored at line 4 but **NEVER READ** in `find_in_text`. Either wire them (add regex crate + word-boundary `\b` wrapping) or remove the fields.
- **`nom-editor/src/commands.rs:5`** — `CommandFn = Box<dyn Fn()>` takes no context. Change to `Box<dyn Fn(&mut EditorContext)>` or `Box<dyn Fn(&mut dyn Any)>` to match Zed `register_action!()` pattern.
- **`nom-memoize/src/tracked.rs:8-14`** — `Tracked<T>` records `(version, access_count)` only. Must record per-method `(method_id, return_value_hash)` pairs to implement comemo's "re-run only if methods you read changed" invariant.
- **`nom-memoize/src/constraint.rs:27`** — `validate()` compares versions, not return-value hashes. Make it replay recorded `(method_id, return_hash)` pairs against current values.
- **`nom-memoize/src/hash.rs:4,12`** — FNV-1a 128-bit → replace with SipHash13 128-bit via `siphasher::sip128::SipHasher13`. Spec-mandated algorithm.
- **`nom-graph/src/execution.rs:28-33`** — `compute_cache_key` missing IS_CHANGED result + ancestor hashes. Replace with `to_hashable([class_type, is_changed_result, sorted_inputs_with_ancestor_indices])` matching ComfyUI `caching.py:101-127`.
- **`nom-graph/src/execution.rs:49-54`** — `plan_execution` hardcodes `input_hash=0`. Compute real hash from upstream outputs. Build `VariablePool { outputs: HashMap<NodeId, HashMap<OutputName, Value>> }`. Implement actual execution loop (not just planning).
7. **`nom-blocks/src/validators.rs`** — add `#[allow(private_bounds)]` to sealed trait pattern (will warn on new Rust editions).
8. **`nom-blocks/src/prose.rs`** — add 2 missing flavours to reach 15: `affine:surface` + `affine:note`.
9. **`nom-canvas-core/src/snapping.rs:8`** — change `GRID_SIZE = 24.0` → `20.0` (match excalidraw reference).
10. **`nom-gpui/src/scene.rs`** — add `order: DrawOrder` field to Quad/MonochromeSprite/PolychromeSprite/Path/Shadow/Underline; make `sort_and_batch()` sort by order (currently sorts shadows by constant `0u8` — no-op).

**Next wave to unblock (Wave C Compiler bridge):**
- `nom-compiler-bridge/src/shared.rs` — `SharedState` with dict_pool + grammar_conn + LRU cache
- `nom-compiler-bridge/src/ui_tier.rs` — sync reads
- `nom-compiler-bridge/src/interactive_tier.rs` — tokio mpsc tokenize/highlight
- `nom-compiler-bridge/src/background_tier.rs` — crossbeam workers compile/plan/verify/deep_think
- `nom-compiler-bridge/src/adapters/highlight.rs` — `stage1_tokenize` → `TokenRole` mapping (~200 LOC, first user-visible wire)
- `nom-compiler-bridge/src/adapters/lsp.rs` / `completion.rs` / `score.rs`
- `nom-compiler-bridge` replaces `StubDictReader` with `SqliteDictReader` wrapping `SharedState.dict_pool`

## Non-Negotiable Rules (apply to EVERY task below)

1. **Read source repos end-to-end** before writing ANY code that borrows their pattern
2. **Use `ui-ux-pro-max` skill** at `.agent/skills/ui-ux-pro-max/` for ALL UI work (Wave A `nom-theme`, Wave D panels/dock, Wave F graph visuals)
3. **Zero foreign identities** in public API — descriptive names only
4. **nom-compiler is CORE** — direct workspace path deps, zero IPC, zero subprocesses
5. **DB IS the workflow engine** — never introduce an external orchestrator (no n8n, no Dify runtime, no BullMQ server)
6. **Every canvas object = DB entry** — `entity: NomtuRef` is NON-OPTIONAL on every block/node/connector from day 1
7. **Canvas = AFFiNE for RAG** — frosted glass, confidence-scored bezier edges, RAG overlay
8. **Doc mode = Zed + Rowboat + AFFiNE** — all three, not just one
9. **Deep thinking = compiler op** — `nom-intent::deep_think()`, streamed to right dock via bridge
10. **GPUI fully Rust — one binary** — no webview, no Electron, no Tauri, no DOM anywhere
11. **Spawn parallel subagents for all multi-file work** — main conversation coordinates, does not serialize file reads
12. **Run `gitnexus_impact` before editing any symbol** — never ignore HIGH/CRITICAL warnings

### Wave dependency graph

```
Wave 0 Bootstrap
   ↓
Wave A (nom-gpui · nom-canvas-core · nom-theme)
   ↓
Wave B (nom-editor · nom-blocks with NomtuRef non-optional + DictReader trait)
   ↓ ← [shared_types: DeepThinkStep/DeepThinkEvent/RunEvent/CompositionPlan defined here]
Wave C (nom-compiler-bridge · SqliteDictReader replaces StubDictReader · can_wire real impl)
   ↓
Wave D (nom-panels · left AFFiNE dock · right Rowboat dock · bottom · multi-window)
   ↓
Wave E (nom-compose · nom-graph · nom-memoize · 16 backends)
   ↓
Wave F (AFFiNE graph RAG · deep_think UI streaming)

Parallel track: Compiler Remaining (GAP-1c → GAP-5 + bootstrap fixpoint proof)
```

---

## Wave 0 — Workspace Bootstrap (prerequisite for all waves)

*Must complete before any Wave A code is written. Sets up the Cargo workspace shell.*

### nom-canvas workspace

- [x] Create `nom-canvas/Cargo.toml` — workspace manifest:
  ```toml
  [workspace]
  resolver = "2"
  members = [
    "crates/nom-gpui",
    "crates/nom-canvas-core",
    "crates/nom-theme",
    "crates/nom-editor",
    "crates/nom-blocks",
    "crates/nom-compiler-bridge",
    "crates/nom-panels",
    "crates/nom-compose",
    "crates/nom-graph",        # DAG execution engine (ComfyUI patterns) — peer of nom-compose
    "crates/nom-memoize",      # typst comemo-pattern incremental compute (Wave E-1 prereq)
    "crates/nom-telemetry",    # metrics/logging spine (Wave D status bar consumer)
    "crates/nom-collab",       # stub crate for future CRDT/yrs work (kept empty, reserved)
    "crates/nom-lint",
    "crates/nom-cli",
  ]

  [workspace.dependencies]
  # GPU stack
  wgpu = "0.19"
  winit = "0.29"
  taffy = "0.4"
  cosmic-text = "0.11"
  etagere = "0.2"
  # Data / storage
  rusqlite = { version = "0.31", features = ["bundled"] }
  sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio-rustls"] }
  # Async
  tokio = { version = "1", features = ["full"] }
  crossbeam-channel = "0.5"
  # Utilities
  bytemuck = { version = "1", features = ["derive"] }
  ropey = "1.6"
  rstar = "0.12"
  lru = "0.12"
  serde = { version = "1", features = ["derive"] }
  serde_json = "1"

  [workspace.lints.rust]
  unsafe_code = "deny"
  ```

- [x] Create `nom-canvas/rust-toolchain.toml`:
  ```toml
  [toolchain]
  channel = "stable"
  components = ["rustfmt", "clippy"]
  ```

- [x] Create stub `Cargo.toml` for each of the 14 crates (name only, deps filled per wave):
  - `crates/nom-gpui/Cargo.toml` — `name = "nom-gpui"`
  - `crates/nom-canvas-core/Cargo.toml` — `name = "nom-canvas-core"`
  - `crates/nom-theme/Cargo.toml` — `name = "nom-theme"`
  - `crates/nom-editor/Cargo.toml` — `name = "nom-editor"`
  - `crates/nom-blocks/Cargo.toml` — `name = "nom-blocks"`
  - `crates/nom-compiler-bridge/Cargo.toml` — `name = "nom-compiler-bridge"`, feature gate `compiler`
  - `crates/nom-panels/Cargo.toml` — `name = "nom-panels"`
  - `crates/nom-compose/Cargo.toml` — `name = "nom-compose"` (vendor facade, ArtifactStore, ProgressSink, FormatTranslator)
  - `crates/nom-graph/Cargo.toml` — `name = "nom-graph"` (DAG engine + 4-tier cache + Kahn sort + IS_CHANGED + VariablePool)
  - `crates/nom-memoize/Cargo.toml` — `name = "nom-memoize"` (typst-style `Tracked<T>` + `Constraint::new()` + `validate()` + `hash128`)
  - `crates/nom-telemetry/Cargo.toml` — `name = "nom-telemetry"` (metrics sink for compile status, compose progress, deep-think step count)
  - `crates/nom-collab/Cargo.toml` — `name = "nom-collab"` (empty stub; reserved for future multi-user collab; do NOT add yrs yet per spec §5)
  - `crates/nom-lint/Cargo.toml` — `name = "nom-lint"`
  - `crates/nom-cli/Cargo.toml` — `name = "nom-cli"`, `[[bin]] name = "nom-canvas"`

- [x] Create `crates/nom-compiler-bridge/Cargo.toml` with cross-workspace path deps:
  ```toml
  [features]
  default = []
  compiler = ["nom-concept", "nom-dict", "nom-grammar", "nom-score", "nom-search"]

  [dependencies]
  nom-concept  = { path = "../../../nom-compiler/crates/nom-concept",  optional = true }
  nom-dict     = { path = "../../../nom-compiler/crates/nom-dict",     optional = true }
  nom-grammar  = { path = "../../../nom-compiler/crates/nom-grammar",  optional = true }
  nom-score    = { path = "../../../nom-compiler/crates/nom-score",    optional = true }
  nom-search   = { path = "../../../nom-compiler/crates/nom-search",   optional = true }
  ```
  Risk fallback: if `nom-concept` pulls too many deps → extract `nom-concept-core` (lex + `Tok`/`Spanned` only, ~200 LOC, no DB deps)

- [x] Verify `cargo check -p nom-canvas-core` passes with empty lib stubs (smoke test workspace is wired correctly)
- [x] Add `nom-canvas/` to root `.gitignore` exceptions (not ignored), add to root `Cargo.toml` if using a root virtual workspace

---

## Wave A — GPUI Substrate + Basic Canvas

### nom-gpui (GPU framework)
*Pattern source: `APP/zed-main/crates/gpui/` — read Scene, primitives, atlas, Element, window end-to-end*

- [x] `scene.rs` — 6 GPU primitives matching Zed exactly:
  - `Quad { bounds: Bounds<Pixels>, background: Option<Hsla>, border_color: Option<Hsla>, border_widths: Edges<Pixels>, corner_radii: Corners<Pixels>, content_mask: ContentMask<Pixels> }`
  - `MonochromeSprite { bounds, content_mask, color: Hsla, tile: AtlasTile, transformation: TransformationMatrix }`
  - `PolychromeSprite { bounds, content_mask, corner_radii, tile: AtlasTile, grayscale: bool }`
  - `Path<Pixels> { bounds, color: Hsla, vertices: Vec<PathVertex<Pixels>>, content_mask }`
  - `Shadow { bounds, corner_radii, blur_radius, color: Hsla, content_mask }`
  - `Underline { origin, width, thickness, color: Option<Hsla>, wavy: bool, content_mask }`
  - `Scene::sort_and_batch()` — stacking context sort → GPU submission order

- [x] `renderer.rs` — 8 wgpu render pipelines (one per primitive), `draw(&scene)`, quad pipeline uses instanced draw with per-quad uniform buffer, depth-less painters algorithm

- [x] `atlas.rs` — glyph atlas following Zed TextureAtlas pattern:
  - `BucketedAtlasAllocator` from etagere crate
  - 4×4 subpixel positioning variants per glyph
  - `AtlasTile { texture_id, bounds: AtlasBounds, padding }` return type
  - cosmic-text `Buffer` → rasterize → etagere alloc → wgpu texture upload
  - LRU eviction when atlas full

- [x] `element.rs` — `Element` trait with exact Zed signatures:
  - `fn request_layout(&mut self, global_id: Option<&GlobalElementId>, cx: &mut WindowContext) -> (LayoutId, Self::State)`
  - `fn prepaint(&mut self, global_id: Option<&GlobalElementId>, bounds: Bounds<Pixels>, state: &mut Self::State, cx: &mut WindowContext)`
  - `fn paint(&mut self, global_id: Option<&GlobalElementId>, bounds: Bounds<Pixels>, state: &mut Self::State, cx: &mut WindowContext)`
  - `GlobalElementId` = stack of `ElementId` for stable identity across repaints

- [x] `styled.rs` — fluent style builder following Zed `Styled` trait:
  - `fn style(&mut self) -> StyleRefinement` returning mutable ref
  - Builder methods: `bg()`, `border()`, `rounded()`, `p()`, `m()`, `text_color()`, `shadow()`, `opacity()`, `overflow_hidden()`
  - `StyleRefinement` merges into base `Style` at layout time

- [x] `layout.rs` — taffy wrapper:
  - `LayoutId` is newtype over `taffy::NodeId`
  - `request_layout(style: taffy::Style, children: &[LayoutId]) -> LayoutId`
  - `layout(id: LayoutId) -> taffy::Layout` for computed bounds
  - `remove_layout_id(id: LayoutId)` on drop

- [x] `window.rs` — winit integration:
  - `ApplicationHandler` impl: `resumed`, `window_event`, `about_to_wait`
  - Frame loop: poll `EventLoop` → accumulate events → request redraw → render
  - Device-lost recovery: rebuild swapchain + re-upload atlas
  - `WindowOptions { title, size, min_size, decorations, transparent }`

- [x] `animation.rs` — Zed animation pattern:
  - `Animation { keyframes: Vec<(f32, T)>, easing: EasingFn, duration: Duration }`
  - `EasingFn` variants: `Linear`, `EaseIn`, `EaseOut`, `EaseInOut`, cubic-bezier
  - Transition system: element state interpolation on style changes
  - Animate on `cx.request_animation_frame()` hook

- [x] `platform.rs` — desktop vs WebGPU split:
  - `Platform` trait: `create_surface`, `get_adapter_options`, `present_mode`
  - `DesktopPlatform` (wgpu native surface, winit)
  - `WebPlatform` (wgpu WebGPU via wasm-bindgen, web_sys canvas)
  - Feature gate: `#[cfg(target_arch = "wasm32")]` / `#[cfg(not(target_arch = "wasm32"))]`

- [x] Pointer events + focus:
  - `MouseDown`, `MouseUp`, `MouseMove`, `MouseEnter`, `MouseLeave`, `Scroll` event types
  - `FocusHandle` = `Arc<AtomicUsize>` focus-id + window-level focus map
  - `cx.focus(&handle)` / `cx.is_focused(&handle)` / `handle.dispatch_action()`
  - Tab-order traversal matching Zed's `FocusableView`

### nom-canvas-core (Infinite canvas engine)

- [x] `viewport.rs`:
  - `Viewport { zoom: f32, pan: Vec2, size: Vec2 }` — zoom 0.1×–32×
  - `screen_to_canvas(pt: Vec2) -> Vec2` = `(pt - size/2 - pan) / zoom`
  - `canvas_to_screen(pt: Vec2) -> Vec2` = `pt * zoom + pan + size/2`
  - `visible_bounds() -> Bounds<f32>` for culling
  - Pinch-to-zoom: accumulate touch events → smooth zoom toward cursor

- [x] `elements.rs` — shape primitives:
  - `CanvasRect { bounds, fill, stroke, corner_radius, rotation }`
  - `CanvasEllipse { bounds, fill, stroke }`
  - `CanvasLine { start, end, stroke_width, color, dashes }`
  - `CanvasArrow { line, head_style: ArrowHead }` (open/closed/filled)
  - `CanvasConnector { src_id, dst_id, route: Vec<Vec2>, confidence: f32, reason: String }` (replaces Arrow for typed edges)

- [x] `selection.rs`:
  - `Selection { ids: BTreeSet<ElementId>, transform_origin: Vec2 }`
  - Rubber-band: track drag rect → test against AABB → finalize on mouse-up
  - 8 resize handles + 1 rotate handle per selection
  - Transform: translate/scale/rotate with snapping constraints applied

- [x] `snapping.rs`:
  - Grid snap: `round_to_grid(pt, grid_size: f32)`
  - Edge snap: find elements within snap_radius → emit guide lines
  - Center snap: snap moving element center to other elements' centers
  - `SnapGuide { axis: Axis, position: f32, color: Hsla }` rendered as overlay

- [x] `hit_test.rs`:
  - Phase 1: AABB broadphase — check `bounds.contains(pt)`
  - Phase 2: precise — connector uses `dist_to_bezier(pt, ctrl_pts) < hit_radius`
  - Hit order: topmost (highest z-index) wins
  - `HitResult { element_id, hit_type: HitType }` where `HitType = Body | Handle(u8) | Connector`

- [x] `spatial_index.rs`:
  - R-tree via `rstar` crate: `RTree<CanvasElementEnvelope>`
  - `CanvasElementEnvelope` implements `RTreeObject` with `Envelope = AABB<[f32; 2]>`
  - `query_in_bounds(bounds) -> Vec<ElementId>` for O(log n) region lookup
  - Incremental update: remove old envelope + insert new on element move

### nom-theme (Design system)
*Pattern source: `APP/AFFiNE-canary/` — read packages/theme end-to-end*

- [x] `tokens.rs` — 73 AFFiNE design tokens + Nom extensions:
  - **Spacing scale:** 4px base grid — spacing-1=4, spacing-2=8, spacing-3=12, spacing-4=16, spacing-6=24, spacing-8=32, spacing-12=48
  - **Radius scale:** radius-none=0, radius-sm=4, radius-md=8, radius-lg=12, radius-xl=16, radius-full=9999
  - **Elevation (shadows):** shadow-sm (0 1px 2px), shadow-md (0 4px 8px), shadow-lg (0 8px 24px), shadow-xl (0 16px 48px)
  - **Frosted glass:** `backdrop_blur: 12px`, `background_alpha: 0.85`, `border: 1px solid rgba(255,255,255,0.12)`
  - **Typography:** Inter 14/1.5 body, Inter 12/1.4 caption, Inter 24/1.2 h1, SCP 13/1.6 code
  - **Graph edge confidence colors (matches NORTH STAR spec §Design Tokens — Tailwind palette):** green `#22C55E` / rgb(34,197,94) (≥0.8), amber `#F59E0B` / rgb(245,158,11) (0.5–0.8), red `#EF4444` / rgb(239,68,68) (<0.5), opacity = confidence value
  - **Motion:** spring(stiffness=400, damping=28) for connect; ease-out 120ms for hover; ease-in-out 200ms for panel resize

- [x] `fonts.rs`:
  - `FontRegistry { inter: FontId, source_code_pro: FontId }`
  - Load from embedded bytes (`include_bytes!`) → cosmic-text `FontSystem::db_mut().load_font_data()`
  - Weight variants: Inter 400/500/600/700, SCP 400/600
  - `resolve_font(family, weight, style) -> cosmic_text::Attrs`

- [x] `icons.rs`:
  - 42 Lucide icons at 24px, compiled to GPU path vertex data
  - `Icon` enum: `ChevronRight`, `ChevronDown`, `Plus`, `Minus`, `X`, `Search`, `Settings`, `Brain`, `Network`, `File`, `Folder`, `Play`, `Pause`, `Stop`, `Zap`, `Link`, `Unlink`, `Lock`, `Unlock`, `Eye`, `EyeOff`, `Copy`, `Trash`, `Edit2`, `Check`, `AlertCircle`, `Info`, `Terminal`, `Code`, `Database`, `Layers`, `Grid`, `List`, `Sidebar`, `PanelLeft`, `PanelRight`, `MessageSquare`, `Tool`, `Cpu`, `GitBranch`, `Sparkles`, `Workflow`
  - `render_icon(icon: Icon, color: Hsla, size: f32) -> Vec<PathVertex<Pixels>>`

---

## Wave B — Editor + Nomtu-Backed Blocks

### nom-editor (Zed quality)
*Pattern source: `APP/zed-main/crates/editor/` — rope buffer, display pipeline, LSP consumer*

- [x] `buffer.rs`:
  - `Buffer { rope: Rope, version: clock::Global, file: Option<Arc<dyn File>> }` — ropey crate
  - `edit(range: Range<usize>, new_text: &str) -> Patch` — atomic, reverse-offset safe
  - `text_for_range(range: Range<usize>) -> Cow<str>` — zero-copy for small ranges
  - Transaction batching: `start_transaction()` / `end_transaction()` for multi-edit undo

- [x] `cursor.rs`:
  - `Selection { start: Anchor, end: Anchor, goal_column: Option<u32>, reversed: bool }`
  - `Anchor { buffer_id: BufferId, excerpt_id: ExcerptId, offset: usize, bias: Bias }`
  - `CursorSet { selections: Vec<Selection> }` — disjoint, merged on overlap
  - Pending selection during mouse drag; committed on mouse-up

- [x] `highlight.rs`:
  - `HighlightSpan { range: Range<usize>, token_role: TokenRole }`
  - `TokenRole` enum: `Keyword`, `Identifier`, `Literal`, `Operator`, `Comment`, `NomtuRef`, `ClauseConnective`, `Unknown`
  - `Highlighter::color_runs(spans: &[HighlightSpan]) -> Vec<(Range<usize>, Hsla)>` — consumer only (Wave C produces spans)
  - Color map from `nom-theme` tokens: Keyword=accent-blue, NomtuRef=accent-purple, Literal=accent-green

- [x] `input.rs`:
  - Keyboard event → `KeyAction` dispatch via action registry
  - IME: `compositionstart` / `compositionupdate` / `compositionend` handling
  - Key bindings: Ctrl+Z undo, Ctrl+Y redo, Ctrl+D duplicate cursor, Ctrl+/ toggle comment

- [x] Display pipeline (Zed pattern, exact module chain):
  - `display_map.rs` — maps buffer offsets to display rows, handles folds/excerpts
  - `wrap_map.rs` — soft-wrap at column width, tracks wrap points
  - `tab_map.rs` — tab expansion to spaces, visual column tracking
  - `line_layout.rs` — per-display-line: `cosmic_text::Buffer` layout + glyph runs → `LineLayout { len, width, runs: Vec<LayoutRun> }`

- [x] `lsp_bridge.rs`:
  - `LspProvider` trait: `hover(&self, path, offset) -> Option<HoverResult>`, `completions(&self, path, offset) -> Vec<CompletionItem>`, `goto_definition(&self, path, offset) -> Option<Location>`
  - `StubLspProvider` — returns empty results (replaced by Wave C `CompilerLspProvider`)

- [x] `completion.rs` — skeletal consumer:
  - `CompletionMenu { items: Vec<CompletionItem>, selected: usize, trigger_pos: usize }`
  - `CompletionItem { label, kind: CompletionKind, detail: Option<String>, insert_text: String }`
  - Renders as floating overlay below cursor

- [x] `scroll.rs` — `ScrollPosition { top_anchor: Anchor, vertical_offset: f32 }`, smooth scroll with inertia
- [x] `clipboard.rs` — multi-selection copy/paste, preserves anchor ordering
- [x] `find_replace.rs` — regex search via `regex` crate, iterative match highlighting
- [x] `indent.rs` — auto-indent on newline (copy leading whitespace of previous non-blank line)
- [x] `commands.rs` — action dispatch table matching Zed's `register_action!()`

### nom-blocks (ALL nomtu-backed from day 1)
*Pattern source: AFFiNE block model + ToolJet widget patterns + yara-x sealed validator*

- [x] `block_model.rs`:
  - `NomtuRef { id: String, word: String, kind: String }` — all 3 fields REQUIRED, zero optionals
  - `BlockModel { entity: NomtuRef, flavour: &'static str, slots: Vec<(String, SlotValue)>, children: Vec<BlockId>, meta: BlockMeta }`
  - `BlockMeta { created_at, updated_at, author, version: u32 }`
  - `entity` field has no `Option<>` wrapper — blocks without a DB entity do NOT exist

- [x] `slot.rs`:
  - `SlotValue` enum: `Text(String)`, `Number(f64)`, `Bool(bool)`, `Ref(NomtuRef)`, `List(Vec<SlotValue>)`, `Blob { hash: [u8; 32], mime: String }`
  - `SlotBinding { clause_name: String, grammar_shape: String, value: SlotValue, is_required: bool, confidence: f32, reason: String }`
  - Confidence scale: 1.0=explicit user-set, 0.8=inferred from grammar, 0.6=heuristic match

- [x] `shared_types.rs` — types used across multiple waves (defined in nom-blocks so Wave D right dock can compile without waiting for Wave F):
  - `DeepThinkStep { hypothesis: String, evidence: Vec<String>, confidence: f32, counterevidence: Vec<String>, refined_from: Option<String> }` — struct shape only, populated by `nom-intent::deep_think()` in GAP-5
  - `DeepThinkEvent { Step(DeepThinkStep), Final(CompositionPlan) }` — streaming event enum
  - `CompositionPlan` — stub re-export (real definition in `nom-compiler/nom-planner`, feature-gated)
  - `RunEvent` — full 14-variant enum (see Wave D details) used by both nom-compiler-bridge and nom-panels
  - Rationale: keeps a single source of truth; avoids circular dep between nom-panels and nom-compiler-bridge

- [x] **Doc mode blocks — 15 AFFiNE flavours (exact flavour strings from AFFiNE source):**
  - `prose.rs`:
    - `affine:paragraph` — `text: Delta` (quill Delta ops)
    - `affine:heading` — `text: Delta`, `level: 1..=6`
    - `affine:list` — `text: Delta`, `type: ListType { Bulleted | Numbered | Todo | Toggle }`, `checked: Option<bool>`
    - `affine:quote` — `text: Delta`
    - `affine:divider` — no fields
    - `affine:callout` — `text: Delta`, `emoji: String`, `style: CalloutStyle`
    - `affine:database` — `title: String`, `views: Vec<DatabaseView>`, `columns: Vec<Column>`, `rows: Vec<Row>`
    - `affine:linked-doc` — `page_id: String`, `params: String` (query-string link params)
    - `affine:bookmark` — `url: String`, `title: Option<String>`, `description: Option<String>`, `favicon: Option<String>`
    - `affine:attachment` — `name: String`, `size: u64`, `blob_hash: [u8;32]`, `mime: String`
    - `affine:image` — `blob_hash: [u8;32]`, `width: Option<f32>`, `height: Option<f32>`, `caption: String`
    - `affine:code` — `language: String`, `text: Delta`, `wrap: bool` — uses nom-editor for `.nomx`
    - `affine:embed-*` — generic embed block for external URLs
  - `nomx.rs` — `affine:code` variant with `language = "nomx"`, wraps nom-editor buffer

- [x] **Canvas/Graph blocks:**
  - `graph_node.rs`:
    - `GraphNode { id: NodeId, entity: NomtuRef, production_kind: String, slots: Vec<SlotBinding>, position: Vec2, size: Vec2, collapsed: bool }`
    - `production_kind` validated against `grammar.kinds` at insert time (NOT a Rust enum)
    - Ports are `Vec<SlotBinding>` derived from `SELECT * FROM clause_shapes WHERE kind = ?`
    - Port positions: inputs on left edge, outputs on right edge, evenly spaced
    - Frosted-glass visual: `background = theme.frosted_glass`, `border = theme.border_subtle`
  - `connector.rs`:
    - `Connector { id: ConnectorId, src: (NodeId, SlotName), dst: (NodeId, SlotName), confidence: f32, reason: String, route: Vec<Vec2> }`
    - `route` = bezier control points (auto-routed, editable)
    - Confidence rendered as edge color from `nom-theme` (green/amber/red)
    - `can_wire_result: (bool, f32, String)` — **NON-OPTIONAL, every Connector MUST have a wire-check result.** In Wave B before the bridge exists, use placeholder `(true, 0.0, "stub - pending Wave C validation")`. In Wave C the real `can_wire()` populates it. Invariant: a Connector without a populated `can_wire_result` is a bug, not an intermediate state.

- [x] **Media/utility blocks:**
  - `media.rs` — `MediaBlock { entity: NomtuRef, blob_hash: [u8;32], mime: String, width: Option<u32>, height: Option<u32>, duration_ms: Option<u64> }` — no `BlobId: String`
  - `drawing.rs` — `DrawingBlock { entity: NomtuRef, strokes: Vec<Stroke> }` where `Stroke { points: Vec<Vec2>, pressure: Vec<f32>, color: Hsla, width: f32 }`
  - `table.rs` — `TableBlock { entity: NomtuRef, columns: Vec<Column>, rows: Vec<Vec<SlotValue>> }`
  - `embed.rs` — `EmbedBlock { entity: NomtuRef, url: String, embed_type: EmbedType, aspect_ratio: f32 }`

- [x] **Compose-preview blocks (6 types — explicit flavour strings):**
  - All read from `ArtifactStore` by `[u8; 32]` SHA-256 hash — NOT `BlobId: String`
  - Flavour naming convention: `nom:compose-<kind>` (parallel to AFFiNE's `affine:<kind>`)
  - `compose/video_block.rs` — flavour `"nom:compose-video"` — `VideoBlock { entity: NomtuRef, artifact_hash: [u8;32], duration_ms: u64, width: u32, height: u32, progress: Option<f32> }`
  - `compose/image_block.rs` — flavour `"nom:compose-image"` — `ImageBlock { entity: NomtuRef, artifact_hash: [u8;32], width: u32, height: u32, prompt_used: String }`
  - `compose/document_block.rs` — flavour `"nom:compose-document"` — `DocumentBlock { entity: NomtuRef, artifact_hash: [u8;32], page_count: u32, mime: String }`
  - `compose/data_block.rs` — flavour `"nom:compose-data"` — `DataBlock { entity: NomtuRef, artifact_hash: [u8;32], row_count: u64, schema: Vec<ColumnSpec> }`
  - `compose/app_block.rs` — flavour `"nom:compose-app"` — `AppBlock { entity: NomtuRef, artifact_hash: [u8;32], target_platform: String, deploy_url: Option<String> }`
  - `compose/audio_block.rs` — flavour `"nom:compose-audio"` — `AudioBlock { entity: NomtuRef, artifact_hash: [u8;32], duration_ms: u64, codec: String }`

- [x] `validators.rs` — yara-x sealed linter pattern:
  - Sealed via `#[allow(private_bounds)]` + `trait BlockValidatorInternal: sealed::Sealed {}` + blanket `impl<T: BlockValidatorInternal> BlockValidator for T {}`
  - `grammar_derivation_check(block: &BlockModel, conn: &Connection) -> Vec<ValidationError>` — queries `grammar.kinds` to confirm `entity.kind` is known
  - `slot_shape_check(slot: &SlotBinding, conn: &Connection) -> bool` — validates `grammar_shape` against `clause_shapes`
  - `ValidationError { span: Range<u32>, message: String, severity: Severity, labels: Vec<Label>, footers: Vec<String> }` matching yara-x `Report` structure — **`Span` = `Range<u32>` byte offsets (not `usize`), matching yara-x's exact type alias `type Span = Range<u32>`**

- [x] `workspace.rs`:
  - `Workspace { entities: HashMap<String, CanvasObject>, layout: SpatialIndex, doc_tree: Vec<BlockId> }`
  - `save(conn: &Connection)` = iterate entities → `upsert_entity()` in nom-dict for each NomtuRef
  - `load(conn: &Connection, query: &str) -> Self` = `SELECT * FROM entries WHERE ...` → reconstruct blocks
  - `CanvasObject` = enum over all block types — dispatches to correct deserializer

### DB wire — trait injection (no direct SQLite in nom-blocks)

- [x] `nom-blocks/src/dict_reader.rs` — define trait, NOT a concrete SQLite connection:
  ```rust
  pub trait DictReader: Send + Sync {
      fn is_known_kind(&self, kind: &str) -> bool;
      fn clause_shapes_for(&self, kind: &str) -> Vec<ClauseShape>;
      fn lookup_entity(&self, word: &str, kind: &str) -> Option<NomtuRef>;
  }
  ```
- [x] `nom-blocks/src/stub_dict.rs` — Wave B implementation: `StubDictReader { known_kinds: HashSet<String>, seed_shapes: HashMap<String, Vec<ClauseShape>> }` loaded from a static TOML fixture; lets Wave B tests run with no DB
- [x] Wave C replaces the injection: `nom-compiler-bridge` provides `SqliteDictReader` which is backed by the `SharedState` dict pool (SINGLE connection owner; no parallel SQLite handles to the same file)
- [x] `BlockModel::insert(dict: &dyn DictReader, ...)` — calls `dict.is_known_kind(kind)`; panics in debug on unknown kind, logs error in release
- [x] Rationale: avoids stale reads from two independent SQLite handles; Wave B never opens the DB directly; the bridge owns the only writer pool

---

## Wave C — nom-compiler-bridge (KEYSTONE)

*Pattern source: GitNexus schema (31 node tables, 20 edge types) + dify event-generator + n8n sandbox*

- [ ] Create `nom-canvas/crates/nom-compiler-bridge/` with `feature = "compiler"` gate
- [ ] Cargo path deps (feature-gated): `nom-concept`, `nom-dict`, `nom-grammar`, `nom-score`, `nom-search` → `../../../nom-compiler/crates/*`
  - Risk fallback: `nom-concept-core` = lex module only (~200 LOC, `Tok`, `Spanned`, `stage1_tokenize`)

- [ ] `shared.rs` — `SharedState`:
  - `dict_pool: SqlitePool` — 1 write connection + N read connections (WAL mode), `pool_size = num_cpus + 4`
  - `grammar_conn: Arc<Mutex<Connection>>` — read-only grammar DB
  - `compile_cache: LruCache<u64, PipelineOutput>` — key = SipHash13 of (source_text, grammar_version)
  - `bm25: BM25Index` — in-memory BM25 loaded from dict on startup

- [ ] `ui_tier.rs` — synchronous, must complete in <2ms (called on UI thread):
  - `lookup_nomtu(word: &str) -> Option<NomtuRef>` — BM25 exact match first, fuzzy fallback
  - `score_atom(word: &str, kind: &str) -> f32` — wraps `nom-score::score_atom`
  - `can_wire(src_kind: &str, src_slot: &str, dst_kind: &str, dst_slot: &str) -> (bool, f32, String)` — pure, no DB (uses preloaded grammar)
  - `grammar_keywords() -> Vec<String>` — cached `SELECT name FROM kinds ORDER BY name`
  - `search_bm25(query: &str, limit: usize) -> Vec<(NomtuRef, f32)>` — BM25Index::search

- [ ] `interactive_tier.rs` — async, tokio mpsc, <100ms budget:
  - `tokenize(source: &str) -> Vec<Spanned<Tok>>` — `stage1_tokenize` off UI thread
  - `highlight_spans(source: &str) -> Vec<HighlightSpan>` — tokenize → adapter → spans
  - `complete_prefix(prefix: &str, kind_filter: Option<&str>) -> Vec<CompletionItem>` — nom-dict prefix search
  - `hover(word: &str) -> Option<HoverResult>` — dict lookup + grammar annotation

- [ ] `background_tier.rs` — crossbeam workers, no time budget (Refly BullMQ pattern ported to Rust):
  - Job queue pattern (Refly `@Processor(QUEUE_NAME) extends WorkerHost { async process(job) }`):
    - `compile(source: &str, opts: CompileOpts) -> Result<PipelineOutput>` → maps to Refly `scaleboxExecute` queue
    - `plan_flow(output: &PipelineOutput) -> CompositionPlan` → maps to Refly `runWorkflow` queue
    - `verify(plan: &CompositionPlan) -> Vec<Diagnostic>` → maps to Refly `verifyNodeAddition` queue
    - `bridge::deep_think_plan(intent: &str, canvas: &Workspace, interrupt: Arc<AtomicBool>) -> Receiver<DeepThinkEvent>` — bridge wrapper over `nom_intent::deep_think()` (GAP-5), maps to Refly `skillExecution` queue. Emits `DeepThinkEvent::Step(DeepThinkStep)` per iteration, then `DeepThinkEvent::Final(CompositionPlan)`.
  - Job enqueue: `crossbeam_channel::Sender<BackgroundJob>` + worker threads pulling from channel
  - Progress streaming: `Sender<RunEvent>` per job → right dock `Receiver<RunEvent>` polls on each frame (Refly `writeSSEResponse` pattern in Rust channels)
  - Plugin/MCP integration point: `PluginRegistry { servers: Vec<McpServerConfig> }` — Refly `MultiServerMCPClient` equivalent, SSE + stdio transport support

- [ ] `adapters/highlight.rs` — first wire, ~200 LOC:
  - `stage1_tokenize(source) -> Vec<Spanned<Tok>>` → map each `Tok` variant to `TokenRole`:
    - `Tok::Keyword(_)` → `TokenRole::Keyword`
    - `Tok::Ident(_)` where dict lookup succeeds → `TokenRole::NomtuRef`
    - `Tok::Ident(_)` otherwise → `TokenRole::Identifier`
    - `Tok::String | Tok::Number` → `TokenRole::Literal`
    - `Tok::Operator(_)` → `TokenRole::Operator`
    - `Tok::Comment(_)` → `TokenRole::Comment`
  - Returns `Vec<HighlightSpan>` consumed by nom-editor `Highlighter::color_runs`

- [ ] `adapters/lsp.rs` — `impl LspProvider for CompilerLspProvider`:
  - Delegates `hover` → `nom-lsp::handle_hover`
  - Delegates `completions` → `nom-lsp::handle_completion`
  - Delegates `goto_definition` → `nom-lsp::handle_definition`

- [ ] `adapters/completion.rs` — nom-dict prefix search → `CompletionItem`:
  - `CompletionKind` mapping: `kind = "verb"` → `CompletionKind::Function`, `"concept"` → `CompletionKind::Class`, `"metric"` → `CompletionKind::Value`
  - Snippet support: `insert_text` = word + common clause template from grammar

- [ ] `adapters/score.rs` — `score_atom` → `StatusBar` compile status:
  - Score ≥ 0.8 → green badge "Valid"
  - Score 0.5–0.8 → amber badge "Low confidence"
  - Score < 0.5 → red badge "Unknown"

- [ ] **Grammar-typed nodes (replaces ALL string-typed patterns):**
  - `production_kind: String` on every `GraphNode` — validated via `is_known_kind()` at insert
  - `SlotBinding` replaces `Port { kind: String }` — derived from `clause_shapes` DB query
  - Real `can_wire(src, src_slot, dst, dst_slot, grammar, entries) -> Result<(bool, f32, String)>`:
    - Query: `SELECT grammar_shape FROM clause_shapes WHERE kind = src_kind AND name = src_slot`
    - Match against: `SELECT expected_shape FROM clause_shapes WHERE kind = dst_kind AND name = dst_slot`
    - If shapes compatible → `(true, 1.0, "explicit grammar match")`
    - If shapes related → `(true, 0.8, "inferred: {src_shape} → {dst_shape}")`
    - If incompatible → `(false, 0.0, "type mismatch: {src_shape} vs {dst_shape}")`
  - Every `Connector`: `confidence: f32` + `reason: String` populated from `can_wire()` result
  - Node palette: live `SELECT name, description FROM grammar.kinds ORDER BY name` — no hardcoded enum ever

- [ ] **GitNexus schema borrow for edge model (31 node concepts, 20 edge types):**
  - Edge confidence pattern: `CodeRelation { type, source_id, target_id, confidence: f32, reason: String, step: Option<u32> }`
  - Same polymorphic edge table used for `Connector` (type = relation name, confidence + reason required)
  - 20 edge type names borrowed conceptually: `Calls`, `Imports`, `Extends`, `Implements`, `Uses`, `Contains`, `Produces`, `Consumes`, `Triggers`, `Configures`, `Validates`, `Transforms`, `Routes`, `Schedules`, `Monitors`, `Documents`, `Tests`, `Deploys`, `Authenticates`, `Authorizes`

---

## Wave D — 3-Column Shell Assembly

### nom-panels (shell infrastructure)
*Pattern source: `APP/zed-main/crates/workspace/` — Panel trait, Dock, PaneGroup, focus system*

- [ ] `dock.rs`:
  - `DockPosition { Left, Right, Bottom }` enum
  - `Dock { position: DockPosition, panel_entries: Vec<PanelEntry>, is_open: bool, active_panel_index: usize, size: f32 }`
  - `PanelEntry { panel: Arc<dyn Panel>, toggle_action: Box<dyn Action>, is_zoomed: bool }`
  - Zed-exact resize: drag divider → update `size` → `cx.notify()`

- [ ] `panel_trait.rs` — `Panel` trait (18 methods matching Zed):
  - Required: `persistent_name() -> &'static str`, `position(cx) -> DockPosition`, `default_size(cx) -> f32`
  - Required: `toggle_action() -> Box<dyn Action>`, `icon(cx) -> Option<Icon>`, `icon_label(cx) -> Option<String>`
  - Required: `is_agent_panel(cx) -> bool`
  - Optional (provided): `activation_priority() -> u32`, `set_active(active, cx)`, `pane(cx) -> Option<View<Pane>>`
  - Focus: `Panel: Focusable + Render` — every panel has a `FocusHandle`
  - Sealed: `PanelInternal` private supertrait prevents external implementations breaking the contract

- [ ] `pane.rs`:
  - `Pane { items: Vec<Box<dyn Item>>, active_item_index: usize, focus_handle: FocusHandle, toolbar: Option<View<Toolbar>> }`
  - `Item` trait: `tab_content(cx) -> AnyElement`, `tab_tooltip(cx) -> Option<String>`, `set_nav_history()`
  - Tab strip: close button on hover, drag-to-reorder, Ctrl+W closes active

- [ ] `pane_group.rs`:
  - `PaneGroup { root: Member }` where `Member::Pane(View<Pane>) | Member::Axis(PaneAxis)`
  - `PaneAxis { axis: Axis, members: Vec<Member>, flexes: Vec<f32> }` — `flexes` sum = 1.0
  - Recursive split: `split(pane_id, axis, direction) -> View<Pane>` inserts new Member
  - Flex resize: drag handle → redistribute flexes maintaining sum = 1.0

- [ ] `shell.rs` — top-level `Shell`:
  - Layout: `flex_col { title_bar | flex_row { left_dock | pane_group | right_dock } | bottom_dock | status_bar }`
  - Title bar: workspace-switcher + sync/cloud/user controls
  - Status bar: left slot (file info) + center slot (compile status from nom-compiler-bridge) + right slot (agent status)
  - Modal stack: command palette, quick open, permission overlays
  - Toast notifications: auto-dismiss after 4s, stack vertically bottom-right

- [ ] `focus.rs`:
  - `Shell → active Pane → active Item` delegation chain
  - `FocusHandle::dispatch_action(action)` → bubble up chain until handled
  - Global actions bypass chain (Cmd+K, Cmd+P)

### Left dock (AFFiNE NavigationPanel pattern)
*Pattern source: `APP/AFFiNE-canary/packages/frontend/core/src/modules/navigation-panel/` — read end-to-end*

- [ ] `CollapsibleSection`:
  - Props: `{ title: String, path: String, initially_open: bool, icon: Option<Icon>, postfix: Option<AnyElement>, children: Vec<AnyElement> }`
  - State keyed by `path` (hierarchical dot-separated key like `"nav.dict.verbs"`) persisted via `SectionStateStore`
  - Open/close toggle on header click; chevron icon rotates 90° with CSS transition matching AFFiNE spring

- [ ] `MenuItem`:
  - Props: `{ label: String, icon: Option<Icon>, postfix: Option<AnyElement>, active: bool, indent_level: u32, on_click: Callback }`
  - Inline collapse toggle button (visible on hover) for items that have children
  - Active state: background = `theme.interactive_selected`, text = `theme.text_primary`
  - Indent: `padding_left = 12px + indent_level * 16px`

- [ ] `QuickSearchInput`:
  - Renders as non-focusable button showing "Search..." with Cmd+K shortcut hint
  - Click → dispatch `OpenCommandPalette` action → modal opens

- [ ] `ResizePanel`:
  - 4 states: `Open` (248px, interactive) | `Floating` (overlays center, still 248px) | `FloatingWithMask` (+ semi-transparent backdrop) | `Close` (0px, hidden)
  - Default width: 248px, min: 248px, max: 480px (user-draggable via resize handle)
  - Transition: ease-in-out 200ms on width change

- [ ] `SidebarScrollableContainer`:
  - Scroll shadow: top shadow appears when scrollTop > 0, bottom shadow when not scrolled to end
  - Shadow color from `theme.shadow_sm` with 24px blur

- [ ] Content:
  - `grammar.kinds` tree: grouped by kind category, each kind = CollapsibleSection with MenuItem per entry
  - BM25 search bar at top: live `search_bm25(query, 20)` → filtered tree
  - Pinned section: user-pinned NomtuRef entries
  - Recent section: last 10 opened/edited entries (persisted across sessions)
  - Settings link at bottom

### Right dock (Rowboat ChatSidebar pattern)
*Pattern source: `APP/rowboat-main/apps/x/apps/renderer/components/chat/` — read ChatSidebar.tsx end-to-end*

- [ ] `ChatSidebar` — primary right-dock component:
  - Multi-tab: `chat_tabs: Vec<ChatTab>` where `ChatTab { run_id: String, title: String, draft_message: String, is_active: bool }`
  - Tab bar: horizontal scroll if > 5 tabs, close button per tab
  - Conversation stream: `messages: Vec<Message>` with `ConversationAnchor` for auto-scroll-to-bottom

- [ ] `RunEvent` union — 14 variants (exact Rowboat pattern):
  - `LLMStream { content: String, delta: String }` — streaming token
  - `ToolInvocation { tool_id: String, name: String, input: serde_json::Value, status: ToolStatus }`
  - `ToolResult { tool_id: String, output: serde_json::Value, error: Option<String> }`
  - `PermissionRequest { tool_id: String, message: String }` — pauses execution for user approval
  - `AskHuman { message: String, question: String }` — agent asks user a question
  - `SpawnSubFlow { agent_name: String, run_id: String }` — spawns sub-agent
  - `TextMessage { role: Role, content: String }` — human/assistant turn
  - `Status { message: String, level: StatusLevel }` — informational
  - `ThinkingStream { delta: String }` — extended thinking token
  - `DeepThinkStep { hypothesis: String, evidence: Vec<String>, confidence: f32, counterevidence: Vec<String>, refined_from: Option<String> }` — Nom addition
  - `ComposeProgress { backend: String, percent: f32, eta_ms: Option<u64> }` — Nom addition
  - `Error { message: String, code: String }` — run-level error
  - `RunCompleted { summary: String }` — terminal event
  - `Interrupt` — user-triggered interrupt (wires to `InterruptFlag`)

- [ ] Tool card (Rowboat collapsible pattern):
  - `Radix Collapsible` equivalent: `ToolCard { tool_id, name, status, is_open, input, output }`
  - `ToolStatus { Pending | Running | Completed | Error | Denied }` with color badge
  - Input/Output tabs: JSON viewer with syntax highlighting
  - Permission overlay on `PermissionRequest`: "Allow" / "Deny" buttons, blurs conversation behind

- [ ] Deep-think reasoning stream:
  - Each `DeepThinkStep` → `ReasoningCard { hypothesis, evidence_list, confidence_badge, counterevidence_list, expand_toggle }`
  - Confidence badge: green/amber/red matching edge confidence colors
  - Stream animation: cards appear sequentially with 80ms stagger (spring animation)
  - Interrupt button: triggers `RunEvent::Interrupt` → sets `InterruptFlag`

- [ ] `ConversationAnchor` (Rowboat dynamic spacer pattern):
  - Last item in scroll container is a zero-height anchor div
  - On new message: `anchor.scrollIntoView({ behavior: "smooth" })` — auto-scroll to bottom
  - User scroll up → disable auto-scroll; user scroll down to anchor → re-enable

- [ ] Multi-tool state: `tool_open_by_tab: HashMap<(TabId, ToolId), bool>` — persists open/close per tab
- [ ] Multi-run history: sidebar list of past runs, click to restore conversation stream
- [ ] Chat input: multi-line with `Shift+Enter` for newline, `Enter` to send, `/` prefix for commands

### Bottom dock

- [ ] Terminal embed: `TerminalView` hosting a PTY via `portable-pty` crate (optional in Wave D, required for Wave E workflow backend)
- [ ] Diagnostics panel: displays `Vec<Diagnostic>` from nom-compiler-bridge `verify()`, clickable source locations
- [ ] Command history: ring buffer of last 100 commands with timestamps

### nom-gpui multi-window

- [ ] Extend `ApplicationHandler` with `windows: HashMap<WindowId, WindowSurface>`
- [ ] `WindowPool { surfaces: HashMap<WindowId, WindowSurface>, shared_gpu: Arc<GpuContext> }`
- [ ] `GpuContext` = `wgpu::Device + wgpu::Queue + wgpu::Instance` shared across all windows
- [ ] Right dock detach: `DockWindow { content: View<ChatSidebar>, position: Vec2, size: Vec2 }`

### Command palette
*Pattern source: Zed `command_palette.rs` + `picker.rs`*

- [ ] `Picker<CommandPaletteDelegate>` — async fuzzy search:
  - `CommandPaletteDelegate::matches(query: &str, items: &[CommandItem]) -> Vec<StringMatch>` via `fuzzy_match` crate
  - Items: registered actions from action registry + BM25 search results + recent commands
- [ ] Action registry: `HashMap<&'static str, Box<dyn Action>>` + keyboard shortcut display
- [ ] Query history: ring buffer 50 entries, up/down arrow to navigate
- [ ] Keybinding display: inline at right of each command item

---

## Wave E — Compose Targets (real implementations, priority order)

*All backends: use `MediaVendor` facade, write to `ArtifactStore` (SHA-256), stream via `ProgressSink`, handle `InterruptFlag`*

### Wave E pre-requisite: `nom-compose` infrastructure (9router pattern)

- [ ] `vendor_trait.rs` — `MediaVendor` trait + `Format { OpenAI, Claude, Gemini, Antigravity, Codex, Kiro }` enum
- [ ] `format_translator.rs` — 2-stage: `source→OpenAI` then `OpenAI→target` via `HashMap<(Format,Format), Box<dyn RequestTranslator>>`; concrete impls: `ClaudeToOpenAI`, `OpenAIClaude`, `GeminiToOpenAI`, `OpenAIToGemini`
- [ ] `account_fallback.rs`:
  - `Account { id, status, rate_limited_until: Option<Instant>, backoff_level: u32, model_locks: HashMap<String,Instant> }`
  - Backoff formula: `cooldown_ms = min(1000u64 * 2u64.pow(backoff_level), 120_000)` — exact 9router formula, cap 120s at level 7+
  - Error classification: 401→2min/no-escalate, 429→exponential+escalate, 402/403→2min/model-lock, 502/503/504/408→30s/no-escalate
  - `filter_available_accounts(accounts, exclude_id)` — skip any with `rate_limited_until > Instant::now()`
- [ ] `executor_registry.rs` — `ExecutorRegistry { executors: HashMap<String, Box<dyn MediaVendor>> }`, `parse_model("provider/model")`, alias resolution
- [ ] `artifact_store.rs` — `ArtifactStore::write(bytes: &[u8], mime: &str) -> [u8;32]` (SHA-256 hash), `read(hash: &[u8;32]) -> Vec<u8>`
- [ ] `progress_sink.rs` — `ProgressSink` wraps `Sender<RunEvent>`, sends `RunEvent::ComposeProgress { backend, percent, eta_ms }`

### E-1: Document backend (`document.rs`)
*Pattern source: typst comemo pattern from `APP/Accelworld/services/other5/typst-main/`*

- [ ] typst comemo incremental layout:
  - `Tracked<T>` wrapper: tracks which methods were called + their return values as constraints
  - `Constraint::new()` + `constraint.validate()` loop: re-run only if inputs changed since last run
  - `hash128` via SipHash13 128-bit for stable content fingerprinting
  - `Frame { size: Axes<Abs>, items: Vec<(Point, FrameItem)> }` output type
  - `FrameItem` enum: `Group(GroupItem)`, `Text(TextItem)`, `Shape(Shape)`, `Image(ImageItem)`, `Link(Destination, Size)`, `Tag(Tag)`
- [ ] PDF export: `krilla` crate (typst's internal renderer) → PDF bytes
- [ ] DOCX export: `docx-rs` crate → DOCX bytes
- [ ] `compose(plan: &CompositionPlan, sink: &ProgressSink, interrupt: &InterruptFlag) -> Result<Artifact>`

### E-2: Video backend (`video.rs`)
*Pattern source: Remotion patterns (DeepWiki) + `APP/Accelworld/services/media/waoowaoo/`*

- [ ] Remotion frame model:
  - `useCurrentFrame()` equivalent: `fn current_frame(composition: &Composition) -> f64` — global frame counter
  - `Sequence` dual coordinate: relative frame within sequence + absolute canvas frame
  - `fps: f64`, `duration_in_frames: u64`, `width: u32`, `height: u32` per composition
- [ ] Render pipeline:
  - N-page pool concurrency: `rayon` thread pool, one frame per thread
  - Each frame: nom-gpui offscreen render → `wgpu::Buffer` readback → RGBA bytes
  - FFmpeg stdin piping: `rawvideo` input format, `-f rawvideo -pix_fmt rgba -s {w}x{h} -r {fps} -i pipe:0`
  - Output: H.264/H.265/VP9/AV1 via FFmpeg codec flags
- [ ] waoowaoo FFmpeg assembly patterns:
  - Concat demuxer: `file 'clip1.mp4'\nduration 2.5\nfile 'clip2.mp4'\n...` → `ffmpeg -f concat -i list.txt`
  - xfade filter: `-filter_complex "[0:v][1:v]xfade=transition=fade:duration=0.5:offset=2.0[v]"`
  - amix audio: `-filter_complex "[0:a][1:a]amix=inputs=2:duration=first[a]"`
  - `dependency_resource_id` batch tracking: all resources needed for a phase declared upfront

### E-3: Data extract (`data_extract.rs`)
*Pattern from opendataloader XY-Cut++ documentation*

- [ ] XY-Cut++ deterministic mode (0.015s/page):
  - Recursive X-then-Y projection splitting on bounding box set
  - Project all bboxes onto X axis → find gaps > threshold → split into column groups
  - For each group: project onto Y axis → find gaps → split into row groups
  - Post-order traversal of the split tree = reading order
  - Pure Rust port — no ML, no GPU required, fully deterministic
- [ ] Hybrid-AI mode (+90% table accuracy, ~0.463s/page):
  - SmolVLM-2 for table region detection: `detect_tables(image: &DynamicImage) -> Vec<TableRegion>`
  - XY-Cut++ within each table region for cell extraction
  - Combine: tables via hybrid-AI, prose via XY-Cut++ deterministic
- [ ] Output: `ExtractedPage { text_blocks: Vec<TextBlock>, tables: Vec<Table>, images: Vec<ImageRef>, reading_order: Vec<usize> }`

### E-4: Web screen (`web_screen.rs`)
*Pattern source: `APP/ToolJet-develop/` — 72 widgets, combineProperties, dependency graph*

- [ ] ToolJet widget catalog (72 widgets, 55 active priority):
  - Form widgets: `TextInput`, `NumberInput`, `TextArea`, `Select`, `MultiSelect`, `Checkbox`, `RadioGroup`, `DatePicker`, `TimePicker`, `ColorPicker`, `FilePicker`, `Password`, `PhoneInput`, `OTPInput`
  - Display widgets: `Text`, `Image`, `Icon`, `Badge`, `Avatar`, `Spinner`, `Alert`, `Tooltip`, `Statistics`, `Timeline`, `Pdf`, `Iframe`
  - Layout widgets: `Container`, `Tabs`, `Steps`, `Accordion`, `Modal`, `Drawer`, `Card`, `Divider`, `SplitPane`
  - Data widgets: `Table`, `List`, `Chart` (6 types via graphify), `Kanban`, `Calendar`, `Tree`, `Map`
  - Action widgets: `Button`, `IconButton`, `ButtonGroup`, `Link`, `Menu`, `DropdownButton`
  - Media widgets: `VideoPlayer`, `AudioPlayer`, `RichTextEditor`, `CodeEditor`, `QRCode`, `Barcode`
- [ ] `combineProperties(universal_props, specific_props, override_props)` deep-merge pattern
- [ ] 43-column grid layout with pixel-perfect positioning
- [ ] `RefResolver` for `$ref`-based property dependencies — topological sort before evaluation
- [ ] Event system: `fireEvent(widget_id, event_name, payload)` → bound action execution

### E-5: Workflow (`scenario_workflow.rs`)
*Pattern source: `APP/Accelworld/services/automation/n8n/` + `APP/Accelworld/services/other4/dify-main/` + `APP/Accelworld/services/other2/ComfyUI-master/`*

- [ ] n8n DAG execution pattern:
  - Pull-based: each node requests inputs from upstream (not push-based)
  - Retry: `retry_count: u32`, `retry_interval: Duration`, exponential backoff
  - Webhook resume: `WaitingExecution` state + webhook ID stored in DB
  - Error workflow: `on_error: Option<WorkflowId>` per node
- [ ] ComfyUI execution cache (4-tier hierarchy):
  - `ExecutionCache` trait with 4 impls: `NullCache` (testing/debug), `LRUCache(cap)` (standard), `RAMPressureCache` (memory-constrained, evicts on pressure), `HierarchicalCache(ram, disk)` (persistent)
  - Cache key = node fingerprint: `SipHash(class_type + sorted_inputs + IS_CHANGED_result)`
  - `IS_CHANGED` lookup hierarchy: custom `IS_CHANGED()` class method → input hash comparison → `false` (always cached)
  - Cache hit → reuse outputs; cache miss → execute node → store outputs
  - `VariablePool { outputs: HashMap<NodeId, HashMap<OutputName, Value>> }` — shared mutable state across all node executions in a run (ComfyUI `PromptExecutor.execute()` pattern)
- [ ] ComfyUI topological sort (Kahn algorithm):
  - `blockCount: HashMap<NodeId, usize>` — count of blocking dependencies per node
  - `blocking: HashMap<NodeId, Vec<NodeId>>` — which nodes each node unblocks
  - Queue: all nodes with `blockCount == 0` → execute → decrement blockers → re-enqueue when `blockCount == 0`
  - Cycle detection: if queue empties before all nodes executed → `CycleError`
- [ ] n8n AST sandbox for code execution:
  - `JsTaskRunnerSandbox { worker_process, stdin_writer, stdout_reader }` pattern in Rust
  - 4 AST sanitizers (implement as Rust `syn` visitors):
    - `ThisSanitizer` — reject `this.` access
    - `PrototypeSanitizer` — reject `__proto__`, `constructor.prototype` patterns
    - `DollarSignValidator` — validate `$` prefixed variables against allowed list
    - `AllowlistSanitizer` — reject any identifier not in explicit allowlist
  - Isolation: execute in subprocess, communicate via stdin/stdout, 128MB memory limit, 5000ms timeout
  - Expression detection: leading `=` character → template expression; otherwise literal string
- [ ] dify node execution pattern:
  - `WorkflowNode` trait with `fn execute(&self, state: &mut GraphRuntimeState) -> impl Iterator<Item = NodeEvent>`
  - `NodeEvent` enum: `StreamChunkEvent { chunk: String }`, `StreamCompletedEvent { outputs: HashMap<String, Value> }`, `ErrorEvent { error: String }`
  - `GraphRuntimeState { variable_pool: VariablePool, execution_history: Vec<NodeId> }`
  - `VariablePool` with `{{ variable_name }}` template resolution (VariableTemplateParser pattern)
  - Push-based event-driven execution: each node emits events, runtime collects into stream

### E-6: Data query (`data_query.rs`)
*Pattern source: `APP/wrenai/` — SemanticModel + 5-stage pipeline*

- [ ] WrenAI 5-stage pipeline:
  - Stage 1 `IntentClassification` — classify query as SQL/explanation/clarification
  - Stage 2 `ContextRetrieval` — retrieve relevant MDL models + columns via BM25 + vector search
  - Stage 3 `SQLGeneration` — LLM with MDL schema context → SQL
  - Stage 4 `SQLCorrection` — execute → catch error → LLM correction loop (max 3 rounds)
  - Stage 5 `Execute` — run corrected SQL against data source → `QueryResult`
- [ ] `SemanticModel { name: String, columns: Vec<ModelColumn>, properties: Vec<Property> }` — MDL definition
- [ ] MDL grounding: inject `SemanticModel` definitions into LLM prompt context

### E-7: Image (`image.rs`)
*Pattern source: `APP/Accelworld/upstreams/Open-Higgsfield/` — 4 studios, 200+ models*

- [ ] Open-Higgsfield 4-studio model:
  - `ImageStudio` — static image gen: `t2iModels` + `i2iModels`
  - `VideoStudio` — video gen: `t2vModels` + `i2vModels` + `v2vModels`
  - `CinemaStudio` — cinematic quality: larger models, longer generation
  - `LipSyncStudio` — `lip_sync_models`
- [ ] Auto-switch: if image uploaded → force `i2vModels` (t2v → i2v mode switch)
- [ ] Model dispatch: `dispatch(prompt, studio, model_id) -> impl Stream<Item = GenerationEvent>`
- [ ] `MediaVendor` facade — one interface, N concrete vendor backends

### E-8: Storyboard / Novel→Video (`storyboard.rs`)
*Pattern source: `APP/Accelworld/services/other4/ArcReel-main/` + `APP/Accelworld/services/media/waoowaoo/`*

**Handles BOTH inputs:**
- **Short-form script → Video** — user supplies a few-line script, pipeline runs all 5 phases
- **Novel → Video (long-form text)** — user supplies `.nomx` prose block or imported `.txt`/`.md`; Phase 1 (`ScriptGeneration`) uses `nom-intent` to chunk novel into scene beats before invoking ArcReel phases

- [ ] `StoryboardInput { Script(String) | Novel { text: String, target_duration_s: u32, style: StyleHint } }`
- [ ] Novel path: `chunk_novel(text, target_duration_s) -> Vec<SceneBeat>` → feeds into `ScriptGeneration` as pre-structured beats
- [ ] ArcReel 5-phase orchestration (runs identically once input is normalised to scene list):
  - Phase 1 `ScriptGeneration` — LLM → structured script `{ scenes: Vec<Scene> }`
  - Phase 2 `StoryboardGeneration` — per-scene: LLM → visual description + shot type
  - Phase 3 `CinematographyGeneration` — camera movements, lighting, composition
  - Phase 4 `ActingGeneration` — character actions, expressions, dialogue timing
  - Phase 5 `Compose` — ffmpeg concat + xfade + amix (see E-2 patterns)
- [ ] waoowaoo Phase 2a‖2b parallel execution:
  - Phase 2a (cinematography) + Phase 2b (acting) run concurrently via `tokio::join!`
  - `dependency_resource_id` declared upfront per phase for batch resource pre-fetching

### E-9: Audio (`audio.rs`)
- [ ] Synthesis: `SynthesisRequest { text, voice_id, speed, pitch }` → audio bytes
- [ ] Codec: `encode(pcm_bytes: &[f32], codec: AudioCodec) -> Vec<u8>` where `AudioCodec { FLAC | AAC | MP3 | Opus }`
- [ ] Timing align: `align(audio_segments: Vec<AudioSegment>, script: &Script) -> Vec<TimedSegment>` — DTW alignment

### E-10 through E-16: Remaining backends
- [ ] **Native screen** (`native_screen.rs`) — delegate to `nom-compiler/nom-llvm` `compile()` + `link_bitcodes()`
- [ ] **Mobile screen** (`mobile_screen.rs`) — iOS/Android target via `nom-llvm` cross-compilation or Flutter-gen
- [ ] **Presentation** (`presentation.rs`) — typst comemo pattern (E-1) + slide layout DSL + PDF/HTML/MP4 export
- [ ] **App bundle** (`app_bundle.rs`) — collect frontend (web_screen) + backend (workflow) + deploy config → tarball artifact
- [ ] **Ad creative** (`ad_creative.rs`) — multi-platform variants: FB 9:16, YT 16:9, IG 1:1 from same composition
- [ ] **Data frame** (`data_frame.rs`) — `Series<T>` + `DataFrame { columns: Vec<Series> }`, hash-join, predicate pushdown (no polars dep)
- [ ] **3D mesh** (`mesh.rs`) — glTF 2.0: `Mesh { primitives: Vec<Primitive> }`, `Primitive { attributes, indices, material }`, export via `gltf` crate

---

## Wave F — AFFiNE Graph RAG + Deep Thinking

### AFFiNE-inspired graph mode
*Pattern: AFFiNE design tokens + GitNexus confidence edges + LlamaIndex RAG + graphify visualization*

- [ ] Graph node cards:
  - Frosted-glass: `background = rgba(token.bg_layer_2, 0.85)`, `backdrop_filter = blur(12px)`, `border = 1px solid rgba(255,255,255,0.12)`
  - Drop shadow: `box-shadow = 0 8px 24px rgba(0,0,0,0.12)` (theme.shadow_lg)
  - Inter 14px body + SCP 12px for slot names
  - Port indicators: colored dots on left (inputs) and right (outputs) edges

- [ ] Edge confidence rendering (GitNexus CodeRelation pattern):
  - Green `#22C55E` (Tailwind green-500) + opacity = confidence for edges ≥ 0.8
  - Amber `#F59E0B` (Tailwind amber-500) + opacity = confidence for edges 0.5–0.8
  - Red `#EF4444` (Tailwind red-500) + opacity = confidence for edges < 0.5
  - Stroke width = `2 + confidence * 2` px (thicker = more confident)

- [ ] Bezier edge routing:
  - Control points: `cp1 = src_port + Vec2(80, 0)`, `cp2 = dst_port - Vec2(80, 0)`
  - Cubic bezier rendered via `nom-gpui` `Path` primitive
  - Label at midpoint: relation name + confidence value

- [ ] RAG visualization overlay (LlamaIndex pattern):
  - Retrieved context from `nom-search::BM25Index::search` → `Vec<(NomtuRef, f32)>`
  - Render as colored arc paths between query node and retrieved nodes
  - Arc color = retrieval score mapped to confidence color scale
  - Toggle visibility: "RAG overlay" toggle button in graph toolbar
  - RRF formula for hybrid retrieval: `score = Σ 1/(rank_k + 60)` (RRF_K = 60, matching LlamaIndex/GitNexus)

- [ ] Node palette:
  - Live `SELECT name, description, category FROM grammar.kinds ORDER BY category, name`
  - Grouped by category (media, concept, metric, verb, ...)
  - Drag from palette → drop on canvas → auto-create `GraphNode` with `SlotBinding`s from `clause_shapes`
  - Search filter: BM25 on name + description fields

- [ ] Spring animations (AFFiNE motion tokens):
  - Connect: spring(stiffness=400, damping=28) on edge appearance
  - Disconnect: ease-out 120ms fade-out
  - Node drag: no animation (direct position tracking)
  - Node appear: ease-out 200ms scale 0.8→1.0 + opacity 0→1

### Deep thinking implementation

- [ ] `nom-intent/src/deep.rs`:
  - `DeepThinkStep { hypothesis: String, evidence: Vec<String>, confidence: f32, counterevidence: Vec<String>, refined_from: Option<String> }`
  - **Two layers, two signatures — not a conflict:**
    - **Compiler layer (nom-intent, GAP-5):** `nom_intent::deep_think(intent: &str, grammar: &Connection, entries: &SqlitePool, interrupt: Arc<AtomicBool>) -> impl Iterator<Item = DeepThinkStep>` — pure iterator, no canvas knowledge
    - **Bridge layer (nom-compiler-bridge, Wave C→F):** `bridge::deep_think_plan(intent: &str, canvas: &Workspace, interrupt: Arc<AtomicBool>) -> Receiver<DeepThinkEvent>` where `DeepThinkEvent { Step(DeepThinkStep) | Final(CompositionPlan) }` — wraps the iterator, drives `plan_flow()` at the end, streams to right dock
  - Right dock receives `DeepThinkEvent::Step` cards; final `CompositionPlan` is handed to compose target picker
  - Max 10 steps; early exit if confidence ≥ 0.92 or `interrupt.load(SeqCst) == true`
  - Each step: call `classify_with_react(intent + hypothesis_context)` → score → append to chain

- [ ] Wire to `background_tier.rs`:
  - `deep_think` call returns `Receiver<DeepThinkStep>` — background thread sends, UI thread receives
  - UI polls receiver on each frame via `cx.spawn(async { ... })` without blocking render

- [ ] Right dock streaming:
  - New `DeepThinkStep` received → prepend `ReasoningCard` to conversation stream
  - Cards are collapsible (expanded by default)
  - "Thinking..." spinner between steps

- [ ] Compose target picker: "Deep Think" toggle:
  - When enabled: run `deep_think()` before `plan_flow()` in background_tier
  - Deep think output `Vec<DeepThinkStep>` passed to `plan_flow()` as context

- [ ] `InterruptFlag`:
  - `Arc<AtomicBool>` shared between right dock interrupt button and `deep_think()` loop
  - Button click → `flag.store(true, SeqCst)` → `deep_think` detects on next iteration → graceful stop
  - Best-effort `CompositionPlan` returned from steps completed so far

### Graph RAG query flow (end-to-end)
*Pattern: LlamaIndex 40+ retrievers + Haystack @component + graphify SVG export*

- [ ] `nom-search` enhanced retrievers (LlamaIndex retriever pattern, 15 postprocessor types):
  - `BM25Retriever` — lexical search, `BM25Index::search(query, top_k)`
  - `VectorRetriever` — semantic search via embeddings (GAP-2 dependency)
  - `HybridRetriever` — RRF fusion: `1/(rank + 60)` formula, combine BM25 + vector results
  - Postprocessors (15 types): `SimilarityThreshold(0.5)`, `TopN(20)`, `KeywordFilter(keywords)`, `MetadataFilter(kind=?)`, `LLMRerank`, `CohereRerank`, `MMR(lambda=0.5)`, `FixedRecency`, `EmbeddingRecency`, `SentenceEmbedding`, `NodeRelationship`, `PrevNextExpansion`, `AutoMerge`, `LongContextReorder`, `TimeWeighted`
- [ ] Haystack `@component` style pipeline:
  - `RAGPipeline { steps: Vec<Box<dyn PipelineComponent>> }`
  - `PipelineComponent::run(inputs: HashMap<String, Value>) -> HashMap<String, Value>`
  - Connect: `pipeline.connect("retriever.documents", "reranker.documents")`
- [ ] graphify visualization patterns for data display in right dock:
  - 6 chart types: `LineChart`, `BarChart`, `ScatterPlot`, `NetworkGraph`, `TreeMap`, `SankeyDiagram`
  - Redux state shape (graphify pattern): `{ charts: { byId: HashMap<ChartId, ChartConfig>, allIds: Vec<ChartId> }, ui: { selectedChart: Option<ChartId>, zoom: f32 }, data: { series: HashMap<SeriesId, Vec<DataPoint>> } }`
  - Real-time updates: WebSocket `socket.emit("data-update", { chartId, series })` → client re-renders (port to Rust: `Sender<ChartUpdate>` per chart panel)
  - History: `undo_stack: Vec<ChartConfig>`, `redo_stack: Vec<ChartConfig>` — same undo/redo pattern as canvas
  - `htmlToImage.toJpeg({ quality: 0.95, width, height })` for snapshot export (Rust: `wgpu` offscreen render → JPEG encode via `image` crate)

---

## Ongoing: Test quality standards

- Every block type: test nomtu backing — `entity.kind` validates against `grammar.kinds` via `is_known_kind()`
- Every `SlotBinding`: test `grammar_shape` against `clause_shapes` table
- Every wire: test `can_wire()` returns correct `(bool, f32, String)` tuple — test explicit match, inferred match, type mismatch
- Every connector: test `confidence` populated from `can_wire()`, not hardcoded
- `stage1_tokenize → Highlighter` pipeline: integration test with real `.nomx` input, verify keyword + NomtuRef roles
- Deep think: unit test `DeepThinkStep` confidence scoring, test interrupt via `AtomicBool`
- typst comemo: test constraint invalidation — mutate input → verify re-run; same input → verify cache hit
- n8n AST sanitizers: test each of 4 sanitizers with valid and invalid JS AST samples
- No "does it construct without panicking" tautological tests
- Property-based tests: BoundsTree R-tree (random inserts + queries), DAG topology (random graph + cycle detection + Kahn sort)
- Rowboat RunEvent: test all 14 variants serialize/deserialize correctly
- Panel trait: test all 7 required methods on at least 2 Panel impls

---

## Compiler Remaining (nom-compiler — parallel track)

*Runs in parallel with NomCanvas waves. nom-compiler crates are UNCHANGED from wave-12; these are the remaining gaps blocking full functionality.*

### GAP-1c — `body_bytes` (in progress)

- [ ] `nom-concept/src/pipeline.rs` — `PipelineOutput.body_bytes: Option<Vec<u8>>` field — populate from stage4 AST serialization
- [ ] `nom-codegen` — emit body bytes for every function entry during codegen pass
- [ ] `nom-dict` — `upsert_entity()` persists `body_bytes` to `entries.body_bytes` BLOB column (schema already exists)
- [ ] Integration test: `stage1_tokenize → run_pipeline → body_bytes.is_some()` on a `.nomx` fixture

### GAP-2 — Embeddings

- [ ] `nom-search/src/embed.rs` — `embed(text: &str, model: EmbedModel) -> Vec<f32>` — uses LLM adapter (not direct API, see GAP-2 provider note below)
- [ ] `nom-dict` — `entries.embedding BLOB` column (f32 × 768, stored as raw bytes) — already in schema stub, needs population
- [ ] `BM25Index::build_with_embeddings(conn)` — load embeddings from DB into in-memory HNSW index (use `hnsw_rs` crate)
- [ ] `VectorRetriever::search(query_embedding: &[f32], top_k: usize) -> Vec<(NomtuRef, f32)>` — cosine similarity via HNSW
- [ ] `HybridRetriever` — merge BM25 + vector results via RRF `1/(rank+60)` (already specified in task.md Wave F)
- [ ] Provider: embed via nom-intent LLM adapter (same 9router 3-tier fallback — see GAP-4 below)

### GAP-3 — Corpus ingestion

- [ ] `nom-corpus/src/ingest.rs` — `ingest_repo(path: &Path, conn: &Connection) -> Result<IngestReport>`
  - Walk `.rs`/`.py`/`.js`/`.ts`/`.go` files → extract top-level function/class/type names
  - For each: `upsert_entity(word, kind, output_type)` with `status = Partial`
  - Skip-list: `node_modules/`, `target/`, `.git/`, generated files
- [ ] `nom-corpus/src/checkpoint.rs` — `Checkpoint { repo_url, last_offset, status }` in `corpus_checkpoints` table — resume on crash
- [ ] `nom-corpus/src/bandwidth.rs` — `BandwidthThrottle { bytes_per_sec: u64 }` — non-optional, default 10 MB/s
- [ ] Corpus sources (priority order):
  - [ ] Local repos: `APP/Accelworld/upstreams/` (228 repos) — `nom corpus ingest repo <path>`
  - [ ] PyPI top-500 — stream-download → parse → discard source (peak disk = one repo at a time)
  - [ ] GitHub top-500/ecosystem (JS/Python/Rust/Go/Java/C++/Swift/Ruby/PHP) — same stream-and-discard discipline
- [ ] `nom corpus status` — show ingestion progress: `{ total_repos, completed, partial, failed, entries_added }`
- [ ] `nom corpus workspace-gc` — delete any lingering source files from failed ingestions

### GAP-4 — nom-intent LLM adapter with 9router pattern

- [ ] `nom-intent/src/llm.rs` — `ReActLlmFn` trait → 4 concrete adapters:
  - `StubAdapter` — deterministic responses for testing
  - `NomCliAdapter` — calls `nom-compiler` CLI itself as oracle (Nom-as-its-own-oracle, no external API key required)
  - `McpAdapter` — calls MCP server tool (Refly MultiServerMCPClient pattern)
  - `ExternalLlmAdapter` — optional real LLM (OpenAI/Anthropic/Gemini via 9router FormatTranslator)
- [ ] `nom-intent/src/retry.rs` — 9router exponential backoff applied to all LLM calls:
  - `retry_with_backoff(f, max_attempts: u32) -> Result<T>`
  - Formula: `cooldown_ms = min(1000u64 * 2u64.pow(attempt), 120_000)` — exact 9router formula
  - On `RateLimited` → increment `backoff_level`, sleep `cooldown_ms`, retry next adapter tier
  - On `Transient` (timeout/503) → sleep 30s, retry same adapter
  - 3-tier fallback: `ExternalLlmAdapter` → `McpAdapter` → `NomCliAdapter` (last resort is always local)

### GAP-5 — `nom-intent::deep_think()` (Wave F dependency)

- [ ] `nom-intent/src/deep.rs`:
  - `DeepThinkStep { hypothesis: String, evidence: Vec<String>, confidence: f32, counterevidence: Vec<String>, refined_from: Option<String> }`
  - `deep_think(intent: &str, grammar: &Connection, entries: &SqlitePool, interrupt: Arc<AtomicBool>) -> impl Iterator<Item = DeepThinkStep>`
  - ReAct loop: max 10 steps, each step calls `classify_with_react(intent + hypothesis_chain_context)`
  - Score: `nom-score::score_atom(hypothesis_word, kind)` → `f32` confidence per step
  - Early exit: `confidence ≥ 0.92` OR `interrupt.load(SeqCst)`
  - Returns iterator so nom-canvas `background_tier` can stream steps as `RunEvent::DeepThinkStep`
- [ ] Unit tests:
  - `deep_think` with `StubAdapter` returns ≥ 1 step with `confidence > 0.0`
  - `interrupt` flag stops loop within 1 additional step of being set
  - `refined_from` chain is non-circular

### Bootstrap proof track (fixpoint — parallel to all above)

- [ ] Stage0 (Rust nom-compiler) → compile `.nomx` fixture → Stage1 binary
- [ ] Stage1 binary → compile same fixture → Stage2 binary
- [ ] Stage2 binary → compile same fixture → Stage3 binary
- [ ] Assert `sha256(Stage2_output) == sha256(Stage3_output)` — byte-identical = fixpoint PROOF
- [ ] Record proof tuple: `(s1_hash, s2_hash, s3_hash, fixpoint_at_date, compiler_manifest_hash)` → insert into `entries` as `kind = "bootstrap_proof"`
- [ ] `nom bootstrap status` CLI command — shows current stage + hashes + whether fixpoint achieved
