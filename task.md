# Nom — Task Execution Checklist

**Date:** 2026-04-18 | **HEAD:** `f0ca908` | **Tests:** 581 | **Workspace:** clean

## Wave P (2026-04-18 Iter 45) — E2+11 fixes: paint bodies + all HIGHs + MEDIUMs
- [x] E2 CRITICAL: GraphNodeElement::paint() + WireElement::paint() push real Quads (5 body+port, 6 wire segments)
- [x] TQ1: task_queue complete() guards state==Running before transition
- [x] CK1: LruCache touch() on get() via Mutex interior mutability
- [x] DP1: Backend trait + BackendRegistry dispatch + NoopBackend stub
- [x] F1: find_replace.rs use_regex → real regex::Regex + find_iter match ranges
- [x] NR1: duplicate NomtuRef removed from graph_mode.rs; re-export canonical nom-blocks::NomtuRef
- [x] NI1: nom-intent expanded 98→240 LOC: ScoredHypothesis + InterruptSignal + rank_hypotheses + react_chain_interruptible
- [x] DEEP1: DeepThinkPanel ingest_events/consume_stream wired; paint_scene emits per-card Quads
- [x] FG1: FrostedRect primitive in nom-gpui::Scene; dock pushes frosted overlay (FROSTED_* tokens)
- [x] FR1: focus ring is 2px border-only Quad via focus_ring_quad() (3 sites: dock, file_tree, quick_search)
- [x] PAL1: NodePalette + PaletteEntry added to nom-panels/left — DB-driven load/search/paint
- [x] MEDIUMs: RRF_K=60.0 const; InternalRule 3rd trait; HierarchicalCache::len sums L1+L2; Panel trait 7 methods

**Wave Q committed f0ca908 (581 tests). Remaining open (WAVE R targets):**
- NI1-REAPPLY — nom-intent ScoredHypothesis/InterruptSignal/rank_hypotheses lost from Wave P commit; re-land (5→10 tests)
- SH1 — nom-memoize hash.rs still uses FNV-1a; TODO says replace with SipHash13
- Coverage: nom-telemetry, nom-collab, nom-cli, nom-lint all under 15 tests — need expansion
- Backend completeness: verify all 16 nom-compose backends have domain models + tests

## Wave N (2026-04-18 Iter 43) — router infra + sandbox + SHA-256 + semantic MDL
- [x] nom-compose vendor_trait.rs: MediaVendor + CostEstimate + StubVendor
- [x] nom-compose provider_router.rs: FallbackLevel 3-tier + retry_delay_ms (1000×2^level, max 120s)
- [x] nom-compose credential_store.rs: kind-keyed secret storage
- [x] nom-graph sandbox.rs: 4 AST sanitizers (DepthLimit + AllowedFunctions + NoSideEffects + TypeCoherence) + eval_expr
- [x] nom-compose store.rs: ContentHash now uses SHA-256 via sha2 (spec §14 compliance)
- [x] nom-compose semantic.rs: WrenAI MDL semantic layer (SemanticModel + SemanticRegistry + SQL generation)

**Sibling docs:** `implementation_plan.md` · `nom_state_machine_report.md` · `docs/superpowers/specs/2026-04-17-nomcanvas-gpui-design.md` · `INIT.md`

## Current Status

| Wave | Done | Evidence |
|------|------|---------|
| [x] 0 Bootstrap | ✅ | 14 crates, workspace clean |
| [x] A GPUI substrate | ✅ | nom-gpui 2938 + nom-canvas-core 2183 + nom-theme 1211 LOC |
| [x] B Editor + Blocks | ✅ | nom-blocks 1313 + nom-editor 1269 LOC |
| [x] C Compiler bridge | ✅ | 3-tier (ui/interactive/background), `--features compiler` clean |
| [x] D Shell | ✅ | nom-panels 1522 LOC, 9 files paint_scene → Quad |
| [x] E Compose backends | ✅ | 16 backends + dispatch + plan + task_queue |
| [x] F RAG + Deep-think | ✅ | classify_with_react + RRF 1/(60+rank) |
| [x] G Stubs populated | ✅ | nom-lint + collab + telemetry |
| [x] K (4 CRITICALs) | ✅ | dc6a025 — U1/W1/COL1/INT1 closed |
| [x] L (MEDIUM) | ✅ | d139644 — deep_think + W3C + RRF |
| [x] M (Infra) | ⚠️ DRIFT | ef9fc84 — 4 claims DRIFT, 1 HIGH bug (TQ1) |
| [x] N Infra+Vendor | ✅ | d6219b1 — vendor/router/cred/sandbox/SHA-256/semantic (523 tests) |
| [x] O Infra+LSP | ✅ | e61a93c — cancel/cache/LSP/sandbox/web_screen (537 tests) |
| [x] P Bug fixes | ✅ | 15a8366 — E2+11 fixes+MEDIUMs (558 tests) |
| [x] Q Quality | ✅ | f0ca908 — SB1+SC1+CW1+DOC1+CB1+E1+rag-confidence (581 tests) |

### Integrity Grep

| Check | Count | Expected |
|---|---|---|
| `use nom_gpui::scene` | 11 | ≥1 |
| `nom_intent` in deep_think.rs | 3 | ≥1 |
| `RgaPos`/`tombstoned` in nom-collab | 28 | ≥1 |
| `RenderPrimitive` custom enum | 0 | 0 |

## Open Missions (Wave R targets)

### HIGH

- [ ] **NI1-REAPPLY** — nom-intent ScoredHypothesis/InterruptSignal/rank_hypotheses lost from Wave P commit; re-land (5→10 tests)

### MEDIUM

- [ ] **SH1** — nom-memoize hash.rs still uses FNV-1a; TODO says replace with SipHash13
- [ ] **COV-TELEMETRY** — nom-telemetry under 15 tests; expand coverage
- [ ] **COV-COLLAB** — nom-collab under 15 tests; expand coverage
- [ ] **COV-CLI** — nom-cli under 15 tests; expand coverage
- [ ] **COV-LINT** — nom-lint under 15 tests; expand coverage
- [ ] **BE-AUDIT** — verify all 16 nom-compose backends have domain models + tests

## Non-Negotiable Rules

1. Read source repos end-to-end before any code borrowing the pattern
2. Always use `ui-ux-pro-max` skill for UI work
3. Zero foreign identities in public API
4. nom-compiler is CORE — direct workspace path deps, zero IPC
5. DB IS the workflow engine — no external orchestrator
6. Every canvas object = DB entry — `entity: NomtuRef` non-optional
7. Canvas = AFFiNE-for-RAG (frosted glass + confidence edges)
8. Doc mode = Zed + Rowboat + AFFiNE
9. Deep thinking = compiler op streamed to right dock
10. GPUI fully Rust — one binary, no webview
11. Spawn parallel subagents for multi-file work
12. Run `gitnexus_impact` before editing any symbol

## Wave Mission Archive

Detail checklists collapsed — retrieval via git log of canonical commits.

### [x] Wave 0 — Bootstrap
- [x] Cargo workspace + 14 crate stubs + rust-toolchain
- [x] `unsafe_code = deny` workspace lint
- [x] cross-workspace path deps to `../../../nom-compiler/crates/*` (feature-gated)

### [x] Wave A — GPUI Substrate (commit `8c7d32e`)
- [x] nom-gpui scene graph (Quad/Sprite/Path/Shadow/Underline) + 8 wgpu pipelines
- [x] cosmic-text + etagere glyph atlas
- [x] Element trait + Styled builder + winit + taffy + animation
- [x] nom-canvas-core viewport + hit-test + rubber-band + R-tree
- [x] nom-theme 73 AFFiNE tokens + Inter + Source Code Pro + 42 Lucide icons

### [x] Wave B — Editor + Blocks (commit `8c7d32e`)
- [x] nom-editor rope + multi-cursor + display/wrap/tab/line maps
- [x] Highlighter::color_runs consumer + LspProvider trait
- [x] nom-blocks with `NomtuRef` non-optional on every block
- [x] AFFiNE block types (heading/para/list/quote/divider/callout/code/db/linked)
- [x] Graph node ports derived from `clause_shapes` (DB-driven)
- [x] DictReader trait injection (no direct `Connection::open` in nom-blocks)
- [x] can_wire placeholder

### [x] Wave C — Compiler Bridge KEYSTONE (commit `fb66e01`, 17/17)
- [x] shared.rs Arc<RwLock<SharedState>> + dict_pool + grammar + LRU
- [x] ui_tier.rs sync: lookup_nomtu, score_atom, can_wire, grammar_keywords, search_bm25
- [x] interactive_tier.rs tokio mpsc: tokenize, highlight_spans, complete_prefix, hover
- [x] background_tier.rs crossbeam: compile, plan_flow, verify, deep_think
- [x] adapters: highlight (stage1 → color_runs) · lsp · completion · score
- [x] First wire: `.nomx` → stage1_tokenize → Highlighter live

### [x] Wave D — Shell (20/20 tests)
- [x] dock.rs (DockPosition/Dock/Panel) · pane.rs (PaneGroup split) · shell.rs
- [x] Left dock: CollapsibleSection + QuickSearchInput + ResizePanel + dict tree
- [x] Right dock: ChatSidebar + tool cards + deep-think stream + multi-agent
- [x] Bottom dock (terminal + diagnostics)
- [x] WindowPool multi-window + Cmd+K palette

### [x] Wave E — 16 Compose Backends (commit `a1ba5a1`, 26/26)
- [x] E-1 document (typst-memoize) · E-2 video (Remotion) · E-3 data_extract (XY-Cut++)
- [x] E-4 web_screen (ToolJet) · E-5 workflow (n8n + AST sandbox)
- [x] E-6 data_query (WrenAI) · E-7 image · E-8 storyboard · E-9 audio
- [x] E-10..16 data_frame/native/mobile/presentation/app_bundle/ad_creative/mesh
- [x] ArtifactStore + ProgressSink + InterruptFlag + MediaVendor dispatch

### [x] Wave F — Graph RAG + Deep Think (commit `be3b9a8`)
- [x] graph_rag.rs (retriever + BFS + cosine + RRF 1/(60+rank))
- [x] graph_mode.rs (force-directed + confidence edges + animate_to_layout)
- [x] deep_think.rs (stream + step chain + token budget + real ReAct)

### [x] Wave G — Stubs populated (commit `546e02d`)
- [x] nom-lint sealed trait (yara-x pattern)
- [x] nom-collab RGA CRDT (RgaPos anchor + tombstone + convergence)
- [x] nom-telemetry W3C traceparent

### [x] Wave K — 4 CRITICALs (commit `dc6a025`, 457 tests)
- [x] U1 — nom-panels paint_scene uses nom_gpui::scene::Quad + nom_theme::tokens (10 files)
- [x] W1 — deep_think imports `use nom_intent::{classify_with_react, react_chain}`
- [x] COL1 — RGA CRDT with RgaPos::{Head, After(OpId)} + tombstoned: bool
- [x] INT1 — 11+ cross-crate `use nom_gpui::scene` imports

### [x] Wave L — MEDIUMs (commit `d139644`, 504 tests)
- [x] with_deep_think wired end-to-end · W3C traceparent format · RRF 1/(60+rank)
- [x] impl Element for GraphNodeElement/WireElement (trait only — bodies empty → E2)

### [x] Wave M — Infra (commit `ef9fc84`, 498 tests)
- [x] nom-lint sealed (2-trait; MEDIUM: add 3rd InternalRule)
- [x] compiler-bridge 3-tier consolidated
- [x] compose dispatch + plan + task_queue (⚠️ TQ1 bug + DP1 drift)
- [x] graph 4-tier cache (⚠️ CK1 LruCache.get broken)

### [x] Wave N — Infra+Vendor (commit `d6219b1`, 523 tests)
- [x] nom-compose vendor_trait.rs: MediaVendor + CostEstimate + StubVendor
- [x] nom-compose provider_router.rs: FallbackLevel 3-tier + retry_delay_ms
- [x] nom-compose credential_store.rs: kind-keyed secret storage
- [x] nom-graph sandbox.rs: 4 AST sanitizers + eval_expr
- [x] nom-compose store.rs: SHA-256 ContentHash via sha2
- [x] nom-compose semantic.rs: WrenAI MDL SemanticModel + SemanticRegistry

### [x] Wave O — Infra+LSP (commit `e61a93c`, 537 tests)
- [x] nom-compiler-bridge: CompilerLspProvider (real completions/diagnostics from nom-compiler)
- [x] nom-compose backends: code_exec (n8n JsTaskRunner + sandbox wiring), web_screen (headless browser stub)
- [x] nom-graph: ExecutionEngine cancel/abort signal
- [x] nom-graph: HierarchicalCache L1+L2 promotion wiring in ExecutionEngine
- [x] Workspace-wide: 537 tests verified across all 14 canvas crates

### [x] Wave P — Bug fixes + MEDIUMs (commit `15a8366`, 558 tests)
- [x] E2 CRITICAL: GraphNodeElement::paint() + WireElement::paint() push real Quads (5 body+port, 6 wire segments)
- [x] TQ1: task_queue complete() guards state==Running before transition
- [x] CK1: LruCache touch() on get() via Mutex interior mutability
- [x] DP1: Backend trait + BackendRegistry dispatch + NoopBackend stub
- [x] F1: find_replace.rs use_regex → real regex::Regex + find_iter match ranges
- [x] NR1: duplicate NomtuRef removed from graph_mode.rs; re-export canonical nom-blocks::NomtuRef
- [x] NI1: nom-intent expanded 98→240 LOC: ScoredHypothesis + InterruptSignal + rank_hypotheses + react_chain_interruptible
- [x] DEEP1: DeepThinkPanel ingest_events/consume_stream wired; paint_scene emits per-card Quads
- [x] FG1: FrostedRect primitive in nom-gpui::Scene; dock pushes frosted overlay (FROSTED_* tokens)
- [x] FR1: focus ring is 2px border-only Quad via focus_ring_quad() (3 sites: dock, file_tree, quick_search)
- [x] PAL1: NodePalette + PaletteEntry added to nom-panels/left — DB-driven load/search/paint
- [x] MEDIUMs: RRF_K=60.0 const; InternalRule 3rd trait; HierarchicalCache::len sums L1+L2; Panel trait 7 methods

### [x] Wave Q — Quality (commit `f0ca908`, 581 tests)
- [x] SB1: nom-graph/src/sandbox.rs — added this_replace, prototype_block, dollar_validate sanitizers
- [x] SC1: score_atom in ui_tier.rs:167 — eliminated per-call UiTier/SharedState allocation
- [x] CW1: nom-blocks/src/connector.rs:62 can_wire — grammar-backed validation wired
- [x] DOC1: ui_tier.rs:40 docstring corrected to `<1ms` (spec §3 compliance)
- [x] CB1: compiler-bridge score.rs adapter dead-code stubs replaced with real implementation
- [x] E1: panels paint_scene divergence from impl Element documented; trait bindings added
- [x] rag-confidence: graph_rag edge-confidence weights (Refly-pattern per-edge scoring)

## Compiler Parallel Track (nom-compiler — UNCHANGED as infra)

- [x] GAP-1c body_bytes · GAP-2 embeddings · GAP-3 corpus ingest
- [x] GAP-4 nom-intent 9router pattern · GAP-5 deep_think backing op
- [ ] Bootstrap fixpoint proof (Wave future)

## History

Iter log in `nom_state_machine_report.md`. Key pivots:
- Iter 30 HARD FREEZE — Executor added surface without closing blockers; lifted Iter 31
- Iter 36-41 — 9 consecutive iters U1 open; Iter 40 single-commit mandate issued
- Iter 42-43 — single-commit landed; Wave K closed 4 CRITICALs
- Iter 45 — Wave L; impl Element claim found SHAPE-ONLY
- Iter 46 — Wave M; 4 DRIFT + TQ1 correctness bug
- Iter 47 — whole-repo scan found E2 CRITICAL (paint body no-op)
- Iter 44 — Wave O closed: CompilerLspProvider + cancel + cache-promotion + sandbox wiring + web_screen (537 tests)
- Iter 45 — Wave P closed: E2 CRITICAL + 10 HIGH/MEDIUM + 1 CRITICAL + MEDIUMs (558 tests, commit 15a8366)
- Iter 46 — Wave Q closed: SB1+SC1+CW1+DOC1+CB1+E1+rag-confidence (581 tests, commit f0ca908)
