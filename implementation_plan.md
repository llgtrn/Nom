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

## Audit Findings — 2026-04-17 iter 6 (commit `205aea9` — batch-2 wave-1 audit)

6 parallel agents (code-reviewer ×3, architect, security-reviewer, code-simplifier) audited the Executor's wave-1 against the iter-5 spec + blueprint. Tests went 44 → 59 (+15). Strong on infrastructure, **real correctness defects found**.

### CRITICAL (BLOCK wave-2 start)

- **`QuadInstance` + `MonoSpriteInstance` missing `order` field** — [quad.wgsl:40-48](nom-canvas/crates/nom-gpui/src/shaders/quad.wgsl#L40-L48), [mono_sprite.wgsl:37-43](nom-canvas/crates/nom-gpui/src/shaders/mono_sprite.wgsl#L37-L43). Breaks Z-sorted rendering — the `BoundsTree` DrawOrder (iter-4 fix) has no way to surface. Add `order: u32` + sort batches by it.
- **`MonoSpriteInstance` missing `transformation` field** — Rotated/scaled glyphs impossible. Zed's struct has this.
- **20-frame overflow guard COMPLETELY MISSING in `InstanceBuffer`** — [buffers.rs:116-126](nom-canvas/crates/nom-gpui/src/buffers.rs#L116-L126). Spec mandated counter + panic on 20-consecutive-frame capacity saturation. Not implemented at all. Under capacity exhaustion, draw calls silently drop. Add `overflow_frames: u32` field + `begin_frame()` reset + `write()` increment + panic at 20.
- **`pub mod context;` missing from `lib.rs`** — [lib.rs:20-21](nom-canvas/crates/nom-gpui/src/lib.rs#L20-L21). `context.rs` is **dead code, unreachable from crate root**. 2-line fix but wave-2 cannot depend on `GpuContext` without it.

### HIGH (fix before wave-2 to avoid breaking-change churn)

- **Shader file split — 4 files instead of 1** — iter-5 spec explicitly mandated a single `shaders.wgsl`. Executor landed `common.wgsl` + `quad.wgsl` + `mono_sprite.wgsl` + `underline.wgsl`. Common header is **copy-pasted 3×** (RenderParams, to_ndc, unit_vertex). Consolidate.
- **Scope creep: `underline.wgsl` in wave-1** — explicitly deferred to wave-3 in iter-5 plan. Harmless but dilutes wave-1.
- **`GammaParams` missing from `globals` bind group layout** — [pipelines.rs:72-84](nom-canvas/crates/nom-gpui/src/pipelines.rs#L72-L84). Spec required 2 entries at `@group(0)` (binding 0 GlobalParams + binding 1 GammaParams). Only binding 0 present. Forces a **breaking change to all pipelines** when wave-2 text rendering needs gamma. Fix now, before the pipeline surface area grows.
- **`recover()` swaps `Arc<Device>`/`Arc<Queue>` but callers hold stale clones** — [context.rs:138-151](nom-canvas/crates/nom-gpui/src/context.rs#L138-L151). No notification mechanism; no generation counter. Wave-2 atlas uploads will hold device clones across recovery boundaries. Either use `Arc<RwLock<Device>>` or doc the invariant that `recover()` invalidates clones.
- **`min_binding_size: None` on globals uniform** — [pipelines.rs:80](nom-canvas/crates/nom-gpui/src/pipelines.rs#L80). Zed uses `NonZeroU64::new(size_of::<GlobalParams>())` for bind-time validation. `None` defers errors to draw time.
- **Texture + sampler bindings `FRAGMENT`-only** — [pipelines.rs:121, :129](nom-canvas/crates/nom-gpui/src/pipelines.rs#L121). Zed uses `VERTEX_FRAGMENT`. If a vertex shader ever needs atlas UV mapping (some sprite layouts do), this must be relaxed.
- **`hsla_to_rgba()` missing in WGSL** — All instance colors use raw `vec4<f32>` RGBA. Zed converts on GPU via `hsla_to_rgba` so themes stay HSL-native. Current approach forces Rust-side pre-conversion, adds a foreign-identity pressure (HSL ↔ RGBA mismatch at the CPU/GPU boundary).
- **`content_mask` renamed to `clip_bounds`** everywhere — harmless but diverges from Zed naming; creates confusion during later porting work. Pick one; rename is cheap now, expensive later.

### MEDIUM (opportunistic)

- **Per-pipeline `ShaderModule` creation** — [pipelines.rs:154-157](nom-canvas/crates/nom-gpui/src/pipelines.rs#L154-L157). Spec said single shared module. 3 separate modules from 3 files. Current design is actually reasonable (modular shaders) — document divergence as intentional or consolidate with file split fix.
- **Unit-quad unpack uses `0.5 * f32(vertex_id & 2u)` instead of `(vertex_id & 2u) >> 1u`** — mathematically identical but not verbatim. Minor style divergence.
- **`is_device_lost` uses `SeqCst`** — [context.rs:129](nom-canvas/crates/nom-gpui/src/context.rs#L129). `Acquire`/`Release` pair sufficient and cheaper (single flag, no dependent data).

### TEST QUALITY GRADE: D+

Strong on pure-math buffer helpers (14 tests cover `align_up`/`compute_write_slot`/`next_capacity` edges). But the **hard, high-risk deliverables have zero coverage**:

- No `wgpu::Device::create_shader_module()` validation test (current naga-parse test catches WGSL syntax errors but NOT pipeline-layout compatibility)
- No Rust mirror for `to_ndc`, `unit_vertex`, `rounded_rect_sdf` → zero boundary tests
- No BindGroupLayout-vs-shader entry-count/type test
- 20-frame overflow guard untested (because unimplemented)
- 1 tautological test: `context_creates_when_adapter_available` asserts `... || true` which makes the assertion always pass
- 3 context tests silently skip on headless CI with no `#[ignore]` gate (cargo reports "passed" without running assertions)

### VERIFIED CLEAN ✅

- **Zero `unsafe` blocks** in wave-1 files; `#![deny(unsafe_code)]` on all three
- **Zero new suspicious deps** — wgpu, pollster, raw-window-handle (workspace), naga 22 (dev) — all mainstream
- **Blueprint conformance — ZERO foreign identifiers in public API**. Doc comments note provenance per existing convention ("Architecture mirrors Zed GPUI's PlatformAtlas" is historical context, not a type name)
- **Zero wrappers**: `InstanceBuffer` + `GpuContext` both add real logic (growth + recovery), not thin re-exports
- **Thread-model READY**: Arc<Device>, Arc<Queue>, Arc<AtomicBool>, exclusive render-thread ownership of buffer
- **Wasm-READY** (pollster dev-dep will need `#[cfg]` gate later, but not blocking now)
- **Device-lost callback** correctly filters `Destroyed` reason (intentional teardown shouldn't trigger recovery)
- **Integer arithmetic safe**: `saturating_mul(2).min(max)`, `checked_add`, no overflow risk

### DIVERGENT-BUT-OK (intentional improvements over Zed)

- `GrowError::AtMax` as typed `thiserror` variant vs Zed's inline panic — strictly better error handling
- `pick_adapter_for_surface` with priority scoring — matches Zed pattern cleanly

### ARCHITECTURAL VERDICT

**Wave-2 is blocked on 4 CRITICAL fixes.** Once `order` field + 20-frame guard + `pub mod context;` + GammaParams binding land, wave-2 (atlas + text + window) can proceed. The remaining HIGH items should be fixed in the same fix-wave to avoid compounding the break when pipeline surface grows.

**Recommended fix-wave sequence for Executor:**
1. Add `pub mod context;` to [lib.rs](nom-canvas/crates/nom-gpui/src/lib.rs) + re-export `GpuContext` (2-line fix)
2. Add `order: u32` + `texture_sort_key: u32` to `QuadInstance` / `MonoSpriteInstance`, update shader structs + sorted insertion in pipelines
3. Implement 20-frame overflow guard: `begin_frame()`/`end_frame()` + `overflow_frames: u32` + panic at 20
4. Consolidate 4 shaders → 1 `shaders.wgsl`; move `underline.wgsl` to wave-3 scoped file (or keep but flag as scope)
5. Add `GammaParams` uniform at `@group(0) @binding(1)` + bind group layout entry
6. Add `hsla_to_rgba()` WGSL fn + port Zed helpers (`blend_color`, `enhance_contrast`)
7. Add test gaps: shader module creation (device-backed), NDC+unit-quad+SDF boundaries (pure-math Rust mirrors), bind-group compat

**Security + blueprint conformance remain STRONG.** No regressions on those axes.

---

## Audit Findings — 2026-04-17 iter 4 (commit `1daa80e` — verifying iter-2 fixes)

6 parallel agents (code-reviewer ×2, architect, code-simplifier, security-reviewer, Explore) verified the Executor's audit-fix commit.

### VERIFIED CLEAN ✅

- **3 CRITICAL rendering-correctness fixes — all VERIFIED-CORRECT:**
  - BatchIterator z-interleave: cutoff = min-other-kind-order, `advance_while(order <= cutoff)`. Trace `[shadow@1, quad@5, shadow@10]` emits 3 correct batches. Matches Zed's Peekable+next_if semantics ([scene.rs:228-270](nom-canvas/crates/nom-gpui/src/scene.rs#L228-L270))
  - BoundsTree overlap-aware: `topmost_intersecting(bounds).map_or(1, |o| o.checked_add(1).expect(...))`. 4 tests including 50-rect non-overlap reuse ([bounds_tree.rs:108-134](nom-canvas/crates/nom-gpui/src/bounds_tree.rs#L108-L134))
  - Sprite `texture_id`: `PrimitiveBatch::{Mono,Poly}Sprites { texture_id, sprites }` struct variants; `finish()` sorts by `(order, tile_tex_id)`; batches break on texture change ([scene.rs:196-217](nom-canvas/crates/nom-gpui/src/scene.rs#L196-L217))

- **4 HIGH API-shape fixes — all VERIFIED:**
  - Hsla: canonical `[0,360)` storage with both `from_degrees`/`from_normalized` constructors and explicit doc. Dual tests confirm equivalence ([color.rs:81-107](nom-canvas/crates/nom-gpui/src/color.rs#L81-L107))
  - Styled `&mut self -> &mut Self`: all 40+ setters migrated, `Sized` bound dropped, new test `mut_ref_setters_compose_with_element_lifecycle` proves borrow-release pattern ([styled.rs:23](nom-canvas/crates/nom-gpui/src/styled.rs#L23))
  - Pixels no-From-f32: both impls deleted; `Pixels(x)` is the only ctor; ScaledPixels/DevicePixels consistent ([geometry.rs](nom-canvas/crates/nom-gpui/src/geometry.rs))
  - `request_layout` Result: split into infallible wrapper + `try_request_layout -> Result<LayoutId, LayoutError>`; `#[from] taffy::TaffyError` variant ([taffy_layout.rs:48-61](nom-canvas/crates/nom-gpui/src/taffy_layout.rs#L48-L61))

- **Security posture preserved:** 0 unsafe blocks, 0 new deps, 0 attacker-triggerable panics, no Arc cycle risk. `checked_add(1)` replaces `saturating_add` on BoundsTree ordering with explicit panic doc.

- **Opportunistic bonus:** MED item #8 (`max_leaf` fast-path in `topmost_intersecting`) landed during overlap-aware rework ([bounds_tree.rs:227-231](nom-canvas/crates/nom-gpui/src/bounds_tree.rs#L227-L231))

### DIVERGENT-BUT-OK (intentional Nom-specific choices, not regressions)

- **Styled `&mut self` vs Zed's `mut self`** — Zed uses owned-self fluent; Nom uses borrow-fluent. Better for `Element`'s `&mut self` lifecycle integration. Documented divergence; no regression.
- **Hsla `[0,360)` storage vs Zed's `[0,1]`** — Nom exposes both conventions via explicit constructors; Zed normalized-only. Clarity over parity.

### STILL OPEN (7 MED + 6 LOW from iter-2 audit)

**MED (not blocking batch-2 start, but some block batch-2 correctness):**
- ⚠️ Missing `request_measured_layout` + `NodeContext` measure closure — **blocks content-sized text/image elements**
- ⚠️ Missing `SubpixelSprite` + `Surface` primitive kinds — **blocks crisp subpixel text when rendered**
- `PlatformAtlas::remove()` — nice-to-have, not blocking
- `AtlasKey.bytes: Vec<u8>` → `Arc<[u8]>` — hot-path optimization, not blocking
- `Scene` fields still `pub` — encapsulation debt
- BoundsTree `walk()` still recursive — stack-overflow risk on >1000 overlapping layers (not batch-2 critical)
- Missing `half_perimeter` heuristic — tree balance suboptimal but correct
- `draw_element` still skips `compute_layout` — documentation gap

**LOW:**
- InMemoryAtlas vertical-overflow check / remove unused `bytemuck` / remove `pub` from `PrimitiveKind` / Styled setter macro / Debug derives / doc comments

### TEST QUALITY GRADE: C+

- STRONG edge coverage: BatchIterator z-interleave, BoundsTree overlap+reuse, Styled borrow-lifecycle (3 of 7 fixes)
- WEAK happy-path-only: sprite texture ABA pattern missing, Hsla boundary (0/360 wrap) + round-trip missing, LayoutError path untested (3 of 7)
- TAUTOLOGICAL: `pixels_explicit_construction_only` asserts `Pixels(42.0).0 == 42.0` — passes regardless of whether `From<f32>` exists. Zero fix-signal. **Flag for rewrite as trybuild compile-fail test.**
- No property-based (proptest/quickcheck) tests on R-tree
- No golden-file / snapshot patterns

### ARCHITECTURAL VERDICT

**Batch-2 GPU work CAN PROCEED.** Blocking-correctness items from iter-2 are all verified clean. Track `SubpixelSprite`/`Surface` kinds as a batch-2-internal prerequisite (add before subpixel text rendering lands, not before pipeline setup starts). Track `request_measured_layout` as a batch-3 blocker (text elements).

### Recommended next Executor actions

1. **Start batch-2 wave-1** (shaders + buffers + GpuContext) — no audit-fix blockers remain
2. **Add the 3 test gaps** before batch-2 merges:
   - Sprite ABA texture pattern
   - Hsla hue boundary + `rgb→hsl→rgb` round-trip
   - `try_request_layout` error path (trigger taffy NaN)
3. **Rewrite tautological `pixels_explicit_construction_only`** as a trybuild compile-fail harness
4. **Backfill 2 MED items alongside batch-2**: `SubpixelSprite`+`Surface` kinds (before subpixel text), iterative `walk()` stack (before >1000-layer scenes)

---

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

### Batch-2 wave-1 concrete skeletons (iter-5 deep-read of Zed `shaders.wgsl` + `wgpu_renderer.rs`)

**Single WGSL file, not four.** Zed keeps all 8 shader entry points in one `shaders.wgsl` (1335 lines). Nom should do the same — easier binding sharing. Subpixel shaders live in a second optional file combined at runtime only when `DUAL_SOURCE_BLENDING` adapter feature is present.

**Bind group layouts** (Zed [wgpu_renderer.rs:487-600](APP/zed-main/crates/gpui_wgpu/src/wgpu_renderer.rs#L487-L600)):
- `globals_layout` — `@group(0)`: binding 0 = `GlobalParams` uniform (viewport_size: vec2, premultiplied_alpha: u32, VERTEX_FRAGMENT), binding 1 = `GammaParams` uniform (REC.601 coeffs + subpixel contrast, FRAGMENT only)
- `instances_layout` — `@group(1)`: binding 0 = storage buffer `read_only: true`. Used by quads/shadows/path_rasterization/underlines
- `instances_with_texture_layout` — `@group(1)`: binding 0 = storage, binding 1 = `Texture2D<Float>` filterable, binding 2 = `Sampler::Filtering`. Used by paths/mono_sprites/subpixel_sprites/poly_sprites
- `surfaces_layout` — optional (video/screen capture), uniform + 2 textures + sampler

**Pipeline layout** — all 8 pipelines share identical shape: `[globals_layout, data_layout]`. No per-pipeline variations.

**Vertex buffer layouts** — **NONE**. All instance data comes from `storage` buffer addressed by `@builtin(instance_index)`. Unit-quad vertices unpacked from `@builtin(vertex_index)`:
```wgsl
let x = f32(vertex_index & 1u);
let y = f32((vertex_index & 2u) >> 1u);
```
Topology: `TriangleStrip` everywhere EXCEPT `path_rasterization` which uses `TriangleList`.

**NDC normalize** (copy verbatim into every vertex shader, Zed [shaders.wgsl:170-171](APP/zed-main/crates/gpui_wgpu/src/shaders.wgsl#L170-L171)):
```wgsl
device_position = position / viewport_size * vec2(2.0, -2.0) + vec2(-1.0, 1.0);
```

**Quad instance struct** (Zed [shaders.wgsl:518-527](.../shaders.wgsl#L518-L527)):
```wgsl
struct Quad {
    order: u32,
    border_style: u32,
    bounds: Bounds,
    content_mask: Bounds,
    background: Background,
    border_color: Hsla,
    corner_radii: Corners,
    border_widths: Edges,
}
```

**Rounded-rect SDF** (copy verbatim, Zed [shaders.wgsl:371-386](.../shaders.wgsl#L371-L386)):
```wgsl
fn quad_sdf_impl(corner_center_to_point: vec2<f32>, corner_radius: f32) -> f32 {
    if (corner_radius == 0.0) {
        return max(corner_center_to_point.x, corner_center_to_point.y);
    } else {
        let signed = length(max(vec2<f32>(0.0), corner_center_to_point)) +
                     min(0.0, max(corner_center_to_point.x, corner_center_to_point.y));
        return signed - corner_radius;
    }
}
```

**Helper functions to port verbatim** ([shaders.wgsl](.../shaders.wgsl) line refs):
- `blend_color()` :390-394 (alpha premultiplication conditioned on `globals.premultiplied_alpha`)
- `hsla_to_rgba()` :241-273 + `linear_srgb_to_oklab()` / `oklab_to_linear_srgb()` :277-310 (gradient interpolation)
- `gaussian()` :319-321 + `erf()` :324-330 + `blur_along_x()` :332-337 (shadow rendering)
- `quarter_ellipse_sdf()` :932-941 (non-uniform border widths)
- `enhance_contrast()` :45-78 (subpixel text sharpening, REC.601 luminance)
- `pick_corner_radius()` :340-354 (quadrant-based corner selection)
- Wavy underline formula :1154-1184 (`WAVE_FREQUENCY = 2.0`, `WAVE_HEIGHT_RATIO = 0.8`, sine + derivative distance)

**Surface config** (Zed [wgpu_renderer.rs:263-339](APP/zed-main/crates/gpui_wgpu/src/wgpu_renderer.rs#L263-L339)):
- Format prefer: `Bgra8Unorm` → `Rgba8Unorm` → first non-sRGB → first
- Alpha: `PreMultiplied` (transparent) OR `Opaque` (no transparency needed)
- Present mode: `Fifo` default; respect user override if supported
- `desired_maximum_frame_latency: 2`

**Path rasterization** (the odd pipeline, [wgpu_renderer.rs:746-935](.../wgpu_renderer.rs#L746-L935)):
- Pass 1 (`vs_path_rasterization` + `fs_path_rasterization`): TriangleList, renders into intermediate RGBA texture with MSAA, blend = `PREMULTIPLIED_ALPHA_BLENDING`
- Pass 2 (`vs_path` + `fs_path`): TriangleStrip, samples the intermediate, custom blend (color: `(One, OneMinusSrcAlpha, Add)`, alpha: `(One, One, Add)`) = additive for winding-order fill
- MSAA sample count queried from adapter: `[4, 2, 1].find(|n| format_features.flags.sample_count_supported(*n))`

**Device creation** (Zed [wgpu_context.rs:117-149](APP/zed-main/crates/gpui_wgpu/src/wgpu_context.rs#L117-L149)):
- `required_features = if adapter.features().contains(DUAL_SOURCE_BLENDING) { DUAL_SOURCE_BLENDING } else { empty() }` — graceful fallback to mono-only text
- `required_limits = Limits::downlevel_defaults().using_resolution(adapter.limits()).using_alignment(adapter.limits())`
- `memory_hints: MemoryHints::MemoryUsage`

**Clip via manual distance** (NOT `@builtin(clip_distance)`) — Zed TODOs at :534, :964, :1036, :1137. Pattern: compute `clip_distances` in vertex, early-return `vec4(0.0)` in fragment if any negative.

**Unit constants for Nom's shaders.wgsl**:
- Antialiasing half-width: `0.5` (for SDF alpha band)
- Wavy underline: `WAVE_FREQUENCY = 2.0`, `WAVE_HEIGHT_RATIO = 0.8`
- REC.601 luminance: `vec3(0.30, 0.59, 0.11)`
- Light-on-dark contrast multiplier: `saturate(4.0 * (0.75 - brightness))`

### Phase 1 batch-2 updated wave plan

**wave-1** (shaders + buffers + GpuContext) — **starting now per task.md header**
- `shaders.wgsl` single file with 4 MVP entries (vs_quad/fs_quad, vs_mono_sprite/fs_mono_sprite) — copy Zed helpers
- 3 bind group layouts (globals + instances + instances_with_texture)
- `WgpuPipelines` struct holding 2 MVP pipelines (quads, mono_sprites)
- `WgpuInstanceBuffer` with 2× growth + 20-frame guard
- `GpuContext` with device-lost flag + DUAL_SOURCE_BLENDING optional feature

**wave-2** (atlas + text + window)
- `WgpuAtlas` replacing `InMemoryAtlas` (etagere per kind, `queue.write_texture` in `before_frame`)
- `text_rasterization.rs` (cosmic-text ShapeLine + swash Render::new)
- `window.rs` + `frame_loop.rs` with winit 0.30 `ApplicationHandler`

**wave-3** (complete 8 pipelines)
- Add shadows, underlines, path_rasterization (2-pass), path, subpixel_sprites (behind feature flag), poly_sprites
- Platform feature flags `native` / `web`
- Hit-testing wiring + Element state storage
- Integration tests

---

## Phase 2 — `nom-canvas-core` + `nom-editor` (decomposition)

Reference reads (this iteration, 3 parallel Explore agents):
- **Excalidraw** `packages/element/` — base element, 8 shapes, hit-test, selection, transform handles, distribution
- **Zed editor** `crates/editor/src/` — buffer, multi-cursor, tree-sitter, inlays, display pipeline
- **AFFiNE GFX** `blocksuite/framework/global/src/gfx/` — viewport, zoom-to-point, coord transforms, grid spatial index

### Part A — `nom-canvas-core` crate (infinite-canvas)

| Module | Responsibility | Pattern sources (file:line) |
|--------|---------------|------------------------------|
| `element.rs` | Base `CanvasElement` trait: `id`, `bounds`, `angle`, `stroke/fill`, `opacity`, `locked`, `group_id`, `frame_id`, `version`, `version_nonce`, `z_index`, `is_deleted`, `bound_elements` | Excalidraw [types.ts:40-82](APP/Accelworld/services/other5/excalidraw-main/packages/element/src/types.ts#L40-L82) |
| `mutation.rs` | `mutate_element()` (in-place, bumps version + nonce); `new_element_with()` (immutable `{...el, ...updates}` for undo snapshots) | Excalidraw [mutateElement.ts:37-178](.../mutateElement.ts#L37-L178) |
| `shapes/mod.rs` | 8 shape variants via enum: `Rectangle`, `Ellipse`, `Diamond`, `Line`, `Arrow` (elbowed option), `Text`, `FreeDraw` (points + pressures), `Image` (fileId + crop) | Excalidraw shape structs |
| `hit_testing.rs` | 2-stage: `is_point_in_rotated_bounds()` AABB fast-reject → per-shape distance: rect `distanceToRectanguloidElement`, diamond sides, ellipse closed-form, line/arrow/freedraw per-segment + bezier | Excalidraw [collision.ts:117-190](.../collision.ts#L117-L190), [distance.ts:29-147](.../distance.ts#L29-L147). **Tolerance = `stroke_width / 2`**. **Cache with WeakRef-style invalidation keyed on `(point, id, version, version_nonce, threshold)`** |
| `viewport.rs` | `Viewport { center: Point, zoom: f32, size: Size }`; bounds `0.1..=10.0` (wider than AFFiNE's `0.1..=6.0` — we need deep-zoom for map-like views); signals for `viewport_updated`/`zooming`/`panning` | AFFiNE [viewport.ts:20-537](APP/AFFiNE-canary/blocksuite/framework/global/src/gfx/viewport.ts#L20-L537) |
| `coords.rs` | `to_model(view_x, view_y) = [viewport_x + vx/zoom/view_scale, ...]`; `to_view()` inverse; **separate translate+scale, NOT matrix** (clearer for debug); `view_scale` DPI factor | AFFiNE [viewport.ts:476-488](.../viewport.ts#L476-L488) |
| `zoom.rs` | `zoom_to_point(pivot, new_zoom)`: `offset = center - pivot; new_center = pivot + offset * (prev_zoom / new_zoom)`. Wheel step `0.1` normalized via `normalize_wheel_delta_y` log-amplification; discrete step `0.25` | AFFiNE [viewport.ts:514-537](.../viewport.ts#L514-L537), [consts.ts:5](APP/AFFiNE-canary/blocksuite/affine/blocks/surface/src/consts.ts#L5) |
| `pan.rs` | space+drag, middle-mouse, trackpad two-finger. Auto-pan at edges: `cal_pan_delta()` max ±30px/tick when pointer nears viewport bounds. **Instant pan (no inertia)** — RAF debouncing for animation only | AFFiNE [panning-utils.ts:4-40](.../panning-utils.ts#L4-L40) |
| `fit.rs` | `fit_to_all(elements, padding)`, `fit_to_selection(ids, padding)`: zoom = `min((w - pad_lr) / bound_w, (h - pad_tb) / bound_h)`, clamped; if zoom < min, padding shrinks to fit. Supports percentage (0..1) or absolute px | AFFiNE [viewport.ts:463-502](.../viewport.ts#L463-L502) |
| `spatial_index.rs` | **Grid-based** (NOT R-tree). `DEFAULT_GRID_SIZE = 3000` model units. `get_grid_index(val) = ceil(val / 3000) - 1`. Element stored in every cell it overlaps. `search(bound, filter)` returns sorted hits. Cache-friendlier than per-element R-tree for dense canvases | AFFiNE [grid.ts:19-296](.../grid.ts#L19-L296). **NOTE:** Excalidraw has no spatial index (linear scan) — we MUST add one |
| `selection.rs` | `Selection { selected_ids: HashSet<ElementId>, hovered: Option<ElementId>, pending: Option<Marquee> }`. O(1) lookup. Ignore locked + deleted. Group-expand via `select_groups_for_selected_elements` | Excalidraw [Scene.ts:126-134](.../Scene.ts#L126-L134), [selection.ts:41-440](.../selection.ts#L41-L440) |
| `marquee.rs` | Rubber-band with `contain` vs `overlap` modes. Overlap: bounds check → linear/freedraw point-in-bounds → intersection vs selection edges. **Frame clipping**: marquee ∩ enclosing-frame-bounds | Excalidraw [selection.ts:91-337](.../selection.ts#L91-L337) |
| `transform_handles.rs` | 8 resize (`n,s,e,w,ne,nw,se,sw`) + 1 rotation. Size per pointer type: mouse 8px / pen 16px / touch 28px, divided by zoom. Omit-sides param for mobile. Rotate handle positions around center via `TransformationMatrix::rotate()` | Excalidraw [transformHandles.ts:31-201](.../transformHandles.ts#L31-L201) |
| `snapping.rs` | (a) Grid snap: element origin → nearest grid cell. (b) Alignment guides: element edges/centers/midpoints vs other elements + viewport center. (c) Equal-spacing distribution via group bounding boxes. Threshold `8px / zoom`. Render guide lines as separate overlay primitives | Excalidraw [dragElements.ts:35-45](.../dragElements.ts#L35-L45), [distribute.ts:17-100](.../distribute.ts#L17-L100). Guides NOT in Excalidraw element pkg — build fresh |
| `history.rs` | Undo/redo via version snapshots. Store `(before_elements, after_elements)` per transaction. `HistoryEntry { id, timestamp, selection_before, selection_after, element_diffs }` | Zed editor transaction pattern adapted |

### Part B — `nom-editor` crate (text editor over nom-gpui)

| Module | Responsibility | Pattern sources (file:line) |
|--------|---------------|------------------------------|
| `buffer.rs` | Single-buffer (defer `MultiBuffer` to Phase 4). Rope via `ropey` crate. Lamport-clock `TransactionId`. `start_transaction` / `end_transaction` with `transaction_depth` counter for nesting | Zed [language/buffer.rs:99-110](APP/zed-main/crates/language/src/buffer.rs#L99-L110), [editor.rs:19986-20033](APP/zed-main/crates/editor/src/editor.rs#L19986-L20033) |
| `anchor.rs` | Stable buffer positions across edits. Tracks `(offset, bias: Bias::Left\|Right)`. Resolves to current offset via rope seeks | Zed text crate anchor pattern |
| `selection.rs` | `Selection { id: SelectionId, start: Anchor, end: Anchor, reversed: bool, goal: SelectionGoal }`. `SelectionGoal::{None, Column(u32), HorizontalPosition(f32)}` — NOT a raw sticky column; recomputed per vertical move | Zed [text/selection.rs](APP/zed-main/crates/text/src/selection.rs), [movement.rs:87-130](APP/zed-main/crates/editor/src/movement.rs#L87-L130) |
| `selections_collection.rs` | `SelectionsCollection { disjoint: Vec<Selection>, pending: Option<Selection> }`. `all()` merges overlaps on demand at (Zed merging at [lines 134-153](APP/zed-main/crates/editor/src/selections_collection.rs#L134-L153)). Public API: `newest`, `all_adjusted`, `count`, `change_selections` | Zed [selections_collection.rs:31-161](APP/zed-main/crates/editor/src/selections_collection.rs#L31-L161) |
| `movement.rs` | Char: `left`, `right`, `saturating_left`, `saturating_right` (lines 39-81). Vertical: `up`, `down` with `HorizontalPosition(f64)` preserved ([lines 84-130](APP/zed-main/crates/editor/src/movement.rs#L84-L130)). Word-boundary via `CharClassifier`. Goal resets on horizontal move | Zed [movement.rs:39-130](APP/zed-main/crates/editor/src/movement.rs#L39-L130) |
| `editing.rs` | `edit(ranges, texts)`: **sort edits in reverse offset order** before apply so earlier positions stay valid. Atomic via `transact(|| ...)` closure wrapping edit + selection update. Autoindent via `AutoindentMode` enum | Zed [editor.rs:4085-4137](APP/zed-main/crates/editor/src/editor.rs#L4085-L4137) |
| `syntax_map.rs` | `SumTree<SyntaxLayerEntry { tree: tree_sitter::Tree, language: Language, offset: usize }>`. **Incremental `sync()` on buffer edit** — re-parse only affected regions, not whole file. Layer stack for embedded languages | Zed [syntax_map.rs:29-166](APP/zed-main/crates/language/src/syntax_map.rs#L29-L166) |
| `highlight.rs` | Run tree-sitter queries on visible ranges. Map `capture_name` ("function.name", "keyword") → `HighlightId` via theme. Emit `HighlightStyle { color, weight, italic }` spans to renderer | Zed [bracket_colorization.rs:126-132](APP/zed-main/crates/editor/src/bracket_colorization.rs#L126-L132) |
| `inlay_map.rs` | Separate from rope. `SumTree<Transform { Isomorphic(len) \| Inlay(InlayId, len) }>` maps buffer-offset ↔ display-offset. Inlay text counts toward display length, NOT input | Zed [display_map/inlay_map.rs:33-72](APP/zed-main/crates/editor/src/display_map/inlay_map.rs#L33-L72) |
| `inlay_hints.rs` | LSP fetch on visible range. `LspInlayHintData { hint_chunk_fetching: HashMap<Range, Task> }`. Debounce: edit vs scroll separately. Invalidate affected on edit; re-fetch on next scroll/refresh | Zed [inlay_hints.rs:45-126](APP/zed-main/crates/editor/src/inlay_hints.rs#L45-L126) |
| `wrap_map.rs` | Soft-wrap `SumTree<Transform>`. Re-wrap in background task; `interpolated: true` flag during in-flight wrap; finalize on idle. O(log N) seek to row | Zed [display_map/wrap_map.rs:31-150](APP/zed-main/crates/editor/src/display_map/wrap_map.rs#L31-L150) |
| `tab_map.rs` | Pre-compute tab widths; expand to spaces in display layer | Zed `display_map/tab_map.rs` |
| `display_map.rs` | Pipeline chain: `Buffer → InlayMap → FoldMap → TabMap → WrapMap → render`. Each layer's snapshot is cloned-on-write and SumTree-indexed so movement respects display coordinates | Zed display_map architecture |
| `line_layout.rs` | Width measurement via nom-gpui's text system (cosmic-text). **Lazy**: defer to background task, use stale/interpolated snapshot during typing | Zed pattern |
| `lsp_bridge.rs` | Bridge to `nom-lsp` (existing compiler crate) for hover, completion, inlay hints, diagnostics. Reuse existing LSP client; no new transport | nom-compiler/crates/nom-lsp |

### Part C — NON-GOALS (explicitly NOT adapting)

From **Zed**:
- `GPUI Entity/Context` + `AsyncAppContext` — we don't have a GPUI runtime yet; use plain `Rc`/`RefCell` + channels
- `MultiBuffer` — single-buffer first; `MultiBuffer` deferred to Phase 4 when multi-file views matter

From **Excalidraw**:
- `RoughJS` hand-drawn aesthetic — we're GPU shaders; skip the roughjs cache entirely
- DOM event coords (`clientX/clientY` + scroll) — we use native winit pointer events

From **AFFiNE**:
- `RxJS Subject`/`BehaviorSubject` — we use nom-native signals (or `tokio::sync::watch`)
- CSS `transform` per-block + Lit web components — we render via wgpu pipelines
- `will-change: transform` CSS hints — irrelevant on GPU

### Part D — Phase 2 test targets (placeholder — see Phase 2 section above for actual list)

---

## Phase 4 — `nom-graph-v2` + `nom-compose` (universal composition)

Reference reads (iter-7, 3 parallel Explore agents):
- **ComfyUI** `services/other2/ComfyUI-master/execution.py` + `comfy_execution/graph.py` + `comfy_execution/caching.py` — DAG execution, Kahn's topo-sort, 4 cache strategies, IS_CHANGED contract
- **n8n** `services/automation/n8n/packages/{core,workflow,@n8n/expression-runtime}/` — workflow engine, isolated-vm sandbox, retry+continueOnFail, credential injection
- **typst** `services/other5/typst-main/` — comemo incremental compilation, Frame/FrameItem hierarchy, parallel layout
- **Remotion** (DeepWiki overview) — programmatic `renderMedia()`/`bundle()`, headless frame capture, FFmpeg mux

### Part A — `nom-graph-v2` crate (DAG execution — used by Graph mode AND Phase 4 composition)

| Module | Responsibility | Pattern source (file:line) |
|--------|---------------|-----------------------------|
| `topology.rs` | Kahn's algorithm with decrement-based `block_count: HashMap<NodeId, usize>`. Lazy cycle detection at execution time (not pre-validation) — cycles manifest as nodes that never reach `block_count == 0` | ComfyUI [graph.py:107-193, :320-337](APP/Accelworld/services/other2/ComfyUI-master/comfy_execution/graph.py#L107-L193) |
| `execution.rs` | Pull-based execution loop: stage ready nodes → execute → decrement downstream `block_count`. External async tasks block via `unblockedEvent` semaphore | ComfyUI [execution.py:704-786](APP/Accelworld/services/other2/ComfyUI-master/execution.py#L704-L786), graph.py:242-245 |
| `node_schema.rs` | `Node { input_types: Schema, function: fn_name, return_types: Vec<TypeId>, output_is_list: Vec<bool>, side_effects: Vec<EffectKind> }` | ComfyUI INPUT_TYPES/FUNCTION/RETURN_TYPES ([graph.py:65-105](APP/Accelworld/services/other2/ComfyUI-master/comfy_execution/graph.py#L65-L105)) |
| `fingerprint.rs` | `Node::fingerprint_inputs(&inputs) -> u64` — receives ONLY constant inputs (no random/time). Hash of class_type + IS_CHANGED result + all ancestor signatures | ComfyUI [execution.py:54-95](APP/Accelworld/services/other2/ComfyUI-master/execution.py#L54-L95), [caching.py:82-127](APP/Accelworld/services/other2/ComfyUI-master/comfy_execution/caching.py#L82-L127) |
| `cache.rs` | Trait `CacheBackend { get, set, poll }` + 4 impls: `None` (no-op), `Lru` (max_size eviction), `RamPressure` (age-biased eviction on OOM signal), `Classic` (hierarchical signature-keyed, default) | ComfyUI [caching.py:103-563](APP/Accelworld/services/other2/ComfyUI-master/comfy_execution/caching.py) |
| `subcache.rs` | Hierarchical subcache for subgraph expansion — keys scoped to `(parent_id, child_id_set)` so sibling subgraphs don't pollute | ComfyUI [caching.py:361-408](APP/Accelworld/services/other2/ComfyUI-master/comfy_execution/caching.py#L361-L408) |
| `progress.rs` | `ProgressHandler` trait + websocket/channel dispatcher; nodes call `ctx.update(value, max)` + `ctx.check_interrupted()` cooperatively | ComfyUI [progress.py:34-78](APP/Accelworld/services/other2/ComfyUI-master/comfy_execution/progress.py#L34-L78), [execution.py:472, :634](APP/Accelworld/services/other2/ComfyUI-master/execution.py#L472) |
| `cancel.rs` | `InterruptFlag: Arc<AtomicBool>` polled by nodes at safe points; `InterruptError` raised + caught cleanly in execution loop; async pending tasks propagate via `ExternalBlocker` | ComfyUI [model_management.py interrupt_processing_flag](APP/Accelworld/services/other2/ComfyUI-master/comfy/model_management.py), [execution.py:594-602, :533-541](APP/Accelworld/services/other2/ComfyUI-master/execution.py#L594-L602) |
| `error.rs` | Fail-fast per-node; preserve successful-upstream cache; `OUTPUT_NODE` marker for UI precedence (executed even if no downstream needs output) | ComfyUI [execution.py:414-686, :760-762](APP/Accelworld/services/other2/ComfyUI-master/execution.py#L414-L686) |

### Part B — `nom-compose` crate (universal composition backends)

| Module | Responsibility | Pattern source |
|--------|---------------|----------------|
| `backend_trait.rs` | `trait CompositionBackend { fn kind(&self) -> NomKind; async fn compose(&self, spec: &Spec, progress: &ProgressHandler, interrupt: &InterruptFlag) -> Result<Output>; }` | Nom-native dispatch layer |
| `dispatch.rs` | `ComposeDispatcher { backends: HashMap<NomKind, Arc<dyn CompositionBackend>> }`. Routes `nom compose <spec>` by kind | Nom-native |
| `task_queue.rs` | Async task queue for >10s operations; tokio-based; per-task progress channel + cancel handle; max concurrency per backend (video 2, image 4, data 8, etc.) | Rust tokio standard patterns |
| `provider_router.rs` | **3-tier fallback per blueprint §12**: Subscription → Cheap → Free. Per-vendor quota `{ used, limit, reset_at }`. Format translation at boundary (Claude↔OpenAI↔Gemini message schemas) | Blueprint §12 + 9router pattern (iter-3 table) |
| `credential_store.rs` | AES-encrypted on-disk JSON; `get_credential(kind, id) -> Result<Decrypted>` decrypted at runtime; never passed through backend `Spec` or serialized output | n8n [credentials.ts:48-65](APP/Accelworld/services/automation/n8n/packages/core/src/credentials.ts#L48-L65) |

### Part C — 6 concrete backends (one per Nom kind)

**media/video** — Remotion-pattern in Rust (adapted — no React)
- `media_video_backend.rs` — consumes nom-gpui `Scene` per frame; parallel frame rasterize via rayon (typst pattern); buffered channel `mpsc::channel(32)` → FFmpeg stdin
- Frame format: PNG via nom-gpui offscreen `TextureView` render OR raw RGBA (flag-configurable); pipe = stdin by default, named-pipe fallback for Windows
- Backpressure: bounded channel; if FFmpeg slow, scene paint blocks → natural frame-rate throttle
- `stitch_frames_to_video(frames_iter, fps, codec) -> Mp4Output` — mirrors Remotion's `stitchFramesToVideo()`
- Ref: Remotion (DeepWiki) + typst Engine::parallelize ([engine.rs:51-100](APP/Accelworld/services/other5/typst-main/crates/typst/src/engine.rs#L51-L100))

**media/image** — diffusion + tiling
- `media_image_backend.rs` — dispatches to on-device (candle/burn) OR cloud (Open-Higgsfield 200+ models); provider chosen by cost + quality
- For tile-based upscale: Kahn-scheduled DAG of tile nodes via `nom-graph-v2`
- Ref: Open-Higgsfield (blueprint reference)

**screen/web** — ToolJet + Dify pattern adapted
- `screen_web_backend.rs` — consumes a `Screen` Nom kind (widgets + layout + data bindings) → emits HTML+WASM bundle OR hosted Dify-style workflow
- Widget lib: 55-widget catalog from ToolJet, reimplemented as Nom primitives
- Ref: ToolJet `APP/ToolJet-develop/`, Dify `APP/Accelworld/services/other4/dify-main/`

**screen/native** — LLVM codegen
- `screen_native_backend.rs` — delegates to `nom-llvm` (existing compiler crate); emits platform binary (ELF/Mach-O/PE) via LLVM
- Ref: `nom-compiler/crates/nom-llvm/`

**data/extract** — opendataloader-pdf XY-Cut++
- `data_extract_backend.rs` — accepts `Spec { source: Path, schema: Option<Schema> }`; emits JSON/CSV; uses XY-Cut++ for PDF layout reconstruction
- Ref: opendataloader-pdf (blueprint reference)

**data/query** — WrenAI semantic layer
- `data_query_backend.rs` — `Spec { mdl: ModelDefinition, query: NomProse }`; uses MDL (Modeling Definition Language) to ground LLM-generated SQL; validates output against schema before exec
- Ref: WrenAI `APP/wrenai/`

**concept/document** — typst + comemo
- `concept_doc_backend.rs` — Nom prose/blocks → `Content` tree → memoized `layout_document_impl` → Frame hierarchy → PDF (via krilla) / PNG (tiny-skia) / SVG (typst-svg pattern)
- Wrap `nom_doc_world` as `Tracked<dyn NomWorld>` trait; mark layout fns with `#[comemo::memoize]` OR port memoization inline (see "Patterns to SKIP" below re: comemo dep)
- Ref: typst [crates/typst/src/lib.rs:33-158](APP/Accelworld/services/other5/typst-main/crates/typst/src/lib.rs#L33-L158), [typst-library/src/layout/frame.rs:18-30](APP/Accelworld/services/other5/typst-main/crates/typst-library/src/layout/frame.rs#L18-L30)

**scenario/workflow** — n8n-pattern with Nom AST sandbox
- `scenario_workflow_backend.rs` — pull-based stack execution; max 1 node at a time; data-dependent queueing via `waiting_execution: HashMap<NodeId, PendingInputs>`
- Retry semantics: `{ retry_on_fail: bool, max_tries: u8 (max 5), wait_between_tries_ms: u32 (max 5000) }` — inline retry loop per node
- `continue_on_fail` + `on_error` route: `ContinueRegularOutput` | `ContinueErrorOutput` | `StopWorkflow`
- Webhook resume: persist `(node_execution_stack, run_data)` per `stored_at` policy; webhook payload triggers `run_partial_workflow2()`
- Ref: n8n [workflow-execute.ts:123, :1509, :1609-1705, :1854-1906](APP/Accelworld/services/automation/n8n/packages/core/src/execution-engine/workflow-execute.ts#L123)

### Part D — `.nom` user-script AST sandbox (shared across backends)

**Most valuable pattern from iter-7** — every backend may evaluate user-authored `.nom` scripts. Unified sandbox.

| Module | Responsibility | Pattern source |
|--------|---------------|----------------|
| `sandbox/isolate.rs` | Wrap user code in isolated WASM instance (wasmtime) OR V8 isolate via `rusty_v8` — 128MB mem limit, 5s timeout | n8n `isolated-vm-bridge.ts:145` (128MB + 5000ms) |
| `sandbox/ast_visitor.rs` | Walk Nom AST before execution; apply 3 sanitizers below. Port of n8n @n8n/tournament visitor | n8n [expression-sandboxing.ts:76-232](APP/Accelworld/services/automation/n8n/packages/workflow/src/expression-sandboxing.ts) |
| `sandbox/this_sanitizer.rs` | Replace `this` identifier with `EMPTY_CONTEXT = { process: {}, require: {}, module: {}, Buffer: {} }` and `.bind(EMPTY_CONTEXT)` on all function expressions | n8n ThisSanitizer (line 137) |
| `sandbox/prototype_sanitizer.rs` | Block `Object.defineProperty`, `setPrototypeOf`, `getOwnPropertyDescriptor(s)`, `__defineGetter__`, `__lookupGetter__`. Wrap `Object` static in `Proxy` that returns `undefined` for blocked methods | n8n [expression.ts:62-105](APP/Accelworld/services/automation/n8n/packages/workflow/src/expression.ts#L62-L105) |
| `sandbox/dollar_validator.rs` | Restrict `$` identifier to function calls or property access (not bare); matches Nom's `$var` scope access convention | n8n DollarSignValidator (line 248) |
| `sandbox/allowlist.rs` | Allowed globals: `DateTime`, `Duration`, `Interval` (from a date-time crate), `extend()`/`extendOptional()`, lazy-proxy user data (read-only). Blocked: process, require, module, Buffer, global, globalThis, `Error.prepareStackTrace` (V8 RCE vector) | n8n same |

**Critical sanitizer snippet** (port from n8n [expression.ts:62-105](APP/Accelworld/services/automation/n8n/packages/workflow/src/expression.ts#L62-L105)) — verbatim pattern:
```ts
const blockedMethods = new Set(['defineProperty', 'defineProperties', 'setPrototypeOf',
  'getPrototypeOf', 'getOwnPropertyDescriptor', 'getOwnPropertyDescriptors',
  '__defineGetter__', '__defineSetter__', '__lookupGetter__', '__lookupSetter__']);
return new Proxy(Object, {
  get(target, prop, receiver) {
    if (blockedMethods.has(prop as string)) return undefined;
    if (prop === 'create') return (proto: object | null) => Object.create(proto); // single-arg only
    return Reflect.get(target, prop, receiver);
  }
});
```

### Part C — backfill from blueprint §12 (5 backends missed in iter-7)

Iter-9 re-read spec §12 and caught **5 backends named in the universal-composition table that weren't decomposed**. Adding now.

**media/storyboard — waoowaoo 4-phase orchestration**
- `media_storyboard_backend.rs` — 4 phases: Phase 1 Planning → (Phase 2a Cinematography ∥ Phase 2b Acting) parallel via `tokio::join!` → Phase 3 Detail enrichment → Phase 4 Asset generation (via task_queue) → Phase 5 FFmpeg composite
- Phase-result type: `PhaseResult { clip_id, plan_panels: Option<Vec<Panel>>, photography_rules: Option<...>, acting_directions: Option<...> }` mirroring waoowaoo ([storyboard-phases.ts:140-146](APP/Accelworld/services/media/waoowaoo/server/processing/storyboard-phases.ts#L140-L146))
- Explicit `NomMediaPhase = Decompose | Cinematography | Acting | Detail | Ffmpeg` enum with retry-once per phase + field validation (ref waoowaoo [storyboard-phases.ts:323-371](APP/Accelworld/services/media/waoowaoo/server/processing/storyboard-phases.ts#L323-L371))
- Prompt construction: template-replace (NOT Jinja, NOT LLM-generated) for determinism + auditability

**media/novel→video — ArcReel agent workflow adapted (Claude SDK → nom-intent)**
- `media_novel_video_backend.rs` — **CRITICAL ADAPTATION**: ArcReel uses Claude SDK directly; Nom MUST use `nom-intent` ReAct agents instead (per blueprint "zero foreign identities" + "direct-linked compiler" mandates)
- 5 specialized agents via nom-intent: `novel_analyst` → `script_writer` → `character_designer` → `storyboard_artist` → `video_composer`
- **Skill vs Subagent boundary** (ArcReel pattern): Skills = deterministic Rust fns (file I/O, API calls wrapped in Nom-owned backend facades); Subagents = reasoning tasks via nom-intent ReAct (ref ArcReel AGENTS.md:196-198)
- Typed handoffs — `NovelAnalysis → ScriptJSON → CharacterDesign → StoryboardPanels → VideoArtifact` (strong Rust types, no freeform prose between agents)
- Session checkpoint storage in `nom-dict` with `EntryKind::AgentSession` so mid-pipeline resume possible (ref ArcReel [session_store.py](APP/Accelworld/services/other4/ArcReel-main/server/agent_runtime/session_store.py))
- `UsageTracker` per-call cost recording: `{ agent, model, input_tokens, output_tokens, usd_cost }` (ref ArcReel [lib/usage_tracker.py:60-88](APP/Accelworld/services/other4/ArcReel-main/lib/usage_tracker.py#L60-L88))
- **Backend facade pattern**: all external LLM calls routed through Nom-owned `MediaVendor` (see Part G below) — no `anthropic-python`/`anthropic-rust` direct imports

**media/audio — synthesis + codec**
- `media_audio_backend.rs` — accepts `Spec { text: Option<NomProse>, voice_id, sample_rate, format: Flac|Aac|Mp3|Opus }`
- Synthesis dispatch: on-device (whisper, rodio) OR cloud vendor; choice via cost+quality
- waoowaoo voice worker pattern (parallel synthesis) + `nom-media` codec abstractions for FLAC/AAC/Opus/MP3
- Timing-alignment layer for lip-sync when paired with media/video output

**data/transform — Polars MVP (nom-data crate, NO polars crate dep)**
- `nom-compiler/crates/nom-data/` — NEW crate, minimum viable columnar engine (~3000 LoC estimate)
- `series.rs` — `Series<T>` = `Arc<Vec<T>>` + `Arc<Bitmap>` for nulls; bit-packed bitmap (ref polars [bitmap/immutable.rs:56-68](APP/Accelworld/upstreams/polars/crates/polars-arrow/src/bitmap/immutable.rs#L56-L68))
- `chunked_array.rs` — `ChunkedArray<T> = Vec<Arc<Vec<T>>>` with `rechunk()` for SIMD perf (ref polars [chunked_array/mod.rs:122, :138-147](APP/Accelworld/upstreams/polars/crates/polars-core/src/chunked_array/mod.rs#L138-L147))
- `dtype.rs` — minimum viable `enum DataType { Int8-Int64, UInt8-UInt64, Float32/64, Bool, String, Date, List(Box<DataType>), Null }` (skip Categorical/Enum/Decimal v1; ref polars [dtype.rs:90-145](APP/Accelworld/upstreams/polars/crates/polars-core/src/datatypes/dtype.rs#L90-L145))
- `simd.rs` — `std::simd::prelude` direct (NOT arrow2/wide crate); filter, map, compare kernels. AVX-512 fast path with `_mm512_maskz_compress_epi8` for x86_64; fallback scalar (ref polars [comparisons/simd.rs:2](APP/Accelworld/upstreams/polars/crates/polars-compute/src/comparisons/simd.rs#L2) + [filter/avx512.rs:48-62](APP/Accelworld/upstreams/polars/crates/polars-compute/src/filter/avx512.rs#L48-L62))
- `plan.rs` — `enum DslPlan { Scan, Filter, Project, GroupBy, Join, Sort }` logical IR; compile to physical via optimizer
- `optimizer.rs` — 2 rules MVP: predicate pushdown + projection pushdown. **SKIP** 20+ additional rules (CSE, slice pushdown, join reordering) — overkill for MVP (ref polars [predicate_pushdown/mod.rs:56-90](APP/Accelworld/upstreams/polars/crates/polars-plan/src/plans/optimizer/predicate_pushdown/mod.rs#L56-L90))
- `join.rs` — hash join default: build hash table on smaller relation, probe with larger (ref polars [hash_join/mod.rs:29-48](APP/Accelworld/upstreams/polars/crates/polars-ops/src/frame/join/hash_join/mod.rs#L29-L48))
- `parallel.rs` — rayon `POOL` wrapper with `.install()` / `.join()` — same pattern as polars (ref polars [lib.rs:39-96](APP/Accelworld/upstreams/polars/crates/polars-core/src/lib.rs#L39-L96))
- `data_transform_backend.rs` — wires nom-data into CompositionBackend trait; accepts `Spec { input: DataSource, pipeline: NomProse }`
- `data_extract_backend.rs` stays in Part C; data/extract and data/transform share dtype + Series types

**media/3D — mesh composition (glTF)**
- `media_3d_backend.rs` — `Spec { mesh: NomMeshSource, materials, animations, export: Gltf|Glb }`
- `nom-media::MeshGeometry` kind (blueprint §8)
- glTF 2.0 export via native Rust writer (no C bindings — use `gltf-json` crate OR write own encoder)
- Scene composition: combine multiple `MeshGeometry` + `Material` + `AnimationClip` into single glTF scene

### Part F-bis — FallbackStrategy 3-variant enum (blueprint §15)

Iter-7 had `provider_router` but missed the 3 concrete strategies spec'd in §15.

```rust
pub enum FallbackStrategy {
    Fallback,   // try tier 1 → tier 2 → tier 3 on each failure
    RoundRobin, // load balance across tiers (distributes quota pressure)
    FillFirst,  // drain tier 1 quota before touching tier 2 (cheapest routing)
}
```

- `provider_router.rs` — `combo_strategies: HashMap<String, FallbackStrategy>` — per-combo override (e.g. "image-hi-fidelity" uses Fallback, "text-bulk-batch" uses FillFirst)
- Default: `Fallback` (blueprint implies this is canonical behavior)
- User-configurable per vendor combo via credentials panel (blueprint §15 comment: "Tier 1: Claude API (subscription) — used first, Tier 2: Local Ollama (cheap) — fallback, Tier 3: Free API (rate-limited) — last resort")

### Part B-bis — `nom-collab` transactor (blueprint §16 immutable event log)

Iter-8 decomposed `nom-collab` but missed the 4th field in `CollaborationEngine`: the `transactor: TransactionLog`.

- `transactor.rs` — immutable append-only event log: `Vec<Transaction { timestamp, client_id, doc_id, yrs_update: Uint8Array }>` persisted to SQLite OR append-only file
- Separate from Y.Doc state snapshot (Part B `persistence.rs`): the transactor records EVERY update for audit, debugging, and time-travel; snapshots are periodic checkpoints
- Retention policy: keep full history OR compact to state-vector snapshots every N updates (configurable; default: keep 30 days, snapshot daily)
- Exposes `replay(doc_id, until: Timestamp) -> Y.Doc` for time-travel debugging + audit queries

### Part C-ter — Remotion concrete VideoComposition struct (blueprint §18)

Iter-7 had `media_video_backend` at high level. Blueprint §18 provides the concrete struct. Adopt:

```rust
pub struct VideoComposition {
    pub fps: u32,
    pub duration_frames: u32,
    pub width: u32,
    pub height: u32,
    pub scenes: Vec<SceneEntry>,
}

pub struct SceneEntry {
    pub from_frame: u32,       // Remotion's Sequence.from
    pub duration: u32,          // Remotion's Sequence.durationInFrames
    pub entity_hash: ContentHash, // content-addressed nomtu reference (Part H artifact store)
}

impl VideoComposition {
    fn render_frame(&self, frame: u32, scene: &mut Scene) {
        // 1. Find active scenes at this frame (Remotion Sequence visibility)
        let active = self.scenes.iter()
            .filter(|s| frame >= s.from_frame && frame < s.from_frame + s.duration);
        // 2. Each scene paints to GPU scene graph; relative_frame = frame - scene_entry.from_frame
        for e in active {
            let relative_frame = frame - e.from_frame;
            e.paint(relative_frame, scene);
        }
    }
}
```

**Why Nom's version beats Remotion:**
- **No browser** — Remotion uses Puppeteer; Nom renders via wgpu directly (blueprint §18 "Why this is better")
- **No V8** — pure Rust, zero JS overhead
- **GPU-accelerated** — wgpu scene → framebuffer at GPU speed, not DOM layout speed
- **Content-addressed** — `SceneEntry.entity_hash` enables per-scene render caching (hit artifact_store → skip re-render)
- **Same renderer** — canvas preview AND video export use identical GPU pipeline (write once, render twice)

Blueprint §18's example `.nomx` source shows the user-facing flow:
```nomx
the media product_video is
  intended to create a 60-second product showcase.
  composes intro_card, product_spin, feature_list, cta_outro.
```
→ Compiler S1-S6 → CompositionPlan → `VideoComposition` with 4 SceneEntry → `export()` → MP4 in artifact_store.

---

### Part G — `MediaVendor` trait abstraction (backfill from blueprint §12 + §15)

Blueprint §12 names `trait MediaVendor`; iter-7 had only `provider_router`. Split properly.

- `vendor_trait.rs` — `pub trait MediaVendor: Send + Sync { fn name(&self) -> &str; fn capabilities(&self) -> Vec<Capability>; async fn generate(&self, req: GenerateRequest) -> Result<GenerateResponse>; fn cost_per_request(&self) -> Cost; }`
- Built-in vendors: `LocalVendor` (nom-llvm, candle on-device) + `CloudVendor` implementations per provider (Anthropic, OpenAI, Gemini, StabilityAI, etc.)
- `capabilities.rs` — `enum Capability { Text, Image, Video, Audio, Embedding, Code, ToolUse, Vision }` — used by router to find matching vendors
- `cost.rs` — `struct Cost { cents_per_1k_input_tokens, cents_per_1k_output_tokens, fixed_cents_per_request }`
- `format_translator.rs` — Claude↔OpenAI↔Gemini message-schema translation at the boundary (not inside each backend)

### Part H — `nom-compose/artifact_store.rs` — content-addressed artifact store (blueprint §12)

Blueprint §12 mandates: `~/.nom/store/<hash>/body.*` content-addressed artifact path.

- `artifact_store.rs` — `struct ArtifactStore { root: PathBuf }` (default `$HOME/.nom/store/`)
- `put(artifact: Artifact) -> ContentHash` — SHA-256 over bytes; write to `<root>/<hash[..2]>/<hash>/body.<ext>`
- `get(hash: ContentHash) -> Result<Artifact>` — read back, verify hash
- `gc(keep_referenced: &[ContentHash])` — remove artifacts not in the keep-set (scheduled daily)
- Metadata sidecar: `<hash>/meta.json` with `{ kind, mime_type, created_at, source_spec_hash, generation_cost_cents }`

### Part I — Compose-output preview block types (blueprint §12 Canvas Integration)

Blueprint §12 specifies 6 preview block types for composition output. These go in `nom-blocks/compose/` subfolder.

- `compose/video_block.rs` — Timeline preview with scrubber + generated-frames thumbnail strip; click-to-play
- `compose/image_block.rs` — Generated image + variant picker (when vendor returned N variants)
- `compose/document_block.rs` — Paginated typst-rendered preview with page nav
- `compose/data_block.rs` — Table view with column-type indicators + sort/filter controls
- `compose/app_block.rs` — Live app preview (web: iframe-isolated render; native: binary info card)
- `compose/audio_block.rs` — Waveform visualizer + playback controls

All 6 delegate to existing `nom-blocks` Block infra (schema + transformer); only `render()` differs.

### Part J — `data_query_backend.rs` detail (blueprint §14 WrenAI 5-stage pipeline)

Iter-7 had `data_query_backend` as one-liner. Blueprint §14 specifies 5-stage pipeline. Decompose:

- **Stage 1: Intent classification** — is this request a query / chart / insight? Use `nom-intent` ReAct classifier
- **Stage 2: Vector retrieval** from semantic model — Qdrant-like embedding index over `SemanticEntity` + `DerivedMetric` + `EntityRelation`; top-k relevant entities fetched
- **Stage 3: LLM-generated grounded query** — prompt includes MDL (Modeling Definition Language) context from step 2; LLM produces SQL / Cypher / etc.
- **Stage 4: Correction loop** — syntax validator runs; if parse fails, feed error back to LLM for correction; max 3 iterations
- **Stage 5: Execute** — query runs against data source; result returned to canvas

Structures (blueprint §14 has the spec):
- `SemanticModel { entities: Vec<SemanticEntity>, metrics: Vec<DerivedMetric>, relationships: Vec<EntityRelation> }`
- `SemanticEntity { name, table, columns, business_meaning }`
- `DerivedMetric { name, formula (e.g. "SUM(orders.price * orders.quantity)"), grouping, filter }`
- `EntityRelation { from, to, kind: OneToMany|ManyToOne|ManyToMany, join_keys }`

### Part K — NON-GOALS (additions from iter-9)

From **waoowaoo/ArcReel**:
- **Direct Claude SDK integration** (ArcReel uses `from claude_agent_sdk import ...` directly) — Nom uses `nom-intent` ReAct agents + Nom-owned `MediaVendor` facades. NEVER import vendor SDKs at the backend layer.
- **Freeform prose between agents** — use typed Rust structs (`NovelAnalysis`, `ScriptJSON`, etc.). ArcReel sometimes does freeform; Nom mandates types.
- **TypeScript Promise.all** → use `tokio::join!` (Rust-native)

From **Polars**:
- **20+ optimizer rules** (CSE, slice/sort/flatten/expand pushdown, join reordering) — MVP ships predicate + projection pushdown only
- **Streaming engine** (batched collect + async I/O + memory spilling) — Nom MVP is in-memory + eager
- **Python interop + pyo3-polars** — Rust-native only
- **arrow2 crate / arrow-rs crate** — Nom implements its own Bitmap + primitive arrays (Polars forked arrow2; we'd end up forking or vendoring)
- **Categorical/Enum/Decimal dtypes** — skip v1; add when real use-case appears

---

### Part E — NON-GOALS (explicitly SKIP, from earlier iterations)

From **ComfyUI**:
- Python-GIL single-process assumptions — Nom targets WASM + multi-process via OMC agents
- PyTorch tensor marshalling (`Tensor.element_size()`) — generalize RAM estimators to ONNX/Vulkan/remote-model agnostic
- Filesystem `custom_nodes/` plugin auto-import — Nom uses explicit registry (nom-cli manifest) with static type validation

From **n8n**:
- Vue UI reflection from TypeScript types — Nom defines scenario schema separately
- NPM-hosted node packages — Nom hardcodes or uses registry file
- Partial-execution graph rewire for AI tool executors — defer; start with sequential

From **typst**:
- Proc-macro `#[comemo::track]` / `#[comemo::memoize]` attribute macros — Nom compiler will have its own memoization primitive. Adopt the PATTERN (Tracked + Constraint), not the crate
- Krilla PDF crate is typst-internal — Nom writes its own PDF backend (part of nom-compose/concept/document)

From **Remotion**:
- React Hooks / Context / Composition components — Nom consumes nom-gpui scene graph directly, not JSX
- Headless Chromium frame capture (Playwright/Puppeteer) — we have native GPU offscreen render
- Webpack/Rspack bundling — not applicable

### Part F — Phase 4 test targets (placeholder — see Phase 4 detailed section above)

---

## Phase 5 — Production Quality

Reference reads (iter-8, 3 parallel Explore agents):
- **yara-x** `upstreams/yara-x/lib/src/compiler/{linters,report,warnings}.rs` — sealed-trait linter, byte-offset diagnostics, runtime rule registration, fix suggestions
- **Huly** `services/other5/huly-main/server/collaborator/` + `foundations/core/packages/text-ydoc/` — Yjs+Hocuspocus CRDT, WebSocket sync, Awareness presence, minimal-collab = 4 services
- **typst comemo** `services/other5/typst-main/crates/typst/src/lib.rs:33-158` — `Tracked<T>` + `Constraint::validate()` + `#[memoize]` with reference-equality keying
- **OpenTelemetry Rust SDK** — W3C trace context, ParentBased(TraceIdRatioBased) sampling, tokio/rayon propagation

### Part A — `nom-lint` crate (Linter framework, yara-x sealed pattern)

| Module | Responsibility | Pattern source |
|--------|---------------|----------------|
| `rule_trait.rs` | Sealed via supertrait binding: `pub trait Rule: RuleInternal {}` + blanket impl `impl<T: RuleInternal> Rule for T {}`; `pub(crate) trait RuleInternal { fn check(&self, report: &ReportBuilder, ast: &AstNode) -> LintResult }` | yara-x [linters.rs:10-18](APP/Accelworld/upstreams/yara-x/lib/src/compiler/linters.rs#L10-L18) |
| `registry.rs` | Runtime registration `linters: Vec<Box<dyn Rule>>` + `add_linter<L: Rule>(&mut self, linter: L)`. No compile-time inventory crate; simpler. | yara-x [mod.rs:416-423, :819-825](APP/Accelworld/upstreams/yara-x/lib/src/compiler/mod.rs#L416-L423) |
| `diagnostic.rs` | `pub struct Diagnostic { span: Span, severity: Severity (Error/Warning/Info), code: &'static str (e.g. "L001"), message: String, fix: Option<Fix> }`. `pub struct Fix { span: Span, replacement: String }` | yara-x [report.rs:26-111](APP/Accelworld/upstreams/yara-x/lib/src/compiler/report.rs#L26-L111), Patch at :46-73 |
| `span.rs` | Byte-offset `Span(Range<u32>)`; `byte_offset_to_line_col(source, offset)` computed on demand, not stored | yara-x [parser/src/lib.rs:41, :64-80](APP/Accelworld/upstreams/yara-x/parser/src/lib.rs#L41) |
| `visitor.rs` | **Nom improvement over yara-x**: dedicated `pub(crate) trait RuleVisitor { fn pre_visit(&mut self, node: &AstNode); fn post_visit(&mut self, node: &AstNode); }` + default walk. yara-x has no visitor (imperative); Nom benefits from one | (Nom-native addition) |
| `incremental.rs` | Incremental relinting keyed on `(file_hash, rule_set_hash, ast_node_hash)`. yara-x re-lints fully on each compile (O(n·m)); Nom caches per (rule, node_hash) — cache hit skips rule.check() | (Nom-native addition, pattern inspired by comemo) |

**SKIP from yara-x:** YARA-specific regex rules, multi-file ReportBuilder (Nom uses per-file linter; future multi-file via registry composition).

### Part B — `nom-collab` crate (Collaboration — minimal Huly pattern, Rust-native)

Huly uses 30 services but minimal-collab = 4: collaborator + presence + account + server core. Nom drops to 2 modules + reuses `nom-dict` for auth.

| Module | Responsibility | Pattern source |
|--------|---------------|----------------|
| `ydoc.rs` | Canvas state as `yrs::Doc` (Rust binding, not Yjs TS). Blocks as `Y.Map`, elements as `Y.Array`, text as `Y.Text`. Encoding: Y.Doc → `Uint8Array` binary via `transaction.encode_state_as_update_v2()`. | Huly [text-ydoc/src/ydoc.ts:19, :35-42](APP/Accelworld/services/other5/huly-main/foundations/core/packages/text-ydoc/src/ydoc.ts#L19) adapted to Yrs |
| `server.rs` | WebSocket server (tokio + tungstenite); per-doc `yrs::Doc` in memory; debounce persistence (10s normal, 60s max). **No Hocuspocus dep** — reimplement the ~12-file protocol in Rust since Hocuspocus is TS-only. | Huly [server/collaborator/src/server.ts:20, :60-105](APP/Accelworld/services/other5/huly-main/server/collaborator/src/server.ts#L20-L105) |
| `sync_protocol.rs` | Dual-channel: (A) WebSocket for real-time update deltas; (B) REST `POST /rpc/:docId` for bulk `getContent`/`createContent`/`updateContent`. Clients use `update_v2` deltas (Uint8Array) | Huly [rpc/methods/updateContent.ts:22-64](APP/Accelworld/services/other5/huly-main/server/collaborator/src/rpc/methods/updateContent.ts#L22-L64) |
| `awareness.rs` | Yrs `Awareness` protocol for ephemeral presence (cursor, selection, user info). Not persisted; lost on disconnect. Separate from doc state | Huly presence plugin + Hocuspocus built-in awareness |
| `auth.rs` | JWT token decode at WebSocket upgrade; check workspace membership; readonly-token → `connection.readonly = true`. **No row-level doc permissions at CRDT level** — enforce at upgrade time | Huly [authentication.ts:36-71](APP/Accelworld/services/other5/huly-main/server/collaborator/src/extensions/authentication.ts#L36-L71) |
| `persistence.rs` | Save Y.Doc binary as blob. Options: (a) `nom-dict` entry with EntryKind::CollabSnapshot (preferred — integrates with dict architecture); (b) PostgreSQL BYTEA (<10MB docs); (c) content-addressed blob store (MinIO-compatible). Daily snapshots to bound write-amp | Huly [storage.ts:61-74](APP/Accelworld/services/other5/huly-main/foundations/server/packages/collaboration/src/storage.ts#L61-L74) |
| `offline.rs` | Client-side Yrs local store (IndexedDB in browser, SQLite on desktop). On reconnect, CRDT merge handles conflicts automatically | Yrs/Yjs CRDT semantics |

**SKIP from Huly:**
- 30-service mesh (kubernetes manifests, multi-region failover, service-to-service RPC)
- MongoDB assumption (use PostgreSQL or nom-dict)
- Kafka event streaming for collab (Hocuspocus in-memory is sufficient; Kafka only for audit/activity)
- TypeScript Hocuspocus (reimplement protocol in Rust)
- Business-domain services (ai-bot, billing, telegram, HR, tracker)

### Part C — `nom-memoize` crate (Incremental compilation, port of typst comemo pattern without the dep)

Per iter-7 SKIP rule: do NOT depend on `comemo` crate; port the pattern.

| Module | Responsibility | Pattern source |
|--------|---------------|----------------|
| `tracked.rs` | `pub struct Tracked<'a, T> { inner: &'a T, constraint: Option<&'a Constraint> }`. Zero-cost newtype; method calls route through auto-generated call-site recording into the active constraint | typst comemo [crates/typst/src/lib.rs:45, :79, :100](APP/Accelworld/services/other5/typst-main/crates/typst/src/lib.rs#L45) |
| `constraint.rs` | `pub struct Constraint { reads: Vec<Read> }`. `fn validate(&self, new_value: &T) -> bool` — returns `true` if all recorded reads match (reference-equality on wrapped Tracked, hash-equality on value types). No TTL; scoped to single compilation iteration | typst [lib.rs:144-158](APP/Accelworld/services/other5/typst-main/crates/typst/src/lib.rs#L144-L158) |
| `memoize_macro/` | `#[nom_memoize]` proc macro — wraps fn body with cache lookup: hash inputs (String/primitives via `Hash + Eq`), reference-equality on `Tracked<T>` params; store result keyed on input-tuple hash. Use `FxHashMap` (not `std::HashMap` — much faster for integer keys) | typst [typst-layout/src/flow/collect.rs](APP/Accelworld/services/other5/typst-main/crates/typst-layout/src/flow/collect.rs) memoized fns |
| `track_macro/` | `#[nom_track]` proc macro — applied to impl block; auto-impl `Track` trait + instrumentation on method calls that records the access into active `Constraint` | typst comemo `#[track]` pattern |
| `cache.rs` | Global cache: `thread_local!(static CACHE: RefCell<FxHashMap<u64, CachedResult>>)`. Thread-local (not cross-thread — typst doesn't use rayon in eval loop either). Clear on compilation boundary via `flush()` | typst single-threaded eval loop [lib.rs:136-186](APP/Accelworld/services/other5/typst-main/crates/typst/src/lib.rs#L136-L186) |

**Thread-safety note:** Explicitly SINGLE-THREAD; memoization cache is thread-local. Cross-thread = fresh cache. Matches typst's design. If Phase 5 needs parallel compilation later, switch to `Arc<Mutex<FxHashMap>>` or sharded cache — design in the trait contract now.

### Part D — `nom-telemetry` crate (Observability, OpenTelemetry Rust SDK)

| Module | Responsibility | Pattern source |
|--------|---------------|----------------|
| `spans.rs` | 4-tier span taxonomy: `ui` (hover/completion, `info` level), `interactive` (S1-S2 classify, intent_analyze, `info` 50% sampled), `background` (S3-S6 pipeline, corpus_index, `debug` 5% sampled), `external` (anthropic_api_call, openai_api_call, `info` always sampled) | OTel Rust best practices |
| `instrument_macro/` | `#[nom_instrument(level = "info", tier = "ui")]` proc macro — thin wrapper over `tracing::instrument` with Nom-specific tags auto-added | `tracing` crate instrument pattern |
| `propagation.rs` | `with_current_otel_context()` for tokio spawn; `extract_trace_context(headers)` + `inject_trace_context(&mut headers)` for WebSocket/HTTP boundaries. W3C traceparent format: `00-<128bit-trace-id>-<64bit-span-id>-<sampled>` | OTel `opentelemetry_sdk::propagation::TraceContextPropagator` |
| `sampler.rs` | `SamplerConfig { env: Env (Dev\|Prod), ratio: f64 }` → `AlwaysOn` (dev) or `ParentBased(TraceIdRatioBased(ratio))` (prod, default 0.01). Trace-ID consistent: all spans in a trace share sampling decision | `opentelemetry_sdk::trace::Sampler` |
| `exporter.rs` | `init_exporter(endpoint: &str) -> TracerProvider`. Configures `BatchSpanProcessor` + W3C propagator. Returns global provider for `tracing-opentelemetry` layer | `opentelemetry-otlp` crate |
| `rayon_bridge.rs` | Helper for rayon threadpool propagation: `rayon_scope(|s| s.spawn(|_| span!("rayon_work").in_scope(|| ...)))` — rayon has no built-in OTel support, explicit pattern | Nom-native helper |

**External API context injection** (for composition backends calling Anthropic/OpenAI):
```rust
let mut headers = reqwest::header::HeaderMap::new();
propagator.inject_context(&cx, &mut HeaderInjector(&mut headers));
client.post(url).headers(headers).send().await
```

### Part E — File watcher + incremental relinting (Pattern S4 from blueprint §10)

| Module | Responsibility | Pattern source |
|--------|---------------|----------------|
| `watcher.rs` | `notify` crate `RecommendedWatcher`; batch-debounce 50ms; emit `ChangeEvent { path, kind: ModifyFile\|CreateFile\|RemoveFile }` | typst batch-debounce pattern (comemo reactivity) |
| `incremental_relint.rs` | On file change: compute new AST node hashes; diff against cached; invalidate only changed nodes' rule results. Full relint when rule set changes | Pattern inspired by comemo `Constraint::validate()` |

### Part F — Phase 5 test targets

- **Sealed trait enforcement** — trybuild compile-fail test: external crate tries `impl Rule for ExternalStruct` → fails with "trait RuleInternal is private"
- **Diagnostic byte-offset→line/col** — ASCII + multi-byte UTF-8 + CRLF line endings, 1000 random offsets round-trip
- **Fix application** — `apply_fix(&mut source, Fix { span, replacement })` produces expected edit; overlapping fixes yield `OverlappingFixes` error
- **Linter incremental relint** — edit 1 node in 1000-node AST; assert only that node's rule results change; others cache-hit
- **CRDT convergence** — 2 clients edit concurrently, random interleave, both converge to identical `Y.Doc` state (property test with 1000 runs)
- **Awareness ephemerality** — client disconnects; awareness state GC'd within 10s; rejoining client sees empty awareness
- **Access control** — non-member JWT → WebSocket upgrade refused with 401; readonly JWT → connection accepts reads, rejects updates
- **Persistence snapshot** — daily cron saves Y.Doc blob; restoration from blob matches original state bit-identically
- **Memoize reference-equality** — two calls with same `Tracked<&Foo>` (same memory address) → cached; different instances with equal value → NOT cached (reference-equality, not value-equality on tracked args)
- **Memoize constraint validate** — mutate tracked value after memoized call, re-call with same args, `Constraint::validate()` returns false → fresh compute
- **Telemetry context propagation** — spawn tokio task, verify child span's `trace_id == parent's trace_id`; spawn rayon work, same; external API call has W3C traceparent injected
- **Sampler consistency** — 1000 traces with ratio 0.01; assert ~10 traces fully sampled (not partial); all spans in a sampled trace share `sampled=1`, all in unsampled share `sampled=0`
- **File watcher debounce** — rapid edit burst (10 edits in 20ms) → 1 ChangeEvent (not 10); edit after 60ms → 2 events

---

## Blueprint-gap backfill (modules named in spec §8 but missed in iter-3/5 decompositions)

Found during iter-8 blueprint re-read. Adding as small subtasks to the right Phases.

### Phase 1 addition (`nom-gpui`)
- **`animation.rs`** — timestamp-based interpolation + easing curves (`cubic-bezier(0.27, 0.2, 0.25, 1.51)` for mode switch per blueprint §5). Drive scene updates at 60fps via winit frame callback

### Phase 2 additions (`nom-editor`)
- **`input.rs`** — keyboard event dispatch + **IME composition events** (CJK, combining marks, compose-key sequences). winit provides `WindowEvent::Ime(Ime::Preedit / Commit)` — route to active cursor's TextBuffer
- **`completion.rs`** — completion UI popup driven by `nom-resolver::resolve()` (3-stage: exact→word→semantic) + `nom-grammar::find_keywords()`. Rendering via nom-gpui popup primitives

### Phase 3 additions (`nom-panels`, `nom-theme`)
- **`properties.rs`** (nom-panels) — right-rail property inspector for selected block/element. Dispatches by block-type flavour; renders editable fields per schema
- **`fonts.rs`** (nom-theme) — Inter + Source Code Pro loading into nom-gpui atlas via cosmic-text. Fallback chain: Inter → system sans → default
- **`icons.rs`** (nom-theme) — Lucide 24px SVG icon set (~1400 icons). Pre-tessellate to Path primitives at load OR rasterize to PolychromeSprite atlas. Lazy-load by usage

---

- **Topological sort correctness** — construct 100 random DAGs with known topological orders; verify output matches
- **Cycle detection lazy trigger** — DAG with cycle executes N non-cyclic nodes first, then raises `CycleError` with cycle participants enumerated
- **IS_CHANGED cache reuse** — call same node twice with identical inputs → second call hits cache; change input → second call misses
- **4-cache-strategy swap** — run same DAG with None/Lru/RamPressure/Classic backends, assert output identical
- **Subcache isolation** — 2 sibling subgraphs with colliding node IDs don't pollute each other's cache
- **Cooperative cancellation** — 10-second node polled at 100ms granularity; interrupt at 500ms; node exits cleanly within one poll interval (<100ms)
- **Retry semantics** — scenario node with `max_tries: 3`, failing twice, passing third: assert 3 attempts + success
- **continueOnFail route** — node fails with `onError: ContinueRegularOutput`: downstream sees empty data, proceeds; with `StopWorkflow`: halts
- **Sandbox escape attempts (security)** — user script attempting `Object.setPrototypeOf`, `process.env`, `require('fs')`, `Error.prepareStackTrace = ...` → all blocked, no effect on host
- **Sandbox timeout** — script running `while(true){}` → killed at 5000ms, host survives
- **Video backend frame pipe** — render 30 test frames, assert FFmpeg stdout contains valid mp4 header + correct duration
- **Document backend incremental** — compile doc twice; second compile reuses ≥90% of memoized layout results (instrument cache hit rate)
- **Provider router fallback** — mock Subscription fails with 429 → Cheap used; Cheap fails with 500 → Free used; Free fails → error bubbled
- **Credential isolation** — scenario workflow logged to disk contains NO credential plaintext (only reference IDs)

---

- **Hit-test correctness** — golden-file per-shape (rect, rotated rect, ellipse, diamond, arrow, freedraw) with boundary points ±tolerance
- **Marquee contain vs overlap** — 2×4 shape grid; select each mode; assert correct subset
- **Zoom-to-point invariant** — under cursor at `(100, 100)` model coords, zoom 1.0 → 3.0, assert cursor still at `(100, 100)`
- **Coord round-trip** — `to_view(to_model(v)) == v` for 1000 random points at zoom ∈ {0.1, 0.5, 1.0, 2.5, 10.0}
- **Grid spatial index** — insert 100k elements, query 1000 random bounds, verify linear-scan equivalence (same result set, sorted)
- **Multi-cursor reverse-offset edit** — 3 cursors at P1<P2<P3; insert "a"; final offsets = P1+1, P2+2, P3+3
- **Goal column** — move down-down-left-down sequence; assert goal resets on horizontal
- **Selection merge** — 2 cursors whose words overlap after `select_word` → single merged selection
- **Inlay offset mapping** — insert inlay at display-offset 10; buffer-offset 10 still resolves; inserting buffer text before inlay shifts both correctly
- **Incremental tree-sitter** — edit single char; assert only affected syntax-layer re-parses (via instrumentation counter)

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
