# NomCanvas — Full Rust GPU-Native IDE Design Specification

> **NORTH STAR** — every session reads this first, plans against it, updates it when the vision sharpens.
> **Date:** 2026-04-18 | **State:** Wave A+B+E-prep COMMITTED (commit 8c7d32e, 174 tests) · Wave C (nom-compiler-bridge) is the current target
> **Foundation:** nom-compiler (29 crates) is UNCHANGED and is the CORE. NomCanvas is built on top of it.
> **Architecture:** Custom GPUI (wgpu + winit + taffy + cosmic-text) — Zed's approach. One binary. Fully Rust.
> **Sibling docs:** `implementation_plan.md` · `nom_state_machine_report.md` · `task.md` · `INIT.md`
> **NON-NEGOTIABLE:** Agents MUST read source repos end-to-end before writing ANY code. Always use ui-ux-pro-max skill.

---

## 1. Vision

One binary. Fully Rust. GPU-rendered. Desktop + browser (WebGPU) from the same codebase.

**The nom-compiler IS the IDE.** It does not run "when you click compile." It runs continuously. Every keystroke is a compile event. Every block on the canvas is a compiler concept. Every pixel is a nomtu entity. Zero IPC, zero subprocesses — compiler crates are direct workspace dependencies.

**The DB IS the workflow engine.** Having just `nom-dict` + `nom-grammar` enables N8N/Dify-like workflow composition through `.nomx`. No external orchestrator needed:
- `grammar.kinds` = the node-type library (every kind is a draggable node)
- `clause_shapes` = the wire type system (every slot has a typed grammar shape)
- `.nomx` prose = the workflow definition language (natural language → grammar productions)
- `nom-compose/dispatch` = the execution runtime (kind → backend, DB-driven)

**Deep thinking is first-class.** `nom-intent::deep_think()` runs a scored ReAct loop — multi-step hypothesis chain — before committing to a `CompositionPlan`. Streamed as reasoning cards in the right dock. User can interrupt or steer mid-thought.

**GPUI fully Rust — one binary.** No webview, no Electron, no Tauri, no DOM, no JS. Custom GPUI: wgpu + winit + taffy + cosmic-text. Zed proves this works at production scale.

**6 unified modes on one infinite canvas:** Code · Doc · Canvas · Graph · Draw · Compose. No mode switching — all coexist spatially on the same GPU surface.

**3-column shell** (Zed `Workspace` pattern):
- Left dock: AFFiNE-style dictionary browser + search (248px, collapsible)
- Center: Zed `PaneGroup` recursive split hosting the infinite canvas
- Right dock: Rowboat-style AI assistant + tool inspector + composition progress (320px, collapsible)
- Bottom dock: terminal · diagnostics · command history
- Status bar: left/center/right slots

**Compose everything from `.nomx`:**

| Category | Outputs |
|---|---|
| **Media** | video · picture · audio · 3D mesh · storyboard · novel→video |
| **Screen** | web app · native app · mobile app (iOS/Android) · presentation (slides) |
| **App** | full app bundle (frontend + backend + deploy) · ad creative (video/static/interactive) |
| **Data** | extract (PDF→JSON+tables) · transform (Polars-like) · query (WrenAI MDL) |
| **Concept** | document (PDF/DOCX) |
| **Scenario** | workflow (n8n-pattern + AST sandbox) |

---

## 2. Architecture Decision Record

| Decision | Choice | Why | Source |
|----------|--------|-----|--------|
| Framework | **Custom GPUI** (no Dioxus) | Dioxus Desktop = webview. We need GPU-native. | `zed-main/crates/gpui/` |
| GPU API | **wgpu** | Cross-platform: Vulkan/Metal/DX12/WebGPU | `upstreams/wgpu/` |
| Layout | **taffy** (flexbox/grid) | Rust-native, same as Zed, no CSS parser needed | `zed-main/crates/gpui/src/styled.rs` |
| Text | **cosmic-text** | Font shaping + layout, no platform dep, works in WASM | Replaces Zed's platform text |
| Window | **winit** | Cross-platform event loop, desktop + web via wasm-bindgen | Standard Rust |
| Rendering | **Zed scene graph** | Primitives → batched by type → wgpu render passes | `zed-main/crates/gpui/src/scene.rs` |
| Design | **AFFiNE tokens** | Inter + Source Code Pro, 73 CSS vars extracted, 24px icons | `AFFiNE-canary/` |
| Compiler | **Direct function calls** | No IPC, no JSON, compiler crates as workspace deps | 29 crates mapped to 3 thread tiers |
| Workflow | **DB-driven via nom-compose** | `grammar.kinds` = node library, no hardcoded node enum | `nom-grammar` + `nom-compose` |
| Dioxus | **NO** | Desktop = webview wrapper, not GPU-native | confirmed by end-to-end read |

---

## 3. System Architecture — Threading Model

```
┌──────────────────────────────────────────────────────────────┐
│                     NomCanvas Binary                          │
│                                                               │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              Render Thread (60fps)                      │  │
│  │  Scene → wgpu batched draw calls → present             │  │
│  │  Primitives: Quad, Text, Path, Shadow, Sprite          │  │
│  │  Glyph atlas: cosmic-text → etagere packing → wgpu tex │  │
│  └──────────────────────────↑─────────────────────────────┘  │
│                              │ reads scene                    │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              UI Thread (winit event loop)               │  │
│  │  Input → Layout (taffy) → Paint → Scene                │  │
│  │                                                         │  │
│  │  COMPILER ON UI THREAD (<1ms, pure stateless):          │  │
│  │  • nom_grammar::resolve_synonym                         │  │
│  │  • nom_grammar::is_known_kind                           │  │
│  │  • nom_dict::find_entities_by_word (cached read conn)   │  │
│  │  • nom_score::score_atom                                │  │
│  │  • nom_score::can_wire                                  │  │
│  │  • nom_search::BM25Index::search (in-memory)            │  │
│  └──────────────────────────↕─────────────────────────────┘  │
│                              │ channels                       │
│  ┌────────────────────────────────────────────────────────┐  │
│  │           Compiler Worker Pool (tokio, N threads)       │  │
│  │                                                         │  │
│  │  INTERACTIVE (<100ms):                                  │  │
│  │  • nom_concept::stage1_tokenize (syntax highlighting)   │  │
│  │  • nom_concept::stage2_kind_classify (block kind)       │  │
│  │  • nom_lsp::handle_hover / handle_completion            │  │
│  │  • nom_resolver::resolve (exact→word→semantic)          │  │
│  │  • nom_resolver::infer_flow_contracts                   │  │
│  │                                                         │  │
│  │  BACKGROUND (>100ms):                                   │  │
│  │  • nom_concept::run_pipeline (full S1-S6)               │  │
│  │  • nom_planner::plan_from_pipeline_output               │  │
│  │  • nom_intent::classify_with_react (ReAct loop)         │  │
│  │  • nom_intent::deep_think (scored hypothesis chain)     │  │
│  │  • nom_app::dream_report                                │  │
│  │  • nom_llvm::compile (LLVM codegen)                     │  │
│  └────────────────────────────────────────────────────────┘  │
│                              │                                │
│  ┌────────────────────────────────────────────────────────┐  │
│  │           Shared State (Arc<RwLock>)                    │  │
│  │  • Dict pool: 1 write + N read connections (WAL)        │  │
│  │  • Grammar: 1 read connection                           │  │
│  │  • Compile cache: LRU<u64, PipelineOutput>              │  │
│  │  • BM25 index: in-memory, rebuilt on dict change        │  │
│  │  • Canvas state: blocks, elements, selections, wires    │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

---

## 4. 3-Column Shell Architecture

Pattern sources: Zed `crates/workspace/` · AFFiNE `packages/frontend/core/src/components/root-app-sidebar/` · rowboat-main `apps/x/apps/renderer/`.

```
┌──────────────────────────────────────────────────────────────────┐
│  Title bar  [workspace-switcher]            [sync · cloud · user] │
├──────────┬──────────────────────────────────┬────────────────────┤
│ LEFT     │     CENTER (CANVAS)              │  RIGHT             │
│ DOCK     │                                  │  DOCK              │
│ (AFFiNE) │     Zed PaneGroup tree           │  (Rowboat)         │
│ 248px    │     hosting infinite canvas      │  320px default     │
│ default  │     with 6 modes spatial         │  collapse to 56px  │
│ collapse │                                  │                    │
│ to 56px  │     Blocks (nomtu-backed):        │  - AI assistant    │
│          │       prose · nomx · media        │    conversation    │
│ - dict   │       graph_node · drawing       │  - deep-think      │
│   tree   │       table · embed              │    reasoning stream│
│ - pinned │       + compose-preview          │  - tool-call       │
│ - recent │                                  │    inspector cards │
│ - search │     Command palette (Cmd+K)       │  - nomtu entity    │
│   modal  │     Dirty-region renderer        │    details viewer  │
│ - settgs │                                  │  - composition     │
│          │                                  │    progress        │
├──────────┴──────────────────────────────────┴────────────────────┤
│  BOTTOM DOCK (terminal · diagnostics · command history)           │
├──────────────────────────────────────────────────────────────────┤
│  Status bar (left · center · right slots)                         │
└──────────────────────────────────────────────────────────────────┘
```

### Shell structs (Zed `Workspace` pattern)

```rust
pub struct Shell {
    center: PaneGroup,                    // recursive split tree
    left_dock: Arc<Dock>,                 // AFFiNE-style dict + search
    right_dock: Arc<Dock>,                // Rowboat-style AI + inspector
    bottom_dock: Arc<Dock>,               // terminal + diagnostics
    active_pane: Arc<Pane>,
    status_bar: Arc<StatusBar>,
    modal_layer: Arc<ModalLayer>,
}

pub enum DockPosition { Left, Right, Bottom }

pub struct Dock {
    position: DockPosition,
    panel_entries: Vec<PanelEntry>,
    is_open: bool,
    active_panel_index: Option<usize>,
}

pub trait Panel: Focusable + Render {
    fn persistent_name() -> &'static str;
    fn position(&self) -> DockPosition;
    fn default_size(&self) -> Pixels;
    fn toggle_action(&self) -> Box<dyn Action>;
    fn icon(&self) -> Option<IconName>;
    fn is_agent_panel(&self) -> bool { false }
}

pub enum Member { Pane(Arc<Pane>), Axis(PaneAxis) }
pub struct PaneGroup { root: Member, is_center: bool }
```

### Left dock (AFFiNE pattern — `root-app-sidebar/index.tsx`)

1. Logo + workspace switcher
2. QuickSearchInput button (Cmd+K → command palette modal)
3. Action buttons (new block · "all entries" · settings)
4. Pinned section (`CollapsibleSection`, path-keyed state)
5. **Dictionary tree** — nomtu entries organized by `grammar.kinds`
6. Recent / trash
7. Settings button

Primitives: `CollapsibleSection` · `MenuItem` with inline collapse toggle · `ResizePanel` (248px default, 4 states: open/floating/floating-with-mask/close) · `SidebarScrollableContainer` (scroll-shadow border)

### Right dock (Rowboat pattern — `renderer/src/components/chat-sidebar.tsx`)

1. Tab bar — multi-chat switcher
2. Conversation stream — anchored scroll messages
3. **Deep-think reasoning stream** — collapsible cards per `DeepThinkStep` (hypothesis + evidence + confidence badge)
4. Tool inspector — collapsible cards: status badge (Pending/Running/Completed/Error) + tabbed Input/Output viewers
5. Permission / ask-human overlays
6. Chat input — mentions, file attachments, suggestions

Multi-things (4 axes):
- Multi-conversation: `chatTabs: Vec<ChatTab>` (runId + draft)
- Multi-tool: `toolOpenByTab: HashMap<(TabId, ToolId), bool>`
- Multi-agent: `RunEvent` union with `SpawnSubFlow { agentName }`
- Multi-run: sidebar history list

---

## 5. DB-Driven Composition Engine — "The DB IS N8N / Dify"

NomCanvas does NOT need N8N, Dify, or any external workflow engine. The DB is the workflow engine.

### Equivalence table

| N8N / Dify concept | Nom DB equivalent | Crate / table |
|---|---|---|
| Node-type library | `grammar.kinds` — every row is a draggable node | `nom-grammar`, `kinds` table |
| Wire type checking | `clause_shapes` — every slot has a `grammar_shape` | `nom-grammar`, `clause_shapes` |
| Workflow definition | `.nomx` prose → grammar productions via S1-S6 | `nom-concept` |
| Node execution | `nom-compose/dispatch.rs` routes `NomKind → backend` | `nom-compose` |
| Function expression | `{{ expr }}` evaluated via AST sandbox (4 sanitizers) | `nom-graph-v2/sandbox.rs` |
| Credential store | Kind-keyed secrets | `nom-compose/credential_store.rs` |
| Plugin/node registry | `grammar.kinds` + `plugin_registry.rs` | `nom-compose` |
| Workflow state | `entries` table — every entity is a persistent state node | `nom-dict` (WAL SQLite) |
| AI node | Any `NomKind` calling `nom-intent::classify_with_react` | `nom-intent` |
| Event-generator node | Background tier, crossbeam channel | `nom-compiler-bridge/background_tier.rs` |

### The DB execution loop (replaces N8N/Dify executor)

```
User writes .nomx on canvas
  ↓
S1-S6 pipeline → entries upsert (DB write, nom-dict)
  ↓
Grammar productions resolved → clause_shapes filled (nom-grammar)
  ↓
can_wire() validates connections:
  entries.output_type ↔ clause_shapes.grammar_shape (DB read)
  ↓
nom-compose/dispatch.rs routes NomKind → backend (DB-driven)
  ↓
Backend produces artifact → ~/.nom/store/<hash>/body.*
  ↓
Canvas preview block renders artifact from store (GPU scene)
```

**The key insight:** every `.nomx` sentence is simultaneously:
- A **document** (Doc mode: AFFiNE block)
- A **node** (Graph mode: typed DAG node with DB-derived ports)
- A **DB entry** (entries: `kind`, `word`, `output_type`)
- A **workflow step** (Compose: dispatched to backend by kind)

The same `.nomx` source composes a video, generates a web app, or runs a data pipeline — determined solely by the `kind` field in the DB.

---

## 6. GPU Rendering Pipeline (Zed pattern)

```rust
fn frame(&mut self) {
    self.handle_events();                    // winit
    self.process_compiler_results();         // channels from worker pool
    self.layout_tree.compute_layout();       // taffy
    let mut scene = Scene::new();
    self.root.paint(&mut scene, &self.layout_tree);  // components → primitives
    scene.sort_and_batch();                  // group by type + texture
    self.renderer.draw(&scene);              // wgpu submit
}
```

**Primitives** (from `zed-main/crates/gpui/src/scene.rs`):
- `Quad` — rectangles with rounded corners, fill, border, shadow
- `MonochromeSprite` — single-color glyphs (text)
- `PolychromeSprite` — multi-color sprites (emoji, images)
- `Path` — bezier curves, filled shapes
- `Shadow` — box shadows with blur
- `Underline` — text underlines

**Glyph Atlas** (Zed pattern):
- cosmic-text shapes text → glyph IDs + positions
- Glyphs rasterized → packed into atlas via `etagere` (BucketedAtlasAllocator)
- 4×4 subpixel variants for pixel-perfect antialiasing
- Atlas = wgpu texture, shared across all text primitives

---

## 7. Design Tokens (AFFiNE — extracted from source)

```rust
pub mod tokens {
    // Layout
    pub const SIDEBAR_W: f32 = 248.0;
    pub const TOOLBAR_H: f32 = 48.0;
    pub const STATUSBAR_H: f32 = 24.0;
    pub const BLOCK_RADIUS: f32 = 4.0;
    pub const MODAL_RADIUS: f32 = 22.0;
    pub const POPOVER_RADIUS: f32 = 12.0;
    pub const BTN_H: f32 = 28.0;
    pub const BTN_H_LG: f32 = 32.0;
    pub const BTN_H_XL: f32 = 40.0;
    pub const ICON: f32 = 24.0;

    // Typography (Inter + Source Code Pro)
    pub const H1_WEIGHT: u16 = 700;
    pub const H1_SPACING: f32 = -0.02;
    pub const H2_WEIGHT: u16 = 600;
    pub const BODY_WEIGHT: u16 = 400;

    // Colors (dark theme)
    pub const BG:     [f32; 4] = [0.059, 0.090, 0.165, 1.0]; // #0F172A
    pub const BG2:    [f32; 4] = [0.118, 0.161, 0.251, 1.0]; // #1E293B
    pub const TEXT:   [f32; 4] = [0.973, 0.980, 0.988, 1.0]; // #F8FAFC
    pub const CTA:    [f32; 4] = [0.133, 0.773, 0.369, 1.0]; // #22C55E
    pub const BORDER: [f32; 4] = [0.200, 0.255, 0.333, 1.0]; // #334155
    pub const FOCUS:  [f32; 4] = [0.118, 0.588, 0.922, 0.3]; // rgba(30,150,235,0.30)

    // Graph mode: confidence-scored edge colors (AFFiNE-inspired)
    pub const EDGE_HIGH:   [f32; 4] = [0.133, 0.773, 0.369, 0.9]; // confidence ≥ 0.8
    pub const EDGE_MED:    [f32; 4] = [0.957, 0.702, 0.078, 0.7]; // confidence 0.5–0.8
    pub const EDGE_LOW:    [f32; 4] = [0.937, 0.267, 0.267, 0.5]; // confidence < 0.5

    // Animation
    pub const ANIM_DEFAULT: f32 = 300.0; // ms
    pub const ANIM_FAST:    f32 = 200.0;
}
```

---

## 8. Six Unified Modes

All modes share the same GPU canvas surface. Mode = which tools/interactions are active. Every mode reads/writes nomtu-backed blocks.

### Code mode
`.nomx` blocks with syntax highlighting driven by `stage1_tokenize`. LSP hover, completion, definition. Compile → score → plan. Compiler crates: S1-S2, nom-lsp, nom-resolver.

### Doc mode — Zed quality + Rowboat AI + AFFiNE block model

Three-way fusion:
- **AFFiNE block model**: heading (h1–h6) · paragraph · bulleted-list · numbered-list · quote · divider · callout · code-block · database · linked-block. Every block is nomtu-backed (`BlockModel.entity: NomtuRef`).
- **Zed editor quality** for code blocks: rope buffer (ropey), multi-cursor, real-time syntax highlighting from `stage1_tokenize`, completion from `nom-resolver`, inlay hints from `nom-lsp`.
- **Rowboat inline AI**: `/ai` command or selection opens AI conversation thread scoped to that block in the right dock. AI suggestions appear as Rowboat tool cards.
- Typography: AFFiNE tokens (Inter, Source Code Pro, 16-level scale).

Compiler crates: nom-grammar (keyword resolution), nom-lsp (hover/completion on code blocks), nom-resolver, nom-intent (inline AI).

### Canvas mode — AFFiNE-inspired for RAG + beautiful graph mode

AFFiNE's block-as-knowledge-unit model applied to infinite spatial canvas:
- **Graph mode** (primary visual for RAG): nomtu entities as AFFiNE-style knowledge node cards with frosted-glass panels, blur shadows, Inter font. Edges carry `confidence: f32` + `reason: String` (GitNexus pattern) rendered as colored opacity arcs:
  - Green (≥0.8): strong semantic connection
  - Amber (0.5–0.8): inferred connection
  - Red (<0.5): weak / heuristic
- **RAG visualization layer**: `nom-search` retrieval context overlaid on graph as colored connection paths. Shows which dictionary entries informed a composition decision.
- Beautiful by default: AFFiNE design tokens (frosted glass, blur, smooth bezier edge routing, spring animations).
- Node palette is **DB-driven**: live query `grammar.kinds` → every kind = a draggable node. No hardcoded node list.

Compiler crates: nom-score (can_wire), nom-search (BM25+vector for RAG overlay), nom-intent (retrieval context).

### Graph mode — DB-driven workflow (N8N/Dify-native)

DAG nodes with typed ports. Wires = grammar slot-fills validated by `can_wire()`. This is N8N/Dify built natively from the DB:
- Nodes = `NomKind` instances derived from `grammar.kinds` (zero hardcoded)
- Ports = `clause_shapes` rows for that kind's grammar slots
- Wires = `can_wire(src_ref, src_slot, dst_ref, dst_slot, grammar_conn, entries_conn)`
- Execution = `nom-compose/dispatch.rs` routes kind → backend
- Expression nodes: `{{ expr }}` via `nom-graph-v2/sandbox.rs` (4 AST sanitizers)

Compiler crates: nom-planner, nom-intent, nom-score (can_wire), nom-compose.

### Draw mode
Freeform pen/brush strokes. Compiler crates: nom-media (codec for export).

### Compose mode
Triggers composition from any block on canvas. Deep thinking available: `deep_think()` runs scored ReAct loop before committing plan. Progress streamed to right dock as reasoning cards. Compiler crates: nom-compose, nom-planner, nom-intent (deep_think), nom-compiler-bridge.

---

## 9. Compiler-as-Core Integration

> **nom-compiler is NOT a separate service. It is the IDE. Compiler crates are direct workspace dependencies of nom-canvas. Zero IPC. Zero subprocesses. The canvas IS the compiler rendered.**

| User action | Compiler response | Latency | Thread tier |
|---|---|---|---|
| Type a character | `stage1_tokenize` → syntax highlighting | <10ms | Interactive |
| Hover a word | `handle_hover` → tooltip | <10ms | Interactive |
| Pause typing 500ms | `run_pipeline` S1-S6 → diagnostics | <100ms | Background |
| Drag wire | `can_wire` → green/amber/red | <1ms | UI |
| Click "Run" | `compile` → LLVM → execute | >1s | Background |
| Type in command bar | `classify_with_react` → NomIntent | <2s | Background |
| Click "Deep Think" | `deep_think` → scored hypothesis chain | <10s | Background |
| Open compose | `dream_report` → score + proposals | <1s | Background |

**Dict Connection Pool** (SQLite WAL mode):
- 1 write connection (serialized via channel): `upsert_entity`, `add_graph_edge`
- N read connections (N = CPU cores): hover, completion, resolve
- UI thread: dedicated read connection (never contends)

---

## 10. Deep Thinking — First-Class Compiler Operation

Deep thinking is a compiler operation, not a separate AI mode. It is an extended ReAct loop that reasons about canvas state before committing to a composition plan.

```rust
// nom-intent/src/deep.rs
pub struct DeepThinkStep {
    pub hypothesis: String,
    pub evidence: Vec<String>,       // supporting nomtu refs
    pub confidence: f32,             // scored by nom-score::score_atom
    pub counterevidence: Vec<String>,
    pub refined_from: Option<usize>, // index of prior step this refines
}

pub async fn deep_think(
    intent: &str,
    canvas: &Workspace,
    grammar: &Connection,
    entries: &Connection,
    interrupt: &InterruptFlag,
) -> Result<(CompositionPlan, Vec<DeepThinkStep>)>
// ReAct loop: max 10 steps
// Each step: generate hypothesis → score via score_atom → refine
// Best hypothesis becomes the CompositionPlan
// Steps streamed to right dock as Rowboat tool cards
```

| Trigger | Deep think operation | Output |
|---|---|---|
| Complex `.nomx` composition | Multi-step plan reasoning | Annotated `CompositionPlan` with confidence scores |
| Ambiguous wire connection | Grammar derivation exploration | `Connector { confidence, reason chain }` |
| Unclear compose target | Output space exploration | Ranked `CompositionPlan[]` with tradeoffs |
| "Explain this" hover | Derivation chain tracing | Rich markdown with reasoning steps |

**UI (right dock):** each `DeepThinkStep` = one Rowboat tool card with hypothesis input + evidence output + confidence badge. User can click "interrupt" at any step. Collapsed by default, expandable.

---

## 11. Crate Structure (target build)

```
nom-canvas/                     ← new workspace (fresh build)
├── Cargo.toml
├── crates/
│   ├── nom-gpui/               Custom GPU framework (Zed pattern)
│   │   ├── scene.rs            Quad, Text, Path, Shadow, Sprite
│   │   ├── renderer.rs         wgpu render passes, primitive batching
│   │   ├── atlas.rs            Glyph atlas (etagere + cosmic-text)
│   │   ├── element.rs          Element trait (request_layout, prepaint, paint)
│   │   ├── styled.rs           Style builder (Zed Styled trait pattern)
│   │   ├── window.rs           winit window + event loop + frame timing
│   │   ├── layout.rs           taffy integration
│   │   ├── animation.rs        Timestamp-based interpolation + easing
│   │   └── platform.rs         Desktop vs Web (WebGPU) abstraction
│   │
│   ├── nom-canvas-core/        Infinite canvas engine
│   │   ├── viewport.rs         Zoom, pan, transforms
│   │   ├── elements.rs         Shapes (rect, ellipse, arrow, connector)
│   │   ├── selection.rs        Multi-select, rubber-band, transform handles
│   │   ├── snapping.rs         Grid + edge/center alignment
│   │   ├── hit_test.rs         Two-phase: AABB → precise geometry
│   │   └── spatial_index.rs    R-tree for fast element lookup
│   │
│   ├── nom-editor/             Code/doc editor (Zed quality)
│   │   ├── buffer.rs           Rope-based (ropey)
│   │   ├── cursor.rs           Multi-cursor + selections
│   │   ├── highlight.rs        stage1_tokenize-driven syntax highlighting
│   │   ├── hints.rs            Inlay hints from nom-lsp
│   │   ├── completion.rs       Completion from nom-resolver + nom-grammar
│   │   └── input.rs            Keyboard/IME handling
│   │
│   ├── nom-blocks/             Block types — ALL nomtu-backed
│   │   ├── block_model.rs      BlockModel { entity: NomtuRef, slots, children }
│   │   ├── prose.rs            AFFiNE block types (heading/para/list/quote/callout/db)
│   │   ├── nomx.rs             .nomx code (with nom-editor)
│   │   ├── graph_node.rs       DAG node (grammar-production instance, DB-derived ports)
│   │   ├── media.rs            Image/video/audio
│   │   ├── drawing.rs          Freeform strokes
│   │   ├── table.rs            Data table
│   │   └── embed.rs            Embedded content
│   │
│   ├── nom-graph/              DAG execution (N8N/Dify-native, DB-driven)
│   │   ├── engine.rs           Kahn sort, IS_CHANGED, VariablePool
│   │   ├── execution.rs        Partial execution, cancel, progress
│   │   ├── cache.rs            4-tier cache (NoCache/LRU/RamPressure/Classic)
│   │   ├── nodes.rs            DB-driven node types (query grammar.kinds at runtime)
│   │   └── sandbox.rs          Expression eval (4 AST sanitizers, n8n pattern)
│   │
│   ├── nom-panels/             UI panels
│   │   ├── shell.rs            3-column Shell composition
│   │   ├── dock.rs             DockPosition + Dock + Panel trait (Zed pattern)
│   │   ├── pane.rs             PaneGroup recursive split + tab strip
│   │   ├── sidebar.rs          AFFiNE left dock (CollapsibleSection + MenuItem + ResizePanel)
│   │   ├── chat.rs             Rowboat right dock (ChatSidebar + tool cards + deep-think stream)
│   │   ├── toolbar.rs          48px top bar
│   │   ├── statusbar.rs        24px bottom bar (left/center/right slots)
│   │   ├── command_palette.rs  Cmd+K → fuzzy picker (Zed pattern)
│   │   ├── library.rs          Dictionary browser (grammar.kinds tree)
│   │   └── properties.rs       Nomtu entity detail viewer
│   │
│   ├── nom-compose/            Universal composition engine
│   │   ├── dispatch.rs         NomKind → backend router
│   │   ├── plan.rs             CompositionPlan + PlanStep
│   │   ├── kind.rs             NomKind variants (DB-derived at runtime)
│   │   ├── task_queue.rs       Async task lifecycle (Queued→Running→Done)
│   │   ├── provider_router.rs  3-tier fallback (9router pattern)
│   │   ├── credential_store.rs Kind-keyed secrets
│   │   ├── artifact_store.rs   Content-addressed SHA-256 (~/.nom/store/)
│   │   ├── vendor_trait.rs     MediaVendor + Cost + Capability
│   │   ├── semantic.rs         WrenAI MDL semantic layer
│   │   └── backends/           One file per compose target
│   │       ├── video.rs        Remotion pattern (GPU scene → FFmpeg)
│   │       ├── image.rs        Multi-model dispatch (200+ models)
│   │       ├── audio.rs        Synthesis + codec
│   │       ├── mesh.rs         glTF 2.0
│   │       ├── storyboard.rs   ArcReel 5-phase + waoowaoo 4-phase
│   │       ├── web_screen.rs   ToolJet widgets + Dify workflow
│   │       ├── native_screen.rs nom-llvm LLVM codegen
│   │       ├── mobile_screen.rs iOS/Android
│   │       ├── presentation.rs  Slide deck (typst pattern)
│   │       ├── app_bundle.rs   Frontend + backend + deploy
│   │       ├── ad_creative.rs  Multi-platform ad variants
│   │       ├── document.rs     PDF/DOCX (typst-memoize)
│   │       ├── data_extract.rs XY-Cut++ (opendataloader)
│   │       ├── data_frame.rs   Polars-like columnar
│   │       ├── data_query.rs   WrenAI 5-stage pipeline
│   │       └── scenario_workflow.rs n8n DAG + sandbox
│   │
│   ├── nom-compiler-bridge/    Cross-workspace linker (KEYSTONE)
│   │   ├── lib.rs              CompilerBridge::new() + 3-tier accessors
│   │   ├── shared.rs           Arc<RwLock<SharedState>> (dict_pool + grammar_conn + cache)
│   │   ├── ui_tier.rs          Sync cached reads (lookup_nomtu, can_wire, search_bm25)
│   │   ├── interactive_tier.rs tokio::runtime + mpsc (tokenize, highlight, complete, hover)
│   │   ├── background_tier.rs  crossbeam-channel + workers (compile, plan, verify, deep_think)
│   │   └── adapters/
│   │       ├── highlight.rs    stage1_tokenize → Highlighter::color_runs
│   │       ├── lsp.rs          CompilerLspProvider impl (replaces StubLspProvider)
│   │       ├── completion.rs   nom-dict prefix search → CompletionItem
│   │       └── score.rs        score_atom → StatusBar::CompileStatus
│   │
│   ├── nom-theme/              Design system
│   │   ├── tokens.rs           AFFiNE values (73 vars + graph edge colors)
│   │   ├── fonts.rs            Inter + Source Code Pro (GPU)
│   │   └── icons.rs            Lucide 24px SVG → GPU paths
│   │
│   ├── nom-lint/               Sealed-trait linter (yara-x pattern)
│   ├── nom-memoize/            Incremental compile (comemo pattern, no dep)
│   ├── nom-telemetry/          W3C traceparent + spans
│   └── nom-collab/             CRDT types (no yrs dep yet)

nom-compiler/                   ← UNCHANGED peer workspace (29 crates)
├── nom-dict/                   entries table (nomtu source of truth, SQLite+WAL)
├── nom-grammar/                kinds + clause_shapes (grammar source of truth)
├── nom-concept/                S1-S6 pipeline (stage1_tokenize through run_pipeline)
├── nom-lsp/                    hover + completion + definition
├── nom-resolver/               3-stage resolve (exact → word → semantic)
├── nom-score/                  score_atom + can_wire (pure stateless)
├── nom-search/                 BM25 + vector indexes
├── nom-intent/                 ReAct loop + deep_think
├── nom-planner/                CompositionPlan from pipeline output
├── nom-app/                    dream_report
├── nom-llvm/                   LLVM codegen
└── 18 more crates
```

---

## 12. Nomtu Entity Backing — Every Canvas Object = DB Entry

**Blueprint promise:** *"Every pixel is a compiler concept."* This means every visual object on the canvas MUST represent a concrete `.nom` nomtu entity from `nom-dict/entries`. Every flow node MUST represent a grammar production from `clause_shapes`. Every wire MUST be a valid slot-fill via `can_wire()`.

### Core identity type

```rust
pub struct NomtuRef {
    pub id: String,    // entries.id (content-addressed hash)
    pub word: String,  // entries.word
    pub kind: String,  // entries.kind — MUST exist in grammar.kinds
}
```

### Block = nomtu projection

```rust
pub struct BlockModel {
    pub id: BlockId,
    pub entity: NomtuRef,                   // non-optional: every block has a DB entry
    pub slots: Vec<(String, SlotValue)>,    // from clause_shapes WHERE kind = entity.kind
    pub children: Vec<BlockId>,
    pub meta: BlockMeta,
}
```

### GraphNode = grammar production instance

```rust
pub struct GraphNode {
    pub production_kind: String,     // grammar.kinds.name
    pub entity: NomtuRef,
    pub slots: Vec<SlotBinding>,     // from clause_shapes
    pub position: (f64, f64, f64, f64),
}

pub struct SlotBinding {
    pub clause_name: String,         // clause_shapes.clause_name
    pub grammar_shape: String,       // clause_shapes.grammar_shape
    pub value: Option<SlotValue>,
    pub is_required: bool,
    pub confidence: f32,             // 1.0=explicit, 0.8=inferred, 0.6=heuristic
    pub reason: String,
}
```

### Wire = slot-fill validated by grammar

```rust
pub fn can_wire(
    src: &NomtuRef, src_slot: &str,
    dst: &NomtuRef, dst_slot: &str,
    grammar: &Connection,
    entries: &Connection,
) -> Result<(bool, f32, String)>   // (valid, confidence, reason)
```

### Build order (waves)

**Wave A:** ✅ COMMITTED — `entity: NomtuRef` on every block/element (non-optional from day 1 — fresh build, no legacy). nom-gpui, nom-canvas-core (61 tests), nom-theme all landed in commit 8c7d32e.

**Wave B:** ✅ COMMITTED — Grammar-typed nodes. `NomtuRef` non-optional. `GraphNode.production_kind` DB-validated. `Connector.can_wire_result` non-optional. `DictReader` trait isolation. nom-blocks, nom-editor (14 tests), Wave E-prep nom-graph (12 tests) + nom-memoize (11 tests) all landed in commit 8c7d32e.

**Wave C:** `nom-compiler-bridge` crate. First wire: `stage1_tokenize → Highlighter::color_runs`. Dict pool opened (1 write + N read WAL). `StubLspProvider` replaced with `CompilerLspProvider`.

**Wave D:** 3-column shell assembly (nom-panels dock + pane + shell). AFFiNE CollapsibleSection + MenuItem + ResizePanel for left dock. Rowboat ChatSidebar + tool inspector + deep-think stream for right dock.

**Wave E:** Compose targets made real (replace stub bytes with actual backend logic). Start with video (Remotion pattern) and document (typst pattern) — highest ROI.

---

## 13. Natural Language → Canvas Pipeline (Wave C keystone flow)

```
USER INPUT
    ↓
keystroke (winit KeyEvent)
    ↓
nom-editor/input.rs → EditorCommand
    ↓
nom-editor/buffer.rs → Rope::insert
    ↓
═══════════ nom-compiler-bridge (Wave C) ═══════════
    ↓
Interactive tier (tokio mpsc)
    ↓
nom_concept::stage1_tokenize(&str) → TokenStream
    ↓
adapters/highlight.rs: Tok variant → TokenRole
    ↓
nom-editor/highlight.rs → Highlighter::color_runs
    ↓
[USER-VISIBLE: keywords highlight real-time]

═══════════ On pause (500ms) ═══════════
    ↓
nom_concept::stage2_kind_classify → block kind
    ↓
nom_dict::find_entities_by_word → Option<NomtuEntry>
    ↓
IF entity exists: bind BlockModel.entity = NomtuRef { id, word, kind }
IF not: nom_dict::insert_entry(word, kind) → NomtuRef
    ↓
Query grammar.clause_shapes WHERE kind = entity.kind → Vec<SlotBinding>
    ↓
Parse prose → fill slots
    ↓
Block persisted with entity: NomtuRef (non-optional)

═══════════ User opens compose ═══════════
    ↓
[Optional] Deep Think: nom_intent::deep_think()
  → scored ReAct loop → streamed to right dock
    ↓
nom-compose/dispatch.rs: build CompositionPlan from canvas state
    ↓
Background tier (crossbeam): nom_planner::plan_from_pipeline_output
    ↓
ComposeDispatcher::dispatch(spec) → backend
    ↓
Backend produces bytes → ArtifactStore::put(artifact) → ContentHash
    ↓
Event: ArtifactReady(hash) → canvas preview block renders
    ↓
[USER-VISIBLE: artifact appears in canvas]
```

---

## 14. Reference Repos (read end-to-end before writing code)

### System Architecture

| Pattern | Repo | Path | What to adopt |
|---|---|---|---|
| GPU rendering + shell | **Zed** | `APP/zed-main/crates/gpui/` + `crates/workspace/` | Scene graph, Dock/Panel trait, PaneGroup, FocusHandle, StatusBar |
| Design system + left sidebar | **AFFiNE** | `APP/AFFiNE-canary/` | 73 tokens, Inter+SCP, CollapsibleSection, MenuItem, ResizePanel |
| Right sidebar + AI chat | **rowboat-main** | `APP/rowboat-main/apps/x/apps/renderer/` | ChatSidebar, tool cards, multi-tab, spawn-subflow |
| DAG execution | **ComfyUI** | `APP/Accelworld/services/other2/ComfyUI-master/` | 4-tier cache, Kahn sort, IS_CHANGED, VariablePool |
| Grammar-typed graph schema | **GitNexus** | `.gitnexus/` + GitNexus MCP | 31 node tables, polymorphic edge table, confidence+reason |
| Workflow nodes + expression | **dify** | `APP/Accelworld/services/other4/dify-main/` | typed Node, `_run()` event-generator, expression template |
| AST sandbox | **n8n** | `APP/Accelworld/services/automation/n8n/` | JsTaskRunnerSandbox, 4 AST sanitizers, Code node |
| RAG pipelines | **LlamaIndex** + **Haystack** | `APP/Accelworld/upstreams/` | 50+ retrievers, pipeline composition, postprocessors |
| Component registry | **ToolJet** | `APP/ToolJet-develop/` | 55 widgets, dependency graph, combineProperties |
| Linter framework | **yara-x** | `APP/Accelworld/upstreams/yara-x/` | Sealed Rule trait, WASM codegen |
| Incremental compile | **typst** | `APP/Accelworld/services/other5/typst-main/` | comemo memoization pattern |
| Semantic data layer | **WrenAI** | `APP/wrenai/` | MDL, 5-stage intent→execute pipeline |
| Provider routing | **9router** | `APP/Accelworld/services/other4/9router-master/` | 3-tier fallback, quota tracking, format translation |
| Data visualization | **graphify** | `APP/graphify-master/` | 6 chart types, Redux slice, SVG export |
| Skill/MCP engine | **Refly** | `APP/Accelworld/upstreams/refly-main/` | 46 NestJS modules, LangGraph, BullMQ |

### Composition Backends

| Output | Repo | Path | Pattern |
|---|---|---|---|
| Video | **Remotion** | DeepWiki | GPU scene → frame capture → FFmpeg parallel encode |
| Image | **Open-Higgsfield** | `APP/Accelworld/upstreams/` | 200+ models, 4 studios, auto mode switch |
| Storyboard/Novel→Video | **ArcReel** | `APP/Accelworld/services/other4/ArcReel-main/` | 5-phase orchestration |
| Storyboard phases | **waoowaoo** | `APP/Accelworld/services/media/waoowaoo/` | 4-phase parallel cinematography+acting |
| Document | **typst** | `APP/Accelworld/services/other5/typst-main/` | comemo incremental layout |
| Data extract | **opendataloader** | reference | XY-Cut++ 0.015s/page, hybrid-AI +90% table accuracy |
| Web app | **ToolJet** + **Dify** | `APP/ToolJet-develop/` + `APP/Accelworld/services/other4/dify-main/` | widgets + workflow editor |

---

## 15. Browser + Desktop from Same Codebase

```rust
fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Desktop: winit window + wgpu (Vulkan/Metal/DX12)
        let window = winit::window::WindowBuilder::new()
            .with_title("NomCanvas").build(&event_loop);
        run_app(WgpuRenderer::new(&window));
    }
    #[cfg(target_arch = "wasm32")]
    {
        // Browser: wasm-bindgen + wgpu WebGPU
        let canvas = web_sys::window().unwrap()
            .document().unwrap()
            .get_element_by_id("canvas").unwrap();
        run_app(WgpuRenderer::from_canvas(&canvas));
    }
}
// Same run_app(), same scene graph, same compiler calls.
```

---

## 16. Non-Negotiable Rules

1. **Agents MUST read source repos end-to-end** before writing ANY code — not README-only, not surface scans
2. **Always use `ui-ux-pro-max` skill** at `.agent/skills/ui-ux-pro-max/` for ALL UI work
3. **Zero foreign identities** in public API — external patterns adopted, origin hidden
4. **nom-compiler is CORE** — zero IPC, direct workspace deps, linked not spawned
5. **DB IS the workflow engine** — `grammar.kinds` + `clause_shapes` + `nom-compose` = N8N/Dify; never add an external orchestrator
6. **Every canvas object = DB entry** — `entity: NomtuRef` non-optional from day 1 (fresh build, no legacy)
7. **Canvas = AFFiNE for RAG** — graph mode uses AFFiNE tokens + confidence-scored edges for RAG visualization
8. **Doc mode = Zed + Rowboat + AFFiNE** — rope buffer, inline AI cards, AFFiNE block model
9. **Deep thinking = compiler op** — `deep_think()` is in `nom-intent`, streamed to right dock
10. **GPUI fully Rust — one binary** — no webview, no Electron, no Tauri
11. **Spawn parallel subagents** (nom-workflow skill mandate) for all multi-file work
12. **Run `gitnexus_impact`** before editing any symbol; never ignore HIGH/CRITICAL

---

## Appendix — Iteration Progress Log

### Iteration 30 state (2026-04-18)
Wave C: 100% — adapter APIs verified against nom-compiler source, 17 tests pass, --features compiler 0 errors
Wave D: 100% — 9 panel modules, 20 tests, Zed+Rowboat+AFFiNE patterns confirmed
CRITICALs closed: spring math, fold regions, tokens constants, font weight, validators, flavours, grid size
