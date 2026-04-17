//! S1 of the dict-split migration (doc 22).
//!
//! Introduces the target `Dict { concepts, entities }` struct ALONGSIDE the existing
//! single-file `NomDict`. No existing API is removed in S1 — this module stands up
//! the two-file surface so subsequent stages can port methods over incrementally.
//!
//! Layout (per doc 22 §1):
//! ```text
//! <dir>/
//!   concepts.sqlite    ← DB1: concept_defs + concept-scoped entry_meta
//!   entities.sqlite       ← DB2: entities    + word-scoped    entry_meta
//!   grammar.sqlite     ← registry (separate, owned by nom-grammar crate)
//!   store/<hash>/...   ← artifact bytes (filesystem, not SQLite)
//! ```
//!
//! S2 (this file) specialises each tier's schema:
//! - `concepts.sqlite` carries only DB1 tables (concept_defs + concepts +
//!   concept_members + required_axes + dict_meta).
//! - `entities.sqlite` carries only DB2 tables (entries + entities + entry_* side
//!   tables + dict_meta).
//!
//! The `dict_meta` key-value table lives on BOTH tiers so each file can track
//! its own freshness independently. Cross-file foreign keys are deliberately
//! absent per doc 22 §1 — `concept_members.entry_id` becomes a dangling hash
//! reference that the Rust layer resolves on read.
//!
//! S3 will port every function that takes `&Connection` to take `&Dict`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::Connection;

pub const CONCEPTS_FILENAME: &str = "concepts.sqlite";
pub const ENTITIES_FILENAME: &str = "entities.sqlite";

/// Schema applied to `concepts.sqlite` — DB1 tier. No cross-file FKs.
pub const CONCEPTS_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS concepts (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    describe    TEXT,
    created_at  TEXT DEFAULT (datetime('now')),
    updated_at  TEXT
);
CREATE INDEX IF NOT EXISTS idx_concepts_name ON concepts(name);

-- concept_members.entry_id is a dangling hash reference into entities.sqlite.
-- No FOREIGN KEY (cross-file FK unsupported by SQLite; Rust layer resolves).
CREATE TABLE IF NOT EXISTS concept_members (
    concept_id  TEXT NOT NULL REFERENCES concepts(id) ON DELETE CASCADE,
    entry_id    TEXT NOT NULL,
    added_at    TEXT DEFAULT (datetime('now')),
    PRIMARY KEY (concept_id, entry_id)
);
CREATE INDEX IF NOT EXISTS idx_concept_members_entry ON concept_members(entry_id);

CREATE TABLE IF NOT EXISTS concept_defs (
    name           TEXT PRIMARY KEY,
    repo_id        TEXT NOT NULL,
    intent         TEXT NOT NULL,
    index_into_db2 TEXT NOT NULL,
    exposes        TEXT NOT NULL DEFAULT '[]',
    acceptance     TEXT NOT NULL DEFAULT '[]',
    objectives     TEXT NOT NULL DEFAULT '[]',
    src_path       TEXT NOT NULL,
    src_hash       TEXT NOT NULL,
    body_hash      TEXT,
    created_at     TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at     TEXT
);
CREATE INDEX IF NOT EXISTS idx_concept_defs_repo ON concept_defs(repo_id);
CREATE INDEX IF NOT EXISTS idx_concept_defs_src ON concept_defs(src_path);

CREATE TABLE IF NOT EXISTS required_axes (
    axis          TEXT NOT NULL,
    scope         TEXT NOT NULL,
    cardinality   TEXT NOT NULL,
    repo_id       TEXT NOT NULL,
    registered_at TEXT NOT NULL,
    PRIMARY KEY (repo_id, scope, axis)
);
CREATE INDEX IF NOT EXISTS idx_required_axes_repo_scope ON required_axes(repo_id, scope);

CREATE TABLE IF NOT EXISTS dict_meta (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
"#;

/// Schema applied to `entities.sqlite` — DB2 tier. No cross-file FKs.
pub const ENTITIES_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS entries (
    id                   TEXT PRIMARY KEY,
    word                 TEXT NOT NULL,
    variant              TEXT,
    kind                 TEXT NOT NULL,
    language             TEXT NOT NULL,
    describe             TEXT,
    concept              TEXT,
    body                 TEXT,
    body_nom             TEXT,
    input_type           TEXT,
    output_type          TEXT,
    pre                  TEXT,
    post                 TEXT,
    status               TEXT NOT NULL,
    translation_score    REAL,
    is_canonical         BOOLEAN DEFAULT 1,
    deprecated_by        TEXT,
    created_at           TEXT DEFAULT (datetime('now')),
    updated_at           TEXT,
    body_kind            TEXT,
    body_bytes           BLOB
);
CREATE INDEX IF NOT EXISTS idx_entries_word ON entries(word);
CREATE INDEX IF NOT EXISTS idx_entries_word_variant ON entries(word, variant);
CREATE INDEX IF NOT EXISTS idx_entries_kind ON entries(kind);
CREATE INDEX IF NOT EXISTS idx_entries_language ON entries(language);
CREATE INDEX IF NOT EXISTS idx_entries_concept ON entries(concept);
CREATE INDEX IF NOT EXISTS idx_entries_status ON entries(status);

CREATE TABLE IF NOT EXISTS entry_scores (
    id                   TEXT PRIMARY KEY REFERENCES entries(id) ON DELETE CASCADE,
    -- Eight original quality dimensions:
    security             REAL,
    reliability          REAL,
    performance          REAL,
    readability          REAL,
    testability          REAL,
    portability          REAL,
    composability        REAL,
    maturity             REAL,
    -- T3.2 (doc 02 §5.10): three additional canonical dimensions
    -- so the score vector matches the planner's quality model. Schema
    -- only — the population pipeline lands with the corpus pilot (T4.1).
    quality              REAL,
    maintenance          REAL,
    accessibility        REAL,
    overall_score        REAL
);
CREATE INDEX IF NOT EXISTS idx_scores_overall ON entry_scores(overall_score);
CREATE INDEX IF NOT EXISTS idx_scores_security ON entry_scores(security);
CREATE INDEX IF NOT EXISTS idx_scores_quality ON entry_scores(quality);
CREATE INDEX IF NOT EXISTS idx_scores_accessibility ON entry_scores(accessibility);

CREATE TABLE IF NOT EXISTS entry_meta (
    id                   TEXT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    key                  TEXT NOT NULL,
    value                TEXT NOT NULL,
    PRIMARY KEY (id, key, value)
);
CREATE INDEX IF NOT EXISTS idx_meta_key_value ON entry_meta(key, value);

CREATE TABLE IF NOT EXISTS entry_signatures (
    id                   TEXT PRIMARY KEY REFERENCES entries(id) ON DELETE CASCADE,
    visibility           TEXT,
    is_async             BOOLEAN DEFAULT 0,
    is_method            BOOLEAN DEFAULT 0,
    return_type          TEXT,
    params_json          TEXT
);
CREATE INDEX IF NOT EXISTS idx_sigs_return ON entry_signatures(return_type);

CREATE TABLE IF NOT EXISTS entry_security_findings (
    finding_id           INTEGER PRIMARY KEY AUTOINCREMENT,
    id                   TEXT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    severity             TEXT NOT NULL,
    category             TEXT NOT NULL,
    rule_id              TEXT,
    message              TEXT,
    evidence             TEXT,
    line                 INTEGER,
    remediation          TEXT
);
CREATE INDEX IF NOT EXISTS idx_findings_entry ON entry_security_findings(id);
CREATE INDEX IF NOT EXISTS idx_findings_severity ON entry_security_findings(severity);
CREATE INDEX IF NOT EXISTS idx_findings_category ON entry_security_findings(category);

CREATE TABLE IF NOT EXISTS entry_refs (
    from_id              TEXT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    to_id                TEXT NOT NULL REFERENCES entries(id),
    PRIMARY KEY (from_id, to_id)
);
CREATE INDEX IF NOT EXISTS idx_refs_to ON entry_refs(to_id);

CREATE TABLE IF NOT EXISTS entry_graph_edges (
    edge_id              INTEGER PRIMARY KEY AUTOINCREMENT,
    from_id              TEXT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    to_id                TEXT NOT NULL REFERENCES entries(id),
    edge_type            TEXT NOT NULL,
    confidence           REAL DEFAULT 1.0
);
CREATE INDEX IF NOT EXISTS idx_edges_from ON entry_graph_edges(from_id);
CREATE INDEX IF NOT EXISTS idx_edges_to ON entry_graph_edges(to_id);
CREATE INDEX IF NOT EXISTS idx_edges_type ON entry_graph_edges(edge_type);

CREATE TABLE IF NOT EXISTS entry_translations (
    translation_id       INTEGER PRIMARY KEY AUTOINCREMENT,
    id                   TEXT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    target_language      TEXT NOT NULL,
    body                 TEXT NOT NULL,
    confidence           REAL,
    translator_version   TEXT,
    created_at           TEXT DEFAULT (datetime('now')),
    UNIQUE(id, target_language, translator_version)
);
CREATE INDEX IF NOT EXISTS idx_trans_entry ON entry_translations(id);
CREATE INDEX IF NOT EXISTS idx_trans_target ON entry_translations(target_language);

CREATE TABLE IF NOT EXISTS entities (
    hash          TEXT PRIMARY KEY,
    word          TEXT NOT NULL,
    kind          TEXT NOT NULL,
    signature     TEXT,
    contracts     TEXT,
    body_kind     TEXT,
    body_size     INTEGER,
    origin_ref    TEXT,
    bench_ids     TEXT,
    authored_in   TEXT,
    composed_of   TEXT,
    status        TEXT NOT NULL DEFAULT 'complete',
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at    TEXT
);
CREATE INDEX IF NOT EXISTS idx_entities_word ON entities(word);
CREATE INDEX IF NOT EXISTS idx_entities_kind ON entities(kind);
CREATE INDEX IF NOT EXISTS idx_entities_authored ON entities(authored_in);

CREATE TABLE IF NOT EXISTS dict_meta (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
"#;

/// The three-file dict as a single Rust-level value. Callers pass `&Dict` (or
/// `&mut Dict` for writes) instead of a raw `Connection`. Each tier gets its own
/// `Connection` backed by its own SQLite file; no cross-file FOREIGN KEYs —
/// cross-tier joins (DB1 concept → DB2 word hashes) happen in the Rust layer.
pub struct Dict {
    pub concepts: Connection,
    pub entities: Connection,
    root: PathBuf,
}

impl Dict {
    /// Open or create both SQLite files inside `dir`. Creates `dir` if missing.
    /// Applies the per-tier specialised schemas.
    pub fn open_dir(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("creating dict directory {}", dir.display()))?;
        let concepts_path = dir.join(CONCEPTS_FILENAME);
        let entities_path = dir.join(ENTITIES_FILENAME);
        Self::open_paths(&concepts_path, &entities_path).map(|mut d| {
            d.root = dir.to_path_buf();
            d
        })
    }

    /// Open or create at explicit paths (tests, migrations, split tooling).
    pub fn open_paths(concepts_path: &Path, entities_path: &Path) -> Result<Self> {
        let concepts = open_tier(concepts_path, "concepts", CONCEPTS_SCHEMA_SQL)?;
        let entities = open_tier(entities_path, "entities", ENTITIES_SCHEMA_SQL)?;
        // Migration: add status column if missing (safe on existing DBs).
        entities
            .execute_batch(
                "ALTER TABLE entities ADD COLUMN status TEXT NOT NULL DEFAULT 'complete'",
            )
            .ok();
        // Root is a best-guess parent dir; S7 migrate tool will set it explicitly.
        let root = concepts_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        Ok(Self {
            concepts,
            entities,
            root,
        })
    }

    /// Open both tiers in memory — fastest, for tests.
    pub fn open_in_memory() -> Result<Self> {
        let concepts = Connection::open_in_memory()?;
        let entities = Connection::open_in_memory()?;
        concepts.pragma_update(None, "foreign_keys", "ON")?;
        entities.pragma_update(None, "foreign_keys", "ON")?;
        concepts.execute_batch(CONCEPTS_SCHEMA_SQL)?;
        entities.execute_batch(ENTITIES_SCHEMA_SQL)?;
        Ok(Self {
            concepts,
            entities,
            root: PathBuf::new(),
        })
    }

    /// The directory that holds both files (or the current dir for in-memory /
    /// ad-hoc paths).
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the path to the equivalent nomdict.db file for compatibility
    /// with NomDict::db_path(). Returns the directory containing the split files.
    pub fn path(&self) -> PathBuf {
        self.root.clone()
    }

    /// Try to open Dict from a nomdict.db path or directory.
    /// Bridges from the legacy NomDict single-file layout to the split two-file layout.
    /// If `path` points to a file, uses its parent directory.
    /// If `path` points to a directory, uses it directly.
    /// Returns Err if Dict files don't exist at the target location.
    pub fn try_open_from_nomdict_path(path: &Path) -> Result<Self> {
        // Determine the target directory:
        // - if path is a file, use its parent
        // - if path is a directory, use it directly
        let target_dir = if path.is_dir() {
            path.to_path_buf()
        } else if let Some(parent) = path.parent() {
            parent.to_path_buf()
        } else {
            anyhow::bail!("Cannot infer directory from path: {}", path.display())
        };

        // Check that both files exist before attempting to open
        let concepts_file = target_dir.join(CONCEPTS_FILENAME);
        let entities_file = target_dir.join(ENTITIES_FILENAME);

        if !concepts_file.exists() {
            anyhow::bail!("concepts tier not found at {}", concepts_file.display());
        }
        if !entities_file.exists() {
            anyhow::bail!("entities tier not found at {}", entities_file.display());
        }

        Self::open_dir(&target_dir)
    }
}

fn open_tier(path: &Path, label: &str, schema_sql: &str) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "creating parent dir for {} tier at {}",
                    label,
                    path.display()
                )
            })?;
        }
    }
    let conn = Connection::open(path)
        .with_context(|| format!("opening {} tier at {}", label, path.display()))?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "cache_size", "-64000")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.busy_timeout(std::time::Duration::from_secs(30))?;
    conn.execute_batch(schema_sql)
        .with_context(|| format!("applying schema to {} tier", label))?;
    Ok(conn)
}

// ── S3a: entities tier API as free functions on &Dict ───────────────
//
// These mirror the NomDict methods on the same names but route to the new
// `entities.sqlite` connection. Per doc 22 §3.2 the target API is free functions
// taking &Dict; `NomDict` methods stay in place as single-file fallback until
// S8 removes them.

use crate::{
    Concept, ConceptRow, EntityRow, EntryFilter, RequiredAxis, row_to_entity, row_to_entry,
};
use nom_types::{
    Entry, EntryScores, EntrySignature, GraphEdge, SecurityFinding, Severity, Translation,
};
use rusqlite::{OptionalExtension, params};

/// Insert-or-update a row in `entities` on the entities tier.
pub fn upsert_entity(d: &Dict, row: &EntityRow) -> Result<()> {
    d.entities.execute(
        "INSERT INTO entities
             (hash, word, kind, signature, contracts, body_kind, body_size,
              origin_ref, bench_ids, authored_in, composed_of, status,
              created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, datetime('now'), NULL)
         ON CONFLICT(hash) DO UPDATE SET
             word        = excluded.word,
             kind        = excluded.kind,
             signature   = excluded.signature,
             contracts   = excluded.contracts,
             body_kind   = excluded.body_kind,
             body_size   = excluded.body_size,
             origin_ref  = excluded.origin_ref,
             bench_ids   = excluded.bench_ids,
             authored_in = excluded.authored_in,
             composed_of = excluded.composed_of,
             status      = excluded.status,
             updated_at  = datetime('now')",
        params![
            row.hash,
            row.word,
            row.kind,
            row.signature,
            row.contracts,
            row.body_kind,
            row.body_size,
            row.origin_ref,
            row.bench_ids,
            row.authored_in,
            row.composed_of,
            row.status,
        ],
    )?;
    Ok(())
}

/// Fetch one `entities` row by hash PK, or `None` if missing.
pub fn find_entity(d: &Dict, hash: &str) -> Result<Option<EntityRow>> {
    let row = d
        .entities
        .query_row(
            "SELECT hash, word, kind, signature, contracts, body_kind, body_size,
                    origin_ref, bench_ids, authored_in, composed_of, status
             FROM entities WHERE hash = ?1",
            params![hash],
            row_to_entity,
        )
        .optional()?;
    Ok(row)
}

/// Return every `entities` row with the given `word` column, ordered by hash.
pub fn find_entities_by_word(d: &Dict, word: &str) -> Result<Vec<EntityRow>> {
    let mut stmt = d.entities.prepare_cached(
        "SELECT hash, word, kind, signature, contracts, body_kind, body_size,
                origin_ref, bench_ids, authored_in, composed_of, status
         FROM entities WHERE word = ?1 ORDER BY hash",
    )?;
    let rows = stmt
        .query_map(params![word], row_to_entity)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Return every `entities` row with the given `kind`, ordered by hash
/// (§10.3.1 alphabetical-smallest tiebreak).
pub fn find_entities_by_kind(d: &Dict, kind: &str) -> Result<Vec<EntityRow>> {
    let mut stmt = d.entities.prepare_cached(
        "SELECT hash, word, kind, signature, contracts, body_kind, body_size,
                origin_ref, bench_ids, authored_in, composed_of, status
         FROM entities WHERE kind = ?1 ORDER BY hash",
    )?;
    let rows = stmt
        .query_map(params![kind], row_to_entity)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Total count of rows in `entities` on the entities tier.
pub fn count_entities(d: &Dict) -> Result<i64> {
    let n: i64 = d
        .entities
        .query_row("SELECT COUNT(*) FROM entities", [], |row| row.get(0))?;
    Ok(n)
}

/// Group `entities` by `status`, descending by count then ascending by label.
/// Returns `(status_label, count)` pairs. Queries the canonical `entities` table.
pub fn count_entities_by_status(d: &Dict) -> Result<Vec<(String, usize)>> {
    let mut stmt = d.entities.prepare(
        "SELECT status, COUNT(*) AS n
         FROM entities
         GROUP BY status
         ORDER BY n DESC, status ASC",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Hashes of entities with `status = 'partial'`, ordered by hash for
/// deterministic batch-resumption. `max = None` returns all rows;
/// `max = Some(n)` caps at `n`.
pub fn find_partial_entity_ids(d: &Dict, max: Option<usize>) -> Result<Vec<String>> {
    let sql = match max {
        Some(n) => format!(
            "SELECT hash FROM entities WHERE status = 'partial' ORDER BY hash LIMIT {}",
            n
        ),
        None => "SELECT hash FROM entities WHERE status = 'partial' ORDER BY hash".to_string(),
    };
    let mut stmt = d.entities.prepare(&sql)?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

// ── S3b: 5 more dict-API free functions ─────────────────────────────
// Per the doc 22 dict-split migration, this batch picks the next
// high-value query helpers. Each mirrors a `NomDict` method on the
// same name routed to the appropriate tier (concepts vs entities) of
// the new `Dict { concepts, entities }` shape. Legacy `NomDict`
// equivalents stay until the final dict-split slice deletes them.

/// Total count of rows in `concept_defs` on the concepts tier.
/// Mirrors `NomDict::count_concept_defs` per doc 22 §3.2.
pub fn count_concept_defs(d: &Dict) -> Result<i64> {
    let n: i64 = d
        .concepts
        .query_row("SELECT COUNT(*) FROM concept_defs", [], |row| row.get(0))?;
    Ok(n)
}

/// Total count of rows in `required_axes` on the concepts tier
/// (M7a per-scope MECE registry). Zero until at least one axis
/// is registered via `nom corpus register-axis`.
/// Mirrors `NomDict::count_required_axes` per doc 22 §3.2.
pub fn count_required_axes(d: &Dict) -> Result<i64> {
    let n: i64 = d
        .concepts
        .query_row("SELECT COUNT(*) FROM required_axes", [], |row| row.get(0))?;
    Ok(n)
}

/// Histogram of `body_kind` values across the entities tier.
/// Returns `(kind_or_untagged, count)` pairs sorted by count desc
/// then kind alpha. The NULL-bucket surfaces as `"(untagged)"` so
/// every result is a uniform `(String, usize)`. Used by
/// `nom store stats` and other diagnostic surfaces.
pub fn body_kind_histogram(d: &Dict) -> Result<Vec<(String, usize)>> {
    let mut stmt = d.entities.prepare(
        "SELECT COALESCE(body_kind, '(untagged)') AS k, COUNT(*) AS n \
         FROM entities GROUP BY body_kind ORDER BY n DESC, k ASC",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Resolve a hash prefix to a single full hash on the entities tier.
/// Returns the full hash if exactly one row matches; errors if zero
/// or multiple match. Mirrors `NomDict::resolve_prefix` per doc 22
/// §3.2 — used by every CLI subcommand that accepts a hash prefix
/// (status, info, why-this-nom, etc.).
pub fn resolve_prefix(d: &Dict, prefix: &str) -> Result<String> {
    if prefix.is_empty() {
        anyhow::bail!("resolve_prefix: empty prefix");
    }
    let pat = format!("{}%", prefix);
    let mut stmt = d
        .entities
        .prepare("SELECT hash FROM entities WHERE hash LIKE ?1 LIMIT 2")?;
    let hashes: Vec<String> = stmt
        .query_map(rusqlite::params![pat], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    match hashes.len() {
        0 => anyhow::bail!("no entity hash matches prefix `{prefix}`"),
        1 => Ok(hashes.into_iter().next().unwrap()),
        _ => anyhow::bail!("ambiguous prefix `{prefix}` — multiple entity hashes match"),
    }
}

/// Total count of rows in `dict_meta` on the entities tier — a
/// quick liveness check that the schema applied. Mirrors what
/// `NomDict` exposes implicitly through other helpers.
pub fn count_entities_meta(d: &Dict) -> Result<i64> {
    let n: i64 = d
        .entities
        .query_row("SELECT COUNT(*) FROM dict_meta", [], |row| row.get(0))?;
    Ok(n)
}

// ── S4 dict-split: 5 more read-only ports ────────────────────────────
//
// Each function is a faithful re-emit of the corresponding `NomDict::*`
// method with `&self.conn` swapped for `&d.entities`. Same SQL, same
// determinism, same result semantics. Tests below verify the wire
// behaviour matches the legacy surface so callers can switch over by
// pure rename when the dict-split migration completes.

/// Group `entities` by `status`, descending by count then ascending by
/// label for stable iteration. Queries the canonical `entities` table.
///
/// Previously queried the legacy `entries` table. Migrated to `entities`
/// so callers automatically reflect the canonical status data. Delegates
/// to [`count_entities_by_status`] internally.
pub fn status_histogram(d: &Dict) -> Result<Vec<(String, usize)>> {
    count_entities_by_status(d)
}

/// Return the raw `body_bytes` blob for an entry. `Ok(None)` for both
/// "row missing" and "row exists but body_bytes is NULL"; callers that
/// need to distinguish must combine with [`find_entity`].
///
/// Migration BLOCKED: the canonical `entities` table has no `body_bytes`
/// column (body storage design is pending). This function must remain
/// reading from `entries` until the body-storage design is resolved.
pub fn get_entry_bytes(d: &Dict, id: &str) -> Result<Option<Vec<u8>>> {
    let result: Option<Option<Vec<u8>>> = d
        .entities
        .query_row(
            "SELECT body_bytes FROM entries WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(result.flatten())
}

/// Ids of entries with `status = 'partial'`, ordered by id for
/// deterministic batch-resumption. `max = None` returns all rows;
/// `max = Some(n)` caps at `n`. Same shape as `NomDict::list_partial_ids`.
///
/// Deprecated: queries the legacy `entries` table. Migrate callers to
/// [`find_partial_entity_ids`] which reads the canonical `entities` table.
#[deprecated(since = "0.0.0", note = "use find_partial_entity_ids(d, max) for the entities tier")]
pub fn list_partial_ids(d: &Dict, max: Option<usize>) -> Result<Vec<String>> {
    let sql = match max {
        Some(n) => format!(
            "SELECT id FROM entries WHERE status = 'partial' ORDER BY id LIMIT {}",
            n
        ),
        None => "SELECT id FROM entries WHERE status = 'partial' ORDER BY id".to_string(),
    };
    let mut stmt = d.entities.prepare(&sql)?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Read every (key, value) row in `entry_meta` for an entry, ordered
/// for deterministic iteration. Mirrors `NomDict::get_meta`.
pub fn get_meta(d: &Dict, id: &str) -> Result<Vec<(String, String)>> {
    let mut stmt = d
        .entities
        .prepare_cached("SELECT key, value FROM entry_meta WHERE id = ?1 ORDER BY key, value")?;
    let rows = stmt
        .query_map(rusqlite::params![id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// List the structural refs out of an entry, ordered by target id.
/// Mirrors `NomDict::get_refs`.
pub fn get_refs(d: &Dict, id: &str) -> Result<Vec<String>> {
    let mut stmt = d
        .entities
        .prepare_cached("SELECT to_id FROM entry_refs WHERE from_id = ?1 ORDER BY to_id")?;
    let rows = stmt
        .query_map(rusqlite::params![id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

// ── S5 dict-split: closure walk + 4 concept-tier ports ───────────────
//
// S5 mixes tiers: closure walks `entry_refs` on the entities tier;
// the four concept fns operate on `concepts` rows on the concepts tier.
// All five return owned data so callers don't need to hold borrows
// across tier boundaries.

/// Breadth-first transitive closure starting at `root_id`, walking
/// `entry_refs.from_id → to_id` on the entities tier. Returns ids in
/// BFS order (root first, then siblings, then grandchildren). Cycles
/// are tolerated — visited ids are skipped. Mirrors `NomDict::closure`.
pub fn closure(d: &Dict, root_id: &str) -> Result<Vec<String>> {
    use std::collections::{HashSet, VecDeque};
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(root_id.to_string());
    while let Some(cur) = queue.pop_front() {
        if !seen.insert(cur.clone()) {
            continue;
        }
        out.push(cur.clone());
        let mut stmt = d
            .entities
            .prepare_cached("SELECT to_id FROM entry_refs WHERE from_id = ?1")?;
        let next: Vec<String> = stmt
            .query_map(rusqlite::params![cur], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for n in next {
            if !seen.contains(&n) {
                queue.push_back(n);
            }
        }
    }
    Ok(out)
}

/// Look up a concept by trimmed name, returning the (id, name, describe)
/// triple. The full `Concept` struct lives in lib.rs; this free-fn
/// returns only the columns the concepts tier actually carries (no
/// timestamps, since the split-DB concepts table omits them).
pub fn get_concept_id_by_name(d: &Dict, name: &str) -> Result<Option<String>> {
    let row = d
        .concepts
        .query_row(
            "SELECT id FROM concepts WHERE name = ?1",
            rusqlite::params![name.trim()],
            |r| r.get::<_, String>(0),
        )
        .optional()?;
    Ok(row)
}

/// Return all concept (id, name) pairs ordered alphabetically by name.
/// Mirrors `NomDict::list_concepts` shape but returns lighter rows
/// since the split-DB concepts table omits timestamps.
pub fn list_concept_ids(d: &Dict) -> Result<Vec<(String, String)>> {
    let mut stmt = d
        .concepts
        .prepare_cached("SELECT id, name FROM concepts ORDER BY name")?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Delete a concept (and cascade-delete its membership rows by FK).
/// Cross-file FK is intentionally absent — the entry rows on the
/// entities tier survive; only the grouping is removed. Mirrors
/// `NomDict::delete_concept`.
pub fn delete_concept(d: &Dict, name: &str) -> Result<()> {
    d.concepts.execute(
        "DELETE FROM concepts WHERE name = ?1",
        rusqlite::params![name.trim()],
    )?;
    Ok(())
}

/// Add one entry id to a concept. INSERT OR IGNORE so it is safe to
/// call repeatedly. Returns `true` if a row was inserted, `false` if
/// the (concept_id, entry_id) pair was already present. Mirrors
/// `NomDict::add_concept_member`. The entry_id is intentionally
/// dangling per doc 22 §1 (no cross-file FK).
pub fn add_concept_member(d: &Dict, concept_id: &str, entry_id: &str) -> Result<bool> {
    let changed = d.concepts.execute(
        "INSERT OR IGNORE INTO concept_members (concept_id, entry_id) VALUES (?1, ?2)",
        rusqlite::params![concept_id, entry_id],
    )?;
    Ok(changed == 1)
}

/// Insert or replace a `concept_defs` row on the concepts tier.
/// Mirrors `NomDict::upsert_concept_def`.
pub fn upsert_concept_def(d: &Dict, row: &ConceptRow) -> Result<()> {
    d.concepts.execute(
        "INSERT INTO concept_defs
             (name, repo_id, intent, index_into_db2, exposes, acceptance,
              objectives, src_path, src_hash, body_hash, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, datetime('now'), NULL)
         ON CONFLICT(name) DO UPDATE SET
             repo_id        = excluded.repo_id,
             intent         = excluded.intent,
             index_into_db2 = excluded.index_into_db2,
             exposes        = excluded.exposes,
             acceptance     = excluded.acceptance,
             objectives     = excluded.objectives,
             src_path       = excluded.src_path,
             src_hash       = excluded.src_hash,
             body_hash      = excluded.body_hash,
             updated_at     = datetime('now')",
        params![
            row.name,
            row.repo_id,
            row.intent,
            row.index_into_db2,
            row.exposes,
            row.acceptance,
            row.objectives,
            row.src_path,
            row.src_hash,
            row.body_hash,
        ],
    )?;
    Ok(())
}

/// Fetch one `concept_defs` row by its primary key (`name`).
/// Mirrors `NomDict::find_concept_def`.
pub fn find_concept_def(d: &Dict, name: &str) -> Result<Option<ConceptRow>> {
    let row = d
        .concepts
        .query_row(
            "SELECT name, repo_id, intent, index_into_db2, exposes, acceptance,
                    objectives, src_path, src_hash, body_hash
             FROM concept_defs WHERE name = ?1",
            rusqlite::params![name],
            |row| {
                Ok(ConceptRow {
                    name: row.get(0)?,
                    repo_id: row.get(1)?,
                    intent: row.get(2)?,
                    index_into_db2: row.get(3)?,
                    exposes: row.get(4)?,
                    acceptance: row.get(5)?,
                    objectives: row.get(6)?,
                    src_path: row.get(7)?,
                    src_hash: row.get(8)?,
                    body_hash: row.get(9)?,
                })
            },
        )
        .optional()?;
    Ok(row)
}

/// Return all `concept_defs` rows for a given `repo_id`, ordered by name.
/// Mirrors `NomDict::list_concept_defs_in_repo`.
pub fn list_concept_defs_in_repo(d: &Dict, repo_id: &str) -> Result<Vec<ConceptRow>> {
    let mut stmt = d.concepts.prepare_cached(
        "SELECT name, repo_id, intent, index_into_db2, exposes, acceptance,
                objectives, src_path, src_hash, body_hash
         FROM concept_defs WHERE repo_id = ?1 ORDER BY name",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![repo_id], |row| {
            Ok(ConceptRow {
                name: row.get(0)?,
                repo_id: row.get(1)?,
                intent: row.get(2)?,
                index_into_db2: row.get(3)?,
                exposes: row.get(4)?,
                acceptance: row.get(5)?,
                objectives: row.get(6)?,
                src_path: row.get(7)?,
                src_hash: row.get(8)?,
                body_hash: row.get(9)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Register or replace a required axis row on the concepts tier.
/// Mirrors `NomDict::register_required_axis`.
pub fn register_required_axis(
    d: &Dict,
    repo_id: &str,
    scope: &str,
    axis: &str,
    cardinality: &str,
) -> rusqlite::Result<()> {
    if !matches!(scope, "app" | "concept" | "module") {
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "unknown scope '{scope}': must be app, concept, or module"
        )));
    }
    if !matches!(cardinality, "at_least_one" | "exactly_one") {
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "unknown cardinality '{cardinality}': must be at_least_one or exactly_one"
        )));
    }
    let axis_norm = axis.trim().to_ascii_lowercase();
    let registered_at = format!(
        "epoch-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    );
    d.concepts.execute(
        "INSERT INTO required_axes (axis, scope, cardinality, repo_id, registered_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(repo_id, scope, axis) DO UPDATE SET
             cardinality   = excluded.cardinality,
             registered_at = excluded.registered_at",
        rusqlite::params![axis_norm, scope, cardinality, repo_id, registered_at],
    )?;
    Ok(())
}

/// Return all `required_axes` rows for a given `repo_id` + `scope`,
/// ordered by axis for deterministic iteration.
/// Mirrors `NomDict::list_required_axes`.
pub fn list_required_axes(
    d: &Dict,
    repo_id: &str,
    scope: &str,
) -> rusqlite::Result<Vec<RequiredAxis>> {
    let mut stmt = d.concepts.prepare_cached(
        "SELECT repo_id, scope, axis, cardinality, registered_at
         FROM required_axes
         WHERE repo_id = ?1 AND scope = ?2
         ORDER BY axis",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![repo_id, scope], |row| {
            Ok(RequiredAxis {
                repo_id: row.get(0)?,
                scope: row.get(1)?,
                axis: row.get(2)?,
                cardinality: row.get(3)?,
                registered_at: row.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

// ── S6 dict-split: 5 entries-tier mutators ────────────────────────────
//
// Faithful re-emit of the legacy `NomDict::*` setter SQL with
// `&self.conn` swapped for `&d.entities`. Same insert-or-update
// semantics, same idempotency. The `EntryScores` impl stays on the
// legacy 8 columns; the three T3.2 dimensions (quality, maintenance,
// accessibility) are populated by a different code path (the corpus
// pilot) and remain NULL through this setter.

/// Insert or replace a concept row on the concepts tier.
/// Mirrors `NomDict::upsert_concept`.
pub fn upsert_concept(d: &Dict, concept: &Concept) -> Result<()> {
    d.concepts.execute(
        "INSERT INTO concepts (id, name, describe, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET
             name       = excluded.name,
             describe   = COALESCE(excluded.describe, concepts.describe),
             updated_at = datetime('now')",
        params![
            concept.id,
            concept.name,
            concept.describe,
            concept.created_at,
            concept.updated_at,
        ],
    )?;
    Ok(())
}

/// Look up a concept by its human-readable name.
/// Mirrors `NomDict::get_concept_by_name`.
pub fn get_concept_by_name(d: &Dict, name: &str) -> Result<Option<Concept>> {
    let row = d
        .concepts
        .query_row(
            "SELECT id, name, describe, created_at, updated_at
             FROM concepts WHERE name = ?1",
            params![name.trim()],
            |row| {
                Ok(Concept {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    describe: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            },
        )
        .optional()?;
    Ok(row)
}

/// Return all concepts ordered alphabetically by name.
/// Mirrors `NomDict::list_concepts`.
pub fn list_concepts(d: &Dict) -> Result<Vec<Concept>> {
    let mut stmt = d.concepts.prepare_cached(
        "SELECT id, name, describe, created_at, updated_at FROM concepts ORDER BY name",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Concept {
                id: row.get(0)?,
                name: row.get(1)?,
                describe: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Remove one entry from a concept (no-op if it was not a member).
/// Mirrors `NomDict::remove_concept_member`.
pub fn remove_concept_member(d: &Dict, concept_id: &str, entry_id: &str) -> Result<()> {
    d.concepts.execute(
        "DELETE FROM concept_members WHERE concept_id = ?1 AND entry_id = ?2",
        params![concept_id, entry_id],
    )?;
    Ok(())
}

/// Fetch all entries belonging to a concept, ordered by entry id.
/// Mirrors `NomDict::get_concept_members`.
///
/// Migration BLOCKED: `concept_members` stores `entry_id` which is an
/// `entries.id` (SHA-256 of AST). The `entities` table uses `hash` as PK
/// but has no concept membership column. Full migration requires either
/// adding entity hashes to `concept_members` or a join-capable bridge.
pub fn get_concept_members(d: &Dict, concept_id: &str) -> Result<Vec<Entry>> {
    let mut member_stmt = d.concepts.prepare_cached(
        "SELECT entry_id FROM concept_members WHERE concept_id = ?1 ORDER BY entry_id",
    )?;
    let entry_ids = member_stmt
        .query_map(params![concept_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut rows = Vec::with_capacity(entry_ids.len());
    let mut entry_stmt = d.entities.prepare_cached(
        "SELECT id, word, variant, kind, language, describe, concept,
                body, body_nom, input_type, output_type, pre, post,
                status, translation_score, is_canonical, deprecated_by,
                created_at, updated_at, body_kind, body_bytes
         FROM entries
         WHERE id = ?1",
    )?;
    for entry_id in entry_ids {
        if let Some(entry) = entry_stmt
            .query_row(params![entry_id], row_to_entry)
            .optional()?
        {
            rows.push(entry);
        }
    }
    Ok(rows)
}

/// Return the count of members in a concept.
/// Mirrors `NomDict::count_concept_members`.
pub fn count_concept_members(d: &Dict, concept_id: &str) -> Result<usize> {
    let n: i64 = d.concepts.query_row(
        "SELECT COUNT(*) FROM concept_members WHERE concept_id = ?1",
        params![concept_id],
        |row| row.get(0),
    )?;
    Ok(n as usize)
}

/// Delete a `required_axes` row and report whether anything changed.
/// Mirrors `NomDict::unregister_required_axis`.
pub fn unregister_required_axis(
    d: &Dict,
    repo_id: &str,
    scope: &str,
    axis: &str,
) -> rusqlite::Result<bool> {
    let axis_norm = axis.trim().to_ascii_lowercase();
    let n = d.concepts.execute(
        "DELETE FROM required_axes WHERE repo_id = ?1 AND scope = ?2 AND axis = ?3",
        params![repo_id, scope, axis_norm],
    )?;
    Ok(n > 0)
}

/// Seed the canonical app-scope axis set.
/// Mirrors `NomDict::seed_standard_axes`.
pub fn seed_standard_axes(
    d: &Dict,
    repo_id: &str,
) -> rusqlite::Result<Vec<(String, String, String)>> {
    const STANDARD: &[(&str, &str, &str)] = &[
        ("app", "correctness", "at_least_one"),
        ("app", "safety", "at_least_one"),
        ("app", "performance", "at_least_one"),
        ("app", "dependency", "at_least_one"),
        ("app", "documentation", "at_least_one"),
    ];
    let mut seeded = Vec::with_capacity(STANDARD.len());
    for (scope, axis, cardinality) in STANDARD {
        register_required_axis(d, repo_id, scope, axis, cardinality)?;
        seeded.push((scope.to_string(), axis.to_string(), cardinality.to_string()));
    }
    Ok(seeded)
}

/// Bulk-add every entry matching `filter` to the concept identified by `concept_id`.
/// Mirrors `NomDict::add_concept_members_by_filter`.
///
/// Migration BLOCKED: reads candidate rows from the legacy `entries` table
/// and inserts `entry_id` values into `concept_members`. The `entities`
/// table has no concept-membership link; this function stays on `entries`
/// until `concept_members` is extended to reference entity hashes.
pub fn add_concept_members_by_filter(
    d: &Dict,
    concept_id: &str,
    filter: &EntryFilter,
) -> Result<usize> {
    let mut sql = String::from(
        "SELECT id, word, variant, kind, language, describe, concept, body, body_nom,
                input_type, output_type, pre, post, status, translation_score,
                is_canonical, deprecated_by, created_at, updated_at, body_kind, body_bytes
         FROM entries WHERE 1=1",
    );
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(k) = &filter.body_kind {
        sql.push_str(" AND body_kind = ?");
        params_vec.push(Box::new(k.clone()) as Box<dyn rusqlite::ToSql>);
    }
    if let Some(l) = &filter.language {
        sql.push_str(" AND language = ?");
        params_vec.push(Box::new(l.clone()) as Box<dyn rusqlite::ToSql>);
    }
    if let Some(s) = filter.status {
        sql.push_str(" AND status = ?");
        params_vec.push(Box::new(s.as_str().to_string()) as Box<dyn rusqlite::ToSql>);
    }
    if let Some(k) = filter.kind {
        sql.push_str(" AND kind = ?");
        params_vec.push(Box::new(k.as_str().to_string()) as Box<dyn rusqlite::ToSql>);
    }
    sql.push_str(&format!(" ORDER BY id LIMIT {}", filter.limit.max(1)));
    let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
    let mut stmt = d.entities.prepare(&sql)?;
    let entries = stmt
        .query_map(rusqlite::params_from_iter(param_refs.iter()), row_to_entry)?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let tx = d.concepts.unchecked_transaction()?;
    let mut added = 0usize;
    {
        let mut insert_stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO concept_members (concept_id, entry_id) VALUES (?1, ?2)",
        )?;
        for entry in &entries {
            let changed = insert_stmt.execute(params![concept_id, entry.id])?;
            if changed == 1 {
                added += 1;
            }
        }
    }
    tx.commit()?;
    Ok(added)
}

/// Insert or replace a concept row on the concepts tier.
/// Mirrors `NomDict::upsert_concept`.
/// Insert or replace an entry's score row. Updates the legacy 8
/// dimensions; the T3.2-extended `quality`, `maintenance`,
/// `accessibility` columns are left untouched (NULL or whatever the
/// corpus pipeline put there).
pub fn set_scores(d: &Dict, id: &str, scores: &EntryScores) -> Result<()> {
    d.entities.execute(
        "INSERT INTO entry_scores (id, security, reliability, performance, readability,
                                   testability, portability, composability, maturity,
                                   overall_score)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(id) DO UPDATE SET
             security      = excluded.security,
             reliability   = excluded.reliability,
             performance   = excluded.performance,
             readability   = excluded.readability,
             testability   = excluded.testability,
             portability   = excluded.portability,
             composability = excluded.composability,
             maturity      = excluded.maturity,
             overall_score = excluded.overall_score",
        params![
            id,
            scores.security,
            scores.reliability,
            scores.performance,
            scores.readability,
            scores.testability,
            scores.portability,
            scores.composability,
            scores.maturity,
            scores.overall_score,
        ],
    )?;
    Ok(())
}

/// Add a (key, value) metadata row. The PK is (id, key, value), so
/// the same key can carry many values (e.g. multiple `tag` entries
/// for the same id). Idempotent per the PK.
pub fn add_meta(d: &Dict, id: &str, key: &str, value: &str) -> Result<()> {
    d.entities.execute(
        "INSERT OR IGNORE INTO entry_meta (id, key, value) VALUES (?1, ?2, ?3)",
        params![id, key, value],
    )?;
    Ok(())
}

/// Insert or replace the signature row for an entry.
pub fn set_signature(d: &Dict, id: &str, sig: &EntrySignature) -> Result<()> {
    d.entities.execute(
        "INSERT INTO entry_signatures
            (id, visibility, is_async, is_method, return_type, params_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET
             visibility  = excluded.visibility,
             is_async    = excluded.is_async,
             is_method   = excluded.is_method,
             return_type = excluded.return_type,
             params_json = excluded.params_json",
        params![
            id,
            sig.visibility,
            sig.is_async,
            sig.is_method,
            sig.return_type,
            sig.params_json,
        ],
    )?;
    Ok(())
}

/// Append a security finding row. Many findings per entry are allowed
/// (PK is `finding_id` AUTOINCREMENT, not the entry id).
pub fn add_finding(d: &Dict, id: &str, finding: &SecurityFinding) -> Result<()> {
    d.entities.execute(
        "INSERT INTO entry_security_findings
             (id, severity, category, rule_id, message, evidence, line, remediation)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            id,
            finding.severity.as_str(),
            finding.category,
            finding.rule_id,
            finding.message,
            finding.evidence,
            finding.line,
            finding.remediation,
        ],
    )?;
    Ok(())
}

/// Idempotent structural-closure-ref insert. PK enforces uniqueness.
pub fn add_ref(d: &Dict, from_id: &str, to_id: &str) -> Result<()> {
    d.entities.execute(
        "INSERT OR IGNORE INTO entry_refs (from_id, to_id) VALUES (?1, ?2)",
        params![from_id, to_id],
    )?;
    Ok(())
}

// ── S7 dict-split: 5 Entry/Score/Finding-returning readers ───────────
//
// Lift the heavier readers off `NomDict`. All five return owned data
// re-using the existing `nom_types::*` shapes (no new types added);
// the SELECT lists are byte-identical with the legacy version so
// callers swap by pure rename.

// LEGACY: blocked on body storage design — `entries` is the only table
// that holds `describe` and `body_bytes`; these readers stay here until
// those columns are promoted to the `entities` tier.
const ENTRY_SELECT: &str = "SELECT id, word, variant, kind, language, describe, concept, body, body_nom, \
     input_type, output_type, pre, post, status, translation_score, \
     is_canonical, deprecated_by, created_at, updated_at, body_kind, body_bytes \
     FROM entries";

/// Look up every entry whose `word` column equals `word`, ordered by
/// id. Empty vec when nothing matches. Mirrors `NomDict::find_by_word`.
///
/// Deprecated: queries the legacy `entries` table. Migrate callers to
/// [`find_entities_by_word`] which reads the canonical `entities` table.
#[deprecated(since = "0.0.0", note = "use find_entities_by_word(d, word) for the entities tier")]
pub fn find_by_word(d: &Dict, word: &str) -> Result<Vec<Entry>> {
    let sql = format!("{} WHERE word = ?1 ORDER BY id", ENTRY_SELECT);
    let mut stmt = d.entities.prepare_cached(&sql)?;
    let rows = stmt
        .query_map(params![word], row_to_entry)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Entries whose `body_kind` equals `kind`, ordered by id, capped by
/// `limit`. Mirrors `NomDict::find_by_body_kind`.
///
/// Deprecated: queries the legacy `entries` table. Migrate callers to
/// [`find_entities_by_body_kind`] which reads the canonical `entities` table.
#[deprecated(since = "0.0.0", note = "use find_entities_by_body_kind(d, kind, limit) for the entities tier")]
pub fn find_by_body_kind(d: &Dict, kind: &str, limit: usize) -> Result<Vec<Entry>> {
    let sql = format!("{} WHERE body_kind = ?1 ORDER BY id LIMIT ?2", ENTRY_SELECT);
    let mut stmt = d.entities.prepare_cached(&sql)?;
    let rows = stmt
        .query_map(params![kind, limit as i64], row_to_entry)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Entities whose `body_kind` equals `kind`, ordered by hash, capped by
/// `limit`. Reads the canonical `entities` table.
pub fn find_entities_by_body_kind(d: &Dict, kind: &str, limit: usize) -> Result<Vec<EntityRow>> {
    let mut stmt = d.entities.prepare_cached(
        "SELECT hash, word, kind, signature, contracts, body_kind, body_size,
                origin_ref, bench_ids, authored_in, composed_of, status
         FROM entities WHERE body_kind = ?1 ORDER BY hash LIMIT ?2",
    )?;
    let rows = stmt
        .query_map(params![kind, limit as i64], row_to_entity)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Case-insensitive substring search on `entries.describe`. Used by
/// the LLM `search_nomtu` tool path — caller wraps the query in
/// `%query%`-style wildcards via the LIKE pattern. Mirrors
/// `NomDict::search_describe`.
///
/// Migration BLOCKED: the canonical `entities` table has no `describe`
/// column (human-readable description lives in `entries` only). This
/// function must remain reading from `entries` until a description column
/// is added to `entities`.
pub fn search_describe(d: &Dict, query: &str, limit: usize) -> Result<Vec<Entry>> {
    let pattern = format!("%{}%", query);
    let sql = format!(
        "{} WHERE describe LIKE ?1 COLLATE NOCASE ORDER BY id LIMIT ?2",
        ENTRY_SELECT
    );
    let mut stmt = d.entities.prepare_cached(&sql)?;
    let rows = stmt
        .query_map(params![pattern, limit as i64], row_to_entry)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Fetch the optional scores row for an entry. Reads the legacy 8
/// dimensions; the T3.2-extended (quality, maintenance, accessibility)
/// columns are read by a separate path.
pub fn get_scores(d: &Dict, id: &str) -> Result<Option<EntryScores>> {
    let row = d
        .entities
        .query_row(
            "SELECT id, security, reliability, performance, readability,
                    testability, portability, composability, maturity, overall_score
             FROM entry_scores WHERE id = ?1",
            params![id],
            |row| {
                Ok(EntryScores {
                    id: row.get(0)?,
                    security: row.get(1)?,
                    reliability: row.get(2)?,
                    performance: row.get(3)?,
                    readability: row.get(4)?,
                    testability: row.get(5)?,
                    portability: row.get(6)?,
                    composability: row.get(7)?,
                    maturity: row.get(8)?,
                    overall_score: row.get(9)?,
                })
            },
        )
        .optional()?;
    Ok(row)
}

/// Fetch every security finding for an entry, ordered by `finding_id`
/// for stable iteration. Mirrors `NomDict::get_findings`.
pub fn get_findings(d: &Dict, id: &str) -> Result<Vec<SecurityFinding>> {
    let mut stmt = d.entities.prepare_cached(
        "SELECT finding_id, id, severity, category, rule_id, message, evidence, line, remediation \
         FROM entry_security_findings WHERE id = ?1 ORDER BY finding_id",
    )?;
    let rows = stmt
        .query_map(params![id], |row| {
            Ok(SecurityFinding {
                finding_id: row.get(0)?,
                id: row.get(1)?,
                severity: Severity::from_str(&row.get::<_, String>(2)?),
                category: row.get(3)?,
                rule_id: row.get(4)?,
                message: row.get(5)?,
                evidence: row.get(6)?,
                line: row.get(7)?,
                remediation: row.get(8)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Add a graph edge between two entries. Mirrors `NomDict::add_graph_edge`.
pub fn add_graph_edge(d: &Dict, edge: &GraphEdge) -> Result<()> {
    d.entities.execute(
        "INSERT INTO entry_graph_edges (from_id, to_id, edge_type, confidence)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            edge.from_id,
            edge.to_id,
            edge.edge_type.as_str(),
            edge.confidence,
        ],
    )?;
    Ok(())
}

/// Append a translation (unique per (id, target_language, translator_version)).
/// Mirrors `NomDict::add_translation`.
pub fn add_translation(d: &Dict, t: &Translation) -> Result<()> {
    d.entities.execute(
        "INSERT OR IGNORE INTO entry_translations
             (id, target_language, body, confidence, translator_version, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            t.id,
            t.target_language,
            t.body,
            t.confidence,
            t.translator_version,
            t.created_at,
        ],
    )?;
    Ok(())
}

/// Bulk-set quality scores for entries in a transaction.
/// Mirrors `NomDict::bulk_set_scores`.
pub fn bulk_set_scores(d: &Dict, scores: &[EntryScores]) -> Result<()> {
    let tx = d.entities.unchecked_transaction()?;
    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR REPLACE INTO entry_scores
                 (id, security, reliability, performance, readability, testability,
                  portability, composability, maturity, overall_score)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )?;
        for s in scores {
            stmt.execute(params![
                s.id,
                s.security,
                s.reliability,
                s.performance,
                s.readability,
                s.testability,
                s.portability,
                s.composability,
                s.maturity,
                s.overall_score,
            ])?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// Fetch a single entry by id. Mirrors `NomDict::get_entry`.
///
/// Deprecated: queries the legacy `entries` table. For entities-tier
/// lookups use [`find_entity`] which reads the canonical `entities` table
/// by hash primary key.
#[deprecated(since = "0.0.0", note = "use find_entity(d, hash) for the entities tier")]
pub fn get_entry(d: &Dict, id: &str) -> Result<Option<Entry>> {
    let row = d
        .entities
        .query_row(
            "SELECT id, word, variant, kind, language, describe, concept, body, body_nom,
                    input_type, output_type, pre, post, status, translation_score,
                    is_canonical, deprecated_by, created_at, updated_at, body_kind,
                    body_bytes
             FROM entries WHERE id = ?1",
            params![id],
            row_to_entry,
        )
        .optional()?;
    Ok(row)
}

/// Bulk INSERT OR IGNORE entries in a transaction.
/// Returns the count of rows actually inserted (duplicates ignored).
/// Mirrors `NomDict::bulk_upsert`.
///
/// LEGACY: blocked on body storage design — writes to `entries` until
/// `body_bytes` has a home on the `entities` tier.
pub fn bulk_upsert(d: &Dict, entries: &[Entry]) -> Result<usize> {
    let tx = d.entities.unchecked_transaction()?;
    let mut inserted = 0;
    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO entries (id, word, variant, kind, language, describe,
                                            concept, body, body_nom, input_type, output_type,
                                            pre, post, status, translation_score, is_canonical,
                                            deprecated_by, created_at, updated_at, body_kind,
                                            body_bytes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
        )?;
        for e in entries {
            let rows = stmt.execute(params![
                e.id,
                e.word,
                e.variant,
                e.kind.as_str(),
                e.language,
                e.describe,
                e.concept,
                e.body,
                e.body_nom,
                e.contract.input_type,
                e.contract.output_type,
                e.contract.pre,
                e.contract.post,
                e.status.as_str(),
                e.translation_score,
                e.is_canonical,
                e.deprecated_by,
                e.created_at,
                e.updated_at,
                e.body_kind,
                e.body_bytes,
            ])?;
            if rows > 0 {
                inserted += 1;
            }
        }
    }
    tx.commit()?;
    Ok(inserted)
}

/// Insert-or-update an entry with selective COALESCE-based field merging.
/// On conflict (same id): replaces unconditional fields (word, variant, kind, etc.),
/// but only updates optional fields (describe, concept, body, etc.) if the new
/// value is NOT NULL (using COALESCE to preserve existing values if new is NULL).
/// Mirrors `NomDict::upsert_entry`.
///
/// LEGACY: blocked on body storage design — writes to `entries` until
/// `body_bytes` and `describe` have homes on the `entities` tier.
pub fn upsert_entry(d: &Dict, entry: &Entry) -> Result<String> {
    d.entities.execute(
        "INSERT INTO entries (id, word, variant, kind, language, describe, concept,
                              body, body_nom, input_type, output_type, pre, post,
                              status, translation_score, is_canonical, deprecated_by,
                              created_at, updated_at, body_kind, body_bytes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)
         ON CONFLICT(id) DO UPDATE SET
             word              = excluded.word,
             variant           = excluded.variant,
             kind              = excluded.kind,
             language          = excluded.language,
             describe          = COALESCE(excluded.describe, entries.describe),
             concept           = COALESCE(excluded.concept, entries.concept),
             body              = COALESCE(excluded.body, entries.body),
             body_nom          = COALESCE(excluded.body_nom, entries.body_nom),
             body_kind         = COALESCE(excluded.body_kind, entries.body_kind),
             body_bytes        = COALESCE(excluded.body_bytes, entries.body_bytes),
             status            = excluded.status,
             translation_score = COALESCE(excluded.translation_score, entries.translation_score),
             is_canonical      = excluded.is_canonical,
             deprecated_by     = COALESCE(excluded.deprecated_by, entries.deprecated_by),
             updated_at        = datetime('now')",
        params![
            entry.id,
            entry.word,
            entry.variant,
            entry.kind.as_str(),
            entry.language,
            entry.describe,
            entry.concept,
            entry.body,
            entry.body_nom,
            entry.contract.input_type,
            entry.contract.output_type,
            entry.contract.pre,
            entry.contract.post,
            entry.status.as_str(),
            entry.translation_score,
            entry.is_canonical,
            entry.deprecated_by,
            entry.created_at,
            entry.updated_at,
            entry.body_kind,
            entry.body_bytes,
        ],
    )?;
    Ok(entry.id.clone())
}

/// Try-insert an entry without a prior existence check (corpus-ingest path).
/// Returns `true` if the row was newly inserted, `false` if the id existed
/// and the INSERT was skipped (no-op).
/// Unlike `upsert_entry`, this does NOT replace on conflict — the existing
/// row is preserved. Designed for corpus deduplication without SELECT overhead.
/// Mirrors `NomDict::upsert_entry_if_new`.
///
/// LEGACY: blocked on body storage design — writes to `entries` until
/// `body_bytes` and `describe` have homes on the `entities` tier.
pub fn upsert_entry_if_new(d: &Dict, entry: &Entry) -> Result<bool> {
    let changed = d.entities.execute(
        "INSERT OR IGNORE INTO entries (id, word, variant, kind, language, describe, concept,
                                        body, body_nom, input_type, output_type, pre, post,
                                        status, translation_score, is_canonical, deprecated_by,
                                        created_at, updated_at, body_kind, body_bytes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
        params![
            entry.id,
            entry.word,
            entry.variant,
            entry.kind.as_str(),
            entry.language,
            entry.describe,
            entry.concept,
            entry.body,
            entry.body_nom,
            entry.contract.input_type,
            entry.contract.output_type,
            entry.contract.pre,
            entry.contract.post,
            entry.status.as_str(),
            entry.translation_score,
            entry.is_canonical,
            entry.deprecated_by,
            entry.created_at,
            entry.updated_at,
            entry.body_kind,
            entry.body_bytes,
        ],
    )?;
    Ok(changed == 1)
}

/// Unified filter query for entries: each present field in the filter
/// adds an AND clause. Results ordered by id for determinism.
/// An empty filter returns the first `limit` entries (default 50).
/// Mirrors `NomDict::find_entries`.
///
/// Deprecated: queries the legacy `entries` table. Migrate callers to
/// [`find_entities`] which reads the canonical `entities` table.
/// Note: `EntryFilter.language` is ignored by `find_entities` since
/// the `entities` table has no `language` column.
#[deprecated(since = "0.0.0", note = "use find_entities(d, f) for the entities tier")]
pub fn find_entries(d: &Dict, f: &EntryFilter) -> Result<Vec<Entry>> {
    let mut sql = String::from(
        "SELECT id, word, variant, kind, language, describe, concept, body, body_nom,
                input_type, output_type, pre, post, status, translation_score,
                is_canonical, deprecated_by, created_at, updated_at, body_kind, body_bytes
         FROM entries WHERE 1=1",
    );
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(k) = &f.body_kind {
        sql.push_str(" AND body_kind = ?");
        params_vec.push(Box::new(k.clone()));
    }
    if let Some(l) = &f.language {
        sql.push_str(" AND language = ?");
        params_vec.push(Box::new(l.clone()));
    }
    if let Some(s) = f.status {
        sql.push_str(" AND status = ?");
        params_vec.push(Box::new(s.as_str().to_string()));
    }
    if let Some(k) = f.kind {
        sql.push_str(" AND kind = ?");
        params_vec.push(Box::new(k.as_str().to_string()));
    }
    sql.push_str(&format!(" ORDER BY id LIMIT {}", f.limit.max(1)));
    let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
    let mut stmt = d.entities.prepare(&sql)?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(param_refs.iter()), row_to_entry)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Unified filter query on the canonical `entities` table: each present field
/// in the filter adds an AND clause. Results ordered by `hash` for determinism.
/// An empty filter returns the first `limit` rows (default 50).
///
/// This is the entities-tier equivalent of the deprecated [`find_entries`].
/// Note: `EntryFilter.language` is silently ignored — the `entities` table
/// has no `language` column. Use [`find_entities_by_word`] or
/// [`find_entities_by_kind`] when narrowing by word or kind alone.
pub fn find_entities(d: &Dict, f: &EntryFilter) -> Result<Vec<EntityRow>> {
    let mut sql = String::from(
        "SELECT hash, word, kind, signature, contracts, body_kind, body_size,
                origin_ref, bench_ids, authored_in, composed_of, status
         FROM entities WHERE 1=1",
    );
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    // `language` is not present on `entities`; skip without error.
    if let Some(k) = &f.body_kind {
        sql.push_str(" AND body_kind = ?");
        params_vec.push(Box::new(k.clone()));
    }
    if let Some(s) = f.status {
        sql.push_str(" AND status = ?");
        params_vec.push(Box::new(s.as_str().to_string()));
    }
    if let Some(k) = f.kind {
        sql.push_str(" AND kind = ?");
        params_vec.push(Box::new(k.as_str().to_string()));
    }
    sql.push_str(&format!(" ORDER BY hash LIMIT {}", f.limit.max(1)));
    let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
    let mut stmt = d.entities.prepare(&sql)?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(param_refs.iter()), row_to_entity)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn open_dir_creates_both_sqlite_files() {
        let dir = tempdir().unwrap();
        let _ = Dict::open_dir(dir.path()).unwrap();
        assert!(dir.path().join(CONCEPTS_FILENAME).exists());
        assert!(dir.path().join(ENTITIES_FILENAME).exists());
    }

    #[test]
    fn open_dir_is_idempotent() {
        let dir = tempdir().unwrap();
        let _ = Dict::open_dir(dir.path()).unwrap();
        let _ = Dict::open_dir(dir.path()).unwrap();
        assert!(dir.path().join(CONCEPTS_FILENAME).exists());
        assert!(dir.path().join(ENTITIES_FILENAME).exists());
    }

    #[test]
    fn open_paths_accepts_arbitrary_layout() {
        let dir = tempdir().unwrap();
        let c = dir.path().join("custom_concepts.db");
        let w = dir.path().join("sub").join("custom_words.db");
        let _ = Dict::open_paths(&c, &w).unwrap();
        assert!(c.exists());
        assert!(w.exists());
    }

    #[test]
    fn open_in_memory_has_both_connections() {
        let d = Dict::open_in_memory().unwrap();
        // Both connections respond to a trivial query — confirms schema applied.
        let a: i64 = d.concepts.query_row("SELECT 1", [], |r| r.get(0)).unwrap();
        let b: i64 = d.entities.query_row("SELECT 1", [], |r| r.get(0)).unwrap();
        assert_eq!(a, 1);
        assert_eq!(b, 1);
    }

    fn table_exists(conn: &Connection, name: &str) -> bool {
        conn.query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [name],
            |_| Ok(true),
        )
        .unwrap_or(false)
    }

    #[test]
    fn concepts_tier_has_db1_tables_only() {
        let d = Dict::open_in_memory().unwrap();
        // DB1 tables present on concepts tier.
        assert!(table_exists(&d.concepts, "concepts"));
        assert!(table_exists(&d.concepts, "concept_members"));
        assert!(table_exists(&d.concepts, "concept_defs"));
        assert!(table_exists(&d.concepts, "required_axes"));
        assert!(table_exists(&d.concepts, "dict_meta"));
        // DB2 tables absent on concepts tier.
        assert!(!table_exists(&d.concepts, "entries"));
        assert!(!table_exists(&d.concepts, "entities"));
        assert!(!table_exists(&d.concepts, "entry_meta"));
        assert!(!table_exists(&d.concepts, "entry_scores"));
    }

    #[test]
    fn words_tier_has_db2_tables_only() {
        let d = Dict::open_in_memory().unwrap();
        // DB2 tables present on entities tier.
        assert!(table_exists(&d.entities, "entries"));
        assert!(table_exists(&d.entities, "entities"));
        assert!(table_exists(&d.entities, "entry_meta"));
        assert!(table_exists(&d.entities, "entry_scores"));
        assert!(table_exists(&d.entities, "entry_signatures"));
        assert!(table_exists(&d.entities, "entry_refs"));
        assert!(table_exists(&d.entities, "entry_graph_edges"));
        assert!(table_exists(&d.entities, "entry_translations"));
        assert!(table_exists(&d.entities, "entry_security_findings"));
        assert!(table_exists(&d.entities, "dict_meta"));
        // DB1 tables absent on entities tier.
        assert!(!table_exists(&d.entities, "concepts"));
        assert!(!table_exists(&d.entities, "concept_defs"));
        assert!(!table_exists(&d.entities, "concept_members"));
        assert!(!table_exists(&d.entities, "required_axes"));
    }

    /// T3.2: assert the three new score dimensions (quality,
    /// maintenance, accessibility) land alongside the original eight.
    /// Schema-only check — population pipeline lands with the corpus
    /// pilot per the approved plan.
    #[test]
    fn entry_scores_has_t3_2_extended_dimensions() {
        let d = Dict::open_in_memory().unwrap();
        let mut stmt = d
            .entities
            .prepare("PRAGMA table_info(entry_scores)")
            .unwrap();
        let cols: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        for required in [
            "security",
            "reliability",
            "performance",
            "readability",
            "testability",
            "portability",
            "composability",
            "maturity",
            "quality",
            "maintenance",
            "accessibility",
            "overall_score",
        ] {
            assert!(
                cols.iter().any(|c| c == required),
                "entry_scores missing column {required}; have {cols:?}"
            );
        }
    }

    /// T3.2 follow-on: the new columns are nullable so existing
    /// inserts that skip them still succeed. Locks the no-population
    /// invariant — schema only, no required defaults.
    #[test]
    fn entry_scores_t3_2_columns_are_nullable() {
        let d = Dict::open_in_memory().unwrap();
        // Insert a parent row in entries first (entry_scores.id FK).
        d.entities
            .execute(
                "INSERT INTO entries (id, word, kind, language, status) \
                 VALUES ('h_test', 'test', 'function', 'nom', 'partial')",
                [],
            )
            .unwrap();
        // Insert into entry_scores using only the original columns.
        d.entities
            .execute(
                "INSERT INTO entry_scores (id, security, reliability) \
                 VALUES ('h_test', 0.5, 0.7)",
                [],
            )
            .unwrap();
        let (q, m, a): (Option<f64>, Option<f64>, Option<f64>) = d
            .entities
            .query_row(
                "SELECT quality, maintenance, accessibility FROM entry_scores WHERE id = 'h_test'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert!(q.is_none() && m.is_none() && a.is_none());
    }

    #[test]
    fn concept_members_has_no_cross_file_fk() {
        // Per doc 22 §1: concept_members.entry_id is a dangling hash reference;
        // inserting a row with a non-existent entry hash must succeed.
        let d = Dict::open_in_memory().unwrap();
        d.concepts
            .execute(
                "INSERT INTO concepts (id, name) VALUES ('cid1', 'test_concept')",
                [],
            )
            .unwrap();
        let nonexistent_entry_hash = "sha256-never-exists-in-words-tier";
        d.concepts
            .execute(
                "INSERT INTO concept_members (concept_id, entry_id) VALUES (?1, ?2)",
                ["cid1", nonexistent_entry_hash],
            )
            .unwrap();
        let n: i64 = d
            .concepts
            .query_row("SELECT COUNT(*) FROM concept_members", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn concept_defs_round_trip() {
        let d = Dict::open_in_memory().unwrap();
        d.concepts
            .execute(
                "INSERT INTO concept_defs (name, repo_id, intent, index_into_db2, src_path, src_hash) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                [
                    "authentication",
                    "demo_repo",
                    "Login and session management",
                    "[]",
                    "authentication/authentication.nom",
                    "deadbeef",
                ],
            )
            .unwrap();
        let got_intent: String = d
            .concepts
            .query_row(
                "SELECT intent FROM concept_defs WHERE name = 'authentication'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(got_intent, "Login and session management");
    }

    #[test]
    fn entities_round_trip_on_split_tier() {
        let d = Dict::open_in_memory().unwrap();
        d.entities
            .execute(
                "INSERT INTO entities (hash, word, kind) VALUES (?1, ?2, ?3)",
                ["abc123", "login_user", "function"],
            )
            .unwrap();
        let got_kind: String = d
            .entities
            .query_row("SELECT kind FROM entities WHERE hash = 'abc123'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(got_kind, "function");
    }

    fn sample_word(hash: &str, word: &str, kind: &str) -> EntityRow {
        EntityRow {
            hash: hash.to_string(),
            word: word.to_string(),
            kind: kind.to_string(),
            signature: None,
            contracts: None,
            body_kind: None,
            body_size: None,
            origin_ref: None,
            bench_ids: None,
            authored_in: None,
            composed_of: None,
            status: "complete".to_string(),
        }
    }

    #[test]
    fn free_fn_upsert_entity_writes_to_words_tier() {
        let d = Dict::open_in_memory().unwrap();
        upsert_entity(&d, &sample_word("h1", "greet", "function")).unwrap();
        let got = find_entity(&d, "h1").unwrap().expect("row");
        assert_eq!(got.word, "greet");
        assert_eq!(got.kind, "function");
    }

    #[test]
    fn free_fn_upsert_entity_is_idempotent() {
        let d = Dict::open_in_memory().unwrap();
        upsert_entity(&d, &sample_word("h2", "a", "function")).unwrap();
        // Same hash → UPDATE path.
        upsert_entity(&d, &sample_word("h2", "a_v2", "module")).unwrap();
        let got = find_entity(&d, "h2").unwrap().expect("row");
        assert_eq!(got.word, "a_v2");
        assert_eq!(got.kind, "module");
        assert_eq!(count_entities(&d).unwrap(), 1);
    }

    #[test]
    fn free_fn_find_entities_by_word_orders_by_hash() {
        let d = Dict::open_in_memory().unwrap();
        upsert_entity(&d, &sample_word("bbb", "shared", "function")).unwrap();
        upsert_entity(&d, &sample_word("aaa", "shared", "function")).unwrap();
        upsert_entity(&d, &sample_word("ccc", "shared", "data")).unwrap();
        let rows = find_entities_by_word(&d, "shared").unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].hash, "aaa");
        assert_eq!(rows[1].hash, "bbb");
        assert_eq!(rows[2].hash, "ccc");
    }

    #[test]
    fn free_fn_find_entities_by_kind_filters_and_orders() {
        let d = Dict::open_in_memory().unwrap();
        upsert_entity(&d, &sample_word("z", "a", "function")).unwrap();
        upsert_entity(&d, &sample_word("m", "b", "function")).unwrap();
        upsert_entity(&d, &sample_word("x", "c", "data")).unwrap();
        let rows = find_entities_by_kind(&d, "function").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].hash, "m");
        assert_eq!(rows[1].hash, "z");
    }

    #[test]
    fn free_fn_find_entity_returns_none_when_missing() {
        let d = Dict::open_in_memory().unwrap();
        assert!(find_entity(&d, "nonexistent").unwrap().is_none());
    }

    #[test]
    fn free_fn_count_entities_starts_at_zero() {
        let d = Dict::open_in_memory().unwrap();
        assert_eq!(count_entities(&d).unwrap(), 0);
        upsert_entity(&d, &sample_word("h", "w", "function")).unwrap();
        assert_eq!(count_entities(&d).unwrap(), 1);
    }

    #[test]
    fn count_entities_by_status_groups_correctly() {
        let d = Dict::open_in_memory().unwrap();
        let mut partial = sample_word("p1", "stub_a", "function");
        partial.status = "partial".to_string();
        let mut partial2 = sample_word("p2", "stub_b", "function");
        partial2.status = "partial".to_string();
        upsert_entity(&d, &partial).unwrap();
        upsert_entity(&d, &partial2).unwrap();
        upsert_entity(&d, &sample_word("c1", "real_a", "function")).unwrap();
        let hist = count_entities_by_status(&d).unwrap();
        // partial=2, complete=1 — higher count first
        assert_eq!(hist, vec![("partial".into(), 2), ("complete".into(), 1)]);
    }

    #[test]
    fn find_partial_entity_ids_returns_only_partials() {
        let d = Dict::open_in_memory().unwrap();
        let mut partial = sample_word("p1", "stub", "function");
        partial.status = "partial".to_string();
        upsert_entity(&d, &partial).unwrap();
        upsert_entity(&d, &sample_word("c1", "real", "function")).unwrap();
        let ids = find_partial_entity_ids(&d, None).unwrap();
        assert_eq!(ids, vec!["p1"]);
    }

    #[test]
    fn find_partial_entity_ids_respects_max_cap() {
        let d = Dict::open_in_memory().unwrap();
        for i in 0..5 {
            let mut row = sample_word(&format!("p{i}"), "stub", "function");
            row.status = "partial".to_string();
            upsert_entity(&d, &row).unwrap();
        }
        let ids = find_partial_entity_ids(&d, Some(3)).unwrap();
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn entity_status_defaults_to_complete_on_roundtrip() {
        let d = Dict::open_in_memory().unwrap();
        upsert_entity(&d, &sample_word("h1", "greet", "function")).unwrap();
        let got = find_entity(&d, "h1").unwrap().expect("row");
        assert_eq!(got.status, "complete");
    }

    #[test]
    fn concepts_tier_has_no_entities_so_writes_go_to_words_only() {
        // Confirms the routing: upsert_entity only touches d.entities, not d.concepts.
        let d = Dict::open_in_memory().unwrap();
        upsert_entity(&d, &sample_word("h", "w", "function")).unwrap();
        let concepts_has_table: bool = d
            .concepts
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'entities'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);
        assert!(
            !concepts_has_table,
            "concepts tier should have no entities table"
        );
        assert_eq!(count_entities(&d).unwrap(), 1);
    }

    #[test]
    fn root_reflects_directory_form() {
        let dir = tempdir().unwrap();
        let d = Dict::open_dir(dir.path()).unwrap();
        assert_eq!(d.root(), dir.path());
    }

    // ── S3b: tests for the next batch of free-function dict APIs ────

    #[test]
    fn count_concept_defs_starts_zero_and_grows_after_insert() {
        let d = Dict::open_in_memory().unwrap();
        assert_eq!(count_concept_defs(&d).unwrap(), 0);
        d.concepts
            .execute(
                "INSERT INTO concept_defs \
                 (name, repo_id, intent, index_into_db2, exposes, acceptance, objectives, \
                  src_path, src_hash, body_hash, created_at, updated_at) \
                 VALUES ('demo_concept', 'r1', 'i', '[]', '', '', '', '', '', '', \
                 '2026-04-14T00:00:00Z', '2026-04-14T00:00:00Z')",
                [],
            )
            .unwrap();
        assert_eq!(count_concept_defs(&d).unwrap(), 1);
    }

    #[test]
    fn count_required_axes_starts_zero() {
        let d = Dict::open_in_memory().unwrap();
        assert_eq!(count_required_axes(&d).unwrap(), 0);
    }

    #[test]
    fn body_kind_histogram_groups_with_untagged_bucket() {
        let d = Dict::open_in_memory().unwrap();
        // Two bc, one untagged (NULL body_kind).
        for (h, kind) in [("h1", Some("bc")), ("h2", Some("bc")), ("h3", None::<&str>)] {
            d.entities
                .execute(
                    "INSERT INTO entities (hash, word, kind, body_kind) \
                     VALUES (?1, 'w', 'function', ?2)",
                    rusqlite::params![h, kind],
                )
                .unwrap();
        }
        let h = body_kind_histogram(&d).unwrap();
        // 'bc' bucket first (count 2), then '(untagged)' (count 1).
        assert_eq!(
            h,
            vec![("bc".to_string(), 2), ("(untagged)".to_string(), 1)]
        );
    }

    #[test]
    fn resolve_prefix_returns_full_hash_when_unique() {
        let d = Dict::open_in_memory().unwrap();
        d.entities
            .execute(
                "INSERT INTO entities (hash, word, kind) VALUES ('abcdef1234', 'w', 'function')",
                [],
            )
            .unwrap();
        let h = resolve_prefix(&d, "abc").unwrap();
        assert_eq!(h, "abcdef1234");
    }

    #[test]
    fn resolve_prefix_errors_when_no_match() {
        let d = Dict::open_in_memory().unwrap();
        let err = resolve_prefix(&d, "zzz").unwrap_err();
        assert!(err.to_string().contains("no entity hash matches"));
    }

    #[test]
    fn resolve_prefix_errors_when_ambiguous() {
        let d = Dict::open_in_memory().unwrap();
        for h in ["abc1", "abc2"] {
            d.entities
                .execute(
                    "INSERT INTO entities (hash, word, kind) VALUES (?1, 'w', 'function')",
                    rusqlite::params![h],
                )
                .unwrap();
        }
        let err = resolve_prefix(&d, "abc").unwrap_err();
        assert!(err.to_string().contains("ambiguous prefix"));
    }

    #[test]
    fn count_entities_meta_starts_zero() {
        let d = Dict::open_in_memory().unwrap();
        assert_eq!(count_entities_meta(&d).unwrap(), 0);
    }

    // ── S4 port tests ──────────────────────────────────────────────

    fn seed_entry(d: &Dict, id: &str, status: &str) {
        d.entities
            .execute(
                "INSERT INTO entries (id, word, kind, language, status, body_bytes) \
                 VALUES (?1, ?2, 'function', 'nom', ?3, x'aabbcc')",
                rusqlite::params![id, id, status],
            )
            .unwrap();
    }

    #[test]
    fn status_histogram_groups_by_status_count_then_label() {
        let d = Dict::open_in_memory().unwrap();
        let mut p1 = sample_word("a", "word_a", "function");
        p1.status = "partial".to_string();
        let mut p2 = sample_word("b", "word_b", "function");
        p2.status = "partial".to_string();
        upsert_entity(&d, &p1).unwrap();
        upsert_entity(&d, &p2).unwrap();
        upsert_entity(&d, &sample_word("c", "word_c", "function")).unwrap();
        let h = status_histogram(&d).unwrap();
        // partial=2, complete=1 — partial first (higher count), then complete
        assert_eq!(h, vec![("partial".into(), 2), ("complete".into(), 1)]);
    }

    #[test]
    fn get_entry_bytes_returns_blob_or_none_consistently() {
        let d = Dict::open_in_memory().unwrap();
        seed_entry(&d, "h_one", "partial");
        let bytes = get_entry_bytes(&d, "h_one").unwrap();
        assert_eq!(bytes, Some(vec![0xaa, 0xbb, 0xcc]));
        assert!(get_entry_bytes(&d, "missing").unwrap().is_none());
    }

    #[test]
    fn list_partial_ids_filters_and_caps() {
        let d = Dict::open_in_memory().unwrap();
        seed_entry(&d, "a", "partial");
        seed_entry(&d, "b", "complete");
        seed_entry(&d, "c", "partial");
        seed_entry(&d, "d", "partial");
        // Without cap — all three partials in id order.
        assert_eq!(
            list_partial_ids(&d, None).unwrap(),
            vec!["a".to_string(), "c".into(), "d".into()]
        );
        // With cap of 2.
        assert_eq!(
            list_partial_ids(&d, Some(2)).unwrap(),
            vec!["a".to_string(), "c".into()]
        );
    }

    #[test]
    fn get_meta_orders_by_key_then_value() {
        let d = Dict::open_in_memory().unwrap();
        seed_entry(&d, "h_meta", "partial");
        for (k, v) in [("ax", "2"), ("ax", "1"), ("zz", "0")] {
            d.entities
                .execute(
                    "INSERT INTO entry_meta (id, key, value) VALUES (?1, ?2, ?3)",
                    rusqlite::params!["h_meta", k, v],
                )
                .unwrap();
        }
        let m = get_meta(&d, "h_meta").unwrap();
        assert_eq!(
            m,
            vec![
                ("ax".into(), "1".into()),
                ("ax".into(), "2".into()),
                ("zz".into(), "0".into()),
            ]
        );
    }

    // ── S5 port tests ──────────────────────────────────────────────

    fn seed_concept(d: &Dict, id: &str, name: &str) {
        d.concepts
            .execute(
                "INSERT INTO concepts (id, name) VALUES (?1, ?2)",
                rusqlite::params![id, name],
            )
            .unwrap();
    }

    fn make_concept_row(name: &str, repo_id: &str) -> ConceptRow {
        ConceptRow {
            name: name.to_string(),
            repo_id: repo_id.to_string(),
            intent: format!("intent of {name}"),
            index_into_db2: r#"[{"hash":"abc","label":"foo"}]"#.to_string(),
            exposes: "[]".to_string(),
            acceptance: "[]".to_string(),
            objectives: "[]".to_string(),
            src_path: format!("src/{name}.nom"),
            src_hash: "deadbeef".to_string(),
            body_hash: None,
        }
    }

    #[test]
    fn closure_walks_entry_refs_in_bfs_order_and_skips_cycles() {
        let d = Dict::open_in_memory().unwrap();
        // Layout: a -> b, a -> c, b -> d, c -> d (diamond), d -> a (cycle)
        for id in ["a", "b", "c", "d"] {
            seed_entry(&d, id, "partial");
        }
        for (from, to) in [("a", "b"), ("a", "c"), ("b", "d"), ("c", "d"), ("d", "a")] {
            d.entities
                .execute(
                    "INSERT INTO entry_refs (from_id, to_id) VALUES (?1, ?2)",
                    rusqlite::params![from, to],
                )
                .unwrap();
        }
        let walk = closure(&d, "a").unwrap();
        // Root first, then b/c (siblings — order depends on insertion),
        // then d. Cycle through d->a is suppressed by `seen`.
        assert_eq!(walk[0], "a");
        assert!(walk.contains(&"b".to_string()));
        assert!(walk.contains(&"c".to_string()));
        assert!(walk.contains(&"d".to_string()));
        assert_eq!(walk.len(), 4, "cycle must not duplicate ids; got {walk:?}");
    }

    #[test]
    fn get_concept_id_by_name_returns_some_then_none() {
        let d = Dict::open_in_memory().unwrap();
        seed_concept(&d, "cid_crypto", "cryptography");
        assert_eq!(
            get_concept_id_by_name(&d, "cryptography").unwrap(),
            Some("cid_crypto".to_string())
        );
        assert_eq!(get_concept_id_by_name(&d, "missing").unwrap(), None);
        // Whitespace trimmed.
        assert_eq!(
            get_concept_id_by_name(&d, "  cryptography  ").unwrap(),
            Some("cid_crypto".to_string())
        );
    }

    #[test]
    fn list_concept_ids_sorts_by_name() {
        let d = Dict::open_in_memory().unwrap();
        seed_concept(&d, "z", "zoology");
        seed_concept(&d, "a", "anthropology");
        seed_concept(&d, "m", "mathematics");
        let v = list_concept_ids(&d).unwrap();
        assert_eq!(
            v,
            vec![
                ("a".to_string(), "anthropology".to_string()),
                ("m".into(), "mathematics".into()),
                ("z".into(), "zoology".into()),
            ]
        );
    }

    #[test]
    fn delete_concept_removes_only_named_row() {
        let d = Dict::open_in_memory().unwrap();
        seed_concept(&d, "c1", "alpha");
        seed_concept(&d, "c2", "beta");
        delete_concept(&d, "alpha").unwrap();
        let v = list_concept_ids(&d).unwrap();
        assert_eq!(v, vec![("c2".to_string(), "beta".to_string())]);
        // Idempotent on missing name.
        delete_concept(&d, "missing").unwrap();
    }

    #[test]
    fn concept_def_round_trip_free_fn() {
        let d = Dict::open_in_memory().unwrap();
        let row = ConceptRow {
            name: "auth_system".to_string(),
            repo_id: "repo-abc".to_string(),
            intent: "Validates JWT tokens for API access".to_string(),
            index_into_db2: r#"[{"hash":"cafebabe","label":"validate_token_jwt"}]"#.to_string(),
            exposes: r#"["validate_token_jwt"]"#.to_string(),
            acceptance: r#"[{"given":"valid jwt","expect":"true"}]"#.to_string(),
            objectives: r#"["security","reliability"]"#.to_string(),
            src_path: "src/auth.nom".to_string(),
            src_hash: "0011223344556677".to_string(),
            body_hash: Some("aabbccdd".to_string()),
        };
        upsert_concept_def(&d, &row).unwrap();

        let fetched = find_concept_def(&d, "auth_system").unwrap().unwrap();
        assert_eq!(fetched, row);
    }

    #[test]
    fn concept_def_upsert_overwrites_free_fn() {
        let d = Dict::open_in_memory().unwrap();
        let mut row = make_concept_row("payments", "repo-pay");
        upsert_concept_def(&d, &row).unwrap();

        let original = find_concept_def(&d, "payments").unwrap().unwrap();
        assert_eq!(original.intent, "intent of payments");

        row.intent = "Process Stripe + PayPal transactions".to_string();
        upsert_concept_def(&d, &row).unwrap();

        let updated = find_concept_def(&d, "payments").unwrap().unwrap();
        assert_eq!(updated.intent, "Process Stripe + PayPal transactions");

        let all = list_concept_defs_in_repo(&d, "repo-pay").unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn register_and_list_roundtrip_free_fn() {
        let d = Dict::open_in_memory().unwrap();
        register_required_axis(&d, "repo-a", "concept", "security", "at_least_one").unwrap();
        register_required_axis(&d, "repo-a", "concept", "safety", "exactly_one").unwrap();

        let axes = list_required_axes(&d, "repo-a", "concept").unwrap();
        assert_eq!(axes.len(), 2, "expected 2 axes, got: {axes:?}");
        assert_eq!(axes[0].axis, "safety");
        assert_eq!(axes[0].cardinality, "exactly_one");
        assert_eq!(axes[1].axis, "security");
        assert_eq!(axes[1].cardinality, "at_least_one");
    }

    #[test]
    fn register_required_axis_rejects_unknown_scope_free_fn() {
        let d = Dict::open_in_memory().unwrap();
        let err = register_required_axis(&d, "repo-x", "planet", "correctness", "at_least_one")
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("planet") || msg.contains("scope"),
            "error must mention invalid scope: {msg}"
        );
    }

    #[test]
    fn register_required_axis_normalizes_axis_free_fn() {
        let d = Dict::open_in_memory().unwrap();
        register_required_axis(&d, "repo-b", "module", " Security ", "at_least_one").unwrap();
        register_required_axis(&d, "repo-b", "module", "SECURITY", "exactly_one").unwrap();
        let axes = list_required_axes(&d, "repo-b", "module").unwrap();
        assert_eq!(axes.len(), 1);
        assert_eq!(axes[0].axis, "security");
        assert_eq!(axes[0].cardinality, "exactly_one");
    }

    #[test]
    fn concept_round_trip_free_fns() {
        let d = Dict::open_in_memory().unwrap();
        let concept = Concept {
            id: "cid_crypto".to_string(),
            name: "cryptography".to_string(),
            describe: Some("Hashing, signing, encryption".to_string()),
            created_at: "2026-04-14T00:00:00Z".to_string(),
            updated_at: None,
        };
        upsert_concept(&d, &concept).unwrap();

        let fetched = get_concept_by_name(&d, "cryptography").unwrap().unwrap();
        assert_eq!(fetched.id, concept.id);
        assert_eq!(fetched.name, concept.name);
        assert_eq!(fetched.describe, concept.describe);

        let listed = list_concepts(&d).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "cryptography");
    }

    #[test]
    fn concept_member_remove_and_count_free_fns() {
        let d = Dict::open_in_memory().unwrap();
        seed_concept(&d, "cid", "math");
        seed_full_entry(&d, "entry_a", "add", None, None);
        seed_full_entry(&d, "entry_b", "mul", None, None);

        assert!(add_concept_member(&d, "cid", "entry_a").unwrap());
        assert!(add_concept_member(&d, "cid", "entry_b").unwrap());
        assert_eq!(count_concept_members(&d, "cid").unwrap(), 2);

        let members = get_concept_members(&d, "cid").unwrap();
        let ids: Vec<&str> = members.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["entry_a", "entry_b"]);

        remove_concept_member(&d, "cid", "entry_a").unwrap();
        assert_eq!(count_concept_members(&d, "cid").unwrap(), 1);
    }

    #[test]
    fn unregister_required_axis_free_fn() {
        let d = Dict::open_in_memory().unwrap();
        register_required_axis(&d, "repo-z", "app", "performance", "at_least_one").unwrap();
        assert!(unregister_required_axis(&d, "repo-z", "app", "performance").unwrap());
        assert!(!unregister_required_axis(&d, "repo-z", "app", "performance").unwrap());
    }

    #[test]
    fn seed_standard_axes_free_fn() {
        let d = Dict::open_in_memory().unwrap();
        let seeded = seed_standard_axes(&d, "app-seed").unwrap();
        assert_eq!(seeded.len(), 5);
        let listed = list_required_axes(&d, "app-seed", "app").unwrap();
        assert_eq!(listed.len(), 5);
    }

    #[test]
    fn add_concept_members_by_filter_free_fn() {
        let d = Dict::open_in_memory().unwrap();
        seed_concept(&d, "cid", "services");
        seed_full_entry(&d, "e1", "auth", Some("module"), Some("Auth module"));
        seed_full_entry(&d, "e2", "cache", Some("module"), Some("Cache module"));
        seed_full_entry(&d, "e3", "log", Some("function"), Some("Log helper"));

        let filter = EntryFilter {
            body_kind: Some("module".to_string()),
            limit: 50,
            ..EntryFilter::default()
        };
        let added = add_concept_members_by_filter(&d, "cid", &filter).unwrap();
        assert_eq!(added, 2);
        assert_eq!(count_concept_members(&d, "cid").unwrap(), 2);
    }

    // ── S6 port tests ──────────────────────────────────────────────

    // ── S7 port tests ──────────────────────────────────────────────

    fn seed_full_entry(
        d: &Dict,
        id: &str,
        word: &str,
        body_kind: Option<&str>,
        describe: Option<&str>,
    ) {
        d.entities
            .execute(
                "INSERT INTO entries (id, word, kind, language, status, body_kind, describe) \
                 VALUES (?1, ?2, 'function', 'nom', 'partial', ?3, ?4)",
                rusqlite::params![id, word, body_kind, describe],
            )
            .unwrap();
    }

    #[test]
    fn find_by_word_returns_all_matches_ordered_by_id() {
        let d = Dict::open_in_memory().unwrap();
        seed_full_entry(&d, "h_b", "add", None, None);
        seed_full_entry(&d, "h_a", "add", None, None);
        seed_full_entry(&d, "h_c", "mul", None, None);
        let v = find_by_word(&d, "add").unwrap();
        let ids: Vec<&str> = v.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["h_a", "h_b"]);
    }

    #[test]
    fn find_by_body_kind_filters_and_caps() {
        let d = Dict::open_in_memory().unwrap();
        seed_full_entry(&d, "a", "x", Some("module"), None);
        seed_full_entry(&d, "b", "y", Some("module"), None);
        seed_full_entry(&d, "c", "z", Some("module"), None);
        seed_full_entry(&d, "d", "w", Some("function"), None);
        let v = find_by_body_kind(&d, "module", 2).unwrap();
        let ids: Vec<&str> = v.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b"]);
    }

    #[test]
    fn search_describe_is_case_insensitive_substring() {
        let d = Dict::open_in_memory().unwrap();
        seed_full_entry(&d, "a", "sha", None, Some("Compute SHA-256 hash"));
        seed_full_entry(&d, "b", "md5", None, Some("Compute MD5 hash"));
        seed_full_entry(&d, "c", "uuid", None, Some("Generate UUID v4"));
        let v = search_describe(&d, "sha", 10).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].id, "a");
        let v2 = search_describe(&d, "compute", 10).unwrap();
        assert_eq!(v2.len(), 2);
    }

    #[test]
    fn get_scores_returns_some_after_set_then_none_for_missing() {
        let d = Dict::open_in_memory().unwrap();
        seed_full_entry(&d, "h_score", "x", None, None);
        let scores = nom_types::EntryScores {
            id: "h_score".into(),
            security: Some(0.9),
            reliability: Some(0.8),
            performance: Some(0.7),
            readability: Some(0.6),
            testability: Some(0.5),
            portability: Some(0.4),
            composability: Some(0.3),
            maturity: Some(0.2),
            overall_score: Some(0.55),
        };
        set_scores(&d, "h_score", &scores).unwrap();
        let got = get_scores(&d, "h_score").unwrap().unwrap();
        assert_eq!(got.security, Some(0.9));
        assert_eq!(got.composability, Some(0.3));
        assert!(get_scores(&d, "missing").unwrap().is_none());
    }

    #[test]
    fn get_findings_returns_all_findings_in_finding_id_order() {
        let d = Dict::open_in_memory().unwrap();
        seed_full_entry(&d, "h_f", "x", None, None);
        for cat in ["a_cat", "b_cat"] {
            add_finding(
                &d,
                "h_f",
                &nom_types::SecurityFinding {
                    finding_id: 0,
                    id: "h_f".into(),
                    severity: nom_types::Severity::High,
                    category: cat.into(),
                    rule_id: None,
                    message: None,
                    evidence: None,
                    line: None,
                    remediation: None,
                },
            )
            .unwrap();
        }
        let v = get_findings(&d, "h_f").unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].category, "a_cat");
        assert_eq!(v[1].category, "b_cat");
    }

    #[test]
    fn set_scores_inserts_then_updates_via_conflict_clause() {
        let d = Dict::open_in_memory().unwrap();
        seed_entry(&d, "h_s", "partial");
        let scores = nom_types::EntryScores {
            id: "h_s".into(),
            security: Some(0.8),
            reliability: Some(0.7),
            performance: Some(0.6),
            readability: Some(0.5),
            testability: Some(0.4),
            portability: Some(0.3),
            composability: Some(0.9),
            maturity: Some(0.95),
            overall_score: Some(0.65),
        };
        set_scores(&d, "h_s", &scores).unwrap();
        // Update with a different overall_score; row must be updated, not duplicated.
        let mut updated = scores.clone();
        updated.overall_score = Some(0.99);
        set_scores(&d, "h_s", &updated).unwrap();
        let n: i64 = d
            .entities
            .query_row(
                "SELECT COUNT(*) FROM entry_scores WHERE id = 'h_s'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
        let overall: Option<f32> = d
            .entities
            .query_row(
                "SELECT overall_score FROM entry_scores WHERE id = 'h_s'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        // EntryScores fields are f32, so the round-trip is f32-precise.
        assert!(matches!(overall, Some(v) if (v - 0.99_f32).abs() < f32::EPSILON));
    }

    #[test]
    fn add_meta_is_idempotent_per_pk_triple() {
        let d = Dict::open_in_memory().unwrap();
        seed_entry(&d, "h_m", "partial");
        add_meta(&d, "h_m", "tag", "alpha").unwrap();
        add_meta(&d, "h_m", "tag", "alpha").unwrap(); // dup, ignored
        add_meta(&d, "h_m", "tag", "beta").unwrap();
        let n: i64 = d
            .entities
            .query_row(
                "SELECT COUNT(*) FROM entry_meta WHERE id = 'h_m'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 2, "two distinct (key, value) tuples expected");
    }

    #[test]
    fn set_signature_inserts_then_updates() {
        let d = Dict::open_in_memory().unwrap();
        seed_entry(&d, "h_sig", "partial");
        let sig = nom_types::EntrySignature {
            id: "h_sig".into(),
            visibility: Some("pub".into()),
            is_async: false,
            is_method: false,
            return_type: Some("i32".into()),
            params_json: "[]".into(),
        };
        set_signature(&d, "h_sig", &sig).unwrap();
        let mut updated = sig.clone();
        updated.is_async = true;
        set_signature(&d, "h_sig", &updated).unwrap();
        let async_flag: bool = d
            .entities
            .query_row(
                "SELECT is_async FROM entry_signatures WHERE id = 'h_sig'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(async_flag);
    }

    #[test]
    fn add_finding_appends_many_rows_per_entry() {
        let d = Dict::open_in_memory().unwrap();
        seed_entry(&d, "h_f", "partial");
        for cat in ["injection", "xss", "csrf"] {
            add_finding(
                &d,
                "h_f",
                &nom_types::SecurityFinding {
                    finding_id: 0,
                    id: "h_f".into(),
                    severity: nom_types::Severity::Critical,
                    category: cat.into(),
                    rule_id: None,
                    message: None,
                    evidence: None,
                    line: None,
                    remediation: None,
                },
            )
            .unwrap();
        }
        let n: i64 = d
            .entities
            .query_row(
                "SELECT COUNT(*) FROM entry_security_findings WHERE id = 'h_f'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 3, "three findings expected");
    }

    #[test]
    fn add_ref_is_idempotent_per_pk_pair() {
        let d = Dict::open_in_memory().unwrap();
        seed_entry(&d, "h_a", "partial");
        seed_entry(&d, "h_b", "partial");
        add_ref(&d, "h_a", "h_b").unwrap();
        add_ref(&d, "h_a", "h_b").unwrap(); // dup, ignored
        let n: i64 = d
            .entities
            .query_row(
                "SELECT COUNT(*) FROM entry_refs WHERE from_id = 'h_a'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn add_concept_member_is_idempotent_and_reports_change() {
        let d = Dict::open_in_memory().unwrap();
        seed_concept(&d, "cid", "math");
        // First add inserts → true.
        assert!(add_concept_member(&d, "cid", "entry_a").unwrap());
        // Second add is a no-op → false.
        assert!(!add_concept_member(&d, "cid", "entry_a").unwrap());
        // Different entry → true again.
        assert!(add_concept_member(&d, "cid", "entry_b").unwrap());
    }

    #[test]
    fn get_refs_orders_by_target_id() {
        let d = Dict::open_in_memory().unwrap();
        seed_entry(&d, "from_id", "partial");
        seed_entry(&d, "to_b", "partial");
        seed_entry(&d, "to_a", "partial");
        for to_id in ["to_b", "to_a"] {
            d.entities
                .execute(
                    "INSERT INTO entry_refs (from_id, to_id) VALUES (?1, ?2)",
                    rusqlite::params!["from_id", to_id],
                )
                .unwrap();
        }
        // Sorted by to_id ascending.
        assert_eq!(
            get_refs(&d, "from_id").unwrap(),
            vec!["to_a".to_string(), "to_b".into()]
        );
    }

    // ── find_entities free function tests ────────────────────────────

    #[test]
    fn find_entities_empty_filter_returns_rows_ordered_by_hash() {
        let d = Dict::open_in_memory().unwrap();
        upsert_entity(&d, &sample_word("zzz", "c", "data")).unwrap();
        upsert_entity(&d, &sample_word("aaa", "a", "function")).unwrap();
        upsert_entity(&d, &sample_word("mmm", "b", "function")).unwrap();
        let f = EntryFilter { limit: 10, ..Default::default() };
        let rows = find_entities(&d, &f).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].hash, "aaa");
        assert_eq!(rows[1].hash, "mmm");
        assert_eq!(rows[2].hash, "zzz");
    }

    #[test]
    fn find_entities_filters_by_kind() {
        let d = Dict::open_in_memory().unwrap();
        upsert_entity(&d, &sample_word("h1", "fn_a", "function")).unwrap();
        upsert_entity(&d, &sample_word("h2", "fn_b", "function")).unwrap();
        upsert_entity(&d, &sample_word("h3", "mod_a", "module")).unwrap();
        let f = EntryFilter {
            kind: Some(nom_types::EntryKind::Function),
            limit: 10,
            ..Default::default()
        };
        let rows = find_entities(&d, &f).unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|r| r.kind == "function"));
    }

    #[test]
    fn find_entities_filters_by_body_kind() {
        let d = Dict::open_in_memory().unwrap();
        let mut r1 = sample_word("h1", "wasm_fn", "function");
        r1.body_kind = Some("wasm".to_string());
        let mut r2 = sample_word("h2", "bc_fn", "function");
        r2.body_kind = Some("bc".to_string());
        upsert_entity(&d, &r1).unwrap();
        upsert_entity(&d, &r2).unwrap();
        let f = EntryFilter {
            body_kind: Some("wasm".to_string()),
            limit: 10,
            ..Default::default()
        };
        let rows = find_entities(&d, &f).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].hash, "h1");
    }

    #[test]
    fn find_entities_respects_limit() {
        let d = Dict::open_in_memory().unwrap();
        for i in 0..5 {
            upsert_entity(&d, &sample_word(&format!("h{i}"), "w", "function")).unwrap();
        }
        let f = EntryFilter { limit: 3, ..Default::default() };
        let rows = find_entities(&d, &f).unwrap();
        assert_eq!(rows.len(), 3);
    }

    #[test]
    fn find_entities_ignores_language_filter_gracefully() {
        // entities table has no language column; the filter should be skipped,
        // not cause an error.
        let d = Dict::open_in_memory().unwrap();
        upsert_entity(&d, &sample_word("h1", "w", "function")).unwrap();
        let f = EntryFilter {
            language: Some("rust".to_string()),
            limit: 10,
            ..Default::default()
        };
        // Should return the row despite language filter (language is ignored).
        let rows = find_entities(&d, &f).unwrap();
        assert_eq!(rows.len(), 1);
    }
}
