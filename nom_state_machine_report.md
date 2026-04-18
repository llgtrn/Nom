# Nom State Machine Report

**Date:** 2026-04-18 | **HEAD:** `fc67aa9` | **Tests:** 8391 | **Workspace:** clean — Waves AP+AQ+AR complete

---

## Iteration 61 — Wave AP COMPLETE (HEAD 679ce6b, 8391 tests)

**5 parallel executor batches. ALL CRITICAL + HIGH items fixed.**

| Gap ID | Fix | Crate |
|--------|-----|-------|
| AL-RENDER-2 | Real wgpu::Surface/Device/Queue fields + full init chain + pollster=0.3 | nom-gpui |
| AL-RENDER-1 | end_frame_render(): CommandEncoder + begin_render_pass + set_pipeline + draw + submit + present | nom-gpui |
| AL-RENDER-3 | VertexBufferLayout (stride=80, 5×Float32x4, Instance); real WGSL QuadIn @location(0-4) + GlobalUniforms + NDC | nom-gpui |
| AM-ATLAS-LRU | evict_lru() calls allocator.deallocate(alloc) per entry; no more allocator.clear() | nom-gpui |
| AL-LAYOUT-TAFFY | LayoutEngine replaced with taffy::TaffyTree + node_map | nom-gpui |
| AL-BACKEND-KIND | BackendKind closed enum DELETED; all dispatch uses runtime &str; UnifiedDispatcher+ComposeContext re-exported | nom-compose |
| AL-GRAMMAR-STATUS | pub status: KindStatus added to GrammarKind; list_kinds() + promote_kind() SQL helpers | nom-compiler-bridge |
| AM-UITIER-DIVERGE | score_atom_impl() extracted; UiTier + UiTierOps both delegate | nom-compiler-bridge |
| AL-DEEPTHINK-CONFIDENCE | edge_color_for_confidence(card.confidence) wired | nom-panels |
| AL-TOOLBAR-HEIGHT | TOOLBAR_H=48.0 deleted; all callers use TOOLBAR_HEIGHT=36.0 | nom-theme |
| AL-FONTS | libre_baskerville_regular, eb_garamond_regular, berkeley_mono_regular added | nom-theme |
| AL-THEME-SYSTEM oled | Theme::oled() constructor added (pure black bg) | nom-theme |
| AN-WORKSPACE-DUP | insert_block() dedup guard; entity() returns Option; remove_node()+remove_connector() | nom-blocks |
| AN-FRAME-SPATIAL rotation+cycle | rotation: f32 field; add_child() returns Result with cycle guard | nom-blocks |
| AN-BLOCKDIFF-WORD | diff_blocks() emits Modified{field:"word"} diffs | nom-blocks |
| AM-SPATIAL-WIRE hit_test | CanvasHitTester with R-tree broadphase in production (no #[cfg(test)] gate) | nom-canvas-core |
| NOM-EDITOR-POINT | Point{row,column} type + Buffer::point_at() + Buffer::offset_from_point() | nom-editor |
| NOM-GRAPH-EXEC | ExecutionEngine::execute() runs plan, calls node logic, stores results in cache | nom-graph |

Previously fixed (Waves AN/AO): AL-CRDT-OVERFLOW, AM-CRDT-IDEMPOTENT, AL-SQL-INJECT, AL-ATOMIC-ORDERING, AM-CONNECTOR-DESER, AN-FRAME-SPATIAL (x/y/w/h), AN-BLOCKDIFF-CONTENT, AN-WORKSPACE-DUP partial, AN-LSP-POSITIONS, NomGraph module.

---

## Open Items (Wave AQ/AT targets)

- ❌ **NOM-GRAPH-ANCESTRY** — Cache keys inspect only immediate parents; transitive closure ancestry walk missing
- ❌ **NOM-BACKEND-SELF-DESCRIBE** — Backend trait missing version/displayName/params schema/input+output declarations
- ❌ **AM-INTENT-STRUCT** — IntentResolver: no bm25_index; resolve() is substring-match; classify_with_react() disconnected
- ❌ **AL-COSMIC** — cosmic_text::FontSystem not initialized; font data still placeholder IDs
- ❌ **AM-SPATIAL-WIRE viewport.rs** — Viewport struct has NO SpatialIndex field
- ❌ **UC-SERVE** — POST /compose axum endpoint not implemented in nom-cli
- ❌ **AL-PALETTE-SEARCH-UI** — node_palette.rs: no 32px search box; no category group header rows
- ❌ **AL-TEST-FRAUD** — semantic.rs: ArtifactDiff + artifact_diff() + 5 tests are cfg(test)-only
- ❌ **AL-FEATURE-TESTS** — ui_tier.rs: zero #[cfg(feature = "compiler")] tests
- ❌ **AN-TEST-DEDUP** — ~85% duplication ratio across 14 crates; target ≤20%

---

## Per-crate Test Counts (Wave AP actuals)

| Crate | Tests |
|---|---|
| nom-gpui | 790 |
| nom-blocks | 560 |
| nom-canvas-core | 575 |
| nom-cli | 400 |
| nom-collab | 546 |
| nom-compiler-bridge | 553 |
| nom-compose | 685 |
| nom-editor | 620 |
| nom-graph | 570 |
| nom-intent | 470 |
| nom-lint | 485 |
| nom-memoize | 468 |
| nom-panels | 601 |
| nom-telemetry | 500 |
| nom-theme | 568 |
| **TOTAL** | **8391** |

---

**Detailed commit history:** `git log --oneline`. This file keeps only latest state + open missions.
