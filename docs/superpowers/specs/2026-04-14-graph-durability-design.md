# Graph Durability Bundle — Design Spec

**Date:** 2026-04-14
**Scope:** `nom-dict` (Phase 1) + `nom-graph` (Phases 2 + 3)
**Brainstorm shape:** β (bundle #3 UID identity + #1 incremental upsert as unified refactor; sequence #4 staleness first and #2 Cypher export last)
**Source discovery:** mined from `GitNexus-main` via MCP Cypher queries 2026-04-14

## Problem

Three compounding gaps in Nom's dict + graph surface:

1. **Dict can silently go stale.** After `cargo test` or a source edit, `nom resolve` / `nom build status` can return outdated results because nothing compares the dict's last-ingested source to the current working-tree source. Users can't tell until mismatches show up downstream.
2. **`nom-graph` node identity is positional** (`Vec<NomtuNode>` indexed by position). A rename (body changes, `body_hash` flips) breaks every existing edge silently.
3. **`nom-graph`'s only entry point is full rebuild** (`from_entries(&[NomtuEntry])`). Every dict mutation forces recomputing the whole graph. At 6095 nodes this is fast; at the M6 PyPI-100 scale (~10k+ entries) it becomes noticeable.

## Solution shape

Three phases, 3 commits, ≥15 new tests:

1. **Phase 1 — staleness detection in `nom-dict`** (½ day). Mirrors the GitNexus `status.ts` pattern 1:1. `dict_last_source_hash` column in the dict's `meta` table; `NomDict::{current_source_hash, stored_source_hash, mark_source_hash, is_stale}` API; `nom store status` CLI.
2. **Phase 2 — unified UID identity + incremental upsert in `nom-graph`** (1–2 weeks). Swap `Vec<NomtuNode>` → `HashMap<NodeUid, NomtuNode>`. Add `upsert_entry(&mut self, &NomtuEntry) -> UpsertOutcome` + `remove_entry`, both with dirty-set edge repair. `prior_hashes: HashMap<NodeUid, Vec<NodeUid>>` preserves rename history.
3. **Phase 3 — Cypher-compatible export** (2 days). `nom graph export --format cypher --out <dir>` emits LadybugDB CSV dump (one `nodes_*.csv` per label + one `edges_*.csv` per edge type + an `import.cypher` `LOAD FROM` script). Enables roundtrip through GitNexus.

## Architecture

### Phase 1 — `nom-dict::DictFreshness`

```
┌─────────────────────────────────────────────────────────────┐
│ Dict (SQLite)                                                │
│                                                              │
│ meta table (new column):                                     │
│   dict_last_source_hash: TEXT NULL                           │
│                                                              │
│ API (NomDict impl):                                          │
│   current_source_hash(repo_root) -> io::Result<String>       │
│     → SHA-256 over sorted map of                             │
│        (rel_path -> sha256_of_file_contents)                 │
│        for every file matched by nom-extract's scan filter   │
│   stored_source_hash() -> Result<Option<String>>             │
│     → reads meta row                                         │
│   mark_source_hash(hash) -> Result<()>                       │
│     → upsert into meta                                       │
│   is_stale(repo_root) -> io::Result<bool>                    │
│     → current != stored                                      │
└─────────────────────────────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────────────┐
│ CLI: nom store status [--dict nomdict.db] [--repo-root .]   │
│  Output (mirrors `gitnexus status`):                         │
│    Repository: /path/to/repo                                 │
│    Dict: nomdict.db (indexed 2026-04-14 03:42)               │
│    Indexed source hash: sha256:a1b2c3...                     │
│    Current source hash: sha256:a1b2c3... | d4e5f6...         │
│    Status: ✅ up-to-date | ⚠️ dict-stale (N files differ)     │
└─────────────────────────────────────────────────────────────┘
```

Reuses existing `nom-extract::scan::scan_directory` + `IGNORED_DIRS` filter, so the hashed file set matches what actually gets ingested.

### Phase 2 — UID identity + incremental upsert

**Identity scheme.** `NodeUid = hex(SHA-256(word || "::" || kind || "::" || body_hash))`. Survives pure syntactic edits (whitespace-only `body_hash` change → new uid, but the prior uid is recorded). Same word+kind with a different body = logically-renamed, not-identical node.

**Data structure.**

```rust
pub type NodeUid = String; // hex SHA-256, 64 chars

pub struct NomtuGraph {
    nodes: HashMap<NodeUid, NomtuNode>,
    edges: HashMap<(NodeUid, NodeUid, EdgeType), NomtuEdge>,
    /// current_uid -> prior uids that renamed into it, oldest first
    prior_hashes: HashMap<NodeUid, Vec<NodeUid>>,
    /// keep fast lookup by (word, variant) for ref resolution
    word_index: HashMap<(String, Option<String>), NodeUid>,
}
```

**Upsert contract.**

```rust
#[derive(Debug, PartialEq, Eq)]
pub enum UpsertOutcome {
    Unchanged,
    Created { uid: NodeUid },
    Updated { uid: NodeUid },         // same uid (body_hash unchanged, metadata refreshed)
    Renamed { from: NodeUid, to: NodeUid }, // body_hash changed; edges reattached
}

impl NomtuGraph {
    pub fn upsert_entry(&mut self, entry: &NomtuEntry) -> UpsertOutcome;
    pub fn remove_entry(&mut self, uid: &NodeUid) -> bool;
    pub fn history_of(&self, current_uid: &NodeUid) -> &[NodeUid];
    // Backward-compat: existing callers still work.
    pub fn from_entries(entries: &[NomtuEntry]) -> Self {
        let mut g = Self::new();
        for e in entries { g.upsert_entry(e); }
        g
    }
}
```

**Dirty-set edge repair** (`Renamed` case): when `upsert_entry` sees a word+kind+variant already present with a *different* body_hash:
1. Compute the new `uid`.
2. For every edge `(old_uid, *, _)` or `(*, old_uid, _)` with `confidence ≥ 0.7`, re-attach to new `uid` (edge key rewritten).
3. Edges with `confidence < 0.7` (fuzzy `SimilarTo` etc.) are dropped; callers re-derive via `build_import_edges` / `build_call_edges` for the touched subgraph.
4. Old `uid` → new `uid` recorded in `prior_hashes`.
5. `word_index` entry swapped.

### Phase 3 — Cypher-compatible export

```
out/
  nodes_NomtuNode.csv    (uid,word,variant,language,kind,body_hash)
  edges_Calls.csv        (from_uid,to_uid,confidence)
  edges_Imports.csv      (from_uid,to_uid,confidence)
  edges_Implements.csv   (...)
  edges_DependsOn.csv    (...)
  edges_SimilarTo.csv    (...)
  import.cypher          (generated LOAD FROM script, LadybugDB syntax)
```

Matches [LadybugDB's `LOAD FROM` CSV ingestion convention](https://docs.ladybugdb.com). Generated `import.cypher` includes schema DDL + one `LOAD FROM "..." INTO ...` per CSV.

CLI: `nom graph export --dict nomdict.db --out ./graph-dump --format cypher`.

Roundtrip test: `npx gitnexus cypher < ./graph-dump/import.cypher` loads the dump; query `MATCH (n:NomtuNode) RETURN count(n)` must match Nom's `cypher` command count.

## Components

- `nom-dict/src/freshness.rs` (Phase 1, ~80 LOC + ~30 test LOC)
- `nom-dict` CLI wiring in `nom-cli/src/store/commands.rs` (Phase 1, `cmd_store_status`)
- `nom-graph/src/lib.rs` Phase-2 rewrite (estimated ~300 LOC changed, ~200 added)
- `nom-graph/src/export.rs` new module (Phase 3, ~150 LOC)
- `nom-cli/src/main.rs` `GraphCmd::Export` variant (Phase 3)

## Data flow (integration test at end of all phases)

```
1. `nom store add examples/concept_demo/app.nom`  → dict receives new entry
2. `nom store mark-fresh`                          → dict_last_source_hash updated
3. Edit examples/concept_demo/app.nom             → working tree diverges
4. `nom store status` → ⚠️ dict-stale              (Phase 1)
5. `nom store sync` → re-ingests; reports Renamed  (Phase 2 upsert)
6. `nom graph export --out /tmp/g --format cypher` (Phase 3)
7. `gitnexus cypher < /tmp/g/import.cypher`        → roundtrip loaded
8. `gitnexus cypher "MATCH (n) RETURN count(n)"`   → matches Nom count
```

## Error handling

All new errors use `thiserror` with structured fields (no `String` soup):

```rust
#[derive(Debug, Error)]
pub enum FreshnessError {
    #[error("scan {repo_root:?} failed: {source}")]
    Scan { repo_root: PathBuf, source: std::io::Error },
    #[error("no meta row for dict — run `nom store init` first")]
    UninitializedMeta,
}

#[derive(Debug, Error)]
pub enum UpsertError {
    #[error("entry {word:?}+{kind:?} has no body_hash — upsert requires hashed entries")]
    MissingBodyHash { word: String, kind: String },
    #[error("identity hash collision: uid {uid} but body_hash mismatch")]
    HashCollision { uid: NodeUid },
}

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("output dir {0:?} is not empty — refusing to clobber; pass --force")]
    NonEmptyOutDir(PathBuf),
    #[error("csv write for {label}: {source}")]
    CsvWrite { label: &'static str, source: csv::Error },
}
```

## Testing strategy

**Phase 1 (≥3 tests, in `nom-dict/src/freshness.rs`):**
- `fresh_dict_reports_stale` — brand-new dict + source dir returns `is_stale = true` (no stored hash yet)
- `unchanged_source_reports_fresh` — mark, no edits, check → `false`
- `edited_source_reports_stale` — mark, edit one file, check → `true`

**Phase 2 (≥8 tests, in `nom-graph/src/lib.rs`):**
- `upsert_on_empty_graph_creates_node`
- `upsert_same_body_returns_unchanged`
- `upsert_metadata_bump_returns_updated`
- `upsert_new_body_returns_renamed_and_preserves_history`
- `renamed_reattaches_calls_edges_above_confidence_threshold`
- `renamed_drops_fuzzy_similar_to_edges`
- `remove_entry_cascades_edges`
- `history_of_returns_full_rename_chain`
- `word_index_stays_consistent_under_upsert`

**Phase 3 (≥4 tests, in `nom-graph/src/export.rs`):**
- `export_emits_expected_files_for_shipped_demo`
- `empty_graph_emits_empty_csvs_with_headers`
- `deterministic_ordering_across_two_runs` (same graph → same bytes)
- `roundtrip_via_ladybug_import` (gated on `GITNEXUS_CLI=true` env var; skips otherwise)

**Integration test at the end** (`nom-cli/tests/graph_durability_e2e.rs`): executes the 8-step data-flow above against a temp dict + demo dir.

Property-test shape (`quickcheck`-style, optional): random mutation sequences against the graph, invariant `∀ entry: upsert is idempotent when body_hash unchanged` must hold.

## Out of scope

- Edge-weight re-computation (fuzzy-similarity scoring stays as-is in Phase 2; future slice can add score-adjustment on rename)
- Community detection re-computation (`detect_communities` still runs full recompute; incremental Louvain is quarters-scale per doc 10 §B)
- Concurrent mutation of `NomtuGraph` (single-thread only; document the invariant; add `!Send` marker if needed)
- Indexing/querying `prior_hashes` in reverse for "what's the current uid of this old uid?" — that's a Phase-4 slice once a consumer needs it.

## Sequencing

Spec approved → writing-plans skill → three implementation commits in order (staleness, identity+upsert, export), each with its own phase-specific test gate green before the next commit. Integration test lands with phase 3. Each commit triggers `npx gitnexus analyze` per hard requirement.

## Spec self-review (2026-04-14)

- ✅ No placeholders, TODOs, or TBDs remain.
- ✅ Phase 1 and Phase 2 interfaces don't contradict (`current_source_hash` + `upsert_entry` are in different crates, no shared types).
- ✅ Error taxonomy is per-phase, no cross-leaking.
- ✅ Integration test ties all three phases together and is testable from a single e2e file.
- ✅ Scope is single-plan-sized (~1 engineering week focused); larger than a wedge but well-contained.

Next step: invoke `writing-plans` skill (or, per session pattern, proceed directly to Phase 1 implementation wedge in the same cycle).
