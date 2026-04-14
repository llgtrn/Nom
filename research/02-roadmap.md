# 02 — Roadmap

The roadmap is the canonical record of every planned, in-flight, and
shipped piece of Nom. Every pending and aspirational item from the
archived research must appear here; none may be silently dropped.

## Phase status

| Phase | Concern | State |
|------:|---------|-------|
| 0 | Workspace + crate scaffold | shipped |
| 1 | Lexer + parser host-language implementation | shipped |
| 2 | Resolver + verifier baseline | shipped |
| 3 | LLVM backend + self-hosting lexer compiles end-to-end | shipped |
| 4 | DIDS — content-addressed dictionary store | shipped |
| 5 | Body-only ingestion + multi-edge graph + intent resolution + lifecycle | in flight |
| 6 | Parser-in-Nom (Stage-1 of self-host) | planned |
| 7 | Resolver + verifier in Nom (Stage-2) | planned |
| 8 | Layered concept / module / entity architecture (three-tier) | shipped (architecture); ingestion in flight |
| 9 | Authoring protocol + LSP + grammar-registry-as-RAG | shipped (schema for grammar.sqlite + LSP slices 1–6); registry population is user's responsibility |
| 10 | Bootstrap (fixpoint Stage-0 → Stage-1 → Stage-2 → Stage-3 byte-identical) + retirement of host language | aspirational |
| 11 | Mathematics-as-language (algebraic laws + dimensional analysis) | aspirational |
| 12 | Closure-level specialization (per-platform binary minimization 70-95%) | aspirational |

## In-flight wedges

- **Grammar-registry row-level CLI** — surface for the user to populate
  grammar.sqlite tables directly. Targets: `nom grammar add-keyword`,
  `nom grammar add-kind`, `nom grammar add-clause-shape`,
  `nom grammar add-quality`, `nom grammar add-pattern`, plus
  `nom grammar import <sql-file>` for batch SQL. None shipped yet.
- **Dict-split S3b–S8** — port the remaining ~35 nom-dict functions
  to free functions on `&Dict`; delete the legacy `NomDict` struct, the
  legacy `entries` table, the legacy `concepts` table, and the
  V2/V3/V4/V5_SCHEMA_SQL constants in the same commit as the native
  replacement ships.
- **`.nomx` v1 + v2 merge** — single canonical source format keeping
  prose readability and typed-slot precision; v1-vs-v2 distinctions
  deleted from parser, tests, tooling.
- **`quality_names.metric_function` column** — currently nullable
  awaiting `nom corpus register-axis` CLI.
- **Strictness lane** — A1/A2/A3/A4/A6 closed; A5 audited with refactor
  pending.
- **Annotator-style staged pipeline** — S1 tokenize → S2 kind_classify
  → S3 shape_extract → S4 contract_bind → S5 effect_bind → S6
  ref_resolve. Shipped; delegate-migration unblocked.

## Aspirational mission-class items (preserved from archive)

- **100-repo corpus** — test/train the compiler against a 100-repo
  surveyed corpus; database carries function/concept/entity rows with
  placeholder stubs for missing items; crashes fixed before advancing.
- **Mass corpus ingestion** — top-N per ecosystem with stream-and-
  discard disk discipline (peak disk equals max per-source size + current
  dict; no source survives ingestion; skip-lists + checkpointing +
  bandwidth throttle non-optional). Most entries land Partial; lifted
  to Complete via canonicalization upgrades.
- **Aesthetic-as-programming** — media primitives composed via the same
  operators as functions; generative images / audio / video / 3D /
  typography rendering; aesthetic skills seeded.
- **AI-invokes-compiler authoring loop** — verify → build → bench →
  flow loop where the compiler is the deterministic oracle and the AI
  is replaceable.
- **Joint multi-app × multi-platform optimization** — bipartite min-
  cost assignment picking one specialization per (entity, platform)
  across all apps; benchmark-driven cost; cross-app specialization
  sharing automatic via content-addressing.
- **Two-track bootstrap protocol** — fixpoint track (Stage-0 →
  Stage-1 → Stage-2 → Stage-3, `s2 == s3` byte-identical) plus parity
  track (≥99% IR equivalence + 100% runtime correctness on test
  corpus).
- **Compiler retirement** — once self-hosting fixpoint holds for 4
  weeks, default flips to Stage-2; host-language compiler enters
  3-month grace period then archives.
- **Mathematics-as-language** — algebraic laws + dimensional analysis
  on function decls; cross-domain composition verification.
- **Universal knowledge composition** — closed kind set extends to
  cover scientific knowledge primitives via the same composition
  operators.

## Open wedge backlog

Wedges queued in the system but not yet shipped. Each needs design +
spec + parser/test work:

- Format-string interpolation surface
- Nested-record-path syntax (compound dot-paths)
- Sum-type `@Union` typed-kind (vs tagged-variant data decl)
- Wire-field-tag clause for serialisation surfaces
- Pattern-shape clause on data decls (regex-as-prose, closed vocabulary)
- Exhaustiveness check on `when` clauses over enum-valued data
- Retry-policy clause (orchestrator directive)
- Watermark clause for streaming-event-time correctness
- Window-aggregation clause (tumbling / sliding / session / global)
- Clock-domain clause for synchronous-logic decls (rising / falling edge)
- QualityName-registration formalization wedge

The wedge backlog table itself is queued to land in the DB (a future
`grammar_wedges` table) so it is queryable instead of prose-tracked.
