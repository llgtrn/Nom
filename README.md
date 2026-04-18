# Nom

An AI-native programming language and IDE. Write prose, get native binaries. The compiler runs
continuously — every keystroke is a compile event, every canvas block is a compiler concept.

## What is Nom?

Nom is a two-part system:

- **The language** — natural-syntax source files (`.nom`, `.nomx`) where the last sentence is the
  result, there are no null values by grammar, and every named thing maps to a dictionary entry
  (`nomtu`).
- **The IDE** — a GPU-rendered canvas (GPUI, Rust-native) where the compiler is the workflow
  engine. No separate build step; composing is compiling.

The dictionary (`nomdict.db`) is the backbone. Every entity, skill, kind, and benchmark lives there.
DB IS the workflow engine — not a cache, not a side-car.

## Quick Start

```bash
# Run all tests (from nom-canvas workspace)
cd nom-canvas && cargo test --workspace

# Build the CLI
cd nom-canvas && cargo build -p nom-cli

# Compile a .nomx file
./target/debug/nom store sync examples/agent_demo
./target/debug/nom build manifest examples/agent_demo --pretty
```

## Workspace Structure

```
Nom/
├── nom-canvas/     # GPU-native IDE shell (16 crates)
│   ├── nom-blocks          — block model + CRDT
│   ├── nom-canvas-core     — viewport, layout, spatial index
│   ├── nom-cli             — CLI entry point (nom binary)
│   ├── nom-collab          — collaborative CRDT sync
│   ├── nom-compiler-bridge — bridge to nom-compiler workspace
│   ├── nom-compose         — hybrid composition (DB→Provider→AI)
│   ├── nom-editor          — rope buffer, syntax highlight
│   ├── nom-gpui            — GPUI renderer, window, atlas
│   ├── nom-graph           — BFS graph RAG + confidence edges
│   ├── nom-intent          — BM25 + ReAct intent classifier
│   ├── nom-lint            — lint rules + diagnostics
│   ├── nom-memoize         — incremental memoization
│   ├── nom-panels          — UI panels (chat, deep-think, props)
│   ├── nom-telemetry       — metrics + tracing
│   ├── nom-theme           — design tokens + color themes
│   └── nom-ux              — UX patterns + screen kinds
└── nom-compiler/   # Language compiler (29 crates)
    ├── nom-dict            — DB schema + nomdict.db writer/reader
    ├── nom-grammar         — grammar kinds + clause shapes
    ├── nom-media           — media pipeline (video/audio/image/3D)
    ├── nom-lsp             — LSP server (JSON-RPC, stdio)
    └── ... (25 more crates: lexer, parser, IR, LLVM codegen, etc.)
```

## Key Features

- **define-that syntax** — `define greet that says hello` replaces `fn greet() -> String`
- **DB-driven kinds** — grammar.kinds table drives palette, autocomplete, and composition routing
- **Three-tier composition** — DB-driven → Provider → AI-leading (`HybridResolver`)
- **Composable media pipeline** — video, audio, image, 3D mesh, web app, native app from one spec
- **ReAct intent loop** — BM25 + LLM chain for ambiguous requests (delta < 0.15 threshold)
- **LSP server** — hover, completion, definition, references, workspace/symbol dispatch
- **8891+ tests** passing across all crates

## Architecture

```
.nom / .nomx source
       │
       ▼
  nom-compiler (29 crates)
  ├── lexer → parser → resolver → type-checker → IR → LLVM codegen
  └── nomdict.db — every entry is a nomtu (hash + word + contract + scores)
       │
       ▼
  nom-canvas (16 crates)
  ├── GPUI renderer — wgpu + winit + taffy + cosmic-text, one binary
  ├── compiler-bridge — zero IPC, direct workspace dep
  ├── HybridResolver — DB-driven → Provider → AI-leading
  └── DB IS the workflow engine (not a cache)
```

## Development

```bash
# Run tests
cd nom-canvas && cargo test --workspace
cd nom-compiler && cargo test --workspace

# Check formatting
cd nom-canvas && cargo fmt --check --all

# Run lints
cd nom-canvas && cargo clippy --workspace

# Build release
cd nom-canvas && cargo build --release
```

## Wave History (recent)

| Wave | Tests | Highlights |
|------|-------|-----------|
| Waves 0–T | 717 | Bootstrap, GPUI substrate, editor, blocks, compose backends |
| Waves V–AB | 2841 | GPU library wiring + 9 coverage waves |
| Waves AF–AK | 6743 | Minimalist UI + 5 coverage waves |
| Waves AL–AO | 8384 | CommandStack, CRDT GC, DB-driven palette/library, spatial, frame, theme |
| Wave AP | 8391 | Renderer pixels, BackendKind, GrammarKind.status, TaffyTree fixes |
| Waves AQ–AT | 8420 | NOM-GRAPH-ANCESTRY, SELF-DESCRIBE, BM25/ReAct, LSP server stub |
| Waves AU–AV | 8446 | Composition API, HybridResolver, video pipeline, CompositionConfig |
| Waves AW–ABA | 8891 | AuthoringProtocol, bootstrap stubs, LLVM IR types, 100 translations |

## Documentation

- [docs/user-manual.md](docs/user-manual.md) — getting started, language syntax, canvas, composition API, CLI reference
- [docs/api-reference.md](docs/api-reference.md) — public types in `nom-compose`, `nom-intent`, `nom-graph`, `nom-panels`
- [CONTRIBUTING.md](CONTRIBUTING.md) — crate organization, adding kinds/skills, code standards

Author: LLg Trn
