# NomCanvas IDE — Design Specification

> **Date:** 2026-04-17
> **Status:** COMPLETE — 228 upstreams + ~100 services scanned end-to-end (11 parallel agents)
> **Compiler baseline:** 1067 tests, 29 crates, GAP-8 AI loop closed, GAP-12 complete
> **Repos scanned:** 228 upstream repos + 19 service categories (~100 repos) = ~328 repos total
> **HARD REQUIREMENT:** Every pattern records source path. Executors MUST study source carefully.
> **HARD REQUIREMENT:** Zero foreign repo identities in Nom codebase. Abstract patterns only.

---

## 1. Vision

NomCanvas is a **canvas-first IDE** where natural language becomes compilable code, apps, video, pictures, web pages, and systems. The canvas is the primary authoring surface — not a code editor with visual features bolted on, but a spatial canvas where prose blocks gradually transform into valid `.nomx` source that the Nom compiler can build into native artifacts.

**What makes this different from every other IDE:**
- Every word the user types is a potential nomtu lookup — the dictionary IS the autocomplete
- Blocks transform from prose to code live, not via a generate-then-edit cycle
- The same canvas surface produces apps, media, documents, and system binaries
- The compiler is a deterministic oracle; AI assists but doesn't own the output

---

## 2. Architecture Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| IDE paradigm | Canvas-first | Nom's NL syntax blurs prose/code; spatial canvas is natural fit |
| Deployment | Tauri hybrid (Rust backend + web frontend) | Direct nom-compiler crate access + rich web canvas ecosystem |
| Primary interaction | Inline prose-to-nomx transformation | Uniquely Nom — every word is a nomtu lookup |
| Canvas engine | Custom (Canvas API + Prosemirror) | Lighter than BlockSuite fork, faster than GPUI port |
| Text editing | Prosemirror (not CodeMirror) | Rich text with inline decorations for progressive nomtu highlighting |
| Real-time matching | WASM bridge from nom-grammar | Sub-millisecond keystroke feedback without IPC round-trip |
| Compilation | Tauri commands to nom-compiler | Zero serialization overhead; direct Rust crate calls |

---

## 3. System Architecture

```
┌─────────────────────────────────────────────────────┐
│                    Tauri Shell                        │
│  ┌───────────────────────┐  ┌──────────────────────┐ │
│  │    Web Frontend (TS)   │  │   Rust Backend       │ │
│  │                        │  │                      │ │
│  │  ┌────────────────┐   │  │  nom-compiler/*      │ │
│  │  │ Spatial Canvas  │   │  │  nom-lsp (direct)    │ │
│  │  │  (Canvas API)   │   │  │  nom-dict (direct)   │ │
│  │  ├────────────────┤   │  │  nom-intent           │ │
│  │  │ Block Engine    │   │  │  nom-llvm            │ │
│  │  │ (Prosemirror)  │◄──┼──┤  nom-app             │ │
│  │  ├────────────────┤   │  │  nom-media            │ │
│  │  │ NomtuMatcher   │   │  │  nom-grammar          │ │
│  │  │ (WASM bridge)  │   │  │                      │ │
│  │  ├────────────────┤   │  │  16 Tauri Commands:   │ │
│  │  │ Preview Pane   │   │  │  - compile_block      │ │
│  │  │ (live output)  │   │  │  - lookup_nomtu       │ │
│  │  └────────────────┘   │  │  - match_grammar      │ │
│  └───────────────────────┘  │  - build_artifact     │ │
│                              │  - dream_report       │ │
│                              │  - score_block        │ │
│                              │  - wire_check         │ │
│                              │  - search_dict        │ │
│                              │  - resolve_intent     │ │
│                              │  - ingest_media       │ │
│                              │  - hover_info         │ │
│                              │  - complete_word      │ │
│                              │  - extract_atoms      │ │
│                              │  - platform_spec      │ │
│                              │  - plan_flow          │ │
│                              │  - security_scan      │ │
│                              └──────────────────────┘ │
└─────────────────────────────────────────────────────┘
```

### 3.1 Rust Backend (Tauri Commands)

Zero new logic — every command delegates to existing nom-compiler crates:

| Command | Crate | Key Function | Target Latency |
|---------|-------|-------------|----------------|
| `lookup_nomtu` | nom-dict | `find_entities`, `resolve_prefix` (60+ fns) | <5ms |
| `match_grammar` | nom-grammar | `search_patterns` (Jaccard), `resolve_synonym` (20+ fns) | <10ms |
| `compile_block` | nom-concept | `run_pipeline` S1→S6 (6 staged annotators, 13 S5 sub-stages) | <500ms |
| `build_artifact` | nom-llvm | `compile`, `link_bitcodes`, `compile_plans` | <5s |
| `hover_info` | nom-lsp | `format_hover_markdown` (contracts/effects/retry/scores) | <10ms |
| `complete_word` | nom-resolver | `resolve` (3-stage: exact→word→semantic), 48-col NomtuEntry | <50ms |
| `dream_report` | nom-app | `dream_report` (score 0-100), `criteria_proposals`, Pareto front | <1s |
| `score_block` | nom-score | `score_atom` (8-dim: security/reliability/perf/read/test/port/compose/maturity) | <5ms |
| `wire_check` | nom-score | `can_wire` → Compatible/NeedsAdapter/Incompatible | <5ms |
| `search_dict` | nom-search | `BM25Index::search` + `reciprocal_rank_fusion` | <50ms |
| `resolve_intent` | nom-intent | `classify_with_react` (5-tool ReAct, NomCliAdapter offline oracle) | <2s |
| `ingest_media` | nom-media | 10 codecs: PNG/JPEG/AVIF/FLAC/Opus/AAC/AV1/WebM/MP4/HEVC | <5s |
| `extract_atoms` | nom-extract | `extract_from_dir` (53 languages, tree-sitter, 50+ concept hints) | <10s |
| `platform_spec` | nom-ux | `Platform::runtime_launch_word` (web/desktop/mobile) | <1ms |
| `plan_flow` | nom-planner | `plan_from_pipeline_output` + 4 fusion passes | <100ms |
| `security_scan` | nom-security | `scan_body` (50+ patterns) + `check_guardrails` | <500ms |

### 3.2 Web Frontend

- **Spatial Canvas (Canvas API):** Infinite canvas with zoom, pan, block positioning. Each block is a rectangular region with drag handles, connection ports, and a content area.
- **Block Engine (Prosemirror):** Each block's content area is a Prosemirror instance with custom node types and decorations for nomtu highlighting.
- **NomtuMatcher (WASM):** Compiled from nom-grammar's Jaccard matcher. Runs in the browser for instant keystroke-level feedback. Heavy compilation stays in Rust.
- **Preview Pane:** Shows the compiled output of the focused block — app UI preview, media render, code artifact, or error diagnostics.

### 3.3 IPC Strategy

Tauri's invoke mechanism passes JSON between web and Rust. For the common case (word lookup), the round-trip is:
1. User types a word → JS debounce (50ms)
2. `invoke('lookup_nomtu', {word})` → Rust does SQLite query on entities.sqlite
3. Result returns → Prosemirror decoration applied

For real-time keystroke matching (no IPC delay), the WASM bridge handles pattern matching locally.

---

## 4. Block System

### 4.1 Block Types

| Block Kind | Content | Compilation Target | v1 | v2 | v3 | v4 |
|------------|---------|-------------------|----|----|----|----|
| `prose` | Free-form natural language | None (draft) | Yes | | | |
| `nomx` | Valid .nomx source | S1-S6 pipeline → LLVM artifact | Yes | | | |
| `media-node` | Media generation DAG node | nom-media render pipeline | | | Yes | |
| `app-preview` | App component composition | nom-app build | | Yes | | |
| `drawing` | Freeform spatial sketch | Reference only | Yes | | | |
| `data-view` | Structured data (table/kanban) | Schema → entity | | Yes | | |

### 4.2 Block Data Model

```
Block {
  id: UUID,
  kind: BlockKind,
  position: {x: f64, y: f64, width: f64, height: f64},
  content: ProsemirrorDoc,
  compilation_state: draft | parsing | compiled | error,
  nomtu_matches: [{word: String, entity_hash: String, confidence: f64}],
  edges: [{target_block_id: UUID, edge_type: EdgeKind}],
  artifact: Option<{hash: String, mime: String, preview_url: String}>,
  metadata: {created_at, updated_at, author}
}
```

### 4.3 Inline Transformation Lifecycle

1. User creates a prose block and types: `"a function that doubles a number"`
2. NomtuMatcher (WASM) scans each word against grammar.sqlite patterns
3. Prosemirror decorations render: green underlines = matched nomtu, gray = unmatched, orange = ambiguous
4. User refines: `"define double_value given input of number, returns number by returning input times two"`
5. All words now match — block border turns green (ready to compile)
6. User hits Ctrl+Enter → Tauri `compile_block` runs S1-S6 pipeline
7. If successful: block kind upgrades `prose` → `nomx`, full syntax highlighting applied
8. Preview pane shows: compiled artifact info, IR output, or rendered result

### 4.4 Block Connections

Blocks connect via typed edges (mirroring nom-dict's 28 edge types):
- **DataFlow:** Output of one block feeds input of another
- **Composition:** One block contains/uses entities from another
- **Reference:** Prose block references a nomx block for context

Connections render as curves on the canvas between block ports.

---

## 5. Versioned Roadmap

### v1 — Foundation (Target: first usable prototype)

| Component | Description | Nom Crates Used |
|-----------|-------------|-----------------|
| Tauri shell | Rust backend + web frontend scaffold | nom-cli (reference) |
| Spatial canvas | Infinite canvas with zoom/pan/blocks | — |
| Prosemirror blocks | Rich text with nomtu decorations | — |
| WASM NomtuMatcher | Real-time word matching in browser | nom-grammar |
| Tauri commands | compile_block, lookup_nomtu, etc. | nom-dict, nom-concept, nom-llvm, nom-lsp |
| Inline transformation | Prose → nomx progressive highlighting | nom-grammar, nom-dict |
| Live compilation | S1-S6 pipeline results as decorations | nom-concept |
| Preview pane | Rendered output of compiled blocks | nom-llvm |
| LSP integration | Hover, goto-def, diagnostics on blocks | nom-lsp |

### v2 — App Builder

| Component | Description | Pattern Source |
|-----------|-------------|---------------|
| Component palette | Dictionary browser for drag-and-drop entities | Low-code app composition |
| Data flow wiring | Visual edges between blocks | Workflow orchestration DAG |
| App preview | Live app rendering from nom-app build | Streaming artifact rendering |
| Data view blocks | Table/kanban/form views of structured data | Multi-view data presentation |

### v3 — Media Pipeline

| Component | Description | Pattern Source |
|-----------|-------------|---------------|
| Media node blocks | Typed input/output ports for media generation | Node-based DAG execution |
| Pipeline execution | Dependency-tracked, partial re-execution | Workflow engine with replay |
| Media preview | Video/image/audio inline in canvas blocks | Multi-modal content synthesis |
| Timeline view | Temporal sequencing for video composition | Storyboard assembly |

### v4 — Intelligence

| Component | Description | Pattern Source |
|-----------|-------------|---------------|
| Output prediction | Preview compilation result before running | Distributed simulation |
| Dream mode | Agent-loop generation with DreamReport scoring | Multi-agent orchestration |
| Smart search | Hybrid keyword + semantic search across canvas | Hybrid search and ranking |
| Auto-fix loop | Compilation error → suggest fix → re-compile | Recovery loop pattern |

### v5 — System Targets

| Component | Description |
|-----------|-------------|
| Cross-platform build | Canvas → native binary for any OS target |
| Embedded targets | Canvas → firmware/embedded binary |
| Cloud deployment | Canvas → deployed web service |

---

## 6. Abstract Patterns Catalog (from 228 upstreams + ~100 services)

> Every pattern below is described abstractly. Zero foreign repo names, zero brand names.
> These are the valuable functions preserved as native Nom capabilities.
>
> **HARD REQUIREMENT:** Every pattern records its **source path** (folder + key files).
> Executors MUST study the source pattern carefully before implementing the native Nom version.
> The source path is for study reference ONLY — no foreign identity enters the Nom codebase.

### 6.1 Patterns from Services

#### Workflow Orchestration
- **Declarative workflow DAG:** Visual node-based execution graph with typed inputs/outputs, cycle detection, partial replay
- **Source:** `APP/Accelworld/services/automation/n8n/` — study `packages/workflow/`, `packages/nodes-base/`, `packages/editor-ui/`
- **Source:** `APP/Accelworld/services/engine/deer-flow/` — study `src/` flow engine architecture
- **Nom mapping:** Canvas block connections ARE the workflow DAG; `compile_block` executes nodes; partial re-compilation on edit

#### Agent Coordination
- **Multi-agent governance:** Hierarchical agent orchestration with budget constraints, task checkout, approval gates
- **Source:** `APP/Accelworld/services/engine/agentscope/` — study agent role definitions, state management, communication protocols
- **Source:** `APP/Accelworld/services/consciousness/claude-subconscious/` — study memory block diffing, context-aware injection
- **Source:** `APP/Accelworld/services/other3/deepagents-main/` — study multi-agent planning pipeline
- **Source:** `APP/Accelworld/services/other3/hermes-agent-main/` — study agent-to-agent protocol
- **Agent-to-agent protocol:** Standardized discovery, capability declaration, async request/response
- **Skill-based execution:** Modular skill registry with dependency injection, sandbox isolation
- **Nom mapping:** v4 dream mode uses agent coordination; skill registry maps to nomtu entity kinds

#### Intelligence & Prediction
- **Semantic code knowledge graph:** AST → knowledge graph with symbol resolution, dependency clustering, process detection
- **Source:** `APP/Accelworld/services/intelligence/gitnexus/` — study `src/analyze/`, symbol resolution, Leiden clustering, process detection
- **Distributed simulation:** Multi-agent environment with personality modeling, memory consolidation, scenario branching
- **Source:** `APP/Accelworld/services/intelligence/mirofish/` — study simulation_config_generator, agent personality framework, outcome aggregation
- **Hybrid search & ranking:** BM25 + semantic vector + reciprocal rank fusion with confidence scoring
- **Source:** `APP/Accelworld/services/intelligence/gitnexus/` — study `src/search/`, BM25+semantic hybrid
- **Trend analysis and monitoring:** Multi-feed aggregation, signal detection, composite risk scoring
- **Source:** `APP/Accelworld/services/intelligence/trendradar/` — study feed normalization, anomaly detection
- **Source:** `APP/Accelworld/services/intelligence/worldmonitor/` — study multi-stream correlation, spatial mapping
- **Source:** `APP/Accelworld/services/intelligence/bettafish/` — study game-tree evaluation patterns
- **Nom mapping:** Already built (nom-graph, nom-search, nom-resolver); canvas surfaces these as visual tools

#### Media Synthesis
- **Multi-modal content pipeline:** Text → script analysis → scene generation → storyboard → video with temporal alignment
- **Source:** `APP/Accelworld/services/media/waoowaoo/` — study script parsing, character/scene generation, temporal sequencing, BullMQ workers
- **Conditional processing:** Intelligent mode switching based on input type (text-to-image vs image-to-image)
- **Source:** `APP/Accelworld/services/media/Open-Higgsfield-AI-main/` — study dual-mode branching (50+ t2i models, 55+ i2i models)
- **Node-based DAG execution:** Typed ports, dependency tracking, partial re-execution on change
- **Source:** `APP/Accelworld/services/other2/ComfyUI-master/` — study `INPUT_TYPES`, `RETURN_TYPES`, `FUNCTION`, `NODE_CLASS_MAPPINGS`, execution engine
- **Video generation from text:** Automated video creation pipeline
- **Source:** `APP/Accelworld/services/media/moneyprinter/` — study text-to-video pipeline stages
- **Source:** `APP/Accelworld/services/other2/MoneyPrinterV2-main/` — study enhanced pipeline
- **Nom mapping:** v3 media-node blocks implement this natively via nom-media

#### Data & Documents
- **Structured data pipeline:** Transform unstructured input into tables with multi-view (grid, kanban, calendar, form)
- **Source:** `APP/Accelworld/services/crm/twenty/` — study entity schema builder, trigger-action automation, view personalization
- **Source:** `APP/Accelworld/services/other5/nocodb-main/` — study multi-view data presentation
- **Document assembly IR:** Template selection, layout planning, word budget, validation, block-level rendering
- **Source:** `APP/Accelworld/services/intelligence/mirofish/` — study report pipeline: template → layout → chapter JSON → HTML/PDF
- **PDF transformation pipeline:** 50+ modular document operations
- **Source:** `APP/Accelworld/services/other5/stirling-pdf-main/` — study modular tool composition
- **Source:** `APP/Accelworld/services/other3/PDFMathTranslate-main/` — study math-aware document processing
- **Nom mapping:** v2 data-view blocks; document generation via nom-app

#### Infrastructure
- **MCP server composition:** Namespace composition, tool filtering, middleware pipeline, endpoint provisioning
- **Source:** `APP/Accelworld/services/mcp/metamcp/` — study server registration, namespace composition, tool override/annotation, middleware
- **Source:** `APP/Accelworld/services/other4/mcp-toolbox-main/` — study database MCP server pattern
- **Persistent memory:** Hierarchical memory (L0 summary / L1 overview / L2 full content) with semantic search
- **Source:** `APP/Accelworld/services/memory/openviking/` — study content tiering, AGFS mounting, transaction journals
- **Real-time synchronization:** Low-latency multi-user with conflict resolution and selective subscription
- **Source:** `APP/Accelworld/services/other5/livekit-main/` — study WebRTC infrastructure, selective subscription
- **Provider orchestration:** Multi-layer fallback routing with quota tracking and auto-switching
- **Source:** `APP/Accelworld/services/other4/9router-master/` — study subscription→cheap→free routing layers
- **Workflow app platform:** LLM-powered app composition with RAG, agents, observability
- **Source:** `APP/Accelworld/services/other4/dify-main/` — study visual workflow builder, RAG pipeline, agent framework
- **Nom mapping:** MCP integration via nom-intent; memory via nom-dict; real-time sync future v2+

#### Security & Threat Detection
- **Autonomous threat detection and remediation:** Multi-agent security scanning with proof-of-concept validation
- **Source:** `APP/Accelworld/services/security/strix/` — study dynamic vulnerability scanning, auto-fix suggestion, severity prioritization
- **Security analysis tooling:** Various security scanning patterns
- **Source:** `APP/Accelworld/services/other/SafeLine-main/` — study WAF patterns
- **Source:** `APP/Accelworld/services/other/wazuh-main/` — study SIEM pattern
- **Source:** `APP/Accelworld/services/other/web-check-master/` — study web security analysis
- **Source:** `APP/Accelworld/services/other/spiderfoot-master/` — study OSINT entity correlation
- **Source:** `APP/Accelworld/services/other/maigret-main/` — study username enumeration patterns
- **Source:** `APP/Accelworld/services/other/sherlock-master/` — study cross-platform identity resolution
- **Source:** `APP/Accelworld/services/other/personal-security-checklist-master/` — study security rule catalogs
- **Nom mapping:** nom-security crate for canvas-level code validation; security scanning as block decoration

#### Financial & Trading
- **Event-driven trading execution:** Real-time data ingestion, technical indicators, position management
- **Source:** `APP/Accelworld/services/trading/nautilus_trader/` — study event-driven architecture, multi-asset normalization
- **Source:** `APP/Accelworld/services/trading/ai-trader/` — study signal generation, backtesting
- **Source:** `APP/Accelworld/services/trading/trading-agents/` — study multi-agent trading coordination
- **Source:** `APP/Accelworld/services/trading/lean/` — study algorithmic framework, quantitative analysis
- **Source:** `APP/Accelworld/services/trading/ctrader-mcp/` — study MCP-based trading bridge
- **Source:** `APP/Accelworld/services/other2/TradingAgents-main/` — study agent-based trading
- **Source:** `APP/Accelworld/services/other2/AI-Trader-main/` — study AI-driven trading strategies
- **Source:** `APP/Accelworld/services/other2/OpenBB-develop/` — study financial data aggregation
- **Source:** `APP/Accelworld/services/other3/ghostfolio-main/` — study portfolio management
- **Nom mapping:** Financial entities as nomtu entries; trading workflows as canvas DAGs

#### Collaboration & UI
- **Social media automation:** Scheduling, analytics, multi-platform publishing
- **Source:** `APP/Accelworld/services/automation/postiz/` — study multi-platform publish pipeline
- **Multi-backend UI adaptation:** Feature detection, graceful degradation, API mocking
- **Source:** `APP/Accelworld/services/ui/mangane/` — study backend capability discovery, fallback states
- **Source:** `APP/Accelworld/services/ui/openspace-dashboard/` — study dashboard composition
- **Collaborative workspace:** Real-time document editing with block model
- **Source:** `APP/Accelworld/services/other5/AppFlowy-main/` — study block-based document model
- **Source:** `APP/Accelworld/services/other5/huly-main/` — study project management workflow
- **Source:** `APP/Accelworld/services/other5/excalidraw-main/` — study canvas drawing primitives
- **Nom mapping:** Canvas collaboration layer in v2+

#### AI Research & Science
- **Automated scientific research pipeline:** Hypothesis → experiment → analysis → paper
- **Source:** `APP/Accelworld/services/other2/AI-Scientist-v2-main/` — study research automation pipeline
- **RAG over any document type:** Universal document → retrieval pipeline
- **Source:** `APP/Accelworld/services/other2/RAG-Anything-main/` — study multi-format RAG
- **Voice synthesis:** Real-time voice generation and cloning
- **Source:** `APP/Accelworld/services/other2/VibeVoice-main/` — study voice synthesis pipeline
- **Agent orchestration at scale:** Lightning-fast agent spawning
- **Source:** `APP/Accelworld/services/other2/agent-lightning-main/` — study agent pool management
- **Nom mapping:** Research pipeline as canvas workflow; RAG via nom-search; voice as media block

#### Business & ERP
- **Accounting system:** Chart of accounts, double-entry ledger, reporting
- **Source:** `APP/Accelworld/services/other3/akaunting-master/` — study accounting models
- **Enterprise resource planning:** Full business process automation
- **Source:** `APP/Accelworld/services/other3/erpnext-develop/` — study module composition pattern
- **Source:** `APP/Accelworld/services/other3/odoo-19.0/` — study modular ERP architecture
- **Billing & subscription:** Usage-based billing, metering, invoicing
- **Source:** `APP/Accelworld/services/other3/lago-main/` — study billing pipeline
- **Scheduling & booking:** Availability rules, event-driven automations
- **Source:** `APP/Accelworld/services/other5/calcom-main/` — study scheduling logic
- **Nom mapping:** Business entities as nomtu; ERP modules as canvas-composable blocks

#### DevOps & Tools
- **Hex editor / binary analysis:** Binary structure inspection and editing
- **Source:** `APP/Accelworld/services/other/ImHex-master/` — study pattern language for binary formats
- **Debugger:** Multi-architecture debugging
- **Source:** `APP/Accelworld/services/other/x64dbg-development/` — study debugger architecture
- **Browser automation:** Single-file web archiving
- **Source:** `APP/Accelworld/services/other/SingleFile-master/` — study page serialization
- **Web scraping agent:** AI-driven web interaction
- **Source:** `APP/Accelworld/services/other3/browser-main/` — study browser agent patterns
- **Source:** `APP/Accelworld/services/other/SWE-agent-main/` — study code agent patterns
- **Typesetting system:** Document compilation with layout engine
- **Source:** `APP/Accelworld/services/other5/typst-main/` — study incremental compilation, layout engine
- **Observability platform:** Tracing, metrics, logging
- **Source:** `APP/Accelworld/services/other5/signoz-main/` — study OpenTelemetry integration
- **Screen recording:** Video capture with sharing
- **Source:** `APP/Accelworld/services/other5/cap-main/` — study screen capture pipeline
- **File management:** Cross-platform virtual filesystem
- **Source:** `APP/Accelworld/services/other5/spacedrive-main/` — study VDFS (virtual distributed filesystem)
- **Survey/form builder:** Question composition, response collection
- **Source:** `APP/Accelworld/services/other5/formbricks-main/` — study form composition model
- **Email server:** Full email stack
- **Source:** `APP/Accelworld/services/other5/stalwart-main/` — study SMTP/IMAP/JMAP patterns
- **Password management:** Secure credential storage
- **Source:** `APP/Accelworld/services/other5/vaultwarden-main/` — study encryption patterns
- **Nom mapping:** Binary patterns as nom-media codec; typesetting via nom-app; observability built into canvas

#### Agent Framework & Orchestration (other6)
- **Agent builder platform:** Visual agent workflow creation
- **Source:** `APP/Accelworld/services/other6/Archon-dev/` — study Plan→Implement→Validate→Review→Create pipeline
- **Temporal scheduling:** Time-based task orchestration
- **Source:** `APP/Accelworld/services/other6/Kronos-master/` — study scheduling patterns
- **Knowledge graph construction:** Automated graph building from documents
- **Source:** `APP/Accelworld/services/other6/graphify-3/` — study document→graph pipeline
- **PDF data extraction:** Structured data from unstructured documents
- **Source:** `APP/Accelworld/services/other6/opendataloader-pdf-main/` — study extraction pipeline
- **Voice-driven interaction:** Conversational AI model
- **Source:** `APP/Accelworld/services/other6/VoxCPM-main/` — study voice conversation model
- **Nom mapping:** Agent builder as canvas workflow; knowledge graph enriches nom-dict; voice as input mode

### 6.2 Patterns from Upstreams (228 repos — consolidating)

#### Compiler & Language Infrastructure
- **Declarative config language with gradual typing:** Composable records with design-by-contract validation
- **Source:** `APP/Accelworld/upstreams/nickel-lang/` — study contract system, merge semantics, type inference
- **Source:** `APP/Accelworld/upstreams/nickel-rs/` + `nickel-rs-actual/` + `nickel-rs-real/` — study Rust implementation
- **Source:** `APP/Accelworld/upstreams/rnix-parser/` — study Nix expression parser architecture
- **Structured shell (shell as typed data):** Commands produce tables not text; pipelines over structured streams
- **Source:** `APP/Accelworld/upstreams/nushell/` — study `crates/nu-command/`, `crates/nu-engine/`, `crates/nu-parser/`
- **LLVM compiler infrastructure:** IR generation, optimization passes, target codegen
- **Source:** `APP/Accelworld/upstreams/llvm-project/` — study `llvm/lib/IR/`, `llvm/lib/CodeGen/`, `clang/lib/`
- **Nom mapping:** Already leveraged via nom-llvm; structured shell pattern informs canvas pipeline composition

#### Cross-Platform UI & Rendering
- **Signals-based reactive UI:** Component-driven with per-framework compilation targets
- **Source:** `APP/Accelworld/upstreams/dioxus/` + `dioxus-main/` — study `packages/signals/`, `packages/core/`, `packages/html/`
- **GPU-accelerated 2D rendering:** High-performance vector graphics on GPU
- **Source:** `APP/Accelworld/upstreams/vello/` — study GPU scene encoding, compute shaders for path rendering
- **Source:** `APP/Accelworld/upstreams/wgpu/` — study WebGPU abstraction layer
- **Hardware-accelerated animation:** GPU-driven transform trees with easing functions
- **Source:** `APP/Accelworld/upstreams/motion-main/` — study animation engine, layout transitions
- **Desktop compositor:** Window management, input handling, display server
- **Source:** `APP/Accelworld/upstreams/smithay/` — study Wayland compositor patterns
- **Source:** `APP/Accelworld/upstreams/cosmic-comp/` + `cosmic-desktop/` + `libcosmic/` — study desktop shell architecture
- **Terminal emulator:** GPU-accelerated text rendering with shell integration
- **Source:** `APP/Accelworld/upstreams/alacritty/` — study OpenGL text rendering, shell integration
- **UI component library:** Design system with composition hierarchy
- **Source:** `APP/Accelworld/upstreams/fluentui/` — study component composition, theme system
- **Source:** `APP/Accelworld/upstreams/iced/` — study Elm-architecture UI in Rust
- **Source:** `APP/Accelworld/upstreams/gtk/` + `glib/` — study widget system, signal/slot pattern
- **Nom mapping:** GPU rendering for canvas performance (v2+); reactive signals for block state; compositor patterns for canvas window management

#### Editor & Code Intelligence
- **Modal text editor:** Tree-sitter parsing, LSP integration, GPU rendering
- **Source:** `APP/Accelworld/upstreams/helix/` — study `helix-term/`, `helix-lsp/`, `helix-core/`
- **Source:** `APP/Accelworld/upstreams/lapce/` — study `lapce-app/`, `lapce-proxy/`, `lapce-rpc/`
- **Full-text search engine:** Inverted index with BM25 scoring, faceted search
- **Source:** `APP/Accelworld/upstreams/tantivy/` — study index structure, scoring, query parser
- **Source:** `APP/Accelworld/upstreams/meilisearch/` — study typo-tolerant search, ranking rules
- **Spreadsheet engine:** Cell-based computation with formulas
- **Source:** `APP/Accelworld/upstreams/luckysheet/` — study formula engine, cell dependency graph
- **Nom mapping:** Editor patterns for nomx block text editing; search for canvas-wide entity lookup; spreadsheet for data-view blocks

#### Async Runtime & Networking
- **High-performance async runtime:** Task scheduling, I/O polling, timer management
- **Source:** `APP/Accelworld/upstreams/tokio/` — study `tokio/src/runtime/`, `tokio/src/net/`, io_uring integration
- **Source:** `APP/Accelworld/upstreams/tokio-uring/` — study io_uring async patterns
- **HTTP proxy/load balancer:** Connection pooling, request routing, TLS termination
- **Source:** `APP/Accelworld/upstreams/pingora/` — study proxy architecture, connection management
- **Source:** `APP/Accelworld/upstreams/linkerd2-proxy/` — study service mesh proxy patterns
- **gRPC framework:** Service definition, streaming, interceptors
- **Source:** `APP/Accelworld/upstreams/tonic/` — study service abstraction, streaming patterns
- **Middleware composition:** Layered request processing pipeline
- **Source:** `APP/Accelworld/upstreams/tower/` — study `Service` trait, layer composition, timeout/retry/rate-limit
- **Message queue:** Pub/sub, request-reply, persistent streams
- **Source:** `APP/Accelworld/upstreams/nats-rs/` + `nats-server/` — study messaging patterns
- **Nom mapping:** Async runtime for canvas backend; middleware for Tauri command pipeline; messaging for collaboration

#### Database & Storage
- **Embedded SQL database:** Pure Rust SQL engine
- **Source:** `APP/Accelworld/upstreams/gluesql/` — study SQL parser, execution engine, storage abstraction
- **Time-series database:** Columnar storage with SQL + PromQL
- **Source:** `APP/Accelworld/upstreams/greptimedb/` — study unified metrics model, query duality
- **Vector database:** HNSW index, filtering, quantization
- **Source:** `APP/Accelworld/upstreams/qdrant/` — study vector indexing, payload filtering
- **Multi-model database:** Document + graph + KV in one engine
- **Source:** `APP/Accelworld/upstreams/surrealdb/` — study multi-model query language
- **Universal storage access:** Abstraction over cloud storage backends
- **Source:** `APP/Accelworld/upstreams/opendal/` — study `core/src/services/`, multi-backend pattern
- **Columnar data processing:** SIMD-optimized, lazy evaluation, streaming
- **Source:** `APP/Accelworld/upstreams/polars/` — study query optimization, expression system
- **Nom mapping:** nom-dict already uses SQLite; vector DB patterns for GAP-2 embedding resolver; columnar for batch processing

#### AI & ML Inference
- **Tensor computation framework:** GPU inference, model loading, quantization
- **Source:** `APP/Accelworld/upstreams/candle/` — study tensor ops, CUDA/Metal backends, model loading
- **Source:** `APP/Accelworld/upstreams/onnxruntime/` — study cross-platform inference, optimization
- **Source:** `APP/Accelworld/upstreams/mlc-llm/` — study compilation-based LLM optimization
- **Source:** `APP/Accelworld/upstreams/tensorrt-llm/` — study high-performance LLM serving
- **LLM serving:** Distributed inference with paged attention
- **Source:** `APP/Accelworld/upstreams/vllm/` — study paged attention, continuous batching
- **Source:** `APP/Accelworld/upstreams/ollama/` — study local model management, API abstraction
- **Tokenizer library:** Fast BPE/WordPiece tokenization
- **Source:** `APP/Accelworld/upstreams/tokenizers/` — study tokenization algorithms, vocabulary management
- **ML model hub:** Model storage, versioning, safe serialization
- **Source:** `APP/Accelworld/upstreams/safetensors/` — study safe tensor serialization format
- **Transformer architecture:** Attention mechanisms, model definitions
- **Source:** `APP/Accelworld/upstreams/transformers/` — study model architecture patterns
- **Source:** `APP/Accelworld/upstreams/gemma/` + `qwen3/` + `mistral/` + `deepseek/` — study model-specific optimizations
- **Nom mapping:** Local inference for v4 intelligence (dream mode, output prediction); tokenizer patterns for nomtu matching

#### Multi-Agent & Orchestration
- **Agent framework:** Multi-agent communication, tool use, planning
- **Source:** `APP/Accelworld/upstreams/autogen/` — study agent conversation patterns, tool registration
- **Source:** `APP/Accelworld/upstreams/deepagents-main/` — study deep agent coordination
- **Source:** `APP/Accelworld/upstreams/agentcore-samples/` — study agent core abstractions
- **LLM application framework:** RAG, agents, query engines
- **Source:** `APP/Accelworld/upstreams/llamaindex/` — study index types, agent workflows, retrieval strategies
- **Source:** `APP/Accelworld/upstreams/langchain/` + `langchain-master/` — study chain composition, tool abstraction
- **Source:** `APP/Accelworld/upstreams/haystack/` — study pipeline composition, component protocol
- **Agent coding assistant:** Code generation, file editing, task planning
- **Source:** `APP/Accelworld/upstreams/claw-code-main/` — study task delegation, recovery loops, notification separation
- **Source:** `APP/Accelworld/upstreams/everything-claude-code-main/` — study 47 agents + 156 skills + homunculus memory
- **Source:** `APP/Accelworld/upstreams/bolt.new-main/` — study streaming LLM→executable, action runner
- **Nom mapping:** Agent patterns for v4 dream mode; coding assistant patterns for canvas auto-fix loop

#### Media & Audio/Video
- **Multimedia framework:** Codec support, transcoding, streaming
- **Source:** `APP/Accelworld/upstreams/ffmpeg/` — study libavcodec, libavformat, filter graph
- **Source:** `APP/Accelworld/upstreams/pipewire/` — study audio/video routing, graph-based processing
- **Image/video diffusion:** Text-to-image, image-to-video generation
- **Source:** `APP/Accelworld/upstreams/bytedance-flux/` — study diffusion pipeline architecture
- **Computer vision:** Object detection, tracking, annotation
- **Source:** `APP/Accelworld/upstreams/supervision-develop/` — study detection/tracking abstractions
- **Nom mapping:** nom-media codec support; canvas media preview; video generation pipeline

#### System & OS
- **Operating system kernel:** Process management, memory management, syscalls
- **Source:** `APP/Accelworld/upstreams/asterinas/` — study Rust OS kernel patterns
- **Source:** `APP/Accelworld/upstreams/theseus/` — study modular OS with live evolution
- **Source:** `APP/Accelworld/upstreams/redox/` — study microkernel Rust OS
- **Container runtime:** OCI-compliant container execution
- **Source:** `APP/Accelworld/upstreams/youki/` — study container lifecycle management
- **Source:** `APP/Accelworld/upstreams/wasmer/` + `wasmtime/` — study WASM runtime, JIT compilation
- **Embedded systems:** Bare-metal async embedded
- **Source:** `APP/Accelworld/upstreams/embassy/` — study async embedded Rust patterns
- **Source:** `APP/Accelworld/upstreams/tock/` — study embedded OS for microcontrollers
- **Source:** `APP/Accelworld/upstreams/hubris/` — study deterministic embedded OS
- **Package management:** Dependency resolution, version management
- **Source:** `APP/Accelworld/upstreams/nix/` + `nix-master/` + `nixpkgs/` — study reproducible builds, declarative packages
- **Source:** `APP/Accelworld/upstreams/winget/` — study Windows package management
- **Nom mapping:** v5 system targets — OS kernel patterns for embedded; WASM for plugin system; package management for nomtu distribution

#### Cryptography & Security
- **TLS implementation:** Certificate management, handshake, record layer
- **Source:** `APP/Accelworld/upstreams/rustls/` — study TLS state machine, certificate verification
- **Cryptographic primitives:** Hash functions, signing, encryption
- **Source:** `APP/Accelworld/upstreams/rustcrypto-hashes/` — study hash algorithm implementations
- **Source:** `APP/Accelworld/upstreams/cosign/` — study container signing, verification
- **Security scanning:** Vulnerability detection, SBOM analysis
- **Source:** `APP/Accelworld/upstreams/trivy/` — study multi-target vulnerability scanning
- **Source:** `APP/Accelworld/upstreams/semgrep/` — study pattern-based static analysis
- **Source:** `APP/Accelworld/upstreams/osv-scanner/` — study vulnerability database querying
- **Source:** `APP/Accelworld/upstreams/yara-x/` — study pattern matching rules engine
- **Source:** `APP/Accelworld/upstreams/nuclei/` — study template-based security scanning
- **Nom mapping:** Content-addressed hashing already in nom-dict; security scanning for nom-security; pattern matching for grammar rules

#### CLI & Developer Tools
- **Modern CLI patterns:** Colored output, fuzzy finding, shell integration
- **Source:** `APP/Accelworld/upstreams/bat/` — study syntax highlighting for terminal
- **Source:** `APP/Accelworld/upstreams/fd/` — study parallel file finding
- **Source:** `APP/Accelworld/upstreams/ripgrep/` — study high-performance text search
- **Source:** `APP/Accelworld/upstreams/starship/` — study cross-shell prompt customization
- **Source:** `APP/Accelworld/upstreams/zoxide/` — study frecency-based directory jumping
- **Source:** `APP/Accelworld/upstreams/atuin/` — study SQLite-backed shell history with sync
- **Source:** `APP/Accelworld/upstreams/git-cliff/` — study conventional commit parsing, changelog generation
- **Git implementation:** Object storage, pack protocol, merge algorithms
- **Source:** `APP/Accelworld/upstreams/gitoxide/` — study pure Rust git implementation
- **Web scraping/download:** Content extraction, format conversion
- **Source:** `APP/Accelworld/upstreams/yt-dlp/` — study extractor plugin architecture, format selection
- **Source:** `APP/Accelworld/upstreams/scraper/` — study CSS selector-based HTML parsing
- **Source:** `APP/Accelworld/upstreams/spider/` — study concurrent web crawling
- **Nom mapping:** CLI patterns already in nom-cli; git for canvas version control; web scraping for corpus ingestion

#### Consensus & Distributed Systems
- **Raft consensus:** Leader election, log replication, state machine
- **Source:** `APP/Accelworld/upstreams/raft-rs/` — study Raft protocol implementation
- **Source:** `APP/Accelworld/upstreams/corro/` — study CRDT-based distributed state
- **Source:** `APP/Accelworld/upstreams/conduit/` — study Matrix federation protocol
- **Parallel computation:** Work-stealing, data parallelism
- **Source:** `APP/Accelworld/upstreams/rayon/` — study work-stealing scheduler, parallel iterators
- **WebAssembly runtime:** Module compilation, sandboxed execution, WASI
- **Source:** `APP/Accelworld/upstreams/wasmer/` — study compiler backends, module caching
- **Source:** `APP/Accelworld/upstreams/wasmtime/` — study Cranelift JIT, component model
- **Source:** `APP/Accelworld/upstreams/spin/` — study WASM microservice composition
- **Nom mapping:** CRDT for canvas collaboration; parallel computation for batch compilation; WASM for canvas plugin system

---

## 7. Identity Rule

**HARD REQUIREMENT:** Zero foreign repo identities in the Nom codebase.

- All patterns are described abstractly using Nom-native vocabulary
- No repo names, brand names, or project names appear in source, comments, docs, or CLI output
- The `word` field of any nomtu entry contains ONLY Nom descriptive names
- Provenance (where a pattern was studied from) lives ONLY in this spec document and memory, never in the codebase
- When implementing a pattern, the implementation is native Nom — not a wrapper, not an adapter, not a bridge

---

## 8. Relation to Existing Nom Crates

NomCanvas does NOT replace any existing crate. It is a new workspace (`nom-canvas/`) that DEPENDS on them:

| Existing Crate | Role in NomCanvas |
|---------------|-------------------|
| nom-dict | Tauri command: lookup_nomtu queries entities.sqlite |
| nom-grammar | WASM bridge: NomtuMatcher uses Jaccard scorer; Tauri command: match_grammar |
| nom-concept | Tauri command: compile_block runs S1-S6 pipeline |
| nom-llvm | Tauri command: build_artifact compiles to native |
| nom-lsp | Tauri command: hover_info, diagnostics routed to canvas |
| nom-resolver | Tauri command: complete_word uses resolver scoring |
| nom-intent | v4: dream mode uses ReAct loop |
| nom-app | v2: app builder uses app manifest + build |
| nom-media | v3: media pipeline uses media render |
| nom-ux | v2: app builder uses UX patterns |
| nom-planner | Compilation: plan_from_pipeline_output for block compilation |
| nom-score | Quality scoring displayed in block decorations |

---

## 9. Open Questions

1. **Canvas persistence format:** Should canvas state be stored as .nom concept, SQLite, or JSON? Recommendation: SQLite (consistent with nom-dict pattern).
2. **Collaboration model:** When does multi-user canvas editing ship? Recommendation: Not v1 — add CRDT layer in v2+.
3. **Plugin system:** Should block types be extensible via WASM plugins? Recommendation: Yes, in v2 — mirrors the WASM extension pattern.
4. **Offline-first:** Should NomCanvas work fully offline? Recommendation: Yes — Tauri + local SQLite makes this natural.
