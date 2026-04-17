# Remaining Work — Execution Tracker

> **CANONICAL TRACKING DOC — MAIN** (Planner/Auditor refreshes every cycle)
> **HEAD:** `1daa80e` on main (audit-fix CI in_progress) | **CI e2b7ecb + 5cb9a60:** GREEN ✅ | **Date:** 2026-04-17
> **Spec:** `docs/superpowers/specs/2026-04-17-nomcanvas-gpui-design.md` (719 lines) — canonical
> **Sibling docs:** `implementation_plan.md`, `nom_state_machine_report.md` (all 4 MUST stay in sync)
> **v1 archived** to `.archive/nom-canvas-v1-typescript/`. Phase 1 batch-1 + audit iteration landed (44/44 tests). batch-2 wave-1 (shaders + buffers + context) starting now.

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
### Phase 1 batch-1 AUDIT FIXES (landed in `1daa80e`, 44/44 tests)

**CRITICAL — all VERIFIED-CORRECT by iter-4 audit ✅:**
- [x] `scene.rs` BatchIterator: cutoff = min-other-kind-order, `advance_while(order <= cutoff)`; trace `[shadow@1, quad@5, shadow@10]` verified → 3 correct batches
- [x] `bounds_tree.rs` insert: `topmost_intersecting(bounds).map_or(1, |o| o.checked_add(1).expect(...))` with explicit overflow panic + doc; 50-rect non-overlap reuse verified
- [x] `scene.rs` PrimitiveBatch sprite variants: `texture_id` field on Mono/Poly struct variants; `finish()` sorts by `(order, tile_tex_id)`; iterator breaks on texture change

**HIGH — all VERIFIED ✅:**
- [x] `color.rs` Hsla: canonical `[0,360)` storage + both `from_degrees`/`from_normalized` constructors (divergent-but-ok vs Zed's `[0,1]`)
- [x] `styled.rs` all 40+ fluent setters migrated to `&mut self -> &mut Self`; `Sized` bound dropped; new `mut_ref_setters_compose_with_element_lifecycle` test proves borrow-release
- [x] `geometry.rs` Pixels: both `From<f32>` impls deleted; `Pixels(x)` is only ctor; consistent across ScaledPixels/DevicePixels
- [x] `taffy_layout.rs`: `try_request_layout -> Result<LayoutId, LayoutError>` with `#[from] taffy::TaffyError`; infallible `request_layout` wrapper preserves callers

**MEDIUM — 1 DONE opportunistically, 8 OPEN:**
- [ ] Add `request_measured_layout` + `NodeContext` measure closure — ⚠️ **blocks content-sized text/image elements**
- [ ] Add `SubpixelSprite` + `Surface` primitive kinds to Scene (8 vecs total) — ⚠️ **blocks crisp subpixel text rendering**
- [ ] Add `PlatformAtlas::remove(key)` for per-tile eviction
- [ ] Change `AtlasKey.bytes: Vec<u8>` → `Arc<[u8]>` or `Borrow`-based key
- [ ] Make `Scene` primitive fields `pub(crate)`; expose read-only accessors
- [ ] Replace BoundsTree recursive `walk()` with explicit `Vec<u32>` stack — stack-overflow risk on >1000 overlapping layers
- [ ] Add `half_perimeter` area heuristic to BoundsTree child selection
- [x] Wire `max_leaf` fast-path in `topmost_intersecting` — landed with overlap-aware rework ([bounds_tree.rs:227-231](nom-canvas/crates/nom-gpui/src/bounds_tree.rs#L227-L231))
- [ ] Fix `draw_element` to call `compute_layout` between phases OR document caller responsibility

**LOW — all still OPEN:**
- [ ] Add vertical-overflow check to InMemoryAtlas shelf wrap
- [ ] Remove unused `bytemuck` dep until batch-2 needs it
- [ ] Remove `pub` from `PrimitiveKind` (or document consumers)
- [ ] Collapse ~25 Styled setters into `style_setter!` macro
- [ ] Add `Debug` derives to `BatchIterator`, `ElementCx`, `LayoutEngine`
- [ ] Doc comments on `ElementCx.rem_size`/`scale_factor`, `AtlasTileRef.uv`, `NodeContext`

### Test gaps identified in iter-4 audit (add BEFORE batch-2 merges)
- [ ] Sprite **ABA texture pattern** test: `(tex_a, order_1), (tex_b, order_2), (tex_a, order_3)` → 3 distinct batches (proves no silent merge of non-adjacent same-texture runs)
- [ ] Hsla **hue boundary** + `rgb→hsl→rgb` round-trip (0°, 360° wrap, saturation/lightness extremes)
- [ ] `try_request_layout` **error path** — trigger taffy with NaN/infinite dims; assert `Err(LayoutError::Taffy(_))`
- [ ] Rewrite `pixels_explicit_construction_only` as **trybuild compile-fail** harness (current test is tautological — asserts `Pixels(42.0).0 == 42.0`, passes even with `From<f32>` re-added)
- [ ] Consider adding **proptest/quickcheck** fuzzing to BoundsTree (random insert sequences + invariant checks)

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

#### Part A — `nom-canvas-core` (infinite-canvas)

**Element model + mutation**
- [ ] `element.rs` — base `CanvasElement` trait: id, bounds, angle, stroke/fill, opacity, locked, group_id, frame_id, version, version_nonce, z_index, is_deleted, bound_elements (ref Excalidraw [types.ts:40-82](APP/Accelworld/services/other5/excalidraw-main/packages/element/src/types.ts#L40-L82))
- [ ] `mutation.rs` — `mutate_element()` (in-place, bump version+nonce) + `new_element_with()` (immutable spread for undo)
- [ ] `shapes/mod.rs` — 8-variant enum: Rectangle, Ellipse, Diamond, Line, Arrow (elbowed), Text, FreeDraw (points+pressures), Image (fileId+crop)

**Hit-testing + spatial index**
- [ ] `hit_testing.rs` — 2-stage: AABB fast-reject → per-shape distance (rect rounded-corner, diamond sides, ellipse closed-form, linear per-segment + bezier). Tolerance = stroke_width/2. Cache keyed on (point, id, version, version_nonce, threshold)
- [ ] `spatial_index.rs` — **Grid-based** (AFFiNE pattern) NOT R-tree. `DEFAULT_GRID_SIZE = 3000` model units. Element stored in all overlapping cells. `search(bound, filter) -> Vec<ElementId>` sorted

**Viewport + coord transforms**
- [ ] `viewport.rs` — `Viewport { center: Point, zoom: f32, size: Size }`; zoom bounds `0.1..=10.0` (deeper than AFFiNE's 6.0 for map-like views); signals: viewport_updated, zooming, panning
- [ ] `coords.rs` — separate translate+scale (NOT matrix): `to_model(vx,vy) = [viewport_x + vx/zoom/view_scale, ...]`; inverse `to_view`. `view_scale` handles DPI
- [ ] `zoom.rs` — zoom-to-point: `new_center = pivot + (center - pivot) * (prev_zoom / new_zoom)`. Wheel step 0.1 log-normalized; discrete 0.25
- [ ] `pan.rs` — space+drag, middle-mouse, trackpad two-finger. Auto-pan at edges ±30px/tick. Instant pan (no inertia), RAF-debounced animation only
- [ ] `fit.rs` — `fit_to_all(elements, padding)` + `fit_to_selection(ids, padding)`; zoom = min((w-pad)/bound_w, (h-pad)/bound_h); supports % (0..1) or absolute px

**Selection + marquee + transform**
- [ ] `selection.rs` — `Selection { selected_ids: HashSet<ElementId>, hovered: Option<ElementId>, pending: Option<Marquee> }`. Ignore locked+deleted. Group-expand. Frame-scoped selection
- [ ] `marquee.rs` — contain vs overlap modes; overlap tests bounds + linear point-in-bounds + edge intersection; frame-clip (marquee ∩ frame-bounds)
- [ ] `transform_handles.rs` — 8 resize (n,s,e,w,ne,nw,se,sw) + 1 rotation. Size per pointer: mouse 8 / pen 16 / touch 28, divided by zoom. Omit-sides param. Rotate positions via TransformationMatrix
- [ ] `snapping.rs` — grid snap + alignment guides (edges/centers/midpoints vs other elements + viewport center) + equal-spacing distribution. Threshold 8px/zoom. Render guides as overlay primitives
- [ ] `history.rs` — undo/redo via version snapshots; `HistoryEntry { id, timestamp, selection_before, selection_after, element_diffs }`

#### Part B — `nom-editor` (text editor over nom-gpui)

**Buffer + anchors + transactions**
- [ ] `buffer.rs` — single-buffer (defer MultiBuffer to Phase 4); rope via `ropey`; Lamport-clock TransactionId; start/end_transaction with transaction_depth counter (ref Zed [buffer.rs:99-110](APP/zed-main/crates/language/src/buffer.rs#L99-L110))
- [ ] `anchor.rs` — stable positions across edits; `(offset, bias: Left|Right)`; resolve via rope seek

**Selection + multi-cursor**
- [ ] `selection.rs` — `Selection { id, start, end, reversed, goal: SelectionGoal }`; `SelectionGoal::{None, Column(u32), HorizontalPosition(f32)}` — NOT raw sticky column
- [ ] `selections_collection.rs` — `SelectionsCollection { disjoint: Vec<Selection>, pending: Option<Selection> }`; `all()` merges overlaps on demand; public: newest, all_adjusted, count, change_selections
- [ ] `movement.rs` — left/right/saturating (ref [movement.rs:39-81](APP/zed-main/crates/editor/src/movement.rs#L39-L81)); up/down with goal preserved (lines 84-130); word via CharClassifier; bracket-match via tree-sitter; goal resets on horizontal move
- [ ] `editing.rs` — `edit(ranges, texts)` sorts reverse-offset-order then applies atomically via `transact(|| ...)`; autoindent via AutoindentMode enum

**Tree-sitter + highlight**
- [ ] `syntax_map.rs` — `SumTree<SyntaxLayerEntry { tree, language, offset }>`; incremental `sync()` on edit re-parses only affected regions (ref [syntax_map.rs:29-166](APP/zed-main/crates/language/src/syntax_map.rs#L29-L166))
- [ ] `highlight.rs` — tree-sitter queries on visible ranges; map capture_name → HighlightId via theme; emit `HighlightStyle { color, weight, italic }` spans

**Inlays + LSP**
- [ ] `inlay_map.rs` — separate from rope; `SumTree<Transform { Isomorphic(len) | Inlay(InlayId, len) }>`; buffer↔display offset mapping
- [ ] `inlay_hints.rs` — LSP fetch on visible range; `hint_chunk_fetching: HashMap<Range, Task>`; debounce edit+scroll separately; invalidate affected on edit
- [ ] `lsp_bridge.rs` — bridge to existing `nom-compiler/crates/nom-lsp` for hover, completion, inlay hints, diagnostics

**Display pipeline**
- [ ] `wrap_map.rs` — soft-wrap `SumTree<Transform>`; background re-wrap with `interpolated: true` flag during flight; O(log N) row seek
- [ ] `tab_map.rs` — pre-compute tab widths; expand in display layer
- [ ] `display_map.rs` — pipeline: Buffer → InlayMap → FoldMap → TabMap → WrapMap → render (ref Zed display_map)
- [ ] `line_layout.rs` — width measurement via nom-gpui text system (cosmic-text); lazy, background task, stale snapshot during typing

#### Part C — test targets

- [ ] Hit-test golden files — per-shape boundary points ±tolerance
- [ ] Marquee contain vs overlap — 2×4 shape grid, assert correct subset per mode
- [ ] Zoom-to-point invariant — pivot at (100,100), zoom 1.0→3.0, assert pivot unchanged
- [ ] Coord round-trip — `to_view(to_model(v)) == v` for 1000 random points × 5 zoom levels
- [ ] Grid spatial index — 100k elements, 1000 random queries, verify linear-scan parity
- [ ] Multi-cursor reverse-offset edit — 3 cursors P1<P2<P3 insert "a"; final P1+1, P2+2, P3+3
- [ ] Goal-column preservation — down-down-left-down; assert goal resets on horizontal
- [ ] Selection merge — 2 cursors whose `select_word` results overlap → single merged
- [ ] Inlay offset mapping — insert inlay at display 10; buffer 10 still resolves; pre-inlay buffer edit shifts both
- [ ] Incremental tree-sitter — edit 1 char; assert only affected layer re-parses (instrumentation counter)

#### Part D — NON-GOALS (do NOT adapt)

- Zed GPUI Entity/Context + AsyncAppContext (no runtime yet — use Rc/RefCell + channels)
- Zed MultiBuffer (single-buffer first; defer to Phase 4)
- Excalidraw RoughJS (we're GPU shaders)
- Excalidraw DOM event coords (native winit instead)
- AFFiNE RxJS Subjects (native signals / tokio::sync::watch)
- AFFiNE CSS transforms + Lit components (wgpu-native)

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
