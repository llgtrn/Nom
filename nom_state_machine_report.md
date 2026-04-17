# Nom State Machine Report

**Date:** 2026-04-18 | **HEAD:** `e61a93c` | **Tests:** 537 | **Workspace:** clean
**Detailed commit history:** `git log --oneline`. This file keeps only the latest state + open missions.

## Current State

- [x] nom-compiler (29 crates) UNCHANGED infra with nomdict.db
- [x] nom-canvas (14 crates) rebuilt fresh, cross-workspace path deps feature-gated
- [x] 537 tests passing across workspace
- [x] All 4 Wave K CRITICALs closed (U1 paint_scene + W1 real ReAct + COL1 RGA + INT1 imports)
- [x] All 4 Wave L MEDIUMs claimed closed (deep_think config + W3C + RRF + impl Element)
- [x] Wave M infra landed (sealed + 3-tier + 4-tier cache + dispatch/plan/task_queue)
- [x] Wave N landed (vendor/provider/cred infra + sandbox 4-sanitizers + SHA-256 store + WrenAI MDL semantic layer)
- [x] Wave O landed (CompilerLspProvider + cancel + cache-promotion + sandbox wiring + web_screen — 537 tests)
- [ ] **E2 CRITICAL OPEN** — impl Element bodies empty (see Iter 47 below)

## Open Missions

- [ ] E2 CRITICAL — `nom-canvas-core/elements.rs:225-226` paint() = `let _ = bounds;` (no draw)
- [ ] TQ1 HIGH — task_queue.rs:48 complete() missing Running guard
- [ ] E1 HIGH — no `impl Element for` on panels (paint_scene divergence undocumented)
- [ ] F1 HIGH — find_replace.rs regex stub uses substring
- [ ] CK1 HIGH — cache.rs:66 LruCache.get() missing touch()
- [ ] DP1 HIGH — dispatch.rs enum-lookup, no `Box<dyn Backend>`
- [ ] DEEP1 HIGH — no nom-panels consumer for deep_think stream
- [ ] NI1 HIGH — nom-intent skeletal at 98 LOC
- [ ] 6 MEDIUMs — RRF_K const + nom-lint 3rd trait + HierarchicalCache::len + UiTierOps alloc + ui_tier timing doc + Panel trait 5/7 methods

## Iteration 44 — Wave O committed (2026-04-18, commit `e61a93c`)

- Committed: CompilerLspProvider (nom-compiler-bridge) + ExecutionEngine cancel/abort signal + HierarchicalCache L1→L2 promotion wiring (nom-graph) + code_exec backend (n8n JsTaskRunner + sandbox wiring) + web_screen backend (headless browser stub) (nom-compose)
- Tests: 537 (+14 vs Wave N baseline of 523)
- New capabilities: real LSP completions/diagnostics from nom-compiler, cancellable execution, cache tier promotion, n8n sandbox wiring, headless web screen stub
- Open: E2 CRITICAL (paint no-op) + TQ1/CK1/DP1/F1/NR1/NI1/DEEP1/FG1 HIGH; FR1/PAL1 MEDIUM + 4 MEDIUMs remain (Wave P)

## Iteration 50 — Wave N committed (2026-04-18, commit `d6219b1`)

- Committed: vendor_trait.rs + provider_router.rs + credential_store.rs (nom-compose) + sandbox.rs (nom-graph) + store.rs SHA-256 + semantic.rs (nom-compose) + sha2 dep
- Tests: 523 (+25 vs Wave M baseline of 498)
- New modules: MediaVendor trait, ProviderRouter 3-tier fallback, CredentialStore kind-keyed, 4 AST sanitizers + eval_expr, SHA-256 ContentHash, SemanticModel/SemanticRegistry MDL
- Open: E2 CRITICAL (paint no-op) + 9 HIGH + 8 MEDIUM remain

## Iteration 49 — Working-tree scan (2026-04-18)

**HEAD:** `fb886fa` unchanged. Executor has **uncommitted** changes in nom-canvas.

**Progress (uncommitted):**
- [~] M1 drift closing — `nom-compose/src/store.rs` replaces FNV-1a 32-byte expansion with real SHA-256 via `sha2 = "0.10"`. `ContentHash::from_bytes` now calls `Sha256::new().update().finalize()`. Test `content_hash_sha256_known_value` asserts known empty-string hash. Legit fix for 2026-04-14 Iter 34 M1 finding.
- [ ] **NEW SURFACE WITHOUT PRIORITY** — `nom-graph/src/sandbox.rs` (328 LOC) added with `sanitize()` + `EvalContext` + `Expr` AST + tests for depth-limit/blocked-function/allowed-function. Partial n8n pattern — only 1 of the 4 AST sanitizers (this_replace, prototype_block, dollar_validate, allowlist) visible by name. Spec §5 expects all 4.

**PRIORITY VIOLATION:** Executor shipped ~328 LOC new sandbox surface while E2 CRITICAL (paint no-op) remains open for 3+ iterations. This is the same pattern that triggered HARD FREEZE at Iter 30 + 36. Re-issue single-commit mandate: E2 FIRST, then ship the sandbox.

**Still open:** E2 CRITICAL + 9 HIGH + 8 MEDIUM (see task.md).

## Iteration 48 — Parallel DB-driven + UI/UX scan (2026-04-18)

**HEAD:** `fb886fa` unchanged. Working tree has doc compaction only.

**Verified PASS** (parallel agents, file:line evidence):
- [x] NomtuRef non-optional on BlockModel + GraphNode + Connector (nom-blocks)
- [x] `can_wire_result: (bool, f32, String)` non-optional in `connector.rs:22`
- [x] Zero hardcoded node-type enums — `graph_node.rs:13` uses `production_kind: String` validated via `DictReader::is_known_kind`
- [x] DB isolation holds — only `sqlite_dict.rs:23,27` opens Connections
- [x] `nom-compiler-bridge/Cargo.toml` feature-gates cross-workspace path deps correctly
- [x] `nom-theme/src/tokens.rs` has 35 AFFiNE tokens (>20 min); BG/BG2/BORDER/FOCUS exact hex match
- [x] Spring solver REAL at `nom-gpui/src/animation.rs:98-128` with AFFiNE defaults (stiffness=400, damping=28)
- [x] Confidence edge colors EDGE_HIGH/MED/LOW defined + routed via `edge_color_for_confidence()`

**NEW FINDINGS added to task.md:**
- [ ] NR1 HIGH — `nom-graph/src/graph_mode.rs:242-244` defines duplicate `NomtuRef` with `Option<u64>` (§12 violation)
- [ ] FG1 HIGH — Frosted glass tokens defined but no blur primitive used anywhere (canvas renders flat rects)
- [ ] FR1 MEDIUM — Focus ring is alpha-fill overlay, not 2px outline stroke
- [ ] PAL1 MEDIUM — Node palette entirely missing (Wave D deliverable)
- [ ] E2 CRITICAL — re-verified `elements.rs:224-227` no-op paint bodies

## Iteration 47 — Cron audit, no new commits (2026-04-18)

**HEAD:** `fb886fa` unchanged. Executor idle since Wave M + docs.

**E2 CRITICAL re-verified** — `nom-canvas-core/src/elements.rs:224-227` `GraphNodeElement::paint()` is `let _ = bounds;` with literal comment "In a real impl: push Quad to Scene with self.position + self.size". `WireElement::paint()` at :250-257 identical. Wave L "impl Element (3-phase GPU lifecycle)" was SHAPE COMPLIANCE ONLY — trait implemented, bodies empty. Canvas silently renders nothing for graph nodes/wires.

**Mandate:** single ~100-150 LOC commit — replace `let _ = bounds;` with real primitive pushes (`Quad { bounds, color, border }` + port circles for GraphNodeElement, bezier line-segment tessellation for WireElement). Same playbook as Iter 42 single-commit close of U1/W1/COL1/INT1.

## Iteration 46 — Wave M (2026-04-18, commit `ef9fc84`)

- Claims: nom-lint sealed + compiler-bridge 3-tier + compose dispatch/plan/task_queue + graph 4-tier cache
- Audit verdict: 4 DRIFT (nom-lint 2-trait not 3, HierarchicalCache::len L1-only, dispatch not Backend trait, UiTierOps alloc) + 1 HIGH correctness bug (TQ1: complete() no Running guard)
- Tests: 498

## Iteration 45 — Wave L (2026-04-18, commit `d139644`)

- Claims closed: with_deep_think wired + RRF + W3C traceparent + impl Element
- Tests: 504
- **Drift found:** "impl Element" is trait only — bodies discarded bounds → origin of E2

## Iteration 43 — Wave K verified (2026-04-18, commit `dc6a025`, tests 457)

- U1: nom-panels paint_scene via `nom_gpui::scene::Quad` + `nom_theme::tokens` (10 files)
- W1: deep_think imports `nom_intent::{classify_with_react, react_chain}` at lines 4, 60, 116
- COL1: RGA CRDT with RgaPos::{Head, After(OpId)} + tombstoned: bool + convergence tests
- INT1: 11 cross-crate `use nom_gpui::scene` matches
- **All 4 original CRITICALs closed.**

## Pre-Iter 43 Summary

- Iter 30 HARD FREEZE (Executor adding surface without closing blockers) — lifted Iter 31
- Iter 36-41 — 9 consecutive iters U1 open; Iter 40 documented single-commit ~200 LOC mandate
- Iter 42 — working-tree diff achieved 3/4 CRITICALs simultaneously (playbook confirmed)
- Waves E/F/G/H/I/J landed between Iter 28-41 adding 16 backends + deep_think + CRDT stub + panel pixel layer

## Reference Commits

| Commit | Wave |
|---|---|
| `e61a93c` | Wave O Infra+LSP |
| `d6219b1` | Wave N Infra+Vendor |
| `fb886fa` | Wave M docs |
| `ef9fc84` | Wave M infra |
| `d139644` | Wave L MEDIUM |
| `dc6a025` | Wave K 4 CRITICALs |
| `a1ba5a1` | Wave E 16 backends |
| `be3b9a8` | Wave F RAG + deep_think |
| `fb66e01` | Wave C keystone |
| `8c7d32e` | Wave A+B commit |

Older entries archived to git history.
