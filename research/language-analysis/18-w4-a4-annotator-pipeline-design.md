# 18 — W4-A4 annotator pipeline design

**Date:** 2026-04-14
**Purpose:** Pin the refactor target for doc 13 §5 A4 — "annotator-style staged parser" — before writing any code. CoreNLP's `AnnotationPipeline.annotate()` contract inspires the design; every stage classifies every token or rejects the input. Captures the typed AST per stage, the stage boundary contracts, and the migration path from today's monolithic `parse_nom` / `parse_nomtu`.

> **Status 2026-04-14:** Design only — no refactor landed. W4-A4 stays ⏳ in doc 13 §5 until the first stage lands.

---

## 1. The refactor target

Today `nom-concept` has two monolithic entry points:

- `pub fn parse_nom(src: &str) -> Result<NomFile, ConceptError>` — multi-concept `.nom` files.
- `pub fn parse_nomtu(src: &str) -> Result<NomtuFile, ConceptError>` — multi-entity `.nomtu` files.

Each drives a single-pass recursive-descent parser over a `Lexer` token stream. Errors fire mid-traversal; there's no staged typed-AST between the token stream and the final structured result.

The target structure mirrors CoreNLP's pipeline:

```
source_text
  │
  ▼
 [S1] tokenize      : &str           → Vec<SpannedTok>
  │
  ▼
 [S2] kind_classify : Vec<SpannedTok> → ClassifiedStream
  │
  ▼
 [S3] shape_extract : ClassifiedStream → ShapedStream
  │
  ▼
 [S4] contract_bind : ShapedStream    → ContractedStream
  │
  ▼
 [S5] effect_bind   : ContractedStream → EffectedStream
  │
  ▼
 [S6] ref_resolve   : EffectedStream   → NomFile | NomtuFile
```

Each arrow is a pure function that **either** returns the next typed AST **or** returns `Err(ConceptError::StageFailure { stage: StageId, location, reason })`. No stage swallows an ambiguity.

## 2. Stage-by-stage typed ASTs

### S1 — Tokenize

Already exists as `Lexer::new + Lexer::next`. Keep it; just expose the full `Vec<SpannedTok>` rather than iterating lazily inside the parser. Payoff: stages S2-S6 can rewind and lookahead without re-lexing.

```rust
pub struct TokenStream {
    pub toks: Vec<SpannedTok>,
    pub source_len: usize,
}
```

### S2 — Kind classify

Walks the token stream, marking where each top-level block starts (`the concept …` / `the function …` / `the data …` / `the module …` / `the composition X …`). Bare top-level prose fails here.

```rust
pub struct ClassifiedStream {
    pub toks: Vec<SpannedTok>,
    /// One entry per top-level block. Block.kind is always a known
    /// `KINDS` value — kindless top-level prose rejects at this stage.
    pub blocks: Vec<BlockBoundary>,
}

pub struct BlockBoundary {
    pub kind: String,            // from KINDS
    pub name: String,            // the identifier after the kind noun
    pub start_tok: usize,        // index into toks
    pub end_tok: usize,          // exclusive
}
```

Stage failure codes: `NOMX-S2-kindless-block`, `NOMX-S2-unknown-kind`.

### S3 — Shape extract

Per block, extract the structural skeleton: signature phrase, body bullets, their pre-dot prose, etc. Every block MUST have an `intended to …` sentence (was also true today — ct14 smoke test locks it). Stage failure surfaces missing required parts before any finer walk.

```rust
pub struct ShapedStream {
    pub toks: Vec<SpannedTok>,
    pub blocks: Vec<ShapedBlock>,
}

pub enum ShapedBlock {
    Concept { name: String, intent: String, body_spans: Vec<(usize, usize)> },
    Function { name: String, signature: SignatureShape, body_spans: Vec<(usize, usize)> },
    Composition { name: String, composes_spans: Vec<(usize, usize)>, glue_span: Option<(usize, usize)>, body_spans: Vec<(usize, usize)> },
    Data { name: String, intent: String, exposes_spans: Vec<(usize, usize)> },
    // Module, Screen, Event, Media follow same pattern.
}
```

Stage failure codes: `NOMX-S3-missing-intent`, `NOMX-S3-empty-body`.

### S4 — Contract bind

Walks each block's body spans and extracts `requires` / `ensures` clauses into typed fields. Any prose between contract keywords that isn't itself a bracketed phrase fails here as "unclassified prose" — enforces the strictness discipline.

```rust
pub struct ContractedStream {
    pub toks: Vec<SpannedTok>,
    pub blocks: Vec<ContractedBlock>,
}

pub struct ContractedBlock {
    pub inner: ShapedBlock,
    pub contracts: Vec<ContractClause>,
    pub remaining_spans: Vec<(usize, usize)>, // for S5 + S6
}
```

Stage failure codes: `NOMX-S4-unclassified-prose`, `NOMX-S4-orphan-contract-verb`.

### S5 — Effect bind

Walks `remaining_spans` for `benefit` / `hazard` / `boon` / `bane` clauses (entity + composition blocks only — concepts emit `NOMX-S5-effect-on-concept` if an effect is found there).

```rust
pub struct EffectedStream {
    pub toks: Vec<SpannedTok>,
    pub blocks: Vec<EffectedBlock>,
}

pub struct EffectedBlock {
    pub inner: ContractedBlock,
    pub effects: Vec<EffectClause>,
    pub ref_spans: Vec<(usize, usize)>, // for S6
}
```

### S6 — Ref resolve

Final stage walks each block's `ref_spans` and builds the index tree of entity refs (typed-slot, v1 bare-word, with/without confidence threshold). Emits the final `NomFile` or `NomtuFile`.

Stage failure codes: `NOMX-S6-malformed-ref`, `NOMX-S6-kind-mismatch`.

## 3. Stage boundary contracts

1. **Every stage is total** over its input domain — it doesn't skip tokens. A stage either consumes all its allotted tokens and returns the next typed AST, or it rejects with a structured error that names the stage.
2. **Errors carry a stage id.** The `ConceptError` enum grows a `StageFailure { stage: StageId, ... }` variant. Editors use the stage id to distinguish "this is a tokenization issue" vs "this is a contract-phrasing issue".
3. **No stage is optional.** Callers that want a lighter pass (e.g., tokenize-only for syntax highlighting) use S1 alone; anything requiring structural meaning runs through S6.
4. **Stages are pure functions.** No shared mutable state between stages. A stage takes its input by reference and allocates its output fresh; this makes stages independently testable and parallelizable if we ever need it.
5. **`validate_strict` moves later.** The existing `nom_concept::strict` module (W4-A3) runs AFTER S6 as a separate opt-in validator over the final `NomFile` / `NomtuFile`. No change to its interface.

## 4. Migration path (three sub-wedges)

### A4a — TokenStream materialization (~0.5d)

Replace the `Lexer::next` iterator consumed by `parse_nom` / `parse_nomtu` with a pre-materialized `Vec<SpannedTok>` and a cursor. Zero behavior change; internal refactor only. Unlocks lookahead-based stages.

### A4b — Stage boundaries without typed ASTs (~1d)

Wrap the existing parser's code blocks in named helper functions `stage2_classify_blocks` / `stage3_shape_extract` / `stage4_contract_bind` / `stage5_effect_bind` / `stage6_ref_resolve`. Each helper still consumes mutable state (the cursor + accumulator) but now has a clear entry point and error attribution. Adds the `ConceptError::StageFailure { stage, … }` variant and threads it through. Zero behavior change; error messages now identify the stage that failed.

### A4c — Typed ASTs between stages (~1.5d)

Split the accumulator state: each stage takes the previous stage's typed output and returns its own. This is where the stage boundaries become real type-system boundaries. Implement incrementally — S2 first (easiest, just tracks block starts), then S4 (contracts), then S5 (effects), then S6 (refs), finally S3 (shape) which is the gnarliest.

Total: ~3 engineer-days as originally estimated.

## 5. Why this design

- **Error attribution** — today a parse error says "expected `the concept`, found `hazard` at position 150"; future error says "S2 kind-classify failed: effect keyword `hazard` outside an entity/composition block". Massive authoring-UX improvement.
- **LSP diagnostics** — editors get structured stage failures with `stage_id` + locational hint. Can surface squiggly underlines colored by stage (lexical / structural / semantic).
- **Concurrent-friendly** — stages are pure functions, so `parse_nom_dir(path)` can lex all files in parallel (S1), then classify all blocks in parallel (S2), etc.
- **Composable with `strict`** — the W4-A3 validator sits cleanly OUTSIDE the pipeline (runs on the final AST); no coupling to intermediate stages.
- **Matches CoreNLP's Annotator contract** — the user's NON-NEGOTIABLE directive (2026-04-14) called out CoreNLP-main as the strictness exemplar. The pipeline design mirrors `AnnotationPipeline.annotate(Annotation)` with `requires()` + `requirementsSatisfied()` contracts.

## 6. Non-goals

- **Not backwards-incompatible.** `parse_nom(src)` and `parse_nomtu(src)` keep their current signatures. The pipeline is the new internal implementation; the public API is unchanged.
- **Not a grammar expansion.** This wedge doesn't add new syntax (W5-W18 do that). It only restructures the existing parser.
- **Not a streaming parser.** We materialize the full token stream; memory is cheap and the files are small. Streaming is a future consideration if we ever parse multi-gigabyte `.nom` files (we won't).
- **Not auto-recovery.** A stage failure is terminal — we don't continue past errors to collect more diagnostics. Rust-analyzer-style error recovery is W19 if the demand surfaces.

## 7. Verification plan

Each of A4a / A4b / A4c lands with regression tests:

1. **All 94 existing nom-concept tests pass** — the pipeline must preserve every current parser behavior.
2. **New stage-boundary tests** — one per stage, asserting the error carries the right `StageId` for a crafted input that fails at that stage.
3. **Round-trip property** — `parse_nom(src) == parse_nom(serialize_back(parse_nom(src)))` for a corpus of the doc 14 translations, so the refactor doesn't accidentally regress any real-world shape.
4. **Benchmark** — the new pipeline must not be > 20% slower than the monolith on the doc 14 corpus. Stage overhead is the main risk.

## 8. Relation to A5

The A5 audit (doc 13 §5, committed 77eb636) surfaced `EntityRef.kind: Option<String>` as a soft spot. The ref-resolve stage (S6) is the natural place to introduce `EntityKindSlot::{Known, UnknownUntilLookup}` — a typed AST boundary would force every caller to decide which variant they construct. A4 and A5 can land together as one refactor if the A4c sub-wedge gets ambitious.

## 9. Cross-references

- [Doc 13 §5 A4](13-nomx-strictness-plan.md) — parent wedge description.
- [Doc 17 §I6](17-nom-authoring-idioms.md) — `intended to …` authoring idiom this pipeline enforces at S3.
- [Doc 16](16-nomx-syntax-gap-backlog.md) — W4 lane progress tracker.
- [`nom-compiler/crates/nom-concept/src/lib.rs:934`](../../nom-compiler/crates/nom-concept/src/lib.rs) — current `parse_composition_decl` (the monolithic baseline).
- [`nom-compiler/crates/nom-concept/src/strict.rs`](../../nom-compiler/crates/nom-concept/src/strict.rs) — W4-A3 validator that runs downstream of S6.
