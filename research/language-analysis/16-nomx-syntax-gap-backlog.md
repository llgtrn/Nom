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
| 86 | NumPy broadcasting → element-wise prose inside outer iteration (compiler vectorizes) | authoring-guide rule | ✅ closed (doc 14 #35) |
| 87 | Axis parameter (`axis=N`) → explicit `for each row`/`for each column` iteration | authoring-guide rule | ✅ closed (doc 14 #35) |
| 88 | `np.where` element-wise ternary → `when … otherwise …` inside outer iteration | authoring-guide rule | ✅ closed (doc 14 #35) |
| 89 | Vectorization is a compiler responsibility, not an author concern | authoring-guide rule | ✅ closed (doc 14 #35; Phase 12 specialization) |
| 90 | N-dimensional arrays decompose to nested `list of` types | authoring-guide rule | ✅ closed (doc 14 #35) |
| 91 | `numerical_stability` QualityName registration | authoring-corpus seed | ⏳ queued (doc 14 #35; accumulates with #66 forward_compatibility) |
| 92 | Airflow `>>` dependency operator → composition `then` chain | authoring-guide rule | ✅ closed (doc 14 #36) |
| 93 | Workflow schedule parameters belong in peer data decl, not on the composition | authoring-guide rule | ✅ closed (doc 14 #36) |
| 94 | Retry-policy clause (`up to N times with delay D, backoff strategy S`) | **W43** grammar rule | ⏳ queued (doc 14 #36) |
| 95 | Task operators (PythonOperator, BashOperator, …) flatten to plain function decls; runtime is build-time concern | authoring-guide rule | ✅ closed (doc 14 #36) |
| 96 | Airflow `@task` decorator is a no-op in translation | authoring-guide rule | ✅ closed (doc 14 #36) |
| 97 | XCom cross-task communication → explicit `uses` + typed returns | authoring-guide rule | ✅ closed (doc 14 #36) |
| 98 | Watermark clause for late-arrival policy on event-time streams | **W44** grammar rule | ⏳ queued (doc 14 #37) |
| 99 | Window-aggregation clause (`window of D over FIELD` with tumbling/sliding/session/global kinds) | **W45** grammar rule (pairs with W40 MECE) | ⏳ queued (doc 14 #37) |
| 100 | Streaming output-modes are a closed choice (append/update/complete); declare sink as typed choice | authoring-guide rule | ✅ closed (doc 14 #37; ties W30) |
| 101 | Streaming triggers are `(kind, interval)` pair where kind is a closed choice | authoring-guide rule | ✅ closed (doc 14 #37) |
| 102 | Stateful-aggregation state TTL → `hazard` clause on the aggregation function | authoring-guide rule | ✅ closed (doc 14 #37) |
| 103 | Streaming sources must carry an explicit data decl; schema inference is an author-helper tool | authoring-guide rule | ✅ closed (doc 14 #37; ties to `nom author infer-schema`) |
| 104 | On-chain functions implicitly `favor determinism` + forbid wall-clock/random/non-chain IO | authoring-guide rule | ✅ closed (doc 14 #38) |
| 105 | On-chain implicit context (caller, value, gas) named explicitly in `requires`/`ensures` clauses | authoring-guide rule | ✅ closed (doc 14 #38) |
| 106 | Checks-effects-interactions invariant declared explicitly as `hazard` when external transfers occur | authoring-guide rule | ✅ closed (doc 14 #38) |
| 107 | Data decls are immutable by default; mutations happen via functions that take a prior value and return a new value | authoring-guide rule | ✅ closed (doc 14 #38; consistent with #25 Haskell + #35 NumPy) |
| 108 | `gas_efficiency` QualityName registration | authoring-corpus seed | ⏳ queued (doc 14 #38; accumulates with #66/#91) |
| 109 | Reactive per-instance state = (state-data, transition-function, screen/view decl) triple — unifies #32 state-machine + #39 SwiftUI into one pattern | authoring-guide rule | ✅ closed (doc 14 #39) |
| 110 | Declarative view trees expressed as prose positional descriptions inside `screen` decl body | authoring-guide rule | ✅ closed (doc 14 #39) |
| 111 | View-modifier chains collapse to a single prose sentence per view component | authoring-guide rule | ✅ closed (doc 14 #39) |
| 112 | Callback props are `reference to function taking T returning U` in a props data decl | authoring-guide rule | ✅ closed (doc 14 #39; ties deferred D2) |
| 113 | Layout primitives as prose: `horizontal row of …`, `vertical column of …`, `stacked`, `trailing/leading spacer` | authoring-guide rule | ✅ closed (doc 14 #39; ties to `nom-ux` UX primitives) |
| 114 | System-image references as platform-resolved prose names (SF Symbols → Material → web icons) | authoring-guide rule | ✅ closed (doc 14 #39; Phase 12 specialization) |
| 115 | `scenario` top-level kind declaration (expands closed kind set 8→9 after W41 property) | **W46** grammar rule | ⏳ queued (doc 14 #40) |
| 116 | Scenario-clause grammar (`given`/`when`/`then` closed 3-keyword set; prose-sentence per clause) | **W47** grammar rule (pairs with W46) | ⏳ queued (doc 14 #40) |
| 117 | Repeat `given`/`when`/`then` keyword on every clause — no `and`-continuation abbreviation | authoring-guide rule | ✅ closed (doc 14 #40; enforced by W47) |
| 118 | Parameterized scenarios = N peer scenario decls OR property decl with matching generator | authoring-guide rule | ✅ closed (doc 14 #40; composes W46 + W41) |
| 119 | Background setup decomposes to shared setup function invoked explicitly in each scenario's `given` clauses — no implicit hooks | authoring-guide rule | ✅ closed (doc 14 #40) |
| 120 | Test fixtures live in named data decls, not inline in scenario prose | authoring-guide rule | ✅ closed (doc 14 #40) |
| 121 | Gherkin `Feature:` block → Nom concept decl whose index references constituent scenario decls | authoring-guide rule | ✅ closed (doc 14 #40) |
| 122 | Clock-domain clause (`at every rising/falling edge of CLOCK`) for synchronous-logic function decls | **W48** grammar rule | ⏳ queued (doc 14 #41) |
| 123 | Fixed-width integer types → `natural from 0 to (2^N)-1` / `integer from -(2^(N-1)) to (2^(N-1))-1` range-typed | authoring-guide rule | ✅ closed (doc 14 #41) |
| 124 | Hardware translations are pure state-transition functions; no blocking/non-blocking mutation choice exists | authoring-guide rule | ✅ closed (doc 14 #41; eliminates a Verilog bug-class) |
| 125 | Asynchronous-input domains declare synchronizer-chain requirement as `hazard` clause | authoring-guide rule | ✅ closed (doc 14 #41) |
| 126 | Multi-edge triggers decompose to peer transition functions + explicit precedence | authoring-guide rule | ✅ closed (doc 14 #41) |
| 127 | Verilog `module` → Nom composition decl with input/output data decls + transition function | authoring-guide rule | ✅ closed (doc 14 #41; same shape as #32/#38/#39) |
| 128 | `synthesizability` QualityName registration | authoring-corpus seed | ⏳ queued (doc 14 #41; accumulates to 4 seeds) |
| 129 | External builds declared in Nom inherit Nom's fixpoint discipline; unpinned inputs rejected at build time | authoring-guide rule | ✅ closed (doc 14 #42; reuses doc 04 §10.3.1) |
| 130 | External source fetches declare expected hash in `requires` clauses; build rejects on hash divergence | authoring-guide rule | ✅ closed (doc 14 #42) |
| 131 | Reproducible-build functions declare hermetic discipline via explicit `requires no ambient state` | authoring-guide rule | ✅ closed (doc 14 #42) |
| 132 | Nix-style laziness decomposes to eager prose in Nom; compiler/build-graph evaluates on demand at build time | authoring-guide rule | ✅ closed (doc 14 #42) |
| 133 | Recursive attrsets decompose to peer data decls with explicit `uses` references, not self-referential single records | authoring-guide rule | ✅ closed (doc 14 #42) |
| 134 | Dependency classes (native/runtime/dev/test) decompose to separate list-typed fields on build-inputs data decl | authoring-guide rule | ✅ closed (doc 14 #42) |
| 135 | Build phases decompose to named function decls composed in order (`configure then build then install`) — never embedded shell strings inside data | authoring-guide rule | ✅ closed (doc 14 #42) |
| 136 | Recursive-relation fixed-points decompose to three `ensures` clauses (base case, inductive step, depth/count bound); compiler chooses evaluation strategy | authoring-guide rule | ✅ closed (doc 14 #43) |
| 137 | SQL `UNION ALL` inside recursive CTEs is implicit in two-ensures decomposition; authors never name the set operator | authoring-guide rule | ✅ closed (doc 14 #43) |
| 138 | Recursion depth bounds live in an `ensures` clause, not a separate `limit` or `depth_cap` field | authoring-guide rule | ✅ closed (doc 14 #43) |
| 139 | CTEs lift to peer top-level decls — no local-scope CTE form in Nom | authoring-guide rule | ✅ closed (doc 14 #43; consistent with flat-namespace preference) |
| 140 | Relation-oriented functions specify post-conditions over the full result set (universal quantifiers), never per-row procedural steps | authoring-guide rule | ✅ closed (doc 14 #43; matches pure-functional discipline) |
| 141 | SQL `ORDER BY` / `GROUP BY` clauses map to `ensures the output is sorted by …` / `ensures the output is grouped by …` | authoring-guide rule | ✅ closed (doc 14 #43) |
| 142 | Stack-based implicit I/O decomposes to explicit named parameters and return-by-name; stack-juggle words collapse into naming variables | authoring-guide rule | ✅ closed (doc 14 #44) |
| 143 | Forth stack-effect comments (`( n -- n*n )`) map to `requires` (pre-stack) + `ensures` (post-stack) clause pair with named values | authoring-guide rule | ✅ closed (doc 14 #44) |
| 144 | Concatenative composition decomposes to named-intermediate prose expressions (same rule as doc 17 §I8 pipelines) | authoring-guide rule | ✅ closed (doc 14 #44) |
| 145 | No author-time `IMMEDIATE`/compile-time words in Nom; code-gen belongs in build-stage passes, never in source surface | authoring-guide rule | ✅ closed (doc 14 #44; matches #29 Lisp-macro rejection) |
| 146 | Forth `IF/THEN/ELSE` → `when … otherwise …` with named values | authoring-guide rule | ✅ closed (doc 14 #44) |
| 147 | Module signatures decompose to data decl with `reference to function` fields | authoring-guide rule | ✅ closed (doc 14 #45) |
| 148 | Functor applications decompose to peer module decls with `uses` clauses referencing abstract functor and signature witness | authoring-guide rule | ✅ closed (doc 14 #45) |
| 149 | Abstract type parameters of functors lift to `identifier`-typed fields on the signature's data decl | authoring-guide rule | ✅ closed (doc 14 #45) |
| 150 | Signature constraints on functor parameters = `uses @Data` typed-slot matches against signature data decl | authoring-guide rule | ✅ closed (doc 14 #45) |
| 151 | Nested modules lift to peer top-level module decls (flat-namespace preference) | authoring-guide rule | ✅ closed (doc 14 #45; same pattern as #30/#34) |
| 152 | Higher-kinded types are compile-time; authors write concrete module instances; resolver elides repetition via typed-slot matching | authoring-guide rule | ✅ closed (doc 14 #45; matches Phase 12 specialization) |
| 153 | Policy defaults decompose to explicit `ensures … by default when no … rule matches` clause | authoring-guide rule | ✅ closed (doc 14 #46) |
| 154 | Disjunctive rule bodies collapse to one function with multiple independent `ensures` clauses (OR semantics) | authoring-guide rule | ✅ closed (doc 14 #46) |
| 155 | Implicit globals (`input`, `data`, `env`, `ctx`) decompose to explicit typed parameters | authoring-guide rule | ✅ closed (doc 14 #46; same rule as #38 Solidity) |
| 156 | Quantifier-vocabulary lock (`every`/`no`/`some`/`at-least N`/`at-most N`/`exactly N` as reserved quantifier tokens) | **W49** grammar rule (retroactive payoff across Prolog/SQL-CTE/Rego/Hypothesis/…) | ⏳ queued (doc 14 #46) |
| 157 | Membership checks as prose `has X among Y` / `is X or Y or Z` — no `in` operator | authoring-guide rule | ✅ closed (doc 14 #46; non-symbol discipline) |
| 158 | Helper rules decompose to peer function decls | authoring-guide rule | ✅ closed (doc 14 #46) |
| 159 | Policy composition = disjunctive-ensures pattern; no separate composition keyword | authoring-guide rule | ✅ closed (doc 14 #46) |
| 160 | TLA+ primed variables (`x'`) decompose to input/output parameters on pure transition functions | authoring-guide rule | ✅ closed (doc 14 #47; same as #41 hardware-RTL) |
| 161 | Temporal `[]P` (always) → `checks for every reachable state, P`; `<>P` (eventually) → `checks some reachable state satisfies P` | authoring-guide rule | ✅ closed (doc 14 #47; reuses W41 + W49) |
| 162 | Disjunctive actions (TLA+ `Next == A \/ B`) → multiple `ensures one valid successor has …` clauses | authoring-guide rule | ✅ closed (doc 14 #47; consistent with #46 Rego) |
| 163 | TLA+ `Spec == Init /\ [][Next]_v` → composition decl (`init then step`) + property decl quantifying over reachable states | authoring-guide rule | ✅ closed (doc 14 #47) |
| 164 | Model-check depth bounds use existing `covers …` clause on the property (same as Hypothesis #33) | authoring-guide rule | ✅ closed (doc 14 #47) |
| 165 | Kind-of-claim (invariant/safety/liveness) stated in `intended to` + `for every`/`some` quantifier — no separate keyword | authoring-guide rule | ✅ closed (doc 14 #47; keeps property decl surface uniform) |
| 166 | Formal-methods tool choice (TLC/Apalache/Coq/Lean/Alloy) is build-stage specialization; source property decl is tool-agnostic | authoring-guide rule | ✅ closed (doc 14 #47) |
| 167 | PDDL typed parameters (`?x - block`) → `requires x is in <typed-list>` clauses on the action function | authoring-guide rule | ✅ closed (doc 14 #48) |
| 168 | PDDL predicates decompose to list-typed fields on state data decl; each list holds tuples for which predicate is true | authoring-guide rule | ✅ closed (doc 14 #48) |
| 169 | PDDL actions (`:action :parameters :precondition :effect`) → function decl + `requires` + `ensures` | authoring-guide rule | ✅ closed (doc 14 #48; same state-data/transition-fn pattern as #32/#38/#39/#41/#47) |
| 170 | PDDL negation `(not P)` → prose `is not in …` / `is nothing` in `requires` clauses | authoring-guide rule | ✅ closed (doc 14 #48) |
| 171 | PDDL problem instances `(:init … :goal …)` → property decls quantifying existentially over action sequences | authoring-guide rule | ✅ closed (doc 14 #48; same shape as TLA+ liveness) |
| 172 | PDDL `:requirements` feature-flags are no-op in Nom — uniform grammar covers STRIPS/ADL/fluents without per-feature opt-in | authoring-guide rule | ✅ closed (doc 14 #48) |
| 173 | `minimum_cost` QualityName registration (for planning, path-finding, scheduling) | authoring-corpus seed | ⏳ queued (doc 14 #48; accumulates to 5 seeds) |
| 174 | Diagrams are `screen` decls that reference the composition they visualize via `uses @Composition` | authoring-guide rule | ✅ closed (doc 14 #49) |
| 175 | Mermaid participants decompose to named function decls; participant name = function location in composition graph | authoring-guide rule | ✅ closed (doc 14 #49) |
| 176 | Mermaid `alt`/`else` blocks → composition decls with conditional branching (same prose as if-then-else; W40 MECE validates) | authoring-guide rule | ✅ closed (doc 14 #49) |
| 177 | Mermaid arrow styles (sync/async/response) decompose to prose clauses on screen decl describing message kind | authoring-guide rule | ✅ closed (doc 14 #49) |
| 178 | Diagram kind (sequence/class/flowchart/ER/state) declared via `intended to` + `the diagram shape is …`; renderer dispatches to Mermaid layout | authoring-guide rule | ✅ closed (doc 14 #49) |
| 179 | Diagrams referencing compositions via `uses` auto-regenerate when composition changes; free-standing prose diagrams rejected (anti-drift by construction) | authoring-guide rule | ✅ closed (doc 14 #49; major correctness win) |
| 180 | Dafny loop invariants → peer property decls whose `checks` clauses name the containing function's reachable states | authoring-guide rule | ✅ closed (doc 14 #50) |
| 181 | `decreases` termination clauses → `checks … strictly decreases on every iteration` in peer property decl | authoring-guide rule | ✅ closed (doc 14 #50) |
| 182 | Dafny `requires`/`ensures` contracts map 1:1 to Nom's existing clauses (reinforces existing surface) | authoring-guide rule | ✅ closed (doc 14 #50) |
| 183 | Imperative while-loops → prose invariant on function + property decl with verification obligations + compiler code-gen | authoring-guide rule | ✅ closed (doc 14 #50; same shape as #25/#35/#43) |
| 184 | Dafny `forall` quantifier → W49 `every` in prose — reuses the quantifier-vocabulary lock | authoring-guide rule | ✅ closed (doc 14 #50) |
| 185 | Verification-tool choice (Z3/CVC5/Alt-Ergo/Lean/Coq) = build-stage specialization; property decl is tool-agnostic | authoring-guide rule | ✅ closed (doc 14 #50; unifies with #47 + #48) |
| 186 | Executable + verification code stay in same `.nomtu`/`.nom` unit as peer function + property decls; shared hash-pinned lock | authoring-guide rule | ✅ closed (doc 14 #50) |
| 187 | WAT typed locals → named values with range-typed naturals or integers; type prefix on ops (`i32.add`) is build-stage selection | authoring-guide rule | ✅ closed (doc 14 #51; merges with #41 Verilog + #44 Forth) |
| 188 | WAT `memory`/`table`/`global` declarations → peer data decls on the Nom module | authoring-guide rule | ✅ closed (doc 14 #51) |
| 189 | WAT structured control flow (`block`/`loop`/`br`/`br_if`) → prose control words (`loop`, `when`, `otherwise`, `exit`) | authoring-guide rule | ✅ closed (doc 14 #51; same rule as #44 Forth row #146) |
| 190 | Fixed-width integer overflow behavior declared explicitly as `hazard` (wrap/saturate/trap are 3 distinct semantics, must be chosen not defaulted) | authoring-guide rule | ✅ closed (doc 14 #51) |
| 191 | WAT exports/imports map to Nom composition index references; no separate export keyword needed | authoring-guide rule | ✅ closed (doc 14 #51; reuses doc 08 composition index) |
| 192 | WAT text/binary as authoring-vs-build artifact; `.nomtu` → WASM binary is plausible Phase-12 specialization target without grammar extension | authoring-guide rule | ✅ closed (doc 14 #51) |
| 193 | Regression formulas (`y ~ x1 + x2 + factor(z)`) decompose to prose listing outcome + predictor `exposes` fields; `factor()` treatment stated in `intended to` | authoring-guide rule | ✅ closed (doc 14 #52) |
| 194 | R/tidyverse `%>%` pipe chains decompose to named intermediate values (reuses doc 17 §I8) | authoring-guide rule | ✅ closed (doc 14 #52) |
| 195 | Dataframes decompose to `list of T` where T is a row-schema data decl; columns are fields on T (same pattern as #43 SQL CTE) | authoring-guide rule | ✅ closed (doc 14 #52) |
| 196 | Statistical-model outputs decompose to data decls with parallel list fields (one list per output column) | authoring-guide rule | ✅ closed (doc 14 #52) |
| 197 | Categorical factor treatment declared in fit function's `intended to` sentence; build-stage handles indicator-variable expansion | authoring-guide rule | ✅ closed (doc 14 #52) |
| 198 | `statistical_rigor` QualityName registration | authoring-corpus seed | ⏳ queued (doc 14 #52; accumulates to 6 seeds) |
| 199 | R non-standard evaluation is a no-op in Nom; capture the intended computation as explicit function/data decl | authoring-guide rule | ✅ closed (doc 14 #52) |
| 200 | OpenAPI type constraints (`minLength`/`maxLength`/`format`/`minimum`/`maximum`/`pattern`) → prose range descriptors on `exposes` fields | authoring-guide rule | ✅ closed (doc 14 #53) |
| 201 | `@Route` typed-slot kind — extends @Kind vocabulary for HTTP/gRPC/event/CLI routing | **W50** grammar rule | ⏳ queued (doc 14 #53; narrow; ships gRPC + event-handler + CLI dispatch) |
| 202 | HTTP status codes → prose outcome-class descriptors on `ensures` clauses; exact numeric codes are build-stage dispatch | authoring-guide rule | ✅ closed (doc 14 #53; keeps source HTTP-framework-agnostic) |
| 203 | Request and response body schemas decompose to separate data decls — never merged via optional fields | authoring-guide rule | ✅ closed (doc 14 #53; prevents common OpenAPI schema-overloading mistake) |
| 204 | Path/query/header parameters → `requires` clauses typed against function parameter list | authoring-guide rule | ✅ closed (doc 14 #53) |
| 205 | Content-type specified via `ensures … serialized as <format>` when non-default; JSON is implicit default | authoring-guide rule | ✅ closed (doc 14 #53) |
| 206 | Concept decl as route-map container (binds N route strings via `@Route` typed-slot to N function decls) | authoring-guide rule | ✅ closed (doc 14 #53; new idiom for doc 17) |
| 207 | Declarative-orchestration decomposes to (desired-state data decl + reconcile function with eventual-consistency `ensures`) | authoring-guide rule | ✅ closed (doc 14 #54) |
| 208 | Label-selector metadata → named identifier-typed fields on desired-state data decl | authoring-guide rule | ✅ closed (doc 14 #54) |
| 209 | K8s resource quantities with unit suffixes (`250m`, `256Mi`) → plain SI-base-unit natural ranges; build stage handles formatting | authoring-guide rule | ✅ closed (doc 14 #54) |
| 210 | Deeply-nested specs → peer data decls with `reference to T` fields; no more than one level of nesting per data decl | authoring-guide rule | ✅ closed (doc 14 #54; same as #30/#34/#38) |
| 211 | Image-tag pinning as `requires` constraint; floating tags (`latest`) rejected at authoring time | authoring-guide rule | ✅ closed (doc 14 #54; prevents a major class of production incidents) |
| 212 | K8s probe kinds (readiness/liveness/startup) → peer data decls sharing common schema with `probe_kind` discriminator | authoring-guide rule | ✅ closed (doc 14 #54) |
| 213 | Multi-resource YAML manifests → multi-decl `.nomtu` files (1 data + 1 reconcile fn per resource + 1 composition for rollout order) | authoring-guide rule | ✅ closed (doc 14 #54) |
| 214 | `availability` QualityName registration | authoring-corpus seed | ⏳ queued (doc 14 #54; accumulates to 7 seeds) |
| 215 | The Elm Architecture IS Nom's unified (state-data, transition-function, screen) reactive-decomposition pattern — cross-ref with #71 + #109 | authoring-guide rule | ✅ closed (doc 14 #55) |
| 216 | Elm algebraic Msg types → data decls with multiple `exposes … at tag N` fields (same as #22 Kotlin sealed) | authoring-guide rule | ✅ closed (doc 14 #55) |
| 217 | Record-update syntax elided in prose — authors state new field values; build stage derives update mechanics | authoring-guide rule | ✅ closed (doc 14 #55; same shape as #50) |
| 218 | Elm `case of` exhaustive pattern match → `when … otherwise …` prose + `ensures exactly one branch fires` (W40) | authoring-guide rule | ✅ closed (doc 14 #55) |
| 219 | Purity is Nom's default stance — boundary-crossing effects flagged via `hazard`; no extra marker needed | authoring-guide rule | ✅ closed (doc 14 #55) |
| 220 | Event-handler bindings in reactive UIs → prose `emits CounterMessage when tapped` inside screen decl layout | authoring-guide rule | ✅ closed (doc 14 #55; same as #39 SwiftUI callback props) |

Totals by destination (after doc 14 #55 Elm pure-FRP translation — **fourteenth 0-new-wedge translation in a row**; 6 authoring-guide closures; **reactive-UI paradigm closed across three concrete frameworks** XState/SwiftUI/Elm with zero per-framework adaptation):

- ⏳ Wedge queued: **43** (unchanged)
- 🧪 Smoke-test todo: **1**
- 📘 Authoring-guide doc-todo: **0**
- ✅ Closed: **170**
- 🧠 Design deferred (open): **0**
- 🔒 Blocked: **2**
- 🌱 Authoring-corpus seed: **7** (forward_compatibility + numerical_stability + gas_efficiency + synthesizability + minimum_cost + statistical_rigor + availability QualityNames)

Backlog size: 231 rows. Closure rate 74% (170/231). **55 translations** in doc 14. Paradigm coverage: imperative + OOP + async + concurrency + pure-functional + ADT + data + shell + build + container + editor-event + CI/CD + math-as-language + actor-model + logic-programming + metaprogramming + schema-IDL + pattern-DSL + state-machine-DSL + property-based-testing + infrastructure-as-code + array-programming + workflow-orchestration + stream-processing + smart-contract + declarative-reactive-UI + BDD-scenario + hardware-description-RTL + purely-functional-package-spec + recursive-relational-query + stack-based-concatenative + parameterized-modules + policy-DSL + temporal-logic-model-checking + AI-planning + visualization-as-code + verified-imperative-programming + portable-binary-target + statistical-computing + HTTP-API-spec + container-orchestration + **pure-FRP-Model-Update-View (Elm)**. Twenty-second consecutive minimal-wedge translation, fourteenth 0-new-wedge. **The Elm Architecture officially named as Nom's unified reactive-decomposition pattern.**

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
- **W43:** Retry-policy clause — small-vocabulary orchestrator directive: `up to N times with delay D, backoff linear|exponential|constant`. Attaches to composition or individual function decl; orchestrator honors the declaration at build-time.
- **W44:** Watermark clause — `watermark the stream at FIELD with an N-UNIT allowed lateness` for event-time correctness on streaming sources. Closed vocabulary for late-arrival policies.
- **W45:** Window-aggregation clause — `window of D over FIELD` with closed kinds `tumbling`/`sliding`/`session`/`global`. Pairs with W40 exhaustiveness-check for per-window totality. Core stream-processing primitive.
- **W46:** `scenario` top-level kind declaration — expands closed kind set 8→9 nouns (8th was W41 `property`). Asserted-behavior claim: named precondition/action/postcondition triple. Covers Gherkin, RSpec behavior blocks, Playwright test descriptions. Orthogonal to function/data/concept/property.
- **W47:** Scenario-clause grammar — closed 3-keyword set `given`/`when`/`then`; each clause is a prose sentence. Ships paired with W46. Keyword repeats on every clause (no `and`-continuation).
- **W48:** Clock-domain clause — `at every rising edge of CLOCK` / `at every falling edge of CLOCK` attaches to synchronous-logic function decls. Expresses temporal contracts without adding a new kind; sits alongside `requires`/`ensures`. Narrow closed vocabulary.
- **W49:** Quantifier-vocabulary lock — reserve `every`/`no`/`some`/`at-least N`/`at-most N`/`exactly N` as quantifier tokens inside `requires`/`ensures` clauses. Retroactive payoff wedge: disambiguates ensure-clause parsing across 15+ prior paradigm translations (Prolog/SQL-CTE/Rego/Hypothesis/property-tests/…).
- **W50:** `@Route` typed-slot kind — narrow extension to the @Kind vocabulary (currently `@Function`/`@Data`/`@Module`/`@Concept`/`@Composition`) for HTTP method+path routes, gRPC methods, event handlers, CLI subcommands. Uses the existing typed-slot resolver; `@Route matching "GET /todos"` syntax. Small wedge, broadly applicable.

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
