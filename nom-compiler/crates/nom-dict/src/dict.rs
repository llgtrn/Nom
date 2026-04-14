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
//! S2 will bring in the properly-split schemas; for now both files get the full
//! `V2_SCHEMA_SQL` to keep the surface obvious. S3 will port every function that
//! takes `&Connection` to take `&Dict` (and route to the right side).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::V2_SCHEMA_SQL;

pub const CONCEPTS_FILENAME: &str = "concepts.sqlite";
pub const WORDS_FILENAME: &str = "words.sqlite";

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
    /// Applies `V2_SCHEMA_SQL` to both files (S2 will specialise per tier).
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
        let concepts = open_tier(concepts_path, "concepts")?;
        let words = open_tier(words_path, "words")?;
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
        concepts.execute_batch(V2_SCHEMA_SQL)?;
        words.execute_batch(V2_SCHEMA_SQL)?;
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

fn open_tier(path: &Path, label: &str) -> Result<Connection> {
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
    conn.execute_batch(V2_SCHEMA_SQL)
        .with_context(|| format!("applying V2_SCHEMA_SQL to {} tier", label))?;
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

    #[test]
    fn v2_schema_applied_to_both_tiers() {
        let d = Dict::open_in_memory().unwrap();
        // The `entries` table exists in V2_SCHEMA_SQL; query its row count on both.
        let c_rows: i64 = d
            .concepts
            .query_row("SELECT COUNT(*) FROM entries", [], |r| r.get(0))
            .unwrap();
        let w_rows: i64 = d
            .words
            .query_row("SELECT COUNT(*) FROM entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!(c_rows, 0);
        assert_eq!(w_rows, 0);
    }

    #[test]
    fn root_reflects_directory_form() {
        let dir = tempdir().unwrap();
        let d = Dict::open_dir(dir.path()).unwrap();
        assert_eq!(d.root(), dir.path());
    }
}
