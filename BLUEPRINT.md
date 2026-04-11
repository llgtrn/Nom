# Nom — The Definitive Blueprint

**Concrete build plan for a compositional programming language
where .nom sentences compose .nomtu words from an online dictionary
into verified, native-compiled applications.**

## The Stack

```
.nom          Sentences you write (English, writing-style, no braces)
.nomtu        Words in the dictionary (text: name + description + contract)
nom.dev       Online dictionary (millions of .nomtu entries)
nom           The compiler CLI (resolve, verify, compile → binary)
.nomiz        Compiled composition graph (the IR)
binary        Native executable (LLVM, assembly-smooth)
```

```
system auth
need hash::argon2 where security>0.9
need store::redis where reliability>0.8
flow request->hash->store->response

↓ nom build auth.nom ↓

→ native binary, verified contracts, glass box report
```

---

## Part I: Coherence Audit — How Well Do the Ideas Fit Together?

### Overall Verdict: 93% Coherent, 3 Tensions to Resolve

The 9 research documents, 2 spec files, and OSS analysis form a remarkably
consistent vision. The core thesis holds across all material:

> **Software is language. Nom is the dictionary. Novel is how you speak it.**

Every document reinforces this from a different angle:

| Document | Angle | Core Contribution |
|----------|-------|------------------|
| novel (spec) | Language vision | The 5th paradigm: compositional-semantic |
| nom (spec) | Dictionary vision | One Nom = one meaning = one contract |
| 01 | Defensive | 47 failures across 40+ languages to avoid |
| 02 | Linguistic | Vietnamese grammar → Novel syntax (13 mappings) |
| 03 | Encoding | Chữ Nôm → NomID architecture (3-layer identifier) |
| 04 | Specification | Draft v0.1 syntax with 10 classifiers, operators |
| 05 | Architectural | Transformer critique + Vietnamese compression |
| 06 | Engineering | 9-phase build plan, 8-layer feature composition |
| 07 | Flexibility | Anchored-flexible syntax, tolerant surface forms |
| 08 | Ecosystem | Vietnamese OSS: underthesea, MELT, PhoGPT |
| 09 | Global | Vietnamese grammar + world vocabulary packs |

### What's FIRM (Consistent Across 3+ Documents)

These decisions appear everywhere and are non-negotiable:

**1. Compositional-semantic paradigm**
Stated in: novel, nom, 01, 02, 04, 05, 06
Programs are composed from dictionary, not written line by line.

**2. Vietnamese analytic grammar as syntax model**
Stated in: novel, 02, 04, 05, 07, 09
Immutable tokens, mandatory classifiers, topic-comment, modifier-follows-head.

**3. Mandatory kind classifiers**
Stated in: 02, 04, 06, 07
Every declaration requires: flow, agent, store, graph, gate, nom, system, pool, view, test.

**4. Three composition operators**
Stated in: novel, 02, 04, 06
`::` (subordinate), `+` (coordinate), `->` (flow).

**5. Effect declarations explicit on every Nom**
Stated in: novel, nom, 01, 04, 05
Effects: network, filesystem, database, clock, random, cpu_intensive.

**6. Scored dictionary with provenance**
Stated in: novel, nom, 03, 04, 05, 06
8 scores (security, quality, performance, reliability, composability,
maintenance, maturity, accessibility) + source repo + license + tests.

**7. Glass box transparency**
Stated in: novel, 04, 05, 06, 09
Composition reports show what Noms were used, scores, contracts, effects.

**8. Graph-aware ownership (no GC, no borrow checker)**
Stated in: novel, 01, 04, 05, 06
Engine infers memory strategy from composition graph topology.

**9. Compilation target: Rust → LLVM → native binary**
Stated in: novel, 04, 05, 06
One backend first. Rust source generation.

**10. Dictionary grows, syntax stable**
Stated in: novel, nom, 01, 04, 06, 09
No version bumps. No breaking changes. New Noms alongside old.

### What's TENTATIVE (Mentioned 1-2 Times, Needs Validation)

**1. Six tonal modifiers (!, ~, ?, ^, .)**
Only in: 02, 04. Conceptually beautiful but:
- Do they compose? (`hash!?` = strict + optional?)
- Do they conflict? (`hash!~` = strict + light = contradiction?)
- Are 6 enough? Too many?
→ **Decision needed:** Keep as research concept. Validate in Phase 1 DSL.

**2. Temporal markers (da/dang/se)**
Only in: 02, 04. Questions:
- Are these compile-time only or runtime-observable?
- How does `se` (deferred) interact with `da` (verified) in a composition?
→ **Decision needed:** Defer to Phase 2. Focus on verified vs. unverified first.

**3. Effect valence (duoc/bi)**
Only in: 02, 04. Questions:
- Is valence a property of the effect or the context?
- Can the same effect be `duoc` in one context and `bi` in another?
→ **Decision needed:** Keep as design goal. Implement as metadata, not syntax.

**4. ~1B Transformer for intent resolution**
Only in: 05. Questions:
- Training data? Labeled NL → Nom concept pairs don't exist yet.
- Is a classifier sufficient instead of a generative model?
→ **Decision needed:** Defer entirely. Phase 0-2 use explicit Nom references.

**5. Semantic locality via LSH in NomID**
Only in: 03. Questions:
- Which embedding model for contracts?
- How to handle LSH collisions at 100M+ scale?
→ **Decision needed:** Start with kind prefix + content hash only. Add LSH later.

### Three Tensions to Resolve

**Tension 1: MVP Simplicity vs. Multilingual Ambition**

Doc 06 says: "Small surface language, defer NL front end."
Doc 09 says: "Locale-tagged alias packs, per-file locale declarations."

**Resolution:** Phase 1-2 use English keywords only. Multilingual aliases are
Phase 3+ concern. The NomID is always language-neutral (content hash).
Aliases are a lookup layer on top.

**Tension 2: "Provably Correct" Claims vs. Implementation Reality**

Docs 01, 04, 05 claim "provably correct via contract composition."
But contract verification is unspecified: language? decidability? complexity?

**Resolution:** Be precise about what's verified:
- Contract TYPE compatibility (input/output matching) = decidable, implement first
- Contract SEMANTIC correctness (does the Nom actually do what it says) = trusted from provenance
- Contract BEHAVIORAL verification (runtime properties) = tested, not proved
Never claim "provably correct" — claim "contract-verified" instead.

**Tension 3: Expressiveness vs. Dictionary Dependence**

Novel's power comes from the dictionary. But:
- At 2.68M Noms: many gaps exist.
- At 12M Noms: better but still incomplete for niche domains.
- Novel algorithms have no Nom — need escape hatch.

**Resolution:** The `nom` classifier IS the escape hatch. Make it first-class,
not second-class. Custom Noms should be as easy to define as library functions.
Over time, custom Noms get extracted into the dictionary.

---

## Part II: The 45 Concrete Technical Decisions (Consolidated)

From all documents, these are the decisions firm enough to implement:

### Syntax

| # | Decision | Source |
|---|---------|--------|
| 1 | 10 mandatory classifiers: flow, agent, store, graph, gate, nom, system, pool, view, test | 02, 04 |
| 2 | Topic-comment declarations: `classifier name { ... }` | 02, 04 |
| 3 | Flow operator: `->` (serial verb, one composite operation) | 02, 04 |
| 4 | Subordinate composition: `::` (narrowing, like xe::lửa) | 02, 04 |
| 5 | Coordinate composition: `+` (broadening, like cha+mẹ) | 02, 04 |
| 6 | Constraint operator: `where` (mà particle) | 02, 04 |
| 7 | Branching: `{ branch A; branch B } -> merge` | 04 |
| 8 | Conditional: `{ ok -> X; err -> Y }` | 04 |
| 9 | No semicolons, no articles, no let/const/var | 02, 04 |
| 10 | Modifier-follows-head: `hash where security > 0.9` | 02, 04 |

### Nom Dictionary

| # | Decision | Source |
|---|---------|--------|
| 11 | Each Nom: one meaning, one contract, one truth | nom |
| 12 | 8 scores: security, quality, performance, reliability, composability, maintenance, maturity, accessibility | nom, 04 |
| 13 | Provenance: source repo, commit hash, license, extraction date, test count | nom, 03 |
| 14 | Contract: in types, out types, effects, preconditions, postconditions | nom, 04 |
| 15 | Variants: multiple implementations ranked by score, canonical selected | nom, 03 |
| 16 | Kind taxonomy: ~200 kinds (like Kangxi radicals) | 03 |
| 17 | Growth: extraction from real repos, not manual curation | nom, 06 |

### NomID Encoding

| # | Decision | Source |
|---|---------|--------|
| 18 | Layer 1: Kind prefix (8-16 bits) — the radical | 03 |
| 19 | Layer 3: Content hash (224 bits) — SHA-256 of contract + impl | 03 |
| 20 | Composition ID: Merkle tree (SHA-256 of operator + child IDs) | 03 |
| 21 | Short form: kind prefix only for common references | 03 |

### Compilation Pipeline

| # | Decision | Source |
|---|---------|--------|
| 22 | Parse: tree-sitter grammar → AST | 06 |
| 23 | Resolve: map names to Nom dictionary concepts | 04, 06 |
| 24 | Select: pick best Noms by score + constraints | 04, 06 |
| 25 | Verify: check contract compatibility across all edges | 04, 06 |
| 26 | Analyze: infer memory strategy + concurrency from graph | 04, 05 |
| 27 | Generate: emit Rust source from Nom implementations | 04, 06 |
| 28 | Compile: Rust → LLVM → native binary | 04 |
| 29 | Report: glass box composition report | 04, 05, 06 |

### Runtime Behavior

| # | Decision | Source |
|---|---------|--------|
| 30 | Memory: move (linear chains), RwLock (shared reads), Arc+Mutex (concurrent writes) | 01, 04 |
| 31 | Concurrency: tokio (IO-bound), rayon (CPU-bound), sequential (dependent) | 01, 04 |
| 32 | Effects: set union through composition graph | 04 |
| 33 | Error handling: contract composition, engine generates handling | 01, 04 |
| 34 | No GC, no manual memory, no borrow checker, no async/await | 01, 04, 05 |

### Principles

| # | Decision | Source |
|---|---------|--------|
| 35 | No inheritance, implicits, operator overloading | 01, 04 |
| 36 | No type erasure, no "any" escape hatch | 01, 04 |
| 37 | No colored functions | 01, 05 |
| 38 | Vietnamese grammar: analytic, SVO, classifiers, topic-comment | 02, 04, 07, 09 |
| 39 | One canonical semantic form, many surface representations | 07, 09 |
| 40 | Compiler written in Rust | 06 |

---

## Part III: The 25 Open Questions (Prioritized)

### Must Answer Before Phase 1

| # | Question | Why Critical |
|---|---------|-------------|
| 1 | **What language do contracts use?** First-order logic? Refinement types? Simple predicates? | Determines if verification is decidable |
| 2 | **How are Noms selected when scores tie?** Deterministic rule needed. | Non-determinism = non-reproducible builds |
| 3 | **What is the minimal contract vocabulary?** in/out types, effects — what else? | Defines what the verifier checks |
| 4 | **How does the escape hatch (`nom`) define implementation?** Inline Rust? External file? | Needed for any real program |
| 5 | **What is the dictionary storage format?** SQLite? Custom? Content-addressed? | Determines tooling architecture |

### Must Answer Before Phase 2

| # | Question | Why Critical |
|---|---------|-------------|
| 6 | How do conditional flows type-check? (`ok -> X; err -> Y`) | Core flow control |
| 7 | How are cycles in composition graphs handled? | Many real patterns have cycles |
| 8 | How does debugging work? Source maps from Rust back to Novel? | Usability requirement |
| 9 | License composition rules? MIT + GPL = ? | Legal requirement for generated code |
| 10 | How are Nom scores computed from source? | Automated extraction depends on this |

### Can Defer to Phase 3+

| # | Question | Notes |
|---|---------|-------|
| 11-15 | Tonal modifiers, temporal markers, effect valence, constraint solver, intent resolution | Innovative but not load-bearing |
| 16-20 | Multilingual aliases, locale parsing, cultural governance, alias packs, mixed-script safety | Important but post-MVP |
| 21-25 | Self-hosting, distributed Nom resolution, graph debugger, formal verification, AI integration | Long-term research |

---

## Part IV: The Build Plan

### Phase 0: Foundation (Weeks 1-4) -- COMPLETE

**Goal:** Freeze the semantic kernel. Prove the concept compiles.

**Status:** Done. The compiler is built (10 crates, 42 tests passing).
`nom build auth.nom` produces an 834KB native binary from 9 lines of .nom.
The parser handles all 10 classifiers plus graph and agent primitives.

```
Delivered:
├── nom-compiler/             # Rust compiler (10 crates, 42 tests)
│   ├── nom-ast/              # AST types: 10 classifiers, all statement types
│   ├── nom-lexer/            # Tokenizer: 42 keywords, operators, literals
│   ├── nom-parser/           # Recursive-descent parser: full grammar
│   ├── nom-resolver/         # Name resolution: .nom -> dictionary lookup
│   ├── nom-verifier/         # Contract verification
│   ├── nom-codegen/          # Rust code generation
│   ├── nom-planner/          # Build planning
│   ├── nom-security/         # Security analysis
│   ├── nom-diagnostics/      # Error reporting
│   └── nom-cli/              # CLI: nom build, nom check
│
├── nomdict (SQLite)          # 10M+ .nomtu dictionary entries
│
├── examples/                 # .nom programs that compile
│   ├── hello.nom             # Minimal: flow with string literal
│   ├── auth.nom              # System with 3 needs + flow + constraints
│   ├── webapi.nom            # HTTP service with branching
│   ├── custom_word.nom       # nom escape hatch with inline Rust
│   └── test_auth.nom         # Test specification with given/when/then
│
└── SYNTAX.md                 # Formal syntax reference
```

**Decisions locked:**
- Contract language: simple typed predicates (not full logic)
  ```
  contract { in: bytes, out: hash(bytes), effects: [cpu], pre: in.len > 0 }
  ```
- Tie-breaking: highest composite score -> most recent extraction -> lexicographic
- Dictionary format: SQLite (nomdict) with 10M+ entries
- Escape hatch: inline Rust blocks inside `nom` declarations
- Parser: hand-written recursive descent (not tree-sitter) for full control

### Phase 1: Parser + Resolver (Weeks 5-12)
**Goal:** Parse .novel files → resolve to Nom references → emit verification report.

```
novelos/                        # The compiler, written in Rust
├── crates/
│   ├── novel-parse/            # tree-sitter parser → AST
│   │   ├── grammar.js          # tree-sitter grammar definition
│   │   └── src/lib.rs          # AST types + parse API
│   │
│   ├── novel-resolve/          # AST → resolved composition graph
│   │   ├── src/dictionary.rs   # Load + query Nom dictionary
│   │   ├── src/resolve.rs      # Name resolution: string → NomID
│   │   └── src/select.rs       # Score-based selection with constraints
│   │
│   ├── novel-verify/           # Contract verification
│   │   ├── src/contracts.rs    # Contract compatibility checking
│   │   ├── src/effects.rs      # Effect propagation (set union)
│   │   └── src/report.rs       # Verification report generation
│   │
│   └── novel-cli/              # Command-line interface
│       └── src/main.rs         # novel check, novel verify, novel report
│
├── dictionary/                 # Nom dictionary (expanded to ~500 Noms)
├── examples/                   # More example programs
└── tests/                      # Integration tests
```

**Milestone:** `novel check auth.novel` parses and reports:
```
Composition: auth_service
  Noms: 4 selected (argon2_hash, redis_session, token_bucket, ring_jwt)
  Contracts: ALL COMPATIBLE ✓
  Effects: [network, database, cpu_intensive]
  Scores: security=0.94, performance=0.89, reliability=0.91
  Warnings: session_store latency may exceed 50ms constraint
```

### Phase 2: Code Generation (Weeks 13-24)
**Goal:** Generate Rust from verified compositions. Compile to binary.

```
Add to novelos/:
├── crates/
│   ├── novel-analyze/          # Infer memory + concurrency strategy
│   │   ├── src/ownership.rs    # Graph topology → move/share/lock decisions
│   │   ├── src/concurrency.rs  # DAG analysis → tokio/rayon/sequential
│   │   └── src/effects.rs      # Effect tracking for runtime assertions
│   │
│   ├── novel-generate/         # Emit Rust source code
│   │   ├── src/rust_emit.rs    # Nom implementations → Rust functions
│   │   ├── src/flow_emit.rs    # Flow graphs → async/sync Rust code
│   │   ├── src/system_emit.rs  # System compositions → Rust modules
│   │   └── src/escape.rs       # Inline `nom` Rust blocks → passthrough
│   │
│   └── novel-build/            # Orchestrate Rust compilation
│       └── src/cargo.rs        # Generate Cargo.toml, invoke rustc
```

**Milestone:** `novel build auth.novel` produces a working binary.
The `auth_service` accepts HTTP requests, rate-limits, authenticates,
manages sessions — all from 30 lines of Novel code.

### Phase 3: Tooling + Dictionary Scale (Weeks 25-40)
**Goal:** Make Novel usable by others. Scale dictionary to 10K+ Noms.

```
Deliverables:
├── novel-lsp/                  # Language Server Protocol
│   ├── completions             # Nom suggestions by kind + score
│   ├── diagnostics             # Contract violations, effect warnings
│   ├── hover                   # Nom details: contract, score, provenance
│   └── go-to-definition        # Navigate to Nom source / dictionary entry
│
├── novel-fmt/                  # Code formatter (canonical style)
│
├── novel-extract/              # Automated Nom extraction from repos
│   ├── src/parse.rs            # Parse Rust/Python/Go/TS source
│   ├── src/contract_infer.rs   # Infer contracts from types + tests
│   ├── src/score.rs            # Compute scores from analysis
│   └── src/emit.rs             # Emit Nom dictionary entries
│
├── novel-report/               # Glass box report generator
│   ├── html/                   # HTML report template
│   ├── terminal/               # Terminal report (colored ASCII)
│   └── json/                   # Machine-readable report
│
└── vscode-novel/               # VS Code extension
    ├── syntax highlighting
    ├── LSP client
    └── composition graph viewer
```

**Milestone:** A developer can `npm install -g novel`, write a .novel file
in VS Code with autocomplete, and produce a working binary with a glass box report.

### Phase 4: Advanced Features (Weeks 41-60)
**Goal:** Agent system, graph primitives, Vietnamese OSS integration.

- `agent` classifier with capabilities, supervision, message passing
- `graph` classifier with native node/edge/query primitives
- Vietnamese NL preprocessing via underthesea integration
- Evaluation harness via MELT
- Dictionary scale to 100K+ Noms with automated extraction
- Multilingual alias packs (English canonical, Vietnamese, Chinese)

### Phase 5: Maturity (Weeks 61+)
**Goal:** Self-hosting, community, ecosystem.

- Self-hosting: write parts of Novelos in Novel
- Community dictionary: open contribution + curation process
- Formal verification research for contract properties
- Additional backends: WASM, direct LLVM (skip Rust)
- Intent resolution: fine-tuned model for NL → Nom concepts

---

## Part V: The Vietnamese OSS Integration Plan

From .analysis/oss-vietnamese/:

| Tool | What It Does | Novel Integration |
|------|-------------|------------------|
| **underthesea** | Vietnamese tokenization, NER, POS, normalization | Phase 3: preprocessing for Vietnamese alias packs, text normalization in editor |
| **MELT** | Multi-language LLM evaluation harness | Phase 3: benchmark Novel's NL understanding against Vietnamese baselines |
| **PhoGPT** | Vietnamese-native 3.7B/7B LLM | Phase 5: baseline for intent resolution model, Vietnamese-first |
| **awsome-vietnamese-nlp** | Resource aggregation | Reference: discovery map for datasets and tools |
| **VnCoreNLP** | Classical NLP (older, Java-based) | Reference only: underthesea is the modern successor |

**Integration priority:** underthesea first (Python, well-maintained, immediately useful
for text normalization and alias resolution).

---

## Part VI: What Makes Novel Different — The One-Page Summary

```
EVERY OTHER LANGUAGE:
    Human writes syntax → compiler infers meaning → generates machine code
    Human must verify correctness → human must debug → human must maintain

NOVEL:
    Human declares intent → engine resolves meaning FROM DICTIONARY → composes verified code
    Contracts verify automatically → glass box shows everything → dictionary evolves

THE DIFFERENCE:
    Other languages start with SYNTAX and hope meaning emerges.
    Novel starts with MEANING and lets syntax be the surface.

    Other languages generate from STATISTICAL PATTERNS (may hallucinate).
    Novel composes from GROUND TRUTH (dictionary of proven code).

    Other languages require EXPERTISE to verify output.
    Novel provides TRANSPARENCY anyone can audit.

WHY VIETNAMESE:
    Vietnamese is the most information-dense language on Earth (8.0 bits/syllable).
    It resolves massive ambiguity through 4-layer cascading filters at O(1) cost.
    It compresses meaning with zero grammatical overhead.
    Novel applies these principles to code:
        Classifiers = O(1) disambiguation
        Contracts = zero-ambiguity composition
        Minimal syntax = maximum meaning per token

THE NAMING:
    Chữ Nôm (字喃) = complex characters carrying meaning → Nom atoms
    Quốc Ngữ replaced Nôm by making the same meanings accessible → Novel
    Novelos = the engine that resolves Novel against Nom

    Phần mềm là ngôn ngữ. Nom là từ điển. Novel là cách bạn nói.
    Software is language. Nom is the dictionary. Novel is how you speak it.
```

---

## Appendix: File Map

```
Novel/
├── BLUEPRINT.md                    ← YOU ARE HERE
├── LICENSE
├── README.md
├── novel                           # Core language description
├── nom                             # Core dictionary description
│
├── research/
│   ├── README.md                   # Research index
│   ├── 01-world-language-survey.md # 47 failures across 40+ languages
│   ├── 02-vietnamese-grammar...md  # 13 Vietnamese→Novel grammar mappings
│   ├── 03-chu-nom-to-nom...md     # CJK encoding → NomID architecture
│   ├── 04-novel-language-spec.md  # Draft v0.1 syntax specification
│   ├── 05-beyond-transformers.md  # Transformer critique + Vietnamese efficiency
│   ├── 06-blueprint-for...md      # Engineering implementation plan
│   ├── 07-vietnamese-flex...md    # Anchored-flexible syntax design
│   ├── 08-vietnamese-oss...md     # OSS ecosystem survey
│   └── 09-vietnamese-lingua...md  # Multilingual architecture
│
├── .analysis/
│   └── oss-vietnamese/             # Cloned Vietnamese NLP/LLM repos
│       ├── awsome-vietnamese-nlp/  # Resource aggregation
│       ├── melt/                   # Evaluation harness
│       ├── PhoGPT/                 # Vietnamese LLM
│       ├── ToRoLaMa/               # Vietnamese chat model
│       ├── underthesea/            # Vietnamese NLP toolkit
│       └── villm-eval/             # Earlier eval framework
│
└── .omx/                           # Session execution logs
```

---

## Next Step

**Phase 0 is complete.** Phase 1 is in progress. Current focus:

1. Semantic resolution -- mapping .nom references to nomdict entries
2. Contract verification -- checking type compatibility across flow edges
3. Expanding the example corpus beyond the initial 5 programs
4. See [Plan.md](Plan.md) for the full language development roadmap
