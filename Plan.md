# Nom Language Development Roadmap

Development plan for the Nom programming language. Focused on the language,
compiler, and dictionary -- not OS or product concerns.

---

## Phase A: Core Compiler -- IN PROGRESS

Build the compilation pipeline from `.nom` source to native binary.

### Compile to Binary -- DONE

The `nom build` command produces native executables.

- 10 compiler crates: nom-ast, nom-lexer, nom-parser, nom-resolver,
  nom-verifier, nom-codegen, nom-planner, nom-security, nom-diagnostics, nom-cli
- 42 tests passing across all crates
- `nom build auth.nom` produces an 834KB Windows binary from 9 lines
- Parser handles all 10 classifiers, 42 keywords, graph and agent primitives
- Lexer supports all operators, literals, comments, blank-line detection

### Semantic Resolution -- DONE

The resolver maps `.nom` references to nomdict dictionary entries.

- nom-resolver crate connects parsed NomRef nodes to dictionary words
- 10M+ .nomtu entries in the SQLite nomdict database
- Word + variant lookup: `hash::argon2` resolves to the argon2 entry

### Import System -- RUNNING

Multi-file programs need to reference declarations across `.nom` files.

- Single-file compilation works end to end
- Cross-file imports not yet implemented
- Planned: `need` statements resolve across the project, not just the dictionary

### Remaining Phase A Work

- [ ] Cross-file import resolution
- [ ] Contract type checking across flow edges
- [ ] Score-based selection when multiple dictionary entries match
- [ ] Glass box composition report generation
- [ ] Error recovery in parser (currently stops at first error)

---

## Phase B: Failure Prevention

Systematically address the 47 failure patterns identified in the world
language survey (see `research/01-world-language-survey.md`).

These are the mistakes made by 40+ programming languages that Nom must avoid.
Each is a concrete technical requirement derived from studying real failures.

### Categories

| Category | Count | Key Items |
|----------|-------|-----------|
| Type system failures | 8 | No null, no implicit coercion, no type erasure |
| Memory safety failures | 6 | No manual memory, no GC pauses, graph-inferred ownership |
| Concurrency failures | 5 | No colored functions, no data races, topology-driven concurrency |
| Composition failures | 7 | No diamond inheritance, no implicit dependencies, explicit effects |
| Error handling failures | 4 | No unchecked exceptions, no silent failures, contract-based errors |
| Ecosystem failures | 6 | No version hell, no trust-by-default, scored dictionary |
| Syntax failures | 5 | No ambiguous grammar, no semicolon debates, writing-style surface |
| Tooling failures | 6 | No slow compilers, no missing source maps, integrated diagnostics |

### Phase B Approach

Each failure pattern becomes a compiler or verifier check. The nom-verifier
and nom-security crates are the enforcement points. The goal: make it
structurally impossible to introduce these failures, not just discouraged.

---

## Phase C: Assembly-Smooth Performance

Make Nom-compiled binaries competitive with hand-written Rust in performance.

### Goals

- Zero-cost composition: the flow graph compiles away to direct function calls
- Graph-inferred memory: move semantics for linear chains, Arc+Mutex only
  where the topology requires sharing
- Topology-driven concurrency: tokio for IO-bound, rayon for CPU-bound,
  sequential for dependent -- all inferred from the composition graph
- No runtime overhead: no reflection, no garbage collector, no interpreter

### Prerequisites

- Phase A contract verification (needed to prove optimizations are safe)
- Phase B failure prevention (needed to guarantee safety invariants)

### Approach

1. Benchmark current codegen output against equivalent hand-written Rust
2. Identify abstraction overhead in generated code
3. Implement graph-aware optimization passes in nom-codegen
4. Target: within 5% of hand-written Rust on standard benchmarks

---

## Phase D: LLVM Backend

Replace the current Rust codegen (Nom -> Rust source -> rustc -> binary) with
direct LLVM IR emission (Nom -> LLVM IR -> binary).

### Why

- Faster compilation: skip Rust parsing and type checking
- Better error messages: source maps point directly to .nom lines
- Smaller binaries: no Rust runtime overhead
- Foundation for self-hosting

### Prerequisites

- Phase C performance baseline (needed to measure improvement)
- Stable IR format (.nomiz) that captures the full composition graph

### Approach

1. Define .nomiz as a stable serialization of the verified composition graph
2. Write LLVM IR emitter that reads .nomiz
3. Gradually migrate from Rust codegen to LLVM codegen
4. Keep Rust codegen as fallback for debugging

---

## Evidence and Research

| Document | What It Informs |
|----------|-----------------|
| `research/01-world-language-survey.md` | Phase B: 47 failure patterns to prevent |
| `research/02-vietnamese-grammar-to-novel-syntax.md` | Classifier design, composition operators |
| `research/03-chu-nom-to-nom-encoding.md` | NomID encoding, dictionary architecture |
| `research/04-novel-language-spec.md` | Phase A: syntax specification |
| `research/05-beyond-transformers.md` | Phase D: why direct compilation matters |
| `research/06-blueprint-for-building.md` | Phase A-D: engineering implementation plan |
| `research/07-vietnamese-flexibility.md` | Anchored-flexible syntax design |
| `SYNTAX.md` | Formal syntax reference (matches compiler implementation) |
| `BLUEPRINT.md` | Full build plan including products and ecosystem |

---

## File Format Reference

| Extension | What | Current Status |
|-----------|------|----------------|
| `.nom` | Source file (sentences) | Compiles to binary |
| `.nomtu` | Dictionary word entry | 10M+ in nomdict |
| `.nomiz` | Compiled composition graph (IR) | Planned for Phase D |
| `nomdict` | Dictionary database (SQLite) | Operational |

---

## Milestones

| Milestone | Status |
|-----------|--------|
| Lexer tokenizes all 42 keywords | Done |
| Parser handles all 10 classifiers | Done |
| Parser handles graph primitives (node, edge, query, constraint) | Done |
| Parser handles agent primitives (capability, supervise, receive, state, schedule) | Done |
| `nom build` produces native binary | Done |
| auth.nom -> 834KB binary from 9 lines | Done |
| 42 compiler tests passing | Done |
| 10M+ dictionary entries in nomdict | Done |
| Cross-file imports | In progress |
| Contract type checking across flow edges | Planned |
| Glass box composition reports | Planned |
| Assembly-smooth performance | Planned |
| LLVM backend | Planned |
