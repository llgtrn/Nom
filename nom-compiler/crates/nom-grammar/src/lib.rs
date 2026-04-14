//! nom-grammar — AI-retrievable basic-syntax registry for Nom.
//!
//! Per doc 21 (research/language-analysis/21-grammar-registry-design.md), this crate owns
//! `grammar.sqlite` — a machine-readable projection of everything the Nom parser accepts,
//! so external AI clients can deterministically query keywords, clause shapes, paradigm
//! mappings, QualityNames, and the closed kind set without reading .md prose.
//!
//! P1 scope: schema-only. Later phases populate from nom-concept + doc 16.

pub mod seed;

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

CREATE TABLE IF NOT EXISTS authoring_rules (
  row_id          INTEGER PRIMARY KEY,
  source_paradigm TEXT NOT NULL,
  gap_summary     TEXT NOT NULL,
  nom_shape       TEXT NOT NULL,
  reuses_rows     TEXT,
  destination     TEXT NOT NULL,
  status          TEXT NOT NULL,
  closed_in       TEXT,
  source_doc_ref  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_authoring_rules_paradigm
  ON authoring_rules(source_paradigm);

CREATE TABLE IF NOT EXISTS quality_names (
  name            TEXT PRIMARY KEY,
  axis            TEXT NOT NULL,
  metric_function TEXT NOT NULL,
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
    pub clause_shapes: u64,
    pub authoring_rules: u64,
    pub quality_names: u64,
    pub kinds: u64,
}

pub fn counts(conn: &Connection) -> Result<RegistryCounts> {
    let count_of = |table: &str| -> Result<u64> {
        let n: i64 =
            conn.query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |r| r.get(0))?;
        Ok(n as u64)
    };
    Ok(RegistryCounts {
        keywords: count_of("keywords")?,
        clause_shapes: count_of("clause_shapes")?,
        authoring_rules: count_of("authoring_rules")?,
        quality_names: count_of("quality_names")?,
        kinds: count_of("kinds")?,
    })
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
                clause_shapes: 0,
                authoring_rules: 0,
                quality_names: 0,
                kinds: 0,
            }
        );
    }

    #[test]
    fn open_readonly_errors_when_missing() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("nonexistent.sqlite");
        assert!(open_readonly(&db).is_err());
    }
}
