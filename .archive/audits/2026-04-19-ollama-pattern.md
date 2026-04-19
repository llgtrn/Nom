# Ollama Pattern Audit — 2026-04-19

> Reference repo: `C:\Users\trngh\Documents\APP\Accelworld\upstreams\ollama`  
> Focus: REST API design, CLI zero-config setup, local inference serving, HTTP server, model download, GPU offloading.  
> Auditor: Pattern-extraction analyst (read-only)

---

## 1. Pattern Summary

Ollama is a **client-server local LLM runtime** written in Go. It exposes a REST API for model management and inference, a CLI that auto-discovers and auto-pulls models, and a subprocess-based inference engine (llama.cpp or an experimental native engine) with automatic GPU layer offloading.

**High-level architecture:**
- **`api/`** — Typed Go client for all REST endpoints. Uses JSON for sync calls and **NDJSON** for streaming (generate, chat, pull/push progress).
- **`cmd/`** — Cobra-based CLI. Zero-config defaults via `OLLAMA_HOST` (default `127.0.0.1:11434`). `ollama run <model>` auto-pulls missing models before entering an interactive REPL.
- **`llm/`** — Spawns a per-model **runner subprocess** (`ollama runner --model <path> --port <rand>`). Supports two backends: legacy llama.cpp (CGO) and an experimental Ollama engine (Rust-like tokenizer + GGML). GPU offloading is computed with a memory-layout solver that assigns transformer layers to available GPUs.
- **`server/`** — Gin HTTP server. Routes dispatch to the **Scheduler**, which maintains a loaded-runner cache, evicts models when VRAM is exhausted, and serializes model loads.
- **`server/download.go`** — Resumable, multi-part blob downloads (up to 16 parts, 100 MB–1 GB each) with exponential-backoff retry, stall detection (30 s), and atomic rename on completion.

---

## 2. Key Source Files

| File | Role | Key Symbols |
|------|------|-------------|
| `api/client.go` | HTTP client | `Client`, `ClientFromEnvironment()`, `do()`, `stream()`, `Generate()`, `Chat()`, `Pull()`, `List()`, `ListRunning()` |
| `api/types.go` | API DTOs | `GenerateRequest`, `ChatRequest`, `PullRequest`, `ProgressResponse`, `Options`, `Runner` (with `NumGPU`, `MainGPU`, `UseMMap`), `Metrics` |
| `cmd/cmd.go` | CLI handlers | `RunHandler`, `CreateHandler`, `PullHandler`, `PushHandler`, `loadOrUnloadModel()`, `generateEmbedding()` |
| `cmd/start.go` | Server bootstrap | `waitForServer()` — polls `Client.Heartbeat()` every 500 ms |
| `envconfig/config.go` | Zero-config env | `Host()`, `Models()`, `KeepAlive()`, `LoadTimeout()`, `AllowedOrigins()` |
| `llm/server.go` | Inference process | `LlamaServer` interface, `llmServer` struct, `NewLlamaServer()`, `StartRunner()`, `Load()`, `LoadRequest`, `LoadOperationFit/Alloc/Commit/Close` |
| `llm/server.go` (layout) | GPU offload solver | `createLayout()`, `buildLayout()`, `verifyLayout()`, `assignLayers()`, `findBestFit()` |
| `server/routes.go` | HTTP routing | `Server` struct, `GenerateHandler()`, `ListHandler()`, `PsHandler()`, `scheduleRunner()` |
| `server/sched.go` | Model lifecycle | `Scheduler`, `InitScheduler()`, `GetRunner()`, `processPending()`, `findRunnerToUnload()` |
| `server/download.go` | Model download | `blobDownload`, `blobDownloadPart`, `downloadBlob()`, `Prepare()`, `Run()`, `downloadChunk()` |
| `server/model.go` | Model parsing | `parseFromModel()`, `layerGGML`, `detectChatTemplate()` |
| `discover/gpu.go` | HW discovery | `GetSystemInfo()`, `cudaJetpack()` — CPU/GPU memory enumeration |

---

## 3. Nom Mapping

> Target: `nom-compose/src/ollama.rs` (does **not yet exist** in the Nom repo).  
> Related existing code: `nom-compose/src/llama_compose.rs` (RAG pipeline types only) and `nom-compose/src/backends/` (no Ollama backend yet).

| Ollama Concept | Proposed Nom Equivalent | Notes |
|----------------|------------------------|-------|
| `api.Client` | `OllamaClient` (reqwest-based) | Read `OLLAMA_HOST` (or `NOM_OLLAMA_HOST`) with default `http://127.0.0.1:11434`. Support NDJSON streaming via `futures::Stream`. |
| `api.GenerateRequest` / `ChatRequest` | Nomx AST nodes for `generate` and `chat` commands | Map `Options` (untyped `map[string]any`) to a strongly-typed Rust struct mirroring `api.Options` + `api.Runner`. |
| `api.ProgressResponse` | `OllamaPullProgress` event | Emit through Nom’s `progress.rs` or `streaming.rs` channels so the Canvas UI can render progress bars. |
| `server.download.blobDownload` | `OllamaPullTask` | Async task using `tokio::fs` + `reqwest`. Replicate 16-part ranged downloads, part-state JSON files, and retry with exponential backoff. |
| `llm.LlamaServer` | `OllamaRunner` trait | Two impls: (1) **HTTP proxy** to an external Ollama binary (fastest to ship), (2) **Native runner** via `llama-cpp-rs` bindings (later). |
| `llm.LoadRequest` / `LoadOperation*` | `OllamaLoadRequest` enum | Fit → Alloc → Commit → Close pipeline for native runner memory planning. |
| `server.Scheduler` | Extend `nom-compose/src/orchestrator.rs` | Add an `OllamaSlot` to the existing orchestrator: keyed by model digest, with LRU eviction and a `max_runners` cap. |
| `envconfig.Host()` | `ollama_host()` helper in `nom-compose/src/context.rs` | Parse scheme+host+port; treat `ollama.com` as HTTPS automatically. |
| `discover.GetSystemInfo()` | `gpu_info` module in `nom-compose/src/backends/` | Use `nvml-wrapper` (NVML) + `rocm-smi-lib` crates. Fall back to `sysinfo` for CPU RAM. |
| `server.Model` / manifest layers | `OllamaModelManifest` | Parse Ollama manifest JSON + GGUF metadata via `gguf-rs` (or shell out to `ollama show`). |

**Recommended integration path for Nom:**
1. **Phase 1 (HTTP wrapper):** Implement `OllamaClient` and `OllamaPullTask` so Nom can talk to an already-installed `ollama` binary. Re-use Ollama’s scheduler, download, and GPU logic entirely.
2. **Phase 2 (Native runner):** Add `OllamaRunner` trait impl using `llama-cpp-rs`. Port the layer-assignment algorithm from `llm/server.go` into Rust.
3. **Phase 3 (Embedded scheduler):** Port `server/sched.go` eviction and runner-ref logic into `nom-compose/src/orchestrator.rs` when Nom needs to manage runners without an external Ollama server.

---

## 4. Licensing / Complexity Notes

**License:** MIT (per `LICENSE` root file). Permissive — allows porting, vendoring, or wrapping without copyleft concerns.

**Code volume in focus areas:**
- `api/`: ~85 KB
- `cmd/`: ~802 KB
- `llm/`: ~70 KB
- `server/`: ~566 KB
- **Total:** ~1.5 MB of Go source.

**Go → Rust port complexity:**
- **Low:** API types, client DTOs, progress events, env-config helpers.
- **Medium:** Multi-part resumable download (`server/download.go`), HTTP route dispatch (`server/routes.go`), manifest parsing (`server/model.go`).
- **High:**
  - **GPU layer offloading solver** (`llm/server.go` lines 510–1100). Iterative memory layout with `createLayout` → `buildLayout` → `verifyLayout` → `assignLayers`. Must account for per-GPU free VRAM, graph overhead, partial-vs-full offload penalties, flash-attention constraints, and cross-platform quirks (Metal mmap issues, Windows CUDA mmap disable, Jetson overhead).
  - **Scheduler eviction** (`server/sched.go`). Concurrency-heavy: pending/finished channels, runner ref-counting, `needsReload` checks (context size, adapters, options), and VRAM-triggered eviction with a 5-second recovery window.
  - **Subprocess runner lifecycle** (`llm/server.go` + `StartRunner`). Spawning `ollama runner` with correct `LD_LIBRARY_PATH` / `PATH` injection, GPU visibility env vars (`CUDA_VISIBLE_DEVICES`, `HIP_VISIBLE_DEVICES`), stdout/stderr pipes, and port discovery (`localhost:0` or ephemeral fallback).

**CGO dependency:** Ollama links llama.cpp via CGO. A Rust port would replace this with `llama-cpp-rs` (or `candle`, `burn`) and must re-implement the `LlamaServer` GRPC-like interface over localhost HTTP.

---

## 5. Adoption Effort Estimate

| Scope | Effort | What it covers |
|-------|--------|----------------|
| **HTTP Client + Pull** | 1–2 weeks | `OllamaClient`, `OllamaPullTask`, env-config, NDJSON streaming, progress events. Nom talks to existing `ollama` binary. |
| **+ Generate/Chat/Embed** | +1 week | Add Nomx AST nodes, request builders, response parsers, metrics (`Metrics` struct). |
| **+ Local Runner (subprocess)** | +2–3 weeks | Port `StartRunner` logic: spawn `ollama runner`, set GPU envs, health-check (`Ping` / `WaitUntilRunning`), forward completion requests. |
| **+ GPU Offload Solver** | +3–4 weeks | Port `createLayout` / `assignLayers` to Rust. Integrate with `nvml-wrapper` for VRAM queries. Handle Metal/CUDA/Vulkan edge cases. |
| **+ Full Scheduler + Eviction** | +2–3 weeks | Port `Scheduler` with pending queue, runner cache, LRU eviction, `max_runners`, and `keep_alive` expiration. |
| **Total native port** | **8–12 weeks** | A complete drop-in replacement of Ollama’s serving stack in Rust. |

**Risk areas:**
- GPU memory estimation drift: Ollama’s solver has platform-specific buffers (`envconfig.GpuOverhead()`, Jetson quirks). Any port will need empirical tuning on target hardware.
- Flash-attention + KV-cache quantization compatibility matrix (model-arch × GPU compute-capability). Ollama encodes this in `llm/server.go` (~200 lines of conditional logic).
- The experimental “Ollama engine” (non-llama.cpp path) is rapidly evolving; porting it is a moving target.

**Recommendation for Nom:** Start with the **1–2 week HTTP-client wrapper** against an externally installed Ollama. This immediately unlocks local LLM capabilities without committing to a multi-month port. Isolate behind the `OllamaRunner` trait so the backend can be swapped to a native Rust runner later without changing Nomx semantics.

---

*Report generated: 2026-04-19*
