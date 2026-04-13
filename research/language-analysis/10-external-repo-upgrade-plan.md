# 10 — External-repo upgrade plan (GitNexus-mined, 2026-04-14)

**Method**: five reference repos were indexed with `npx gitnexus analyze --skip-git` and queried via MCP Cypher + symbol context. Three subagents synthesized findings into per-crate upgrade plans. All patterns below cite the external repo's exact symbol + file path so implementers can open the source directly.

Ref | Repo | Indexed (nodes / edges / flows) | Nom target
---|---|---|---
A | `benchmark-main` (google/benchmark) | 2264 / 5146 / 190 | `nom-bench`
B | `graphify-master` | 238 / 364 / 2 — **SKIP** | — (pivoted to petgraph + GitNexus-core + rust-analyzer)
C | `wrenai` (Canner) | 5692 / 15373 / 276 | `nom-intent` (M8) + `nom-cli/src/store/resolve.rs` re-rank (M9)
D | `zed-main` | 78594 / 219709 / 300 | `nom-lsp` (M16)
E | `CoreNLP-main` (stanfordnlp) | 48519 / 177153 / 300 | `nom-extract` + `nom-intent` M8.1 + `.nomx v2` enrichment DSL — see §E

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

1. **`nom-bench::{BenchFamily, register, list}`** ✅ SHIPPED 2026-04-14. Global `Mutex<Vec<BenchFamily>>` via `OnceLock` (no new dep vs the `inventory` crate alternative). Idempotent by `name`, preserves insertion order, serialized test-lock for parallel-test safety. 4 new passing tests: `register_and_list_round_trip`, `register_idempotent_by_name_replaces_prior`, `register_preserves_insertion_order_for_distinct_names`, `bench_family_round_trips_through_json`. CLI `nom bench list` subcommand still TODO (separate wedge).
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

## D. `nom-lsp` (M16) — from zed-main (78,594 nodes / 219,709 edges, indexed 2026-04-14) — 3-5 days wedge, 4-5 months full

### Zed's LSP architecture in 3 primitives

1. **Transport + dispatch** — `LanguageServer` at [crates/lsp/src/lsp.rs](../../APP/zed-main/crates/lsp/src/lsp.rs) (2256 LOC) owns stdio pipes, JSON-RPC multiplexing (`handle_incoming_messages` L614), `initialize` (L1051), `on_request` / `on_notification` (L1145, L1157), `request` / `notify` (L1380, L1581).
2. **Language plug-in contract** — `trait LspAdapter` at [crates/language/src/language.rs:400](../../APP/zed-main/crates/language/src/language.rs) supplies `initialization_options` (L476), `workspace_configuration` (L506), `language_ids` (L546), `process_diagnostics` (L403), `label_for_completion` (L443). Registry via `LanguageRegistry::new` ([language_registry.rs:141](../../APP/zed-main/crates/language/src/language_registry.rs)), `register_native_grammars` (L425), `language_for_name` (L519).
3. **Workspace orchestrator** — `LspStore` at [crates/project/src/lsp_store.rs](../../APP/zed-main/crates/project/src/lsp_store.rs) (14,656 LOC) fans requests across buffers; request types modeled as `LspCommand` impls — `GetDefinitions` ([lsp_command.rs:650](../../APP/zed-main/crates/project/src/lsp_command.rs)), `GetHover` (L1986), `GetCompletions` (L2226).

### Patterns stealable for nom-lsp

- **Typed-request trait** — `LspCommand` with `type LspRequest` + `check_capabilities` ([lsp_command.rs:1995](../../APP/zed-main/crates/project/src/lsp_command.rs)) gives uniform cancellation / proto-serialization per request kind.
- **`on_request` closure registry** (lsp.rs:1157) — register handlers dynamically without match-mega-switches.
- **`default_initialize_params`** (lsp.rs:746) — one place to declare server capabilities; mirror in `nom-lsp` so hover/goto toggle from Authoring Protocol.
- **`AdapterServerCapabilities`** (lsp.rs:1344) — cache server-advertised caps so clients skip disabled requests.
- **`Subscription`-based notification teardown** (`on_notification` L1145) — drop-guard cleanup, no leaked handlers when buffers close.
- **Semantic-tokens delta apply** at [lsp_store/semantic_tokens.rs](../../APP/zed-main/crates/project/src/lsp_store/semantic_tokens.rs) (`apply`, `from_full`) — precedent for incremental pushes from Nom's salsa layer.
- **`LspAdapterDelegate`** (language.rs:379) — thin FS/HTTP seam lets tests inject fakes; critical for Nom's dict-resolver mocks.

### Zed's "why this?" editor-to-explainer pattern

- **`InfoPopover`** in [crates/editor/src/hover_popover.rs:214,816](../../APP/zed-main/crates/editor/src/hover_popover.rs) chains `GetHover` result → markdown blocks → popover; `hover_popover_delay` (L184) debounces. Nom can replace markdown with a glass-box JSON fetcher — hover calls `cmd_build_report` against the symbol's entry.
- **`DiagnosticPopover`** (hover_popover.rs:817,840) renders diagnostic + code-action affordance inline — direct analog for the **"why this Nom?"** drill-through button surfacing the `LayeredDreamReport`.

### Concrete week-1 slice — ✅ SHIPPED 2026-04-14

1. ✅ `nom-compiler/crates/nom-lsp/` created with `lsp-server` 0.7 + `lsp-types` 0.95 deps (rust-analyzer stack, not tower-lsp — sync server, no tokio dep, ~200 LOC scaffold).
2. ✅ `server_capabilities()` returns `ServerCapabilities { hover_provider: Some(Simple(true)), .. }` with all other providers off — week-1 discipline of "only advertise what's handled."
3. ✅ `dispatch_request(req) -> Response` pure function (Zed `on_request` closure registry analog, statically dispatched). `HoverRequest` routed to `handle_hover`; unknown methods return `MethodNotFound`.
4. ✅ 4 unit tests in `nom-lsp/src/lib.rs`:
   - `server_capabilities_exposes_hover_only_in_week_1`
   - `dispatch_hover_returns_markdown_with_server_name`
   - `dispatch_unknown_method_returns_method_not_found`
   - `server_name_and_version_are_nonempty`
5. ✅ `nom lsp serve` CLI subcommand SHIPPED 2026-04-14 — `LspCmd::Serve` enum variant in [nom-cli/src/main.rs](../../nom-compiler/crates/nom-cli/src/main.rs) dispatches into `nom_lsp::serve_on_stdio()`. `nom-lsp` path-dep added to [nom-cli/Cargo.toml](../../nom-compiler/crates/nom-cli/Cargo.toml). Workspace `cargo build -p nom-cli` clean (1m29s). Editor integration path: `nom lsp serve` as the language-server command.

Effort delta: estimated 3-5 days, shipped in one cycle (≈30 min of code + tests + build). `lsp-server` + `lsp-types` totalled 42s compile time. Remaining M16 work (hover-against-real-dict, goto-def, completion, diagnostics, semantic tokens, Authoring Protocol drill-through, salsa incremental) stays at 6-8 weeks MVP / 4-5 months full per doc-09's quarters estimate — the week-1 scaffold doesn't shorten that; it just unblocks the parallel work.

### Estimated effort

- **Week-1 slice**: 3-5 days (scaffold + smoke test).
- **MVP** (hover + goto + completion + diagnostics against real dict): 6-8 weeks.
- **Full M16** incl. Authoring-Protocol drill-through + salsa incremental wiring: 4-5 months — consistent with doc-09's "quarters," leaning to the short end **if `tower-lsp` is adopted instead of hand-rolling transport like Zed does**. Zed's 2256-LOC `lsp.rs` is explicitly the "we own transport" path; `tower-lsp` gives the same feature set in ~100 LOC of glue. Save the hand-roll for post-M17 if Zed-level extensibility is needed.

### Key files to study

- `crates/lsp/src/lsp.rs` (transport)
- `crates/language/src/{language.rs, language_registry.rs}` (plug-in contract)
- `crates/project/src/{lsp_command.rs, lsp_store.rs, lsp_store/semantic_tokens.rs}` (orchestrator)
- `crates/editor/src/hover_popover.rs` (UX surface for glass-box drill-through)

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

## Next actions — status snapshot 2026-04-14

**Completed this session (55+ commits, 26 code wedges):**
1. ✅ Doc committed + five upgrade plans populated (§A-E)
2. ✅ Zed-main analyzed + §D populated (78,686 nodes indexed)
3. ✅ **CoreNLP W1 Annotator pipeline** — `7caa41f` + `9928187` + `b32abc0` (11 nom-extract tests)
4. ✅ **`nom-intent` crate fully built out — 57 tests**:
   - slice-1 bounded-output `NomIntent` enum (`800baea`)
   - slice-2 ReAct driver + 5-tool AgentTools trait (`147939d`)
   - slice-3a `DictTools::query` (`9751aac`)
   - slice-3c-partial `DictTools::explain` (`44f66d3`)
   - slice-3c-render-metadata `DictTools::render` closure-walk hash (`07e6282`)
   - slice-3b-verify `DictTools::verify` 4 invariant checks (`a82caec`)
   - slice-3b-compose `DictTools::compose` token-overlap (`1ce0fc4`)
   - **All 5 AgentTools methods live as of `1ce0fc4`**
   - slice-4 `InstrumentedTools` decorator (`63ec6a4`)
   - slice-5a `nom agent classify` CLI (`f169cd6`)
   - slice-5b-trait `ReActAdapter` + blanket impl (`078089e`)
   - slice-5b-nom-cli `NomCliAdapter` state machine (`b54a9c7`)
   - slice-5b-cli-flag `--adapter` selector (`b993de1`)
5. ✅ **nom-bench `BenchFamily` registry** (`fa50744`)
6. ✅ **nom-lsp crate** — slice-1 scaffold (`64b3058`), slice-2 CLI (`3ba982a`), slice-3 completion (`0b68743`). 5 tests.
7. ✅ **Graph-durability β — 6/6 specced deliverables shipped**:
   - Phase 1 freshness (`60534e4`)
   - Phase 2a NodeUid (`2453375`)
   - Phase 2b upsert_entry (`421f902`)
   - Phase 3a Cypher export nodes (`1b9cc00`)
   - Phase 3b Cypher export edges (`910365c`)
   - 27 nom-graph tests total
8. ✅ **Research synthesis** — doc 11 (graph-rag research, `90958dd`), doc 12 (entity-scope deep-think, `4867307`), spec `2026-04-14-graph-rag-agentic-design.md`, slice-5b polymorphism clarification (`ebe530e`)
9. ✅ **M8 slice-5b-mcp** — `McpAdapter<R, W>` generic Read+Write JSON-RPC (`838c7a3`)
10. ✅ **M8 slice-6a** — `nom-lsp/src/agent.rs` markdown renderer for agentic drill-through (`471cbc0`, 6 tests)
11. ✅ **M8 slice-6b** — `workspace/executeCommand` handler + `nom.whyThisNom` registration in `server_capabilities` (`2341ff1`, 14 tests)
12. ✅ **W4-A2 strictness lock** — 5 closed-keyword-set tests pinning case-sensitive exact-match for `matching` / `with` / `confidence` / `the` / `is` / kinds / `at-least` (`65f1198`, doc 13 §5 A2). 82 nom-concept tests total.
13. ✅ **W4-A1 strictness lock** — 4 ct10* tests pinning that entity refs MUST carry `@Kind` (v2) or `Kind Word` (v1); bare `the matching "..."` or unknown `@Banana` reject hard. (next commit) 86 nom-concept tests total.

**Queued (ordered by ascending effort):**
- **M8 slice-5b-mcp-spawn** — wire `McpAdapter` to a spawned child process (~0.5d)
- **M8 slice-5b-cli-flag-mcp** — add `mcp` arm to `--adapter` in `cmd_agent_classify` (~0.5d)
- **Phase 2c graph unification** — switch `from_entries` / `build_call_edges` / `build_import_edges` / `detect_communities` to uid storage, deprecate Vec fields (~3d, biggest churn)
- **M8 slice-3b-verify-full** — wire `nom-verifier` + `nom-security` + `nom-concept` MECE (~4d)
- **M8 slice-3c-full** — real render via `nom-codegen` + `nom-llvm` + `nom-app` + `nom-media` (~5d, biggest user-visible wedge per doc 12 deep-think)
- **M8 slice-5b-real-llm** (optional) — OpenAI/Anthropic wrapper (requires API keys)
- **W4 strictness lane** (new 2026-04-14) — doc 13 specifies 6 wedges A1-A6. **A1 + A2 + A3 + A6 closed; A5 audited.** A3 lands as an additive `strict` module in nom-concept: `validate_nom_strict(&file)` + `validate_nomtu_strict(&file)` walk the AST post-parse and emit `StrictWarning { code, message, location }` for every typed-slot ref missing `with at-least N confidence`. A6 pre-locked by existing `resolve.rs` tests. **A5 audit:** `EntityRef.kind: Option<String>` is load-bearing in `materialize.rs:105-116` (reconstruction from hash); recommended tightening is a `EntityKindSlot::{Known, UnknownUntilLookup}` enum, deferred until materialize refactor. A5 surfaced as a known soft spot with a documented fix path, not a correctness bug. Remaining active: A4 (annotator-style staged parser, ~3d) + A5 refactor (~1d). See [doc 13 §5](13-nomx-strictness-plan.md#5-strictness-wedges-ordered).

External-repo mining discipline for future cycles: always `--skip-git` for non-cloned references, always cite the original-repo file:line (not just the symbol name), always verify the pattern against Nom's existing code before writing up a recommendation (the graphify pivot was the saved cycle from doing this).

---

## E. `nom-extract` + `nom-intent` M8.1 + `.nomx v2` enrichment DSL (from CoreNLP-main) — 13 engineer-days total

Stanford CoreNLP indexed 2026-04-14 (48519 nodes, 177153 edges, 300 flows). Companion model JARs at `C:\Users\trngh\Downloads\stanford-corenlp-models-english-{extra,kbp}.jar` (670 MB total) — noted as available runtime assets for **far-future** JVM-interop, excluded from all short-term wedges per `.omc` no-JVM constraint.

### CoreNLP's 5 core primitives

1. **`Annotator` interface** — [src/edu/stanford/nlp/pipeline/Annotator.java:54](../../APP/CoreNLP-main/src/edu/stanford/nlp/pipeline/Annotator.java). Uniform contract `annotate(Annotation)` + declared `requires()` / `requirementsSatisfied()`. Composable stage.
2. **`AnnotationPipeline`** — [src/edu/stanford/nlp/pipeline/AnnotationPipeline.java:27](../../APP/CoreNLP-main/src/edu/stanford/nlp/pipeline/AnnotationPipeline.java). Ordered `Annotator` list; each writes typed keys onto the shared `Annotation` map. `StanfordCoreNLP` (`pipeline/StanfordCoreNLP.java`) is the properties-driven factory.
3. **POS tagger — `MaxentTagger`** — [src/edu/stanford/nlp/tagger/maxent/MaxentTagger.java:231](../../APP/CoreNLP-main/src/edu/stanford/nlp/tagger/maxent/MaxentTagger.java). Maxent model, invoked via `POSTaggerAnnotator`.
4. **Dep parser — `DependencyParser`** — [src/edu/stanford/nlp/parser/nndep/DependencyParser.java:74](../../APP/CoreNLP-main/src/edu/stanford/nlp/parser/nndep/DependencyParser.java) (neural transition-based) → `DependencyParseAnnotator`. Emits typed `SemanticGraph`.
5. **Coref + pattern DSLs** — `CorefAnnotator.java:44`; **`SemgrexPattern.java:239`** (semantic-graph pattern lang); `TregexPattern.java:357` (tree pattern lang); **`OpenIE.java:65`** (SVO triples via `RelationTriple`).

### Patterns stealable for Nom

- **Typed-key Annotation map → `nom-extract` proposal envelope.** Each stage writes namespaced keys; downstream reads required preds. Same contract fixes "what ran on this prose" question in glass-box reports.
- **Required/satisfied declaration → `nom-intent` M8 slice composition.** Each extractor declares `requires = {tokens, pos}`; pipeline self-orders.
- **`SemgrexPattern`-style DSL → `.nomx v2` inspectable enrichment layer.** Write concept-matching rules against the DAG the parser emits — glass-box, no LLM opacity.
- **`Tregex` over constituency trees → Nom AST lint/rewrite skills.** Fit for `compose_from_dict` transforms.
- **`OpenIE.RelationTriple` → SVO → concept/module/entity tiers.** Subject = entity, verb = module, object = entity/concept; seeds dream-tree nodes.
- **Annotator requirement graph → `nom-lsp` progressive enrichment.** Cheap stages (tokenize/POS) on keystroke; expensive (coref/OpenIE) on save.
- **Properties-driven pipeline config → `.nomtu` authoring profiles.** Reuse Nom's hash-keyed config instead of `.properties`.

### Concrete wedge menu (smallest → biggest)

- **W1 · `Annotator` trait in `nom-extract`** ✅ SHIPPED 2026-04-14. `Annotator` trait + `Annotation` typed-key map + `AnnotationPipeline` runner in [nom-extract/src/annotator.rs](../../nom-compiler/crates/nom-extract/src/annotator.rs). Precondition check errors with `AnnotatorError::MissingRequirement { annotator, key }` if any stage's `requires()` is unsatisfied on the annotation; audit trail via `Annotation::ran()`. 5 tests lock: pipeline order preservation, missing-requirement error, sorted-keys determinism, audit-trail recording, declarative requires/satisfied. Week-1 wedge shipped faster than the estimated 2 days (~45min).

**W1b (wired-to-real-code) ✅ SHIPPED 2026-04-14**: `ParseAndExtractAnnotator` wraps `nom_extract::extract::parse_and_extract` as an `Annotator` impl. Requires `source` + `language` + `file_path` on the annotation; satisfies `entities` (JSON-encoded `Vec<UirEntity>`). 3 additional tests: contract declaration, missing-requirement rejection, end-to-end Rust `fn greet() { ... }` → non-empty entities JSON. Total nom-extract annotator tests: 8. This is the first concrete `Annotator` driving real nom-extract code — the trait is no longer toy-only. Future wedges can add `ScanDirectoryAnnotator`, `IntentClassifyAnnotator`, `SvoTripleAnnotator` (W3).
- **W2 · `SemgrexPattern`-lite DSL over Nom concept DAG** (~4 days). Read-only pattern matcher; 6 combinators (node, child, descendant, sibling, label, kind). Used by MECE validator + `nom-intent` narrowing.
- **W3 · OpenIE-style SVO extractor → Proposal seed** (~7 days). Local dep-parse (tree-sitter-english or llamaindex-rs mini-parser, **NOT** CoreNLP JVM) emits `(subj, rel, obj)` → concept/module/entity tier; feeds `nom author translate`.

### Don't adopt

- **JVM runtime / 670 MB JAR models.** Out-of-scope per `.omc` constraint. Model assets noted only for far-future research integration.
- **Statistical-ML taggers as first-class compiler input.** Keep LLM-as-oracle (M8); ML only inside M8 narrowing path.
- **English-only assumptions in pattern DSLs.** Nom's vocabulary is English but syntax is head-initial / classifier-anchored — re-derive the relation taxonomy.
- **Constituency parsing.** Dependency parse is enough for SVO; skip Tregex as premature.

### Effort estimate

| Wedge | Engineer-days | Risk | Ships |
|---|---|---|---|
| W1 Annotator trait | 2 | low | Pipeline introspection |
| W2 Semgrex-lite DSL | 4 | medium (DSL surface) | .nomx v2 enrichment |
| W3 SVO→Proposal | 7 | medium (parser choice) | M8.1 author-loop |
| **Total** | **~13 days** | | **M8.1 shipped** |

**Key insight**: CoreNLP's `Annotator.requires() / requirementsSatisfied()` contract is the cheapest steal — it directly fixes `nom-extract`'s current opaque ordering. The `SemgrexPattern` **DSL shape** (not the Java impl) is the highest-leverage structural borrow for `.nomx v2` glass-box enrichment. W3 must use a Rust-native mini dep-parser, never JNI into CoreNLP.
