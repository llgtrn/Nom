# Nom Compiler — Full State Machine Report

> **Sources:** GitNexus MCP (6098 symbols · 15340 edges · 300 flows · Nom index),  
> all `research/0X-*.md` docs, `CYCLE-3-MIGRATION-SPEC.md`, and direct crate file inspection.  
> **Date:** 2026-04-16
> **Latest update:** 2026-04-16 — Batch 10. **GAP-5a COMPLETE** — match expression recovery (`parse_match_expr`, 4 tests) + nested multi-arg call recovery (`split_args_balanced`, 3 tests); ast_bridge.rs now 1487 lines. **GAP-6 metadata specialization SHIPPED** — `ResolverMetadata` struct, kind-aware scoring, contract validation node insertion, `optimize_plan_with_context`; 25 planner tests. **GAP-1c advanced** — `find_entities` canonical filter query on entities table, 5 legacy functions deprecated, 103 dict tests. **1067 tests passing (single-threaded), 0 failed, 34 ignored.**

---

## 0. Latest Session Update — 2026-04-16

**Batch 10 (2026-04-16 — 4-agent parallel execution):**
- **GAP-5a COMPLETE** — match expression recovery: `parse_match_expr` lowers multi-arm `when/then` chains to chained `IfExpr` via `else_ifs`; 4 new tests. ✅
- **GAP-5a COMPLETE** — nested multi-arg call recovery: `split_args_balanced` tracks paren depth for comma splitting; 3 new tests. ✅
- `ast_bridge.rs` now **1487 lines** with 26 bridge tests (was 753 lines, 19 tests). ✅
- **GAP-6 COMPLETE** — resolver metadata-backed specialization: `ResolverMetadata` struct, `kind_adjusted_score` helper, contract validation node insertion, `optimize_plan_with_context` orchestrator; 25 planner tests (was 17). ✅
- **GAP-1c advanced** — `find_entities` canonical filter query on `entities` table; `get_entry`, `find_entries`, `list_partial_ids`, `find_by_word`, `find_by_body_kind` all deprecated; 4 blocked migrations documented; 103 dict tests (was 98). ✅
- **1067 tests passing (single-threaded), 0 failed, 34 ignored** (up from 1050). ✅

**Batch 8 (2026-04-16T32:00 JST):**
- GAP-12 watermark clause (streaming): S5j extraction, `watermark_clause.rs` test file, part of final 4 wedges. ✅
- GAP-12 window-aggregation clause: S5k extraction, `window_clause.rs` test file, 37 new tests total across 4 files. ✅
- GAP-12 clock-domain clause: S5l extraction, `clock_domain_clause.rs` test file. ✅
- GAP-12 QualityName-registration formalization: S5m extraction, `quality_declarations.rs` test file. ✅
- **GAP-12 is now 11/11 COMPLETE — entire grammar wedge backlog cleared.** ✅
- S5 extraction chain is now S5a–S5m (13 sub-stages). ✅
- LSP: exact source ranges for goto-definition shipped; 31 LSP tests (up from 27). ✅
- **GAP-8 newly unblocked** — nom-llvm linking shipped in batch 2; ReAct loop can now produce real artifacts. ✅
- **1043 tests passing (single-threaded), 0 failed, 34 ignored.** ✅

**Batch 7 (2026-04-16T30:00 JST):**
- GAP-12 exhaustiveness check: `WhenClause` extraction (S5i) + `check_exhaustiveness()` validator, 9 tests. ✅
- **7/11 grammar wedges now shipped** (retry-policy, @Union, format-string, nested-record-path, wire-field-tag, pattern-shape, exhaustiveness). ✅
- S5 extraction chain is now S5a–S5i (9 sub-stages). ✅
- LSP enriched hover: structured Markdown with contracts/effects/retry/format/body_kind/scores; 27 LSP tests (up from 19). ✅
- GAP-1c: `resolve_prefix` now queries `entities` table first; `find_entities_by_body_kind` added; `find_by_word` / `find_by_body_kind` deprecated. ✅
- **1002 tests passing, 0 failed** — crossed the 1000-test milestone. ✅

**Batch 6 (2026-04-16T28:00 JST):**
- GAP-12 nested-record-path (`accesses`): S5f extraction, `accesses` field on entity decl, 9 tests. ✅
- GAP-12 wire-field-tag (`field X tagged Y`): S5g extraction, `FieldTag` struct, 9 tests. ✅
- GAP-12 pattern-shape (`shaped like`): S5h extraction, 8 tests. ✅
- Planner fusion: `fuse_identity_nodes`, `fuse_consecutive_maps`, `collapse_single_branch` + `optimize_plan` orchestrator, 17 planner tests. ✅
- **6/11 grammar wedges now shipped** (retry-policy, @Union, format-string, nested-record-path, wire-field-tag, pattern-shape). ✅
- S5 extraction chain is now S5a–S5h (8 sub-stages). ✅
- **985 tests passing, 0 failed, 33 ignored** (up from 952 after batch 5). ✅

**Batch 5 (2026-04-16T26:00 JST):**
- GAP-12 `@Union` sum-type: S5d extraction, `UnionVariants` struct, 8 tests — first full wedge shipped end-to-end. ✅
- GAP-12 format-string interpolation: S5e extraction, skip-on-prose rule, 9 tests — second full wedge shipped. ✅
- Bridge contracts/effects propagation: `requires`/`ensures` → `Describe`, `benefit`/`hazard` → `Effects` with `Good`/`Bad` modifiers. 4 new bridge tests; 19 total bridge tests. ✅
- **3 grammar wedges now shipped** (retry-policy from batch 3 + @Union + format-string from batch 5). ✅
- Established grammar wedge pattern: lexer token → Tok enum → S5 extraction → EntityDecl field → baseline.sql → tests. ✅
- `ast_bridge.rs` now handles: functions, arithmetic, calls, conditionals, loops, multi-statement bodies, contracts, effects. ✅
- **952 tests passing, 0 failed, 33 ignored** (up from 931 after batch 4). ✅

**Batch 4 (2026-04-16T24:00 JST):**
- `nom-llvm/src/context.rs` inline test module un-gated — zero `#[cfg(any())]` gates remaining in entire codebase. ✅
- 20 unit tests in nom-llvm (was 17 after batch 3). ✅
- 8 self-host test files (`self_host_pipeline`, `self_host_ast`, `self_host_codegen`, `self_host_parser`, `self_host_planner`, `self_host_smoke`, `self_host_verifier`, `self_host_parse_smoke`) converted from parser-dependent to structural verification. ✅
- `parser_subset_probe.rs` deleted (was historical artifact after nom-parser removal). ✅
- GAP-5a loop recovery: `for-each`, `for-in`, `while-do`, `repeat-until` constructs → 15 bridge tests (was 10). ✅
- GAP-1c: `status_histogram()` migrated to `entities` table; `list_partial_ids()` deprecated. ✅
- **931 tests passing, 0 failed, 33 ignored** (was 899 before session, 931 after batch 4). ✅

**Batch 3 (2026-04-16T23:30 JST):**
- Status column added to `entities` table — migration guard in `dict.rs:270` (`ALTER TABLE entities ADD COLUMN status TEXT NOT NULL DEFAULT 'complete'`). ✅
- `RetryPolicy` struct defined in `nom-concept/src/lib.rs:119` with `max_attempts`, `backoff_ms`, `jitter`, `on_errors` fields — GAP-12 retry-policy grammar wedge begun; `retry_policy: Option<RetryPolicy>` field on entity decl. ✅
- `retry-policy` pattern added to `grammar.sqlite` baseline data. ✅

**Batch 2 (2026-04-16T22:00 JST):**
- `store_cli.rs` fully rewritten: `NomDict` → `Dict`, flow-style fixtures → `.nomx`, 4 tests passing, file-level `#![cfg(any())]` gate removed. ✅
- `phase4_acceptance.rs` **renamed to `dids_store_e2e.rs`** and rewritten with `.nomx` + `Dict`; `#[ignore]` removed; 1 test passing. ✅ (original `phase4_acceptance.rs` no longer exists)
- `nom build link` subcommand **fully wired**: reads `.bc` files → `link_bitcodes()` → clang compilation with `llc` + system linker fallback → executable output. ✅ (SHIPPED — not a stub)
- GAP-1c first migration: `cmd_store_stats` now reads from canonical `entities` table (not legacy `entries`). 23+ legacy-table functions across 7 files inventoried. ✅
- All 5 nom-llvm external test files migrated (`builtins.rs`, `enums.rs`, `lists.rs`, `strings.rs`, `tuples.rs`): 16 tests, zero `#[cfg(any())]` in external test files. ✅

**Batch 1 (2026-04-16T20:00 JST):**
- Phase 0: 36 stale files removed from workspace (error logs, test outputs, Python scripts). ✅
- GAP-5a: `ast_bridge.rs` extended to **753 lines** — added conditionals (`if/then/else`, `when/then/otherwise`), multi-statement bodies, comparison operators. 10 bridge tests. ✅
- GAP-5b: `link_bitcodes()` + `compile_plans()` implemented in `nom-llvm/src/lib.rs` (lines 48 + 84) — real multi-module bitcode linking via inkwell. 4 new inline tests in lib.rs; plus the 5 external test files noted above. ✅
- One `#[cfg(any())]` remains in `nom-llvm/src/context.rs:239` (inline test module that still depends on removed `nom-parser`; external test files are fully ungated). ⚠️
- GAP-1c audit: 23+ legacy `entries`/`concepts`-table functions across 7 files inventoried and documented. ✅

**Previously completed (2026-04-16T21:00 JST):**
- `nom-cli/src/store/commands.rs` migrated to Dict free functions; `Dict` exports added.
- Both `nom-parser` and `nom-lexer` crate directories deleted — workspace is **29 crates**.
- `self-host` feature fully removed from `nom-cli/Cargo.toml`.
- `nom-planner` exposes `plan_from_pipeline_output(PipelineOutput) -> CompositionPlan` for `.nomtu` module compositions.
- `nom build compile` plans directly from `nom-concept` S1-S6 `PipelineOutput`; falls back to AST bridge for concept/entity-only files.
- `nom-lsp` advertises `textDocument/definition` and resolves hover + goto-definition from split dict via `NOM_DICT`.
- `nom-cli` runs on larger worker stack (fixes Windows `STATUS_STACK_OVERFLOW`).

**Verification evidence (batch 8):**
- `cargo check --workspace` → `Finished dev profile [unoptimized + debuginfo] target(s) in 20.54s` (0 errors). ✅
- `cargo test --workspace -- --test-threads=1` → **1043 tests passing, 0 failed, 34 ignored**. ✅
- GAP-12 complete: watermark S5j, window-aggregation S5k, clock-domain S5l, QualityName S5m — 37 new tests across 4 test files. ✅
- GAP-12 11/11 COMPLETE — entire grammar wedge backlog cleared. S5 extraction chain is S5a–S5m (13 sub-stages). ✅
- LSP exact source ranges for goto-definition shipped; 31 LSP tests. ✅
- GAP-8 newly unblocked (nom-llvm linking + bridge fidelity sufficient for real artifact production). ✅

**Verification evidence (batch 7):**
- `cargo check --workspace` → `Finished dev profile [unoptimized + debuginfo] target(s) in 0.73s` (0 errors). ✅
- `cargo test --workspace` → **1002 tests passing, 0 failed**. ✅
- GAP-12 exhaustiveness check: S5i extraction, `WhenClause`, `check_exhaustiveness()`, 9 tests. ✅
- LSP enriched hover: structured Markdown with contracts/effects/retry/format/body_kind/scores; 27 LSP tests. ✅
- GAP-1c `resolve_prefix`: queries `entities` first; `find_entities_by_body_kind` added; `find_by_word`/`find_by_body_kind` deprecated. ✅
- 7/11 grammar wedges shipped. S5 extraction chain is S5a–S5i (9 sub-stages). ✅

**Verification evidence (batch 6):**
- `cargo check --workspace` → `Finished dev profile [unoptimized + debuginfo] target(s) in 1.10s` (0 errors). ✅
- `cargo test --workspace` → **985 tests passing, 0 failed, 33 ignored**. ✅
- GAP-12 nested-record-path: S5f extraction, `accesses` field, 9 tests. ✅
- GAP-12 wire-field-tag: S5g extraction, `FieldTag` struct, 9 tests. ✅
- GAP-12 pattern-shape: S5h extraction, `shaped like`, 8 tests. ✅
- Planner: `fuse_identity_nodes`, `fuse_consecutive_maps`, `collapse_single_branch`, `optimize_plan`, 17 planner tests. ✅
- 6/11 grammar wedges shipped. S5 extraction chain is S5a–S5h (8 sub-stages). ✅

**Verification evidence (batch 5):**
- `cargo check --workspace` → `Finished dev profile [unoptimized + debuginfo] target(s) in 1.08s` (0 errors). ✅
- `cargo test --workspace` → **952 tests passing, 0 failed, 33 ignored**. ✅
- GAP-12 `@Union`: S5d extraction, `UnionVariants` struct, 8 tests. ✅
- GAP-12 format-string: S5e extraction, skip-on-prose rule, 9 tests. ✅
- Bridge contracts/effects: requires/ensures → Describe, benefit/hazard → Effects (Good/Bad). 19 total bridge tests. ✅
- Grammar wedge pattern documented: lexer token → Tok → S5 extraction → EntityDecl field → baseline.sql → tests. ✅
- `ast_bridge.rs`: handles functions, arithmetic, calls, conditionals, loops, multi-statement, contracts, effects. ✅

**Verification evidence (batch 4):**
- `cargo check --workspace` → `Finished dev profile` (0 errors). ✅
- `cargo test --workspace` → **931 tests passing, 0 failed, 33 ignored**. ✅
- `grep -rn "cfg(any())" crates/ --include="*.rs"` → zero matches. ✅
- `ast_bridge.rs`: **753+ lines**, 15 bridge tests (loop constructs added). ✅
- `link_bitcodes` + `compile_plans`: exported from `nom-llvm/src/lib.rs` at lines 48 and 84 (verified). ✅
- `nom build link` wired to `link_bitcodes()` at `main.rs:6521` (verified). ✅
- `dids_store_e2e.rs` exists; `phase4_acceptance.rs` absent (verified). ✅
- `parser_subset_probe.rs` absent (deleted). ✅
- Status column migration in `dict.rs:270` (verified). ✅
- `RetryPolicy` struct in `nom-concept/src/lib.rs:119` (verified). ✅
- Zero `#[cfg(any())]` gates anywhere in codebase. ✅
- Workspace crate count: 29. ✅

---

## 1. Mission & Target State (condensed)

| Axis | Current | Target |
|------|---------|--------|
| Source format | `.nomx` S1-S6 pipeline shipped (GAP-0 wiring in progress) | Single canonical `.nomx` → determines `.nom` concepts (macro scope) first → then resolves `.nomtu` entities (micro scope) → compiles to `.bc` |
| Resolver | Alphabetical-smallest-hash stub | Embedding-driven semantic re-rank |
| Dictionary | Legacy single `nomdict.db`; dict-split 52/60 free fns ported toward 3-DB target | **3-DB target:** `grammar.sqlite` (grammar registry) + `concepts.sqlite` (macro/concept tier) + `entities.sqlite` (micro/entity tier, 100M+ rows); all legacy tables deleted |
| Grammar registry | `grammar.sqlite` shipped — 258 patterns, 9 kinds, 43 keywords (one of the 3 target DBs) | Complete catalog in `grammar.sqlite`, zero Rust bundled data, all quality axes registered |
| Self-hosting | Stage-1 ✅: LLVM self-hosting lexer compiles end-to-end | Bootstrap fixpoint: Stage-2 compiler output == Stage-3 compiler output (byte-identical proof) |
| Corpus ingestion | 0 production packages; separate flow from authored-source pipeline | External repos → tree-sitter extract → pre-parsed entities → `entities.sqlite` at 100M+ rows; `.bc` compiled and stored separately |
| LSP | Slices 1-7a shipped | Full hover/completion/goto-def with embedding ranking |
| AI authoring loop | ReAct stub + DictTools smoke test | Deterministic verify→build→bench→flow loop |

---

## 2. Component Registry — All 29 Crates

### 2.1 Language Frontend (Parse & Classify)

#### `nom-lexer` ✅ DELETED (GAP-4 complete, 2026-04-16)
| Aspect | Detail |
|--------|--------|
| **State** | **Crate directory deleted.** Previously tokenised source files for the OLD `nom-parser` flow-classifier grammar. No active parse path depends on this crate. `nom-concept`'s internal prose-English lexer (handling `the`, `is`, `uses`, `composes`, `@Kind`, etc.) is the only active lexer. |

#### `nom-parser` ✅ DELETED (GAP-4 complete, 2026-04-16)
| Aspect | Detail |
|--------|--------|
| **State** | **Crate directory deleted.** All `Cargo.toml` entries removed from workspace, `nom-cli`, and `nom-llvm`. No `nom-parser` import exists anywhere in the workspace. `parse_with_bridge_or_legacy()` is bridge-only (calls `nom_concept::stages::run_pipeline` then `ast_bridge::bridge_to_ast()`). No legacy fallback. |
| **Note** | Flow-style syntax (`flow request->hash->store->response`, classifier keywords) is documented in INIT.md as historical reference only. |

#### `nom-ast`
| Aspect | Detail |
|--------|--------|
| **Function** | Shared AST node types used across pipeline stages |
| **Input / Output** | Pure data structures; consumed by concept, codegen, verifier |
| **State** | Shipped |
| **Gap** | None directly; depends on nomx v1/v2 merge completing |

#### `nom-grammar`
| Aspect | Detail |
|--------|--------|
| **Function** | Schema + connection API + query helpers for `grammar.sqlite` |
| **Features** | 7 tables: `schema_meta`, `keywords`, `keyword_synonyms`, `clause_shapes`, `kinds`, `quality_names`, `patterns`; `resolve_synonym`, `is_known_kind`, `is_known_quality`, `search_patterns` (Jaccard), `fuzzy_tokens`, `jaccard` |
| **Input** | SQLite path; SQL queries from callers |
| **Output** | `RegistryCounts`, `PatternMatch`, row results |
| **Data shipped** | `data/baseline.sql` — 9 kinds + 20 quality_names + 43 keywords + 7 keyword_synonyms + 43 clause_shapes + 258 patterns |
| **State** | Shipped; awareness-only (zero grammar data in Rust source) |
| **Gap** | `quality_names.metric_function` column still nullable — awaiting `nom corpus register-axis` CLI; 11 wedge shapes queued |

---

### 2.2 Staged Source Parsing Pipeline (`nom-concept`) ⚠️ WIRING IN PROGRESS (GAP-0)

> **Critical fact from source code:** `cmd_store_add` now calls the S1-S6 pipeline (`nom-concept::stages::run_pipeline`) and writes split-dictionary rows directly. The remaining GAP-0 work is follow-through: richer entity materialization, artifact compilation, and cleanup of leftover legacy parser callers.

This is the new prose-English `.nomx` parser. **GitNexus confirmed** the orchestrator is `run_pipeline_with_grammar` in `stages.rs:1164`. Its internal lexer lives in `nom-concept/src/lib.rs` and handles English prose tokens (`the`, `is`, `uses`, `composes`, `@Kind`, `at-least`, number literals, quoted strings).

**File Format Pipeline (canonical authored-source flow):**

```
  Author writes:  myfile.nomx
  (prose English: "the concept auth is intended to..."
                  "the function login_user is given email, returns session.")
            │
  ┌─────────▼──────────┐
  │  S1 tokenize        │  stage1_tokenize_with_synonyms
  │  (+ synonym rewrite)│  → consults grammar.sqlite.keyword_synonyms
  └─────────┬──────────┘
            │ TokenStream
  ┌─────────▼──────────┐
  │  S2 kind_classify   │  → validates against grammar.sqlite.kinds
  └─────────┬──────────┘
            │ ClassifiedStream
  ┌─────────▼──────────┐
  │  S3 shape_extract   │  → asserts clause_shapes rows exist per kind
  └─────────┬──────────┘
            │ ShapedStream
  ┌─────────▼──────────┐
  │  S4 contract_bind   │  → requires/ensures clauses bound to decl
  └─────────┬──────────┘
            │ ContractedStream
  ┌─────────▼──────────┐
  │  S5 effect_bind     │  → hazard/benefit/favor clauses extracted
  └─────────┬──────────┘
            │ EffectedStream
  ┌─────────▼──────────┐
  │  S6 ref_resolve     │  → typed-slot refs pinned to name@hash
  └─────────┬──────────┘
            │ PipelineOutput
            │
            │  STEP 1 (MACRO SCOPE — first):  concept declarations
            ├─► PipelineOutput::Nom(NomFile)
            │       │  → .nom file format
            │       │  → concepts.sqlite → concept_defs table
            │       │     { name, intent, index_into_db2, exposes, acceptance }
            │       │
            │       │  Once concepts are determined ↔ allowed to resolve micro scope:
            │       ↓
            │  STEP 2 (MICRO SCOPE — after concepts settled):  entity declarations
            └─► PipelineOutput::Nomtu(NomtuFile)
                    │  → .nomtu file format
                    │  → entities.sqlite → entities table
                    │     { hash, word, kind, signature, contracts, composed_of }
```

> **Note on `.bc` compilation:** Compiling entities to LLVM bitcode (`.bc`) is a **separate step** that happens AFTER entity rows are stored. For authored `.nomtu` files the compilation is local; for externally-ingested corpus files it goes through the corpus ingestion flow (see §2.7). The artifact store is populated independently of the S1-S6 parse pipeline.

**File format semantics:**
- **`.nomx`** — authored prose source. Input to S1-S6 pipeline.
- **`.nom`** — Tier-2 concept file. Macro scope. `concept` kind blocks. → `concepts.sqlite.concept_defs`.
- **`.nomtu`** — Tier-1/0 entity file. Micro scope. `function / data / module / screen / event / media / property / scenario` blocks. → `entities.sqlite.entities`. May later be compiled to `.bc` by `nom-llvm`.


**Additional modules in `nom-concept`:**
| Module | Function |
|--------|----------|
| `acceptance.rs` | `PredicateBinding` — W49 quantifier vocab (`every/no/some/at-least N`) |
| `closure.rs` | BFS dependency closure; `ClosureError`, `Color` (graph coloring) |
| `flow_edge.rs` | T2.1 — pure-data structural smell detection: `ConsecutiveDuplicate`, `LoopReference`, `SelfReference` |
| `mece.rs` | MECE validator — enforces required axis coverage at concept layer |
| `strict.rs` | Strictness lane (A1–A6); `NOMX-S*` error codes |
| `stages.rs` | `StageId` enum, `PipelineOutput` enum; all Sx functions |

**State:** A1/A2/A3/A4/A6 closed. A5 pending refactor. Grammar-aware S3 now validates both clause-shape presence and required-clause presence, with coverage in `clause_shape_guard.rs`. `cmd_store_add`, `nom build manifest` effect collection, and `nom build compile` planner wiring now use `nom_concept::stages::run_pipeline`. Remaining gaps are around downstream artifact generation, bridge fidelity, and replacement fixtures for gated legacy tests.  
**Gap:** See GAP-5 — the parser deletion is complete, but the `.nomx` ingest path still needs full body recovery and final `.bc`/linking follow-through.

---

### 2.3 Resolution & Verification

#### `nom-resolver`
| Aspect | Detail |
|--------|--------|
| **Function** | Match `uses the @Kind matching "…"` typed-slot refs to dictionary entities |
| **Features** | `IntentResolver` trait; `JaccardOverIntents` (deterministic, ships); `SemanticEmbedding` stub (errors `Unavailable`) |
| **Input** | Intent prose string + kind + confidence threshold |
| **Output** | `IntentRow` — hash of best-matching entity |
| **State** | T3.1 shipped; 58 lib tests green |
| **Gap** | **Production embedding backend** not wired (T4.2 gate); confidence default open question |

#### `nom-verifier`
| Aspect | Detail |
|--------|--------|
| **Function** | Per-pass invariant checking |
| **Features** | S1: every byte consumed; S2: exactly one kind; S3: required clauses; S4: contract clause termination; S5: boon/hazard consistency; S6: every ref resolves |
| **State** | Shipped |
| **Gap** | Solver-backed contract checks (B.requires ⇐ A.ensures) queued for T2.x |

---

### 2.4 Code Generation & Runtime

#### `nom-codegen`
| Aspect | Detail |
|--------|--------|
| **Function** | Host-language AST → intermediate representation |
| **Input** | AST nodes |
| **Output** | IR for LLVM |
| **State** | Shipped |

#### `nom-llvm`
| Aspect | Detail |
|--------|--------|
| **Function** | LLVM backend; emits bitcode `.bc` |
| **Features** | `compile_block_stmt`, `type_store_size`, `resolve_type`, `compile_source_to_bc`; type store size calculation; expression codegen |
| **Input** | IR from `nom-codegen` |
| **Output** | LLVM bitcode written to artifact store |
| **Key process (GitNexus)** | `Compiles_point_struct → Nom_string_type` (8 steps, cross-community) |
| **State** | Self-hosting lexer end-to-end through LLVM ✅ |
| **Gap** | Aesthetic backends (image/audio/video/3D/typography) are aspirational `body_kind` targets |

#### `nom-runtime`
| Aspect | Detail |
|--------|--------|
| **Function** | Runtime support for compiled Nom programs |
| **State** | Scaffold shipped |
| **Gap** | Largely stubbed; production runtime work gates on bootstrap |

#### `nom-planner`
| Aspect | Detail |
|--------|--------|
| **Function** | Composition planning — fuse, reorder, specialize pipeline steps |
| **State** | Shipped — `plan_from_pipeline_output`, dead-node elimination, `fuse_pure_nodes`, `specialize_variants`, `infer_concurrency_strategy`, `fuse_identity_nodes`, `fuse_consecutive_maps`, `collapse_single_branch`, `optimize_plan` orchestrator; 17 planner tests |
| **Gap** | Resolver metadata-backed specialization; self-host tests (`self_host_planner.rs`) exist |

---

### 2.5 Dictionary & Storage Layer

#### `nom-dict`
| Aspect | Detail |
|--------|--------|
| **Function** | Dual-tier SQLite connection manager + free-function API |
| **Features** | `Dict { concepts: Connection, entities: Connection }` struct; `open_dir`, `open_paths`, `open_in_memory` constructors; 52 public free functions ported (44 migration-surface helpers) |
| **Input** | DB paths / in-memory |
| **Output** | `Entry`, `Concept`, `ConceptDef`, `Entity`, `EntryFilter`, etc. |
| **State** | 52/60 free fns ported; **8 legacy NomDict methods remain** (Cycle 3 target) |

**8 outstanding NomDict → `&Dict` migrations:**

| # | Method | Risk | Consumers |
|---|--------|------|-----------|
| 1 | `upsert_entry` | 🔴 HIGH | nom-app (20+ sites), nom-corpus (1) |
| 2 | `upsert_entry_if_new` | 🔴 HIGH | nom-cli (1), nom-corpus (1) |
| 3 | `find_entries` | 🔴 HIGH | nom-cli (2), nom-dict self |
| 4 | `get_entry` | 🟡 MEDIUM | nom-app (3) startup-critical |
| 5 | `bulk_upsert` | 🟡 MEDIUM | none (future corpus load) |
| 6 | `add_graph_edge` | 🟢 LOW | none |
| 7 | `add_translation` | 🟢 LOW | none |
| 8 | `bulk_set_scores` | 🟢 LOW | none |

**Dictionary Data Model:**

```
entities.sqlite (DB2)
├── entities            ← canonical (hash PK, name, kind, signature, …)
├── entry_scores        ← 11 quality dimensions (T3.2 extended)
├── entry_meta
├── entry_signatures
├── entry_security_findings
├── entry_refs
├── entry_graph_edges   ← 28 edge types (Calls, Imports, Writes, …)
├── entry_translations
└── entries             ← LEGACY — scheduled for deletion (dict-split S8)

concepts.sqlite (DB1)
├── concept_defs        ← canonical (name PK, repo_id, intent, index_into_db2, …)
├── required_axes       ← MECE registry per scope
├── dict_meta
├── concepts            ← LEGACY — scheduled for deletion
└── concept_members     ← LEGACY — scheduled for deletion

grammar.sqlite (registry)
├── schema_meta
├── keywords
├── keyword_synonyms
├── clause_shapes
├── kinds
├── quality_names
└── patterns

~/.nom/store/<hash>/body.{bc,avif,mp4,wav,svg,…}   ← artifact store
```

#### `nom-store` (via `nom-cli/src/store/`)
> Not a crate — lives as modules inside `nom-cli` (Store community in GitNexus):

| Module | Key Functions |
|--------|---------------|
| `commands.rs` | `compile_source_to_bc`, `contract_from_decl` |
| `materialize.rs` | `materialize_concept_graph_from_db`, `collect_resolved_hashes_from_index` |
| `resolve.rs` | Store-level resolution helpers |
| `sync.rs` | Sync/freshness tracking |
| `add_media.rs` | Media artifact ingestion |

---

### 2.6 Intent & AI Loop

#### `nom-intent`
| Aspect | Detail |
|--------|--------|
| **Function** | ReAct/agentic loop driving the AI authoring pipeline |
| **Features** | `DictTools { query, render }` → `Rendered { target, bytes_hash }`; `Observation::Error` vs `Reject` distinction; SHA-256 plan hash (64-char hex); `StubAdapter`, `NomCli`, `MCP` concrete `ReAct` impls |
| **Input** | Agent action + dict state |
| **Output** | `Observation` (success / error) |
| **State** | T2.2 shipped; 63 lib + 2 integration tests green |
| **Gap** | Real bytecode-linking + binary emission ship in slice-3c-full (nom-llvm wiring) |

#### `nom-flow`
| Aspect | Detail |
|--------|--------|
| **Function** | Flow-step recording tree; captures execution flows per build/test |
| **State** | Scaffold; flow-capture toggle is an open question |

#### `nom-graph`
| Aspect | Detail |
|--------|--------|
| **Function** | Graph operations over `entry_graph_edges` (28 edge types, BFS closure) |
| **Features** | Durability design spec at `docs/superpowers/specs/2026-04-14-graph-durability-design.md`; GraphRAG agentic design at companion spec |
| **State** | Schema live; graph API active; durability + RAG design committed |

---

### 2.7 Corpus & Ingestion

#### `nom-corpus`
| Aspect | Detail |
|--------|--------|
| **Function** | Corpus ingestion pipeline — pull packages, extract entities, write to dict |
| **Features** | `Ecosystem.flag_value`, `ecosystem_from_str`; `upsert_entry` / `upsert_entry_if_new` calls; skip-list + checkpoint |
| **Input** | Package ecosystem identifier + network |
| **Output** | Rows in `entities` table with `status = 'partial'` |
| **State** | Schema and pipeline shape designed; blocked on network access, ~50 GB free disk, Windows DLL fix |
| **Gap** | M6 corpus pilot (PyPI top-100) not started due to external gates |

#### `nom-extract`
| Aspect | Detail |
|--------|--------|
| **Function** | Extract source-level entities from foreign-language codebases (50+ languages via tree-sitter) |
| **Features** | 30+ tree-sitter grammars (Rust, TS, Python, C, C++, Go, Java, C#, Ruby, PHP, Scala, Haskell, OCaml, Julia, Bash, HTML, CSS, JSON, YAML, TOML, Lua, R, Zig, Elm, ObjC, Verilog, Make, CMake, Racket, Erlang, D, Fortran, Proto, Elixir, Groovy, Swift, Dart, Nix, GLSL, GraphQL, LaTeX) |
| **State** | Grammars declared in `Cargo.toml`; extraction logic scaffold |
| **Gap** | 50-language extraction not production-validated |

---

### 2.8 Scoring & Security

#### `nom-score`
| Aspect | Detail |
|--------|--------|
| **Function** | Compute / store 11 quality dimension scores per entity |
| **Schema** | `entry_scores`: security, reliability, performance, readability, testability, portability, composability, maturity, overall_score + T3.2 new: quality, maintenance, accessibility |
| **State** | T3.2 schema shipped; population pipeline blocked on corpus pilot (T4.1) |
| **Gap** | ML-derived scores wait for embedding gate |

#### `nom-security`
| Aspect | Detail |
|--------|--------|
| **Function** | Security finding storage and analysis |
| **Schema** | `entry_security_findings` table |
| **State** | Schema live; analysis pipeline TBD |

---

### 2.9 Language Server Protocol

#### `nom-lsp`
| Aspect | Detail |
|--------|--------|
| **Function** | LSP server for `.nomx` authoring assistance |
| **Features** | Slices 1-6: stdio server, classify CLI, agentic-RAG markdown rendering, `executeCommand` handler, ReAct adapter trait + stubs; Slice 7a: pattern-driven completion via `search_patterns` backend; Slice 7b: dict-backed hover + goto-definition; Batch 7: enriched hover — structured Markdown with contracts/effects/retry/format/body_kind/scores (27 LSP tests); Batch 8: exact source ranges for goto-definition (31 LSP tests) |
| **Input** | LSP JSON-RPC over stdio |
| **Output** | Completion items, hover, diagnostics, execute-command |
| **State** | Slices 1-7b + exact source ranges shipped (31 tests) |
| **Gap** | Embedding-ranked completion (→ GAP-2) |

#### `nom-search`
| Aspect | Detail |
|--------|--------|
| **Function** | Text search over dictionary & patterns |
| **Features** | Jaccard fuzzy token overlap backend (deterministic); `nom grammar pattern-search` CLI |
| **State** | Shipped; deterministic floor in place |
| **Gap** | Embedding-driven semantic search is planned layer on top |

---

### 2.10 CLI

#### `nom-cli`
This is the primary user-facing binary. **GitNexus** identifies `Commands` enum at `main.rs:64-384` and `BuildCmd` at `main.rs:387-494`.

**Command surface (confirmed shipped):**

| Namespace | Subcommands |
|-----------|-------------|
| `nom build` | status, manifest, dream |
| `nom store` | add, list, sync |
| `nom grammar` | init, import, status, add-kind, add-keyword, add-synonym, add-quality, add-clause-shape, add-pattern, pattern-list, pattern-show, pattern-stats, pattern-search |
| `nom concept` | list, show, delete |
| `nom corpus` | register-axis (planned), embed (planned) |
| `nom author` | (Dict dual-path bridging — Cycle 3) |
| `nom run` | llvm |
| `nom quality` | (quality report) |
| `nom report` | (build report) |
| `nom check` | (strict check) |
| `nom extract` | (tree-sitter extraction) |
| `nom test` | (pipeline test) |
| `nom mcp` | (MCP tool server — LLM entry search via `EntryFilter`) |
| `nom media` | import |
| `nom locale` | |
| `nom fmt` | |

**Key execution flows (GitNexus):**

| Process | Steps | Type |
|---------|-------|------|
| `Cmd_store_add → compile_source_to_bc → contract_from_decl → ModuleCompiler` | 5 | cross-community |
| `Cmd_store_add → CompositionPlan → To_nomiz` | 4 | cross-community |
| `Main → collect_resolved_hashes_from_index → materialize_concept_graph_from_db` | 4 | cross-community |
| `Build_report → collect_resolved_hashes_from_index` | 4 | intra-community |
| `Cmd_app_dream → collect_resolved_hashes_from_index` | 3 | cross-community |
| `Cmd_build_status → find_entity` | 3 | intra-community |
| `Cmd_test → skip_whitespace → peek_span` | 6 | cross-community |
| `Cmd_quality → is_classifier → peek → advance` | 6 | cross-community |
| `Compiles_point_struct → Nom_string_type` | 8 | cross-community (LLVM codegen) |
| `Write_stmt → Write_u64` | 6 | intra-community |

---

### 2.11 Supporting Crates

| Crate | Function | State |
|-------|----------|-------|
| `nom-types` | Shared type definitions (Entry, Entity, GraphEdge, Translation, EntryScores, etc.) | Shipped |
| `nom-diagnostics` | Error formatting — `miette` + `ariadne` wrappers; `NOMX-Sx-*` error codes | Shipped |
| `nom-config` | `~/.nom/` directory config; path resolution | Shipped |
| `nom-app` | High-level application integration; 20+ `upsert_entry` call sites | Shipped; Cycle 3 migration pending |
| `nom-bench` | Benchmark recording; `bench_ids` linkage in entity rows | Scaffold |
| `nom-media` | Media ingest — `IngestedPng` struct; `ingest_by_extension` → PNG/AVIF/MP4/WAV/SVG | Shipped |
| `nom-translate` | `entry_translations` table; `add_translation` | Scaffold |
| `nom-locale` | Locale/i18n plumbing | Scaffold |
| `nom-ux` | Terminal UX helpers | Scaffold |
| `nom-concept` | (see §2.2 above) | Core shipped |

---

## 3. The Two Functions — What the Code Actually Does Today

There are **two separate functions** in the system. GAP-0 is the wiring that connects them through the correct pipeline.

---

### The Canonical End-to-End Pipeline (What the Code Actually Does)

> **Key fact verified from code:** `body_bytes` (`.bc` blobs AND media blobs) are stored **inline as SQLite BLOBs** inside the `entries` table — NOT in a filesystem path like `~/.nom/store/`. The LLVM step at build time is a **linker** (not yet implemented; currently outputs a JSON BOM).

```
  ─────────────────────────────────────────────────────────────────────────
  FLOW A: AUTHORED SOURCE  (nom store add myfile.nomx)
  ─────────────────────────────────────────────────────────────────────────

  myfile.nomx
       │  nom-concept S1-S6  (GAP-0 wiring in progress)
       │
       │  STEP 1 — MACRO SCOPE  (concept declarations first)
       ├─► .nom format ──────────► concepts.sqlite → concept_defs
       │                              { name, intent, index_into_db2,
       │                                exposes, acceptance, objectives }
       │
       │  STEP 2 — MICRO SCOPE  (entity declarations, after concepts settled)
       └─► .nomtu format ─────────► entities.sqlite → entities
                                       { hash, word, kind, signature,
                                         contracts, composed_of,
                                         body_bytes=None,  ← NO .bc yet
                                         body_kind=None  }

  ─────────────────────────────────────────────────────────────────────────
  FLOW B: MEDIA INGEST  (nom store add-media myfile.png)
  ─────────────────────────────────────────────────────────────────────────

  myfile.{png,avif,flac,mp4,webm,…}
       │  nom-media::ingest_by_extension()
       │  → canonical re-encode  (e.g. PNG → AVIF by default)
       │  → SHA-256(canonical_bytes) → id
       │
       ▼  dict_db.upsert_entry(&Entry {
            id:          SHA-256(canonical_bytes),
            word:        "file_stem",
            kind:        MediaUnit,
            body_bytes:  Some(canonical_bytes),  ← BLOB stored INLINE in SQLite
            body_kind:   "avif" / "flac" / "mp4" / …,
            status:      Complete,
          })
  entries table  ← bytes in entries.body_bytes BLOB column, not on filesystem

  ─────────────────────────────────────────────────────────────────────────
  FLOW C: CORPUS INGEST  (nom corpus clone-and-ingest <url>)
  ─────────────────────────────────────────────────────────────────────────

  external repo (GitHub, PyPI, npm, crates.io, …)
       │  git clone
       │  nom_corpus::ingest_directory()
       │  → walk files → detect language by extension
       │  → SHA-256(raw_source) → id
       │
       ▼  dict_db.upsert_entry_if_new(&Entry {
            body_bytes:  Some(raw_source_bytes),  ← RAW SOURCE, not .bc
            body_kind:   None,                    ← not compiled yet
            status:      Partial,
          })
  entries table  ← source code stored as BLOB; NO LLVM at this stage

  ─────────────────────────────────────────────────────────────────────────
  FLOW D: BUILD STATUS  (nom build status)
  ─────────────────────────────────────────────────────────────────────────

  materialize_concept_graph_from_db()
       │  read concept_defs ← concepts.sqlite
       │  BFS over index_into_db2 hashes
       │  find_entity(hash) ← entities.sqlite
       ▼
  ConceptGraph { concepts, modules }
       │  resolve_closure() → match word refs to entity hashes
       ▼
  Status report: "N/M words resolved"

  ─────────────────────────────────────────────────────────────────────────
  FLOW E: BUILD BOM  (nom app dream / nom app build-bom)
  ─────────────────────────────────────────────────────────────────────────

  closure_entries()  → load all entries in manifest BFS closure
       │  filter: body_kind == "bc"  (entries with compiled bitcode)
       │  read body_bytes from entries table  ← BLOB in SQLite
       ▼
  JSON Bill-of-Materials (nom-bom-v0):
  {
    "format": "nom-bom-v0",
    "entries": [
      { "entry_id": "<hash>", "word": "...",
        "body_kind": "bc", "body_bytes_len": N }
    ],
    "note": "JSON BOM until LLVM linker lands;
             downstream consumes body_bytes from dict by entry_id"
  }

  ─────────────────────────────────────────────────────────────────────────
  FLOW F: LLVM LINK → APP BINARY  (NOT YET IMPLEMENTED)
  ─────────────────────────────────────────────────────────────────────────

  JSON BOM → read body_bytes per entry_id from entries table
       │  llvm-link (link all .bc blobs into one module)
       │  llc / clang  (compile to native / wasm / apk)
       ▼
  app.bin / app.wasm / app.apk

  ⚠️  This step exists only as a comment in build_core_aspect():
  "JSON BOM until LLVM linker lands"
  The actual LLVM link call is FUTURE WORK.
```

---

### The Three-Database Layout (Target: 100M+ entities)

> **Storage reality (from code):** `body_bytes` — both `.bc` bitcode and media blobs — are stored **inline as SQLite BLOBs** in the `entries` table today. A separate filesystem content-addressed store (`~/.nom/store/`) is aspirational architecture, not yet implemented.

```
  grammar.sqlite        ← Grammar registry (one of the 3 canonical DBs)
  ├── schema_meta
  ├── keywords
  ├── keyword_synonyms
  ├── clause_shapes
  ├── kinds
  ├── quality_names
  └── patterns             (258 patterns today, growing)

  concepts.sqlite       ← DB1: Concept tier (MACRO scope)
  ├── concept_defs         (name PK, repo_id, intent, index_into_db2,
  │                          exposes, acceptance, objectives)
  ├── required_axes        (MECE registry per scope)
  ├── dict_meta            (freshness tracking)
  ├── concepts             ← LEGACY — scheduled for deletion
  └── concept_members      ← LEGACY — scheduled for deletion

  entities.sqlite       ← DB2: Entity tier (MICRO scope — SCALES TO 100M+ ROWS)
  ├── entities             (hash PK, word, kind, signature, contracts, composed_of, …)
  ├── entry_scores         (11 quality dimensions)
  ├── entry_meta           (EAV metadata)
  ├── entry_signatures
  ├── entry_security_findings
  ├── entry_refs
  ├── entry_graph_edges    (28 edge types: Calls, Imports, Writes, …)
  ├── entry_translations
  └── entries              ← LEGACY — body_bytes BLOB here today
                              scheduled for migration + deletion (dict-split S8)

  LEGACY entries.body_bytes column layout:
  ┌───────────────┬──────────────────────────────────────────────┐
  │ body_kind     │ body_bytes contents                          │
  ├───────────────┼──────────────────────────────────────────────┤
  │ "bc"          │ LLVM bitcode (.bc compiled from source)      │
  │ "avif"        │ canonical AVIF image bytes                   │
  │ "flac"        │ canonical FLAC audio bytes                   │
  │ "mp4"         │ MP4 container bytes                          │
  │ NULL          │ raw source code (corpus ingest, status=Partial) │
  └───────────────┴──────────────────────────────────────────────┘

  Aspirational (future): content-addressed filesystem store
  ~/.nom/store/<hash>/body.{bc,avif,mp4,wav,svg,…}
  (avoids SQLite BLOB size limits at 100M+ row scale)
```

> **Design note on cross-tier references:** SQLite does not support foreign keys across `.sqlite` files. The `concept_defs.index_into_db2` column stores a JSON array of entity hashes. Cross-tier joins are resolved **in the Rust layer** (via `materialize_concept_graph_from_db`), not via SQL.

---

### Function 1: Ingest Source → Write to 3-DB system (GAP-0 in progress)

> **Entry point:** `nom store add <file.nomx>` → `cmd_store_add()` in `nom-cli/src/store/commands.rs`

**Current (migrated CLI path):**
```
  nom store add myfile.nomx
    → nom-concept::run_pipeline()  ← S1-S6 prose pipeline
    → PipelineOutput::Nom          ← concept_defs write via `upsert_concept_def`
    → PipelineOutput::Nomtu        ← entities write via `upsert_entity`
    → (artifact compile still pending follow-up)
```

**Target (GAP-0, wiring in progress) — macro before micro:**
```
  nom store add myfile.nomx
    → nom-concept::run_pipeline()  ← S1-S6 prose pipeline
    → PipelineOutput::Nom          → concept_defs (concepts.sqlite)   [MACRO]
    → PipelineOutput::Nomtu        → entities (entities.sqlite)       [MICRO]
                                      body_bytes = None  (no .bc yet at ingest)
                                      body_kind  = None
    → (future separate step) nom-llvm compile
                                   → entity.body_bytes = Vec<u8>  ← .bc BLOB
                                      entity.body_kind  = "bc"
```

**Media ingest (already working today):**
```
  nom store add-media myfile.png
    → nom-media::ingest_by_extension()  ← canonical re-encode
    → SHA-256(canonical_bytes) → id
    → entries.upsert_entry(&Entry {
        body_bytes: Some(canonical_bytes),  ← inline BLOB
        body_kind:  "avif",
        status:     Complete,
      })
```

**Corpus ingest (external repos):**
```
  nom corpus clone-and-ingest <url>
    → git clone + walk files
    → upsert_entry_if_new(&Entry {
        body_bytes: Some(raw_source_bytes),  ← raw source, NOT .bc
        body_kind:  None,                    ← not compiled at ingest time
        status:     Partial,
      })
    Note: No LLVM compilation happens during corpus ingest.
          Translation to .bc is a future nom-translate step.
```

---

### Function 2: Build App from DB (materialize_concept_graph_from_db)

> **Entry point:** `nom build status` / `nom build dream` → `materialize_concept_graph_from_db()` in `nom-cli/src/store/materialize.rs`  
> **This does NOT re-parse source.** It reads from the 3-DB system and reconstructs the concept graph.

```
  nom build status
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
  ConceptGraph { concepts, modules }
       │
       ▼
  nom-concept::closure::ConceptClosure  ← BFS walk
       │
       ▼
  Build report / status output / nom-llvm link
```

---

### The Gap (GAP-0)

```
 FUNCTION 1 (Source Ingest)           FUNCTION 2 (Build App)
  ─────────────────────────           ──────────────────────
  .nomx → S1-S6 (GAP-0 partially wired) ───► concept_defs + entities ───► materialize → build
       remaining gap → richer artifact compile/materialize follow-through
```

---

**Corpus ingestion pipeline (separate path):**
```
nom-corpus  →  [fetch package]  →  nom-extract (tree-sitter)
           →  nom-dict (upsert_entry / upsert_entity)
           →  entities.sqlite   status='partial'
           →  nom-score         (scores=NULL until canonicalization)
           →  nom-security      (findings=NULL until analysis)
           →  nom-bench         (bench_ids=[] until benchmark)
```

**AI authoring loop (intent → artifact):**
```
Author writes intent prose
       │
       ▼
nom-intent (ReAct loop)
   DictTools::query    ← entities.sqlite + grammar.sqlite.patterns (Jaccard)
   DictTools::render   ← artifact store (bytes_hash)
   Observation         → agent observation / error
       │
       ▼
nom-lsp (completion surface)
   pattern-search      ← grammar.sqlite.patterns (Jaccard)
   executeCommand      ← nom-concept pipeline
       │
       ▼
nom-resolver (intent.rs)
   JaccardOverIntents  ← entity descriptions
   SemanticEmbedding   ← STUB (T4.2 gate)
       │ resolved hash
       ▼
  source locked to name@hash
```

---

## 4. State Machine — Current vs Target

### Phase Status Matrix (from roadmap + checklog)

| Phase | Concern | Status | Completeness |
|-------|---------|--------|--------------|
| 0 | Workspace + 29-crate scaffold (nom-parser + nom-lexer deleted) | ✅ Shipped | 100% |
| 1 | Lexer + parser (host-language) | ✅ Shipped | 100% |
| 2 | Resolver + verifier baseline | ✅ Shipped | 100% |
| 3 | LLVM backend + self-hosting lexer e2e | ✅ Shipped | 100% |
| 4 | DIDS content-addressed dict store | ✅ Shipped | 100% |
| 5 | Body-only ingestion + multi-edge graph + intent + lifecycle | 🔄 In-flight | ~65% |
| 6 | Parser-in-Nom (Stage-1 self-host) | 📋 Planned | 0% |
| 7 | Resolver + verifier in Nom (Stage-2) | 📋 Planned | 0% |
| 8 | Three-tier concept/module/entity architecture | 🔄 In-flight | Architecture 100%; ingestion 0% |
| 9 | Authoring protocol + LSP + grammar-registry-as-RAG | 🔄 In-flight | ~75% |
| 10 | Bootstrap fixpoint | 🌟 Aspirational | 0% |
| 11 | Mathematics-as-language | 🌟 Aspirational | 0% |
| 12 | Closure-level specialization | 🌟 Aspirational | 0% |

---

## 5. Gap Analysis — Current → Target

### GAP-0: Pipeline Wiring — nom-concept S1-S6 Integration 🟢 FUNCTIONALLY COMPLETE

**Discovery (Session 2026-04-15):** The pipeline wiring is substantially further along than previous documentation indicated. Investigation revealed:

**Already Implemented:**
- ✅ `cmd_store_add` uses `nom-concept::stages::run_pipeline()` + Dict writes to split DB
- ✅ `cmd_fmt` bridges PipelineOutput → SourceFile AST via ast_bridge.rs for formatting  
- ✅ `cmd_check` bridges PipelineOutput → SourceFile AST via ast_bridge.rs for verification
- ✅ `cmd_report` bridges PipelineOutput → SourceFile AST via ast_bridge.rs for security
- ✅ `nom build manifest` uses `nom-concept::stages::run_pipeline` for effect collection
- ✅ All Dict routing uses free functions (no struct method calls in CLI)

**Architecture:**
The ast_bridge module (`nom-cli/src/ast_bridge.rs`) provides seamless translation from PipelineOutput to legacy SourceFile AST, enabling gradual migration without breaking existing validators (nom-verifier, nom-security). This is the correct bridge strategy for GAP-4 (nom-parser deletion) — the architecture can preserve all verification logic while eliminating the legacy parser.

**Pipeline Execution Path:**
```
.nomx (authored prose) ──► nom-concept S1-S6 ─────► PipelineOutput
                                                    │
                                    ┌───────────────┼───────────────┐
                                    │               │               │
                              Binary Fmt      Bridge to AST    Entity Write
                             (format_source)  (ast_bridge)    (to entities.sqlite)
                                    │               │               │
                                  Output ◄─ {check,report,verify} ◄─ Dict
```

**Remaining Work:**
- Artifact compilation (`.bc` generation) still pending nom-llvm integration
- GAP-4: Delete nom-parser from test surfaces (currently only used for legacy test backward compatibility)

---

### GAP-1: Dict-Split Completion (Cycle 3) 🟢 SUBSTANTIALLY COMPLETE

**Discovery (Session 2026-04-15):** GAP-1 is further along than reported. The migration is PHASE 2-complete with Phase 3 (cleanup) remaining.

**What's done (Phase 1 & 2):** 
- Ported all internal compiler logic in `nom-cli`, `nom-app`, `nom-resolver`, and `nom-intent` (both source and tests) from `NomDict` to `Dict` free functions.
- `upsert_entry`, `upsert_entry_if_new`, `find_entries`, `get_entry`, etc., are now accessed via `nom_dict::dist` free functions.
- All concept command handlers (`cmd_concept_*`) open `Dict` directly and use split-aware free functions.
- `nom store sync` opens `Dict` directly, drives `.nom` / `.nomtu` sources through pipeline, writes through free functions.
- `nom corpus register-axis`, `list-axes` open `Dict` directly with split-aware functions.
- `nom-corpus` ingest entry points take `&Dict` instead of `&NomDict`, transactions preserved on `entities` tier.
- ✅ `nom store add-media` migrated with `Dict::try_open_from_nomdict_path` + `nom_dict::upsert_entry` (committed 2026-04-15)
- ✅ `nom author` prose-to-dict migration removed NomDict fallback, uses Dict exclusively (committed 2026-04-15)

**Batch 7 additions (Phase 3):**
- ✅ `resolve_prefix` now queries `entities` table first (canonical), falling back to legacy `entries` if not found.
- ✅ `find_entities_by_body_kind` added as canonical replacement for `find_by_body_kind`.
- ✅ `find_by_word` and `find_by_body_kind` deprecated (marked with `#[deprecated]`).

**What's missing (Phase 3 - Cleanup):**
- Porting backward-compatible atom commands (`extract`, `score`, `stats`, `coverage`) to `.nomtu` pipeline
- Complete deletion of `NomDict` struct, legacy `entries`/`concepts` tables, V2-V5 schema

**Status:** READY for Phase 3 final cleanup and NomDict deletion.

---

### GAP-2: Embedding / Semantic Resolver 🔴 CRITICAL PATH

**What's missing:** `SemanticEmbedding` impl in `nom-resolver/src/intent.rs` always returns `Err(Unavailable)`.

**Impact:** 
- `nom-lsp` completion ranking is Jaccard-only (good deterministic floor, no semantic depth)  
- S6 `ref_resolve` uses alphabetical tiebreaker — non-semantic resolution
- Pattern search is Jaccard-only — patterns that are semantically close but textually different aren't ranked correctly

**Blockers:** 
1. What embedding model guarantees byte-reproducible outputs across builds/machines? (open question)
2. Network access during corpus-fenced cycles
3. Per-kind index vs. combined index decision for 10^8-row scale

**Actions:**
- Decide embedding model (determinism + reproducibility requirements)
- Wire `nom corpus embed` command to populate per-entity embeddings
- Replace `JaccardOverIntents` tiebreaker in S6 with embedding re-rank

---

### GAP-3: Corpus Ingestion (M6 Pilot) 🔴 BLOCKED

**What's missing:** Zero production packages ingested. Dictionary is empty except for test entries.

**External blocks:**
1. Network access (currently fenced)
2. ~50 GB free disk on workspace volume  
3. Windows DLL-load fix (archived doc 15 §3)

**When unblocked:**
- PyPI top-100 ingestion via `nom-corpus`
- `status = 'partial'` rows land in `entities.sqlite`
- Canonicalization pass lifts to `'complete'`
- 11-dimension quality scoring populates `entry_scores`

---

### GAP-4: `.nomx` Format Unification ✅ COMPLETE (2026-04-16)

**Shipped:**
- `nom-parser` crate directory deleted (`crates/nom-parser/` gone).
- `nom-lexer` crate directory deleted (`crates/nom-lexer/` gone).
- All entries for both crates removed from workspace `Cargo.toml`, `nom-cli/Cargo.toml`, and `nom-llvm/Cargo.toml`.
- `self-host` feature **fully removed** from `nom-cli/Cargo.toml` — no `[features]` section remains.
- `parse_with_bridge_or_legacy()` in `main.rs`: bridge-only — calls `nom_concept::stages::run_pipeline(source)` then `ast_bridge::bridge_to_ast()`. No legacy fallback.
- `read_extractable_atoms` removed (absent from codebase).
- `link_bitcode_to_executable` stub removed from `manifest.rs` (absent from codebase).

**Remaining limitations (not blockers for GAP-4):**
- Bridge (`ast_bridge.rs`, 753 lines) produces enriched but not fully imperative statement bodies — arithmetic, call/return, conditionals (if/then/else, when/then/otherwise), multi-statement bodies, and comparison operators shipped; loop constructs and complex match remain as GAP-5a.
- One inline test module in `nom-llvm/src/context.rs:239` remains `#[cfg(any())]` gated (depends on removed `nom-parser`); all 5 external test files are ungated.
- Workspace is now **29 crates** (down from 31).

---

### GAP-5: S3 Required-Clause Presence Check ✅ SHIPPED

**What shipped:** S3 now checks both that `clause_shapes` rows exist for a kind and that every `is_required=1` clause is present in the block body before the grammar-aware pipeline advances.

**Evidence:**
- `nom-concept/src/stages.rs` now queries `required_clauses_for_kind()` and rejects with `missing-required-clause`
- `nom-concept/tests/clause_shape_guard.rs` now covers both the missing-clause rejection and the all-required-clauses-present success path

### GAP-5a: AST Bridge Body Recovery ✅ COMPLETE

**Current state:** `ast_bridge.rs` (1487 lines, 26 tests) handles: functions, arithmetic expressions, function call expressions, return statements, **prose-conditional recovery** (`"if X is Y then Z"` / `"if X then Y else Z"` → `IfExpr`), `when/then/otherwise` conditionals, **multi-arm match expressions** (`when X is Y then Z, when X is A then B, otherwise C` → chained `IfExpr` via `else_ifs`), multi-statement bodies, comparison operators, **loop constructs** (`for-each`, `for-in`, `while-do`, `repeat-until`), **nested multi-arg calls** (`outer(inner(x), y)` via paren-balanced arg splitting), and **contracts/effects propagation** (`requires`/`ensures` → `Describe`, `benefit`/`hazard` → `Effects` with `Good`/`Bad` modifiers). Bridge is sufficient for check/verify/report/fmt and covers all major control-flow, pattern-matching, and semantic-annotation patterns.

**Remaining:** None. GAP-5a is complete.

---

### GAP-6: real `nom-planner` ✅ COMPLETE

**Shipped 2026-04-16:**
- `Planner::plan_from_pipeline_output(&PipelineOutput)` derives `CompositionPlan` directly from `.nomtu` module compositions emitted by `nom-concept` S1-S6.
- `nom build compile` now runs `run_pipeline` once, chooses direct pipeline planning when flows exist, and falls back to AST-bridge planning for concept/entity-only files.
- Regression coverage proves a `.nomtu` module composition becomes one planned flow with ordered nodes and edges.

**Batch 6 additions:**
- `fuse_identity_nodes`, `fuse_consecutive_maps`, `collapse_single_branch` + `optimize_plan` orchestrator shipped. 17 planner tests. ✅

**Batch 10 additions:**
- `ResolverMetadata` struct (word, kind, contracts, score) — bridges dictionary data into planner without direct `nom-dict` dependency. ✅
- `kind_adjusted_score` — kind-specific scoring: functions weight performance, data weights reliability. ✅
- Contract validation node insertion — `validate:<word>` nodes inserted before contract-bearing entities. ✅
- `optimize_plan_with_context` — orchestrator threading resolver metadata through all passes. ✅
- 25 planner tests (up from 17). ✅

**Remaining:** None. GAP-6 is complete.

---

### GAP-7: LSP MVP 🟢 EXACT SOURCE RANGES SHIPPED

**Shipped 2026-04-16 (batches 6–8):**
- Hover resolves the word/hash under cursor against split-dict `entities` rows via `NOM_DICT`.
- Goto-definition resolves the same entity row and returns `origin_ref` / `authored_in` as an LSP location.
- Server capabilities now advertise `definition_provider`.
- **Batch 7:** Enriched hover — structured Markdown payload includes contracts, effects, retry policy, format string, body_kind, and quality scores. 27 LSP tests (up from 19).
- **Batch 8:** Exact source ranges for goto-definition shipped. 31 LSP tests (up from 27).

**Shipped so far:** Slices 1-7b + exact source ranges (stdio, classify, RAG markdown, executeCommand, ReAct adapter, pattern completion, dict-backed hover + goto-def, enriched hover, exact source ranges).

**Gap:** Embedding-ranked completion still depends on GAP-2.

---

### GAP-8: AI Authoring Loop Closure 🟡 NEWLY UNBLOCKED

**Status change (batch 8):** GAP-8 is newly unblocked. The nom-llvm linking pipeline (shipped batch 2) combined with sufficient bridge fidelity means the ReAct loop can now produce real artifacts end-to-end.

**Remaining actions:**
- Wire `nom-llvm` dependency through `nom-intent`
- Replace `Rendered { bytes_hash }` stub with actual artifact store write
- Close loop: intent prose → resolver → dict lookup → composition → manifest → artifact build

---

### GAP-9: Phase 6 — Parser-in-Nom 📋 PLANNED

**What's missing:** The parser is still in Rust. Stage-1 self-hosting requires the parser authored in `.nomx`.

**Prerequisite chain:** `.nomx` format unified → grammar registry complete → planner working → LLVM backend stable → parser authoring begins.

---

### GAP-10: Bootstrap Fixpoint 🌟 ASPIRATIONAL

**Goal:** Full self-hosting compiler — Stage-2 binary output == Stage-3 binary output (byte-identical).

**What this means:**
```
  Stage-1: Nom compiler (written in Rust) compiles the Nom-authored lexer/parser/resolver
               └─► produces Stage-2 compiler binary

  Stage-2: Stage-2 binary compiles the same Nom-authored lexer/parser/resolver source
               └─► produces Stage-3 compiler binary

  Fixpoint proof: hash(Stage-2 output) == hash(Stage-3 output)  ← byte-identical
```

**Why byte-identical matters:** Any non-determinism in codegen (hash-map iteration order, timestamps, FS traversal) breaks the fixpoint. Byte identity is the only unforgeable proof the compiler can correctly reproduce itself.

**Status today:** Stage-1 milestone ✅ — LLVM self-hosting lexer compiles end-to-end. Full fixpoint is Phase 10.

**Prerequisite chain:** GAP-9 (parser authored in `.nomx`) → Phase 7 (resolver + verifier in `.nomx`) → determinism audit of all codegen passes → fixpoint proof.

---

### GAP-11: Quality Registration CLI 🟡 MINOR

**What's missing:** `quality_names.metric_function` column is nullable; `nom corpus register-axis` CLI doesn't exist yet.

---

### GAP-12: Grammar Wedge Backlog ✅ COMPLETE (11/11)

All 11 shapes shipped. Grammar wedge pattern: lexer token → Tok enum → S5 extraction → EntityDecl field → baseline.sql → tests. S5 extraction chain is now S5a–S5m (13 sub-stages). Entire backlog cleared in batch 8.

| Wedge | Status |
|-------|--------|
| **Retry-policy clause** | **✅ SHIPPED** — `RetryPolicy` struct, `retry_policy: Option<RetryPolicy>` on entity decl, `retry-policy` pattern in `grammar.sqlite` baseline (batch 3) |
| **Sum-type `@Union` typed-kind** | **✅ SHIPPED (batch 5)** — S5d extraction, `UnionVariants` struct, 8 tests |
| **Format-string interpolation** | **✅ SHIPPED (batch 5)** — S5e extraction, skip-on-prose rule, 9 tests |
| **Nested-record-path syntax** | **✅ SHIPPED (batch 6)** — S5f extraction, `accesses` field, 9 tests |
| **Wire-field-tag clause** | **✅ SHIPPED (batch 6)** — S5g extraction, `FieldTag` struct, 9 tests |
| **Pattern-shape clause on data decls** | **✅ SHIPPED (batch 6)** — S5h extraction, `shaped like`, 8 tests |
| **Exhaustiveness check on `when` clauses** | **✅ SHIPPED (batch 7)** — S5i extraction, `WhenClause`, `check_exhaustiveness()` validator, 9 tests |
| **Watermark clause (streaming)** | **✅ SHIPPED (batch 8)** — S5j extraction, `watermark_clause.rs` test file |
| **Window-aggregation clause** | **✅ SHIPPED (batch 8)** — S5k extraction, `window_clause.rs` test file |
| **Clock-domain clause** | **✅ SHIPPED (batch 8)** — S5l extraction, `clock_domain_clause.rs` test file |
| **QualityName-registration formalization** | **✅ SHIPPED (batch 8)** — S5m extraction, `quality_declarations.rs` test file |

---

## 6. Dependency Graph — Gap Closure Order

```
GAP-0  Wire nom-concept S1-S6 into cmd_store_add
  └──► enables concept_defs + entities tables to be populated by parse
  └──► enables Function 2 (materialize_concept_graph_from_db) to have real data
  └──► enables GAP-4 (can delete nom-parser after wiring)

GAP-1  Dict-Split Complete
  └──► enables GAP-3 (corpus ingest uses Dict::upsert_entry)

GAP-3  Corpus Ingest (M6 Pilot)
  └──► enables GAP-2 (needs entity rows to train/serve embeddings)

GAP-2  Embedding Resolver
  └──► enables GAP-7 (LSP embedding-ranked completion)
  └──► enables GAP-8 (ReAct loop with real resolution)

GAP-4  nom-parser deletion (after GAP-0 wiring done)
  └──► enables GAP-5 (clean test surface for required-clause check)
  └──► enables GAP-9 (parser-in-Nom needs single format)

GAP-5  S3 Required-Clause Check ✅
GAP-6  Real Planner
  └──► both enable GAP-9

GAP-9  Parser-in-Nom (Phase 6)
  └──► enables Phase 7 (Resolver + Verifier in Nom)
  └──► enables GAP-10 (Bootstrap fixpoint)

GAP-10 Bootstrap Fixpoint (Phase 10)
  └──► enables Compiler Retirement
```

---

## 7. Test Coverage Summary

| Suite | Tests | Status |
|-------|-------|--------|
| `nom-concept` (Phase E proofs P1-P7) | 7 proofs | ✅ All green |
| Corpus dashboard | 84/88 blocks (95.5%) | ✅ 4 remaining = authoring-side gaps |
| Pattern catalog uniqueness | 258/258 distinct intents; Jaccard max 0.273 < 0.5 threshold | ✅ |
| `nom-intent` | 63 lib + 2 integration | ✅ |
| `nom-resolver` | 58 lib | ✅ |
| `nom-dict` | 94 lib | ✅ |
| `nom-llvm` external tests | 16 tests across `builtins.rs`, `enums.rs`, `lists.rs`, `strings.rs`, `tuples.rs` | ✅ All ungated |
| `nom-llvm` inline tests (`lib.rs`) | 5 tests (including `link_bitcodes_two_modules`, `compile_plans_two_modules`) | ✅ |
| `nom-llvm` inline tests (`context.rs`) | 3 tests | ✅ Un-gated (batch 4) |
| Self-host pipeline (`nom-cli`) | `self_host_pipeline`, `self_host_ast`, `self_host_codegen`, `self_host_parser`, `self_host_planner`, `self_host_smoke`, `self_host_verifier`, `self_host_rust_parity`, `self_host_meta`, `self_host_parse_smoke` | ✅ (various) |
| E2E (`nom-cli`) | `build_report_e2e`, `build_manifest_e2e`, `acceptance_preserve_e2e`, `concept_demo_e2e`, `agent_demo_e2e`, `layered_dream_smoke`, `three_tier_recursive_e2e`, `corpus_axes_smoke`, `store_cli` (4 tests), `store_sync_smoke`, `avif_ingest`, `bc_body_round_trip`, `media_import`, `mcp_smoke`, `locale_smoke`, `dids_store_e2e` (1 test, replaces `phase4_acceptance`) | Mostly ✅ (`parser_subset_probe.rs` deleted — historical) |

---

## 8. Hard Blockers (External Gates)

| Blocker | Affects | Resolution |
|---------|---------|------------|
| Network access fenced | GAP-2 (embedding model), GAP-3 (corpus ingest) | Environment provisioning |
| ~50 GB free disk required | GAP-3 (stream-and-discard discipline) | Disk cleanup / larger volume |
| Windows DLL-load issue (doc 15 §3) | GAP-3 (corpus pipeline startup) | Apply documented fix |
| Determinism of embedding model outputs | GAP-2 (resolver), GAP-10 (bootstrap) | Choose model with reproducible output spec |

---

## 9. Recommended Execution Order

```
DONE THIS SESSION (2026-04-16 — eight batches):
  ✅ Phase 0: 36 stale files removed
  ✅ GAP-4: nom-parser + nom-lexer deleted; self-host feature removed; 29-crate workspace
  ✅ Store commands migrated to Dict free functions
  ✅ GAP-5a: ast_bridge.rs — arithmetic + call/return + conditionals (if/then/else, when/then/otherwise) + multi-statement bodies + comparison operators + loop constructs (for-each, for-in, while-do, repeat-until) + contracts/effects propagation; 19 bridge tests
  ✅ GAP-5b: link_bitcodes() + compile_plans() SHIPPED in nom-llvm; nom build link CLI wired end-to-end
  ✅ All 5 nom-llvm external test files migrated (16 tests); context.rs inline tests un-gated; 20 nom-llvm tests total
  ✅ store_cli.rs rewritten (Dict + .nomx, 4 tests, cfg(any()) removed)
  ✅ phase4_acceptance.rs renamed to dids_store_e2e.rs (Dict + .nomx, 1 test, #[ignore] removed)
  ✅ parser_subset_probe.rs deleted (historical, nom-parser gone)
  ✅ 8 self-host test files converted from parser-dependent to structural verification
  ✅ GAP-1c: cmd_store_stats + status_histogram() reads canonical entities; list_partial_ids() deprecated; 23+ functions inventoried
  ✅ Status column added to entities table
  ✅ GAP-12 started: RetryPolicy struct + retry_policy field + grammar.sqlite pattern (batch 3)
  ✅ GAP-12 @Union sum-type: S5d extraction, UnionVariants struct, 8 tests (batch 5)
  ✅ GAP-12 format-string: S5e extraction, skip-on-prose rule, 9 tests (batch 5)
  ✅ GAP-12 nested-record-path: S5f extraction, accesses field, 9 tests (batch 6)
  ✅ GAP-12 wire-field-tag: S5g extraction, FieldTag struct, 9 tests (batch 6)
  ✅ GAP-12 pattern-shape: S5h extraction, shaped like, 8 tests (batch 6)
  ✅ Planner fusion: fuse_identity_nodes, fuse_consecutive_maps, collapse_single_branch + optimize_plan orchestrator, 17 planner tests (batch 6)
  ✅ GAP-12 exhaustiveness check: S5i extraction, WhenClause, check_exhaustiveness() validator, 9 tests (batch 7)
  ✅ LSP enriched hover: contracts/effects/retry/format/body_kind/scores, 27 LSP tests (batch 7)
  ✅ GAP-1c resolve_prefix → entities-first query; find_entities_by_body_kind added; find_by_word/find_by_body_kind deprecated (batch 7)
  ✅ 1002 tests passing — 1000-test milestone crossed (batch 7)
  ✅ GAP-12 watermark clause: S5j extraction, watermark_clause.rs test file (batch 8)
  ✅ GAP-12 window-aggregation clause: S5k extraction, window_clause.rs test file (batch 8)
  ✅ GAP-12 clock-domain clause: S5l extraction, clock_domain_clause.rs test file (batch 8)
  ✅ GAP-12 QualityName-registration formalization: S5m extraction, quality_declarations.rs test file (batch 8)
  ✅ GAP-12 11/11 COMPLETE — entire grammar wedge backlog cleared; S5 chain is S5a–S5m (13 sub-stages) (batch 8)
  ✅ LSP exact source ranges for goto-definition shipped; 31 LSP tests (batch 8)
  ✅ GAP-8 newly unblocked (nom-llvm linking + bridge sufficient for real artifact production) (batch 8)
  ✅ ZERO cfg(any()) gates in entire codebase
  ✅ 1043 tests passing (single-threaded), 0 failed, 34 ignored

NOW (unblocked, next steps):
  1. GAP-8: Close AI authoring loop — wire nom-llvm through nom-intent, replace Rendered stub with artifact store write
  2. GAP-5a: Complete remaining bridge recovery (complex match, nested multi-arg calls)
  3. GAP-1c: Complete legacy entries/concepts table migration
       (delete NomDict struct, legacy entries/concepts tables, V2-V5 schema constants)
  4. GAP-6: Replace heuristic specialization with resolver metadata-backed specialization

WHEN NETWORK + DISK AVAILABLE:
  5. Start GAP-3: M6 corpus pilot (PyPI top-100)
  6. After corpus rows land: GAP-2: Wire embedding index (nom corpus embed)

AFTER EMBEDDING INDEX:
  7. GAP-7: Add embedding-ranked completion
  8. GAP-8: Close AI authoring loop (slice-3c-full)

PHASE 6+:
  9. GAP-9: Author parser in .nomx
  10. Phase 7: Author resolver + verifier in .nomx
  11. GAP-10: Bootstrap fixpoint proof
```

---

## 10. Session Progress (2026-04-16, eight batches)

### Completed This Session

**GAP-12: Batch 8 — FULLY COMPLETE (11/11) ✅**
- ✅ Watermark clause (streaming): S5j extraction, `watermark_clause.rs` test file
- ✅ Window-aggregation clause: S5k extraction, `window_clause.rs` test file
- ✅ Clock-domain clause: S5l extraction, `clock_domain_clause.rs` test file
- ✅ QualityName-registration formalization: S5m extraction, `quality_declarations.rs` test file
- ✅ 37 new tests across 4 test files; **GAP-12 11/11 COMPLETE** — entire grammar wedge backlog cleared
- ✅ S5 extraction chain is now S5a–S5m (13 sub-stages)

**GAP-7: Batch 8 — Exact Source Ranges ✅**
- ✅ Exact source ranges for goto-definition shipped; 31 LSP tests (up from 27)

**GAP-8: Batch 8 — Newly Unblocked ✅**
- ✅ nom-llvm linking (shipped batch 2) + bridge fidelity now sufficient; ReAct loop can produce real artifacts

**1043-Test Milestone ✅**
- ✅ **1043 tests passing (single-threaded), 0 failed, 34 ignored**

**GAP-12: Batch 7 — Exhaustiveness Check ✅**
- ✅ Exhaustiveness check on `when` clauses: S5i extraction, `WhenClause` struct, `check_exhaustiveness()` validator, 9 tests
- ✅ 7/11 grammar wedges shipped; S5 extraction chain is S5a–S5i (9 sub-stages)

**GAP-7: LSP Enriched Hover (batch 7) ✅**
- ✅ Structured Markdown hover payload: contracts, effects, retry policy, format string, body_kind, quality scores
- ✅ 27 LSP tests (up from 19)

**GAP-1c: resolve_prefix + find_entities_by_body_kind (batch 7) ✅**
- ✅ `resolve_prefix` queries `entities` table first (canonical path)
- ✅ `find_entities_by_body_kind` added as canonical replacement for `find_by_body_kind`
- ✅ `find_by_word` and `find_by_body_kind` deprecated

**1000-Test Milestone ✅**
- ✅ **1002 tests passing, 0 failed** — crossed the 1000-test threshold

**GAP-12: Batch 6 Grammar Wedges (nested-record-path, wire-field-tag, pattern-shape) ✅**
- ✅ Nested-record-path (`accesses`): S5f extraction, `accesses` field on entity decl, 9 tests
- ✅ Wire-field-tag (`field X tagged Y`): S5g extraction, `FieldTag` struct, 9 tests
- ✅ Pattern-shape (`shaped like`): S5h extraction, 8 tests
- ✅ 6/11 grammar wedges now shipped; S5 extraction chain is S5a–S5h (8 sub-stages)

**GAP-6: Planner Fusion Passes (batch 6) ✅**
- ✅ `fuse_identity_nodes` — eliminates no-op identity nodes from plan
- ✅ `fuse_consecutive_maps` — merges adjacent map nodes into single step
- ✅ `collapse_single_branch` — collapses branch nodes with only one active path
- ✅ `optimize_plan` orchestrator — runs all fusion passes in sequence
- ✅ 17 planner tests total

**Phase 0: Workspace Cleanup**
- ✅ 36 stale files removed (error logs, test outputs, Python scripts)

**GAP-1 Phase 3c: Store Commands Migration**
- ✅ Migrated `nom-cli/src/store/commands.rs` to use Dict free functions
- ✅ Added Dict exports to `nom-dict/src/lib.rs`
- ✅ `store_cli.rs` rewritten (Dict + .nomx, 4 tests, file-level gate removed)
- ✅ `phase4_acceptance.rs` → `dids_store_e2e.rs` (Dict + .nomx, 1 test, no `#[ignore]`)
- ✅ `cmd_store_stats` + `status_histogram()` reads canonical `entities` table
- ✅ `list_partial_ids()` deprecated in favor of entities-table query
- ✅ 23+ legacy-table functions across 7 files inventoried for GAP-1c

**GAP-4: Full Completion**
- ✅ Both `nom-lexer` and `nom-parser` crate directories deleted
- ✅ `self-host` feature fully removed from `nom-cli/Cargo.toml`
- ✅ `parser_subset_probe.rs` deleted (historical after nom-parser removal)
- ✅ Workspace at **29 crates** (from 31 pre-GAP-4)

**GAP-5a: Bridge Extension + Loop Recovery**
- ✅ `ast_bridge.rs` with conditionals + multi-statement bodies + comparison operators + loop constructs (for-each, for-in, while-do, repeat-until)
- ✅ 15 bridge tests

**GAP-5b: LLVM Binary Linking — SHIPPED**
- ✅ `link_bitcodes(bitcode_blobs: &[Vec<u8>]) -> Result<LlvmOutput, LlvmError>` at `nom-llvm/src/lib.rs:48`
- ✅ `compile_plans(plans: &[CompositionPlan]) -> Result<LlvmOutput, LlvmError>` at `nom-llvm/src/lib.rs:84`
- ✅ `nom build link` subcommand wired at `main.rs:6521` — reads `.bc` files → `link_bitcodes()` → clang/llc fallback → executable
- ✅ All 5 nom-llvm external test files ungated (16 tests)
- ✅ `context.rs` inline tests un-gated — 20 nom-llvm tests total, zero `#[cfg(any())]` anywhere
- ✅ 8 self-host test files converted from parser-dependent to structural verification

**GAP-12: Retry-Policy Wedge (partial)**
- ✅ `RetryPolicy` struct at `nom-concept/src/lib.rs:119`
- ✅ `retry_policy: Option<RetryPolicy>` on entity decl
- ✅ `retry-policy` pattern in `grammar.sqlite` baseline

**GAP-12: @Union Sum-Type Wedge (batch 5) ✅**
- ✅ S5d extraction for `@Union` typed-kind
- ✅ `UnionVariants` struct in nom-concept
- ✅ 8 tests

**GAP-12: Format-String Interpolation Wedge (batch 5) ✅**
- ✅ S5e extraction
- ✅ Skip-on-prose rule
- ✅ 9 tests

**Bridge Contracts/Effects (batch 5) ✅**
- ✅ `requires`/`ensures` → `Describe` in ast_bridge.rs
- ✅ `benefit`/`hazard` → `Effects` with `Good`/`Bad` modifiers
- ✅ 4 new bridge tests; 19 total bridge tests

**Dict / Schema**
- ✅ Status column added to `entities` table (migration in `dict.rs:270`)

### Ground Truth Verification (2026-04-16T32:00 JST — batch 8)
- `cargo check --workspace` → `Finished dev profile [unoptimized + debuginfo] target(s) in 20.54s` (0 errors) ✅
- `cargo test --workspace -- --test-threads=1` → **1043 tests passing, 0 failed, 34 ignored** ✅
- GAP-12 complete: watermark S5j, window-aggregation S5k, clock-domain S5l, QualityName S5m — 37 new tests ✅
- LSP exact source ranges for goto-definition: 31 LSP tests ✅
- GAP-8 newly unblocked ✅
- S5 extraction chain: S5a–S5m (13 sub-stages) ✅
- `grep -rn "cfg(any())" crates/ --include="*.rs"` → zero matches ✅

### Next Steps (High Priority)

**GAP-5a Remaining:**
- [ ] Complex match expression recovery
- [ ] Nested multi-arg call recovery

**GAP-1c Completion:**
- [ ] Migrate remaining `entries` reads to `entities` + `entry_meta` + `entry_signatures`
- [ ] Migrate ingest/body writes out of `entries.body_bytes`
- [ ] Delete `entries` from `ENTITIES_SCHEMA_SQL`
- [ ] Delete legacy `concepts` / `concept_members` after compatibility replaced
- [ ] Delete `NomDict` struct entirely

**GAP-12: COMPLETE ✅**
- All 11 wedges shipped; S5 chain S5a–S5m; 37 new tests across watermark_clause.rs, window_clause.rs, clock_domain_clause.rs, quality_declarations.rs

**GAP-8 (now unblocked):**
- [ ] Wire nom-llvm through nom-intent (replace Rendered stub with real artifact store write)
- [ ] Close loop: intent prose → resolver → dict lookup → composition → manifest → artifact build

**GAP-6 Remaining:**
- [ ] Replace heuristic specialization with resolver metadata-backed specialization

**GAP-7 Remaining:**
- [ ] Embedding-ranked completion (→ GAP-2)

*Report last updated: 2026-04-16T32:00 JST — eight-batch session summary*
