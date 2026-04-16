# Nom Programming Language — QWEN Context

> **Date:** 2026-04-16 (deep-dive session 6 — ten-batch session)
> **Last updated:** 2026-04-16 — Batch 10. **GAP-5a COMPLETE** — match expression + nested multi-arg call recovery shipped; ast_bridge.rs now 1487 lines. **GAP-6 metadata specialization SHIPPED** — ResolverMetadata, kind-aware scoring, contract validation nodes, 25 planner tests. **GAP-1c advanced** — find_entities canonical function, 5 deprecations, 103 dict tests. **1067 tests passing (single-threaded), 0 failed, 34 ignored.**

## Latest Session Delta — 2026-04-16 (ten batches)

**Batch 10 (4-agent parallel execution):**
- GAP-5a match expression recovery: `parse_match_expr` lowers multi-arm `when/then` chains to chained `IfExpr` via `else_ifs`; 4 new tests. ✅
- GAP-5a nested multi-arg call recovery: `split_args_balanced` tracks paren depth; 3 new tests. ✅
- `ast_bridge.rs` now **1487 lines** with 26 bridge tests. **GAP-5a COMPLETE.** ✅
- GAP-6 resolver metadata-backed specialization: `ResolverMetadata` struct, `kind_adjusted_score`, contract validation node insertion, `optimize_plan_with_context`; 25 planner tests. ✅
- GAP-1c: `find_entities` canonical filter query; `get_entry`/`find_entries`/`list_partial_ids`/`find_by_word`/`find_by_body_kind` deprecated; 4 blocked migrations documented; 103 dict tests. ✅
- **1067 tests passing (single-threaded), 0 failed, 34 ignored.** ✅

**Batch 8:**
- GAP-12 watermark clause (streaming): S5j extraction, `watermark_clause.rs` test file.
- GAP-12 window-aggregation clause: S5k extraction, `window_clause.rs` test file.
- GAP-12 clock-domain clause: S5l extraction, `clock_domain_clause.rs` test file.
- GAP-12 QualityName-registration formalization: S5m extraction, `quality_declarations.rs` test file.
- **GAP-12 is now 11/11 COMPLETE — entire grammar wedge backlog cleared.**
- 37 new tests across 4 test files. S5 extraction chain is now S5a–S5m (13 sub-stages).
- LSP exact source ranges for goto-definition shipped; 31 LSP tests (up from 27).
- **GAP-8 newly unblocked** — nom-llvm linking shipped + bridge fidelity sufficient for real artifact production.
- **1043 tests passing (single-threaded), 0 failed, 34 ignored.**

**Batch 7:**
- GAP-12 exhaustiveness check on `when` clauses: S5i extraction, `WhenClause` struct, `check_exhaustiveness()` validator, 9 tests.
- **7/11 grammar wedges now shipped** (retry-policy, @Union, format-string, nested-record-path, wire-field-tag, pattern-shape, exhaustiveness).
- S5 extraction chain is now S5a–S5i (9 sub-stages).
- LSP enriched hover: structured Markdown with contracts/effects/retry/format/body_kind/scores; 27 LSP tests (up from 19).
- GAP-1c: `resolve_prefix` queries `entities` first; `find_entities_by_body_kind` added; `find_by_word` / `find_by_body_kind` deprecated.
- **1002 tests passing, 0 failed** — 1000-test milestone crossed.

**Batch 6:**
- GAP-12 nested-record-path (`accesses`): S5f extraction, `accesses` field on entity decl, 9 tests.
- GAP-12 wire-field-tag (`field X tagged Y`): S5g extraction, `FieldTag` struct, 9 tests.
- GAP-12 pattern-shape (`shaped like`): S5h extraction, 8 tests.
- Planner fusion: `fuse_identity_nodes`, `fuse_consecutive_maps`, `collapse_single_branch` + `optimize_plan` orchestrator, 17 planner tests.
- **6/11 grammar wedges now shipped** (retry-policy, @Union, format-string, nested-record-path, wire-field-tag, pattern-shape).
- S5 extraction chain is now S5a–S5h (8 sub-stages).
- **985 tests passing, 0 failed, 33 ignored** (was 952 after batch 5).

**Batch 5:**
- GAP-12 `@Union` sum-type: S5d extraction, `UnionVariants` struct, 8 tests — first grammar wedge shipped end-to-end.
- GAP-12 format-string interpolation: S5e extraction, skip-on-prose rule, 9 tests — second grammar wedge shipped.
- Bridge contracts/effects: `requires`/`ensures` → `Describe`, `benefit`/`hazard` → `Effects` with `Good`/`Bad` modifiers. 4 new bridge tests; 19 total bridge tests.
- 3 grammar wedges now shipped (retry-policy + @Union + format-string). Grammar wedge pattern: lexer token → Tok enum → S5 extraction → EntityDecl field → baseline.sql → tests.
- `ast_bridge.rs` now handles: functions, arithmetic, calls, conditionals, loops, multi-statement, contracts, effects.
- **952 tests passing, 0 failed, 33 ignored** (was 931 after batch 4).

**Batch 4:**
- `nom-llvm/src/context.rs` inline test module un-gated — **zero `#[cfg(any())]` gates in entire codebase**. 20 nom-llvm unit tests total (was 17).
- 8 self-host test files converted from parser-dependent to structural verification (new passing tests).
- `parser_subset_probe.rs` deleted — was historical artifact after nom-parser removal.
- GAP-5a loop recovery: `for-each`, `for-in`, `while-do`, `repeat-until` constructs added to `ast_bridge.rs` — 15 bridge tests (was 10).
- GAP-1c: `status_histogram()` migrated to `entities` table; `list_partial_ids()` deprecated.
- **931 tests passing, 0 failed, 33 ignored.**

**Batch 3:**
- Status column added to `entities` table — migration guard in `dict.rs:270`.
- `RetryPolicy` struct at `nom-concept/src/lib.rs:119` with `max_attempts`, `backoff_ms`, `jitter`, `on_errors`; `retry_policy: Option<RetryPolicy>` on entity decl. GAP-12 retry-policy grammar wedge begun.
- `retry-policy` pattern added to `grammar.sqlite` baseline.

**Batch 2:**
- `store_cli.rs` rewritten — `NomDict` → `Dict`, flow-style fixtures → `.nomx`, 4 tests passing, file-level `#![cfg(any())]` removed.
- `phase4_acceptance.rs` **renamed to `dids_store_e2e.rs`** — `.nomx` + `Dict`, `#[ignore]` removed, 1 test passing. `phase4_acceptance.rs` no longer exists.
- `nom build link` subcommand fully wired — reads `.bc` files → `link_bitcodes()` → clang compilation with `llc` + system linker fallback → executable. NOT a stub.
- GAP-1c: `cmd_store_stats` now reads canonical `entities` table. 23+ legacy-table functions inventoried across 7 files.
- All 5 nom-llvm external test files (`builtins.rs`, `enums.rs`, `lists.rs`, `strings.rs`, `tuples.rs`) migrated — 16 tests, zero `#[cfg(any())]` in external files.

**Batch 1:**
- 36 stale workspace files removed (Phase 0).
- `ast_bridge.rs` extended to **753 lines** — conditionals (`if/then/else`, `when/then/otherwise`), multi-statement bodies, comparison operators. 10 bridge tests.
- `link_bitcodes(bitcode_blobs: &[Vec<u8>]) -> Result<LlvmOutput, LlvmError>` at `nom-llvm/src/lib.rs:48`. `compile_plans(plans: &[CompositionPlan]) -> Result<LlvmOutput, LlvmError>` at `nom-llvm/src/lib.rs:84`. 4 new inline tests.

## Previous Session Delta — 2026-04-16T21:00 JST

- `nom-cli/src/store/commands.rs` migrated to Dict free functions; Dict exports added to `nom-dict/src/lib.rs`.
- **Both `nom-lexer` and `nom-parser` crates confirmed deleted** — `crates/nom-lexer/` and `crates/nom-parser/` do not exist. Workspace is **29 crates**.
- `self-host` feature is **fully removed** from `nom-cli/Cargo.toml` — no `[features]` section at all.
- `link_bitcode_to_executable` stub **removed from `manifest.rs`** (confirmed absent via grep).
- `read_extractable_atoms` removed (confirmed absent from codebase).
- `chrono_like_now` confirmed in active use at `author.rs:317` and `store/mod.rs:111` — keep.
- `ast_bridge.rs` was **584 lines** then — prose-conditional recovery (`if X then Y else Z`) shipped alongside arithmetic and call/return recovery.
- `cargo check --workspace` → `Finished dev profile` (0 errors).

## Earlier Session Delta — 2026-04-16T18:40 JST

- `nom-planner` now has `plan_from_pipeline_output(PipelineOutput) -> CompositionPlan` for S1-S6 `.nomtu` module compositions.
- `nom build compile` now plans directly from `nom-concept` pipeline output when flows exist, with AST-bridge fallback for concept/entity-only files.
- `nom-lsp` hover and `textDocument/definition` now resolve split-dict entity rows via `NOM_DICT`.
- `nom-cli` dispatch runs on a larger worker stack to avoid Windows `STATUS_STACK_OVERFLOW` before subcommand execution.
- `nom author`, `nom store add`, and `nom mcp serve` now create/open split-dict roots when given legacy-looking `nomdict.db` paths.
- Legacy v1 integration lanes that still depend on removed `nom-parser` syntax or single-file `NomDict` behavior are explicitly gated with replacement notes.

## Project Overview

**Nom** is a programming language implementing a **compositional-semantic programming paradigm** — described as "the fifth paradigm." You write English sentences describing what you want. The compiler looks up each word in a content-addressed dictionary, verifies contracts, and produces a native binary.

**Tagline:** *"A nomtu is a word. A .nom is a sentence. A binary is a story."*

**Author:** LLg Trn

**Implementation:** 29-crate Rust workspace (edition 2024)

---

## Architecture: The Two Functions

The system implements **two separate functions** connected by a 3-database store:

### Function 1: Ingest Source → Write to 3-DB System

```
Author writes:  myfile.nomx  (prose English)
       │
       │  nom-concept S1-S6 staged pipeline:
       │    S1 tokenize (+ synonym rewrite via grammar.sqlite when available)
       │    S2 kind_classify (validates against closed KINDS set)
       │    S3 shape_extract (intent extraction per block)
       │    S4 contract_bind (requires/ensures clauses)
       │    S5 effect_bind (hazard/benefit/boon/bane clauses)
       │    S6 ref_resolve (typed-slot refs → name@hash)
       │
       ├─► STEP 1 MACRO SCOPE (first):  concept declarations
       │     → PipelineOutput::Nom → concepts.sqlite → concept_defs table
       │       { name, repo_id, intent, index_into_db2, exposes, acceptance, objectives }
       │
       └─► STEP 2 MICRO SCOPE (after concepts settled):  entity declarations
             → PipelineOutput::Nomtu → entities.sqlite → entities table
               { hash, word, kind, signature, contracts, composed_of,
                 body_kind=None, body_size=len(source) }
```

**Hash identity:** `sha256(serde_json::to_vec(&entity_decl))` — computed from JSON-serialized
pipeline AST, not raw source bytes.

### Function 2: Build App from DB

```
nom build status / manifest / dream
       │
       ▼
  concepts.sqlite → concept_defs rows
       │   { name, intent, index_into_db2 (JSON entity hashes) }
       │
       ▼  BFS over index clauses
  entities.sqlite → find_entity(hash)
       │   ├─ leaf entity → hash ref in closure
       │   └─ composed_of entity → NomtuFile { CompositionDecl }
       │
       ▼
  ConceptGraph { concepts, modules }  ← BFS walk
       │
       ▼
  resolve_closure() → word-to-hash matching
       │
       ▼
  Build report / status / JSON manifest
```

### LLVM Binary Linking — SHIPPED (2026-04-16 Batch 2)

```
nom build link <path-to-.bc-files>
       │  reads .bc files from directory or single file
       │  link_bitcodes(&blobs) ← ✅ SHIPPED in nom-llvm/src/lib.rs:48
       │  clang compilation → executable
       │  fallback: llc + system linker if clang unavailable
       ▼
  native executable (or error if clang/llc not found)
```

> **Honest status (2026-04-16T23:30):** `link_bitcodes(bitcode_blobs: &[Vec<u8>]) -> Result<LlvmOutput, LlvmError>` is fully implemented at `nom-llvm/src/lib.rs:48`. `compile_plans(plans: &[CompositionPlan]) -> Result<LlvmOutput, LlvmError>` is at line 84. The `nom build link` CLI subcommand is wired to `link_bitcodes()` at `main.rs:6521` and produces a real executable via clang (with llc + linker fallback). This closes the LLVM pipeline end-to-end.

---

## The Three-Database Layout

### grammar.sqlite — Grammar Registry (awareness-only)

```
├── schema_meta
├── keywords            (43 keywords)
├── keyword_synonyms    (7 synonyms)
├── clause_shapes       (43 shapes across 9 kinds)
├── kinds               (9 kinds: function, module, concept, screen,
│                        data, event, media, property, scenario)
├── quality_names       (20 names, metric_function nullable)
└── patterns            (258 patterns, Jaccard-searchable)
```

### concepts.sqlite — DB1: Concept Tier (MACRO scope)

```
├── concept_defs        (canonical: name PK, repo_id, intent,
│                         index_into_db2 JSON, exposes, acceptance, objectives)
├── required_axes       (MECE registry per scope)
├── dict_meta           (freshness tracking)
├── concepts            ← LEGACY — scheduled for deletion (dict-split S8)
└── concept_members     ← LEGACY — scheduled for deletion (dict-split S8)
```

### entities.sqlite — DB2: Entity Tier (MICRO scope — scales to 100M+)

```
├── entities            (canonical: hash PK, word, kind, signature,
│                         contracts, body_kind, body_size, composed_of, …)
├── entry_scores        (11 quality dimensions + overall_score)
├── entry_meta          (EAV metadata — status, language, etc.)
├── entry_signatures
├── entry_security_findings
├── entry_refs          (dependency graph edges)
├── entry_graph_edges   (typed edges: Calls, Imports, Writes, …)
├── entry_translations
└── entries             ← LEGACY — body_bytes BLOB stored here today
                          scheduled for migration + deletion (dict-split S8)
                          ~50+ SQL queries still target this table
```

**Storage reality:** `body_bytes` (both `.bc` bitcode and media blobs) are stored **inline as
SQLite BLOBs** in the legacy `entries` table. The artifact store on filesystem
(`dict_db.root()/store/<hash>/`) is partially wired (used by `materialize_closure_body` for
body reads) but ingestion still writes to `entries`.

**Cross-tier references:** `concept_defs.index_into_db2` stores a JSON array of entity hashes.
Cross-tier joins resolved in **Rust layer** via `materialize_concept_graph_from_db`.

---

## All 29 Crates

### Core Pipeline (inner ring)

| Crate | Path | Role |
|-------|------|------|
| **nom-cli** | `crates/nom-cli/src/main.rs` | CLI entry point — 6337-line dispatch, Commands enum covers 30+ subcommand groups |
| **nom-lexer** | ~~`crates/nom-lexer`~~ | ✅ **DELETED (GAP-4 complete, 2026-04-16)** — crate directory removed; was the flow-style classifier tokenizer. The `nom-concept` internal prose-English lexer is the only active lexer. |
| **nom-parser** | ~~`crates/nom-parser`~~ | ✅ **DELETED (GAP-4 complete, 2026-04-16)** — crate directory removed; all `Cargo.toml` entries removed from workspace, nom-cli, and nom-llvm. `parse_with_bridge_or_legacy()` is bridge-only (no fallback). |
| **nom-ast** | `crates/nom-ast` | Shared AST types: SourceFile, Declaration, Statement, Expression, FlowChain, Classifier (10 kinds) |
| **nom-concept** | `crates/nom-concept` | **Core** — Tier-1 (`.nomtu`) + Tier-2 (`.nom`) AST + staged S1-S6 parser (3192 lines). Internal prose-English lexer (`the`, `is`, `uses`, `composes`, `@Kind`, `at-least`) |
| **nom-resolver** | `crates/nom-resolver` | Match typed-slot refs to dictionary entities. `intent.rs` (Jaccard ranking), `v2.rs` (use-statement resolution), `v2_rewrite.rs` (AST rewrite: `#hash@name`) |
| **nom-verifier** | `crates/nom-verifier` | Per-pass invariant checking: every clause closed, contracts terminate, refs resolve |
| **nom-planner** | `crates/nom-planner` | Converts verified `SourceFile` and S1-S6 `PipelineOutput` module compositions into `CompositionPlan`. GAP-6 pipeline wiring shipped + batch 6 fusion passes (`fuse_identity_nodes`, `fuse_consecutive_maps`, `collapse_single_branch`, `optimize_plan`), 17 planner tests. Resolver metadata-backed specialization still pending. `.nomiz` serialization. |
| **nom-codegen** | `crates/nom-codegen` | Host-language AST → intermediate representation for LLVM |
| **nom-dict** | `crates/nom-dict` | **Dual-tier SQLite** — `Dict { concepts: Connection, entities: Connection }`, 50+ public free functions. Also contains legacy `V2_SCHEMA_SQL` with `entries` table (used by extract/score/corpus). |
| **nom-types** | `crates/nom-types` | Shared types: UIR, EntryKind (28 variants), AtomKind (36 variants), GraphEdge, EntryScores (8+3+1 fields), body_kind tags, self_host_tags |

### Specialized Subsystems

| Crate | Role | Status |
|-------|------|--------|
| **nom-security** | Two-layer security: compile-time score gates + body-level scanning | Schema live |
| **nom-llvm** | LLVM backend via inkwell. `compile(plan: &CompositionPlan) → LlvmOutput { ir_text, bitcode }`. `link_bitcodes(&[Vec<u8>]) → LlvmOutput`. `compile_plans(&[CompositionPlan]) → LlvmOutput`. Internal context/enums/expressions/functions/runtime/statements/structs/types modules | Shipped — IR + single-plan bitcode + multi-module linking + executable emission. 5 external test files (16 tests) ungated. `context.rs` inline tests un-gated (batch 4) — 20 total nom-llvm tests, zero `#[cfg(any())]`. |
| **nom-runtime** | Runtime support: string ops, print, alloc, file I/O | Scaffold |
| **nom-lsp** | LSP 3.17 server (stdio JSON-RPC). Slices 1-7b shipped: hover, completion, pattern-driven completion via Jaccard, enriched hover (contracts/effects/retry/format/body_kind/scores); Batch 8: exact source ranges for goto-definition. 31 LSP tests. | Slices 1-7b + source ranges |
| **nom-grammar** | Grammar registry API: 7 tables, `resolve_synonym`, `is_known_kind`, `search_patterns` (Jaccard), `fuzzy_tokens` | Shipped |
| **nom-search** | Hybrid BM25 + semantic search over dictionary | Jaccard floor |
| **nom-graph** | Knowledge graph builder over `entry_graph_edges` | Active |
| **nom-extract** | Extract entities from 50+ languages via tree-sitter | Grammars declared |
| **nom-score** | Quality scoring: 9-dimension heuristic + `bulk_set_scores` | Shipped |
| **nom-translate** | `entry_translations` table; translate `.nomtu` bodies to Rust | Scaffold |
| **nom-config** | `~/.nom/` directory config, path resolution | Shipped |
| **nom-media** | Media ingest: `ingest_by_extension` → PNG/AVIF/MP4/WAV/SVG canonical re-encode | Shipped |
| **nom-corpus** | Corpus ingestion: git clone, PyPI top list, ecosystem scanning | Blocked on network |
| **nom-ux** | Terminal UX helpers | Scaffold |
| **nom-intent** | ReAct/agentic loop: `DictTools`, `ReAct` trait, `StubAdapter`/`NomCli`/`MCP` impls | T2.2 shipped |
| **nom-locale** | Locale/i18n: BCP47 tags, NFC normalizer, confusable detector | Scaffold |
| **nom-flow** | Flow-step recording tree | Scaffold |
| **nom-bench** | Benchmark recording | Scaffold |
| **nom-app** | App-composition surface: dreaming, multi-aspect build, 20+ `upsert_entry` call sites | Shipped |

### Supporting Crates

| Crate | Role |
|-------|------|
| **nom-diagnostics** | Error formatting — `miette` + `ariadne` wrappers; `NOMX-Sx-*` error codes |
| **nom-grammar** | Grammar registry (see above) |

---

## CLI Command Surface (from main.rs ground truth)

| Namespace | Subcommands | Honest Status |
|-----------|-------------|---------------|
| `nom build` | compile, status, manifest, report, verify-acceptance, link | `link` SHIPPED — reads .bc → link_bitcodes() → clang/llc → executable |
| `nom store` | add, get ⚠️, closure, verify, gc, stats, list ⚠️, sync, add-media | `get` and `list` stubbed — disabled during V2 migration |
| `nom grammar` | init, import, status, add-kind, add-keyword, add-synonym, add-quality, add-clause-shape, add-pattern, pattern-list, pattern-show, pattern-stats, pattern-search | Shipped |
| `nom concept` | new, add, add-by, list, show, delete | Shipped |
| `nom corpus` | scan, ingest, ingest-parent, clone-ingest, clone-batch, ingest-pypi, register-axis, list-axes, seed-standard-axes | Network-blocked |
| `nom author` | start, check, translate | Shipped |
| `nom app` | dream (app/concept/module tiers), build | Shipped |
| `nom agent` | classify (ReAct loop) | Stub adapter |
| `nom lsp` | serve | Slices 1-7a |
| `nom locale` | list, validate, check-confusable, apply | Scaffold |
| `nom media` | import | Shipped |
| `nom run` | (LLVM target path) | Bridge-only: `parse_with_bridge_or_legacy()` → nom-concept pipeline |
| `nom check` | (strict check) | Bridge-only: `parse_with_bridge_or_legacy()` → ast_bridge |
| `nom report` | (security report) | Bridge-only: `parse_with_bridge_or_legacy()` → ast_bridge |
| `nom test` | (pipeline test, property-based) | Bridge-only: `parse_with_bridge_or_legacy()` → ast_bridge |
| `nom fmt` | (canonical format) | Bridge-only: `parse_with_bridge_or_legacy()` → ast_bridge |
| `nom quality` | (quality assessment) | Bridge-only: `parse_with_bridge_or_legacy()` → ast_bridge |
| `nom extract` | tree-sitter extraction | Shipped |
| `nom score` | quality scoring | Shipped |
| `nom stats` | dictionary statistics | Shipped |
| `nom coverage` | extraction coverage | Shipped |
| `nom import` | import from Novelos DB | Shipped |
| `nom precompile` | pre-compile `.nomtu` bodies to LLVM `.bc` | Shipped |
| `nom translate` | translate `.nomtu` bodies to Rust | Scaffold |
| `nom graph` | build knowledge graph | Shipped |
| `nom search` | hybrid BM25 + semantic search | Shipped |
| `nom audit` | security audit | Shipped |
| `nom dict` | search legacy nomdict entries | Shipped |
| `nom mcp` | MCP tool server | Shipped |

---

## Compilation Pipeline (What Is Actually Shipped Today)

```
.nom/.nomtu source
       │
       ▼
  [1] Parse — nom-concept S1-S6 staged pipeline (GAP-0 complete)
       │
       ▼
  [2] nom store sync <repo> — parse all .nom/.nomtu, hash, upsert into split DB
       │
       ▼
  [3] nom build status <repo> — materialize concept graph, walk closure,
       run stub resolver, check MECE objectives, report build-readiness
       │
       ▼
  [4] nom build manifest <repo> — emit JSON build manifest (post-order, leaves first)
       │
       ▼
  [5] Planner (nom-planner) — CompositionPlan from SourceFile or S1-S6 PipelineOutput
       │   GAP-6 direct pipeline planning shipped for .nomtu module compositions
       ▼
  [6] Codegen (nom-codegen) — generates IR for LLVM
       │
       ▼
  [7] LLVM (nom-llvm) — compile(plan) → LlvmOutput { ir_text, bitcode } via inkwell
       │
       ▼
  [8] Link — nom_llvm::link_bitcodes(&blobs) ← ✅ SHIPPED (2026-04-16)
       │   clang compilation → executable (llc + linker fallback)
       ▼
  native executable via `nom build link <path>`
```

### Three Compile Targets (declared, partially wired)

```bash
nom build compile --target rust <path>   # Generates Rust source (partially working)
nom build compile --target llvm <path>   # Produces .bc + .ll via LLVM (IR only)
nom build link <path>                    # ✅ SHIPPED — reads .bc → link_bitcodes() → executable via clang/llc
```

---

## nom-concept S1-S6 Pipeline (Ground Truth)

The pipeline lives in `nom-concept/src/stages.rs` (2618 lines) and `nom-concept/src/lib.rs` (3192 lines).

### Stage Functions (all implemented)

| Stage | Function | Rejects if |
|-------|----------|-----------|
| S1 | `stage1_tokenize` | Never fails — wraps `lex::collect_all_tokens` |
| S1' | `stage1_tokenize_with_synonyms` | synonym maps to >1 canonical token |
| S2 | `stage2_kind_classify` | non-`the` at top level; unknown kind; `@Kind` at top level |
| S2' | `stage2_kind_classify_with_grammar` | kind not in `grammar.sqlite.kinds` |
| S3 | `stage3_shape_extract` | concept block missing `intended to …`; no closing `.` |
| S4 | `stage4_contract_bind` | unterminated contract; empty contract prose |
| S5 | `stage5_effect_bind` | unterminated effect; empty effect names; non-Word effect name |
| S5b | `stage5b_favor_validate` | quality name not in `grammar.sqlite.quality_names` |
| S6 | `stage6_ref_resolve` | assembles final `NomFile` / `NomtuFile` |

### Closed Kind Set (KINDS const — must match grammar.sqlite.kinds)

`"function"`, `"module"`, `"concept"`, `"screen"`, `"data"`, `"event"`, `"media"`, `"property"`, `"scenario"`

### Two Pipeline Variants

```
BASIC (cmd_store_add default):          GRAMMAR-AWARE (sync uses this when grammar.sqlite exists):
  run_pipeline(src)                       run_pipeline_with_grammar(src, &conn)
  S1 tokenize                             S1' tokenize_with_synonyms → keyword_synonyms
  S2 kind_classify                        S2' kind_classify_with_grammar → kinds
  S3 shape_extract                        S3' shape_extract_with_grammar → clause_shapes
  S4 contract_bind                        S4  contract_bind
  S5 effect_bind                          S5  effect_bind
  S6 ref_resolve                          S5b favor_validate → quality_names
                                          S6  ref_resolve
```

Both produce identical `PipelineOutput::Nom(NomFile)` or `PipelineOutput::Nomtu(NomtuFile)`.

---

## nom-store Commands — Honest Status

| Command | Implementation | Status |
|---------|-------------|--------|
| `cmd_store_add` | `run_pipeline → upsert_concept_def / upsert_entity + persist_inline_bc_artifact` | ✅ Operational |
| `cmd_store_get` | `eprintln!("store get command disabled during V2 migration"); return 1` | ⛔ Disabled |
| `cmd_store_closure` | real BFS via `closure()` | ✅ Operational |
| `cmd_store_verify` | real reachability + status check | ✅ Operational |
| `cmd_store_stats` | `count_entities / count_concept_defs / body_kind_histogram / status_histogram` | ✅ Operational |
| `cmd_store_list` | `eprintln!("nom store list is disabled during V2 migration"); return 1` | ⛔ Disabled |
| `cmd_store_gc` | BFS keep-set + `DELETE FROM entries WHERE id = ?1` (targets legacy `entries` table, not `entities`) | ⚠️ Partial — wrong table |

---

## File Format Semantics

| Extension | Format | Scope | Destination |
|-----------|--------|-------|-------------|
| `.nomx` | Authored prose source | Input to S1-S6 | Pipeline entry |
| `.nom` | Tier-2 concept file | MACRO scope — `concept` kind blocks | `concepts.sqlite.concept_defs` |
| `.nomtu` | Tier-1 entity file | MICRO scope — function/data/module/etc blocks | `entities.sqlite.entities` |

### `.nomx` Example (prose English)

```nomx
the function fetch_url is
  intended to fetch the body of an https URL.
  given a url of text, returns text.
  requires response is not empty.
  benefit cache_hit, fast_path.
  hazard timeout.
```

### `.nom` Example (concept declaration)

```nom
the concept minimal_safe_agent is
  intended to compose a small set of tools an LLM can plan with safely.
  uses the concept agent_safety_policy,
       the @Function matching "fetch the body of an https URL",
       the function read_file matching "read text from a workspace path".
  exposes read_file, write_file.
  this works when the safety policy is composed.
  favor security then composability then speed.
```

---

## Flow-Style Syntax (Legacy `nom-parser` surface — DELETED)

`nom-parser` was an optional dep behind `features.self-host`. Both `nom-parser` and `nom-lexer` crates are **fully deleted in GAP-4 (2026-04-16)**. The `self-host` feature is also fully removed (no `[features]` section in `nom-cli/Cargo.toml`). No `nom-parser` or `nom-lexer` import exists anywhere in the workspace. The flow-style syntax they parsed is documented here for historical reference only.

All CLI parse surfaces now route through `parse_with_bridge_or_legacy()` which calls `nom_concept::stages::run_pipeline(source)` → `ast_bridge::bridge_to_ast()`. There is no fallback to the old flow-style parser.

### 10 Classifiers

`system`, `flow`, `store`, `graph`, `agent`, `test`, `nom`, `gate`, `pool`, `view`

### Key Constructs

```nom
need hash::argon2 where security>0.9      # Import dictionary word
require latency<50ms                       # Declare constraint
effects only [network database cpu]        # Whitelist side effects
flow request->hash->store->response        # Data flow pipeline
```

---

## Identity & Content-Addressing

```
Canonical hash = sha256(serde_json::to_vec(&entity_decl))
```

Hash is computed from the **JSON-serialized pipeline-output struct**, not raw source bytes. Both `EntityDecl` and `CompositionDecl` are hashed via `serde_json`. Hash prefixes (minimum 8 hex chars) work as short identifiers. Store supports: `add`, `closure` (BFS walk), `verify` (reachability), `gc` (garbage collect), `stats`.

**Note:** `store get` and `store list` are **disabled** during V2 migration.

---

## MECE Objectives System

Concepts declare ranked objectives. The MECE checker validates:
- **ME violation:** child concepts collide on the same axis
- **CE violation:** required axes not covered
- Registry of required axes per repo/scope maintained in `required_axes` table (DB1 concepts.sqlite)

---

## Gap Analysis (Current → Target, Ground Truth)

### GAP-0: Pipeline Wiring ✅ FUNCTIONALLY COMPLETE

`cmd_store_add` and `store sync` are wired to the S1-S6 pipeline with grammar-aware fallback. Both persist entity and concept rows to respective split-DB tiers. `persist_inline_bc_artifact` is called after each entity upsert (writes body bytes to `store/<hash>/` path if available). The grammar-aware variant (`run_pipeline_with_grammar`) is used by `sync_repo` when grammar.sqlite exists.

### GAP-1: Dict-Split Cleanup 🟡 SUBSTANTIALLY COMPLETE (with honest gaps)

**Shipped:**
- `NomDict` struct deleted (Session 15). All free functions migrated to `nom-dict/src/dict.rs`.
- All CLI test surfaces migrated off `NomDict`.
- Legacy integration tests in `nom-llvm`, `nom-types` gated with `#[cfg(any())]` — they do NOT compile or run.
- `link_bitcode_to_executable` **removed from `manifest.rs`** — no longer even stubbed; `nom build link` has no implementation.

**Remaining:**
- Legacy `entries` table still heavily used (~50+ SQL queries) for extract/score/stats/coverage/corpus.
- `cmd_store_gc` deletes from `entries` (wrong table — should delete from `entities` + cascade).
- `cmd_store_get` and `cmd_store_list` disabled — re-implementation deferred.
- Legacy `concepts` and `concept_members` tables not yet deleted from schema.

### GAP-2: Embedding / Semantic Resolver 🔴 CRITICAL PATH

`SemanticEmbedding` impl in `nom-resolver/src/intent.rs` always returns `Err(Unavailable)`. Blocks LSP embedding-ranked completion, semantic re-rank.

### GAP-3: Corpus Ingestion 🔴 BLOCKED

Zero production packages ingested. Blocked on: network access, ~50 GB disk, Windows DLL issues.

### GAP-4: `.nomx` Format Unification ✅ COMPLETE (2026-04-16, fully confirmed)

**Shipped:**
- `nom-parser` crate directory deleted (`crates/nom-parser/` gone). ✅
- `nom-lexer` crate directory deleted (`crates/nom-lexer/` gone). ✅
- All entries for both crates removed from workspace `Cargo.toml`, `nom-cli/Cargo.toml`, `nom-llvm/Cargo.toml`. ✅
- `self-host` feature **fully removed** from `nom-cli/Cargo.toml` — no `[features]` section exists. ✅ (not even a vestigial empty entry)
- `parse_with_bridge_or_legacy()` in `main.rs`: bridge-only — calls `nom_concept::stages::run_pipeline(source)` then `ast_bridge::bridge_to_ast()`. **No legacy fallback exists.**
- `link_bitcode_to_executable` stub removed from `manifest.rs`. ✅
- `read_extractable_atoms` removed. ✅
- Workspace is now **29 crates**. ✅

**Remaining limitations (not blockers for GAP-4):**
- Bridge (`ast_bridge.rs`, 584 lines) produces enriched but not fully imperative statement bodies — arithmetic, call/return, and conditional (if/then/else) recovery shipped; full imperative coverage is GAP-5a.
- `nom-llvm` integration tests remain `#[cfg(any())]` gated.

### GAP-5a: AST Bridge Body Recovery 🟢 CONTRACTS/EFFECTS SHIPPED

`ast_bridge.rs` handles: functions, arithmetic, call/return, prose-conditional (`if X then Y else Z`), `when/then/otherwise` conditionals, multi-statement bodies, comparison operators, **loop constructs** (`for-each`, `for-in`, `while-do`, `repeat-until`), and **contracts/effects propagation** (`requires`/`ensures` → `Describe`, `benefit`/`hazard` → `Effects` with `Good`/`Bad` modifiers). 19 bridge tests. Remaining: complex match expressions, nested multi-arg calls.

### GAP-5b: LLVM Binary Linking ✅ SHIPPED (2026-04-16)

`link_bitcodes(bitcode_blobs: &[Vec<u8>]) -> Result<LlvmOutput, LlvmError>` is implemented at `nom-llvm/src/lib.rs:48`. `compile_plans(plans: &[CompositionPlan]) -> Result<LlvmOutput, LlvmError>` is at line 84. The `nom build link` CLI subcommand is wired to `link_bitcodes()` at `main.rs:6521` — reads `.bc` files, links via inkwell, compiles with clang (llc + linker fallback), produces a native executable. 4 inline tests + 16 external tests cover multi-module linking scenarios.

### GAP-6: Real Planner 🟢 PIPELINE WIRING + FUSION SHIPPED

**Shipped:** Dead node elimination, duplicate node merging, node fusion (`fuse_pure_nodes`), variant specialization (`specialize_variants`), topology-aware concurrency strategy (`infer_concurrency_strategy`), type-flow validation, direct S1-S6 `PipelineOutput` planning via `plan_from_pipeline_output`, and (batch 6) `fuse_identity_nodes`, `fuse_consecutive_maps`, `collapse_single_branch`, `optimize_plan` orchestrator. 17 planner tests.

**Reality check:** `nom build compile` now uses direct pipeline planning for `.nomtu` module compositions when flows exist, then falls back to AST-bridge planning for concept/entity-only files. Resolver metadata-backed specialization is still heuristic.

### GAP-7: LSP MVP 🟢 EXACT SOURCE RANGES SHIPPED

Slices 1-7b + exact source ranges shipped. Hover: dict-backed entity lookup via `NOM_DICT`. Goto-definition: `origin_ref` / `authored_in` location with exact source ranges (batch 8). Enriched hover (batch 7): structured Markdown with contracts, effects, retry policy, format string, body_kind, quality scores. 31 LSP tests (up from 27). Missing: embedding-ranked completion (→ GAP-2).

### GAP-8: AI Authoring Loop Closure 🟡 NEWLY UNBLOCKED

Newly unblocked (batch 8): nom-llvm linking shipped in batch 2; combined with sufficient bridge fidelity, the ReAct loop can now produce real artifacts. Remaining: wire nom-llvm through nom-intent, replace `Rendered { bytes_hash }` stub with actual artifact store write.

### GAP-9–10: Self-Hosting + Bootstrap Fixpoint 🌟 ASPIRATIONAL

```
Stage-1: Rust compiler compiles Nom-authored lexer/parser → Stage-2 binary
Stage-2: Stage-2 compiles same source → Stage-3 binary
Fixpoint: hash(Stage-2) == hash(Stage-3) ← byte-identical proof
```

### GAP-11: Quality Registration CLI ✅ SHIPPED

`nom corpus register-axis` CLI live. `nom grammar add-quality` stores `metric_function`. All 20 baseline qualities have descriptive metric function names in `grammar.sqlite`.

---

## Dead-Disabled Code (⛔ Things That Look Done But Aren't)

| Item | File | Reality |
|------|------|---------|
| `link_bitcode_to_executable` | `manifest.rs` | **Removed** — replaced by `nom_llvm::link_bitcodes()` wired in `nom build link` |
| `cmd_store_get` | `store/commands.rs` | `eprintln! + return 1` stub |
| `cmd_store_list` | `store/commands.rs` | `eprintln! + return 1` stub |
| `cmd_store_gc DELETE` | `store/commands.rs` | Deletes from `entries` (legacy), not `entities` |
| `nom-llvm` inline tests (`context.rs`) | `nom-llvm/src/context.rs` | ✅ Un-gated (batch 4) — 3 tests now active |
| `nom-types tests` | `nom-types/src/lib.rs` | `#[cfg(any())]` — never compiles |
| `self-host feature` | `nom-cli/Cargo.toml` | **Fully removed** — no `[features]` section exists |

---

## Test Coverage Summary

| Suite | Tests | Status |
|-------|-------|--------|
| `nom-concept` (Phase E proofs P1-P7) | 7 proofs | ✅ |
| Corpus dashboard | 84/88 blocks (95.5%) | ✅ |
| Pattern catalog | 258/258 distinct, Jaccard max 0.273 | ✅ |
| `nom-intent` | 63 lib + 2 integration | ✅ |
| `nom-resolver` | 58 lib | ✅ |
| `nom-dict` | 94 lib | ✅ |
| `nom-concept` | 76+ lib | ✅ |
| `nom-llvm` external tests | 16 tests (builtins, enums, lists, strings, tuples) | ✅ All ungated |
| `nom-llvm` inline tests (`lib.rs`) | 5 tests (link_bitcodes, compile_plans, empty-returns-error) | ✅ |
| `nom-llvm` inline tests (`context.rs`) | 3 tests | ✅ Un-gated (batch 4) |
| `nom-types` | Legacy tests | ⛔ `#[cfg(any())]` disabled |
| E2E (`nom-cli`) | concept_demo, `store_cli` (4 tests), `dids_store_e2e` (1 test, replaces phase4_acceptance), store_sync, avif_ingest, bc_body_round_trip, mcp_smoke, etc. | Mostly ✅ (`parser_subset_probe.rs` deleted) |
| Self-host pipeline | 8 files converted to structural verification + 2 others; 10 total | ✅ |
| GAP-12 final wedges | 37 new tests: watermark_clause.rs, window_clause.rs, clock_domain_clause.rs, quality_declarations.rs | ✅ (batch 8) |
| LSP exact source ranges | 31 LSP tests (up from 27) | ✅ (batch 8) |
| **Total** | **1043 tests passing (single-threaded), 0 failed, 34 ignored** — batch 8 | ✅ |

**Windows caveat:** `nom-cli` test binaries link `nom-llvm` which needs LLVM-C.dll on PATH in
some test configurations. The `STATUS_DLL_NOT_FOUND` error seen locally is a Windows environment
issue, not a compiler bug. CI uses Ubuntu with llvm-18.

---

## Building and Running

### Prerequisites

- **Rust** (auto-installed via `rust-toolchain.toml`)
- **LLVM 18** (for `nom-llvm`; Windows needs LLVM-C.dll on PATH for `--target llvm` flows)

### Build

```bash
cd nom-compiler
cargo check --workspace     # Verify compilation (no LLVM DLL needed)
cargo build --release        # Full build
```

### Run the Pipeline

```bash
./target/release/nom store add examples/hello.nomx
./target/release/nom store sync examples/concept_demo
./target/release/nom build status examples/concept_demo
./target/release/nom build manifest examples/concept_demo --pretty
```

### Test

```bash
cargo test --workspace                        # Full suite
cargo test -p nom-concept                     # Core pipeline
cargo test -p nom-dict                        # Dictionary layer
cargo test -p nom-cli                         # E2E + self-host
```

---

## Multi-Language Corpus

The workspace declares tree-sitter grammars for **50 languages**: Rust, TypeScript, Python, C, C++, Go, Java, C#, Ruby, PHP, Scala, Haskell, OCaml, Julia, Bash, HTML, CSS, JSON, YAML, TOML, Lua, R, Zig, Elm, Objective-C, Verilog, Make, CMake, Racket, Erlang, D, Fortran, Proto, Elixir, Groovy, Swift, Dart, Nix, GLSL, GraphQL, LaTeX. Extraction via `nom-extract`.

---

## Development Conventions

### Code Style

- **Language:** Rust, edition 2024
- **Error handling:** `thiserror` for error types, `miette` + `ariadne` for diagnostics
- **Serialization:** `serde` + `serde_json`
- **CLI:** `clap` with derive macros
- **Database:** `rusqlite` (bundled SQLite)
- **LLVM:** `inkwell` (LLVM 18 backend)
- **Tracing:** `tracing` + `tracing-subscriber`

### GitNexus Integration

This project is indexed by **GitNexus**. Per `AGENTS.md`:

- **MUST** run `gitnexus_impact({target, direction: "upstream"})` before editing any symbol
- **MUST** run `gitnexus_detect_changes()` before committing
- Use `gitnexus_query({query})` to find execution flows
- Use `gitnexus_context({name})` for full symbol context

---

## Important Files

| File | Why It Matters |
|------|---------------|
| `nom_state_machine_report.md` | **Authoritative state report** — full ground-truth source analysis |
| `BLUEPRINT.md` | Definitive build plan, 45 technical decisions, 25 open questions |
| `SYNTAX.md` | Complete syntax reference — every keyword, operator, grammar rule |
| `Plan.md` | Roadmap — phase-by-phase implementation |
| `research/SPEC.md` | Authoritative language design specification |
| `research/NOMTU.md` | Dictionary database format specification |
| `nom-compiler/Cargo.toml` | Workspace definition — **29 crates** (nom-parser + nom-lexer deleted in GAP-4) |
| `nom-compiler/crates/nom-cli/src/main.rs` | 6323-line CLI dispatch (ground truth for all commands) |
| `nom-compiler/crates/nom-concept/src/stages.rs` | 2618-line S1-S6 pipeline (ground truth for parse stages) |
| `nom-compiler/crates/nom-dict/src/dict.rs` | 1245-line Dict API (ground truth for storage layer) |

---

## Self-Check Before Finishing

Before completing any code modification:

1. Run `gitnexus_impact` for all modified symbols
2. No HIGH/CRITICAL risk warnings ignored
3. `gitnexus_detect_changes()` confirms changes match expected scope
4. All d=1 (WILL BREAK) dependents were updated
5. Run `cargo check --workspace` — the workspace must compile cleanly
6. After committing: run `npx gitnexus analyze` to refresh index
