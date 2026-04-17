# NomCanvas IDE — Design Specification

> **Date:** 2026-04-17 | **Status:** v1 BUILT + AUDITED
> **HEAD:** `ad9b98c` | 40 modules, 20 commands, 35 commits

---

## Vision

Canvas-first IDE where natural language becomes compilable code, apps, video, pictures, web, and systems. Prose blocks transform into valid `.nomx` in real-time via inline nomtu matching.

## Architecture

| Decision | Choice | Rationale |
|----------|--------|-----------|
| IDE paradigm | Canvas-first | Nom's NL syntax blurs prose/code |
| Deployment | Tauri hybrid | Direct nom-compiler access + web canvas |
| Interaction | Inline prose-to-nomx | Every word is a nomtu lookup |
| Canvas | Custom (Canvas API + Prosemirror) | Lighter than BlockSuite, faster than GPUI |
| Text | Prosemirror | Rich text with inline decorations |
| Matching | WASM bridge (nom-grammar) | Sub-ms keystroke feedback |
| Design | Fira Code/Sans, `#0F172A`, `#22C55E` | ui-ux-pro-max generated |

## 40 Frontend Modules

**Core:** main, schema, canvas, styles
**Editing:** transform, parser, hints, folding, multicursor, streaming
**Canvas:** elements, snapping, wiring, quality, renderer, grid, selection
**Graph:** engine, graph-ui, connectors
**Intelligence:** intent, dream, actions, lsp-bridge, search-panel
**Project:** project, library, credentials, settings, history
**UI:** shortcuts, palette, minimap, toolbar, statusbar, theme, cheatsheet, notification, preview, artifact-preview

## 20 Tauri Commands

| Command | Crate | Real? |
|---------|-------|-------|
| compile_block | nom-concept | Yes |
| lookup_nomtu | nom-dict | Yes |
| match_grammar | nom-grammar | Yes |
| hover_info | nom-dict | Yes |
| complete_word | nom-dict+grammar | Yes |
| search_dict | nom-dict | Yes |
| plan_flow | nom-concept | Yes |
| dream_report | nom-app | Yes |
| score_block | nom-score | Yes |
| wire_check | nom-score | Yes |
| ingest_media | nom-media | Yes |
| extract_atoms | nom-extract | Yes |
| security_scan | nom-security | Yes |
| lsp_request | nom-lsp (child) | Yes |
| store/get_credential | file storage | Yes |
| platform_spec | nom-ux | Yes |
| build_artifact | nom-llvm | LLVM-gated |
| resolve_intent | nom-intent | LLVM-gated |

## Versioned Roadmap

| Version | Status | Scope |
|---------|--------|-------|
| **v1** | **BUILT** | Canvas + inline transform + LSP + compile + graph engine + quality + wiring + preview |
| **v2** | Planned | App builder (component composition + data flow) |
| **v3** | Planned | Media pipeline (node graph + nom-media render) |
| **v4** | Planned | Intelligence (output prediction + dream mode + streaming LLM) |
| **v5** | Planned | System targets (OS/embedded/cross-platform) |

## Audit Summary

8 CRITICAL, 13 HIGH, 24 MEDIUM, 14 LOW. Fix queue in `implementation_plan.md`.

## Identity Rule

Zero foreign repo identities in Nom codebase. All patterns abstracted. Source paths for study recorded in implementation_plan.md.
