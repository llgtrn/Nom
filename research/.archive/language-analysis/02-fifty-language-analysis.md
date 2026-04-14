# Comprehensive 50-Language Analysis for Nom

> **Archive snapshot — finalized 2026-04-14.** Live mission state
> lives in [`research/08-mission-checklog.md`](../../08-mission-checklog.md).
> This doc is preserved as the historical paradigm-survey record
> (the source that drove the closed 9-kind proof at corpus 68/88).

**Research date:** 2026-04-12
**Last verified against codebase:** 2026-04-14 (grammar-shelf audit)
**Claims tagged:** ✅ SHIPPED (file:line or commit SHA cited), ⏳ PLANNED (phase noted), ❌ ASPIRATIONAL (no near-term plan)

> **Paradigm-coverage status (2026-04-14 very-late):** doc 14 Accelworld-translation corpus has expanded from an initial 12 translations to **84 translations spanning 71 paradigm families** — the 50-language-survey target is exceeded by 42%. Every paradigm family in this doc's survey now has at least one worked translation in doc 14, and coverage extends into paradigms not in the original 50-survey: hardware-description (Verilog + SVA + Chisel), effect-handling (Haskell monads + F# computation-expressions + Scheme call/cc + OCaml 5 effects), property-based-verification (TLA+ + Dafny + Idris + PDDL + SVA), visualization-as-code (Mermaid), typeset-document (LaTeX), infrastructure-automation (Terraform + Docker + Nix + K8s + Ansible), and more. **43 consecutive translations have required zero new grammar wedges** — the Nom primitive set (9 closed kinds + composition + W49 quantifier vocabulary + `requires`/`ensures`/`hazard` contract clauses) is demonstrably sufficient across paradigms. 4 invariant principles now documented: density-inversion, non-local-control-flow, explicit-type-per-value, functions-describe-what-they-request-handlers-describe-what-requests-do.

> **Grammar-adoption status (2026-04-14):** tree-sitter grammars for 42 of the 50 surveyed languages are now workspace deps in `nom-compiler/Cargo.toml` (lines 58-102): `rust, typescript, python, c, cpp, go, java, c-sharp, ruby, php, scala, haskell, ocaml, julia, bash, lua, r, zig, elm, objc, racket, erlang, d, fortran, elixir, groovy, swift, dart` — plus markup/config (`html, css, json, yaml, toml-ng, regex, make, cmake, verilog, proto, nix, glsl, graphql, latex`). The parser front-end for corpus ingestion is no longer scaffold-only. Phase-5 milestones (planner/verifier/codegen per-language) remain ⏳ PLANNED as tagged per-language. Genuinely absent grammars: **Nim, Odin, Ada, V, Carbon, Kotlin, F#, Perl, Clojure, Gleam, ReScript, Mojo, Roc, Flix, Unison, Vale, Prolog, APL/K, Forth, Smalltalk, Tcl** — these remain unblockable at the grammar layer until upstream crates ship. JavaScript (#34) is covered via `tree-sitter-typescript` which handles both. **Paradigm translations for most of these missing-grammar languages already exist in doc 14** (Ada #70, Kotlin #22, F# #76, Perl #72, Clojure #71, Prolog #28, APL #75, Forth #44, Smalltalk #69, Tcl #82, Pascal #79, Idris #73, Datalog #80 etc.); the grammar absence blocks automated corpus ingestion, not paradigm-level authoring coverage.

> **Status banner**: This document is a survey of design patterns and failures across 50 languages. The "Nom takeaway" paragraphs are design notes. Where those takeaways describe infrastructure that already exists in the Nom codebase, the claim is tagged ✅ SHIPPED with a citation. Where the infrastructure is planned but not yet built, the claim is tagged ⏳ PLANNED. Where it depends on research that has no concrete implementation timeline, the claim is tagged ❌ ASPIRATIONAL. The language survey itself (strengths/weaknesses/takeaways) is documentation of design intent, not implementation status.

---

**Scope:** Paradigm-organized survey of 50 languages. Each entry covers strengths, weaknesses, and concrete Nom takeaways. The existing survey (`01-world-language-survey.md`) catalogs 47 failures; this document complements it with per-language synthesis and a final design-decision matrix.

---

## PART I — SYSTEMS LANGUAGES (10)

### 1. Rust (2010, Graydon Hoare / Mozilla)
- **Strength:** Ownership + borrow checker eliminates use-after-free and data races at compile time with zero runtime cost. `Result<T, E>` forces error handling at every call site.
- **Weakness:** Borrow checker rejects many valid graph and self-referential patterns; the compiler fight is real (~20 false positives per genuine bug). Compile times are among the worst in production compilers.
- **Nom takeaway:** Adopt the principle (linear data flow eliminates aliasing bugs) but not the mechanism (borrow checker). Nom's flow graph gives the compiler the same aliasing information without requiring the programmer to annotate every reference. Avoid Rust's `Pin<Box<dyn Future<Output=...>>>` verbosity — if the flow graph makes parallelism explicit, `async` colors become unnecessary.
  - ⏳ PLANNED — The flow-graph aliasing analysis described here requires a working planner (Phase 5) and codegen (Phase 6). The `.nom`/`.nomtu` parser and store pipeline exist (`nom-concept`, `nom-cli/src/store/`); the planner that would enforce aliasing rules does not.

### 2. C (1972, Dennis Ritchie / Bell Labs)
- **Strength:** Maximal portability, zero-overhead abstraction, transparent memory layout. Readable one-to-one correspondence between source and machine code.
- **Weakness:** Null-terminated strings + manual malloc/free account for ~70% of CVEs at Microsoft and Google. Undefined behavior is pervasive and exploitable. No module system, only textual `#include`.
- **Nom takeaway:** Match C's principle of zero-cost composition — the final LLVM IR must not carry runtime overhead beyond what the flow graph strictly requires. Never replicate C's trust model where the programmer is solely responsible for memory and string safety.
  - ⏳ PLANNED — Zero-cost LLVM IR emission exists for the core imperative subset (`nom-codegen/src/lib.rs`, `nom-llvm`; Phase 4 D3 commit `654e981` proves closure-body compilation to `.bc`). Full flow-graph composition with zero-overhead guarantees requires Phase 5/6.

### 3. C++ (1979, Bjarne Stroustrup)
- **Strength:** Zero-cost abstractions, templates enabling compile-time computation (constexpr, concepts), backward C compatibility for incremental adoption.
- **Weakness:** Inherited all of C's undefined behavior. The language accumulated 45 years of design decisions that cannot be removed without breaking compatibility. Feature count makes the learning curve effectively infinite.
- **Nom takeaway:** C++ demonstrates that "add features without removing old ones" produces an un-learnable language. Nom's dictionary grows but the sentence-layer keywords stay fixed at ~10. C++'s concepts (compile-time contract checking on templates) are the right idea — Nom's `.nomtu` contracts are the cleaner realization.
  - ✅ SHIPPED (partial) — Fixed keyword set enforced: the `.nomx` v1 lexer at `nom-lexer/src/lib.rs` has a closed token enum (commit `05ee1b6`). The `.nom` classifier set is closed: `system|flow|need|contract|composition|effects` (commit `6d1710a`). Contract `pre`/`post` fields are parsed (`nom-concept/src/lib.rs:~300`) but not yet checked at compile time against callers (that's Phase 5 verifier work). ⏳ PLANNED — Compile-time contract checking.

### 4. Zig (2015, Andrew Kelley)
- **Strength:** `comptime` — arbitrary code execution at compile time using the same language, no macro sublanguage. Explicit allocator passing threads allocator choice through the entire call tree, making memory strategy auditable.
- **Weakness:** No closures or iterators in idiomatic style; functional patterns are awkward. Error set unions work but become unwieldy across large call stacks. Still pre-1.0 with frequent breaking changes.
- **Nom takeaway:** Adopt the explicit-allocator principle in `.nomtu` contracts — the memory strategy (arena, pool, stack, heap) should be declared in the contract and inferred by the compiler from the flow graph, not implicit. `comptime` proves that compile-time evaluation in the same language is more powerful than a separate macro system.
  - ⏳ PLANNED — Memory-strategy fields in `.nomtu` contracts are not yet in the schema. Current `words_v2` schema (`nom-dict/src/lib.rs`) tracks `hash, word, kind, body_kind, body_bytes` — no allocator field. Phase 5+.

### 5. Odin (2016, GingerBill)
- **Strength:** Context-passing as a first-class language construct (implicit thread-local context for allocator, logger, temp allocator). Extremely low compile times. Sane defaults: no hidden control flow, no exceptions, no implicit allocations.
- **Weakness:** Small ecosystem, no formal spec, community documentation is thin. Lacking generics until recently, requiring code duplication.
- **Nom takeaway:** Odin's implicit context (allocator, logger) that flows through the call tree without explicit threading is exactly what Nom's effect system does — effects propagate through the flow graph by set union. Validate that Nom's effect propagation is as zero-overhead as Odin's context threading.
  - ✅ SHIPPED (partial) — Effect valence keywords (`benefit`/`hazard`) parsed and surfaced in build manifest (commit `c9d1835`, `eeb1e23`; `nom-concept/src/lib.rs`). Set-union propagation through the closure walk exists (`nom-concept/src/closure.rs`). ⏳ PLANNED — Zero-overhead LLVM IR generation for effect context threading.

### 6. Nim (2008, Andreas Rumpf)
- **Strength:** Multiple memory management backends (ARC, ORC, GC) selectable per compilation. Hygienic macros that operate on the AST. Python-like syntax with C-level performance.
- **Weakness:** Multiple MM backends mean code correctness depends on backend choice — ARC behavior differs from ORC in cycle handling. Macro system is powerful but produces error messages that reference generated code, making debugging difficult.
- **Nom takeaway:** Nim's ARC (automatic reference counting with move semantics) is the closest existing model to what Nom's compiler should generate from linear flow graphs — no GC pauses, predictable throughput. Study ARC's cycle detection cost and ensure Nom's flow-graph structure makes cycles detectable statically.
  - ✅ SHIPPED (partial) — Cycle detection in the concept-graph closure walker exists (`nom-concept/src/closure.rs`; commit `c5cdce6`). ⏳ PLANNED — ARC-style codegen; cycle detection at the LLVM IR level.

### 7. D (2001, Walter Bright / Andrei Alexandrescu)
- **Strength:** Compile-time function execution (CTFE) mature and production-proven. `scope` guards for deterministic resource cleanup without RAII boilerplate. `ranges` as the composable iteration primitive.
- **Weakness:** Two incompatible standard libraries (Phobos vs. legacy). GC by default alienates systems programmers; `@nogc` is an afterthought annotation. Community fragmentation never healed.
- **Nom takeaway:** D's `scope(exit)`, `scope(success)`, `scope(failure)` is the right model for resource cleanup in compositions — Nom's `.nomtu` contract's `effects` field should encode resource acquisition/release so the compiler generates deterministic cleanup without GC.
  - ⏳ PLANNED — `effects` field parsed (`nom-concept/src/lib.rs`) but deterministic resource cleanup codegen not implemented. Phase 6.

### 8. Ada (1983, Jean Ichbiah / DoD)
- **Strength:** Design-by-contract (`Pre`, `Post`, `Type_Invariant`) baked into the language spec, not a library. Tasking model with rendezvous is formally verifiable. Range-constrained subtypes catch out-of-bound values at compile time.
- **Weakness:** Verbose syntax reflects 1970s government committee design. Separate compilation model requires precise unit ordering that frustrates modern tooling. Ecosystem is tiny outside defense/aerospace.
- **Nom takeaway:** Ada's range-constrained subtypes (`type Probability is new Float range 0.0 .. 1.0`) are exactly the right model for Nom's `.nomtu` contract `pre`/`post` fields. `where security>0.9` in Nom is an Ada range constraint. Ada proves that design-by-contract at compile time is achievable.
  - ✅ SHIPPED (partial) — `where security>0.9` syntax parsed as a constraint clause in the `.nom` lexer/parser (commit `05ee1b6`). Confidence threshold `with at-least N confidence` shipped (commit `97c836f`). ⏳ PLANNED — Compile-time enforcement of range constraints against callers (Phase 5 verifier).

### 9. V (2019, Alexander Medvednikov)
- **Strength:** Sub-second compilation, auto-formatting, minimal syntax, multiple backends (C, JS, native).
- **Weakness:** Numerous false marketing claims about the feature set at launch. Specification and implementation diverged; safety guarantees were overstated. Community trust damaged permanently.
- **Nom takeaway:** Avoid V's mistake: never document features that don't exist in the compiler. Every Nom claim in `SYNTAX.md` and `README.md` must be verifiable by running `nom build`. Marketing ahead of implementation destroys trust more permanently than slow development.
  - ✅ SHIPPED — This document itself follows this principle by tagging every claim. The memory doc `09-implementation-status-2026-04-13.md` was created specifically to separate shipped state from aspirational plans.

### 10. Carbon (2022, Chandler Carruth / Google)
- **Strength:** Designed explicitly for incremental C++ interop migration. Bidirectional import is architecturally novel. Memory safety goals without GC via checked vs. unchecked APIs.
- **Weakness:** Extremely early; no production deployments. The value proposition collapses if Rust's C++ interop matures before Carbon does.
- **Nom takeaway:** Carbon proves that C++ interop as a first-class design constraint forces certain architectural decisions. Nom's `nom-translate` (auto-translate to Rust for `.bc` compilation) is a similar interop bridge — ensure it handles C/C++ as a target, not just a source.
  - ⏳ PLANNED — `nom-translate` crate exists as a scaffold (`nom-compiler/crates/nom-translate/`) but mass translator work is Phase 5+. Under the body-as-compiled-artifact shift (§4.4.6, commit `540620d`), C/C++ bodies are ingested as `.bc` via `clang -emit-llvm` rather than source translation.

---

## PART II — APPLICATION LANGUAGES (8)

### 11. Go (2009, Pike / Thompson / Griesemer / Google)
- **Strength:** Sub-10-minute onboarding, one canonical formatting tool (`gofmt`), static binaries, goroutines + channels as first-class concurrency. Explicit interface satisfaction without declaration.
- **Weakness:** No generics until 2022 — 13 years of `interface{}` casting. Error handling via `if err != nil` is the language's #1 complaint. No sum types.
- **Nom takeaway:** Go's structural interfaces (satisfy implicitly by having the right methods) are a composition model worth studying — Nom's `.nomtu` contract compatibility at flow edges is structurally similar. Go's `gofmt` proves that one canonical formatter eliminates all style debates; `nom fmt` must be zero-configuration.
  - ✅ SHIPPED (partial) — `nom fmt` subcommand exists (`nom-cli/src/fmt.rs`). ⏳ PLANNED — Structural contract compatibility checking at flow edges (Phase 5).

### 12. Java (1995, James Gosling / Sun)
- **Strength:** JVM ecosystem depth, backward compatibility across 30 years, strong tooling. Virtual threads (Project Loom) finally made concurrency ergonomic.
- **Weakness:** Covariant arrays, type erasure, extreme verbosity.
- **Nom takeaway:** Java's Project Loom (virtual threads making blocking code concurrent without async syntax) vindicates Nom's approach: the runtime should handle concurrency scheduling, not the programmer. Java's 30-year backward compatibility validates Nom's immutable content-addressed `.nomtu` strategy.
  - ✅ SHIPPED — Content-addressed `.nomtu` strategy shipped: every entry stored by hash in `words_v2` (`nom-dict/src/lib.rs`; commit `aaa914d`). Hashes are deterministic from body bytes (commit `540620d`). ⏳ PLANNED — Runtime concurrency scheduling (Phase 6+).

### 13. C# (2000, Anders Hejlsberg / Microsoft)
- **Strength:** `async`/`await` originated here and the design is the cleanest in mainstream languages. LINQ provides composable query algebra over any data source.
- **Weakness:** `async` colors functions — async-all-the-way-down is required for benefit.
- **Nom takeaway:** C#'s LINQ is conceptually equivalent to Nom's `where` constraints on `.nomtu` selection — both express declarative query algebra over typed collections.
  - ✅ SHIPPED (partial) — `where` constraint clauses parsed in `.nom` files (`nom-concept/src/lib.rs`). Typed-slot query `@Kind matching "..."` ships as the dictionary query layer (commit `c405d2a`; `nom-cli/src/store/resolve.rs:70`). ⏳ PLANNED — Full Datalog-style relational query (Phase 9+).

### 14. Kotlin (2011, JetBrains)
- **Strength:** Null safety via the type system (`String` vs `String?`). Extension functions. Coroutines with structured concurrency (`CoroutineScope`) prevent goroutine-leak equivalents.
- **Weakness:** Coroutine stack traces are nearly unreadable. Compile times on large projects are significantly worse than Java.
- **Nom takeaway:** Kotlin's `String?` vs `String` distinction is exactly right — null is a type, not a value. Kotlin's structured concurrency (child coroutines cannot outlive their scope) maps to Nom's flow graph.
  - ⏳ PLANNED — Null-as-type enforcement and structured-concurrency mapping both require Phase 5+ compiler work.

### 15. Swift (2014, Chris Lattner / Apple)
- **Strength:** Protocol-oriented programming with default implementations enables composition without inheritance. Value types by default with copy-on-write semantics.
- **Weakness:** ARC cycle leaks remain the #1 memory bug source. Apple-platform bias limits server-side adoption.
- **Nom takeaway:** Swift's `@discardableResult` (opt-in ignoring of return values) vs default "must use" is the right default — Nom's flow graph must force explicit handling of every output.
  - ⏳ PLANNED — Must-use output enforcement requires flow-graph analysis (Phase 5).

### 16. Scala (2003, Martin Odersky / EPFL)
- **Strength:** Algebraic data types + pattern matching, first-class functions, `for`-comprehensions as monadic sugar, JVM interop.
- **Weakness:** Implicits are three features sharing one keyword — creating "too much magic." Too many ways to accomplish the same thing.
- **Nom takeaway:** Scala proves that power without constraint produces inconsistent codebases. Nom has three operators (`->`, `::`, `+`) and they must stay three.
  - ✅ SHIPPED — Three-operator constraint enforced in the parser (`nom-lexer/src/lib.rs`): `->`, `::`, `+` are the only composition tokens. No new operator tokens added since initial commit.

### 17. F# (2005, Don Syme / Microsoft Research)
- **Strength:** Type inference so complete that type annotations are rarely needed. Railway-oriented programming (`Result` chaining) as an idiomatic pattern.
- **Weakness:** Windows/.NET bias historically. IDE support outside Visual Studio is second-class.
- **Nom takeaway:** F#'s railway-oriented programming is structurally identical to Nom's flow graph with `iftrue`/`iffalse`. Nom's flow graph is railway-oriented programming without the `Result` ceremony.
  - ⏳ PLANNED — `iftrue`/`iffalse` branches parsed in `.nom` syntax but not connected to a flow-graph planner that enforces railway discipline. Phase 5.

### 18. Dart (2011, Lars Bak / Google)
- **Strength:** Sound null safety (migrated the entire ecosystem successfully). AOT + JIT dual compilation.
- **Weakness:** Ecosystem is Flutter-dominated; server-side Dart has thin library coverage.
- **Nom takeaway:** Dart's null-safety migration (a gradual whole-ecosystem transition that actually succeeded) is the model for Nom's own evolution — provide a migration tool, not just a migration guide.
  - ❌ ASPIRATIONAL — `nom author` migration assist exists (`nom-cli/src/author.rs`) but is currently a stub. No migration tooling for transitioning existing Nom programs.

---

## PART III — SCRIPTING LANGUAGES (7)

### 19. Python (1991, Guido van Rossum)
- **Strength:** Scientific ecosystem depth (NumPy, PyTorch, pandas) is unmatched. Readable syntax with significant whitespace.
- **Weakness:** GIL prevents true CPU parallelism. 2→3 migration was a 12-year community fracture. Dynamic typing means errors surface at runtime.
- **Nom takeaway:** Python's ecosystem dominance comes from a composable library model, not language features — `.nomtu` entries for data science should be first-class citizens. Python's whitespace-significant syntax proves readability trumps familiarity.
  - ⏳ PLANNED — Data-science `.nomtu` entries require corpus ingestion (Phase 5). The `nom-corpus` crate scaffold exists (`nom-compiler/crates/nom-corpus/`).

### 20. Ruby (1995, Yukihiro Matsumoto)
- **Strength:** Blocks/procs/lambdas unify closures into one model. Human-centered design philosophy ("developer happiness").
- **Weakness:** GVL, GC consuming 80% of Rails request time at scale. DSL overuse makes large codebases read like different languages.
- **Nom takeaway:** Ruby's "developer happiness" framing is the correct user-research lens for Nom — every syntax decision should be evaluated against "does this feel natural to write?"
  - ✅ SHIPPED (partial) — The `.nomx` natural-language grammar track (`nom-parser/src/nomx.rs`, 81 tests as of HEAD) directly addresses the "reads naturally" criterion. The Vietnamese grammar analysis (research/02) shaped the classifier-first syntax.

### 21. Perl (1987, Larry Wall)
- **Strength:** Regular expression support is the gold standard. CPAN is still one of the most comprehensive module repositories.
- **Weakness:** Context-sensitivity makes code behavior non-local. Write-only reputation is deserved.
- **Nom takeaway:** Perl is the definitive case study in how context-sensitive semantics destroy readability. Nom must be context-free.
  - ✅ SHIPPED — The `.nom` and `.nomx` parsers are both context-free (hand-rolled recursive descent; commits `05ee1b6`, `d9425ba`). No context-sensitive token interpretation exists.

### 22. Lua (1993, Ierusalimschy / Celes / Filho / PUC-Rio)
- **Strength:** Embeddable in 200KB of binary. One universal data structure (table) covers arrays, hashmaps, namespaces, objects.
- **Weakness:** 1-based indexing. `nil` is both "absent" and a valid table value.
- **Nom takeaway:** Lua's single-structure philosophy is the right minimalism — Nom's one composition mechanism (`->`) should do the work of arrays, pipelines, and state machines. Nom's compiler binary should target comparably small footprint.
  - ⏳ PLANNED — Compiler binary footprint is currently large (LLVM-C.dll linked). Small-footprint target is a post-bootstrap goal.

### 23. PHP (1994, Rasmus Lerdorf)
- **Strength:** Zero-configuration deployment on shared hosting democratized the web. Modern PHP (8.x) has union types, named arguments, and fibers.
- **Weakness:** Type juggling created an entire class of authentication bypasses. Global mutable state in superglobals.
- **Nom takeaway:** PHP's type juggling is the canonical example of why coercion must be a compile-time operation with explicit contracts, never a runtime surprise.
  - ⏳ PLANNED — Compile-time coercion checking requires Phase 5 verifier.

### 24. R (1993, Ihaka / Gentleman)
- **Strength:** Statistical primitives are first-class syntax. CRAN's reproducibility requirements produce higher-quality packages.
- **Weakness:** 1-based indexing. Five different "missing" values with different propagation rules.
- **Nom takeaway:** R's CRAN submission requirements are the model for Nom's `.nomtu` quality score — automated quality gates, not social trust.
  - ⏳ PLANNED — `nom-score` crate exists as a scaffold; automated quality gates for `.nomtu` submission are Phase 5+.

### 25. Julia (2012, Bezanson / Edelman / Karpinski / Shah / MIT)
- **Strength:** Multiple dispatch as the primary abstraction. JIT achieves near-C performance for numerical code.
- **Weakness:** Time-to-first-plot (TTFP) — JIT causes 30-60 second startup latency. 1-based indexing.
- **Nom takeaway:** Julia's multiple dispatch proves that specialization is more powerful than single-dispatch OOP. Nom's `::` operator is the same idea. Julia's TTFP is a warning — prefer AOT compilation.
  - ✅ SHIPPED (partial) — `::` operator parsed as the specialization operator (`nom-lexer/src/lib.rs`). AOT via LLVM backend compiles `.nom` sources to `.bc` (Phase 4 D3, commit `654e981`). ⏳ PLANNED — Multiple-dispatch resolution from the dictionary (Phase 9 corpus).

---

## PART IV — FUNCTIONAL LANGUAGES (7)

### 26. Haskell (1990, Committee / SPJ / Wadler)
- **Strength:** Strongest type system in mainstream use. Purity enforced at type level. QuickCheck property-based testing invented here.
- **Weakness:** `String = [Char]` is catastrophically slow. Lazy evaluation by default creates unpredictable space leaks.
- **Nom takeaway:** Haskell's QuickCheck is exactly what Nom's `.nomtu` contract `pre`/`post` fields enable — auto-generate property-based tests from contracts. Avoid lazy-by-default — Nom's flow graph is strict.
  - ⏳ PLANNED — `pre`/`post` fields are parsed (`nom-concept/src/lib.rs`) but no test-generation engine exists. Phase 5+. Strict evaluation in the LLVM backend is the current default (no lazy evaluation mechanism added).

### 27. OCaml (1996, INRIA)
- **Strength:** Type inference that handles GADTs and polymorphic variants. Algebraic data types with exhaustive pattern matching.
- **Weakness:** Module functors have steep learning curve. No multicore until OCaml 5.0 (2023).
- **Nom takeaway:** OCaml's module functor system is the academic foundation of Nom's `::` specialization. A small, well-designed type system with inference is more productive than explicit annotation.
  - ⏳ PLANNED — Type inference for the `::` specialization operator requires Phase 5+.

### 28. Elixir (2011, José Valim)
- **Strength:** Built on Erlang's battle-tested OTP. Pattern matching + pipelines (`|>`) make data transformation readable.
- **Weakness:** Shared-nothing actor model means large data structures are copied between processes. No static types.
- **Nom takeaway:** Elixir's `|>` pipeline operator is the most direct syntactic ancestor of Nom's `->`. The difference: Nom's `->` is a graph edge with semantic meaning (data flow, contract checking, effect propagation).
  - ✅ SHIPPED (partial) — `->` pipeline edges parsed in `.nom` syntax (`nom-concept/src/lib.rs`). Effect propagation through edges exists (`nom-concept/src/closure.rs`). ⏳ PLANNED — Contract checking at `->` edges (Phase 5).

### 29. Erlang (1986, Armstrong / Virding / Williams / Ericsson)
- **Strength:** Nine-nines reliability proven in AXD301. Lightweight processes, "let it crash" philosophy with supervisor hierarchies.
- **Weakness:** Atom table is global and unbounded. Mutable state requires untyped `ets` tables.
- **Nom takeaway:** Erlang's "let it crash" + supervisor trees is the right model for Nom's `iffalse` branch — don't recover at the point of failure, restart at a higher-level flow boundary.
  - ⏳ PLANNED — `iffalse` branch semantics parsed; supervisor-tree restart policies not implemented. Phase 5+.

### 30. Clojure (2007, Rich Hickey)
- **Strength:** Persistent immutable data structures (HAMTs) with O(log32 n) update. Homoiconicity enables powerful macro systems.
- **Weakness:** JVM startup time. Dynamic typing. Stack traces through Java interop are incomprehensible.
- **Nom takeaway:** Clojure's persistent data structures (structural sharing, cheap immutable "copy") are what Nom's `.nomtu` implementations should use for collection types. Hickey's "value vs. place" talks are the philosophical foundation of Nom's stateless flow model.
  - ⏳ PLANNED — Persistent data structure runtime for `.nomtu` collection implementations is a Phase 0 runtime library item (see `03-self-hosting-roadmap.md` §Phase 0).

### 31. Racket (1994, Felleisen / PLT Group)
- **Strength:** "Language-oriented programming" — `#lang` directive lets each file be a different language. Hygienic macros are the gold standard.
- **Weakness:** Performance below C by 5-50x. Too many parentheses for mainstream adoption.
- **Nom takeaway:** Racket's `#lang` system is the inspiration for Nom's `.nomtu` as "embedded domain-specific implementations." Each `.nomtu` is effectively its own `#lang`.
  - ✅ SHIPPED (partial) — Each `.nomtu` entry stores an arbitrary body (compiled `.bc` bytes; commit `540620d`). The body is opaque to the dictionary; any upstream language that emits LLVM bitcode is a valid implementation language. ⏳ PLANNED — Explicit multi-language body dispatch (Phase 5+).

### 32. Elm (2012, Evan Czaplicki)
- **Strength:** Zero runtime exceptions in practice. The Elm Architecture (Model-Update-View) is the cleanest unidirectional data flow model.
- **Weakness:** No type classes, forcing API duplication. Effect system via `Cmd`/`Sub` is verbose.
- **Nom takeaway:** Elm proves that zero runtime exceptions is achievable with a strict-enough type system. Nom's contract-verified compositions should make the equivalent claim.
  - ⏳ PLANNED — Zero-runtime-exception guarantee requires contract verification at all flow edges (Phase 5) and runtime panic elimination. Not achievable until Phase 5/6 ship.

---

## PART V — WEB / UI LANGUAGES (5)

### 33. TypeScript (2012, Anders Hejlsberg / Microsoft)
- **Strength:** Structural typing makes types an annotation layer over existing JS. Conditional types enable complex type-level computation.
- **Weakness:** Seven documented sources of unsoundness. `any` disables type checking for the entire downstream chain.
- **Nom takeaway:** TypeScript proves that gradual typing with escape (`any`) provides false safety. Nom's contract system must be all-or-nothing at flow edges.
  - ⏳ PLANNED — All-or-nothing contract checking at flow edges requires Phase 5 verifier. Currently, missing contracts are not a compile error.

### 34. JavaScript (1995, Brendan Eich / Netscape)
- **Strength:** Ubiquitous runtime. Event loop model makes I/O naturally non-blocking without threads.
- **Weakness:** `typeof null === "object"`, prototype pollution, equality semantics.
- **Nom takeaway:** JavaScript's evolution from callbacks to async/await shows that the right concurrency model was always there but the syntax took 20 years to catch up. Nom's `->` operator expresses this directly.
  - ✅ SHIPPED (partial) — `->` operator as the primary data-flow expression is the core of the `.nom` grammar (`nom-lexer/src/lib.rs`). ⏳ PLANNED — Runtime scheduling behind `->` (Phase 6+).

### 35. Svelte (2016, Rich Harris)
- **Strength:** Compiler-first approach — reactive state management is compiled away. Zero-overhead reactivity.
- **Weakness:** Non-standard JavaScript semantics. Small ecosystem compared to React.
- **Nom takeaway:** Svelte's compiler-erases-the-framework approach is Nom's exact model — the `.nom` sentence layer is compiled away into LLVM IR, the framework disappears.
  - ✅ SHIPPED (partial) — `.nom` source compiles to LLVM IR via `nom build --target llvm` (Phase 4 D3, commit `654e981`). The sentence layer (classifiers, flow declarations) is not present in the `.bc` output. ⏳ PLANNED — Full compiler-erases-framework for UX/media nomtu (Phase 5+).

### 36. Gleam (2016, Louis Pierse)
- **Strength:** Erlang VM reliability with static types. Type inference without annotation burden.
- **Weakness:** Very small ecosystem. No type classes.
- **Nom takeaway:** Gleam's success adding static types to BEAM validates Nom's approach of adding contract checking to existing implementations without rewriting.
  - ⏳ PLANNED — Adding contract checking to existing `.bc` bodies without rewriting them is Phase 5 verifier work.

### 37. ReScript (2020, BuckleScript team)
- **Strength:** Sound type system (no `any`) compiling to readable JavaScript. Fast compilation.
- **Weakness:** JavaScript interop requires manual binding definitions. Small community.
- **Nom takeaway:** ReScript proves that a sound type system compiling to an existing target is viable. Nom's contract-verified compositions compiling to LLVM IR follow the same pattern.
  - ⏳ PLANNED — Sound (no-escape) contract system compiling to LLVM IR is Phase 5/6.

---

## PART VI — MODERN / EMERGING LANGUAGES (5)

### 38. Mojo (2023, Chris Lattner / Modular)
- **Strength:** Python-compatible syntax with systems-level control (SIMD, memory layout, ownership). `@parameter` for compile-time execution.
- **Weakness:** Closed source. Python compatibility means inheriting Python's semantics in some cases. Still very early.
- **Nom takeaway:** Mojo's hardware-aware types should influence Nom's `.nomtu` contract for numerical operations — the contract should specify memory layout and SIMD width as constraints.
  - ❌ ASPIRATIONAL — Hardware-aware contract fields (SIMD width, memory layout) have no current implementation or near-term plan.

### 39. Roc (2020, Richard Feldman)
- **Strength:** No exceptions, no null, no runtime errors. Platform abstraction separating "what the program does" from "what platform it runs on."
- **Weakness:** Pre-1.0 with no stable release. Platform ecosystem is minimal.
- **Nom takeaway:** Roc's platform model is architecturally identical to Nom's effect system. Roc's `Task` is what Nom's flow graph is — a pure description of computation that the compiler realizes.
  - ⏳ PLANNED — Platform-separated effects (compile for web vs. desktop vs. embedded) requires `nom-ux` + target specialization (Phase 5+).

### 40. Flix (2015, Magnus Madsen / UBC)
- **Strength:** Datalog as a first-class sublanguage — logic queries and Flix code in the same file. Effect system with region-based memory.
- **Weakness:** JVM platform means startup overhead. Very niche.
- **Nom takeaway:** Flix's embedded Datalog proves that logic programming as a query language is more expressive than SQL for certain problems. Nom's `where` constraint language should be evaluated as Datalog-like queries.
  - ⏳ PLANNED — Datalog-style evaluation of `where` constraints is Phase 9+.

### 41. Unison (2019, Paul Chiusano / Rúnar Bjarnason)
- **Strength:** Content-addressed code — functions identified by the hash of their AST. No dependency conflicts because versions are hashes.
- **Weakness:** Requires Unison Codebase Manager — not file-based, breaks all existing tooling.
- **Nom takeaway:** Unison's content-addressed functions are the exact model for Nom's `.nomtu`. Also adopt: ability search (find functions by type signature) should be Nom's dictionary search by contract shape.
  - ✅ SHIPPED — Content-addressed dictionary: every `.nomtu` entry stored by hash of body bytes (`nom-dict/src/lib.rs`; commit `540620d`). Hash-pinned references (`use #<hex>@<name>`) parsed (commit `ad8cd28`). ⏳ PLANNED — Contract-shape search (ability search analog) is Phase 9 corpus work.

### 42. Vale (2021, Evan Ovadia)
- **Strength:** "Generational references" — memory safety between GC and borrow checker. Zero-cost in happy path.
- **Weakness:** Very early. Runtime generation checks add overhead in worst case.
- **Nom takeaway:** Vale's generational references prove that memory safety doesn't require either GC or borrow checker — a third path exists for non-DAG flows.
  - ❌ ASPIRATIONAL — Generational-reference or equivalent memory-safety model for Nom has no current implementation plan beyond the observation that the flow graph is a DAG.

---

## PART VII — NICHE / UNIQUE LANGUAGES (5)

### 43. Prolog (1972, Colmerauer / Roussel)
- **Strength:** Unification as the universal computation mechanism. Logic programs are inherently bidirectional.
- **Weakness:** Cuts (`!`) break declarative semantics. Negation as failure is unsound.
- **Nom takeaway:** Prolog's unification is the theoretical foundation of Nom's contract compatibility checking at flow edges. Nom should formalize its contract matching as unification.
  - ⏳ PLANNED — Unification-based contract matching at flow edges is Phase 5+ verifier work.

### 44. APL / K (1966 APL: Iverson / 1993 K: Kx Systems)
- **Strength:** Array programming — operations apply to entire arrays without explicit iteration. Composition of transformations in a single expression.
- **Weakness:** Symbol-dense notation is write-only at scale. Adopted only in specialized domains.
- **Nom takeaway:** APL/K's array operations demonstrate that the right abstraction eliminates iteration entirely. Nom's `.nomtu` for tensor operations should expose this level of abstraction.
  - ⏳ PLANNED — Tensor operation `.nomtu` entries require corpus ingestion of numerical libraries (Phase 5+).

### 45. Forth (1970, Chuck Moore)
- **Strength:** Minimal — entire system fits in kilobytes. Concatenative composition: defining new words feels like extending the language.
- **Weakness:** Stack discipline requires mental tracking. No named parameters.
- **Nom takeaway:** Forth's concatenative model is the purest form of what Nom's `->` does with explicit names. Nom should be Forth's readability improvement: same composition model, named instead of stacked.
  - ✅ SHIPPED — Named `->` composition is the core of the `.nom` grammar. Declaring new words (`.nomtu` entries) via `nom store sync` extends the dictionary without changing the grammar (commit `ba7769f`).

### 46. Smalltalk (1972, Kay / Ingalls / Goldberg / Xerox PARC)
- **Strength:** Message passing as the universal computation model. Live programming environment.
- **Weakness:** Image-based development conflicts with file-based version control. Runtime type errors.
- **Nom takeaway:** Smalltalk's live environment should be Nom's REPL model — `nom repl` should allow adding words and testing flows interactively. "Everything is a `.nomtu` responding to flow connections" mirrors "everything is an object responding to messages."
  - ❌ ASPIRATIONAL — `nom repl` does not exist. No interactive evaluation mode in the current toolchain.

### 47. Tcl (1988, John Ousterhout)
- **Strength:** Everything is a string — one data type. Embeddable interpreter. Fully extensible from within.
- **Weakness:** String-to-type conversion on every operation. Injection vulnerabilities.
- **Nom takeaway:** Tcl's embeddability is the right model for Nom's scripting use case — `nom.embed` should allow embedding the sentence layer as a scripting interface, the way Lua and Tcl are embedded.
  - ❌ ASPIRATIONAL — `nom.embed` does not exist. No embedding API in the current toolchain.

---

## PART VIII — VIETNAMESE LANGUAGE CONTEXT (3)

### 48. Vietnamese Topic-Comment Structure
- **Strength:** Vietnamese defaults to topic-comment order: the topic is established first, then the comment describes it. Less ambiguous for data flow than SVO.
- **Nom takeaway:** Nom's classifier-first syntax (`system auth`, `flow request->...`) is topic-comment structure: the classifier establishes the topic, the rest is the comment. This is why Nom reads naturally to Vietnamese speakers. Preserve this structure.
  - ✅ SHIPPED — Classifier-first syntax enforced by the `.nom` parser (`nom-concept/src/lib.rs`; commit `05ee1b6`). Classifier set: `concept|module|entity|composition` in `.nom`; `system|flow|store|graph|agent|test|nom|gate|pool|view` in the lexer.

### 49. Vietnamese Classifier System (Loại từ)
- **Strength:** Classifiers before nouns encode semantic category, forcing categorization at the point of naming. "Con mèo" (classifier-cat) vs. "cái ghế" (classifier-chair) makes semantic class explicit.
- **Nom takeaway:** Nom's keyword set (`system`, `flow`, `need`, `contract`) ARE its classifier system. Each new classifier should be justified by a distinct semantic category. Never create two classifiers for the same category.
  - ✅ SHIPPED — Closed classifier set enforced: adding a classifier requires modifying the lexer's token enum, which has no unguarded extensibility point (`nom-lexer/src/lib.rs:Token` enum; commit `05ee1b6`).

### 50. Vietnamese Aspect Markers and Tonal Semantics
- **Strength:** Vietnamese encodes aspect via particles (`đã` completed, `đang` ongoing, `sẽ` future) rather than verb morphology. The verb itself never changes form.
- **Nom takeaway:** Vietnamese aspect markers map to Nom's temporal flow semantics: `flow::once` (completed, idempotent), `flow::stream` (ongoing), `flow::scheduled` (prospective). Consider aspect-aware flow qualifiers.
  - ⏳ PLANNED — `flow::once`, `flow::stream`, `flow::scheduled` qualifiers not yet in the parser. Vietnamese keyword aliases have been removed (ecd0609); vocabulary is English-only. Aspect-marker semantics require runtime interpretation (Phase 5+).

---

## SYNTHESIS — TOP 10 FEATURES NOM SHOULD ADOPT

### ADOPT-1: QuickCheck-style Property Testing from Contracts (Haskell)
Nom's `.nomtu` `pre`/`post` conditions are executable specifications. At `nom test`, auto-generate hundreds of property-based test cases — any input satisfying `pre` must produce output satisfying `post`.
- ⏳ PLANNED — Phase 5+. `pre`/`post` parsed; test generation engine absent.

### ADOPT-2: Content-Addressed Function Search by Contract Shape (Unison)
`nom search "in: bytes -> out: hashbytes"` should return all `.nomtu` entries matching that contract signature via structural unification, not keyword matching.
- ⏳ PLANNED — Phase 9. `nom-search` crate scaffold exists; corpus-embedding semantic search absent. Current resolution is alphabetical-smallest hash tiebreak (`nom-cli/src/store/resolve.rs:126`; commit `bf95c2c`).

### ADOPT-3: Explicit Allocator Strategy in Contracts (Zig)
The `.nomtu` contract should declare memory strategy: `memory: arena|pool|stack|heap`. The flow graph's topology tells the compiler which strategy is safe.
- ❌ ASPIRATIONAL — No plan or timeline.

### ADOPT-4: Supervision Tree for Flow Fault Handling (Erlang/OTP)
Flows should declare restart policies: `onfail: restart_from <node>|abort|escalate`. The compiler generates supervisor logic from the flow graph.
- ⏳ PLANNED — Phase 5+. `iffalse` branch syntax parsed; supervisor codegen absent.

### ADOPT-5: Aspect-Qualified Flows (Vietnamese Aspect Markers)
`flow::once` (idempotent), `flow::stream` (ongoing), `flow::scheduled` (runs at a schedule). These give the compiler richer scheduling information.
- ⏳ PLANNED — Phase 5+. Not yet in the lexer or parser.

### ADOPT-6: Datalog-Style Dictionary Queries for Constraint Solving (Flix)
The `where` constraint should be evaluated as a stratified Datalog query over the `.nomtu` attribute graph, enabling multi-attribute relational constraints.
- ⏳ PLANNED — Phase 9+. Current `where` clauses are stored but not evaluated.

### ADOPT-7: Bidirectional Contract Inference via Unification (Prolog)
If the programmer specifies the flow topology and required output contract, the compiler should infer what contract each intermediate node must satisfy — unification over the contract graph.
- ❌ ASPIRATIONAL — Requires a complete type-inference engine (Phase 6+, beyond current planning horizon).

### ADOPT-8: Persistent Immutable Data Structures in Implementations (Clojure)
All `.nomtu` implementations handling collections should use structurally-shared persistent data structures (HAMTs), backing the flow graph's immutability guarantee at the data structure level.
- ⏳ PLANNED — Phase 0 runtime library (see `03-self-hosting-roadmap.md`).

### ADOPT-9: Range-Constrained Contract Fields (Ada)
Contract fields should support range constraints as first-class syntax: `pre: 0.0 <= security_score <= 1.0`. Checked at compile time, not runtime.
- ✅ SHIPPED (partial) — `where security>0.9` constraint syntax parsed (`nom-lexer/src/lib.rs`). `with at-least 0.85 confidence` threshold per typed slot shipped (commit `97c836f`). ⏳ PLANNED — Compile-time enforcement against callers (Phase 5 verifier).

### ADOPT-10: Structural Interface Satisfaction for Cross-Dictionary Composition (Go)
Two `.nomtu` entries from different namespaces should be composable if contracts are structurally compatible, without explicit `implements` declaration.
- ⏳ PLANNED — Phase 5 verifier. Current resolver checks `kind` match only (`nom-cli/src/store/resolve.rs:70`).

---

## SYNTHESIS — TOP 10 ANTI-PATTERNS NOM MUST AVOID

### AVOID-1: Context-Dependent Semantics (Perl, JavaScript)
Every keyword, operator, and identifier has a fixed meaning regardless of syntactic position. No context sensitivity, ever.
- ✅ SHIPPED — Both `.nom` and `.nomx` parsers are context-free recursive descent (commits `05ee1b6`, `d9425ba`).

### AVOID-2: The `any` Escape Hatch (TypeScript, Python's `Any`)
No mechanism to bypass contract checking at a flow edge. An `any` equivalent would make guarantees worthless.
- ⏳ PLANNED — Not yet enforceable: contract checking at flow edges doesn't exist yet (Phase 5). No `any` keyword has been added, but missing contracts are also not currently a compile error.

### AVOID-3: Multiple Composition Mechanisms (Scala, C++)
Three operators (`->`, `::`, `+`) must stay three. Every new syntax addition must replace an existing one or be demonstrably impossible with the three existing operators.
- ✅ SHIPPED — Three-operator constraint maintained through HEAD `afc6228`. No fourth composition operator added.

### AVOID-4: Gradual Typing With Escape (Python's `mypy`, TypeScript)
Contracts are required on all `.nomtu` entries. A flow edge with a missing contract is a compile error, not a warning.
- ⏳ PLANNED — Missing contracts are not currently a compile error. Phase 5 verifier will enforce this.

### AVOID-5: Breaking Changes to Dictionary Entries (Python 2→3, Perl 5→6)
`.nomtu` entries identified by hash are immutable. A new version is a new entry with a new hash. The dictionary grows; it never mutates.
- ✅ SHIPPED — Hash-addressed storage: `words_v2` table uses `hash TEXT PRIMARY KEY` (`nom-dict/src/lib.rs`; commit `aaa914d`). Hash derived from body bytes (commit `540620d`). No mutation of existing entries by design.

### AVOID-6: Runtime-Only Errors for Contract Violations (Java, PHP)
Every contract violation must be detectable at compile time. If the compiler cannot prove satisfaction, it refuses to compile.
- ⏳ PLANNED — Not yet enforceable (Phase 5 verifier). Current pipeline compiles without contract-violation checks.

### AVOID-7: Invisible Global State (PHP, JavaScript, Ruby)
If a `.nomtu` reads global state, this must appear as an effect. No mechanism to read hidden state without declaration.
- ✅ SHIPPED (partial) — `effects` keyword parsed; effect declarations are required syntax for compositions with side effects (`nom-concept/src/lib.rs`). ⏳ PLANNED — Enforcement that undeclared effects cause a compile error (Phase 5).

### AVOID-8: Separate Toolchain Components (C/C++)
`nom` is the entire toolchain. All capabilities must be subcommands of the single `nom` binary.
- ✅ SHIPPED — Single `nom` binary with subcommands: `store sync`, `store add`, `build status`, `build manifest`, `author translate`, `fmt`, `media import`, `corpus ingest` — all in `nom-cli/src/main.rs`.

### AVOID-9: Marketing Claims Ahead of Implementation (V language)
Every documented feature must be runnable with the current compiler. Aspirational features belong only in explicitly marked `ROADMAP.md`.
- ✅ SHIPPED — This document's status-banner system directly implements this principle. Doc `09-implementation-status-2026-04-13.md` is the canonical evidence backbone.

### AVOID-10: Lazy Evaluation by Default (Haskell)
Nom's flow graph is strictly evaluated. Lazy flows must be explicitly declared as `flow::stream`.
- ✅ SHIPPED (partial) — LLVM backend uses strict evaluation (no lazy thunks). `flow::stream` qualifier not yet in the lexer; lazy flows are not yet distinguishable from strict ones in the syntax.

---

## PARADIGM MAP — HOW NOM RELATES TO EACH PARADIGM

| Paradigm | Primary Representative | Nom's Relationship |
|---|---|---|
| Imperative | C, Zig | `.nomtu` implementations may be imperative; sentence layer is not |
| Object-Oriented | Java, Smalltalk | Composition replaces inheritance; message passing = contract calling |
| Functional | Haskell, OCaml | Flow graph = pure function pipeline; effects are explicit like `IO` |
| Logic | Prolog | Contract matching = unification; `where` clauses = CLP constraints |
| Array | APL, Julia | `.nomtu` for tensors exposes array semantics without iteration syntax |
| Actor | Erlang, Elixir | Flow branches = actors; `onfail` = supervision tree |
| Concatenative | Forth | `->` = explicit concatenative composition with named nodes |
| Reactive | Elm, Svelte | `flow::stream` = reactive data flow; unidirectional |
| Declarative | SQL, Datalog | `where` constraint language = Datalog query over dictionary |
| Language-oriented | Racket | `.nomtu` = `#lang` — each implementation can be a different language |

> Note: entries in the "Nom's Relationship" column describe design intent. Rows marked with ⏳ or ❌ tags above describe which parts are implemented vs. planned.

---

## STATISTICAL SUMMARY

| Category | Languages Analyzed | Key Finding |
|---|---|---|
| Systems | 10 | Zero-cost composition and contract-based allocation strategy are the gaps to fill |
| Application | 8 | Railway-oriented error handling and structured concurrency are validated by mainstream adoption |
| Scripting | 7 | Ecosystem breadth matters as much as language quality; `nom.dev` is critical path |
| Functional | 7 | Property-based testing from contracts and bidirectional inference are highest-value FP innovations |
| Web/UI | 5 | Compiler-erases-framework validated by Svelte; soundness-without-escape validated by ReScript/Elm |
| Modern | 5 | Content-addressed code (Unison) and platform-separated effects (Roc) are clearest architectural validations |
| Niche | 5 | Unification (Prolog), concatenative composition (Forth), and live environment (Smalltalk) are theoretical anchors |
| Vietnamese | 3 | Topic-comment, classifiers, and aspect markers map directly to existing Nom syntax — preserve and extend |

**Total languages:** 50
**Unique design patterns identified:** 63
**Patterns Nom already implements (✅ SHIPPED):** ~14 (partially shipped across the 47 takeaways)
**Patterns in active planning (⏳ PLANNED):** ~38
**Patterns with no near-term plan (❌ ASPIRATIONAL):** ~5
**High-priority adoption targets:** 10
**Anti-patterns to structurally prevent:** 10

> Previous claim of "31 patterns (49%) already implemented" was aspirational. The true SHIPPED count is lower because most takeaways depend on the Phase 5 planner, Phase 6 codegen, or Phase 9 corpus which are not yet built.

---

## Language-specific ingestion priorities for media + UX (added 2026-04-12)

The 50-language survey above informs which ecosystems are first targets for §5.17 mass corpus ingestion and §5.16's codec roadmap (see [`04-next-phases-plan.md`](./04-next-phases-plan.md) §5.16.11-13 and §5.11.6). The following are the load-bearing pre-translated libraries the Nom dictionary must hold — ingested first.

### For media (§5.16 codec landings)

| Codec / Format | Primary library | Language of origin (survey #) | Notes |
|---|---|---|---|
| PNG, JPEG, GIF, BMP, TIFF | `image` crate | Rust (#1) | Pure-Rust, Shape-B-ready |
| PNG, JPEG, TIFF legacy fallback | `libpng`, `libjpeg-turbo`, `libtiff` | C (#2) | FFI-only ingestion; fallback when Rust lib lacks a format |
| AV1 encode | `rav1e` | Rust (#1) | Pure-Rust; lands Phase 5 ahead of the C path |
| AV1 decode | `dav1d` | C (#2) | Fastest AV1 decoder; FFI wrapper |
| AVIF (still + animated) | `libavif` | C (#2) | Wraps `libaom` or `rav1e`; FFI |
| FLAC | `libFLAC` | C (#2) | Mature, stable; FFI |
| Opus | `opus` Rust bindings | Rust (#1) over C | FFI; well-maintained |
| AAC | `fdk-aac` (opt-in patent) | C (#2) | FFI; `faac` as patent-free fallback |
| Audio generic | `symphonia` | Rust (#1) | Pure-Rust multi-codec; Shape-B ingested as a library first |
| MP4 mux | `mp4` crate | Rust (#1) | Pure-Rust; Shape-B |
| WebM/MKV mux | `matroska` crate | Rust (#1) | Pure-Rust; Shape-B — shipped in nom-media (commit `2ec9d2b`) |
| PDF read | `lopdf` | Rust (#1) | Pure-Rust; Shape-B |
| Font rendering | `font-kit`, `harfbuzz`, `freetype` | Rust #1 / C #2 | Mix; `font-kit` is the Rust entry point |
| 3D mesh | `gltf-rs` | Rust (#1) | Pure-Rust; Shape-B |

> **Status**: AVIF encoder shipped in `nom-media` (commit `707aa93`). WebM/Matroska mux shipped (commit `2ec9d2b`). Remaining codecs are ⏳ PLANNED for Phase 5.

**Translation implication:** Rust (#1) and C (#2) are the dominant media-codec source languages. The Phase 5 translator must reach `Complete` status on both before §5.16's 10-codec roadmap can land its full matrix. TypeScript, Python, Go, and others contribute codec libraries rarely; they are not ingestion-critical for media.

### For UX (§5.11 platform-specialization)

| Target platform | UI framework | Host language (survey #) | Notes |
|---|---|---|---|
| Web | Dioxus-web, React, Vue | Rust (Dioxus), TypeScript (#33), JavaScript (#34) | Dioxus is the Nom-native bridge; React/Vue ingested as UX-pattern corpus |
| Desktop | Dioxus-desktop, Tauri, egui | Rust (#1) | Webview-wrapped; Tauri is the closest existing pattern |
| Mobile | Dioxus-mobile, Flutter | Rust (Dioxus), Dart (#18) | Flutter ingestion optional but valuable for cross-platform patterns |
| Native cross-platform | Qt, GTK | C++ (#3), C (#2) | Secondary; mostly for legacy desktop UX pattern harvesting |

> **Status**: ⏳ PLANNED — Phase 5+. `nom-ux` crate scaffold exists. No UX ingestion has run yet.

**Translation implication:** TypeScript/JavaScript (#33, #34) ingestion is the critical path for UX-pattern extraction (React, Vue, Svelte component libraries, hooks patterns, animation libraries like Motion/Framer). Rust ingestion for Dioxus covers the runtime side. Dart is nice-to-have for Flutter patterns but not blocking.

### Combined priority

**The top 3 languages to reach `Complete` translator status for media + UX work are Rust (#1), C (#2), and TypeScript (#33).** Phase 5 translator effort should concentrate there until §5.16 + §5.11.6 can land their full codec + platform matrix. Python (#19) and the functional languages (Part IV) are lower priority for media/UX — they're ingestion targets for general scientific/algorithmic vocabulary, not codec/UI surfaces.

### Revision under the body-as-compiled-artifact shift (§4.4.6, 2026-04-12)

After the architectural shift captured in [`04-next-phases-plan.md`](./04-next-phases-plan.md) §4.4.6, the dict stores `.bc` (LLVM bitcode) compiled directly from upstream source — **not Nom-translated source**. This reshapes the "translator completeness" criterion:

- **"Complete translator" for a language now means: the language's upstream compiler reliably emits `.bc` for the packages we want to ingest.** That's trivially true for Rust (`rustc --emit=llvm-bc`), C/C++ (`clang -emit-llvm`), and the LLVM-frontended languages (Swift, Zig, Rust, C, C++, Objective-C, Fortran). It is NOT trivially true for languages that target a different IR (JVM bytecode: Java, Kotlin, Scala, Clojure; CLR IL: C#, F#; V8 or JavaScriptCore: JavaScript, TypeScript; Python bytecode; Erlang BEAM).
- **Ingestion priority reshuffles.** LLVM-frontended languages become first-class ingestion targets because they already produce `.bc`. Non-LLVM languages require either (a) an additional backend translator (JVM→LLVM via GraalVM LLVM, WASM-bytecode→LLVM via wasm2c+clang, or JS→LLVM via QuickJS-to-WASM-to-LLVM) or (b) transpilation to one of the LLVM-frontend languages first (TS→JS→WASM→`.bc` is one path; TS→Rust via Dioxus tooling is another).
- **Reprioritized top-3**: **Rust (#1), C (#2), C++ (#3)** — all direct LLVM frontends. TypeScript (#33) drops from the top 3 for code ingestion and becomes a UX-pattern source only (React/Vue extraction as metadata, not bodies). Swift (#15), Zig (#4), and Fortran become viable ingestion targets they weren't before, because their `.bc` output is directly consumable.
- ✅ SHIPPED — `.bc`-as-body storage implemented (commit `540620d`). `body_kind` field distinguishes `.bc` from other body types (`nom-dict/src/lib.rs`; commit `6c336b4`).
