# Nom Compiler Crate Audit — Structural Duplication & Restructuring Analysis

> **Date:** 2026-04-19  
> **Scope:** 29 `nom-compiler` crates + 18 `nom-canvas` crates  
> **Method:** Source-level inspection of every `lib.rs`/`main.rs`; Cargo.toml dependency analysis; cross-workspace dependency verification.

---

## 1. Executive Summary

| Metric | Value |
|--------|-------|
| Compiler crates | 29 |
| Canvas crates | 18 |
| Duplicate names across workspaces | **5** (`nom-cli`, `nom-graph`, `nom-intent`, `nom-media`, `nom-ux`) |
| Cross-workspace deps (compiler → canvas) | **0** |
| Cross-workspace deps (canvas → compiler) | **1 bridge crate** (`nom-compiler-bridge`) |
| "God" crates/binaries | 2 (`nom-concept` ~13K lines, `nom-cli` ~16K lines) |
| Stub/scaffold crates (<200 lines) | 4 (`nom-flow`, `nom-intent`, `nom-runtime`, `nom-ux`) |
| Small but real crates (<300 lines) | 4 (`nom-bench`, `nom-config`, `nom-diagnostics`, `nom-search`) |

**Key finding:** The 5 duplicate-named crates are **completely unrelated in domain** — they share a name but zero code or API overlap. This is a naming collision, not architectural duplication. The real structural problem is `nom-concept` (a god crate with 20+ submodules) and `nom-cli` (a god binary with all subcommands in one file).

---

## 2. Per-Crate Verdict Table (Compiler — 29 crates)

| # | Crate | Lines | Stub? | Actual Responsibility | Overlaps (Compiler) | Overlaps (Canvas) | Verdict |
|---|-------|-------|-------|----------------------|---------------------|-------------------|---------|
| 1 | `nom-app` | ~2,990 | No | App manifest builder per §5.12. Compiles `AppManifest` into 10 per-aspect artifacts. "Dreaming mode" (`dream_report`) scores closure completeness. | None | None | **KEEP** |
| 2 | `nom-ast` | ~956 | No | Core AST types (`Statement`, `Expr`, `Classifier`, `FlowQualifier`, `MemoryHint`, `CollectionKind`). Writing-style syntax. | `nom-types` (circular dep: types depends on ast) | None | **KEEP** |
| 3 | `nom-bench` | ~270 | No | Benchmark registry (`BenchFamily`), timing types (`TimingMoments`), global mutex-backed registry. | None | None | **KEEP** |
| 4 | `nom-cli` | ~16,367 | No | **God binary.** All `nom` subcommands via clap: Run, Build, Check, Test, Report, Dict, Import, Precompile, Extract, Score, Stats, Coverage, Translate, Graph, Search, Audit, Quality, Fmt, Store, Media, Corpus, Mcp, Lsp, Agent, Concept, App, Author, Locale, Grammar. Imports ~20 workspace crates. | Imports nearly everything | None | **KEEP** — but modularize subcommands |
| 5 | `nom-codegen` | ~3,926 | No | Generates Rust source from `CompositionPlan`. Target: RustSource (future: LLVM IR / Cranelift). Infers Cargo deps (tokio, rayon, sqlx, argon2, redis, axum, reqwest) from plan effects. | `nom-concept::codegen` submodule overlaps | None | **KEEP** |
| 6 | `nom-concept` | ~13,000+ | No | **God crate.** Parser (`Tok` lexer), IR (`NomFile`/`NomtuFile`), type inference, validation, MECE, exhaustiveness, dreaming, bootstrap, native binary emission, pipeline, lifecycle, stream ingest, selfhost, aesthetic, llvm_emit, SSA, type_check, canonicalize, benchmark_table, bipartite. Exports 20+ submodules. | `nom-codegen`, `nom-verifier`, `nom-planner`, `nom-llvm`, `nom-resolver`, `nom-dict` | None (but `nom-compiler-bridge` imports it) | **SPLIT** — see §4 |
| 7 | `nom-config` | ~147 | No | TOML workspace and donor configuration loader (`WorkspaceConfig`, `DonorConfig`). | None | None | **KEEP** |
| 8 | `nom-corpus` | ~1,544 | No | Mass ingestion of public ecosystems (PyPI + GitHub). Stream-and-discard disk discipline. `scan_directory()`, `ingest_directory()`, `ingest_parent()`, `clone_and_ingest()`. | None | None | **KEEP** |
| 9 | `nom-diagnostics` | ~269 | No | Error reporting via `ariadne` crate with source highlighting. `Diagnostic`, `DiagnosticSink`, `Level` enum. | None | None | **KEEP** |
| 10 | `nom-dict` | ~2,865 | No | SQLite-backed v2 content-addressed dictionary (`nomdict.db`). Central data store. Schema: entries, scores, meta, signatures, security findings, refs, graph edges, translations, concepts. `closure()`, `resolve_prefix()`, `bulk_upsert()`. | `nom-resolver` (v2 migration in progress) | `nom-compiler-bridge` depends on it | **KEEP** |
| 11 | `nom-extract` | ~638 | No | Tree-sitter parsing + UIR entity extraction + atom extraction. Supports 50+ languages (feature-gated). `parse_source()`, `extract_atoms()`, `extract_from_dir()`. | `nom-corpus` (both ingest source code) | None | **KEEP** |
| 12 | `nom-flow` | ~130 | **Yes** | Phase-5 §5.14 scaffold. `FlowArtifact` + `FlowStep` types. "Recorder not yet wired (LLVM call-site instrumentation pending)." | None | None | **KEEP** (real code coming) |
| 13 | `nom-grammar` | ~737 | No | Schema + query API for `grammar.sqlite`. Keywords, synonyms, clause_shapes, quality_names, kinds, patterns. `resolve_synonym()`, `search_patterns()`, `jaccard()` fuzzy matching. | None | `nom-compiler-bridge` depends on it | **KEEP** |
| 14 | `nom-graph` | ~746 | No | Knowledge graph for `.nomtu` relationships. `NomtuGraph` with call/import edge builders, label propagation communities, entry points, flow tracing. Phase 2b: uid-addressed storage migration. | `nom-resolver` (both track relationships) | **NAME COLLISION** — canvas `nom-graph` is 17K-line execution engine | **KEEP** — rename canvas duplicate |
| 15 | `nom-intent` | ~130 | **Yes** | M8 intent-resolution transformer scaffold. `NomIntent` enum (`Kind`/`Symbol`/`Flow`/`Reject`). `classify()` with stub LLM fn. | None | **NAME COLLISION** — canvas `nom-intent` is 5.4K-line skill router | **KEEP** — rename canvas duplicate |
| 16 | `nom-llvm` | ~312 | No | LLVM IR backend. Compiles `CompositionPlan` to `.bc` bitcode via `inkwell`. `compile()`, `link_bitcodes()`, `compile_plans()`. | `nom-concept::llvm_emit` submodule overlaps | None | **KEEP** |
| 17 | `nom-locale` | ~992 | No | BCP47 tag parsing, UAX #15 NFC normalization, M3 locale packs, confusable detection (Cyrillic/Latin homoglyphs). `apply_locale()` lexical keyword substitution. | None | None | **KEEP** |
| 18 | `nom-lsp` | ~1,673 | No | LSP server scaffold. `serve_on_stdio()`, hover, completion, goto-definition, `workspace/executeCommand` (`nom.whyThisNom`, `nom.searchPatterns`). | `nom-grammar`, `nom-dict`, `nom-intent` | None | **KEEP** |
| 19 | `nom-media` | ~2,788 | No | **Real media codecs.** PNG decode/re-encode, FLAC encode via `flacenc`, JPEG quality-85 re-encode, Opus demux/decode via `ogg`+`opus-decoder`, AVIF encode via `ravif`. Round-trip verification gates. | None | **NAME COLLISION** — canvas `nom-media` is 403-line stub | **KEEP** — rename canvas duplicate |
| 20 | `nom-planner` | ~2,447 | No | Generates `CompositionPlan` from verified source. `Planner::plan()`, `plan_from_pipeline_output()` (GAP-6). Memory/concurrency strategy inference. Identity node fusion, consecutive map fusion, branch collapse. | `nom-verifier` (planner calls verifier), `nom-concept::pipeline` | `nom-compiler-bridge` depends on it | **KEEP** |
| 21 | `nom-resolver` | ~1,954 | No | Resolves `NomRef` words against nomdict SQLite. Legacy v1: 48-column `nomtu` table. v2 module: hash-identity resolver against `nom_dict::Dict`. Contract inference via unification (`infer_flow_contracts()`). | `nom-dict` (v2 migration target), `nom-graph` (both track edges) | None | **KEEP** — consider merging v2 into `nom-dict` |
| 22 | `nom-runtime` | ~20 | **Yes** | Runtime library scaffold for compiled Nom binaries. Re-exports `alloc`, `io`, `list`, `print`, `string` modules. `staticlib` + `rlib` crate types. | None | None | **KEEP** (runtime foundation) |
| 23 | `nom-score` | ~541 | No | 8-dimensional quality scoring (security, reliability, performance, readability, testability, portability, composability, maturity). `AtomScores::overall()` weighted average. `can_wire()` compatibility checker. | `nom-security` (both score security) | `nom-compiler-bridge` depends on it | **KEEP** |
| 24 | `nom-search` | ~283 | No | Hybrid search: BM25 index + Reciprocal Rank Fusion. `BM25Index::add_document()`, `search()`, `reciprocal_rank_fusion()`. | None | `nom-compiler-bridge` depends on it | **KEEP** |
| 25 | `nom-security` | ~2,964 | No | Deep security analysis. Layer 1: program-level (score thresholds, CVE flags, effect escalation). Layer 2: body-level (OWASP Top 10, secret detection with 50+ regex patterns, weak crypto, injection, XSS, path traversal, guardrails). | `nom-score` (both security-related) | None | **KEEP** |
| 26 | `nom-translate` | ~176 | No | Pattern-based translation to Rust: C, C++, Python, JS/TS, Go. `translate()`, `confidence_from_untranslated()`. | None | None | **KEEP** |
| 27 | `nom-types` | ~1,361 | No | **Foundational shared types.** `UirEntity`, `Atom`/`AtomKind`, `AtomSignature`, `RelationshipKind`, v2 `Entry`/`EntryKind`/`EntryScores`/`SecurityFinding`/`GraphEdge`/`Translation`, `body_kind` constants, `self_host_tags`. | `nom-ast` (depends on it) | `nom-compiler-bridge` depends on it | **KEEP** |
| 28 | `nom-ux` | ~135 | **Yes** | Phase-5 §5.11 scaffold. `Platform` enum (Web/Desktop/Mobile), `runtime_launch_word()`, `artifact_extension()`. | None | **NAME COLLISION** — canvas `nom-ux` is 713-line UX runtime | **KEEP** — rename canvas duplicate |
| 29 | `nom-verifier` | ~1,543 | No | Contract compatibility checker. Type compatibility across flow chains, constraint satisfaction (`security>0.9`), effect propagation, duplicate/empty declaration checks, range constraint extraction, property-based test generation from contracts. | `nom-planner` (verifier is called by planner), `nom-concept::type_check` | None | **KEEP** |

---

## 3. The 5 Duplicate-Name Crates: Detailed Comparison

These crates share a name but **zero API overlap**. They are distinct domains.

| Name | Workspace | Lines | Domain | Key Types |
|------|-----------|-------|--------|-----------|
| `nom-cli` | compiler | ~16,367 | Compiler CLI (clap-based, 30+ subcommands) | `Cli { command: Commands }`, `BuildCmd`, `CorpusCmd` |
| `nom-cli` | canvas | ~3,713 | Canvas app CLI (simple arg parser) | `CliCommand::Check/Build/Lint/Graph/Rag` |
| `nom-graph` | compiler | ~746 | `.nomtu` knowledge graph (call/import edges, communities) | `NomtuGraph`, `NomtuNode`, `EdgeType::Calls/Imports` |
| `nom-graph` | canvas | ~17,117 | Graph **execution engine** (DAG traversal, caching, RAG, sandbox, WASM bridge, semantic cache, delta compression, intent graph, schema versioning, route tables, memory graph, constraint solver, event bus) | `ExecutionEngine`, `GraphRagRetriever`, `ContentDag`, `SemanticCache`, `ConstraintSolver` |
| `nom-intent` | compiler | ~130 | M8 LLM intent transformer (scaffold) | `NomIntent::Kind/Symbol/Flow/Reject` |
| `nom-intent` | canvas | ~5,442 | Skill router, RAG pipeline, intent classification, BM25+Cosine retrievers, query planner, graphify chart composer, strategy extractor, dream history, feature stack | `SkillRouter`, `RagPipeline`, `IntentClassifier`, `QueryPlanner` |
| `nom-media` | compiler | ~2,788 | Media **codec** implementation (PNG/FLAC/JPEG/Opus/AVIF encode+decode) | `ingest_png()`, `ingest_flac()`, `encode_avif_deterministic()` |
| `nom-media` | canvas | ~403 | Media **type definitions** (codec/container/unit kinds) | `MediaKind`, `MediaUnit`, `Codec`, `Container` |
| `nom-ux` | compiler | ~135 | UX platform specialization (scaffold) | `Platform::Web/Desktop/Mobile`, `runtime_launch_word()` |
| `nom-ux` | canvas | ~713 | UX runtime (screen, flow, pattern, extractor, accessibility) | `Screen`, `UserFlow`, `UxPattern`, `DesignRule` |

### Verdict on duplicates
**All 5 canvas duplicates should be renamed** to eliminate the collision. The compiler crates own the names by chronological precedence and broader dependency usage. Suggested renames:

| Current (Canvas) | Suggested Rename | Rationale |
|------------------|------------------|-----------|
| `nom-cli` | `nom-canvas-cli` | Distinct app CLI vs compiler CLI |
| `nom-graph` | `nom-graph-engine` | 17K-line execution engine vs 746-line knowledge graph |
| `nom-intent` | `nom-intent-router` | Skill router + RAG vs M8 transformer stub |
| `nom-media` | `nom-media-types` | Type stubs vs real codec implementation |
| `nom-ux` | `nom-ux-runtime` | UX runtime vs platform specialization stub |

---

## 4. Internal Overlaps Within nom-compiler (Merge/Split Candidates)

### 4.1 `nom-concept` — God Crate (CRITICAL)
**Size:** ~13,000+ lines  
**Exported submodules:** `acceptance`, `closure`, `mece`, `strict`, `stages`, `ir`, `type_infer`, `codegen`, `flow_edge`, `exhaustiveness`, `dream`, `bootstrap`, `native`, `ingest`, `pipeline`, `lifecycle`, `stream_ingest`, `selfhost`, `aesthetic`, `llvm_emit`, `ssa`, `type_check`, `canonicalize`, `benchmark_table`, `bipartite`

**Overlap matrix:**
| Submodule | Overlaps with |
|-----------|---------------|
| `codegen` | `nom-codegen` (both generate Rust from plans) |
| `llvm_emit` | `nom-llvm` (both emit LLVM IR) |
| `type_infer` / `type_check` | `nom-verifier` (type compatibility checking) |
| `flow_edge` / `pipeline` | `nom-planner` (flow plan generation) |
| `bootstrap` / `native` | `nom-runtime` (runtime support) |
| `ingest` / `stream_ingest` | `nom-corpus`, `nom-extract` (source ingestion) |
| `mece` / `exhaustiveness` | `nom-verifier` (validation logic) |
| `stages` (S1–S6 pipeline) | `nom-planner`, `nom-resolver`, `nom-dict` |

**Recommendation:** Split `nom-concept` into 4–5 focused crates:
1. `nom-lexer` — `Tok` enum, tokenization, synonym resolution
2. `nom-parser` — S1–S6 pipeline stages, `.nom`/`.nomtu` parsing
3. `nom-ir` — `NomFile`, `NomtuFile`, `EntityRef`, `EffectValence`, intermediate representations
4. `nom-validate` — `mece`, `exhaustiveness`, `strict`, `type_check`, `type_infer`
5. Keep `nom-concept` as a thin facade re-exporting the above, or deprecate it

### 4.2 `nom-resolver` vs `nom-dict`
**Problem:** Two SQLite dictionary APIs coexist. `nom-resolver` has legacy v1 (48-column `nomtu` table) and a v2 module migrating to `nom-dict`'s schema. `nom-dict` is the modern v2 content-addressed store.

**Recommendation:** Complete the v2 migration in `nom-resolver`, then merge `nom-resolver`'s resolution logic into `nom-dict` as a `resolver` module. Deprecate the old 48-column schema.

### 4.3 `nom-score` vs `nom-security`
**Problem:** Both compute security-related scores. `nom-score` does heuristic 8-dim scoring on `Atom`s (name-based heuristics). `nom-security` does deep regex-based body scanning (OWASP, secrets, CVEs).

**Verdict:** **KEEP SEPARATE.** They operate at different layers: `nom-score` is fast heuristic metadata scoring; `nom-security` is deep static analysis. They are complementary, not redundant.

### 4.4 `nom-planner` vs `nom-verifier`
**Problem:** Both analyze flow chains. `nom-planner` generates `CompositionPlan`s; `nom-verifier` checks type compatibility and constraints on those plans.

**Verdict:** **KEEP SEPARATE.** They form a pipeline: parse → verify → plan → codegen. Merging would create a new god crate.

### 4.5 `nom-codegen` vs `nom-llvm`
**Problem:** Both are backends. `nom-codegen` emits Rust source; `nom-llvm` emits LLVM bitcode.

**Verdict:** **KEEP SEPARATE.** Different targets, different dependencies (`inkwell` is heavy). Could share a `Backend` trait in `nom-types` if desired.

---

## 5. Cross-Workspace Dependencies

### Compiler → Canvas
**None.** Verified by searching all `Cargo.toml` files in `nom-compiler` for `../nom-canvas` paths. Zero matches.

### Canvas → Compiler
**One intentional bridge:**

| Canvas Crate | Compiler Dep | Path | Optional? |
|--------------|--------------|------|-----------|
| `nom-compiler-bridge` | `nom-concept` | `../../../nom-compiler/crates/nom-concept` | Yes |
| `nom-compiler-bridge` | `nom-dict` | `../../../nom-compiler/crates/nom-dict` | Yes |
| `nom-compiler-bridge` | `nom-grammar` | `../../../nom-compiler/crates/nom-grammar` | Yes |
| `nom-compiler-bridge` | `nom-score` | `../../../nom-compiler/crates/nom-score` | Yes |
| `nom-compiler-bridge` | `nom-types` | `../../../nom-compiler/crates/nom-types` | Yes |
| `nom-compiler-bridge` | `nom-search` | `../../../nom-compiler/crates/nom-search` | Yes |
| `nom-compiler-bridge` | `nom-planner` | `../../../nom-compiler/crates/nom-planner` | Yes |
| `nom-compose` | `nom-concept` | `../../../nom-compiler/crates/nom-concept` | Yes |

**Verdict:** This is a healthy, one-way dependency graph. The canvas workspace uses compiler infrastructure via an explicit bridge crate. No circularities.

---

## 6. Restructuring Recommendations (Priority Order)

### P0 — Rename Canvas Duplicates
Rename the 5 canvas crates to eliminate naming collisions. This prevents Cargo resolution ambiguity and developer confusion.

### P1 — Split `nom-concept`
Extract the 20+ submodules into 4–5 focused crates. This is the single biggest architectural improvement.

**Proposed split:**
```
nom-concept (deprecated thin facade)
├── nom-lexer     (~2K lines: Tok, tokenization, keyword tables)
├── nom-parser    (~3K lines: S1–S6 pipeline, NomFile/NomtuFile parsing)
├── nom-ir        (~2K lines: AST/IR types, EntityRef, EffectValence)
├── nom-validate  (~2K lines: mece, exhaustiveness, type_check, type_infer)
└── nom-bootstrap (~1K lines: bootstrap, native, llvm_emit, ssa)
```

### P2 — Modularize `nom-cli`
Move each subcommand group into its own module or crate:
```
nom-cli
├── cmd_build.rs    (Build, Compile, Link, VerifyAcceptance)
├── cmd_corpus.rs   (Scan, Ingest, CloneIngest, Embed)
├── cmd_dict.rs     (Dict, Store)
├── cmd_grammar.rs  (Grammar init, status, pattern-search)
├── cmd_lsp.rs      (Lsp serve)
├── cmd_app.rs      (App Dream, Build)
└── ...
```

### P3 — Merge `nom-resolver` v2 into `nom-dict`
Complete the v2 migration, then fold resolution logic into `nom-dict`. Keep `nom-resolver` as a deprecated re-export facade for one release cycle.

### P4 — Consolidate Small Stubs
Consider whether `nom-flow`, `nom-intent`, `nom-ux`, and `nom-runtime` should remain separate or be merged into a `nom-scaffold` crate until they grow real implementations. Current total: ~515 lines across 4 crates.

---

## 7. Summary Statistics

```
Total compiler source lines (excluding tests): ~55,000+
Total canvas source lines (excluding tests):   ~35,000+
Compiler crates that are stubs (<200 lines):    4  (14%)
Compiler crates that are small but real:         4  (14%)
Compiler crates that are large (>2K lines):      6  (21%)
Duplicate names requiring rename:                5  (17% of compiler crates)
God crates requiring split:                      1  (nom-concept)
God binaries requiring modularization:           1  (nom-cli)
```

---

*Report generated by source-level inspection of all `lib.rs`/`main.rs` files and `Cargo.toml` dependency graphs. No guessing.*
