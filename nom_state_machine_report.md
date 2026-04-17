# Nom Compiler + NomCanvas IDE — State Machine Report

> **CANONICAL TRACKING DOC — MAIN** (Planner/Auditor refreshes every cycle)
> **HEAD:** `56604c4` on main (wave-10 landed, **1272 workspace tests green** under `RUSTFLAGS="-D warnings -A deprecated"`) | **Date:** 2026-04-17
> **Sibling docs:** `implementation_plan.md`, `task.md`, `docs/superpowers/specs/2026-04-17-nomcanvas-gpui-design.md` (all 4 MUST stay in sync)
> **Standing status lines:** Compiler-as-core = **0% runtime / ~7% stubbed** · 19-repo vendoring = **58% integrated** (8 DEEP, 3 PATTERN, 6 REF-only, 2 NOT-USED)
> **Compiler:** 29 crates, 1067 tests (unchanged) | **Canvas v1:** ARCHIVED to `.archive/nom-canvas-v1-typescript/`
> **NomCanvas:** **13 crates shipped** through wave-10. Phase 1-5 all have code. Wave-4 landed nom-theme + nom-panels + nom-blocks; wave-5 added 5 remaining block types + editor display pipeline + theme fonts/icons; wave-6 added 6 new Phase 4/5 crates (nom-graph-v2, nom-compose, nom-lint, nom-memoize, nom-telemetry, nom-collab); wave-7 added artifact_store/vendor_trait/video_composition/format_translator/semantic + rayon_bridge + watcher + sandbox + typography + command_history; wave-8 stubbed all 10 Phase 4 backends + `register_all_stubs()`; wave-9 added scenario_workflow + plugin_registry + 2 integration tests + cursor + shortcuts + tree_query + validators; wave-10 wired linter-added modules (motion, transition, layout, rendering_hints, presence, commands, lint/rules).
> **Foundation:** Everything built around Nom language. 9 kinds compose everything in the world.

## Session 2026-04-17 log — Waves 4→10

| Commit | Wave | Delivery | Tests |
|--------|------|----------|-------|
| `c2d7090` | 4 | nom-theme + nom-panels + nom-blocks scaffolds + 10 new modules in existing crates | 376 |
| `24f7e05` | CI | Silence dead_code/unused_import under `RUSTFLAGS=-D warnings` (5 sites) | — |
| `4592b85` | 5 | 5 remaining Phase 3 blocks (media/graph_node/drawing/table/embed) + editor display pipeline (syntax_map/display_map/lsp_bridge/inlay_hints/wrap_map/tab_map) + theme fonts/icons | 519 |
| `9f3df57` | 6 | 6 new crates: nom-graph-v2 (Kahn + 4 caches) + nom-compose (dispatch + queue + router) + nom-lint (sealed trait) + nom-memoize (thread-local) + nom-telemetry (W3C) + nom-collab (CRDT types) + nom-editor/line_layout + compose preview blocks + **HIGH animation div-by-zero** + **MEDIUM EmbedKind brand-name rename** + **MEDIUM CI env var** | 751 |
| `2e47d5d` | 7 | compose {artifact_store, vendor_trait, video_composition, format_translator, semantic} + telemetry/rayon_bridge + lint/watcher + graph-v2/sandbox + theme/typography + panels/command_history | 870 |
| `365db9b` | 8 | 10 Phase 4 backend stubs + `register_all_stubs()` covering all 11 NomKind variants (video, image, web_screen, native_screen, data_extract, data_query, storyboard_narrative, audio, data_frame, mesh) | 1028 |
| `4096db9` | 9 | scenario_workflow (last Phase 4 backend) + plugin_registry + 2 integration tests (end_to_end DAG + dispatch_all_kinds) + cursor + shortcuts + tree_query + validators + **HIGH storyboard phase skip** + **MEDIUM SrgbColor rename + FractionalIndex hoist** | 1155 |
| `56604c4` | 10 | Wire linter-added modules: motion + transition + layout + rendering_hints + presence + commands + nom-lint/rules (trailing_whitespace, double_blank_lines) + fix `CommandError::Failed` dead_code | 1272 |

**Audit cycle payoff:** 2 HIGH + 5 MEDIUM + 1 LOW findings identified across wave-6/wave-8 audits, all resolved within the same session without destabilising the test baseline. Each fix was file-isolated with explicit acceptance criteria.

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
| Phase 1 batch-1 | **100% landed + all 3 CRITICAL + 4 HIGH audit fixes VERIFIED** (44/44 tests) | 7 MED + 6 LOW + 5 test gaps still open, none block batch-2 start |
| Phase 1 batch-2 | **95%** | iter-5 added concrete WGSL skeletons (bind groups, unit-quad unpack, NDC normalize, SDF quad, helper fn list), 3 waves (shaders+buffers+ctx / atlas+text+window / full 8 pipelines). Zed shaders.wgsl 1335-line single file + wgpu_renderer.rs pipeline/surface patterns cited throughout. Still need: winit 0.30 ApplicationHandler scaffold + wgpu_context device-lost recovery detail |
| Phase 2 (canvas+editor) | 85% | 29 subtasks across 2 crates + 10 test targets + 6 explicit non-goals + Excalidraw/Zed/AFFiNE file:line citations |
| Phase 3 (blocks+panels) | **85%** | iter-5 decomposed into ~45 subtasks: shared infra (5) + 7 block types × (schema + render + events + transformer) + nom-panels (7) + nom-theme (4) + 4 ADOPT patterns + 3 SKIP patterns + 8 test targets. AFFiNE defineBlockSchema pattern ported as `define_block_schema!` macro |
| Phase 4 (composition) | **100%** | iter-9 backfilled 5 missing backends (media/storyboard waoowaoo 4-phase, media/novel→video ArcReel adapted via nom-intent, media/audio, data/transform Polars MVP new `nom-data` crate, media/3D glTF) + `MediaVendor` trait (blueprint §12) + content-addressed artifact store `~/.nom/store/<hash>/` + 6 compose-output preview block types + WrenAI 5-stage pipeline detail + 11 iter-9-specific SKIP items. Total: ~95 Phase 4 subtasks. **Zero foreign identities mandate honored**: ArcReel Claude-SDK-direct → adapted to nom-intent agents + MediaVendor facade; polars crate → write own nom-data MVP |
| Phase 5 (production) | **90%** | iter-8 decomposed into ~35 subtasks across 5 new crates: nom-lint (6 modules, yara-x sealed pattern + Nom-improvement visitor + incremental cache), nom-collab (7 modules, Huly-minimal Rust port of Hocuspocus w/ Yrs), nom-memoize (5 modules, comemo pattern port WITHOUT the crate dep), nom-telemetry (6 modules, 4-tier OTel + W3C propagation), file-watcher (2 modules). 13 test targets including compile-fail sealed-trait check + CRDT convergence property test |
| **Overall plan** | **100%** | Blueprint fully decomposed across all 18 sections. Remaining work is EXECUTION, not planning. Executor fix-wave stalled 5 iterations blocks wave-2 start |

**Blueprint-gap backfill (iter-8):** caught 6 modules named in spec §8 but missed in earlier decompositions — `nom-gpui/animation.rs`, `nom-editor/input.rs` (IME!) + `completion.rs`, `nom-panels/properties.rs`, `nom-theme/fonts.rs` + `icons.rs`. Added as Phase 1/2/3 addenda.

## Iteration 17 — 2026-04-17 (HEAD `365db9b`, end-to-end promise audit)

**User question:** "Everything is composed through natural language — words, sentences, grammar — and the canvas is the place to show it. Is the structure now like that?"

**Answer: NO. The blueprint's core promise is 0% delivered.** Render substrate + 1028 tests + 12 crates + 11 backend stubs all exist, but the **input path (prose→compiler) and output path (artifact→canvas preview) are both completely disconnected**.

### Input path — prose → compiler: BROKEN (6 missing wires)

User can type into a `ropey::Rope`-backed `Buffer` ([nom-editor/src/buffer.rs:30-34](nom-canvas/crates/nom-editor/src/buffer.rs#L30-L34)) if someone sends `EditorCommand`s, but the text dies there as a plain Rust `String`:

1. **No winit event loop → editor wire** — `KeyEvent`/`EditorCommand` types exist in `nom-editor/src/input.rs:82-106`, but nothing in `frame_loop.rs` dispatches OS keyboard events to them
2. **No Buffer → Block model wire** — `ProseBlock.text: String` ([prose.rs:31-36](nom-canvas/crates/nom-blocks/src/prose.rs#L31-L36)) and `NomxBlock.source: String` ([nomx.rs:15-16](nom-canvas/crates/nom-blocks/src/nomx.rs#L15-L16)) are plain fields never updated from editor buffer changes
3. **No Block → compiler wire** — grep `nom-canvas/crates/**` for `use nom_concept`/`use nom_grammar`/`use nom_lsp`/`use nom_dict`: **0 matches**
4. **No SyntaxMap producer** — `nom-editor/src/syntax_map.rs:68` has `on_edit` signaling "needs reparse" but no parser ever registered
5. **`Highlighter::color_runs` consumer exists, no producer** — [highlight.rs:93-106](nom-canvas/crates/nom-editor/src/highlight.rs#L93-L106) is ready to color `HighlightSpan`s but nothing in the crate creates them
6. **LSP client uninstantiated** — `nom-lsp` exists in compiler; never constructed from canvas

**Result:** typing `the media product_video is...` produces pure keystrokes → `Rope::insert`. Zero synonym resolution, zero dictionary lookup, zero kind classification, zero syntax highlighting driven by compiler.

### Output path — compose → canvas: BROKEN (5 missing wires)

1. **`ComposeDispatcher::dispatch()` only called in tests** — grep confirms zero non-test callers across `nom-canvas/crates/`
2. **Two disconnected `CompositionPlan` structs** — `nom-compose/src/plan.rs:30` (canvas-side, test-only construction) vs `nom-compiler/crates/nom-planner/src/lib.rs:189` (compiler-side). No bridge between them.
3. **Backends return placeholder bytes** — `StubVideoBackend::compose()` at [video.rs:120-132](nom-canvas/crates/nom-compose/src/backends/video.rs#L120-L132) returns `ComposeOutput { bytes: Vec::new(), mime_type: "video/mp4" }`. Doc comment: "Real impl spawns an ffmpeg process" — real impl absent.
4. **`ArtifactStore::put` has 0 non-test callers** — grep across entire repo confirms
5. **Compose preview blocks don't read `ArtifactStore`** — `VideoBlockProps.source_id: String` opaque; no read path from artifact hash

**Result:** no UI can trigger a compose; plan can't be built from user prose; stubs return empty bytes; no artifact flows back to canvas preview.

### The keystone wire (architect's pick)

**`stage1_tokenize` adapter** from `nom_concept::stage1_tokenize` → `nom-editor/src/highlight.rs::Highlighter::color_runs`.

**Why THE keystone:**
1. **Both endpoints exist and match.** Producer at [stages.rs:112](nom-compiler/crates/nom-concept/src/stages.rs#L112) is a pure fn (`&str → Result<TokenStream>`) — no DB, no async, no runtime. Consumer at [highlight.rs:25-34, :93-106](nom-canvas/crates/nom-editor/src/highlight.rs#L25-L106) takes `&[HighlightSpan]`. Adapter is a straight match arm: `Tok::The|Tok::Is|Tok::Composes|...` → `TokenRole::Keyword`, `Tok::Word(_)` → `Ident`, etc.
2. **Crosses the workspace boundary with minimum scope.** Establishes `path = "../nom-compiler/crates/nom-concept"` pattern — every future wire (hover, completion, compose) reuses it.
3. **First user-visible compiler behavior.** Type `the greeting is...` → keywords highlight as the compiler parses. 10-second demo possible after keystone lands.

**Risk:** `nom-concept` pulls `rusqlite` via `nom-grammar`. Fallback: extract `nom-concept-core` crate with only `lex` module + `Tok`/`Spanned` types (~200 LOC, no DB deps) before linking.

**Subsequent dependency chain (auto-unlocks after keystone):**
```
stage1 adapter (keystone)
  → S2 kind_classify drives fold markers
  → hover via nom-grammar::resolve_synonym (statusbar)
  → Async runtime required HERE for nom-lsp
  → completion via nom-resolver
  → Cmd+K compose via S1-S6 pipeline + first real backend
  → video/media artifact flow
```

### Verdict

The 12 crates + 1028 tests ship a **working render/edit substrate** that is LOOKING FOR a compiler to drive it. Every trait seam is in place (`LspProvider`, `Highlighter`, `CompileStatus`, `CompositionBackend`, `ArtifactStore`). The scaffolding is honest — nothing is secretly reimplementing compiler logic inline. It's waiting for the cross-workspace Cargo dep and a single adapter.

**The one wire that makes the blueprint's promise visible: stage1 tokenize → syntax highlighting on the canvas.** Until that lands, the compose-by-natural-language story is aspirational architecture, not working code.

---

## Iteration 16 — 2026-04-17 (HEAD `365db9b`, 5m-cron loop tick 1)

**Late-breaking: wave-8 `365db9b` also landed during this tick** — 10 Phase 4 backend stubs in `nom-compose/src/backends/`: video, image, web_screen, native_screen, data_extract, data_query, storyboard_narrative (5+4-phase pipelines), audio, data_frame (Polars pattern, no external deps), mesh (glTF 2.0). Registered via `register_all_stubs()` covering all 11 `NomKind` variants. Tests 870 → **1028** (+158).

**Vendoring implications (pending deep audit next tick):** storyboard_narrative.rs landing 5-phase + 4-phase pipelines likely promotes **ArcReel** and **waoowaoo** PATTERN→DEEP. `image.rs` + `data_extract.rs` stubs likely promote **Open-Higgsfield** + **opendataloader-pdf** REF→PATTERN. Conservative estimate: **58% → ~63-68% vendoring** after wave-8 audit confirms stub substance.

---



2 new commits during dispatch: `9f3df57` wave-6 (6 new crates, 751 tests) + `2e47d5d` wave-7 (60 module additions across 7 crates, 870 tests). **12 nom-canvas crates now** (nom-collab added in wave-6).

### (A) Compiler-as-core: STILL 0% runtime integration

Grep-verified post-wave-7: zero `use nom_*::` imports across nom-canvas, zero `path = "../../../nom-compiler/...` deps, only 4 files mention `nom-compiler` (all in comments deferring wiring). **Wave-7's "telemetry bridge" is `nom-telemetry/rayon_bridge.rs`** (rayon span propagation for tracing) — unrelated to compiler-canvas wiring despite similar name.

**Architect designed concrete `nom-compiler-bridge` crate spec** this tick (ready for Executor to land):

- **Module layout** (10 files): `lib.rs`, `shared.rs` (Arc<RwLock> dict pool + grammar + compile_cache + bm25), `dispatcher.rs` (Request/Response enums), `ui_tier.rs` (sync cached reads), `interactive_tier.rs` (tokio mpsc), `background_tier.rs` (crossbeam + worker thread), `adapters/{highlight,lsp,completion,score}.rs`
- **Cargo path deps**: `nom-concept`, `nom-dict`, `nom-grammar`, `nom-score`, `nom-search` via `../../../nom-compiler/crates/*`. New external: `tokio`, `crossbeam-channel`. Gated behind `feature = "compiler"` so existing code compiles without
- **Public API** (~15 fns): `CompilerBridge::new(config)`, `bridge.ui().{lookup_nomtu, score_atom, can_wire, grammar_keywords}`, `bridge.interactive().{tokenize, highlight_spans, complete_prefix, hover}`, `bridge.background().{compile, plan_flow, verify}`
- **First-wire target — `stage1_tokenize` → Highlighter**: highest leverage because both producer (`nom_concept::stage1_tokenize` at `stages.rs:112-117`, pure fn, no DB) and consumer (`nom-editor/src/highlight.rs:42-107` with `Highlighter::color_runs`) are fully implemented. Only the `Spanned.pos → byte_range` length computation is non-trivial (scan forward to next token's pos).
- **LOC estimate**: ~350 LOC for scaffold + first adapter; ~500 LOC to include tokio runtime for Interactive tier.

### (B) Vendoring: 53% → **58%** (+5pp)

Wave-7 deepened two repos:

- **n8n REFERENCED-ONLY → DEEP**: `nom-graph-v2/src/sandbox.rs` ports **all 4 AST sanitizers** from n8n expression-sandbox pattern (this_replace, prototype_block, dollar_validate, allowlist + sanitize combinator) with 17 tests. This was the "critical security-load-bearing finding" from iter-7.
- **Remotion PATTERN → DEEP**: `nom-compose/src/video_composition.rs` lands concrete `VideoComposition + SceneEntry + active_scenes + validate` with 18 tests — matches the blueprint §18 architecture exactly.

Bonus: **WrenAI refreshed** with full `SemanticModel + SemanticEntity + DerivedMetric + EntityRelation + validate` (13 tests in `nom-compose/src/semantic.rs`), **9router pattern deepened** via `format_translator.rs` (Anthropic/OpenAI/Google wire-format mapping, 10 tests), **artifact_store.rs** (content-addressed store from blueprint §12, 10 tests).

**Updated vendoring table:**

| Tier | % | Repos |
|------|---|-------|
| **DEEP** | **42%** (8/19) | Zed, AFFiNE, yara-x, typst, WrenAI, 9router, **n8n** (new), **Remotion** (promoted) |
| **PATTERN** | **16%** (3/19) | ComfyUI, waoowaoo, ArcReel |
| **REFERENCED-ONLY** | **32%** (6/19) | Refly, LlamaIndex, Haystack, ToolJet, Open-Higgsfield, opendataloader-pdf |
| **NOT-USED** | **11%** (2/19) | Dioxus (explicit NO), Huly (scaffolded as nom-collab, deferred) |

**Next-tick promotion target picked by agent (scored 24/25):** LlamaIndex REF → PATTERN. Reason: (1) unblocks semantic search layer used by compiler-bridge, (2) natural home in `nom-compose/dispatch.rs` via new `retriever.rs` + `postprocessor.rs` + `rag_backend.rs` (~150 LOC), (3) zero SKIP violations (no polars, no anthropic SDK, no comemo). Scaffold: `trait Retriever { retrieve(query, top_k) -> Vec<RetrievedDoc> }` + `Postprocessor enum { Rerank, Filter, Summarize, Fusion }` + `RagBackend impl CompositionBackend`. Exit to DEEP when RAG query <50ms on 1K-doc corpus + 5-entity coverage.

### New crate: `nom-collab` (wave-6)

- 8 modules, 33 tests: `DocId + DocSnapshot + TransactionLog + SyncMessage + Awareness GC + AuthClaims stub + PersistenceBackend + OfflineQueue`
- Still NOT-USED vendoring-wise (needs yrs/Hocuspocus-compatible impl; iter-15 classified as scaffolding-only). Would take Huly NOT-USED → PATTERN once a real CRDT impl lands.

### Per-doc updates this tick

- All 5 canonical doc headers refreshed to `HEAD: 2e47d5d` + `870 tests` + `12 crates` + standing (A) 0% / (B) 58% status lines
- state report iter-16 overlay (this section)
- implementation_plan.md phase line reflects wave-7 feature set
- INIT.md keeps user-facing A/B % bullets
- spec header moved DESIGN→IMPLEMENTATION date forward

---

## Iteration 15 audit — 2026-04-17 (HEAD `4592b85`, 6-agent parallel audit on compiler-as-core + 19-repo vendoring)

User question: "how does nom-compiler act as core role in nom-canvas, and what % of the 19 reference repos are actually vendored?"

### (A) Compiler-as-core integration: **0% runtime / ~7% stubbed**

**5 of 6 agents converged independently** on the same finding: `nom-compiler` and `nom-canvas` are **completely disjoint workspaces** with no dependency in either direction.

| Metric | Blueprint claim (§2/§3/§4/§7) | Reality @ HEAD `4592b85` |
|--------|-------------------------------|-------------------------|
| Cargo deps (compiler crates in nom-canvas crates' Cargo.toml) | "Direct function calls. Compiler crates linked as dependencies." | **0 of 15** compiler crates referenced |
| `use nom_*::` imports in nom-canvas/**/*.rs | ~15 expected | **0 matches** across 4000+ .rs files |
| Call sites (function invocations) | "Type char → stage1_tokenize, <10ms" etc. | **0 call sites** |
| Thread-tier dispatcher (UI / Interactive / Background) | §3 mandates 3-tier with channels | **0 of 3 tiers** implemented (TelemetryTier enum exists but is observability-only) |
| Dict connection pool (1 write + N read WAL) | §3 requirement | **0** — no rusqlite/nom-dict anywhere in nom-canvas |
| Tokio runtime for Interactive tier | §3 implicit | **0** — `LspProvider` trait is sync; no `#[tokio::main]` |
| Continuous-compile loop | §1 "runs continuously, rendering its own state" | **0** — frame_loop.rs is pure GPU, zero compiler calls |

**The 7 integration subclaims from blueprint §7 (compiler-as-core integration table):**

| Subclaim | Status |
|----------|--------|
| Type char → `stage1_tokenize` → highlighting | **NOT-STARTED** (`highlight.rs` ready but no producer) |
| Hover → `handle_hover` → tooltip | **STUBBED** (`LspProvider` trait defined, only `StubLspProvider` HashMap-backed exists) |
| Pause typing → `run_pipeline` S1-S6 | NOT-STARTED |
| Drag wire → `can_wire` → green/amber/red | NOT-STARTED |
| Click Run → LLVM compile + execute | NOT-STARTED |
| Command bar → `classify_with_react` | NOT-STARTED (command_palette.rs is fuzzy string matcher, no intent classification) |
| Dream dashboard → `dream_report` | NOT-STARTED |

**Evidence of honest stubbing (not divergent reimplementation):**
- `nom-editor/src/completion.rs:1-3` comment: *"MVP stub...real wiring lands with compiler integration"*
- `nom-editor/src/hints.rs:1-2` comment: *"actual LSP wiring lands with nom-compiler integration"*
- `nom-editor/src/lsp_bridge.rs:1-4` comment: *"does NOT depend on nom-lsp directly...trait the bridge implements"*
- `nom-collab/lib.rs:1-5` comment: *"Phase 5 scaffolding: No yrs dependency, no WebSocket server"*

The canvas crates are **structurally honest scaffolds** — they define the right trait boundaries (LspProvider, Highlighter, CompileStatus) for future wiring, but have zero production impl.

**Highest-leverage missing integration:** wire `nom-compiler` as a path dep in `nom-canvas/Cargo.toml` workspace (or cross-workspace path dep from each canvas crate) and call `nom_concept::stage1_tokenize` on keystroke in `nom-editor/src/highlight.rs`. That single wire unlocks syntax highlighting (most user-visible), validates cross-workspace linking, and establishes the channel pattern all other integrations reuse.

### (B) 19-repo vendoring assessment: **53% actively integrated**

| Tier | Count | % | Repos |
|------|-------|---|-------|
| **DEEP** (pattern ported with file:line evidence in code) | 6 | **32%** | Zed (scene/bounds_tree/element/styled), AFFiNE (73 tokens + Inter/SCP), yara-x (sealed trait in `nom-lint/rule_trait.rs`), typst (comemo pattern in `nom-memoize/`), WrenAI (MDL in `nom-compose/semantic.rs`), 9router (3-tier fallback in `nom-compose/provider_router.rs`) |
| **PATTERN** (architecture adapted, scaffolded) | 4 | **21%** | ComfyUI (Kahn sort in `nom-graph-v2/topology.rs`; IS_CHANGED deferred), waoowaoo (4-phase PhaseResult partially scaffolded), ArcReel (video scene graph + frame routing scaffolded), Remotion (GPU scene→FFmpeg architecture present in `video_composition.rs`) |
| **REFERENCED-ONLY** (named in docs, no code evidence) | 7 | **37%** | Refly, LlamaIndex, Haystack, ToolJet, n8n, Open-Higgsfield, opendataloader-pdf |
| **NOT-USED** (zero evidence) | 2 | **11%** | Dioxus (explicit NO — "Dioxus desktop = webview"), Huly (scaffolding-only, no yrs dep) |

**Vendoring = DEEP + PATTERN = 10 / 19 = 53%.**

Note: this is structural vendoring of design patterns. **Runtime integration with the compiler is separate** and is 0% (see section A). The 19-repo vendoring % applies to Phase 1-3 rendering + editor + block + theme layer, which has shipped successfully. The compiler-as-core integration is the unshipped axis.

### What should improve

**TIER-1 BLOCKING (nom-compiler integration, highest user impact):**
1. Add `nom-compiler/crates/*` as path deps in `nom-canvas/Cargo.toml` workspace — enables all subsequent wiring
2. Wire `nom_concept::stage1_tokenize` into `nom-editor/src/highlight.rs` (single channel producer, existing consumer)
3. Set up tokio Runtime + 3-tier dispatcher (UI sync / Interactive async / Background thread pool)
4. Open `nom-dict` SQLite connection pool (1 write + N read WAL)
5. Wire `nom_lsp::handle_hover/handle_completion` replacing `StubLspProvider`
6. Per blueprint §7 table — wire each of the 7 user-action-to-compiler-call rows one by one

**TIER-2 (finish 19-repo vendoring):**
7. 7 REFERENCED-ONLY repos either need pattern ports (if Phase 4/5 needs them) OR should be dropped from the "read end-to-end" list in INIT.md / blueprint §17 to match reality

**TIER-3 (housekeeping, from iter-14):**
- 8 open audit items (bytes_per_row alignment, FrameHandler wiring, pointer routing, GammaParams, SubpixelVariant 4×4, pub-use Renderer, line_layout.rs, 20-frame guard)

### Planning-lie disclosure

The iter-14 update I made to INIT.md said: *"11 crates shipped. 519 tests green. Phase 1 + Phase 2 100% + Phase 3 96%."* That was accurate for **render/edit/block layers**. It was **silent on compiler integration** — which is the central architectural claim of the blueprint. Readers would reasonably interpret "Phase 2/3 done" as "the compiler is wired." The honest status is: **rendering substrate complete + compiler integration entirely deferred**. Suggest adding a one-line compiler-integration-% to the canonical doc headers so this stays visible.

---

## Iteration 14 audit — 2026-04-17 (HEAD `4592b85`, 6-agent parallel audit)

Save point was `cb40522`. 8 commits landed, tests 113 → **519** (+406), 1 crate → **11 crates**.

### Phase completion status (ground truth from code scan)

| Phase | Status | Evidence |
|-------|--------|----------|
| Phase 1 (nom-gpui) | ✅ Enhanced | 21 modules, 170 tests; waves 2-3 + batch-3 shipped shadows/poly_sprites/subpixel_sprites pipelines + device_lost + animation + 8-pipeline renderer dispatch |
| Phase 2 (nom-canvas-core + nom-editor) | ✅ **100%** | 30/30 planned modules + 4 bonus · nom-canvas-core 62 tests, nom-editor 142 tests · All 15 canvas-core modules (element, mutation, shapes, hit_testing, spatial_index, viewport, coords, zoom, pan, fit, selection, marquee, transform_handles, snapping, history) + 15 editor modules |
| Phase 3 (nom-theme + nom-panels + nom-blocks) | ✅ **96%** | 25/26 planned modules + 4 bonus · nom-theme 41 tests (tokens, fonts, icons, +color, +mode) · nom-panels 29 tests (sidebar, toolbar, preview, library, properties, command_palette, statusbar, +mode_switcher) · nom-blocks 76 tests (all 7 block types + 5 shared-infra + flavour) |
| Phase 4 (nom-compose + nom-graph-v2) | 🟡 Scaffolded | nom-graph-v2 25 tests (topology, fingerprint, schema), nom-compose 7 tests (kind composition). Backends + MediaVendor + artifact_store + per-kind backends NOT yet audited this run |
| Phase 5 (nom-lint + nom-memoize + nom-telemetry + nom-collab) | 🟡 Partial | nom-lint 10 tests (landed), nom-memoize 0 tests, nom-telemetry 0 tests, nom-collab **NOT PRESENT** |

### Audit-backlog status (from iter-6/10/13 flagged items)

**3 of 13 fully closed · 2 partial · 8 still open:**

| # | Item | Status | Evidence |
|---|------|--------|----------|
| 1 | `order: u32` on GpuQuad Pod struct + WGSL | ❌ MISSING | renderer.rs:69, quad.wgsl — 6 fields, no order |
| 2 | `order`+`texture_sort_key`+`transformation` on GpuMonoSprite | ❌ MISSING | renderer.rs:91, mono_sprite.wgsl |
| 3 | 20-frame overflow guard in InstanceBuffer | ❌ MISSING | buffers.rs:116, no counter |
| 4 | `bytes_per_row` 256-align in wgpu_atlas | ❌ MISSING | wgpu_atlas.rs:287, raw multiplication |
| 5 | GammaParams uniform `@group(0) @binding(1)` | ❌ MISSING | pipelines.rs — zero matches |
| 6 | SubpixelSprite render pipeline gated on dual_source_blending | ✅ **PRESENT** | pipelines.rs:60 `Option<wgpu::RenderPipeline>`, line 345 gate |
| 7 | SubpixelVariant 4×4 (y:0..4, divisor 4.0) | 🟡 PARTIAL | text.rs:35 struct exists but y still 0..2, divisor 2.0 |
| 8 | `pub mod renderer` in lib.rs | 🟡 PARTIAL | lib.rs:32 module public but no `pub use Renderer` re-export |
| 9 | Concrete FrameHandler wiring Layout+Element+Renderer | ❌ MISSING | Only NoopHandler + WritingHandler (test stubs) |
| 10 | Pointer event routing CursorMoved → hit_test → ElementId | ❌ MISSING | frame_loop.rs handles only RedrawRequested/Resized/CloseRequested |
| 11 | BoundsTree-backed hit-test (not O(N)) | ✅ **PRESENT** | scene.rs:360 `hit_test` uses `bounds_tree` O(log N) with brute-force fallback |
| 12 | Pixel-readback test for drawn primitive | ✅ **PRESENT** | gpu_integration.rs — 5 tests: `single_quad_pixel_color_matches`, `two_quads_correct_z_order`, pixel_diff_* |
| 13 | BoundsTree proptest | ❌ MISSING | Zero proptest/quickcheck usage anywhere (open 10+ iterations) |

### Test quality (iter-14 vs iter-13)

- **Grade D+ → C+**. Pixel-readback gap CLOSED (was the biggest deficiency).
- Silent-skips dropped **70%** (23 → 7, all in wgpu_atlas.rs only).
- 6 `copy_texture_to_buffer` + `map_async` call sites now test actual pixel output.
- Still zero proptest/quickcheck after 13+ iterations.
- No scenario-level integration tests (no cross-crate end-to-end).
- 2 crates with 0 tests: nom-memoize, nom-telemetry.
- ~13 tautological "construct-and-drop" tests remain.

### Security posture — CLEAN

- `unsafe` block count: **1** (unchanged, pre-existing in `window.rs:89`, documented SAFETY)
- `unsafe_code = deny` enforced workspace-wide + 50+ file-level `#![deny(unsafe_code)]`
- New deps: **1** — `ropey 1.6` for nom-editor (well-maintained, 5M+ downloads, no CVEs)
- **Zero SKIP violations**: no `comemo`, no `polars`, no `anthropic-*`, no `openai-*`, no `Hocuspocus` — all plan-forbidden deps absent
- Zero secrets / credentials in source
- All `bytemuck::Pod` structs `#[repr(C)]` with no padding holes

### What should improve (prioritized fix list)

**HIGH (blocks real rendering):**
1. `bytes_per_row` 256-alignment in `wgpu_atlas.rs:287` — will trigger wgpu validation panic on first non-256-aligned glyph on Vulkan/DX12
2. Concrete FrameHandler impl wiring LayoutEngine + Element::paint + Renderer — without this, no Element will ever render on screen outside unit tests
3. Pointer event routing (CursorMoved → hit_test → ElementId dispatch) — canvas is non-interactive without this
4. GammaParams uniform — blocks correct subpixel text rendering

**MEDIUM:**
5. SubpixelVariant 4×4 (one-line fix: y:0..4 + divisor 4.0) — currently 4×2 diverges from blueprint §4
6. `pub use Renderer` re-export from nom-gpui (1-line fix) — makes Renderer ergonomic for downstream crates
7. `line_layout.rs` in nom-editor — only Phase 3 module missing (may have been merged into display_map; confirm + document or add)
8. 20-frame overflow guard for InstanceBuffer — silent VRAM exhaustion risk under unusual load

**LOW (cosmetic):**
9. `order: u32` on GPU Pod structs — CPU-side Scene::finish() painter's algo currently handles Z-order; GPU field only needed if depth-buffer-based sorting added
10. BoundsTree proptest — open 13+ iterations; audit hygiene item
11. Clean up 7 remaining silent-skip tests in wgpu_atlas.rs
12. Rewrite ~13 tautological tests to exercise invariants

### Doc drift fixed this iteration

- INIT.md: HEAD e2b7ecb → 4592b85, tests 31 → 519, 1 crate → 11 crates
- task.md: header refreshed, "starting now" phrasing removed
- nom_state_machine_report.md: HEAD refresh + per-crate test count
- implementation_plan.md: HEAD refresh + phase status line added
- Design spec: HEAD 6196ef1 → 4592b85, status DESIGN → IMPLEMENTATION

### Recommendation for Executor

Single fix commit can close 6 of 8 open HIGH/MED items in ~1 day:
- `pub use Renderer` in lib.rs (1 line)
- `bytes_per_row` 256-align in wgpu_atlas::flush_uploads (5 lines)
- SubpixelVariant y→0..4 + divisor 4.0 (2 lines)
- GammaParams uniform + BGL binding (~20 lines + WGSL)
- Concrete FrameHandler impl wiring Layout→Element→Renderer end-to-end (~80 lines)
- Pointer event routing via scene.hit_test (~30 lines)

Then wave-6 can focus on depth-of-impl for Phase 4/5 crates (nom-compose backends, nom-memoize, nom-telemetry, nom-collab).

---

**Iteration 13 delta:** Commit `cb40522` landed (batch-2 wave-3 — renderer + hit-test + integration, 99→113 tests). 6 parallel audit agents. Key findings:

**✅ Wave-3 shipped real code, not stubs:**
- `Renderer::draw()` ([renderer.rs:189-279](nom-canvas/crates/nom-gpui/src/renderer.rs#L189-L279)) iterates `scene.batches()` with proper bind groups (globals + instances), `begin_render_pass` with Clear→Store, `TriangleStrip` `pass.draw(0..4, 0..count)` per pipeline, `FrameGlobals` uniform written pre-frame. 3 of 4 primitive types wired (Quad, MonoSprite, Underline); SubpixelSprite + Shadow + Path + Poly are empty match arms (silent drop)
- `Scene::hit_test(point) -> Option<HitResult>` ([scene.rs:206](nom-canvas/crates/nom-gpui/src/scene.rs#L206)) works; 3 tests pass (basic hit, miss, topmost-by-order)
- Headless `gpu_integration.rs` test: full offscreen wgpu Device + RenderPass + `buffer.map_async` readback + pixel assertion on clear color
- Security posture preserved: **1 unsafe block unchanged**, 0 new deps, all Pod `#[repr(C)]` correct, device-lost properly propagated

**⚠️ iter-10 fix-wave: 0 of 7 items addressed. But severity re-classification warranted:**
- `order: u32` on Scene primitives EXISTS ([scene.rs:15](nom-canvas/crates/nom-gpui/src/scene.rs#L15)); `Scene::finish()` sorts by it; renderer paints in batch order = **painter's algorithm works**. Z-sort is NOT broken at runtime — CPU-side sort handles it. Missing `order` field on **GPU-side Pod structs** (QuadInstance WGSL, GpuQuad Rust) matters only if we add depth-buffer-based sorting later. **Re-classify iter-10 items 1-2 from CRITICAL → MEDIUM.**
- `bytes_per_row` alignment: agent-disagreement; Agent 1 says MISSING at [wgpu_atlas.rs:287, :311](nom-canvas/crates/nom-gpui/src/wgpu_atlas.rs#L287) (raw `width * bpp`, no 256-align); Agent 6 claimed LANDED but referenced `using_alignment(adapter.limits())` which is a Limits builder method, not bytes_per_row. **Ground truth: STILL MISSING**. First glyph with width non-multiple-of-256/bpp will trigger wgpu validation panic on Vulkan/DX12. Still HIGH.
- 20-frame overflow guard: MISSING. Silent VRAM exhaustion risk remains. Still MEDIUM (was CRITICAL; no evidence of exhaustion at test load).
- GammaParams: MISSING. Still HIGH — blocks subpixel text.
- SubpixelSprite pipeline: MISSING. Still MEDIUM — silent drop of subpixel sprites (empty match arm at [renderer.rs:272](nom-canvas/crates/nom-gpui/src/renderer.rs#L272))
- SubpixelVariant 4×4: MISSING. LOW — cosmetic AA quality.

**⚠️ Integration is "plumbed, not done":**
- `Renderer` struct is NOT `pub` in [lib.rs:21-39](nom-canvas/crates/nom-gpui/src/lib.rs#L21-L39) — external code can't construct it. Crate-private API.
- Only `FrameHandler` impl is `NoopHandler` (test-only) at [frame_loop.rs:274](nom-canvas/crates/nom-gpui/src/frame_loop.rs#L274). No `ExampleFrameHandler` / `CanvasFrameHandler` that actually calls Renderer
- `LayoutEngine::compute_layout` never called from frame path. Layout disconnected from render.
- `Element::paint` never invoked from frame path. Tests populate Scene manually via `insert_quad()`
- Hit-test uses O(N) brute-force, NOT BoundsTree (explicitly deferred). `HitResult` returns primitive index, NOT `ElementId`. No DrawOrder→ElementId reverse map. No pointer event routing (zero `CursorMoved`/`MouseInput` handlers). `ElementStateMap` scaffolding exists but not wired.
- No `examples/` dir. Can't produce on-screen pixel from scratch without writing custom FrameHandler that manually instantiates (currently private) Renderer.

**❌ Test quality: STILL D+ (unchanged since iter-6, 3 iterations flat)**
- +14 tests; ~3 meaningful (batch-iterator + texture-break). 3 tautological (`renderer_constructs`, `pipelines_construct_on_bgra_and_rgba`, size-assertion checks)
- Silent-skips WORSENED: 15+ → **23 total** (`let Some(...) = gpu_pair() else { return }` pattern), now in 6 files
- The 1 pixel-readback test only verifies clear color (empty scene), not drawn primitives
- Still NO: BoundsTree proptest, bytes_per_row alignment test, subpixel variant diff test, multi-frame test, pipeline selection test, DUAL_SOURCE_BLENDING fallback test

**Wave-4 priority list (ordered by blocking severity):**
1. **Make `Renderer` `pub`** (1-line fix in lib.rs) — or nothing outside the crate can render
2. **Fix `bytes_per_row` 256-alignment** in wgpu_atlas — will crash on real hardware
3. **Write concrete FrameHandler** that wires Layout+Element+Scene+Renderer end-to-end
4. **Wire winit pointer events → hit_test → ElementId dispatch**
5. **Add pixel-readback test for drawn primitive** (not just clear color)
6. **Add BoundsTree proptest** (iter-4 gap still open after 10 iterations)
7. Then GammaParams + SubpixelSprite pipeline before subpixel text renders

**Iteration 12 delta:** +0.5 pp (loop-edge). Added **cross-phase integration test matrix** — a dimension the plan didn't have (per-phase tests exist, but no end-to-end tests spanning phases). 10 integration scenarios defined:

| # | Scenario | Phases spanned | Key assertion |
|---|----------|----------------|---------------|
| I1 | User opens empty canvas → drags prose block → types → saves | 1→2→3 | Block persists to nom-dict; reloads identical |
| I2 | Compile .nomx file → emit error → error decoration appears at exact char offset | 1→2+compiler | Error span == tree-sitter token span |
| I3 | Drag connector between 2 graph nodes → wire scores green | 3+compiler | `nom-score::can_wire()` called sub-1ms |
| I4 | Write prose describing video → press "compose" → MP4 artifact appears in preview block | 3→4 | `~/.nom/store/<hash>/body.mp4` exists + plays |
| I5 | 2 clients edit same canvas via WebSocket → changes converge | 1→2→3→5-collab | CRDT property: both see identical Y.Doc state |
| I6 | Linter catches bug → Fix applied via keyboard shortcut | 2+5-lint | Diagnostic.fix.apply() produces expected text edit |
| I7 | Edit 1-char in large file → tree-sitter incremental re-parse only affected region | 2+5-memoize | Instrumentation counter: ≤ 1 layer re-parsed |
| I8 | Open 500-element canvas → zoom 0.1→10 pivot at center | 1→2 | 60fps maintained; zoom-to-point invariant holds |
| I9 | User prose "extract tables from PDF" → data block with 100-row table | 3→4 | opendataloader-pdf XY-Cut++ output matches golden |
| I10 | Run composition of 50 scenes → task queue shows progress → cancel at frame 25 | 3→4 | Frame 26+ not rendered; artifact_store has partial + cancel marker |

These are not yet in task.md — recording here as a planning artifact. Promoting to task.md contingent on wave-3 landing (integration tests depend on working render path).

**Iteration 11 delta:** 0 pp. No new commits (HEAD still `910f29a`). **Direct source-code verification** via grep confirms all 6 iter-10 defects remain in working tree:
- `order: u32` — zero hits in `nom-canvas/crates/nom-gpui/src/` for instance struct order field
- `overflow_frames` / `OVERFLOW_FRAMES` — zero hits (guard unimplemented)
- `bytes_per_row` alignment — only `align_up` helper in `buffers.rs`; `wgpu_atlas.rs` doesn't use it (still misaligned)
- `GammaParams` — zero hits (uniform absent)
- SubpixelSprite **pipeline** — zero hits for `subpixel_sprites` pipeline (the Scene Kind + BatchIterator exist but no `RenderPipeline` in pipelines.rs)
- SubpixelVariant 4×4 — `y\.0 / 4.0` pattern not found (still `y/2.0` = 4×2)

**Loop has reached its useful planning limit.** Plan is 100%. 6 critical defects block wave-3. Next productive action is Executor landing the 6-item fix commit per iter-10 architectural verdict (which already lists them actionably). Further 1-minute planning iterations at this point generate no new signal; recommend either (a) pause cron until Executor advances, or (b) shift loop goal from "expand plan" to "audit after each commit" (cron wakes, sees no commit, exits in <5s; wakes on commit, dispatches 6 audit agents).

**Iteration 10 delta:** Commit `910f29a` landed (batch-2 wave-2 — atlas + text + window + frame_loop, 59→99 tests). 6 parallel agents audited (5 full reports, 1 API-overloaded). **Mixed outcome:**

- ✅ **text.rs, window.rs, frame_loop.rs, wgpu_atlas.rs STRUCTURE** all MATCH iter-5 spec (cosmic-text ShapeLine+Shaping::Advanced, swash ScaleContext+Render::new, Bgra8→Rgba8 fallback, Fifo, desired_maximum_frame_latency:2, BucketedAtlasAllocator per slab, parking_lot Mutex Arc-wrapped). ApplicationHandler trait, no thread::sleep, device-loss recovery wired.
- ✅ **Security posture preserved** — 1 pre-existing unsafe in window.rs:89 (documented SAFETY), 0 new unsafe, all Pod `#[repr(C)]`, font loading via system DB only, resize clamped, zero foreign identifiers in public API, zero wrappers (GpuAtlas + WindowSurface + App add real logic).
- ❌ **iter-6 fix-wave STILL 3/4 UNRESOLVED (5 loop iterations stalled)**: `QuadInstance` + `MonoSpriteInstance` still have NO `order: u32` (Z-sort broken in rendering), `MonoSpriteInstance` missing `transformation` (rotated glyphs impossible), 20-frame overflow guard STILL unimplemented (silent VRAM exhaustion). `pub mod context;` was already fixed in wave-1 `205aea9`.
- ❌ **NEW wave-2 HIGH defects**: (a) `bytes_per_row` NOT aligned to `COPY_BYTES_PER_ROW_ALIGNMENT=256` in [wgpu_atlas.rs:309-311](nom-canvas/crates/nom-gpui/src/wgpu_atlas.rs#L309-L311) — wgpu validation error on first non-256-aligned glyph; (b) `GammaParams` STILL missing from globals BGL ([pipelines.rs:72-84](nom-canvas/crates/nom-gpui/src/pipelines.rs#L72-L84)) — subpixel text will render with wrong gamma; (c) SubpixelSprite render pipeline ABSENT ([renderer.rs:259-260](nom-canvas/crates/nom-gpui/src/renderer.rs#L259-L260) logs "batch skipped (batch-3)") — subpixel glyphs silently dropped; (d) 4×4 subpixel variants reduced to 4×2 ([text.rs:33-38](nom-canvas/crates/nom-gpui/src/text.rs#L33-L38)) — divergence from blueprint §4.
- **Test quality: D+ (unchanged from iter-6)**. +40 tests dominated by happy-path + silent-skip GPU tests. iter-4 gaps: 4 of 5 filled. New gaps: bytes_per_row alignment, subpixel differentiation, window lifecycle, redraw-request dispatch, DUAL_SOURCE_BLENDING fallback. 3 tautological tests flagged.
- **Architectural verdict**: wave-3 CAN proceed at architecture level, but Executor MUST land a single fix commit with: `order` fields + `transformation` + 20-frame guard + bytes_per_row alignment + GammaParams + SubpixelSprite pipeline + 4×4 variants — otherwise first real text rendering will panic (bytes_per_row) or render visibly wrong (Z-sort + gamma + missing subpixel path).

**Iteration 9 delta:** +4 pp (95→99% overall). Read blueprint §12-15; backfilled 5 missing Phase 4 backends (media/storyboard, media/novel→video, media/audio, data/transform, media/3D) + MediaVendor trait + artifact_store + 6 compose-output preview blocks + WrenAI 5-stage pipeline. 2 parallel agents read waoowaoo+ArcReel + Polars.

**Iteration 8 delta:** +5 pp (82→95%). Blueprint §8-11 re-read surfaced 6 missed modules (animation/input/completion/properties/fonts/icons). 3 agents read yara-x + Huly + typst-comemo + OpenTelemetry. Phase 5 decomposed 15→90% across 5 new crates (nom-lint, nom-collab, nom-memoize, nom-telemetry, file-watcher).

**Iteration 7 delta:** +8 pp. 3 agents ComfyUI + n8n + typst/Remotion. Phase 4 20→90%.

**Iteration 6 delta:** 0 pp. Commit `205aea9` (wave-1 59/59 tests) audited; 4 CRITICAL + 7 HIGH flagged.

**Iteration 1 delta:** +5 pp (Phase 1 batch-2 decomposed to 12 tasks with Zed citations)
**Iteration 2 delta:** +7 pp — Commit `e2b7ecb` landed; 6 agents found 3 CRITICAL + 4 HIGH + 10 MED + 6 LOW; archive clean, 0 unsafe/CVEs
**Iteration 3 delta:** +13 pp — 3 agents decomposed Phase 2 into 29 tasks from Excalidraw/Zed-editor/AFFiNE-GFX end-to-end reads
**Iteration 4 delta:** +6 pp — Commit `1daa80e` (Executor audit-fix, 31→44 tests). 6 parallel agents verified **all 3 CRITICAL + 4 HIGH landed cleanly**. 1 bonus MED (`max_leaf` fast-path) opportunistically done. Security posture preserved. Test quality grade **C+**: strong on z-interleave + bounds overlap + styled borrow-lifecycle; weak on sprite ABA, HSL boundaries, LayoutError path. One tautological test flagged for trybuild rewrite. Two intentional Zed-divergences noted (Styled `&mut self`, Hsla `[0,360)`) — functional, documented, not regressions. **Architectural verdict: batch-2 GPU work can proceed.**
**Iteration 5 delta:** +11 pp — 3 parallel Explore agents deep-read: (1) Zed `shaders.wgsl` 1335-line single file → all WGSL skeletons + helper functions (quad_sdf_impl, gaussian, erf, oklab, enhance_contrast) extracted verbatim; (2) Zed `wgpu_renderer.rs` + `wgpu_context.rs` → 4 bind-group-layout patterns + surface config + DUAL_SOURCE_BLENDING optional feature + MSAA adapter gating; (3) AFFiNE blocks/ → schema-first `defineBlockSchema()` pattern, universal block model (id+flavour+props+children), 7-Nom-block mapping to AFFiNE refs, Lit/CSS/floating-ui flagged as SKIP. Phase 1 batch-2 bumped 85→95% with 3-wave decomposition (shaders+buffers+ctx / atlas+text+window / full 8 pipelines). Phase 3 bumped 25→85% with ~45 subtasks covering shared infra + 7 block types × 4 aspects (schema/render/events/transformer) + nom-panels + nom-theme + 8 test targets.

**New standing rule (2026-04-17):** Every Planner/Auditor iteration MUST read the blueprint `docs/superpowers/specs/2026-04-17-nomcanvas-gpui-design.md` FIRST — before git log, before agents, before commits. Added to `nom-planner-auditor` skill as NON-NEGOTIABLE section, saved as persistent memory `feedback_always_read_blueprint.md`.

**Iteration 10 delta:** +1 pp (99→**100%**). Final blueprint pass: §15-18 read; backfilled `FallbackStrategy` 3-variant enum (Fallback/RoundRobin/FillFirst), `nom-collab::transactor` (missing 4th field in `CollaborationEngine` per §16 — immutable event log separate from state snapshots), and Remotion-pattern concrete `VideoComposition` / `SceneEntry` struct from §18 with content-addressing advantage over Remotion (per-scene render cache via artifact_store hash). **Plan is complete.** Every section of the 719-line blueprint is decomposed into actionable subtasks with file:line citations across 5 phases (Phase 1 batch-1+2 GPU framework, Phase 2 canvas+editor, Phase 3 blocks+panels, Phase 4 universal composition with 12 backends, Phase 5 production quality with 5 new crates).

**⚠️ EXECUTION STALL (5 iterations and counting):** iter-6 fix-wave at wave-1 commit `205aea9` remains unlanded. Blocks wave-2 (atlas+text+window) regardless of how detailed the plan gets. The 4 CRITICAL defects (missing `order` field on both instance structs, missing `transformation` on sprite, unimplemented 20-frame overflow guard, dead-code `context.rs` not re-exported in lib.rs) must ship before wave-2 can begin. Cron loop's marginal value is now near-zero until Executor advances — future iterations should be audit-focused on whatever the Executor finally commits, not further expansion. Recommend either (a) pause cron and let Executor catch up, or (b) keep cron running but expect 0-pp iterations until a new commit lands.

**Iteration 9 delta:** +4 pp (95→99% overall). Read blueprint §12-15 first (standing rule); surfaced 5 backend gaps in iter-7 decomposition: media/storyboard (waoowaoo 4-phase), media/novel→video (ArcReel agent workflow), media/audio (synthesis+codec), data/transform (Polars MVP new `nom-data` crate), media/3D (glTF). Also surfaced `MediaVendor` trait + `artifact_store` + 6 compose-output preview blocks + WrenAI 5-stage pipeline detail. 2 parallel Explore agents read waoowaoo+ArcReel (combined) and Polars. **Key zero-foreign-identities adaptation**: ArcReel uses Claude SDK directly; Nom MUST NOT — instead use `nom-intent` ReAct agents + `MediaVendor` facades with format translation at boundary. Phase 4 100%. **iter-6 fix-wave STILL PENDING EXECUTOR** (4 iterations stalled: missing `order` field, missing `transformation`, missing 20-frame overflow guard, dead-code `context.rs`). This blocks wave-2 regardless of how detailed the plan gets — planning has now meaningfully exceeded execution and the loop's marginal value is diminishing until Executor advances.

**Iteration 8 delta:** +5 pp (82→95% overall) — No new commits (HEAD still `205aea9`; iter-6 fix-wave pending). Re-read blueprint §8 + §9 + §10 + §11 first (standing rule); surfaced 6 module gaps in my earlier decompositions: `nom-gpui/animation.rs`, `nom-editor/{input,completion}.rs`, `nom-panels/properties.rs`, `nom-theme/{fonts,icons}.rs` — added as Phase 1/2/3 addenda. Then 3 parallel Explore agents deep-read: (1) yara-x → sealed trait via supertrait binding, runtime `Vec<Box<dyn Rule>>` registration, byte-offset `Span`, Fix struct for auto-fix; (2) Huly → Hocuspocus+Yjs architecture, ~22-service inventory with 4-service minimal-collab core (collaborator + presence + account + server); (3) typst comemo + OpenTelemetry Rust SDK → `Tracked<T>`+`Constraint::validate()` reference-equality memoization, W3C traceparent + ParentBased(TraceIdRatioBased(0.01)) sampling. Phase 5 decomposed 15→90% with ~35 subtasks across 5 new crates (nom-lint + nom-collab + nom-memoize + nom-telemetry + file-watcher) including port-comemo-without-the-crate-dep + reimplement-Hocuspocus-in-Rust as explicit SKIP-dependencies directives.

**Iteration 7 delta:** +8 pp — No new commits (HEAD still `205aea9`; iter-6 fix-wave pending). 3 parallel Explore agents deep-read: (1) ComfyUI execution.py + graph.py + caching.py → Kahn topo-sort with lazy cycle detection + IS_CHANGED contract + 4 cache strategies (None/Lru/RamPressure/Classic) + hierarchical subcache + cooperative cancellation; (2) n8n workflow-execute.ts + expression-sandboxing.ts → pull-based stack exec + retry/continueOnFail + isolated-vm sandbox with ThisSanitizer + PrototypeSanitizer + DollarSignValidator (critical for `.nom` script safety, all 3 ported verbatim); (3) typst + Remotion → Tracked<dyn World> memoization + Frame/FrameItem Arc<LazyHash> + rayon-parallel layout + GPU scene→frame→FFmpeg pipe pattern. Phase 4 decomposed 20→90% (~60 subtasks across nom-graph-v2 + nom-compose + 7 backends + shared AST sandbox + 14 test targets including security-focused sandbox-escape tests). Plan overall: 82→90%.

**Iteration 6 delta:** 0 pp (held at 82%) — Commit `205aea9` (Executor batch-2 wave-1, 44→59 tests). 6 parallel agents audited against iter-5 spec + blueprint. **4 CRITICAL defects block wave-2 start**: (1) `QuadInstance`+`MonoSpriteInstance` missing `order` field (breaks Z-sorted rendering); (2) `MonoSpriteInstance` missing `transformation` field (no rotated glyphs); (3) 20-frame overflow guard completely UNIMPLEMENTED (spec-mandated safety); (4) `context.rs` dead code — `pub mod context;` missing from lib.rs. 7 HIGH items: 4-file shader split vs 1-file spec, missing GammaParams binding, `recover()` Arc-staleness, min_binding_size None, FRAGMENT-only texture visibility, missing hsla_to_rgba, clip_bounds vs content_mask naming. **Security + blueprint conformance: CLEAN** (0 unsafe, 0 wrappers, 0 foreign identifiers, thread-model + wasm + compiler-linkage all READY). **Test quality grade: D+** — strong on pure-math buffer helpers, zero coverage on shader compat / SDF boundaries / NDC / unit-quad / 20-frame guard; 1 tautological test; 3 silent-skip tests. Plan stays at 82% because wave-1 didn't ADD plan detail — it generated a fix-wave requirement list that the Executor must address before wave-2 can start.
