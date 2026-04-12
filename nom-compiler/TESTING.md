# Testing Nom

One-page pointer for running the tests that matter. CI runs all of
these per-crate via `.github/workflows/ci.yml`; this doc is for
local iteration.

## Quick loop

```sh
cd nom-compiler
cargo check --workspace --message-format short
cargo test -p nom-app                     # compile-output aspects
cargo test -p nom-parser nomx             # .nomx parser
cargo test -p nom-lexer nomx              # .nomx lexer
```

## Crate matrix

| Crate | What it tests |
|-------|---------------|
| `nom-types` | body_kind tags, self_host_tags consts, canonical IDs |
| `nom-dict` | SQLite schema + CRUD, histograms, closure walk, concepts |
| `nom-lexer` | token stream of classic Nom; `nomx` module tests are the natural-language tokenizer |
| `nom-parser` | classic Nom AST; `nomx` module tests are the natural-language parser |
| `nom-planner` | composition plans, concurrency / memory strategies |
| `nom-verifier` | contract typing, effect tracking |
| `nom-codegen` | Rust-source emission |
| `nom-llvm` | LLVM IR emission, inkwell integration |
| `nom-corpus` | clone-and-ingest, PyPI top, body-compile determinism |
| `nom-app` | 10 §5.12 OutputAspect populators, dreaming-mode score |
| `nom-cli` | end-to-end: all the above wired through the CLI |

## Key gate tests

Drift in these is a process-level signal:

| Test | Catches |
|------|---------|
| `nom-corpus::tests::compile_nom_to_bc_is_deterministic` | LLVM output non-determinism (Risk #1 §10.3.1) |
| `nom-app::tests::compile_app_to_artifacts_is_deterministic` | Aspect-populator non-determinism |
| `nom-cli::tests::self_host_rust_parity` | `self_host_tags` const ↔ `.nom` scaffold literal drift |
| `nom-cli::tests::self_host_pipeline` | Every `.nom` in stdlib/self_host compiles end-to-end |
| `nom-cli::tests::self_host_meta` | Every `.nom` scaffold has its acceptance test |
| `nom-cli::tests::parser_subset_probe` | Current parser surface + aspirational-feature recovery behavior |
| `nom-parser::nomx::tests::parses_todo_app_nomx_end_to_end` | `.nomx` full-grammar sample stays parseable |

## Fixtures

Prose drafts that exercise `nom author translate`:

```
examples/draft_sentence.md      — 1 intent line
examples/draft_paragraph.md     — intent + 8 sketch bullets
examples/draft_essay.md         — intent + sketch + constraints (13+ bullets)
examples/draft_todo_app.md      — the canonical brainstorm
```

`.nomx` samples:

```
examples/hello.nomx             — block define with 1 binding
examples/todo_app.nomx          — record + choice + 3 defines
examples/greet_sentence.nomx    — 3 `to ... respond with ...` one-liners
examples/loops.nomx             — for-each + while + nested when/unless
examples/contracts.nomx         — require / ensure / throughout clauses
examples/mixed_forms.nomx       — record + choice + block-define + 2 `to`-oneliners in one file
```

## Running a single test

```sh
cargo test -p nom-app --quiet \
    compile_app_to_artifacts_is_deterministic -- --nocapture
```

Use `-- --nocapture` to see `println!` / `eprintln!` output; by
default `cargo test` captures it unless the test fails.

## Windows caveats

`nom-cli`'s bin-crate tests transitively link `nom-llvm`, which
means the test exe needs LLVM-C.dll on PATH at exe startup. If
you see `STATUS_DLL_NOT_FOUND`, either install LLVM 18 and put
its `bin/` on PATH, or run the tests on WSL / Linux / macOS.

CI pre-installs `llvm-18` on the Ubuntu runner, so this is a
local-dev-only issue.
