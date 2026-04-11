# Phase 2: LLVM Backend + 50-Language Analysis + Self-Hosting

**Date:** 2026-04-12  
**Status:** Design  
**Approach:** B — Imperative-first LLVM, incremental migration

## Goals

1. **Direct .bc compilation** — `.nom → LLVM IR → .bc → native binary` with no Rust middle layer
2. **50-language analysis** — Extract patterns and weaknesses from 50+ languages to strengthen Nom
3. **Self-hosting** — Long-term goal: rewrite Nom compiler in Nom itself

## Architecture

### New Crate: `nom-llvm`

Pipeline: `.nom → Lexer → Parser → AST → Verifier → Planner → nom-llvm → .bc/.ll → native binary`

The `nom-llvm` crate takes a `CompositionPlan` (same input as `nom-codegen`) and emits LLVM IR using the `inkwell` crate (safe Rust bindings to LLVM C API).

### Module Structure

```
nom-compiler/crates/nom-llvm/
├── Cargo.toml          # inkwell dependency
├── src/
│   ├── lib.rs          # Public API: compile(plan) → Result<Vec<u8>>
│   ├── context.rs      # LLVM Context/Module/Builder, type cache
│   ├── types.rs        # Nom TypeExpr → LLVM type mapping
│   ├── functions.rs    # FnDef → LLVM function compilation
│   ├── statements.rs   # Let, If, For, While, Match → IR instructions
│   ├── expressions.rs  # BinOp, UnaryOp, calls, field access → IR values
│   ├── structs.rs      # StructDef → named LLVM struct types
│   ├── enums.rs        # EnumDef → tagged union (i8 tag + payload)
│   └── runtime.rs      # Minimal runtime stubs (print, alloc, string)
```

### Type Mapping

| Nom Type | LLVM IR Type | Representation |
|----------|-------------|----------------|
| `number` / `f64` | `double` | 64-bit IEEE float |
| `integer` / `i64` | `i64` | 64-bit signed integer |
| `bool` | `i1` | Single bit |
| `text` / `String` | `{i8*, i64}` | Fat pointer: data + length |
| `list[T]` | `{T*, i64, i64}` | Data pointer + length + capacity |
| `struct` | Named struct | Direct field layout |
| `enum` | `{i8, [max_variant_size x i8]}` | Tag + payload bytes |
| `()` / Unit | `void` | No return |

### Compilation Strategy (Imperative Core First)

**Phase 2a — LLVM imperative core:**
- `fn` definitions → LLVM functions with proper calling convention
- `let` / `mut` → `alloca` + `store`/`load` instructions
- `if/else` → conditional branches between basic blocks
- `for` / `while` / `loop` → loop headers with phi nodes or alloca
- `match` → switch instruction or chained comparisons
- `struct` → named LLVM struct types with GEP field access
- `enum` → tagged union with extractvalue/insertvalue
- Binary/unary ops → corresponding LLVM instructions
- Function calls → LLVM call instructions
- String literals → global constants with fat pointer wrapper
- `return` → ret instruction

**Phase 2a deliverable:** `nom build hello.nom → hello.bc → hello (native binary)` for programs using imperative constructs.

**Phase 2a deferred:** Declarative flows, contracts, effects, agents, graphs stay on Rust codegen (`nom-codegen`). These will be migrated in a later phase once the runtime model is designed.

### Runtime Library (`nom-runtime`)

A minimal C/Rust static library linked into every Nom binary:
- `nom_print(text)` — stdout output
- `nom_alloc(size) → ptr` — heap allocation
- `nom_string_concat(a, b) → text` — string concatenation
- `nom_string_eq(a, b) → bool` — string comparison
- `nom_panic(msg)` — runtime error with message

This keeps the LLVM IR clean while providing essential runtime support.

### CLI Integration

New `nom-cli` target option:
```
nom build --target llvm hello.nom    # → hello.bc
nom build --target native hello.nom  # → hello.bc → hello (via llc+lld)
nom build hello.nom                  # → default (llvm when available)
```

## Phase 2b: 50-Language Analysis

### Approach

Expand `nom-extract` with tree-sitter grammars for 50+ languages. For each language:
1. Parse representative codebases with tree-sitter
2. Extract: type system patterns, error handling, concurrency model, memory model, syntax idioms
3. Identify weaknesses and strengths
4. Generate `.nomtu` entries cataloging findings
5. Feed into Nom language design decisions

### Language Groups (Priority Order)

1. **Systems:** Rust, C, C++, Zig, Odin, Nim, D, Ada, Forth (memory, performance)
2. **Application:** Go, Java, C#, Kotlin, Swift, Scala, F# (type systems, ergonomics)
3. **Scripting:** Python, Ruby, Perl, Lua, PHP, Tcl, Bash (productivity, DSLs)
4. **Functional:** Haskell, OCaml, Elixir, Erlang, Clojure, Racket, Elm (correctness, composition)
5. **Modern:** TypeScript, Dart, Julia, Crystal, V, Gleam, Roc, Mojo (modern design)
6. **Niche:** Prolog, APL/J/K, Forth, Tcl, Smalltalk, Io, Factor (unique paradigms)
7. **Vietnamese context:** Consider Vietnamese programming concepts and naming patterns

### Deliverable

Research report in `research/language-analysis/` with per-language findings and a synthesis document identifying Nom improvements.

## Phase 2c: Self-Hosting (Long-term)

Once the LLVM backend handles the imperative core:
1. Rewrite `nom-lexer` in Nom (simplest pass, string processing)
2. Rewrite `nom-parser` in Nom (recursive descent, pattern matching)
3. Rewrite `nom-planner` in Nom
4. Rewrite `nom-llvm` in Nom (the compiler compiles itself)

### Prerequisites for Self-Hosting

- Working LLVM backend for imperative code
- String processing primitives in runtime
- File I/O in runtime
- Module/import system implemented
- Standard library with collections

## Success Criteria

- [ ] `nom build --target llvm` produces valid .bc file from imperative Nom code
- [ ] Native binary runs correctly for: arithmetic, control flow, structs, enums, functions
- [ ] At least 50 languages analyzed with findings documented
- [ ] Nom syntax improvements identified from language analysis
- [ ] Self-hosting roadmap with clear prerequisites tracked

## Risks

1. **inkwell/LLVM version compatibility** — Pin to LLVM 17 or 18, use inkwell's version feature flags
2. **String/collection runtime complexity** — Keep runtime minimal, grow incrementally
3. **Enum representation** — Tagged unions need careful alignment; start simple (fixed-size payload)
4. **50-language tree-sitter coverage** — Not all grammars are maintained; may need fallback regex extraction for some languages
