# 08 — Layered Concept / Module / Entity Architecture

> **Archive snapshot — finalized 2026-04-14.** layered concept/module/entity architecture; shipped.
> Live mission state lives in [`research/08-mission-checklog.md`](../../08-mission-checklog.md).
> See also the grammar blueprint plan at
> `C:\Users\trngh\.claude\plans\mighty-jumping-snowglobe.md`
> and corpus closure proof at 68/88 (77%) via
> `nom-compiler/crates/nom-concept/tests/closure_against_archive.rs`.


> **Last verified against codebase: 2026-04-14, HEAD `64b3058`.**

**Status: ~82% implemented as of 2026-04-14.** Filed 2026-04-13. Builds on docs 02-07; resolves the three open tensions in [07-keyed-similarity-syntax-proposal.md §6](07-keyed-similarity-syntax-proposal.md). Reality gains since original filing: M2 acceptance preservation, M4 three-tier recursive ingest, M5 layered dreaming (a+b+c), M7 MECE CE-check (a+c) — see doc 09 for the consolidated shipped inventory.

## Implementation status (2026-04-14, HEAD `64b3058`)

| Feature | Status | Commit(s) |
|---|---|---|
| **Tier 0** — atoms in DB2 (`words_v2`) | ✅ SHIPPED | `aaa914d` |
| **Tier 1** — `.nomtu` parser (`nom-concept` Tier-1) | ✅ SHIPPED | `05ee1b6` |
| **Tier 1** — recursive-ingest compiler (bytes per §4.3) | ✅ SHIPPED | M4 (`three_tier_recursive_e2e.rs`) |
| **Tier 2** — `.nom` parser (`nom-concept` Tier-2) | ✅ SHIPPED | `d9425ba` |
| **Tier 2** — recursive ingest (concept → modules → atoms) | ✅ SHIPPED | M4 (`three_tier_recursive_e2e.rs`) |
| **Two databases** — DB1 (`concept_defs`) + DB2 (`words_v2`), additive | ✅ SHIPPED | `aaa914d` |
| **Closure walker** — DFS post-order with cycle detection | ✅ SHIPPED | `c5cdce6` |
| **Resolver stub** — `resolve_closure`, alphabetical-smallest tiebreak | ✅ SHIPPED | `bf95c2c` + `c405d2a` |
| **Resolver** — Phase-9 corpus-embedding semantic re-rank | ⏳ PLANNED | Phase 9 |
| **MECE validator** — ME-collision check (exit 1 on collision) | ✅ SHIPPED | `c63a6a7` |
| **MECE validator** — CE-check (collectively-exhaustive) | ✅ SHIPPED | M7a `bcadcb3` (required_axes registry) + M7c `4307c5a` (layered-dream integration) |
| `nom store sync` | ✅ SHIPPED | `ba7769f` |
| `nom build status` (+ `--write-locks`) | ✅ SHIPPED | `bf95c2c` |
| `nom build manifest` | ✅ SHIPPED | `fef0419` |
| **Lock writeback** — `name@hash` for v1 refs (§8.2) | ✅ SHIPPED | `a04b91e` |
| **Lock writeback** — typed-slot refs intentionally NOT written back (§3.5) | ✅ SHIPPED (by design) | `c405d2a` |
| **Layered dreaming** (§9) — concept-tier + module-tier `nom dream` | ✅ SHIPPED | M5a `f0ae193` + M5b `e28f69d` + M5c `cc9641b` |
| **Acceptance-predicate preservation engine** (§9.1) | ✅ SHIPPED | M2 (`acceptance_preserve_e2e.rs`) |
| **Cascade through dream** (§9.3) | ⏳ PLANNED | Phase 8/9 |
| **AppManifest deprecation** — `app.nom` as root concept | ⏳ PLANNED | Phase 10 |
| **`nom-concept` crate** — Tier-1 + Tier-2 parser | ✅ SHIPPED (as part of `nom-concept`) | `05ee1b6` + `d9425ba` |
| **`nom-module`** crate (or sub-module) | ✅ SHIPPED as sub-module of `nom-concept` | `05ee1b6` |
| **`@Kind` sigil** (§8.1 reversal — user reversed prose-only decision) | ✅ SHIPPED | `c9d1835` |

## 1. Insight

Nom today has two implicit layers: a **source layer** (`.nomx` text) and a **compiled-artifact layer** (the dict — bodies as bytes per [04-next-phases-plan.md §4.4.6](04-next-phases-plan.md)). `Concept` exists only as `EntryKind` plus an `entries.concept` column — post-hoc clustering metadata, never a file, never an input to the build. Composition happens at the artifact layer through `AppManifest` ([04-next-phases-plan.md §5.12.1](04-next-phases-plan.md)) over already-compiled bodies.

That is one tier short. Authors reason in **groupings** ("the auth flow", "the render pipeline") that have no file today.

This doc proposes three explicit tiers above the artifact store:

- **Tier 0 — Atomic dictionary entities** in DB2 (the word dictionary). Ingested from corpus or authored inline. No file form by default.
- **Tier 1 — `.nomtu` files** (multi-entity DB2 containers, **small scope**). One file holds several entities, optionally with composition expressions among them.
- **Tier 2 — `.nom` files** (multi-concept DB1 containers, **big scope**). One file holds several concepts. A concept is a *dictionary-relative index over DB2*: it doesn't hold its entities, it indexes them.

Plus the existing **artifact store** as a fourth (bytes-only) layer.

Each tier carries its own scoring/relevance, per the keyed-similarity insight in doc 07. Concept-tier ranks compositions by acceptance-criteria fit; module-tier ranks each `.nomtu`'s composition by signature/behavior fit; entity-tier scores per-body benchmarks. **No cross-tier normalization** — the design refuses to fake a weighted sum across incommensurable axes.

This buys authorial leverage (the concept file is what humans edit) without touching the byte-determinism invariants (artifacts stay hash-keyed).

## 2. Two databases

### 2.1 DB1 — Concept Dictionary

Per-repo index over DB2. Stores the dictionary-relative index that "makes a concept real."

```sql
concepts (
  name           TEXT PRIMARY KEY,
  repo_id        TEXT,
  intent         TEXT,                     -- prose
  index_into_db2 TEXT,                     -- the manifest the compiler follows
  exposes        TEXT,
  acceptance     TEXT,                     -- predicates (JSON)
  objectives     TEXT,                     -- ranked QualityName list
  src_path       TEXT,                     -- path to the .nom file
  src_hash       TEXT,
  body_hash      TEXT                      -- hash of the compiled concept body
)
```

The `index_into_db2` column is load-bearing: a concept *without* this index is just prose; a concept *with* this index can be instantiated.

Populated when `nom-concept` parses each `.nom` in a repo. Mirrored to/from `.nom` files via `nom store sync`.

### 2.2 DB2 — Word Dictionary

```sql
words (
  hash          TEXT PRIMARY KEY,
  word          TEXT,                      -- feature-stack (see §6.5)
  kind          TEXT,                      -- function|module|concept|screen|data|event|media
  signature     TEXT,
  contracts     TEXT,
  body_kind     TEXT,                      -- bc|avif|...
  body_size     INTEGER,
  origin_ref    TEXT,
  bench_ids     TEXT,
  authored_in   TEXT,                      -- path to .nomtu (NULL if ingested from corpus)
  composed_of   TEXT                       -- JSON list of entity hashes (NULL if atomic)
)
```

Three row shapes:

| Shape | `authored_in` | `composed_of` | Source |
|---|---|---|---|
| Atomic + ingested | NULL | NULL | corpus |
| Atomic + authored | path to `.nomtu` | NULL | declared inline in a `.nomtu` file |
| Composed | path to `.nomtu` | list of hashes | declared inline + emits a composition |

A single `.nomtu` may populate many `words` rows. The `authored_in` column makes them greppable back to source.

### 2.3 Artifact store

`~/.nom/store/<hash>/body.{bc,avif,...}`. Unchanged. Both atomic entities and compiled compositions write here, keyed by their own hash.

## 3. File model & directory layout

Two equally-valid styles, freely mixable.

### 3.1 Style A — per-folder concept

```
my-app/
  app.nom                          # 1 concept (root); composes the others
  authentication/
    authentication.nom             # 1 concept; index references modules below
    auth_flow.nomtu                # 4 DB2 rows (3 entities + 1 composition)
    token_helpers.nomtu            # 5 DB2 rows
  rendering/
    rendering.nom                  # 1 concept
    layout.nomtu                   # 3 DB2 rows
    paint.nomtu                    # 1 DB2 row
  shared/
    shared.nom                     # 1 concept
    string_helpers.nomtu           # 8 DB2 rows
```

### 3.2 Style B — umbrella `.nom`

```
my-app/
  app.nom                          # holds 4 concepts in one file
  nomtu/
    auth_flow.nomtu
    token_helpers.nomtu
    layout.nomtu
    paint.nomtu
    string_helpers.nomtu
```

Both styles produce the **same DB1 + DB2 rows**. File partitioning is organizational; the indexes that "make the concepts real" are identical.

### 3.3 Invariants

- `.nom` is a **multi-concept container** (1..N DB1 rows).
- `.nomtu` is a **multi-entity container** (1..N DB2 rows).
- Both are source files with **lock semantics** (resolved-hash writeback after first build).
- Both **ingest from the underlying tier at compile time** — never inline bytes.
- No `.nomtu` duplication across concepts: different concepts mean separate folders; cross-references resolve through DB2 by name.

## 4. `.nomtu` semantics

A `.nomtu` is a small-scope source file that may declare or reference several DB2 entities. Composition is optional.

- Each entity carries `(word, kind, signature, contracts, body_kind, ...)`.
- The file groups entities by small-scope ("token-validation helpers", "layout primitives").
- Lock: every `matching` clause is rewritten to `name@hash` after first build; the entire `.nomtu` is the lock for the entities it contains.
- At compile time, the compiler **ingests `.bc` + metadata** for every referenced entity AND **compiles every entity declared inline**. Composition expressions, when present, emit additional composed bodies.
- A single `.nomtu` may yield zero, one, or many new bodies.

Closer in spirit to a Python package's `__init__.py` + sibling functions in one file than to a Rust file with `pub mod`. The grouping is for human comprehension; DB2 flattens it for queries.

## 5. `.nom` semantics

A `.nom` is a big-scope source file that may declare or reference one or more concepts. Symmetric to `.nomtu` but one tier up.

- Each concept declared inline carries name, intent, **index** (the structured DB2-references manifest), exposed names, acceptance predicates, dream objectives.
- A concept does not hold its entities — it is a **dictionary-relative index** over DB2. To "make the concept real," the compiler follows the index.
- Lock: every reference is hash-pinned after first build.
- Ingestion-on-parse: `nom-concept` parses each `.nom` and populates DB1. One `.nom` may emit N rows.
- The root `app.nom` is just a Tier-2 concept (or many concepts) whose index composes the others. AppManifest deprecates to a generated view over `app.nom`.

## 6. Grammar

EBNF additions to `.nomx` (doc 06).

### 6.1 `.nomtu` (module/entity container)

```
NomtuFile        = (EntityDecl | CompositionDecl)+

EntityDecl       = "the" Kind Word "is" SignatureBody ContractClause* "."
Kind             = "function" | "module" | "screen" | "data" | "event" | "media"
Word             = FeatureStack                                  // see §6.5
SignatureBody    = (existing .nomx body grammar)
ContractClause   = "requires" Predicate | "ensures" Predicate

CompositionDecl  = "the module" Word "composes"
                     EntityRef ("then" EntityRef)*
                   ("with" Glue)?
                   ContractClause* "."
EntityRef        = "the" Kind Word ("@" Hash)? ("matching" Phrase)?
Glue             = NaturalLanguageDescription
```

### 6.2 `.nom` (concept container)

```
NomFile          = ConceptDecl+

ConceptDecl      = "the concept" Word "is"
                     "intended to" IntentPhrase "."
                     IndexClause+
                     ExposesClause?
                     AcceptanceClause*
                     ObjectiveClause?
                     "."

IndexClause      = "uses" EntityRef ("," EntityRef)*
                 | "extends the concept" Word "with" ChangeSet
ExposesClause    = "exposes" Word ("," Word)*
AcceptanceClause = "this works when" Predicate
ObjectiveClause  = "favor" QualityName ("then" QualityName)*
```

### 6.3 Side-by-side example

**Authoring an `auth_flow.nomtu`:**

```
the function validate_token_jwt_hmac_sha256 is
  given a token of text, returns yes or no.
  requires the token is non-empty.
  ensures the result reflects whether the token's signature verifies.

the function issue_session_jwt_short_lived is
  given a user identity, returns a session token of text.
  ensures the token expires within fifteen minutes.

the module auth_jwt_session_compose composes
  the function validate_token_jwt_hmac_sha256 then
  the function issue_session_jwt_short_lived
  with "validate first; only issue when the token verifies."
  ensures no session is issued for an invalid token.
```

**Authoring an `authentication.nom`:**

```
the concept authentication_jwt_basic is
  intended to let users with valid tokens reach the dashboard.

  uses the module auth_jwt_session_compose,
       the function logout_session_invalidate_all,
       the function refresh_session_rotate.

  exposes auth_jwt_session_compose, logout_session_invalidate_all.

  this works when users with valid tokens reach the dashboard
                within two hundred milliseconds.
  this works when invalid tokens are rejected
                before any database read.

  favor security then speed.
```

## 7. Compiler pipeline

| Stage | Command | Reads | Writes |
|---|---|---|---|
| Translate | `nom author translate <draft.md>` | prose | `.nom` + `.nomtu` skeletons |
| Sync | `nom store sync <repo/>` | `.nom` + `.nomtu` files | DB1 + DB2 rows; resolves `matching` → `name@hash` |
| Build module | `nom build module <m>.nomtu` | DB2 atoms (bytes + metadata) | DB2 rows + composed bodies in artifact store |
| Build concept | `nom build concept <c>.nom` | DB2 modules + DB1 concepts | DB1 row + body in artifact store |
| Build app | `nom build <repo/>` | the root `app.nom` recursively | full closure |
| Dream | `nom dream <repo/>` | DB1 + DB2 + benchmarks | refined locks; Pareto report |

Crate ownership:

- **`nom-concept`** (new crate) — `.nom` parsing, concept-graph resolution, sits between `nom-parser` and `nom-planner`.
- **`nom-module`** (new crate, or a sub-module of `nom-concept` if the symmetry permits) — `.nomtu` parsing, Tier-1 composition resolution, ingest-`.bc`+metadata weave.
- **`nom-store sync`** (new CLI subcommand) — file ↔ DB reconciliation.
- **`nom-verifier`** — acceptance predicates as typed obligations.
- **`nom-app`** — layered `DreamReport` with Pareto trees.
- **`nom-dict`** — split current `entries` into `concepts` (DB1) + `words` (DB2) tables.

The compiler is a **recursive ingestor**: a concept ingests modules; a module ingests atoms. At each tier the body is built from the bytes + metadata of the tier below — never from source above.

## 8. Three doc-07 tensions resolved

### 8.1 Tension 1 — Kind marker

**Original resolution: prose form `the NOUN`** from a closed set: `function`, `module`, `concept`, `screen`, `data`, `event`, `media`. Sigils (`@Function`) rejected as C-shape regression; parenthesized prose (`(as a Function)`) rejected as awkward inside composition expressions. The `the NOUN` pattern is unambiguous because the noun set is closed.

> **UPDATE (2026-04-13)**: User reversed this decision. Both forms now coexist:
> `the function login_user matching "..."` (v1 bare-word; `typed_slot = false`) and
> `the @Function matching "..."` (v2 typed-slot; `typed_slot = true`). Shipped via commit
> `c9d1835`. The `EntityRef.typed_slot` flag in `nom-concept/src/lib.rs` discriminates them.

### 8.2 Tension 2 — Lock storage

**Resolution: `.nom` and `.nomtu` source carry `name@hash` after first build.** The source file IS the lock. No `.nom.lock` sidecar. Authoring writes `the function login matching "..."`; the resolver writes back `the function login@a1b2 matching "..."`. The `matching` clause stays as documentation + regeneration hint. Source self-describes; no truth split.

> **Status**: ✅ SHIPPED for v1 refs (commit `a04b91e`). Typed-slot refs (`@Kind`) are
> intentionally NOT written back — per doc 07 §3.5 (commit `c405d2a`). The resolved
> hash for typed-slot refs is recorded in the build manifest only.

### 8.3 Tension 3 — Resolver

**Resolution: deterministic per-kind embedding index, no LLM in the build path.** Built by `nom corpus embed` (Phase 9). LLM dispatch is reserved for `nom author`; never enters `nom build`, `nom store sync`, or `nom dream`. Preserves §10.3.1 fixpoint discipline (Stage 2 ≡ Stage 3 byte-identical).

> **Status**: ✅ Stub shipped (commits `bf95c2c` + `c405d2a`) — alphabetical-smallest
> hash tiebreak as deterministic placeholder. ⏳ Phase-9 corpus-embedding re-rank pending.

## 9. Layered dreaming ✅ SHIPPED (M5a+b+c)

Per the keyed-similarity insight (doc 07): each tier scores independently; no cross-tier normalization.

- **Tier-0 dream** (`nom nomtu dream <hash>`) — atomic body. Score = local benchmarks. Already planned for Phase 12 specialization. ⏳ PLANNED
- **Tier-1 dream** (`nom module dream <m>.nomtu`) — module composition. Score = (acceptance ∧ objectives) tuple. Iteration swaps atomic refs against alternatives from the embedding index. ✅ SHIPPED (M5a/b)
- **Tier-2 dream** (`nom concept dream <c>.nom`) — concept composition. Score = (acceptance ∧ objectives) tuple over modules. ✅ SHIPPED (M5a/b)
- **Tier-3 dream** (`nom app dream`) — root. Recursively dreams; returns a **Pareto front of dream-trees**. User picks; no silent weighting. ✅ SHIPPED via `LayeredDreamReport { tier, leaf, child_reports, pareto_front }` (M5c `cc9641b`)

`DreamReport` grows `tree: LayeredScoreNode` and `pareto: Vec<TreeCandidate>`. Existing `epic_score` stays as a leaf-tier scalar for back-compat.

### 9.1 Acceptance preservation ✅ SHIPPED (M2)

**Acceptance predicates are local** to the concept that declares them. Apps re-declare what they care about; predicates do not propagate up automatically.

`nom dream` is the **preservation engine**: every iteration that swaps a child must re-evaluate ALL of the parent's predicates and refuse swaps that drop or weaken any. Reports must surface "predicate X went from satisfied to vacuous because we swapped Y" instead of silently absorbing it. Because predicates are prose, they need a runtime check, not a static gate.

> **Status**: ✅ SHIPPED (M2) — preservation engine re-evaluates all parent predicates
> per swap; swaps that drop/weaken any predicate are refused. See `acceptance_preserve_e2e.rs`.
> Predicate parse/storage in manifest landed earlier via `fef0419`.

### 9.2 Objective inheritance + MECE validation

Objectives are typed by `QualityName`, registered in the corpus with a metric function and an axis label.

When concept *child* composes into concept *parent*, the union of their objectives is computed and **MECE-validated**:

- **Mutually Exclusive (ME)**: no two objectives in the union may share an axis (two `speed` objectives would collide).
- **Collectively Exhaustive (CE)**: every axis registered as "required at this composition layer" in the corpus must be covered by at least one objective in the union.

**AI authoring is gated** behind MECE. `nom author` cannot publish a concept whose objectives fail validation; the AI must surface unmet axes back to the human.

**Singletons** ("one DB per app", "one auth provider") are modeled as registered axes with `cardinality = exactly_one_per_app`. The MECE validator enforces them automatically — zero coverage is a CE failure; multiple coverage is an ME failure. AI cannot introduce a duplicate singleton without explicit human override.

**Dreaming uses the MECE-validated objective set as the Pareto frontier targets.**

> **Status**:
> - ME-collision check: ✅ SHIPPED (commit `c63a6a7`) — exit 1 on axis collision;
>   `examples/agent_demo/` intentionally collides to prove it works.
> - CE-check: ✅ SHIPPED (M7a `bcadcb3` — required_axes registry; M7c `4307c5a` — integrated into layered dream).
> - Singleton enforcement: ⏳ PLANNED — the design is doc'd; corpus `exactly_one_per_app`
>   registration deferred to Phase 9.

### 9.3 Cascade through dream ⏳ PLANNED (Phase 8/9)

When concept A changes, downstream concepts that index it are NOT silently re-pinned. `nom store sync` detects the change and marks downstream concepts as **stale**; the next `nom dream` is responsible for preserving the entire predicate set without misleading.

- Cascade is **dream-mediated**, not sync-mediated.
- Sync is automatic; cascading hash updates are NOT — the user must invoke `nom dream` (or `nom rebuild`) explicitly.
- A downstream concept's dream IS re-triggered when an upstream changes (the staleness flag is the trigger).
- Reports surface what was preserved vs. re-resolved vs. failed and why.

> **Status**: ⏳ PLANNED — `nom store sync` is shipped (commit `ba7769f`) but staleness
> propagation and dream-mediated cascade are not yet implemented.

## 10. Migration

| Phase | What lands | Status |
|---|---|---|
| Inside Phase 4 (now) | `.nom` + `.nomtu` file formats; `nom store sync`; DB1/DB2 schema split. Pure metadata work; doesn't touch the artifact store. | ✅ SHIPPED — commits `aaa914d` + `05ee1b6` + `d9425ba` + `ba7769f` |
| Before Phase 5 | `nom-concept` + `nom-module` parsing; recursive-ingest compiler. Phase 5's planner needs concept + module boundaries as input. | ✅ SHIPPED — parsing + recursive-ingest compiler (M4 `three_tier_recursive_e2e.rs`) |
| Phase 8 or 9 | Tier-1 + Tier-2 dreaming. Requires embedding index (Phase 9) for swaps and acceptance predicates (Phase 8) for scoring. | ✅ SHIPPED (M5a+b+c); embedding-index re-rank still Phase 9 |
| Phase 12 (unchanged) | Tier-0 specialization; now the leaf of the Pareto tree. | ⏳ PLANNED |
| AppManifest deprecation | `app.nom` becomes the root. AppManifest is a generated view, not a separate artifact. | ⏳ PLANNED |

## 11. Open questions resolved + carry-overs

All twelve numbered questions from the planning round are resolved per user authoring on 2026-04-13. Implementation status added below.

1. **Media kind marker** — `media`. Closed kind set: `function`, `module`, `concept`, `screen`, `data`, `event`, `media`. ✅ Closed kind set shipped in lexer (`nom-concept/src/lib.rs`, commit `c9d1835`).
2. **`.nomtu` sharing** — no duplication; cross-references by name through DB2; singleton resources live in a shared concept. ✅ Design enforced by parser/resolver; `nom store sync` populates DB2 (commit `ba7769f`).
3. **Empty concept folder** — warning + dream-seed. ⏳ PLANNED — no warning emitted yet.
4. **Orphan reaping** — manual `nom store gc`; never automatic. ⏳ PLANNED — `nom store gc` subcommand not yet implemented.
5. **Tier-2 override scope** — *deferred*; revisit after the rest of the architecture stabilizes. ⏳ DEFERRED — no change from original resolution; still unresolved per doc 09.
6. **Acceptance predicate portability** — local + dream-preserved (§9.1). ✅ Predicates parsed + stored in build manifest (commit `fef0419`); ✅ dream-preservation engine SHIPPED (M2).
7. **Dream-objective inheritance** — MECE validator (§9.2). ✅ ME check shipped (commit `c63a6a7`); ✅ CE check shipped (M7a `bcadcb3` + M7c `4307c5a`).
8. **Custom quality registration** — corpus table. ⏳ PLANNED — no corpus quality table yet.
9. **Cross-`.nomtu` references** — yes; resolved against DB2. ✅ SHIPPED — resolver does DB2 lookup by kind + word (commits `bf95c2c` + `c405d2a`).
10. **Mixed kinds in one `.nomtu`** — yes, with the singleton caveat from item 2. ✅ SHIPPED — parser accepts multiple entity declarations of different kinds in one `.nomtu` file.
11. **Lock cascade** — dream-mediated (§9.3). ⏳ PLANNED — cascade design documented; not yet implemented.
12. **Word naming for 100M+ entities** — feature-stacks (§6.5 below). ✅ Design shipped in doc; ⏳ corpus-root-vocabulary registry is Phase-5+ work.
13. **Singleton enforcement** — registered as axes with `cardinality=exactly_one`, enforced via the MECE validator (§9.2). ⏳ PLANNED — ME collision check shipped; `cardinality=exactly_one_per_app` axis registration deferred to Phase 9.

**Carry-over open questions** (doc 09 scope, not resolved in the original 13):

- **Doc 07 vs doc 08 syntax tension on `the NOUN` form** — both forms now coexist via `EntityRef.typed_slot` (commit `c9d1835`). Long-term: should `the NOUN` form be deprecated in favor of `@Kind`? Unresolved.
- **Aspect marker semantics** (`verified`/`active`/`deferred`) — `0%`; low semantic value without runtime. ⏳ PLANNED Phase 9+.
- **Multi-locale demo strategy** — `agent_demo_vn` deleted and VN keyword vocabulary fully removed (ecd0609). Vocabulary is English-only ASCII; Vietnamese inspires grammar structure only.

### 6.5 (referenced from §6 above) — Feature-stack word naming

Words are NOT flat strings. They are **feature-stacks** of the form `<root>_<feature1>_<feature2>_..._<featureN>`. Each feature narrows the meaning by one axis; longer stacks = more specific entities.

Examples:

- `validate_token_jwt_hmac_sha256_v2`
- `compose_brutalist_webpage_responsive_dark`
- `db_postgres_connection_pool_async`
- `auth_session_jwt_refresh_rotating`

Properties:

- Two entities sharing a prefix differ at their first divergent feature — natural disambiguation.
- Word IDs are unique across DB2 — exact-stack collision is a build failure (the resolver fails loudly).
- A **root vocabulary** registered in the corpus (~50-200 root tokens like `validate`, `compose`, `db`, `auth`, `render`, `parse`) bounds the namespace combinatorially. A depth-6 stack from a 100-root vocab yields ~10^12 candidate IDs — comfortable for 10^8 entities.
- **Hash suffix `@<hash>`** is a disambiguator of last resort, used when two distinct bodies legitimately deserve the same feature stack.
- **Kind marker stays orthogonal** — `the function validate_token_jwt_hmac_sha256` — the kind is grammar metadata, not part of the stack.
- **Concept names follow the same convention** at a separate scope vocabulary (`app_my_app`, `concept_authentication_jwt_session`, `concept_render_brutalist_v3`). Concept-root vocab is smaller (~30-100 roots).
- **`.nomtu` composed bodies** derive their stack from the union of features of the entities they compose, with the composing concept's name as a prefix when needed (`auth_jwt_session_compose`).
- **Feature ordering is canonical** (alphabetical within stack position) so the same feature set always produces the same ID.
- **AI authoring** uses the corpus-registered vocabulary; new features require vocab review (prevents vocab explosion).

---

This proposal is the minimum scope for the layered architecture. Landing it is multi-quarter; the **shape** must be right before the first character of `nom-concept` is typed.
