# Part 6: Blueprint for Building Novel

**How to turn the research into an actual programming language, what the world's languages still have not solved, and how to compose a new language without repeating old mistakes.**

Research conducted: 2026-04-11
Primary references: official language docs, implementation tutorials, and language-workbench sources

---

## Executive Summary

Novel already has a strong idea:

- language as semantic composition, not token-by-token coding
- Vietnamese grammar as compression and disambiguation strategy
- Nom as the atomic unit of meaning
- Novelos as the verifier and composer

What is still missing is the engineering blueprint.

This document fills that gap:

1. It maps the global programming-language landscape into the design pressures each family optimizes for.
2. It identifies the problems that even modern languages still do not solve cleanly.
3. It explains how to compose a programming language as a stack of layers, not as "syntax plus compiler".
4. It proposes a concrete implementation plan for Novel from parser to verifier to code generation.
5. It recommends a realistic MVP so Novel becomes buildable, not just visionary.

The central conclusion is simple:

**Do not build Novel as "a whole new everything" on day one.**
Build it as:

- a small surface language
- a semantic graph IR
- a verified Nom dictionary
- a deterministic resolver/verifier
- one excellent backend

That is the shortest path from concept to reality.

---

## 1. The World Programming Language Map

Every programming language family is an optimization for a different pain:

| Family | Representative Languages | What it optimizes for | What it usually sacrifices |
|---|---|---|---|
| Systems | C, C++, Rust, Zig | control, performance, predictable layout | ergonomics, safety, compile complexity |
| Managed runtime | Java, C#, Kotlin, Swift | productivity, tooling, large-team maintainability | runtime overhead, deep abstraction layers |
| Dynamic/scripting | Python, JavaScript, Ruby, PHP | speed of writing, flexibility, REPL culture | runtime safety, predictability, packaging coherence |
| Functional | Haskell, OCaml, F#, Elm, Koka | composition, reasoning, strong semantics | learning curve, ecosystem size, runtime model tradeoffs |
| Concurrent/distributed | Erlang, Elixir, Go | process-level concurrency, deployment simplicity | precision in effects, backpressure ergonomics, memory tuning |
| Logic/data | SQL, Prolog, Datalog | declarative problem solving | integration with general application logic |
| Language-oriented | Racket, Neverlang, MPS-style systems | custom language creation, feature composition | mainstream adoption, operational simplicity |
| Portable IR platforms | LLVM, WebAssembly, MLIR | retargetability, optimization pipelines, interoperability | source-level ergonomics are externalized to frontends |

Novel should not try to "beat" every family on its own terms.
It should instead combine the strongest properties:

- systems-level verifiability
- managed-language tooling quality
- scripting-level intent expression
- functional-style composition
- language-workbench modularity
- LLVM/Wasm-grade lowering discipline

That makes Novel a meta-language architecture, not just another syntax.

---

## 2. What Languages Still Have Not Solved

Part 1 already documented many failures across 40+ languages. The deeper question is:
**what is still unsolved even after modern languages learned from older mistakes?**

### 2.1 Concurrency is still split into awkward worlds

Python's accepted PEP 703 is direct evidence that even one of the world's most important languages still struggles with the cost of its original concurrency model. Removing or disabling the GIL requires major runtime changes, ABI complications, and migration pain.

Async/await improved ergonomics, but Bob Nystrom's classic "What Color is Your Function?" critique still applies at the structural level: async code often remains a second function world with different calling rules, wrappers, and control-flow constraints.

**Novel implication:** concurrency should be inferred from the composition graph as much as possible, not manually threaded through every function boundary.

### 2.2 Effect systems are promising, but not yet mainstream-practical

Koka is one of the strongest proofs that effect types and handlers are real, not fantasy. But its own documentation still admits it is a research language and not yet production-ready, with libraries and package management still weaker than mainstream languages.

That means the idea is good, but the delivery problem remains open.

**Novel implication:** effect tracking should be central, but the first implementation must keep the effect vocabulary small and operationally useful.

### 2.3 Language evolution still fractures ecosystems

Rust's edition system shows one of the cleanest modern answers: allow incompatible surface changes without splitting the ecosystem, and keep crates interoperable across editions. WebAssembly goes further with an evergreen standard model while preserving backward compatibility.

These are strong patterns, but most languages still evolve through painful migrations, hidden breakage, or ecosystem forks.

**Novel implication:** keep the grammar small and stable; evolve meaning through dictionary growth, metadata, and editions of semantics only when absolutely necessary.

### 2.4 Tooling is still treated as "after the language"

Tree-sitter exists because language tooling needs fast, incremental, error-tolerant parsing. Many languages were designed before IDE responsiveness, partial parsing, structural search, and language servers became table stakes.

**Novel implication:** parser structure, recoverable syntax, source ranges, and semantic IDs must be part of the language design from day one.

### 2.5 Most languages compose libraries, not language features

Racket demonstrates that languages can be ecosystems for language-oriented programming, not just programs. Neverlang goes further and treats syntax, type-checking, evaluation, and tooling as composable feature slices.

Mainstream languages rarely do this well. They let you compose libraries, but not cleanly compose syntax, semantics, effects, tooling, and lowering behavior together.

**Novel implication:** Nom composition should not stop at function reuse. It should compose:

- syntax-level constructs
- semantic contracts
- effect rules
- verification rules
- backend lowering strategies
- observability/reporting behavior

### 2.6 Verification still happens too late

Most languages verify syntax, maybe types, and then defer the real risk to tests, runtime incidents, or human review. Novel's big opportunity is to shift more checks into semantic composition:

- contract compatibility
- effect permissions
- provenance and trust
- resource strategy
- backend suitability

**Novel implication:** verification is not a compiler pass near the end. It is the organizing principle of the entire pipeline.

### 2.7 AI-era development exposed a new missing layer: grounded intent

This is the newest gap. Most languages optimize for humans typing implementation detail. LLM systems optimize for generating plausible text. Neither gives a strong, native bridge from human intent to verifiable system construction.

Novel can occupy that gap if it is built around:

- explicit concepts
- constrained resolution
- deterministic selection
- glass-box reporting

That is likely the most important unsolved space Novel can own.

---

## 3. How To Compose A Programming Language

The mistake many new language projects make is thinking a language is:

`syntax + parser + compiler`

That is too shallow.

A real language is composed across at least eight layers:

| Layer | Question | Novel choice |
|---|---|---|
| Lexical | What are the atoms? | keywords, classifiers, operators, Nom references |
| Surface syntax | How do humans write programs? | Vietnamese-inspired analytic grammar |
| Semantic core | What do programs mean? | composition graph of typed Noms |
| Static reasoning | What can be proved before execution? | contracts, effects, constraints, provenance |
| Execution model | How does work happen? | graph-derived sequential/parallel strategy |
| IR/lowering | How does meaning become executable? | semantic graph IR -> Rust/LLVM, later Wasm |
| Tooling model | How do editors and debuggers understand it? | tree-sitter + semantic IDs + glass-box report |
| Evolution model | How does the language change safely? | stable grammar, growing dictionary, versioned semantics |

If any one of these is weak, the language feels unstable.

### 3.1 Compose features vertically, not only horizontally

Neverlang's biggest lesson is that a language feature is not only syntax. A real feature has multiple roles:

- syntax
- typing/verification
- evaluation or lowering
- tooling support
- debugging/observability behavior

Novel should define each Nom or core construct the same way.

For example, a `rate_limiter` Nom should not just mean "some implementation exists".
It should carry:

- interface contract
- effect declaration
- performance/security scores
- allowed composition contexts
- lowering recipe
- report/explanation metadata

That is language composition in the deep sense.

### 3.2 Separate human syntax from machine semantics

LLVM and MLIR tutorials both show the right architectural instinct:

- start with a readable source language
- lower into increasingly explicit intermediate forms
- attach more semantics as you descend
- keep optimization and code generation on structured IR, not raw source

Novel should preserve this separation:

1. `source Novel` for humans
2. `semantic graph IR` for meaning and verification
3. `backend IR` for optimization and execution

Do not let backend concerns leak back into user syntax too early.

### 3.3 Compose semantics around contracts, not inheritance

Inheritance-centered languages tend to entangle reuse with subtyping, construction, visibility, and runtime behavior. Novel already points in a better direction:

- use contract compatibility
- use capability/effect declarations
- use graph composition
- use explicit constraints

This should stay foundational.

### 3.4 Compose for partial understanding

Tree-sitter's lesson is important: tools must keep working when the program is incomplete or temporarily invalid.

Novel should support partial composition states:

- unresolved Nom references
- constraint conflicts
- missing backend candidates
- soft verification warnings

That allows live editing, interactive explanation, and incremental resolution instead of all-or-nothing compilation.

---

## 4. Blueprint: How To Build Novel For Real

This is the concrete build plan.

### Phase 0: Freeze the design kernel

Before writing compiler code, freeze a minimal semantic kernel:

- classifiers
- declarations
- flow syntax
- composition operators
- constraint syntax
- effect vocabulary
- Nom metadata schema

Deliverables:

- EBNF or tree-sitter grammar sketch
- Nom schema spec
- verifier invariants list
- 10-20 worked examples

Success criterion:
two different people can write the same example in nearly the same way.

### Phase 1: Build the parser and source model

Use tree-sitter for the first parser because it gives:

- incremental parsing
- error recovery
- editor integration potential
- stable syntax trees for tooling

Deliverables:

- `tree-sitter-novel` grammar
- CST to AST lowering
- source spans and node IDs
- parse error diagnostics

Success criterion:
the parser can round-trip all examples and survive malformed input.

### Phase 2: Build the Nom dictionary format

This is the true heart of Novel.

A Nom record should minimally contain:

- NomID
- kind/classifier
- human name and aliases
- input/output contract
- effect set
- trust/provenance metadata
- quality scores
- backend implementations available
- test evidence
- explanation text for reports

Use a simple serializable format first:

- JSON
- YAML
- or SQLite if lookup/querying becomes important early

Do not build a distributed ecosystem first.
Build a local, deterministic dictionary first.

Success criterion:
Novel can resolve a small but useful closed-world dictionary with no network dependency.

### Phase 3: Build the resolver

The resolver turns user-written references into candidate Noms.

Responsibilities:

- name lookup
- alias expansion
- kind filtering
- constraint filtering
- score ranking
- ambiguity reporting

The output should not be executable code.
It should be a **resolved-but-not-yet-approved semantic graph**.

Success criterion:
for each unresolved token, the engine can explain why it chose a candidate or why it could not.

### Phase 4: Build the verifier

This is the make-or-break phase.

The verifier should check:

- input/output contract compatibility
- effect compatibility
- declared constraints
- purity boundaries
- provenance/security thresholds
- resource model conflicts
- graph cycles and illegal execution shapes

Important design choice:
split diagnostics into:

- hard failure: cannot compose
- soft warning: composes, but below preferred threshold
- choice point: multiple valid candidates remain

Success criterion:
verification reports are understandable to a non-expert reader.

### Phase 5: Build the semantic graph IR

Do not compile directly from AST to Rust.
Insert a proper Novel IR between them.

Suggested IR layers:

1. AST
2. Resolved graph IR
3. Verified/annotated graph IR
4. Backend plan IR

Each layer should remove ambiguity and add operational detail.

Annotated graph IR should include:

- selected Nom for each node
- effect annotations
- concurrency annotations
- storage/runtime strategy
- report metadata

Success criterion:
the IR is rich enough that code generation becomes mostly mechanical.

### Phase 6: Build one excellent backend

Start with one backend only:

- Rust source generation

Why Rust first:

- strong safety model
- excellent ecosystem
- access to LLVM
- easy native binaries
- reasonable fit for effectful systems code

Do not target many backends initially.
Every extra backend multiplies semantic complexity.

Success criterion:
one end-to-end Novel program compiles into a readable Rust project and passes tests.

### Phase 7: Add reporting as a first-class artifact

Novel's differentiator is not only code generation.
It is explanation.

Every compile should emit a glass-box report containing:

- what each source construct resolved to
- what alternatives were rejected
- what trust/provenance data mattered
- what effects exist
- what runs in parallel vs sequentially
- what guarantees were proved
- what remains assumed, not proved

Success criterion:
the report is useful even if the generated code is never read.

### Phase 8: Add a tiny standard library, not a huge ecosystem

The first Nom dictionary should be opinionated and small.

Suggested initial domains:

- text transform
- JSON parsing/validation
- filesystem read/write
- HTTP endpoint composition
- hashing/password work
- rate limiting
- simple store/cache abstraction

These are enough to prove the model.

Do not build first:

- GUI
- distributed agents
- full NL-to-Nom interpretation
- complex graph database features
- self-hosting

### Phase 9: Tooling and developer loop

Once the language can resolve and verify simple programs, add:

- formatter
- LSP server
- hover on Nom metadata
- "why this Nom?" explanation command
- graph visualization
- verification diff between revisions

Tooling should be generated from the semantic pipeline, not bolted on manually.

---

## 5. The Real MVP For Novel

The current docs are ambitious. That is good for vision, but dangerous for delivery.

The actual first shippable Novel should be much smaller.

### Keep in MVP

- `flow`, `system`, `test`
- `need`, `flow:`, `require`, `effects`
- `->`, `where`, `::`, `+`
- a tiny effect vocabulary
- Nom scores and provenance
- parser -> resolver -> verifier -> Rust generation
- glass-box report

### Defer until later

- `agent`, `graph`, `pool`, `view` as full features
- natural language front door
- multiple backends
- advanced ownership inference
- distributed scheduling
- self-hosted compiler
- full marketplace/package ecosystem

This is the most important tactical recommendation in this document.

If Novel tries to ship all ideas at once, it will likely become another elegant language paper with no durable runtime reality.

---

## 6. Recommended Internal Architecture

```text
.novel source
    |
    v
tree-sitter parser
    |
    v
CST -> AST lowering
    |
    v
resolver
    |
    v
candidate semantic graph
    |
    v
verifier + scorer + disambiguator
    |
    v
verified graph IR
    |
    +--> glass-box report
    |
    v
backend planner
    |
    v
Rust emitter
    |
    v
Cargo / rustc / LLVM
    |
    v
binary + tests + report bundle
```

Recommended implementation language for Novelos:

- Rust for the core engine

Why:

- good fit with tree-sitter and LLVM-adjacent tooling
- strong enums and pattern matching for AST/IR work
- ownership discipline helps compiler internals stay explicit
- easy CLI, serialization, and testing support

---

## 7. Open Research Risks

Novel is promising, but these are still hard problems:

### 7.1 Semantic aliasing

Two Noms can look equivalent but differ in hidden assumptions.
You need a strong contract vocabulary or the resolver will make pretty but wrong substitutions.

### 7.2 Scoring can become pseudo-objective

"Security: 0.96" is useful only if the score has clear derivation, provenance, update policy, and auditability.
Otherwise it becomes decorative numerology.

### 7.3 Graph-aware ownership is harder than it sounds

The research vision is strong, but memory strategy inference across shared mutable graphs is one of the hardest systems problems in the whole project.
Treat it as later-stage research, not day-one compiler truth.

### 7.4 Verification boundaries must be explicit

Novel should clearly separate:

- proved
- checked heuristically
- trusted from provenance
- assumed

If these are blurred, the "glass box" promise collapses.

### 7.5 Dictionary curation is a socio-technical problem

The language is only as strong as the Nom dictionary:

- who can publish
- who can review
- how trust is earned
- how versions coexist
- how malicious or low-quality Noms are excluded

This is a governance system as much as a technical one.

---

## 8. The Blueprint In One Sentence

**Build Novel as a verified semantic composition engine with a small human syntax, a strict Nom dictionary, a graph IR, one backend, and first-class reports.**

That is the shortest path to a real language.

---

## 9. Immediate Next Steps For This Repo

1. Freeze a tiny `Novel MVP` grammar from Part 4 instead of expanding the syntax surface further.
2. Define a machine-readable Nom schema with example entries for 10-20 core Noms.
3. Write three canonical end-to-end examples:
   - secure login flow
   - file ingest and validate flow
   - rate-limited API endpoint
4. Build `tree-sitter-novel` and parser tests.
5. Implement resolver and verifier before any natural-language front end.
6. Emit Rust for one example and make the generated project readable.
7. Add a report format before expanding language surface area.

---

## 10. Primary References

These were especially useful because they speak directly to language design, language implementation, composition, and modern ecosystem pain:

- PEP 703, "Making the Global Interpreter Lock Optional in CPython"  
  https://peps.python.org/pep-0703/

- Bob Nystrom, "What Color is Your Function?"  
  https://journal.stuffwithstuff.com/2015/02/01/what-color-is-your-function/

- LLVM, "My First Language Frontend with LLVM"  
  https://llvm.org/docs/tutorial/MyFirstLanguageFrontend/

- MLIR, "Toy Tutorial"  
  https://mlir.llvm.org/docs/Tutorials/Toy/

- Crafting Interpreters  
  https://craftinginterpreters.com/index.html

- tree-sitter  
  https://github.com/tree-sitter/tree-sitter

- Racket Languages / language-oriented programming ecosystem  
  https://www.racket-lang.org/languages.html

- Neverlang language workbench  
  https://neverlang.di.unimi.it/

- Koka language book  
  https://koka-lang.github.io/koka/doc/book.html

- Rust Edition Guide  
  https://doc.rust-lang.org/edition-guide/editions/

- WebAssembly 2.0 / evergreen standard model  
  https://webassembly.org/news/2025-03-20-wasm-2.0/

---

## Final Take

Novel should not ask:

> "How do we invent a prettier syntax than other languages?"

It should ask:

> "How do we make meaning explicit, composition deterministic, verification central, and evolution non-destructive?"

If Novel answers that well, the syntax will matter.
If it does not, the syntax will not save it.
