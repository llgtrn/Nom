# Wasmtime Pattern Audit — Nom WASM Sandbox

**Date:** 2026-04-19  
**Source:** `C:\Users\trngh\Documents\APP\Accelworld\upstreams\wasmtime` (main branch)  
**Target Nom surface:** `nom-compose/src/wasm.rs` (WASM sandbox for polyglot plugins)  
**Auditor:** Pattern-extraction analyst (read-only)

---

## 1. Pattern Summary

Wasmtime’s embedding API follows a strict lifecycle:

```
Config → Engine → Module → Store<T> → Linker<T> → Instance
```

| Layer | Responsibility |
|-------|----------------|
| **Engine** | Global compilation & runtime environment. Owns the compiler (Cranelift/Winch) and is shared across threads. |
| **Module** | Immutable, compiled WASM artifact. Cheap to clone (Arc-backed). Supports JIT (`Module::new`) and AOT (`serialize`/`deserialize`). |
| **Store<T>** | Container for all mutable WASM state (instances, memories, tables, globals). `T` is arbitrary host state accessible via `Caller<'_, T>`. Short-lived; dropping it frees everything inside. |
| **Linker<T>** | Name-based import resolver. Maps `(module, name)` strings to host functions or other `Extern`s. Populated once at startup, reused across `Store`s. |
| **Instance** | Live instantiation of a `Module` inside a `Store`. Exports are fetched by name and cast to typed handles (`TypedFunc`, `Memory`, `Global`, etc.). |

**Host-function binding pattern:**  
`Linker::func_wrap(module, name, closure)` consumes any `impl IntoFunc<T, Params, Results>`. The `IntoFunc` trait (implemented via macro for arities 0–N) auto-generates the WASM trampoline so the closure looks like ordinary Rust. A `Caller<'_, T>` can be the first parameter to give the closure access to the host `Store` state.

**Sandboxing levers:**
- **Fuel:** `Config::consume_fuel(true)` + `Store::add_fuel(n)` → deterministic instruction-budget traps.
- **Epoch interruption:** `Config::epoch_interruption(true)` + `Engine::increment_epoch()` → lightweight, timer-style pre-emption (cheaper than fuel).
- **Stack limits:** `Config::max_wasm_stack(size)`.
- **Memory limits:** `Store::limiter(|ctx| &mut MyLimiter)` where `MyLimiter: ResourceLimiter` controls memory/table growth.
- **WASI capability sandbox:** Filesystem, network, clocks, and randomness are not granted by default; they must be explicitly added via `wasmtime_wasi::p1::add_to_linker_sync` or `p2` variants, and the host supplies a `WasiCtx` that can restrict directory pre-opens, permissions, etc.

**Allocation strategies:**
- `InstanceAllocationStrategy::OnDemand` (default) — allocate at instantiation, free on `Store` drop.
- `InstanceAllocationStrategy::Pooling` — pre-allocate pools of memories/tables for high-density, low-latency instantiation.

---

## 2. Key Source Files

### Public API (crates/wasmtime/src/)

| File | Structs / Traits / Functions | What to study |
|------|------------------------------|---------------|
| `runtime/store.rs` | `Store<T>`, `StoreContext<T>`, `StoreContextMut<T>`, `AsContext`, `AsContextMut` | Host-state container, context threading, fuel/epoch bookkeeping, resource limiter attachment. |
| `runtime/instance.rs` | `Instance`, `Instance::new`, `Instance::new_async`, `InstancePre` | Instantiation flow, export enumeration, type-checking imports. |
| `runtime/module.rs` | `Module`, `Module::new`, `Module::deserialize` | Compilation, caching, serialization, registry of compiled artifacts. |
| `runtime/linker.rs` | `Linker<T>`, `Linker::func_wrap`, `Linker::func_wrap_async`, `Linker::instantiate`, `Definition`, `ImportKey` | Name-based resolution, host-function insertion, duplicate/shadowing policies. |
| `runtime/func.rs` | `Func`, `TypedFunc`, `Func::wrap`, `IntoFunc<T, Params, Results>`, `Caller<'_, T>` | Typed/untyped calling conventions, host-function trampoline generation. |
| `runtime/memory.rs` | `Memory`, `Memory::read`, `Memory::write`, `Memory::data`, `MemoryAccessError` | Safe linear-memory access rules; raw pointers are discouraged because growth can relocate backing storage. |
| `config.rs` | `Config`, `InstanceAllocationStrategy`, `PoolingAllocationConfig` | Tuning knobs: fuel, epochs, stack size, memory creator, feature gates. |

### Runtime VM (crates/wasmtime/src/runtime/vm/)

| File | Key items | Relevance |
|------|-----------|-----------|
| `vm/instance.rs` | `Instance` (internal), `InstanceHandle`, `InstanceAllocationRequest`, `VMContext` | Low-level layout: memories, tables, vmctx, pinning invariants. Understand if writing a custom allocator or inspecting memory layout. |
| `vm/memory.rs` | `Memory`, `RuntimeMemoryCreator`, `ExportMemory` | Backing store for linear memory; supports custom memory creators for sandboxed heaps. |
| `vm/instance/allocator.rs` | `InstanceAllocator`, `OnDemandInstanceAllocator` | Where memories/tables are actually allocated. |

### WASI bindings

| Crate / File | Key items | Relevance |
|--------------|-----------|-----------|
| `crates/wasi-common/src/lib.rs` | `WasiCtx`, `WasiFile`, `WasiDir`, `Table`, `add_to_linker` (macro-generated) | Legacy Preview 1. Trait-based design lets embedders swap filesystem implementations (e.g., virtual FS). |
| `crates/wasi/src/lib.rs` | `WasiCtx`, `WasiCtxBuilder`, `WasiView`, `p1`, `p2`, `p3` | Modern WASI. `p2` uses the Component Model. `WasiCtxBuilder` sets pre-opens, env vars, args, clocks, RNG. |
| `crates/wasi/src/p1.rs` (implied) | `add_to_linker_sync`, `add_to_linker_async` | Convenience one-liners to graft all WASI imports onto a `Linker`. |

---

## 3. Nom Mapping

### Current Nom WASM code

- **`nom-canvas/crates/nom-canvas-graph/src/wasm_bridge.rs`** exists but is **compile-time metadata only**: `WasmTarget`, `WasmModule`, `WasmFeatureGate`, `WasmBridge`. It tracks target triples, export names, and feature flags. There is **no runtime engine, linker, or sandbox** here yet.
- **`nom-compose/src/wasm.rs`** does **not exist** yet. This is the intended location for the polyglot-plugin sandbox.

### Proposed Nom architecture (inferred from Wasmtime patterns)

```rust
// nom-compose/src/wasm.rs  (conceptual mapping)

use wasmtime::{Config, Engine, Linker, Module, Store, Instance, TypedFunc};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, p1};

/// Host state passed to every sandboxed plugin call.
pub struct NomPluginHost {
    pub wasi: WasiCtx,
    // Nom-specific resources: graph handles, I/O streams, etc.
}

/// Lightweight wrapper around wasmtime’s lifecycle.
pub struct WasmSandbox {
    engine: Engine,
    linker: Linker<NomPluginHost>,
}

impl WasmSandbox {
    pub fn new() -> anyhow::Result<Self> {
        let mut config = Config::new();
        config.consume_fuel(true);
        config.epoch_interruption(true);
        config.max_wasm_stack(512 * 1024);
        // Optional: pooling allocator for high-density plugin farms
        // config.allocation_strategy(InstanceAllocationStrategy::pooling());

        let engine = Engine::new(&config)?;
        let mut linker = Linker::new(&engine);

        // Bind Nom host functions
        linker.func_wrap("nom", "log", |caller: Caller<'_, NomPluginHost>, msg: i32, len: i32| {
            // read from caller's memory, log, etc.
        })?;

        // Add WASI for sandboxed I/O
        p1::add_to_linker_sync(&mut linker, |host| &mut host.wasi)?;

        Ok(Self { engine, linker })
    }

    pub fn instantiate(&self, wasm_bytes: &[u8]) -> anyhow::Result<PluginInstance> {
        let module = Module::new(&self.engine, wasm_bytes)?;
        let wasi = WasiCtxBuilder::new()
            .inherit_stdio()
            .preopened_dir(..., "/sandbox")?
            .build();
        let mut store = Store::new(&self.engine, NomPluginHost { wasi });
        store.add_fuel(10_000_000)?;

        let instance = self.linker.instantiate(&mut store, &module)?;
        Ok(PluginInstance { store, instance })
    }
}

/// Per-plugin handle.
pub struct PluginInstance {
    store: Store<NomPluginHost>,
    instance: Instance,
}
```

### Gaps Nom must fill

1. **ResourceLimiter implementation** — to cap per-plugin memory/table growth.
2. **Fuel bookkeeping** — reset/add fuel per call; decide trap-vs-retry policy.
3. **Epoch coordination** — if using async or multi-tenant plugin farms, Nom needs an epoch increment loop (timer-driven or task-count-driven).
4. **Host-function ABI** — Wasmtime’s `func_wrap` is Rust-native; Nom needs to define its own stable import namespace (e.g., `"nom" "log"`, `"nom" "graph_get"`) and marshal types across the WASM boundary (i32/i64 handles + linear-memory buffers).
5. **Component model vs core WASM** — If Nom plugins are plain core WASM, use `Module` + `Linker`. If they use WIT interfaces, upgrade to `wasmtime::component::Component` + `component::Linker`.

---

## 4. Licensing / Complexity Notes

- **License:** Apache-2.0 (confirmed by `LICENSE` header in repo root). Compatible with Nom’s own licensing; attribution and license copy required if vendoring or redistributing compiled artifacts.
- **Scale:** ~1,917 `.rs` files across the workspace; `crates/wasmtime/src/` alone has 232 files.
- **Maturity:** Production-grade. Used by Fastly, Fermyon, Embark, and the Bytecode Alliance. Ships with:
  - Two compilers: Cranelift (default) and Winch (fast-debug).
  - Async execution with fiber-based stack switching.
  - WebAssembly Component Model (WASI Preview 2).
  - GC, exception handling, stack switching (experimental).
- **Build cost:** Heavy. `wasmtime` pulls in `wasmtime-environ`, `cranelift-*`, `wasmparser`, `wiggle`, etc. Compile times are significant; expect minutes on a clean build. AOT artifact serialization (`Module::serialize`) is recommended for distribution.
- **Runtime cost:** Near-native for JIT-compiled code. Fuel/epoch instrumentation adds minor overhead; epoch checks are cheaper than fuel decrements.

---

## 5. Adoption Effort Estimate

| Task | Effort | Notes |
|------|--------|-------|
| Add `wasmtime` + `wasmtime-wasi` deps to `nom-compose/Cargo.toml` | 1 hr | Straightforward; pick compatible versions (current stable ~29.x). |
| Wrap `Engine` / `Store<NomHost>` / `Linker<NomHost>` lifecycle | 1–2 days | Boilerplate plus error handling. Need to decide if `Engine` is global (recommended) or per-sandbox. |
| Implement `ResourceLimiter` for memory/table caps | 1 day | Simple trait impl; tie limits to Nom’s plugin quota system. |
| Wire fuel or epoch-based execution limits | 1 day | Fuel is easier to reason about; epochs are better for async multi-tenant scenarios. |
| Integrate WASI (`WasiCtxBuilder`, pre-opens, stdio) | 1–2 days | Depends on how much filesystem/network access Nom plugins need. Start with `p1` (stable); migrate to `p2` if Component Model is desired. |
| Design & implement Nom host-function ABI | 2–3 days | The hardest part: defining a stable import namespace, linear-memory serialization protocol, and handle types for Nom graph objects. |
| Tests (unit + integration with sample `.wasm` plugins) | 2 days | Need test WASM modules (Rust `wasm32-wasi` target is easiest). |
| **Total rough estimate** | **1.5–2 weeks** for a minimal working sandbox; **3–4 weeks** for production-hardened multi-tenant plugin farm with async support and component-model compatibility. |

### Risk flags
- **HIGH:** Do not attempt to reimplement memory management or trampolines. Rely on `wasmtime`’s safe APIs (`Memory::read/write`, `Linker::func_wrap`). Raw pointer games violate Wasmtime’s safety invariants.
- **MEDIUM:** Store is not GC’d internally. Long-running Nom processes that create many plugin instances must drop the `Store` (or use a fresh `Store` per plugin) to avoid unbounded memory growth.
- **LOW:** Version lock-in. Wasmtime serializes its crate version into AOT artifacts; upgrading Wasmtime requires recompiling plugins.

---

*End of audit.*
