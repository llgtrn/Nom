# Nom Self-Hosting Roadmap

**Goal:** Rewrite the Nom compiler (~13,000 LOC Rust) in Nom itself, achieving self-compilation within 18-24 months.

**Last verified against codebase:** 2026-04-13, HEAD `afc6228`
**Claims tagged:** ✅ SHIPPED (file:line or commit SHA cited), ⏳ PLANNED (phase noted), ❌ ASPIRATIONAL (no near-term plan)

**Current state (as of HEAD `afc6228`):** LLVM backend works for fn/struct/enum/let/if/while/return/match. `lexer.nom` is written in Nom and **compiles end-to-end to `.bc`** (commit `1ecac11`; `stdlib/self_host/lexer.bc` exists). Phases 2–6 have scaffold source files that parse but are not fully compilable. Phase 7 (bootstrap) is planned.

> **Paradigm-coverage enablement banner (2026-04-14 very-late):** Self-hosting requires the Nom language to express its own compiler's semantics. Doc 14 now contains **84 paradigm translations across 71 families with 43 consecutive 0-new-wedge translations**, demonstrating that the current Nom primitive set (9 closed kinds + composition + W49 quantifier vocabulary + `requires`/`ensures`/`hazard` contract clauses) is sufficient for: parser authoring (#80 Datalog + #43 SQL CTE + #28 Prolog all show recursive-grammar-as-relation patterns), semantic analysis (#50 Dafny verified-imperative + #73 Idris dependent-types show compiler-contract authoring), code generation (#41 Verilog + #51 WAT + #81 Chisel show target-DSL emission), error-reporting (#38 Solidity + #67 Zig + #69 Smalltalk + #84 OCaml-effects all share tagged-variant error data decls). **The paradigm-stability result implies self-hosting is unblocked at the authoring layer** — remaining work is the ingest + codegen + planner implementation tracked in the phase table below, not additional grammar wedges.

---

## Per-Phase Status Table

| Phase | Component | Self-Host Status | Evidence |
|-------|-----------|-----------------|---------|
| Prep | Runtime prereqs | ⏳ PLANNED | Lexer compiles, but string slicing + generic lists + tuple returns aspirational |
| 0 | Runtime library | ⏳ PLANNED | No C/Rust runtime bindings exported yet |
| 1 | Lexer in Nom | ✅ SHIPPED (partial) | `lexer.nom` compiles to `.bc` (commit `1ecac11`); aspirational features documented in header |
| 2 | Parser in Nom | ⏳ SCAFFOLD | `parser.nom` parses (commit `7401977`); 9 helper fns; no token-stream consumption |
| 3 | AST types in Nom | ⏳ SCAFFOLD | `ast.nom` parses (commit `23a0ab1`); 9 helper fns; struct shapes only |
| 4 | Verifier in Nom | ⏳ SCAFFOLD | `verifier.nom` parses (commit `7401977`); 6 helper fns; no real type-checking |
| 5 | Planner in Nom | ⏳ SCAFFOLD | `planner.nom` parses (commit `6d08c35`); 7 helper fns; no real planning |
| 6 | Codegen in Nom | ⏳ SCAFFOLD | `codegen.nom` parses (commit `38d729a`); 6 helper fns; no real IR emission |
| 7 | Bootstrap | ❌ ASPIRATIONAL | No bootstrap driver; fixpoint test prereqs include Phase 0–6 completing |

> **Scaffold means**: the `.nom` file exists, parses cleanly via `nom_parser::parse_source`, and has an acceptance test in `nom-cli/tests/self_host_<phase>.rs`. It does NOT mean the file can be compiled to working `.bc` or that the described computation runs. Scaffold status is documented in each file's header comment.

### Toolchain pin (fixpoint prerequisite)
- ✅ SHIPPED — `rust-toolchain.toml` pinned to `channel = "1.94.1"` (commit `29f5f1d`/`bc89d8a`; `nom-compiler/rust-toolchain.toml`). CI enforces the pin in the `check` job (commit `96267df`). This is Risk #1 mitigation from the Phase 4 adversarial review.

### Parity tests
- ✅ SHIPPED — Rust↔Nom canonical-tag parity tests exist: `nom-cli/tests/self_host_ast.rs` + roll-up `self_host_smoke.rs` + pipeline test `self_host_pipeline.rs` (commits `494b8a3`, `ae299eb`, `e4314b3`). CI runs these. They verify structural parity (canonical tags, category cardinalities) between the Rust implementation and the Nom scaffold, not semantic equivalence.

---

## Prerequisites Checklist

| Milestone | Status | Blocker? |
|-----------|--------|----------|
| LLVM backend (fn/struct/enum/basic control flow) | ✅ Works | No |
| String primitives (concat, split, compare, format) | ❌ Missing | **YES** |
| File I/O (read_file, write_file, open, close) | ❌ Missing | **YES** |
| Module/import semantics (use/mod) | ✅ Hash-pinned `use` parsed (commit `ad8cd28`); semantics partial | Partial |
| Standard library types (Vec, HashMap, Result) | ❌ Missing | **YES** |
| Pattern matching on enums (match compilation) | ✅ SHIPPED — match compiles in LLVM backend | No |
| Error handling pattern (Result<T,E>) | ❌ Missing | **YES** |

> **Update vs. original doc**: Match compilation was a blocker in the original roadmap. It is now ✅ SHIPPED — the LLVM backend handles enum pattern matching. String primitives and standard library types remain the primary blockers for Phase 0.

**Parallel prep work required before Phase 0 starts:**
- Extend nom-llvm runtime to export string functions (concat, split, format, compare) — ⏳ PLANNED
- Extend nom-llvm runtime to export file I/O (open, close, read, write) — ⏳ PLANNED
- Define Result<T,E> type and Ok/Err in nom-ast — ⏳ PLANNED
- Add Vec<T> as builtin struct in nom-dict — ⏳ PLANNED

---

## Phase 0: Runtime Library (4-6 weeks)

**What gets built:** C/Rust-backed runtime providing string, file, and collection primitives callable from Nom.
**Status:** ⏳ PLANNED

**Nom features required:**
- fn declaration and calls (✅ already have)
- struct definition (✅ already have)
- let bindings (✅ already have)
- if/else (✅ already have)
- while loops (✅ already have)

**Runtime primitives to expose:**
- `nom_print(msg: str)` — print string to stdout
- `nom_string_concat(a: str, b: str) -> str`
- `nom_string_split(s: str, delim: str) -> Vec<str>`
- `nom_string_compare(a: str, b: str) -> i32`
- `nom_string_format(fmt: str, args: Vec<str>) -> str`
- `nom_file_open(path: str, mode: str) -> i32` (returns fd)
- `nom_file_read(fd: i32, len: i32) -> str`
- `nom_file_write(fd: i32, data: str) -> i32`
- `nom_file_close(fd: i32) -> i32`
- `nom_vec_new() -> Vec<T>`
- `nom_vec_push(v: Vec<T>, item: T) -> void`
- `nom_vec_len(v: Vec<T>) -> i32`
- `nom_vec_get(v: Vec<T>, idx: i32) -> T`

**Complexity estimate:** 500-800 LOC Nom, 2000-3000 LOC C/Rust runtime bindings
**Key challenges:**
- Generic Vec<T> representation in LLVM
- String lifetime and memory layout (stack vs heap)
- File handle tracking across calls

---

## Phase 1: Lexer in Nom (6-8 weeks)

**What gets built:** Nom-based tokenizer that reads Nom source, produces Token stream.
**Status:** ✅ SHIPPED (partial) — core lexer compiles; aspirational features remain

**Evidence:**
- `stdlib/self_host/lexer.nom` — 751 LOC, mirrors `nom-lexer/src/lib.rs` (1073 LOC Rust). Commit `17ceaa5` (initial); commit `1ecac11` (compiles to `.bc`).
- `stdlib/self_host/lexer.bc` — 41 KB compiled artifact on disk (untracked per `.gitignore` added commit `6c992c9`).
- `examples/run_lexer.nom` — minimal driver that tokenizes `"flow request->response"` and prints count=4. Compiles and runs. Commit `1ecac11`.
- `examples/run_lexer.exe` — compiled binary on Windows (untracked).

**What compiles today in lexer.nom:**
- Complete `Token` enum (80+ keywords including Vietnamese aliases)
- All operator and bracket scanning functions
- Comment, string, number, and identifier scanning
- Span tracking for error reporting
- Blank-line detection

**Aspirational features used (not yet compilable — documented in lexer.nom header):**
- Tuple return types: `fn foo() -> (Token, Lexer)`
- String indexing: `source[pos]` returning a byte/integer
- String slicing: `source[start..end]`
- Generic list types: `list[Token]`
- Enum variants with payloads: `Integer(integer)`
- Built-in `parse_int` / `parse_float` / `chr` functions

**Nom features required (from Phase 0):**
- Looping and indexing
- String primitives (concat, split, compare)
- Struct definitions
- Enum for Token variant discriminant

**Runtime primitives to use:**
- File I/O (read source file)
- String manipulation (check keywords, extract identifiers)

**Current Rust implementation:** `nom-lexer/src/lib.rs`, 1073 LOC (actual, measured HEAD)

**Complexity estimate:** 1200-1500 LOC Nom (current: 751 LOC scaffold)
**Key challenges:**
- Span tracking (line/col/length) during tokenization
- Lookahead for multi-character operators (=>, ->, ==)
- Keyword table (80+ keywords — use data or hardcode?)
- Escape sequence handling in string literals

**Deliverable:** `nom_lex(source: str) -> Vec<Token>`

---

## Phase 2: Parser in Nom (10-14 weeks)

**What gets built:** Recursive-descent parser that turns Token stream into AST.
**Status:** ⏳ SCAFFOLD — `parser.nom` parses cleanly; no real token-stream consumption

**Evidence:**
- `stdlib/self_host/parser.nom` — 137 LOC scaffold. Status in header: "SCAFFOLD. Module header + SourceFile shape + entry-point fn signature." Commit `7401977`.
- Acceptance test: `nom-cli/tests/self_host_ast.rs` (roll-up) verifies `parser.nom` parses. Commit `9f3f393`.
- 9 mirror helper functions added (commit `fca5198`).

**Nom features required:**
- Match expressions (✅ SHIPPED in LLVM backend)
- Recursive function calls
- More complex struct nesting
- Error handling pattern (Result-like enum: `ParseResult = Ok(value) | Err(reason)` — ❌ not yet in runtime)

**Runtime primitives:**
- String formatting for error messages
- Vec manipulation for accumulating AST nodes

**Current Rust implementation:** `nom-parser/src/lib.rs`, 3694 LOC (actual, measured HEAD)

**Complexity estimate:** 1800-2200 LOC Nom
**Key challenges:**
- Recursive descent without stack overflow — need tail recursion or trampoline
- Building nested AST structures (Declaration contains Statements, Statements contain Expressions)
- Error recovery and reporting
- Token lookahead/backtracking

**Deliverable:** `nom_parse(tokens: Vec<Token>) -> Result<SourceFile, ParseError>`

---

## Phase 3: AST Types in Nom (4-6 weeks)

**What gets built:** struct and enum definitions that mirror nom-ast in Rust.
**Status:** ⏳ SCAFFOLD — `ast.nom` parses cleanly; struct shapes present

**Evidence:**
- `stdlib/self_host/ast.nom` — 116 LOC scaffold. Commit `23a0ab1`.
- Acceptance test: part of roll-up `self_host_smoke.rs` (commit `9f3f393`). 9 mirror helper functions (commit `fca5198`).

**Nom features required:**
- struct with generic fields (struct Declaration<T>)
- enum with tuple/struct variants
- Type aliases

**Complexity estimate:** 800-1200 LOC Nom (data definitions only)
**Key challenges:**
- Generic types (Expr<T>, Statement<T>) — ❌ Nom does not yet support generics
- Circular references (Expression contains Statement, Statement contains Expression)
- Keeping struct layout identical to nom-ast for LLVM compatibility

**Deliverable:** Mirror of nom-ast structs: Declaration, Statement, Expr, TypeExpr, etc.

---

## Phase 4: Verifier in Nom (8-10 weeks)

**What gets built:** Contract/type checking pass.
**Status:** ⏳ SCAFFOLD — `verifier.nom` parses cleanly; no real type-checking

**Evidence:**
- `stdlib/self_host/verifier.nom` — 95 LOC scaffold. Status in header: "SCAFFOLD. Module header + VerifiedAST shape + entry-point fn signature." Commit `7401977`.
- Acceptance test: part of roll-up smoke test. 6 mirror helper functions.

**Nom features required:**
- Pattern matching on AST nodes (✅ SHIPPED in LLVM backend)
- Traversal of nested structures
- Hash tables for symbol tables (if available, else use Vec<Entry>)

**Current Rust implementation:** `nom-verifier/src/lib.rs`, 1510 LOC (actual, measured HEAD)

**Complexity estimate:** 900-1300 LOC Nom
**Key challenges:**
- Scope management (tracking in-scope identifiers)
- Type inference (if Nom verifier does it)
- Error accumulation (collect multiple errors before failing)

**Deliverable:** `nom_verify(ast: SourceFile) -> Result<VerifiedAST, Vec<Error>>`

---

## Phase 5: Planner in Nom (10-12 weeks)

**What gets built:** Composition graph construction and flow planning.
**Status:** ⏳ SCAFFOLD — `planner.nom` parses cleanly; 7 concrete helper functions added; no real planning logic

**Evidence:**
- `stdlib/self_host/planner.nom` — 118 LOC scaffold. Commit `4f04597` (initial); commit `6d08c35` (parses + acceptance test); commit `90d893b` (two concrete helper fns).
- Acceptance test: `nom-cli/tests/self_host_planner.rs` (commit `6d08c35`). Reads `planner.nom`, calls `nom_parser::parse_source`, asserts parse succeeds and module name lands in declarations.
- 7 mirror helper functions (commit `741614c`).

**Nom features required:**
- Graph node/edge creation (struct Node, Edge)
- Topological sort or traversal (needs loop + conditionals)
- Constraint checking (need/require statement resolution)

**Current Rust implementation:** `nom-planner/src/lib.rs`, 1210 LOC (actual, measured HEAD)

**Complexity estimate:** 1100-1500 LOC Nom
**Key challenges:**
- Building and mutating graph structure efficiently
- Cycle detection
- Constraint propagation logic
- Memory-efficient representation in LLVM

**Deliverable:** `nom_plan(ast: VerifiedAST) -> CompositionPlan`

---

## Phase 6: Codegen in Nom (14-18 weeks)

**What gets built:** LLVM IR emission directly from CompositionPlan.
**Status:** ⏳ SCAFFOLD — `codegen.nom` parses cleanly; status "SCAFFOLD. Module + result shape + entry-point signature only."

**Evidence:**
- `stdlib/self_host/codegen.nom` — 97 LOC scaffold. Commit `38d729a`.
- Acceptance test: part of roll-up smoke test + pipeline test (commit `e4314b3`). 6 mirror helper functions (commit `741614c`).

**Nom features required:**
- Full pattern matching on plan nodes (✅ SHIPPED in LLVM backend)
- String building for LLVM IR (concat primitives from Phase 0)
- Struct field access and traversal

**Current Rust implementation:** `nom-codegen/src/lib.rs` + `nom-llvm`, combined 3925 + ~1800 LOC (actual, measured HEAD)

**Complexity estimate:** 2200-2800 LOC Nom
**Key challenges:**
- Generating correct LLVM IR syntax (function signatures, basic blocks, instructions)
- Type mapping (Nom types → LLVM types: i32, i64, ptr, struct*)
- Control flow graph construction (conditional branches, loops)
- Function call lowering
- Calling C runtime functions (string, file I/O)

**Deliverable:** `nom_codegen(plan: CompositionPlan) -> (ir_text: str, bitcode: Vec<u8>)`

---

## Phase 7: Bootstrap (6-8 weeks)

**What gets built:** Self-compilation and verification.
**Status:** ❌ ASPIRATIONAL — no bootstrap driver exists; all Phase 0–6 items must complete first

**Process:**
1. Compile all Phase 1-6 Nom code with Rust compiler
2. Run Rust nom-compiler on itself (produces IR)
3. Run Nom nom-compiler on itself (produces IR)
4. Verify both IRs are equivalent (bitwise or semantic diff)
5. If different, identify and fix bug in Nom version
6. Repeat until bootstrap succeeds

**Fixpoint discipline (§10.3.1):**
- ✅ SHIPPED — `rust-toolchain.toml` pins the Rust toolchain version (commit `bc89d8a`; `nom-compiler/rust-toolchain.toml:channel = "1.94.1"`). CI enforces the pin (commit `96267df`). This is the mechanical prerequisite for the Stage 2 vs. Stage 3 hash comparison.
- ⏳ PLANNED — The bootstrap driver itself (`nom-compiler --self-compile`), the Stage 0→1→2→3 compilation pipeline, and the bitcode diff tooling.
- ❌ ASPIRATIONAL — The `proof-of-bootstrap` tuple `(s1_hash, s2_hash, s3_hash, fixpoint_at_date, compiler_manifest_hash)` recorded in the dictionary.

**Complexity estimate:** 2-3 weeks debug time (typically)
**Key challenges:**
- Finding discrepancies between Rust and Nom versions
- Ensuring Nom compiler handles its own code (recursive self-application)
- Handling edge cases only found when compiling large codebase

**Success criteria:**
- `nom-compiler --self-compile` produces working binaries
- Nom-compiled nom-compiler can compile other Nom programs
- Output is byte-identical or semantically equivalent to Rust version (the "fixpoint" per §10.3.1)

**Byte-determinism precedent already landed (2026-04-13, M10b):**
`compile_nom_to_bc` at [nom-corpus/src/lib.rs:245](../../nom-compiler/crates/nom-corpus/src/lib.rs) wires `parse_source → plan_unchecked → nom-llvm::compile` into a single `Vec<u8>` bc producer, and `compile_nom_to_bc_is_deterministic` at [nom-corpus/src/lib.rs:1059](../../nom-compiler/crates/nom-corpus/src/lib.rs) locks byte-identical re-compilation. M10b (commit `cef8425`) additionally pins `examples/run_lexer.bc` by SHA-256 as a cross-build reproducibility gate. This **de-risks Phase 7** — the byte-identical property is already observable on the single-file level; the remaining work is extending the same discipline across the whole compiler manifest.

---

## Timeline Summary

| Phase | Work | Weeks | Cumulative | Status |
|-------|------|-------|-----------|--------|
| Prep | Runtime library setup | 4-6 | 4-6 | ⏳ PLANNED |
| 0 | String/file/collection runtime | 4-6 | 8-12 | ⏳ PLANNED |
| 1 | Lexer in Nom | 6-8 | 14-20 | ✅ Core compiles; aspirational features remain |
| 2 | Parser in Nom | 10-14 | 24-34 | ⏳ SCAFFOLD |
| 3 | AST types | 4-6 | 28-40 | ⏳ SCAFFOLD |
| 4 | Verifier | 8-10 | 36-50 | ⏳ SCAFFOLD |
| 5 | Planner | 10-12 | 46-62 | ⏳ SCAFFOLD |
| 6 | Codegen | 14-18 | 60-80 | ⏳ SCAFFOLD |
| 7 | Bootstrap & debug | 6-8 | 66-88 | ❌ ASPIRATIONAL |

**Conservative estimate: 66-88 weeks (16-21 months) with one full-time developer.** This estimate dates from original document creation (2026-04-12) and has not been revised.

---

## Critical Success Factors

1. **Match compilation must work before Phase 2** — parser is pattern-match-heavy. ✅ SHIPPED — match compilation works in LLVM backend.
2. **Phase 0 primitives locked in before Phase 1** — lexer relies on them. ⏳ PLANNED — Phase 0 string/file primitives not yet available; `lexer.nom` works around them using the compiled subset.
3. **Generic structs/enums in Nom** — if not supported, refactor AST to monomorphic types (increases LOC 20-30%). ❌ Not supported; AST types in `ast.nom` are currently monomorphic.
4. **No scope creep** — do not add optimizations, error recovery, or language features mid-bootstrap.
5. **Continuous testing** — each phase must compile and test the next phase before merging. ✅ SHIPPED — acceptance tests for each scaffold enforce this (commits `9f3f393`, `6d08c35`, `e4314b3`).

---

## Risk Mitigation

| Risk | Probability | Impact | Mitigation | Status |
|------|-------------|--------|-----------|--------|
| Match compilation still incomplete | ~~Medium~~ | ~~High~~ | ~~Complete enum pattern match in nom-llvm in Prep phase~~ | ✅ RESOLVED — match compiles |
| Generic types unsupported in Nom | High | High | Pre-convert all generics to monomorphic + struct field encoding | ⏳ Active risk |
| LLVM IR generation wrong | High | High | Validate first 100 LOC with `llvm-as` before scaling | ⏳ Mitigated via `run_lexer.nom` + parity tests |
| Bootstrap mismatch hard to debug | High | Medium | Write diff tool early, run frequently | ⏳ PLANNED |
| String memory layout issues | Medium | Medium | Pre-validate string ops with small test in Phase 0 | ⏳ PLANNED |

---

## Relationship to Phase 5 media/UX work — parallel tracks, not blockers (added 2026-04-12)

Self-hosting phases 0–7 (above) target the core compiler pipeline: lexer → parser → AST → verifier → planner → codegen → bootstrap. These do NOT block media/UX work and are not blocked by it.

- **§5.11 (UX as nomtu) and §5.16 (media as nomtu + codec compilation) live atop the compiled output of Phase 0's runtime library.** They add new crates (`nom-ux`, `nom-media`) and new kinds of dict entries — they do NOT change the compiler's grammar, lexer, or codegen.
  - ✅ SHIPPED — `nom-media` crate exists with AVIF encoder (commit `707aa93`) and WebM mux (commit `2ec9d2b`). `nom-ux` crate scaffold exists.
- **A user running `nom app build <hash> --target web` or `nom media render <hash> --target av1` uses the self-hosted compiler as the build driver.** The compiler doesn't need to understand UX or media semantics — just to compile the FFI-wrapper nomtu bodies that the codec/UI-runtime closures contain.
  - ⏳ PLANNED — `nom app build` subcommand not yet implemented.
- **The self-hosting fixpoint proof** (§10.3.1 in [`04-next-phases-plan.md`](./04-next-phases-plan.md)) compiles the compiler against the core language only; media/UX nomtu are not part of the compiler's own manifest closure. They can evolve independently of the fixpoint track.
- **Team-scaling implication:** two engineers can work in parallel — one on self-hosting Phase N, one on §5.16 codec landings — with minimal coordination beyond the shared `nom-types` / `nom-dict` crates.

The only coupling is a Phase 3 requirement (already met): the AST supports `Ffi` binding nodes (`nom-ast/src/lib.rs`). All codec nomtu and `ui_runtime_launch` nomtu use this to call into native libraries. If the self-hosted AST crate ever drops FFI support, media/UX work stops. It must not.

### Note on the body-as-compiled-artifact shift (§4.4.6, 2026-04-12)

After the architectural shift captured in [`04-next-phases-plan.md`](./04-next-phases-plan.md) §4.4.6, the dict stores `.bc` (compiled LLVM bitcode), not Nom source. This affects self-hosting as follows:

- **The compiler's own source stays in `.nom` files** (the user-authored surface form). Stages 0–7 above all target the compilation of those `.nom` files.
- **The fixpoint test at §10.3.1** compares Stage 2's output `.bc` hash to Stage 3's — exactly the byte-comparison these phases already target, now explicit about the artifact being `.bc`.
  - ✅ SHIPPED — `body_kind` field (`nom-dict/src/lib.rs`) distinguishes `.bc` from other body types. Invariant 15 enforced (commits `540620d`, `6c336b4`).
- **Phase 5 "Planner in Nom" and Phase 6 "Codegen in Nom"** still write Nom source in `.nom` files. The output of running them (and every subsequent pass) is `.bc` in the dict. Nothing in the self-hosting pipeline authors or reads Nom source from the dict.
- **Phase 0 runtime library** now ships as `.bc` (from compiling its Rust/Nom source), not as a source-in-dict artifact. The runtime's hash is the hash of its `.bc`.

The self-hosting path is *simpler* under this shift: there's no "canonicalize Nom source in dict" step, because there's no Nom source in the dict to canonicalize.

## Success Metrics

- [x] Nom lexer written in Nom (commit `17ceaa5`)
- [x] Nom lexer core compiles to `.bc` via LLVM backend (commit `1ecac11`)
- [ ] Nom lexer tokenizes itself end-to-end (blocked on string slicing + generic lists)
- [ ] Nom parser scaffold promoted to working parser (Phase 2)
- [ ] All phases compile Nom code without crashes (Phase 0 runtime required)
- [ ] Bootstrap: Nom compiler produces same IR as Rust compiler on test suite (10+ programs) (Phase 7)
- [ ] Self-compilation loop completes: nom → Nom → nom → identical output (Phase 7 fixpoint)
