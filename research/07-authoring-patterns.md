# 07 — Authoring Patterns

The native authoring patterns live in `grammar.sqlite.patterns`, not in this
document. Query them with `nom grammar seed && sqlite3 ~/.nom/grammar.sqlite
"SELECT pattern_id, intent FROM patterns ORDER BY pattern_id;"`.

This file records the **mission** for the pattern catalog: every pattern
useful to Nom authoring is captured in `patterns` rows expressed entirely
in Nom's vocabulary. Foreign-language names are absent by invariant. The
goal is 100% preservation per the studied-pattern preservation rule, after
which the source documents in `.archive/` are deleted.

## Current state

Ten founding native patterns shipped, populated by
`nom_grammar::seed::seed_patterns`:

1. `tagged-variant-error-data`
2. `fault-tolerant-supervision-concept`
3. `property-with-generator-and-peer-lemmas`
4. `given-when-then-scenario`
5. `request-handler-effect-separation`
6. `composition-pipeline-with-named-intermediates`
7. `quantified-rate-limit-contract`
8. `closed-state-machine-with-per-state-transitions`
9. `data-parallel-reduction-with-associativity`
10. (founding row 5, kept canonical)

Each row carries:
- `intent` — what the pattern accomplishes (Nom vocabulary, name-free)
- `nom_kinds` — which kinds the pattern uses
- `nom_clauses` — which clause-shapes get used
- `typed_slot_refs` — which @Kind refs the pattern needs
- `example_shape` — parser-acceptable Nom-source template
- `hazards` — known build-stage-checkable pitfalls
- `favors` — quality axes the pattern optimizes (cross-references quality_names)
- `source_doc_refs` — research docs distilled from (doc-numbers only)

## Target state

Approximately 100-150 unique Nom-native pattern rows covering every
captured insight from the archived translation corpus. Coverage axes:

- One row per data-shape pattern (tagged-variant errors, structural
  records, primitive payloads, composite messages)
- One row per concept-coordination pattern (supervision, request-handler
  separation, throttling, multi-step pipelines, state machines)
- One row per property-class pattern (universal claims, generator-bounded
  domains, peer-lemma proof obligations)
- One row per scenario-class pattern (happy-path, edge-case, failure-
  mode triples)
- One row per parallel-class pattern (data-parallel reductions, distributed
  fan-out, work-stealing, locality-aware partitioning)
- One row per safety-class pattern (range-typed numerics, exhaustiveness-
  checked transitions, contract-bounded inputs)
- One row per stream-class pattern (windowed aggregation, watermark-aware,
  late-event-tolerant)

## Mission

Each cycle distills one to five additional patterns from the archive into
native rows. The pattern row is rejected unless:

1. The `example_shape` is parser-acceptable (run through the parser before
   seeding)
2. The pattern uniquely matches a class of intents (resolver picks this row,
   not a near-duplicate)
3. The pattern produces an app, not just a parse-able file
4. Hazards are build-stage-checkable, not narrative

Once the catalog covers every pattern that the archive's translations
captured, archive files delete per the no-legacy rule.

## Integration with the resolver

When an author writes `uses the @Kind matching "<intent prose>"`, the
resolver consults the `patterns` table alongside `entities`. Pattern rows
whose `intent` similarity exceeds the confidence threshold are surfaced as
template suggestions; the author selects one and the build stage emits the
pattern's `example_shape` parameterized to the surrounding context.
