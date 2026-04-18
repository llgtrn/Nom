# Nom

**One binary. Fully Rust. GPU-rendered. Compiler-as-IDE.**

Nom is a GPU-native IDE where the compiler runs continuously — every keystroke is a compile event, every canvas block is a compiler concept. Built on wgpu + winit + taffy + cosmic-text.

## Architecture

- **nom-compiler** (29 crates) — the core. Direct workspace deps, zero IPC.
- **nom-canvas** (14 crates) — the GPU-native IDE shell.
- **nomdict.db** — the dictionary. Every entity is a nomtu entry. DB IS the workflow engine.

## Getting Started

```bash
# Build
cd nom-canvas && cargo build --release

# Test
cd nom-canvas && cargo test --workspace

# Serve the compose API
cd nom-canvas && cargo run -p nom-cli --features serve
```

## Compose

Nom composes any artifact from `.nomx` prose:
- **Media**: video · picture · audio · 3D mesh
- **Screen**: web app · native app · mobile app
- **Data**: extract · transform · query
- **Concept**: document · presentation

## Status

| Axis | Progress |
|------|----------|
| A · nom-compiler | 44% |
| B · Nom language | 40% |
| C · nom-canvas ↔ compiler | 65% |
| D · Overall platform | 90% |

8446+ tests passing.

## How It Works

The layered concept architecture demonstrated in [nom-compiler/examples/agent_demo/](nom-compiler/examples/agent_demo/):

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

Pipeline you can run today:

```bash
cd nom-compiler
cargo build -p nom-cli
./target/debug/nom store sync examples/agent_demo
./target/debug/nom build status examples/agent_demo
./target/debug/nom build manifest examples/agent_demo --pretty
```

## The Files

```
.nom        what you write (sentences in English, no braces)
.nomtu      a word in the dictionary (name + description + contract + scores)
.nomiz      compiled composition graph (the IR, ready for LLVM)
nomdict     the online dictionary (millions of .nomtu entries)
nom         the CLI (compiler, builder, debugger — one tool)
binary      the final app (native, no runtime overhead)
```

## Multi-Target Compilation

```bash
nom build --target llvm myapp.nom      # LLVM bitcode (.bc + .ll)
nom build --target native myapp.nom    # machine code executable
nom build --target rust myapp.nom      # Rust source
```

Powered by inkwell 0.5 with LLVM 18.

## Composition API

`ComposeContext` + `HybridResolver` route every request through three tiers: DB-driven → Provider → AI-led.

```rust
use nom_compose::{ComposeContext, ComposeTier, HybridResolver};

// Tier 1: DB-driven (grammar kinds with Complete status)
let ctx = ComposeContext::new("picture", "a sunset over mountains");

// Override to AI-led tier when the kind is novel
let ctx_ai = ComposeContext::new("custom_artifact", "my input")
    .with_tier(ComposeTier::AiLeading)
    .with_intent("render as a poster");

// HybridResolver tries each tier in order, returns the first success
let result = resolver.resolve(&ctx)?;
```

## Video Pipeline

`CompositionConfig` declares render parameters; `VideoCodec` selects the output format.

```rust
use nom_compose::{CompositionConfig, VideoCodec};

let cfg = CompositionConfig {
    fps: 60,
    duration_frames: 300,   // 5 seconds at 60 fps
    width: 3840,
    height: 2160,
    default_codec: Some(VideoCodec::Hevc),
};

// Register under a stable id — config_fn is called per render
registry.register("4k_promo", Box::new(move || cfg.clone()))?;
let active = registry.get_config("4k_promo").unwrap();
assert_eq!(active.fps, 60);
```

## Wave History

| Wave | Tests | Highlights |
|------|-------|-----------|
| Waves 0–T | 717 | Bootstrap, GPUI substrate, editor, blocks, compose backends, GPU library |
| Waves V–AB | 2841 | GPU library wiring + 9 coverage waves |
| Waves AF–AK | 6743 | Minimalist UI + 5 coverage waves |
| Wave AL | 7241 | CommandStack, CRDT GC, panel serialization |
| Waves AM–AO | 8384 | DB-driven palette/library/SQL-inject fix/CRDT fix/spatial/frame/theme |
| Wave AP | 8391 | All 4 criticals fixed: renderer pixels, BackendKind deleted, GrammarKind.status, TaffyTree |
| Wave AQ | 8413 | NOM-GRAPH-ANCESTRY, SELF-DESCRIBE, BM25/classify_with_react, viewport spatial |
| Wave AR | 8391 | B4 46 kinds, B7 skills, B5 side-tables, B9 CLI, C3 compiler default, zero foreign names |
| Wave AT | 8420 | LSP server stub (JSON-RPC dispatch), lsp_server module, 4 new tests |
| Wave AU | 8434 | Composition API docs, HybridResolver tier routing, ComposeContext builder |
| Wave AV | 8446 | Video pipeline, CompositionConfig/VideoCodec registry, README improvements |

## Documents

- [SYNTAX.md](SYNTAX.md) — Formal syntax reference
- [BLUEPRINT.md](BLUEPRINT.md) — Build plan and technical decisions
- [research/SPEC.md](research/SPEC.md) — The language specification
- [research/NOMTU.md](research/NOMTU.md) — The dictionary database format

Author: LLg Trn
