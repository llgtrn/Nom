# Nom Compiler + NomCanvas IDE — State Machine Report

> **CANONICAL TRACKING DOC — MAIN** (Planner/Auditor refreshes every cycle)
> **HEAD:** `e2b7ecb` on main | **CI:** canvas job GREEN (cargo check + test 23s on ubuntu-latest); compiler matrix still running | **Date:** 2026-04-17
> **Sibling docs:** `implementation_plan.md`, `task.md`, `docs/superpowers/specs/2026-04-17-nomcanvas-gpui-design.md` (all 4 MUST stay in sync)
> **Compiler:** 29 crates, 1067 tests | **Canvas v1:** ARCHIVED to `.archive/nom-canvas-v1-typescript/`
> **NomCanvas:** Custom GPUI (wgpu+winit+taffy+cosmic-text). Full Rust. GPU-native. Phase 1 foundation landed.
> **Foundation:** Everything built around Nom language. 9 kinds compose everything in the world.

---

## Current State

**Compiler:** 29 crates, 1067 tests. GAP-4/5a/5b/6/7/8/12 shipped. bootstrap.rs (GAP-10) landed. nom-intent has 7 modules (ReAct, prompt, tools, rerank).

**Canvas v1:** ARCHIVED to `.archive/nom-canvas-v1-typescript/`.

**NomCanvas Phase 1 batch-1 (NEW):** 1 crate (nom-gpui), 9 modules, 31/31 tests passing. Scene graph (6 primitive types, z-ordered via BoundsTree R-tree), Element trait (3-phase lifecycle), taffy layout wrapper, Styled fluent builder, PlatformAtlas trait + in-memory impl, geometry + color primitives. Zero foreign identities; zero wrappers — every type is native-implemented. Deps: `wgpu 22`, `taffy 0.6`, `cosmic-text 0.12`, `winit 0.30`, `etagere 0.2`, `bytemuck 1`, `parking_lot`, `smallvec`. Remaining in Phase 1: wgpu renderer, cosmic-text/etagere atlas wiring, winit window loop, browser/desktop platform abstraction.

**NomCanvas Design:** `docs/superpowers/specs/2026-04-17-nomcanvas-gpui-design.md` (719 lines). Custom GPUI. Compiler-as-core. Universal composition. 5 unified modes. 19+ repos read end-to-end.

**nom-workflow skill:** Upgraded with AUDIT lane, full Superpowers list (17 skills), graphify integration, 21 reference repos.

## NomCanvas Key Decisions

| Decision | Choice | Why |
|----------|--------|-----|
| Framework | Custom GPUI | Dioxus desktop = webview (confirmed by end-to-end reading) |
| GPU | wgpu | Cross-platform: Vulkan/Metal/DX12 + WebGPU for browser |
| Layout | taffy | Flexbox/grid in Rust (same as Zed) |
| Text | cosmic-text | Font shaping without platform dependency |
| Compiler | Direct function calls | No IPC, no JSON, no Tauri — compiler crates linked as deps |
| Video | Remotion pattern in Rust | GPU scene graph → frame capture → FFmpeg pipe |
| Design | AFFiNE tokens | Inter + Source Code Pro, 73 variables, pixel-perfect |

## 19 End-to-End Repo Readings

Zed (GPUI rendering), AFFiNE (design system), ComfyUI (DAG + 4 caches), Refly (46 modules + MCP), LlamaIndex (50+ stores), Haystack (component pipelines), ToolJet (55 widgets), n8n (304 nodes + AST sandbox), yara-x (sealed trait linter), typst (comemo incremental), Dioxus (webview confirmed), ArcReel (video agents), waoowaoo (4-phase storyboard), Open-Higgsfield (200+ models), opendataloader-pdf (XY-Cut++), WrenAI (semantic MDL), Huly (30 services + CRDT), 9router (3-tier fallback routing), Remotion (programmatic video via DeepWiki).

## Session Summary

- Brainstormed → designed → built v1 (37 commits, 46 modules)
- 8-agent audit found 59 issues → fixed 8 CRITICAL
- 2nd audit: 3 CRITICAL + 5 HIGH remaining in v1
- 6-agent deep-dive extracted 60 patterns from 48 repos
- v2 design: Custom GPUI + compiler-as-core + universal composition
- 19 repos read fully end-to-end (not README — actual source)
- Remotion video pattern adapted for GPU-native Rust

## Plan Completeness Tracker (loop goal → 100%)

| Scope | Detailed % | Notes |
|-------|-----------|-------|
| Phase 1 batch-1 | 100% landed, audit found **3 CRITICAL + 4 HIGH + 10 MED + 6 LOW** | Fixes now listed as batch-2 prerequisite in task.md |
| Phase 1 batch-2 | 85% | 12 subtasks + audit-fix prerequisites + Zed citations. Still need: wgsl shader skeletons, binding-group layouts spelled out |
| Phase 2 (canvas+editor) | 30% | High-level bullets only; needs per-module task decomposition |
| Phase 3 (blocks+panels) | 25% | 7 block types + AFFiNE tokens listed; no sub-task breakdown |
| Phase 4 (composition) | 20% | Backends named; no concrete interface specs |
| Phase 5 (production) | 15% | Patterns named; no acceptance criteria |
| **Overall plan** | **~52%** | Cron loop iterates to close gaps each minute |

**Iteration 1 delta:** +5 pp (Phase 1 batch-2 decomposed to 12 tasks with Zed citations)
**Iteration 2 delta:** +7 pp — Commit `e2b7ecb` landed; 6 parallel audit agents found 3 CRITICAL rendering-correctness bugs + 4 HIGH API-shape issues + 10 MED pattern gaps. Prerequisites now blocking batch-2 captured in plan + task.md. Archive move verified clean; security posture verified (0 unsafe, 0 CVEs, 0 secrets).
