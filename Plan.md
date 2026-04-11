# Nom Language Development Roadmap

Development plan for the Nom programming language.

---

## Architectural Decision: LLVM Bitcode (.bc) as Universal Artifact

Every .nomtu in the dictionary gets pre-compiled to LLVM bitcode (.bc).
The compiler generates ONLY glue IR and links against pre-compiled .bc files.

```
WHY .bc:
  C code      -> clang    -> .bc
  Rust code   -> rustc    -> .bc     ALL produce the SAME format
  Go code     -> gollvm   -> .bc
  C++ code    -> clang++  -> .bc
  Python      -> translate to C/Rust first -> .bc

  Then:
  .nom -> LLVM IR glue -> llvm-link all .bc files -> opt -> llc -> binary

  Language-neutral. All 10M nomtu usable, not just Rust.
  Full LTO across the entire composition.
  Sub-second builds (only compile glue, link pre-compiled .bc).
```

### .nomtu Format Evolution

```
Phase 1 (now):    nomtu = { word, body: "source text", language }
                  Compiler: paste body into Rust -> cargo build
                  
Phase 2 (next):   nomtu = { word, body: "source text", artifact: "hash.bc" }
                  Compiler: emit LLVM IR glue -> llvm-link with .bc -> binary
                  Pre-compile: run once per nomtu -> store .bc in registry

Phase 3 (mature): nomtu = { word, artifact: "hash.bc" }
                  nom.dev serves .bc files directly
                  Source text = provenance only, not needed for compilation
```

### Dependency Bundling

Each .nomtu .bc bundles its ENTIRE dependency tree:

```
argon2.nomtu:
  source: fn argon2_hash(data: &[u8]) -> Vec<u8> { ... }
  deps: rand_core, password-hash, base64ct
  artifact: argon2-bundle.bc  (argon2 + all deps, one file)
  contract: in=bytes, out=hash, effects=[cpu]
  
nom build links against argon2-bundle.bc
  -> no dependency resolution at build time
  -> no Cargo.toml needed
  -> no crates.io download
```

### .bc vs Alternatives

| | .rlib (Rust) | .bc (LLVM) | .wasm | .o (object) |
|--|-------------|-----------|-------|-------------|
| Language lock-in | Rust only | Any | Any | Any |
| Version coupling | rustc version | Stable IR | Stable | None |
| Cross-language link | No | Yes | Via host | Platform-specific |
| LTO optimization | Limited | Full | Limited | None |
| Our 10M nomtu | 992K usable | All 10M | All 10M | All 10M |

---

## Phase A: Core Compiler -- DONE

- 10 compiler crates, 42 tests passing
- `nom build auth.nom` produces 834KB native binary from 9 lines
- Semantic resolution with describe-field fallback
- Smart domain mapping (26K nomtu with real code bodies)
- Enrichment pass pulls real code from dictionary into generated output
- Parser handles all 10 classifiers, 42 keywords, graph/agent primitives

---

## Phase B: .bc Pre-Compiler -- NEXT

Build the infrastructure to pre-compile .nomtu bodies into LLVM bitcode.

### Step 1: Score and filter dictionary (1-2 days)
- [ ] Score all 26K nomtu with bodies (body length, signature quality, language)
- [ ] Filter: keep only self-contained functions (< 3 external type refs)
- [ ] Rank: prefer functions with typed signatures matching composable contracts

### Step 2: .bc pre-compilation pipeline (1 week)
- [ ] For each Rust .nomtu body: wrap in minimal crate with `extern "C"` entry
- [ ] Compile: `rustc --emit=llvm-bc` -> .bc file
- [ ] For each C .nomtu body: `clang -emit-llvm -c` -> .bc file
- [ ] Bundle dependencies: link .bc with dep .bc files -> single bundle.bc
- [ ] Store artifact path in nomtu table (new column: `artifact_path`)
- [ ] Add `nom precompile` CLI command

### Step 3: LLVM glue emitter (1 week)
- [ ] Add inkwell (Rust LLVM bindings) to nom-codegen
- [ ] Emit LLVM IR for flow glue: call @hash(input) -> call @store(result) -> return
- [ ] Handle pipeline concurrency in IR (or link against tokio .bc)
- [ ] Error handling: Result type as {i1, ptr} tagged union in LLVM IR
- [ ] Add `nom build --backend=llvm` flag (keep Rust backend as fallback)

### Step 4: Link and optimize (days)
- [ ] llvm-link: glue.bc + hash.bc + store.bc -> combined.bc
- [ ] opt: optimization passes (inlining, dead code, GVN, vectorize)
- [ ] llc: combined.bc -> native object -> system linker -> binary
- [ ] Benchmark: compare .bc build vs cargo build (target: 10x faster)

### Expected Result
```
nom build auth.nom
  -> emit LLVM IR glue (microseconds)
  -> link pre-compiled .bc files (milliseconds)
  -> optimize + compile to native (seconds)
  -> binary

Total: ~5 seconds instead of ~2 minutes
All 10M nomtu usable regardless of source language
```

---

## Phase C: Failure Prevention (47 patterns)

See `research/motivation/01-world-language-survey.md` for the full list.

| Category | Count | Enforced By |
|----------|-------|-------------|
| Memory safety | 9 | Graph topology -> ownership inference at IR level |
| Concurrency | 6 | Flow graph -> parallel/sequential at IR level |
| Type systems | 7 | Contract verification at link time |
| Error handling | 4 | Contract ok/err -> Result in IR |
| Composition | 4 | Three operators only (-> :: +) |
| Effects | 2 | Effect declarations verified at link time |
| Evolution | 4 | Content-addressed .bc, nom.lock pins hashes |
| Security | 6 | Score-gated selection, provenance tracking |
| Build/Tooling | 4 | nom is the entire toolchain |
| Syntax | 6 | 10 keywords, 3 operators, writing-style |

With .bc: many of these become LLVM-level enforcement (noalias for ownership,
branch weights for error paths, function attributes for effects).

---

## Phase D: Assembly-Smooth Performance

With .bc as the artifact format, performance optimization = LLVM optimization:

- Nom fusion: inline adjacent .bc functions (LLVM inlining pass)
- Arena allocation: LLVM alloca with known sizes from graph topology
- noalias everywhere: graph topology proves no aliasing
- Branch hints: contract ok/err -> llvm.expect intrinsic
- SoA layout: LLVM SLP vectorizer on collection flows
- Full LTO: all .bc merged, maximum cross-function optimization

Target benchmarks (from research/motivation/10-how-novel-replaces-everything.md):
- vs GC languages: 2-10x faster
- vs hand-written Rust: within 5-15%
- vs expert assembly: within 10-20%

---

## Phase E: nom.dev Registry

- Serve .bc artifacts (not source text)
- Content-addressed: hash of .bc = identity
- nom.lock pins exact .bc hashes
- Reproducible builds: same .nom + same nom.lock = same binary, always

---

## Milestones

| Milestone | Status |
|-----------|--------|
| Lexer/parser (42 keywords, 10 classifiers) | Done |
| nom build -> native binary (Rust backend) | Done |
| 10M+ nomtu in dictionary with real code bodies | Done |
| Smart domain mapping (26K semantically named nomtu) | Done |
| Dictionary enrichment (real code embedded in output) | Done |
| Score and filter dictionary | Next |
| .bc pre-compilation pipeline | Planned |
| LLVM glue emitter (inkwell) | Planned |
| .bc link + optimize -> binary | Planned |
| 47 failure prevention at IR level | Planned |
| Assembly-smooth benchmarks | Planned |
| nom.dev registry serving .bc | Planned |

---

## Evidence and Research

| Document | What It Informs |
|----------|-----------------|
| `research/motivation/01-world-language-survey.md` | 47 failure patterns to prevent |
| `research/motivation/10-how-novel-replaces-everything.md` | Assembly-smooth compilation architecture |
| `research/motivation/16-competitive-analysis-and-roadmap.md` | Competitive position and survival strategy |
| `research/motivation/02-vietnamese-grammar-to-novel-syntax.md` | Classifier design, composition operators |
| `research/deferred/05-beyond-transformers.md` | Why direct compilation matters |
| `SYNTAX.md` | Formal syntax reference |
| `BLUEPRINT.md` | Full build plan |
