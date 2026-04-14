# 02 — Roadmap

The roadmap is the single source of truth for what is shipped, in flight, and
queued. Every pending and aspirational item from the archived research must
appear here or in the mission-checklog (doc 08); none may be silently dropped.

## Phase status

| Phase | Concern | State |
|------:|---------|-------|
| 0 | Workspace + crate scaffold | shipped |
| 1 | Lexer + parser host-language implementation | shipped |
| 2 | Resolver + verifier baseline | shipped |
| 3 | LLVM backend + self-hosting lexer compiles end-to-end | shipped |
| 4 | DIDS (deterministic identity / dictionary store) — content addressed | shipped |
| 5 | Body-only ingestion + multi-edge graph + intent resolution + lifecycle | in flight |
| 6 | Parser-in-Nom (Stage-1 of self-host) | planned |
| 7 | Resolver + verifier in Nom (Stage-2) | planned |
| 8 | Layered concept / module / entity architecture (three-tier) | shipped (architecture); ingestion in flight |
| 9 | Authoring protocol + LSP + grammar-registry-as-RAG | shipped (P1-P3 grammar registry; LSP slice 1-6); pattern catalog in flight |
| 10 | Bootstrap (fixpoint Stage-0 → Stage-1 → Stage-2 → Stage-3, byte-identical) + retirement of host language | aspirational |
| 11 | Mathematics-as-language (algebraic laws + dimensional analysis) | aspirational |
| 12 | Closure-level specialization (per-platform binary minimization 70-95%) | aspirational |

## In-flight wedges (granular)

- **Pattern migration** — distill every captured insight from archived doc 14
  + doc 16 into native rows in `grammar.sqlite.patterns`. 10 of estimated
  ~100-150 founding rows shipped. Per-cycle cadence; archived sources delete
  only after 100% preservation verified.
- **Dict-split S3b–S8** — port remaining ~35 nom-dict functions to free
  functions on `&Dict`; delete legacy `NomDict` struct, legacy `entries`
  table, legacy `concepts` table, V2/V3/V4/V5_SCHEMA_SQL constants in the
  same commit as native replacement ships per the no-legacy rule.
- **`.nomx` v1 + v2 merge** — single canonical source format keeping prose
  readability + typed-slot precision; v1 vs v2 distinctions deleted from
  parser, tests, tooling.
- **Grammar-registry placeholder fixes** — `quality_names.metric_function`
  remains nullable until `nom corpus register-axis` ships; `kinds.allowed_*`
  populated from `clause_shapes` + `ALLOWED_REFS_FOR_KIND` derivation.
- **CoreNLP-inspired strictness lane (W4)** — A1/A2/A3/A4/A6 closed; A5
  audited with refactor pending.
- **Annotator-style staged pipeline** — S1 tokenize → S2 kind_classify → S3
  shape_extract → S4 contract_bind → S5 effect_bind → S6 ref_resolve shipped;
  delegate-migration unblocked.

## Aspirational mission-class items (preserved from archive)

- **100-repo corpus** — test/train the compiler against 100 repos from
  surveyed corpora; database carries function/concept/entity rows with
  placeholder stubs for missing items; crashes fixed before advancing.
- **Mass corpus ingestion** — top-N per ecosystem with stream-and-discard
  disk discipline (peak disk equals max per-source size + current dict; no
  source survives ingestion; skip-lists + checkpointing + bandwidth throttle
  non-optional). Most entries land Partial initially; lifted to Complete via
  canonicalization upgrades.
- **Aesthetic-as-programming** — media primitives composed via the same
  operators as functions; generative images / audio / video / 3D /
  typography; aesthetic skills seeded.
- **AI-invokes-compiler authoring loop** — verify → build → bench → flow
  loop where the compiler is the deterministic oracle and the AI is
  replaceable.
- **Joint multi-app × multi-platform optimization** — bipartite min-cost
  assignment picking one specialization per (entity, platform) across all
  apps; benchmark-driven cost; cross-app specialization sharing automatic via
  content-addressing.
- **Two-track bootstrap protocol** — fixpoint track (Stage-0 → Stage-1 →
  Stage-2 → Stage-3, `s2 == s3` byte-identical = the proof of Nom as a
  language) plus parity track (≥99% IR equivalence + 100% runtime
  correctness on test corpus).
- **Compiler retirement** — once self-hosting fixpoint holds for 4 weeks,
  default flips to the Nom-authored compiler; host-language compiler enters
  3-month grace period then archives.
- **Mathematics-as-language** — algebraic laws + dimensional analysis on
  function decls; cross-domain composition verification.
- **Universal knowledge composition** — closed kind set extends to cover
  scientific knowledge primitives via the same composition operators.

## Open wedge backlog (~44 grammar wedges)

Tracked as queued rows in `grammar.sqlite` once the wedge-tracking schema
lands; placeholder list lives in archive doc 16. Examples: format-string
interpolation, nested-record-path syntax, sum-type @Union typed-kind,
wire-field-tag clause, exhaustiveness check on `when` clauses, retry-policy
clause, watermark + window-aggregation clauses, clock-domain clause,
QualityName-registration formalization wedge.
