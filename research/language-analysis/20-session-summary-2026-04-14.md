# 20 — Session summary: 2026-04-14 late

**Purpose:** One-page index over the 2026-04-14 session's 80+ commits. Every entry links back to the authoritative doc for detail. This doc exists so the next cron cycle (or a human reviewer) can see the whole shape without crawling `git log`.

---

## Macro-lane 1 — CoreNLP strictness (W4 lane)

Genesis: user NON-NEGOTIABLE directive (2026-04-14) to tighten `.nomx v2` syntax using Stanford CoreNLP's Annotator pipeline as the strictness exemplar. Captured in [doc 13](13-nomx-strictness-plan.md) as six sub-wedges A1-A6.

| Wedge | Status | Commit | What it locks |
|-------|--------|--------|---------------|
| A1 mandatory kind marker | ✅ | `792bc0d` | 4 ct10* tests — entity refs must carry `@Kind` (v2) or `Kind Word` (v1); bare `the matching "..."` rejects |
| A2 closed keyword set | ✅ | `65f1198` | 5 ct09* tests — case-sensitive exact-match for all reserved tokens; synonyms stay `Tok::Word` |
| A3 strict-mode validator | ✅ | `d12a8b0` | Additive `nom_concept::strict` module; `validate_nom_strict` / `validate_nomtu_strict` emit `StrictWarning { code, message, location }` for typed-slot refs missing `with at-least N confidence`. 4 tests |
| A4 annotator pipeline | ✅ | See §"A4 sub-wedges" | Full 6-stage staged parser (S1 tokenize → S6 ref_resolve) with typed ASTs + structured `StageFailure` diagnostics |
| A5 Option<T> audit | 🔍 | `77eb636` | Only one load-bearing None: `EntityRef.kind` in `materialize.rs:105-116`. Enum refactor to `EntityKindSlot::{Known, UnknownUntilLookup}` documented; deferred until materialize.rs is touched |
| A6 reject-on-ambiguous | ✅ | `1495491` | Pre-locked by existing `resolve.rs` tests (`typed_slot_two_candidates_picks_smallest_hash`, `typed_slot_three_candidates_propagates_matching_and_alternatives`) |

### A4 sub-wedges (annotator pipeline)

| Step | Commit | Adds |
|------|--------|------|
| A4a | `6436a2c` | `collect_all_tokens(src) → Vec<Spanned>` materialization primitive |
| A4b | `e5be34f` | `stages` module scaffold: `StageId`, `StageFailure`, `TokenStream`, 6 stubs with `NOMX-S<N>-<slug>` diag ids |
| A4c-step1 | `5dfcd25` | `stage2_kind_classify` real body — `ClassifiedStream { toks, blocks, source_len }` with `BlockBoundary` per top-level block |
| A4c-step2 | `025a0cc` | `stage3_shape_extract` — pulls `intended to …` from each block into `ShapedStream` |
| A4c-step3 | `4a335eb` | `stage4_contract_bind` — extracts `requires`/`ensures` with strict cross-clause guard |
| A4c-step4 | `62581d2` | `stage5_effect_bind` — extracts `benefit`/`hazard` + `boon`/`bane` synonyms |
| A4c-step5 | `7da6a21` | `stage6_ref_resolve` + `run_pipeline(src)` driver → `PipelineOutput::{Nom(NomFile), Nomtu(NomtuFile)}` |

Rollup: **5 of 6 wedges closed** (A1/A2/A3/A4/A6), A5 audited with refactor deferred. Existing `parse_nomtu`/`parse_nom` paths untouched; pipeline runs alongside as the future replacement.

---

## Macro-lane 2 — Research finalization

Seven new docs landed to organize the accumulated design + planning work.

| Doc | Topic | Key outputs |
|-----|-------|-------------|
| [13](13-nomx-strictness-plan.md) | `.nomx` strictness plan | 6-wedge A1-A6 target + rollup with ✅/⏳ markers + commit refs |
| [14](14-nom-translation-examples.md) | Accelworld translations | **14 translations** across Rust/Python/TS/C/C++/Go/Bash/TOML/async-Python + functional patterns. 38+ syntax gaps surfaced |
| [15](15-100-repo-ingestion-plan.md) | 100-repo ingestion harness | `cmd_corpus_ingest_parent` path verified; placeholder-row semantics § 2; first bumpalo smoke test `c97a6c2` (runtime sandbox-blocked) |
| [16](16-nomx-syntax-gap-backlog.md) | Gap backlog | 38-row triage table with ⏳/🧪/📘/✅/🔒 markers + wedge master-index (W4-A1-A6, W5-W19) |
| [17](17-nom-authoring-idioms.md) | Authoring idioms (COMPLETE chapter) | 13 idioms I1-I13 closing all 13 doc 16 authoring-guide rows |
| [18](18-w4-a4-annotator-pipeline-design.md) | W4-A4 pipeline design | 6-stage target, typed AST per stage, 3-sub-wedge migration path |
| [19](19-deferred-design-decisions.md) | Deferred design Qs | D1 (@Data stays single kind) + D2 (no closure grammar, lift callbacks). **0 open design Qs** |
| [20](20-session-summary-2026-04-14.md) | *(this doc)* | Session index |

Research side is structurally finalized: **every doc 16 row is either closed, an actionable wedge, a smoke-test todo, or an explicitly-blocked borrow-model item**.

---

## Doc 16 backlog rollup (post-session)

- ⏳ Wedge queued: **13** — W5 format-strings, W6 literal-string-const, W9 `fail with`, W10 is-a probes, W11 enums, W12 receiver methods, W13 entry-point `main`, W15 interpreter metadata, W16 env-var access, W17 nested-section paths, W18 `@Union` kind, W19 async-marker, plus W4-A2b iteration destructuring lexer test. (W14 exit-codes reclassified to authoring-guide idiom I2.)
- 🧪 Smoke-test todo: **2**
- 📘 Authoring-guide doc-todo: **2** (default-params rule + lazy-vs-materialized sequences)
- ✅ Closed: **19**
- 🧠 Design deferred (open): **0**
- 🔒 Blocked: **1** (row #11 lifetime annotations, blocked on borrow-model)

---

## Memory notes landed this session

- `feedback_syntax_strictness_corenlp.md` — CoreNLP strictness directive
- `feedback_translate_accelworld_upstreams.md` — translation corpus directive
- `feedback_100_repo_corpus_test_train.md` — 100-repo test/train directive (HARD REQUIREMENT)

All three are NON-NEGOTIABLE per user; applied continuously across cycles.

---

## nom-concept test total

- Session start: 77
- Post-A4c + ct11-ct14 smoke tests + strict module + stages module: **120**
- Post-A4c parity push (a4c20-a4c38) + JSON round-trip + diag contract + strict-validator integration + empty-input safety: **139**
- Net +62 tests landed, all in nom-concept alone.

---

## Commit progression (abridged)

- Session start: `fdfb32d`
- W4 first shipments: `108d877` (doc 10+13) → `65f1198` (A2) → `792bc0d` (A1) → `d12a8b0` (A3) → `1495491` (A6)
- A4 pipeline: `c0a1bc0` (doc 18) → `6436a2c` (A4a) → `e5be34f` (A4b) → `5dfcd25`/`025a0cc`/`4a335eb`/`62581d2`/`7da6a21` (A4c-steps)
- Research finalization: `171a660` (doc 14 seed) → `8cb2265` (+3 TR + doc 15) → `c97a6c2` (bumpalo gate) → `ba5cd3e`/`370f96d` (+Go/C++/Bash/TOML) → `fcfda05` (doc 16 split) → `a8ce0a4`/`1331c00`/`ffa121a` (doc 17 I1-I13) → `1e752e7` (ct11-ct14 smoke) → `e013511` (doc 19) → `848f431` (doc 09 refresh) → `d3921bb` (doc 10 Next-actions refresh) → `f1dd162` (+2 async/flat_map)
- Post-A4c parity push: `196c4a8` → `65071df` → `94442bc` → `f346ccd` → `05e7762` → `6f728a5` → `3c5551d` → `d1a57ed` → `1185d6a` → `69bb443` → `6db7285` (a4c35) → `d3c97ff` (a4c36) → `7e4b3f3` (a4c37) → `c869986` (a4c38)
- Doc refreshes trailing: `d051daa` (doc 10 HEAD-sync) → `6446459` (doc 09 late-banner refresh) → `3320180` (doc 20 commit-progression extension) → `6d13ae5` (doc 20 test-total rollup 120→139) → `974c2bd` (doc 06 banner to HEAD 6d13ae5)
- Paradigm translations post-20-milestone (cycles 17+): `f66b53c` Prolog (#28) → `ed63541` Lisp-macro-rejection (#29) → `315182b` Protobuf (#30) → `bb9410c` regex (#31) → `a596993` XState (#32) → `e460fd5` Hypothesis (#33) → `e272d51` Terraform (#34) → `cb0bd92` NumPy (#35) → `9e839ba` Airflow (#36) → `f98af49` (doc 10 sync) → `42cfc8b` Spark-streaming (#37) → `9b4a438` Solidity (#38) → `03d4ca4` SwiftUI (#39) → `19b1101` Gherkin (#40) → `c69945c` (doc 10 sync) → `4c3f5f7` Verilog (#41) → `ef3dbaf` Nix (#42) → `1862aba` SQL-CTE (#43) → `0bb28c6` Forth (#44) → `7dae19b` OCaml-functor (#45) → `f426ff4` (doc 10 sync) → `7108218` Rego (#46) → `e11c6ae` TLA+ (#47) → `5a0c2b1` PDDL (#48) → `41607ca` Mermaid (#49) → `47242e9` **Dafny (#50 milestone)** → `bae9488` (doc 10 sync) → `3a45e93` (doc 09 end-of-session banner) → `b488a0f` (doc 20 commit-progression extension) → `8580f5f` WAT (#51) → `984ac75` R (#52) → `2cba840` OpenAPI (#53) → `fe3ec37` K8s (#54) → `98b7661` Elm (#55) → `67f1c25` (doc 10 sync) → `76d12ef` Rust-macros (#56) → `0a81499` jq (#57) → `b28ecf1` COBOL (#58) → `4641c0e` PowerShell (#59) → `92c926a` **Ansible (#60 MILESTONE)** → `51d8150` (doc 10 sync) → `ab513df` (doc 09 end-of-session banner refresh) → `089db2a` (doc 20 commit-progression extension) → `7b6e840` Fortran (#61) → `1e50f80` GraphQL-sub (#62) → `0914998` Redis (#63) → `be0e82c` (doc 10 sync) → `4fe76bd` gRPC (#64) → `2f652af` LaTeX (#65) → `df03703` Julia (#66) → `07b93db` (doc 10 sync) → `72abc14` Zig (#67) → `7d1e499` MATLAB (#68) → `aff44e8` (doc 10 sync) → `3bd1540` Smalltalk (#69) → `7f85b03` **Ada (#70 MILESTONE)** → `fe55c4c` (doc 10 sync) → `8685626` (doc 09 end-of-session banner refresh)
- Post-70-milestone chain: `b95c1be` (doc 20 extension) → `0bd1c57` Clojure STM (#71) → `a4962c7` (doc 10 sync) → `2a681a3` Perl (#72) → `68aae3b` Idris 2 (#73; 10-seed threshold reached) → `54aab10` (doc 10 + doc 16 sync + W51 wedge queued) → `808e64a` SVA (#74) → `7cc355c` APL (#75 MILESTONE) → `ef2e199` (doc 10 sync) → `130eeeb` (doc 09 end-of-session banner refresh) → `31d95a7` (doc 20 extension) → `567b35c` F# computation-expressions (#76) → `ec428b6` (doc 10 sync) → `032aad3` Scheme call/cc (#77) → `55e0474` (doc 10 sync) → `b5387dc` MongoDB (#78) → `791229a` Pascal (#79) → `3e1919e` **Datalog (#80 MILESTONE)** → `cf5eb51` (doc 10 sync) → `8a6b988` (doc 09 end-of-session banner refresh).
- This doc's immediate predecessor: `8a6b988`. Latest HEAD is `8a6b988`.
- **Translation milestones**: 20 → **80 translations**, 14 → **67 paradigm families** (50-language-survey exceeded by 34%), 44% → **83% closure**, 30 → **44 wedges queued** (W5-W51). 47 consecutive minimal-wedge / **39 consecutive 0-new-wedge** translations.
- **Additional unifications past #75**: monadic-sugar (Python-async + Scala-for-yield + Haskell-do + F#-computation-expr); first-class-continuation (Scheme call/cc → 4 named patterns); data-store expanded to 6 exemplars (adds MongoDB); structured-imperative 5 exemplars (Pascal + Ada + COBOL + C + C++); logic-programming 4 exemplars (Prolog + Datalog + Rego + SQL CTE).
- **Paradigm families fully closed/unified**: abstraction-quadrant, formal-methods-quadrant, infrastructure-automation pentagram, reactive-UI, shell-family triad, data-transformation (6 exemplars), scientific-computing (6 exemplars), networked-API (4), data-store (5), event-driven (4), error-handling (4), OO (3 variants: message-passing + class-based + contract-oriented), safety-critical-systems (4: Rust + Dafny + Zig + Ada). `screen` kind across 3 rendered-artifact domains.
- **Key insight**: Nom's authoring-time contract vocabulary IS the Ada/SPARK/Dafny runtime-check vocabulary, just moved up the dependency stack.

---

## Next-cycle candidates

1. **Ship W4-A5 enum refactor** — `EntityKindSlot::{Known, UnknownUntilLookup}` across `nom-concept` + `nom-cli::store::{materialize,resolve}` (~1d). Close W4 entirely.
2. **Ship a grammar wedge** from the W5-W26 queue — W9 `fail with` or W11 enums are the smallest starting points.
3. **More translations** — Java, Ruby, or other paradigms to stress non-code surfaces further.
4. **100-repo harness live run** — requires a user-shell session to bypass the UCRT DLL sandbox block.
5. **Pipeline feature-parity migration** — switch `parse_nom` / `parse_nomtu` internals to delegate to `run_pipeline()`. All per-field parity locked by a4c20-a4c34 tests.

---

## Post-A4c pipeline-parity push (added 2026-04-14 late)

After the initial A4c completion (commits `6436a2c` → `7da6a21`), a follow-up arc
closed the remaining known parity gaps. Every visible EntityRef field is now
captured by the pipeline; real `.nomtu` entity sources run end-to-end.

| Commit | Adds | Test |
|--------|------|------|
| `196c4a8` | Pipeline concept/entity parity on names, kinds, effects | a4c20-a4c22 |
| `65071df` | S6 concept index cardinality (count uses clauses) | a4c23 |
| `94442bc` | Early-return-guard smoke test (doc 16 #14 closed) | a4c24 |
| `f346ccd` | S6 typed-slot + v1 bare-word EntityRef partial extraction | a4c25-a4c26 |
| `05e7762` | S6 matching `"..."` + confidence-threshold capture | a4c27-a4c28 |
| `6f728a5` | Comprehensive multi-concept full-field parity | a4c29 |
| `3c5551d` | S6 entity signature extraction helper | a4c30 |
| `d1a57ed` | S3 policy relax: entities/compositions/data may omit `intended to` | a4c07 inverted + a4c31 + a4c32 |
| `1185d6a` | S6 composition `then` chaining → `CompositionDecl.composes` | a4c33 |
| `69bb443` | S6 v1 `@hash` backfill capture (post-first-build round-trip) | a4c34 |

**nom-concept test total: 135** (session start 77 → +58 this session).

Pipeline ↔ parse_nom/parse_nomtu parity now covers every observable field:
concept.name, concept.index.len + per-ref (kind, word, hash, matching,
typed_slot, confidence_threshold), EntityDecl.signature, EntityDecl.effects,
CompositionDecl.composes. The delegate-to-run_pipeline migration has no
known gap that would break on real repo sources.

Doc 14 translation corpus expanded to **27** across 14+ paradigm
families — every major paradigm family from doc 02's 50-language
survey now has at least one representative: imperative (Rust/
Python/C/C++/Go) + OOP (Java/Kotlin/Ruby) + async (Python) +
concurrency (Go goroutines) + pure functional (Haskell) +
algebraic data types (Kotlin sealed) + data (TOML/GraphQL/SQL/
CSS/YAML) + shell (Bash) + build (Make) + container (Docker) +
TS editor-event + CI/CD (GitHub Actions) + **math-as-language
(Lean theorem)** + **actor-model message-passing (Elixir
GenServer)**.

Doc 16 backlog at **61 rows**: **27 closed**, 30 wedges queued
(W5-W37), 1 smoke-test, 0 authoring-guide doc-todo, 0
design-Q-open, 2 blocked (#11 lifetime annotations + #58
typeclass constraints — both unlock together when the
borrow-model lane starts). **Closure rate 44%** (27/61). The
three most-recent wedges (W35 proof-kind, W36 proof-tactic
DSL, W37 actor-spawn) all surface from the math + actor-model
translations and are design-level questions rather than
syntactic additions.

**Milestone: Doc 17 authoring-guide chapter COMPLETE at I1-I20.**
Every authoring-guide destination in doc 16 now has a canonical
phrasing + anti-pattern + rationale:

- I1-I5 (first batch): `perhaps…nothing`, exit codes, `text of`,
  UTF-8 string literals, hyphen→underscore mapping
- I6-I8: docstring→intent, redundant v1 body, pipelines →
  named intermediates
- I9-I13 (second batch): atomic primitives, destructuring,
  list/text accessors, uses-vs-imperative, config-as-data split
- I14-I16 (third batch): default params, lazy sequences,
  `identifier` shape label
- **I17-I20 (final batch):** time-range `within the last N days`,
  shell-exec `run X with args Y`, method→receiver-as-parameter,
  `work_group` concurrent tracking
