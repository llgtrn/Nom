# 13 — `.nomx` strictness plan (CoreNLP-inspired)

**Date:** 2026-04-14
**Purpose:** Record the user's NON-NEGOTIABLE directive that Nom's `.nomx v2` grammar must become as strict as possible, using Stanford CoreNLP's Annotator pipeline as the exemplar. Pin down what "strict" means here so subsequent wedges have a concrete target.

> **Status 2026-04-14:** Directive received; no wedges shipped yet. This doc captures the audit scope and the first wedge candidates. Subsequent commits will cite §A1-A6 below as the items they close.

---

## 1. The directive (verbatim)

> more NON-NEGOTIATIONABLE that syntax have to be stricter as much as possible C:\Users\trngh\Documents\APP\CoreNLP-main and C:\Users\trngh\Documents\APP\nlp

## 2. What `APP/nlp` actually contains

Enumeration (2026-04-14):

```
APP/nlp/{1,2}/META-INF/
APP/nlp/{1,2}/edu/stanford/nlp/models/{coref,kbp,lexparser,ner,pos-tagger,sentiment,srparser}
```

Both top-level dirs are unpacked CoreNLP model JAR contents — `.ser.gz` classifiers and grammar tables, no source. This is NOT a second source repo to mine. **Interpret the directive as: CoreNLP-main is the canonical strictness source; APP/nlp is its trained-model companion.**

Runtime-asset note: two further model JARs (`stanford-corenlp-models-english-{extra,kbp}.jar`, ~670 MB) live in `C:\Users\trngh\Downloads\`. Per doc 10 §E these remain far-future JVM-interop candidates, not short-term wedge inputs.

## 3. Why CoreNLP is the right strictness exemplar

CoreNLP enforces a staged, typed-slot annotation contract that is structurally close to what Nom wants at parse time:

- **`Annotator` interface** (`src/edu/stanford/nlp/pipeline/Annotator.java:54`) — every stage declares `requires()` and `requirementsSatisfied()`. Missing requirement = pipeline refuses to run. No "best-effort" pass.
- **`AnnotationPipeline.annotate()`** (`src/edu/stanford/nlp/pipeline/AnnotationPipeline.java:27`) — stages run in declared order, each writes typed keys onto the shared `Annotation` map; downstream stages see exactly what predecessors annotated.
- **`CoreLabel`** — typed-slot data object; every annotated token carries kind metadata (POS, NER tag, lemma, ...). Mirrors the discipline we already use in Nom's `find_words_v2_by_kind` typed-slot resolver.
- **Properties-driven factory** (`StanfordCoreNLP`) — pipeline construction rejects unknown / misspelled properties up front.

The single most transferable pattern: **every stage classifies every token or the input is rejected**. Nom's v2 parser should emulate that; ambiguity at parse time becomes `ParseError::Ambiguous`, not a silent `Option::None`.

## 4. Audit scope

Files to audit for soft-paths (Option drops, unwrap_or_default, bare "best-effort" logic):

- `nom-compiler/crates/nom-lexer/src/nomx.rs` (542 lines)
- `nom-compiler/crates/nom-parser/src/nomx.rs` (1475 lines)
- the v2-keyed lookup in `nom-compiler/crates/nom-dict/src/lib.rs` (typed-slot resolver)

Audit grep baseline (2026-04-14):

- Parser: one `// lexer can't tell which — parser-side disambiguation on` at `nomx.rs:517`. No other matches for `unwrap_or_default|Option<Kind>|TODO|FIXME|best.?effort|ambigu|or_else\(\|\|\s*Ok`.
- Lexer: zero matches for `unwrap_or_default|TODO|FIXME|best.?effort|lenient|fallback`.

Interpretation: current strictness is already high. The wedges below tighten the remaining corners, not rip out a loose core.

## 5. Strictness wedges (ordered)

### A1 — Mandatory kind marker on every entity ✅

Every block-level entity MUST start with `@Kind` or be inside a container whose kind is statically inferred. Bare prose without a kind marker = `ParseError::MissingKindMarker`. Affects: `nom-parser/src/nomx.rs` block dispatcher. Estimated 1d.

**Shipped 2026-04-14 commit `792bc0d`:** 4 ct10* tests in nom-concept lock that `the matching "x"` / `the @Banana …` / `the login_user …` all reject; `the function X …` and `the @Function …` both pass.

### A2 — Closed keyword set audit ✅

Verify lexer rejects synonyms and case-variant spellings for `matching`, `with`, `at-least`, `confidence`, `the`, `a`, `an`, `that`. Add failing-unit tests for each forbidden variant (e.g., `Matching`, `match`, `matches`). Estimated 0.5d.

**Shipped 2026-04-14 commit `65f1198`:** 5 ct09* tests in nom-concept lock case-sensitive exact-match for all reserved tokens; synonyms like `match`/`matches` stay `Tok::Word`; lowercase `function` etc. canonicalize to `Tok::Kind`.

### A3 — Confidence threshold requirement on agentic resolvers ✅

`@Kind matching "..."` without `with at-least N confidence` should emit a strict-mode warning and in `--strict-mode` fail the parse. Opt-in today; default-on once downstream code is audited. Estimated 1d.

**Shipped 2026-04-14 commit `d12a8b0`:** purely-additive `nom_concept::strict` module. `validate_nom_strict(&file)` / `validate_nomtu_strict(&file)` walk the AST post-parse and emit `StrictWarning { code: "NOMX-A3", message, location }` for every typed-slot ref missing a confidence threshold. 4 tests (s01-s04); default parser unchanged.

### A4 — Annotator-style staged parser ⏳ (design spec landed)

Refactor `parse_nomx_source` into explicit stages: `tokenize → kind_classify → signature_extract → contract_attach → resolve_references`. Each stage takes and returns a typed AST (`TokenStream → ClassifiedAst → SignedAst → ContractedAst → ResolvedAst`). Every stage MUST classify every node or reject. Estimated 3d (biggest wedge; largest refactor risk).

**Design spec 2026-04-14:** [doc 18](18-w4-a4-annotator-pipeline-design.md) pins the 6-stage target (S1 tokenize → S2 kind_classify → S3 shape_extract → S4 contract_bind → S5 effect_bind → S6 ref_resolve), the typed AST per stage, the stage-boundary contracts, the 3-sub-wedge migration path (A4a tokenstream materialize ~0.5d / A4b stage boundaries without typed ASTs ~1d / A4c typed ASTs between stages ~1.5d), and how A5 ties in. Implementation can now proceed in small cycles.

### A5 — No lossy `Option` fields on typed AST 🔍 (audit complete, refactor deferred)

Every `pub struct` in the nomx AST audited for `Option<T>` fields that represent "we could not determine this"; replace with required `T` or with explicit `Unresolved` variants. Estimated 1d.

**Audit 2026-04-14 (no code change this cycle):**

| Field | Form | Verdict |
|---|---|---|
| `CompositionDecl.glue: Option<String>` | syntactic optional (`with <glue>` clause) | ✅ keep |
| `EntityRef.kind: Option<String>` | **LOSSY** — parser always sets `Some(k)`, but [`nom-cli/src/store/materialize.rs:105-116`](../../nom-compiler/crates/nom-cli/src/store/materialize.rs) sets `kind: None` when reconstructing compositions from a `composed_of` JSON blob (kind unknown until a DB lookup is done) | ⚠️ tighten |
| `EntityRef.word: String` | empty-string sentinel when `typed_slot = true` | ⚠️ known — `typed_slot` flag disambiguates, but empty-string is a soft failure mode |
| `EntityRef.hash: Option<String>` | pre-first-build unresolved | ✅ keep |
| `EntityRef.matching: Option<String>` | syntactic optional | ✅ keep |
| `EntityRef.confidence_threshold: Option<f64>` | syntactic optional (A3 policy chooses whether to warn) | ✅ keep |

**Recommended refactor (deferred):** replace `EntityRef.kind: Option<String>` with an explicit enum:

```rust
pub enum EntityKindSlot {
    /// Parser-assigned kind: one of the closed `KINDS` set.
    Known(String),
    /// Reconstructed from a hash without doing a kind lookup; callers
    /// that need the kind must resolve via `find_words_v2_by_hash`.
    UnknownUntilLookup,
}
```

This makes the "we don't know yet" case **explicit and self-documenting**, rather than sharing the `Option::None` shape with "this field was omitted at parse time". The change is ~1 engineer-day across `nom-concept` + `nom-cli::store::materialize` + `nom-cli::store::resolve` + any other callsite that pattern-matches `.kind`. Deferred until a natural window (e.g., when `materialize.rs` gets refactored for the 100-repo ingestion harness).

**Current state is NOT a correctness bug** — all consumers either set `Some(k)` or construct `None` only in materialize and recover the kind before use. The audit upgrades A5 from "possibly broken" to "known soft spot with a documented fix path". No behavior changes until the refactor lands.

### A6 — Reject-on-ambiguous in the dict resolver ✅ (already locked)

`find_words_v2_by_kind` already uses alphabetical tiebreak on equal confidence. Audit the callers to ensure a genuine ambiguity (multiple candidates above threshold with identical score) surfaces as `ResolverError::Ambiguous` rather than silently picking. Estimated 0.5d.

**Audited 2026-04-14 (no commit needed):** the existing tests `typed_slot_two_candidates_picks_smallest_hash` and `typed_slot_three_candidates_propagates_matching_and_alternatives` in [nom-cli/src/store/resolve.rs](../../nom-compiler/crates/nom-cli/src/store/resolve.rs) already pin the intended behavior — N candidates yield the alphabetically-smallest hash, `stats.ambiguous += 1`, and the `alternatives` field carries the rejected candidates. **The design decision is NOT "reject hard on ambiguity"** (that would break the §10.3.1 fixpoint discipline — the compiler must be deterministic over dict state) but "surface ambiguity via structured report fields". The current `ResolveStats { resolved, still_unresolved, ambiguous }` + per-ref `alternatives` + planned `nom build status` diagnostic (doc 07 §3.3) is the canonical reporting path. No new code needed.

Total: ~7 engineer-days planned; **4 of 6 wedges closed** (A1, A2, A3, A6); **A5 audit complete** with recommended refactor deferred. Remaining: A4 (~3d annotator-pipeline refactor, largest) + A5 refactor (~1d when materialize.rs gets touched by the 100-repo harness).

## 6. Vocabulary invariant (unchanged)

The strictness lane MUST preserve the existing Vietnamese-GRAMMAR-English-VOCAB invariant (`feedback_vn_grammar_not_vocab.md`, commit `ecd0609`). CoreNLP's strictness lives in HOW tokens are classified, not in WHICH vocabulary is used. No VN tokens get reintroduced as part of this work.

## 7. Relation to existing roadmap

- Doc 09 §"Queued wedges" grows one new category: **W-strictness (A1-A6)**.
- Doc 10 §"Next actions" grows one bullet referencing this doc.
- `memory/feedback_syntax_strictness_corenlp.md` captures the permanent directive.

## 8. Verification

Acceptance = (a) each wedge ships with a failing-before-passing-after test; (b) existing 300+ Nom tests continue to pass; (c) `npx gitnexus analyze` after each push keeps the graph current.

No cross-cutting "pilot against CoreNLP outputs" test — the model-JAR data in `APP/nlp/` is not a test oracle for Nom (different languages, different scope). CoreNLP supplies the structural contract, not the ground-truth.
