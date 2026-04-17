# Remaining Work — Execution Tracker

> **CANONICAL TRACKING DOC — MAIN** (Planner/Auditor refreshes every cycle)
> **HEAD:** `e2b7ecb` on main | **CI:** canvas job GREEN (cargo check + test 23s on ubuntu-latest); compiler matrix still running | **Date:** 2026-04-17
> **Spec:** `docs/superpowers/specs/2026-04-17-nomcanvas-gpui-design.md` (719 lines) — canonical
> **Sibling docs:** `implementation_plan.md`, `nom_state_machine_report.md` (all 4 MUST stay in sync)
> **v1 archived** to `.archive/nom-canvas-v1-typescript/`. `nom-canvas/` Phase 1 batch-1 landed on main (31/31 tests pass).

---

## Priority 1: NomCanvas Implementation (Custom GPUI — Full Rust)

### Phase 1 — GPU Framework (nom-gpui) — batch-1 LANDED (31/31 tests)
- [x] Scene graph (Quad, Shadow, Underline, MonochromeSprite, PolychromeSprite, Path) — [scene.rs](nom-canvas/crates/nom-gpui/src/scene.rs)
- [x] BoundsTree R-tree DrawOrder assignment (MAX_CHILDREN=12) — [bounds_tree.rs](nom-canvas/crates/nom-gpui/src/bounds_tree.rs)
- [x] Core geometry (Point, Size, Bounds, Pixels/ScaledPixels/DevicePixels, TransformationMatrix) — [geometry.rs](nom-canvas/crates/nom-gpui/src/geometry.rs)
- [x] Color (Rgba, Hsla, source-over blend) — [color.rs](nom-canvas/crates/nom-gpui/src/color.rs)
- [x] Glyph atlas trait (PlatformAtlas + AtlasTextureKind Mono/Subpixel/Polychrome + InMemoryAtlas for tests) — [atlas.rs](nom-canvas/crates/nom-gpui/src/atlas.rs)
- [x] Element trait (request_layout → prepaint → paint with caller-owned state) — [element.rs](nom-canvas/crates/nom-gpui/src/element.rs)
- [x] taffy layout wrapper (LayoutEngine + to_taffy conversion) — [taffy_layout.rs](nom-canvas/crates/nom-gpui/src/taffy_layout.rs)
- [x] Styled fluent builder (.flex_col().w().padding().bg().rounded()) — [styled.rs](nom-canvas/crates/nom-gpui/src/styled.rs)
### Phase 1 batch-1 AUDIT FIXES (PREREQUISITE — fix before batch-2 GPU work)

**CRITICAL — wrong rendering without these:**
- [ ] `scene.rs` BatchIterator: stop emitting all-remaining-of-kind; use `Peekable`+`next_if` advancing only until next kind's order (ref [zed scene.rs:316-373](APP/zed-main/crates/gpui/src/scene.rs#L316-L373))
- [ ] `bounds_tree.rs` insert: replace monotonic `saturating_add(1)` with overlap-aware `max_intersecting + 1` (ref [zed bounds_tree.rs:119-135](APP/zed-main/crates/gpui/src/bounds_tree.rs#L119-L135)); panic or Result on `next_order` overflow
- [ ] `scene.rs` PrimitiveBatch sprite variants: add `texture_id`; sort sprites by `(order, tile_id)` in `finish()`; break batches on `texture_id` change

**HIGH — API shape fixes (do before downstream crates depend on nom-gpui):**
- [ ] `color.rs` Hsla: adopt `[0,1]` hue convention OR add explicit `from_degrees`/`from_normalized` constructors
- [ ] `styled.rs` all 40+ fluent setters: change from `fn m(mut self, ...) -> Self` to `fn m(&mut self, ...) -> &mut Self`
- [ ] `geometry.rs` Pixels: remove `From<f32>` + `From<Pixels> for f32`; Styled setters take `impl Into<Pixels>`
- [ ] `taffy_layout.rs` request_layout: return `Result<LayoutId, LayoutError>` (match `compute_layout` contract)

**MEDIUM — pattern backfills from Zed:**
- [ ] Add `request_measured_layout` + `NodeContext` measure closure (ref [zed taffy.rs:80-104](APP/zed-main/crates/gpui/src/taffy.rs#L80-L104))
- [ ] Add `SubpixelSprite` + `Surface` primitive kinds to Scene (8 vecs total)
- [ ] Add `PlatformAtlas::remove(key)` for per-tile eviction
- [ ] Change `AtlasKey.bytes: Vec<u8>` → `Arc<[u8]>` or `Borrow`-based key
- [ ] Make `Scene` primitive fields `pub(crate)`; expose read-only accessors
- [ ] Replace BoundsTree recursive walk with explicit `Vec<u32>` stack
- [ ] Add `half_perimeter` area heuristic to BoundsTree child selection
- [ ] Wire `max_leaf` fast-path in `topmost_intersecting`
- [ ] Fix `draw_element` to call `compute_layout` between phases OR document caller responsibility

**LOW:**
- [ ] Add vertical-overflow check to InMemoryAtlas shelf wrap
- [ ] Remove unused `bytemuck` dep until batch-2 needs it
- [ ] Remove `pub` from `PrimitiveKind` (or document consumers)
- [ ] Collapse ~25 Styled setters into `style_setter!` macro
- [ ] Add `Debug` derives to `BatchIterator`, `ElementCx`, `LayoutEngine`
- [ ] Doc comments on `ElementCx.rem_size`/`scale_factor`, `AtlasTileRef.uv`, `NodeContext`

### Phase 1 batch-2 (NEXT) — GPU substrate (Zed `crates/gpui_wgpu/` reference)

- [ ] `gpu_pipeline.rs` — 8 `wgpu::RenderPipeline` + shared bind group layouts (quads, shadows, path_rasterization, paths, underlines, mono_sprites, subpixel_sprites, poly_sprites)
- [ ] `shaders/` — 4 WGSL files (quad, sprite, path, underline); unit-quad vertex scaled by bounds
- [ ] `gpu_buffers.rs` — Instance storage buffer `@group(1) @binding(0)`, 2× growth, max_buffer_size clamp, 20-frame overflow guard
- [ ] `wgpu_atlas.rs` — Replace `InMemoryAtlas` with `etagere::BucketedAtlasAllocator` per kind; 1024² default → clamp `max_texture_dimension_2d`; `PendingUpload` → `queue.write_texture()` in `before_frame()`
- [ ] `text_rasterization.rs` — cosmic-text `ShapeLine` + Advanced shaping → swash `ScaleContext` → atlas tile (subpixel_bgra or Alpha format)
- [ ] `window.rs` — winit 0.30 `Window` + `wgpu::Surface`; Bgra8 preferred / Rgba8 fallback; alpha PreMultiplied/Opaque; Fifo present mode default
- [ ] `frame_loop.rs` — `EventLoop::run_app` + `ApplicationHandler`; acquire → draw → present per frame; OS-vsync cadence (NO 60fps tick)
- [ ] `device_lost.rs` — `GpuContext` device-lost flag + recreate path
- [ ] Feature flags `native` (default) vs `web` (wasm32) in Cargo.toml; `#[cfg(target_arch = "wasm32")]` gates
- [ ] Hit-testing wiring — `Scene::topmost_intersecting(point)` + winit pointer event routing to ElementId
- [ ] Element state storage — `HashMap<ElementId, Box<dyn Any>>` on Window
- [ ] Integration tests — headless offscreen `TextureView` render; atlas round-trip; path intermediate-texture; resize/device-lost; 60-frame buffer-growth soak

### Phase 2 — Canvas + Editor (nom-canvas-core + nom-editor)
- [ ] Infinite canvas (viewport, zoom, pan, coordinate transforms)
- [ ] Elements (8 shapes + hit testing + transform handles)
- [ ] Spatial index (R-tree for fast lookup)
- [ ] Rope-based text buffer (ropey crate)
- [ ] Multi-cursor + selections
- [ ] S1-S2 driven syntax highlighting
- [ ] Inlay hints from nom-lsp

### Phase 3 — Blocks + Panels (nom-blocks + nom-panels)
- [ ] 7 block types (prose, nomx, media, graph_node, drawing, table, embed)
- [ ] AFFiNE design tokens (73 variables, pixel-perfect)
- [ ] Sidebar (248px), toolbar (48px), preview, library, command palette, statusbar (24px)
- [ ] 5 unified modes (Code + Doc + Canvas + Graph + Draw)

### Phase 4 — Composition Engine (nom-compose)
- [ ] Backend dispatch (trait CompositionBackend)
- [ ] Video export (GPU scene → frame capture → FFmpeg pipe)
- [ ] Provider router (3-tier fallback, quota tracking)
- [ ] Task queue (async media generation with progress)
- [ ] Data extraction (PDF/DOCX → structured data)
- [ ] Semantic data layer (WrenAI MDL pattern)

### Phase 5 — Production Quality
- [ ] Sealed-trait linter framework (yara-x pattern)
- [ ] Incremental compilation (typst comemo pattern)
- [ ] OpenTelemetry tracing
- [ ] Collaboration (CRDT via Yrs, Huly pattern)

## v1 ARCHIVED (`.archive/nom-canvas-v1-typescript/`)

TypeScript v1 archived. All 3 CRITICAL issues (credentials, sandbox-eval, CSS grid) are moot — fresh Rust rewrite.

## Compiler Remaining

- [ ] GAP-1c body_bytes | GAP-2 embeddings | GAP-3 corpus | GAP-9/10 bootstrap
