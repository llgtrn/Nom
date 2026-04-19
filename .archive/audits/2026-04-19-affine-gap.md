# AFFiNE Pattern Adoption Gap Analysis
**Date:** 2026-04-19  
**Auditor:** Gap-analysis auditor (read-only)  
**Scope:** `nom-gpui`, `nom-panels`, `nom-theme`, `nom-blocks` vs. AFFiNE canary  
**Status:** PARTIAL — "tokens real, frosted stub" (confirmed)

---

## 1. Pattern Summary

AFFiNE’s frontend is built on three tightly coupled subsystems that Nom claims to adopt:

1. **Design Token System** (`@toeverything/theme` v1.1.23+) — A runtime CSS-variable design system with 70+ semantic tokens (colors, sizes, fonts, shadows, radii) accessed via `cssVar()` / `cssVarV2()` and injected into Lit-based Web Components.

2. **Block Editor Architecture** (`blocksuite/affine/blocks/`) — A CRDT-backed block tree using `BlockComponent<Model>` (Lit), `defineBlockSchema()`, Yjs `Boxed(new Y.Map())`, and a `SurfaceBlockModel` with spatial indexing (`GridManager`, `LayerManager`), connector graph indices, and incremental update subscriptions.

3. **Dual Rendering Pipeline** (`blocksuite/affine/blocks/surface/src/renderer/`) — Two interchangeable renderers:
   - `CanvasRenderer`: HTML5 Canvas 2D + `RoughCanvas` sketchy drawing, stacking canvas layers with z-index, pooled canvas reuse, viewport culling, and debug metrics.
   - `DomRenderer`: Incremental DOM updates, placeholder fallback during zoom, element renderer extension registry (`DomElementRendererIdentifier`), and dirty-element tracking.

4. **Animation & Motion** — CSS transitions using `cubic-bezier` curves (no spring physics on web; spring exists only in iOS SwiftUI). Full cubic Bezier geometry library (`curve.ts`) for path editing.

5. **Collapsible Panels** — `ResizePanel` with `useTransitionState`, drag handles with document-level mouse tracking, min/max bounds, floating mode, and CSS-variable-driven `animationTimeout`.

---

## 2. Key Source Files (AFFiNE Reference)

| Subsystem | File | What It Contains |
|-----------|------|------------------|
| Design Tokens | `blocksuite/affine/shared/src/theme/css-variables.ts` | 70+ `--affine-*` CSS custom properties exported as `ColorVariables`, `SizeVariables`, `FontFamilyVariables`, `StyleVariables` |
| Design Tokens | `packages/frontend/component/src/theme/theme.css.ts` | `globalStyle` injection using `cssVar('textPrimaryColor')` and `cssVar('fontFamily')` |
| Design Tokens | `packages/frontend/core/src/modules/theme/services/theme.ts` | `AppThemeService` — runtime theme entity/service |
| Frosted Glass | `packages/frontend/component/src/ui/modal/overlay-modal.tsx` | `backdropFilter: 'blur(2px)'` — only real backdrop-filter usage in component lib |
| Frosted Glass | `blocksuite/affine/blocks/surface/src/surface-block.ts` | CSS: `background-color: var(--affine-background-primary-color)` with radial grid gradient |
| Block Model | `blocksuite/affine/blocks/surface/src/surface-model.ts` | `SurfaceBlockModel` extends `BaseSurfaceModel`; Yjs `Boxed(new Y.Map())`; connector endpoint indexing |
| Block Model | `blocksuite/affine/blocks/root/src/page/page-root-block.ts` | `PageRootBlockComponent` — root document block with keyboard manager, viewport resize observer, click-on-blank-area handling |
| Rendering | `blocksuite/affine/blocks/surface/src/renderer/canvas-renderer.ts` | `CanvasRenderer` class — 957 lines: stacking canvas, pooled reuse, viewport culling, `_renderByBound`, debug metrics (`CanvasRendererDebugMetrics`) |
| Rendering | `blocksuite/affine/blocks/surface/src/renderer/dom-renderer.ts` | `DomRenderer` class — 756 lines: incremental updates, `_renderIncremental`, placeholder/full element modes, `elementsUpdated` Subject |
| Rendering | `blocksuite/affine/gfx/shape/src/element-renderer/shape/index.ts` | `shape: ElementRenderer<ShapeElementModel>` — canvas shape rendering with `RoughCanvas` |
| Rendering | `blocksuite/affine/gfx/shape/src/element-renderer/shape-dom/index.ts` | `shapeDomRenderer` — DOM-based shape rendering with SVG polygon retention, CSS transforms |
| Bezier | `blocksuite/framework/global/src/gfx/curve.ts` | `getBezierPoint`, `getBezierTangent`, `getBezierNormal`, `getBezierCurvature`, `getBezierCurveBoundingBox`, `curveIntersects` (cubic root solving) |
| Panels | `packages/frontend/component/src/components/resize-panel/resize-panel.tsx` | `ResizePanel` + `ResizeHandle` — drag resize, toggle, `useTransitionState`, 300ms `animationTimeout` |
| Panels | `packages/frontend/component/src/components/resize-panel/resize-panel.css.ts` | `panelWidthVar`, `animationTimeout` CSS variables via `vanilla-extract/css` |

---

## 3. Gap Analysis

### 3.1 Design Token System

| Capability | AFFiNE | Nom | Gap Severity |
|------------|--------|-----|--------------|
| Runtime CSS variables | `cssVar()`, `cssVarV2()` resolve at runtime in browser | Compile-time Rust constants only (`tokens.rs`) | **HIGH** |
| Semantic color scale | `--affine-black-10` … `--affine-black-90`, `--affine-white-10` … `--affine-white-90` | Missing opacity-scale tokens | **HIGH** |
| Tag colors | `--affine-tag-blue`, `--affine-tag-green`, etc. | Missing | **MEDIUM** |
| Shadow tokens | `--affine-shadow-1`, `--affine-shadow-2`, `--affine-shadow-3` | `ShadowToken` struct exists (`SHADOW_SM` … `SHADOW_XL`) but no CSS/shader mapping | **MEDIUM** |
| Font family tokens | `--affine-font-family`, `--affine-font-number-family`, `--affine-font-code-family` | `fonts.rs` exists but no token-to-shader binding | **MEDIUM** |
| Editor width / z-index | `--affine-editor-width`, `--affine-z-index-modal` | Missing | **LOW** |
| Theme v2 | `cssVarV2('text/primary')` — hierarchical naming | No v2 token hierarchy | **MEDIUM** |
| Token injection | `unsafeCSSVar()` binds tokens to Lit `css` tagged templates | No equivalent for WGSL uniform binding | **HIGH** |

**Nom’s `token_system.rs`** seeds only 6 colors + 4 spacing tokens. **Nom’s `tokens.rs`** defines 73 constants but they are static Rust values with no runtime theming engine, no dark/light CSS injection, and no shader uniform bridge.

### 3.2 Frosted Glass / Backdrop Filter

| Capability | AFFiNE | Nom | Gap Severity |
|------------|--------|-----|--------------|
| Backdrop blur shader | CSS `backdrop-filter: blur(2px)` in modals; `--affine-background-overlay-panel-color` semantic token | `FrostedRect` exists in scene, but `Renderer::draw_frosted_rects()` is a **software stub** | **CRITICAL** |
| Gaussian blur pre-pass | Real GPU blur in CSS compositor | Comment: *"A real GPU implementation would run a two-pass Gaussian blur over the captured framebuffer region… left as a future extension in the `Reserved6` pipeline slot"* | **CRITICAL** |
| Blur radius | 2px (modal), 24px (design mandate) | `FROSTED_BLUR_RADIUS = 12.0` (tokens.rs), `20.0` (`FrostedGlassToken::default_dark()`) — inconsistent with spec | **MEDIUM** |
| Tint color | `--affine-background-overlay-panel-color` | `FrostedGlassToken::tint: [u8; 4]` exists but never read by renderer | **MEDIUM** |

**Concrete finding:** `nom-gpui/src/renderer.rs:898-931` (`draw_frosted_rects`) decomposes each `FrostedRect` into two `QuadInstance`s: a grey fill quad and a white border quad. There is **zero blur computation**. The shader (`QUAD_FRAG_WGSL`) returns `in.color` unmodified — no sampling of background pixels, no separable Gaussian kernel.

### 3.3 Block Editor Architecture

| Capability | AFFiNE | Nom | Gap Severity |
|------------|--------|-----|--------------|
| Block schema system | `defineBlockSchema()`, `BlockSchemaExtension`, flavour strings, metadata roles | No schema definitions; `BlockModel::insert()` only validates `kind` against `DictReader` | **HIGH** |
| CRDT backend | Yjs `Boxed(new Y.Map())` for `SurfaceBlockProps.elements` | No Yjs, no operational transform | **CRITICAL** |
| Block component base | `BlockComponent<Model>` (Lit Web Component) | No `BlockComponent` trait or Lit integration | **CRITICAL** |
| Spatial indexing | `GridManager` + `LayerManager` with z-index | Missing entirely | **CRITICAL** |
| Connector graph | `SurfaceBlockModel._connectorIdsByEndpoint`, `_connectorEndpoints` | `Connector` module exists in `nom-blocks` but no graph index | **HIGH** |
| Viewport | `Viewport` class with zoom, pan, bounds, `viewportUpdated` Observable | Missing | **HIGH** |
| Selection | `SurfaceSelection`, `BlockSelection`, `TextSelection` | Missing selection system | **HIGH** |
| Incremental updates | `elementAdded`, `elementRemoved`, `elementUpdated`, `localElementUpdated` subscriptions | No pub/sub at block model level | **HIGH** |

**Concrete finding:** `nom-blocks/src/block_model.rs` defines a plain `BlockModel` struct with `slots: Vec<(String, SlotValue)>` and `children: Vec<BlockId>`. There is no `BlockStdScope`, no `GfxController`, no rendering lifecycle.

### 3.4 Collaborative Editing Surface

| Capability | AFFiNE | Nom | Gap Severity |
|------------|--------|-----|--------------|
| Real-time sync | Y.Map subscriptions propagate to `CanvasRenderer.refresh()` and `DomRenderer.refresh()` | No CRDT, no websocket, no sync | **CRITICAL** |
| Local vs. remote elements | `localElementAdded`, `localElementDeleted` vs. `elementAdded`, `elementRemoved` | No distinction | **HIGH** |
| Update coalescing | `requestConnectedFrame` coalesces multiple model updates into single render | No RAF coalescing in Nom | **MEDIUM** |

### 3.5 Bezier Animation Curves

| Capability | AFFiNE | Nom | Gap Severity |
|------------|--------|-----|--------------|
| CSS easing curves | `cubic-bezier(0.2, 1, 0.3, 1)` (tooltip), `cubic-bezier(0.25, 0.1, 0.25, 1)` (toast), `ease-in-out` (panels) | `BezierCurve` struct with `ease()`, `ease_in()`, `ease_out()`, `ease_in_out()`, `linear()` | **LOW** (Nom has curves) |
| Bezier geometry for paths | `getBezierPoint`, `getBezierTangent`, `getBezierNormal`, `getBezierCurvature`, `getBezierCurveBoundingBox`, `curveIntersects` | `BezierCurve::sample(t)` only — no tangent, normal, curvature, bounding box, or intersection | **HIGH** |
| Path pipeline integration | Bezier curves used in `RoughCanvas` + shape connectors | `Path` primitive exists in scene but `PATH_VERT_WGSL` / `PATH_FRAG_WGSL` are stubs returning `(0,0,0,1)` | **CRITICAL** |

### 3.6 Spring Physics

| Capability | AFFiNE | Nom | Gap Severity |
|------------|--------|-----|--------------|
| Spring physics (web) | **None** — AFFiNE web uses CSS `cubic-bezier` only | `easing::spring(stiffness, damping)` with underdamped oscillator; `MOTION_SPRING_STIFFNESS = 400.0`, `MOTION_SPRING_DAMPING = 28.0` | **Nom exceeds AFFiNE here** |
| Integration | N/A (no spring on web) | Exists as pure function but **not wired to any animation runner** | **MEDIUM** |

**Paradox:** Nom has a more complete spring physics implementation than AFFiNE’s web frontend, but it is inert — no `AnimationRunner` ticks it, no render loop samples it.

### 3.7 Collapsible Panel Patterns

| Capability | AFFiNE | Nom | Gap Severity |
|------------|--------|-----|--------------|
| Resize handle | `ResizeHandle` component with `onMouseDown` → document `mousemove`/`mouseup`, cursor `col-resize`, toggle-on-click | Missing entirely | **CRITICAL** |
| Min/max bounds | `minWidth`, `maxWidth` props enforced in `onMouseMove` | `PANEL_MIN_WIDTH`, `PANEL_MAX_WIDTH` constants exist but no enforcement logic | **HIGH** |
| Transition state machine | `useTransitionState({ timeout: 300 })` from `react-transition-state` — `exited` / `entering` / `entered` / `exiting` | No state machine; `Dock::toggle()` is instant boolean flip | **HIGH** |
| Animation CSS variable | `animationTimeout` vanilla-extract variable bound to `margin-left`, `margin-right`, `transform`, `background` transitions | `ANIM_DEFAULT_MS = 300.0`, `ANIM_FAST_MS = 200.0` exist but no CSS transition output path | **MEDIUM** |
| Floating mode | `ResizePanelProps.floating` — panel detaches and overlays | Missing | **MEDIUM** |
| Flex-based pane splits | `PaneGroup` recursive `SplitDirection::Horizontal` / `Vertical` with `adjust_flex()` | `PaneGroup` exists with `PaneAxis` and flex adjustment, but **no drag handles** and **no animation** | **MEDIUM** |

### 3.8 Editor Canvas Composition (Rendering Pipeline)

| Capability | AFFiNE | Nom | Gap Severity |
|------------|--------|-----|--------------|
| Dual renderer | `CanvasRenderer` + `DomRenderer` switchable via `enable_dom_renderer` feature flag | Only single `Renderer` (wgpu) | **HIGH** |
| Stacking canvases | `CanvasRenderer._stackingCanvas[]` with z-index, pooled reuse (`_stackingCanvasPool`), `stackingCanvasUpdated` Observable | Missing | **HIGH** |
| Viewport culling | `grid.search(bound, { filter: ['canvas', 'local'] })` + `intersects(getBoundWithRotation(element), bound)` | Missing — all quads rendered unconditionally | **HIGH** |
| Incremental updates | `DomRenderer._renderIncremental()` dirty-element tracking, `_updateState` with `dirtyElementIds` | No dirty-rect or incremental update system | **HIGH** |
| Element renderer registry | `std.getOptional<ElementRenderer>(ElementRendererIdentifier(element.type))` | No extension registry for renderers | **HIGH** |
| Placeholder mode | `usePlaceholder = true` during zoom — renders grey rects instead of full elements | Missing | **MEDIUM** |
| Debug metrics | `CanvasRendererDebugMetrics` with `lastRenderDurationMs`, `canvasMemoryMegabytes`, `renderCount`, `coalescedRefreshCount` | `FrameStats` has counters but no timing, no memory tracking | **MEDIUM** |
| Shadow pass | Two-pass Gaussian blur for drop shadows | `draw_shadows()` increments counter only — no actual shadow pass | **CRITICAL** |
| Sprite pipelines | Monochrome + polychrome sprite atlas pipelines | `SPRITE_VERT_WGSL` / `SPRITE_FRAG_WGSL` are stubs returning `(0,0,0,1)` and `(0,1,0,1)` | **CRITICAL** |
| Path pipeline | Tessellated path vertices | `PATH_VERT_WGSL` / `PATH_FRAG_WGSL` are stubs | **CRITICAL** |
| Underline pipeline | Wavy/thin line segments | `UNDERLINE_VERT_WGSL` / `UNDERLINE_FRAG_WGSL` are stubs | **CRITICAL** |

**Concrete finding:** `nom-gpui/src/renderer.rs:776-820` (`draw` method) calls `draw_quads()`, `draw_paths()`, `draw_monochrome_sprites()`, `draw_polychrome_sprites()`, `draw_underlines()`, `draw_frosted_rects()`. Of these, only `draw_quads()` actually creates `QuadInstance` data and submits to `draw_quads_gpu()`. All other methods are no-ops or counter increments. `end_frame_render()` at line 613-681 does perform a real wgpu render pass, but the scene → GPU data flow is broken (CB2 in `maria-hill-magik-sif.md`).

---

## 4. Adoption Effort Estimate

| Gap Area | Effort | Files to Touch | Blockers |
|----------|--------|----------------|----------|
| **Runtime token system** (CSS variable bridge) | 1 week | `nom-theme/src/token_system.rs`, `nom-gpui/src/shaders.rs` (uniform binding), `nom-gpui/src/renderer.rs` | Need shader uniform buffer for theme colors |
| **Frosted glass GPU pass** | 2 weeks | `nom-gpui/src/renderer.rs` (Reserved6 → real blur), `nom-gpui/src/shaders.rs` (blur WGSL), `nom-gpui/src/scene.rs` | Requires framebuffer capture + two-pass separable Gaussian kernel; wgpu compute or render-to-texture |
| **Block schema + component base** | 2 weeks | `nom-blocks/src/block_schema_v2.rs`, new `nom-blocks/src/component.rs`, `nom-gpui/src/element.rs` | Needs Lit-like component trait or Dioxus bridge; decision: stay GPUI-native or add DOM portal? |
| **CRDT backend (Yjs)** | 3 weeks | `nom-blocks/src/block_model.rs`, `nom-blocks/src/crdt_merge.rs`, new `nom-collab/src/yjs.rs` | Heavy dependency; consider `yrs` crate |
| **Spatial indexing (GridManager / LayerManager)** | 2 weeks | New `nom-gpui/src/grid.rs`, `nom-gpui/src/layer.rs`, `nom-blocks/src/block_tree.rs` | Needs integration with `BlockModel` children + absolute positioning |
| **Dual renderer (Canvas + DOM fallback)** | 3 weeks | New `nom-gpui/src/dom_renderer.rs`, `nom-gpui/src/canvas_renderer.rs`, refactor `nom-gpui/src/renderer.rs` | Architecture decision: is DOM fallback needed for a Rust-native app? |
| **Viewport + culling** | 1 week | `nom-gpui/src/renderer.rs`, `nom-gpui/src/scene.rs`, `nom-gpui/src/types.rs` | Blocked on scene graph → GPU submission (CB2) |
| **Bezier geometry library** | 1 week | `nom-theme/src/tokens.rs` (extend `BezierCurve`), new `nom-gpui/src/bezier.rs` | Pure math — low risk |
| **Spring animation runner** | 1 week | `nom-gpui/src/animation.rs` (wire `AnimationHandle` into render loop), `nom-gpui/src/window.rs` | Blocked on CB1 (event loop wiring) |
| **Collapsible panels (resize handles + transitions)** | 1 week | `nom-panels/src/dock.rs`, `nom-panels/src/pane.rs`, new `nom-panels/src/resize_handle.rs` | Needs mouse-drag event handling in `nom-gpui/src/window.rs` |
| **Element renderer extension registry** | 1 week | New `nom-gpui/src/renderer_registry.rs`, refactor `nom-gpui/src/renderer.rs` | Needs trait objects for `ElementRenderer` |
| **Debug metrics** | 3 days | `nom-gpui/src/renderer.rs` (add `Instant` timing), `nom-gpui/src/scene.rs` | Low risk |

**Total realistic effort: 8–10 weeks** (assuming CB1/CB2 fixed first).  
**Minimum viable frosted glass: 2 weeks** (blur shader only).  
**Minimum viable block editor parity: 4 weeks** (schema + viewport + culling, no CRDT).

---

## 5. Priority Recommendations

1. **Fix CB1/CB2 first** — without a working render loop, all GPU-side gaps (frosted glass, shadows, sprites) are academic.
2. **Implement Gaussian blur pre-pass** — this is the single biggest visual gap between "stub" and "real" frosted glass. Use wgpu compute shader or ping-pong render textures.
3. **Add `GridManager` + viewport culling** — immediately improves performance and unlocks AFFiNE-style incremental rendering.
4. **Wire spring physics to the render loop** — Nom already has better spring math than AFFiNE web; making it run is low-hanging fruit.
5. **Build `ResizeHandle` with drag logic** — needed for the "Zed PaneGroup" metaphor in the design spec.
6. **Defer Yjs/CRDT** — collaborative editing is architecturally important but not on the critical path for a solo demo.

---

*Report generated from end-to-end source reads of 20+ AFFiNE files and 15+ Nom files. All component/function names verified against actual source.*
