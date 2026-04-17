# Nom Programming Language — Context

> **HEAD:** `e2b7ecb` on main (canvas CI GREEN 23s) | **Date:** 2026-04-17
> **Compiler:** 29 crates, 1067 tests | **Canvas v1:** ARCHIVED (`.archive/nom-canvas-v1-typescript/`)
> **NomCanvas:** Full Rust GPU-native IDE. Custom GPUI (wgpu+winit+taffy+cosmic-text). **Phase 1 batch-1 landed** — nom-gpui crate (9 modules: geometry, color, bounds_tree, scene, atlas, style, styled, element, taffy_layout). 31/31 tests green. Zero foreign identifiers.
> **Foundation:** Everything built around Nom language. Compiler IS the IDE. Dictionary IS the knowledge base.

## NomCanvas — Full Rust GPU-Native Universal Composition Engine

**Spec:** `docs/superpowers/specs/2026-04-17-nomcanvas-gpui-design.md` (719 lines)

**Architecture:** Custom GPUI (NOT Dioxus — Dioxus desktop = webview). wgpu + winit + taffy + cosmic-text.
**Core:** Compiler IS the IDE. 10 crates → 3 thread tiers (UI <1ms, Interactive <100ms, Background >100ms).
**Modes:** Code + Doc + Canvas + Graph + Draw — all on one infinite GPU surface.
**Composition:** Universal — write Nom → get video/image/PDF/data/app/audio/3D.
**Video:** Remotion pattern in Rust — GPU scene graph renders frames → FFmpeg pipe → MP4 (no browser).
**Targets:** Desktop (Vulkan/Metal/DX12) + Browser (WebGPU) from same binary.

**19 repos read end-to-end:** Zed (GPUI), AFFiNE (design), ComfyUI (DAG), Refly (skill+MCP), LlamaIndex (RAG), Haystack (pipeline), ToolJet (components), n8n (workflow), yara-x (linter), typst (incremental compile), Dioxus (confirmed webview), ArcReel (video agents), waoowaoo (storyboard), Open-Higgsfield (200+ models), opendataloader-pdf (data extraction), WrenAI (semantic layer), Huly (collaboration), 9router (provider routing), Remotion (programmatic video via DeepWiki).

## Compiler (29 crates, unchanged)

GAP-4/5a/5b/6/7/8/12 shipped. 1067 tests. GAP-1c in progress. GAP-2/3 blocked. GAP-9/10 planned. bootstrap.rs (GAP-10 real impl) landed.

## NON-NEGOTIABLE Rules

1. Everything built around Nom language foundation (9 kinds compose everything)
2. Executing agents MUST read source repos end-to-end before writing ANY code
3. Always use `ui-ux-pro-max` skill for ALL UI design
4. Zero foreign identities in Nom codebase
5. MACRO point of view every iteration
6. Spawn subagents to plan/structure/create tasks
7. Strict external comparison (study from source paths)
