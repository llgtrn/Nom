# Nom — Implementation Plan

> **CANONICAL TRACKING DOC — MAIN** (Planner/Auditor refreshes every cycle)
> **HEAD:** `6196ef1` | **Uncommitted:** nom-canvas/ Rust workspace (crates/nom-gpui, 31/31 tests) + v1 archive move | **Date:** 2026-04-17
> **Spec:** `docs/superpowers/specs/2026-04-17-nomcanvas-gpui-design.md` (719 lines) — also canonical
> **Sibling docs:** `nom_state_machine_report.md`, `task.md` (all 4 MUST stay in sync)
> **Foundation:** Everything built around Nom language. 9 kinds compose everything.

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

9 modules / 31 tests green / 0 foreign identifiers:

| Module | Role | Pattern Source (end-to-end read) |
|--------|------|----------------------------------|
| `geometry.rs` | Point/Size/Bounds/Pixels triad/TransformationMatrix | zed/gpui/geometry.rs |
| `color.rs` | Rgba + Hsla + source-over blend | zed/gpui/color.rs |
| `bounds_tree.rs` | R-tree (MAX_CHILDREN=12) DrawOrder assignment | zed/gpui/bounds_tree.rs |
| `scene.rs` | 6 typed Vec primitives + batched iterator | zed/gpui/scene.rs |
| `atlas.rs` | PlatformAtlas trait + AtlasTextureKind + InMemoryAtlas | zed/gpui/platform.rs |
| `style.rs` | Style → taffy::Style conversion | zed/gpui/style.rs + taffy 0.6 API |
| `styled.rs` | Fluent builder (.flex_col().w().padding().bg()) | zed/gpui/styled.rs |
| `element.rs` | 3-phase lifecycle trait with caller-owned state | zed/gpui/element.rs |
| `taffy_layout.rs` | LayoutEngine wrapper + bounds caching | zed/gpui/taffy.rs |

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
