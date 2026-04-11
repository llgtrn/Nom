//! nom-resolver: Resolves NomRef words against the nomdict SQLite database.
//!
//! Everything is `.nomtu` — one table, one format, one truth.
//!
//! The nomtu table unifies word metadata, contract info, scores,
//! provenance, and implementation body in a single row.
//!
//! Schema (auto-created if missing):
//! ```sql
//! CREATE TABLE nomtu (
//!     id            INTEGER PRIMARY KEY AUTOINCREMENT,
//!     word          TEXT NOT NULL,
//!     variant       TEXT,
//!     hash          TEXT,
//!     describe      TEXT,
//!     kind          TEXT,
//!     input_type    TEXT,
//!     output_type   TEXT,
//!     effects       TEXT DEFAULT '[]',
//!     pre           TEXT,
//!     post          TEXT,
//!     security      REAL DEFAULT 0.0,
//!     performance   REAL DEFAULT 0.0,
//!     quality       REAL DEFAULT 0.0,
//!     reliability   REAL DEFAULT 0.0,
//!     source        TEXT,
//!     source_path   TEXT,
//!     language      TEXT DEFAULT 'rust',
//!     license       TEXT,
//!     body          TEXT,
//!     signature     TEXT,
//!     version       TEXT,
//!     tests         INTEGER DEFAULT 0,
//!     is_canonical  BOOLEAN DEFAULT 0,
//!     created_at    TEXT DEFAULT (datetime('now')),
//!     UNIQUE(word, variant, language)
//! );
//! ```

use nom_ast::NomRef;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResolverError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("word not found: {word}{variant}", variant = .variant.as_deref().map(|v| format!("::{v}")).unwrap_or_default())]
    NotFound { word: String, variant: Option<String> },
    #[error("ambiguous reference: {word} matches {count} variants")]
    Ambiguous { word: String, count: usize },
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

/// A unified nomtu entry — identity, meaning, contract, scores,
/// provenance, and body all in one row. This IS the dictionary.
/// Everything that was an "atom" or a "word" or an "implementation"
/// is now a single .nomtu entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NomtuEntry {
    pub id: i64,
    // identity
    pub word: String,
    pub variant: Option<String>,
    pub hash: Option<String>,
    pub atom_id: Option<String>,       // original qualified path (e.g., "crates/foo/src/lib.rs:function:hash")
    // meaning
    pub describe: Option<String>,
    pub kind: Option<String>,
    pub labels: Vec<String>,           // classification tags (e.g., ["rust", "public", "async"])
    pub concept: Option<String>,       // semantic concept for pattern matching
    // contract
    pub input_type: Option<String>,
    pub output_type: Option<String>,
    pub effects: Vec<String>,
    pub pre: Option<String>,
    pub post: Option<String>,
    // scores
    pub security: f64,
    pub performance: f64,
    pub quality: f64,
    pub reliability: f64,
    // provenance
    pub source: Option<String>,
    pub source_path: Option<String>,   // file:line in the external repo
    pub language: String,
    pub license: Option<String>,
    // body (the actual code copied from external repos)
    pub body: Option<String>,
    pub signature: Option<String>,
    // meta
    pub version: Option<String>,
    pub tests: i64,
    pub is_canonical: bool,
}

impl Default for NomtuEntry {
    fn default() -> Self {
        Self {
            id: 0,
            word: String::new(),
            variant: None,
            hash: None,
            atom_id: None,
            describe: None,
            kind: None,
            labels: vec![],
            concept: None,
            input_type: None,
            output_type: None,
            effects: vec![],
            pre: None,
            post: None,
            security: 0.0,
            performance: 0.0,
            quality: 0.0,
            reliability: 0.0,
            source: None,
            source_path: None,
            language: "rust".to_owned(),
            license: None,
            body: None,
            signature: None,
            version: None,
            tests: 0,
            is_canonical: false,
        }
    }
}

impl NomtuEntry {
    /// Returns true if this entry satisfies a named score threshold.
    pub fn satisfies_score(&self, metric: &str, threshold: f64) -> bool {
        let value = match metric {
            "security" => self.security,
            "performance" => self.performance,
            "quality" => self.quality,
            "reliability" => self.reliability,
            _ => 0.0,
        };
        value >= threshold
    }
}

/// Backward-compatible alias — all code that used `WordEntry` still compiles.
pub type WordEntry = NomtuEntry;

/// Backward-compatible alias — no more separate impl type.
pub type ImplEntry = NomtuEntry;

/// The resolver opens (and initialises) a nomdict SQLite database.
pub struct Resolver {
    conn: Connection,
}

impl Resolver {
    /// Open or create a nomdict database at the given path.
    pub fn open(path: &str) -> Result<Self, ResolverError> {
        let conn = Connection::open(path)?;
        let resolver = Self { conn };
        resolver.init_schema()?;
        Ok(resolver)
    }

    /// Open an in-memory database (useful for tests).
    pub fn open_in_memory() -> Result<Self, ResolverError> {
        let conn = Connection::open_in_memory()?;
        let resolver = Self { conn };
        resolver.init_schema()?;
        Ok(resolver)
    }

    fn init_schema(&self) -> Result<(), ResolverError> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS nomtu (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                -- identity
                word          TEXT NOT NULL,
                variant       TEXT,
                hash          TEXT,
                atom_id       TEXT,
                -- meaning
                describe      TEXT,
                kind          TEXT,
                labels        TEXT DEFAULT '[]',
                concept       TEXT,
                -- contract
                input_type    TEXT,
                output_type   TEXT,
                effects       TEXT DEFAULT '[]',
                pre           TEXT,
                post          TEXT,
                -- scores
                security      REAL DEFAULT 0.0,
                performance   REAL DEFAULT 0.0,
                quality       REAL DEFAULT 0.0,
                reliability   REAL DEFAULT 0.0,
                -- provenance
                source        TEXT,
                source_path   TEXT,
                language      TEXT DEFAULT 'rust',
                license       TEXT,
                -- body (actual code from external repos)
                body          TEXT,
                signature     TEXT,
                -- meta
                version       TEXT,
                tests         INTEGER DEFAULT 0,
                is_canonical  BOOLEAN DEFAULT 0,
                created_at    TEXT DEFAULT (datetime('now')),
                UNIQUE(word, variant, language)
            );
            CREATE INDEX IF NOT EXISTS idx_nomtu_word ON nomtu(word);
            CREATE INDEX IF NOT EXISTS idx_nomtu_word_variant ON nomtu(word, variant);
            CREATE INDEX IF NOT EXISTS idx_nomtu_kind ON nomtu(kind);
            CREATE INDEX IF NOT EXISTS idx_nomtu_language ON nomtu(language);
            CREATE INDEX IF NOT EXISTS idx_nomtu_concept ON nomtu(concept);
            CREATE INDEX IF NOT EXISTS idx_nomtu_atom_id ON nomtu(atom_id);
            CREATE INDEX IF NOT EXISTS idx_nomtu_canonical ON nomtu(word, variant, is_canonical);",
        )?;
        Ok(())
    }

    /// Insert or replace a nomtu entry.
    pub fn upsert(&self, entry: &NomtuEntry) -> Result<(), ResolverError> {
        let effects_json = serde_json::to_string(&entry.effects)?;
        let labels_json = serde_json::to_string(&entry.labels)?;
        self.conn.execute(
            "INSERT INTO nomtu (word, variant, hash, atom_id, describe, kind, labels, concept,
                input_type, output_type, effects, pre, post,
                security, performance, quality, reliability,
                source, source_path, language, license,
                body, signature, version, tests, is_canonical)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26)
             ON CONFLICT(word, variant, language) DO UPDATE SET
                hash=excluded.hash,
                atom_id=excluded.atom_id,
                describe=excluded.describe,
                kind=excluded.kind,
                labels=excluded.labels,
                concept=excluded.concept,
                input_type=excluded.input_type,
                output_type=excluded.output_type,
                effects=excluded.effects,
                pre=excluded.pre,
                post=excluded.post,
                security=excluded.security,
                performance=excluded.performance,
                quality=excluded.quality,
                reliability=excluded.reliability,
                source=excluded.source,
                source_path=excluded.source_path,
                license=excluded.license,
                body=excluded.body,
                signature=excluded.signature,
                version=excluded.version,
                tests=excluded.tests,
                is_canonical=excluded.is_canonical",
            params![
                entry.word,
                entry.variant,
                entry.hash,
                entry.atom_id,
                entry.describe,
                entry.kind,
                labels_json,
                entry.concept,
                entry.input_type,
                entry.output_type,
                effects_json,
                entry.pre,
                entry.post,
                entry.security,
                entry.performance,
                entry.quality,
                entry.reliability,
                entry.source,
                entry.source_path,
                entry.language,
                entry.license,
                entry.body,
                entry.signature,
                entry.version,
                entry.tests,
                entry.is_canonical,
            ],
        )?;
        Ok(())
    }

    /// Backward-compatible alias for [`upsert`](Self::upsert).
    pub fn upsert_impl(&self, entry: &NomtuEntry) -> Result<(), ResolverError> {
        self.upsert(entry)
    }

    /// Resolve a [`NomRef`] to its database entry.
    ///
    /// Resolution order:
    /// 1. Exact (word, variant) match
    /// 2. Exact word match (any variant)
    /// 3. Semantic search by `describe` field (the word as a natural language query)
    ///
    /// Selection priority: `is_canonical` DESC, prefer `language='rust'`,
    /// then highest `quality`, then newest (highest `id`).
    pub fn resolve(&self, nom_ref: &NomRef) -> Result<NomtuEntry, ResolverError> {
        let word = &nom_ref.word.name;
        let variant = nom_ref.variant.as_ref().map(|v| v.name.as_str());

        if let Some(v) = variant {
            // Exact (word, variant) match, then semantic fallback
            match self.resolve_exact(word, Some(v)) {
                Ok(entry) => Ok(entry),
                Err(ResolverError::NotFound { .. }) => {
                    // Semantic fallback: search describe for "word variant"
                    let query = format!("{word} {v}");
                    let results = self.search_by_describe(&query, 1)?;
                    results.into_iter().next().ok_or_else(|| ResolverError::NotFound {
                        word: word.to_owned(),
                        variant: Some(v.to_owned()),
                    })
                }
                Err(e) => Err(e),
            }
        } else {
            // Try exact match first (word with NULL variant = canonical)
            match self.resolve_exact(word, None) {
                Ok(entry) => Ok(entry),
                Err(ResolverError::NotFound { .. }) => {
                    // Fall back: check if there's exactly one variant
                    let entries = self.resolve_all_variants(word)?;
                    match entries.len() {
                        0 => {
                            // Semantic fallback: search describe for the word
                            let results = self.search_by_describe(word, 1)?;
                            results.into_iter().next().ok_or_else(|| ResolverError::NotFound {
                                word: word.to_owned(),
                                variant: None,
                            })
                        }
                        1 => Ok(entries.into_iter().next().unwrap()),
                        n => Err(ResolverError::Ambiguous {
                            word: word.to_owned(),
                            count: n,
                        }),
                    }
                }
                Err(e) => Err(e),
            }
        }
    }

    /// Resolve by exact (word, variant) pair.
    ///
    /// When multiple language rows exist for the same (word, variant),
    /// prefers canonical, then Rust, then highest quality.
    pub fn resolve_exact(&self, word: &str, variant: Option<&str>) -> Result<NomtuEntry, ResolverError> {
        let row = if let Some(v) = variant {
            self.conn.query_row(
                &format!("{SELECT_COLS} FROM nomtu WHERE word=?1 AND variant=?2
                 ORDER BY {ORDER_CLAUSE} LIMIT 1"),
                params![word, v],
                Self::row_to_entry,
            ).optional()?
        } else {
            self.conn.query_row(
                &format!("{SELECT_COLS} FROM nomtu WHERE word=?1 AND variant IS NULL
                 ORDER BY {ORDER_CLAUSE} LIMIT 1"),
                params![word],
                Self::row_to_entry,
            ).optional()?
        };

        row.ok_or_else(|| ResolverError::NotFound {
            word: word.to_owned(),
            variant: variant.map(|v| v.to_owned()),
        })
    }

    /// Get all variants for a word (best row per variant).
    pub fn resolve_all_variants(&self, word: &str) -> Result<Vec<NomtuEntry>, ResolverError> {
        let mut stmt = self.conn.prepare(
            &format!("{SELECT_COLS} FROM nomtu WHERE word=?1
             ORDER BY {ORDER_CLAUSE}"),
        )?;
        let entries = stmt
            .query_map(params![word], Self::row_to_entry)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(entries)
    }

    /// Semantic search: find entries whose describe column matches a query string.
    pub fn search_by_describe(&self, query: &str, limit: usize) -> Result<Vec<NomtuEntry>, ResolverError> {
        let pattern = format!("%{query}%");
        let mut stmt = self.conn.prepare(
            &format!("{SELECT_COLS} FROM nomtu WHERE describe LIKE ?1
             ORDER BY {ORDER_CLAUSE} LIMIT ?2"),
        )?;
        let entries = stmt
            .query_map(params![pattern, limit as i64], Self::row_to_entry)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(entries)
    }

    // ── Body / implementation queries ────────────────────────────────

    /// Get the best implementation body for a word.
    ///
    /// Selection: prefer `is_canonical=true`, then `language='rust'`,
    /// then highest `quality`, then newest.
    pub fn get_body(&self, word: &str, variant: Option<&str>) -> Result<Option<NomtuEntry>, ResolverError> {
        // 1. Try exact (word, variant) match with body
        if let Some(v) = variant {
            let sql = format!("{SELECT_COLS} FROM nomtu WHERE word=?1 AND variant=?2 AND body IS NOT NULL AND length(body) > 0
                 ORDER BY {ORDER_CLAUSE} LIMIT 1");
            if let Some(entry) = self.conn.query_row(&sql, params![word, v], Self::row_to_entry).optional()? {
                return Ok(Some(entry));
            }
        }

        // 2. Fallback: ANY entry for this word with a body (pick best by quality)
        let sql = format!("{SELECT_COLS} FROM nomtu WHERE word=?1 AND body IS NOT NULL AND length(body) > 0
             ORDER BY {ORDER_CLAUSE} LIMIT 1");
        if let Some(entry) = self.conn.query_row(&sql, params![word], Self::row_to_entry).optional()? {
            return Ok(Some(entry));
        }

        // 3. Semantic fallback: search describe for the word
        let pattern = format!("%{word}%");
        let sql = format!("{SELECT_COLS} FROM nomtu WHERE describe LIKE ?1 AND body IS NOT NULL AND length(body) > 0
             ORDER BY {ORDER_CLAUSE} LIMIT 1");
        let result = self.conn.query_row(&sql, params![pattern], Self::row_to_entry).optional()?;
        Ok(result)
    }

    /// Backward-compatible alias for [`get_body`](Self::get_body).
    pub fn get_impl(&self, word: &str, variant: Option<&str>) -> Result<Option<NomtuEntry>, ResolverError> {
        self.get_body(word, variant)
    }

    /// Get all language variants for a word.
    pub fn get_all_variants(&self, word: &str, variant: Option<&str>) -> Result<Vec<NomtuEntry>, ResolverError> {
        let sql = if variant.is_some() {
            format!("{SELECT_COLS} FROM nomtu WHERE word=?1 AND variant=?2
             ORDER BY quality DESC, id DESC")
        } else {
            format!("{SELECT_COLS} FROM nomtu WHERE word=?1 AND variant IS NULL
             ORDER BY quality DESC, id DESC")
        };

        let mut stmt = self.conn.prepare(&sql)?;
        let entries = if let Some(v) = variant {
            stmt.query_map(params![word, v], Self::row_to_entry)?
                .collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map(params![word], Self::row_to_entry)?
                .collect::<Result<Vec<_>, _>>()?
        };
        Ok(entries)
    }

    /// Backward-compatible alias for [`get_all_variants`](Self::get_all_variants).
    pub fn get_all_impls(&self, word: &str, variant: Option<&str>) -> Result<Vec<NomtuEntry>, ResolverError> {
        self.get_all_variants(word, variant)
    }

    /// Mark an entry as canonical (and unmark others for the same word+variant).
    pub fn set_canonical(&self, word: &str, variant: Option<&str>, language: &str) -> Result<(), ResolverError> {
        if let Some(v) = variant {
            self.conn.execute(
                "UPDATE nomtu SET is_canonical=0 WHERE word=?1 AND variant=?2",
                params![word, v],
            )?;
            self.conn.execute(
                "UPDATE nomtu SET is_canonical=1 WHERE word=?1 AND variant=?2 AND language=?3",
                params![word, v, language],
            )?;
        } else {
            self.conn.execute(
                "UPDATE nomtu SET is_canonical=0 WHERE word=?1 AND variant IS NULL",
                params![word],
            )?;
            self.conn.execute(
                "UPDATE nomtu SET is_canonical=1 WHERE word=?1 AND variant IS NULL AND language=?2",
                params![word, language],
            )?;
        }
        Ok(())
    }

    /// Get the entry for a specific language.
    pub fn get_impl_by_language(&self, word: &str, variant: Option<&str>, language: &str) -> Result<Option<NomtuEntry>, ResolverError> {
        let sql = if variant.is_some() {
            format!("{SELECT_COLS} FROM nomtu WHERE word=?1 AND variant=?2 AND language=?3
             ORDER BY quality DESC, id DESC LIMIT 1")
        } else {
            format!("{SELECT_COLS} FROM nomtu WHERE word=?1 AND variant IS NULL AND language=?2
             ORDER BY quality DESC, id DESC LIMIT 1")
        };

        let result = if let Some(v) = variant {
            self.conn.query_row(&sql, params![word, v, language], Self::row_to_entry).optional()?
        } else {
            self.conn.query_row(&sql, params![word, language], Self::row_to_entry).optional()?
        };
        Ok(result)
    }

    /// Convenience method to import a nomtu entry from an atom.
    #[allow(clippy::too_many_arguments)]
    pub fn import_nomtu(
        &self,
        word: &str,
        variant: Option<&str>,
        language: &str,
        body: &str,
        signature: Option<&str>,
        source_path: Option<&str>,
        hash: Option<&str>,
        quality: f64,
    ) -> Result<(), ResolverError> {
        let entry = NomtuEntry {
            word: word.to_owned(),
            variant: variant.map(|v| v.to_owned()),
            language: language.to_owned(),
            body: Some(body.to_owned()),
            signature: signature.map(|s| s.to_owned()),
            source_path: source_path.map(|s| s.to_owned()),
            hash: hash.map(|s| s.to_owned()),
            quality,
            ..NomtuEntry::default()
        };
        self.upsert(&entry)
    }

    /// Backward-compatible alias for [`import_nomtu`](Self::import_nomtu).
    #[allow(clippy::too_many_arguments)]
    pub fn import_atom(
        &self,
        word: &str,
        variant: Option<&str>,
        language: &str,
        body: &str,
        signature: Option<&str>,
        source_path: Option<&str>,
        hash: Option<&str>,
        quality: f64,
    ) -> Result<(), ResolverError> {
        self.import_nomtu(word, variant, language, body, signature, source_path, hash, quality)
    }

    fn row_to_entry(row: &rusqlite::Row) -> rusqlite::Result<NomtuEntry> {
        // Column order matches SELECT_COLS:
        //  0: id, 1: word, 2: variant, 3: hash, 4: atom_id,
        //  5: describe, 6: kind, 7: labels, 8: concept,
        //  9: input_type, 10: output_type, 11: effects, 12: pre, 13: post,
        // 14: security, 15: performance, 16: quality, 17: reliability,
        // 18: source, 19: source_path, 20: language, 21: license,
        // 22: body, 23: signature, 24: version, 25: tests, 26: is_canonical
        let labels_json: String = row.get(7).unwrap_or_else(|_| "[]".to_owned());
        let labels: Vec<String> = serde_json::from_str(&labels_json).unwrap_or_default();
        let effects_json: String = row.get(11).unwrap_or_else(|_| "[]".to_owned());
        let effects: Vec<String> = serde_json::from_str(&effects_json).unwrap_or_default();
        Ok(NomtuEntry {
            id: row.get(0)?,
            word: row.get(1)?,
            variant: row.get(2)?,
            hash: row.get(3)?,
            atom_id: row.get(4)?,
            describe: row.get(5)?,
            kind: row.get(6)?,
            labels,
            concept: row.get(8)?,
            input_type: row.get(9)?,
            output_type: row.get(10)?,
            effects,
            pre: row.get(12)?,
            post: row.get(13)?,
            security: row.get(14)?,
            performance: row.get(15)?,
            quality: row.get(16)?,
            reliability: row.get(17)?,
            source: row.get(18)?,
            source_path: row.get(19)?,
            language: row.get(20)?,
            license: row.get(21)?,
            body: row.get(22)?,
            signature: row.get(23)?,
            version: row.get(24)?,
            tests: row.get(25)?,
            is_canonical: row.get(26)?,
        })
    }
}

/// Column list for SELECT queries (must match `row_to_entry` field order).
const SELECT_COLS: &str =
    "SELECT id, word, variant, hash, atom_id, describe, kind, labels, concept, \
     input_type, output_type, effects, pre, post, \
     security, performance, quality, reliability, \
     source, source_path, language, license, \
     body, signature, version, tests, is_canonical";

/// Default ORDER BY clause: canonical first, then Rust, then quality, then newest.
const ORDER_CLAUSE: &str =
    "is_canonical DESC, \
     CASE WHEN language='rust' THEN 0 ELSE 1 END, \
     quality DESC, \
     id DESC";

#[cfg(test)]
mod tests {
    use super::*;
    use nom_ast::{Identifier, NomRef, Span};

    fn dummy_span() -> Span {
        Span::new(0, 1, 1, 1)
    }

    fn make_ref(word: &str, variant: Option<&str>) -> NomRef {
        NomRef {
            word: Identifier::new(word, dummy_span()),
            variant: variant.map(|v| Identifier::new(v, dummy_span())),
            span: dummy_span(),
        }
    }

    fn sample_entry(word: &str, variant: Option<&str>) -> NomtuEntry {
        NomtuEntry {
            word: word.to_owned(),
            variant: variant.map(|v| v.to_owned()),
            describe: Some("a hashing function".to_owned()),
            input_type: Some("bytes".to_owned()),
            output_type: Some("hash".to_owned()),
            security: 0.95,
            performance: 0.7,
            reliability: 0.99,
            hash: Some("abc123".to_owned()),
            ..NomtuEntry::default()
        }
    }

    #[test]
    fn round_trip() {
        let resolver = Resolver::open_in_memory().unwrap();
        let entry = sample_entry("hash", Some("argon2"));
        resolver.upsert(&entry).unwrap();

        let nom_ref = make_ref("hash", Some("argon2"));
        let found = resolver.resolve(&nom_ref).unwrap();
        assert_eq!(found.word, "hash");
        assert_eq!(found.variant.as_deref(), Some("argon2"));
        assert!((found.security - 0.95).abs() < 1e-9);
    }

    #[test]
    fn not_found_error() {
        let resolver = Resolver::open_in_memory().unwrap();
        let nom_ref = make_ref("missing", None);
        assert!(matches!(resolver.resolve(&nom_ref), Err(ResolverError::NotFound { .. })));
    }

    #[test]
    fn semantic_search() {
        let resolver = Resolver::open_in_memory().unwrap();
        resolver.upsert(&sample_entry("hash", Some("argon2"))).unwrap();
        let results = resolver.search_by_describe("hashing", 10).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn satisfies_score() {
        let entry = sample_entry("hash", None);
        assert!(entry.satisfies_score("security", 0.9));
        assert!(!entry.satisfies_score("security", 0.99));
        assert!(entry.satisfies_score("reliability", 0.9));
    }

    // ── Unified table tests ─────────────────────────────────────────

    #[test]
    fn upsert_and_get_body() {
        let resolver = Resolver::open_in_memory().unwrap();
        let entry = NomtuEntry {
            word: "hash".to_owned(),
            variant: Some("sha256".to_owned()),
            language: "rust".to_owned(),
            body: Some("fn hash(data: &[u8]) -> [u8; 32] { todo!() }".to_owned()),
            signature: Some(r#"{"inputs":["bytes"],"outputs":["hash"],"effects":[]}"#.to_owned()),
            source_path: Some("crypto/src/hash.rs".to_owned()),
            hash: Some("abc123".to_owned()),
            quality: 0.85,
            ..NomtuEntry::default()
        };
        resolver.upsert(&entry).unwrap();

        let found = resolver.get_body("hash", Some("sha256")).unwrap().unwrap();
        assert_eq!(found.word, "hash");
        assert_eq!(found.variant.as_deref(), Some("sha256"));
        assert_eq!(found.language, "rust");
        assert!(found.body.as_deref().unwrap().contains("fn hash"));
        assert!((found.quality - 0.85).abs() < 1e-9);
    }

    #[test]
    fn canonical_prefers_rust() {
        let resolver = Resolver::open_in_memory().unwrap();

        // Insert a high-quality Python impl
        resolver.import_nomtu("sort", None, "python", "def sort(lst): return sorted(lst)", None, None, Some("py1"), 0.95).unwrap();
        // Insert a lower-quality Rust impl
        resolver.import_nomtu("sort", None, "rust", "fn sort(v: &mut Vec<i32>) { v.sort(); }", None, None, Some("rs1"), 0.70).unwrap();

        // get_body should prefer Rust even though Python has higher quality
        let found = resolver.get_body("sort", None).unwrap().unwrap();
        assert_eq!(found.language, "rust");
    }

    #[test]
    fn canonical_flag_overrides_language() {
        let resolver = Resolver::open_in_memory().unwrap();

        resolver.import_nomtu("sort", None, "rust", "fn sort() {}", None, None, Some("rs1"), 0.70).unwrap();
        resolver.import_nomtu("sort", None, "python", "def sort(): pass", None, None, Some("py1"), 0.95).unwrap();

        // Mark Python as canonical
        resolver.set_canonical("sort", None, "python").unwrap();

        let found = resolver.get_body("sort", None).unwrap().unwrap();
        assert_eq!(found.language, "python");
        assert!(found.is_canonical);
    }

    #[test]
    fn import_nomtu_convenience() {
        let resolver = Resolver::open_in_memory().unwrap();
        resolver.import_nomtu(
            "encrypt", Some("aes"), "rust",
            "fn encrypt(key: &[u8], data: &[u8]) -> Vec<u8> { todo!() }",
            Some(r#"{"inputs":["key","data"],"outputs":["ciphertext"],"effects":["crypto"]}"#),
            Some("crypto/src/aes.rs"),
            Some("hash456"),
            0.90,
        ).unwrap();

        let found = resolver.get_body("encrypt", Some("aes")).unwrap().unwrap();
        assert_eq!(found.language, "rust");
        assert!(found.body.as_deref().unwrap().contains("encrypt"));
        assert_eq!(found.source_path.as_deref(), Some("crypto/src/aes.rs"));
        assert_eq!(found.hash.as_deref(), Some("hash456"));
    }

    #[test]
    fn get_all_variants_returns_all_languages() {
        let resolver = Resolver::open_in_memory().unwrap();

        resolver.import_nomtu("parse", None, "rust", "fn parse() {}", None, None, Some("rs1"), 0.80).unwrap();
        resolver.import_nomtu("parse", None, "python", "def parse(): pass", None, None, Some("py1"), 0.75).unwrap();
        resolver.import_nomtu("parse", None, "go", "func parse() {}", None, None, Some("go1"), 0.70).unwrap();

        let all = resolver.get_all_variants("parse", None).unwrap();
        assert_eq!(all.len(), 3);

        let languages: Vec<&str> = all.iter().map(|e| e.language.as_str()).collect();
        assert!(languages.contains(&"rust"));
        assert!(languages.contains(&"python"));
        assert!(languages.contains(&"go"));
    }

    #[test]
    fn get_by_language() {
        let resolver = Resolver::open_in_memory().unwrap();
        resolver.import_nomtu("fmt", None, "rust", "fn fmt() {}", None, None, Some("rs1"), 0.80).unwrap();
        resolver.import_nomtu("fmt", None, "go", "func fmt() {}", None, None, Some("go1"), 0.90).unwrap();

        let go_impl = resolver.get_impl_by_language("fmt", None, "go").unwrap().unwrap();
        assert_eq!(go_impl.language, "go");

        let missing = resolver.get_impl_by_language("fmt", None, "python").unwrap();
        assert!(missing.is_none());
    }

    // ── Backward compatibility aliases ──────────────────────────────

    #[test]
    fn backward_compat_get_impl() {
        let resolver = Resolver::open_in_memory().unwrap();
        resolver.import_atom("sort", None, "rust", "fn sort() {}", None, None, Some("rs1"), 0.80).unwrap();
        let found = resolver.get_impl("sort", None).unwrap().unwrap();
        assert_eq!(found.language, "rust");
    }

    #[test]
    fn backward_compat_get_all_impls() {
        let resolver = Resolver::open_in_memory().unwrap();
        resolver.import_atom("x", None, "rust", "fn x() {}", None, None, Some("r1"), 0.8).unwrap();
        resolver.import_atom("x", None, "go", "func x() {}", None, None, Some("g1"), 0.7).unwrap();
        let all = resolver.get_all_impls("x", None).unwrap();
        assert_eq!(all.len(), 2);
    }
}
