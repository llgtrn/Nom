# Phase 3 Implementation Plan -- NomCanvas Blocks + Panels

> **Date:** 2026-04-17
> **Parent spec:** `docs/superpowers/specs/2026-04-17-nomcanvas-gpui-design.md` (sections 5-8)
> **Status:** PLAN -- ready for parallel-agent execution
> **Dependency:** Phase 2 (nom-canvas-core + nom-editor) must land first
> **Reference repos read end-to-end:**
>   - AFFiNE blocksuite `AFFiNE-canary/blocksuite/affine/blocks/` -- 20 block types, store/view/block triple, CaptionedBlockComponent base class, adapter pattern, edgeless dual-mode rendering
>   - ToolJet widgets `ToolJet-develop/frontend/src/Editor/Components/` -- 55+ widgets, `component.definition.properties` resolution, `Inspector/Components/DefaultComponent.jsx` accordion panel, `RenderWidget.jsx` Zustand store binding

---

## 1. Scope

### 1.1 What We Build
Two new Rust crates added to the `nom-canvas/crates/` workspace:

| Crate | Files | Purpose |
|-------|-------|---------|
| `nom-blocks` | 9 source + 1 lib.rs + tests | 7 block types that live on the infinite canvas |
| `nom-panels` | 8 source + 1 lib.rs + tests | 7 UI panels that frame the canvas viewport |

Plus a companion crate:

| Crate | Files | Purpose |
|-------|-------|---------|
| `nom-theme` | 3 source + 1 lib.rs | Design tokens, font config, icon paths (spec section 5) |

### 1.2 What We Do NOT Build
- No compositor/composition backends (that is Phase 5 / nom-compose).
- No DAG execution engine (that is nom-graph, Phase 4).
- No CRDT / collaboration layer (that is v3).
- No browser/WASM target (Phase 6).
- No nom-compiler changes -- compiler crates are consumed as read-only dependencies.

### 1.3 Line Budget
- `nom-blocks`: ~2,500 lines of Rust (trait + 7 blocks + tests)
- `nom-panels`: ~2,000 lines of Rust (7 panels + tests)
- `nom-theme`: ~400 lines (tokens + fonts + icons)
- Total: ~4,900 lines

---

## 2. Crate `nom-theme` (prerequisite -- build first)

Both `nom-blocks` and `nom-panels` depend on design tokens. Build this first.

### 2.1 `tokens.rs` -- Design Token Constants

Extract the 73 AFFiNE-derived CSS variables from spec section 5 into typed Rust constants.

**Pattern source:** AFFiNE stores tokens as CSS custom properties across multiple `.css` files. We consolidate into a single Rust module with const values grouped by category.

```
nom-canvas/crates/nom-theme/src/tokens.rs
```

Contents (from spec section 5, lines 143-182):
- Layout constants: `SIDEBAR_W` (248.0), `TOOLBAR_H` (48.0), `STATUSBAR_H` (24.0), `BLOCK_RADIUS` (4.0), `MODAL_RADIUS` (22.0), `POPOVER_RADIUS` (12.0), `BTN_H` (28.0), `BTN_H_LG` (32.0), `BTN_H_XL` (40.0), `ICON` (24.0)
- Typography: heading weights (700/600/400), letter-spacing (-0.02em for H1), quote bar dimensions
- Colors (Nom dark theme): `BG` (#0F172A), `BG2` (#1E293B), `TEXT` (#F8FAFC), `CTA` (#22C55E), `BORDER` (#334155), `FOCUS` (rgba)
- Animation: `ANIM_DEFAULT` (300ms), `ANIM_FAST` (200ms), cubic-bezier string
- Shadows: button shadow (0, 1, 5, 0.12 alpha)

**Acceptance criteria:**
- [ ] All 73 token values from the spec present as `pub const`
- [ ] Colors are `Rgba` (from nom-gpui), layout values are `f32`
- [ ] `#[cfg(test)]` confirms token values match spec exactly
- [ ] Zero foreign identifiers -- all constant names use Nom vocabulary

### 2.2 `fonts.rs` -- Font Configuration

Two font families (from spec section 5, line 158):
- **Inter** -- UI text (body, headings, labels)
- **Source Code Pro** -- code blocks, .nomx editor

Define structs for font family + weight + size presets that `nom-gpui/text.rs` (cosmic-text) consumes.

```rust
pub struct FontPreset {
    pub family: &'static str,
    pub weight: u16,
    pub size: f32,
    pub line_height: f32,
    pub letter_spacing: f32,
}
```

Presets: `H1`, `H2`, `H3`, `BODY`, `BODY_SMALL`, `CODE`, `CODE_SMALL`, `LABEL`, `CAPTION`.

**Acceptance criteria:**
- [ ] 9 font presets covering all spec typography
- [ ] Each preset has family, weight, size, line_height, letter_spacing
- [ ] `CODE` and `CODE_SMALL` use Source Code Pro; all others use Inter

### 2.3 `icons.rs` -- Icon Path Data

24px icon set (Lucide-style) converted to GPU-renderable path data. Each icon is a `Vec<PathSegment>` that `nom-gpui/scene.rs` can render as a `Path` primitive.

Initial icon set (minimum for panels + blocks):
- Navigation: sidebar-toggle, search, settings, command
- Editing: bold, italic, underline, code, link
- Blocks: text, code-block, image, table, graph, brush, embed
- Actions: play, stop, compile, score, plan

**Acceptance criteria:**
- [ ] Minimum 20 icons as const path data arrays
- [ ] Each icon renders at 24x24px bounding box
- [ ] Test that each icon has non-zero path segments

### 2.4 `lib.rs`
```rust
pub mod tokens;
pub mod fonts;
pub mod icons;
```

**Acceptance criteria:**
- [ ] Crate compiles with zero warnings
- [ ] Added to workspace `Cargo.toml` members

---

## 3. Crate `nom-blocks`

### 3.1 `block.rs` -- Block Trait + Registry

**Pattern source -- AFFiNE:** Every block in AFFiNE follows a triple: `Store` (data/schema), `View` (rendering), and `Block` (component logic). See:
- `AFFiNE-canary/blocksuite/affine/blocks/paragraph/src/store.ts:9-17` -- `StoreExtensionProvider` registers schema + adapters
- `AFFiNE-canary/blocksuite/affine/blocks/code/src/code-block.ts:39` -- `CaptionedBlockComponent<CodeBlockModel>` is the base class pattern
- Each block has: `connectedCallback()` (lifecycle), `renderBlock()` (painting), and per-block state via signals

**Pattern source -- ToolJet:** Component registry via `AllComponents` map + `component.definition.properties` dictionary.
- `ToolJet-develop/frontend/src/Editor/component-properties-resolution.js:15-29` -- `resolveProperties` iterates `component.definition.properties`
- `ToolJet-develop/frontend/src/AppBuilder/AppCanvas/RenderWidget.jsx:50-62` -- `getComponentToRender` dispatches by type

**Nom-native abstraction (no wrappers, no adapters):**

```rust
/// The seven block kinds that live on the NomCanvas infinite canvas.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BlockKind {
    Prose,
    Nomx,
    Media,
    GraphNode,
    Drawing,
    Table,
    Embed,
}

/// Unique identifier for a block instance.
pub type BlockId = u64;

/// Version counter for CRDT conflict resolution.
pub type BlockVersion = u32;

/// Every block on the canvas implements this trait.
pub trait Block {
    /// Which kind of block this is.
    fn kind(&self) -> BlockKind;

    /// Stable identifier for this block.
    fn id(&self) -> BlockId;

    /// Axis-aligned bounding box in canvas coordinates.
    fn bounds(&self) -> Bounds<Pixels>;

    /// Set new bounds (resize/move).
    fn set_bounds(&mut self, bounds: Bounds<Pixels>);

    /// CRDT version counter.
    fn version(&self) -> BlockVersion;

    /// Which of the 5 unified modes this block primarily belongs to.
    fn mode(&self) -> CanvasMode;

    /// Editable properties for the property inspector panel.
    fn properties(&self) -> Vec<BlockProperty>;

    /// Update a single property by key.
    fn set_property(&mut self, key: &str, value: PropertyValue) -> Result<(), BlockError>;

    /// Three-phase rendering: emit primitives into the scene graph.
    /// Delegates to nom-gpui Element trait internally.
    fn paint(&self, scene: &mut Scene, viewport: &Viewport, theme: &tokens::Theme);
}
```

**Block property system** (abstracted from ToolJet's `definition.properties` + AFFiNE's signal-based props):

```rust
pub struct BlockProperty {
    pub key: String,
    pub label: String,
    pub value: PropertyValue,
    pub section: PropertySection,
}

pub enum PropertyValue {
    Text(String),
    Number(f64),
    Boolean(bool),
    Color(Rgba),
    Choice(String, Vec<String>),
    Range { value: f64, min: f64, max: f64 },
}

pub enum PropertySection {
    Data,
    Style,
    Layout,
    Events,
}
```

**Block registry** (from ToolJet's `AllComponents` flat map):

```rust
pub struct BlockRegistry {
    factories: HashMap<BlockKind, Box<dyn Fn(BlockId, Bounds<Pixels>) -> Box<dyn Block>>>,
}

impl BlockRegistry {
    pub fn new() -> Self { /* register all 7 block kinds */ }
    pub fn create(&self, kind: BlockKind, id: BlockId, bounds: Bounds<Pixels>) -> Box<dyn Block>;
}
```

**Acceptance criteria:**
- [ ] `Block` trait defined with all 9 methods
- [ ] `BlockKind` enum with 7 variants
- [ ] `BlockProperty` + `PropertyValue` + `PropertySection` types defined
- [ ] `BlockRegistry` creates all 7 block kinds
- [ ] Tests: registry round-trip for each kind; property get/set round-trip

### 3.2 `prose.rs` -- Rich Text Block

**Purpose:** Headings (H1-H6), paragraphs, lists (ordered/unordered), blockquotes, callouts. The "document mode" block.

**Pattern source -- AFFiNE:** `paragraph/src/paragraph-block.ts:36-383`
- Extends `CaptionedBlockComponent<ParagraphBlockModel>`
- `type$` signal distinguishes heading levels vs paragraph vs quote (line 245)
- Collapsible headings: `collapsed` prop + `calculateCollapsedSiblings` (lines 247-266)
- Rich text via `<rich-text>` element with inline range provider (line 339-352)
- Placeholder text when empty and focused (lines 148-175)

**Pattern source -- AFFiNE callout:** `callout/src/callout-block.ts`
- Block with emoji icon prefix + rich text body
- Toolbar + slash menu integration (configs/ directory)

**Nom-native implementation:**

```rust
pub struct ProseBlock {
    id: BlockId,
    bounds: Bounds<Pixels>,
    version: BlockVersion,
    /// What variant of prose this is.
    variant: ProseVariant,
    /// The text content -- stored as a flat rope with inline formatting spans.
    content: RopeWithSpans,
    /// Whether headings are collapsed (hides siblings until next heading of same or higher level).
    collapsed: bool,
    /// Placeholder text shown when empty.
    placeholder: Option<String>,
}

pub enum ProseVariant {
    Paragraph,
    Heading { level: u8 }, // 1-6
    OrderedList { start: u32 },
    UnorderedList,
    Blockquote,
    Callout { icon: String },
}
```

`RopeWithSpans` wraps `ropey::Rope` with inline formatting spans (bold, italic, code, link). This is the Rust-native equivalent of AFFiNE's `<rich-text>` element + Prosemirror marks.

```rust
pub struct InlineSpan {
    pub start: usize,  // byte offset in rope
    pub end: usize,
    pub format: InlineFormat,
}

pub enum InlineFormat {
    Bold,
    Italic,
    Code,
    Underline,
    Strikethrough,
    Link(String),
    Color(Rgba),
}
```

**Paint strategy:**
1. Compute text layout via cosmic-text (from `nom-gpui/text.rs`)
2. Apply heading weight/size from `nom-theme/fonts.rs` based on variant
3. Render quote bar (2px wide, 18px radius) for `Blockquote` variant
4. Render bullet/number prefix for list variants
5. Emit `Text` + `Quad` (background) + optional `Path` (quote bar) primitives to scene

**Acceptance criteria:**
- [ ] All 6 `ProseVariant` types implemented
- [ ] `RopeWithSpans` supports insert, delete, format_range, unformat_range
- [ ] `InlineFormat` covers bold, italic, code, underline, strikethrough, link, color
- [ ] Heading collapse state persists across paint cycles
- [ ] Tests: insert text + apply bold span + verify spans survive rope edit
- [ ] Tests: heading H2 collapses siblings, uncollapse restores them
- [ ] Paint emits correct primitives for each variant (at least 1 quad + text per variant)

### 3.3 `nomx.rs` -- Code Block (.nomx Editor)

**Purpose:** A block that embeds `nom-editor` for editing `.nomx` source code. The "code mode" block.

**Pattern source -- AFFiNE:** `code/src/code-block.ts:39-492`
- `CodeBlockComponent extends CaptionedBlockComponent<CodeBlockModel>` (line 39)
- Language signal + highlight tokens signal (lines 46-64)
- Shiki-based syntax highlighting with async language loading (lines 105-156)
- Hot keys: Tab/Shift-Tab for indentation (lines 240-340), Enter captures sync (line 351), Mod-Enter inserts paragraph below (lines 356-374)
- Copy code via Slice/clipboard (lines 380-392)
- Preview mode toggle via signal (lines 44-52, 415-419)
- Line numbers conditional rendering (lines 409-413, 446-453)

**Nom-native implementation:**

```rust
pub struct NomxBlock {
    id: BlockId,
    bounds: Bounds<Pixels>,
    version: BlockVersion,
    /// The embedded editor state (from nom-editor crate).
    editor: EditorState,
    /// Whether to show line numbers.
    show_line_numbers: bool,
    /// Whether preview mode is active (shows compiled output instead of source).
    preview_mode: bool,
    /// Current language (always "nomx" but extensible).
    language: String,
    /// Cached syntax tokens from nom-concept stage1_tokenize.
    highlight_tokens: Vec<SyntaxToken>,
    /// Cached compilation diagnostics from background pipeline run.
    diagnostics: Vec<Diagnostic>,
}
```

**Compiler integration** (from spec section 7, lines 206-211):
- On every keystroke: `stage1_tokenize` runs on the interactive thread (<10ms) to produce `highlight_tokens`
- On 500ms pause: `run_pipeline` S1-S6 runs on background thread, produces `diagnostics`
- Hover: `handle_hover` from nom-lsp returns tooltip
- Completion: `handle_completion` from nom-resolver + nom-grammar

The `NomxBlock` owns an `EditorState` from `nom-editor` (buffer.rs, cursor.rs, input.rs) and delegates text editing to it. Syntax highlighting colors come from `highlight_tokens` + `nom-theme` color tokens.

**Hot keys** (adapted from AFFiNE code block):
- Tab: indent selected lines by 2 spaces
- Shift-Tab: unindent selected lines
- Enter: newline with auto-indent
- Mod-Enter: insert new prose block below (exit code block)

**Preview mode:** When `preview_mode == true`, instead of rendering the source, render the compiled output. For MVP this shows the pipeline diagnostics as colored text. Full preview (rendered app output) deferred to Phase 5.

**Acceptance criteria:**
- [ ] `NomxBlock` embeds `EditorState` from nom-editor
- [ ] Syntax token cache updated on content change
- [ ] Diagnostic overlay renders inline (red underline + message)
- [ ] Line numbers toggle works
- [ ] Preview mode toggle switches between source and diagnostic view
- [ ] Tests: create block, insert ".nomx" text, verify highlight_tokens non-empty
- [ ] Tests: Tab indents, Shift-Tab unindents
- [ ] Tests: preview_mode toggle preserves editor state

### 3.4 `media.rs` -- Image/Video/Audio Block

**Purpose:** Display and interact with media content. Shows preview/thumbnail with playback controls.

**Pattern source -- AFFiNE:** `image/src/image-block.ts:34-198`
- `ImageBlockComponent extends CaptionedBlockComponent<ImageBlockModel>` (line 34)
- `ResourceController` manages blob loading via content-addressed `sourceId` (lines 42-45)
- Blob URL state signal (line 47)
- Resize via `ImageSelection` (lines 35-40)
- Click handler for selection (lines 79-89)
- Hover tooltip via `ToolbarRegistryIdentifier` (lines 91-111)
- Render states: loading (spinner), error (broken icon), loaded (image) (lines 137-183)

**Nom-native implementation:**

```rust
pub struct MediaBlock {
    id: BlockId,
    bounds: Bounds<Pixels>,
    version: BlockVersion,
    /// The media kind determines playback controls and rendering.
    media_kind: MediaKind,
    /// Content-addressed hash pointing to artifact store (~/.nom/store/<hash>/).
    content_hash: Option<String>,
    /// Display state while content loads.
    load_state: MediaLoadState,
    /// For video/audio: current playback position in seconds.
    playback_position: f64,
    /// For video/audio: total duration in seconds.
    duration: f64,
    /// For video/audio: whether currently playing.
    is_playing: bool,
    /// Caption text (from AFFiNE's CaptionedBlockComponent pattern).
    caption: Option<String>,
}

pub enum MediaKind {
    Image,
    Video,
    Audio,
}

pub enum MediaLoadState {
    /// No content hash assigned yet.
    Empty,
    /// Content is being fetched from artifact store.
    Loading,
    /// Content loaded and ready to display.
    Loaded { thumbnail: TextureHandle },
    /// Content failed to load.
    Error(String),
}
```

**Paint strategy:**
- `Empty`: render placeholder with media icon + "Drop media here" text
- `Loading`: render spinning progress indicator (animated quad rotation)
- `Loaded` (Image): blit texture via `PolychromeSprite` primitive
- `Loaded` (Video): blit current frame texture + render scrubber bar below
- `Loaded` (Audio): render waveform visualization + playback controls
- `Error`: render broken-media icon + error message text

**Acceptance criteria:**
- [ ] Three `MediaKind` variants: Image, Video, Audio
- [ ] Content loading via content hash (artifact store lookup)
- [ ] Four `MediaLoadState` states render correctly
- [ ] Video/audio playback position tracking
- [ ] Caption text below media
- [ ] Tests: state machine transitions (Empty -> Loading -> Loaded -> Error)
- [ ] Tests: playback position clamps to [0, duration]

### 3.5 `graph_node.rs` -- DAG Node Block

**Purpose:** A node in the visual programming graph (Graph mode). Has typed input/output ports and wires.

**Pattern source -- spec section 6:** Graph mode uses nom-planner + nom-intent. DAG nodes with typed ports, wires checked by `nom_score::can_wire`.

**Pattern source -- ComfyUI (from end-to-end reading referenced in spec line 601):**
- Nodes have named inputs (typed) and outputs (typed)
- Wires connect output port -> input port
- `IS_CHANGED` method determines if node needs re-execution
- 4-tier cache: in-memory LRU, disk, RAM-pressure eviction, content-addressed

**Nom-native implementation:**

```rust
pub struct GraphNodeBlock {
    id: BlockId,
    bounds: Bounds<Pixels>,
    version: BlockVersion,
    /// Human-readable name (displayed in title bar).
    title: String,
    /// The nomtu entity hash this node represents (if any).
    entity_hash: Option<String>,
    /// Input ports (left side of the node).
    inputs: Vec<Port>,
    /// Output ports (right side of the node).
    outputs: Vec<Port>,
    /// Current execution state.
    execution_state: NodeExecutionState,
    /// Whether the node's output is cached.
    cached: bool,
}

pub struct Port {
    pub name: String,
    pub port_type: PortType,
    pub connected_to: Option<(BlockId, usize)>, // (other node, port index)
}

pub enum PortType {
    /// Untyped -- accepts any connection.
    Any,
    /// Text data.
    Text,
    /// Numeric data.
    Number,
    /// Media content (image/video/audio hash).
    Media,
    /// Nomtu entity reference.
    Entity,
    /// Boolean flag.
    Boolean,
}

pub enum NodeExecutionState {
    Idle,
    Running,
    Succeeded,
    Failed(String),
}
```

**Wire validation:** When a user drags a wire from an output port to an input port, call `nom_score::can_wire(source_type, target_type)` to get green (compatible), amber (coercible), or red (incompatible) feedback. This happens on the UI thread (<1ms per spec section 7).

**Paint strategy:**
- Title bar: `Quad` with rounded top corners + `Text` (title)
- Body: `Quad` with node content (entity info or embedded mini-editor)
- Input ports: small circles on left edge, colored by `PortType`
- Output ports: small circles on right edge, colored by `PortType`
- Execution state: border color changes (idle=border, running=blue, succeeded=green, failed=red)
- Cached indicator: small diamond icon in title bar

**Acceptance criteria:**
- [ ] Ports with typed connections (input/output)
- [ ] Wire validation returns green/amber/red via can_wire integration point
- [ ] Execution state visual feedback (4 states)
- [ ] Cache indicator renders
- [ ] Tests: connect two compatible ports succeeds
- [ ] Tests: connect two incompatible ports returns red
- [ ] Tests: execution state transitions are valid (Idle->Running->Succeeded|Failed)

### 3.6 `drawing.rs` -- Freeform Drawing Block

**Purpose:** Freeform pen/brush strokes for sketching and annotation (Draw mode).

**Pattern source -- AFFiNE:** `surface/src/renderer/canvas-renderer.ts:1-80`
- Canvas renderer with overlay system, viewport-aware rendering
- Elements have rotation, bounds, and are rendered via `ElementRenderer` dispatch
- Dirty-region tracking for efficient repaint (spec pattern U1)

**Nom-native implementation:**

```rust
pub struct DrawingBlock {
    id: BlockId,
    bounds: Bounds<Pixels>,
    version: BlockVersion,
    /// Sequence of strokes that make up this drawing.
    strokes: Vec<Stroke>,
    /// Currently active stroke (in-progress drawing, None when idle).
    active_stroke: Option<Stroke>,
}

pub struct Stroke {
    /// Ordered sequence of points with pressure data.
    pub points: Vec<StrokePoint>,
    /// Visual properties of this stroke.
    pub style: StrokeStyle,
}

pub struct StrokePoint {
    pub x: f32,
    pub y: f32,
    /// Pen pressure (0.0 to 1.0). Desktop mice report 1.0.
    pub pressure: f32,
}

pub struct StrokeStyle {
    pub color: Rgba,
    pub width: f32,
    pub tool: DrawingTool,
}

pub enum DrawingTool {
    Pen,
    Brush,
    Highlighter,
    Eraser,
}
```

**Paint strategy:**
- Each `Stroke` is converted to a series of `Path` primitives
- Width varies with pressure: `effective_width = style.width * point.pressure`
- Catmull-Rom spline interpolation between points for smoothness
- `Highlighter` renders with reduced opacity (0.3)
- `Eraser` strokes are not rendered but mark regions for deletion

**Acceptance criteria:**
- [ ] 4 drawing tools: Pen, Brush, Highlighter, Eraser
- [ ] Pressure-sensitive stroke width
- [ ] Smooth curves via spline interpolation
- [ ] Active stroke renders in real-time during drawing
- [ ] Eraser removes intersecting stroke segments
- [ ] Tests: add stroke with 3 points, verify path segment count
- [ ] Tests: pressure 0.5 produces half-width stroke
- [ ] Tests: eraser tool marks stroke for deletion

### 3.7 `table.rs` -- Data Table Block

**Purpose:** Display tabular data (CSV, JSON, query results). Editable cells with typed columns.

**Pattern source -- AFFiNE:** `data-view/src/data-view-block.ts:49-319`
- `DataViewBlockComponent extends CaptionedBlockComponent<DataViewBlockModel>` (line 49)
- `DataSource` abstraction (line 216-223): `BlockQueryDataSource` backs data from block children
- `DataViewRootUILogic` manages the entire data view lifecycle (lines 248-305)
- Widget presets for different views: table, kanban (lines 202-214)
- Selection model: `DatabaseSelection` with `viewSelection` (lines 176-199)
- Header widget with title + view bar + tools (lines 153-174)

**Pattern source -- ToolJet:** `Table/` directory has `AddNewRowComponent.jsx`, `Boolean.jsx`, `columns/actions.jsx`, `columns/autogenerateColumns.js`, plus `load-properties-and-styles.js` for dynamic column configuration.

**Nom-native implementation:**

```rust
pub struct TableBlock {
    id: BlockId,
    bounds: Bounds<Pixels>,
    version: BlockVersion,
    /// Table title (displayed in header).
    title: String,
    /// Column definitions.
    columns: Vec<ColumnDef>,
    /// Row data.
    rows: Vec<Vec<CellValue>>,
    /// Current view mode.
    view_mode: TableViewMode,
    /// Currently selected cell (row, col).
    selection: Option<(usize, usize)>,
    /// Sort state.
    sort_column: Option<usize>,
    sort_ascending: bool,
}

pub struct ColumnDef {
    pub name: String,
    pub col_type: ColumnType,
    pub width: f32,
    pub visible: bool,
}

pub enum ColumnType {
    Text,
    Number,
    Boolean,
    Date,
    Link,
    Select(Vec<String>),
}

pub enum CellValue {
    Text(String),
    Number(f64),
    Boolean(bool),
    Date(String),
    Link(String),
    Null,
}

pub enum TableViewMode {
    Table,
    // Future: Kanban, Calendar, Gallery
}
```

**Paint strategy:**
- Header row: `Quad` (darker background from `tokens::BG2`) + column name `Text` primitives
- Data rows: alternating background `Quad` + cell `Text` primitives
- Column resize handles: thin `Quad` on column borders
- Selected cell: `Quad` with `tokens::FOCUS` border
- Sort indicator: small triangle icon in sorted column header

**Acceptance criteria:**
- [ ] Column definitions with 6 types
- [ ] Row data as `Vec<Vec<CellValue>>`
- [ ] Single-cell selection model
- [ ] Column sorting by any column
- [ ] Column resize via width property
- [ ] Tests: create 3x3 table, sort by column 1, verify row order
- [ ] Tests: select cell (1,2), verify selection state
- [ ] Tests: add row, verify row count increases
- [ ] Tests: hide column, verify it is excluded from paint

### 3.8 `embed.rs` -- Embedded Content Block

**Purpose:** Embed external content (URLs, iframes, rich previews). The catch-all block for content that does not fit the other 6 types.

**Pattern source -- AFFiNE:** `embed/src/` directory
- 6 embed subtypes: figma, github, html, iframe, loom, youtube (see `embed/src/index.ts:13-18`)
- Common base: `EmbedBlockComponent` (line 7) with shared adapter pattern
- Each subtype has its own rendering logic but shares the embed chrome (border, resize handles, toolbar)
- Edgeless mode adapter: `toEdgelessEmbedBlock` (line 11)

**Nom-native implementation:**

```rust
pub struct EmbedBlock {
    id: BlockId,
    bounds: Bounds<Pixels>,
    version: BlockVersion,
    /// What kind of embed this is.
    embed_kind: EmbedKind,
    /// The source URL or content reference.
    source: String,
    /// Title extracted from metadata or user-provided.
    title: Option<String>,
    /// Description/preview text.
    description: Option<String>,
    /// Thumbnail hash (content-addressed).
    thumbnail_hash: Option<String>,
    /// Loading state.
    load_state: EmbedLoadState,
}

pub enum EmbedKind {
    /// Generic URL with link preview card.
    Link,
    /// Rich HTML content rendered in a sandboxed frame.
    Html,
    /// Reference to another NomCanvas document.
    Document,
    /// Reference to a nomtu entity in the dictionary.
    Entity,
}

pub enum EmbedLoadState {
    Pending,
    Loaded,
    Error(String),
}
```

**Paint strategy:**
- `Link`: card with thumbnail (if available), title, description, URL domain
- `Html`: placeholder card with "HTML embed" label (actual rendering deferred to browser target)
- `Document`: inline preview of referenced document's first block
- `Entity`: nomtu entity card with kind badge, word, description, score

Note: AFFiNE has 6 vendor-specific embeds (figma, github, loom, youtube). We abstract to 4 content-type-based kinds. No vendor-specific code.

**Acceptance criteria:**
- [ ] 4 `EmbedKind` variants
- [ ] Source URL/reference stored
- [ ] Metadata (title, description, thumbnail) loaded asynchronously
- [ ] Three load states with distinct rendering
- [ ] Entity embed shows nomtu kind + word + description
- [ ] Tests: create Link embed, set source, verify load_state transitions
- [ ] Tests: Entity embed displays kind badge

### 3.9 Tests (per block)

Each block module includes `#[cfg(test)] mod tests` with:
1. **Construction test:** Create block via registry, verify kind + default properties
2. **Property round-trip test:** Set each property, read it back, verify value
3. **Bounds mutation test:** Set bounds, verify version increments
4. **Paint test:** Call paint(), verify scene contains expected primitive types and counts

**Aggregate test file:** `nom-blocks/tests/block_integration.rs`
- Creates one of each block kind
- Places all 7 on a mock canvas
- Verifies they paint without panic
- Verifies property inspector returns correct section for each

**Acceptance criteria:**
- [ ] Each of the 7 blocks has >= 4 unit tests
- [ ] Integration test creates all 7 blocks and paints them
- [ ] Total test count >= 35

---

## 4. Crate `nom-panels`

### 4.1 `panel.rs` -- Panel Trait + Layout Manager

**Pattern source -- AFFiNE:** The sidebar is 248px collapsible. Toolbar is 48px fixed top. Status bar is 24px fixed bottom. These values come from spec section 5 tokens.

**Pattern source -- ToolJet:** `AppBuilder/LeftSidebar/` implements a collapsible sidebar with inspector panels. The right side has `Inspector/` with accordion sections per component.

```rust
/// Common trait for all panels in the NomCanvas layout.
pub trait Panel {
    /// Unique panel identifier.
    fn id(&self) -> &str;

    /// Whether the panel is currently visible.
    fn visible(&self) -> bool;
    fn set_visible(&mut self, visible: bool);

    /// Fixed dimension (width for sidebars, height for bars).
    fn fixed_size(&self) -> f32;

    /// Paint the panel into the scene at the given bounds.
    fn paint(&self, scene: &mut Scene, bounds: Bounds<ScaledPixels>, theme: &tokens::Theme);
}

/// Manages the spatial arrangement of all panels around the canvas viewport.
pub struct PanelLayout {
    pub sidebar: SidebarPanel,
    pub toolbar: ToolbarPanel,
    pub preview: PreviewPanel,
    pub library: LibraryPanel,
    pub properties: PropertiesPanel,
    pub command: CommandPanel,
    pub statusbar: StatusbarPanel,
}

impl PanelLayout {
    /// Compute the canvas viewport rect after subtracting all visible panel areas.
    pub fn canvas_viewport(&self, window_size: Size<Pixels>) -> Bounds<Pixels>;

    /// Paint all visible panels.
    pub fn paint_all(&self, scene: &mut Scene, window_size: Size<Pixels>, theme: &tokens::Theme);
}
```

**Acceptance criteria:**
- [ ] `Panel` trait with id, visible, fixed_size, paint
- [ ] `PanelLayout` struct holding all 7 panels
- [ ] `canvas_viewport()` correctly subtracts panel areas from window
- [ ] Tests: all panels visible -> viewport is reduced by expected amounts
- [ ] Tests: hide sidebar -> viewport width increases by 248px

### 4.2 `sidebar.rs` -- 248px Collapsible Sidebar

**Pattern source -- AFFiNE:** Left sidebar is 248px, collapses to 0px with animation. Contains: page tree, favorites, trash, tags. Toggle via hamburger icon.

**Nom equivalent:** Contains:
- Workspace name + avatar (top section)
- Document tree (canvas pages)
- Dictionary quick-access (recent nomtu entries)
- Favorites (pinned blocks/pages)

```rust
pub struct SidebarPanel {
    visible: bool,
    /// Animation progress (0.0 = collapsed, 1.0 = expanded).
    expand_progress: f32,
    /// Currently selected section.
    active_section: SidebarSection,
    /// Sections in the sidebar.
    sections: Vec<SidebarSection>,
}

pub enum SidebarSection {
    Documents,
    Dictionary,
    Favorites,
}
```

**Paint strategy:**
- Background: `Quad` with `tokens::BG2`, width = `SIDEBAR_W * expand_progress`
- Section headers: `Text` with `fonts::LABEL` preset
- Items: `Text` with `fonts::BODY_SMALL` preset, optional icon prefix
- Active item: `Quad` highlight with `tokens::FOCUS` background
- Divider between sections: 1px `Quad` with `tokens::BORDER`

**Acceptance criteria:**
- [ ] 248px width when fully expanded
- [ ] Collapse animation via `expand_progress` interpolation
- [ ] 3 sidebar sections with navigation
- [ ] Active section highlighting
- [ ] Tests: toggle visible flips state
- [ ] Tests: expand_progress 0.0 yields 0px effective width

### 4.3 `toolbar.rs` -- 48px Top Bar

**Pattern source -- AFFiNE:** Top toolbar contains: back/forward, page title, share, mode switcher. Height is 48px.

**Nom equivalent:**
- Left group: sidebar toggle, back/forward navigation
- Center: canvas title (editable)
- Right group: mode indicators (Code/Doc/Canvas/Graph/Draw), compile button, share

```rust
pub struct ToolbarPanel {
    visible: bool,
    /// Active mode (determines which tools are highlighted).
    active_mode: CanvasMode,
    /// Canvas document title.
    title: String,
    /// Whether compile is running.
    compile_running: bool,
}

pub enum CanvasMode {
    Code,
    Doc,
    Canvas,
    Graph,
    Draw,
}
```

**Acceptance criteria:**
- [ ] 48px height
- [ ] 5 mode buttons, active one highlighted with CTA color
- [ ] Title editable (text input state)
- [ ] Compile button shows running state (spinner or green check)
- [ ] Tests: mode switch updates active_mode
- [ ] Tests: compile_running toggling changes button appearance

### 4.4 `preview.rs` -- Output Preview Panel

**Purpose:** Shows compiled output. Supports 6 preview modes (from spec section 6).

```rust
pub struct PreviewPanel {
    visible: bool,
    width: f32, // resizable, default 320px
    /// Which preview mode is active.
    preview_mode: PreviewMode,
    /// Cached preview content.
    content: PreviewContent,
}

pub enum PreviewMode {
    /// Compiled .nomx output (diagnostics, pipeline stages).
    Compile,
    /// Rendered app preview.
    App,
    /// Score/quality dashboard.
    Score,
    /// Plan output from nom-planner.
    Plan,
    /// Dream report from nom-app.
    Dream,
    /// Raw data output (tables, JSON).
    Data,
}

pub enum PreviewContent {
    Empty,
    Text(String),
    Diagnostics(Vec<Diagnostic>),
    // Future: rendered frames, app previews, charts
}
```

**Acceptance criteria:**
- [ ] 6 preview modes with tab switcher
- [ ] Resizable width (drag handle on left edge)
- [ ] Content area scrolls vertically
- [ ] Tests: mode switch updates preview_mode
- [ ] Tests: set content, verify paint produces text primitives

### 4.5 `library.rs` -- Dictionary Browser Panel

**Purpose:** Browse and search the nom dictionary. Drag entities onto canvas to create blocks.

```rust
pub struct LibraryPanel {
    visible: bool,
    width: f32, // default 300px
    /// Search query text.
    search_query: String,
    /// Filtered results from nom-search BM25 index.
    results: Vec<LibraryEntry>,
    /// Currently selected entry.
    selected: Option<usize>,
    /// Category filter.
    category_filter: Option<String>,
}

pub struct LibraryEntry {
    pub hash: String,
    pub word: String,
    pub kind: String,
    pub score: f32,
    pub description: String,
}
```

**Compiler integration:** Search queries go through `nom_search::BM25Index::search` (in-memory, on UI thread per spec section 3). Entity details fetched via `nom_dict::find_entities_by_word`.

**Acceptance criteria:**
- [ ] Search input field at top
- [ ] Results list with kind badge + word + score
- [ ] Category filter dropdown
- [ ] Drag-from-library creates new block on canvas
- [ ] Tests: search with query, verify results filtered
- [ ] Tests: select entry, verify selected index updates

### 4.6 `properties.rs` -- Property Inspector Panel

**Purpose:** When a block is selected, shows its editable properties grouped by section.

**Pattern source -- ToolJet:** `Inspector/Components/DefaultComponent.jsx:32-76`
- Properties grouped into accordion sections: Data, Events, Validation, Additional Actions
- `baseComponentProperties` function generates accordion items from `componentMeta.properties`
- Each property renders via `renderElement` utility

**Pattern source -- AFFiNE:** Each block exposes properties via signals. Property changes are captured via `store.updateBlock(model, { ...props })`.

**Nom-native implementation:**

```rust
pub struct PropertiesPanel {
    visible: bool,
    width: f32, // default 280px
    /// The currently inspected block (if any).
    inspected_block: Option<BlockId>,
    /// Properties grouped by section.
    sections: Vec<PropertySectionGroup>,
    /// Which sections are collapsed.
    collapsed_sections: HashSet<String>,
}

pub struct PropertySectionGroup {
    pub name: String,
    pub section: PropertySection,
    pub properties: Vec<BlockProperty>,
}
```

**Paint strategy:**
- When no block selected: "Select a block to inspect" placeholder
- When block selected: block kind header + accordion sections
- Each section: collapsible header + property rows
- Property rows render based on `PropertyValue` type:
  - Text -> text input field
  - Number -> number input with +/- buttons
  - Boolean -> toggle switch
  - Color -> color swatch + picker
  - Choice -> dropdown
  - Range -> slider

**Acceptance criteria:**
- [ ] Shows "Select a block" when nothing selected
- [ ] Accordion sections collapse/expand
- [ ] All 6 `PropertyValue` types have distinct rendering
- [ ] Property edits propagate back to the inspected block
- [ ] Tests: set inspected_block, verify sections populate from block.properties()
- [ ] Tests: collapse section, verify collapsed_sections set updated

### 4.7 `command.rs` -- Command Palette (Cmd+K)

**Purpose:** Quick-access command palette for searching blocks, actions, and dictionary entries.

```rust
pub struct CommandPanel {
    visible: bool, // false by default, toggled via Cmd+K
    /// Search query.
    query: String,
    /// Matched commands/entities.
    results: Vec<CommandResult>,
    /// Currently highlighted result.
    highlighted: usize,
}

pub struct CommandResult {
    pub icon: IconId,
    pub label: String,
    pub description: String,
    pub action: CommandAction,
    pub score: f32,
}

pub enum CommandAction {
    CreateBlock(BlockKind),
    NavigateTo(String), // page/document ID
    RunCompile,
    ToggleMode(CanvasMode),
    OpenPanel(&'static str),
    SearchDictionary(String),
}
```

**Paint strategy:**
- Modal overlay centered on screen with `tokens::MODAL_RADIUS` (22px)
- Search input at top
- Results list below with icon + label + description
- Highlighted result has `tokens::FOCUS` background
- Keyboard navigation: Up/Down to move, Enter to execute, Escape to close

**Acceptance criteria:**
- [ ] Modal overlay with 22px border radius
- [ ] Search filters results by label match
- [ ] Keyboard navigation: up/down/enter/escape
- [ ] 6 command action types
- [ ] Tests: type query, verify results filter
- [ ] Tests: arrow down moves highlighted index
- [ ] Tests: enter on CreateBlock returns correct BlockKind

### 4.8 `statusbar.rs` -- 24px Bottom Bar

**Purpose:** Status information at the bottom of the window.

```rust
pub struct StatusbarPanel {
    visible: bool,
    /// Left section: mode indicator + cursor position.
    cursor_info: String,
    /// Center section: compilation status.
    compile_status: CompileStatus,
    /// Right section: zoom level + dict entry count + score.
    zoom_percent: f32,
    dict_entry_count: u64,
    current_score: Option<f32>,
}

pub enum CompileStatus {
    Idle,
    Compiling,
    Success { elapsed_ms: u64 },
    Error { message: String },
}
```

**Acceptance criteria:**
- [ ] 24px height
- [ ] Three sections: left (cursor info), center (compile status), right (zoom + stats)
- [ ] Compile status shows 4 states with distinct colors
- [ ] Zoom displayed as percentage
- [ ] Tests: set compile_status to each variant, verify text output

### 4.9 Tests (per panel)

Each panel module includes `#[cfg(test)] mod tests` with:
1. **Visibility toggle test**
2. **Paint test:** Verify scene primitives emitted for default state
3. **Interaction test:** Verify state mutations (e.g., search query in library)

**Aggregate test file:** `nom-panels/tests/layout_integration.rs`
- Creates `PanelLayout` with all panels
- Computes canvas viewport with various visibility combinations
- Verifies viewport math (window - toolbar - statusbar - sidebar = canvas area)

**Acceptance criteria:**
- [ ] Each of the 7 panels has >= 3 unit tests
- [ ] Layout integration test with at least 4 visibility combinations
- [ ] Total test count >= 25

---

## 5. Dependency Graph

```
nom-theme (no deps within workspace)
    |
    v
nom-blocks --> nom-canvas-core (Element, Bounds, Viewport)
           --> nom-editor (EditorState for NomxBlock)
           --> nom-gpui (Scene, Rgba, Pixels, primitives)
           --> nom-theme (tokens, fonts, icons)
    |
    v
nom-panels --> nom-blocks (Block trait, BlockKind, BlockProperty)
           --> nom-gpui (Scene, Element trait, rendering)
           --> nom-theme (tokens, fonts, icons)
```

**Cargo.toml additions to workspace:**

```toml
[workspace]
members = [
    "crates/nom-gpui",
    "crates/nom-canvas-core",
    "crates/nom-editor",
    "crates/nom-theme",       # NEW
    "crates/nom-blocks",      # NEW
    "crates/nom-panels",      # NEW
]
```

**Workspace dependency additions:**
```toml
[workspace.dependencies]
# Existing deps used by new crates:
# ropey, smallvec, thiserror, parking_lot (already in workspace)
# No new external dependencies required for Phase 3.
```

---

## 6. Parallel Dispatch Strategy

Phase 3 can be executed in 3 cycles with up to 10 agents per cycle.

### Cycle 1: Foundation (nom-theme + nom-blocks scaffold)

| Agent | Task | Files | Depends On |
|-------|------|-------|------------|
| A1 | `nom-theme` crate: Cargo.toml + lib.rs + tokens.rs + fonts.rs + icons.rs | 5 files | Nothing |
| A2 | `nom-blocks` scaffold: Cargo.toml + lib.rs + block.rs (trait + registry + property types) | 3 files | Nothing |
| A3 | `nom-blocks/prose.rs` (RopeWithSpans + InlineFormat + ProseVariant) | 1 file | A2 (block trait) |
| A4 | `nom-blocks/nomx.rs` (NomxBlock shell + editor integration stubs) | 1 file | A2 (block trait) |
| A5 | `nom-blocks/media.rs` (MediaBlock + MediaLoadState state machine) | 1 file | A2 (block trait) |
| A6 | `nom-blocks/drawing.rs` (DrawingBlock + Stroke + spline math) | 1 file | A2 (block trait) |
| A7 | `nom-blocks/table.rs` (TableBlock + ColumnDef + CellValue) | 1 file | A2 (block trait) |
| A8 | `nom-blocks/graph_node.rs` (GraphNodeBlock + Port + PortType) | 1 file | A2 (block trait) |
| A9 | `nom-blocks/embed.rs` (EmbedBlock + EmbedKind) | 1 file | A2 (block trait) |
| A10 | `nom-blocks/tests/block_integration.rs` | 1 file | A2 (block trait) |

**Note:** A3-A10 can start as soon as A2 pushes the block trait (within cycle 1). A1 runs fully independently.

### Cycle 2: Panels

| Agent | Task | Files | Depends On |
|-------|------|-------|------------|
| B1 | `nom-panels` scaffold: Cargo.toml + lib.rs + panel.rs (trait + PanelLayout) | 3 files | Cycle 1 |
| B2 | `nom-panels/sidebar.rs` | 1 file | B1 |
| B3 | `nom-panels/toolbar.rs` | 1 file | B1 |
| B4 | `nom-panels/preview.rs` | 1 file | B1 |
| B5 | `nom-panels/library.rs` | 1 file | B1 |
| B6 | `nom-panels/properties.rs` | 1 file | B1 |
| B7 | `nom-panels/command.rs` | 1 file | B1 |
| B8 | `nom-panels/statusbar.rs` | 1 file | B1 |
| B9 | `nom-panels/tests/layout_integration.rs` | 1 file | B1 |
| B10 | Block integration tests (flesh out block_integration.rs from A10) | 1 file | Cycle 1 |

### Cycle 3: Polish + Compilation Verification

| Agent | Task | Files | Depends On |
|-------|------|-------|------------|
| C1 | `cargo check --workspace` -- fix compilation errors across all 3 new crates | All | Cycle 2 |
| C2 | `cargo test --workspace` -- fix failing tests | All | C1 |
| C3 | Paint integration: verify each block type renders at least 1 primitive | test file | C2 |
| C4 | Panel layout math verification: window sizes 1920x1080, 1280x720, 800x600 | test file | C2 |
| C5 | Update workspace Cargo.toml + verify `cargo build --workspace` succeeds | Cargo.toml | C2 |

---

## 7. Testing Strategy

### 7.1 Unit Tests (per module)

| Module | Min Tests | Focus |
|--------|-----------|-------|
| `tokens.rs` | 5 | Value correctness, color component ranges |
| `fonts.rs` | 3 | Preset completeness, family strings |
| `icons.rs` | 3 | Non-empty paths, bounding box |
| `block.rs` | 8 | Registry round-trip, property system |
| `prose.rs` | 7 | RopeWithSpans CRUD, variant rendering, collapse |
| `nomx.rs` | 6 | Editor state, highlight cache, preview toggle |
| `media.rs` | 5 | Load state machine, playback bounds |
| `graph_node.rs` | 6 | Port connections, wire validation, execution state |
| `drawing.rs` | 5 | Stroke building, pressure scaling, spline |
| `table.rs` | 6 | Sort, select, add/remove rows, hide columns |
| `embed.rs` | 4 | Embed kinds, load states |
| `panel.rs` | 4 | Layout math, visibility toggling |
| `sidebar.rs` | 3 | Expand/collapse, section navigation |
| `toolbar.rs` | 3 | Mode switching, compile state |
| `preview.rs` | 3 | Mode switching, content display |
| `library.rs` | 3 | Search filtering, selection |
| `properties.rs` | 4 | Accordion collapse, property type rendering |
| `command.rs` | 4 | Search filtering, keyboard navigation |
| `statusbar.rs` | 3 | Three sections, compile status variants |

**Total unit tests: ~85**

### 7.2 Integration Tests

| Test File | Tests | Focus |
|-----------|-------|-------|
| `nom-blocks/tests/block_integration.rs` | 5 | All 7 blocks created, painted, properties inspected |
| `nom-panels/tests/layout_integration.rs` | 5 | Viewport math, panel visibility combinations |

**Total integration tests: ~10**

### 7.3 Test Commands

```bash
# All tests
cargo test --workspace -p nom-theme -p nom-blocks -p nom-panels

# Single crate
cargo test -p nom-blocks

# Single module
cargo test -p nom-blocks -- prose::tests
```

---

## 8. Estimated Completion

| Cycle | Duration | Agents | Deliverable |
|-------|----------|--------|-------------|
| Cycle 1 | 1 cron cycle | 10 | nom-theme + nom-blocks (all 7 blocks) |
| Cycle 2 | 1 cron cycle | 10 | nom-panels (all 7 panels) |
| Cycle 3 | 1 cron cycle | 5 | Compilation fixes + test verification |

**Total: 3 cron cycles at up to 10 parallel agents per cycle.**

### Exit Criteria (Phase 3 complete when):
- [ ] `cargo check --workspace` passes with zero errors and zero warnings
- [ ] `cargo test --workspace` passes all ~95 tests
- [ ] All 7 block types can be created via BlockRegistry
- [ ] All 7 panels render to scene graph
- [ ] PanelLayout.canvas_viewport() correctly computes available canvas area
- [ ] No foreign identifiers anywhere in the codebase
- [ ] No wrapper/adapter layers -- every function does real work
- [ ] No leftover TODO(critical) items

---

## 9. Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| `nom-editor` crate API not stable for NomxBlock embedding | Medium | NomxBlock uses `EditorState` via public API only; if API changes, adapter is internal to NomxBlock (not a separate crate) |
| Drawing spline math is complex | Low | Use simple Catmull-Rom (well-documented algorithm); fidelity improvements can be deferred |
| Table block scope creep (kanban, gallery views) | Medium | Phase 3 ships Table view only; other views are Phase 5 |
| nom-theme token values drift from spec | Low | Tests pin every value against spec section 5; any drift fails CI |
| GPU primitive types in nom-gpui may need extension | Medium | Drawing block needs `Path` primitive; verify it exists in `nom-gpui/scene.rs` before Cycle 1 |

---

## 10. File Manifest

```
nom-canvas/crates/
  nom-theme/
    Cargo.toml
    src/
      lib.rs
      tokens.rs       (~200 lines)
      fonts.rs         (~100 lines)
      icons.rs         (~100 lines)

  nom-blocks/
    Cargo.toml
    src/
      lib.rs
      block.rs         (~250 lines -- trait, registry, property types)
      prose.rs         (~400 lines -- RopeWithSpans, 6 variants, paint)
      nomx.rs          (~350 lines -- editor embed, highlight cache, preview)
      media.rs         (~300 lines -- 3 kinds, load state machine, playback)
      graph_node.rs    (~350 lines -- ports, wire validation, execution state)
      drawing.rs       (~300 lines -- strokes, pressure, spline, eraser)
      table.rs         (~350 lines -- columns, cells, sort, selection)
      embed.rs         (~200 lines -- 4 kinds, metadata, load state)
    tests/
      block_integration.rs  (~100 lines)

  nom-panels/
    Cargo.toml
    src/
      lib.rs
      panel.rs         (~150 lines -- trait, PanelLayout, viewport math)
      sidebar.rs       (~250 lines -- sections, expand animation)
      toolbar.rs       (~200 lines -- mode buttons, compile indicator)
      preview.rs       (~250 lines -- 6 modes, content area)
      library.rs       (~250 lines -- search, results, drag)
      properties.rs    (~300 lines -- accordion, 6 value renderers)
      command.rs       (~250 lines -- modal, search, keyboard nav)
      statusbar.rs     (~150 lines -- three sections, compile status)
    tests/
      layout_integration.rs  (~80 lines)
```

**Total new files: 25**
**Total new lines: ~4,880**

---

## 11. Reference Citations

### AFFiNE BlockSuite Patterns Borrowed

| Pattern | Source File | What We Took |
|---------|------------|--------------|
| Block base class with lifecycle | `code/src/code-block.ts:39` (`CaptionedBlockComponent`) | Block trait with paint + properties |
| Store/Schema registration | `paragraph/src/store.ts:9-17` (`StoreExtensionProvider`) | BlockRegistry factory pattern |
| Signal-based reactive props | `code/src/code-block.ts:44-64` (`signal`, `computed`) | Property change tracking via version counter |
| Rich text with inline formatting | `paragraph/src/paragraph-block.ts:339-352` (`<rich-text>`) | RopeWithSpans + InlineFormat |
| Heading collapse | `paragraph/src/paragraph-block.ts:247-266` (`collapsed` + `calculateCollapsedSiblings`) | ProseVariant::Heading with collapse state |
| Code block syntax highlighting | `code/src/code-block.ts:105-156` (Shiki token caching) | NomxBlock highlight_tokens cache (using nom-concept S1 instead of Shiki) |
| Image resource loading | `image/src/image-block.ts:42-45` (`ResourceController`) | MediaBlock load_state machine |
| Data view with DataSource | `data-view/src/data-view-block.ts:216-223` (`BlockQueryDataSource`) | TableBlock with ColumnDef abstraction |
| Embed subtypes | `embed/src/index.ts:13-18` (6 vendor embeds) | 4 content-type-based EmbedKind (no vendor specifics) |
| Surface renderer | `surface/src/renderer/canvas-renderer.ts:1-80` | DrawingBlock stroke-to-path rendering approach |
| 248px sidebar | AFFiNE layout constants | tokens::SIDEBAR_W |

### ToolJet Widget Patterns Borrowed

| Pattern | Source File | What We Took |
|---------|------------|--------------|
| Component property resolution | `component-properties-resolution.js:15-29` (`resolveProperties`) | BlockProperty with PropertyValue variants |
| Property sections (Data/Style/Events) | `Inspector/Components/DefaultComponent.jsx:50-55` (section splitting) | PropertySection enum (Data, Style, Layout, Events) |
| Widget rendering dispatch | `AppBuilder/AppCanvas/RenderWidget.jsx:50-62` (`getComponentToRender`) | BlockRegistry.create() factory dispatch |
| Resolved properties + styles | `RenderWidget.jsx:73-90` (Zustand store binding) | Properties panel reads block.properties() |
| Accordion inspector | `DefaultComponent.jsx:75` (`<Accordion items={accordionItems}>`) | PropertiesPanel accordion sections |
| Component definition metadata | `component-properties-resolution.js:16-28` (`component.definition.properties`) | Block.properties() returns Vec<BlockProperty> |
