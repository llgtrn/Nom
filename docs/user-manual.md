# Nom User Manual

Nom is a GPU-native IDE where the compiler runs continuously. Every canvas block
is a compiler concept; every keystroke is a compile event. The workspace is built
on wgpu + winit + taffy + cosmic-text and ships as a single Rust binary.

---

## 1. Getting Started

### Installation

Build from source using Rust stable (see `nom-canvas/rust-toolchain.toml` for
the pinned toolchain version):

```bash
cd nom-canvas
cargo build --release
```

The resulting binary is `nom-canvas/target/release/nom`. For the composition
API server add the `serve` feature:

```bash
cargo run -p nom-cli --features serve
```

Run the full test suite to verify your build:

```bash
cd nom-canvas && cargo test --workspace
```

### First Workspace

A workspace is a directory containing `.nom` source files and an optional
`nomdict.db` dictionary. Sync a directory of examples into the dictionary,
then inspect the resulting manifest:

```bash
nom store sync examples/agent_demo
nom build status examples/agent_demo
nom build manifest examples/agent_demo --pretty
```

The dictionary (`nomdict.db`) is the workflow engine. Every entity — function,
concept, skill, benchmark — is a row in that database.

### The 9 Foundation Kinds

Nom ships with nine composition kinds that drive the default backend registry:

| Kind | Output | Notes |
|------|--------|-------|
| `video` | Video stream | Rendered via `VideoBackend`; codec selected from `VideoCodec` |
| `picture` | Image | Rendered via `ImageBackend` |
| `audio` | Audio stream | `AudioBackend` with configurable `AudioCodec` and `AudioContainer` |
| `mesh` | 3D mesh | `MeshBackend`; primitive shapes via `MeshPrimitive` |
| `web_screen` | Web app | `WebScreenBackend` |
| `native_screen` | Desktop app | `NativeScreenBackend` |
| `mobile_screen` | Mobile app | `MobileScreenBackend` |
| `data` | Structured dataset | `DataBackend` for extraction, frames, and queries |
| `document` | Document | `DocumentBackend`; slides via `PresentationSpec` |

Additional kinds (46+ total) are seeded in `grammar.kinds` via
`nom-dict/src/grammar.kinds.baseline.sql`. New kinds require no Rust change —
they are added as rows, and dispatch routes by runtime string.

---

## 2. The Nom Language

### define/that Syntax

The high-level authoring format uses natural-language sentences:

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

Every statement ends at a blank line. No braces surround declaration bodies.
Classifiers (`system`, `flow`, `store`, `graph`, `agent`, `test`, `nom`,
`gate`, `pool`, `view`) open each declaration; the classifier determines which
statements are legal inside it.

### .nomx Format

`.nomx` files are the prose-first authoring surface. A brainstorm draft in
plain English is gradually converted to Nom syntax until the file is entirely
`.nom`. The `nom author start` and `nom author check` commands guide this
motion and report how much of the file has been converted.

Composition requests that do not match a known kind are handled by
`AiGlueOrchestrator`, which generates `.nomx` glue code via the
`ReActLlmFn` adapter and stores the result as a `GlueBlueprint`.

### Composing Artifacts

The three composition operators are:

| Operator | Meaning | Example |
|----------|---------|---------|
| `->` | Sequential data flow | `request->hash->store->response` |
| `::` | Variant specialization | `hash::argon2` |
| `+` | Coordinate composition | Reserved; recognized by lexer |

A flow described with `->` compiles to a `FlowGraph` where nodes are
dispatch targets and edges carry labelled connections. `FlowGraph::topological_order()`
returns the execution order via Kahn's algorithm.

---

## 3. Canvas Interface

### Panels Overview

The NomCanvas IDE uses a three-column layout managed by `Dock`:

- **Left panel** — `LeftPanelLayout` hosts four tabs: `NodePalette` (DB-driven
  list of available kinds), `FileTreePanel` (workspace file navigator),
  `LibraryPanel` (seeded library entries from `nomdict.db`), and
  `QuickSearch`. The icon rail (`IconRail`) on the far left switches between
  tabs.
- **Center panel** — `CenterLayout` contains one or more editor/canvas views
  managed by `TabManager`. Tabs can be `Editor`, `Canvas`, `Preview`, or
  `Welcome`. Splits are horizontal or vertical via `SplitDirection`.
- **Right panel** — houses `PropertiesPanel`, `ChatSidebarPanel`,
  `DeepThinkPanel`, `HypothesisTreeNav`, `IntentPreviewCard`, and
  `AiReviewCard`. The bottom strip adds `DiagnosticsPanel`, `StatusBar`,
  and `TerminalPanel`.

### Creating Nodes

Drag a `PaletteEntry` from the node palette onto the canvas surface. Each
entry maps to a `NomtuRef` (hash + word pair) and a `production_kind` string.
The `insert_block()` method on the workspace deduplication guard ensures no
two blocks share the same `NomtuRef`.

### Wiring Edges

Connect two canvas nodes by drawing from one port to another. Connectors are
validated via `Connector::new_with_validation()`, the only constructor
available. The `CanvasHitTester` uses an R-tree broadphase for hit detection
during drag operations.

### Confidence Edges

Edges in the canvas carry a confidence score in `[0.0, 1.0]`. The
`edge_color_for_confidence()` helper maps that score to a rendered colour so
high-confidence connections are visually distinct from low-confidence ones.
`IntentPreviewCard::confidence_label()` maps scores to `"high"` (≥ 0.8),
`"medium"` (≥ 0.5), or `"low"`.

---

## 4. Composition API

### ComposeContext, ComposeResult, ComposeTier

`ComposeContext` carries the kind string, raw input, tier override, and an
optional intent query. Construct with the builder pattern:

```rust
use nom_compose::{ComposeContext, ComposeTier};

let ctx = ComposeContext::new("picture", "a sunset over mountains");

let ctx_ai = ComposeContext::new("custom_artifact", "my input")
    .with_tier(ComposeTier::AiLeading)
    .with_intent("render as a poster");
```

`ComposeTier` has three variants: `DbDriven` (grammar kinds with Complete
status), `Provider` (registered vendor backends), and `AiLeading` (AI glue
generation via `AiGlueOrchestrator`).

`ComposeResult` carries the artifact string, the tier actually used, a
confidence score, and an optional `glue_hash` (present when the AI tier
generated `.nomx` glue).

### CompositionConfig, CompositionRegistry

`CompositionConfig` declares render parameters for a video composition:
`fps`, `duration_frames`, `width`, `height`, and `default_codec`. Register
configs under stable string IDs:

```rust
use nom_compose::{CompositionConfig, VideoCodec, CompositionRegistry};

let cfg = CompositionConfig {
    fps: 60, duration_frames: 300, width: 3840, height: 2160,
    default_codec: Some(VideoCodec::Hevc),
};

let registry = CompositionRegistry::new();
registry.register("4k_promo", Box::new(move || cfg.clone())).unwrap();
let active = registry.get_config("4k_promo").unwrap();
```

### VideoRenderConfig, RenderProgress

`VideoRenderConfig` controls the render pipeline: `concurrency` (parallel
frame workers), `ffmpeg_path`, and an optional `on_progress` callback.
`RenderProgress` is emitted after each batch and carries `rendered_frames`,
`encoded_frames`, `total_frames`, `stage` (`Rendering | Encoding | Muxing |
Complete`), and `elapsed_ms`. The `percent()` helper returns a `[0.0, 1.0]`
fraction.

### interpolate() and spring()

`interpolate(frame, input_range, output_range, easing, extrapolate_left,
extrapolate_right)` maps a frame value through an optional easing function
with four extrapolation policies: `Clamp`, `Extend`, `Identity`, `Wrap`.

`spring(frame, fps, config, from, to)` computes a spring-physics value using
`SpringConfig` fields `damping`, `mass`, `tension`, and `overshoot_clamping`.
At frame 0 the value equals `from`; at large frames it settles near `to`.

---

## 5. CLI Reference

The `nom` binary dispatches to subcommand groups. All groups are implemented
in `nom-canvas/crates/nom-cli/src/`.

### nom compose

Runs a composition request via `HybridResolver`. Supports streaming output
via an axum endpoint when built with `--features serve` (`POST /compose`).

### nom bench

Benchmark a dictionary entry by hash, compare two entries, or check for
regressions against a baseline:

```
nom bench <hash>          # time and record to entry_benchmarks
nom bench compare A B     # compare two benchmark records
nom bench regress <base>  # flag regressions vs baseline
nom bench curate          # curate stored results
```

### nom flow

Record, inspect, diff, or trace middleware for a named execution flow:

```
nom flow record <name>
nom flow show <artifact_id>
nom flow diff <id_a> <id_b>
nom flow middleware <artifact_id>
```

### nom media

Import media files into the dictionary, render or transcode them, and find
similar entries by perceptual hash:

```
nom media import <path>
nom media import-dir <path>
nom media render <hash> <output>
nom media transcode <input> <codec>
nom media diff <hash_a> <hash_b>
nom media similar <hash> --limit N
```

### nom corpus

Stream-and-discard ingestion from package indexes, code hosts, or local
repositories. Source files are never retained after ingestion; disk use at
any moment equals at most the size of the current working repository plus the
dictionary.

```
nom corpus ingest repo <path>
nom corpus ingest pypi <count>
nom corpus ingest github <count>
nom corpus status
nom corpus pause / resume
nom corpus report
nom corpus workspace-gc
```

### nom app

Create, import, build, and inspect application manifests:

```
nom app new <name>
nom app import <path>
nom app build <name>
nom app build-report <name>
nom app explain-selection <selection>
```

### nom ux

Seed UX patterns from a source directory into the dictionary:

```
nom ux seed <path>
```

---

## 6. Extending Nom

### Adding New Kinds

All dispatch is runtime string-based — `production_kind: String` on every
node, `backend_kind: String` on every `FlowNode`. To add a new kind:

1. Insert a row into `grammar.kinds` via the DB or `nom-dict` helpers.
2. Implement the backend logic (a struct that produces a `ComposeResult`).
3. Register the backend with `BackendRegistry::register()` using the same
   kind string.

No Rust enum changes are required; the closed `BackendKind` enum was deleted
in Wave AP.

### Writing Skills

Skills are `SkillDefinition` rows in the dictionary. Each skill carries a
stable `id`, a `name`, a `description` used for fuzzy lookup, and JSON Schema
strings for input and output. Register them with `SkillRouter::register()` or
insert them directly into the `grammar.kinds` table. Nine built-in skills are
seeded in `grammar.kinds.baseline.sql`: `author_nom_app`,
`compose_from_dict`, `debug_nom_closure`, `extend_nom_compiler`,
`ingest_new_ecosystem`, `use_ai_loop`, `compose_brutalist_webpage`,
`compose_generative_art_piece`, and `compose_lofi_audio_loop`.

### Integrating Backends

Implement `ReActLlmFn` to connect any LLM adapter. Four concrete
implementations ship with the crate: `StubLlmFn` (testing), `NomCliLlmFn`
(spawns the `nom` binary as the oracle), `McpLlmFn` (delegates via an MCP
tool call), and `RealLlmFn` (external API endpoint). Pass the adapter to
`AiGlueOrchestrator::new()` to wire it into the AI-leading tier of
`HybridResolver`.
