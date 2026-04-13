# 10 — External-repo upgrade plan (GitNexus-mined, 2026-04-14)

**Method**: five reference repos were indexed with `npx gitnexus analyze --skip-git` and queried via MCP Cypher + symbol context. Three subagents synthesized findings into per-crate upgrade plans. All patterns below cite the external repo's exact symbol + file path so implementers can open the source directly.

Ref | Repo | Indexed (nodes / edges / flows) | Nom target
---|---|---|---
A | `benchmark-main` (google/benchmark) | 2264 / 5146 / 190 | `nom-bench`
B | `graphify-master` | 238 / 364 / 2 — **SKIP** | — (pivoted to petgraph + GitNexus-core + rust-analyzer)
C | `wrenai` (Canner) | 5692 / 15373 / 276 | `nom-intent` (M8) + `nom-cli/src/store/resolve.rs` re-rank (M9)
D | `zed-main` | (user-driven analyze) | `nom-lsp` (M16) — see §D

Previously live in our own index: `Nom` (251 / 5947 / 300). Refs A–C map to concrete remaining-work items in doc 09. Ref B was speculatively included; the indexing itself proves it irrelevant (a React/D3 chart app, zero graph-theoretic code) — the pivot to petgraph + gitnexus-core below is the actionable outcome.

---

## A. `nom-bench` upgrade (from google/benchmark) — 3 weeks

### Patterns worth stealing

1. **Global registry via singleton** — `BenchmarkFamilies::GetInstance` at [src/benchmark_register.cc:79](../../APP/benchmark-main/src/benchmark_register.cc) holds every `BenchmarkInstance` ([include/benchmark/benchmark.h:19-77](../../APP/benchmark-main/include/benchmark/benchmark.h)); registration decoupled from execution so discovery runs before the harness.
2. **Stateful iteration loop** — `State::KeepRunning` + `State::PauseTiming`/`ResumeTiming` ([src/benchmark.cc:183-273](../../APP/benchmark-main/src/benchmark.cc)) turn the user body into a resumable coroutine where only measured regions count.
3. **Typed user counters** — `State::SetBytesProcessed` plus the `Counter` struct ([include/benchmark/counter.h](../../APP/benchmark-main/include/benchmark/counter.h)) attach semantic metrics (bytes/s, items/s, cache-miss rate) per-run, aggregated per-thread.
4. **Fixtures with SetUp/TearDown** — `Fixture::SetUp` / `Fixture::TearDown` ([test/fixture_test.cc:12-17](../../APP/benchmark-main/test/fixture_test.cc)) amortize expensive state across iterations without polluting timing.
5. **Reporter + statistics split** — `BenchmarkReporter::ReportRuns` with concrete `ConsoleReporter::PrintHeader` ([src/console_reporter.cc:61-77](../../APP/benchmark-main/src/console_reporter.cc)) + `StatisticsMean`/`StatisticsMedian` ([src/statistics.cc:34-60](../../APP/benchmark-main/src/statistics.cc)) feed pluggable console/JSON/CSV sinks via `DoOneRepetition`.

### Concrete nom-bench upgrade steps

1. **`nom-bench/src/registry.rs`** — `BenchFamilyRegistry` (use the `inventory` crate for compile-time singleton). CLI `nom bench list` enumerates it.
2. **`nom-bench/src/state.rs`** — `State::keep_running(&mut self)` iterator + `pause_timing` / `resume_timing`; extend `BenchmarkRun` with `Counters` map (`bytes_processed`, `items_processed`).
3. **`nom-bench/src/fixture.rs`** — `trait Fixture { fn setup(&mut self, _: &State); fn teardown(&mut self, _: &State); }` invoked by runner per-repetition.
4. **`nom-bench/src/runner.rs`** — `ThreadRunner` modeled on `ThreadRunnerDefault` ([src/benchmark_runner.cc:198-221](../../APP/benchmark-main/src/benchmark_runner.cc)); collects per-thread timers into existing `TimingMoments`.
5. **`nom-bench/src/reporter/{mod,console,json}.rs`** — `trait Reporter { fn report_runs(&mut self, &[BenchmarkRun]); }` + `StatisticsAggregator` computing p50/p95/p99 + mean/stddev, feeding the existing `entry_benchmarks` side-table.

### Effort

~3 weeks (1 engineer): week 1 registry+State, week 2 fixtures+threaded runner, week 3 reporters+statistics+CLI wiring. JSON round-trip tests already exist; extend with golden reporter-output tests mirroring `test/reporter_output_test.cc`.

---

## B. `nom-graph` upgrade (graphify skipped; pivoted to petgraph + GitNexus-core + rust-analyzer) — 2 days

### Why graphify was skipped

Graphify is a React/Redux + Node/Express/PostgreSQL CSV→D3 chart webapp. Zero graph-theoretic code; no structural analog to call-edge extraction or community detection.

### Better sources

- **`petgraph` (Rust crate)** — production algorithms: Tarjan SCC, Dijkstra, centrality, Louvain via `petgraph-community`. Direct drop-in for our hand-rolled adjacency `HashMap`.
- **GitNexus itself** (`gitnexus/packages/core`) — incremental graph updates, Cypher-style query, UID-stable node identity. Maps onto `NomtuGraph` 1:1.
- **rust-analyzer `ide-db`** — salsa incremental recompute; template for making `from_entries` delta-aware rather than full-rebuild.

### Concrete nom-graph upgrade steps

1. **Dep swap** — add `petgraph` to `nom-graph/Cargo.toml`; replace custom adjacency `HashMap` with `petgraph::Graph<NomtuNode, NomtuEdge>`. Reuse built-in SCC/centrality. (~3h, mechanical)
2. **Replace label-propagation** — current `detect_communities` uses label-prop; swap for **Louvain** (via `petgraph-community` or port GitNexus's cluster algo). Higher-quality modularity on dense call graphs. (~6h)
3. **Incremental `upsert_entry`** — mirror GitNexus's delta-update pattern so `from_entries` isn't re-run for the whole dict on every change. (~4h)
4. **Cypher-compatible export** — emit so `nom-graph` dumps can be loaded by GitNexus for cross-tool queries. (~2h)

### Effort

~2 days. Unblocks doc-04 §5.15 joint-specialization (which needs community detection that doesn't rebuild on every ingestion).

---

## C. `nom-intent` (M8) + M9 re-rank upgrade (from wrenai) — 3-6 weeks MVP, 2 quarters full

**Context**: M8 is doc 09's biggest risk — the only probabilistic component. Zero Rust code exists for `nom-intent` today.

### How WrenAI bounds LLM output (the key technique)

1. **JSON-schema structured output + Pydantic `Literal`.** [`IntentClassificationResult`](../../APP/wrenai/wren-ai-service/src/pipelines/generation/intent_classification.py:321-323) uses `Literal["MISLEADING_QUERY","TEXT_TO_SQL","GENERAL","USER_GUIDE"]`; `INTENT_CLASSIFICAION_MODEL_KWARGS` (:327-335) passes `model_json_schema()` as `response_format` so the LLM cannot emit other tokens.
2. **RAG-prefiltered prompt context.** `IntentClassification` (:338-400) runs `table_retrieval` → `dbschema_retrieval` (:177-231) against Qdrant **before** the LLM call — only tables/columns that passed vector retrieval appear in the prompt, collapsing the LLM's vocabulary to the current project.
3. **Validate-and-retry correction loop.** `SQLCorrection` ([sql_correction.py:138](../../APP/wrenai/wren-ai-service/src/pipelines/generation/sql_correction.py)) re-prompts with the engine's error message (:76-78 `Error Message: {{invalid_generation_result.error}}`), treating the deterministic engine as the oracle. `post_process` in intent_classification (:300-315) falls back to a safe default on parse failure.

### Patterns stealable for nom-intent (M8)

- **Mirror `IntentClassificationResult` → Rust**:
  ```rust
  enum NomIntent {
      Kind(EntryKind),
      Symbol(Uid),
      Flow(FlowId),
      Reject(Reason),
  }
  ```
  Emitted via OpenAI/Anthropic structured-output JSON schema. The `Reject(Reason)` arm is **the bounded-output guarantee** — any LLM output that doesn't match a registered Nom concept returns `Reject(Unparseable)`.
- **Two-stage retrieve-then-classify**: ANN over DB2 `entry_embedding` (Qdrant-equivalent) narrows to ≤50 candidate nomtu before prompting — collapses the LLM's task from "invent a concept" to "pick from 50".
- **Hard reject on parse failure**: mimic WrenAI's `post_process` fallback. Never invent a symbol.
- **Chain-of-thought with bounded reasoning budget**: `reasoning: max 20 words` (line 27). Cheaper and auditable.
- **Few-shot via `sql_samples` / `histories` slot** (:124-131): feed prior successful nomtu→prose pairs from `entry_meta`.
- **Langfuse-style tracing via `@observe`**: Nom's `flow_steps` table already supports this — wire a decorator around every `nom-intent` call.

### What M9 semantic re-rank can borrow

- **Two-tier recall/precision**: `ScoreFilter` in sql_pairs_retrieval.py + `table_retriever top_k=50` then `dbschema_retriever top_k=100` (:345-357). Nom's `find_words_v2_by_kind` should go `top_k_by_kind=50 → top_k_by_score×similarity=10`.
- **`OutputFormatter` pattern** ([sql_pairs_retrieval.py:18-33](../../APP/wrenai/wren-ai-service/src/pipelines/indexing/sql_pairs_retrieval.py)) — metadata-preserving re-shape. Adopt for `entry_benchmarks`-informed reranking.
- **Historical-question retriever**: reuses past queries as implicit training. Nom should rerank by `entry_meta.last_used` + benchmark cost.

### Week-1 slice for `nom-intent` — ✅ SHIPPED 2026-04-14

Crate created at [nom-compiler/crates/nom-intent/](../../nom-compiler/crates/nom-intent/) with `lib.rs` exposing:

```rust
pub fn classify(prose: &str, ctx: &IntentCtx, llm: &LlmFn) -> Result<NomIntent, IntentError>;
pub fn retrieve_candidates(prose: &str, k: usize) -> Result<Vec<String>, IntentError>;
pub fn validate(intent: NomIntent, candidates: &[String], threshold: f32) -> NomIntent;
```

With `NomIntent::{Kind(String), Symbol(String), Flow(String), Reject(Reason)}` as the exhaustive return type (mirrors WrenAI's `Literal[...]` bounding), `Reason::{Unparseable, UnknownKind, UnknownSymbol, BelowConfidenceThreshold}` for typed rejections, and `LlmFn = Box<dyn Fn(&str, &[String]) -> Result<NomIntent, IntentError>>` so tests pass deterministic closures.

**4 passing tests in the initial skeleton** (no real LLM yet):
- `classify_returns_symbol_when_llm_emits_match_in_candidates` — happy path
- `validate_rejects_symbol_not_in_candidates` — **the bounded-output guarantee**
- `validate_passes_symbol_when_candidates_empty` — degenerate case pre-M6 pilot
- `reject_variant_round_trips_through_validate` — reject is terminal

This lets the whole pipeline ship before any real LLM integration — the closure stub proves the bounding/validation discipline holds.

### Effort

- **Thin LLM wrapper + DB2 retrieval + schema-bounded JSON**: 3 weeks MVP
- **Correction loop + few-shot from `entry_meta` + tracing**: +3 weeks
- **Full deferred-05 ~1B transformer fine-tune on nomtu corpus**: 2 quarters (real work)

Realistic M8 MVP is **~Q1 2026-Q3** from today; hardened **+1 quarter**. Depends on M6 pilot shipping first (needs a populated dict to retrieve candidates from).

---

## D. `nom-lsp` (M16) — from zed-main (placeholder — user-driven analyze)

*This section is intentionally empty pending user's zed-main analysis. Expected mining targets per doc 09 M16 deliverable:*

- LSP server scaffolding (`crates/lsp/`)
- Hover + completion + go-to-def implementations
- Authoring Protocol reference client
- "why this Nom?" drill-through from editor to glass-box report
- Incremental typecheck wiring (salsa-style)

When the indexing completes, queries like `npx gitnexus query "lsp hover completion" --repo zed-main` + `cypher "MATCH (fn:Function) WHERE fn.filePath CONTAINS 'lsp' RETURN fn.name LIMIT 50"` should surface the concrete patterns.

---

## Prioritization

Mapping onto doc 09's "Actual remaining work" critical path:

| nom-bench | nom-graph | nom-intent (M8) | nom-lsp (M16) |
|---|---|---|---|
| post-1.0 cosmetic | days — worth doing now | weeks MVP, quarters full | quarters |
| 3 weeks | 2 days | blocks on M6 first | blocks on M10 real |

**Order suggestion**:
1. **nom-graph (2 days)** — smallest, immediate compounding value (detect_communities improvement unblocks §5.15).
2. **M8 MVP with stub LLM (3 weeks)** — prove the bounded-output discipline before M6 populates the dict; by the time M6 ships, nom-intent is ready to consume it.
3. **nom-bench (3 weeks)** — can run in parallel with either above; nom-bench has no upstream dependencies.
4. **M16 nom-lsp (quarters)** — starts after doc 09 M10 real-port lands.

---

## Next actions

1. Land this doc as `research/language-analysis/10-external-repo-upgrade-plan.md` (happening with this commit).
2. **User's zed-main analyze** completes → §D gets populated.
3. **Micro-commit for `nom-graph` petgraph swap** — smallest concrete step, 3h of work, high signal. Ship as a standalone commit with before/after benchmark comparison.
4. **Create `nom-intent` crate skeleton** per §C week-1 slice — compile green + one deterministic test against a stub LLM.
5. **nom-bench registry** — `inventory` crate dep + `BenchFamilyRegistry` + `nom bench list` CLI wire-up as a 1-day wedge before the full 3-week plan.

External-repo mining discipline for future cycles: always `--skip-git` for non-cloned references, always cite the original-repo file:line (not just the symbol name), always verify the pattern against Nom's existing code before writing up a recommendation (the graphify pivot was the saved cycle from doing this).
