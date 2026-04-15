# Nom Compiler — Full State Machine Report

> **Sources:** GitNexus MCP (6098 symbols · 15340 edges · 300 flows · Nom index),  
> all `research/0X-*.md` docs, `CYCLE-3-MIGRATION-SPEC.md`, and direct crate file inspection.  
> **Date:** 2026-04-15

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

## 2. Component Registry — All 31 Crates

### 2.1 Language Frontend (Parse & Classify)

#### `nom-lexer`
| Aspect | Detail |
|--------|--------|
| **Function** | Tokenise source files for the OLD `nom-parser` flow-classifier grammar |
| **Features** | `SpannedToken`, `Token` enum; classifier keywords (`nom`, `flow`, `store`, `test`, `agent`, `graph`, `gate`, `pool`, `view`), statement keywords (`need`, `require`, `effects`, `flow`, `describe`, `contract`, `implement`, `given`, `when`, `then`), identifiers, string/numeric literals, punctuation, `ColCol` (`::`) path separator |
| **Input** | Raw `&str` source |
| **Output** | `Vec<SpannedToken>` |
| **Key symbols** | `skip_whitespace`, `peek`, `peek_span`, `advance`, `is_classifier` |
| **State** | Shipped; self-hosting lexer compiles through LLVM backend end-to-end |
| **Note** | This is a **different lexer** from the one inside `nom-concept/src/lib.rs`. The `nom-concept` internal lexer handles prose-English grammar (`the`, `is`, `uses`, `composes`, `@Kind`, etc.). These two lexers serve the two separate parsers described below. |

#### `nom-parser` ⚠️ LEGACY — scheduled for deletion
| Aspect | Detail |
|--------|--------|
| **Function** | **Function 1: Parse authored source code into a `SourceFile` AST.** This is the OLD, currently-live parser. Entry point: `parse_source(&str) → SourceFile`. Uses the `nom-lexer` flow-classifier token set. |
| **Grammar** | `declaration* EOF`; each declaration starts with a classifier keyword (`nom`, `flow`, `store`, `test`, `agent`, `graph`, etc.) followed by an identifier name, then statement body (`need`, `require`, `effects`, `describe`, `contract`, `implement`, `given/when/then`, `let`, `if`, `for`, `while`, `match`, `fn`, `struct`, `enum`, `trait`, `impl`, `use`, `mod`) |
| **Input** | Any source file (no `.nomx`-specific extension enforced) |
| **Output** | `SourceFile { declarations: Vec<Declaration> }` → fed directly to `nom-llvm` for `.bc` compilation |
| **State** | Legacy parser crate still exists. `cmd_store_add` no longer uses it, but the general `nom-cli` parse/build/check/report/fmt flows still depend on it because the temporary AST bridge from `nom-concept` does not yet preserve full statement bodies. |
| **Docstring says** | *"the legacy flow-style entry-format parser scheduled for deletion as the .nom / .nomtu pipeline absorbs its remaining callers"* |
| **Gap** | Must eventually be replaced by `nom-concept` S1-S6 everywhere. Today the new prose syntax is live for `nom store add`, while the broader legacy CLI remains intentionally on `nom-parser` until a full-fidelity bridge or native `.nom` / `.nomtu` execution path lands. |

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

**State:** A1/A2/A3/A4/A6 closed. A5 pending refactor. Grammar-aware S3 now validates both clause-shape presence and required-clause presence, with coverage in `clause_shape_guard.rs`. `cmd_store_add` is wired to the pipeline, and `nom build manifest` effect collection now also uses `nom_concept::stages::run_pipeline`, closing the last legacy `.nomtu` reparse path in `nom-cli`. Remaining gaps are around downstream artifact generation and deletion of residual legacy parser call sites.  
**Gap:** See GAP-0 / GAP-4 — the pipeline is live in `nom-cli`, but `nom-parser` still exists for legacy/test-only surfaces and the `.nomx` ingest path still stores rows without the final `.bc` artifact step.

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
| **State** | Stubbed (real planner-in-Nom is in-flight) |
| **Gap** | Replacement of stub is in-flight; self-host tests (`self_host_planner.rs`) exist |

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
| **Features** | Slices 1-6: stdio server, classify CLI, agentic-RAG markdown rendering, `executeCommand` handler, ReAct adapter trait + stubs; Slice 7a: pattern-driven completion via `search_patterns` backend (17 dispatch tests) |
| **Input** | LSP JSON-RPC over stdio |
| **Output** | Completion items, hover, diagnostics, execute-command |
| **State** | Slices 1-7a shipped |
| **Gap** | Full hover / goto-def / embedding-ranked completion (LSP MVP) planned but not started |

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
| 0 | Workspace + 31-crate scaffold | ✅ Shipped | 100% |
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

### GAP-0: Pipeline Wiring — `nom-concept` S1-S6 into `cmd_store_add` 🟡 IN-FLIGHT

**What changed:** The new prose-English S1-S6 parser (`nom-concept/stages.rs`) is now the `cmd_store_add` entry point. The ingest path also now writes sync-compatible entity/module rows: compositions are stored as `kind = "module"`, `composed_of` is a JSON list of hashes-or-words instead of raw `EntityRef` structs, and provenance is preserved consistently (`authored_in` / `origin_ref`) across both `store add` and `store sync`. `nom store sync` now also drives `.nom` / `.nomtu` sources through the new `nom_concept::stages::run_pipeline` path instead of legacy parse wrappers. `store add` also now hashes entity/module declarations with the same canonical serde-based content hash shape that `store sync` uses, so the same `.nomtu` declaration does not get one id through single-file ingest and a different id through repo sync. Concept rows also derive `repo_id` from the source file's parent directory instead of hard-coding `"default"`. That last piece matters because `build status`, `build manifest`, and `materialize_concept_graph_from_db()` all query by repo basename. The remaining work is to flesh out post-parse artifact generation so the path does more than split-DB row creation.

**The correct file-format pipeline (now being wired):**
```
  .nomx (authored prose)
    └─► S1-S6 (nom-concept)
          ├─► PipelineOutput::Nom    → .nom format → concept_defs (concepts.sqlite)
          └─► PipelineOutput::Nomtu → .nomtu format → entities (entities.sqlite)
                                          └─► nom-llvm → .bc → artifact store
```

**Until fully wired, these are blocked:**
- `PipelineOutput::Nomtu` rows still land without the final `.bc` artifact follow-through
- build/run/check/report/fmt still execute through the legacy `SourceFile` parser path rather than a full-fidelity `.nom` / `.nomtu` path
- `.nom`/`.nomtu` tier split is not yet the primary executable build path for authored source
- `materialize_concept_graph_from_db` can read split-DB rows, but downstream artifact expectations are still incomplete

**Actions:**
1. ✅ Replace `nom-parser::parse_source()` in `cmd_store_add` with `nom-concept::run_pipeline()`
2. ✅ Route `PipelineOutput::Nom` → `concept_defs` write via `nom_dict::upsert_concept_def()`
3. ✅ Route `PipelineOutput::Nomtu` → `entities` write via `nom_dict::upsert_entity()` with sync-compatible module/provenance payloads in both `store add` and `store sync`, and with aligned declaration hashing across those paths; concept rows now also land under a build-visible `repo_id`; `.bc` compile → artifact store still open
4. ✅ Keep the broader `nom-cli` parser/build surfaces on `nom-parser` for now instead of the lossy temporary AST bridge, while `nom build manifest` now also reads effects via `nom-concept::stages::run_pipeline`
5. `[ ]` Delete `nom-parser` once all callers are migrated (GAP-4)

**Effort:** Medium — S1-S6 pipeline done; wiring + `Dict` routing is the remaining work.

---

### GAP-1: Dict-Split Completion (Cycle 3) 🟢 SUBSTANTIALLY COMPLETE

**What's done (Phase 1 & 2):** 
- Ported all internal compiler logic in `nom-cli`, `nom-app`, `nom-resolver`, and `nom-intent` (both source and tests) from `NomDict` to `Dict` free functions.
- `upsert_entry`, `upsert_entry_if_new`, `find_entries`, `get_entry`, etc., are now accessed via `nom_dict::dict` free functions.
- Introduced `open_nomdict_legacy()` in `nom-cli` strictly for backward compatibility of atom-level commands (`extract`, `score`, `stats`, `coverage`) that still expect the old `NomDict` struct interfaces until they are fully migrated to the `.nomtu` tier.
- `nom-cli` concept command handlers (`cmd_concept_new`, `cmd_concept_add`, `cmd_concept_add_by`, plus the earlier list/show/delete bridge) now open `Dict` directly and use split-aware free functions.
- `nom store sync` now opens `Dict` directly, drives `.nom` / `.nomtu` sources through `nom_concept::stages::run_pipeline`, and writes `concept_defs` / `entities` through free functions instead of `NomDict` methods.
- `nom corpus register-axis`, `nom corpus seed-standard-axes`, and `nom corpus list-axes` now open `Dict` directly and use split-aware required-axis free functions.
- `nom-corpus` core ingest/clone entry points (`ingest_directory`, `ingest_parent`, `clone_and_ingest`, `clone_batch`, `ingest_pypi_top`) now take `&Dict` instead of `&NomDict`, with transactions preserved on the `entities` tier.
- ✅ `nom store add-media` migrated to use `Dict::try_open_from_nomdict_path` + `nom_dict::upsert_entry` free function (2026-04-15)

**What's missing (Phase 3 - Cleanup):**
- Porting the backward-compatible atom commands (`extract`, `score`, `stats`, `coverage`) to use the S1-S6 pipeline and `.nomtu` formats.
- Complete deletion of `NomDict` struct, the old legacy `entries` and `concepts` tables, and V2-V5 schema constants.

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

### GAP-4: `.nomx` Format Unification 🟡 IN-FLIGHT

**What's missing:** v1 vs v2 distinction not fully purged from parser, tests, tooling. `store add`, `store sync`, and `build manifest` now use the new `nom-concept` pipeline, but the broader CLI parse/build/check/report/fmt surfaces and self-host regression tests still depend on legacy `nom-parser` today.

**Actions:**
- Delete remaining v1-only code paths in `nom-parser` and tests
- Confirm single `run_pipeline_with_grammar` is the sole entry point everywhere
- Merge `nomx v1 + v2` test fixtures

---

### GAP-5: S3 Required-Clause Presence Check ✅ SHIPPED

**What shipped:** S3 now checks both that `clause_shapes` rows exist for a kind and that every `is_required=1` clause is present in the block body before the grammar-aware pipeline advances.

**Evidence:**
- `nom-concept/src/stages.rs` now queries `required_clauses_for_kind()` and rejects with `missing-required-clause`
- `nom-concept/tests/clause_shape_guard.rs` now covers both the missing-clause rejection and the all-required-clauses-present success path

---

### GAP-6: real `nom-planner` 🟡 IN-FLIGHT

**What's missing:** Current planner is stubbed. Composition plan fusion/reorder/specialization is not implemented.

**Actions:**
- Implement planner logic (fuse pure steps, reorder independent steps)
- Wire through `nom-llvm` specialization hooks
- Ship self_host_planner.rs tests green

---

### GAP-7: LSP MVP 🟡 PLANNED

**What's missing:** Full hover, goto-def, embedding-ranked completion.

**Shipped so far:** Slices 1-7a (stdio, classify, RAG markdown, executeCommand, ReAct adapter, pattern completion).

**Gap:** Embedding-ranked completion (depends on GAP-2), hover resolution, goto-def dictionary lookup.

---

### GAP-8: AI Authoring Loop Closure 🟡 IN-FLIGHT

**What's missing:** Real bytecode-linking + binary emission in the ReAct loop (slice-3c-full).

**Actions:**
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

### GAP-12: Grammar Wedge Backlog 📋 QUEUED

11 shapes queued (each needs design + parser + tests):

| Wedge |
|-------|
| Format-string interpolation |
| Nested-record-path syntax |
| Sum-type `@Union` typed-kind |
| Wire-field-tag clause |
| Pattern-shape clause on data decls |
| Exhaustiveness check on `when` clauses |
| Retry-policy clause |
| Watermark clause (streaming) |
| Window-aggregation clause |
| Clock-domain clause |
| QualityName-registration formalization |

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
| Self-host pipeline (`nom-cli`) | `self_host_pipeline`, `self_host_ast`, `self_host_codegen`, `self_host_parser`, `self_host_planner`, `self_host_smoke`, `self_host_verifier`, `self_host_rust_parity`, `self_host_meta`, `self_host_parse_smoke` | ✅ (various) |
| E2E (`nom-cli`) | `build_report_e2e`, `build_manifest_e2e`, `acceptance_preserve_e2e`, `concept_demo_e2e`, `agent_demo_e2e`, `layered_dream_smoke`, `three_tier_recursive_e2e`, `corpus_axes_smoke`, `store_cli`, `store_sync_smoke`, `avif_ingest`, `bc_body_round_trip`, `media_import`, `mcp_smoke`, `locale_smoke`, `phase4_acceptance`, `parser_subset_probe` | Mostly ✅ |

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
NOW (unblocked):
  0. Complete GAP-0: Wire nom-concept S1-S6 into cmd_store_add  ← MOST CRITICAL
     (Without this, the designed architecture never executes in production)
  1. Complete GAP-1: Port 8 NomDict methods → &Dict free fns (Cycle 3)
  2. Complete GAP-4: Delete nom-parser after GAP-0 wiring
  3. Complete GAP-6: Ship real nom-planner (fuse + reorder)
  4. Complete GAP-11: nom corpus register-axis CLI

WHEN NETWORK + DISK AVAILABLE:
  6. Start GAP-3: M6 corpus pilot (PyPI top-100)
  7. After corpus rows land: GAP-2: Wire embedding index (nom corpus embed)

AFTER EMBEDDING INDEX:
  8. GAP-7: LSP MVP (hover + goto-def + embedding completion)
  9. GAP-8: Close AI authoring loop (slice-3c-full)
  10. GAP-12: Ship priority grammar wedges (retry-policy first)

PHASE 6+:
  11. GAP-9: Author parser in .nomx
  12. Phase 7: Author resolver + verifier in .nomx
  13. GAP-10: Bootstrap fixpoint proof
```

---

## 10. Session Progress (2026-04-15)

### Completed This Session

**GAP-1 Phase 3a: Store Add-Media Migration**
- ✅ Migrated `nom-cli/src/store/add_media.rs` from `NomDict` to `Dict`
- ✅ Replaced `NomDict::open` with `Dict::try_open_from_nomdict_path`
- ✅ Replaced `dict.upsert_entry()` method call with `nom_dict::upsert_entry()` free function
- ✅ Verified compilation succeeds
- ✅ Committed: "Migrate nom-cli store add-media to use Dict free functions"

### Next Steps (High Priority)

**Remaining GAP-0 Work** (Pipeline Wiring):
- [ ] fmt.rs: Update `format_source()` to work with both legacy AST and new pipeline
- [ ] check/report: Create bridge for verifier/security checker to work with new formats
- [ ] Update test suites to use new pipeline where appropriate

**Remaining GAP-1 Work** (Dict-Split Migration):
- [ ] author.rs: Migrate `write_proposals_to_dict()` NomDict fallback path to Dict
- [ ] Update remaining test files that directly call `NomDict::open`
- [ ] Verify all Dict migrations are consistent in error handling

**GAP-4 Preparation** (nom-parser Deletion):
- [ ] Create an inventory of all remaining nom_parser dependencies
- [ ] Design bridge strategy for legacy test surfaces
- [ ] After all migrations complete, remove nom-parser crate

*Report last updated: 2026-04-15 by automated session*
