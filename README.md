# Nom — Write Software Like Writing Sentences

**A nomtu is a word. A .nom is a sentence. A binary is a story.**

Nom is the fifth programming paradigm: **compositional-semantic programming.**
You write sentences describing what you want. The compiler looks up each word
in an online dictionary, verifies the grammar (contracts), and produces a
native binary.

Author: LLg Trn

> **Implementation status (2026-04-13, HEAD `9dd262e`)**: early-stage but runnable.
> See [research/language-analysis/09-implementation-status-2026-04-13.md](research/language-analysis/09-implementation-status-2026-04-13.md)
> for a code-verified per-feature breakdown. Aspirational claims in this README
> are marked in [research/](research/) docs with ✅ SHIPPED / ⏳ PLANNED / ❌ ASPIRATIONAL tags.

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

## How It Works (shipped pipeline, `.nomx v2` keyed form per doc 07)

The layered concept architecture (doc 08) that ships today — demonstrated
in [nom-compiler/examples/agent_demo/](nom-compiler/examples/agent_demo/):

```nom
the concept minimal_safe_agent is
  intended to compose a small set of tools an LLM can plan with safely.

  uses the concept agent_safety_policy,
       the @Function matching "fetch the body of an https URL",
       the function read_file matching "read text from a workspace path",
       the function write_file matching "write text to a workspace path".

  exposes read_file, write_file.

  this works when the safety policy is composed.

  favor security then composability then speed.
```

Pipeline you can run today (Linux/macOS — Windows skipped because `nom`
links LLVM-C.dll for compile commands; metadata pipeline works anywhere):

```sh
cd nom-compiler
cargo build -p nom-cli
./target/debug/nom store sync examples/agent_demo
./target/debug/nom build status examples/agent_demo
./target/debug/nom build status examples/agent_demo --write-locks
./target/debug/nom build manifest examples/agent_demo --pretty
```

The pipeline does: parse → walk concept-graph closure → resolve typed-slot
refs (kind-only lookup; Phase-9 will add semantic re-rank) → check MECE
objective collisions → emit JSON build manifest for a Phase-5 planner.

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

## Status (code-verified 2026-04-13)

**Shipped** (evidence: commit SHAs in [doc 09](research/language-analysis/09-implementation-status-2026-04-13.md)):

- 27 crates in `nom-compiler/crates/` — including the new `nom-concept` (Tier-1+Tier-2 parser for `.nomtu` and `.nom` files per doc 08)
- Test counts: `nom-concept` 76/76, `nom-dict` 24/24, `nom-parser` 81/81, `nom-lexer` 29/29, `nom-media` 50/50+3 ignored
- 27-crate workspace builds clean (1 pre-existing dead-code warning)
- `.nomx v2 (keyed)` syntax ~98% implemented: `@Kind` sigil (commit `c9d1835`), typed-slot resolver (`c405d2a`), `with at-least N confidence` threshold (`97c836f`), per-slot top-K diagnostic (`853e70b`)
- Effect valence (motivation 02 §9 / motivation 10 §E #4 — genuinely novel): `benefit`/`hazard` keywords on entities; English-only (`c9d1835`)
- MECE objectives validator fires on real input (commit `c63a6a7` — agent_demo intentionally collides to prove it)
- Three e2e demos: `concept_demo/` (`a04b91e`), `agent_demo/` AI-agent composition (`e2d4eb4`), `agent_demo_vn/` Vietnamese keyword aliases (`c601f31`)
- Pipeline commands: `nom store sync`, `nom build status --write-locks`, `nom build manifest --pretty` (commits `ba7769f`, `bf95c2c`, `fef0419`)
- `§4.4.6` body-bytes invariant enforced (commits `540620d` + `6c336b4`)
- `§10.3.1` Rust toolchain pin (`1.94.1`) + CI enforcement (commits `29f5f1d` + `bc89d8a` + `96267df`)
- AVIF canonical encoder via ravif (commit `707aa93` chain); bitcode-into-body migration (`540620d`)

**Planned / aspirational** (see per-feature tables in [research/language-analysis/](research/language-analysis/)):

- Phase-5 planner-in-Nom — compiler is currently all Rust; porting to `.nom` is multi-quarter
- Phase-9 per-kind corpus-embedding resolver — current resolver is alphabetical-smallest tiebreak stub
- Layered dreaming (concept-tier + module-tier `nom dream`) — only app-level `nom app dream` exists
- Mass corpus ingestion at scale (dictionary currently holds only demo fixtures)
- AppManifest deprecation in favor of `app.nom` root concept

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
