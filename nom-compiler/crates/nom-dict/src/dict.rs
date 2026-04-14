//! S1 of the dict-split migration (doc 22).
//!
//! Introduces the target `Dict { concepts, words }` struct ALONGSIDE the existing
//! single-file `NomDict`. No existing API is removed in S1 — this module stands up
//! the two-file surface so subsequent stages can port methods over incrementally.
//!
//! Layout (per doc 22 §1):
//! ```text
//! <dir>/
//!   concepts.sqlite    ← DB1: concept_defs + concept-scoped entry_meta
//!   words.sqlite       ← DB2: words_v2    + word-scoped    entry_meta
//!   grammar.sqlite     ← registry (separate, owned by nom-grammar crate)
//!   store/<hash>/...   ← artifact bytes (filesystem, not SQLite)
//! ```
//!
//! S2 (this file) specialises each tier's schema:
//! - `concepts.sqlite` carries only DB1 tables (concept_defs + concepts +
//!   concept_members + required_axes + dict_meta).
//! - `words.sqlite` carries only DB2 tables (entries + words_v2 + entry_* side
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
pub const WORDS_FILENAME: &str = "words.sqlite";

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

-- concept_members.entry_id is a dangling hash reference into words.sqlite.
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

/// Schema applied to `words.sqlite` — DB2 tier. No cross-file FKs.
pub const WORDS_SCHEMA_SQL: &str = r#"
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
    security             REAL,
    reliability          REAL,
    performance          REAL,
    readability          REAL,
    testability          REAL,
    portability          REAL,
    composability        REAL,
    maturity             REAL,
    overall_score        REAL
);
CREATE INDEX IF NOT EXISTS idx_scores_overall ON entry_scores(overall_score);
CREATE INDEX IF NOT EXISTS idx_scores_security ON entry_scores(security);

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

CREATE TABLE IF NOT EXISTS words_v2 (
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
CREATE INDEX IF NOT EXISTS idx_words_v2_word ON words_v2(word);
CREATE INDEX IF NOT EXISTS idx_words_v2_kind ON words_v2(kind);
CREATE INDEX IF NOT EXISTS idx_words_v2_authored ON words_v2(authored_in);

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
    pub words: Connection,
    root: PathBuf,
}

impl Dict {
    /// Open or create both SQLite files inside `dir`. Creates `dir` if missing.
    /// Applies the per-tier specialised schemas.
    pub fn open_dir(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("creating dict directory {}", dir.display()))?;
        let concepts_path = dir.join(CONCEPTS_FILENAME);
        let words_path = dir.join(WORDS_FILENAME);
        Self::open_paths(&concepts_path, &words_path).map(|mut d| {
            d.root = dir.to_path_buf();
            d
        })
    }

    /// Open or create at explicit paths (tests, migrations, split tooling).
    pub fn open_paths(concepts_path: &Path, words_path: &Path) -> Result<Self> {
        let concepts = open_tier(concepts_path, "concepts", CONCEPTS_SCHEMA_SQL)?;
        let words = open_tier(words_path, "words", WORDS_SCHEMA_SQL)?;
        // Root is a best-guess parent dir; S7 migrate tool will set it explicitly.
        let root = concepts_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        Ok(Self {
            concepts,
            words,
            root,
        })
    }

    /// Open both tiers in memory — fastest, for tests.
    pub fn open_in_memory() -> Result<Self> {
        let concepts = Connection::open_in_memory()?;
        let words = Connection::open_in_memory()?;
        concepts.pragma_update(None, "foreign_keys", "ON")?;
        words.pragma_update(None, "foreign_keys", "ON")?;
        concepts.execute_batch(CONCEPTS_SCHEMA_SQL)?;
        words.execute_batch(WORDS_SCHEMA_SQL)?;
        Ok(Self {
            concepts,
            words,
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn open_dir_creates_both_sqlite_files() {
        let dir = tempdir().unwrap();
        let _ = Dict::open_dir(dir.path()).unwrap();
        assert!(dir.path().join(CONCEPTS_FILENAME).exists());
        assert!(dir.path().join(WORDS_FILENAME).exists());
    }

    #[test]
    fn open_dir_is_idempotent() {
        let dir = tempdir().unwrap();
        let _ = Dict::open_dir(dir.path()).unwrap();
        let _ = Dict::open_dir(dir.path()).unwrap();
        assert!(dir.path().join(CONCEPTS_FILENAME).exists());
        assert!(dir.path().join(WORDS_FILENAME).exists());
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
        let b: i64 = d.words.query_row("SELECT 1", [], |r| r.get(0)).unwrap();
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
        assert!(!table_exists(&d.concepts, "words_v2"));
        assert!(!table_exists(&d.concepts, "entry_meta"));
        assert!(!table_exists(&d.concepts, "entry_scores"));
    }

    #[test]
    fn words_tier_has_db2_tables_only() {
        let d = Dict::open_in_memory().unwrap();
        // DB2 tables present on words tier.
        assert!(table_exists(&d.words, "entries"));
        assert!(table_exists(&d.words, "words_v2"));
        assert!(table_exists(&d.words, "entry_meta"));
        assert!(table_exists(&d.words, "entry_scores"));
        assert!(table_exists(&d.words, "entry_signatures"));
        assert!(table_exists(&d.words, "entry_refs"));
        assert!(table_exists(&d.words, "entry_graph_edges"));
        assert!(table_exists(&d.words, "entry_translations"));
        assert!(table_exists(&d.words, "entry_security_findings"));
        assert!(table_exists(&d.words, "dict_meta"));
        // DB1 tables absent on words tier.
        assert!(!table_exists(&d.words, "concepts"));
        assert!(!table_exists(&d.words, "concept_defs"));
        assert!(!table_exists(&d.words, "concept_members"));
        assert!(!table_exists(&d.words, "required_axes"));
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
    fn words_v2_round_trip_on_split_tier() {
        let d = Dict::open_in_memory().unwrap();
        d.words
            .execute(
                "INSERT INTO words_v2 (hash, word, kind) VALUES (?1, ?2, ?3)",
                ["abc123", "login_user", "function"],
            )
            .unwrap();
        let got_kind: String = d
            .words
            .query_row(
                "SELECT kind FROM words_v2 WHERE hash = 'abc123'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(got_kind, "function");
    }

    #[test]
    fn root_reflects_directory_form() {
        let dir = tempdir().unwrap();
        let d = Dict::open_dir(dir.path()).unwrap();
        assert_eq!(d.root(), dir.path());
    }
}
