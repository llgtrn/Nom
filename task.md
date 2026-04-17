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

### Phase 1 batch-2 wave-1 (LANDED in `205aea9`, 59/59 tests) — AUDIT FIX-WAVE NEEDED

**CRITICAL — block wave-2 start:**
- [ ] **`lib.rs` re-export `GpuContext`** — add `pub mod context;` at [lib.rs:20-21](nom-canvas/crates/nom-gpui/src/lib.rs#L20-L21). Currently `context.rs` is dead code, unreachable from crate root (2-line fix)
- [ ] **Add `order: u32` to `QuadInstance`** — [quad.wgsl:40-48](nom-canvas/crates/nom-gpui/src/shaders/quad.wgsl#L40-L48) — WGSL struct + Rust Pod struct. Breaks Z-sorted rendering without this
- [ ] **Add `order: u32` + `texture_sort_key` to `MonoSpriteInstance`** — [mono_sprite.wgsl:37-43](nom-canvas/crates/nom-gpui/src/shaders/mono_sprite.wgsl#L37-L43) — also add missing `transformation: TransformationMatrix` field for rotated/scaled glyphs
- [ ] **Implement 20-frame overflow guard in `InstanceBuffer`** — [buffers.rs:116-126](nom-canvas/crates/nom-gpui/src/buffers.rs#L116-L126). Add `overflow_frames: u32` field; `begin_frame()` resets on successful write; `write()` increments when capacity + max exceeded; panic at 20. Without this, capacity exhaustion silently drops draw calls

**HIGH — fix during same fix-wave to avoid breaking-change churn:**
- [ ] **Consolidate 4 WGSL files → 1 `shaders.wgsl`** — currently `common.wgsl` + `quad.wgsl` + `mono_sprite.wgsl` + `underline.wgsl` with common header duplicated 3×
- [ ] **Add `GammaParams` uniform at `@group(0) @binding(1)`** — [pipelines.rs:72-84](nom-canvas/crates/nom-gpui/src/pipelines.rs#L72-L84). `{ gamma_ratios: vec4, grayscale_enhanced_contrast: f32, subpixel_enhanced_contrast: f32 }`, FRAGMENT visibility. Prevents breaking change when wave-2 text rendering lands
- [ ] **Fix `recover()` Arc-staleness** — [context.rs:138-151](nom-canvas/crates/nom-gpui/src/context.rs#L138-L151). Either wrap in `Arc<RwLock<Device>>` OR add generation counter OR document clone-invalidation invariant prominently
- [ ] **Use `NonZeroU64::new(size_of::<GlobalParams>())` for `min_binding_size`** — [pipelines.rs:80](nom-canvas/crates/nom-gpui/src/pipelines.rs#L80). Bind-time validation beats draw-time crash
- [ ] **Widen texture + sampler visibility to `VERTEX_FRAGMENT`** — [pipelines.rs:121, :129](nom-canvas/crates/nom-gpui/src/pipelines.rs#L121). Currently FRAGMENT-only; will break if vertex shader needs atlas coords
- [ ] **Add `hsla_to_rgba()` WGSL helper** + port Zed's `blend_color()`, `enhance_contrast()`. Keep themes HSL-native on GPU side; eliminates CPU↔GPU color mismatch
- [ ] **Rename `clip_bounds` → `content_mask`** everywhere to match Zed's canonical naming (cheap now, expensive later)

**MEDIUM — opportunistic:**
- [ ] Move `underline.wgsl` out of wave-1 (scope creep — was deferred to wave-3)
- [ ] Change `is_device_lost()` ordering from `SeqCst` → `Acquire`/`Release` pair — [context.rs:129](nom-canvas/crates/nom-gpui/src/context.rs#L129)
- [ ] Document per-pipeline `ShaderModule` as intentional divergence from iter-5 spec (or consolidate)

**TEST GAPS — add before wave-2 compounds them (grade was D+):**
- [ ] **Shader module creation test** — `wgpu::Device::create_shader_module(include_str!("shaders.wgsl"))` per shader file; assert no validation error (requires adapter + `pollster::block_on`)
- [ ] **Rust mirrors of `to_ndc` + `unit_vertex` + `rounded_rect_sdf`** — pure-math boundary tests (center→NDC (0,0), origin→NDC (-1,1), corner inside SDF < 0, edge == 0, outside > 0)
- [ ] **BindGroupLayout compat test** — create layouts matching shader expectations; assert entry counts + binding types
- [ ] **20-frame overflow guard test** — `#[should_panic]` after exactly 20 frames at max capacity; no panic at frame 19
- [ ] **Fix tautological test** — `context_creates_when_adapter_available` asserts `... || true` (always passes). Remove `|| true`, assert real invariant
- [ ] **Annotate skip-if-no-GPU tests** with visible skip markers — currently 3 context tests silently skip in headless CI

### Phase 1 batch-2 wave-2 — LANDED (this cycle, 99/99 tests)

- [x] MVP pipelines: `pipelines.rs` — 3 `wgpu::RenderPipeline` (quads, mono_sprites, underlines) + shared globals_bgl + instances_bgl + sprite_instances_bgl; BlendState::PREMULTIPLIED_ALPHA; TriangleStrip topology; 3 tests ✅ (shadows/path_rasterization/paths/subpixel_sprites/poly_sprites deferred to batch-3)
- [x] Shaders already in wave-1 (common+quad+mono_sprite+underline WGSL) — sufficient for MVP pipelines ✅
- [x] `buffers.rs` landed wave-1: `InstanceBuffer` 2× growth + max_buffer_size clamp + storage-offset alignment; 14 pure-math tests ✅
- [x] `wgpu_atlas.rs` (661 lines): GPU-backed atlas, `etagere::BucketedAtlasAllocator` per kind (R8Unorm/Bgra8Unorm/Bgra8Unorm), 1024² default → `max_texture_dimension_2d` clamp, `PendingUpload` → `queue.write_texture()` via `flush_uploads()`, slab-overflow bumps `AtlasTextureId.index`; 6 tests ✅
- [x] `text.rs` (386 lines): cosmic-text 0.12 `ShapeLine::new_in_buffer` → `layout_to_buffer` with `Shaping::Advanced`; swash 0.2 `ScaleContext` + `Render::new` with `Format::Alpha` (mono) or `Format::subpixel_bgra()`; `FontId`/`GlyphId`/`SubpixelVariant`/`RenderGlyphParams`; 3 tests ✅
- [x] `window.rs` (315 lines): `WindowSurface` with format negotiation (Bgra8→Rgba8→any), alpha (PreMultiplied→Opaque→any), present mode (Fifo default = OS-vsync); `wgpu::Surface<'static>` via `create_surface_unsafe` (one scoped `#![allow(unsafe_code)]` with SAFETY doc); resize + acquire; 9 tests ✅
- [x] `frame_loop.rs` (270 lines): winit 0.30 `App<H: FrameHandler>` + `ApplicationHandler` impl; `resumed` lazy-creates window+surface; `window_event` handles RedrawRequested (acquire → draw → present), Resized, CloseRequested; device-loss recovery via `pollster::block_on(gpu.recover())`; 4 tests ✅
- [x] Audit MEDIUM: `SubpixelSprite` added as 7th primitive kind (finish sort, BatchIterator wiring, PrimitiveBatch variant, PrimitiveKind) ✅
- [x] Audit MEDIUM: `request_measured_layout` + real `NodeContext { measure: Option<MeasureFn> }` using taffy 0.6 `new_leaf_with_context` + `compute_layout_with_measure` ✅
- [x] Audit MEDIUM: `Scene` fields `pub(crate)` + 7 read-only accessors ✅
- [x] Test-gap audit: ABA sprite texture pattern, Hsla 360° wrap + saturation=0 gray + lightness 0/1 extremes, `try_request_layout` NaN observation, `assert_not_impl_all!(Pixels: From<f32>)` via `static_assertions` ✅

### Phase 1 batch-2 wave-3 — LANDED (this cycle, 113 tests: 104 unit + 9 integration)

- [x] `renderer.rs` (637 LOC, 6 tests) — main draw loop: `atlas.flush_uploads()`, writes GlobalParams uniform, resets InstanceBuffers, iterates `scene.batches()`, dispatches Quads/MonochromeSprites/Underlines to pipelines; pre-collects atlas TextureViews before render pass (wgpu borrow-lifetime requirement); `draw(0..4, 0..N)` TriangleStrip×N instances. GpuQuad=96B, GpuMonoSprite=64B, GpuUnderline=64B (WGSL-layout tests). Shadows/SubpixelSprites/PolychromeSprites/Paths silently skipped (batch-3).
- [x] Feature flags `native` (default = winit) vs `web` (wasm32 = no winit) — both `cargo check` (default) and `cargo check --no-default-features --features web` succeed. `frame_loop` + `window` gated `#[cfg(feature = "native")]`.
- [x] Hit-testing wiring — `Scene::hit_test(point) -> Option<HitResult { kind, index, order }>` via O(N) brute-force scan across all 7 collections (`check_collection!` macro). 3 tests. BoundsTree integration deferred to batch-3.
- [x] Element state storage — `ElementStateMap { map: HashMap<ElementId, Box<dyn Any>> }` attached to `App<H>` as `pub element_state`. Typed `get_or_insert<T, F>()` + `remove()` + `len()`. 2 tests.
- [x] Integration tests — `tests/gpu_integration.rs` (351 LOC, 9 tests all green on Windows DX12): atlas round-trip + slab overflow (Gpu + InMemory variants), buffer-growth doubles-and-clamps, Scene batch iterator exhausts cleanly, headless clear-to-color readback, pipelines construct on Bgra8Unorm + Rgba8Unorm, InMemoryAtlas shelf fill.

### Phase 1 batch-2 wave-3 deferred / wave-4 backlog

- [ ] Remaining 5 pipelines (shadows, subpixel_sprites, poly_sprites, path_rasterization, paths) — batch-3
- [ ] `device_lost.rs` — factor in-place recovery out of `App::window_event` into reusable module
- [ ] BoundsTree integration in Scene for O(log N) hit_test (currently O(N) brute force)
- [ ] Wire `App::element_state` into `FrameHandler` callback so elements can read/write typed state cross-frame
- [ ] Pixel-diff integration tests (current tests assert "no panic"; need pixel correctness)
- [x] **CI HEADLESS regression fix** — added `nom_gpui::should_skip_gpu_tests()` helper (checks `NOM_SKIP_GPU_TESTS` env OR missing `DISPLAY`+`WAYLAND_DISPLAY` on Linux). Guard applied to 22 GPU/winit-dependent tests across 7 files (context/frame_loop/window/pipelines/renderer/wgpu_atlas/gpu_integration). Windows 113/113 still green; ubuntu-latest CI now skips GPU tests and runs only CPU-safe ones.

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

Reference: AFFiNE `blocksuite/affine/` (blocks + model + std). Use `defineBlockSchema()` pattern in Rust via `define_block_schema!` macro. Every block = schema + renderer + event handler + transformer + transitions.

#### Part A — `nom-blocks` crate (7 block types)

**Shared infrastructure (do first)**
- [ ] `block_schema.rs` — `define_block_schema!` macro: flavour, props, metadata {version, role: Content|Hub, parent: &[&str], children: &[&str]}. Enforce parent/children at insert time (ref AFFiNE [paragraph-model.ts:29-56](APP/AFFiNE-canary/blocksuite/affine/model/src/blocks/paragraph/paragraph-model.ts#L29-L56))
- [ ] `block_model.rs` — `BlockModel<Props>` base: id, flavour, props, children Vec<BlockId>, meta {created_at, created_by, updated_at, updated_by, comments} (ref AFFiNE BlockMeta type mixin)
- [ ] `block_transformer.rs` — `trait BlockTransformer { fn from_snapshot(&self, snap: &Snapshot) -> Result<BlockData>; fn to_snapshot(&self, data: &BlockData) -> Snapshot; }` for asset + schema-version migration (ref AFFiNE [image-transformer.ts](APP/AFFiNE-canary/blocksuite/affine/model/src/blocks/image/image-transformer.ts))
- [ ] `block_selection.rs` — union of per-block-type Selection variants (TextSelection, BlockSelection, ImageSelection, DatabaseSelection); blockId + selection range; multi-block via Vec (ref AFFiNE [shared/src/selection/](APP/AFFiNE-canary/blocksuite/affine/shared/src/selection/))
- [ ] `block_config.rs` — `ConfigExtensionFactory<BlockConfig>(flavour)`-style runtime configuration injection per block type (ref AFFiNE [paragraph-block-config.ts:8-9](APP/AFFiNE-canary/blocksuite/affine/blocks/paragraph/src/paragraph-block-config.ts#L8-L9))

**prose block (text paragraph, lists, headings)**
- [ ] schema: `flavour="nom:prose"`, props={text: RichText, text_align: TextAlign, kind: ProseKind (text|h1..h6|quote|bulleted_list|numbered_list|todo), collapsed: bool}, metadata={role: Content, parent: ["nom:note", "nom:callout", "nom:prose"], children: []}
- [ ] renderer: inline text via nom-gpui text rasterization; bullet/number glyph via atlas; collapsed state hides children
- [ ] transitions: prose→heading via `toH1()` etc. (clone text, delete, add new — ref AFFiNE [convert-to-numbered-list.ts](APP/AFFiNE-canary/blocksuite/affine/blocks/list/src/commands/convert-to-numbered-list.ts))
- [ ] events: Enter at end → sibling prose; Tab → indent (change parent); Backspace at start → merge with prev sibling

**nomx block (Nom source code with inline compile/preview)**
- [ ] schema: `flavour="nom:nomx"`, props={source: String, lang: "nom"|"nomx", wrap: bool, caption: Option<String>, line_numbers: bool}, metadata={role: Content, parent: ["nom:note"], children: []}
- [ ] renderer: tree-sitter highlighting from nom-editor; gutter with line numbers + run/eval buttons; inline preview pane for output
- [ ] Compiler integration — direct call into nom-compiler crates (no IPC); incremental re-compile on edit; surface errors as inline decorations

**media block (image/video/audio — split into Image + Attachment variants)**
- [ ] schema (Image): `flavour="nom:image"`, props={source_id: BlobId, xywh: String, rotate: f32, width: u32, height: u32, caption: Option<String>, index: FractionalIndex}, metadata={role: Content, parent: ["nom:note", "nom:surface"], children: []}
- [ ] schema (Attachment): `flavour="nom:attachment"`, props={source_id: BlobId, name: String, size: u64, mime: String, embed: bool, caption: Option<String>}
- [ ] renderer: Image via nom-gpui PolychromeSprite (atlas-backed); Attachment as icon + filename for non-embedded
- [ ] blob I/O: `BlobManager` with content-addressed storage; transformer maps source_id → blob bytes (ref AFFiNE [attachment-transformer.ts](APP/AFFiNE-canary/blocksuite/affine/model/src/blocks/attachment/attachment-transformer.ts))

**graph_node block (DAG node for compute graphs)**
- [ ] schema: `flavour="nom:graph_node"`, props={xywh: String, index: FractionalIndex, inputs: Vec<Port>, outputs: Vec<Port>, kind: String (operator name), config: JsonValue, child_element_ids: Vec<BlockId>}, metadata={role: Hub, parent: ["nom:surface"], children: ["nom:prose", "nom:nomx", "nom:media"]}
- [ ] renderer: `GfxBlockComponent` equivalent — viewport-aware via nom-canvas-core Viewport signal; transform via xywh + zoom (ref AFFiNE [frame-block.ts:53-70](APP/AFFiNE-canary/blocksuite/affine/blocks/frame/src/frame-block.ts#L53-L70))
- [ ] port connections: edge primitives in nom-gpui scene; hit-testing for connect/disconnect

**drawing block (freehand sketch over surface)**
- [ ] schema: `flavour="nom:drawing"`, props={xywh: String, strokes: Vec<Stroke>, index: FractionalIndex}. Stroke = {points: Vec<(f32, f32, f32)> (x,y,pressure), color: Rgba, width: f32}
- [ ] renderer: stroke tessellation → nom-gpui Path primitives (2-pass path_rasterization pipeline)
- [ ] input: pointer events from nom-canvas-core; pressure from tablet APIs; simplify/smooth on stroke-end

**table block (structured data grid)**
- [ ] schema: `flavour="nom:table"`, props={columns: Vec<Column>, rows: Vec<Row>, views: Vec<View>, title: RichText}. Column = {id, name, kind: ColumnKind (text|number|select|date|relation)}. View = {kind: ViewKind (grid|kanban|calendar), ...}
- [ ] renderer: grid view via nom-gpui primitives; cell editing delegates to per-column-kind renderer
- [ ] data-view orchestrator: multi-view switching (ref AFFiNE [database-block.ts](APP/AFFiNE-canary/blocksuite/affine/blocks/database/src/database-block.ts) + `@blocksuite/data-view`)

**embed block (external content / iframe-like)**
- [ ] schema: `flavour="nom:embed"`, props={url: String, title: Option<String>, description: Option<String>, thumbnail: Option<BlobId>, kind: EmbedKind (iframe|bookmark|linked_doc|synced_doc|youtube|figma)}, metadata={role: Content, parent: ["nom:note", "nom:surface"], children: []}
- [ ] renderer: Bookmark = preview card via nom-gpui; iframe/youtube = placeholder with "open externally" (no webview in Nom); linked_doc/synced_doc = reference resolution + recursive render

#### Part B — `nom-panels` crate (shell chrome)

- [ ] `sidebar.rs` — 248px fixed width; document tree + search + recent; collapsible
- [ ] `toolbar.rs` — 48px height; block-type-specific contextual actions; file-level ops
- [ ] `preview.rs` — right-pane for nomx/media preview; reactive on source edit
- [ ] `library.rs` — reusable components/snippets browser; drag-to-insert
- [ ] `command_palette.rs` — Cmd/Ctrl+K; fuzzy search over commands + blocks + files; action dispatcher
- [ ] `statusbar.rs` — 24px height; compile status, cursor position, git state, diagnostics count
- [ ] `mode_switcher.rs` — 5 unified modes toggle (Code | Doc | Canvas | Graph | Draw); persist per-document preference

#### Part C — `nom-theme` crate (AFFiNE tokens wired)

- [ ] `tokens.rs` — 73 design variables as Rust consts + HSLA values (NOT CSS vars, NOT @toeverything/theme)
- [ ] Inter + Source Code Pro font loading into nom-gpui atlas; fallback chain
- [ ] Lucide icons: 24px SVG viewBox → tessellate into Path primitives OR rasterize into PolychromeSprite atlas
- [ ] dark/light mode toggle via token swap + redraw (no DOM repaint)

#### Part D — ADOPT patterns + SKIP patterns

**ADOPT from AFFiNE:**
- Schema-first block declaration with metadata-driven hierarchy
- Transformer abstraction for serialization + asset I/O
- Selection-as-Service (per-block-type Selection variants with blockId)
- ConfigExtensionFactory for runtime injection

**SKIP (not applicable to Rust wgpu):**
- Lit web components + shadow DOM (no DOM in Nom)
- CSS-based theming via @toeverything/theme (use Rust consts)
- floating-ui DOM positioning (use viewport-relative gfx math)

#### Part E — Phase 3 test targets

- [ ] Schema validation — invalid parent/children combos rejected at insert
- [ ] Transformer round-trip — each of 7 block types: model → snapshot → model (bit-identical)
- [ ] Block-to-block conversion — prose h1→bulleted_list preserves text + bumps version
- [ ] Multi-block selection — 3-block range select/copy/delete; assert contiguous
- [ ] Graph_node port connection — connect output→input, render edge; disconnect cleans up
- [ ] Drawing smoothing — 1000-point raw stroke → simplified stroke preserves start/end/key turns
- [ ] Table view switching — grid → kanban preserves selected cell/row context
- [ ] Embed URL parsing — detect youtube/figma/generic; correct EmbedKind variant

### Phase 4 — `nom-graph-v2` + `nom-compose` (universal composition)

Reference reads (iter-7): ComfyUI execution.py + graph.py + caching.py, n8n workflow-execute.ts + expression-sandboxing.ts, typst lib.rs + frame.rs, Remotion DeepWiki.

#### Part A — `nom-graph-v2` crate (DAG execution)

- [ ] `topology.rs` — Kahn's algorithm with `block_count: HashMap<NodeId, usize>`; lazy cycle detection at exec time (ref ComfyUI [graph.py:107-193](APP/Accelworld/services/other2/ComfyUI-master/comfy_execution/graph.py#L107-L193))
- [ ] `execution.rs` — pull-based loop: stage ready → execute → decrement downstream; `unblockedEvent` semaphore for external async (ref ComfyUI [execution.py:704-786](APP/Accelworld/services/other2/ComfyUI-master/execution.py#L704-L786))
- [ ] `node_schema.rs` — `Node { input_types, function, return_types, output_is_list, side_effects }` declarative schema
- [ ] `fingerprint.rs` — `Node::fingerprint_inputs(&inputs) -> u64` with constant-only inputs; hash class_type + IS_CHANGED + ancestor signatures (ref ComfyUI [execution.py:54-95](APP/Accelworld/services/other2/ComfyUI-master/execution.py#L54-L95))
- [ ] `cache.rs` — `trait CacheBackend { get, set, poll }` + 4 impls: None (no-op), Lru (max_size), RamPressure (age-biased OOM), Classic (default, hierarchical signature) (ref ComfyUI [caching.py:103-563](APP/Accelworld/services/other2/ComfyUI-master/comfy_execution/caching.py#L103-L563))
- [ ] `subcache.rs` — hierarchical subcache for subgraph expansion, keys `(parent_id, child_id_set)` (ref [caching.py:361-408](APP/Accelworld/services/other2/ComfyUI-master/comfy_execution/caching.py#L361-L408))
- [ ] `progress.rs` — `ProgressHandler` trait + channel dispatch; nodes call `ctx.update(value, max)` + `ctx.check_interrupted()` cooperatively
- [ ] `cancel.rs` — `InterruptFlag: Arc<AtomicBool>`; InterruptError raised + caught; `ExternalBlocker` for async pending
- [ ] `error.rs` — fail-fast per-node + preserve successful-upstream cache; `OUTPUT_NODE` marker for UI precedence

#### Part B — `nom-compose` crate (backend dispatch)

- [ ] `backend_trait.rs` — `trait CompositionBackend { fn kind() -> NomKind; async fn compose(&self, spec, progress, interrupt) -> Result<Output>; }`
- [ ] `dispatch.rs` — `ComposeDispatcher { backends: HashMap<NomKind, Arc<dyn CompositionBackend>> }`; routes `nom compose <spec>` by kind
- [ ] `task_queue.rs` — tokio async queue for >10s ops; per-task progress channel + cancel handle; per-backend concurrency cap (video 2, image 4, data 8)
- [ ] `provider_router.rs` — 3-tier Subscription→Cheap→Free fallback; per-vendor quota `{ used, limit, reset_at }`; format translation Claude↔OpenAI↔Gemini
- [ ] `credential_store.rs` — AES-encrypted on-disk JSON; `get_credential(kind, id)` decrypted at runtime, never in Spec (ref n8n [credentials.ts:48-65](APP/Accelworld/services/automation/n8n/packages/core/src/credentials.ts#L48-L65))

#### Part C — 6 concrete backends

**media/video — Remotion+typst-parallelize adapted**
- [ ] `media_video_backend.rs` — nom-gpui `Scene` per frame; parallel rasterize via rayon; `mpsc::channel(32)` → FFmpeg stdin; backpressure via bounded channel
- [ ] Frame format flag: PNG (default, via nom-gpui offscreen TextureView) vs raw RGBA; pipe = stdin default, named-pipe fallback for Windows
- [ ] `stitch_frames_to_video(frames_iter, fps, codec) -> Mp4Output` — mirrors Remotion `stitchFramesToVideo`

**media/image — diffusion + tiling**
- [ ] `media_image_backend.rs` — dispatch to on-device (candle/burn) OR cloud (Open-Higgsfield 200+ models); cost+quality-based selection
- [ ] Tile-based upscale: Kahn-scheduled DAG of tile nodes via nom-graph-v2

**screen/web — ToolJet+Dify adapted**
- [ ] `screen_web_backend.rs` — consume `Screen` kind (widgets + layout + data bindings) → HTML+WASM bundle
- [ ] 55-widget catalog from ToolJet reimplemented as Nom primitives

**screen/native — nom-llvm**
- [ ] `screen_native_backend.rs` — delegate to `nom-compiler/crates/nom-llvm` (existing); emit ELF/Mach-O/PE via LLVM

**data/extract — opendataloader-pdf XY-Cut++**
- [ ] `data_extract_backend.rs` — `Spec { source: Path, schema: Option<Schema> }` → JSON/CSV; XY-Cut++ layout reconstruction

**data/query — WrenAI MDL**
- [ ] `data_query_backend.rs` — `Spec { mdl, query: NomProse }`; MDL grounds LLM-generated SQL; schema validation before exec

**concept/document — typst + comemo pattern**
- [ ] `concept_doc_backend.rs` — Nom prose/blocks → `Content` tree → memoized layout → Frame hierarchy → PDF/PNG/SVG
- [ ] `Tracked<dyn NomWorld>` trait wrapper; port memoization primitive (do NOT depend on `comemo` crate — see SKIP list)
- [ ] `Frame { size, items: Arc<LazyHash<Vec<(Point, FrameItem)>>>, kind }` — immutable hash-indexed (ref typst [frame.rs:18-30](APP/Accelworld/services/other5/typst-main/crates/typst-library/src/layout/frame.rs#L18-L30))
- [ ] Constraint-based multi-pass layout with introspection stabilization check
- [ ] Parallel layout of independent items via rayon (ref typst `Engine::parallelize` [engine.rs:51-100](APP/Accelworld/services/other5/typst-main/crates/typst/src/engine.rs#L51-L100))

**scenario/workflow — n8n adapted**
- [ ] `scenario_workflow_backend.rs` — pull-based stack execution; single-node-at-a-time; `waiting_execution: HashMap<NodeId, PendingInputs>` for data-dependent queueing
- [ ] Retry loop `{ retry_on_fail, max_tries: 0..=5, wait_between_tries_ms: 0..=5000 }` (ref n8n [workflow-execute.ts:1609-1705](APP/Accelworld/services/automation/n8n/packages/core/src/execution-engine/workflow-execute.ts#L1609-L1705))
- [ ] `continue_on_fail` + `on_error: { ContinueRegularOutput | ContinueErrorOutput | StopWorkflow }` (ref n8n :1854-1906)
- [ ] Webhook resume: persist `(node_execution_stack, run_data)`; payload triggers `run_partial_workflow2()`

#### Part D — `.nom` user-script AST sandbox (shared across backends)

- [ ] `sandbox/isolate.rs` — wasmtime WASM instance OR `rusty_v8` isolate; 128MB mem limit, 5000ms timeout (ref n8n isolated-vm-bridge.ts)
- [ ] `sandbox/ast_visitor.rs` — walk Nom AST pre-exec; apply 3 sanitizers (ref n8n [expression-sandboxing.ts:76-232](APP/Accelworld/services/automation/n8n/packages/workflow/src/expression-sandboxing.ts#L76-L232))
- [ ] `sandbox/this_sanitizer.rs` — replace `this` with `EMPTY_CONTEXT { process: {}, require: {}, module: {}, Buffer: {} }`; `.bind(EMPTY_CONTEXT)` on all fn exprs
- [ ] `sandbox/prototype_sanitizer.rs` — Proxy-wrap `Object` static, return `undefined` for: defineProperty, defineProperties, setPrototypeOf, getPrototypeOf, getOwnPropertyDescriptor(s), __defineGetter__, __defineSetter__, __lookupGetter__, __lookupSetter__
- [ ] `sandbox/dollar_validator.rs` — `$` identifier only as fn call or property access (not bare); matches Nom `$var` scope convention
- [ ] `sandbox/allowlist.rs` — allowed: `DateTime`/`Duration`/`Interval`, `extend()`, lazy-proxy read-only user data. Blocked: process, require, module, Buffer, global, globalThis, `Error.prepareStackTrace` (V8 RCE vector)

#### Part C-bis — backfill from blueprint §12 (5 backends missed in iter-7, added in iter-9)

**media/storyboard — waoowaoo 4-phase:**
- [ ] `media_storyboard_backend.rs` — 4 phases: Planning → (Cinematography ∥ Acting via `tokio::join!`) → Detail enrichment → Asset-queue → FFmpeg composite
- [ ] `NomMediaPhase = Decompose|Cinematography|Acting|Detail|Ffmpeg` enum with retry-once-per-phase + field validation (ref waoowaoo [storyboard-phases.ts:323-371](APP/Accelworld/services/media/waoowaoo/server/processing/storyboard-phases.ts#L323-L371))
- [ ] `PhaseResult { clip_id, plan_panels, photography_rules, acting_directions }` typed handoff
- [ ] Template-replace prompt construction (NOT Jinja, NOT LLM-generated) for determinism

**media/novel→video — ArcReel adapted via nom-intent (NOT Claude SDK):**
- [ ] `media_novel_video_backend.rs` — 5 agents via `nom-intent` ReAct: `novel_analyst → script_writer → character_designer → storyboard_artist → video_composer`
- [ ] Skill vs Subagent boundary: Skills = deterministic Rust fns, Subagents = reasoning via nom-intent (ref ArcReel AGENTS.md:196-198)
- [ ] Typed handoffs: `NovelAnalysis → ScriptJSON → CharacterDesign → StoryboardPanels → VideoArtifact`
- [ ] Session checkpoints in `nom-dict` with `EntryKind::AgentSession` for mid-pipeline resume
- [ ] `UsageTracker` per-call: `{ agent, model, input_tokens, output_tokens, usd_cost }`
- [ ] **Zero foreign identities**: all external LLM calls via `MediaVendor` facade (Part G) — never import `anthropic-*` SDKs directly

**media/audio — synthesis + codec:**
- [ ] `media_audio_backend.rs` — `Spec { text, voice_id, sample_rate, format: Flac|Aac|Mp3|Opus }`
- [ ] On-device (whisper + rodio) or cloud dispatch; cost+quality-based
- [ ] Timing-alignment layer for lip-sync pairing with media/video

**data/transform — Polars MVP, new `nom-data` crate (NO polars dep):**
- [ ] `series.rs` — `Series<T> = (Arc<Vec<T>>, Arc<Bitmap>)`; bit-packed null bitmap (ref polars [bitmap/immutable.rs:56-68](APP/Accelworld/upstreams/polars/crates/polars-arrow/src/bitmap/immutable.rs#L56-L68))
- [ ] `chunked_array.rs` — `ChunkedArray<T> = Vec<Arc<Vec<T>>>` with `rechunk()` (ref polars [chunked_array/mod.rs:138-147](APP/Accelworld/upstreams/polars/crates/polars-core/src/chunked_array/mod.rs#L138-L147))
- [ ] `dtype.rs` — MVP: `{Int8-64, UInt8-64, Float32/64, Bool, String, Date, List, Null}` (skip Categorical/Enum/Decimal v1)
- [ ] `simd.rs` — `std::simd::prelude` direct (NOT arrow2/wide); AVX-512 fast-path `_mm512_maskz_compress_epi8` for x86_64 + scalar fallback (ref polars [comparisons/simd.rs:2](APP/Accelworld/upstreams/polars/crates/polars-compute/src/comparisons/simd.rs#L2))
- [ ] `plan.rs` — `enum DslPlan { Scan, Filter, Project, GroupBy, Join, Sort }` logical IR
- [ ] `optimizer.rs` — MVP: predicate pushdown + projection pushdown only (skip 20+ rules) (ref [predicate_pushdown/mod.rs:56-90](APP/Accelworld/upstreams/polars/crates/polars-plan/src/plans/optimizer/predicate_pushdown/mod.rs#L56-L90))
- [ ] `join.rs` — hash join default (build on smaller, probe larger) (ref [hash_join/mod.rs:29-48](APP/Accelworld/upstreams/polars/crates/polars-ops/src/frame/join/hash_join/mod.rs#L29-L48))
- [ ] `parallel.rs` — rayon `POOL` wrapper (ref polars [lib.rs:39-96](APP/Accelworld/upstreams/polars/crates/polars-core/src/lib.rs#L39-L96))
- [ ] `data_transform_backend.rs` — `Spec { input: DataSource, pipeline: NomProse }` → CompositionBackend trait wiring

**media/3D — mesh composition (glTF 2.0):**
- [ ] `media_3d_backend.rs` — `Spec { mesh: NomMeshSource, materials, animations, export: Gltf|Glb }`
- [ ] `nom-media::MeshGeometry` kind integration
- [ ] glTF 2.0 Rust-native writer (no C bindings — `gltf-json` crate or custom)
- [ ] Scene composition: combine `MeshGeometry` + `Material` + `AnimationClip` into single scene

#### Part F-bis — FallbackStrategy 3 variants (blueprint §15, iter-10)

- [ ] `provider_router.rs` add `enum FallbackStrategy { Fallback, RoundRobin, FillFirst }`
- [ ] `combo_strategies: HashMap<String, FallbackStrategy>` for per-combo routing overrides
- [ ] Default `Fallback`; user-configurable per vendor combo via credentials panel

#### Part B-bis — `nom-collab::transactor` (blueprint §16, iter-10)

- [ ] `transactor.rs` — immutable append-only event log: `Vec<Transaction { timestamp, client_id, doc_id, yrs_update: Uint8Array }>` to SQLite or append-only file
- [ ] Separate from Y.Doc snapshot in `persistence.rs` (transactor = every update for audit/time-travel; snapshots = periodic checkpoints)
- [ ] Retention: keep 30 days full + daily snapshots (configurable)
- [ ] `replay(doc_id, until: Timestamp) -> Y.Doc` API for time-travel debugging + audit queries

#### Part C-ter — VideoComposition concrete struct (blueprint §18, iter-10)

- [ ] `VideoComposition { fps, duration_frames, width, height, scenes: Vec<SceneEntry> }`
- [ ] `SceneEntry { from_frame, duration, entity_hash: ContentHash }` — hash refs `artifact_store` (Part H)
- [ ] `render_frame(frame, &mut Scene)` — find active scenes via `from_frame..from_frame+duration` range; paint with `relative_frame = frame - from_frame`
- [ ] `export(output_path)` — pre-spawn FFmpeg (Remotion pattern); per-frame `Scene::new() → render_frame → capture_frame (wgpu offscreen) → ffmpeg.stdin.write_all`
- [ ] Per-scene render cache: if `artifact_store.get(entity_hash)` hits, skip re-render (content-addressing enables this — Remotion can't)
- [ ] Shared GPU pipeline: canvas preview AND video export use identical wgpu codepath

#### Part G — `MediaVendor` trait (backfill from blueprint §12+§15)

- [ ] `vendor_trait.rs` — `trait MediaVendor: Send+Sync { fn name, fn capabilities, async fn generate, fn cost_per_request }`
- [ ] `LocalVendor` impls: nom-llvm, candle on-device
- [ ] `CloudVendor` impls per provider (Anthropic, OpenAI, Gemini, StabilityAI) — call via reqwest + OTel-injected headers
- [ ] `capabilities.rs` — `enum Capability { Text, Image, Video, Audio, Embedding, Code, ToolUse, Vision }`
- [ ] `cost.rs` — `Cost { cents_per_1k_input_tokens, cents_per_1k_output_tokens, fixed_cents_per_request }`
- [ ] `format_translator.rs` — Claude↔OpenAI↔Gemini message-schema translation at boundary

#### Part H — `artifact_store.rs` content-addressed store (blueprint §12)

- [ ] `artifact_store.rs` — `ArtifactStore { root: PathBuf = $HOME/.nom/store/ }`
- [ ] `put(artifact) -> ContentHash` — SHA-256; write `<root>/<hash[..2]>/<hash>/body.<ext>`
- [ ] `get(hash)` — read + verify hash; returns `Artifact`
- [ ] `gc(keep_referenced: &[ContentHash])` — remove not-in-keep-set; scheduled daily
- [ ] Metadata sidecar `<hash>/meta.json`: `{ kind, mime_type, created_at, source_spec_hash, generation_cost_cents }`

#### Part I — Compose-output preview block types (blueprint §12 Canvas Integration)

- [ ] `nom-blocks/compose/video_block.rs` — timeline scrubber + generated-frames thumbnail strip
- [ ] `nom-blocks/compose/image_block.rs` — generated image + variant picker
- [ ] `nom-blocks/compose/document_block.rs` — paginated typst-rendered preview + page nav
- [ ] `nom-blocks/compose/data_block.rs` — table view + column-type indicators + sort/filter
- [ ] `nom-blocks/compose/app_block.rs` — web iframe preview OR native binary info card
- [ ] `nom-blocks/compose/audio_block.rs` — waveform visualizer + playback controls

#### Part J — WrenAI 5-stage pipeline for data/query backend (blueprint §14)

- [ ] Stage 1: Intent classification via `nom-intent` ReAct (query / chart / insight)
- [ ] Stage 2: Vector retrieval over `SemanticEntity` + `DerivedMetric` + `EntityRelation` (Qdrant-like embedding index)
- [ ] Stage 3: LLM grounded-query generation (MDL context from step 2 → SQL/Cypher/etc)
- [ ] Stage 4: Correction loop — syntax validator; feedback loop max 3 iterations
- [ ] Stage 5: Execute against data source + return to canvas
- [ ] Data structures: `SemanticModel { entities, metrics, relationships }`; `SemanticEntity { name, table, columns, business_meaning }`; `DerivedMetric { name, formula, grouping, filter }`; `EntityRelation { from, to, kind, join_keys }`

#### Part K — NON-GOALS (iter-9 additions)

**waoowaoo/ArcReel:**
- [ ] Direct Claude SDK integration (NEVER import `anthropic-python`/`anthropic-rust` at backend layer — use `MediaVendor` facade only)
- [ ] Freeform prose between agents (mandate typed Rust structs)
- [ ] TypeScript `Promise.all` → use `tokio::join!`

**Polars:**
- [ ] 20+ optimizer rules (MVP = predicate + projection pushdown only)
- [ ] Streaming engine (MVP = in-memory eager)
- [ ] Python interop + pyo3-polars
- [ ] arrow2 / arrow-rs crate dep (Nom implements its own Bitmap)
- [ ] Categorical/Enum/Decimal dtypes v1

#### Part E — NON-GOALS (do NOT adapt)

- Python GIL + single-process assumptions from ComfyUI (Nom targets WASM + multi-process via OMC agents)
- PyTorch tensor marshalling (generalize RAM estimators)
- Filesystem `custom_nodes/` auto-import (Nom uses explicit registry)
- Vue UI reflection from TS types (Nom defines scenario schema separately)
- NPM-hosted node packages (hardcoded or registry file)
- `comemo` crate dependency (adopt the Tracked+Constraint PATTERN, write Nom's own memoization primitive — compiler won't depend on comemo)
- React Hooks + headless Chromium frame capture (direct nom-gpui scene graph)
- Webpack/Rspack bundling (not applicable)

#### Part F — Phase 4 test targets

- [ ] Topological sort — 100 random DAGs, output matches known order
- [ ] Cycle detection lazy trigger — DAG with cycle executes non-cyclic nodes first, raises `CycleError` with participants
- [ ] IS_CHANGED cache reuse — identical inputs → hit; changed input → miss
- [ ] 4-strategy swap — None/Lru/RamPressure/Classic produce identical DAG output
- [ ] Subcache isolation — sibling subgraphs with colliding node IDs don't pollute
- [ ] Cooperative cancellation — 10-second node polled every 100ms, interrupt at 500ms exits within <100ms
- [ ] Retry — `max_tries: 3`, fail twice pass third → 3 attempts + success
- [ ] continueOnFail route — `ContinueRegularOutput` proceeds with empty data; `StopWorkflow` halts
- [ ] **Sandbox escape (security)** — `Object.setPrototypeOf`, `process.env`, `require('fs')`, `Error.prepareStackTrace =` → all blocked, host unaffected
- [ ] Sandbox timeout — `while(true){}` killed at 5000ms, host survives
- [ ] Video backend — 30 test frames → valid mp4 header + correct duration via FFmpeg
- [ ] Document backend incremental — 2nd compile reuses ≥90% memoized layout (instrument cache hit rate)
- [ ] Provider router fallback — Subscription 429 → Cheap; Cheap 500 → Free; Free → error bubbled
- [ ] Credential isolation — scenario log contains zero credential plaintext, only reference IDs

### Phase 5 — Production Quality

Reference reads (iter-8): yara-x linter sealed-trait, Huly CRDT/Hocuspocus (adapted to Rust), typst comemo (port without dep), OpenTelemetry Rust SDK.

#### Part A — `nom-lint` crate (sealed-trait linter framework)

- [ ] `rule_trait.rs` — sealed: `pub trait Rule: RuleInternal {}` + `pub(crate) trait RuleInternal { fn check() -> LintResult }` (ref yara-x [linters.rs:10-18](APP/Accelworld/upstreams/yara-x/lib/src/compiler/linters.rs#L10-L18))
- [ ] `registry.rs` — runtime `linters: Vec<Box<dyn Rule>>` + `add_linter<L: Rule>(&mut self, linter: L)` (ref [mod.rs:416-423](APP/Accelworld/upstreams/yara-x/lib/src/compiler/mod.rs#L416-L423))
- [ ] `diagnostic.rs` — `Diagnostic { span, severity: Error|Warning|Info, code: &'static str, message, fix: Option<Fix> }`; `Fix { span, replacement }` (ref [report.rs:26-111](APP/Accelworld/upstreams/yara-x/lib/src/compiler/report.rs#L26-L111))
- [ ] `span.rs` — byte-offset `Span(Range<u32>)`; `byte_offset_to_line_col()` on demand (ref yara-x [parser/src/lib.rs:41](APP/Accelworld/upstreams/yara-x/parser/src/lib.rs#L41))
- [ ] `visitor.rs` — **Nom improvement**: dedicated `RuleVisitor { pre_visit, post_visit }` trait + default walk (yara-x has no visitor)
- [ ] `incremental.rs` — cache keyed on `(file_hash, rule_set_hash, ast_node_hash)`; cache hit skips rule.check()

#### Part B — `nom-collab` crate (minimal Huly pattern, Rust-native)

- [ ] `ydoc.rs` — canvas state as `yrs::Doc` (Yrs Rust binding, NOT Yjs TS); blocks=Y.Map, elements=Y.Array, text=Y.Text; binary encoding via `transaction.encode_state_as_update_v2()`
- [ ] `server.rs` — tokio + tungstenite WebSocket server; per-doc `yrs::Doc` in memory; debounce 10s normal / 60s max. Reimplement Hocuspocus protocol in Rust (no TS dep)
- [ ] `sync_protocol.rs` — dual-channel: WebSocket for real-time update_v2 deltas + REST `POST /rpc/:docId` for bulk ops (ref Huly [updateContent.ts:22-64](APP/Accelworld/services/other5/huly-main/server/collaborator/src/rpc/methods/updateContent.ts#L22-L64))
- [ ] `awareness.rs` — Yrs `Awareness` sub-protocol for ephemeral presence (cursor, selection, user info); not persisted; lost on disconnect
- [ ] `auth.rs` — JWT decode at WS upgrade; check workspace membership; readonly token → `connection.readonly = true` (ref Huly [authentication.ts:36-71](APP/Accelworld/services/other5/huly-main/server/collaborator/src/extensions/authentication.ts#L36-L71))
- [ ] `persistence.rs` — save Y.Doc binary as blob via `nom-dict` (EntryKind::CollabSnapshot) OR PostgreSQL BYTEA (<10MB) OR content-addressed blob store. Daily snapshots to bound write-amp
- [ ] `offline.rs` — client-side Yrs local store (IndexedDB browser, SQLite desktop); CRDT merge handles conflicts on reconnect

#### Part C — `nom-memoize` crate (incremental compilation, port comemo pattern without dep)

- [ ] `tracked.rs` — `pub struct Tracked<'a, T> { inner: &'a T, constraint: Option<&'a Constraint> }`; zero-cost newtype
- [ ] `constraint.rs` — `Constraint { reads: Vec<Read> }`; `validate(&self, new_value: &T) -> bool` via reference-equality on Tracked + hash-equality on values (ref typst [lib.rs:144-158](APP/Accelworld/services/other5/typst-main/crates/typst/src/lib.rs#L144-L158))
- [ ] `memoize_macro/` — `#[nom_memoize]` proc macro wrapping fn body with FxHashMap cache lookup; reference-equality on `Tracked<T>` params, hash on values
- [ ] `track_macro/` — `#[nom_track]` proc macro; auto-impl Track trait + call-site instrumentation into active Constraint
- [ ] `cache.rs` — `thread_local!(static CACHE: RefCell<FxHashMap<u64, CachedResult>>)` — thread-local (matches typst), flush on compilation boundary

#### Part D — `nom-telemetry` crate (OpenTelemetry Rust SDK)

- [ ] `spans.rs` — 4-tier taxonomy: `ui` (info, hover/completion), `interactive` (info, 50% sampled, S1-S2/intent), `background` (debug, 5% sampled, S3-S6/corpus), `external` (info, always sampled, anthropic/openai APIs)
- [ ] `instrument_macro/` — `#[nom_instrument(level = "info", tier = "ui")]` proc macro wrapping `tracing::instrument` with Nom tags
- [ ] `propagation.rs` — `with_current_otel_context()` for tokio; `extract/inject_trace_context()` for HTTP/WebSocket; W3C `traceparent: 00-<128bit>-<64bit>-<sampled>`
- [ ] `sampler.rs` — `SamplerConfig { env: Dev|Prod, ratio: f64 }` → `AlwaysOn` dev / `ParentBased(TraceIdRatioBased(0.01))` prod
- [ ] `exporter.rs` — `init_exporter(endpoint)` with `BatchSpanProcessor` + W3C propagator; global TracerProvider for tracing-opentelemetry layer
- [ ] `rayon_bridge.rs` — helper for rayon pool: `rayon_scope(|s| s.spawn(|_| span!("work").in_scope(|| ...)))`

#### Part E — File watcher + incremental relinting (Pattern S4 from blueprint §10)

- [ ] `watcher.rs` — `notify` crate `RecommendedWatcher`; 50ms batch debounce; emit `ChangeEvent { path, kind }`
- [ ] `incremental_relint.rs` — diff AST node hashes on change; invalidate only changed-node rule results; full relint when rule set changes

#### Part F — Phase 5 test targets

- [ ] **Sealed trait enforcement** — trybuild compile-fail: external `impl Rule for ExternalStruct` fails with "trait RuleInternal is private"
- [ ] Diagnostic byte-offset ↔ line/col round-trip — ASCII + multi-byte UTF-8 + CRLF; 1000 offsets
- [ ] Fix application + overlap detection — `apply_fix()` produces expected edit; overlapping fixes → `OverlappingFixes` error
- [ ] Incremental relint — edit 1 node in 1000-node AST; only that rule result changes
- [ ] **CRDT convergence property test** — 2 clients random interleaved edits (1000 runs); both converge to identical Y.Doc
- [ ] Awareness ephemerality — client disconnects; state GC'd within 10s
- [ ] Access control — non-member JWT → 401; readonly → connection accepts reads rejects updates
- [ ] Persistence snapshot — daily cron saves; restoration bit-identical
- [ ] Memoize reference-equality — same memory address cached; equal-value-different-instance NOT cached
- [ ] Memoize constraint validate — mutate tracked value; Constraint::validate() returns false on re-call
- [ ] Telemetry context propagation — tokio spawn + rayon pool + external API all share `trace_id`
- [ ] Sampler consistency — 1000 traces @ 0.01 ratio; ~10 fully sampled; all spans in one trace share decision
- [ ] File watcher debounce — 10 edits in 20ms → 1 event; edit after 60ms → 2 events

### Blueprint-gap backfill (iter-8, modules from spec §8 I missed earlier)

**Phase 1 (`nom-gpui`):**
- [ ] `animation.rs` — timestamp-based interpolation + easing curves; `cubic-bezier(0.27, 0.2, 0.25, 1.51)` for mode switch (blueprint §5)

**Phase 2 (`nom-editor`):**
- [ ] `input.rs` — keyboard event dispatch + IME composition (winit `WindowEvent::Ime(Ime::Preedit|Commit)`) for CJK, combining marks, compose-key sequences
- [ ] `completion.rs` — completion UI popup driven by `nom-resolver::resolve()` (3-stage: exact→word→semantic) + `nom-grammar::find_keywords()`

**Phase 3 (`nom-panels`, `nom-theme`):**
- [ ] `nom-panels/properties.rs` — right-rail property inspector; dispatches by block-type flavour; renders editable fields per schema
- [ ] `nom-theme/fonts.rs` — Inter + Source Code Pro loading into nom-gpui atlas via cosmic-text; fallback chain Inter → system sans → default
- [ ] `nom-theme/icons.rs` — Lucide 24px SVG (~1400 icons) — pre-tessellate to Path primitives OR rasterize to PolychromeSprite atlas; lazy-load by usage

## v1 ARCHIVED (`.archive/nom-canvas-v1-typescript/`)

TypeScript v1 archived. All 3 CRITICAL issues (credentials, sandbox-eval, CSS grid) are moot — fresh Rust rewrite.

## Compiler Remaining

- [ ] GAP-1c body_bytes | GAP-2 embeddings | GAP-3 corpus | GAP-9/10 bootstrap
