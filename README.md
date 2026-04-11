# Nom — Write Software Like Writing Sentences

**A nomtu is a word. A .nom is a sentence. A binary is a story.**

Nom is the fifth programming paradigm: **compositional-semantic programming.**
You write sentences describing what you want. The compiler looks up each word
in an online dictionary, verifies the grammar (contracts), and produces a
native binary. Like magic — you describe, it creates.

Author: TRAN NGUYEN HOANG LONG

## Quick Start

```bash
# Build the compiler (requires Rust toolchain)
cd nom-compiler
cargo build --release

# Compile a .nom file to a native binary
nom build examples/auth.nom

# Run the resulting binary
./examples/.nom-out/auth/target/release/auth
```

Milestone: `auth.nom` (9 lines) compiles to an 834KB native Windows binary.

## How It Works

```
You write:     system auth
               need hash::argon2 where security>0.9
               need store::redis where reliability>0.8
               flow request->hash->store->response

Compiler:      looks up hash.nomtu and store.nomtu in dictionary (nom.dev)
               verifies contracts (hash output → store input compatible?)
               downloads implementations
               compiles via LLVM to native binary

Result:        a working auth service binary
               from 4 lines of writing
```

## The Files

```
.nom        what you write (sentences in English, writing-style, no braces)
.nomtu      a word in the dictionary (text: name + description + contract + scores)
.nomiz      compiled composition graph (the IR, ready for LLVM)
nomdict     the online dictionary (nom.dev, millions of .nomtu entries)
nom         the CLI (compiler, builder, debugger — one tool for everything)
binary      the final app (native, assembly-smooth, no runtime overhead)
```

## Hybrid Declarative-Imperative Syntax

Nom combines declarative orchestration with general-purpose programming.
Classifiers start declarations. Blank lines separate them.
English by default. Foreign words welcome when English is imprecise.

```nom
system chatbot
need nlp::understand where accuracy>0.85
need memory::vector where recall>0.8
need llm::generate where quality>0.9
flow message->nlp->memory->llm->reply

nom validate
  fn check(input: text) -> bool {
    if input.len() > 0 {
      return true
    }
    return false
  }

  struct Config {
    max_tokens: integer,
    temperature: number
  }

test chatresponse
given chatbot with mockllm
when message "hello"
then reply contains greeting
```

## Why Nom Exists

Every AI code generator hallucinates (17-89% of the time).
Nom composes from verified dictionary entries (0% fabrication in dictionary boundary).

Every package manager trusts by default ($60B supply chain losses in 2025).
Nom verifies contracts, scores quality, tracks provenance for every .nomtu.

Every language requires braces, semicolons, tabs, and years of learning.
Nom reads like writing. Describe what you want. Get a working application.

## Documents

- [SYNTAX.md](SYNTAX.md) — **Formal syntax reference** (every keyword, operator, and construct)
- [BLUEPRINT.md](BLUEPRINT.md) — Build plan and technical decisions
- [Plan.md](Plan.md) — Language development roadmap
- [research/SPEC.md](research/SPEC.md) — The language specification
- [research/NOMTU.md](research/NOMTU.md) — The dictionary database format
- [research/motivation/](research/motivation/) — Why this design (5 docs)
- [research/deferred/](research/deferred/) — Phase 2+ research (7 docs)

## Status

Compiler works. `nom build` produces native binaries. `nom test --execute` compiles and runs test flows.

- 20 crates: nom-ast, nom-lexer, nom-parser, nom-resolver, nom-verifier, nom-codegen, nom-planner, nom-security, nom-diagnostics, nom-cli, nom-llvm, nom-translate, nom-extract, nom-graph, nom-search, nom-score, and more
- 194 tests passing across all crates
- 10 classifiers, 100+ keywords, 23 operators
- **Declarative:** need, require, effects, flow, describe, contract, implement
- **Imperative:** let, fn, if/else, for, while, match, struct, enum, return
- **Type system:** Named types, generics (`list[text]`), function types, tuples, references
- Graph queries with `union(...)`, `intersect(...)`, `difference(...)` algebra
- Agent primitives: capability, supervise, receive, state, schedule
- Pattern matching with wildcards, literals, bindings, variant destructuring
- Parser error recovery (reports all errors, not just the first)
- 9-dimension quality scoring (security, reliability, performance, readability, testability, portability, composability, maturity, overall)
- 10M+ .nomtu dictionary entries in nomdict (SQLite)
- 42 parseable languages via tree-sitter grammars (Rust, Python, TypeScript, JavaScript, Go, C, C++, Java, C#, Ruby, PHP, Swift, Scala, Haskell, OCaml, Elixir, Lua, R, Julia, Bash, HTML, CSS, JSON, YAML, TOML, Zig, Dart, Erlang, Elm, Nix, D, Objective-C, Fortran, CMake, Make, Protobuf, Regex, Verilog, Racket, GLSL, GraphQL, LaTeX, Groovy)
- `nom build auth.nom` produces an 834KB native binary from 9 lines of .nom

## Multi-Target Compilation

Nom now compiles to multiple backends:

- **LLVM backend** (`--target llvm`) — Produces LLVM bitcode (.bc) and IR (.ll) files
- **Native binary** (`--target native`) — Compiles to machine code via llc
- **Rust codegen** (`--target rust`, default) — Generates Rust source for portability

```bash
nom build --target llvm myapp.nom      # → myapp.bc + myapp.ll
nom build --target native myapp.nom    # → myapp (executable)
nom build --target rust myapp.nom      # → myapp.rs
```

Powered by inkwell 0.5 with LLVM 18. Supports functions, structs, enums, let/if/else/while/match, return statements, field access, binary operations, and function calls.

## New CLI Commands

```bash
nom test --property <file>              # Auto-generate property tests from contracts
nom dict --contract "bytes -> hash"     # Search dictionary by contract shape
```

## Flow Qualifiers and Fault Handling

New ADOPT-5 and ADOPT-4 features for production-grade orchestration:

```nom
flow::once request->hash->store         # Idempotent, runs once (default)
flow::stream events->process->output    # Ongoing, produces values over time
flow::scheduled daily->cleanup->report  # Runs on a schedule

flow request->hash->store onfail abort           # Default: stop on failure
flow request->hash->store onfail retry 3        # Retry up to 3 times
flow request->hash->store onfail restart_from hash  # Restart from a node
flow request->hash->store onfail skip           # Skip failed node, continue
flow request->hash->store onfail escalate       # Escalate to parent
```

## Runtime Library

Nom now includes a standard library (nom-runtime) with:

- `NomString` — Fat pointer string type
- `print`/`println` — Console output
- `alloc`/`free` — Memory management
- File I/O operations
- `panic` — Error handling

For dirty worktrees where repo-wide change detection is noisy, use `scripts/verify-scoped-changes.ps1 <paths...>` to verify just the files you touched.
