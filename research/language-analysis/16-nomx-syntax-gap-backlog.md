# 16 — `.nomx` syntax gap backlog

**Date:** 2026-04-14
**Purpose:** Dedicated home for the growing list of `.nomx` v1/v2 syntax gaps surfaced by the Accelworld/upstreams translation corpus (doc 14). Split off from doc 14 once the gap list crossed the 20-item threshold. Every gap routes to one of three destinations:

- **W-wedge** — a concrete strictness-lane wedge in doc 13 §5
- **authoring-guide entry** — a convention/idiom note for the authoring docs
- **design deferred** — an open question to revisit when blocking work lands

> **Status 2026-04-14:** 35 gaps enumerated from doc 14 translations #1-12; no destination closures yet this cycle. Each cycle's gap-additions append here and get triaged into one of the three destinations. A gap is "closed" once either (a) its wedge ships with a test, (b) its authoring-guide note is written, or (c) its deferred status is pinned with a blocker.

---

## Triage format

| # | Gap | Destination | Status |
|--:|-----|-------------|--------|
| 1 | Iteration destructuring (`for each K and V in M`) | W4-A2b lexer test | ⏳ queued |
| 2 | Format-string interpolation | **W5** grammar rule | ⏳ queued |
| 3 | `returns nothing` grammar pin | W4-A1 addendum | ⏳ queued (A1 lock tests landed; addendum pending) |
| 4 | Path/file subkinds vs generic `@Data` | design deferred | ✅ doc 19 §D1 (stay `@Data`; feature-stack subkinds) |
| 5 | Union types / sum-return at v2 (`text or record`, `Result<A,B>`) | **W18** `@Union` kind (renumbered from earlier "W5") | ⏳ queued |
| 6 | Literal-string constants (Python `Literal[...]`) | **W6** grammar rule | ⏳ queued |
| 7 | Docstring → `intended to` mapping | authoring-guide note | ✅ doc 17 §I6 |
| 8 | Primitive-return idiom (`the result is true`) | authoring-guide + W4-A1 addendum | ⏳ queued |
| 9 | Sum-return phrasing at v1 (already works) | smoke-test | ✅ ct14 (nom-concept) |
| 10 | Atomic-state primitives | authoring-corpus seed | ✅ doc 17 §I9 |
| 11 | Lifetime annotations | design deferred (borrow-model) | 🧠 blocked on §borrow-model |
| 12 | Redundant v1 body when fully delegated | authoring-guide simplification rule | ✅ doc 17 §I7 |
| 13 | Destructuring parameters (TS `{state,dispatch}: EditorView`) | authoring-guide note | ✅ doc 17 §I10 |
| 14 | Early-return guards (works) | smoke-test | ✅ a4c24 (stages tests) |
| 15 | Callback closures | design deferred (§closures) | ✅ doc 19 §D2 (lift to named entities) |
| 16 | `fail with "..."` expression grammar | **W9** grammar rule | ⏳ queued |
| 17 | Multi-predicate short-circuit fail | subsumed by W9 | — |
| 18 | `hazard` effect rendering | smoke-test (works today) | ✅ ct12 + ct13 (benefit/boon synonym) |
| 19 | `is-a` runtime type probes | **W10** grammar rule | ⏳ queued |
| 20 | `perhaps...nothing` idiom (works) | authoring-guide anchor | ✅ doc 17 §I1 |
| 21 | Enum / sum-type declarations | **W11** grammar rule | ⏳ queued |
| 22 | Receiver-form methods (`func (o OS) String()`) | **W12** grammar rule | ⏳ queued |
| 23 | Entry-point `main` special-case | **W13** grammar rule | ⏳ queued |
| 24 | Exit-code vocabulary (`success`/`failure`/`code N`) | authoring-guide | ✅ doc 17 §I2 |
| 25 | `text-sprintf` idiom | authoring-guide note | ✅ doc 17 §I3 |
| 26 | List/text accessor primitives (`at(0)`, `find_last`, `after`) | authoring-corpus seeds | ✅ doc 17 §I11 |
| 27 | Preferred form (`uses` vs imperative verbs) for side-effects | authoring-guide decision | ✅ doc 17 §I12 |
| 28 | Interpreter/shebang metadata clause | **W15** grammar rule | ⏳ queued |
| 29 | Environment-variable access vocabulary | **W16** grammar rule | ⏳ queued |
| 30 | Globbing / file-tree query primitives | authoring-corpus seeds | ✅ doc 17 §I8 (partial — authoring idiom shipped; corpus seed still pending) |
| 31 | Process pipelines → named intermediate values | authoring-guide rule | ✅ doc 17 §I8 |
| 32 | Nested-section path syntax (TOML dot-paths) | **W17** grammar rule | ⏳ queued |
| 33 | Config-as-data vs. config-as-code split | authoring-guide clarification | ✅ doc 17 §I13 |
| 34 | Non-ASCII string literals verbatim | authoring-guide + smoke-test | ✅ doc 17 §I4 + ct11 smoke test (see nom-concept/src/lib.rs) |
| 35 | Hyphen-keys → underscore-identifiers mapping | authoring-guide rule | ✅ doc 17 §I5 |
| 36 | Async-marker clause (`async def` / async functions) | **W19** grammar rule | ⏳ queued (doc 14 #13) |
| 37 | Default parameter values (`callbacks = None`) | authoring-guide rule | ✅ doc 17 §I14 |
| 38 | Iterator vs. materialized sequences (lazy by default) | authoring-guide rule | ✅ doc 17 §I15 |
| 39 | Relational-algebra keywords (`project … from …`) | **W20** grammar rule | ⏳ queued (doc 14 #15) |
| 40 | Selector-predicate clause on data instances | **W21** grammar rule | ⏳ queued (doc 14 #16) |
| 41 | Typed dimension literals (`8px`, `4px`) | **W22** grammar rule | ⏳ queued (doc 14 #16) |
| 42 | Color literal grammar (`#0366d6`) | **W23** grammar rule | ⏳ queued (doc 14 #16) |
| 43 | `within the last N days` time-range idiom | authoring-corpus seed | ✅ doc 17 §I17 |
| 44 | Nested nullability modifiers (`perhaps list of perhaps text`) | **W24** grammar rule | ⏳ queued (doc 14 #17) |
| 45 | `identifier` as distinct data shape (GraphQL `ID!`) | authoring-corpus seed | ✅ doc 17 §I16 |
| 46 | Build-time dependency graph (Make prereqs) | **W25** grammar rule | ⏳ queued (doc 14 #18) |
| 47 | Multi-stage / stage-chain declarations (Dockerfile) | **W26** grammar rule | ⏳ queued (doc 14 #19) |
| 48 | Shell-exec primitive (canonical arg + stdout semantics) | authoring-corpus seed | ✅ doc 17 §I18 |
| 49 | Pinned-external-action ref grammar (`actions/checkout@v4`) | **W27** grammar rule | ⏳ queued (doc 14 #20) |
| 50 | Event-trigger declarations (`runs when X happens`) | **W28** grammar rule | ⏳ queued (doc 14 #20) |
| 51 | Visibility modifiers (`private` for scope hiding) | **W29** grammar rule | ⏳ queued (doc 14 #21) |
| 52 | `choice X is one of A, B, C` grammar (sealed/enum) | **W30** grammar rule — merges with W11 | ⏳ queued (doc 14 #22) |
| 53 | Method → receiver-as-parameter rule (Ruby classes, OOP methods) | authoring-guide rule | ✅ doc 17 §I19 |
| 54 | Concurrent-spawn clause (`start a worker that …`) | **W31** grammar rule | ⏳ queued (doc 14 #24) |
| 55 | Channel-type grammar with capacity annotation | **W32** grammar rule | ⏳ queued (doc 14 #24) |
| 56 | Finalizer clause (defer-style cleanup) | **W33** grammar rule | ⏳ queued (doc 14 #24) |
| 57 | `work_group` idiom for concurrent work tracking | authoring-guide rule | ✅ doc 17 §I20 |
| 58 | Typeclass-style constraints (`requires keys support ordering`) | **W34** grammar rule (blocks on borrow-model) | 🔒 blocked |
| 59 | `theorem` / `proof` kind declaration | **W35** grammar / kind-set decision | ⏳ queued (doc 14 #26, links deferred 11 §B) |
| 60 | Proof-tactic DSL (`by induction with | zero => rfl`) | **W36** grammar rule | ⏳ deferred (waits on W35 + math infra) |
| 61 | Actor-spawn + message-passing clause | **W37** grammar rule (may subsume W31) | ⏳ queued (doc 14 #27) |
| 62 | Logic-query / Prolog unification — translates to list-returning function + `ensures` clause | authoring-guide rule | ✅ closed (doc 14 #28; no wedge needed — existing shape suffices) |
| 63 | Lisp macro / metaprogramming — translates to higher-order function taking body-producing closure (links deferred D2) | authoring-guide rule | ✅ closed (doc 14 #29; Nom intentionally rejects macros — closure-lifting covers every common macro-use) |
| 64 | Wire-field-tag clause (`at field N` / `at tag N` for proto3, CBOR, Avro, Cap'n Proto) | **W38** grammar rule | ⏳ queued (doc 14 #30) |
| 65 | Nested enum → peer data decl flattening | authoring-guide rule | ✅ closed (doc 14 #30; existing flat-namespace preference + lift-to-peer rule covers all nested enums) |
| 66 | `forward_compatibility` QualityName registration | authoring-corpus seed | ⏳ queued (doc 14 #30) |
| 67 | Pattern-shape clause on data decls (regex-as-prose: `matches text of the shape …`) | **W39** grammar rule | ⏳ queued (doc 14 #31) |
| 68 | Capture-group → `exposes` field mapping | authoring-guide rule | ✅ closed (doc 14 #31; 1:1 mapping with type derived from character class) |
| 69 | Explicit anchoring in pattern prose (`anchored at start and end`) | authoring-guide rule | ✅ closed (doc 14 #31; anchored-vs-substring patterns are distinct shapes) |
| 70 | Regex alternation → `one of` (merges with W30 choice/enum grammar) | authoring-guide rule | ✅ closed (doc 14 #31; alternation reuses W30 choice shape) |
| 71 | State-machine decomposition tuple (states-data, events-data, transition-fn, timeout-fn, entry/exit-effect fns) | authoring-guide rule | ✅ closed (doc 14 #32; existing function + data + when-clause shape suffices) |
| 72 | Exhaustiveness-check over `when` clauses on enum-valued data (MECE over state × event) | **W40** grammar rule (ties W30 + MECE validator) | ⏳ queued (doc 14 #32) |
| 73 | Time-driven transitions → peer `*_timeout` function returning duration per state | authoring-guide rule | ✅ closed (doc 14 #32) |
| 74 | Entry/exit actions → peer effect-valenced functions with explicit hazard clauses | authoring-guide rule | ✅ closed (doc 14 #32) |
| 75 | `property` top-level kind declaration (expands closed kind set 7→8) | **W41** grammar rule | ⏳ queued (doc 14 #33) |
| 76 | Generator-shape clause (domain-range vocabulary for property input domains) | **W42** grammar rule (reuses W39 vocabulary) | ⏳ queued (doc 14 #33) |
| 77 | Shrinking-target declaration on properties (shortest/smallest/simplest) | authoring-guide rule | ✅ closed (doc 14 #33; embedded in W41 property grammar) |
| 78 | Locked regression seeds live in source file, not sidecar cache | authoring-guide rule | ✅ closed (doc 14 #33; ties to doc 08 lock-in-source principle) |
| 79 | Stateful properties = state-machine decomposition + post-step invariant check as property decl | authoring-guide rule | ✅ closed (doc 14 #33; composes #71 + W41) |
| 80 | HCL provider-version constraints stay as quoted text inside `requires` clauses | authoring-guide rule | ✅ closed (doc 14 #34) |
| 81 | HCL dotted refs (`aws_s3_bucket.logs.id`) decompose to two-step access (`uses @Data` + prose field select) | authoring-guide rule | ✅ closed (doc 14 #34) |
| 82 | HCL `resource` block → data decl + provision function + optional composition fan-out | authoring-guide rule | ✅ closed (doc 14 #34; canonical IaC decomposition) |
| 83 | HCL dependency DAG = composition `then` chain (no new wedge) | authoring-guide rule | ✅ closed (doc 14 #34; reuses a4c33 composition chain) |
| 84 | HCL `${var.region}` interpolation → typed-slot `@Data` or named-identifier prose ref | authoring-guide rule | ✅ closed (doc 14 #34) |
| 85 | HCL `lifecycle` meta-arguments → `hazard` clauses with explicit rationale | authoring-guide rule | ✅ closed (doc 14 #34) |

Totals by destination (after doc 14 #34 Terraform HCL translation closed 6 authoring-guide rules with 0 new wedges — strongest "no new wedge" translation yet):

- ⏳ Wedge queued: **35** (unchanged)
- 🧪 Smoke-test todo: **1**
- 📘 Authoring-guide doc-todo: **0**
- ✅ Closed: **46**
- 🧠 Design deferred (open): **0**
- 🔒 Blocked: **2**
- 🌱 Authoring-corpus seed: **1** (forward_compatibility QualityName)

Backlog size: 87 rows. Closure rate 53% (46/87). **34 translations** in doc 14. Paradigm coverage: imperative + OOP + async + concurrency + pure-functional + ADT + data + shell + build + container + editor-event + CI/CD + math-as-language + actor-model + logic-programming + metaprogramming + schema-IDL + pattern-DSL + state-machine-DSL + property-based-testing + **infrastructure-as-code (Terraform HCL)**. Terraform adds 0 new wedges — validates that the 7-noun closed kind set is sufficient for real-world engineering domains.

## Wedge master index (for cross-ref with doc 13)

- **W4 (strictness lane, 6 sub-wedges A1-A6):** A1 ✅ A2 ✅ A3-A6 ⏳ — see doc 13 §5.
- **W5:** Format-string interpolation grammar.
- **W6:** Literal-string constants (Python-`Literal`-style).
- **W9:** `fail with "..."` expression grammar (subsumes W-short-circuit-fail).
- **W10:** `is-a` runtime type probes.
- **W11:** Enum / sum-type declarations.
- **W12:** Receiver-form methods.
- **W13:** Entry-point `main` special-case.
- **W14:** Exit-code vocabulary.
- **W15:** Interpreter/shebang metadata clause.
- **W16:** Environment-variable access vocabulary.
- **W17:** Nested-record-path syntax (TOML dot-paths).
- **W18:** `@Union` typed-kind for sum-types (replaces earlier ambiguous "W5" reference).
- **W38:** Wire-field-tag clause (`at field N` / `at tag N` for proto3, CBOR, Avro, Cap'n Proto — per-field stable numeric IDs).
- **W39:** Pattern-shape clause on data decls (regex-as-prose: closed 8-10-word vocabulary for character-classes, quantifiers, anchoring, case-folding, alternation).
- **W40:** Exhaustiveness-check over `when` clauses on enum-valued data — totality gate for state-machine transitions and any `when current is X` branching on a closed set. Ties into W30 + existing MECE validator.
- **W41:** `property` top-level kind declaration — expands closed kind set from 7 to 8 nouns. Universally-quantified claim over a generator; orthogonal to function/data/concept.
- **W42:** Generator-shape clause — closed-vocabulary domain-range descriptors for property input generators (`list lengths from N to M`, `integers from -X to Y`, `text of shape …`). Reuses W39 pattern-shape vocabulary.

Existing lanes not duplicated here: W7 placeholder rows (doc 15 §2); W8 100-repo harness (doc 15 §3-§7).

## How to use this doc

**Adding a gap:**

1. Translate a function in doc 14; note the gap in the translation's "Gaps surfaced" section.
2. Append a row here with a destination pick.
3. If the destination is a new W-wedge, reserve the next W-number and cross-reference doc 13.

**Closing a gap:**

1. Wedge: ship the commit, update Status to ✅, link the commit hash.
2. Authoring-guide: write the entry, update Status to ✅, link the entry.
3. Deferred: pin the blocker reason, update Status to 🔒, note when to revisit.

**Review cadence:** every five new gaps, re-read the 🧠 design-deferred rows to see whether any blocker lifted.
