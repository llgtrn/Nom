# Nom Self-Hosting

This directory contains Nom components written in Nom itself -- the path toward a self-hosting compiler.

## What is self-hosting?

A self-hosting compiler is one that can compile its own source code. Achieving self-hosting is a major milestone for any programming language because it proves the language is expressive enough to implement complex systems software.

## Current status

### `lexer.nom` -- Tokenizer (Phase 1)

The Nom lexer, written in Nom. This file mirrors the Rust implementation in `nom-lexer/src/lib.rs` and produces the same token types. It demonstrates that Nom's imperative syntax (fn, struct, enum, match, if/else, while, for, let) is sufficient to express a real tokenizer.

**What works today:**
- Valid Nom syntax that the parser recognizes
- Complete token type enum matching the Rust lexer
- All 80+ keywords including Vietnamese natural language aliases
- Full operator and bracket scanning
- Comment, string, number, and identifier scanning
- Blank-line detection logic
- Span tracking for error reporting

**Aspirational features used (not yet compilable):**
- Tuple return types: `fn foo() -> (Token, Lexer)`
- String indexing: `source[pos]` returning a byte/integer
- String slicing: `source[start..end]`
- Generic list types: `list[Token]`
- Enum variants with payloads: `Integer(integer)`
- Built-in `parse_int` / `parse_float` / `chr` functions

These represent concrete targets for the compiler to grow into.

## Roadmap

Phase numbers below reference [`03-self-hosting-roadmap.md`](../../../research/language-analysis/03-self-hosting-roadmap.md).

| Phase | Component | Status | Artifact |
|-------|-----------|--------|----------|
| 1 | Lexer | Written + compiles via LLVM | `lexer.nom` (+ `.bc` / `.ll`) |
| 2 | Parser | Planned | — |
| 3 | AST types | Planned | — |
| 4 | Verifier | Planned | — |
| 5 | Planner | Scaffolded (parses; default_entry_point + is_empty_plan helpers) | `planner.nom` |
| 6 | Codegen | Scaffolded (parses; default_entry_symbol + is_empty_source helpers) | `codegen.nom` |
| 7 | Bootstrap | Planned | — |

### `planner.nom` — scaffold (Phase 5)

Landed 2026-04-12 as a skeleton: the `Node` / `Edge` / `CompositionPlan` / `VerifiedAST` struct shapes + `nom_plan(ast) -> CompositionPlan` entry-point signature returning an empty plan. Real construction (graph building + topological sort + cycle detection + constraint propagation) arrives incrementally per the 10-12 week roadmap estimate. The Rust reference lives in [`nom-planner/src/lib.rs`](../../crates/nom-planner/src/lib.rs) (~700 LOC); the Nom target is 1100-1500 LOC once complete.

## Design decisions

- **Functional style**: The lexer passes `Lexer` structs through functions rather than mutating shared state. Each function returns a new lexer state. This aligns with Nom's emphasis on clarity and testability.
- **ASCII constants**: Character comparisons use integer ASCII values (e.g., `ch == 35` for `#`) since Nom does not yet have character literals.
- **Mirror the Rust impl**: The token types, keyword table, and scanning logic match `nom-lexer/src/lib.rs` exactly, so the self-hosted lexer produces identical output.
