# vLLM Pattern Audit — 2026-04-19

**Reference:** `C:\Users\trngh\Documents\APP\Accelworld\upstreams\vllm` (vLLM v1 architecture)  
**Auditor:** Pattern-extraction analyst (Nom project)  
**Scope:** PagedAttention memory management, async engine/scheduling, distributed workers, OpenAI-compatible API server, throughput/batching patterns.

---

## 1. Pattern Summary

vLLM is a high-throughput LLM inference engine built around **PagedAttention** (block-level KV cache management) and a **unified scheduler** that treats prefill and decode as a single token-budget scheduling problem. The v1 architecture (located under `vllm/v1/`) separates the frontend (async API server) from the backend engine core via ZMQ-based IPC, supports multiple parallel strategies (TP/PP/DP), and aggressively optimizes GPU utilization through CUDA graphs, persistent input buffers, micro-batching for pipeline parallelism, and speculative decoding.

Key architectural patterns observed:

- **Block-based KV cache:** The KV cache is split into fixed-size blocks (`KVCacheBlock`). A `BlockPool` manages allocation, freeing, and prefix caching via a hash-to-block map (`BlockHashToBlockMap`). This eliminates external fragmentation and enables prefix caching at block granularity.
- **Unified scheduling:** The `Scheduler` (`vllm/v1/core/sched/scheduler.py`) does not distinguish prefill vs decode phases. Each request has `num_computed_tokens` and `num_tokens_with_spec`. The scheduler assigns tokens within a `max_num_scheduled_tokens` budget, supporting chunked prefills, prefix caching hits, and speculative tokens naturally.
- **Async engine core:** `AsyncLLM` (`vllm/v1/engine/async_llm.py`) wraps the engine. The actual execution loop runs in a background process (`EngineCore` / `EngineCoreProc`) communicated via `EngineCoreClient` (ZMQ + msgpack). This decouples request ingestion from GPU execution.
- **Persistent GPU buffers:** `InputBuffers` and `InputBatch` (`vllm/v1/worker/gpu/input_batch.py`) allocate GPU memory once and reuse it across steps, avoiding allocation overhead during inference.
- **CUDA graph acceleration:** `CudaGraphManager` / `ModelCudaGraphManager` (`vllm/v1/worker/gpu/cudagraph_utils.py`, `vllm/v1/worker/gpu_model_runner.py`) capture decode and mixed-mode graphs to eliminate CPU launch overhead.
- **Micro-batching / compute-communication overlap:** `UBatchContext` (`vllm/v1/worker/ubatching.py`) implements double-buffered micro-batching (2 u-batches by default) using threading barriers and CUDA events. This overlaps communication (e.g., pipeline parallelism sends) with computation.
- **Speculative decoding:** Integrated at the worker level (`vllm/v1/worker/gpu/spec_decode/`) with support for EAGLE, Medusa, n-gram, and draft-model-based speculation.
- **Modular entrypoints:** The OpenAI-compatible API server (`vllm/entrypoints/openai/api_server.py`) is a FastAPI app that dynamically registers routers and uses `build_async_engine_client` to manage engine lifecycle.

---

## 2. Key Source Files

### PagedAttention / KV Cache Management (`vllm/v1/core/`)

| File | Role |
|------|------|
| `vllm/v1/core/kv_cache_manager.py` | `KVCacheManager` — top-level allocator. Computes prefix cache hits, delegates block allocation to `BlockPool`. Returns `KVCacheBlocks`. |
| `vllm/v1/core/block_pool.py` | `BlockPool` — owns all `KVCacheBlock` instances, maintains `FreeKVCacheBlockQueue` (eviction order) and `BlockHashToBlockMap` (prefix cache lookup). |
| `vllm/v1/core/kv_cache_utils.py` | `KVCacheBlock`, `BlockHash`, `FreeKVCacheBlockQueue`, block hashing utilities. |
| `vllm/v1/core/kv_cache_coordinator.py` | Coordinates KV cache state across distributed workers (DCP/PCP aware). |
| `vllm/v1/core/single_type_kv_cache_manager.py` | Per-KV-cache-group manager for heterogeneous cache specs. |

### Scheduling (`vllm/v1/core/sched/`)

| File | Role |
|------|------|
| `vllm/v1/core/sched/scheduler.py` | `Scheduler` — main scheduling loop. Token-budget-based, preempts lowest-priority requests when KV cache is exhausted. Outputs `SchedulerOutput`. |
| `vllm/v1/core/sched/request_queue.py` | `RequestQueue` (ABC), `FCFSRequestQueue`, `PriorityRequestQueue` — queue policies. |
| `vllm/v1/core/sched/output.py` | `SchedulerOutput`, `NewRequestData`, `CachedRequestData` — data passed to workers. |
| `vllm/v1/core/sched/async_scheduler.py` | Async variant of the scheduler for non-blocking scheduling. |

### Engine (`vllm/v1/engine/`)

| File | Role |
|------|------|
| `vllm/v1/engine/async_llm.py` | `AsyncLLM` — public async API. Manages `InputProcessor`, `OutputProcessor`, and `EngineCoreClient`. |
| `vllm/v1/engine/llm_engine.py` | `LLMEngine` — synchronous wrapper for backwards compatibility. |
| `vllm/v1/engine/core.py` | `EngineCore` / `EngineCoreProc` — inner execution loop. Initializes scheduler, executor, and batch queue for PP. |
| `vllm/v1/engine/core_client.py` | `EngineCoreClient`, `AsyncMPClient`, `SyncMPClient`, `DPAsyncMPClient`, `DPLBAsyncMPClient` — ZMQ-based IPC clients. |
| `vllm/v1/engine/output_processor.py` | Converts `EngineCoreOutputs` into streamed `RequestOutput`. |
| `vllm/v1/engine/input_processor.py` | Converts user prompts into `EngineCoreRequest`. |

### Worker / Model Runner (`vllm/v1/worker/`)

| File | Role |
|------|------|
| `vllm/v1/worker/gpu_worker.py` | `Worker` — GPU worker. Initializes device, distributed env, loads model via `model_runner`, handles sleep/wake for GPU memory offloading. |
| `vllm/v1/worker/gpu_model_runner.py` | `GPUModelRunner` (V1) — forward pass execution, attention metadata construction, sampling, logits processing. |
| `vllm/v1/worker/gpu/model_runner.py` | `GPUModelRunner` (V2) — refactored common runner shared by all models. Uses `InputBatch`, `BlockTables`, `Sampler`, and `CudagraphDispatcher`. |
| `vllm/v1/worker/gpu/input_batch.py` | `InputBatch`, `InputBuffers` — persistent GPU tensors for batched inputs. Includes Triton kernel `prepare_prefill_inputs_kernel`. |
| `vllm/v1/worker/gpu/cudagraph_utils.py` | `CudaGraphManager`, `BatchExecutionDescriptor` — captures and replays CUDA graphs for decode/mixed batches. |
| `vllm/v1/worker/ubatching.py` | `UBatchContext` — double-buffered micro-batching with threading barriers and CUDA events for PP overlap. |
| `vllm/v1/worker/gpu/block_table.py` | `BlockTables` — manages per-request block ID mappings on GPU. |
| `vllm/v1/worker/worker_base.py` | `WorkerBase` — abstract worker interface. |

### Executor (`vllm/v1/executor/`)

| File | Role |
|------|------|
| `vllm/v1/executor/abstract.py` | `Executor` (ABC) — selects backend (`ray`, `mp`, `uni`, `external_launcher`). Defines `collective_rpc`. |
| `vllm/v1/executor/uniproc_executor.py` | `UniProcExecutor` — single-process executor. Uses `ThreadPoolExecutor` for async outputs when `max_concurrent_batches > 1`. |
| `vllm/v1/executor/multiproc_executor.py` | `MultiprocExecutor` — multi-process executor with shared-memory message queues (`MessageQueue`) for `SchedulerOutput` broadcast. |

### Entrypoints (`vllm/entrypoints/`)

| File | Role |
|------|------|
| `vllm/entrypoints/openai/api_server.py` | FastAPI app builder. `build_app()`, `build_async_engine_client()`, `init_app_state()`. Registers routers dynamically based on `supported_tasks`. |
| `vllm/entrypoints/openai/server_utils.py` | Lifespan, middleware (auth, request ID, scaling), exception handlers. |
| `vllm/entrypoints/openai/generate/api_router.py` | OpenAI `/v1/completions` and `/v1/chat/completions` routes. |

### Attention Backends (`vllm/v1/attention/`)

| File | Role |
|------|------|
| `vllm/v1/attention/backend.py` | `AttentionBackend`, `AttentionMetadata`, `AttentionMetadataBuilder` — pluggable attention interface. |
| `vllm/v1/attention/backends/utils.py` | Shared utilities: slot mapping, block table construction, chunked-local-attention splitting. |

---

## 3. Nom Mapping

**Target:** `nom-compose/src/vllm.rs` (high-throughput inference serving backend for Nom Canvas)

The following table maps vLLM patterns to a hypothetical Rust implementation in Nom. The mapping preserves the logical architecture while adapting to Rust’s ownership model and async ecosystem (Tokio).

| vLLM Component | Nom Equivalent (`nom-compose/src/vllm.rs`) | Notes |
|----------------|--------------------------------------------|-------|
| `KVCacheManager` + `BlockPool` | `KvCacheManager` with `BlockPool` (arena + free list) | Use a fixed-size arena (`Vec<KvBlock>`) and a doubly-linked free list (or `VecDeque` of indices) to avoid heap allocations per step. Prefix cache: `HashMap<BlockHash, Vec<usize>>` mapping to block indices. |
| `Scheduler` | `Scheduler` (token-budget loop) | Rust `async` not needed inside the hot loop; run in a dedicated Tokio task or `std::thread`. Use `BinaryHeap<Request>` for priority policy, `VecDeque<Request>` for FCFS. |
| `EngineCore` | `EngineCore` (background task) | A Tokio task or `rayon` thread that receives `SchedulerOutput` from a channel, calls `collective_rpc("execute_model", ...)`, and sends outputs back. |
| `EngineCoreClient` | `EngineCoreClient` (IPC abstraction) | If multi-process: use `tokio::net::UnixStream` or shared-memory channels (`ipc-channel` crate) with `rkyv` / `bincode` serialization. In-process: direct `tokio::sync::mpsc` channels. |
| `AsyncLLM` | `VllmEngine` (public async API) | Wraps `EngineCoreClient`. Spawns an `output_handler` Tokio task that streams `RequestOutput` to callers. |
| `Worker` | `GpuWorker` | One per GPU process. Initializes NCCL/RCCL via `cudarc` or `rustacuda`. Loads model weights. Manages `ModelRunner`. |
| `GPUModelRunner` | `GpuModelRunner` | Executes `forward()` using `candle`, `burn`, or custom CUDA kernels (via `cust`). Manages `InputBatch` and `BlockTables`. |
| `InputBatch` + `InputBuffers` | `InputBatch` + `InputBuffers` | Pre-allocate GPU buffers (`DeviceBuffer` via `rustacuda` or `wgpu`) for `input_ids`, `positions`, `query_start_loc`, `seq_lens`. Re-slice each step. |
| `CudaGraphManager` | `CudaGraphManager` | If using CUDA via `cudarc`, capture graphs with `cuStreamBeginCapture` / `cuGraphInstantiate`. Replay for uniform decode batches. Skip if using a pure Rust compute stack. |
| `UBatchContext` | `UbatchContext` (micro-batching) | Two-thread double-buffering with `std::sync::Barrier` and CUDA events (`cudaEvent_t`). One thread computes, the other handles PP sends. Rust `std::thread::scope` may help. |
| `BlockTables` | `BlockTables` | GPU tensor (`DeviceBuffer<i32>`) storing block IDs per request. Updated each step from the scheduler’s `KVCacheBlocks`. |
| `api_server.py` | `api_server.rs` (Axum/FastAPI equiv) | Axum router. `build_app()` registers routes. `build_async_engine_client()` returns a `VllmEngine` handle in an `Arc<...>`. |
| `RequestQueue` | `RequestQueue` trait + `FcfsQueue` / `PriorityQueue` | Trait with `add`, `pop`, `peek`. `PriorityQueue` uses `std::collections::BinaryHeap` ordered by `(priority, arrival_time)`. |

### Critical Rust-specific adaptations

1. **Memory safety with KV blocks:** The Python `BlockPool` uses reference counts and manual linked-list manipulation. In Rust, represent the free list as `Vec<Option<NonZeroUsize>>` or a custom intrusive list with `unsafe` if zero-cost is required. Prefix caching can use a `DashMap<BlockHash, Vec<usize>>` for concurrent lookups.

2. **Async scheduler loop:** Python’s `Scheduler.schedule()` is synchronous and runs in a tight loop. In Rust, run it in a `tokio::task::spawn_blocking` or dedicated thread to avoid event-loop jitter, feeding `SchedulerOutput` into an `tokio::sync::mpsc::channel`.

3. **GPU model execution:** vLLM heavily relies on PyTorch and custom Triton/CUDA kernels. A Nom Rust port would need:
   - A compute backend: `candle-core` (pure Rust, limited kernel set) or FFI to `vllm`/`TensorRT-LLM` executors.
   - Attention: FlashInfer and FlashAttention are CUDA-only. Either wrap them via `cxx`/`bindgen` or adopt a Rust-native attention implementation (slower but portable).

4. **Serialization:** Replace `msgspec.msgpack` / `cloudpickle` with `rkyv` (zero-copy) or `bincode` for IPC between API server and engine core.

---

## 4. Licensing / Complexity Notes

### Licensing
- **License:** Apache-2.0 (SPDX identifiers present in every file).  
- **Implication for Nom:** Direct copy-paste of Python code into Nom is prohibited if Nom is not Apache-2.0 compatible. Pattern reimplementation in Rust from scratch is clean-room safe, but any direct port of logic should note provenance. Custom CUDA kernels in `csrc/` are also Apache-2.0.

### Complexity
- **CUDA dependency:** vLLM is deeply tied to CUDA/ROCm (PyTorch, NCCL, custom Triton kernels, CUDA graphs). A pure-Rust inference backend without GPU kernel equivalents would lose the throughput benefits that make vLLM interesting.
- **Line count:** The `vllm/v1/` directory alone is ~25–30 kLOC. The full repo exceeds 150 kLOC including kernels, model definitions, and entrypoints.
- **Build system:** CMake + PyTorch extensions + Triton JIT compilation. A Rust port would replace this with `cargo` + `build.rs` for CUDA compilation (e.g., `cuda-builder` or manual `nvcc` invocation).
- **Operational complexity:** vLLM manages distributed state (TP/PP/DP), elastic scaling, sleep/wake for GPU memory, KV cache offloading, and multimodal encoder caches. Replicating even the single-GPU path is a multi-month effort.

---

## 5. Adoption Effort Estimate

| Phase | Scope | Effort | Risk |
|-------|-------|--------|------|
| **P0 — Scheduler + BlockPool** | Port `Scheduler`, `BlockPool`, `RequestQueue`, `KVCacheManager` to Rust with fake model runner. | 2–3 weeks | Low. Pure algorithmic code. |
| **P1 — Async Engine Core** | `EngineCore`, `EngineCoreClient` (in-process channels), `VllmEngine` async API, `InputBatch` buffers. | 2–3 weeks | Medium. Async state machine must match Python semantics. |
| **P2 — GPU Model Runner (single GPU)** | Integrate a Rust GPU backend (`candle` or FFI to `libtorch`). Implement attention metadata, `BlockTables`, sampling. | 4–6 weeks | High. Kernel gap is the main blocker. |
| **P3 — CUDA Graphs + Micro-batching** | Capture/replay CUDA graphs, implement `UbatchContext` for PP overlap. | 3–4 weeks | High. Requires stable CUDA graph APIs in Rust bindings. |
| **P4 — Distributed Executor** | TP (NCCL all-reduce), PP (send/recv), DP (load balancing). `MultiprocExecutor` equivalent. | 4–6 weeks | High. NCCL bindings and process orchestration are brittle. |
| **P5 — API Server + Entrypoints** | Axum server, OpenAI-compatible routes, request lifecycle management. | 1–2 weeks | Low. Straightforward web layer. |
| **P6 — Speculative Decode + Prefix Cache Tuning** | EAGLE/n-gram speculation, advanced prefix cache eviction. | 3–4 weeks | Medium. Algorithmically complex but no new infra. |

**Total realistic estimate:** 5–7 engineer-months for a feature-parity single-GPU port; 9–12 months for multi-GPU distributed inference matching vLLM v1 throughput.

**Recommended strategy for Nom:**
1. Do **not** reimplement the full GPU kernel stack. Instead, treat vLLM as an **external executor**: spawn vLLM processes and control them via the OpenAI API or a thin gRPC/UnixSocket wrapper.
2. If a native Rust backend is required, scope down to **CPU inference** or **ONNX/TensorRT-LLM** delegation, and only port the scheduler (`P0`) to manage batching policy inside Nom.
3. If full Rust-GPU is a hard requirement, budget for a dedicated GPU-kernel engineer and evaluate `candle` kernel coverage before committing to `P2`.

---
*End of audit.*
