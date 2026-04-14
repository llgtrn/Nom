# 09 — Open Questions

Active design questions that are not yet decided. Every entry is a
question, not a status update; status lives in the mission-checklog.

## Resolver

- The resolver currently uses an alphabetical-smallest-hash tiebreaker as a
  deterministic stub. The replacement is embedding-driven semantic re-rank.
  Open: what embedding model satisfies the deterministic-build requirement
  (the model's outputs must be byte-reproducible across builds and machines)?
- Per-kind embedding indexes (one per closed kind) versus a single combined
  index — which scales better for a 10^8-row dictionary?
- Confidence threshold default — strict mode requires `with at-least N
  confidence`, but what should N default to when omitted in a permissive
  build? Current draft: 0.6.

## Wedges queued

The following wedges are queued in the grammar registry but not yet
shipped. Each needs design + spec + parser/test work:

- Format-string interpolation surface
- Nested-record-path syntax (compound dot-paths)
- Sum-type `@Union` typed-kind (vs. tagged-variant data decl)
- Wire-field-tag clause for serialisation surfaces
- Pattern-shape clause on data decls (regex-as-prose with closed vocabulary)
- Exhaustiveness check on `when` clauses over enum-valued data
- Retry-policy clause (orchestrator directive)
- Watermark clause for streaming-event-time correctness
- Window-aggregation clause (tumbling / sliding / session / global)
- Clock-domain clause for synchronous-logic decls (rising/falling edge)
- QualityName-registration formalization wedge

## Bootstrap

- The fixpoint requires byte-identical Stage-2 and Stage-3 outputs. Open:
  which compiler outputs exactly are compared — the LLVM bitcode? the
  manifest hash? the artifact-store closure?
- The parity track requires ≥99% IR equivalence on the test corpus. Open:
  what test corpus, of what minimum size, is the parity bar measured
  against?
- After the default flips to the Nom-authored compiler, the host-language
  compiler enters a 3-month grace period. Open: during grace, are both
  compilers produced for every release, or only on parity-test failures?

## Concept layer

- Concept staleness propagation — when an upstream concept changes, the
  downstream concepts that index it become stale. Open: should the
  staleness propagate eagerly (sync time) or lazily (next-build time)?
  Current direction: dream-mediated rather than sync-mediated.
- Singleton enforcement — the app's one DB / one API / one auth provider
  must not be duplicated across concepts. Current direction: model as
  registered axes in the corpus with `cardinality = exactly_one_per_app`,
  enforced by the MECE validator.

## Authoring loop

- The authoring loop progresses brainstorm.md → .nomx → built artifact.
  Open: at what threshold does the brainstorm content trigger automatic
  parse attempts? Current draft: when ≥80% of sentences match the closed
  keyword vocabulary.
- Glass-box reports — every build emits a report explaining which entities
  resolved, which thresholds were met, which hazards fired. Open: format
  (HTML / SARIF / Nom-native artifact)?

## Pattern catalog

- The catalog has crossed the original 100-150 target and now sits
  at **258 rows** spanning 22 themes. Both halves of the completion
  bar are **enforced**, and the uniqueness half is enforced at two
  layers (exact-string + fuzzy):
  - `every_pattern_intent_is_distinct` — exact-string distinct
    (258 distinct intents across 258 rows)
  - `every_pattern_intent_pair_jaccard_below_threshold` — fuzzy
    token-overlap distinct (every pair of normalized domain-word sets
    shares less than 50% of their union; the catalog's observed max
    is 0.273 against a 0.5 threshold, so there is ~2× headroom)
  - `pipeline_never_panics_on_any_example_shape` +
    `pattern_example_shapes_dashboard` — every `example_shape` parses
    end-to-end (258/258)
  The fuzzy check uses a deterministic Jaccard backend (no embedding
  model, no network, no nondeterministic dependency) so the result
  is byte-stable across runs and machines. When embedding-driven
  re-rank lands later, a parallel test with a semantic backend can
  run alongside; the Jaccard test stays as the deterministic floor.

## Bench + flow

- Benchmark canonicalisation — entities with different bodies but
  identical benchmark profiles should be resolvable as one canonical
  pick per platform. Open: what tolerance distinguishes "identical
  enough"?
- Flow-step recording — every execution flow is captured as a tree of
  flow-step rows. Open: how is flow capture toggled (per-build, per-test,
  always)?
