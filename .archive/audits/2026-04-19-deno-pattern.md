# Deno Reference Repo Pattern Audit

**Date:** 2026-04-19  
**Target:** Deno upstream at `C:\Users\trngh\Documents\APP\Accelworld\upstreams\deno`  
**Focus areas:** `runtime/`, `libs/core/`, `ext/`, `runtime/permissions/`, untrusted JS/TS sandboxing  
**Output for:** `nom-compose/src/sandbox.rs` — secure JS/TS sandbox for web compose

---

## 1. Pattern Summary

Deno’s security model is **capability-based sandboxing around a V8 isolate**. All JavaScript/TypeScript execution happens inside a `JsRuntime` (a V8 isolate wrapper). Native capabilities are exposed only through **ops** — Rust functions bound to V8 via the `#[op2]` macro. Ops are grouped into **extensions** (`deno_core::extension!`) that register both Rust op implementations and JS glue code. Every op that touches a native resource (file system, network, env, subprocess, FFI, import) delegates to a **central permission checker** (`PermissionsContainer`). The permission model is quad-state (`Granted`, `GrantedPartial`, `Prompt`, `Denied`, `DeniedPartial`, `Ignored`) and supports an out-of-process **permission broker** (`PermissionBroker`) for interactive prompting. Workers (`MainWorker`, `WebWorker`) encapsulate the runtime and carry their own `PermissionsContainer`, enabling per-isolate policy enforcement.

---

## 2. Key Source Files

| File | Role | Key Symbols |
|------|------|-------------|
| `libs/core/runtime/jsruntime.rs` | V8 isolate wrapper & event loop | `JsRuntime`, `JsRuntimeForSnapshot`, `RuntimeOptions`, `JsRuntimeState`, `ContextState` |
| `libs/core/extensions.rs` | Extension registration system | `Extension`, `ExtensionFileSource`, `ExtensionFileSourceCode`, `OpDecl`, `ExtensionBuilder` |
| `libs/core/ops.rs` | Per-op context & state | `OpCtx`, `OpState`, `OpId`, `PromiseId`, `ExternalOpsTracker` |
| `libs/ops/lib.rs` / `libs/ops/op2/` | Macro-driven V8↔Rust binding | `#[op2]` proc-macro attribute, `op2_macro()` |
| `runtime/worker.rs` | Main user-facing worker | `MainWorker`, `WorkerServiceOptions`, `create_validate_import_attributes_callback()` |
| `runtime/web_worker.rs` | Isolated Web Worker | `WebWorker`, `WebWorkerInternalHandle`, `WorkerControlEvent`, `WorkerId`, `WorkerThreadType` |
| `runtime/permissions/lib.rs` | Permission policy engine | `PermissionsContainer`, `Permissions`, `UnaryPermission<T>`, `PermissionState`, `PermissionDeniedError`, `check_read_all()`, `check_write_all()`, `check_net()`, `check_run()`, `check_env()`, `check_sys()`, `check_ffi_all()` |
| `runtime/permissions/broker.rs` | Out-of-process permission prompt | `PermissionBroker`, `maybe_check_with_broker()`, `BrokerResponse` |
| `ext/fs/lib.rs` | Example extension (fs) | `deno_core::extension!(deno_fs, ops = [op_fs_open_sync, op_fs_read_file_async, ...], esm = ["30_fs.js"], state = |state, options| { state.put(options.fs); })` |
| `ext/fs/ops.rs` | Fs op implementations | `op_fs_open_sync()`, `op_fs_read_file_async()`, `FsOpsError` |
| `libs/core/ops_builtin.rs` | Built-in safe ops | `op_print()`, `op_close()`, `op_resources()`, `op_void_sync()`, `op_void_async()` |

---

## 3. Nom Mapping

**Target file:** `nom-compose/src/sandbox.rs` (does not yet exist in `nom-canvas/crates/nom-compose/src/`).  
**Current state:** Nom’s sandbox lives in `nom-canvas-graph/src/sandbox.rs` — it is a *pure-Rust expression evaluator* (`Expr`, `SandboxValue`, `eval_expr()`, `DepthLimitSanitizer`, `AllowedFunctionsSanitizer`) with no V8 or JS/TS execution. `nom-compose/src/backends/code_exec.rs` (`CodeExecBackend`) only handles `eval:` integer prefixes.

| Deno Pattern | Nom Equivalent (`nom-compose/src/sandbox.rs`) |
|--------------|-----------------------------------------------|
| `JsRuntime` + `RuntimeOptions` | A `SandboxRuntime` struct wrapping a V8 isolate (via `deno_core` or `rusty_v8`). Accepts `SandboxRuntimeOptions { extensions, permissions, module_loader }`. |
| `Extension` + `#[op2]` | A minimal `nom_compose` extension declared with `deno_core::extension!` exposing only compose-safe ops: e.g., `op_compose_read_input()`, `op_compose_emit_output()`, `op_compose_log()`. No `deno_fs`, `deno_net`, or `deno_run` extensions loaded by default. |
| `OpState` / `OpCtx` | `SandboxOpState` holding an `Arc<Mutex<SandboxPermissions>>` and compose-specific globals (artifact store handle, progress sink). |
| `PermissionsContainer` | `SandboxPermissions` struct with `read: bool`, `write: bool`, `net: bool`, `env: bool`, `run: bool`. Default = **all denied**. Each op calls `permissions.check_read(path)?` etc. before executing. |
| `UnaryPermission<T>` + `PermissionState` | Simplified to a flat `PermissionSet { read_paths: Vec<PathBuf>, allow_net: bool, ... }` because Nom’s threat surface is smaller than Deno’s CLI. |
| `PermissionBroker` | Optional `SandboxPermissionBroker` trait for interactive compose workflows (e.g., user prompt before network fetch). Falls back to deny if no broker is set. |
| `MainWorker` / `WebWorker` | `SandboxWorker` struct owning one `JsRuntime` + one `SandboxPermissions`. Spawned per compose job. `WebWorker` isolation pattern can be reused if compose scripts run in parallel. |
| `ModuleLoader` | `SandboxModuleLoader` resolving only `nomx:` URIs and pre-approved ES modules from the artifact store. No arbitrary filesystem or HTTP imports. |

---

## 4. Licensing/Complexity Notes

- **License:** MIT (fully compatible with Nom). See `deno/LICENSE.md`.
- **V8 dependency:** `deno_core` pulls in Google V8 (C++). Build requires Python, Clang/LLVM, and adds **significant compile time** (tens of minutes on first build, gigabytes of artifacts).
- **Codebase scale:** The Deno repo is massive. `runtime/permissions/lib.rs` alone is ~10 k lines. The `ext/` directory contains 30+ extensions. Adopting `deno_core` means inheriting this maintenance surface even if most extensions are disabled.
- **Alternative engines:** If full TS/JS compatibility is not required, `rquickjs` (QuickJS) is orders of magnitude smaller and faster to build, but lacks the mature op-binding and snapshot infrastructure of `deno_core`.
- **Security provenance:** Deno’s model is battle-tested in production CLI and edge-runtime deployments. The permission quad-state logic and broker IPC are well-audited patterns.

---

## 5. Adoption Effort Estimate

| Approach | Effort | Notes |
|----------|--------|-------|
| **A. Full `deno_core` integration** | **High (4–6 weeks)** | Add `deno_core` crate to `nom-canvas/Cargo.toml`. Define `nom_compose` extension with safe ops. Re-implement `PermissionsContainer` as `SandboxPermissions`. Integrate V8 build into Nom’s CI. Heavy binary size increase. |
| **B. `rusty_v8` directly + custom ops** | **Very High (8+ weeks)** | Rebuild Deno’s op-binding and extension machinery from scratch. Not recommended. |
| **C. `rquickjs` lightweight sandbox** | **Medium (1–2 weeks)** | Swap V8 for QuickJS. Good enough for expression-like scripts. Loses TS support and Deno’s mature snapshot/inspector ecosystem. |
| **D. Keep current pure-Rust evaluator** | **Low (days)** | Extend `nom-canvas-graph/src/sandbox.rs` with more `Expr` variants. No JS/TS, but no new dependencies. |

**Recommendation:** If `nom-compose` truly needs to run **untrusted JS/TS**, Option A (`deno_core`) is the architecturally sound choice because it gives Deno’s permission model, worker isolation, and op system for “free.” However, the **build-time and binary-size tax is severe**. If the use case is limited to safe expressions and small glue scripts, Option D (extend the existing Rust evaluator) or Option C (QuickJS) is more proportional to Nom’s current scope.
