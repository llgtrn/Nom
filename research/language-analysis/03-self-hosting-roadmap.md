# Nom Self-Hosting Roadmap

**Goal:** Rewrite the Nom compiler (~13,000 LOC Rust) in Nom itself, achieving self-compilation within 18-24 months.

**Current state:** LLVM backend works for fn/struct/enum/let/if/while/return. Lexer, parser, verifier, and planner are Rust. Module/import system parsed but no semantics. Match compilation returns Unsupported.

---

## Prerequisites Checklist

| Milestone | Status | Blocker? |
|-----------|--------|----------|
| LLVM backend (fn/struct/enum/basic control flow) | ✅ Works | No |
| String primitives (concat, split, compare, format) | ❌ Missing | **YES** |
| File I/O (read_file, write_file, open, close) | ❌ Missing | **YES** |
| Module/import semantics (use/mod) | ❌ Parsed only | **YES** |
| Standard library types (Vec, HashMap, Result) | ❌ Missing | **YES** |
| Pattern matching on enums (match compilation) | ❌ Unsupported | **YES** |
| Error handling pattern (Result<T,E>) | ❌ Missing | **YES** |

**Parallel prep work required before Phase 0 starts:**
- Extend nom-llvm runtime to export string functions (concat, split, format, compare)
- Extend nom-llvm runtime to export file I/O (open, close, read, write)
- Implement enum pattern matching in LLVM codegen
- Define Result<T,E> type and Ok/Err in nom-ast
- Add Vec<T> as builtin struct in nom-dict

---

## Phase 0: Runtime Library (4-6 weeks)

**What gets built:** C/Rust-backed runtime providing string, file, and collection primitives callable from Nom.

**Nom features required:**
- fn declaration and calls (already have)
- struct definition (already have)
- let bindings
- if/else
- while loops

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

**Nom features required (from Phase 0):**
- Looping and indexing
- String primitives (concat, split, compare)
- Struct definitions
- Enum for Token variant discriminant

**Runtime primitives to use:**
- File I/O (read source file)
- String manipulation (check keywords, extract identifiers)

**Current Rust implementation:** nom-lexer, ~800 LOC

**Complexity estimate:** 1200-1500 LOC Nom  
**Key challenges:**
- Span tracking (line/col/length) during tokenization
- Lookahead for multi-character operators (=>, ->, ==)
- Keyword table (90+ keywords — use data or hardcode?)
- Escape sequence handling in string literals

**Deliverable:** `nom_lex(source: str) -> Vec<Token>`

---

## Phase 2: Parser in Nom (10-14 weeks)

**What gets built:** Recursive-descent parser that turns Token stream into AST.

**Nom features required:**
- Match expressions (critical for pattern matching on Token enum)
- Recursive function calls
- More complex struct nesting
- Error handling pattern (Result-like enum: `ParseResult = Ok(value) | Err(reason)`

**Runtime primitives:**
- String formatting for error messages
- Vec manipulation for accumulating AST nodes

**Current Rust implementation:** nom-parser, ~1200 LOC

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

**Nom features required:**
- struct with generic fields (struct Declaration<T>)
- enum with tuple/struct variants
- Type aliases

**Complexity estimate:** 800-1200 LOC Nom (data definitions only)  
**Key challenges:**
- Generic types (Expr<T>, Statement<T>) — does Nom support generics yet?
- Circular references (Expression contains Statement, Statement contains Expression)
- Keeping struct layout identical to nom-ast for LLVM compatibility

**Deliverable:** Mirror of nom-ast structs: Declaration, Statement, Expr, TypeExpr, etc.

---

## Phase 4: Verifier in Nom (8-10 weeks)

**What gets built:** Contract/type checking pass.

**Nom features required:**
- Pattern matching on AST nodes
- Traversal of nested structures
- Hash tables for symbol tables (if available, else use Vec<Entry>)

**Current Rust implementation:** nom-verifier, ~600 LOC

**Complexity estimate:** 900-1300 LOC Nom  
**Key challenges:**
- Scope management (tracking in-scope identifiers)
- Type inference (if Nom verifier does it)
- Error accumulation (collect multiple errors before failing)

**Deliverable:** `nom_verify(ast: SourceFile) -> Result<VerifiedAST, Vec<Error>>`

---

## Phase 5: Planner in Nom (10-12 weeks)

**What gets built:** Composition graph construction and flow planning.

**Nom features required:**
- Graph node/edge creation (struct Node, Edge)
- Topological sort or traversal (needs loop + conditionals)
- Constraint checking (need/require statement resolution)

**Current Rust implementation:** nom-planner, ~700 LOC

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

**Nom features required:**
- Full pattern matching on plan nodes
- String building for LLVM IR (concat primitives from Phase 0)
- Struct field access and traversal

**Current Rust implementation:** nom-codegen, nom-llvm combined ~1800 LOC

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

**Process:**
1. Compile all Phase 1-6 Nom code with Rust compiler
2. Run Rust nom-compiler on itself (produces IR)
3. Run Nom nom-compiler on itself (produces IR)
4. Verify both IRs are equivalent (bitwise or semantic diff)
5. If different, identify and fix bug in Nom version
6. Repeat until bootstrap succeeds

**Complexity estimate:** 2-3 weeks debug time (typically)  
**Key challenges:**
- Finding discrepancies between Rust and Nom versions
- Ensuring Nom compiler handles its own code (recursive self-application)
- Handling edge cases only found when compiling large codebase

**Success criteria:**
- `nom-compiler --self-compile` produces working binaries
- Nom-compiled nom-compiler can compile other Nom programs
- Output is byte-identical or semantically equivalent to Rust version

---

## Timeline Summary

| Phase | Work | Weeks | Cumulative |
|-------|------|-------|-----------|
| Prep | Runtime library setup | 4-6 | 4-6 |
| 0 | String/file/collection runtime | 4-6 | 8-12 |
| 1 | Lexer in Nom | 6-8 | 14-20 |
| 2 | Parser in Nom | 10-14 | 24-34 |
| 3 | AST types | 4-6 | 28-40 |
| 4 | Verifier | 8-10 | 36-50 |
| 5 | Planner | 10-12 | 46-62 |
| 6 | Codegen | 14-18 | 60-80 |
| 7 | Bootstrap & debug | 6-8 | 66-88 |

**Conservative estimate: 66-88 weeks (16-21 months) with one full-time developer.**

---

## Critical Success Factors

1. **Match compilation must work before Phase 2** — parser is pattern-match-heavy
2. **Phase 0 primitives locked in before Phase 1** — lexer relies on them
3. **Generic structs/enums in Nom** — if not supported, refactor AST to monomorphic types (increases LOC 20-30%)
4. **No scope creep** — do not add optimizations, error recovery, or language features mid-bootstrap
5. **Continuous testing** — each phase must compile and test the next phase before merging

---

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|-----------|
| Match compilation still incomplete | Medium | High | Complete enum pattern match in nom-llvm in Prep phase |
| Generic types unsupported in Nom | Medium | High | Pre-convert all generics to monomorphic + struct field encoding |
| LLVM IR generation wrong | High | High | Validate first 100 LOC with `llvm-as` before scaling |
| Bootstrap mismatch hard to debug | High | Medium | Write diff tool early, run frequently |
| String memory layout issues | Medium | Medium | Pre-validate string ops with small test in Phase 0 |

---

## Relationship to Phase 5 media/UX work — parallel tracks, not blockers (added 2026-04-12)

Self-hosting phases 0–7 (above) target the core compiler pipeline: lexer → parser → AST → verifier → planner → codegen → bootstrap. These do NOT block media/UX work and are not blocked by it.

- **§5.11 (UX as nomtu) and §5.16 (media as nomtu + codec compilation) live atop the compiled output of Phase 0's runtime library.** They add new crates (`nom-ux`, `nom-media`) and new kinds of dict entries — they do NOT change the compiler's grammar, lexer, or codegen.
- **A user running `nom app build <hash> --target web` or `nom media render <hash> --target av1` uses the self-hosted compiler as the build driver.** The compiler doesn't need to understand UX or media semantics — just to compile the FFI-wrapper nomtu bodies that the codec/UI-runtime closures contain.
- **The self-hosting fixpoint proof** (§10.3.1 in [`04-next-phases-plan.md`](./04-next-phases-plan.md)) compiles the compiler against the core language only; media/UX nomtu are not part of the compiler's own manifest closure. They can evolve independently of the fixpoint track.
- **Team-scaling implication:** two engineers can work in parallel — one on self-hosting Phase N, one on §5.16 codec landings — with minimal coordination beyond the shared `nom-types` / `nom-dict` crates.

The only coupling is a Phase 3 requirement (already met): the AST supports `Ffi` binding nodes. All codec nomtu and `ui_runtime_launch` nomtu use this to call into native libraries. If the self-hosted AST crate ever drops FFI support, media/UX work stops. It must not.

### Note on the body-as-compiled-artifact shift (§4.4.6, 2026-04-12)

After the architectural shift captured in [`04-next-phases-plan.md`](./04-next-phases-plan.md) §4.4.6, the dict stores `.bc` (compiled LLVM bitcode), not Nom source. This affects self-hosting as follows:

- **The compiler's own source stays in `.nom` files** (the user-authored surface form). Stages 0–7 above all target the compilation of those `.nom` files.
- **The fixpoint test at §10.3.1** compares Stage 2's output `.bc` hash to Stage 3's — exactly the byte-comparison these phases already target, now explicit about the artifact being `.bc`.
- **Phase 5 "Planner in Nom" and Phase 6 "Codegen in Nom"** still write Nom source in `.nom` files. The output of running them (and every subsequent pass) is `.bc` in the dict. Nothing in the self-hosting pipeline authors or reads Nom source from the dict.
- **Phase 0 runtime library** now ships as `.bc` (from compiling its Rust/Nom source), not as a source-in-dict artifact. The runtime's hash is the hash of its `.bc`.

The self-hosting path is *simpler* under this shift: there's no "canonicalize Nom source in dict" step, because there's no Nom source in the dict to canonicalize.

## Success Metrics

- [ ] Nom lexer compiles and lexes itself
- [ ] Nom parser compiles and parses itself
- [ ] All phases compile Nom code without crashes
- [ ] Bootstrap: Nom compiler produces same IR as Rust compiler on test suite (10+ programs)
- [ ] Self-compilation loop completes: nom → Nom → nom → identical output
