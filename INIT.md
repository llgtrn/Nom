# Nom — Session Context

> **Date:** 2026-04-18 | **State:** fresh build — all previous nom-canvas code deleted, rebuilding correctly
> **nom-compiler:** 29 crates UNCHANGED — this is the CORE. Direct workspace deps for everything.
> **NomCanvas:** starting fresh — GPUI substrate from scratch, correct architecture from day 1

---

## Foundation

Everything composed through natural language — words, sentences, grammar.

**The compiler IS the IDE.** Zero IPC. Zero subprocesses. nom-compiler crates are direct workspace dependencies of nom-canvas. Every keystroke is a compile event. Every block is a compiler concept. The canvas IS the compiler rendered.

**The DB IS the workflow engine.** `grammar.kinds` = node-type library. `clause_shapes` = wire type system. `.nomx` = workflow definition language. `nom-compose/dispatch` = execution runtime. No N8N, no Dify, no external orchestrator — the DB does it natively.

**Deep thinking is first-class.** `nom-intent::deep_think()` is a compiler operation — scored ReAct loop, max 10 hypothesis steps, streamed to the right dock as Rowboat reasoning cards.

**GPUI fully Rust — one binary.** wgpu + winit + taffy + cosmic-text. Desktop + browser (WebGPU). No webview, no Electron, no Tauri.

---

## Six Architectural Invariants

1. **Canvas = AFFiNE for RAG** — graph mode: nomtu entities as knowledge node cards, edges carry `confidence + reason`, RAG retrieval context as colored arc overlays, AFFiNE design tokens (frosted glass, blur, bezier routing)
2. **Doc mode = Zed + Rowboat + AFFiNE** — rope buffer (Zed), AFFiNE block model (heading/para/callout/database/linked-block), Rowboat inline AI cards in right dock
3. **DB-driven = N8N/Dify via `.nomx`** — `grammar.kinds` + `clause_shapes` + `nom-compose` = the workflow engine; no external orchestrator
4. **Deep thinking = compiler op** — `nom-intent::deep_think()`, streamed hypothesis chain, user can interrupt
5. **GPUI fully Rust — one binary** — no webview anywhere
6. **nom-compiler is CORE** — direct workspace deps, zero IPC, all canvas objects are DB entries

---

## Build Order

| Wave | What | Key output |
|------|------|-----------|
| **Wave A** | GPUI substrate + basic canvas | nom-gpui · nom-canvas-core · nom-theme |
| **Wave B** | Editor + nomtu-backed blocks | nom-editor · nom-blocks (NomtuRef non-optional) |
| **Wave C** | Compiler bridge (KEYSTONE) | nom-compiler-bridge · stage1→Highlighter first wire · can_wire() · DB-driven node palette |
| **Wave D** | 3-column shell | nom-panels: dock + pane + shell + AFFiNE left dock + Rowboat right dock |
| **Wave E** | Compose backends (real) | 16 backends: video · document · data · web · workflow · image · audio · ... |
| **Wave F** | Graph RAG + deep thinking | AFFiNE graph mode · confidence edges · RAG overlay · deep_think() UI |

---

## What a `.nomx` sentence does

```
the media intro_video is
  intended to create a 30-second brand intro.
  uses the @media matching 'logo animation' with at-least 0.8 confidence.
  composes title_card, logo_reveal, tagline_fade.
```

1. **Document** (Doc mode): rendered as AFFiNE prose block
2. **DB entry**: `entries { word: "intro_video", kind: "media", output_type: "video/mp4" }`
3. **Graph node** (Graph mode): `GraphNode` with ports from `clause_shapes WHERE kind='media'`
4. **Workflow step** (Compose): `NomKind::MediaVideo → video_backend::compose()`
5. **RAG node** (Canvas mode): shown as knowledge card, edges to `logo_animation` and 3 sub-scenes

---

## Canonical Tracking Docs

| File | Role |
|------|------|
| `docs/superpowers/specs/2026-04-17-nomcanvas-gpui-design.md` | NORTH STAR: full architecture, all modes, crate structure, reference repos, 12 non-negotiables |
| `implementation_plan.md` | Build waves A–F + vendoring plan + non-negotiable rules |
| `task.md` | Execution checklist with checkboxes — all waves |
| `nom_state_machine_report.md` | Iteration log — why fresh build, architecture lessons, compiler status |
| `INIT.md` | This file — quick orientation |

**Read the spec first every session.** Plan and spec diverge → update plan, not spec.

---

## Non-Negotiable Rules

1. Agents MUST read source repos end-to-end before writing ANY code
2. Always use `ui-ux-pro-max` skill at `.agent/skills/ui-ux-pro-max/` for ALL UI work
3. Zero foreign identities in public API
4. nom-compiler is CORE — zero IPC, direct workspace deps, linked not spawned
5. DB IS the workflow engine — never add an external orchestrator
6. Every canvas object = DB entry — `entity: NomtuRef` non-optional from day 1
7. Canvas = AFFiNE for RAG — confidence-scored edges, frosted glass, bezier routing
8. Doc mode = Zed + Rowboat + AFFiNE — all three, not just one
9. Deep thinking = compiler op — `deep_think()` in nom-intent, streamed to right dock
10. GPUI fully Rust — one binary, no webview
11. Spawn parallel subagents for all multi-file work
12. Run `gitnexus_impact` before editing any symbol; never ignore HIGH/CRITICAL

---

## Reference Repos (end-to-end reads required)

| Repo | What to adopt | Path |
|------|---------------|------|
| Zed | GPU scene graph + shell (Dock/Panel/PaneGroup) | `APP/zed-main/crates/gpui/` + `crates/workspace/` |
| AFFiNE | Design tokens + left sidebar patterns | `APP/AFFiNE-canary/` |
| rowboat-main | Right dock: ChatSidebar + tool cards + multi-agent | `APP/rowboat-main/apps/x/apps/renderer/` |
| GitNexus | Graph schema: confidence+reason edges, typed nodes | `.gitnexus/` + MCP tools |
| ComfyUI | DAG execution: 4-tier cache, Kahn sort | `APP/Accelworld/services/other2/ComfyUI-master/` |
| n8n | AST sandbox, Code node, credential injection | `APP/Accelworld/services/automation/n8n/` |
| dify | Typed workflow nodes, event-generator pattern | `APP/Accelworld/services/other4/dify-main/` |
| yara-x | Sealed linter trait | `APP/Accelworld/upstreams/yara-x/` |
| typst | Incremental compile (comemo pattern) | `APP/Accelworld/services/other5/typst-main/` |
| WrenAI | Semantic MDL + 5-stage query pipeline | `APP/wrenai/` |
| 9router | 3-tier provider fallback + quota | `APP/Accelworld/services/other4/9router-master/` |
| Remotion | Programmatic video: GPU→frame→FFmpeg | DeepWiki |
| ToolJet | Component registry, 55 widgets | `APP/ToolJet-develop/` |
| graphify | Data visualization patterns | `APP/graphify-master/` |
