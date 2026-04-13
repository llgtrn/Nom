# 09 — Ultimate Path from 2026-04-13 to Nom 1.0

**HEAD**: `fdfb32d` (origin/main). **Session arc**: 28 commits past `1ae2136` (docs 07+08 landed; `.nomx v2` keyed syntax + MECE validator + agent_demo + manifest handoff + store.rs refactor + research rewrite shipped). **Shipped surface**: doc 07 at ~98%, doc 08 at ~58%, doc 03 Phase 4 at 100%, doc 04 §4.4.6 + §10.3.1 enforced, effect valence + Vietnamese alias packs landed.

Unlike the prior doc 09 (a status snapshot), this revision is a **PATH**: starting from HEAD `fdfb32d`, the ordered sequence of milestones that converts the current shipped surface into the vision pulled from 20 processed research docs + 7 deferred docs (total 4,382 lines of deferred research synthesized here for the first time).

## Where we are

The pipeline `source → DB1/DB2 store → ConceptGraph closure → resolved refs → MECE objectives → Manifest JSON` runs end-to-end today. Two demos exercise it: `examples/concept_demo` (minimal) and `examples/agent_demo` (motivation 16 §5 killer-app seed: 6 tools + safety policy + intentional MECE collision). `examples/agent_demo_vn` has been deleted (ecd0609 — fully English vocabulary directive). e2e tests at `crates/nom-cli/tests/{concept_demo,agent_demo,build_manifest}_e2e.rs` (Windows gated because `nom` links LLVM-C.dll). Test totals (post-session): `nom-concept` 77/77 (was 92 pre-`ecd0609` VN removal, then +4 from M7a MECE-CE ⇒ net 77), `nom-dict` 29/29 (+5 from M7a required_axes), `nom-locale` 25/25 (M3a 12 + M3c 3 + M3b-minimal 5 + misc), `nom-app` 30/30 (M5a+b+c + M7c), `nom-media` 50. Typed-slot `@Kind` refs resolve via `find_words_v2_by_kind` with alphabetical-smallest deterministic tiebreak; `with at-least N confidence` threshold syntax shipped; per-slot top-K diagnostic surfaces alternatives in `nom build status`.

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

### M3 — Locale pack scaffold + canonicalization layer (deferred 07 §9 three-layer model) 🟢 infrastructure only (M3a ✅; M3c infrastructure ✅ / vocab parked; M3b ⏳)

- **Unblocks**: M11 multi-locale workspaces. Locale packs provide the structural hooks (BCP47, NFC, confusables, grammar-style rules).
- **Scope correction (2026-04-13)**: per user directive, Nom borrows Vietnamese **GRAMMAR STYLE** (classifier phrases, modifier-after-head, effect valence) — vocabulary stays **English**. Keyword translation (cái/hàm/là → the/function/is) is NOT the locale-pack purpose. `LocalePack.keyword_aliases` exists but stays empty for vi-VN; future locale features land as grammar-rules, not keyword translations.
- **Deliverable**: `crates/nom-locale/` new workspace crate with `LocalePack { id: BCP47, keyword_aliases, nom_aliases, register_metadata }`, UTS #39 confusable detector, UAX #15 NFC normalizer. `nom locale {list, validate, apply}` CLI. Test gate: `locale_vi_en_roundtrip_e2e.rs`.
- **Evidence**: `4b04b1d` + `5b59f82` Vietnamese keyword aliases previously shipped in the lexer; `b3fe503` reverted the nom-locale alias bake; **ecd0609 fully removes all VN keyword aliases from lexer + tests + demos**. **M3a** (commit `49869ab`) lands the `nom-locale` crate with `LocaleTag::parse` (BCP47), `normalize_nfc` (UAX #15), `LocalePack`+`RegisterMetadata`, baked `builtin_packs()` for vi-VN + en-US (both with empty `keyword_aliases`), `is_confusable` stub (returns `Deferred`), and `nom locale list` / `nom locale validate` CLI. **M3c** ships `apply_locale()` lexical pass (infrastructure; vi-VN pack is a no-op pass-through). deferred 09 §6 specifies the full standards stack (BCP 47, CLDR, UAX 31/15, UTS 39/55). M3b remains: load UTS #39 confusables.txt to upgrade `is_confusable` from `Deferred` stub.
- **Effort**: M3a + M3c infrastructure shipped; M3b (confusable data) remains days.
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

### M7 — MECE CE-check via corpus-registered required axes (doc 08 §9.2) 🟢 scaffolded

- **Unblocks**: the "collectively-exhaustive" half of MECE that currently ships as a stub. After this, an agent_demo that declares `database` but forgets `auth_provider` is caught at build time.
- **Deliverable**: `entry_required_axes` table in corpus side-tables + `nom build status` extension checking each composition against registered per-kind required sets. Test gate: `mece_ce_missing_axis_e2e.rs` exits 1.
- **Evidence**: `c63a6a7` MECE ME-check shipped (same validator pattern extends); **M7a** (commit `bcadcb3`) landed the `required_axes` SQLite registry + `check_mece_with_required_axes` CE logic + `nom corpus register-axis` / `list-axes` CLI, independent of corpus content. nom-dict 29/29 (+5), nom-concept 92/92 (+4). Build.rs + manifest.rs now call the registry-aware variant; empty registry → vacuous CE pass. M7b will seed the standard axes (correctness / safety / performance) once M6 corpus pilot runs.
- **Effort**: M7a scaffold shipped in this cycle; M7b (default axis seeding) is days after M6.
- **Deps**: M6 (corpus pilot populates the canonical axis list) — **M7a has no M6 dep and is already usable by authors registering axes manually via `nom corpus register-axis`**.

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

### M10 — Phase-5 planner-in-Nom port (doc 03 Phase 5) 🟢 gates shipped (M10a+M10b); real port deferred

- **Unblocks**: the compiler starts eating its own manifests. After this, `RepoManifest` → executable plan is internal Nom composition, not external tooling.
- **Deliverable**: `nom plan <manifest.json>` producing `BuildPlan` JSON (ordered step list + effect-aware scheduling). Planner itself authored as `.nomtu` entries in `stdlib/self_host/planner/`. Test gate: `planner_self_host_e2e.rs`.
- **Evidence**: doc 04 §10.3.1 fixpoint toolchain pinned `29f5f1d`/`bc89d8a`/`96267df`; manifest JSON handoff shipped `fef0419`; self-host lexer precedent at `nom-compiler/stdlib/self_host/lexer.ll`. **M10a** (commit `249fe62`) audited all 7 `.nom` files under `stdlib/self_host/` + `examples/run_lexer.nom` (all VN-clean post-`ecd0609`) and landed `self_host_parse_smoke.rs` as the regression gate. **M10b** (commit `cef8425`) pinned `run_lexer.bc` SHA-256 `085e8fa6...83296` as the §10.3.1 byte-determinism gate (Linux-CI first-run verification). The real port (authoring planner logic as `.nomtu`s + `nom plan` CLI) stays quarters-scale.
- **Effort**: gates shipped in days; real port remains quarters (doc 03 flags this as multi-week, user memory says "parked").
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
HEAD (f34e6f8)
  ├── M1   (glass-box report)              ✅ shipped — 41aedfe
  ├── M2   (acceptance preservation)       ✅ shipped — fb9c252
  ├── M4   (three-tier byte ingest)        ✅ shipped — 8aab409
  ├── M5   (layered dreaming a+b+c)        ✅ shipped — f0ae193 + e28f69d + cc9641b
  ├── M7a  (MECE CE registry + check)      ✅ shipped — bcadcb3
  ├── M7c  (MECE in layered dream)         ✅ shipped — 4307c5a
  ├── M3a  (nom-locale + BCP47 + NFC)      ✅ shipped — 49869ab
  ├── M3c  (apply_locale + CLI)            ✅ infrastructure — 339a74a revert-b3fe503 (vocab stays empty per fully-English directive; infra is generic)
  ├── VN-vocab-removal (fully English)     ✅ shipped — ecd0609 (31 lexer arms + agent_demo_vn/ deleted)
  ├── M10a (self-host parse gate)          ✅ shipped — 249fe62
  ├── M10b (run_lexer.bc reproducibility)  ✅ shipped — cef8425 (Linux-CI verify on first run)
  ├── ci-matrix-fix                        ✅ shipped — f34e6f8 (6 crates added to Linux CI)
  ├── M3b-minimal (30 common confusables)  ✅ shipped — 712ede4 (Cyrillic/Greek→Latin; M3b-full = UTS #39 full table)
  ├── M6   (PyPI-100 corpus pilot)         — weeks   ← next critical-path slot (needs network; park for manual run)
  ├── M3b-full (UTS #39 confusables.txt)   — days   (data-file load to replace the baked 30-pair table)
  ├── M9   (embedding re-rank)             — weeks
  ├── M10c+ (compile-to-IR subset for self_host) — weeks
  ├── M10  (real planner-in-Nom port)      — quarters
  ├── M15  (parser-in-Nom)                 — quarters
  ├── M16  (LSP)                           — quarters
  └── M17  (fixpoint bootstrap)            — quarters  ← 1.0 cut here
```

Milestones M3/M11 (multi-locale), M7/M12/M14 (MECE-CE + laws + cross-domain), M8 (intent transformer), M13 (mass corpus), M18 (specialization) are **post-1.0 enhancements** that land on the parallel tracks but don't gate the bootstrap proof. 1.0 = "compiler is self-hosted + Rust archived."

## Remaining work (GitNexus-verified 2026-04-14)

Index `4140126`, stats: **251 files, 5947 nodes, 13743 edges, 300 processes, 226 communities, 28 workspace crates, 57 `cmd_*` CLI functions, 30 e2e + smoke tests under `nom-cli/tests/`.** All facts below are pulled from live Cypher queries against `.gitnexus/lbug`; where facts reference specific symbols, those symbols were confirmed to exist at the pinned commit.

### Already shipped (do not re-do)

| Path element | Evidence (GitNexus-cited) | Status |
|---|---|---|
| Concept/Module/Entry graph | `nom-concept::ConceptGraph` + `resolve_closure` walker + `visit_entity_ref` recursion + 300 processes | ✅ |
| DB1/DB2 dict + three-tier closure | `nom-dict::{find_word_v2, find_words_v2_by_kind, list_concept_defs_in_repo, count_concept_defs, count_required_axes}` + typed-slot resolution | ✅ |
| Resolver + MECE-ME + MECE-CE | `nom-concept::mece` + `check_mece_with_required_axes` + CE registry table | ✅ (M7a) |
| Manifest JSON handoff | `cmd_build_manifest` ([nom-cli/src/build.rs:436](../../nom-compiler/crates/nom-cli/src/build.rs)) + manifest roots | ✅ |
| Glass-box report | `cmd_build_report` ([nom-cli/src/report.rs:528](../../nom-compiler/crates/nom-cli/src/report.rs)) | ✅ (M1) |
| Acceptance-predicate preservation | `cmd_build_verify_acceptance` ([nom-cli/src/build.rs:500](../../nom-compiler/crates/nom-cli/src/build.rs)) + `acceptance_preserve_e2e.rs` | ✅ (M2) |
| Three-tier recursive ingest | `three_tier_recursive_e2e.rs` + recursive `visit_entity_ref` | ✅ (M4) |
| Layered dreaming (M5a+b+c) | `LayeredDreamReport` + `Cmd_app_dream` process + `layered_dream_smoke.rs` | ✅ (M5) |
| Locale packs | `nom-locale::{LocaleTag::parse, apply_locale, builtin_packs, normalize_nfc, is_confusable}` + 25 unit tests + `cmd_locale_{list,validate,apply,check-confusable}` | ✅ (M3a+M3b-min+M3c) |
| Self-host gates | 10 `self_host_*` test files: `parse_smoke`, `ast`, `codegen`, `parser`, `planner`, `verifier`, `pipeline`, `rust_parity`, `meta`, `smoke` — including `every_self_host_nom_file_compiles_to_bc` + `every_self_host_nom_has_its_own_acceptance_test` meta-gate + `run_lexer_bc_reproducibility_smoke.rs` SHA-256 pin | ✅ (M10a+M10b) |
| Media pipeline (AVIF) | `nom-media::ingest`, `cmd_media_import`, `cmd_store_add_media`, `avif_ingest.rs` + `bc_body_round_trip.rs` + `media_import.rs` tests | ✅ |
| Corpus infrastructure | 9 `cmd_corpus_*` (scan, ingest, ingest_parent, ingest_pypi, clone_batch, clone_ingest, register_axis, list_axes) + `nom-corpus::{checkpoint, equivalence_gate}` + 5-language translator (C/C++/Python/JS/Go) | ✅ scaffold (M7a dep) |
| `nom-intent` M8-slice1 (bounded-output discipline) | `NomIntent::{Kind, Symbol, Flow, Reject(Reason)}` + `LlmFn` closure type + 4 tests locking reject-on-not-in-candidates | ✅ 2026-04-14 |
| `nom-bench` family registry (Google-Benchmark pattern) | `BenchFamily` + `register` + `list` via `OnceLock<Mutex<Vec<_>>>` + 4 registry tests | ✅ 2026-04-14 |
| `nom-lsp` M16-slice1 (LSP transport + hover stub, lsp-server + lsp-types deps) | `dispatch_request` + `handle_hover` markdown reply "nom-lsp vX — hover stub alive" + `server_capabilities()` exposes hover_provider only + 4 unit tests locking dispatch correctness | ✅ 2026-04-14 |
| `nom-lsp` M16-slice2 (`nom lsp serve` CLI) | `Commands::Lsp { action: LspCmd::Serve }` shells stdin/stdout through `nom_lsp::serve_on_stdio()` | ✅ 2026-04-14 (commit `3ba982a`) |
| `nom-lsp` M16-slice3 (completion handler, 11 keywords) | `textDocument/completion` dispatches to 11 `.nomx v2` keywords (define/to/record/choice/when/unless/for/while/require/ensure/throughout) + 2 tests | ✅ 2026-04-14 (commit `0b68743`) |
| M7b standard axes seed | `NomDict::seed_standard_axes(repo_id)` registers correctness/safety/performance/dependency/documentation at app-scope; `nom corpus seed-standard-axes` CLI; 3 tests | ✅ 2026-04-14 (commit `647665b`) |
| CoreNLP W1 Annotator trait (nom-extract) | `trait Annotator { requires / requirements_satisfied / annotate }` + `Annotation` typed-key map + `AnnotationPipeline` runner + 5 tests | ✅ 2026-04-14 (commit `7caa41f`) |
| CoreNLP W1b `ParseAndExtractAnnotator` | Wraps `parse_and_extract` as Annotator impl; 3 new tests (8 nom-extract total) | ✅ 2026-04-14 (commit `9928187`) |
| CoreNLP W1c `ScanDirectoryAnnotator` | Wraps `scan_directory` as Annotator impl; 3 tests (11 nom-extract total) | ✅ 2026-04-14 (commit `b32abc0`) |
| Graph-durability Phase 1 (dict freshness) | `nom-dict::freshness` — `compute_source_hash` + `{current,stored,mark,is_stale}_source_hash` + `dict_meta` V5 table + 6 tests | ✅ 2026-04-14 (commit `60534e4`) |
| M8 slice-2 ReAct driver | `ReActStep / AgentAction / Observation / AgentTools / ReActBudget` + `classify_with_react` loop + `StubTools` + 7 tests | ✅ 2026-04-14 (commit `147939d`) |
| M8 slice-3a `DictTools::query` | First live AgentTools impl — hash-exact + kind-substring retrieval + max_results budget + 6 tests | ✅ 2026-04-14 (commit `9751aac`) |
| M8 slice-3c-partial `DictTools::explain` | 2nd live tool — `format_entry_summary(row, depth)` with one-line/multi-line variants + 4 tests | ✅ 2026-04-14 (commit `44f66d3`) |
| Graph-durability Phase 2a `NodeUid` | `compute_node_uid(entry)` + `NO_BODY_SENTINEL` null-byte-separator identity + 8 tests | ✅ 2026-04-14 (commit `2453375`) |
| Graph-durability Phase 2b `upsert_entry` | `UpsertOutcome::{Unchanged,Created,Updated,Renamed}` + `prior_hashes` chain + `history_of` + parallel `uid_nodes` storage + 8 tests | ✅ 2026-04-14 (commit `421f902`) |
| Graph-durability Phase 3a Cypher export (nodes) | `export_to_dir(graph, out_dir, force)` emits LadybugDB CSV + `import.cypher` LOAD FROM script; RFC 4180 escaping; deterministic sort; 5 tests | ✅ 2026-04-14 (commit `1b9cc00`) |
| M8 slice-5a `nom agent classify` CLI | Terminal-reachable ReAct loop: `Commands::Agent { AgentCmd::Classify }` with DictTools + stub LLM; 2 smoke tests | ✅ 2026-04-14 (commit `f169cd6`) |
| Graph-durability Phase 2b `upsert_entry` + rename chain | `UpsertOutcome::{Unchanged,Created,Updated,Renamed}` + `prior_hashes` chain + `history_of` + parallel `uid_nodes` HashMap storage; 8 tests | ✅ 2026-04-14 (commit `421f902`) |
| Graph-durability Phase 3a Cypher export (nodes) | `export_to_dir()` emits LadybugDB CSV + `import.cypher` LOAD FROM script; RFC 4180 escaping; deterministic sort; 5 tests | ✅ 2026-04-14 (commit `1b9cc00`) |
| Graph-durability Phase 3b edges export + REL TABLEs | `edges_<EdgeType>.csv` per variant; endpoint resolution via `word_variant_index`; CREATE REL TABLE scripts; 3 tests | ✅ 2026-04-14 (commit `910365c`) |
| M8 slice-3c-render-metadata `DictTools::render` | Closure walk → deterministic SHA-256 render-plan hash; 6 tests (idempotent across runs, differs on composite vs leaf) | ✅ 2026-04-14 (commit `07e6282`) |
| M8 slice-3b-verify `DictTools::verify` | 4 local invariant checks (code kinds require signature/body_kind, composites require composed_of, kind×body_kind consistency, hash integrity); 6 tests | ✅ 2026-04-14 (commit `a82caec`) |
| M8 slice-3b-compose `DictTools::compose` | Token-overlap scoring over retrieved context; kind→intent variant mapping; alphabetical tiebreak; 6 tests — **all 5 AgentTools live** | ✅ 2026-04-14 (commit `1ce0fc4`) |
| M8 slice-4 `InstrumentedTools` | Decorator pattern with forward-only LogEntry accumulation; duration_us attribution; drain+entries accessors; 6 tests | ✅ 2026-04-14 (commit `63ec6a4`) |
| Spec: slice-5b LLM adapter polymorphism | Design note — NomCliAdapter + McpAdapter + optional RealLlm all qualify; spec section added to `2026-04-14-graph-rag-agentic-design.md` | ✅ 2026-04-14 (commit `ebe530e`) |
| M8 slice-5b-trait `ReActAdapter` | Trait extraction + blanket impl for `Fn` closures (zero breaking change); 2 tests lock contract | ✅ 2026-04-14 (commit `078089e`) |
| M8 slice-5b-nom-cli `NomCliAdapter` | Compiler-as-oracle deterministic state machine over transcript; 11-transition coverage; 11 tests | ✅ 2026-04-14 (commit `b54a9c7`) |
| M8 slice-5b-cli-flag `--adapter {nom-cli,stub}` | Adapter selector in `cmd_agent_classify`; Box<dyn ReActAdapter> wrapped as ReActLlmFn; 2 new smoke tests (4 total) | ✅ 2026-04-14 (commit `b993de1`) |
| Multi-graph surface | `cmd_graph` ([nom-cli/src/main.rs:4562](../../nom-compiler/crates/nom-cli/src/main.rs)) → `NomtuGraph::{from_entries, build_call_edges, build_import_edges, detect_communities}` | ✅ |
| Foreign-source translator | `cmd_translate` + `nom-translate::{translate, translate_c_to_rust, translate_cpp_to_rust, translate_python_to_rust, translate_js_to_rust, translate_go_to_rust}` | ✅ |
| Author loop | `cmd_author_{translate, start, check}` + `author_check_handles_full_todo_app_nomx` + `.nomx v2` grammar fixtures (contracts, greet, loops, mixed_forms, hello, todo_app) | ✅ |
| Audit / security | `cmd_audit` + `CryptoPattern` + `SecurityFinding` + `redact_secret` + `SecurityReport` | ✅ scaffold (intentionally light — post-M1 concern) |
| MCP server | `cmd_mcp_serve` ([nom-cli/src/mcp.rs:22](../../nom-compiler/crates/nom-cli/src/mcp.rs)) + `mcp_smoke.rs` | ✅ |

### Actual remaining work (ordered by critical path)

#### Short-horizon (days — 1-2 wk)

1. **M3b-full: UTS #39 confusables.txt load.** Current: `is_confusable` ships with ~30 baked pairs (Cyrillic а, Greek α vs Latin a) plus NFC normalization. Missing: the **full official Unicode confusables table** (~6500 pairs). Blocked on network fetch (user todo #3). One day once data available; codec + test scaffolding already in [nom-locale/src/lib.rs](../../nom-compiler/crates/nom-locale/src/lib.rs).

2. **M7b: seed standard axes** ✅ SHIPPED 2026-04-14. `NomDict::seed_standard_axes(repo_id)` registers the canonical `correctness / safety / performance / dependency / documentation` set at app-scope with `at_least_one` cardinality; idempotent via INSERT OR REPLACE. `nom corpus seed-standard-axes --repo-id <id>` CLI subcommand ships alongside. 3 passing tests in `nom-dict`: canonical-five-axes, idempotent, scoped-per-repo-id. No M6 dep — authors can seed defaults today.

3. **Doc-reality gap follow-ups** (from the "Doc/reality gaps" section below): doc 04 §5.17 `nom-translate` re-wording + risk #10 `nom-graph` ownership + doc 03 M10b determinism citation. Half-day.

#### Medium-horizon (weeks)

4. **M6: PyPI-100 corpus pilot.** Every CLI piece exists (`cmd_corpus_ingest_pypi` + checkpoint + equivalence_gate with Rust translator). Missing: an actual network-fed run to populate `entries` table. User todo #4 — blocks on manual network access. **Hard gate** because M7b/M8/M9/M12/M13 all depend on a populated dict. Practical risk: a single ingestion bug on the first live run corrupts the dict; keep a clean backup before starting.

5. **M9: Phase-9 corpus-embedding re-rank.** Stub in `nom-cli/src/store/resolve.rs` (referenced in doc 04 risk #7). Needs `entry_embeddings` side-table + `find_words_v2_by_kind` rank composition `(score × similarity × provenance)`. Weeks + depends on M6 to have content to embed.

6. **M8: Intent-resolution transformer.** **M8-slice1 ✅ 2026-04-14**: `nom-intent` crate skeleton shipped with `NomIntent::{Kind, Symbol, Flow, Reject(Reason)}` enum + bounded-output validator + `LlmFn` closure type for deterministic tests + 4 passing tests proving the Reject-on-not-in-candidates discipline. The full M8 still needs (a) real LLM adapter (OpenAI/Anthropic structured-output), (b) ANN over DB2 `entry_embedding` (needs M6), (c) correction loop ← WrenAI `SQLCorrection` pattern. 3-6 weeks MVP, 2 quarters hardened. Deferred 05 §Hybrid specifies ~1B params + bounded output.

7. **M10c: compile-to-IR subset for self-host.** M10a+b gate parse + bc reproducibility; there's no IR-to-exec loop yet from self-host `.nom` sources. Weeks.

#### Long-horizon (quarters — true self-hosting chain)

8. **M10: real planner-in-Nom.** Currently `stdlib/self_host/planner.nom` parses clean and compiles to bc (M10a+b), but no `cmd_plan` consumes it. Real port = write planner logic as `.nomtu` compositions and plumb `nom plan <manifest.json>` through. Quarter+.

9. **M15: parser-in-Nom.** `stdlib/self_host/parser.nom` is the target file (gates in place). Quarter+ (doc 04 estimates 10-14 weeks).

10. **M16: LSP + Authoring Protocol.** `nom-lsp` crate doesn't exist. Doc 04 Phase 9 CORE, memory lists as CORE. Quarter (12-16 weeks).

11. **M17: fixpoint bootstrap (1.0 cut).** Stage0-Rust → Stage1 → Stage2 → Stage3 with `s2 == s3` byte-identical. Proof-of-bootstrap tuple recorded in dict. Requires M10 + M15 + M16 shipped first. Rust then archived to `.archive/rust-<version>/`. **This is the 1.0 gate — nothing else is.**

#### Post-1.0 (genuinely optional until M17)

12. **M11: multi-locale workspaces** (`locale <bcp47>` directive in parser). M3 infrastructure is in; no parser wire-up.
13. **M12: algebraic laws + dimensional analysis.** `laws: [...]` + `dimensions: [...]` fields on entries; verifier passes. Weeks + needs populated dict.
14. **M13: mass corpus** (top-500 PyPI + top 500/ecosystem GitHub). §5.17 full protocol run. Quarters + needs M6/M7b complete.
15. **M14: cross-domain Modelica connectors.** Needs M12 + M13.
16. **M18: closure-level specialization** (70-95% binary reduction). Needs M13 + M17.

### Critical-path answer in one line

**Nom 1.0 = M6 (weeks) → M9 (weeks) → M10 real (quarter) → M15 (quarter) → M16 (quarter) → M17 (quarter).** Everything else is either shipped, parallel, or post-1.0. The shortest honest time to 1.0 from today is **~18 months** if M6 starts soon and the self-hosting chain doesn't hit the rustc-style "10× underestimate" trap that doc 09 risk #3 warns about.

### What is NOT on the critical path (explicit non-blockers)

- **Every `cmd_*` CLI polish** — shipped surface already covers demo flows.
- **`nom-flow` + `nom-bench`** — remain scaffold (FlowArtifact + BenchmarkRun exist in graph; low inbound usage). Post-M13 work.
- **`nom-security` expansion** — `SecurityReport` is adequate for M1; ecosystem hardening is post-1.0.
- **Additional `.nomx` grammar fixtures** — 6 active fixtures (contracts, greet_sentence, loops, mixed_forms, hello, todo_app) sufficient for `.nomx v2` lock-in.
- **Vietnamese vocabulary reintroduction** — permanently rejected per ecd0609 + user memory; only VN grammar-style shapes Nom.


## Honest risks

1. **M6/M13 corpus ingestion is disk-stream-and-discard or bust.** Doc 04 §5.17 says peak disk = max per-repo-source + current-dict; any caching of intermediates blows the budget. Skip-lists + checkpointing + bandwidth throttle are non-optional. Risk: a sloppy ingestion run corrupts the dict.
2. **M8 intent transformer is the only probabilistic component.** Deferred 05 §Hybrid. If its output isn't hard-bounded to registered Nom kinds + validated against DB2, the whole "zero hallucination" thesis collapses. Must reject any token not in the registered output space.
3. **M10/M15/M17 self-hosting underestimated historically.** Doc 04 marks Phase 5 multi-week, user memory says "parked." Every self-hosting language (rustc took ~10 years to full parity) runs long. M10 being a quarter is aggressive.
4. **M11 multi-locale = immediate security surface.** Deferred 09 §11 flags confusable attacks (UTS #39), translation drift, tooling burden. Locale pack governance needs community review before expanding beyond VI/EN.
5. **M14 cross-domain laws are easy to write, hard to verify.** Deferred 11 §7.1 semantic aliasing: two Noms that look equivalent but differ in hidden assumptions. Conservation laws need airtight contract vocabulary or the verifier makes pretty but wrong claims.
6. **M3 Vietnamese positioning risk.** Vocabulary is fully English; VN keyword aliases and `agent_demo_vn` have been removed (ecd0609). Deferred 09 §11.5 cultural asymmetry warning: if Vietnamese provides deep grammar but English dominates visible tooling, system silently re-centralizes on English. M11 must genuinely ship non-English locales or the lingua-franca claim is hollow.
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

## Doc/reality gaps surfaced by GitNexus (2026-04-14)

GitNexus (code knowledge graph at `.gitnexus/`) was queried against every doc in `research/` to audit coverage. Three crates are **materially under-documented** — the graph shows active call chains while the docs treat them as name-only scaffolds or miss them entirely. Ranked by gap severity:

### Gap 1: `nom-graph` has **zero doc mentions**, but the crate is integrated end-to-end

Source evidence (all in `nom-compiler/crates/nom-graph/src/lib.rs`):

- `impl NomtuGraph` (line 72) — public graph type
- `NomtuGraph::from_entries` (line 84) — constructs graph from dict entries
- `build_call_edges` (line 130) — resolves CALLS edges
- `build_import_edges` (line 162) — resolves IMPORTS edges
- `detect_communities` (line 242) — Louvain-style clustering
- 5 unit tests (lines 577, 603, 635, 648, 671 — CALLS detection, rust-imports, communities, closures)

CLI surface: `cmd_graph` at [nom-cli/src/main.rs:4562](../../nom-compiler/crates/nom-cli/src/main.rs). This crate turns the dict into a queryable call/import graph on top of which the whole "Nom measures itself" story (doc 04 §§5.17, 5.18, 5.19) rides. Treat `nom-graph` as a **first-class peer** to `nom-concept` and `nom-extract`, not an extraction helper.

### Gap 2: `nom-translate` is a **real 5-language dispatcher**, not a name

Source evidence (`nom-compiler/crates/nom-translate/src/`):

- `translate(body, from_language)` dispatcher — [lib.rs:31](../../nom-compiler/crates/nom-translate/src/lib.rs)
- `translate_c_to_rust` — [c.rs:6](../../nom-compiler/crates/nom-translate/src/c.rs)
- `translate_cpp_to_rust` — [cpp.rs:9](../../nom-compiler/crates/nom-translate/src/cpp.rs)
- `translate_python_to_rust` — [python.rs:6](../../nom-compiler/crates/nom-translate/src/python.rs)
- `translate_js_to_rust` — [js.rs:6](../../nom-compiler/crates/nom-translate/src/js.rs)
- `translate_go_to_rust` — [go.rs:6](../../nom-compiler/crates/nom-translate/src/go.rs)

CLI surface: `cmd_translate` at [nom-cli/src/main.rs:4344](../../nom-compiler/crates/nom-cli/src/main.rs). This is the ingest path for doc 04 §5.17 mass-corpus work — every foreign-language function body that lands in the dict routes through this crate. Every doc in `research/` that discusses §5.17 should cite `nom-translate` as the dispatch layer.

### Gap 3: `nom-corpus` is **more than a skeleton**

Source evidence: `compile_nom_to_bc` at [nom-corpus/src/lib.rs:246](../../nom-compiler/crates/nom-corpus/src/lib.rs) wires `parse_source → plan_unchecked → nom-llvm::compile` into a single bc-returning function; determinism test at [nom-corpus/src/lib.rs:1060](../../nom-compiler/crates/nom-corpus/src/lib.rs). The §5.17 ingestion pipeline depends on this path being deterministic (byte-identical `.bc` for the same input), and the test locks that property.

### Under-reflected but not gaps

- `nom-security` — `SecurityReport` exists in graph, only 1 doc mention. Intentional: it is a post-M1 concern (risk #7 above).
- `nom-flow` / `nom-bench` — scaffold-level as the docs say; leave as-is until §5.16 benchmarks and §5.17 flow recording enter the path.
- `nom-diagnostics`, `nom-config` — single-doc mentions each, both are small infra crates, no action needed.

### Actions (follow-up commits)

1. Doc 04 §§5.17–5.19 should add a `nom-graph` subsection and cite `cmd_graph` as the live surface.
2. Every `nom-translate` reference that calls it "scaffold" should be replaced with "5-language dispatcher".
3. Doc 03 (self-hosting roadmap) should note that `compile_nom_to_bc` determinism is already pinned — this de-risks M10b.
4. New line in doc 04 risks: "Risk #10 — `nom-graph` ownership" (graph analysis surface exists but no doc lists its SLOs, invariants, or rebuild cost).

---

This doc will stay valid until the first milestone ships. When M1 lands, revise the "Where we are" paragraph and strike M1 from the path; leave the rest of the ordering untouched. Every re-derivation of the path should produce the same ordering from the same research inputs — that is itself a fixpoint check on the plan.
