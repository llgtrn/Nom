# 12 — Entity scope, parse granularity, and the compose-from-artifacts pipeline

**Date:** 2026-04-14
**Purpose:** Deep-think + web-research pass answering seven architectural questions user raised 2026-04-14: *what exactly IS an entity in the dict, at what scope do we parse, how do dependencies work, how do we ingest .bc + media from the dict to compose an app, how does nom-graph participate in the concept layer, and how does external-repo ingestion produce entities?*

> This is a research note, not a design spec. It grounds the next brainstorm/spec round rather than prescribing a single implementation. Three web searches + Nom's existing taxonomy inspection + GitNexus graph structure feed the synthesis.

---

## 1. What IS an entity in Nom's dict?

**Answer: an entity is a hash-addressed content unit with a typed kind, a contract, and optional compiled/rendered bytes.** Not a file, not a function, not a module — a **composable unit**. The current `EntryKind` enum ([nom-types/src/lib.rs:396](../../nom-compiler/crates/nom-types/src/lib.rs)) already spans **27 kinds** across three bands:

### Band A — code kinds (11)
`Function, Method, Schema, ApiEndpoint, Ffi, ExternalOpaque, Module, Trait, Struct, Enum, TestCase`

### Band B — surface/app kinds (12)
`UxPattern, DesignRule, Screen, UserFlow, Skill, AppManifest, DataSource, Query, AppAction, AppVariable, Page, Concept`

### Band C — runtime/media kinds (4)
`BenchmarkRun, FlowArtifact, MediaUnit, Codec, Container`

Plus `AtomKind` (sibling taxonomy) carries **38 higher-level patterns** — `AuthFlow, RetryLogic, RagPipeline, AgentToolLoop, EtlPipeline, …`. Atoms compose from entries; entries are the leaves.

### The rule: **an entity is what can be hash-addressed with its full contract**

Not smaller (a statement or block has no contract worth hashing).
Not larger (a whole file is too coarse — multiple entities per file is normal).
The granularity matches **the unit you'd reuse across two apps**: if you can say "I want `auth_session_compose`", it's an entity. If you have to say "the auth_session_compose code from lines 42-87 of concept_demo", you haven't carved out the entity yet.

This aligns with the 2026 Code-Property-Graph consensus (Nature article, CodeGraph/FalkorDB) — nodes at function/class/module granularity with typed edges. Nom is slightly more granular than CPG on the code side (methods distinct from functions, schemas distinct from structs) and significantly richer on the surface/runtime side.

---

## 2. What scope do we parse at?

**Answer: function-level for code entities, block-level for contracts/assertions, file-level for frames and concepts.** Three scope tiers in one pass.

### Tier 1 — function-level (for `Function, Method, ApiEndpoint, TestCase`, etc.)

Tree-sitter's `locals` + `tags` query model ([tree-sitter docs](https://tree-sitter.github.io/tree-sitter/4-code-navigation.html)) gives function bodies as natural scopes. Each function becomes one `NomtuEntry` with:
- `word` = the function name
- `kind` = `Function`/`Method`/`ApiEndpoint`/etc (inferred from context — `#[test]` → TestCase, handler mount → ApiEndpoint)
- `body_hash` = SHA-256 of the parsed AST canonicalized (not the raw bytes — canonicalization strips comments + whitespace so trivial reformatting doesn't create rename chains)
- `signature` = input/output types extracted from the signature node
- `contract` = the pre/post/require/ensure clauses if present

**Scope decision**: nested local functions are separate entries unless tightly coupled (hoistable). A 3-line closure inside `greet()` is NOT its own entry; a named inner function CAN be.

### Tier 2 — block-level (for contracts, effects, invariants)

Contracts don't have independent identity — they're attributes of their enclosing function. Same for effect annotations. They attach as fields on the parent entry, not as separate entries.

### Tier 3 — file-level (for `Concept, UserFlow, AppManifest, Page`)

Top-level concepts span an entire `.nom` file. `concept_demo/app.nom` is ONE `Concept` entry pointing at N child entries. This matches GRACE's "cross-level edges link every file node to the functions it declares" pattern (arxiv 2509.05980).

### Rejected granularities (explicitly not-entities)

- **Statements / expressions / block AST** — these are internal to an entity's body, not separately composable. Embedded in `body_hash`.
- **Whole repos** — a repo is a collection of concepts + modules, not itself an entity. Repo-level metadata lives in `entry_meta`.
- **Tokens** — obviously. Lexing is Tier-0.

---

## 3. How do dependencies work?

**Answer: typed edges between entity UIDs**, already modeled in `EdgeType` ([nom-types/src/lib.rs:711](../../nom-compiler/crates/nom-types/src/lib.rs)) with **~25 variants**:

### Code dependencies
`Calls, Imports, Implements, DependsOn, SimilarTo, SupersededBy, ContractMatches`

### UX/app dependencies
`Styles, Constrains, Recommends, InteractsWith, TransitionsTo, Specializes, BindsTo, Triggers, Reads, Writes, NavigatesTo, RunsOn`

### Flow/media
`HasFlowArtifact, FlowsTo, Encodes, ContainedIn, UsesColor, UsesPalette, Derives, EmbeddedGlyph, Frame, RendersOn`

**This is already a Code Property Graph + more.** What CPG calls CALLS / AST / CFG / PDG edges, Nom flattens into typed `EdgeType` variants but **extends to UI + runtime + artifact layers** — which no standard CPG covers.

### Dependency resolution (per entry)

Each `NomtuEntry`'s `depends_on: Vec<String>` field carries the **pre-walk seed** (bare-word deps the resolver must ground). After `nom store sync`:
1. Resolver walks seed refs → short-circuits `SupersededBy` to the head (doc 04 §5.10.2)
2. Closure walker from seed set → builds the **transitive depends_on closure**
3. MECE validator checks the closure against `required_axes` registry

**The closure IS the build manifest.** For `app.nom` → all entries needed to build it. For a function entry → all entries whose code `Calls` / `Imports` it. For a UserFlow → all Screens it `TransitionsTo`.

---

## 4. How do we ingest .bc + media + compose into a real app?

**Answer: `body_kind` tags + `body_bytes` column + the Phase 3 render tool.** Already 80% built.

### The ingest side (one-way, stream-and-discard per doc 04 §5.17.2)

For every entity, `body_bytes: Option<Vec<u8>>` holds the compiled artifact. `body_kind: Option<String>` tags what it is:
- `body_kind="llvm-bc"` + `body_bytes` = LLVM bitcode (from `compile_nom_to_bc` or from external Rust source translated via `nom-corpus` equivalence gate)
- `body_kind="avif"` + `body_bytes` = canonical image bytes (from `nom-media` PNG/JPEG → AVIF re-encoder)
- `body_kind="wasm"` + `body_bytes` = compiled WASM module
- `body_kind="rust-src"` + `body_bytes` = Rust source (for entries that stay source-level)

Invariant #15 from doc 04 §4.4.6: **`body_kind` tags the shape, `body_bytes` holds the contents, `body` (text) is deprecated**. Text-only entries use `body_kind="text/nomtu"` + bytes.

### The compose side (agentic, M8 driven)

The **5 grouped tools** from the M8 spec ([docs/superpowers/specs/2026-04-14-graph-rag-agentic-design.md](../../docs/superpowers/specs/2026-04-14-graph-rag-agentic-design.md)) map cleanly onto the pipeline:

| Tool | Covers ingest-to-compose step |
|---|---|
| `query` | retrieve candidate entries by subject/kind (what could fit this slot) |
| `compose` | propose a concept linking selected entries (what to wire together) |
| `verify` | MECE-check the composition + per-entry security/perf gates |
| `render` | **invoke the underlying toolchain to produce the app artifact** — link LLVM bitcode, bundle AVIFs, emit AppManifest JSON |
| `explain` | show the glass-box report for the final artifact |

**`render` is where ingested artifacts become an app.** Today (slice-3a+explain shipped) `DictTools::render` returns `Observation::Error("not yet wired (slice-3c)")`. Slice-3c wires:
1. For app targets → read all closure entries' `body_bytes` + `body_kind`
2. Group by kind: llvm-bc entries → linker input, avif entries → asset bundle, rust-src → final compile pass
3. Drive `nom-codegen` + `nom-llvm` + `nom-app::cmd_app_build` to produce the native binary + asset dir
4. Return `Observation::Rendered { target, bytes_hash }`

This is exactly the pipeline `cmd_app_build` ([nom-cli/src/main.rs:1500](../../nom-compiler/crates/nom-cli/src/main.rs)) ships for direct-build flows. **The agentic path reuses the same primitives, just iteratively + with CRAG-style re-query when composition fails.**

---

## 5. How does `nom-graph` help build the concept layer?

**Answer: it's the retrieval substrate Concepts query when they compose.** Two specific roles:

### Role 1 — community detection for "what's near this concept?"

`detect_communities()` on the graph gives natural clusters (e.g. all auth-adjacent entries cluster together because of dense `Calls`/`DependsOn` edges among them). When a Concept declares `use auth_session` via typed slot, the resolver can:
1. Find the `auth_session` entry's community
2. Surface all community members as high-relevance candidates for the rest of the concept's slots
3. Rank by (community cohesion × entry score × dependency distance)

This is **Microsoft GraphRAG's Local Search** applied to Nom's code graph.

### Role 2 — closure walk = build manifest

`build_call_edges` + `build_import_edges` + follow `DependsOn` = the exact closure the build driver needs. For `app.nom`:
- Start at the AppManifest entry
- Walk all outgoing Calls/Imports/DependsOn/Triggers/Reads/Writes
- Deduplicate by UID
- Sort topologically
- Hand to `cmd_app_build` as the build manifest

**Concept declaration → closure → build manifest is one pass through nom-graph.** Doc 08 §9 layered dreaming uses this same closure; the difference is that dreaming iterates swaps, while build commits the closure to a hash-pinned manifest.

### What Phase 2b's `upsert_entry` changes

Before Phase 2b: every dict mutation forced `NomtuGraph::from_entries()` full rebuild. At M6 PyPI-100 scale (10k+ entries) this is ~seconds every sync.
After Phase 2b: incremental uid-addressed updates. Renames produce `Renamed { from, to }` outcomes so the resolver can invalidate just the affected concepts, not the whole graph.

---

## 6. How does the system parse code AS entities AND ingest the concept/components of the external repo?

**Answer: two parallel extraction paths that share the same `NomtuEntry` output shape.** Both feed the same `entries` + `words_v2` tables.

### Path 1 — native `.nom` / `.nomx` authoring (the clean path)

1. Author writes `.nom` file
2. `nom store sync` parses via `nom-parser` → emits `NomtuEntry` directly
3. `body_kind="text/nomtu"` or `"llvm-bc"` after compile
4. Edges derived from `use` statements + `Calls`/`DependsOn` in the AST

### Path 2 — external repo ingestion (`nom-corpus`)

1. `nom corpus ingest https://github.com/foo/bar` or PyPI top-100
2. For each source file, `nom-extract::parse_and_extract` (tree-sitter) emits `UirEntity` objects at function/method/class granularity
3. `nom-corpus::equivalence_gate::run_gate` lifts each `UirEntity` into a `NomtuEntry`:
   - Extract body via `nom-corpus::equivalence_gate::translators::rust` (or python/js/go/cpp depending on source language)
   - Compile to LLVM bc if possible → `body_kind="llvm-bc"` + `body_bytes=<compiled>`
   - If translation fails → `body_kind="rust-src"` + original source bytes, status = `Partial`
   - Otherwise → status = `Complete`
4. Contracts and effects **inferred heuristically** from the source (e.g. `#[test]` → `TestCase`, `async fn` → adds `IO` effect, `#[non_exhaustive]` → `open_contract=true`)
5. Edges derived from tree-sitter's `Calls` queries + Cargo.toml `[dependencies]` → `Imports`/`DependsOn`

### What's SHARED between the paths (the invariant)

Both paths produce `NomtuEntry` rows. The dict doesn't know the difference. Downstream — MECE validator, resolver, build driver, agentic compose — cares only about `kind` + `body_kind` + `depends_on`, not provenance.

**Provenance lives in `entry_meta`** (source repo URL + commit SHA + license + translation confidence). It's queryable but not load-bearing for composition.

### Component-level ingestion (external repo's concepts)

A well-organized external repo has its OWN concept layer (packages, modules, classes). `nom-extract` preserves this:
- File-level `Module` entries with `depends_on` = all its imports
- Class-level `Struct`/`Trait`/`Enum` entries with method methods as inner `Function` entries
- Package-level `Concept` entry (one per Cargo.toml or package.json) with `depends_on` = all its Module entries

So ingesting `serde` produces ~200 `Function` entries (leaf primitives), ~50 `Struct`/`Enum` entries, ~15 `Module` entries, and 1 `Concept` entry (`concept=serde`). An app composing with serde pulls just the concept + its closure, not the whole corpus.

---

## 7. Scope + components summary: what the system IS, in one diagram

```
 ┌─────────────────────────────────────────────────────────────────────┐
 │  External sources                                                     │
 │  • Author's .nom/.nomx files        (Path 1 — native)                │
 │  • PyPI top-500 (10M functions)     (Path 2 — nom-corpus)            │
 │  • GitHub top-500/ecosystem         (Path 2)                         │
 │  • Media: PNG/JPEG/MP3/MP4/OBJ      (Path 3 — nom-media)             │
 └──────────────┬──────────────────────────────────────────────────────┘
                │
                ▼
 ┌─────────────────────────────────────────────────────────────────────┐
 │  Extraction + translation layer                                      │
 │  • nom-parser (for .nom/.nomx)                                       │
 │  • nom-extract (tree-sitter, 42 languages)                           │
 │  • nom-corpus::equivalence_gate (Rust/C/C++/Py/JS/Go → Nom IR)       │
 │  • nom-media (PNG/JPEG→AVIF, MP3→Opus, etc.)                         │
 │  Output shape: Vec<UirEntity> or Vec<NomtuEntry>                     │
 └──────────────┬──────────────────────────────────────────────────────┘
                │
                ▼
 ┌─────────────────────────────────────────────────────────────────────┐
 │  Dict (nomdict.db — SQLite, WAL)                                     │
 │  • entries table: id, hash, word, kind, body_hash, body_bytes, …    │
 │  • words_v2 table: id, hash, word, kind, body_kind, body_size, …    │
 │  • entry_meta (EAV): provenance, license, translation confidence     │
 │  • entry_edges: typed EdgeType with confidence                       │
 │  • entry_required_axes (M7a): MECE CE registry                       │
 │  • dict_meta (V5): freshness hash                                    │
 └──────────────┬──────────────────────────────────────────────────────┘
                │
                ▼
 ┌─────────────────────────────────────────────────────────────────────┐
 │  Graph + retrieval layer (nom-graph, Phase 2b)                       │
 │  • NodeUid = content-hash identity                                   │
 │  • uid_nodes: HashMap<NodeUid, NomtuNode>                            │
 │  • prior_hashes: rename chain                                        │
 │  • detect_communities() — GraphRAG substrate                         │
 │  • build_call_edges / build_import_edges — closure walk              │
 │  • export_to_dir() — Cypher CSV for cross-tool roundtrip (Phase 3a)  │
 └──────────────┬──────────────────────────────────────────────────────┘
                │
                ▼
 ┌─────────────────────────────────────────────────────────────────────┐
 │  Agentic-RAG compose loop (nom-intent)                               │
 │  classify_with_react(prose, budget, llm, tools) iterates:            │
 │                                                                       │
 │    Thought → Action(query|compose|verify|render|explain)             │
 │           ↘ Observation ↙                                            │
 │                                                                       │
 │  Bounded to 4 iters; LLM emits only registered NomIntent variants.   │
 │  Reject(Reason) = CRAG weak-retrieval signal, not loop terminal.     │
 └──────────────┬──────────────────────────────────────────────────────┘
                │
                ▼
 ┌─────────────────────────────────────────────────────────────────────┐
 │  Compose-from-artifacts render (Phase 3c-full, pending)              │
 │  render(app_uid, "native"):                                          │
 │    1. Walk closure from app_uid                                      │
 │    2. Group by body_kind                                             │
 │    3. Link llvm-bc → native binary (via nom-llvm)                   │
 │    4. Bundle avif/opus/wasm assets                                   │
 │    5. Emit glass-box report                                          │
 │  Output: { binary: Vec<u8>, assets: Dir, report: ReportJson }        │
 └──────────────────────────────────────────────────────────────────────┘
```

---

## 8. Concrete implications for upcoming wedges

The research confirms the current slice trajectory is correct:

1. **Phase 2c (unify Vec/uid storage)** — blocks efficient closure walking at M6 corpus scale. Still top priority.
2. **Phase 3b (edges export)** — small wedge, useful for cross-tool diff & GitNexus roundtrip validation.
3. **M8 slice-3b (compose + verify)** — reuses MECE validator as Self-RAG critique; matches research §3 "separate critique from generate" pattern.
4. **M8 slice-3c-full (render)** — the last missing link to close the compose-from-artifacts pipeline. **This is the biggest remaining user-visible wedge.**
5. **M8 slice-5b (real LLM adapter)** — unblocks actual ReAct behavior (not just stub).
6. **Corpus ingestion pilot (M6 PyPI-100)** — validates Path 2 works at scale. Blocked on network.

### Unexpected discovery from the research

The web searches revealed that **Code Property Graph + LLM integration** is an active 2026 research area (arxiv 2603.24837 "Bridging CPGs and Language Models"). Nom's architecture converges on this — typed edges + hash-addressed nodes + LLM-driven composition — but adds:
- **Media kinds** (CPGs are code-only; Nom treats images/audio/video as first-class graph members)
- **Contract + effect typing on every node** (CPG has AST/CFG/PDG but no contract layer)
- **Hash-pinned determinism** (CPGs are rebuild-from-source; Nom has byte-level manifest roundtrip)

These are genuine differentiators, not reinventions. Keep prioritizing what only Nom does.

## Sources
- [FalkorDB CodeGraph — build queryable knowledge graphs from code](https://www.falkordb.com/blog/code-graph/)
- [Code Property Graph + LLMs (arxiv 2603.24837)](https://arxiv.org/html/2603.24837)
- [DepsRAG: LLM-managed dependencies (arxiv 2405.20455)](https://arxiv.org/html/2405.20455v3/)
- [GRACE: repo-aware code completion via hierarchical graph fusion (arxiv 2509.05980)](https://arxiv.org/html/2509.05980v1)
- [Code vulnerability via augmented PDG + CodeBERT](https://www.nature.com/articles/s41598-025-23029-4)
- [Tree-sitter code navigation (tags + locals queries)](https://tree-sitter.github.io/tree-sitter/4-code-navigation.html)
- [LLVM project wiki](https://en.wikipedia.org/wiki/LLVM)
- [Compiling to LLVM Bitcode (GraalVM docs)](https://www.graalvm.org/latest/reference-manual/llvm/Compiling/)
- [Using Graph Databases for Dependency Analysis (2026)](https://medium.com/@vignesh.komarasamy/using-graph-databases-for-dependency-analysis-in-software-testing-490f1c740468)
