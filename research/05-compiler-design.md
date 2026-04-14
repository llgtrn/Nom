# 05 — Compiler Design

## Pipeline

The compiler runs source through six staged passes, each pass adding one
layer of structure. Every pass is pure: input → output value, no shared
mutable state.

| Pass | Concern | Output |
|------|---------|--------|
| S1 tokenize | Surface lexing — recognize keyword tokens, identifier tokens, string + numeric literals, punctuation | TokenStream |
| S2 kind_classify | For every block, identify the declared kind (`function` / `module` / `concept` / etc.) | ClassifiedStream |
| S3 shape_extract | Pull out the structural shape (signature, exposes fields, generator) per kind | ShapedStream |
| S4 contract_bind | Pull `requires` / `ensures` clauses and bind them to the surrounding decl | ContractedStream |
| S5 effect_bind | Pull `hazard` / `favor` clauses (effect valence) | EffectedStream |
| S6 ref_resolve | Rewrite every typed-slot ref against the dictionary; pin `name@hash` | PipelineOutput |

## Strictness lane (W4)

Closed sub-wedges enforce the parser's reject-on-ambiguous discipline:

- A1 — entity refs MUST carry @Kind (in v2) or kind-noun (in v1)
- A2 — `matching` / `with` / `confidence` / kind-noun / `at-least` are
  case-sensitive exact-match
- A3 — typed-slot refs missing `with at-least N confidence` produce a strict
  warning
- A4 — the S1-S6 annotator pipeline is the canonical parse path
- A5 — refactor pending; current behavior matches v1 acceptance
- A6 — empty / whitespace-only input is a clean no-op, not an error

## Resolver

The `ref_resolve` pass walks each `uses the @Kind matching "..."` clause and
queries the dictionary for entities whose kind matches `@Kind` and whose
embedding-similarity to the prose intent exceeds the declared confidence
threshold. Below threshold, the build fails with a per-slot diagnostic
listing the top-K candidates and their scores.

The current resolver uses an alphabetical-smallest-hash tiebreaker as a
deterministic stub; the embedding-driven re-rank lands with the corpus
ingestion pipeline.

## Codegen

LLVM is the current backend target. The self-hosting lexer compiles
end-to-end through the LLVM pipeline. Future codegen work spans aesthetic
backends (image / audio / video / 3D / typography rendering) as separate
`body_kind` targets in the artifact store.

## Verifier

Per-pass invariants:

- S1: every byte of input is consumed; no orphan tokens.
- S2: every block has exactly one declared kind from the closed set.
- S3: every kind has all its required clauses (per `clause_shapes` table).
- S4: contract clauses terminate with `.` and do not cross into another
  clause keyword without a closing period.
- S5: effect-valence pairs (boon / hazard) consistent.
- S6: every typed-slot ref resolves to a hash, or the build fails.

## Bootstrap protocol (aspirational)

Two tracks run in parallel:

- **Fixpoint track**: Stage-0 (host-language compiler) compiles Stage-1 (Nom-
  authored compiler source). Stage-1 compiles Stage-2 (same Nom source).
  Stage-2 compiles Stage-3. The proof of Nom as a language is `s2 == s3`
  byte-identical, the same discipline used by self-hosting compilers in
  other ecosystems.
- **Parity track**: ≥99% IR equivalence + 100% runtime correctness on the
  test corpus between Stage-0 output and Stage-2 output, sustained for 4
  weeks. Then the default flips, host-language compiler enters 3-month
  grace, then archives.

The proof-of-bootstrap tuple `(s1_hash, s2_hash, s3_hash, fixpoint_at_date,
compiler_manifest_hash)` records permanently in the dictionary.
