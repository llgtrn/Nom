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
| 221 | Variadic-input macros → variadic function decls (`takes zero or more values`) + indexed positional post-conditions | authoring-guide rule | ✅ closed (doc 14 #56; reinforces #29 Lisp-macro rejection) |
| 222 | Syntactic-niceties at source-token level (trailing commas, optional semicolons) are invisible in Nom translations | authoring-guide rule | ✅ closed (doc 14 #56) |
| 223 | Macro hygiene is vacuous in Nom — no call-site expansion means no capture hazard (reinforces closure-lifting #29 + D2) | authoring-guide rule | ✅ closed (doc 14 #56) |
| 224 | Macros-as-generics → typed-slot parameters in function decls (same shape as #45 OCaml-functor sig params) | authoring-guide rule | ✅ closed (doc 14 #56) |
| 225 | Procedural macros (`#[derive(X)]`) → build-stage transformations authored as Nom function decls consumed by build pipeline, not source annotations | authoring-guide rule | ✅ closed (doc 14 #56) |
| 226 | Identifier-synthesizing macros rejected; feature-stack identifiers authored explicitly | authoring-guide rule | ✅ closed (doc 14 #56; matches MEMORY.md feature-stack roadmap) |
| 227 | jq pipe operators decompose to named intermediate prose (reuses doc 17 §I8 + R-pipe rule from #52) | authoring-guide rule | ✅ closed (doc 14 #57) |
| 228 | jq path expressions → prose positional phrases; source data decl `exposes` fields provide type surface | authoring-guide rule | ✅ closed (doc 14 #57) |
| 229 | jq `select(P)` → `for each X where P …` prose OR two-sided `ensures` set-equality (same as #46 Rego / #43 SQL CTE) | authoring-guide rule | ✅ closed (doc 14 #57) |
| 230 | jq inline object constructors → named data decls with prose-extraction (flat-namespace per #34/#54) | authoring-guide rule | ✅ closed (doc 14 #57) |
| 231 | jq stream-generator semantics → explicit list-returning functions with two-sided `ensures` set-equality | authoring-guide rule | ✅ closed (doc 14 #57) |
| 232 | Iteration vs indexing distinguished in prose (`for each X in …` vs `the Nth X in …`); never via bracket syntax | authoring-guide rule | ✅ closed (doc 14 #57) |
| 233 | jq null-on-missing-path → `perhaps T` at schema level (same as #55 Elm optionality discipline) | authoring-guide rule | ✅ closed (doc 14 #57) |
| 234 | Fixed-point decimal → `real from X to Y.YY` + `hazard` on floating-point representation; audit-grade code requires explicit decimal precision | authoring-guide rule | ✅ closed (doc 14 #58; W51 decimal-precision candidate noted, not urgent) |
| 235 | COBOL divisions (IDENTIFICATION / DATA / PROCEDURE / ENVIRONMENT) → peer top-level decls in `.nomtu`; division role implicit in decl kind | authoring-guide rule | ✅ closed (doc 14 #58) |
| 236 | COBOL level numbers (01/05/77) → peer data decls with `reference to T` fields; nested record shapes lift to flat peers | authoring-guide rule | ✅ closed (doc 14 #58; reuses #30 + #54 flat-namespace) |
| 237 | COBOL working-storage globals → explicit function parameters (inputs) + peer data decls (shared constants / policies) | authoring-guide rule | ✅ closed (doc 14 #58) |
| 238 | COBOL arithmetic verbs (`COMPUTE`/`ADD TO`/`SUBTRACT FROM`/`MULTIPLY BY`/`DIVIDE INTO`) → `ensures X equals Y op Z` clauses | authoring-guide rule | ✅ closed (doc 14 #58) |
| 239 | COBOL `DISPLAY` / I/O statements → peer formatting functions; I/O not inline in computation functions | authoring-guide rule | ✅ closed (doc 14 #58; preserves purity) |
| 240 | Rounding mode stated explicitly in `ensures` clauses (banker's / half-up / truncate / floor / ceiling); no default | authoring-guide rule | ✅ closed (doc 14 #58) |
| 241 | `auditability` QualityName registration | authoring-corpus seed | ⏳ queued (doc 14 #58; accumulates to 8 seeds) |
| 242 | Bash vs PowerShell distinction (string-pipes vs object-pipes) collapses at Nom source level; choice is build-stage target selection | authoring-guide rule | ✅ closed (doc 14 #59) |
| 243 | Implicit pipeline-item references (`$_`/`it`/`self`/`this`) → explicit named values in prose | authoring-guide rule | ✅ closed (doc 14 #59) |
| 244 | PowerShell verb-noun cmdlets → Nom function decls with feature-stack names; verb-noun is a naming convention, not a grammar feature | authoring-guide rule | ✅ closed (doc 14 #59; aligns with MEMORY.md feature-stack roadmap) |
| 245 | Named parameters always (PowerShell `-Path`); positional parameters rejected outside single-argument functions | authoring-guide rule | ✅ closed (doc 14 #59; prevents call-site ambiguity) |
| 246 | Inline script-block predicates decompose to named filter functions OR `ensures every X satisfies P` clauses | authoring-guide rule | ✅ closed (doc 14 #59) |
| 247 | Filesystem-enumeration non-atomicity is a `hazard` on any recursive-scan function | authoring-guide rule | ✅ closed (doc 14 #59) |
| 248 | Idempotence declared explicitly via `ensures the operation is idempotent — a second call with the same spec makes no further changes` | authoring-guide rule | ✅ closed (doc 14 #60) |
| 249 | Jinja2 / Go-template / ERB / Liquid in-string interpolation → explicit data-decl references; no `{{ }}` at source level | authoring-guide rule | ✅ closed (doc 14 #60) |
| 250 | Deferred-handler scheduling → change-flag return + peer handler-scheduler function at end of composition; no implicit `changed=true fires handler` magic | authoring-guide rule | ✅ closed (doc 14 #60) |
| 251 | Privilege-escalation requirements → `requires the caller has <role> rights` clauses on function decls | authoring-guide rule | ✅ closed (doc 14 #60) |
| 252 | Ansible inventory targeting → host-group identifier input parameters; inventory resolution is build-stage | authoring-guide rule | ✅ closed (doc 14 #60) |
| 253 | Task-order and handler-order captured by composition `then` chain + handler-scheduler function; no separate ordering keyword | authoring-guide rule | ✅ closed (doc 14 #60) |
| 254 | Multi-host parallel execution → build-stage higher-order wrapper over per-host composition | authoring-guide rule | ✅ closed (doc 14 #60; reuses Phase 12 specialization principle) |
| 255 | Array-slice stencil arithmetic → prose stencil descriptions inside `ensures` clauses; compiler stencil pass generates per-cell loops or SIMD kernels | authoring-guide rule | ✅ closed (doc 14 #61; same pattern as #35 NumPy) |
| 256 | Implicit-DO array assignments (`T_new(1:N, 1:N) = f(T)`) → `ensures every X …` quantified clauses (uses W49) | authoring-guide rule | ✅ closed (doc 14 #61) |
| 257 | Numerical-stability conditions (CFL, Courant, Reynolds-limit) → explicit `requires` + accompanying `hazard` on violation | authoring-guide rule | ✅ closed (doc 14 #61; moves a bug-class from runtime to authoring-time) |
| 258 | Boundary-vs-interior distinction stated explicitly in `ensures` clauses; never implicit from slice arithmetic | authoring-guide rule | ✅ closed (doc 14 #61) |
| 259 | Implicit-typing conventions rejected; every Nom value has an explicit type via `exposes` or prose range | authoring-guide rule | ✅ closed (doc 14 #61) |
| 260 | Fortran PARAMETER constants → data-decl fields on a dedicated policy data decl (like PayrollPolicy in #58) | authoring-guide rule | ✅ closed (doc 14 #61) |
| 261 | Fortran PROGRAM/SUBROUTINE/FUNCTION distinctions collapse to plain Nom function decls; entry-point status is build-stage configuration | authoring-guide rule | ✅ closed (doc 14 #61) |
| 262 | Push-subscription functions → callbacks data decl + subscribe fn with per-callback `ensures` + unsubscribe-handle return | authoring-guide rule | ✅ closed (doc 14 #62) |
| 263 | Observable/Rx callback triples (`next`/`error`/`complete`) → callbacks data decl with 3 `reference to function` fields | authoring-guide rule | ✅ closed (doc 14 #62) |
| 264 | GraphQL operation variables (`$channelId`) → Nom function parameters with explicit types; no `$` prefix at source | authoring-guide rule | ✅ closed (doc 14 #62) |
| 265 | Delivery-ordering scope (per-channel/per-key/global) declared as explicit `hazard` clause on subscription functions | authoring-guide rule | ✅ closed (doc 14 #62) |
| 266 | Transport-layer silent-disconnect is standard `hazard` on persistent-connection functions; callers own heartbeat+reconnect logic | authoring-guide rule | ✅ closed (doc 14 #62) |
| 267 | Persistent-subscription functions return unsubscribe handle as primary return (matches #12 event-listener pattern) | authoring-guide rule | ✅ closed (doc 14 #62) |
| 268 | Message-delivery semantics (at-most-once / exactly-once / at-least-once) stated explicitly via W49 quantifiers in `ensures` clauses | authoring-guide rule | ✅ closed (doc 14 #62; reinforces W49 payoff) |
| 269 | Atomic-group operations → single function decl with `ensures … happen as a single atomic group from other observers' perspectives` | authoring-guide rule | ✅ closed (doc 14 #63) |
| 270 | Compare-and-set operations → two-branch `ensures` clauses (success path + no-op path, both explicit) | authoring-guide rule | ✅ closed (doc 14 #63) |
| 271 | TTL fields on state-carrying data decls → `natural from 1 to N` with explicit upper bound; unbounded TTLs rejected | authoring-guide rule | ✅ closed (doc 14 #63; prevents runaway-memory bugs) |
| 272 | Pub/sub broadcast "active-at-publication-time" via paired `ensures` + `hazard`; durable delivery requires a different primitive | authoring-guide rule | ✅ closed (doc 14 #63) |
| 273 | Cluster-failover consistency gaps → explicit `hazard` on multi-step atomic groups that span commit boundaries | authoring-guide rule | ✅ closed (doc 14 #63) |
| 274 | Distributed-lock TTL-vs-work-duration → two-hazard pair (holder-crash + work-outlasts-TTL); callers own duration bounds | authoring-guide rule | ✅ closed (doc 14 #63) |
| 275 | Redis key-naming conventions (colon hierarchies) → build-stage key-derivation function from named data-decl fields; source never hardcodes key strings | authoring-guide rule | ✅ closed (doc 14 #63; prevents key-collision bugs) |
| 276 | gRPC RPC kinds (unary/server-stream/client-stream/bidi) decompose via W49-quantified `ensures` patterns; no `stream` keyword at Nom source | authoring-guide rule | ✅ closed (doc 14 #64) |
| 277 | Bidirectional-streaming half-close semantics → explicit `ensures closing one peer's half does not close the other peer's` | authoring-guide rule | ✅ closed (doc 14 #64) |
| 278 | gRPC service packages → Nom concept decls via W50 `@Route` typed-slot + shared route prefix; one concept per gRPC service | authoring-guide rule | ✅ closed (doc 14 #64; reuses #53 concept-as-route-map) |
| 279 | Long-lived server-streaming RPCs declare resumability guidance as `hazard` with cursor advice; caller owns resume logic | authoring-guide rule | ✅ closed (doc 14 #64) |
| 280 | Transport-layer reordering on bidi-streams → application-level sequence tracking delegated to callers via explicit `hazard` | authoring-guide rule | ✅ closed (doc 14 #64) |
| 281 | RPC deadlines and cancellation tokens → `requires` (deadline-bound on caller) + `hazard` (cancellation may surface mid-stream) | authoring-guide rule | ✅ closed (doc 14 #64) |
| 282 | Typeset documents (LaTeX/Typst/Pandoc/AsciiDoc) → `screen` decls with layout prose; cross-references as typed-slot references | authoring-guide rule | ✅ closed (doc 14 #65; reinforces #49 screen generalization) |
| 283 | Math content stays as text fields on `Equation` data decls; semantic math goes in `property`/`concept` decls (#26 Lean pattern) | authoring-guide rule | ✅ closed (doc 14 #65) |
| 284 | Cross-reference resolution via build-stage `ensures every referenced label is declared` or peer validation function | authoring-guide rule | ✅ closed (doc 14 #65; prevents dead-link errors at authoring time) |
| 285 | LaTeX `\usepackage` directives are no-op in Nom; rendering engine provides full feature set as build-stage target selection | authoring-guide rule | ✅ closed (doc 14 #65) |
| 286 | LaTeX math-mode environments (`$...$`/`\[...\]`/`align`/`equation`) → prose context on screen decl's layout description | authoring-guide rule | ✅ closed (doc 14 #65) |
| 287 | Build-time content macros (`\today`/`\pagenumber`/`\thepage`) → data-decl fields populated by build-stage render-time injection | authoring-guide rule | ✅ closed (doc 14 #65) |
| 288 | `accessibility` QualityName registration | authoring-corpus seed | ⏳ queued (doc 14 #65; accumulates to 9 seeds — approaching 10-seed formalization threshold) |
| 289 | Multiple-dispatch → per-concrete-type named functions + dispatch function whose `ensures` enumerates each type's branch; no function-name overloading at Nom source | authoring-guide rule | ✅ closed (doc 14 #66) |
| 290 | Abstract-type hierarchies + subtyping (Julia `abstract type` + `<:`) → sum-type data decls with tagged variants (reuses #22 + #55) | authoring-guide rule | ✅ closed (doc 14 #66) |
| 291 | Parametric-type subtype bounds (`Vector{<:Shape}`) → `list of <tagged-union-data-decl>` | authoring-guide rule | ✅ closed (doc 14 #66) |
| 292 | Julia `sum(fn, xs)` higher-order → explicit prose `sum of every element's contribution` with per-variant `ensures`; function-as-first-arg elided | authoring-guide rule | ✅ closed (doc 14 #66) |
| 293 | Unicode identifiers (Julia `π`/`α`/`∑`) → English prose names; no non-ASCII at Nom source level (reinforces MEMORY.md English-vocabulary invariant) | authoring-guide rule | ✅ closed (doc 14 #66) |
| 294 | Julia per-type method specialization → Nom Phase 12 closure-level specialization (reinforces existing principle) | authoring-guide rule | ✅ closed (doc 14 #66) |
| 295 | Zig error-union return types (`Err!T`) → named failure-data-decl (tagged-variant) + multi-variant `ensures` clauses specifying exactly when each variant returns | authoring-guide rule | ✅ closed (doc 14 #67; same shape as #38 Solidity typed-errors) |
| 296 | Zig declared error sets → named data decls with tagged variants; error-set membership is the decl's `exposes` field list | authoring-guide rule | ✅ closed (doc 14 #67) |
| 297 | Zig `comptime` parameters/values → `ensures the returned value is a build-time constant` clause + `favor performance`; no `comptime` keyword at Nom source | authoring-guide rule | ✅ closed (doc 14 #67) |
| 298 | Integer arithmetic that may overflow declares the overflow branch as explicit `ensures` variant + `hazard` (wrap/saturate/trap) | authoring-guide rule | ✅ closed (doc 14 #67; reinforces #51 WAT + #41 Verilog) |
| 299 | "No hidden control flow" invariant satisfied by default in Nom: every control-flow branch is an `ensures` clause; no exceptions or hidden jumps | authoring-guide rule | ✅ closed (doc 14 #67) |
| 300 | Zig slices (`[]T`) → `list of T` or `byte sequence` with implicit length reference; caller carries the length | authoring-guide rule | ✅ closed (doc 14 #67) |
| 301 | Exhaustive path coverage stated at function-decl level via multiple `ensures` variants; no `return` statements at Nom source level | authoring-guide rule | ✅ closed (doc 14 #67) |
| 302 | MATLAB matrices → shape-carrying data decl with rows/columns fields + nested-list cells; shape constraints in per-op `requires`/`ensures` | authoring-guide rule | ✅ closed (doc 14 #68) |
| 303 | Multi-value returns → bundled data-decl returns; destructuring is caller-side per doc 17 §I10 | authoring-guide rule | ✅ closed (doc 14 #68) |
| 304 | Output-shape constraints stated via `ensures` clauses referencing named input fields (e.g., `A.rows`) | authoring-guide rule | ✅ closed (doc 14 #68) |
| 305 | MATLAB slicing (`A(:, 1:k)` etc.) → prose positional descriptions (`the first k rows/columns/top-left block`) — reuses #61 Fortran | authoring-guide rule | ✅ closed (doc 14 #68) |
| 306 | String-flag function variants (MATLAB `'econ'`/`'full'`) → distinct Nom function decls; prevents typo-caused runtime errors | authoring-guide rule | ✅ closed (doc 14 #68) |
| 307 | MATLAB `assert(P, msg)` → `requires P` at function-decl; build stage checks statically where possible, runtime where necessary | authoring-guide rule | ✅ closed (doc 14 #68) |
| 308 | Matrix transposes decomposed to prose (`the transpose of X`); no operator at Nom source | authoring-guide rule | ✅ closed (doc 14 #68) |
| 309 | Shape-query functions (`size(A)`) → direct field access on shape-carrying data decl | authoring-guide rule | ✅ closed (doc 14 #68) |
| 310 | Smalltalk message-sends → plain function decls with receiver as first parameter; no postfix-send operator (reuses #23 Ruby rule) | authoring-guide rule | ✅ closed (doc 14 #69) |
| 311 | Smalltalk `self`/`this`/implicit receivers → explicit-parameter access; no implicit receiver at Nom source (reuses #46 / #38) | authoring-guide rule | ✅ closed (doc 14 #69) |
| 312 | Smalltalk `ifTrue:`/`ifFalse:` with block arguments → `when P … otherwise …` prose; non-trivial blocks closure-lifted per doc 19 D2 | authoring-guide rule | ✅ closed (doc 14 #69) |
| 313 | Mutable-instance-variable assignments → functions returning fresh instance with updated field (reuses #50 Dafny + #55 Elm) | authoring-guide rule | ✅ closed (doc 14 #69) |
| 314 | Smalltalk `^` returns → function-decl `returns` + `ensures` description (reuses #67 Zig rule 301) | authoring-guide rule | ✅ closed (doc 14 #69) |
| 315 | Smalltalk `error:` convention → tagged-variant error data decl + per-variant `ensures` (reuses #38 / #67 / #25 error-family) | authoring-guide rule | ✅ closed (doc 14 #69) |
| 316 | Class-method vs instance-method distinction collapses to plain function decls in Nom; difference is whether the decl takes a receiver parameter | authoring-guide rule | ✅ closed (doc 14 #69) |
| 317 | Cross-aggregate atomicity → `ensures … appear as a single atomic group` + `hazard` naming distributed-coordination requirement (reuses #63 Redis cluster-failover pattern) | authoring-guide rule | ✅ closed (doc 14 #69) |
| 318 | Ada subtype range constraints → range-typed integers/naturals on `exposes` fields + `requires` clauses; runtime checks become build-stage static where decidable | authoring-guide rule | ✅ closed (doc 14 #70; reuses #41/#51/#58/#66 range-typed-integer rule) |
| 319 | Ada `out`/`in out` parameter modes → return-fresh-instance from data decl; no mutable-parameter passing at Nom source | authoring-guide rule | ✅ closed (doc 14 #70) |
| 320 | Ada task types → state-carrying data decl + per-entry function decls; task-scheduling semantics are build-stage specialization | authoring-guide rule | ✅ closed (doc 14 #70; reuses #27 Elixir GenServer + #32 XState) |
| 321 | Ada `if/elsif/else/end if` → `when X … when Y … otherwise …` prose (reuses #50 Dafny pattern) | authoring-guide rule | ✅ closed (doc 14 #70) |
| 322 | Range-typed boundary-clamping is an explicit `hazard` on safety-critical functions; fault-on-boundary is opt-in via peer verification | authoring-guide rule | ✅ closed (doc 14 #70) |
| 323 | Ada `package body` vs package spec → collapses to single `.nomtu` file; interface/implementation split is build-stage concern | authoring-guide rule | ✅ closed (doc 14 #70) |
| 324 | Ada run-time constraint checks → Nom build-stage static checks (preferred) plus runtime-check fallback for undecidable cases | authoring-guide rule | ✅ closed (doc 14 #70) |
| 325 | STM transactions → function decls with atomic-group `ensures` + concurrency-fairness `ensures`; underlying STM is build-stage concern | authoring-guide rule | ✅ closed (doc 14 #71; reuses #63 Redis atomic-group + #69 Smalltalk cross-aggregate) |
| 326 | Ref/atom/agent dereference and update operators elided in prose; use `prior X` and `returned X` on function decls | authoring-guide rule | ✅ closed (doc 14 #71) |
| 327 | Immutable-map modifications (Clojure `assoc`/`update`, ImmutableJS, persistent trees) → `ensures` describing returned contents relative to prior collection | authoring-guide rule | ✅ closed (doc 14 #71; reuses #55 Elm + #69 Smalltalk) |
| 328 | Free-form exceptions with attached payloads (`ex-info`, Python `raise X(msg, data)`) → tagged-variant error data decls with per-variant fields carrying payload | authoring-guide rule | ✅ closed (doc 14 #71; reuses #38 + #67 + #69) |
| 329 | Clojure keyword keys (`:owner`) → data-decl `exposes` fields; colon prefix is source convention, not semantic | authoring-guide rule | ✅ closed (doc 14 #71) |
| 330 | Clojure namespaces → `.nomtu` modules + `uses @Module`/`uses @Data` references in concept decls | authoring-guide rule | ✅ closed (doc 14 #71) |
| 331 | Clojure `reduce`/`map`/`vals` pipelines → single-sentence declarative `ensures` clauses (reuses #11/#52/#57 pipeline rules) | authoring-guide rule | ✅ closed (doc 14 #71) |
| 332 | Transaction-boundary markers (`dosync`, `BEGIN`/`COMMIT`, `@transactional`) collapse to function-decl boundary when fn carries atomic-group `ensures` | authoring-guide rule | ✅ closed (doc 14 #71) |
| 333 | Perl sigils (`$`/`@`/`%`/`\@`/`\%`) → plain named identifiers; type info carried by parameter annotation or prose | authoring-guide rule | ✅ closed (doc 14 #72) |
| 334 | Implicit context variables (`$_`, `@_`, `$1`…`$9`, awk's `$0`) → explicit named parameters + explicit capture-group names (reuses #59 + #46) | authoring-guide rule | ✅ closed (doc 14 #72) |
| 335 | Perl regex with capture groups → pattern-shape data decl (W39) + `exposes` fields naming each capture; function references captures by field name | authoring-guide rule | ✅ closed (doc 14 #72) |
| 336 | Context-sensitive return behavior (`wantarray`/`scalar`) → distinct function decls for each context variant (reuses #68 MATLAB string-flag rule) | authoring-guide rule | ✅ closed (doc 14 #72) |
| 337 | Perl postfix `or`/`unless`/`if` modifiers → `when X is … otherwise …` prose or `requires`/`ensures` at function-decl level | authoring-guide rule | ✅ closed (doc 14 #72) |
| 338 | Opt-in strictness pragmas (`use strict`, JS `"use strict"`, `--strict` flags) are no-op in Nom — strictness is always-on via W4-A3 | authoring-guide rule | ✅ closed (doc 14 #72) |
| 339 | Perl reference operators (`\`, `->`, `$$ref`, `@$ref`) → plain named parameters; value-vs-reference is build-stage decision | authoring-guide rule | ✅ closed (doc 14 #72) |
| 340 | Dependent-type value parameters → `exposes` fields on data decls + `requires`/`ensures` arithmetic over those fields; build-stage discharge where decidable | authoring-guide rule | ✅ closed (doc 14 #73; existing requires/ensures IS dependent-type-level predication) |
| 341 | GADT indexed data constructors → data decls whose `exposes` include type-level indices as named fields | authoring-guide rule | ✅ closed (doc 14 #73) |
| 342 | Totality annotations (Idris `total`/Agda termination-checker) → `ensures the function is total` clauses; build-stage discharge via termination analysis | authoring-guide rule | ✅ closed (doc 14 #73) |
| 343 | Type-directed pattern-matching exhaustiveness → W40 exhaustiveness-check + `ensures` over each variant (reuses existing wedge) | authoring-guide rule | ✅ closed (doc 14 #73) |
| 344 | Custom infix operators (Idris `(::)`) → named functions or named record-construction; no user-defined infix at Nom source | authoring-guide rule | ✅ closed (doc 14 #73) |
| 345 | Idris `?hole` elaboration → Nom build-stage typed-slot resolver (Phase 9) + MECE validator | authoring-guide rule | ✅ closed (doc 14 #73) |
| 346 | Peano-naturals encoding (Idris/Agda/Coq `Nat = Z | S Nat`) → plain `natural from 0 to N` range-typed primitives; build stage selects representation | authoring-guide rule | ✅ closed (doc 14 #73) |
| 347 | `totality` QualityName registration | authoring-corpus seed | ⏳ queued (doc 14 #73; **accumulates to 10 seeds — QualityName-registration formalization threshold reached**) |
| 348 | Temporal-implication operators (SVA `\|->`/`\|=>`/`##N`) → `checks for every reachable cycle t, if A-at-t then B-at-(t+N)` prose; reuses W41 + W49 quantifier vocab | authoring-guide rule | ✅ closed (doc 14 #74) |
| 349 | SVA clocking events (`@(posedge clk)`) → W48 clock-domain clause + `exposes clock_edge_kind` on AssertionContext data decl | authoring-guide rule | ✅ closed (doc 14 #74) |
| 350 | SVA `disable iff (COND)` → `ensures an assertion with COND produces no diagnostic` contract clause on enforcing function | authoring-guide rule | ✅ closed (doc 14 #74) |
| 351 | SVA `assert property` → Nom `property` decl; SVA `cover property` → peer coverage-histogram function | authoring-guide rule | ✅ closed (doc 14 #74) |
| 352 | SVA assertion labels collapse to Nom property decl name; no separate label syntax | authoring-guide rule | ✅ closed (doc 14 #74) |
| 353 | SVA severity (`$info`/`$warning`/`$error`/`$fatal`) → `hazard` clause + `favor auditability`/`favor correctness`; build stage maps to runtime action | authoring-guide rule | ✅ closed (doc 14 #74) |
| 354 | SVA formal-tool choice (JasperGold/VC Formal/Symbiyosys) → build-stage specialization (reuses #47 + #50 tool-choice rule) | authoring-guide rule | ✅ closed (doc 14 #74) |
| 355 | APL glyph primitives → English prose names; no non-ASCII symbolic operators at Nom source (reinforces #66 Julia + MEMORY.md English-vocab) | authoring-guide rule | ✅ closed (doc 14 #75) |
| 356 | APL right-to-left evaluation → named intermediate values (reuses doc 17 §I8 + #52 + #57 pipe rules) | authoring-guide rule | ✅ closed (doc 14 #75) |
| 357 | APL outer-products and reductions → `ensures` clauses quantifying over elements (reuses #35 + #61 + #66 + #68 scientific-computing rules) | authoring-guide rule | ✅ closed (doc 14 #75) |
| 358 | APL tacit/dfn style (implicit `⍵`) → explicit named-parameter function decls (reuses #44 + #46 rules) | authoring-guide rule | ✅ closed (doc 14 #75) |
| 359 | Golf-style terseness → verbose declarative prose via density-inversion principle — now 3 exemplars (Forth + Perl + APL) | authoring-guide rule | ✅ closed (doc 14 #75) |
| 360 | Set-comprehension-heavy code (APL primes, Haskell list-comp, SQL SELECT) → two-sided `ensures` set-equality clauses (reuses #43 + #57 rules) | authoring-guide rule | ✅ closed (doc 14 #75) |
| 361 | Computation-expression blocks (`async {}`/`seq {}`/`task {}`) → function decls with monad-typed return + `ensures … when awaited …` clauses; build stage threads bind for target runtime | authoring-guide rule | ✅ closed (doc 14 #76) |
| 362 | Monadic bind (`let!`/`do!`/`<-`/`for yield`) → named-intermediate prose with explicit sequencing (reuses doc 17 §I8) | authoring-guide rule | ✅ closed (doc 14 #76) |
| 363 | Result-monad short-circuit propagation → explicit `ensures … never runs when earlier step failed` short-circuit clauses | authoring-guide rule | ✅ closed (doc 14 #76; reinforces #67 Zig + #25 Haskell) |
| 364 | F# `match`/`with` on discriminated unions → `when X is Variant1 … when X is Variant2 … otherwise` prose + W40 exhaustiveness | authoring-guide rule | ✅ closed (doc 14 #76) |
| 365 | Runtime interop (`Async.RunSynchronously`, JS top-level `await`, Python `asyncio.run`) is build-stage runtime-entry-point concern | authoring-guide rule | ✅ closed (doc 14 #76) |
| 366 | Custom workflow-builders (F# `XxxBuilder`, Haskell `MonadTrans`) → build-stage lowerings; authoring surface is `ensures` contract only | authoring-guide rule | ✅ closed (doc 14 #76) |
| 367 | Scheme `call/cc` decomposes to 4 named patterns at authoring time: early-return / generator / exception / coroutine; no first-class continuation value at Nom source | authoring-guide rule | ✅ closed (doc 14 #77) |
| 368 | Early-return `call/cc` idiom → `ensures … terminates early at first matching X; no subsequent Y is examined` contract clause | authoring-guide rule | ✅ closed (doc 14 #77) |
| 369 | Generator via saved continuation → stateful function + tagged-variant GeneratorOutcome data decl + `hazard` on concurrent use | authoring-guide rule | ✅ closed (doc 14 #77) |
| 370 | Scheme `set!` on closed variables → stateful-function `ensures state preserved across invocations` + `hazard` on concurrent access | authoring-guide rule | ✅ closed (doc 14 #77) |
| 371 | Tail-call optimization is build-stage responsibility; author-time intent expressible via `ensures constant stack usage regardless of input size` | authoring-guide rule | ✅ closed (doc 14 #77) |
| 372 | `dynamic-wind` / try-finally semantics → `ensures cleanup ran under all paths` contract + cleanup in peer function | authoring-guide rule | ✅ closed (doc 14 #77) |
| 373 | MongoDB pipeline stages (`$match`/`$lookup`/`$unwind`/`$group`/`$sort`/`$limit`) → collapse to function-level `ensures` clauses; build-stage optimizer chooses stage order | authoring-guide rule | ✅ closed (doc 14 #78) |
| 374 | Document-store joins (MongoDB `$lookup`, CouchDB views, Elasticsearch parent/child) → prose join conditions inside `ensures` quantifiers (reuses #43 SQL) | authoring-guide rule | ✅ closed (doc 14 #78) |
| 375 | Array-flattening (`$unwind`, Postgres `jsonb_array_elements`, BigQuery `UNNEST`) → implicit in `ensures` quantification over nested elements | authoring-guide rule | ✅ closed (doc 14 #78) |
| 376 | Group-aggregation accumulators (`$sum`/`$avg`/`$max`/`$count`/`$first`/`$last`) → `exposes` fields on result data decl; semantics in `ensures` clause | authoring-guide rule | ✅ closed (doc 14 #78) |
| 377 | MongoDB `$field` references → plain prose field access; no `$` prefix at Nom source (reuses #72 Perl sigils principle) | authoring-guide rule | ✅ closed (doc 14 #78) |
| 378 | Database-index presence hazards → explicit `hazard` clause on any function that joins or sorts by a field; callers own index lifecycle | authoring-guide rule | ✅ closed (doc 14 #78) |
| 379 | Typed-timestamp literals across databases (ISODate/TIMESTAMP/datetime) → `timestamp` field type + prose comparisons (at-least/at-most/between) | authoring-guide rule | ✅ closed (doc 14 #78) |

Totals by destination (after doc 14 #78 MongoDB aggregation pipeline translation — **thirty-seventh 0-new-wedge translation in a row**; 7 authoring-guide closures; **data-store paradigm family now has 6 exemplars unified**: SQL + GraphQL + jq + Protobuf + Redis + MongoDB):

- ⏳ Wedge queued: **44** (unchanged)
- 🧪 Smoke-test todo: **1**
- 📘 Authoring-guide doc-todo: **0**
- ✅ Closed: **329**
- 🧠 Design deferred (open): **0**
- 🔒 Blocked: **2**
- 🌱 Authoring-corpus seed: **10** (unchanged)

Backlog size: 399 rows. Closure rate 82% (329/399). **78 translations** in doc 14. Forty-fifth consecutive minimal-wedge, **thirty-seventh 0-new-wedge**. **Data-store family now has 6 exemplars unified.**

- ⏳ Wedge queued: **44** (+W51 QualityName-registration formalization wedge; see index below)
- 🧪 Smoke-test todo: **1**
- 📘 Authoring-guide doc-todo: **0**
- ✅ Closed: **304**
- 🧠 Design deferred (open): **0**
- 🔒 Blocked: **2**
- 🌱 Authoring-corpus seed: **10** (forward_compatibility + numerical_stability + gas_efficiency + synthesizability + minimum_cost + statistical_rigor + availability + auditability + accessibility + totality QualityNames) — **10/10 threshold reached**

Backlog size: 374 rows. Closure rate 81% (304/374). **74 translations** in doc 14. Thirty-third consecutive 0-new-wedge. **Property-based-verification paradigm family fully covered across 5 exemplars** (TLA+ + Dafny + Idris + PDDL + SVA). Paradigm coverage: imperative + OOP + async + concurrency + pure-functional + ADT + data + shell + build + container + editor-event + CI/CD + math-as-language + actor-model + logic-programming + metaprogramming + schema-IDL + pattern-DSL + state-machine-DSL + property-based-testing + infrastructure-as-code + array-programming + workflow-orchestration + stream-processing + smart-contract + declarative-reactive-UI + BDD-scenario + hardware-description-RTL + purely-functional-package-spec + recursive-relational-query + stack-based-concatenative + parameterized-modules + policy-DSL + temporal-logic-model-checking + AI-planning + visualization-as-code + verified-imperative-programming + portable-binary-target + statistical-computing + HTTP-API-spec + container-orchestration + pure-FRP + token-tree-macros + JSON-transformation-DSL + business-data-processing + object-pipeline-shell + idempotent-automation + scientific-computing + push-subscription + key-value-store + bidirectional-streaming-RPC + typeset-document + multiple-dispatch-scientific + comptime-error-union-systems + matrix-oriented-scientific + pure-message-passing-OO + safety-critical-strong-typing-tasking + STM-immutable-data + **text-processing-regex-sigils (Perl)**. Thirty-ninth consecutive minimal-wedge translation, **thirty-first 0-new-wedge in a row**.

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
- **W51:** QualityName-registration formalization wedge — **UNBLOCKED 2026-04-14** by reaching the 10-seed threshold via Idris 2 translation (#73, `totality` seed). Formalizes the QualityName registry surface (doc 08 §8 + MEMORY.md roadmap item 8): registered axes with metric functions, cardinality constraints (`exactly_one_per_app` etc.), MECE-validator participation. Seeds to bake: forward_compatibility / numerical_stability / gas_efficiency / synthesizability / minimum_cost / statistical_rigor / availability / auditability / accessibility / totality. Ships the `nom corpus register-axis` CLI per MEMORY.md roadmap item 8 + extends M7a required-axes registry (nom-dict commit `bcadcb3`).

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
