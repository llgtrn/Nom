# 09 — Ultimate Path from 2026-04-13 to Nom 1.0

**HEAD**: `fdfb32d` (origin/main). **Session arc**: 28 commits past `1ae2136` (docs 07+08 landed; `.nomx v2` keyed syntax + MECE validator + agent_demo + manifest handoff + store.rs refactor + research rewrite shipped). **Shipped surface**: doc 07 at ~98%, doc 08 at ~58%, doc 03 Phase 4 at 100%, doc 04 §4.4.6 + §10.3.1 enforced, effect valence + Vietnamese alias packs landed.

Unlike the prior doc 09 (a status snapshot), this revision is a **PATH**: starting from HEAD `fdfb32d`, the ordered sequence of milestones that converts the current shipped surface into the vision pulled from 20 processed research docs + 7 deferred docs (total 4,382 lines of deferred research synthesized here for the first time).

## Where we are

The pipeline `source → DB1/DB2 store → ConceptGraph closure → resolved refs → MECE objectives → Manifest JSON` runs end-to-end today. Three demos exercise it: `examples/concept_demo` (minimal), `examples/agent_demo` (motivation 16 §5 killer-app seed: 6 tools + safety policy + intentional MECE collision), and `examples/agent_demo_vn` (inert Vietnamese locale pack — kept per user "don't extend"). e2e tests at `crates/nom-cli/tests/{concept_demo,agent_demo,agent_demo_vn,build_manifest}_e2e.rs` (Windows gated because `nom` links LLVM-C.dll). Test totals: `nom-concept` 76, `nom-dict` 24, `nom-media` 50. Typed-slot `@Kind` refs resolve via `find_words_v2_by_kind` with alphabetical-smallest deterministic tiebreak; `with at-least N confidence` threshold syntax shipped; per-slot top-K diagnostic surfaces alternatives in `nom build status`.

## Where we're going

The vision, synthesized across all research: Nom is a **verified semantic composition engine** (deferred 06 §8 blueprint) where the unit of the language is the `.nomtu` entry (a hash-addressed Nom with kind + contract + effects + provenance + scores); Vietnamese provides the **grammar skeleton** — classifier-first, head-initial, topic-markable, alias-rich — while vocabulary is **globally lexical** via locale packs mapping to the same NomID (deferred 09 §3, §5); the dictionary is structurally **isomorphic to Vietnamese morpheme composition** (deferred 11: ~50-80 math morphemes → all of math; deferred 12: 145 morphemes → all of science), so ~200 kinds can generate all of computable knowledge via `-> :: +` operators that are **simultaneously categorical morphism composition, modus ponens, and function application** (Curry-Howard-Lambek, deferred 11 §B); composition is verified at every stage through contract compatibility, effect tracking, dimensional analysis, and algebraic laws (deferred 11 §C, deferred 12 Modelica across/through §); the only AI component is a **thin intent-resolution transformer** mapping natural language to bounded Nom concepts (deferred 05 §Hybrid); every compile emits a **glass-box report** (deferred 06 §Phase 7); the compiler itself is eventually **remade in Nom via a fixpoint bootstrap** (doc 04 Phase 10) and Rust is archived.

## The path

Each milestone: **Name** · **Unblocks** · **Deliverable** · **Evidence** · **Effort** · **Deps**.

### M1 — Per-slot top-K glass-box in manifest + report format

- **Unblocks**: transition from "fa1ba8a diagnostic" to a first-class glass-box artifact per deferred 06 §Phase 7 (the promised differentiator). Every later milestone needs reports to explain its own decisions.
- **Deliverable**: `nom build report <repo>` CLI emitting a `ReportBundle` JSON (resolved NomID + alternatives + rejection reasons + scores + effect list + MECE outcome + provenance trail) + rendered human form. Test gate: `report_roundtrip_e2e.rs`.
- **Evidence**: `853e70b` shipped per-slot top-K diagnostic; `fef0419` shipped manifest JSON plumbing. The structure already exists, the surfacing is a thin wrap.
- **Effort**: days.
- **Deps**: none (current HEAD).

### M2 — Acceptance-predicate preservation engine (doc 08 §9.1)

- **Unblocks**: the "preserve, never mislead" discipline that makes dreaming safe. Without this, M5 layered dreaming can drift.
- **Deliverable**: `crates/nom-concept/src/acceptance.rs` implementing predicate-preservation checks across revisions + `nom build verify-acceptance` subcommand. Test gate: `acceptance_preserve_e2e.rs`.
- **Evidence**: doc 08 §9.1 ⏳ marked; MECE validator shipped `c63a6a7` proves the hard-check pattern.
- **Effort**: days.
- **Deps**: M1 (report surfaces preservation outcomes).

### M3 — Locale pack scaffold + canonicalization layer (deferred 07 §9 three-layer model)

- **Unblocks**: "Vietnamese grammar, global vocabulary" direction. Once aliases resolve to the same NomID, M11 multi-locale workspaces become feasible.
- **Deliverable**: `crates/nom-locale/` new workspace crate with `LocalePack { id: BCP47, keyword_aliases, nom_aliases, register_metadata }`, UTS #39 confusable detector, UAX #15 NFC normalizer. `nom locale {list, validate, apply}` CLI. Test gate: `locale_vi_en_roundtrip_e2e.rs`.
- **Evidence**: `4b04b1d` + `5b59f82` Vietnamese alias packs already shipped as one-off keyword aliasing; deferred 09 §6 specifies the full standards stack (BCP 47, CLDR, UAX 31/15, UTS 39/55).
- **Effort**: weeks.
- **Deps**: none; proceeds in parallel with M1/M2.

### M4 — Three-tier recursive byte ingest (doc 08 §4.3) ✅ SHIPPED

- **Unblocks**: the layered compiler — atoms → `.nomtu` → `.nom` → cross-concept closure — that doc 08 promised but only shipped single-pass.
- **Deliverable**: `resolve_closure` extended to recurse through nested concept refs in any tier; `nom store sync --recursive`. Test gate: `three_tier_recursive_e2e.rs` with a concept that imports a concept that imports a module.
- **Evidence**: `c5cdce6` concept-graph closure walker (single-tier); `aaa914d` DB1+DB2 schema supports the tiering. ✅ SHIPPED — `visit_entity_ref` in `crates/nom-concept/src/closure.rs` now recurses into nested concept declarations when `kind == "concept"` and the word matches a known concept in the graph; 4 new unit tests + `three_tier_recursive_e2e.rs`. `--recursive` flag omitted — recursion is always-on (backwards compat: roots without nested concept refs behave identically to pre-M4).
- **Effort**: days.
- **Deps**: M1.

### M5 — Layered dreaming (concept-tier + module-tier) (doc 08 §9) ✅

- **Unblocks**: the `nom app dream` loop running at three tiers instead of one — proposals at the right granularity (whole app vs. concept vs. module vs. entry).
- **Deliverable**: `nom app dream --tier {app|concept|module} [--repo-id <id>] [--pareto-front]` with `LayeredDreamReport { tier, label, leaf, child_reports, pareto_front }`. Recursion through child concepts via concept graph; cycle protection; Pareto front over (app_score, complete, low_partial_ratio) — no cross-tier normalization per doc 08 §6.
- **Evidence**: shipped in three commits — `f0ae193` + `9b3f501` (M5a wedge: tier flag + LayeredDreamReport scaffolding), `e28f69d` (M5b: child_reports populated via nom-concept ConceptGraph closure walk), `cc9641b` (M5c: pareto_front populated via 3-axis dominance scan + `--pareto-front` text flag). Tests: `nom-app` 26/26 (was 15 pre-M5; +11 across M5a/b/c) + `layered_dream_smoke.rs` 6 assertions on Linux CI. Module-tier (`--tier module`) prints "not yet implemented (module-tier coming in M5b)" placeholder — wedge intentionally narrow; full module-tier dreaming reuses the same recursive shape against module CompositionDecls and lands when M6 corpus pilot starts producing modules worth dreaming over.
- **Effort**: weeks → **shipped in days as 3-slice wedge**.
- **Deps**: M2 ✅ (preservation) + M4 ✅ (recursive tiers).

### M6 — Corpus ingestion: PyPI top-100 pilot (doc 04 §5.17 + deferred 08 §Path A)

- **Unblocks**: the dictionary growth that underlies everything else. Without corpus, the MECE CE-check is stub, the semantic re-rank is stub, and the scaling claim (145 morphemes → all science) is unvalidated. A small pilot validates the pipeline before the full multi-ecosystem run.
- **Deliverable**: `nom corpus ingest pypi --top 100 --budget 50GB` running stream-and-discard per §5.17 (peak disk = max per-repo-source + current-dict); checkpoint + resume; workspace-gc; ingestion report. Test gate: at least 10K `.nomtu` entries with Partial status in DB2.
- **Evidence**: `nom-corpus` crate scaffolded per memory `project_nom_roadmap`; `underthesea` + `MELT` identified as preprocessing/eval references (deferred 08 §4).
- **Effort**: weeks.
- **Deps**: M1 (reports surface ingestion outcomes). Can run in parallel with M2-M5.

### M7 — MECE CE-check via corpus-registered required axes (doc 08 §9.2)

- **Unblocks**: the "collectively-exhaustive" half of MECE that currently ships as a stub. After this, an agent_demo that declares `database` but forgets `auth_provider` is caught at build time.
- **Deliverable**: `entry_required_axes` table in corpus side-tables + `nom build status` extension checking each composition against registered per-kind required sets. Test gate: `mece_ce_missing_axis_e2e.rs` exits 1.
- **Evidence**: `c63a6a7` MECE ME-check shipped (same validator pattern extends); doc 08 §9.2 ⏳.
- **Effort**: days after M6.
- **Deps**: M6 (corpus must register axes first).

### M8 — Intent-resolution transformer (thin LLM integration) (deferred 05 §Hybrid + motivation 13)

- **Unblocks**: the **prose → .nom artifact** pipeline described in user memory `project_nom_prose_to_artifact`. After this, "build me a secure API" resolves to `[http_server, auth_flow, tls, rate_limiter]` deterministically.
- **Deliverable**: `crates/nom-intent/` — bounded-output transformer wrapper (output domain = registered Nom kinds) + validation against DB2 (did it produce real concepts?) + confidence threshold. `nom author resolve-intent <prose>` CLI. Test gate: `intent_bounded_output_e2e.rs` — outputs MUST resolve or report unresolvable.
- **Evidence**: deferred 05 §Hybrid specifies ~1B params, bounded output space; `underthesea` + `PhoGPT` baselines identified (deferred 08 §5 Path A). AI-invokes-compiler protocol per user memory §5.19.
- **Effort**: weeks.
- **Deps**: M6 (needs a populated dictionary to resolve to).

### M9 — Phase-9 corpus-embedding semantic re-rank (doc 07 §3.3 pending piece)

- **Unblocks**: replace the alphabetical-smallest hash tiebreak with genuine semantic scoring; closes doc 07 to 100%.
- **Deliverable**: per-kind embedding index in `entry_embeddings` side-table; `find_words_v2_by_kind` ranks by (score × similarity × provenance). Test gate: `embedding_rerank_regression_e2e.rs` — same demo resolves to higher-scored Nom than alphabetical pick would.
- **Evidence**: `bf95c2c` + `c405d2a` shipped the deterministic stub; user memory roadmap §5.10 canonicalization lifts Partial → Complete.
- **Effort**: weeks.
- **Deps**: M6.

### M10 — Phase-5 planner-in-Nom port (doc 03 Phase 5)

- **Unblocks**: the compiler starts eating its own manifests. After this, `RepoManifest` → executable plan is internal Nom composition, not external tooling.
- **Deliverable**: `nom plan <manifest.json>` producing `BuildPlan` JSON (ordered step list + effect-aware scheduling). Planner itself authored as `.nomtu` entries in `stdlib/self_host/planner/`. Test gate: `planner_self_host_e2e.rs`.
- **Evidence**: doc 04 §10.3.1 fixpoint toolchain pinned `29f5f1d`/`bc89d8a`/`96267df`; manifest JSON handoff shipped `fef0419`; self-host lexer precedent at `nom-compiler/stdlib/self_host/lexer.ll`.
- **Effort**: quarters (doc 03 flags this as multi-week, user memory says "parked").
- **Deps**: M1, M4, M9.

### M11 — Multi-locale workspaces (deferred 09 §8 per-file locale + deferred 07 §3.1-3.4)

- **Unblocks**: a source file can declare `locale vi` or `locale ar script Arab` and the parser normalizes to canonical AST. Core of "lexical democracy without semantic fragmentation."
- **Deliverable**: `locale <bcp47> [script <code>]` directive in parser; normalization layer between parse and resolve; formatter preserves locale. Test gate: `parse_same_ast_across_locales_e2e.rs` — VN, EN, ES sources produce byte-identical AST after normalization.
- **Evidence**: deferred 07 §9 three-layer model (surface / normalization / semantic); deferred 09 §8 parser pipeline.
- **Effort**: weeks.
- **Deps**: M3 (locale packs must exist first).

### M12 — Algebraic laws + dimensional analysis on Noms (deferred 11 §C + deferred 12 Modelica)

- **Unblocks**: the "optimize by law" capability that no existing language does — associative reduction, commutative parallelization, dimensional type errors at compile time. Unblocks cross-domain composition (M14).
- **Deliverable**: `laws: [associative, commutative, identity, inverse, distributive]` field on Nom entries; `dimensions: [length / time]` field; verifier pass that rejects `newtons + meters_per_second`; optimizer pass that re-associates multiplication chains. Test gate: `dimension_mismatch_rejected_e2e.rs` + `associative_reassoc_optimizer_e2e.rs`.
- **Evidence**: deferred 11 §C concrete schema; deferred 12 Modelica across/through Power=effort×flow universality.
- **Effort**: weeks.
- **Deps**: M6 (laws live in dict).

### M13 — Mass corpus: top-500 PyPI + top 500/ecosystem GitHub (doc 04 §5.17)

- **Unblocks**: the scaling claim. 145 morphemes → all of science needs >1M entries to validate. Also unblocks M14 cross-domain.
- **Deliverable**: full §5.17 protocol run across JS/Python/Rust/Go/Java/C++/Swift/Ruby/PHP; `nom corpus report` produces per-ecosystem completion metrics; Partial entries systematically lifted to Complete via §5.10.
- **Evidence**: M6 pilot validates the pipeline; memory roadmap §5.17 specifies the full protocol.
- **Effort**: quarters.
- **Deps**: M6, M7.

### M14 — Cross-domain connectors (Modelica across/through, deferred 12 §Architecture)

- **Unblocks**: the universal-knowledge-composition claim. Physics/chemistry/biology/economics compose in one flow with automatic conservation-law verification.
- **Deliverable**: `connector` kind + `across` / `through` contract fields + conservation-law verifier. Test gate: electromechanical motor example from deferred 12 §Modelica compiles and verifies `power_in >= power_out`.
- **Evidence**: deferred 12 §Modelica; Vietnamese morpheme cross-domain reuse (trường, năng_lượng, cân_bằng) is the linguistic precedent.
- **Effort**: weeks.
- **Deps**: M12 (needs laws+dimensions), M13 (needs domain coverage in dict).

### M15 — Parser-in-Nom prereqs + Parser in Nom (doc 04 Phases 6-7)

- **Unblocks**: compiler self-hosting step 2. After this, `.nomtu` parser + `.nom` parser + `.nomx` parser are all expressed as Nom compositions.
- **Deliverable**: `stdlib/self_host/parser/` tree parallel to existing lexer; `nom lex+parse` pipeline runnable from self-host `.nomtu`. Test gate: `parser_self_host_e2e.rs` produces byte-identical AST to Rust parser on full demo corpus.
- **Evidence**: `05ee1b6` + `d9425ba` hand-rolled parsers shipped in Rust (source of truth for Nom port); lexer precedent done (memory: Phase 3 complete).
- **Effort**: quarters (doc 04: 10-14 weeks for Phase 7 alone).
- **Deps**: M10.

### M16 — LSP + Authoring Protocol (doc 04 Phase 9, user memory lists as CORE)

- **Unblocks**: the "tooling is generated from semantic pipeline, not bolted on" principle (deferred 06 §Phase 9). Editor hover explains every Nom; "why this Nom?" command drops users into the glass-box report.
- **Deliverable**: `nom-lsp` crate implementing LSP server; Authoring Protocol spec + reference client. Test gate: `lsp_hover_why_this_nom_e2e.rs`.
- **Evidence**: doc 04 §9 CORE designation; deferred 06 §Phase 9 checklist (formatter, LSP, hover, graph viz, verification diff).
- **Effort**: quarters (doc 04: 12-16 weeks).
- **Deps**: M1, M8. Can proceed in parallel with M13-M15.

### M17 — Phase-10 fixpoint bootstrap + rust-nomc retirement (doc 04 Phase 10)

- **Unblocks**: the proof that Nom is a language. This is the definition, from the rustc/GHC/OCaml/Zig lineage.
- **Deliverable**: Stage0-Rust → Stage1 → Stage2 → Stage3; `s2 == s3` byte-identical. Proof-of-bootstrap tuple `(s1_hash, s2_hash, s3_hash, fixpoint_at_date, compiler_manifest_hash)` recorded permanently in the dict. Parity track: ≥99% IR equivalence + 100% runtime correctness on test corpus. Cutover: 4wk parity validation → default flip → Rust source archived to `.archive/rust-<version>/` → 3mo grace.
- **Evidence**: user memory `project_nom_language_evolution.md` + roadmap Phase 10; §10.3.1 toolchain pin + fixpoint discipline already enforced.
- **Effort**: quarters (multi-month).
- **Deps**: M10, M15, M16. **This is the 1.0 gate.**

### M18 — Phase-12 closure-level specialization (70-95% binary reduction) (doc 04 Phase 12)

- **Unblocks**: the performance/deploy story that makes Nom credible for production (motivation 16 §6 "proving fast/scales/lasts").
- **Deliverable**: `nom bench {<hash>, compare, regress, curate}` driving closure-level specialization via `entry_benchmarks`; bipartite min-cost assignment per §5.15 (joint multi-app × multi-platform). Test gate: `specialization_binary_reduction_e2e.rs` shows ≥70% reduction on demo corpus.
- **Evidence**: memory roadmap §5.15 joint optimization; `entry_benchmarks` typed side-table designed.
- **Effort**: quarters.
- **Deps**: M13, M17.

## Parallel tracks

Three parallel lanes can proceed after the current HEAD:

- **Glass-box / verification lane**: M1 → M2 → M7 → M12 → M14.
- **Dictionary / corpus lane**: M6 → M9 → M13 → M18.
- **Surface / tooling lane**: M3 → M11 → M16.

Self-hosting lane M10 → M15 → M17 is the critical chain; it depends on outputs of all three parallel lanes before 1.0.

## Critical path to Nom 1.0 (motivation 16 "usable 1.0")

```
HEAD (cc9641b)
  ├── M1  (glass-box report)        ✅ shipped — 41aedfe
  ├── M2  (acceptance preservation) ✅ shipped — fb9c252
  ├── M4  (three-tier byte ingest)  ✅ shipped — 8aab409
  ├── M5  (layered dreaming)        ✅ shipped — f0ae193 + e28f69d + cc9641b
  ├── M6  (PyPI-100 corpus pilot)   — weeks   ← next critical-path slot
  ├── M9  (embedding re-rank)       — weeks
  ├── M10 (planner-in-Nom)          — quarters
  ├── M15 (parser-in-Nom)           — quarters
  ├── M16 (LSP)                     — quarters
  └── M17 (fixpoint bootstrap)      — quarters  ← 1.0 cut here
```

Milestones M3/M11 (multi-locale), M7/M12/M14 (MECE-CE + laws + cross-domain), M8 (intent transformer), M13 (mass corpus), M18 (specialization) are **post-1.0 enhancements** that land on the parallel tracks but don't gate the bootstrap proof. 1.0 = "compiler is self-hosted + Rust archived."

## Honest risks

1. **M6/M13 corpus ingestion is disk-stream-and-discard or bust.** Doc 04 §5.17 says peak disk = max per-repo-source + current-dict; any caching of intermediates blows the budget. Skip-lists + checkpointing + bandwidth throttle are non-optional. Risk: a sloppy ingestion run corrupts the dict.
2. **M8 intent transformer is the only probabilistic component.** Deferred 05 §Hybrid. If its output isn't hard-bounded to registered Nom kinds + validated against DB2, the whole "zero hallucination" thesis collapses. Must reject any token not in the registered output space.
3. **M10/M15/M17 self-hosting underestimated historically.** Doc 04 marks Phase 5 multi-week, user memory says "parked." Every self-hosting language (rustc took ~10 years to full parity) runs long. M10 being a quarter is aggressive.
4. **M11 multi-locale = immediate security surface.** Deferred 09 §11 flags confusable attacks (UTS #39), translation drift, tooling burden. Locale pack governance needs community review before expanding beyond VI/EN.
5. **M14 cross-domain laws are easy to write, hard to verify.** Deferred 11 §7.1 semantic aliasing: two Noms that look equivalent but differ in hidden assumptions. Conservation laws need airtight contract vocabulary or the verifier makes pretty but wrong claims.
6. **M3 Vietnamese positioning risk.** User clarified vocabulary stays English; `agent_demo_vn` was reclassified "keep but don't extend." Deferred 09 §11.5 cultural asymmetry warning: if Vietnamese provides deep grammar but English dominates visible tooling, system silently re-centralizes on English. M11 must genuinely ship non-English locales or the lingua-franca claim is hollow.
7. **Scoring as pseudo-objective (deferred 06 §7.2).** "security: 0.96" is decorative numerology unless derivation + provenance + update policy + auditability are explicit. M1 glass-box must surface score provenance, not just the number.
8. **Graph-aware ownership is later-stage (deferred 06 §7.3).** Memory strategy inference across shared mutable graphs is one of the hardest systems problems. Do not block 1.0 on ownership sophistication; ship simple strategy + improve post-1.0.
9. **Dictionary curation is socio-technical (deferred 06 §7.5).** Who publishes, reviews, earns trust, excludes malicious Noms — this is governance, not code. Parallel to M13 mass ingestion, a review/trust model must exist or the dict becomes a supply-chain attack surface.

## What we're NOT building (and why)

1. **No real per-kind LLM resolver in the build path.** Doc 04 §10.3.1 fixpoint forbids it: the compiler MUST be deterministic. M8 intent resolver is a **pre-build** author aid, not a build step. The build loop sees only deterministic hash-addressed entries.
2. **No natural-language-to-Nom generation in v1 grammar (deferred 06 §Phase 0 freeze).** The canonical grammar stays small and stable. M8 is a thin translation layer between prose and bounded Nom concepts, not a generative NL front-end that invents new syntax.
3. **No multiple backends before LLVM is excellent.** Deferred 06 §Phase 6 "every extra backend multiplies semantic complexity." Wasm / JS / WASI come post-1.0, after M17 fixpoint.
4. **No GUI toolkit, distributed agents, or graph DB in the first stdlib.** Deferred 06 §Phase 8 explicitly defers these. The initial dict is: text transform, JSON, filesystem, HTTP endpoint composition, hashing, rate limiting, cache — enough to prove the model.
5. **No free word order.** Deferred 07 §1 correction: Vietnamese is NOT order-free, it's **anchored-flexible**. M11 multi-locale adds lexical variation with strong anchors, not structural scrambling.
6. **No aspect markers runtime** (motivation 02 §8). User memory: parked — low semantic value without a runtime that interprets `active`/`verified`/`deferred`. Revisit post-M17.
7. **No Vietnamese function-name identifiers** (memory project_nom_language_evolution). `expect_word` enforces ASCII for function names; diacritic-in-identifier is a separate, bigger scope than diacritic-in-keyword. Locale packs localize keywords, not identifiers.
8. **No free synonym explosion.** Deferred 07 §4.3, deferred 09 §11.1: alias packs are explicit, documented, community-governed. Uncontrolled synonyms destroy readability and create confusable-identifier security holes.
9. **No compilation to many IRs before the fixpoint proof.** Multiple IRs (AST → resolved → verified → backend plan per deferred 06 §Phase 5) are internal to the compiler, not user-visible targets. The fixpoint proof (M17) must pin ONE toolchain.
10. **No self-modifying dictionary at build time.** §5.10 canonicalization upgrades Partial → Complete is an **offline** maintenance operation, not a build-time side effect. Build must be a pure function of inputs + pinned dict.

---

This doc will stay valid until the first milestone ships. When M1 lands, revise the "Where we are" paragraph and strike M1 from the path; leave the rest of the ordering untouched. Every re-derivation of the path should produce the same ordering from the same research inputs — that is itself a fixpoint check on the plan.
