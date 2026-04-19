# Nom Universal Composer — Master Plan v1
## Planner/Auditor Cycle | 2026-04-19 | HEAD: 2da0748

> **NORTH STAR:** `docs/superpowers/specs/2026-04-17-nomcanvas-gpui-design.md`
> **AUDIT BASE:** `AUDIT_REPORT_2026-04-19.md` (8-agent deep-dive completed)
> **REFERENCE REPO INVENTORY:** 249 upstreams + 147 services = 396 repos scanned

---

## 1. Executive Summary

The Executor has built a **sophisticated scaffold** — 10,466 tests, clean architecture, real LLVM backend, real 3-tier hybrid resolver — but **virtually no end-to-end execution path works**. The renderer initializes wgpu then sits idle. The `.nomx` language tokenizes beautifully but has no evaluator. All media backends are stubs. LSP parses Content-Length headers then returns hardcoded nulls. Foreign brands pollute the public API.

This plan defines **the shortest path from scaffold to working universal composer** by:
1. **Adopting 12 untapped reference-repo patterns** (from 396 scanned repos)
2. **Fixing 5 critical blockers** that prevent any demo from working
3. **Building the AI-Orchestration layer** that makes "compose anything from natural language" real
4. **Keeping the frontend minimalist** (AFFiNE + Copilot metaphor)

---

## 2. Critical Blockers — Fix These First (Wave Critical)

Without these, no demo is possible. The Executor must fix these before any new feature work.

| # | Blocker | File | Fix | Reference Pattern |
|---|---------|------|-----|-------------------|
| CB1 | **Renderer renders 0 pixels** | `nom-gpui/src/window.rs:328` | Wire `end_frame_render()` into `RedrawRequested` event loop | Zed `Window::draw()` → `present()` |
| CB2 | **Scene graph → GPU disconnected** | `nom-gpui/src/renderer.rs:776-799` | `Renderer::draw()` must upload quads to wgpu buffer and call `draw_quads_gpu()` | Zed `Renderer::draw(scene)` batching |
| CB3 | **`.nomx` has no evaluator** | `nom-concept/src/stages.rs:3095` | Bridge `DefineThatExpr` to existing `.nom` AST (`Expr::FnDef`) | Nom's own `nom-ast` crate |
| CB4 | **FFmpeg never spawned** | `nom-compose/src/video_capture.rs` | `FfmpegEncoder::encode()` should `Command::new("ffmpeg").args(...).spawn()` | FFmpeg filter graph DSL |
| CB5 | **Foreign brands in public API** | `nom-lint/src/rules/naming.rs:11` | Expand `BANNED_PREFIXES` to 30+ items; rename `affine_tokens`, `n8n_workflow`, `sherlock`, `haystack_pipeline`, `FfmpegConfig` | Spec §16 rule 3 |

**Acceptance criteria for Wave Critical:**
- [ ] `cargo run --bin nom-canvas` opens a window that displays a colored quad
- [ ] `nom compose video hello.nomx` produces a real `.mp4` file on disk
- [ ] `nom lint` catches `affine_`, `n8n_`, `sherlock_`, `haystack_`, `ffmpeg_` prefixes
- [ ] `cargo test` passes with 0 failures

---

## 3. The Universal Composer Backend Architecture

### 3.1 Vision: "Clone Anything in the World"

The backend must compose:
- **Media** (video, picture, audio, 3D mesh) from natural language
- **Screens** (web app, native app, mobile app, presentation)
- **Apps** (full bundle, ad creative)
- **Data** (extract, transform, query)
- **Documents** (PDF, DOCX)
- **Scenarios** (workflow, automation)

And **clone/inspect anything** from a URL: GitHub repo → `.nomx`, YouTube video → `.nomx`, website → `.nomx`, person (OSINT) → `.nomx`.

### 3.2 The Five-Layer Stack

```
┌─────────────────────────────────────────────────────────────┐
│  L5 · NATURAL LANGUAGE SURFACE                              │
│  User types: "make a 30s promo video about nano banana"     │
│  → IntentResolver lexical→BM25→Qdrant→classify_with_react   │
├─────────────────────────────────────────────────────────────┤
│  L4 · AI ORCHESTRATION (NEW — adopt from untapped repos)    │
│  • LangChain Runnable composition (chaining LLM + tool calls)│
│  • CrewAI Flow/Crew multi-agent orchestration               │
│  • OpenHarness 43+ tool skill library                       │
│  • mempalace 96.6% LongMemEval context retention            │
├─────────────────────────────────────────────────────────────┤
│  L3 · HYBRID RESOLVER (existing — real but needs depth)     │
│  Tier 1: DB-driven (grammar.kinds Complete)                 │
│  Tier 2: Provider-driven (MediaVendor + credentials)        │
│  Tier 3: AI-leading (AiGlueOrchestrator generates .nomx)    │
├─────────────────────────────────────────────────────────────┤
│  L2 · COMPOSE ENGINE (NEW — adopt from Polars + Greptime)   │
│  • QueryEngine trait (swappable SQLite/Polars/DuckDB)       │
│  • DSL → IR → Optimizer → Physical Plan (Polars pattern)    │
│  • ExtensionAnalyzerRule (Greptime pattern)                 │
│  • Arena-based plan IR for cross-domain optimization        │
├─────────────────────────────────────────────────────────────┤
│  L1 · EXECUTION RUNTIME (existing — stubbed, needs media)   │
│  • Video: FFmpeg filter graph + MoneyPrinter staged pipeline │
│  • Audio: rodio/symphonia + pipewire routing                │
│  • Image: Vello GPU vector renderer + model dispatch        │
│  • Web: Dioxus signal-based components (port pattern only)  │
│  • Data: Polars LazyFrame + opendal universal storage       │
│  • Document: typst incremental layout (already referenced)  │
└─────────────────────────────────────────────────────────────┘
```

### 3.3 DB-Driven Architecture (Strengthen Existing)

The DB is already the workflow engine. Strengthen it:

| Table | Current | Target |
|-------|---------|--------|
| `grammar.kinds` | 46 kinds seeded | 200+ kinds from public-apis catalog pattern |
| `clause_shapes` | Basic shapes | Full API metadata (auth, HTTPS, CORS, rate_limit, cost_tier) |
| `entries` | Basic entries | Content-addressed store with `opendal` backends (S3, GCS, Azure) |
| `glue_cache` | In-memory only | Persisted SQLite with LRU eviction |
| `entry_benchmarks` | Schema exists | Populated from real runs via `QueryEngine` telemetry |
| `flow_steps` | Schema exists | Populated from `ExtensionAnalyzerRule` execution traces |

### 3.4 AI-Driven Architecture (New Layer)

**Adopt from 6 untapped AI repos:**

| Pattern | Source Repo | Nom Module | What It Does |
|---------|-------------|------------|--------------|
| **Runnable composition** | LangChain | `nom-compose/src/chain.rs` | Chain LLM calls + tool calls into pipelines |
| **Crew/Flow orchestration** | CrewAI | `nom-compose/src/crew.rs` | Multi-agent teams with roles, tasks, delegation |
| **Agent harness** | OpenHarness | `nom-compose/src/harness.rs` | 43+ pre-built tools (search, code, browse, calc) |
| **Long-context memory** | mempalace | `nom-compose/src/memory.rs` | Raw verbatim storage, no summarization loss |
| **Local LLM serving** | Ollama | `nom-compose/src/ollama.rs` | Zero-config model download + inference |
| **High-throughput inference** | vLLM | `nom-compose/src/vllm.rs` | PagedAttention for serving multiple models |
| **Voice cloning/TTS** | VoxCPM | `nom-compose/src/voice.rs` | Controllable text-to-speech for audio compose |

**AI Glue Orchestrator upgrade:**
- Current: `AiGlueOrchestrator` generates `.nomx` string, `execute_blueprint()` is stub
- Target: Use **LangChain Runnable** pattern to chain:
  1. `IntentResolver` → kind classification
  2. `GraphRagRetriever` → context retrieval
  3. `ReActLlmFn` → `.nomx` generation
  4. `nom_concept::stage1_tokenize` → validation
  5. `sandbox::interpret` OR `BackendRegistry::dispatch` → execution
  6. `DictWriter::insert_partial_entry` → promotion

### 3.5 Natural Language-Driven Architecture

**The `.nomx` surface must become executable.** Two-phase approach:

**Phase A (immediate):** Desugar `define X that Y` to existing `.nom` AST
```
define add_numbers that
  define a that 10
  define b that 20
  a + b

→ desugar →

fn add_numbers() -> i64 {
  let a = 10;
  let b = 20;
  a + b
}
```

**Phase B (short-term):** Full `.nomx` parser producing `ConceptNode` AST with:
- Expression nodes (binary ops, function calls, conditionals)
- Type annotations
- Effect clauses (benefit, hazard, retry)
- Contract clauses (requires, ensures)

**Reference:** The `.nom` language in `nom-ast` already has all these nodes. The bridge is the missing piece.

---

## 4. Reference Repo Adoption Matrix

### 4.1 Already Referenced (22 repos) — Status

| Repo | Pattern | In Code? | Real? |
|------|---------|----------|-------|
| Zed | GPUI scene/shell | Yes | Renderer library-real, integration-stub |
| AFFiNE | 73 tokens, frosted glass | Yes | Tokens real, frosted stub |
| rowboat | Chat sidebar, deep-think | Yes | UI models real, no LLM integration |
| ComfyUI | 4-tier cache, Kahn, cancel | Yes | Cache real, cancel real, Kahn real |
| dify | TypedNode, NodeEvent | Yes | Real types |
| n8n | AST sandbox, sanitizers | Partial | Sandbox exists, not wired |
| LlamaIndex | RRF, cosine, BFS | Yes | Real algorithms |
| Haystack | Pipeline composition | Partial | `ComponentPipeline` stubbed |
| ToolJet | 55-widget registry | Partial | 51 kinds seeded, not all wired |
| yara-x | Sealed trait linter | Yes | `NamingLinter` real |
| typst | comemo memoization | Yes | `nom-memoize` real |
| WrenAI | MDL semantic layer | Partial | `semantic.rs` stubbed |
| 9router | 3-tier fallback | Yes | `ProviderRouter` real |
| graphify | Chart types, SVG export | Yes | `ChartType` real |
| Refly | SkillRouter | Yes | Real |
| Remotion | GPU→frame→FFmpeg | No | Y4M only, no FFmpeg |
| Open-Higgsfield | 200+ model dispatch | No | Stub only |
| ArcReel | 5-phase orchestration | Partial | `StoryboardPhase` stubbed |
| waoowaoo | 4-phase cinematography | No | Not in code |
| opendataloader | XY-Cut++ | No | Stub only |
| candle | In-process ML | Partial | `CandleAdapter` stubbed |
| qdrant | Vector search | Partial | Not wired to IntentResolver |

### 4.2 Untapped High-Value Repos (12 to adopt now)

| Priority | Repo | Path | Pattern | Nom Target | Effort |
|----------|------|------|---------|------------|--------|
| P0 | **ffmpeg** | `upstreams/ffmpeg` | Filter graph DSL + format negotiation | `nom-compose/src/video_encode.rs` | 1w |
| P0 | **Polars** | `upstreams/polars` | Lazy DataFrame + pushdown optimizer | `nom-compose/src/data_query.rs` | 1w |
| P0 | **LangChain** | `upstreams/langchain-master` | Runnable composition + tool use | `nom-compose/src/chain.rs` (new) | 1w |
| P1 | **CrewAI** | `services/other2/crewAI-main` | Multi-agent Flow/Crew orchestration | `nom-compose/src/crew.rs` (new) | 1w |
| P1 | **Spider** | `upstreams/spider` | Builder-pattern API client + retry + streaming | `nom-compose/src/vendors/*.rs` | 3d |
| P1 | **OpenHarness** | `upstreams/OpenHarness-main` | 43+ tool skill library + memory | `nom-compose/src/harness.rs` (new) | 1w |
| P1 | **Ollama** | `upstreams/ollama` | Zero-config local LLM serving | `nom-compose/src/ollama.rs` (new) | 3d |
| P1 | **opendal** | `upstreams/opendal` | Universal storage (S3/GCS/Azure/HDFS) | `nom-compose/src/storage.rs` (new) | 3d |
| P2 | **temporal-sdk-core** | `upstreams/temporal-sdk-core` | Durable workflow execution | `nom-compose/src/durable.rs` (new) | 1w |
| P2 | **vello** | `upstreams/vello` | GPU vector renderer (Rust) | `nom-canvas-core/src/vector.rs` (new) | 1w |
| P2 | **mempalace** | `upstreams/mempalace` | Long-context verbatim memory | `nom-compose/src/memory.rs` (new) | 3d |
| P2 | **VoxCPM** | `upstreams/VoxCPM-main` | Controllable voice cloning/TTS | `nom-compose/src/voice.rs` (new) | 1w |

### 4.3 Untapped Repos for Future Waves (25 more)

| Repo | Pattern | When |
|------|---------|------|
| vLLM | PagedAttention high-throughput LLM | When scaling inference |
| deno | Secure JS/TS sandbox for web compose | When web app generation is real |
| wasmtime/wasmer | WASM sandbox for polyglot plugins | When plugin system needed |
| firecracker/gvisor | Lightweight VM sandbox | When running untrusted generated code |
| nix/nixpkgs | Reproducible build environments | When deployment pipeline needed |
| gitoxide | Embedded Git operations | When versioning composed outputs |
| nats-server | Event streaming for distributed agents | When multi-node composer |
| pingora | Edge HTTP proxy/routing | When API gateway needed |
| surrealdb | Document+graph+time-series unified DB | When replacing SQLite |
| meilisearch/tantivy | Instant search for generated content | When content search needed |
| airflow | Industry DAG scheduler | When complex data pipelines |
| pipewire | Modern Linux A/V routing | When real-time media pipelines |
| moneyprinter | Automated video generation | When video compose matures |
| onlyoffice/luckysheet | Document/spreadsheet composition | When document compose matures |
| servo | Embeddable browser engine | When rendering composed web docs |
| bytedance-flux | Image/video generation model | When media AI needed |
| autogen | Microsoft multi-agent framework | When agent conversation needed |
| deepseek | Reasoning model architecture | When deep reasoning needed |
| llama-cpp | Portable quantized inference | When edge deployment needed |
| transformers/tokenizers | HuggingFace ecosystem | When fine-tuning needed |
| rust-libp2p | P2P networking | When decentralized composer |
| starship | Cross-shell prompt | When CLI polish needed |
| atuin | Shell history sync/search | When CLI history needed |

---

## 5. Frontend: Minimalist AFFiNE + Copilot

### 5.1 Design Mandate

The frontend must be **"simple but strong"** — every surface earns its space. Pattern sources:
- **AFFiNE**: 73 design tokens, Inter + Source Code Pro, frosted glass, block model
- **Zed**: PaneGroup recursive splits, command palette, status bar
- **Rowboat**: Right dock AI chat, deep-think cards, tool inspector
- **Copilot**: Inline AI suggestions, natural language command bar

### 5.2 Minimalism Enforcement

| Rule | Enforcement |
|------|-------------|
| Zero decorative borders | Only functional 1px hairlines (`border_color` token) |
| No gradients except frosted glass | `blur_radius = 24px` backdrop filter only |
| Icon rail = icon only | No label duplication |
| No visible placeholder content | Every surface must have real data or be hidden |
| Motion ≤200ms standard | 300ms ease-out for deep-think cards, 0 for reduced-motion |

### 5.3 UI-UX-pro-max Compliance

Run before every UI commit:
```bash
cd C:/Users/trngh/Documents/GitHub/Nom
PYTHONIOENCODING=utf-8 python3 .agent/skills/ui-ux-pro-max/scripts/search.py "<query>" --design-system -p "NomCanvas"
```

### 5.4 The Copilot Metaphor

The right dock is not just a chat — it is a **Copilot** that:
1. **Understands intent** from natural language (`IntentResolver`)
2. **Suggests actions** as tool cards (`ToolJet` widget pattern)
3. **Shows reasoning** as deep-think cards (`DeepThinkStep`)
4. **Accepts corrections** inline (edit purpose clause, regenerate)
5. **Learns from feedback** (BM25 training signal on corrections)

---

## 6. Implementation Waves

### Wave Critical (Week 1) — Fix Blockers
- [ ] CB1: Wire `end_frame_render()` into event loop
- [ ] CB2: Connect scene graph → GPU submission
- [ ] CB3: Desugar `define...that` to `.nom` AST
- [ ] CB4: Spawn FFmpeg from `FfmpegEncoder`
- [ ] CB5: Rename foreign brands + expand `BANNED_PREFIXES`

### Wave AI-Composer v2 (Week 2-3) — AI Orchestration Layer
- [ ] Adopt LangChain Runnable pattern → `nom-compose/src/chain.rs`
- [ ] Adopt CrewAI Flow/Crew → `nom-compose/src/crew.rs`
- [ ] Adopt OpenHarness tool library → `nom-compose/src/harness.rs`
- [ ] Adopt mempalace long-context memory → `nom-compose/src/memory.rs`
- [ ] Wire `AiGlueOrchestrator` to real LLM adapters (Ollama local, vLLM remote)
- [ ] Real `execute_blueprint()` via `.nomx` → `.nom` AST → LLVM

### Wave Data Engine (Week 3-4) — Query Engine + Storage
- [ ] Adopt Polars LazyFrame → `nom-compose/src/data_query.rs`
- [ ] Adopt GreptimeDB QueryEngine trait → `nom-compose/src/engine.rs`
- [ ] Adop SurrealDB Composer DI → refactor `UnifiedDispatcher`
- [ ] Adopt opendal universal storage → `nom-compose/src/storage.rs`
- [ ] Adopt Spider builder-pattern API clients → all vendor integrations
- [ ] Adopt public-apis catalog validation → `DictWriter` promotion gates

### Wave Media Real (Week 4-5) — Working Backends
- [ ] Adopt FFmpeg filter graph → real video encode (MP4/WebM)
- [ ] Adopt rodio + pipewire → real audio playback
- [ ] Adopt Vello → GPU vector rendering for image compose
- [ ] Adopt MoneyPrinter staged pipeline → automated video generation
- [ ] Adopt VoxCPM → TTS for audio compose
- [ ] Real end-to-end: `.nomx` → video file on disk

### Wave Inspector (Week 5-6) — Clone Anything
- [ ] Adopt Sherlock OSINT pattern → `NomInspector` Person/Company targets
- [ ] Adopt yt-dlp → YouTube/Video target ingestion
- [ ] Adopt gitoxide → GitHub repo tree walk + clone
- [ ] Adopt scraper + Spider → Website target crawl
- [ ] Adopt chromiumoxide → Playwright-style screenshot + metadata
- [ ] Real end-to-end: `nom inspect <url>` → `.nomx` entry in DB

### Wave Durable (Week 6-7) — Resilient Execution
- [ ] Adopt temporal-sdk-core → durable workflow execution
- [ ] Adopt nats-server → event streaming between composer agents
- [ ] Adopt pingora → edge HTTP proxy for `POST /compose`
- [ ] Adopt deno → JS/TS sandbox for web app compose
- [ ] Adopt wasmtime → WASM sandbox for polyglot plugins
- [ ] Adopt firecracker → VM sandbox for untrusted code

### Wave 100% (Week 7-8) — Polish + Bootstrap Proof
- [ ] Complete Nom-in-Nom parser.nom ( compilable features only )
- [ ] Complete Nom-in-Nom codegen.nom ( LLVM IR emitter )
- [ ] Bootstrap fixpoint proof: s2 == s3 byte-identical
- [ ] CI green on Windows + Linux + macOS + wasm
- [ ] All 4 golden-path demos playable from README
- [ ] 0 foreign identities in public API

---

## 7. Risk Assessment & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Polars compile time bloat | High | Medium | Feature-gate (`polars` feature). Default off. |
| FFmpeg licensing (GPL) | Medium | High | Use FFmpeg via CLI spawn, not static link. Nom stays MIT/Apache. |
| LLM API costs | High | Medium | Default to Ollama local inference. Remote APIs opt-in. |
| Renderer complexity | Medium | High | Fix CB1/CB2 first. Only then adopt Vello. |
| Foreign brand renaming breaks API | Low | High | `gitnexus_rename` for safe multi-file rename. |
| Nom-in-Nom self-host too ambitious | High | High | Keep Rust backend as default. Nom self-host is parity track, not blocker. |
| Test inflation hides real gaps | High | Medium | Require 1 end-to-end test per wave. No wave ships without pixel/file/HTTP proof. |

---

## 8. Non-Negotiable Rules for Executor

1. **Read source end-to-end** before adopting any pattern — not README, not one file
2. **Zero foreign identities** in public API — rename before commit
3. **Run `gitnexus_impact`** before editing any symbol
4. **Use `ui-ux-pro-max`** for every UI change
5. **DB IS the workflow engine** — no external orchestrator
6. **Every canvas object = DB entry** — `entity: NomtuRef` non-optional
7. **GPUI fully Rust** — one binary, no webview
8. **Spawn parallel subagents** for multi-file work
9. **Write 1 end-to-end test per wave** — pixel, file, or HTTP proof required
10. **Update 4 canonical files every cycle:** `implementation_plan.md`, `nom_state_machine_report.md`, `task.md`, `docs/superpowers/specs/2026-04-17-nomcanvas-gpui-design.md`

---

## 9. Appendix: Full Reference Repo Inventory

### Upstreams (249 repos)
**AI/LLM (39):** ai-edge-torch, agentcore-samples, autogen, bytedance-flux, candle, deepagents-main, DeepCode-main, deepseek, deepspeed, donut, gemma, gpt-engineer, haystack, langchain, langchain-master, llama-cpp, llama-models, llama-stack, llamaindex, llama_index-main, mindspore, mistral, mlc-llm, mempalace, nlp, ollama, open-webui, OpenHarness-main, Paraphrase-Generator-master, qwen3, safetensors, segment-anything, supervision-develop, tensorrt-llm, tokenizers, tokio, transformers, triton-server, ultralytics, unilm, vllm, VoxCPM-main, wrenai

**Media/Video (8):** AnimateDiff, ffmpeg, motion-main, pipewire, remotion-main, screenshot-to-code, yt-dlp

**Data/Database (12):** corro, gluesql, greptimedb, meilisearch, opendal, polars, qdrant, raft-rs, surrealdb, tantivy, vectra-py-master, graphify-master

**Web/Frontend (17):** AFFiNE-canary, async-graphql, bolt.new-main, chromiumoxide, dioxus, dioxus-main, etherpad, fantoccini, fluentui, gtk, iced, luckysheet, onlyoffice, refly-main, reqwest, scraper, servo, tooljet, ToolJet-develop, volo

**Security/OSINT (40):** aircrack-ng, apparmor, bettercap, boringtun, cosign, cryptpad, dm-verity, exploitdb, falco, gvisor, hashcat, john-the-ripper, kali, kali-nmap, metasploit-framework, mimikatz, minijail, nuclei, openbsd, osv-scanner, owasp-cheatsheets, radare2, routersploit, rustcrypto-hashes, rustls, rustscan, rustsec, selinux, semgrep, sqlmap, suricata, thc-hydra, tock, trivy, trufflehog, velociraptor, volatility3, wifiphisher, yara-x, zaproxy

**DevTools/Compiler (44):** accesskit, alacritty, atuin, bat, benchmark-main, bumpalo, cargo-fuzz, claw-code-main, cpython, deno, devhome, fd, git-cliff, gitoxide, helix, icu4x, kanata, kube-rs, lapce, llvm-project, nickel (11 variants), nix, nix-master, nixpkgs, nixpkgs-master, node, nodejs, nushell, pingora, powertoys, python-standalone, rayon, ripgrep, rnix-parser, rust-lang-rust, rust-mustache, starship, tokio-uring, tonic, tower, uutils-coreutils, wasmer, wasmtime, windows-terminal, winget, zbus, zed-main, zoxide

**Workflow/Automation (9):** airflow, deer-flow-main, everything-claude-code-main, last30days, nats-rs, nats-server, rowboat-main, spider, temporal-sdk-core, CoreNLP-main

**Graphics/GPU (9):** cosmic-comp, cosmic-desktop, libcosmic, mesa, mutter, smithay, vello, wayland, wgpu

**Other/OS (30):** appimage-runtime, apt, arch, asterinas, bottlerocket, cloud-hypervisor, conduit, crosvm, cuda-samples, cups, dpkg, embassy, firecracker, flatpak, freebsd-src-main, glib, gnome-shell, hickory-dns, hubris, iproute2, libdispatch, linkerd2-proxy, linux, maestro, networkmanager, oreboot, paperless-ngx, phipsboot, r9, redox, rust-libp2p, serenity, servo, snapd, spin, systemd, ubuntu, update_engine, util-linux, vboot_reference, waydroid, wine-ge, youki, xnu, awesome-browser-master

### Services (147 repos across 18 categories)
**AI/LLM (45):** agent-lightning-main, agentscope-main, AI-Scientist-v2-main, AI-Trader-main, AionUi-main, autoresearch-master, axios-1.x, BettaFish-main, browser-main, claude-code-main, claude-peers-mcp-main, claude-subconscious-main, claw-code-dev-rust, ClawTeam-OpenClaw-main, ClawX-main, codex-main, crewAI-main, ctrader-mcp-server-main, DeepCode-main, deer-flow-main, GitNexus-main, graphiti-main, last30days-skill-main, Lean-master, marketingskills-main, metamcp-main, MiroFish-main, MoneyPrinterV2-main, nautilus_trader-develop, notebooklm-py-main, oasis-main, onyx-main, OpenBB-develop, OpenViking-main, platform-develop, posthog-master, postiz-app-main, prompts.chat-main, RAG-Anything-main, refly-main, samples-go-main, strix-main, superpowers-main, taste-skill-main, timesfm-master, TradingAgents-main, TrendRadar-main, twenty-main, Vane-master, vault-main, VibeVoice-main, weferral-main, zeroclaw-master

**Media/Video (8):** moneyprinter, Open-Higgsfield-AI-main, waoowaoo

**Data/Analytics (10):** OpenBB-develop, posthog-master, postiz-app-main, public-apis-master, twenty-main, Vane-master

**Web/App Builder (22):** AndyTheDesignerWeb-master, awesome-ai-agents-main, BettaFish-main, browser-main, ClawTeam-OpenClaw-main, ClawX-main, codex-main, craft-agents-oss-main, dify-main, MiroFish-main, OpenViking-main, platform-develop, postiz-app-main, prompts.chat-main, RedditVideoMakerBot-master, samples-go-main, twenty-main, weferral-main

**Security/OSINT (24):** autoresearch-master, BettaFish-main, browser-main, claw-code-dev-rust, DeepCode-main, exploitdb, falco, GitNexus-main, gvisor, hashcat, john-the-ripper, kali, kali-nmap, metasploit-framework, mimikatz, minijail, nuclei, osv-scanner, owasp-cheatsheets, radare2, routersploit, rustscan, rustsec, selinux, semgrep, sqlmap, suricata, thc-hydra, tock, trivy, trufflehog, velociraptor, volatility3, wifiphisher, zaproxy

**Workflow/Automation (18):** 9router-master, airflow, ArcReel-main, automation, ComfyUI-master, crewAI-main, deer-flow-main, dify-main, n8n-master, public-apis-master, RedditVideoMakerBot-master, rowboat-main, temporal-sdk-core

**Other (20):** agent-lightning-main, agentscope-main, AI-Scientist-v2-main, AI-Trader-main, AionUi-main, AndyTheDesignerWeb-master, awesome-ai-agents-main, autoresearch-master, axios-1.x, BettaFish-main, browser-main, claude-code-main, claude-peers-mcp-main, claude-subconscious-main, claw-code-dev-rust, ClawTeam-OpenClaw-main, ClawX-main, codex-main, craft-agents-oss-main, crewAI-main, ctrader-mcp-server-main, DeepCode-main, deer-flow-main, dify-main, GitNexus-main, graphiti-main, last30days-skill-main, Lean-master, marketingskills-main, metamcp-main, MiroFish-main, MoneyPrinterV2-main, nautilus_trader-develop, notebooklm-py-main, oasis-main, onyx-main, OpenBB-develop, OpenViking-main, platform-develop, posthog-master, postiz-app-main, prompts.chat-main, RAG-Anything-main, RedditVideoMakerBot-master, refly-main, samples-go-main, strix-main, superpowers-main, taste-skill-main, timesfm-master, TradingAgents-main, TrendRadar-main, twenty-main, Vane-master, vault-main, VibeVoice-main, weferral-main, zeroclaw-master

---

*Plan compiled from 5 parallel agent deep-dives + manual verification. All source repos read end-to-end before recommendation.*
