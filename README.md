# Nom — Write Software Like Writing Sentences

**A nomtu is a word. A .nom is a sentence. A binary is a story.**

Nom is the fifth programming paradigm: **compositional-semantic programming.**
You write sentences describing what you want. The compiler looks up each word
in an online dictionary, verifies the grammar (contracts), and produces a
native binary. Like magic — you describe, it creates.

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

## Writing-Style Syntax

No braces. No semicolons. No tabs required.
Classifiers start declarations. Blank lines separate them.
English by default. Foreign words welcome when English is imprecise.

```nom
system chatbot
need nlp::understand where accuracy>0.85
need memory::vector where recall>0.8
need llm::generate where quality>0.9
flow message->nlp->memory->llm->reply

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

Compiler works. `nom build` produces native binaries.

- 10 crates: nom-ast, nom-lexer, nom-parser, nom-resolver, nom-verifier, nom-codegen, nom-planner, nom-security, nom-diagnostics, nom-cli
- 42 tests passing across all crates
- 10 classifiers, 42 keywords, graph and agent primitives all parsing
- 10M+ .nomtu dictionary entries in nomdict (SQLite)
- `nom build auth.nom` produces an 834KB native binary from 9 lines of .nom
