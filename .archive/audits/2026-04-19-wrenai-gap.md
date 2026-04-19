# Gap Audit: WrenAI Semantic Layer → Nom `semantic.rs`

**Auditor:** Gap-analysis auditor (subagent)  
**Date:** 2026-04-19  
**WrenAI ref:** `C:\Users\trngh\Documents\APP\Accelworld\upstreams\wrenai\`  
**Nom target:** `nom-canvas/crates/nom-compose/src/semantic.rs`  
**Status:** PARTIAL — `semantic.rs` stubbed (confirmed by `maria-hill-magik-sif.md` §4.1)

---

## 1. Pattern Summary

WrenAI implements a **full Semantic Layer** through its **Model Definition Language (MDL)**. The architecture decouples user-facing semantic models from physical database schemas via:

1. **Manifest / MDL** — A JSON-schema-governed document (`wren-mdl/mdl.schema.json`) that defines `models`, `relationships`, `views`, `metrics`, and `enumDefinitions`.
2. **Physical-to-Logical Mapping** — Each `ModelMDL` points to a physical table via `tableReference` (`catalog`/`schema`/`table`) or a SQL expression via `refSql`, while exposing a clean business name (`name`) and display metadata (`properties.displayName`, `properties.description`).
3. **Semantic Query Rewriting** — Natural language questions are converted to SQL by LLM pipelines that are fed **DDL-derived schema context** (not raw DB catalogs). The engine then validates generated SQL against the MDL via `dry-run` / `dry-plan` / `preview` endpoints.
4. **Index + Retrieve Pipeline** — MDL documents are chunked, embedded, and stored in a vector document store (`qdrant`) for retrieval-augmented SQL generation.
5. **Engine Abstraction** — Three engine backends (`WrenEngine`, `WrenIbis`, `WrenUI`) execute SQL against the semantic model, abstracting the underlying datasource (DuckDB, Postgres, BigQuery, Snowflake, etc.).

Nom's `semantic.rs` contains only a **structural stub**: `SemanticColumn`, `SemanticDataType`, `SemanticModel`, and `SemanticRegistry`. It can generate trivial `SELECT … FROM` strings but has **zero** relationship awareness, **zero** calculated-field support, **zero** query-rewriting infrastructure, and **zero** engine abstraction.

---

## 2. Key Source Files (WrenAI)

| File | Role |
|------|------|
| `wren-mdl/mdl.schema.json` | JSON Schema for the full MDL Manifest (models, relationships, views, metrics, enums, column-level access control, session properties). |
| `wren-ui/src/apollo/server/mdl/mdlBuilder.ts` | `MDLBuilder` class — builds a `Manifest` from UI `Model`, `ModelColumn`, `RelationInfo`, and `View` entities. Handles `tableReference`, `refSql`, calculated fields, relation columns, and view statements. |
| `wren-ui/src/apollo/server/mdl/type.ts` | TypeScript interfaces: `Manifest`, `ModelMDL`, `ColumnMDL`, `RelationMDL`, `ViewMDL`, `TableReference`, `WrenEngineDataSourceType`. |
| `wren-ui/src/apollo/server/adaptors/wrenEngineAdaptor.ts` | `WrenEngineAdaptor` — HTTP client for the Rust wren-engine. Exposes `previewData(sql, mdl)`, `dryRun(sql, manifest)`, `getNativeSQL(sql, manifest)` (dry-plan), and `validateColumnIsValid`. |
| `wren-ai-service/src/providers/engine/wren.py` | Engine provider implementations: `WrenUI`, `WrenIbis`, `WrenEngine`. Each implements `execute_sql()`, `dry_plan()`, `get_func_list()`, `get_sql_knowledge()`. |
| `wren-ai-service/src/pipelines/indexing/db_schema.py` | `DDLChunker` — converts MDL `models`/`relationships`/`views`/`metrics` into faux DDL documents for vector indexing. Handles column batching, foreign-key constraints, calculated-field comments, metric dimensions/measures. |
| `wren-ai-service/src/pipelines/indexing/table_description.py` | `TableDescriptionChunker` — creates searchable `TABLE_DESCRIPTION` documents from MDL models/metrics/views for semantic retrieval. |
| `wren-ai-service/src/pipelines/indexing/project_meta.py` | `ProjectMeta` — indexes `data_source` and `project_id` metadata. |
| `wren-ai-service/src/pipelines/retrieval/db_schema_retrieval.py` | `DbSchemaRetrieval` — embeds the user query, retrieves relevant tables via vector search, then uses an LLM to prune columns per table before returning structured DDL context. |
| `wren-ai-service/src/pipelines/generation/sql_generation.py` | `SQLGeneration` — LLM pipeline that consumes retrieved schema context, calculated-field instructions, metric instructions, JSON-field instructions, SQL samples, and user instructions to generate ANSI SQL. |
| `wren-ai-service/src/pipelines/generation/utils/sql.py` | `SQLGenPostProcessor` — validates generated SQL via `engine.execute_sql()` dry-run or `engine.dry_plan()`. Contains `_DEFAULT_CALCULATED_FIELD_INSTRUCTIONS`, `_DEFAULT_METRIC_INSTRUCTIONS`, `_DEFAULT_JSON_FIELD_INSTRUCTIONS`. |
| `wren-ai-service/src/pipelines/common.py` | `build_table_ddl()` — builds `CREATE TABLE` DDL strings from MDL schema documents for LLM context windows. |
| `wren-ai-service/src/web/v1/services/ask.py` | `AskService` — orchestrates the full end-to-end flow: intent classification → historical question retrieval → schema retrieval → SQL generation → SQL diagnosis → correction loop. |

---

## 3. Gap Analysis

### 3.1 Missing: Manifest / MDL Schema

**WrenAI:** The `Manifest` (defined in `mdl.schema.json` and `type.ts`) is the central contract. It contains:
- `catalog`, `schema`, `dataSource`
- `models` — each with `name`, `tableReference` or `refSql`, `columns`, `primaryKey`, `properties`
- `relationships` — `name`, `models[2]`, `joinType`, `condition`, `properties`
- `views` — `name`, `statement`, `properties`
- `metrics` — `name`, `baseObject`, `dimension[]`, `measure[]`, `timeGrain[]`
- `enumDefinitions` — `name`, `values[]`

**Nom:** `SemanticRegistry` is a `Vec<SemanticModel>`. There is no manifest container, no datasource metadata, no relationships array, no views, no metrics, no enums.

**Gap severity:** CRITICAL — without a manifest, there is no single document that the query engine can consume.

### 3.2 Missing: Physical-to-Logical Decoupling

**WrenAI:** `ModelMDL` decouples via two mechanisms:
1. `tableReference: { catalog?, schema?, table }` — maps a business model name to a physical table.
2. `refSql` — maps a business model name to an arbitrary SQL subquery.

In `mdlBuilder.ts`, `buildTableReference()` extracts `catalog`/`schema`/`table` from model properties, and `getColumnExpression()` returns `"sourceColumnName"` when the logical column name differs from the physical one.

**Nom:** `SemanticModel` has `source_table: String` — a single bare string. No `catalog`, no `schema`, no `refSql`, no expression remapping.

**Gap severity:** HIGH — Nom cannot handle multi-catalog setups or logical renaming.

### 3.3 Missing: Relationships & Join Generation

**WrenAI:** `RelationMDL` defines `joinType` (`ONE_TO_ONE`, `ONE_TO_MANY`, `MANY_TO_ONE`, `MANY_TO_MANY`) and `condition` (e.g. `"OrdersModel.custkey = CustomerModel.custkey"`). In `mdlBuilder.ts`, `addRelation()` injects **relation columns** into both sides of the relationship so that the LLM sees foreign keys as ordinary columns. In `db_schema.py`, `_relationship_command()` emits `FOREIGN KEY … REFERENCES …` DDL for context.

**Nom:** Zero relationship support. `SemanticModel` has no relation list, and `to_select_sql()` only emits single-table `SELECT`.

**Gap severity:** CRITICAL — cross-model questions (the primary value of a semantic layer) are impossible.

### 3.4 Missing: Calculated Fields

**WrenAI:** `ColumnMDL.isCalculated` plus `expression` (e.g. `"SUM(orders.totalprice)"`). `mdlBuilder.ts` `addCalculatedField()` adds these columns with their expressions. `db_schema.py` prefixes calculated columns with `-- This column is a Calculated Field\n-- column expression: …` in DDL. `sql.py` injects `_DEFAULT_CALCULATED_FIELD_INSTRUCTIONS` into the LLM prompt.

**Nom:** `SemanticColumn` has `name`, `data_type`, `description`. No `is_calculated`, no `expression`, no `not_null`.

**Gap severity:** HIGH — business metrics defined in the semantic layer cannot be generated.

### 3.5 Missing: Metrics (OLAP Cube Abstraction)

**WrenAI:** `MetricMDL` has `baseObject`, `dimension[]`, `measure[]`, `timeGrain[]`. In `db_schema.py`, `_convert_metrics()` renders metrics as virtual `CREATE TABLE` DDL with dimension and measure columns. `sql.py` injects `_DEFAULT_METRIC_INSTRUCTIONS` teaching the LLM to query metrics directly.

**Nom:** No metric concept exists.

**Gap severity:** MEDIUM-HIGH — pre-aggregated business objects are a core WrenAI pattern.

### 3.6 Missing: Views

**WrenAI:** `ViewMDL` has `name` and `statement` (SQL). `mdlBuilder.ts` `addView()` adds them to the manifest. `db_schema.py` `_convert_views()` emits `CREATE VIEW … AS …` DDL.

**Nom:** No view concept exists.

**Gap severity:** MEDIUM — views simplify complex queries; their absence limits composability.

### 3.7 Missing: Semantic Query Rewriting Pipeline

**WrenAI:** A full multi-stage pipeline:
1. **Indexing** — `DBSchema` pipeline (`db_schema.py`) chunks MDL into DDL documents and embeds them.
2. **Retrieval** — `DbSchemaRetrieval` (`db_schema_retrieval.py`) embeds the user question, retrieves relevant tables via vector search, then uses an LLM (`table_columns_selection_generator`) to prune columns per table.
3. **Generation** — `SQLGeneration` (`sql_generation.py`) feeds schema DDL + calculated-field instructions + metric instructions + JSON instructions + SQL samples + user instructions into an LLM to produce ANSI SQL.
4. **Post-processing** — `SQLGenPostProcessor` (`sql.py`) validates the SQL via `engine.dry_run()` or `engine.dry_plan()`.
5. **Correction loop** — `AskService` (`ask.py`) retries up to `max_sql_correction_retries` if validation fails.

**Nom:** `to_select_sql()` is the only SQL generation. It does `SELECT cols FROM source_table`. No LLM integration, no retrieval, no validation, no correction.

**Gap severity:** CRITICAL — the entire natural-language → SQL value proposition is absent.

### 3.8 Missing: Engine Abstraction

**WrenAI:** Three engine providers in `wren.py`:
- `WrenEngine` — talks to Rust wren-engine (`/v1/mdl/dry-run`, `/v1/mdl/preview`).
- `WrenIbis` — talks to Python Ibis connector (`/v3/connector/{source}/query`, `/dry-plan`).
- `WrenUI` — talks to Wren UI GraphQL preview endpoint.

All implement `execute_sql()`, `dry_plan()`, and datasource-specific knowledge retrieval.

**Nom:** No engine trait/interface. `data_query.rs` (backend consumer) calls `to_select_sql()` directly and does its own identifier safety check (`is_safe_identifier`). There is no dry-run, no preview, no datasource abstraction.

**Gap severity:** HIGH — without an engine, generated SQL cannot be validated or executed.

### 3.9 Missing: Column-Level Metadata & Access Control

**WrenAI:** `ColumnMDL` carries `notNull`, `relationship`, `properties.description`, `properties.displayName`, `columnLevelAccessControl` (operator, threshold, session properties). `mdlBuilder.ts` maps all of these from the UI model.

**Nom:** `SemanticColumn` has `description: Option<String>`. No `not_null`, no `relationship`, no `properties`, no access control.

**Gap severity:** MEDIUM — missing metadata degrades LLM context quality.

### 3.10 Missing: DataSource Typing

**WrenAI:** `WrenEngineDataSourceType` enum covers 13+ sources (`BIGQUERY`, `POSTGRES`, `SNOWFLAKE`, `DUCKDB`, `MYSQL`, `CLICKHOUSE`, `TRINO`, `REDSHIFT`, `DATABRICKS`, `ATHENA`, `MSSQL`, `ORACLE`, `DATAFUSION`). The datasource drives SQL dialect rules in `sql.py` and engine routing in `wren.py`.

**Nom:** `SemanticDataType` is an enum of 7 logical types (`String`, `Integer`, `Float`, `Boolean`, `Date`, `Timestamp`, `Json`). There is no datasource enum at all.

**Gap severity:** MEDIUM — without datasource awareness, SQL generation cannot apply dialect-specific rules.

---

## 4. Adoption Effort Estimate

| Component | WrenAI Reference | Nom Target | Estimated Effort | Priority |
|-----------|------------------|------------|------------------|----------|
| **Manifest schema + serde** | `mdl.schema.json`, `type.ts` | `nom-compose/src/mdl.rs` | 2–3 days | P0 |
| **Physical-to-logical mapping** | `mdlBuilder.ts` `buildTableReference()`, `getColumnExpression()` | Extend `SemanticModel` with `table_reference`, `ref_sql`, `expression` on `SemanticColumn` | 1–2 days | P0 |
| **Relationship model + join DDL** | `mdlBuilder.ts` `addRelation()`, `db_schema.py` `_relationship_command()` | `SemanticRelationship` struct, FK-injection logic | 2–3 days | P0 |
| **Calculated fields** | `mdlBuilder.ts` `addCalculatedField()`, `sql.py` instructions | `is_calculated: bool`, `expression: String` on `SemanticColumn` | 1 day | P1 |
| **Metrics abstraction** | `mdl.schema.json` metrics, `db_schema.py` `_convert_metrics()` | `SemanticMetric` struct with dimensions/measures/time_grain | 2 days | P1 |
| **Views abstraction** | `mdl.schema.json` views, `mdlBuilder.ts` `addView()` | `SemanticView` struct with `statement` | 1 day | P1 |
| **DDL context builder** | `db_schema.py` `DDLChunker`, `common.py` `build_table_ddl()` | `MdlToDdl` converter that emits `CREATE TABLE` / `CREATE VIEW` strings | 3–4 days | P0 |
| **Vector indexing pipeline** | `db_schema.py`, `table_description.py`, `project_meta.py` | Integrate with existing `nom-resolver` embedding + `qdrant` backend | 3–5 days | P1 |
| **Schema retrieval pipeline** | `db_schema_retrieval.py` | Embed query → retrieve tables → LLM column-pruning → return DDL context | 4–5 days | P1 |
| **SQL generation pipeline** | `sql_generation.py`, `sql.py` | Prompt builder + LLM generator + structured output (`{"sql": ...}`) | 4–5 days | P1 |
| **SQL validation / post-processor** | `sql.py` `SQLGenPostProcessor` | Trait-based `SqlValidator` with `dry_run()` and `dry_plan()` methods | 2–3 days | P1 |
| **Engine abstraction trait** | `wren.py` `WrenEngine`, `WrenIbis`, `core/engine.py` `Engine` | `trait Engine { async fn execute_sql(...); async fn dry_plan(...); }` | 2–3 days | P0 |
| **Ask orchestration service** | `ask.py` `AskService` | `AskOrchestrator` that chains intent → retrieval → generation → validation → correction | 5–7 days | P2 |
| **Column access control** | `mdl.schema.json` `columnLevelAccessControl` | `AccessControlRule` struct on `SemanticColumn` | 2 days | P3 |
| **Datasource enum + dialect rules** | `type.ts` `WrenEngineDataSourceType`, `sql.py` rules | `DataSource` enum, dialect-specific SQL generation hints | 2 days | P2 |

### Total Effort (Minimal Viable Semantic Layer)

To reach **parity with WrenAI's core semantic-layer functionality** (manifest, relationships, calculated fields, DDL context generation, engine trait, and basic SQL validation):

- **Rust implementation:** ~15–20 engineering days
- **LLM pipeline wiring:** ~10–15 engineering days (depends on existing `nom-resolver` / `nom-intent` infrastructure)
- **Testing + integration:** ~5–7 engineering days

**Conservative total: 5–6 weeks** for a single engineer, or **2–3 weeks** with a focused 2-person team.

### Immediate Next Steps (if adopting)

1. **Replace `semantic.rs` with `mdl.rs`** — define `Manifest`, `Model`, `Column`, `Relationship`, `View`, `Metric` structs matching WrenAI schema.
2. **Implement `MtlToDdl`** — convert manifest to DDL strings for LLM context (port `DDLChunker` logic).
3. **Add `Engine` trait** — abstract dry-run / preview / execute behind a trait with at least a DuckDB/SQLite backend.
4. **Wire into existing `nom-compose` dispatch** — replace `data_query.rs`'s direct `to_select_sql()` call with manifest-driven query generation.

---

*End of audit.*
