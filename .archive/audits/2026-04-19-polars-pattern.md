# Polars Pattern Audit — Nom Data-Query Layer

**Date:** 2026-04-19
**Source:** C:\Users\trngh\Documents\APP\Accelworld\upstreams\polars (main branch)
**Scope:** polars-lazy, polars-plan, polars-core, polars-io
**Analyst:** Pattern-extraction sub-agent

---

## 1. Pattern Summary

Polars implements a **three-stage query pipeline**: (1) a fluent DSL / user-facing builder, (2) an arena-allocated intermediate representation (IR) that is optimized via rewrite rules, and (3) a physical executor. The key insight for Nom is that Polars does *not* execute eagerly; every method on `LazyFrame` mutates a logical plan (`DslPlan`) until `collect()` triggers optimization and physical planning.

| Stage | Crate | Key Abstraction | What it does |
|-------|-------|-----------------|--------------|
| **DSL / Builder** | polars-lazy | `LazyFrame`, `DslBuilder` | Chains operations (`filter`, `select`, `group_by`, `join`) into a `DslPlan`. |
| **IR & Optimize** | polars-plan | `IR`, `Arena<IR>`, `Arena<AExpr>` | Lowers `DslPlan` → `IR` (arena-based DAG). Runs `StackOptimizer` with pluggable `OptimizationRule`s. |
| **Execute** | polars-lazy + polars-mem-engine | `Executor`, `create_physical_plan` | Converts optimized IR into a physical plan and executes it (in-memory or streaming). |
| **Expression system** | polars-plan + polars-core | `Expr` (DSL), `AExpr` (IR) | Lazy expressions reference columns, literals, and functions. Types are resolved during IR conversion. |
| **Universal I/O** | polars-io | `CsvReadOptions`, `ParquetOptions`, `LazyCsvReader`, `LazyFileListReader` | File scanners return a `LazyFrame` directly; predicates and projections are pushed down into the reader via `ScanIOPredicate`. |

**Critical patterns observed:**

* **Deferred execution via plan mutation.** `LazyFrame` (in `polars-lazy/src/frame/mod.rs`) holds a `DslPlan` and an `OptFlags` bitset. Each transform returns `Self::from_logical_plan(lp, opt_state)` — no data is touched.
* **Arena-based IR.** `polars-plan/src/plans/ir/mod.rs` defines the `IR` enum (variants: `Scan`, `Filter`, `Select`, `GroupBy`, `Join`, `Sort`, `Slice`, `Sink`, etc.). Nodes are `usize` indices into `Arena<IR>`; expressions are indices into `Arena<AExpr>`. This makes tree rewriting cheap (no recursive `Arc` clones).
* **Pluggable optimizer rules.** `polars-plan/src/plans/optimizer/mod.rs` constructs a `Vec<Box<dyn OptimizationRule>>` and runs `StackOptimizer::optimize_loop`. Built-in rules include `PredicatePushDown`, `ProjectionPushDown`, `SlicePushDown`, `SimplifyExprRule`, `TypeCoercionRule`, `CountStar`, `DelayRechunk`, and `FlattenUnionRule`.
* **Pushdown abstraction at the I/O boundary.** `polars-io/src/predicates.rs` defines `PhysicalIoExpr` (evaluate a predicate against a micro-batch), `ScanIOPredicate` (bundles predicate + live columns + skip-batch predicate), and `SkipBatchPredicate` (statistics-driven pruning). Parquet and IPC readers implement these to skip row groups / files before decompression.
* **Unified scan args.** All lazy file readers (`LazyCsvReader`, `scan_parquet`, etc.) funnel into `DslPlan::Scan` with a `Box<FileScanDsl>` variant and a `UnifiedScanArgs` struct that carries schema, projection, row index, pre-slice, and cloud options.

---

## 2. Key Source Files

| File | Role | Key Names |
|------|------|-----------|
| `crates/polars-lazy/src/frame/mod.rs` | LazyFrame API, `collect()` entry | `LazyFrame`, `IntoLazy`, `OptFlags`, `collect_with_engine`, `to_alp_optimized` |
| `crates/polars-lazy/src/scan/csv.rs` | Lazy CSV scanner | `LazyCsvReader`, `LazyFileListReader`, `finish()` → `DslBuilder::scan_csv` |
| `crates/polars-plan/src/dsl/builder_dsl.rs` | DSL plan builder | `DslBuilder`, `DslPlan`, `scan_parquet`, `scan_csv`, `filter`, `project`, `group_by`, `join` |
| `crates/polars-plan/src/plans/ir/mod.rs` | Optimized IR definition | `IR` enum, `IRPlan`, `IRPlanRef`, `Node`, `Arena<IR>`, `Arena<AExpr>` |
| `crates/polars-plan/src/plans/optimizer/mod.rs` | Optimization driver | `optimize()`, `StackOptimizer`, `OptimizationRule`, `run_projection_predicate_pushdown` |
| `crates/polars-plan/src/plans/optimizer/predicate_pushdown/mod.rs` | Predicate pushdown rule | `PredicatePushDown::optimize` |
| `crates/polars-plan/src/plans/optimizer/projection_pushdown/mod.rs` | Projection pushdown rule | `ProjectionPushDown::optimize` |
| `crates/polars-io/src/predicates.rs` | I/O pushdown traits | `PhysicalIoExpr`, `ScanIOPredicate`, `SkipBatchPredicate`, `ColumnPredicateExpr`, `ColumnStats` |
| `crates/polars-io/src/csv/read/mod.rs` | CSV reader | `CsvReadOptions`, `CsvReader` |
| `crates/polars-io/src/parquet/read/mod.rs` | Parquet reader | `ParquetOptions`, `ParquetReader`, `ParquetColumnExpr` |
| `crates/polars-core/src/lib.rs` | Thread pool & core types | `POOL`, `THREAD_POOL`, `DataFrame`, `Series` |

---

## 3. Nom Mapping

### Current Nom artifacts

* **`nom-compose/src/data_query.rs`** — `NomQueryFrame`
  Already wraps Polars `LazyFrame` directly. It provides builder methods (`filter`, `select`, `group_by` + `aggregate`, `sort`, `limit`, `join`) and `collect()`. Data sources are abstracted via the `DataSource` enum (`Sqlite`, `Csv`, `Json`, `Parquet`, `Memory`). This file is **already** using the Polars lazy pattern, but it does so *outside* Nom's own trait boundary.

* **`nom-compose/src/engine.rs`** — `QueryEngine` trait
  Defines a swappable engine:
  ```rust
  pub trait QueryEngine: Send + Sync {
      fn plan(&self, sql: &str) -> Result<Box<dyn QueryPlan>, String>;
      fn execute(&self, plan: &dyn QueryPlan) -> Result<QueryOutput, String>;
  }
  pub trait QueryPlan: Send + Sync {
      fn explain(&self) -> String;
  }
  pub enum QueryOutput { DataFrame(DataFrame), Rows(Vec<Vec<Value>>), Scalar(Value) }
  ```
  The `PolarsEngine` implementation is currently a **stub** — it ignores the plan and returns a hard-coded one-row DataFrame. The `SqliteEngine` is fully functional and demonstrates the intended abstraction.

* **`nom-compose/src/backends/data_query.rs`** — `DataQuerySpec`
  A semantic-model SQL generator. It builds `SELECT ... FROM ... WHERE ... LIMIT` strings from a registry and writes them to an `ArtifactStore`. It is entirely SQL-string-oriented and does not interact with `NomQueryFrame` or `QueryEngine`.

### Mapping & gaps

| Polars Pattern | Nom Equivalent | Gap / Action |
|----------------|----------------|--------------|
| `LazyFrame` (DSL builder) | `NomQueryFrame` | **Low gap.** `NomQueryFrame` already mirrors this, but it leaks Polars types directly instead of going through `QueryEngine`. |
| `DslPlan` / `IR` arena | *None* | **High gap.** Nom has no internal IR or arena. It relies on Polars to do all planning. If Nom wants its own optimizer (e.g., semantic-model-aware rewrites), it would need a custom plan tree. |
| `OptimizationRule` + `StackOptimizer` | *None* | **High gap.** Nom cannot inject custom rewrite rules into Polars. To do so, Nom would need its own IR layer that lowers into Polars IR only at execution time. |
| `collect_with_engine` | `QueryEngine::execute` | **Medium gap.** `PolarsEngine` should be upgraded to: (1) accept a plan, (2) convert it to `DslPlan` / `IRPlan`, (3) call `to_alp_optimized()` or `optimize()`, (4) run `create_physical_plan` and execute, (5) wrap the result in `QueryOutput::DataFrame`. |
| `ScanIOPredicate` / `PhysicalIoExpr` | *None* | **Medium gap.** Nom's I/O layer does not expose pushdown predicates to file scanners. If Nom introduces its own storage backends (e.g., remote semantic caches), it will need an equivalent trait boundary. |
| `LazyCsvReader` / `scan_parquet` | `NomQueryFrame::from_csv`, `from_parquet` | **Low gap.** These are already thin wrappers. They could be unified behind a `DataSourceResolver` trait to make testing easier. |
| Polars SQLContext | `NomQueryFrame::filter` | **Medium gap.** `filter` currently uses `SQLContext` to parse a SQL string into a `LazyFrame`. This is convenient but opaque — Nom cannot inspect or optimize the resulting plan. A native expression builder (or a Nom IR → Polars IR translator) would give back control. |

### Recommended integration path

1. **Make `PolarsEngine` real.** Implement `plan()` by parsing the input SQL (or Nom DSL) into a Polars `DslPlan`, wrap it in a newtype that implements `QueryPlan`, and store the `DslPlan` / `OptFlags`. Implement `execute()` by calling `LazyFrame::from_logical_plan(...).to_alp_optimized()` → `create_physical_plan` → `execute()` → `QueryOutput::DataFrame`.
2. **Unify `DataQuerySpec` with `QueryEngine`.** Instead of emitting a raw SQL string, `DataQuerySpec::compose` should produce a `Box<dyn QueryPlan>` (via `PolarsEngine::plan` or `SqliteEngine::plan`) so that Nom's semantic layer is engine-agnostic.
3. **Defer custom IR.** Do not build a Nom-native IR until semantic-model optimizations (e.g., model join reordering, cache-aware pushdown) are required. Until then, rely on Polars' proven optimizer and treat `IRPlan::describe()` as the `QueryPlan::explain()` implementation.

---

## 4. Licensing / Complexity Notes

### Licensing

* **Root repository:** MIT License (Copyright 2025 Ritchie Vink; portions Copyright 2024 NVIDIA CORPORATION & AFFILIATES).
* **Sub-crate exceptions:**
  * `crates/polars-arrow/LICENSE` contains Apache-2.0 code (derived from Apache Arrow).
  * `crates/polars-parquet/LICENSE` contains Apache-2.0 code (derived from Apache Parquet).
* **Nom implication:** Because Nom links against the `polars` meta-crate (which pulls in `polars-arrow` and `polars-parquet`), the distributed binary is subject to **both MIT and Apache-2.0** terms. Apache-2.0 is compatible with Nom's own licensing, but attribution notices must be preserved in any shipped artifact.

### Complexity / Compile-time risks

* **Massive dependency tree.** Polars consists of ~30+ workspace crates plus deep transitive dependencies (`arrow`, `parquet2`, `rayon`, `hashbrown`, `simd-json`, etc.).
* **Feature-flag brittleness.** Enabling `parquet`, `csv`, `json`, `temporal`, `dtype-decimal`, etc. changes which modules compile and which optimizer rules are active. A minimal Nom build should gate Polars features aggressively (e.g., `polars = { version = "...", default-features = false, features = ["lazy", "csv", "parquet", "json"] }`).
* **Compile-time bloat.** Even a trimmed Polars dependency adds tens of seconds to clean release builds. On Windows with MSVC, `polars-core`'s monomorphized `ChunkedArray` kernels and `rayon` thread-pool initialization noticeably increase link times.
* **Upgrade hazard.** Polars releases are frequent and breaking. The `LazyFrame` API, feature flags, and internal IR variants change between minor versions. Pinning an exact version in `Cargo.toml` is recommended.
* **`unsafe` surface.** `polars-core` uses `unsafe` for zero-copy slicing, bitmap operations, and chunked array casts. This is generally well-audited but increases the trust boundary Nom inherits.

---

## 5. Adoption Effort Estimate

| Level | Work | Timeline | Risk |
|-------|------|----------|------|
| **A — Keep current wrapper** | Continue using `NomQueryFrame` as a thin `LazyFrame` wrapper. No changes to `QueryEngine`. | **Hours** | Low risk, but Nom remains tightly coupled to Polars public API and cannot swap engines transparently. |
| **B — Real `PolarsEngine`** | Implement `QueryEngine`/`QueryPlan` for real: parse SQL → `DslPlan` → optimize → execute. Wire `DataQuerySpec` to emit plans instead of raw SQL strings. | **1–2 weeks** | Medium risk. Requires understanding `polars-plan` IR conversion (`to_alp`) and physical plan creation (currently in `polars-mem-engine`). |
| **C — Custom Nom IR + optimizer** | Introduce a Nom-native logical plan tree and optimizer rules, then lower to Polars IR only at the bottom. This mirrors Polars' own architecture but gives Nom full control over semantic-model rewrites. | **1–2 months** | High risk. Re-inventing a query optimizer is error-prone; only justified if Nom needs domain-specific rules (e.g., automatic model federation, cost-based cache routing). |

### Bottom-line recommendation

**Adopt Level B.** It preserves the `QueryEngine` abstraction (keeping the SQLite fallback viable), leverages Polars' mature optimizer and execution engine, and adds only ~1–2 weeks of integration work. Level C should be deferred until Nom has concrete, measured requirements for optimizer rules that Polars cannot express.
