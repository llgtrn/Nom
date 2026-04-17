# Remaining Work — Execution Tracker

> **CANONICAL TRACKING DOC — MAIN** (Planner/Auditor refreshes every cycle)
> **HEAD:** `6196ef1` | **Uncommitted:** nom-canvas/ Rust workspace (Phase 1 batch-1 31/31) + v1 archive move | **Date:** 2026-04-17
> **Spec:** `docs/superpowers/specs/2026-04-17-nomcanvas-gpui-design.md` (719 lines) — canonical
> **Sibling docs:** `implementation_plan.md`, `nom_state_machine_report.md` (all 4 MUST stay in sync)
> **v1 archived** to `.archive/nom-canvas-v1-typescript/`. `nom-canvas/` Phase 1 batch-1 lives (31/31 tests pass).

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
- [ ] wgpu renderer (consume PrimitiveBatch → draw_indexed per batch)
- [ ] cosmic-text + etagere wiring into PlatformAtlas
- [ ] winit window + event loop + 60fps frame timing
- [ ] Platform abstraction (desktop native vs browser WebGPU)

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
