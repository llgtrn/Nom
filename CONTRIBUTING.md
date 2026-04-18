# Contributing to Nom

This document describes how to build, test, and extend Nom. Everything here
reflects the actual state of the codebase; no aspirational items are included.

---

## Development Setup

### Requirements

- Rust stable (see `nom-canvas/rust-toolchain.toml` for the pinned version)
- LLVM 18 (required by `nom-llvm` in the compiler workspace)
- SQLite (linked at build time by `rusqlite`)

### Build

```bash
# Canvas workspace (IDE + composition API)
cd nom-canvas
cargo build

# Compiler workspace (language toolchain)
cd nom-compiler
cargo build
```

### Test

```bash
# Canvas workspace
cd nom-canvas
cargo test --workspace

# Compiler workspace
cd nom-compiler
cargo test --workspace
```

The CI matrix runs on Ubuntu, Windows, and macOS (see
`.github/workflows/ci.yml`). All three platforms must pass before merging.

### Formatting and Linting

```bash
# Format
cargo fmt

# Lint (zero warnings required)
cargo clippy -- -D warnings
```

The workspace `Cargo.toml` enables `#![deny(unsafe_code)]` and
`#![deny(warnings)]` project-wide. A PR that introduces a clippy warning or
unsafe block will fail CI.

---

## Crate Organization

### nom-canvas workspace (`nom-canvas/crates/`)

| Crate | Role |
|-------|------|
| `nom-blocks` | Block primitives: models, connectors, tables, dataviews |
| `nom-canvas-core` | Core canvas primitives: elements, hit-testing, selection, snapping, spatial index, viewport |
| `nom-canvas-tests` | Integration tests for the canvas workspace |
| `nom-cli` | CLI entry point: `nom bench`, `nom flow`, `nom media`, `nom corpus`, `nom app`, `nom ux`, and the `serve` feature |
| `nom-collab` | Peer-to-peer collaboration: `PeerId`, CRDT operations, conflict resolution |
| `nom-compiler-bridge` | Optional bridge to the compiler workspace; `CandleAdapter` for on-device ML inference |
| `nom-compose` | Composition API: `ComposeContext`, `HybridResolver`, `FlowGraph`, `AiGlueOrchestrator`, video pipeline, animation |
| `nom-editor` | Text editor primitives: `Buffer`, `Point`, cursor, clipboard, completion |
| `nom-gpui` | GPU rendering substrate: animation, atlas, element primitives, wgpu init chain |
| `nom-graph` | DAG execution: `ExecutionEngine`, caching, `NomGraph`, sandbox expression evaluator |
| `nom-intent` | Intent resolution: `IntentResolver`, `SkillRouter`, `SkillDefinition`, BM25 index, `classify_with_react` |
| `nom-lint` | Rule-based linter with sealed rule trait; reports `Diagnostic` items |
| `nom-media` | Media primitives: `MediaUnit`, `MediaKind`, `Codec`, `Container` |
| `nom-memoize` | Memoization layer: content-addressed memo cache, constraint tracking, hash helpers |
| `nom-panels` | All IDE panels: left (`NodePalette`, `LibraryPanel`, `FileTreePanel`), center (`TabManager`, `CenterLayout`), right (`DeepThinkPanel`, `ChatSidebarPanel`, `IntentPreviewCard`, `AiReviewCard`), bottom (`DiagnosticsPanel`, `StatusBar`, `TerminalPanel`) |
| `nom-telemetry` | Structured event log: `TelemetryEvent`, metrics aggregation |
| `nom-theme` | Design tokens, `FontRegistry`, `Icon`, `TypeStyle`; three themes (`light`, `dark`, `oled`) |
| `nom-ux` | UX primitives: `UserFlow`, `FlowStep`, `UxPattern`, `DesignRule`, `Screen` |

### nom-compiler workspace (`nom-compiler/crates/`)

The compiler workspace contains 29 crates including `nom-ast`, `nom-concept`,
`nom-dict`, `nom-grammar`, `nom-intent`, `nom-llvm`, `nom-lsp`, `nom-planner`,
`nom-resolver`, `nom-runtime`, and others. The canvas workspace references
compiler crates only via optional feature-gated path dependencies — no IPC,
no network calls at build time.

---

## Adding New Functionality

### The Wave Pattern

Development is organized in waves. A wave is a focused batch of related
changes committed together with a test count target. Each wave:

1. Opens with a list of numbered items (`AL-NAME`, `B4`, `C5-V3`, etc.)
2. Every item ships with tests — no item is complete without them
3. The wave is committed only when all items are checked and the total test
   count increases

When contributing, identify which wave axis your work belongs to:

| Axis | Scope |
|------|-------|
| A | nom-compiler language features |
| B | Nom language (kinds, skills, syntax, CLI) |
| C | nom-canvas ↔ compiler integration |
| D | Platform-wide (docs, tooling, CI, UX) |

### Test Requirements

Every PR must include tests for the changed behaviour. The minimum bar:

- New public functions: at least one passing `#[test]`
- New structs/enums: constructor roundtrip + meaningful field assertions
- New CLI subcommands: at least one `run()` call that returns `Ok`
- Bug fixes: a test that would have failed before the fix

Tests live next to the code in `#[cfg(test)]` modules. Integration tests go
in `nom-canvas-tests` or in a crate-level `tests/` directory.

### Adding a New Kind

All dispatch is runtime string-based. To add a new composition kind:

1. Insert a row into `grammar.kinds` (use `nom-dict` helpers or raw SQL).
2. Implement the backend: a struct with a `compose(input)` method that returns
   `ComposeResult`.
3. Register it: `BackendRegistry::register(Box::new(MyBackend))`.
4. Write at least one test that dispatches the kind through the registry.

No Rust enum changes are required. The closed `BackendKind` enum was deleted
in Wave AP; all dispatch now routes via runtime `&str`.

### Adding a New Skill

Skills are `SkillDefinition` rows. To add one:

1. Construct a `SkillDefinition` with a stable `id`, human `name`, prose
   `description`, and JSON Schema strings for input and output.
2. Register it: `SkillRouter::register(def)`.
3. Alternatively, insert it directly into the `grammar.kinds` table.

---

## Code Standards

### Zero Foreign Identifiers

No external brand names appear in any public API, type name, variable name,
comment, CLI output, or database column. Names must be abstract descriptions
of the pattern. Source attribution belongs in `entry_meta`, not in `word`.

### No Wrapper Layers

Every function does real work. Thin re-exports that exist only for
compatibility are not permitted. When new code replaces old code, the old code
is deleted in the same commit.

### DB is the Source of Truth

Grammar, kinds, skills, and entries live in `nomdict.db`. Hardcoded seed
arrays in Rust source are not permitted. The `nom-grammar` crate ships a
schema and query helpers only; it does not contain `KINDS_SEED` constants or
`seed_*` functions.

### One Binary, Fully Rust

The canvas IDE is a single native binary. No webview, no Electron, no IPC
between the IDE and the compiler. The `nom-compiler-bridge` crate links the
compiler in-process via optional feature flags.

---

## Non-Negotiable Rules

These rules apply to every change:

1. Read source repos end-to-end before borrowing any pattern from them.
2. Use the `ui-ux-pro-max` skill for all UI work.
3. Zero foreign identities in any public API surface.
4. `nom-compiler` is core — direct workspace path deps, zero IPC.
5. The DB is the workflow engine — no external orchestrator.
6. Every canvas object is a DB entry — `entity: NomtuRef` is non-optional.
7. The canvas visual model is frosted glass + confidence edges.
8. Deep thinking is a compiler operation streamed to the right dock panel.
9. GPUI is fully Rust — one binary, no webview.
10. Run `gitnexus_impact` before editing any symbol.
11. Run `gitnexus_detect_changes` before committing to confirm scope.
12. `cargo clippy -- -D warnings` must produce zero warnings before any commit.
