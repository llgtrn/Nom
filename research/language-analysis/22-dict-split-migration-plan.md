# 22 — Dict-Split Migration Plan (3 separated SQLite files)

**Filed 2026-04-14 very-very-late.** Status: **Draft plan; requires user sign-off before execution.**

> **User directive 2026-04-14:** "have to be 3 seperated" SQLite files. Current state: one physical `dict.sqlite` holds both DB1 (`concept_defs`) and DB2 (`words_v2`) as sibling tables; `grammar.sqlite` (from doc 21 P1, commit `b53f74c`) is already separate. Target: **three physical files** — `concepts.sqlite` + `words.sqlite` + `grammar.sqlite` — with artifact bodies continuing to live at `~/.nom/store/<hash>/body.*` (filesystem tree, not a DB).

This doc specifies the migration: **why** each tier wants its own file, **how** to reshape `nom-dict`'s API with minimum caller churn, **which** CLI flag semantics change, and **what** the atomic commit sequence looks like.

---

## 1. Final target layout

```
~/.nom/
  concepts.sqlite    ← DB1: concept_defs + entry_meta (concept rows) + related
  words.sqlite       ← DB2: words_v2 + entry_meta (word rows) + related
  grammar.sqlite     ← registry (5 tables, doc 21)
  store/<hash>/...   ← artifact bytes (unchanged)
```

**Invariant**: no cross-file foreign keys. DB1 stores hash strings that reference DB2 rows; the Rust layer resolves them.

---

## 2. Why three files — the lifecycle + scale + distribution argument

| File | Write frequency | Typical scale (today) | Typical scale (target) | Distribution model |
|---|---|---|---|---|
| `concepts.sqlite` | low (on `nom store sync <repo>`) | ≤100s rows | 10k–1M rows | per-repo or per-user |
| `words.sqlite` | high (on `nom corpus ingest`) | ≤1k rows | 10^8 rows | per-user (shared corpus) |
| `grammar.sqlite` | zero after build (regenerated from code) | ~5k rows | ~10k rows | shipped inside the `nom` binary release |

**Key reasons the split is load-bearing, not cosmetic:**

1. **Scale mismatch** — DB1 is per-repo (≤1M rows upper bound); DB2 targets 10^8 rows from PyPI + GitHub corpora. Co-mingling forces every DB1 operation through indexes sized for DB2's worst case.
2. **Write-pattern mismatch** — DB1 writes on repo-edit (interactive); DB2 writes in bulk during `nom corpus ingest`. Separating eliminates mailbox-like SQLite lock contention during ingest runs that block interactive sync.
3. **Distribution mismatch** — DB2 (100M corpus) is shared across users, distributable via `nom-corpus pack/unpack`. DB1 is per-repo private. Forcing them into one file forces the distribution unit to include either too much or too little.
4. **Backup granularity** — user may want to `git init` inside a concept-only workspace without dragging 10 GB of word corpus; separating files makes this trivial.
5. **Corruption isolation** — a corrupt `words.sqlite` (bulk ingest crash) should not require rebuilding concepts. Today it would.

---

## 3. API reshape: `nom-dict` internal surface

### 3.1 Current surface (simplified)

```rust
// In nom-dict today
pub fn open_dict(path: &Path) -> rusqlite::Result<Connection>;
pub fn insert_concept(conn: &Connection, c: &ConceptDef) -> Result<()>;
pub fn insert_word(conn: &Connection, w: &WordV2) -> Result<()>;
pub fn find_words_v2_by_kind(conn: &Connection, kind: &str) -> Result<Vec<WordV2>>;
// ...and ~40 other methods, all taking &Connection.
```

### 3.2 Target surface (Option A — directory-addressed, recommended)

```rust
// New primary type
pub struct Dict {
    pub concepts: Connection,   // points at concepts.sqlite
    pub words: Connection,      // points at words.sqlite
}

impl Dict {
    /// Open both SQLite files inside a directory. Creates them if missing.
    pub fn open_dir(dir: &Path) -> Result<Self>;
    /// Open from two explicit paths (tests, migrations).
    pub fn open_paths(concepts: &Path, words: &Path) -> Result<Self>;
}

// API methods now take &Dict or &mut Dict; individual &Connection methods become
// private helpers. Callers that previously passed &Path dict → still pass &Path dir
// (directory semantics).

pub fn insert_concept(d: &Dict, c: &ConceptDef) -> Result<()>;     // writes to d.concepts
pub fn insert_word(d: &Dict, w: &WordV2) -> Result<()>;            // writes to d.words
pub fn find_words_v2_by_kind(d: &Dict, kind: &str) -> Result<Vec<WordV2>>;
```

The `open_dict(path: &Path)` helper stays as a **compat shim**: if `path` is a file, open it as a single `concepts.sqlite`-only or raise an error with migration guidance; if a directory, call `open_dir`. Gradual migration only — new callers use `Dict::open_dir`.

### 3.3 Why Option A over B (two flags) and C (ATTACH)

| Option | Blast radius | Cross-tier queries | Backup story | Recommendation |
|---|---|---|---|---|
| **A (dir-addressed)** | CLI flags unchanged; `nom-dict` internals rewritten | Rust-side joins only | `tar -cf dict.tar ~/.nom/concepts.sqlite ~/.nom/words.sqlite` | ✅ pick this |
| B (two flags) | ~40 CLI call-sites: every `--dict <path>` → `--concepts <path> --words <path>` | Rust-side | Same as A | ❌ ecosystem breakage |
| C (ATTACH) | One `Connection` with `ATTACH DATABASE` | SQL JOIN across files works | SQLite backup API knows nothing about attach; must script manually | ❌ leaky abstraction |

Option A also mirrors Git's layout: `.git/` is a directory, not one file, and nobody objects.

---

## 4. Staged execution plan

**Each stage is one commit + tests green + push + GitNexus analyze. No stage is partially landed.**

| # | Stage | Touches | Guard | Estimate |
|---|---|---|---|---|
| **S1** | Introduce `Dict { concepts, words }` struct alongside existing API (nothing removed yet) | `nom-dict/src/lib.rs`, `nom-dict/src/dict.rs` (new) | existing tests pass | 0.5 day |
| **S2** | Add `open_dir(&Path) -> Dict` + internal schema split (two `SCHEMA_SQL` strings) + `Dict::open_paths` | `nom-dict/src/lib.rs` | new tests: round-trip concept, round-trip word, directory creation | 0.5 day |
| **S3** | Port all ~40 `nom-dict` functions to take `&Dict`; keep `&Connection` variants as `#[deprecated]` thin wrappers | `nom-dict/src/*.rs` | all nom-dict tests still pass | 1 day |
| **S4** | Update `nom-concept` callers (tier-3 ingestor) | `nom-concept/src/{lib,stages,strict,closure}.rs` | concept_demo e2e green | 0.5 day |
| **S5** | Update `nom-cli` handlers — `nom store`, `nom concept`, `nom build`, `nom corpus`, `nom author`, `nom agent` | `nom-cli/src/{store,concept,build,corpus,author,mcp,main}.rs` | cli smoke tests green | 1 day |
| **S6** | Update `nom-app`, `nom-intent`, `nom-lsp`, `nom-corpus` | 4 crates | per-crate tests green | 1 day |
| **S7** | Write `nom store migrate split` — reads old `dict.sqlite`, emits `concepts.sqlite` + `words.sqlite` in target dir, archives source | `nom-cli/src/store/migrate.rs` (new) | round-trip test: old file → split → reopen → identical row counts | 0.5 day |
| **S8** | Remove `#[deprecated]` single-Connection variants; docs 08 + 21 updates | `nom-dict/src/lib.rs`, `research/language-analysis/08-*.md`, `research/language-analysis/21-*.md` | full workspace `cargo test` clean | 0.5 day |

**Total**: ~5.5 days of staged commits.

---

## 5. CLI-flag semantics (unchanged for users)

Today:
```bash
nom store sync --dict ~/.nom/mydict.sqlite ./myrepo
```

After migration:
```bash
nom store sync --dict ~/.nom/ ./myrepo
# nom-dict resolves ~/.nom/concepts.sqlite + ~/.nom/words.sqlite under the directory
```

Default path unchanged — `~/.nom/` remains the default dict location; it's now interpreted as a directory. Users with explicit `--dict path/to/file.sqlite` get a migration-instructing error on the first run after S8. `nom store migrate split` is the one-shot fix.

---

## 6. Backward-compat + migration UX

### 6.1 Single-run migration

```bash
$ nom store sync --dict ~/.nom/dict.sqlite myrepo
error: nom-dict now addresses a directory, not a single file.
       To migrate: `nom store migrate split --source ~/.nom/dict.sqlite --dest ~/.nom/`
       After migration, use `--dict ~/.nom/` (directory).
```

### 6.2 What `nom store migrate split` does

1. Opens source `dict.sqlite` read-only.
2. Creates dest dir if missing; creates `concepts.sqlite` + `words.sqlite` with current schemas.
3. Copies `concept_defs` + concept-scoped `entry_meta` rows into `concepts.sqlite`.
4. Copies `words_v2` + word-scoped `entry_meta` rows into `words.sqlite`.
5. Verifies row counts match source totals; writes `~/.nom/dict.sqlite.archived-<timestamp>.sqlite` as a safety backup.
6. Emits a summary: `migrated N concepts, M words → ~/.nom/{concepts,words}.sqlite`.

One-shot; idempotent (detects already-migrated dir and reports).

---

## 7. Risk analysis

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| A caller leaks `&Connection` across modules, breaks on split | high | tests fail loudly | S3 audits every public API; deprecation warnings flag stragglers |
| Performance regression from two-file address resolution | low | transparent | benchmark S3+S4; SQLite file-open is cheap (~20 μs); cached after first open |
| Corrupt dest mid-migration | low | loss of un-archived data | S7 writes `.archived-<ts>.sqlite` before any mutation |
| Embeddings (M9) code path assumes one file | medium | M9 delayed | M9 hasn't shipped yet; S3 API forces it to take `&Dict` from start |
| Test fixtures hardcode single-file pattern | high | many tests red | S3 updates fixtures as part of port; S7 gates full green |
| User workflow breakage if they had `--dict path/to/file.sqlite` in scripts | low | confusing error | S5 adds the migration-instructing error message (§6.1) |

---

## 8. Non-goals (explicitly out of scope)

- **NOT** changing artifact-store (`~/.nom/store/`) — filesystem tree stays.
- **NOT** introducing a connection pool. One `Dict` per logical scope is fine; opens are cheap.
- **NOT** adding FTS5 in this migration — deferred until M9 embeddings land.
- **NOT** changing grammar.sqlite schema or location — shipped in P1, stays separate.
- **NOT** merging `entry_meta` across tiers — if a concept_meta row is needed and a word_meta row is needed, each lives in its own DB.

---

## 9. Rollback protocol

If S1–S7 land but a critical bug surfaces before S8:
1. `git revert` the S3-to-S6 callers commit-by-commit.
2. Existing users keep running with single `dict.sqlite` via the `#[deprecated]` compat shim.
3. File a blocker for the bug; redo S3+ when fixed.

After S8, rollback means reversing S8 (re-introducing the shim) and then calling `nom store migrate merge` (would need to be written; not in the forward plan).

---

## 10. Integration with doc 08 + doc 21

- **Doc 08 §2** (DB1 + DB2 schema) — S8 updates §2 to say "DB1 and DB2 are **separate physical SQLite files** inside the dict directory." Preserves the logical DB1/DB2 model.
- **Doc 08 §4** (compile pipeline) — unchanged; compile reads from `&Dict`, not raw Connection.
- **Doc 21 §7** (relation to existing crates) — update the "nom-dict — does NOT overlap" row to reflect the directory address; `grammar.sqlite` stays its own sibling file.
- **MEMORY.md** — append a one-line project memory: "Dict directory = concepts.sqlite + words.sqlite + grammar.sqlite, separate files per doc 22."

---

## 11. Verification criteria (S8 sign-off)

1. `cargo test --workspace` fully green (current baseline: ~300+ tests across the workspace).
2. `nom store sync --dict /tmp/fresh_dir ./examples/concept_demo` works end-to-end on an empty directory.
3. `nom store migrate split --source /tmp/old_dict.sqlite --dest /tmp/new_dir` round-trips a pre-migration fixture with identical row counts.
4. `nom grammar init --path /tmp/new_dir/grammar.sqlite` + `nom grammar status --path /tmp/new_dir/grammar.sqlite` shows `schema_version=1` and zero rows (unchanged from P1).
5. Full build walk: `nom build <repo>` on a migrated workspace closes through concepts → words → artifact store with identical `app_manifest_hash` to pre-migration build (byte-identical reproducibility).
6. Docs 08 + 21 + MEMORY.md reflect the new layout; no stale `single dict.sqlite` references remain.
7. A smoke-test that deletes `words.sqlite` while `concepts.sqlite` stays intact confirms isolation: concept-level commands report "word-corpus missing" cleanly instead of crashing.

---

## 12. Open questions (requires user answer before S1 starts)

1. **Directory path default** — `~/.nom/` (current default for dict file's parent) vs `~/.nom/dict/` (new subdir)? **Recommendation: `~/.nom/`** — matches current user muscle memory; new files land as siblings of the existing `store/`.
2. **Migration trigger** — auto-run migration when old file is detected? **Recommendation: no, require explicit `nom store migrate split`** — silent migrations on data are risky.
3. **Archive retention** — `dict.sqlite.archived-<ts>.sqlite` kept forever, or auto-pruned after 90 days? **Recommendation: kept forever; user can `rm` manually.**
4. **Should nom-grammar directory-address too?** For symmetry with dict? **Recommendation: no** — grammar.sqlite is one file because the registry is small, generated, and single-concern. Over-splitting reduces clarity.

---

## 13. Execution pre-conditions

Before S1 lands, the user confirms:

- [ ] This plan is the right shape (Option A, 8 stages, ~5.5 days)
- [ ] Directory default = `~/.nom/` (question 12.1)
- [ ] Migration is explicit, not automatic (question 12.2)
- [ ] Grammar stays single-file (question 12.4)

If any answer differs, plan revises before execution begins.

---

**Next step**: user sign-off (§13 checkboxes) → S1 lands on next /loop fire. Expect ~1 stage per /loop cycle, so all 8 stages complete in ~8 cycles (well inside the /loop cadence).
