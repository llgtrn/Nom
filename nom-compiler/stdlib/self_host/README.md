# Nom Self-Hosting

This directory contains Nom components written in Nom itself -- the path toward a self-hosting compiler.

> **See also:** the experimental `.nomx` natural-language grammar
> track — an alternative surface syntax (`define X that takes Y
> and returns Z:` style) proposed in
> [`05-natural-language-syntax.md`](../../../research/language-analysis/05-natural-language-syntax.md).
> Lexer + parser prototypes live; self-host scaffolds migrate to
> `.nomx` once the grammar stabilizes enough to express them.
> Current `.nomx` status: [`proposal §10`](../../../research/language-analysis/05-natural-language-syntax.md#10-implementation-status-updated-2026-04-13).

## What is self-hosting?

A self-hosting compiler is one that can compile its own source code. Achieving self-hosting is a major milestone for any programming language because it proves the language is expressive enough to implement complex systems software.

## Current status

### `lexer.nom` -- Tokenizer (Phase 1)

The Nom lexer, written in Nom. This file mirrors the Rust implementation in `nom-lexer/src/lib.rs` and produces the same token types. It demonstrates that Nom's imperative syntax (fn, struct, enum, match, if/else, while, for, let) is sufficient to express a real tokenizer.

**What works today:**
- Valid Nom syntax that the parser recognizes
- Complete token type enum matching the Rust lexer
- All 80+ English keywords (vocabulary fully English; Vietnamese grammar style inspires structure only — commit `ecd0609` removed the earlier VN keyword-alias experiment)
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

| Phase | Component | Status | Artifact | Helpers |
|-------|-----------|--------|----------|---------|
| 1 | Lexer | Written + compiles via LLVM | `lexer.nom` (+ `.bc` / `.ll`) | — |
| 2 | Parser | Scaffolded, parses, compiles | `parser.nom` | 9 fns |
| 3 | AST types | Scaffolded, parses, compiles | `ast.nom` | 9 fns |
| 4 | Verifier | Scaffolded, parses, compiles | `verifier.nom` | 6 fns |
| 5 | Planner | Scaffolded, parses, compiles | `planner.nom` | 7 fns |
| 6 | Codegen | Scaffolded, parses, compiles | `codegen.nom` | 6 fns |
| 7 | Bootstrap | Planned | — | — |

Each scaffold is gated by an acceptance test under `crates/nom-cli/
tests/self_host_<phase>.rs` plus the roll-up `self_host_smoke.rs`
(parse gate) and `self_host_pipeline.rs` (full parse → plan → codegen
pipeline). A meta test (`self_host_meta.rs`) asserts every `.nom`
file has its acceptance test. CI enforces all of them per commit.

### Rust ↔ Nom parity

Canonical tag strings that both sides emit — `"abort"`, `"calls"`,
`"pure"`, `"nom_main"`, `"fn"`, `"integer"`, etc. — live as
`pub const` in [`nom_types::self_host_tags`](../../crates/nom-types/
src/lib.rs). 22 consts + 6 `*_ALL` slices (CLASSIFIERS, EDGE_KINDS,
RUST_TYS, EFFECTS, DECL_KINDS, PRIM_TYPES). The parity test
(`self_host_rust_parity.rs`) asserts each `.nom` scaffold contains
`return "<const_value>"` for its matching helper — drift on either
side fails CI at commit time.

## Design decisions

- **Functional style**: The lexer passes `Lexer` structs through functions rather than mutating shared state. Each function returns a new lexer state. This aligns with Nom's emphasis on clarity and testability.
- **ASCII constants**: Character comparisons use integer ASCII values (e.g., `ch == 35` for `#`) since Nom does not yet have character literals.
- **Mirror the Rust impl**: The token types, keyword table, and scanning logic match `nom-lexer/src/lib.rs` exactly, so the self-hosted lexer produces identical output.
