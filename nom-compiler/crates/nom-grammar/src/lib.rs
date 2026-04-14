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

/// Returns the count of `clause_shapes` rows for the given kind.
/// S3 calls this to detect the empty-registry condition: a kind with
/// zero clause_shapes rows means the user has not declared the per-
/// kind grammar surface, so the parser cannot validate the block's
/// shape.
pub fn clause_shapes_row_count_for_kind(conn: &Connection, kind: &str) -> Result<u64> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM clause_shapes WHERE kind = ?1",
        params![kind],
        |r| r.get(0),
    )?;
    Ok(n as u64)
}

/// Returns the names of every clause where `is_required = 1` for the
/// given kind, ordered by `position`. S3 (or a future cross-stage
/// validator) calls this to assert that every required clause is
/// present in the block body.
///
/// `is_required` semantics: 0 = optional, 1 = required, 2 = required-
/// at-least-one-of (one_of_group). This helper returns only the
/// `is_required = 1` rows; the one-of-group case is a future helper.
pub fn required_clauses_for_kind(conn: &Connection, kind: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT clause_name FROM clause_shapes \
         WHERE kind = ?1 AND is_required = 1 \
         ORDER BY position",
    )?;
    let names = stmt
        .query_map(params![kind], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(names)
}

/// Returns true if the given quality name has a row in the
/// `quality_names` table. The S5b favor-validator calls this to
/// reject any `favor X` clause whose X is not registered.
pub fn is_known_quality(conn: &Connection, name: &str) -> Result<bool> {
    let row: Result<i64, rusqlite::Error> = conn.query_row(
        "SELECT 1 FROM quality_names WHERE name = ?1",
        params![name],
        |r| r.get(0),
    );
    match row {
        Ok(_) => Ok(true),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// Returns the count of rows in the `quality_names` table. The S5b
/// validator uses this to detect the empty-registry condition when
/// the source contains at least one `favor` clause.
pub fn quality_names_row_count(conn: &Connection) -> Result<u64> {
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM quality_names", [], |r| r.get(0))?;
    Ok(n as u64)
}

/// Stopwords stripped before [`fuzzy_tokens`] tokenization. Closed
/// list, never reordered, so the same input always produces the same
/// token set across runs and machines. Shared by the CLI's
/// `nom grammar pattern-search` and the CI test
/// `every_pattern_intent_pair_jaccard_below_threshold` so both use
/// the exact same backend.
pub const FUZZY_STOPWORDS: &[&str] = &[
    "a","the","of","to","and","or","with","for","in","on","as","an","is",
    "into","from","by","that","this","its","at","be","are","it","one","two",
    "each","every","any","all","no","not","then","than","only","also","same",
];

/// Tokenize a free-form intent string into a normalized set of domain
/// words for Jaccard-similarity comparison. Lowercase, alphabetic-only,
/// length ≥ 3, not in [`FUZZY_STOPWORDS`]. Deterministic — the same
/// input always produces the same set in the same order.
pub fn fuzzy_tokens(intent: &str) -> std::collections::BTreeSet<String> {
    let mut out = std::collections::BTreeSet::new();
    let mut cur = String::new();
    let lower = intent.to_lowercase();
    for ch in lower.chars().chain(std::iter::once(' ')) {
        if ch.is_ascii_alphabetic() {
            cur.push(ch);
        } else {
            if cur.len() >= 3 && !FUZZY_STOPWORDS.contains(&cur.as_str()) {
                out.insert(std::mem::take(&mut cur));
            } else {
                cur.clear();
            }
        }
    }
    out
}

/// Jaccard similarity between two token sets — `|a ∩ b| / |a ∪ b|`.
/// Returns `0.0` when either set is empty (avoids div-by-zero). Used
/// by the catalog uniqueness test and by `nom grammar pattern-search`.
pub fn jaccard(
    a: &std::collections::BTreeSet<String>,
    b: &std::collections::BTreeSet<String>,
) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 {
        return 0.0;
    }
    inter as f64 / union as f64
}

/// One row in a pattern-search result: similarity score, pattern id,
/// intent prose. Sorted by `score` descending then `pattern_id`
/// ascending — stable across runs and machines.
#[derive(Debug, Clone, PartialEq)]
pub struct PatternMatch {
    pub score: f64,
    pub pattern_id: String,
    pub intent: String,
}

/// Search the patterns table by free-form prose. Returns up to
/// `limit` matches whose Jaccard similarity (over [`fuzzy_tokens`]
/// of the query and each row's intent) is at least `threshold`,
/// sorted by score descending then `pattern_id` ascending.
///
/// This is the canonical backend the CLI's `nom grammar pattern-
/// search` calls; consumers (LSP, resolver, dream loop, tests) can
/// call it directly instead of duplicating the loop. Deterministic —
/// the same query against the same DB always returns the same Vec.
pub fn search_patterns(
    conn: &Connection,
    query: &str,
    threshold: f64,
    limit: usize,
) -> Result<Vec<PatternMatch>> {
    let q = fuzzy_tokens(query);
    if q.is_empty() {
        return Ok(Vec::new());
    }
    let mut stmt = conn
        .prepare("SELECT pattern_id, intent FROM patterns")
        .context("preparing pattern-search query")?;
    let rows: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .context("querying patterns")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("collecting pattern rows")?;
    let mut scored: Vec<PatternMatch> = rows
        .into_iter()
        .filter_map(|(id, intent)| {
            let row = fuzzy_tokens(&intent);
            let s = jaccard(&q, &row);
            if s >= threshold {
                Some(PatternMatch { score: s, pattern_id: id, intent })
            } else {
                None
            }
        })
        .collect();
    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.pattern_id.cmp(&b.pattern_id))
    });
    scored.truncate(limit);
    Ok(scored)
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
    fn clause_shapes_row_count_for_kind_returns_zero_when_empty() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("g.sqlite");
        let conn = init_at(&db).unwrap();
        assert_eq!(clause_shapes_row_count_for_kind(&conn, "function").unwrap(), 0);
    }

    #[test]
    fn clause_shapes_row_count_for_kind_counts_only_matching_kind() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("g.sqlite");
        let conn = init_at(&db).unwrap();
        // Insert two rows for function, one for property
        conn.execute(
            "INSERT INTO clause_shapes (kind, clause_name, is_required, position, grammar_shape, source_ref) \
             VALUES ('function', 'intended', 1, 1, 'intended to <prose>.', 'phaseB3-test')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO clause_shapes (kind, clause_name, is_required, position, grammar_shape, source_ref) \
             VALUES ('function', 'requires', 0, 2, 'requires <prose>.', 'phaseB3-test')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO clause_shapes (kind, clause_name, is_required, position, grammar_shape, source_ref) \
             VALUES ('property', 'generator', 1, 1, 'generator <prose>.', 'phaseB3-test')",
            [],
        ).unwrap();
        assert_eq!(clause_shapes_row_count_for_kind(&conn, "function").unwrap(), 2);
        assert_eq!(clause_shapes_row_count_for_kind(&conn, "property").unwrap(), 1);
        assert_eq!(clause_shapes_row_count_for_kind(&conn, "scenario").unwrap(), 0);
    }

    #[test]
    fn required_clauses_for_kind_returns_only_required_rows_in_position_order() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("g.sqlite");
        let conn = init_at(&db).unwrap();
        // Insert clauses for `scenario`: given/when/then required, favor optional
        for (clause, required, position) in [
            ("intended", 1, 1),
            ("given", 1, 2),
            ("when", 1, 3),
            ("then", 1, 4),
            ("favor", 0, 5),
        ] {
            conn.execute(
                "INSERT INTO clause_shapes (kind, clause_name, is_required, position, grammar_shape, source_ref) \
                 VALUES ('scenario', ?1, ?2, ?3, '...', 'phaseB3-test')",
                params![clause, required, position],
            ).unwrap();
        }
        let required = required_clauses_for_kind(&conn, "scenario").unwrap();
        assert_eq!(required, vec!["intended", "given", "when", "then"]);
        // Other kinds have no required clauses
        assert!(required_clauses_for_kind(&conn, "function").unwrap().is_empty());
    }

    #[test]
    fn is_known_quality_round_trip() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("g.sqlite");
        let conn = init_at(&db).unwrap();
        // Empty table — every name unknown
        assert!(!is_known_quality(&conn, "auditability").unwrap());
        assert_eq!(quality_names_row_count(&conn).unwrap(), 0);
        // Insert a row
        conn.execute(
            "INSERT INTO quality_names (name, axis, cardinality, source_ref) \
             VALUES ('auditability', 'ops', 'any', 'phaseB4-test')",
            [],
        )
        .unwrap();
        assert!(is_known_quality(&conn, "auditability").unwrap());
        assert!(!is_known_quality(&conn, "totality").unwrap()); // unrelated
        assert_eq!(quality_names_row_count(&conn).unwrap(), 1);
    }

    #[test]
    fn fuzzy_tokens_normalizes_lowercase_alpha_and_strips_stopwords() {
        let toks = fuzzy_tokens("The Quick brown-fox JUMPS over the lazy dog");
        // 'the' is a stopword; case-folded; non-alpha splits; len >= 3.
        assert!(toks.contains("quick"));
        assert!(toks.contains("brown"));
        assert!(toks.contains("fox"));
        assert!(toks.contains("jumps"));
        assert!(toks.contains("over"));
        assert!(toks.contains("lazy"));
        assert!(toks.contains("dog"));
        assert!(!toks.contains("the"));
    }

    #[test]
    fn search_patterns_basic_ordering_and_threshold() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("g.sqlite");
        let conn = init_at(&db).unwrap();
        // Seed three minimal patterns.
        for (id, intent) in [
            ("alpha-cache", "cache pure function results"),
            ("beta-supervise", "supervise child processes with restart"),
            ("gamma-unrelated", "render typeset glyphs along baseline"),
        ] {
            conn.execute(
                "INSERT INTO patterns \
                 (pattern_id, intent, nom_kinds, nom_clauses, typed_slot_refs, \
                  example_shape, hazards, favors, source_doc_refs) \
                 VALUES (?1, ?2, '[]', '[]', '[]', '', '[]', '[]', '[]')",
                rusqlite::params![id, intent],
            )
            .unwrap();
        }

        // Strong cache query → alpha-cache must be top hit.
        let hits = search_patterns(&conn, "cache pure function results", 0.1, 10).unwrap();
        assert!(!hits.is_empty());
        assert_eq!(hits[0].pattern_id, "alpha-cache");

        // Threshold rejects below: a query with zero overlap returns empty.
        let none = search_patterns(&conn, "moose elk antelope", 0.1, 10).unwrap();
        assert!(none.is_empty());

        // limit truncates.
        let one = search_patterns(&conn, "function results child render", 0.0001, 1).unwrap();
        assert_eq!(one.len(), 1);

        // empty query (all stopwords) returns empty.
        let empty = search_patterns(&conn, "the of a to", 0.0, 10).unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn jaccard_known_values() {
        let a = fuzzy_tokens("cache pure function results");
        let b = fuzzy_tokens("cache pure function results");
        assert!((jaccard(&a, &b) - 1.0).abs() < 1e-9);

        let c = fuzzy_tokens("totally unrelated subject matter here");
        let j_ac = jaccard(&a, &c);
        assert!(j_ac >= 0.0 && j_ac < 0.2, "got {j_ac}");

        // Empty side returns 0.
        let empty = std::collections::BTreeSet::new();
        assert_eq!(jaccard(&empty, &a), 0.0);
        assert_eq!(jaccard(&a, &empty), 0.0);
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
