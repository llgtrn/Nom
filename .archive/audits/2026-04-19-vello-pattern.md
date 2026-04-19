# Vello Pattern Audit Report

**Date:** 2026-04-19  
**Source:** `C:\Users\trngh\Documents\APP\Accelworld\upstreams\vello` (main branch, ~v0.8.0)  
**Auditor:** Pattern-extraction analyst (subagent)  

---

## 1. Pattern Summary

Vello is a **GPU-compute-centric 2D vector graphics renderer** built on `wgpu`. Its core innovation is moving almost the entire render pipeline—path flattening, tiling, clipping, binning, and rasterization—into **compute shaders**, using prefix-scan algorithms to parallelize work that traditionally requires sequential CPU processing.

**High-level flow:**

```
Scene API (fill, stroke, draw_image, draw_glyphs, push_layer)
    ↓
Encoding  – packed byte streams (path_tags, path_data, draw_tags, draw_data, transforms, styles)
    ↓
Resolver  – late-bind resources (gradients → ramp cache, glyphs → outline encodings, images → atlas)
    ↓
RenderConfig – compute workgroup counts & buffer sizes from Layout
    ↓
Recording  – command graph of ~20 compute dispatch stages (coarse phase)
    ↓
WgpuEngine – upload buffers, execute dispatches, bind resource pooling
    ↓
Fine rasterization – per-tile AA (Area, MSAA8, MSAA16) → output texture
```

Key architectural traits:
- **Coarse → Fine split**: Coarse phase builds per-tile command lists (`ptcl_buf`) and segment buffers; fine phase executes them per 16×16 tile.
- **Bump allocation on GPU**: Dynamic buffers (`lines`, `tiles`, `segments`, `ptcl`) are sized heuristically; a `BumpAllocators` struct tracks actual usage, enabling robust memory retry.
- **CPU fallback**: Most compute stages have CPU implementations in `vello_shaders::cpu`, allowing hybrid execution for debugging or WebGPU limitations.
- **No retained GPU scene**: Every frame re-encodes and re-dispatches the full pipeline (scene is CPU-side, upload is cheap).

---

## 2. Key Source Files

### 2.1 Scene Composition API (`vello/src/`)

| File | Role | Key Types |
|------|------|-----------|
| `scene.rs` | User-facing draw API | `Scene`, `DrawGlyphs` |
| `lib.rs` | Renderer entry point | `Renderer`, `RenderParams`, `AaConfig`, `AaSupport` |
| `render.rs` | Build coarse/fine command graph | `Render`, `FineResources`, `render_encoding_full()` |
| `wgpu_engine.rs` | Execute recordings on wgpu | `WgpuEngine`, `ExternalResource`, `BindMap`, `ResourcePool` |
| `recording.rs` | Command buffer abstraction | `Recording`, `Command`, `BufferProxy`, `ImageProxy`, `ResourceProxy`, `BindType` |
| `shaders.rs` | Shader registration | `FullShaders`, `full_shaders()` |

### 2.2 GPU Encoding / Path Tiling (`vello_encoding/src/`)

| File | Role | Key Types |
|------|------|-----------|
| `encoding.rs` | Serialize scene to byte streams | `Encoding`, `Resources`, `StreamOffsets` |
| `path.rs` | Path tag/data encoding, stroke capping | `PathEncoder`, `PathTag`, `PathMonoid`, `LineSoup`, `PathSegment`, `Style` |
| `draw.rs` | Draw object tags & data | `DrawTag`, `DrawMonoid`, `DrawColor`, `DrawLinearGradient`, `DrawRadialGradient`, `DrawSweepGradient`, `DrawImage`, `DrawBeginClip` |
| `resolve.rs` | Late-binding & packing | `Resolver`, `Layout`, `Patch`, `resolve_solid_paths_only()` |
| `config.rs` | GPU config & buffer sizing | `RenderConfig`, `ConfigUniform`, `WorkgroupCounts`, `BufferSizes`, `BumpAllocators` |
| `glyph.rs` | Glyph run metadata | `Glyph`, `GlyphRun` |
| `glyph_cache.rs` | Outline glyph caching | `GlyphCache` |
| `image_cache.rs` | Image atlas allocation | `ImageCache`, `Images` |

### 2.3 Shader Integration (`vello_shaders/src/`)

| File | Role | Key Types |
|------|------|-----------|
| `lib.rs` | Shader metadata & generated table | `ComputeShader`, `WgslSource`, `MslSource`, `SHADERS` |
| `types.rs` | Binding type descriptors | `BindType`, `BindingInfo`, `WorkgroupBufferInfo` |
| `compile/` | Runtime WGSL→MSL/preprocess (hot_reload) | `compile::ShaderInfo` |
| `cpu/` | CPU fallback implementations | `cpu::flatten`, `cpu::binning`, `cpu::coarse`, `cpu::fine`, etc. |

### 2.4 WGSL Shader Stages (`vello_shaders/shader/`)

The pipeline is ~20 compute shaders executed in order:

1. `pathtag_reduce.wgsl` / `pathtag_reduce2.wgsl` – parallel reduction over path tag stream.
2. `pathtag_scan.wgsl` / `pathtag_scan1.wgsl` / `pathtag_scan_large.wgsl` – prefix scan producing `TagMonoid` (transform/style/path indices).
3. `bbox_clear.wgsl` – zero path bounding boxes.
4. `flatten.wgsl` – flatten beziers to `LineSoup`, compute path bboxes (Euler spiral approximation).
5. `draw_reduce.wgsl` / `draw_leaf.wgsl` – reduce draw tags to `DrawMonoid`, produce draw objects & clip inputs.
6. `clip_reduce.wgsl` / `clip_leaf.wgsl` – hierarchical clip bounding box reduction.
7. `binning.wgsl` – assign draw objects to 16×16 bins.
8. `tile_alloc.wgsl` – allocate per-path tile slices.
9. `path_count_setup.wgsl` / `path_count.wgsl` – count segments per tile (indirect dispatch).
10. `backdrop.wgsl` – compute winding backdrops per tile.
11. `coarse.wgsl` – generate per-tile command lists (`ptcl`).
12. `path_tiling_setup.wgsl` / `path_tiling.wgsl` – write `PathSegment`s into tile slices.
13. `fine.wgsl` – final per-tile rasterization (Area or MSAA), composite to output image.

Shared modules: `shared/config.wgsl`, `shared/bbox.wgsl`, `shared/blend.wgsl`, `shared/bump.wgsl`, `shared/clip.wgsl`, `shared/cubic.wgsl`, `shared/drawtag.wgsl`, `shared/pathtag.wgsl`.

---

## 3. Nom Mapping

**Target in Nom:** `nom-canvas-core/src/` (GPU vector rendering integration) and `nom_gpui::scene::Scene` (current immediate-mode quad renderer).

### 3.1 Current Nom Canvas State

- `nom-canvas-core/src/elements.rs` defines CPU-side primitives: `CanvasRect`, `CanvasEllipse`, `CanvasLine`, `CanvasArrow`, `GraphNodeElement`, `WireElement`. Colors are `[f32; 4]` RGBA, coordinates are f32 canvas-space.
- `nom-canvas-core/src/render_pipeline.rs` defines a **stub frame-graph**: `RenderPhase` (Geometry, Lighting, PostProcess, Overlay, Present), `DrawCommand` (Clear, DrawRect, DrawText, Composite), `FrameGraph`, `RenderPipelineCoordinator`. Currently `execute_stub()` only counts commands—no GPU submission.
- `nom-canvas-core/src/webgpu.rs` has `WebGpuRenderer` (stub) and `FrostedRenderPass` for blur/glass effects.
- `nom-canvas-core/src/bezier.rs` has `BezierCurve`, `BezierPoint`, `AnimatedBezier` with de Casteljau evaluation—**CPU-only**, no GPU flattening.
- `nom_gpui/src/scene.rs` (observed via integration test) maintains `quads` vector for immediate-mode rectangles; wires are decomposed into multiple quads.

### 3.2 What Vello Provides That Nom Lacks

| Nom Gap | Vello Solution | Relevant Vello Struct/Trait |
|---------|---------------|----------------------------|
| No GPU path rasterization | Full compute pipeline for fill/stroke of arbitrary beziers | `Scene::fill()`, `Scene::stroke()`, `PathEncoder` |
| No text glyph caching | `skrifa`-based outline extraction + `GlyphCache` encoding outlines as mini `Encoding`s | `DrawGlyphs`, `GlyphCache`, `Resolver` |
| No image atlas | `ImageCache` with atlas allocator (`guillotiere` backing) | `ImageCache`, `Images`, `Resolver::resolve_pending_images()` |
| No gradient ramps | `RampCache` builds 1-D RGBA gradient texture | `RampCache`, `Ramps` |
| No layering / clipping | `push_layer` / `pop_layer` with blend modes and clip shapes | `Scene::push_layer()`, `Scene::pop_layer()`, `DrawBeginClip` |
| No AA strategy choice | Area AA, MSAA8, MSAA16 selectable at renderer init | `AaConfig`, `AaSupport` |
| Stub frame graph only | `Recording` + `WgpuEngine` implements real dispatch graph | `Recording`, `Command`, `WgpuEngine::run_recording()` |

### 3.3 Suggested Integration Points

1. **Replace `nom_gpui::Scene` quads with `vello::Scene` encoding**
   - Map `CanvasRect` → `scene.fill(Fill::NonZero, transform, brush, None, &rect)`.
   - Map `CanvasEllipse` → `Ellipse::new(...)` or `Circle::new(...)` via `peniko::kurbo` shapes.
   - Map `CanvasLine` / `CanvasArrow` → `scene.stroke(&stroke, transform, brush, None, &path)`.
   - Map `GraphNodeElement` labels → `scene.draw_glyphs(font).font_size(...).draw(...)`.

2. **Bridge `RenderPipelineCoordinator` to `vello::Renderer`**
   - Instead of `execute_stub()`, build a `vello::RenderParams` per frame and call `renderer.render_to_texture(device, queue, &scene, &texture_view, &params)`.
   - Reuse `RenderPhase` enum to order Vello renders vs. Nom custom post-processing (e.g., frosted glass).

3. **Extend `nom-canvas-core/src/bezier.rs`**
   - Keep `BezierCurve` for animation/CPU evaluation, but when rendering, pass `kurbo::BezPath` (or `peniko::kurbo` shapes) directly to Vello’s `PathEncoder` so flattening happens on GPU.

4. **Leverage `vello_encoding` independently**
   - If Nom wants to retain its own renderer, `vello_encoding::Encoding` + `Resolver` can still be used to generate packed scene buffers; the `Recording` dispatch graph in `vello::render::Render` can be adapted to a different compute backend.

---

## 4. Licensing / Complexity Notes

### 4.1 Licensing
- **Vello**: Dual-licensed **Apache-2.0 OR MIT** (`LICENSE-APACHE`, `LICENSE-MIT`).
- **WGSL shaders**: The shader directory includes an `UNLICENSE` dedication for the WGSL source files, effectively public domain.
- **Key dependencies** (all permissive):
  - `wgpu` v28.0.0 — MIT/Apache-2.0
  - `peniko` v0.6.0 — MIT/Apache-2.0 (styling, brushes, kurbo geometry)
  - `skrifa` v0.40.0 — MIT/Apache-2.0 (font parsing, glyph outlines)
  - `bytemuck` v1.25.0 — MIT/Apache-2.0 (zero-copy casting for GPU structs)
  - `naga` v28.0.0 — MIT/Apache-2.0 (shader translation)
  - `guillotiere` v0.7.0 — zlib/libpng license (atlas allocator)

No copyleft concerns for Nom’s intended use.

### 4.2 Complexity Assessment
- **Rust MSRV**: 1.92 (workspace `rust-version`). Nom should verify toolchain alignment.
- **Shader stage count**: ~20 compute dispatches + 2–3 fine-raster variants. The pipeline is **not trivial**—understanding the data flow between `tag_monoids`, `draw_monoids`, `path_bboxes`, `clip_bboxes`, `bin_headers`, `tiles`, `segments`, and `ptcl` requires studying the coarse-phase code in `vello/src/render.rs`.
- **Dynamic GPU memory**: Buffer sizes are currently **heuristic** (hard-coded powers of two in `BufferSizes::new`). The `bump_estimate` feature exists but is incomplete. For production use, Nom would need to either accept over-allocation or implement robust retry logic (read back `BumpAllocators`, resize, re-run coarse).
- **CPU fallback**: `vello_shaders::cpu` provides reference implementations for most stages, which aids debugging but doubles the surface area.
- **Text complexity**: Vello’s text path supports COLR, bitmap emoji (PNG, BGRA, mask), and variable fonts with normalized coords. The COLR path (`try_draw_colr`) recursively paints into the scene via `ColorPainter`—this is sophisticated and may be overkill if Nom only needs basic text labels.
- **Image atlas limitations**: `ImageCache` has a max atlas size; large images fail to allocate and are silently dropped (zero dimensions). Nom would need to handle this or use texture arrays.

---

## 5. Adoption Effort Estimate

### Option A: Full Vello Integration (Recommended for GPU vector fidelity)
**Effort:** ~3–4 weeks (1 senior Rust/GPU engineer)  
**Tasks:**
1. Add `vello`, `vello_encoding`, `peniko` dependencies to `nom-canvas` workspace.
2. Replace `nom_gpui::Scene` quad accumulation with `vello::Scene` encoding:
   - Implement `Into<peniko::kurbo::Shape>` for `CanvasRect`, `CanvasEllipse`, `CanvasLine`, `CanvasArrow`.
   - Convert Nom colors (`[f32; 4]`) to `peniko::Color`.
3. Wire `RenderPipelineCoordinator` → `vello::Renderer`:
   - Initialize `wgpu::Device/Queue` once (Nom already has WebGPU stubs).
   - Per frame: encode scene → `renderer.render_to_texture(...)` → blit to swapchain.
4. Text: integrate `skrifa` font loading; map Nom label strings to `DrawGlyphs` runs.
5. Images: migrate Nom image brushes through Vello’s `ImageCache` / atlas.
6. Layers: map Nom’s frosted-glass / overlay passes to Vello `push_layer`/`pop_layer` with blend modes, or keep them as separate render passes after Vello.
7. Testing: validate against existing golden-path tests; MSAA vs Area AA quality checks.

**Risks:**
- `BumpAllocators` heuristics may need tuning for Nom’s typical scene complexity.
- WebGPU/WASM path needs verification; Vello supports WASM but compute shader availability varies.
- `Renderer::new` shader compilation is expensive (~hundreds of ms); must be cached/reused.

### Option B: Incremental Adoption (Use `vello_encoding` only)
**Effort:** ~1–2 weeks  
**Approach:** Use `vello_encoding::Encoding` + `Resolver` to pack scene data, but feed the resulting `Recording` into a custom Nom compute backend instead of `WgpuEngine`. This makes sense only if Nom plans to build its own GPU pipeline and just wants Vello’s scene encoding logic.

### Option C: Minimal Dependency (Shader inspiration only)
**Effort:** ~4–6 weeks  
**Approach:** Port the WGSL flatten/tiling/fine shaders and CPU reference code into Nom’s existing `nom_gpui` renderer. High effort because the prefix-scan data structures (`PathMonoid`, `DrawMonoid`, bump allocators) are tightly coupled to the shader code.

### Recommendation
**Option A** is the pragmatic path: Vello is designed as an embeddable library, and Nom’s stub `RenderPipelineCoordinator` + `WebGpuRenderer` are natural integration points. The primary work is adapter code (shape/color conversions and scene lifetime management), not algorithm reimplementation.

---

*End of report.*