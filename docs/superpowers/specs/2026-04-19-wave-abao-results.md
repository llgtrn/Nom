# Wave ABAO Results

**Date:** 2026-04-19
**Source baseline:** `ec01e9d` tracking docs marked Wave ABAO planned after Wave ABAN.
**Result scope:** documentation capture for the uncommitted Wave ABAO implementation slice.

## Summary

Wave ABAO adds five small, crate-local primitives that continue the ABT-ABAN coverage-wave pattern: one focused module per crate, each exported from its crate root and protected by module-local unit tests.

The wave adds 45 new module tests across the five ABAO modules. Targeted verification ran 46 matching tests because the `nom-compose` filter also matched one pre-existing `scenario_workflow` test.

## Modules

| Area | File | Result | New tests |
|---|---|---|---:|
| Blocks | `nom-canvas/crates/nom-blocks/src/table_block.rs` | Table cells, rows, blocks, alignment metadata, colspan width calculation, header/data partitioning, and CSV serialization. | 9 |
| Collaboration | `nom-canvas/crates/nom-collab/src/conflict_resolver.rs` | Conflict kind/side classification, severity keys, resolution strategies, processed counters, resolution rate, and automatic strategy selection. | 9 |
| Compiler bridge | `nom-canvas/crates/nom-compiler-bridge/src/type_map.rs` | Type kind classification, `TypeId` keys, `TypeInfo` labels, concrete-type filtering, builtin counting, and name resolution. | 9 |
| Compose | `nom-canvas/crates/nom-compose/src/workflow_compose.rs` | Workflow node types, display symbols, node/edge graph model, active-node filtering, outgoing-edge lookup, and composer summaries. | 9 |
| GPUI | `nom-canvas/crates/nom-gpui/src/layer_stack.rs` | Layer kind z-ordering, layer ids, visibility/opacity checks, effective z sorting, stack depth, top-visible lookup, and kind counts. | 9 |

## Public Surface

The slice exports the new modules from existing crate roots:

- `nom-blocks`: `CellAlign`, `TableCell`, `BlockTableRow`, `TableBlock`, `TableSerializer`.
- `nom-collab`: `ConflictKind`, `ConflictSide`, `Conflict`, `ResolutionStrategy`, `ConflictResolver`.
- `nom-compiler-bridge`: `TypeKind`, `TypeId`, `TypeInfo`, `TypeMap`, `TypeResolver`.
- `nom-compose`: `WorkflowNodeType`, `WorkflowEdge`, `WorkflowComposer`, `WorkflowGraphCompose`, `WorkflowNodeCompose`.
- `nom-gpui`: `LayerKind`, `LayerId`, `Layer`, `LayerStack`, `LayerCompositor`.

## Verification

Targeted tests were run from `nom-canvas`:

```text
cargo test -p nom-blocks table_block
9 passed; 0 failed

cargo test -p nom-collab conflict_resolver
9 passed; 0 failed

cargo test -p nom-compiler-bridge type_map
9 passed; 0 failed

cargo test -p nom-compose workflow_compose
10 passed; 0 failed
Note: includes one pre-existing scenario_workflow test matched by the filter.

cargo test -p nom-gpui layer_stack
9 passed; 0 failed
```

Total verification observed: 46 matching tests passed, 0 failed.

## Design Trace

Wave ABAO keeps the design intent from the active specs:

- `2026-04-17-nomcanvas-gpui-design.md`: strengthens compositor layering and block primitives needed by the canvas shell.
- `2026-04-17-nomcanvas-ide-design.md`: adds editor-facing structured block and workflow graph primitives without introducing a webview path.
- `2026-04-18-hybrid-compose-design.md`: gives workflow composition a typed graph surface that can later sit behind hybrid resolver tiers.
- `2026-04-18-nom-universal-composer-design.md`: continues the universal composer direction with graph-shaped workflow composition.
- `2026-04-19-nom-inspector-design.md`: preserves the inspector direction by keeping results as typed, queryable primitives instead of opaque strings.

## Follow-Up

The code slice itself remains uncommitted in this workspace. This document intentionally commits only the collected ABAO results so the implementation can be reviewed or committed independently.

