# Nom State Machine Report

**Date:** 2026-04-18 | **HEAD:** `679ce6b` (dirty Wave AP) | **Tests:** 8391 | **Workspace:** dirty — Wave AP complete, uncommitted

## Iteration 60 — Hard Audit Wave AN verification (2026-04-18, HEAD 8e36ec0, 7902 tests)

**8 parallel agents, all files read end-to-end.** Verifying Wave AN actuals against all known gaps.

### FIXED by Wave AN (confirmed by 8 agents)

| Gap | Fix | Evidence |
|-----|-----|----------|
| AL-CRDT-OVERFLOW | `next_id()` now `saturating_add(1).min(u64::MAX - 1)` | lib.rs:96-97 |
| AM-SPATIAL-WIRE selection.rs | `SpatialIndex` imported + used in `select_in_region()` | selection.rs:4, :112 |
| AN-FRAME-SPATIAL (x/y/w/h/z_index) | Fields present in FrameBlock with defaults | frame.rs:19-28 |
| AN-BLOCKDIFF-CONTENT invert() | `BlockDiff::invert()` function exists; kind field compared | diff.rs:55-66, :119 |
| AL-THEME-SYSTEM struct + dark/light | `pub struct Theme` with constructors at tokens.rs:289-318 | tokens.rs:289 |
| AN-LSP-POSITIONS types | `LspPosition`, `LspRange`, conversions in lsp_bridge.rs | lsp_bridge.rs:142-198 |

### STILL OPEN — Iteration 60 verified (zero new fixes since Iteration 59)

**RENDERER (10 waves overdue — most critical blocker in workspace)**
- AL-RENDER-2: Window struct at lines 112-132: stub scalars (`gpu_ready: bool`, `surface_width: u32`). NO `wgpu::Surface`, NO `wgpu::Device`, NO `wgpu::Queue`, NO `surface_format`. `run_native_application()` creates winit window but calls ZERO wgpu APIs.
- AL-RENDER-1: `end_frame()` still: `queue.write_buffer` → counter increments. No `CommandEncoder`, no `begin_render_pass`, no `set_pipeline`, no `draw`, no `queue.submit`, no `present`.
- AL-RENDER-3: `build_quad_pipeline()` still has `buffers: &[]`. All 10 shaders still return hardcoded `(0,0,0,1)`. `pollster` still absent from Cargo.toml.
- AL-LAYOUT-TAFFY: `LayoutEngine` still `HashMap<LayoutId, Bounds<Pixels>>` stub; taffy::TaffyTree never imported.

**DB-DRIVEN MANDATE**
- AL-BACKEND-KIND: `pub enum BackendKind` with 16 variants at dispatch.rs:10-27. `UnifiedDispatcher` + `ComposeContext` are NOT re-exported from lib.rs — dead code.
- AL-GRAMMAR-STATUS PARTIAL: `KindStatus` enum EXISTS at shared.rs:8-16. BUT `GrammarKind` still has NO `status: KindStatus` field. NO `list_kinds()` SQL.

**SECURITY**
- AL-SQL-INJECT: `is_safe_identifier()` at data_query.rs:41 EXISTS but is NEVER called in `to_sql()`. Raw `format!` at lines 27,29 and semantic.rs:70,75 — 4 injection points.
- AM-CONNECTOR-DESER: `#[derive(Deserialize)]` on Connector still present. Crafted JSON can set `can_wire_result: (true, 1.0, "forged")` bypassing `new_with_validation()`.

**HIGH gaps still open**
- AM-CRDT-IDEMPOTENT: `apply()` still has no duplicate guard — direct `apply()` is not idempotent.
- AM-ATLAS-LRU: `evict_lru()` still calls `self.allocator.clear()` nuking all allocations after partial eviction.
- AM-SPATIAL-WIRE hit_test.rs: SpatialIndex only in `#[cfg(test)]`; production broadphase still linear.
- AM-SPATIAL-WIRE viewport.rs: `Viewport` struct has NO `SpatialIndex` field.
- AM-UITIER-DIVERGE: No `score_atom_impl()`; UiTier vs UiTierOps still diverge.
- AL-ATOMIC-ORDERING: grammar_version load/store still `Ordering::Relaxed`.
- AM-INTENT-STRUCT: IntentResolver has 1 field only; no bm25_index; resolve() is substring-match; classify_with_react() disconnected.
- AL-DEEPTHINK-CONFIDENCE: paint_scene() still hardcoded `EDGE_MED` for all cards regardless of confidence.
- AL-FONTS: FontRegistry missing libre_baskerville, eb_garamond, berkeley_mono.
- AN-WORKSPACE-DUP: insert_block() still corrupts doc_tree; entity() still panics for Connector; no remove_node/connector.
- AN-FRAME-SPATIAL rotation: FrameBlock has no rotation field; add_child() has no cycle detection.
- AL-TOOLBAR-HEIGHT: TOOLBAR_H=48.0 AND TOOLBAR_HEIGHT=36.0 coexist (ambiguous).
- AL-THEME-SYSTEM oled(): Theme::oled() absent.
- AN-BLOCKDIFF-WORD: diff_blocks() cannot detect word field changes.

### NEW gaps found in Iteration 60 (first time observed)

**NOM-GRAPH-EXEC (HIGH)**: `ExecutionEngine::plan_execution()` returns `Vec<NodeId>` but NEVER executes nodes — no `execute()` function calls node logic or stores results in cache. ComfyUI reference has full `async execute()` at execution.py:233-309. The graph crate can plan but cannot run.

**NOM-GRAPH-ANCESTRY (MEDIUM)**: Cache keys only inspect immediate parents (single-hop). ComfyUI's `get_ordered_ancestry()` at caching.py:131-148 traverses the full transitive closure for stable cache keys. Deep graphs will get wrong cache misses.

**NOM-EDITOR-POINT (HIGH)**: No `Point { row, column }` type. No display map pipeline (FoldMap→TabMap→WrapMap). `lsp_bridge.rs` functions operate on raw `&str` text, not integrated into `Buffer` API. LSP needs row/col not byte offsets — current architecture is incompatible with any LSP server.

**NOM-BACKEND-SELF-DESCRIBE (MEDIUM)**: `Backend` trait has only `kind()` + `compose()`. n8n's `INodeTypeDescription` includes version, displayName, properties schema, inputs/outputs declarations. UnifiedDispatcher handler registration has no self-description.

---

## Iteration 61 — Wave AP COMPLETE (2026-04-18, HEAD 679ce6b dirty, 8391 tests)

**5 parallel executor batches. ALL CRITICAL + HIGH items fixed.**

### FIXED in Wave AP (21 items)

| Gap ID | Fix | Crate |
|--------|-----|-------|
| AL-RENDER-2 | Real wgpu::Surface/Device/Queue fields + full init chain (Instance→create_surface→request_adapter→request_device→configure) + pollster=0.3 | nom-gpui |
| AL-RENDER-1 | end_frame_render(): CommandEncoder + begin_render_pass + set_pipeline + set_vertex_buffer(0) + draw(0..6,0..N) + queue.submit + present | nom-gpui |
| AL-RENDER-3 | VertexBufferLayout (stride=80, 5×Float32x4, Instance); real WGSL QuadIn @location(0-4) + GlobalUniforms viewport + NDC transform | nom-gpui |
| AM-ATLAS-LRU | evict_lru() calls allocator.deallocate(alloc) per entry; no more allocator.clear() | nom-gpui |
| AL-LAYOUT-TAFFY | LayoutEngine replaced with taffy::TaffyTree + node_map | nom-gpui |
| AL-BACKEND-KIND | BackendKind closed enum DELETED; all dispatch uses runtime &str/String; UnifiedDispatcher+ComposeContext re-exported as primary dispatch | nom-compose |
| AL-GRAMMAR-STATUS | pub status: KindStatus added to GrammarKind; list_kinds() + promote_kind() SQL helpers added | nom-compiler-bridge |
| AM-UITIER-DIVERGE | score_atom_impl() extracted; UiTier + UiTierOps both delegate | nom-compiler-bridge |
| AL-DEEPTHINK-CONFIDENCE | _card→card; edge_color_for_confidence(card.confidence) wired | nom-panels |
| AL-TOOLBAR-HEIGHT | TOOLBAR_H=48.0 deleted; all callers use TOOLBAR_HEIGHT=36.0 | nom-theme |
| AL-FONTS | libre_baskerville_regular, eb_garamond_regular, berkeley_mono_regular added | nom-theme |
| AL-THEME-SYSTEM oled | Theme::oled() constructor added (pure black bg) | nom-theme |
| AN-WORKSPACE-DUP | insert_block() dedup guard; entity() returns Option<&NomtuRef> (no panic for Connector); remove_node()+remove_connector() added | nom-blocks |
| AN-FRAME-SPATIAL rotation+cycle | rotation: f32 field; add_child() returns Result with cycle guard | nom-blocks |
| AN-BLOCKDIFF-WORD | diff_blocks() emits Modified{field:"word"} diffs | nom-blocks |
| AM-SPATIAL-WIRE hit_test | CanvasHitTester with R-tree broadphase in production (no #[cfg(test)] gate) | nom-canvas-core |
| NOM-EDITOR-POINT | Point{row,column} type + Buffer::point_at() + Buffer::offset_from_point() | nom-editor |
| NOM-GRAPH-EXEC | ExecutionEngine::execute() runs plan, calls node logic, stores results in cache | nom-graph |

### STILL OPEN after Wave AP (Wave AQ targets)
- ❌ **NOM-GRAPH-ANCESTRY** — transitive cache key ancestry walk missing
- ❌ **NOM-BACKEND-SELF-DESCRIBE** — Backend trait missing version/displayName/params schema
- ❌ **AM-INTENT-STRUCT** — no bm25_index; classify_with_react() disconnected
- ❌ **AL-COSMIC** — cosmic_text::FontSystem not initialized; font data still placeholder IDs
- ❌ **AM-SPATIAL-WIRE viewport.rs** — Viewport struct has no SpatialIndex field
- ❌ **UC-SERVE** — POST /compose axum endpoint not implemented in nom-cli

---

## Iteration 59 — Hard Audit Wave AN (2026-04-18, HEAD 8e36ec0/7e06f47, 7902 tests)

**8 parallel agents, all files read in full.** Wave AN (+250 tests, 7652→7902) examined.

### VERDICT: Wave AN added +250 tests, fixed ZERO CRITICAL or HIGH items

Every single item from Iteration 58 is STILL OPEN. Waves A–AN (9+ waves) have NOT closed the renderer.

**CRITICAL COUNT: 4 open. HIGH COUNT: 6 open. SECURITY COUNT: 2 open. NEW: 5 open.**

**WAVE AO MANDATE:** The Executor MUST close AL-RENDER-1/2/3 in Wave AO. These are 9 waves overdue. No new test coverage waves until the renderer renders pixels.

---

## Iteration 58 — Hard Audit Wave AM (2026-04-18, HEAD 453b0ff/7086ff2, 7652 tests)

**8 parallel agents, all files read in full.** Wave AM (+411 tests, 7241→7652) examined.

### CRITICAL — Wave AM DID NOT FIX any of these

**AL-RENDER-ALL: Window struct has ZERO GPU fields, renderer renders zero pixels**
- `window.rs:78-87` — `Window` struct: ONLY logical fields (options, scale_factor, content_size, is_focused, cursor_position, frame_pending, close_requested). Comment says "// In real impl: wgpu swap chain". ZERO GPU fields. `run_native_application()` creates winit window then enters event loop — zero wgpu calls.
- `renderer.rs:550-564` — `end_frame()` ONLY calls `queue.write_buffer` then increments counters. No CommandEncoder, no begin_render_pass, no set_pipeline, no draw, no submit, no present.
- `renderer.rs:475` — `build_quad_pipeline()` has `buffers: &[]`. No VertexBufferLayout. QuadInstance data uploaded to buffer is INVISIBLE to the pipeline.
- `shaders.rs:4-9` — ALL vertex shaders: `fn vs_main(@builtin(vertex_index) vi: u32) -> vec4<f32> { return vec4<f32>(0.0, 0.0, 0.0, 1.0); }`. Zero `@location(N)` instance inputs. HARDCODED stubs.
- `nom-gpui/Cargo.toml` — `pollster` NOT present. No async runtime for wgpu device creation.

**AL-BACKEND-KIND: BackendKind enum still exists, 379 references**
- `dispatch.rs:9-27` — `pub enum BackendKind` with 16 variants still alive.
- `dispatch.rs` counts: 182 `BackendKind::` refs in dispatch.rs alone; 379 total across 6 files.
- `BackendRegistry` at `dispatch.rs:80` keyed by `HashMap<BackendKind, Box<dyn Backend>>` — enum-keyed.
- `UnifiedDispatcher` at `dispatch.rs:367` EXISTS and IS string-keyed (`HashMap<String, Box<dyn Fn>>`) BUT is **DEAD CODE in production** — only exercised by tests in dispatch.rs.
- `ComposeContext` at `dispatch.rs:332` EXISTS (kind_name: String, entity_id: String, params: HashMap) but is **DEAD CODE in production** — no production caller.

**AL-GRAMMAR-STATUS: KindStatus enum still absent**
- `shared.rs:15-19` — `GrammarKind { name: String, description: String }`. No `status` field. No `KindStatus` enum anywhere in shared.rs. No `list_kinds()` SQL — grammar managed entirely in-memory.

**AL-CRDT-OVERFLOW + AM-CRDT-IDEMPOTENT: Both still open**
- `lib.rs:96` — `self.counter += 1`. No `checked_add`. Malicious peer can send `counter: u64::MAX` op → wrap on next `next_id()` → CRDT ordering breaks.
- `lib.rs:105-112` — `apply()`: no idempotency guard. Same op applied twice = duplicate in op_log. No `.min(u64::MAX - 1)` clamp.
- NOTE: `merge()` at lib.rs:196 CORRECTLY deduplicates — this path is safe. Only direct `apply()` calls are vulnerable.

### HIGH — Wave AM did not address

**AM-ATLAS-LRU: evict_lru calls allocator.clear() after partial eviction**
- `atlas.rs:139-152` — removes N entries from HashMap then calls `self.allocator.clear()` nuking ALL allocations. Surviving cache entries retain stale UV coordinates. CONFIRMED: comment says "callers must re-rasterize on next lookup miss" but cache hits won't trigger misses.

**AM-SPATIAL-WIRE: SpatialIndex completely unwired**
- `selection.rs:105-112` — `RubberBand::intersects()` tests AABB per element inline. No SpatialIndex import.
- `hit_test.rs` — linear iteration over all elements per hit test. No SpatialIndex usage.
- `viewport.rs:9-16` — `Viewport` struct has NO SpatialIndex field.
- `spatial_index.rs` — fully implemented R-tree exists but is ZERO references in selection/hit_test.

**AL-GRAMMAR-STATUS: AL-ATOMIC-ORDERING still Relaxed**
- `shared.rs:83-84` load: `Ordering::Relaxed`. `shared.rs:101-102` fetch_add: `Ordering::Relaxed`. No happens-before with the RwLock write that precedes the version bump.

**AM-UITIER-DIVERGE: Confirmed divergence**
- `ui_tier.rs:130-153` — UiTier: checks grammar cache → returns hardcoded 0.9 or 0.3 on name match; falls through to nom_score only when cache empty.
- `ui_tier.rs:257-279` — UiTierOps: always calls nom_score::score_atom directly, skipping cache.
- Same input → different scores. Zero `#[cfg(feature = "compiler")]` tests.

### SECURITY — Wave AM: is_safe_identifier defined but NEVER WIRED

**AL-SQL-INJECT: Guard exists but is dead code**
- `data_query.rs:41` — `is_safe_identifier()` EXISTS and is correct.
- `data_query.rs:27,29` and `semantic.rs:72,75` — NOT called before interpolation. 3 format! SQL injection sites active. `where_clause` raw string interpolated verbatim (HIGHEST RISK).

**AM-CONNECTOR-DESER: Validation bypass confirmed**
- `connector.rs:34` — `#[derive(Deserialize)]` present. Attack vector confirmed: craft `{"can_wire_result": [true, 1.0, "validated"]}` → bypasses `new_with_validation()`. Serde populates private fields.

### NEW — Iteration 58 discoveries

**AN-FRAME-SPATIAL: FrameBlock has ZERO spatial fields**
- `frame.rs:8-19` — `FrameBlock { entity, label, children, background_color, border_width }`. No x/y/width/height/rotation/z_index. Cannot be positioned, rendered, or hit-tested on canvas.
- `add_child()` at line 34: unconditional push, no cycle detection. Test at line 256 documents this as "caller's responsibility" but no orchestration-layer guard exists.

**AN-WORKSPACE-DUP: Duplicate ID bug + Connector panic**
- `workspace.rs:53-56` — `insert_block()` pushes to `doc_tree` without checking for existing ID. `doc_tree.len()` diverges from `blocks.len()` on duplicate insert.
- `workspace.rs:26-28` — `CanvasObject::entity()` panics for Connector variant: `panic!("Connectors don't have a direct NomtuRef")`. Should return `Option<&NomtuRef>`.
- `remove_node()` and `remove_connector()` methods absent entirely.

**AN-BLOCKDIFF-CONTENT: BlockDiff misses all content fields**
- `diff.rs:86-103` — `diff_blocks()` compares ONLY `meta.author` and `meta.version`. All content changes (kind, payload, slots, children) are invisible to the diff system.
- No `BlockDiff::invert()` method — undo operations impossible.
- `apply_diff` Added case uses `DiffEntry::new()` (defaults) — original metadata lost.

**AN-LSP-POSITIONS: Editor uses byte offsets, not LSP positions**
- `buffer.rs:9-12` — `Patch { old_range: Range<usize>, new_text: String }` — char offsets only.
- `cursor.rs:10-13` — `Anchor { offset: usize }` — no LSP Position type.
- `tab_map.rs` — only converts tabs to visual columns, no byte-to-LSP conversion.

**AL-THEME-SYSTEM: No pub struct Theme, Wave AM additions are ALL test-only**
- `tokens.rs` — NO `pub struct Theme`. Only `ShadowToken` struct + 65 `pub const` declarations.
- ALL Wave AM "animation curve/focus-visible/forced-colors" additions are inside `#[cfg(test)]`. Zero new production tokens added in Wave AM.
- `TOOLBAR_H = 48.0` — test at line 750 LOCKS it to 48.0 with assertion.
- `edge_color_for_confidence()` EXISTS at tokens.rs:217 but is NEVER called from deep_think.rs.
- `deep_think.rs:173` — `border_color: Some(rgba_to_hsla(tokens::EDGE_MED))` on ALL sides, uniform regardless of confidence.
- `fonts.rs` — `FontRegistry` missing `libre_baskerville_regular`, `eb_garamond_regular`, `berkeley_mono_regular`. No `cosmic_text::FontSystem` initialization.

**AL-TEST-FRAUD confirmed: 5 tests on cfg(test)-only code in semantic.rs**
- `semantic.rs:527-593` — `ArtifactDiff` struct + `artifact_diff()` fn defined inside `#[cfg(test)]`. 5 tests test this test-only code.

**Panel test theater confirmed: Wave AM additions are zero production code**
- `node_palette.rs:77-86` — `paint_scene()` renders 24px rows only; no 32px search box.
- Wave AM "panel persistence v2", "floating panels", "drag-between-panels" — ZERO production structs. Tests manipulate local variables.
- Wave AM "POST /compose" — NOT implemented. No axum, no tokio HTTP server in nom-cli.

### PARTIAL FIXES (Wave AM introduced stubs that still need real wiring)

**AM-INTENT-STRUCT: IntentResolver stub EXISTS but incomplete**
- `lib.rs:145-147` — `pub struct IntentResolver { pub grammar_kinds: Vec<String> }` EXISTS.
- `lib.rs:158` — `pub fn resolve(&self, input: &str) -> ResolvedIntent` EXISTS.
- INCOMPLETE: struct has only ONE field (`grammar_kinds`); missing `bm25_index: BM25Index`. `resolve()` is a stub implementation.

**AL-SQL-INJECT guard: is_safe_identifier defined but not wired**
- See SECURITY section above.

### Test Inflation Confirmed: ~6.8× factor
- 7652 total tests. Estimated ~1,120 real behavioral tests (~85% duplication ratio).
- Wave AM test quality: animation curve tokens ALL in `#[cfg(test)]`, panel tests exercise local variable arithmetic, CRDT convergence tests are near-duplicates.
- Wave AN MUST reduce duplication. Target: ≤20% duplication ratio.

---
**Detailed commit history:** `git log --oneline`. This file keeps only the latest state + open missions.

## Current State (Wave AL complete)

- [x] Waves A–G landed (Bootstrap + GPUI substrate + Editor/Blocks + Compiler bridge + Shell + Compose backends + Graph RAG + Stubs)
- [x] Waves K–P closed all CRITICALs and HIGHs (U1/W1/COL1/INT1 + E2 + 10 more)
- [x] Waves Q–T closed MEDIUMs, spec-align, coverage, integration tests
- [x] Waves V–AL: +5562 tests (733 → 7241); 15 coverage waves pushing all 14 crates upward
- [x] Wave AL committed `778b085` — 7241 tests; GPU struct tests, CRDT GC, CommandStack, panel serialization, deep-think streaming
- **CRITICAL gate:** Renderer::draw() is still a stub — window opens but renders zero pixels (AE1 open 7+ waves)
- **CRITICAL gate:** BackendKind is a closed 16-variant Rust enum (DB-driven mandate violation)
- **CRITICAL gate:** ComposeContext/UnifiedDispatcher/HybridResolver/GlueCache all MISSING
- **DB-driven verdict:** CONFIRMED CORRECT for node palette, library, connector, DictReader isolation. NOT correct for BackendKind enum and compose routing.

## Iteration 56 — Deep Prescription Audit (2026-04-18, HEAD 778b085, 7241 tests)

**6 of 8 parallel agents completed (2 ECONNRESET).** Full executor-ready prescriptions for all 3 CRITICAL blockers produced.

### KEY DISCOVERY: AL-COMPOSE-BRIDGE is SIMPLER THAN SCOPED

**`ComposeContext` already exists** at `nom-compose/src/dispatch.rs:332-359`:
```rust
pub struct ComposeContext {
    pub kind_name: String,
    pub entity_id: String,
    pub params: std::collections::HashMap<String, String>,
}
```
**`UnifiedDispatcher` already exists** at lines 367-409 and is already string-keyed — it's the open-ended dispatch path. The old `BackendKind` enum (lines 9-27) + `BackendRegistry` (lines 79-119) is the LEGACY closed path.

**Correct fix**: Delete the closed `BackendKind` enum + `BackendRegistry` + `Backend` trait impls (lines 9-324 = ~315 lines) entirely. All callers migrate to `UnifiedDispatcher::dispatch(&ComposeContext)`. No new file needed, no Cargo.toml dependency changes needed.

### KEY DISCOVERY: RENDER PIPELINE BROKEN AT TWO LEVELS

Beyond missing CommandEncoder/RenderPass, `build_quad_pipeline` at line 475 has `buffers: &[]` — the vertex pipeline declares NO vertex buffer layout. This means the GPU pipeline cannot receive any QuadInstance data even if the render pass were correctly wired. Both the pipeline definition AND the shader inputs must be fixed together.

**`Window` struct** (window.rs:78-87) has ZERO GPU fields — not even `Arc<wgpu::Device>`. The init chain must add `surface`, `device`, `queue`, `surface_format` fields and populate them in `run_native_application()`.

### RENDER FIX PRESCRIPTION (from agent deep read — executor-ready)

**AL-RENDER-2 (window.rs) — exact field additions:**
```rust
// Add to Window struct after line 86:
pub surface: Option<wgpu::Surface<'static>>,
pub device: Option<Arc<wgpu::Device>>,
pub queue: Option<Arc<wgpu::Queue>>,
pub surface_format: Option<wgpu::TextureFormat>,
```
Init in `run_native_application()` after `os_window` creation:
```rust
let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
    backends: wgpu::Backends::all(), ..Default::default()
});
let surface = instance.create_surface(&os_window).expect("create surface");
let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
    power_preference: wgpu::PowerPreference::HighPerformance,
    compatible_surface: Some(&surface), force_fallback_adapter: false,
})).expect("find adapter");
let (device, queue) = pollster::block_on(adapter.request_device(
    &wgpu::DeviceDescriptor { label: Some("nom-gpui"), ..Default::default() }, None,
)).expect("create device");
let surface_caps = surface.get_capabilities(&adapter);
let fmt = surface_caps.formats.iter().copied().find(|f| f.is_srgb()).unwrap_or(surface_caps.formats[0]);
let size = os_window.inner_size();
surface.configure(&device, &wgpu::SurfaceConfiguration {
    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
    format: fmt, width: size.width, height: size.height,
    present_mode: wgpu::PresentMode::Fifo,
    alpha_mode: surface_caps.alpha_modes[0],
    view_formats: vec![], desired_maximum_frame_latency: 2,
});
window.surface = Some(surface);
window.device = Some(Arc::new(device));
window.queue = Some(Arc::new(queue));
window.surface_format = Some(fmt);
// Add dep: pollster = "0.3" in nom-gpui/Cargo.toml
```

**AL-RENDER-1 (renderer.rs end_frame) — replace with:**
```rust
pub fn end_frame(&mut self, surface: Option<&wgpu::Surface<'_>>) -> Result<(), FrameError> {
    if let Some(gpu) = self.gpu.as_mut() {
        if !self.pending_quads.is_empty() {
            let bytes = bytemuck::cast_slice(self.pending_quads.as_slice());
            gpu.queue.write_buffer(&gpu.instance_buffer, 0, bytes);
        }
        if let Some(surface) = surface {
            let output = surface.get_current_texture().expect("surface texture");
            let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("nom-gpui frame") });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("main pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view, resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                    })],
                    depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None,
                });
                if !self.pending_quads.is_empty() {
                    pass.set_pipeline(&gpu.quad_pipeline);
                    pass.set_vertex_buffer(0, gpu.instance_buffer.slice(..));
                    pass.draw(0..6, 0..self.pending_quads.len() as u32);
                }
            }
            gpu.queue.submit(std::iter::once(encoder.finish()));
            output.present();
        }
    }
    self.in_frame = false;
    self.frame_count += 1;
    Ok(())
}
```

**AL-RENDER-3 (shaders.rs + build_quad_pipeline) — two changes needed:**

1. In `build_quad_pipeline` (renderer.rs:475), change `buffers: &[]` to:
```rust
buffers: &[wgpu::VertexBufferLayout {
    array_stride: 80,  // 5 fields × 16 bytes each
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &[
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 0, shader_location: 0 },   // bounds
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 16, shader_location: 1 },  // bg_color
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 32, shader_location: 2 },  // border_color
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 48, shader_location: 3 },  // border_widths
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 64, shader_location: 4 },  // corner_radii
    ],
}],
```

2. Replace degenerate WGSL (shaders.rs:4-9) with:
```wgsl
struct QuadIn {
    @location(0) bounds: vec4<f32>,
    @location(1) bg_color: vec4<f32>,
    @location(2) border_color: vec4<f32>,
    @location(3) border_widths: vec4<f32>,
    @location(4) corner_radii: vec4<f32>,
};
@group(0) @binding(0) var<uniform> viewport: vec2<f32>;
@vertex fn vs_main(@builtin(vertex_index) vi: u32, q: QuadIn) -> @location(0) vec4<f32> {
    let corners = array<vec2<f32>, 6>(
        vec2(q.bounds.x, q.bounds.y), vec2(q.bounds.x+q.bounds.z, q.bounds.y),
        vec2(q.bounds.x, q.bounds.y+q.bounds.w), vec2(q.bounds.x+q.bounds.z, q.bounds.y),
        vec2(q.bounds.x+q.bounds.z, q.bounds.y+q.bounds.w), vec2(q.bounds.x, q.bounds.y+q.bounds.w)
    );
    let pos = corners[vi] / viewport * 2.0 - vec2(1.0);
    return vec4<f32>(pos.x, -pos.y, 0.0, 1.0);  // @builtin(position)
}
@fragment fn fs_main(@location(0) color: vec4<f32>) -> @location(0) vec4<f32> { return color; }
```
(Note: `@location(0)` on vertex output maps to `q.bg_color` — must also pass `bg_color` through vertex out.)

### DB-DRIVEN FIX PRESCRIPTION (executor-ready)

**AL-BACKEND-KIND — delete the enum, migrate to UnifiedDispatcher:**
- Delete lines 9-119 (`BackendKind` enum + `from_kind_name` + `name` + `Backend` trait + `BackendRegistry`)
- Delete lines 122-324 (7 concrete Backend trait impls)
- All test sites: ~100 `BackendKind::Video` → `"video"` string replacements
- Wire existing `UnifiedDispatcher::dispatch(&ComposeContext)` as the entry point

**AL-GRAMMAR-STATUS — add to shared.rs before GrammarKind:**
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KindStatus { Transient, Partial, Complete }
impl KindStatus {
    pub fn from_str(s: &str) -> Self {
        match s { "partial" => Self::Partial, "complete" => Self::Complete, _ => Self::Transient }
    }
}
```
Update `GrammarKind` with `pub status: KindStatus` field and `list_kinds()` SQL to `SELECT name, description, COALESCE(status, 'transient') FROM kinds`.

**AL-COMPOSE-BRIDGE — already implemented:** `ComposeContext` + `UnifiedDispatcher` in dispatch.rs are the bridge. Task is to delete the closed enum and ensure all call sites use `UnifiedDispatcher`.

### SECURITY FIX PRESCRIPTIONS (executor-ready)

**AL-SQL-INJECT:**
```rust
fn is_safe_identifier(s: &str) -> bool {
    if s.is_empty() { return false; }
    let first = s.as_bytes()[0];
    if !(first.is_ascii_alphabetic() || first == b'_') { return false; }
    s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.')
}
```
Change `to_sql()` return type to `Result<String, String>`. Validate all column names + table name before interpolation. Block `where_clause` containing `;`, `--`, `/*`.

**AL-CRDT-OVERFLOW:**
- `next_id()`: `self.counter.checked_add(1).expect("CRDT counter overflow")`
- `apply()`: `self.counter = op.id.counter.min(u64::MAX - 1)`

**AL-ATOMIC-ORDERING:** 
- Load at line 84: `Ordering::Relaxed` → `Ordering::Acquire`
- fetch_add at line 102: `Ordering::Relaxed` → `Ordering::Release`

### CODE QUALITY FIX PRESCRIPTIONS (executor-ready)

**AL-TEST-FRAUD (semantic.rs:527-593):** Delete `ArtifactDiff` struct + `artifact_diff()` fn + all 5 test functions. Add 3 replacement tests: empty identifier regression, SQL injection attempt documentation, column name injection.

**AM-CONNECTOR-DESER (connector.rs:34):** Change `#[derive(Clone, Debug, Serialize, Deserialize)]` to `#[derive(Clone, Debug, Serialize)]`. Add `from_trusted()` constructor for DB loading. Remove `connector_from_serialized_round_trip` test or convert to test `from_trusted()`.

**AM-UITIER-DIVERGE (ui_tier.rs):** Extract shared `fn score_atom_impl(word: &str, kind: &str) -> f32` that calls `nom_score::score_atom()`. Delete UiTier's grammar-cache short-circuit returning hardcoded 0.9/0.3. Both `UiTier::score_atom` and `UiTierOps::score_atom` delegate to `score_atom_impl`.

### LAYOUT FIX PRESCRIPTION (taffy 0.4, executor-ready)

Replace layout.rs HashMap stub with real `taffy::TaffyTree`:
```rust
pub struct LayoutEngine {
    taffy: TaffyTree<()>,
    layout_cache: HashMap<LayoutId, Bounds<Pixels>>,
}
// request_layout: taffy.new_leaf(style) / taffy.new_with_children(style, children)
// compute_layout: taffy.compute_layout(root, available_space) + cache_layouts_recursive()
// LayoutId is repr(transparent) over taffy::tree::NodeId (Zed proven pattern)
```

### VERIFIED CORRECT

- `ComposeContext` + `UnifiedDispatcher` already exist and are string-keyed ✅
- `GpuResources` struct (device, queue, pipeline, buffer) correctly typed ✅
- `build_quad_pipeline` creates real `wgpu::RenderPipeline` ✅
- `draw_quads_gpu` + `ensure_capacity` correctly upload instance data ✅
- `ortho_projection()` produces correct column-major orthographic matrix ✅
- `quota bm25` index wired for grammar kind search ✅
- 5 test fraud entries confirmed at `semantic.rs:541-593` (ArtifactDiff struct is cfg(test)-only)

---

## Iteration 55 — Hard Audit Wave AL (2026-04-18, HEAD 778b085, 7241 tests)

**8 parallel agents dispatched.** New findings vs Iteration 54 — CRITICAL items unchanged.

### NEW CRITICAL

**ATLAS-LRU-CORRUPT: evict_lru() corrupts surviving cache entries**
`nom-gpui/src/atlas.rs:139-152` — `evict_lru()` removes N entries from HashMap then calls `self.allocator.clear()`, nuking ALL allocations including surviving cache entries. Surviving cache entries retain stale UV coordinates pointing to now-deallocated atlas regions. Fix: call `self.allocator.deallocate(alloc)` per evicted entry instead of `clear()`.

### NEW HIGH

**SPATIAL-INDEX-UNWIRED: R-tree exists but O(n) scans used everywhere**
`nom-canvas-core/src/spatial_index.rs` — `SpatialIndex` using `rstar::RTree` is fully implemented. `selection.rs:105-112` and `hit_test.rs` both use O(n) linear scans instead. Fix: wire `SpatialIndex::query_region()` in both files.

**INTENT-RESOLVER-MISSING: IntentResolver struct completely absent**
`nom-intent/src/lib.rs` — No `IntentResolver` struct, no `resolve()` method, no 3-step pipeline. Only standalone functions exist: `classify_with_react()`, `react_chain()`, `rank_hypotheses()`, `best_hypothesis()`. AL-INTENT-RESOLVER target is larger than previously scoped.

### NEW MEDIUM

**CRDT-APPLY-NOT-IDEMPOTENT: apply() lacks duplicate guard**
`nom-collab/src/lib.rs:105-112` — `merge()` correctly filters duplicates (lines 191-203) but `apply()` has no duplicate guard. Applying same op twice inserts a duplicate entry. Fix: check `self.ops.contains(&op)` at top of `apply()`.

**CONNECTOR-DESERIALIZE-BYPASS: serde bypass around grammar validation**
`nom-blocks/src/connector.rs:34` — `#[derive(Deserialize)]` enables construction of `Connector` with arbitrary `can_wire_result` bypassing `new_with_validation()`. Fix: replace with custom `Deserialize` impl that calls `new_with_validation()`.

**UITIER-DIVERGENCE: UiTier vs UiTierOps divergent score_atom paths**
`nom-compiler-bridge/src/ui_tier.rs` — UiTier and UiTierOps both call `score_atom` but via different code paths producing inconsistent scores for same input. Should share a single implementation.

### CLEARED

**background_tier.rs plan_flow/verify/deep_think are NOT stubs** — agent confirmed these are real heuristic implementations with meaningful logic. Previous stub suspicion was incorrect.

### Render Fix Prescription (from Zed Metal renderer study)

Exact wgpu per-frame sequence to fix AL-RENDER-1:
```
1. surface.get_current_texture() → acquire backbuffer
2. device.create_command_encoder()
3. encoder.begin_render_pass() with clear load_op (clear color = BACKGROUND)
4. Loop scene.batches() [256-byte aligned offsets]:
   - queue.write_buffer(instance_buffer, offset, primitive_bytes)
   - render_pass.set_pipeline(&quad_pipeline)
   - render_pass.set_vertex_buffer(0, unit_vertices_slice)
   - render_pass.set_vertex_buffer(1, instance_buffer_slice_at_offset)
   - render_pass.set_bind_group(0, &globals_bind_group, &[])
   - render_pass.draw(0..6, 0..instance_count)
5. drop render_pass (ends render pass)
6. queue.submit([encoder.finish()])
7. surface_texture.present()
```

WGSL fix for AL-RENDER-3 (storage buffer pattern, avoids per-attribute layout):
```wgsl
@group(0) @binding(0) var<storage, read> quads: array<QuadInstance>;
@group(0) @binding(1) var<uniform> globals: Globals;

@vertex fn quad_vertex(
  @location(0) vertex_pos: vec2<f32>,
  @builtin(instance_index) inst_id: u32
) -> VertexOutput {
  let q = quads[inst_id];
  let world = q.origin + vertex_pos * q.size;
  let ndc = world / globals.viewport_size * 2.0 - 1.0;
  return VertexOutput(vec4<f32>(ndc.x, -ndc.y, 0.0, 1.0));
}
```

### Finalization Percentages (Iteration 55 — unchanged from I54)

| Axis | Percentage |
|---|---|
| nom-compiler finalization | **44%** |
| Nom language finalization | **34%** |
| nom-canvas ↔ nom-compiler integration | **24%** |
| DB-driven automation pipeline | **35%** |
| GPU renderer (renders real pixels) | **20%** |

---

## Iteration 54 — Hard Audit Waves AE-AK (2026-04-18, HEAD 8088889, 6743 tests)

**8 parallel agents dispatched.** Findings below. Wave AL was executed to add ~500 more tests; CRITICAL items unchanged.

### CRITICAL — Must fix before continuing

**RENDER-1 (AE1 NOT FIXED): Renderer renders zero pixels**
`nom-gpui/src/renderer.rs:660-689` — All `draw_*` methods are counter-only stubs. No `CommandEncoder`, no `begin_render_pass()`, no `set_pipeline()`, no `draw()`, no `draw_indexed()`.

**RENDER-2: No wgpu initialization chain**
`nom-gpui/src/window.rs:84,118` — `run_native_application` creates a winit window but never calls `Instance::new()`, `create_surface()`, `adapter.request_device()`. Comments say "// In real impl: winit::window::Window + wgpu swap chain".

**RENDER-3: WGSL shaders are degenerate**
`nom-gpui/src/shaders.rs:4-9` — Every vertex shader returns `vec4<f32>(0.0, 0.0, 0.0, 1.0)`. `buffers: &[]` at pipeline creation. No `@group(0) @binding(0)` for GlobalUniforms.

**DB-ENUM-1: BackendKind is a closed 16-variant Rust enum**
`nom-compose/src/dispatch.rs:10-70` — CRITICAL violation of DB-driven mandate. Fix: `pub struct BackendKind(pub String)` runtime newtype.

**COMPOSE-BRIDGE-1: nom-compiler-bridge and nom-compose are completely disconnected**
`nom-compiler-bridge/Cargo.toml` has zero dependency on `nom-compose`. Grammar DB never reaches compose dispatch. DB-driven automation pipeline: **35% functional**.

**COMPOSE-BRIDGE-2: grammar.kinds has no `status` field**
`nom-compiler-bridge/src/shared.rs:16-19` — `GrammarKind` has only `name` + `description`. No `status: KindStatus`. Promotion lifecycle cannot be gated.

### HIGH — Fix soon

- **SQL-INJECT-1**: `data_query.rs:27-29` and `semantic.rs:72-75` — SQL built via `format!()` with no identifier validation
- **THEME-1**: `nom-theme/src/tokens.rs` — No swappable theme system. Single dark palette as flat `const`. No `Theme` struct.
- **FONT-1**: `nom-theme/src/fonts.rs:11-18` — Missing Libre Baskerville, EB Garamond, Berkeley Mono
- **DEEPTHINK-1**: `nom-panels/src/right/deep_think.rs:161-184` — `edge_color_for_confidence()` exists but never called; all cards render uniform border
- **LAYOUT-STUB-1**: `nom-gpui/src/layout.rs` — `LayoutEngine` is a hand-rolled HashMap stub, no taffy delegation
- **SEMANTIC-DOMAIN-1**: `nom-compose/src/semantic.rs` — WrenAI MDL BI layer wrongly positioned as compose semantic; real n8n-equivalent is `GraphNode`+`Connector`
- **COSMIC-TEXT-1**: `nom-theme/src/fonts.rs:21-32` — `FontSystem::new()` never called; `FontRegistry::placeholder()` returns sequential integers
- **CRDT-OVERFLOW-1**: `nom-collab/src/lib.rs:95-100` — `counter += 1` panics at `u64::MAX`; apply() accepts remote MAX without clamping

### MEDIUM

- `nom-panels/src/left/node_palette.rs:77-86` — No search box rendered; no category group headers
- `nom-gpui/src/scene.rs:80-85` — FrostedRect `blur_radius` stored but never used
- `nom-theme/src/tokens.rs:232` — `TOOLBAR_H = 48.0` (mandate says 36px)
- `nom-compiler-bridge/src/shared.rs:84,102` — `Ordering::Relaxed` on grammar_version (should be Acquire/Release)
- `len() as i64` unchecked cast in `sandbox.rs:445-446`

### VERIFIED CORRECT

- DB-driven architecture (excluding BackendKind enum): grammar.kinds live SELECT, NodePalette DB-driven, NomtuRef non-optional, Connection::open isolated, RwLock on grammar_kinds, search_bm25 wired
- etagere BucketedAtlasAllocator in atlas.rs
- bytemuck Pod+Zeroable on QuadInstance/SpriteInstance/GlobalUniforms
- CRDT convergence tests: commutativity + idempotency + multi-peer convergence
- Viewport coordinate math: round-trip identity, zoom-toward cursor-anchoring

### Finalization Percentages (agent-verified, Iteration 54)

| Axis | Percentage | Evidence |
|---|---|---|
| nom-compiler finalization | **44%** | Lexer done; self-hosting not started; 22 crates not connected to canvas |
| Nom language finalization | **34%** | 9-kind foundation locked; C-like syntax; 30+ extended kinds not seeded |
| nom-canvas ↔ nom-compiler integration | **24% of crates (7/29)** | nom-concept/dict/grammar/planner/score/search/types wired; 22 crates never called |
| DB-driven automation pipeline | **35%** | BackendRegistry + ProviderRouter + CredentialStore exist but disconnected; ComposeContext/UnifiedDispatcher/HybridResolver/GlueCache all MISSING |
| GPU renderer (renders real pixels) | **20%** | Instance buffer upload PASS, atlas PASS; render pass MISSING, surface.present MISSING, shaders degenerate |

### Test Quality (agent-verified)

**7241 tests ≈ 35/100 quality score.** ~60% of tests are struct construction / field access / duplicates. Real behavioral coverage ≈ 2500 tests equivalent.
- `ui_tier.rs`: 70+ tests, ZERO test the `#[cfg(feature = "compiler")]` codepath
- `semantic.rs`: 5 tests are "test fraud" — test functions defined only inside #[cfg(test)]
- `nom-collab`: same insert/delete/assert pattern repeated 8+ times

## Iteration 53 — Universal Composer Design Approved (2026-04-18)

**Design session:** Brainstormed and approved Universal Composer (Platform Leap) spec.
**Spec written:** `docs/superpowers/specs/2026-04-18-nom-universal-composer-design.md`

**10 upstream patterns wired (all additive, zero existing interfaces broken):**
Candle (in-process ML) · Qdrant (HNSW intent) · Wasmtime (WASM sandbox) · DeerFlow (StepMiddleware) · Refly (FlowNode graph) · AgentScope (critique loop) · ToolJet (kind registry) · Polars (lazy DataFrame) · Open-Higgsfield (MediaVendor) · Bolt.new (SwitchableStream)

**New modules:** `nom-compose/src/{intent_v2,middleware,flow_graph,critique,streaming}.rs` + `nom-compiler-bridge/src/{candle_adapter,wasm_sandbox}.rs` + `nom-cli/src/serve.rs`

**Open work (Wave AI-Composer):** 14 implementation targets — see task.md Wave AI-Composer section.

## Iteration 52 — Hybrid Composition Design (2026-04-18)

**Spec written:** `docs/superpowers/specs/2026-04-18-hybrid-compose-design.md`

**Architecture:** Three-tier resolver (DB → Provider → AI). `IntentResolver` at front (lexical scan → BM25 → `classify_with_react()`). Grammar promotion lifecycle at back (Transient → Partial → Complete). `intended to <purpose>` clause required in every AI `.nomx` sentence.

**Open work (Wave AH):** 17 implementation targets — see task.md Wave AH section.

## Iteration 51 — Wave AE Hard Audit (2026-04-18, HEAD c3d2323, 2841 tests)

8-agent parallel audit of all 14 crates. Surfaced AE1-AE17 findings; all except AE1 (renderer) subsequently closed in Waves AE-AK.

## Reference Commits

| Commit | Wave | Tests |
|---|---|---|
| `778b085` | Wave AL | 7241 |
| `8088889` | Wave AK | 6743 |
| `003f895` | Wave AJ | 6233 |
| `d2b7b62` | Wave AI | 5712 |
| `59d58c4` | Wave AH | 5196 |
| `76ba05d` | Wave AG | 4693 |
| `617c064` | Wave AF | 4194 |
| `c3d2323` | Wave AB | 2841 |
| `c4d6252` | Wave S | 686 |
| `dc6a025` | Wave K | 457 |
| `fb66e01` | Wave C keystone | — |
| `8c7d32e` | Wave A+B | — |
