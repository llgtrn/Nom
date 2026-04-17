# NomCanvas — Full Rust GPU-Native IDE Design Specification

> **CANONICAL TRACKING DOC — MAIN DESIGN SPEC** (Planner/Auditor refreshes every cycle)
> **Date:** 2026-04-17 | **HEAD:** `56604c4` (wave-10 landed, **1272 tests**, 13 crates shipped). **⚠️ "Compose by natural language on canvas" promise: 0% delivered** (iter-17 audit) — input path (prose→compiler) dead in 6 wires, output path (artifact→canvas) dead in 5 wires. Keystone: `stage1_tokenize → Highlighter` adapter (~200 LOC).
> **Status:** DESIGN → IMPLEMENTATION. Phase 1 ✅ · Phase 2 100% ✅ · Phase 3 ~100% ✅ · Phase 4/5 scaffolded (nom-compose 94 tests, nom-graph-v2 64, nom-lint 39, nom-telemetry 36, nom-collab 33, nom-memoize 17). **Compiler-as-core integration still 0% runtime** (bridge crate spec designed iter-16, not yet implemented). **Vendoring 58% integrated** (8 DEEP · 3 PATTERN · 6 REF · 2 NOT-USED).
> **Sibling tracking docs:** `implementation_plan.md`, `nom_state_machine_report.md`, `task.md` (all 4 MUST stay in sync)
> **Architecture:** Custom GPUI (wgpu + winit + taffy + cosmic-text) — Zed's approach
> **FOUNDATION:** Everything is built around the Nom language. The compiler IS the IDE. The dictionary IS the knowledge base. The grammar IS the type system. The .nomx format IS the universal input. External patterns are studied and ABSTRACTED into Nom-native implementations — zero foreign identities, zero wrappers, zero adapters.
> **End-to-end readings:** 9 repos fully read (Zed, AFFiNE, ComfyUI, Refly, LlamaIndex, Haystack, ToolJet, n8n, yara-x, typst, Dioxus) + all 10 nom-compiler crates
> **NON-NEGOTIABLE:** Executing agents MUST read source repos end-to-end before writing ANY code. Always use ui-ux-pro-max skill.

---

## 1. Vision

One binary. Fully Rust. GPU-rendered. Browser + desktop from same codebase. The nom-compiler doesn't run "when you click compile" — it runs **continuously**, rendering its own state as the UI. Every pixel is a compiler concept.

**5 unified modes on one infinite canvas:** Code + Doc + Canvas + Graph + Draw. No mode switching — all coexist spatially.

---

## 2. Architecture Decision Record

| Decision | Choice | Why | Source |
|----------|--------|-----|--------|
| Framework | **Custom GPUI** (no Dioxus) | Dioxus Desktop = webview. We need GPU-native. Zed proves custom GPUI works. | `zed-main/crates/gpui/` end-to-end reading |
| GPU API | **wgpu** | Cross-platform (Vulkan/Metal/DX12/WebGPU). Compiles to browser via WebGPU. | `upstreams/wgpu/` |
| Layout | **taffy** (flexbox/grid) | Same as Zed. Rust-native. No CSS parser needed. | `zed-main/crates/gpui/src/styled.rs` |
| Text | **cosmic-text** | Font shaping + layout. No platform dependency. Works in WASM. | Replaces Zed's platform text system |
| Window | **winit** | Cross-platform window/event loop. Desktop + (future) web via wasm-bindgen. | Standard Rust windowing |
| Rendering | **Zed's Scene Graph pattern** | Primitives (Quad, Text, Path, Shadow) → batched by type → wgpu render passes | `zed-main/crates/gpui/src/scene.rs` |
| Design language | **AFFiNE tokens** | Inter + Source Code Pro, 73 CSS variables extracted, 24px icons | `AFFiNE-canary/` end-to-end reading |
| Compiler integration | **Direct function calls** | No IPC. No JSON. Compiler crates linked as dependencies. | 10 crates mapped to 3 thread tiers |

---

## 3. System Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                     NomCanvas Binary                       │
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
│  │  COMPILER ON UI THREAD (<1ms):                          │  │
│  │  • nom_grammar::resolve_synonym                         │  │
│  │  • nom_grammar::is_known_kind                           │  │
│  │  • nom_dict::find_entities_by_word (cached read conn)   │  │
│  │  • nom_score::score_atom (pure, stateless)              │  │
│  │  • nom_score::can_wire (pure, stateless)                │  │
│  │  • nom_search::BM25Index::search (in-memory)            │  │
│  └──────────────────────────↕─────────────────────────────┘  │
│                              │ channels                       │
│  ┌────────────────────────────────────────────────────────┐  │
│  │           Compiler Worker Pool (tokio, N threads)       │  │
│  │                                                         │  │
│  │  INTERACTIVE (<100ms):                                  │  │
│  │  • nom_concept::stage1_tokenize (syntax highlighting)   │  │
│  │  • nom_concept::stage2_kind_classify (block kind)       │  │
│  │  • nom_lsp::handle_hover (dict lookup + markdown)       │  │
│  │  • nom_lsp::handle_completion (entities + patterns)     │  │
│  │  • nom_lsp::handle_definition (source location)         │  │
│  │  • nom_resolver::resolve (3-stage: exact→word→semantic) │  │
│  │  • nom_resolver::infer_flow_contracts (type propagation)│  │
│  │                                                         │  │
│  │  BACKGROUND (>100ms):                                   │  │
│  │  • nom_concept::run_pipeline (full S1-S6)               │  │
│  │  • nom_planner::plan_from_pipeline_output               │  │
│  │  • nom_verifier::verify                                 │  │
│  │  • nom_app::dream_report                                │  │
│  │  • nom_security::scan_body                              │  │
│  │  • nom_intent::classify_with_react (ReAct loop)         │  │
│  │  • nom_llvm::compile (LLVM codegen)                     │  │
│  │  • nom_extract::extract_from_dir (tree-sitter)          │  │
│  └────────────────────────────────────────────────────────┘  │
│                              │                                │
│  ┌────────────────────────────────────────────────────────┐  │
│  │           Shared State (Arc<RwLock>)                    │  │
│  │  • Dict pool: 1 write + N read connections (WAL mode)  │  │
│  │  • Grammar: 1 read connection                           │  │
│  │  • Compile cache: LRU<u64, PipelineOutput>              │  │
│  │  • BM25 index: in-memory, rebuilt on dict change        │  │
│  │  • Canvas state: blocks, elements, selections, wires   │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

---

## 4. GPU Rendering Pipeline (from Zed end-to-end reading)

```rust
// Per-frame cycle (target: <16ms)
fn frame(&mut self) {
    // 1. Process input events (winit)
    self.handle_events();
    
    // 2. Update reactive state (compiler results arrive via channels)
    self.process_compiler_results();
    
    // 3. Layout pass (taffy flexbox/grid)
    self.layout_tree.compute_layout();
    
    // 4. Paint pass — components emit scene primitives
    let mut scene = Scene::new();
    self.root.paint(&mut scene, &self.layout_tree);
    
    // 5. Sort + batch primitives (group by type + texture)
    scene.sort_and_batch();
    
    // 6. Submit to wgpu
    self.renderer.draw(&scene);
}
```

**Primitives** (from `zed-main/crates/gpui/src/scene.rs`):
- `Quad` — rectangles with rounded corners, fill, border, shadow
- `MonochromeSprite` — single-color glyphs (text)
- `PolychromeSprite` — multi-color sprites (emoji, images)
- `Path` — bezier curves, filled shapes
- `Shadow` — box shadows with blur
- `Underline` — text underlines

**Glyph Atlas** (from Zed):
- cosmic-text shapes text → glyph IDs + positions
- Glyphs rasterized to bitmap, packed into atlas via `etagere` (BucketedAtlasAllocator)
- 4×4 subpixel variants for pixel-perfect antialiasing
- Atlas is a wgpu texture, shared across all text primitives

---

## 5. Design Tokens (from AFFiNE end-to-end reading)

```rust
// Extracted from AFFiNE source — pixel-perfect values
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
    pub const H1_SPACING: f32 = -0.02; // em
    pub const H2_WEIGHT: u16 = 600;
    pub const BODY_WEIGHT: u16 = 400;
    pub const QUOTE_LINE_H: f32 = 26.0;
    pub const QUOTE_BAR_W: f32 = 2.0;
    pub const QUOTE_BAR_R: f32 = 18.0;
    
    // Colors (Nom dark theme)
    pub const BG: [f32; 4] = [0.059, 0.090, 0.165, 1.0];       // #0F172A
    pub const BG2: [f32; 4] = [0.118, 0.161, 0.251, 1.0];      // #1E293B
    pub const TEXT: [f32; 4] = [0.973, 0.980, 0.988, 1.0];      // #F8FAFC
    pub const CTA: [f32; 4] = [0.133, 0.773, 0.369, 1.0];      // #22C55E
    pub const BORDER: [f32; 4] = [0.200, 0.255, 0.333, 1.0];   // #334155
    pub const FOCUS: [f32; 4] = [0.118, 0.588, 0.922, 0.3];    // rgba(30,150,235,0.30)
    
    // Animation
    pub const ANIM_DEFAULT: f32 = 300.0; // ms
    pub const ANIM_FAST: f32 = 200.0;
    pub const ANIM_SWITCH: &str = "cubic-bezier(0.27, 0.2, 0.25, 1.51)";
    
    // Shadows (from AFFiNE)
    pub const SHADOW_BTN: Shadow = Shadow { x: 0.0, y: 1.0, blur: 5.0, color: [0,0,0,0.12] };
}
```

---

## 6. Five Unified Modes

All modes share the same GPU canvas surface. Mode = which tools/interactions are active.

| Mode | Block Types | Compiler Crates Used | Tool Palette |
|------|------------|---------------------|-------------|
| **Code** | .nomx blocks with syntax highlighting | S1-S2 (tokenize+classify), nom-lsp, nom-resolver | Compile, Score, Plan |
| **Doc** | Rich text (headings, lists, quotes, tables) | nom-grammar (keyword resolution) | Format, Insert, Style |
| **Canvas** | Shapes, connectors, images, spatial layout | nom-score (can_wire for connections) | Shape, Arrow, Text, Image |
| **Graph** | DAG nodes with typed ports, wires | nom-planner (plan_from_pipeline_output), nom-intent | Run, Debug, Step |
| **Draw** | Freeform pen/brush strokes | nom-media (codec for export) | Pen, Brush, Eraser, Color |

---

## 7. Compiler-as-Core Integration

**The compiler doesn't "run when you click compile." It runs continuously.**

| User Action | Compiler Response | Latency | Thread |
|-------------|------------------|---------|--------|
| Type a character | `stage1_tokenize` → syntax highlighting | <10ms | Interactive |
| Hover a word | `handle_hover` → tooltip with contracts/effects | <10ms | Interactive |
| Pause typing (500ms) | `run_pipeline` S1-S6 → diagnostics | <100ms | Background |
| Drag wire between blocks | `can_wire` → green/amber/red | <1ms | UI |
| Click "Run" | `compile` → LLVM bitcode → execute | >1s | Background |
| Type in command bar | `classify_with_react` → NomIntent | <2s | Background |
| Open dream dashboard | `dream_report` → score + proposals | <1s | Background |

**Dict Connection Pool:**
- SQLite WAL mode allows concurrent readers
- 1 write connection (serialized via channel) for `upsert_entity`, `add_graph_edge`
- N read connections (N = CPU cores) for hover, completion, resolve
- UI thread gets dedicated read connection (never contends)

---

## 8. Crate Structure

```
nom-canvas/
├── Cargo.toml              # workspace
├── crates/
│   ├── nom-gpui/           # Custom GPU framework (Zed pattern)
│   │   ├── scene.rs        # Quad, Text, Path, Shadow, Sprite
│   │   ├── renderer.rs     # wgpu render passes, primitive batching
│   │   ├── atlas.rs        # Glyph atlas (etagere + cosmic-text)
│   │   ├── element.rs      # Element trait (request_layout, prepaint, paint)
│   │   ├── styled.rs       # Style builder (like Zed's Styled trait)
│   │   ├── window.rs       # winit window + event loop + frame timing
│   │   ├── layout.rs       # taffy integration
│   │   ├── animation.rs    # Timestamp-based interpolation + easing
│   │   └── platform.rs     # Desktop (native) vs Web (WebGPU) abstraction
│   │
│   ├── nom-canvas-core/    # Canvas engine
│   │   ├── viewport.rs     # Infinite canvas: zoom, pan, transforms
│   │   ├── elements.rs     # Shapes (rect, ellipse, arrow, line, connector)
│   │   ├── selection.rs    # Multi-select, rubber-band, transform handles
│   │   ├── snapping.rs     # Grid + edge/center alignment
│   │   ├── hit_test.rs     # Two-phase: AABB → precise geometry
│   │   └── spatial_index.rs # R-tree for fast element lookup
│   │
│   ├── nom-editor/         # Code/doc editor (Zed quality)
│   │   ├── buffer.rs       # Rope-based (ropey crate)
│   │   ├── cursor.rs       # Multi-cursor + selections
│   │   ├── highlight.rs    # S1-S2 driven syntax highlighting
│   │   ├── hints.rs        # Inlay hints from nom-lsp
│   │   ├── completion.rs   # Completion from nom-resolver + nom-grammar
│   │   └── input.rs        # Keyboard/IME handling
│   │
│   ├── nom-blocks/         # Block types (AFFiNE pattern)
│   │   ├── prose.rs        # Rich text (headings, lists, quotes)
│   │   ├── nomx.rs         # .nomx code (with editor)
│   │   ├── graph_node.rs   # DAG node (ComfyUI pattern)
│   │   ├── media.rs        # Image/video/audio
│   │   ├── drawing.rs      # Freeform strokes
│   │   ├── table.rs        # Data table
│   │   └── embed.rs        # Embedded content
│   │
│   ├── nom-graph/          # DAG execution (ComfyUI+n8n patterns)
│   │   ├── engine.rs       # Kahn sort, IS_CHANGED, VariablePool
│   │   ├── execution.rs    # Partial execution, cancel, progress
│   │   ├── cache.rs        # LRU + RAM-pressure (ComfyUI pattern)
│   │   └── nodes.rs        # Built-in node types
│   │
│   ├── nom-panels/         # UI panels
│   │   ├── sidebar.rs      # 248px collapsible (AFFiNE)
│   │   ├── toolbar.rs      # 48px top bar
│   │   ├── preview.rs      # Output preview (6 modes)
│   │   ├── library.rs      # Dictionary browser
│   │   ├── properties.rs   # Property inspector
│   │   ├── command.rs      # Command palette
│   │   └── statusbar.rs    # 24px bottom bar
│   │
│   └── nom-theme/          # Design system
│       ├── tokens.rs       # AFFiNE values (73 variables)
│       ├── fonts.rs        # Inter + Source Code Pro (GPU)
│       └── icons.rs        # Lucide 24px SVG → GPU paths
│
├── nom-compiler/           # UNCHANGED — 29 crates, linked as deps
│   └── crates/
│       ├── nom-dict/       # → Shared state (Dict pool)
│       ├── nom-grammar/    # → UI thread (resolve_synonym)
│       ├── nom-concept/    # → Interactive + background (S1-S6)
│       ├── nom-lsp/        # → Interactive (hover, completion)
│       ├── nom-resolver/   # → Interactive (resolve, type inference)
│       ├── nom-score/      # → UI thread (score_atom, can_wire)
│       ├── nom-intent/     # → Background (ReAct loop)
│       ├── nom-planner/    # → Background (plan generation)
│       ├── nom-app/        # → Background (dream_report, build)
│       ├── nom-llvm/       # → Background (LLVM codegen)
│       └── ... (19 more)
```

---

## 9. Browser + Desktop from Same Codebase

```rust
fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Desktop: winit window + wgpu (Vulkan/Metal/DX12)
        let window = winit::window::WindowBuilder::new()
            .with_title("NomCanvas")
            .build(&event_loop);
        let renderer = WgpuRenderer::new(&window);
        run_app(renderer);
    }
    
    #[cfg(target_arch = "wasm32")]
    {
        // Browser: wasm-bindgen + wgpu (WebGPU)
        let canvas = web_sys::window().unwrap()
            .document().unwrap()
            .get_element_by_id("canvas").unwrap();
        let renderer = WgpuRenderer::from_canvas(&canvas);
        run_app(renderer);
    }
}
```

**Same `run_app()`, same scene graph, same compiler calls.** Only the window/surface creation differs.

---

## 10. 60 Patterns Integration Map

| Pattern | Where in v2 | Crate |
|---------|------------|-------|
| R1-R4 (RAG + hybrid search) | nom-canvas-core search panel | nom-search + new vector module |
| A1-A2 (Skill + MCP) | nom-graph node types | nom-intent + new skill registry |
| U1 (Damage rects) | nom-gpui/renderer.rs | Dirty-region tracking |
| U7 (Element map) | nom-canvas-core/spatial_index.rs | R-tree lookup |
| M1-M2 (DAG + cache) | nom-graph/engine.rs + cache.rs | ComfyUI 4-tier pattern |
| S2 (Linter framework) | nom-compiler/nom-concept | yara-x sealed trait pattern |
| S4 (File watcher) | nom-canvas-core/project.rs | typst comemo + batch debounce |
| D1-D2 (Component registry + dep graph) | nom-blocks/ + nom-graph/ | ToolJet combineProperties |
| D6 (Sandboxed eval) | nom-graph/expression.rs | n8n AST-based sandbox |

---

## 11. Migration from v1

The current 45 TypeScript modules + Tauri backend are **replaced entirely**. The 29 nom-compiler crates are **unchanged** — they become direct dependencies of nom-canvas.

| v1 (TypeScript + Tauri) | v2 (Full Rust + wgpu) |
|------------------------|----------------------|
| 45 .ts modules | ~12 Rust crates |
| 20 Tauri IPC commands | Direct function calls |
| Prosemirror (JS) | Custom editor (Rust) |
| DOM + Canvas overlay | GPU scene graph |
| JSON serialization | Zero-copy Rust structs |
| 250KB JS bundle | Single native binary |
| Desktop only | Desktop + Browser (WebGPU) |

---

## 12. Universal Composition Engine — "Compose Everything"

NomCanvas is not just an IDE. It's a **universal composition engine**. Write Nom → get ANYTHING:

```
"the media intro_video is
 intended to create a 30-second brand intro.
 uses the @media matching 'logo animation' with at-least 0.8 confidence.
 composes title_card, logo_reveal, tagline_fade."
```
→ Compiler S1-S6 → CompositionPlan → **video output**

### Output Backends (new crate: `nom-compose/`)

| Nom Kind | Output | Backend | Source Repo (end-to-end read) |
|----------|--------|---------|-------------------------------|
| `media` video | MP4/WebM | ComfyUI DAG + async queue | `ComfyUI-master/` — 4 cache tiers, 100+ nodes |
| `media` image | PNG/AVIF | Diffusion pipeline (50+ models) | `Open-Higgsfield-AI-main/` — multi-studio, auto mode switch |
| `media` storyboard | Video sequence | 4-phase AI orchestration | `waoowaoo/` — script→scenes→cinematography→composite |
| `media` novel→video | Full film | Agent workflow (Claude SDK) | `ArcReel-main/` — novel→script→characters→storyboard→video |
| `media` audio | FLAC/AAC | Audio synthesis | `waoowaoo/` voice workers + nom-media codecs |
| `screen` web | HTML/WASM | App manifest → deploy | `ToolJet-develop/` 55 widgets + `dify-main/` workflow |
| `screen` native | Binary | LLVM codegen | `nom-llvm/` compile() + link_bitcodes() |
| `data` extract | JSON/CSV/MD | PDF/doc extraction (80+ langs) | `opendataloader-pdf-main/` — XY-Cut++ reading order |
| `data` transform | Tables | Polars-like pipeline | `polars/` — SIMD columnar processing |
| `concept` document | PDF/DOCX | Typst-like layout engine | `typst-main/` — comemo incremental compilation |
| `scenario` workflow | Execution trace | n8n-like DAG execution | `n8n/` — 304 node types + AST sandbox |
| `media` 3D | glTF | Mesh composition | nom-media MeshGeometry kind |

### Composition Pipeline

```
User writes Nom on canvas
     ↓
S1-S6 Pipeline (classify kind, extract shape, resolve refs)
     ↓
nom-planner: Generate CompositionPlan
     ↓
nom-compose: Route to correct backend
     ↓
┌─────────────────────────────────────────────────────┐
│  Backend Dispatch (protocol-based, from ArcReel)     │
│                                                       │
│  trait CompositionBackend {                            │
│    fn capabilities(&self) -> Vec<Capability>;         │
│    async fn compose(&self, plan: &CompositionPlan)     │
│      -> Result<Artifact>;                             │
│    fn progress(&self) -> ProgressStream;               │
│  }                                                    │
│                                                       │
│  Implementations:                                     │
│  • VideoBackend (ComfyUI DAG pattern)                 │
│  • ImageBackend (multi-model dispatch)                │
│  • StoryboardBackend (4-phase waoowaoo pipeline)      │
│  • DocumentBackend (typst layout engine)              │
│  • DataBackend (extraction + transform)               │
│  • AppBackend (LLVM compile or web deploy)            │
│  • AudioBackend (synthesis + codec)                   │
│  • ThreeDBackend (mesh composition)                   │
└─────────────────────────────────────────────────────┘
     ↓
Content-addressed artifact: ~/.nom/store/<hash>/body.*
     ↓
Canvas preview: GPU-rendered in preview panel
```

### Task Queue (from ArcReel + waoowaoo patterns)

Media generation is async (>10s). Need a task queue:

```rust
// nom-compose/queue.rs
pub struct CompositionQueue {
    workers: Vec<Worker>,      // per-backend worker pool
    tasks: TaskStore,          // SQLite task lifecycle
    rate_limiter: RateLimiter, // per-vendor RPM limits
    cost_tracker: CostTracker, // multi-currency usage tracking
}

// Task lifecycle: Queued → Running → Succeeded/Failed/Cancelled
// Progress streaming: SSE or channel-based for canvas UI updates
```

### Phase Orchestration (from waoowaoo 4-phase pattern)

Complex compositions (novel→video) need multi-phase execution:

```
Phase 1: Planning (script analysis, clip extraction)
Phase 2a: Cinematography (composition, lighting, color)  } parallel
Phase 2b: Acting (character directions, expressions)     }
Phase 3: Detail enrichment + prompt generation
Phase 4: Asset generation (images→video) via task queue
Phase 5: FFmpeg composite + audio sync
```

### Vendor Abstraction (from ArcReel protocol pattern)

```rust
// nom-compose/vendors.rs
pub trait MediaVendor: Send + Sync {
    fn name(&self) -> &str;
    fn capabilities(&self) -> Vec<MediaCapability>;
    async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse>;
    fn cost_per_request(&self) -> Cost;
}

// Built-in vendors: local (nom-llvm), cloud (API-based)
// User configures API keys via credential store
```

### Data Extraction (from opendataloader pattern)

```rust
// nom-compose/data.rs — PDF/doc → structured data
pub trait DataExtractor {
    fn extract(&self, input: &[u8], format: DataFormat) -> Result<ExtractedData>;
    fn supported_formats(&self) -> Vec<&str>; // pdf, docx, xlsx, html, csv
}

// Modes: deterministic (XY-Cut++, 0.015s/page) or hybrid-AI (0.463s/page, +90% table accuracy)
// Output: Markdown (reading order) + JSON (bounding boxes) + tables
```

### Canvas Integration

On the NomCanvas canvas, all composition types render as blocks:
- **Video block**: Timeline preview with scrubber, generated frames
- **Image block**: Generated image with variant picker
- **Document block**: Paginated preview (typst-rendered)
- **Data block**: Table view with column types
- **App block**: Live app preview (web) or binary info (native)
- **Audio block**: Waveform visualizer with playback

All blocks are content-addressed (SHA-256 hash) and stored in `~/.nom/store/`.

---

## 13. The Nom Language as Universal Composition

Every output type uses the **same Nom syntax**:

```nomx
the media product_showcase is
  intended to create a 60-second product video with 3D rotation.
  requires the data product_specs matching "product dimensions" with at-least 0.9 confidence.
  uses the @media matching "3D product turntable" with at-least 0.8 confidence.
  composes intro_card, product_spin, feature_callouts, outro_cta.
  favor visual_quality.
  favor brand_consistency.
```

The compiler handles the rest: resolves `product_specs` from the dictionary, finds the best `3D product turntable` media template, composes the 4 sections, scores quality, and produces a video artifact.

**This is what makes Nom different from every other tool.** You don't need to learn video editing, image generation, PDF tools, data pipelines, or app frameworks. You write English sentences describing what you want, and the compiler — which has a dictionary of 100M+ nomtu entries — figures out how to build it.

---

## 14. Semantic Data Layer (from WrenAI end-to-end reading)

NomCanvas needs a **semantic layer** so the compiler understands data context — not just code structure.

```rust
// nom-compose/semantic.rs — WrenAI MDL pattern adapted for Nom
pub struct SemanticModel {
    pub entities: Vec<SemanticEntity>,      // tables/models with business meaning
    pub metrics: Vec<DerivedMetric>,         // computed facts (revenue = sum(price * qty))
    pub relationships: Vec<EntityRelation>,  // joins between entities
}

// When user writes: "the data sales_report is intended to show monthly revenue"
// The semantic layer grounds "monthly revenue" to:
//   metric: revenue = SUM(orders.price * orders.quantity)
//   grouped by: orders.created_at (monthly)
//   source: orders table
// No LLM hallucination — grounded by schema context.
```

**Pipeline** (Hamilton+Haystack pattern from WrenAI):
1. Intent classification (is this a query, chart, or insight request?)
2. Vector retrieval from semantic model (Qdrant)
3. LLM generates query grounded by MDL context
4. Correction loop validates syntax
5. Execute against data source

---

## 15. Provider Router (from 9router end-to-end reading)

NomCanvas dispatches to multiple AI/media providers with intelligent fallback:

```rust
// nom-compose/router.rs — 9router 3-tier pattern
pub struct ProviderRouter {
    tiers: Vec<ProviderTier>,           // subscription → cheap → free
    quotas: QuotaTracker,               // per-provider RPM + token limits
    format_translator: FormatTranslator, // Claude ↔ OpenAI ↔ Gemini formats
    combo_strategies: HashMap<String, FallbackStrategy>, // per-combo routing
}

pub enum FallbackStrategy {
    Fallback,   // try 1 → 2 → 3
    RoundRobin, // load balance across tiers
    FillFirst,  // drain tier 1 quota before tier 2
}

// User configures providers in credentials panel:
// Tier 1: Claude API (subscription) — used first
// Tier 2: Local Ollama (cheap) — fallback
// Tier 3: Free API (rate-limited) — last resort
```

---

## 16. Real-Time Collaboration (from Huly end-to-end reading)

For multi-user canvas editing (v3+):

```rust
// nom-canvas-core/collab.rs — Huly CRDT pattern
pub struct CollaborationEngine {
    crdt: YDoc,                         // Yrs (Rust port of Yjs)
    event_bus: EventBus,                // Redpanda/channel-based events
    presence: PresenceTracker,          // cursor positions, selections
    transactor: TransactionLog,         // immutable event log
}

// Architecture: Client ←→ CRDT sync ←→ Event bus ←→ Persistence
// 50+ Huly packages show plugin-per-domain is the right granularity
```

---

## 17. Fully Functional Backend Summary

Every subsystem maps to a real production pattern from an end-to-end-read repo:

| Subsystem | Pattern Source | Read Level |
|-----------|---------------|------------|
| GPU rendering | Zed GPUI (scene.rs, renderer, atlas, text) | Full crate |
| Design system | AFFiNE (73 CSS vars, Inter, Source Code Pro) | Full theme |
| DAG execution | ComfyUI (4 caches, 100+ nodes, dynamic graph) | Full repo |
| RAG retrieval | LlamaIndex (50+ stores, 8 postprocessors) + Haystack (pipeline) | Full repos |
| Skill composition | Refly (46 NestJS modules, LangGraph, BullMQ) | Full repo |
| Component registry | ToolJet (55 widgets, Zustand, @dnd-kit) | Full repo |
| Workflow engine | n8n (304 nodes, AST sandbox, credential encryption) | Full repo |
| Linter framework | yara-x (sealed trait, WASM codegen) | Full compiler |
| Incremental compile | typst (comemo memoization, rayon parallelism) | Full compiler |
| Video generation | ArcReel (Claude agents), waoowaoo (4-phase storyboard) | Full repos |
| Image generation | Open-Higgsfield (200+ models, 4 studios) | Full repo |
| Data extraction | opendataloader-pdf (XY-Cut++, hybrid AI) | Full repo |
| Semantic data | WrenAI (MDL, Hamilton pipelines, provider abstraction) | Full repo |
| Provider routing | 9router (3-tier fallback, quota tracking, format translation) | Full repo |
| Collaboration | Huly (30 services, CRDT, Redpanda events, 50 plugins) | Full repo |
| Framework decision | Dioxus (confirmed: Desktop=webview, NOT GPU-native) | Full repo |

**Total: 19 repos read end-to-end.** Not README-only. Not surface scans. Every module, every dependency, every data flow.

---

## 18. Programmatic Video (from Remotion deep-dive via DeepWiki)

Remotion proves **video = code**. In Nom: **video = .nomx source**.

### Remotion's Architecture (React-based)
```
React composition → webpack bundle → headless Puppeteer browser
→ seekToFrame(N) → screenshot → frame buffer
→ FFmpeg stdin (parallel encoding) → MP4
```

Key patterns:
- **Frame = function(time)** — component receives frame number, returns visual
- **Dual coordinate system** — absolute frames (composition) + relative frames (per Sequence)
- **Concurrency via page pool** — N browser pages render frames in parallel
- **Parallel encoding** — frames pipe to pre-spawned FFmpeg stdin (no disk temp files)
- **Offthread compositor** — native binary extracts video frames without browser main thread

### Nom's Equivalent (GPU-native, no browser)

```rust
// nom-compose/video.rs — Remotion pattern in Rust

pub struct VideoComposition {
    fps: u32,
    duration_frames: u32,
    width: u32,
    height: u32,
    scenes: Vec<SceneEntry>,  // each scene = a nomtu entity
}

pub struct SceneEntry {
    from_frame: u32,          // Remotion's Sequence.from
    duration: u32,            // Remotion's Sequence.durationInFrames
    entity_hash: String,      // content-addressed nomtu reference
}

impl VideoComposition {
    /// Render one frame — the scene graph IS the frame
    fn render_frame(&self, frame: u32, scene: &mut Scene) {
        // 1. Find active scenes at this frame (Remotion's Sequence visibility)
        let active = self.scenes.iter()
            .filter(|s| frame >= s.from_frame && frame < s.from_frame + s.duration);
        
        // 2. Each scene paints to the GPU scene graph
        for scene_entry in active {
            let relative_frame = frame - scene_entry.from_frame;
            scene_entry.paint(relative_frame, scene);
        }
    }
    
    /// Export video — GPU capture → codec → file
    pub fn export(&self, output_path: &str) -> Result<()> {
        // Pre-spawn FFmpeg for parallel encoding (Remotion pattern)
        let mut ffmpeg = prespawn_ffmpeg(output_path, self.fps, self.width, self.height);
        
        // Render each frame via GPU scene graph (no browser!)
        for frame in 0..self.duration_frames {
            let mut scene = Scene::new();
            self.render_frame(frame, &mut scene);
            
            // Capture GPU framebuffer → raw pixels
            let pixels = self.renderer.capture_frame(&scene);
            
            // Pipe to FFmpeg stdin (Remotion's parallel encoding pattern)
            ffmpeg.stdin.write_all(&pixels)?;
        }
        
        ffmpeg.wait()?;
        Ok(())
    }
}
```

### Why this is better than Remotion:
- **No browser** — Remotion needs Puppeteer (Chrome). NomCanvas renders directly via wgpu.
- **No JavaScript** — Pure Rust, no V8 overhead.
- **GPU-accelerated** — wgpu renders scenes at GPU speed, not DOM layout speed.
- **Content-addressed** — Each scene is a nomtu entity with hash. Cache individual scene renders.
- **Same renderer** — The canvas preview AND the video export use the exact same GPU pipeline.

### How it works in Nom language:

```nomx
the media product_video is
  intended to create a 60-second product showcase.
  composes intro_card, product_spin, feature_list, cta_outro.

the media intro_card is
  intended to show the brand logo for 3 seconds.
  uses the @media matching "logo animation fade-in" with at-least 0.8 confidence.

the media product_spin is
  intended to show the product rotating 360 degrees over 15 seconds.
  uses the @media matching "3D turntable animation" with at-least 0.7 confidence.
```

The compiler resolves each `uses` reference against the dictionary, builds a `VideoComposition` with `SceneEntry` per media entity, and calls `export()` — rendering every frame through the same GPU scene graph that powers the canvas IDE.

---

## 21. Session 2026-04-17 Implementation Log — Waves 4→10

This appendix tracks what landed against the spec during the 2026-04-17 execution session. Architectural content above is canonical; this section records commit → spec-clause coverage.

**8 commits on main, 204 → 1272 tests, 3 → 13 crates.**

| Commit | Wave | Spec sections advanced |
|--------|------|------------------------|
| `c2d7090` | 4 | §8 Phase 3 (nom-theme tokens/color/mode, nom-panels shell, nom-blocks shared infra + prose + nomx) · §5 animation curves |
| `24f7e05` | CI | `.github/workflows/ci.yml` canvas job brought into line with §19 headless-GPU discipline |
| `4592b85` | 5 | §8 Phase 3 remaining blocks (media, graph_node, drawing, table, embed) · §7 editor display pipeline (syntax_map, display_map, lsp_bridge, inlay_hints, wrap_map, tab_map) · §9 theme fonts + icons |
| `9f3df57` | 6 | §10-12 Phase 4/5 scaffolds: nom-graph-v2 (§11 DAG + 4 caches) · nom-compose (§12 dispatch + queue + router + credential) · nom-lint (§13 sealed-trait linter) · nom-memoize (§14 comemo pattern without dep) · nom-telemetry (§15 W3C traceparent) · nom-collab (§16 CRDT types) · nom-editor line_layout (§7 cosmic-text interface) · nom-blocks compose/*_block preview types (§12 canvas integration) |
| `2e47d5d` | 7 | §12 artifact_store / vendor_trait / video_composition / format_translator · §14 semantic (WrenAI MDL types) · §15 rayon_bridge · §13 file-watcher scaffold · §11 AST-sandbox data-structure sanitizers · §9 typography scale · §6 command history |
| `365db9b` | 8 | §12 all 10 concrete backends stubbed + `register_all_stubs()`: video, image, web_screen, native_screen, data_extract (XY-Cut++), data_query (WrenAI 5-stage), storyboard_narrative (4-phase + 5-phase typed pipelines), audio, data_frame (Polars-inspired minimal), mesh (glTF 2.0) |
| `4096db9` | 9 | §12 last backend: scenario_workflow (n8n-style retry + webhook resume) · §15 plugin_registry · §6 cursor primitive · §6 shortcut registry + platform normalize · §8 tree_query + whole-tree validators · integration tests for §11 DAG + §12 dispatcher |
| `56604c4` | 10 | §5 transition primitive · §9 motion tokens · §6 panels layout solver · §6 rendering_hints (hover/select overlay layer) · §16 presence sibling to awareness · §7 command handlers · §13 2 concrete lint rules (trailing whitespace, double blank lines) |

**Audit findings resolved this session:** 2 HIGH (animation `Duration::ZERO` NaN, `NarrativeResult::completed_phase()` skip) + 5 MEDIUM (EmbedKind brand names → `VideoStream`/`DesignFile`, three `Rgba` types → `SrgbColor` rename, `FractionalIndex` dedup to `block_model.rs`, CI `NOM_SKIP_GPU_TESTS` env var, `RUSTFLAGS=-D warnings` dead_code cleanup) + 1 LOW (`CommandError::Failed` `#[allow(dead_code)]`).

**Still 0% per the spec's Vision (§1):** the render substrate exists, but the natural-language → compiler → canvas pipe remains fully disconnected — see iter-17 audit in `nom_state_machine_report.md`. The keystone unlock is the `nom_concept::stage1_tokenize → Highlighter::color_runs` adapter (~200 LOC), blocked on the cross-workspace cargo path-dep decision between `nom-compiler/` and `nom-canvas/`.
