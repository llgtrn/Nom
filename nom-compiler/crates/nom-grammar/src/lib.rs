//! nom-grammar — schema + query API for `grammar.sqlite`.
//!
//! This crate is grammar-AWARENESS code: it owns the SQL schema, opens
//! connections, and answers structural queries. The grammar DATA itself
//! (kind names, keyword tokens, clause shapes, quality names, patterns)
//! lives in the DB, never in this crate. Population is the user's
//! responsibility — via row-level CLI commands, SQL scripts, or whatever
//! tooling the user prefers. The DB starts empty after `nom grammar init`.

use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use std::path::Path;

pub const SCHEMA_VERSION: u32 = 1;

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS schema_meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS keywords (
  token          TEXT PRIMARY KEY,
  role           TEXT NOT NULL,
  kind_scope     TEXT,
  source_ref     TEXT NOT NULL,
  shipped_commit TEXT NOT NULL,
  notes          TEXT
);

-- Synonym registry. Maps every accepted alternative phrasing to a single
-- canonical keyword. The S1 lexer pass consults this table and rewrites
-- any matching surface token into its canonical form before the rest of
-- the pipeline runs. Equivalence between phrasings is rule-based (closed
-- table), never learned. Same input + same DB rows → same canonical
-- token stream.
CREATE TABLE IF NOT EXISTS keyword_synonyms (
  synonym           TEXT PRIMARY KEY,
  canonical_keyword TEXT NOT NULL,
  source_ref        TEXT NOT NULL,
  shipped_commit    TEXT NOT NULL,
  notes             TEXT
);
CREATE INDEX IF NOT EXISTS idx_keyword_synonyms_canonical
  ON keyword_synonyms(canonical_keyword);

CREATE TABLE IF NOT EXISTS clause_shapes (
  kind            TEXT NOT NULL,
  clause_name     TEXT NOT NULL,
  is_required     INTEGER NOT NULL,
  one_of_group    TEXT,
  position        INTEGER NOT NULL,
  grammar_shape   TEXT NOT NULL,
  min_occurrences INTEGER NOT NULL DEFAULT 0,
  max_occurrences INTEGER,
  source_ref      TEXT NOT NULL,
  notes           TEXT,
  PRIMARY KEY (kind, clause_name, position)
);

CREATE TABLE IF NOT EXISTS quality_names (
  name            TEXT PRIMARY KEY,
  axis            TEXT NOT NULL,
  metric_function TEXT,                   -- nullable: populated by `nom corpus register-axis`
  cardinality     TEXT NOT NULL,
  required_at     TEXT,
  source_ref      TEXT NOT NULL,
  notes           TEXT
);

CREATE TABLE IF NOT EXISTS kinds (
  name            TEXT PRIMARY KEY,
  description     TEXT NOT NULL,
  allowed_clauses TEXT NOT NULL,
  allowed_refs    TEXT NOT NULL,
  shipped_commit  TEXT NOT NULL,
  notes           TEXT
);

-- Nom-native authoring patterns. Each row captures a reusable shape an
-- author can drop into a .nomx source. Patterns are described entirely in
-- Nom's vocabulary; foreign-language origins are absent by invariant. The
-- intent is that an AI client queries this table to find the canonical
-- shape for a given problem class without ever consulting external docs.
CREATE TABLE IF NOT EXISTS patterns (
  pattern_id       TEXT PRIMARY KEY,
  intent           TEXT NOT NULL,
  nom_kinds        TEXT NOT NULL,
  nom_clauses      TEXT NOT NULL,
  typed_slot_refs  TEXT NOT NULL,
  example_shape    TEXT NOT NULL,
  hazards          TEXT NOT NULL,
  favors           TEXT NOT NULL,
  source_doc_refs  TEXT NOT NULL,
  created_at       TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_patterns_intent ON patterns(intent);
"#;

/// Initialize an empty grammar.sqlite at the given path. Idempotent — safe to call on
/// an existing file; creates tables if absent and stamps schema_version.
pub fn init_at(path: impl AsRef<Path>) -> Result<Connection> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating parent dir for {:?}", path))?;
    }
    let conn = Connection::open(path)
        .with_context(|| format!("opening grammar.sqlite at {:?}", path))?;
    conn.execute_batch(SCHEMA_SQL)
        .context("applying grammar schema")?;
    conn.execute(
        "INSERT OR REPLACE INTO schema_meta(key, value) VALUES ('schema_version', ?1)",
        params![SCHEMA_VERSION.to_string()],
    )?;
    Ok(conn)
}

/// Read-only open; errors if the file doesn't exist.
pub fn open_readonly(path: impl AsRef<Path>) -> Result<Connection> {
    let path = path.as_ref();
    anyhow::ensure!(path.exists(), "grammar.sqlite not found at {:?}", path);
    let conn = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )?;
    Ok(conn)
}

/// Return the stored schema_version, or 0 if missing.
pub fn schema_version(conn: &Connection) -> Result<u32> {
    let v: Option<String> = conn
        .query_row(
            "SELECT value FROM schema_meta WHERE key = 'schema_version'",
            [],
            |r| r.get(0),
        )
        .ok();
    Ok(v.as_deref().and_then(|s| s.parse().ok()).unwrap_or(0))
}

/// Count rows in each top-level registry table. Useful for `nom grammar status`.
#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct RegistryCounts {
    pub keywords: u64,
    pub keyword_synonyms: u64,
    pub clause_shapes: u64,
    pub quality_names: u64,
    pub kinds: u64,
    pub patterns: u64,
}

pub fn counts(conn: &Connection) -> Result<RegistryCounts> {
    let count_of = |table: &str| -> Result<u64> {
        let n: i64 =
            conn.query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |r| r.get(0))?;
        Ok(n as u64)
    };
    Ok(RegistryCounts {
        keywords: count_of("keywords")?,
        keyword_synonyms: count_of("keyword_synonyms")?,
        clause_shapes: count_of("clause_shapes")?,
        quality_names: count_of("quality_names")?,
        kinds: count_of("kinds")?,
        patterns: count_of("patterns")?,
    })
}

/// Look up the canonical keyword for a surface token. Returns `Ok(None)` if
/// the token is not registered as a synonym (callers treat as unchanged).
/// Returns `Ok(Some(canonical))` if a row maps the surface token to a
/// canonical keyword. Errors only on SQL failure.
///
/// This is the read API S1 (tokenize) calls during synonym resolution.
pub fn resolve_synonym(conn: &Connection, surface: &str) -> Result<Option<String>> {
    let row = conn
        .query_row(
            "SELECT canonical_keyword FROM keyword_synonyms WHERE synonym = ?1",
            params![surface],
            |r| r.get::<_, String>(0),
        );
    match row {
        Ok(canonical) => Ok(Some(canonical)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Returns true if the given kind name has a row in the `kinds` table.
/// This is the read API S2 (kind_classify) calls during the strict
/// kind-validation pass.
///
/// An empty `kinds` table → every kind check returns false → S2 will
/// reject every block, surfacing the empty-registry condition rather
/// than silently passing.
pub fn is_known_kind(conn: &Connection, kind: &str) -> Result<bool> {
    let row: Result<i64, rusqlite::Error> = conn.query_row(
        "SELECT 1 FROM kinds WHERE name = ?1",
        params![kind],
        |r| r.get(0),
    );
    match row {
        Ok(_) => Ok(true),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// Returns the count of rows in the `kinds` table. Useful for the
/// schema-completeness proof — an empty table means S2 cannot accept
/// any source.
pub fn kinds_row_count(conn: &Connection) -> Result<u64> {
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM kinds", [], |r| r.get(0))?;
    Ok(n as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn init_creates_schema_and_stamps_version() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("grammar.sqlite");
        let conn = init_at(&db).unwrap();
        assert_eq!(schema_version(&conn).unwrap(), SCHEMA_VERSION);
    }

    #[test]
    fn init_is_idempotent() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("grammar.sqlite");
        let _ = init_at(&db).unwrap();
        let _ = init_at(&db).unwrap();
        let conn = open_readonly(&db).unwrap();
        assert_eq!(schema_version(&conn).unwrap(), SCHEMA_VERSION);
    }

    #[test]
    fn fresh_registry_has_zero_rows_in_every_table() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("grammar.sqlite");
        let conn = init_at(&db).unwrap();
        let c = counts(&conn).unwrap();
        assert_eq!(
            c,
            RegistryCounts {
                keywords: 0,
                keyword_synonyms: 0,
                clause_shapes: 0,
                quality_names: 0,
                kinds: 0,
                patterns: 0,
            }
        );
    }

    #[test]
    fn open_readonly_errors_when_missing() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("nonexistent.sqlite");
        assert!(open_readonly(&db).is_err());
    }

    #[test]
    fn keyword_synonyms_table_exists_after_init() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("g.sqlite");
        let conn = init_at(&db).unwrap();
        // Schema fragment query: confirm table is present in sqlite_master
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'keyword_synonyms'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);
        assert!(exists);
        // Confirm the canonical-keyword index is also present
        let idx_exists: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = 'idx_keyword_synonyms_canonical'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);
        assert!(idx_exists);
    }

    #[test]
    fn resolve_synonym_returns_none_for_unregistered_surface() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("g.sqlite");
        let conn = init_at(&db).unwrap();
        assert_eq!(resolve_synonym(&conn, "expects").unwrap(), None);
    }

    #[test]
    fn resolve_synonym_returns_canonical_when_row_exists() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("g.sqlite");
        let conn = init_at(&db).unwrap();
        conn.execute(
            "INSERT INTO keyword_synonyms (synonym, canonical_keyword, source_ref, shipped_commit, notes) \
             VALUES ('expects', 'requires', 'phaseA-test', 'test', NULL)",
            [],
        )
        .unwrap();
        assert_eq!(
            resolve_synonym(&conn, "expects").unwrap(),
            Some("requires".to_string())
        );
        // Other surface tokens still resolve to None
        assert_eq!(resolve_synonym(&conn, "demands").unwrap(), None);
    }

    #[test]
    fn synonym_round_trip_via_inserts_and_deletes() {
        // P5 proof — pure DB round-trip without any grammar pipeline call.
        let dir = tempdir().unwrap();
        let db = dir.path().join("g.sqlite");
        let conn = init_at(&db).unwrap();

        // Step 1: empty synonym table → no resolution
        assert_eq!(resolve_synonym(&conn, "assumes").unwrap(), None);

        // Step 2: insert a synonym row → resolution succeeds
        conn.execute(
            "INSERT INTO keyword_synonyms (synonym, canonical_keyword, source_ref, shipped_commit, notes) \
             VALUES ('assumes', 'requires', 'P5-round-trip', 'test', NULL)",
            [],
        )
        .unwrap();
        assert_eq!(
            resolve_synonym(&conn, "assumes").unwrap(),
            Some("requires".to_string())
        );

        // Step 3: delete the row → resolution returns None again
        conn.execute(
            "DELETE FROM keyword_synonyms WHERE synonym = 'assumes'",
            [],
        )
        .unwrap();
        assert_eq!(resolve_synonym(&conn, "assumes").unwrap(), None);
    }

    #[test]
    fn is_known_kind_returns_false_when_table_empty() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("g.sqlite");
        let conn = init_at(&db).unwrap();
        assert!(!is_known_kind(&conn, "function").unwrap());
        assert!(!is_known_kind(&conn, "anything").unwrap());
        assert_eq!(kinds_row_count(&conn).unwrap(), 0);
    }

    #[test]
    fn is_known_kind_returns_true_after_row_inserted() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("g.sqlite");
        let conn = init_at(&db).unwrap();
        conn.execute(
            "INSERT INTO kinds (name, description, allowed_clauses, allowed_refs, shipped_commit, notes) \
             VALUES ('function', 'a verb', '[]', '[]', 'phaseB-test', NULL)",
            [],
        )
        .unwrap();
        assert!(is_known_kind(&conn, "function").unwrap());
        assert!(!is_known_kind(&conn, "concept").unwrap()); // unrelated row absent
        assert_eq!(kinds_row_count(&conn).unwrap(), 1);
    }

    #[test]
    fn counts_includes_keyword_synonyms_field() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("g.sqlite");
        let conn = init_at(&db).unwrap();
        conn.execute(
            "INSERT INTO keyword_synonyms (synonym, canonical_keyword, source_ref, shipped_commit, notes) \
             VALUES ('demands', 'requires', 'count-test', 'test', NULL)",
            [],
        )
        .unwrap();
        let c = counts(&conn).unwrap();
        assert_eq!(c.keyword_synonyms, 1);
        assert_eq!(c.keywords, 0); // unchanged
    }
}
