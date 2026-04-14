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
}

fn open_tier(path: &Path, label: &str, schema_sql: &str) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating parent dir for {} tier at {}", label, path.display()))?;
        }
    }
    let conn = Connection::open(path).with_context(|| {
        format!("opening {} tier at {}", label, path.display())
    })?;
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

use crate::{EntityRow, row_to_entity};
use rusqlite::{OptionalExtension, params};

/// Insert-or-update a row in `entities` on the entities tier.
pub fn upsert_entity(d: &Dict, row: &EntityRow) -> Result<()> {
    d.entities.execute(
        "INSERT INTO entities
             (hash, word, kind, signature, contracts, body_kind, body_size,
              origin_ref, bench_ids, authored_in, composed_of,
              created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, datetime('now'), NULL)
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
                    origin_ref, bench_ids, authored_in, composed_of
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
                origin_ref, bench_ids, authored_in, composed_of
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
                origin_ref, bench_ids, authored_in, composed_of
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
        _ => anyhow::bail!(
            "ambiguous prefix `{prefix}` — multiple entity hashes match"
        ),
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
        let a: i64 = d
            .concepts
            .query_row("SELECT 1", [], |r| r.get(0))
            .unwrap();
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
            .query_row(
                "SELECT kind FROM entities WHERE hash = 'abc123'",
                [],
                |r| r.get(0),
            )
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
        assert!(!concepts_has_table, "concepts tier should have no entities table");
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
        for (h, kind) in [
            ("h1", Some("bc")),
            ("h2", Some("bc")),
            ("h3", None::<&str>),
        ] {
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
        assert_eq!(h, vec![("bc".to_string(), 2), ("(untagged)".to_string(), 1)]);
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
}
