# OpenDAL Pattern Audit -- Nom Storage Mapping

**Date:** 2026-04-19
**Repo:** `C:\Users\trngh\Documents\APP\Accelworld\upstreams\opendal`
**Focus:** Operator trait, service adapters, middleware layers, raw operation traits
**Nom Target:** `nom-canvas/crates/nom-compose/src/storage.rs` (`UniversalStorage`)

---

## 1. Pattern Summary

OpenDAL uses a **three-tier storage abstraction**:

1. **User API -- `Operator`**
   `Operator` (in `core/core/src/types/operator/operator.rs`) is the public, `Clone + Send + Sync` handle. It holds a type-erased `Accessor` (dyn) and delegates all calls (`read`, `write`, `list`, `stat`, etc.) to it. `OperatorBuilder<A: Access>` (in `core/core/src/types/operator/builder.rs`) constructs an `Operator` from a service builder and allows stacking layers before `finish()`.

2. **Backend Contract -- `Access` trait**
   `Access` (in `core/core/src/raw/accessor.rs`) is the single trait every backend must implement. It uses **RPITIT** (`impl Future<Output = ...>`) and declares four associated types:
   - `type Reader: oio::Read;`
   - `type Writer: oio::Write;`
   - `type Lister: oio::List;`
   - `type Deleter: oio::Delete;`
   Methods (`read`, `write`, `list`, `delete`, `stat`, `copy`, `rename`, `presign`, `create_dir`) have default bodies returning `ErrorKind::Unsupported`, so backends only implement what they support. Capabilities are advertised via `Arc<AccessorInfo>`.

3. **Middleware -- `Layer` + `LayeredAccess`**
   `Layer<A: Access>` (in `core/core/src/raw/layer.rs`) defines `fn layer(&self, inner: A) -> Self::LayeredAccess`.
   `LayeredAccess` mirrors `Access` but adds `fn inner(&self) -> &Self::Inner` for delegation. Every layer crate provides:
   - a public config struct (e.g., `RetryLayer`, `LoggingLayer`) implementing `Layer<A>`;
   - a hidden accessor struct (e.g., `RetryAccessor<A, I>`, `LoggingAccessor<A, I>`) implementing `LayeredAccess` (and therefore `Access` via a blanket impl).
   This gives **zero-cost composition** at the type level; layers are unwrapped by monomorphization at `finish()`.

---

## 2. Key Source Files

| Component | File Path | Role |
|-----------|-----------|------|
| **Operator** | `core/core/src/types/operator/operator.rs` | Public `Operator` struct (dyn `Accessor` wrapper). |
| **OperatorBuilder** | `core/core/src/types/operator/builder.rs` | `OperatorBuilder<A: Access>` with `.layer(L)` and `.finish()`. |
| **Access trait** | `core/core/src/raw/accessor.rs` | Core backend contract (`Access`, `AccessDyn`, `AccessorInfo`). |
| **Layer trait** | `core/core/src/raw/layer.rs` | `Layer<A: Access>` and `LayeredAccess` traits; blanket `impl<L: LayeredAccess> Access for L`. |
| **Operations** | `core/core/src/raw/ops.rs` | Strongly-typed operation args: `OpRead`, `OpWrite`, `OpList`, `OpDelete`, `OpStat`, `OpPresign`, `OpCopy`, `OpRename`, `OpCreateDir`. |
| **S3 Builder** | `core/services/s3/src/backend.rs` | `S3Builder` (public alias `S3`) implements `Builder`; configures `S3Config`, endpoints, credentials. |
| **S3 Core** | `core/services/s3/src/core.rs` | `S3Core` holds `Arc<AccessorInfo>`, signer, HTTP client; low-level request signing & dispatch. |
| **S3 Backend** | `core/services/s3/src/backend.rs` (impl `Access`) | `S3Backend` implements `Access` with `S3Writer`, `S3ListerV1/V2`, `S3Deleter`. |
| **FS Builder** | `core/services/fs/src/backend.rs` | `FsBuilder` (public alias `Fs`) implements `Builder`; canonicalizes root, sets `Capability`. |
| **FS Backend** | `core/services/fs/src/backend.rs` | `FsBackend` implements `Access` using `tokio::fs` file handles (`FsReader`, `FsWriter`, `FsLister`). |
| **Retry Layer** | `core/layers/retry/src/lib.rs` | `RetryLayer<I>` implements `Layer<A>`; `RetryAccessor` wraps `backon::ExponentialBuilder` and retries on `Error::is_temporary()`. |
| **Logging Layer** | `core/layers/logging/src/lib.rs` | `LoggingLayer<I>` implements `Layer<A>`; `LoggingAccessor` logs every operation start/finish/fail via `LoggingInterceptor`. |
| **GCS Backend** | `core/services/gcs/src/backend.rs` / `core.rs` | Same pattern as S3: `GcsBuilder` -> `GcsCore` -> `GcsBackend` implementing `Access`. |

---

## 3. Nom Mapping

**Current state:** `nom-canvas/crates/nom-compose/src/storage.rs` already wraps OpenDAL directly:

```rust
pub struct UniversalStorage {
    operator: Operator,
}
```

- **Builders used:** `Fs::default().root(path)`, `S3::default().bucket(b).region(r)`, `Gcs::default().bucket(b)`, `Azblob::default().account_name(a).container(c)`.
- **API surface:** `read`, `write`, `list` -- all via `operator.blocking()`.
- **No layers:** `UniversalStorage` does not apply any `Layer` (no retry, no logging, no metrics).

**How Nom maps to OpenDAL patterns:**

| Nom Concept | OpenDAL Equivalent | Gap / Opportunity |
|-------------|--------------------|-------------------|
| `UniversalStorage` | `Operator` | Nom already owns the wrapper. Could expose `Operator` directly or add layer composition. |
| Builder constructors (`from_local`, `from_s3`, ...) | `Operator::new(builder)` | Could be replaced with a single `from_config` or URI-based init (`Operator::from_uri`) for runtime dispatch. |
| Blocking API | `operator.blocking()` | Currently used. If Nom moves to async, the `Access` trait futures are ready. |
| Capability discovery | `op.info().native_capability()` | Not used. Nom could query `Capability` to disable unsupported operations (e.g., `copy`, `rename`) per backend. |
| Retry / logging | `RetryLayer`, `LoggingLayer` | **Missing.** Adding `.layer(RetryLayer::default())` and `.layer(LoggingLayer::default())` before `.finish()` would give resilience and observability for free. |
| Streaming reads | `Reader` / `AsyncRead` | Nom currently buffers full files (`Vec<u8>`). Could switch to `operator.reader(path)` for large-object streaming. |

**Practical next step for Nom:**
1. Import `opendal_layer_retry` and `opendal_layer_logging`.
2. Update `UniversalStorage::from_*` methods to chain layers before `finish()`.
3. Optionally expose `Operator::from_uri` so backends are selected by URI string rather than hard-coded builder methods.

---

## 4. Licensing / Complexity Notes

### Licensing
- **Apache-2.0** (confirmed in repo root `LICENSE`).
  Safe for Nom to depend on or copy small trait definitions with attribution. Full service adapters are large enough that vendoring is discouraged; dependency is the pragmatic path.

### Complexity Observations

| Risk | Detail |
|------|--------|
| **Feature-gate bloat** | `core/core/src/lib.rs` conditionally compiles `blocking`, `docs`, and each service/layer is its own crate with its own `Cargo.toml`. A full workspace build pulls in 60+ service crates and 20+ layer crates. |
| **Deep generic stacks** | `LayeredAccess` + `Access` blanket impl + associated types (`Reader`, `Writer`, `Lister`, `Deleter`) produce long monomorphized types. Error messages can be verbose. |
| **Async trait overhead (minimal)** | OpenDAL uses native RPITIT (`impl Future` in trait) rather than `async-trait`, so there is no extra boxing at the trait boundary. However, the `AccessDyn` trait exists for dynamic dispatch when needed. |
| **Service impl size** | S3 backend (`backend.rs` 1,277 lines, `core.rs` 1,749 lines) and retry layer (`lib.rs` 945 lines) are non-trivial. Re-implementing even one service correctly is weeks of work. |
| **Capability matrix** | Every backend must correctly set `Capability` flags (e.g., `write_can_append`, `list_with_recursive`, `copy`, `rename`). Mismatching these causes subtle runtime bugs because the `Operator` front-end trusts the backend's advertised capabilities. |

---

## 5. Adoption Effort Estimate

| Scenario | Effort | Rationale |
|----------|--------|-----------|
| **A. Extend existing `UniversalStorage` with layers (retry, logging)** | **0.5 - 1 day** | Add crate deps, call `.layer(...)` before `.finish()`, run existing tests. |
| **B. Add a new OpenDAL-supported backend (e.g., Azure Datalake, HDFS)** | **0.5 day** | Only requires a new `from_*` constructor; backend logic lives in OpenDAL. |
| **C. Replace OpenDAL with a custom `Access`-like trait + local/fs backend only** | **2 - 3 weeks** | Must re-create `Access`, `Layer`, `Capability`, `AccessorInfo`, `Op*`, and at least one backend. Blocking/async dual API doubles the work. |
| **D. Replicate OpenDAL's full service matrix (S3, GCS, Azure, HDFS, ...)** | **6 - 12 months** | Each service has auth, endpoint discovery, error parsing, and streaming semantics. Not viable for Nom's timeline. |

**Recommendation:** Nom should **continue depending on OpenDAL** and adopt its layer composition pattern rather than re-implementing it. The immediate win is adding `RetryLayer` and `LoggingLayer` to `UniversalStorage` for production-grade resilience and observability.
