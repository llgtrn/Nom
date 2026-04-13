# Keyed-Similarity Syntax & Philosophy — A Nom Redesign Proposal

**Status: Draft, AI-synthesized, requires human authoring per §6.**
Filed 2026-04-13. Companion to [05-natural-language-syntax.md](./05-natural-language-syntax.md) and [06-nomx-keyword-set.md](./06-nomx-keyword-set.md).
External input: *"Similarity and Relevance in Multiscreen"* (arXiv 2604.01178v1, §3.1–3.2) — reachable on 2026-04-13.

---

## 1. Insight from the paper

The Multiscreen paper makes one architectural claim that cuts straight to Nom's grammar-vs-vocabulary tension:

> "The query–key dot product defines a similarity in the range [−1, 1], since each query and key vector is normalized to unit length. This similarity is then independently thresholded and transformed to produce relevance values in the range [0, 1], **without normalization across keys**."
>
> "Because the relevance αᵈᵢⱼ is not normalized to sum to one, the screening unit can also represent the absence of relevant context."

Two ideas follow:

1. **Per-key relevance, not per-query softmax.** A match is scored against each key on its own scale. No global competition.
2. **Absence is representable.** A query may legitimately match *nothing*, and the system can say so rather than picking the least-bad option.

Map onto Nom:

| Multiscreen | Nom |
|---|---|
| Key | `EntryKind` (Function, Concept, Skill, AppManifest, MediaUnit, UxPattern, Screen, Query, DataSource, Benchmark, …) |
| Key-local scoring scale | Each kind has its own metric space — a similarity between two `Function` contracts is not on the same axis as a similarity between two `MediaUnit` perceptual hashes |
| Query | A typed slot in a `.nom` source position |
| "No normalization across keys" | **Cross-kind retrieval is keyed, not flat-vector.** An author asking for "the Function most like X" and "the MediaUnit most like X" gets two independent answers on two independent scales; neither answer "beats" the other |
| "Absence of relevant context" | A typed slot may resolve to *no* nomtu — the compiler reports "no Function in the dict within threshold for this prompt" instead of silently binding something wrong |

Nom's dict already has this structure at the *data* level — 25+ `EntryKind` variants, each with its own edge family and metric surface. What the syntax has missed is exposing the **slot's kind** at the author's surface.

## 2. Critique of `.nomx` as it stands

What `.nomx` (doc 05) gets right:

- **Prose readability.** `define X that takes Y and returns Z` is a sentence a non-programmer can re-tell.
- **Last-sentence-is-result.** Eliminates the `return` ceremony.
- **Zero-mistake phrasing.** `a maybe-text`, `either X or an error`, `when given …, ensure …` are Ada+Haskell guarantees in English.
- **Two-track migration.** `.nomx` coexists with `.nom`, both lower to the same AST.

What it misses, once the keyed-similarity lens is applied:

1. **All references are untyped name lookup.** `the name from the profile` assumes a flat identifier space. In practice Nom's dict is 25+ kind-separated spaces; `name` might be a field of a Struct, the label of a Concept, the word of a Skill, a caption of a MediaUnit, or a proper-noun AppVariable. The grammar can't distinguish these without running type inference, which means error messages for bad references are always post-hoc.
2. **No surface form for retrieval.** If the author wants "the Function most similar to this prose description" (the natural authoring mode for a two-tier AI-assisted language, per §language-model framing of doc 04), the syntax has no form for it. They fall back to hash-pin or nomtu-word, which defeats the AI bridge.
3. **No "no match" syntax.** Because retrieval is implicit in name lookup, there is no way for a source to express "accept no candidate if none clears the threshold." The language implicitly picks the top-ranked, which is the softmax trap the Multiscreen paper calls out.
4. **The C-to-prose move stopped one level short.** Doc 05 replaced `fn`/`struct`/`enum`/`return` with prose. It did not replace the *reference idiom* — dotted paths, bare identifiers, global namespace. Reference is where the dictionary is actually used, and reference is still C-shaped.

The redesign: **keep the sentence grammar of `.nomx`, but make every reference carry its kind.**

## 3. Proposed syntax

### 3.1 The typed-slot reference

A reference in Nom source may take three forms, in increasing specificity:

```
<slot> ::= <typed-slot> | <word-ref> | <hash-pin>
<typed-slot>  ::= "the" <kind> "matching" <prompt>
                | "a"   <kind> "matching" <prompt>
                | "any" <kind> "matching" <prompt>
<word-ref>    ::= <word> ( "@" <hash-prefix> )?   -- unchanged from .nomx
<hash-pin>    ::= <word> "@" <full-hash>          -- unchanged
<kind>        ::= "Function" | "Concept" | "Skill" | "Screen"
                | "MediaUnit" | "Benchmark" | "UxPattern" | …    -- closed set = EntryKind::ALL
<prompt>      ::= <quoted-prose> | <noun-phrase>
```

Design rules:

- **`the @Kind matching …`** — retrieve the single best nomtu of that kind above the kind's local threshold. If none clears the threshold, it is a compile error with the kind's ranked top-K candidates in the diagnostic.
- **`a @Kind matching …`** — same, but at most one is required; the form accepts "no match" and binds to `nothing`. Forces pattern match on use.
- **`any @Kind matching …`** — returns a list; may be empty. Plural retrieval.
- The `@Kind` suffix marker (`@` already reserved for hashes) signals "this slot is kind-scoped." No keyword collision.
- The prompt is a quoted prose string *or* a noun phrase already in the sentence; the parser lowers it to the kind's retrieval query.

### 3.2 Side-by-side: `.nomx` v1 vs keyed-similarity `.nomx` v2

```
.nomx v1                                 .nomx v2 (keyed)
define render for a user:                define render for a user:
  when the user is logged in,              when the user is logged in,
    show the dashboard.                      show the @Screen matching "authenticated home".
  otherwise,                               otherwise,
    show the landing page.                   show the @Screen matching "unauthenticated landing".
```

```
.nomx v1                                 .nomx v2 (keyed)
define compose-intro for a post:         define compose-intro for a post:
  the style is brutalist.                  the style is a @UxPattern matching "brutalist".
  the hero is a large image.               the hero is a @MediaUnit matching "hero for " plus post.title.
  render the post with the style          render the post with the style
    and the hero.                            and the hero.
```

```
.nomx v1                                 .nomx v2 (keyed)
define tokenize of source:               define tokenize of source:
  use lexer.                               use the @Function matching
  call lexer on source.                      "produces a token stream from source text".
                                           call it on source.
```

### 3.3 Composition: how typed slots compose

When multiple typed slots appear in one expression, each is resolved **independently**, in its own kind's metric space, without cross-kind normalization. The compiler's diagnostic surface reports per-slot top-K, never a single merged ranking:

```
define hero-card for a post:
  let pattern    = the @UxPattern   matching "card, centered, elevated".
  let typography = the @DesignRule  matching "serif display, wide tracking".
  let image      = the @MediaUnit   matching post.title.
  compose pattern with typography and image.
```

On compile, the diagnostic (if any slot fails its threshold) is:

```
slot 1 (@UxPattern)   resolved: card_centered_elevated@a1b2  (score 0.91)
slot 2 (@DesignRule)  resolved: serif_display_wide@c3d4      (score 0.88)
slot 3 (@MediaUnit)   NO MATCH above 0.72 threshold.
                      top candidates:
                        hero_generic_sunset@e5f6  (0.61)
                        hero_abstract_flow@g7h8   (0.58)
                        hero_typography_only@i9j0 (0.54)
                      action: author one, pick one with `a @MediaUnit`,
                              or rephrase the prompt.
```

No cross-kind ranking. Each slot is its own decision.

### 3.4 First-class "no match"

Because retrieval is explicit, the source can express threshold acceptance:

```
define greeting for a user:
  the tone is a @UxPattern matching "friendly warm greeting".
  when the tone is nothing,
    respond with "hello " followed by user.name.
  otherwise,
    render the tone around user.name.
```

`a @Kind matching …` is structurally equivalent to `Option<Hash<Kind>>` — the language requires the author to handle both arms. This is the direct structural analog of Multiscreen's "represent the absence of relevant context."

### 3.5 Backwards compatibility with `.nomx` v1

Pure additive. `.nomx` v1 source parses unchanged. The new form is triggered only when the parser sees an `@Kind` token in a reference position. Files that never use `@Kind` compile identically. Internally, v1's bare-word reference lowers to an implicit `the @<inferred-kind> matching "<word>"` where the kind is inferred from the surrounding sentence context (the same inference v1 already has to do to resolve names). The v2 form surfaces the inference at the author's layer.

The two-track roadmap of doc 05 §6 is unchanged: `.nom` → `.nomx` v1 → `.nomx` v2 is a vocabulary addition, not a grammar break.

## 4. Zero-mistake guarantees revisited

Against doc 05 §5:

| Guarantee | Survives? | Notes |
|---|---|---|
| §5.1 No null / undefined | Yes, strengthened | `a @Kind matching …` makes the absent case a first-class type, not an opt-in wrapper. |
| §5.2 No data races | Yes, unchanged | Slot retrieval is value-level; no shared mutable state in references. |
| §5.3 No panics | Yes, unchanged | `the @Kind` that cannot resolve is a compile error, not a runtime panic. |
| §5.4 No integer overflow | Yes, unchanged | Orthogonal. |
| §5.5 No aliasing | Yes, unchanged | Slots resolve to content-addressed hashes; aliasing is by value. |
| §5.6 No memory leaks | Yes, unchanged | Orthogonal. |
| **NEW: §5.7 No wrong-kind reference** | — | Structurally impossible. `the @Screen matching …` cannot resolve to a `Function`. The compiler never attempts cross-kind fallback. |
| **NEW: §5.8 No silent retrieval drift** | — | Hash-pin is still the deterministic form. `the @Kind matching …` is non-deterministic across dict evolutions by design; the compiler records the resolved hash in the build manifest, and `nom check --audit` flags drift between builds (ties into Phase 9 LSP + Authoring Protocol, doc 04 §language-model framing). |
| **NEW: §5.9 No softmax bind** | — | Because thresholds are per-kind and retrieval does not compete across kinds, the compiler cannot be tricked into binding "the least-bad of a bad set." Below-threshold fails loudly. |

The critical upgrade: §5.7 and §5.9 were previously properties that type inference *hopefully* delivered; in v2 they are grammatical invariants.

## 5. Migration into the roadmap

Against doc 05 §6 and doc 04 phase map:

| Phase | What ships |
|---|---|
| **Doc 05 M1–M3** (already spec'd) | `.nomx` v1 — prose declarations, bare-word references. No changes. |
| **Doc 05 M4** + **new M4.5** | M4.5 lands the `@Kind` slot grammar as a **lexer + parser extension**, pure additive. No retrieval implementation yet; the parser emits a `TypedSlot` AST node that any resolver can later bind. |
| **Doc 04 Phase 5 (ingestion)** | Populates the dict with enough kind-scoped entries that per-kind retrieval is meaningful. Until this phase delivers ≥10k entries per active kind, typed-slot retrieval falls back to word lookup. |
| **Doc 04 Phase 8 (architectural ADOPT)** — **dependency** | The embedding-retrieval substrate noted in doc 04 §language-model framing is the natural home for per-kind metric spaces. One index per kind, not one global index. |
| **Doc 04 Phase 9 (LSP + Authoring Protocol)** | Authoring Protocol gains a `retrieve` RPC typed by `EntryKind`, returning ranked candidates per the kind's local metric. IDEs render the per-slot top-K picker. |
| **Doc 04 Phase 10 (Bootstrap)** | The self-hosted compiler must itself emit `TypedSlot` diagnostics. No change to the fixpoint proof (§10.3.1) — typed-slot resolution is a build-manifest side-effect, not a compile-output byte. |

**Landing point:** v2 grammar is spec-ready today; its implementation predates Phase 5 only as a parser/AST addition. The retrieval semantics postdate Phase 5 (needs vocabulary) and depend on Phase 8 (per-kind embedding indexes). LSP surface postdates Phase 9.

This is parallel to `.nomx` v1's own path — grammar first, runtime later.

## 6. Open questions — need human authoring

1. **Kind vocabulary surface.** Is `@Function` / `@Screen` / `@MediaUnit` acceptable as a prose-adjacent marker, or does it feel like a sigil regression? Alternatives considered: `the Function-kind named …`, `(as a Function)`, trailing `… as a Function`. The `@` form is terse and already reserved. **Tension:** prose-readability vs. kind-visibility.
2. **Vietnamese phrasing of typed slots.** `@Kind matching …` in Vietnamese: `@Kind khớp với …` or `hàm phù hợp với …`? Doc 05 §8 open question extends to v2. Per-kind names themselves (Function, Screen, MediaUnit) may or may not translate.
3. **Threshold authoring.** Who sets the per-kind threshold, and where? Options: (a) a dict-global default per kind, (b) per-file pragma, (c) per-slot inline (`the @Function matching "…" with at-least 0.85 confidence`). Inline is most explicit but noisiest; global is cleanest but brittle across dict evolutions. **Tension:** determinism of the build vs. friction at authoring time.
4. **Build-reproducibility of non-hash-pinned slots.** A `the @Function matching "…"` slot resolves to a hash at build time. Is that hash recorded in the source file (self-rewriting editor), in a side-car manifest (`.nom.lock`), or only in the dict's build manifest? The doc 04 §language-model-framing invariant "Determinism survives AI mediation" must survive this. **Tension:** authoring-surface mutability vs. source-as-truth.
5. **Cross-kind compound prompts.** If a prompt like "hero image of a brutalist carousel" naturally wants to retrieve a MediaUnit *and* a UxPattern, does the grammar allow `the @MediaUnit+UxPattern matching …` (product retrieval) or does the author always have to decompose into two slots? Product retrieval is seductive but re-introduces cross-kind normalization, which the whole proposal is designed to avoid.
6. **"No match" as type, not value.** Should `a @Kind matching …` be typed `Option<Hash<Kind>>`, `Maybe<Kind>`, or a new kind-parametric `Perhaps<Kind>` with its own pattern-match syntax (`perhaps …, nothing`)? Doc 05 §5 proposes `a maybe-<T>`; extending to kind-parametric form is a grammar question.
7. **Confidence as a first-class value.** Should the resolved score be readable in-source (`the @Function matching "…" where score ≥ 0.9`) so the author can gate on confidence? If yes, score becomes a value type and feeds into contracts (`ensure confidence(result) ≥ 0.85`).
8. **Interaction with `Specializes` edges.** A `the @Screen matching "home"` resolves to one `Screen`; but `Screen` commonly has `Specializes` variants per platform. Does the retrieval resolve the base `Screen` (and the build path picks the Specialization per target), or does retrieval resolve a `Screen × Platform` pair? The former is cleaner but leaves a second, hidden retrieval step.
9. **Empty-dict behavior at bootstrap.** Before Phase 5 ingestion delivers corpus, what does `the @Function matching "…"` do? Options: (a) always compile error, (b) fall back to word lookup with a warning, (c) disable the form behind a feature flag until the dict hits a size threshold.
10. **AI as the resolver vs. embedding-index as the resolver.** Doc 04 §language-model-framing says the AI is the *search surface*, not the *generation surface*, and determinism survives AI mediation. Does `the @Kind matching …` dispatch through the AI (flexible, approximate, non-reproducible across LLM versions) or through a deterministic embedding index (reproducible, limited, less semantic)? Proposed answer: **deterministic index primary, AI is the authoring helper that suggests the prompt prose.** Needs user ratification.

## 7. One-line philosophy

> *Every reference declares its kind. Retrieval is local. Absence is a first-class answer.*

The same way `.nomx` v1 replaced C-shaped declarations with sentence forms, `.nomx` v2 replaces flat identifier lookup with keyed retrieval — and inherits, grammatically, the per-key-similarity discipline that the Multiscreen paper argues is the right architectural choice when keys are genuinely typed.

---

*This is a design document, not an implementation plan. Before any `TypedSlot` AST node lands in `nom-parser`, §6 must be answered by the user.*
