# 07 — Authoring Patterns

## Mission

Nom's authoring patterns live in `grammar.sqlite.patterns` as queryable
rows. Patterns describe reusable shapes an author can drop into `.nomx`
source: tagged-variant errors, fault-tolerant supervision concepts,
property declarations with generators, given/when/then scenarios, etc.
Each row carries the intent (Nom vocabulary, name-free), the kinds and
clauses it uses, the typed-slot refs it needs, a parser-acceptable
example shape, known build-stage-checkable hazards, the quality axes it
optimizes, and the research docs it was distilled from.

## Current state

The `patterns` table schema is shipped as part of grammar.sqlite (see
doc 06). The `nom-grammar` crate ships only the schema + connection +
query API; it carries zero pattern data. After `nom grammar init`, the
patterns table is empty.

Population is the user's responsibility — via direct SQL, via future
row-level CLI commands (`nom grammar add-pattern`), or via SQL import
files. No `seed_patterns()` function exists in Rust source; bundling
data inside the binary is forbidden by the awareness-only rule.

## Target state

The patterns table holds the full catalog of authoring patterns useful
to Nom synthesis. Coverage axes:

- One row per data-shape pattern (tagged-variant errors, structural
  records, primitive payloads, composite messages)
- One row per concept-coordination pattern (fault-tolerant supervision,
  request/handler effect separation, throttling, multi-step pipelines,
  state machines)
- One row per property-class pattern (universal claims, generator-
  bounded domains, peer-lemma proof obligations)
- One row per scenario-class pattern (happy-path, edge-case, failure-
  mode triples)
- One row per parallel-class pattern (data-parallel reductions,
  distributed fan-out, work-stealing, locality-aware partitioning)
- One row per safety-class pattern (range-typed numerics,
  exhaustiveness-checked transitions, contract-bounded inputs)
- One row per stream-class pattern (windowed aggregation, watermark-
  aware, late-event-tolerant)

Every row must satisfy the grammar-for-synthesis quality bar:
1. The `example_shape` parses cleanly through the compiler
2. The pattern uniquely matches a class of intents (no duplicates)
3. The pattern produces an app, not just a parse-able file
4. Hazards are build-stage-checkable, not narrative warnings

## Pattern row schema

| Column | Type | Holds |
|---|---|---|
| `pattern_id` | TEXT PK | kebab-case Nom-native pattern name |
| `intent` | TEXT | what the pattern accomplishes (Nom vocabulary) |
| `nom_kinds` | TEXT JSON | which kinds the pattern uses |
| `nom_clauses` | TEXT JSON | which clause-shapes get used |
| `typed_slot_refs` | TEXT JSON | which @Kind refs the pattern needs |
| `example_shape` | TEXT | minimal parser-acceptable Nom-source example |
| `hazards` | TEXT JSON | build-stage-checkable pitfalls (Nom vocabulary) |
| `favors` | TEXT JSON | quality axes optimized (cross-references quality_names) |
| `source_doc_refs` | TEXT JSON | research docs distilled from (doc-numbers only) |
| `created_at` | TEXT | timestamp |

`idx_patterns_intent` indexes the intent column for resolver-style
similarity lookups.

## Integration with the resolver

When an author writes `uses the @Kind matching "<intent prose>"`, the
resolver consults the `patterns` table alongside `entities`. Pattern
rows whose `intent` similarity exceeds the confidence threshold are
surfaced as template suggestions; the author selects one and the build
stage emits the pattern's `example_shape` parameterized to the
surrounding context. The semantic-similarity backend is the embedding
index (planned, not yet shipped).

## Why patterns live in the DB

- A pattern catalog grows continuously as authors discover new shapes;
  rebuilding the binary on every catalog edit is the wrong cadence.
- The DB is queryable, auditable, backed up, edited independently of
  any code release.
- Hardcoded const arrays in Rust source duplicate the schema in two
  places — any drift is silent-bug territory.
- AI clients query the DB directly to determine intent → synthesis;
  the DB is the canonical source of truth for what authoring shapes
  exist.
