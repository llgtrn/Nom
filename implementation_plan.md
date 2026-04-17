# Nom — Implementation Plan

> **CANONICAL TRACKING DOC — MAIN** (Planner/Auditor refreshes every cycle)
> **HEAD:** `e2b7ecb` on main | **CI:** canvas job GREEN (cargo check + test 23s on ubuntu-latest); compiler matrix still running | **Date:** 2026-04-17
> **Spec:** `docs/superpowers/specs/2026-04-17-nomcanvas-gpui-design.md` (719 lines) — also canonical
> **Sibling docs:** `nom_state_machine_report.md`, `task.md` (all 4 MUST stay in sync)
> **Foundation:** Everything built around Nom language. 9 kinds compose everything.
> **Zed gpui_wgpu end-to-end read** (7 files + Cargo.toml: wgpu/cosmic-text 0.17/swash 0.2.6/etagere 0.2) complete; batch-2 MVP = quads + mono_sprites + buffer growth + atlas upload; defer path_rasterization + subpixel-dual-source to batch-3.

---

## Architecture: Custom GPUI + Compiler-as-Core

```
nom-canvas/ (12 Rust crates, replacing 46 TypeScript modules)
├── nom-gpui/        ✓ scene graph + BoundsTree + Element/Styled/Layout + PlatformAtlas trait (batch-1)
│                      ▸ wgpu renderer + cosmic-text atlas + winit window + platform abstraction (batch-2)
├── nom-canvas-core/ — viewport, elements, selection, snapping, spatial index
├── nom-editor/      — rope buffer, multi-cursor, tree-sitter highlight
├── nom-blocks/      — prose, nomx, media, graph_node, drawing, table, embed
├── nom-graph-v2/    — DAG engine (Kahn+IS_CHANGED), cache (LRU+RAM), execution
├── nom-panels/      — sidebar(248px), toolbar(48px), preview, library, command, statusbar(24px)
├── nom-theme/       — AFFiNE tokens (73 vars), Inter+Source Code Pro, Lucide icons
├── nom-compose/     — universal composition (video, image, audio, doc, data, app, 3D)
└── nom-compiler/    — UNCHANGED 29 crates, linked as direct deps
```

## Phase 1 batch-1 (LANDED 2026-04-17) — nom-gpui foundation

9 modules / 31 tests green / 0 foreign identifiers. **100% data model + trait APIs, zero GPU substrate.**

| Module | Role | Pattern Source (end-to-end read) |
|--------|------|----------------------------------|
| `geometry.rs` | Point/Size/Bounds/Pixels triad/TransformationMatrix | zed/gpui/geometry.rs |
| `color.rs` | Rgba + Hsla + source-over blend | zed/gpui/color.rs |
| `bounds_tree.rs` | R-tree (MAX_CHILDREN=12) DrawOrder assignment | zed/gpui/bounds_tree.rs |
| `scene.rs` | 6 typed Vec primitives + batched iterator | zed/gpui/scene.rs |
| `atlas.rs` | PlatformAtlas trait + AtlasTextureKind + InMemoryAtlas (no GPU upload yet) | zed/gpui/platform.rs |
| `style.rs` | Style → taffy::Style conversion | zed/gpui/style.rs + taffy 0.6 API |
| `styled.rs` | Fluent builder (.flex_col().w().padding().bg()) — 40+ methods | zed/gpui/styled.rs |
| `element.rs` | 3-phase lifecycle trait with caller-owned state | zed/gpui/element.rs |
| `taffy_layout.rs` | LayoutEngine wrapper + bounds caching (no measure fns yet) | zed/gpui/taffy.rs |

## Audit Findings — 2026-04-17 iter 2 (commit `e2b7ecb`)

6 parallel review agents (code-reviewer×2, architect, security-reviewer, code-simplifier, Explore). Evidence-rich; findings consolidated below.

### CRITICAL (must fix BEFORE Phase 1 batch-2 GPU work — wrong rendering otherwise)

- **BatchIterator drains all-remaining-of-kind** — [scene.rs:227-258](nom-canvas/crates/nom-gpui/src/scene.rs#L227-L258). Zed advances only until next kind's order ([zed gpui/scene.rs:316-373](APP/zed-main/crates/gpui/src/scene.rs#L316-L373)). Under z-interleaving (`shadow@1, shadow@10, quad@5`), Nom emits wrong order. **Flagged independently by code-reviewer + architect + API-reviewer.**
- **BoundsTree uses monotonic `next_order.saturating_add(1)` instead of overlap-aware `max_intersecting + 1`** — [bounds_tree.rs:88-107](nom-canvas/crates/nom-gpui/src/bounds_tree.rs#L88-L107). Zed reuses orders for non-overlapping rects ([zed bounds_tree.rs:119-135](APP/zed-main/crates/gpui/src/bounds_tree.rs#L119-L135)). Breaks batch coalescing + `push_layer`/`pop_layer` z-order. Latent overflow bug: once `next_order` saturates, every insert gets same order, tree degrades.
- **PrimitiveBatch sprite variants lack `texture_id`** — [scene.rs:183-189](nom-canvas/crates/nom-gpui/src/scene.rs#L183-L189). GPU renderer can't bind correct atlas per draw call. Zed breaks batches on `texture_id` change and sorts sprites by `(order, tile_id)` ([zed scene.rs:143-147](APP/zed-main/crates/gpui/src/scene.rs#L143-L147)).

### HIGH (fix before downstream crates depend on nom-gpui)

- **HSL hue convention mismatch** — [color.rs:71-104](nom-canvas/crates/nom-gpui/src/color.rs#L71-L104). Nom uses `[0,360)` degrees; Zed uses `[0,1]`. Theme integration will silently produce wrong colors. Add explicit `from_degrees`/`from_normalized` constructors.
- **`Styled` trait consumes `self`** — [styled.rs:18](nom-canvas/crates/nom-gpui/src/styled.rs#L18). All 40+ setters return `Self` by value, incompatible with `Element`'s `&mut self` phases. Change to `&mut self -> &mut Self`.
- **`Pixels` has `From<f32>` / `From<Pixels> for f32`** — [geometry.rs:31-41](nom-canvas/crates/nom-gpui/src/geometry.rs#L31-L41). Enables silent unit confusion; `ScaledPixels`/`DevicePixels` lack these (inconsistent). Remove; `Styled` setters should take `impl Into<Pixels>`.
- **`LayoutEngine::request_layout` panics** — [taffy_layout.rs:52-53](nom-canvas/crates/nom-gpui/src/taffy_layout.rs#L52-L53). `.expect()` on taffy error, but `compute_layout` returns `Result<(), LayoutError>`. Make consistent.

### MEDIUM (pattern gaps vs Zed — backfill during batch-2)

- **Missing `request_measured_layout`** — [taffy_layout.rs:29-31](nom-canvas/crates/nom-gpui/src/taffy_layout.rs#L29-L31). Empty `NodeContext`; no measure closure support. Content-sized elements (text, images) can't report intrinsic dims. Zed ref: [taffy.rs:80-104](APP/zed-main/crates/gpui/src/taffy.rs#L80-L104).
- **Missing `SubpixelSprite` + `Surface` primitive kinds** — Nom has 6 vecs, Zed has 8. `SubpixelSprite` needed for crisp text (dual-source blend).
- **Missing `PlatformAtlas::remove()`** — [atlas.rs:69-86](nom-canvas/crates/nom-gpui/src/atlas.rs#L69-L86). Only bulk `clear()`; no per-tile eviction.
- **`AtlasKey.bytes: Vec<u8>`** — [atlas.rs:57](nom-canvas/crates/nom-gpui/src/atlas.rs#L57). Hot-path allocation on every cache lookup. Use `Arc<[u8]>` or `Borrow`-based key.
- **`Scene` fields are `pub`** — [scene.rs:100-106](nom-canvas/crates/nom-gpui/src/scene.rs#L100-L106). Callers bypass `insert_*` + `finish()` contract. `pub(crate)` + read-only accessors.
- **BoundsTree recursive walk** — [bounds_tree.rs:198-220](nom-canvas/crates/nom-gpui/src/bounds_tree.rs#L198-L220). Stack-overflow risk on degenerate overflow-bucket chains. Zed uses explicit `Vec<u32>` stack.
- **Missing `half_perimeter` area heuristic** in BoundsTree child selection — [zed bounds_tree.rs:248](APP/zed-main/crates/gpui/src/bounds_tree.rs#L248).
- **Missing fast-path in `topmost_intersecting`** — Zed checks `max_leaf` bounds before full walk ([zed bounds_tree.rs:143-149](APP/zed-main/crates/gpui/src/bounds_tree.rs#L143-L149)).
- **`draw_element` skips `compute_layout`** — [element.rs:76-77](nom-canvas/crates/nom-gpui/src/element.rs#L76-L77). Resolves zero-default bounds; test only passes because asserts are trivially true.

### LOW

- InMemoryAtlas no vertical-overflow check (test-only impl, acceptable for now) — [atlas.rs:144-148](nom-canvas/crates/nom-gpui/src/atlas.rs#L144-L148)
- `bytemuck` declared but unused (remove until batch-2 needs it)
- `max_leaf` field written but never read — [bounds_tree.rs:63](nom-canvas/crates/nom-gpui/src/bounds_tree.rs#L63) (wire up the fast-path above)
- `PrimitiveKind` enum is `pub` but no external consumers — [scene.rs:88-95](nom-canvas/crates/nom-gpui/src/scene.rs#L88-L95)
- `styled.rs` 40+ setters → ~25 collapse into `style_setter!` macro
- Debug derives missing on `BatchIterator`, `ElementCx`, `LayoutEngine`
- Doc debt on `ElementCx.rem_size`/`scale_factor` units, `AtlasTileRef.uv` layout, `NodeContext` purpose

### VERIFIED CORRECT

- ✅ Archive move clean — 0 dangling refs, CI uses `cargo` not `npm/vite`, new `nom-canvas/Cargo.toml` workspace valid
- ✅ `unsafe_code = deny` holds (0 unsafe blocks across all 10 files)
- ✅ Zero secrets / credentials / known-CVE deps
- ✅ Element 3-phase lifecycle matches Zed exactly (intentional Nom simplifications)
- ✅ `TransformationMatrix::compose` math verified correct
- ✅ `Rgba::blend` handles `a == 0` degenerate case
- ✅ `parking_lot::Mutex` used correctly (no poison state)
- ✅ Error types use `thiserror` with typed variants (not strings)

---

## Phase 1 batch-2 (NEXT) — GPU substrate, 8 pipelines, surface binding

Zed reference: `APP/zed-main/crates/gpui_wgpu/` — wgpu renderer (cross-platform, Metal-free), ~1800 LOC. Adopt as-is; Nom is already wgpu-first.

### 8 wgpu pipelines (Zed `wgpu_renderer.rs:83-94`)

| Pipeline | Instance struct | Shader work | Reference |
|----------|-----------------|-------------|-----------|
| `quads` | Quad (bounds, bg, border, corner_radii) | rounded-rect + border + optional shadow mask | `wgpu_renderer.rs:174-484` |
| `shadows` | Shadow (bounds, color, blur_radius, offset) | gaussian blur fake via SDF | same |
| `path_rasterization` | PathRasterizationVertex (xy, st, color, bounds) | rasterize tri-mesh to intermediate MSAA texture | `wgpu_renderer.rs:1073-1315` (drop current pass → rasterize → resume) |
| `paths` | PathSprite (bounds, atlas_tile) | sample rasterized-path intermediate texture | same |
| `underlines` | Underline (origin, thickness, color, wavy) | thin stroke + optional wave | same |
| `mono_sprites` | MonochromeSprite (bounds, atlas_tile, color) | atlas R8Unorm lookup + tint | same |
| `subpixel_sprites` | SubpixelSprite (bounds, atlas_tile, color) | atlas Bgra8Unorm subpixel, dual-source blend (check adapter feature) | same |
| `poly_sprites` | PolychromeSprite (bounds, atlas_tile) | atlas Bgra8Unorm direct sample (emoji/images) | same |

### Batch-2 concrete tasks

1. **`gpu_pipeline.rs`** — `WgpuPipelines` struct holding 8 `wgpu::RenderPipeline` + shared bind group layouts (view uniform, atlas textures, sampler).
2. **`shaders/` directory** — 4 WGSL files: `quad.wgsl`, `sprite.wgsl`, `path.wgsl`, `underline.wgsl`. Unit-quad vertex (0..1 × 0..1) scaled by bounds, NDC normalize at end (Zed `shaders.wgsl:169-177`).
3. **`gpu_buffers.rs`** — Instance buffer (storage, `@group(1) @binding(0)`) with 2× growth + `max_buffer_size` clamp + panic-after-20-frames guard (`wgpu_renderer.rs:1504-1510`).
4. **`wgpu_atlas.rs`** — Replace `InMemoryAtlas` with real `WgpuAtlas`: `etagere::BucketedAtlasAllocator` per `AtlasTextureKind`, 1024×1024 default, grow to `device.limits().max_texture_dimension_2d`. `PendingUpload` queue → `queue.write_texture()` in `before_frame()` (`wgpu_atlas.rs:56-98`).
5. **`text_rasterization.rs`** — `cosmic_text::ShapeLine` + `cosmic_text::Shaping::Advanced` → `swash::scale::ScaleContext` + `Render::new()` with `Format::subpixel_bgra()` / `Format::Alpha` → bytes into `PlatformAtlas::get_or_insert_with((font_id, glyph_id, subpixel_variant))` (`cosmic_text_system.rs:286-353`).
6. **`window.rs`** — `winit::Window` + `wgpu::Surface` binding. Format negotiation: Bgra8 preferred, Rgba8 fallback; alpha `PreMultiplied` or `Opaque`; present mode **Fifo** default, expose `preferred_present_mode` (`wgpu_renderer.rs:263-303, :335`). NO busy-loop — OS vsync drives cadence.
7. **`frame_loop.rs`** — `run()` using winit 0.30 `EventLoop::run_app` with `ApplicationHandler`. Per-frame: acquire → draw scene → present. No explicit 60fps tick.
8. **`device_lost.rs`** — `GpuContext` shared across windows; `device_lost` flag + re-create path (`wgpu_renderer.rs:1748-1760`). Critical for mobile/Android later.
9. **Platform feature flags** — `Cargo.toml` features: `native` (default, winit+wgpu) vs `web` (wasm32, WebGPU). `#[cfg(target_arch = "wasm32")]` gates winit usage.
10. **Hit-testing wiring** — expose `BoundsTree::topmost_intersecting(point)` on `Scene`; route winit pointer events through it; dispatch to `ElementId`.
11. **Element state storage** — persistent `HashMap<ElementId, Box<dyn Any>>` on `Window` for focus/interaction state (currently placeholder).
12. **Integration tests** — "full element tree → scene → batches → GPU draw" headless test using `wgpu` offscreen `TextureView`.

### Batch-2 test targets

- Headless render of single quad → pixel-perfect PNG diff
- Atlas round-trip: insert glyph → sample UV → verify bytes
- Path rasterization: triangle mesh → intermediate texture → sample correctness
- Window open/close/resize without device-lost panic
- 60 consecutive frames without buffer-growth panic

## Compiler Thread Tiers (compiler IS the IDE)

| Tier | Thread | Crates | Max |
|------|--------|--------|-----|
| UI | Main | nom-grammar (synonym), nom-dict (cached read), nom-score (pure), nom-search (BM25) | <1ms |
| Interactive | Async pool | nom-concept S1-S2, nom-lsp hover/complete/def, nom-resolver resolve | <100ms |
| Background | Dedicated | nom-concept S1-S6, nom-planner, nom-app dream, nom-security, nom-intent ReAct, nom-llvm | >100ms |
| Composition | Queue | Video (FFmpeg), image (diffusion), doc (typst), data (extract), app (deploy) | >10s |

## Universal Composition Backends

| Nom Kind | Output | Pattern Source |
|----------|--------|---------------|
| `media` video | MP4 | Remotion (GPU frame→FFmpeg pipe) + ComfyUI DAG |
| `media` image | PNG/AVIF | Open-Higgsfield (200+ models) |
| `media` storyboard | Video sequence | waoowaoo (4-phase) + ArcReel (agent workflow) |
| `screen` web | HTML/WASM | ToolJet (55 widgets) + Dify (workflow) |
| `screen` native | Binary | nom-llvm compile + link |
| `data` extract | JSON/CSV | opendataloader-pdf (XY-Cut++) |
| `concept` document | PDF | typst (comemo incremental) |
| `scenario` workflow | Trace | n8n (304 nodes, AST sandbox) |

## Provider Router (9router pattern)

3-tier fallback: Subscription → Cheap → Free. Per-vendor quota tracking. Format translation (Claude↔OpenAI↔Gemini).

## Semantic Layer (WrenAI pattern)

MDL grounds data queries by schema context. LLM generates SQL/queries from Nom prose, validated against semantic model.

## Collaboration (Huly pattern, v3+)

CRDT (Yrs) + event bus. 30-service Huly architecture adapted to Nom canvas state.

## 60 Patterns Catalog

Full catalog with source paths in v2 spec. 13 MUST, 18 HIGH, 21 MED, 8 LOW across 6 clusters (RAG, Agent, Canvas, Media, Security, Data).

## NON-NEGOTIABLE

1. Everything on Nom language foundation | 2. End-to-end reading before ANY code | 3. ui-ux-pro-max for ALL UI | 4. Zero foreign identities | 5. MACRO view | 6. Spawn subagents | 7. Strict external comparison
