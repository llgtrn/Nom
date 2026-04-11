//! nom-resolver: Resolves NomRef words against the nomdict SQLite database.
//!
//! Everything is `.nomtu` — one table, one format, one truth.
//!
//! The nomtu table unifies word metadata, contract info, scores,
//! provenance, and implementation body in a single row.
//!
//! Schema (auto-created if missing): 48-column .nomtu format.
//! See `init_schema()` for the full CREATE TABLE statement.

use nom_ast::NomRef;
pub use nom_types::NomtuEntry;
use rusqlite::{Connection, OptionalExtension, params};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResolverError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("word not found: {word}{variant}", variant = .variant.as_deref().map(|v| format!("::{v}")).unwrap_or_default())]
    NotFound {
        word: String,
        variant: Option<String>,
    },
    #[error("ambiguous reference: {word} matches {count} variants")]
    Ambiguous { word: String, count: usize },
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
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
                id                  INTEGER PRIMARY KEY AUTOINCREMENT,
                -- identity
                word                TEXT NOT NULL,
                variant             TEXT,
                kind                TEXT NOT NULL DEFAULT '',
                hash                TEXT UNIQUE,
                body_hash           TEXT,
                -- meaning
                describe            TEXT,
                concept             TEXT,
                labels              TEXT DEFAULT '[]',
                -- contract
                input_type          TEXT,
                output_type         TEXT,
                effects             TEXT DEFAULT '[]',
                pre                 TEXT,
                post                TEXT,
                signature           TEXT,
                depends_on          TEXT DEFAULT '[]',
                -- scores (8 + overall)
                security            REAL DEFAULT 0.0,
                reliability         REAL DEFAULT 0.0,
                performance         REAL DEFAULT 0.0,
                readability         REAL DEFAULT 0.0,
                testability         REAL DEFAULT 0.0,
                portability         REAL DEFAULT 0.0,
                composability       REAL DEFAULT 0.0,
                maturity            REAL DEFAULT 0.0,
                overall_score       REAL DEFAULT 0.0,
                -- security audit
                audit_passed        BOOLEAN DEFAULT 0,
                audit_max_severity  TEXT,
                audit_findings      TEXT,
                -- provenance
                source_repo         TEXT,
                source_path         TEXT,
                source_line         INTEGER,
                source_commit       TEXT,
                author              TEXT,
                language            TEXT DEFAULT 'rust',
                -- body & translation
                body                TEXT,
                rust_body           TEXT,
                translate_confidence REAL,
                -- graph metadata
                community_id        TEXT,
                callers_count       INTEGER DEFAULT 0,
                callees_count       INTEGER DEFAULT 0,
                is_entry_point      BOOLEAN DEFAULT 0,
                -- precompiled artifacts
                bc_path             TEXT,
                bc_hash             TEXT,
                bc_size             INTEGER,
                -- agent metadata
                capabilities        TEXT,
                supervision         TEXT,
                schedule            TEXT,
                -- meta
                version             TEXT,
                tests               INTEGER DEFAULT 0,
                is_canonical        BOOLEAN DEFAULT 0,
                deprecated_by       TEXT,
                created_at          TEXT DEFAULT (datetime('now')),
                updated_at          TEXT,
                UNIQUE(word, variant, language)
            );
            CREATE INDEX IF NOT EXISTS idx_nomtu_word ON nomtu(word);
            CREATE INDEX IF NOT EXISTS idx_nomtu_word_variant ON nomtu(word, variant);
            CREATE INDEX IF NOT EXISTS idx_nomtu_kind ON nomtu(kind);
            CREATE INDEX IF NOT EXISTS idx_nomtu_language ON nomtu(language);
            CREATE INDEX IF NOT EXISTS idx_nomtu_concept ON nomtu(concept);
            CREATE INDEX IF NOT EXISTS idx_nomtu_hash ON nomtu(hash);
            CREATE INDEX IF NOT EXISTS idx_nomtu_source_repo ON nomtu(source_repo);
            CREATE INDEX IF NOT EXISTS idx_nomtu_overall_score ON nomtu(overall_score);
            CREATE INDEX IF NOT EXISTS idx_nomtu_community ON nomtu(community_id);
            CREATE INDEX IF NOT EXISTS idx_nomtu_canonical ON nomtu(word, variant, is_canonical);",
        )?;
        Ok(())
    }

    /// Insert or replace a nomtu entry.
    pub fn upsert(&self, entry: &NomtuEntry) -> Result<(), ResolverError> {
        let labels_json = serde_json::to_string(&entry.labels)?;
        let effects_json = serde_json::to_string(&entry.effects)?;
        let depends_on_json = serde_json::to_string(&entry.depends_on)?;
        self.conn.execute(
            "INSERT INTO nomtu (
                word, variant, kind, hash, body_hash,
                describe, concept, labels,
                input_type, output_type, effects, pre, post, signature, depends_on,
                security, reliability, performance,
                readability, testability, portability, composability, maturity, overall_score,
                audit_passed, audit_max_severity, audit_findings,
                source_repo, source_path, source_line, source_commit, author, language,
                body, rust_body, translate_confidence,
                community_id, callers_count, callees_count, is_entry_point,
                bc_path, bc_hash, bc_size,
                capabilities, supervision, schedule,
                version, tests, is_canonical, deprecated_by, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,
                     ?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,
                     ?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,
                     ?31,?32,?33,?34,?35,?36,?37,?38,?39,?40,
                     ?41,?42,?43,?44,?45,?46,?47,?48,?49,?50,?51)
             ON CONFLICT(word, variant, language) DO UPDATE SET
                kind=excluded.kind,
                hash=excluded.hash,
                body_hash=excluded.body_hash,
                describe=excluded.describe,
                concept=excluded.concept,
                labels=excluded.labels,
                input_type=excluded.input_type,
                output_type=excluded.output_type,
                effects=excluded.effects,
                pre=excluded.pre,
                post=excluded.post,
                signature=excluded.signature,
                depends_on=excluded.depends_on,
                security=excluded.security,
                reliability=excluded.reliability,
                performance=excluded.performance,
                readability=excluded.readability,
                testability=excluded.testability,
                portability=excluded.portability,
                composability=excluded.composability,
                maturity=excluded.maturity,
                overall_score=excluded.overall_score,
                audit_passed=excluded.audit_passed,
                audit_max_severity=excluded.audit_max_severity,
                audit_findings=excluded.audit_findings,
                source_repo=excluded.source_repo,
                source_path=excluded.source_path,
                source_line=excluded.source_line,
                source_commit=excluded.source_commit,
                author=excluded.author,
                body=excluded.body,
                rust_body=excluded.rust_body,
                translate_confidence=excluded.translate_confidence,
                community_id=excluded.community_id,
                callers_count=excluded.callers_count,
                callees_count=excluded.callees_count,
                is_entry_point=excluded.is_entry_point,
                bc_path=excluded.bc_path,
                bc_hash=excluded.bc_hash,
                bc_size=excluded.bc_size,
                capabilities=excluded.capabilities,
                supervision=excluded.supervision,
                schedule=excluded.schedule,
                version=excluded.version,
                tests=excluded.tests,
                is_canonical=excluded.is_canonical,
                deprecated_by=excluded.deprecated_by,
                updated_at=excluded.updated_at",
            params![
                entry.word,                 // 1
                entry.variant,              // 2
                entry.kind,                 // 3
                entry.hash,                 // 4
                entry.body_hash,            // 5
                entry.describe,             // 6
                entry.concept,              // 7
                labels_json,                // 8
                entry.input_type,           // 9
                entry.output_type,          // 10
                effects_json,               // 11
                entry.pre,                  // 12
                entry.post,                 // 13
                entry.signature,            // 14
                depends_on_json,            // 15
                entry.security,             // 16
                entry.reliability,          // 17
                entry.performance,          // 18
                entry.readability,          // 19
                entry.testability,          // 20
                entry.portability,          // 21
                entry.composability,        // 22
                entry.maturity,             // 23
                entry.overall_score,        // 24
                entry.audit_passed,         // 25
                entry.audit_max_severity,   // 26
                entry.audit_findings,       // 27
                entry.source_repo,          // 28
                entry.source_path,          // 29
                entry.source_line,          // 30
                entry.source_commit,        // 31
                entry.author,               // 32
                entry.language,             // 33
                entry.body,                 // 34
                entry.rust_body,            // 35
                entry.translate_confidence, // 36
                entry.community_id,         // 37
                entry.callers_count,        // 38
                entry.callees_count,        // 39
                entry.is_entry_point,       // 40
                entry.bc_path,              // 41
                entry.bc_hash,              // 42
                entry.bc_size,              // 43
                entry.capabilities,         // 44
                entry.supervision,          // 45
                entry.schedule,             // 46
                entry.version,              // 47
                entry.tests,                // 48
                entry.is_canonical,         // 49
                entry.deprecated_by,        // 50
                entry.updated_at,           // 51
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
    /// then highest `overall_score`, then newest (highest `id`).
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
                    results
                        .into_iter()
                        .next()
                        .ok_or_else(|| ResolverError::NotFound {
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
                            results
                                .into_iter()
                                .next()
                                .ok_or_else(|| ResolverError::NotFound {
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
    pub fn resolve_exact(
        &self,
        word: &str,
        variant: Option<&str>,
    ) -> Result<NomtuEntry, ResolverError> {
        let row = if let Some(v) = variant {
            self.conn
                .query_row(
                    &format!(
                        "{SELECT_COLS} FROM nomtu WHERE word=?1 AND variant=?2
                 ORDER BY {ORDER_CLAUSE} LIMIT 1"
                    ),
                    params![word, v],
                    Self::row_to_entry,
                )
                .optional()?
        } else {
            self.conn
                .query_row(
                    &format!(
                        "{SELECT_COLS} FROM nomtu WHERE word=?1 AND variant IS NULL
                 ORDER BY {ORDER_CLAUSE} LIMIT 1"
                    ),
                    params![word],
                    Self::row_to_entry,
                )
                .optional()?
        };

        row.ok_or_else(|| ResolverError::NotFound {
            word: word.to_owned(),
            variant: variant.map(|v| v.to_owned()),
        })
    }

    /// Get all variants for a word (best row per variant).
    pub fn resolve_all_variants(&self, word: &str) -> Result<Vec<NomtuEntry>, ResolverError> {
        let mut stmt = self.conn.prepare(&format!(
            "{SELECT_COLS} FROM nomtu WHERE word=?1
             ORDER BY {ORDER_CLAUSE}"
        ))?;
        let entries = stmt
            .query_map(params![word], Self::row_to_entry)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(entries)
    }

    /// Semantic search: find entries whose describe column matches a query string.
    pub fn search_by_describe(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<NomtuEntry>, ResolverError> {
        let pattern = format!("%{query}%");
        let mut stmt = self.conn.prepare(&format!(
            "{SELECT_COLS} FROM nomtu WHERE describe LIKE ?1
             ORDER BY {ORDER_CLAUSE} LIMIT ?2"
        ))?;
        let entries = stmt
            .query_map(params![pattern, limit as i64], Self::row_to_entry)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(entries)
    }

    /// Search by contract shape: find entries whose input/output types match a signature.
    ///
    /// Accepts signatures like:
    /// - `"in: bytes -> out: hash"`
    /// - `"bytes -> hash"`
    /// - `"-> hash"` (any input, output matches "hash")
    /// - `"bytes ->"` (input matches "bytes", any output)
    ///
    /// Uses SQL LIKE matching for flexibility (partial type names work).
    pub fn search_by_contract(
        &self,
        signature: &str,
        limit: usize,
    ) -> Result<Vec<NomtuEntry>, ResolverError> {
        let (input_type, output_type) = Self::parse_contract_signature(signature);

        let (sql, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) =
            match (input_type.as_deref(), output_type.as_deref()) {
                (Some(inp), Some(out)) => {
                    let inp_pat = format!("%{inp}%");
                    let out_pat = format!("%{out}%");
                    (
                        format!(
                            "{SELECT_COLS} FROM nomtu \
                             WHERE input_type LIKE ?1 AND output_type LIKE ?2 \
                             ORDER BY {ORDER_CLAUSE} LIMIT ?3"
                        ),
                        vec![
                            Box::new(inp_pat) as Box<dyn rusqlite::types::ToSql>,
                            Box::new(out_pat),
                            Box::new(limit as i64),
                        ],
                    )
                }
                (Some(inp), None) => {
                    let inp_pat = format!("%{inp}%");
                    (
                        format!(
                            "{SELECT_COLS} FROM nomtu \
                             WHERE input_type LIKE ?1 \
                             ORDER BY {ORDER_CLAUSE} LIMIT ?2"
                        ),
                        vec![
                            Box::new(inp_pat) as Box<dyn rusqlite::types::ToSql>,
                            Box::new(limit as i64),
                        ],
                    )
                }
                (None, Some(out)) => {
                    let out_pat = format!("%{out}%");
                    (
                        format!(
                            "{SELECT_COLS} FROM nomtu \
                             WHERE output_type LIKE ?1 \
                             ORDER BY {ORDER_CLAUSE} LIMIT ?2"
                        ),
                        vec![
                            Box::new(out_pat) as Box<dyn rusqlite::types::ToSql>,
                            Box::new(limit as i64),
                        ],
                    )
                }
                (None, None) => {
                    // No useful filter — return empty
                    return Ok(vec![]);
                }
            };

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|b| b.as_ref()).collect();
        let mut stmt = self.conn.prepare(&sql)?;
        let entries = stmt
            .query_map(params_refs.as_slice(), Self::row_to_entry)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(entries)
    }

    /// Parse a contract signature string into (input_type, output_type).
    ///
    /// Supported formats:
    /// - `"in: bytes -> out: hash"` => (Some("bytes"), Some("hash"))
    /// - `"bytes -> hash"`          => (Some("bytes"), Some("hash"))
    /// - `"-> hash"`                => (None, Some("hash"))
    /// - `"bytes ->"`               => (Some("bytes"), None)
    fn parse_contract_signature(sig: &str) -> (Option<String>, Option<String>) {
        let sig = sig.trim();

        if let Some((left, right)) = sig.split_once("->") {
            let input = left.trim().strip_prefix("in:").unwrap_or(left.trim()).trim();
            let output = right
                .trim()
                .strip_prefix("out:")
                .unwrap_or(right.trim())
                .trim();

            let input = if input.is_empty() { None } else { Some(input.to_owned()) };
            let output = if output.is_empty() {
                None
            } else {
                Some(output.to_owned())
            };

            (input, output)
        } else {
            // No arrow — treat entire string as keyword matching both columns
            let s = sig.strip_prefix("in:").unwrap_or(sig).trim();
            if s.is_empty() {
                (None, None)
            } else {
                (Some(s.to_owned()), Some(s.to_owned()))
            }
        }
    }

    // ── Body / implementation queries ────────────────────────────────

    /// Get the best implementation body for a word.
    ///
    /// Selection: prefer `is_canonical=true`, then `language='rust'`,
    /// then highest `overall_score`, then newest.
    pub fn get_body(
        &self,
        word: &str,
        variant: Option<&str>,
    ) -> Result<Option<NomtuEntry>, ResolverError> {
        // 1. Try exact (word, variant) match with body
        if let Some(v) = variant {
            let sql = format!("{SELECT_COLS} FROM nomtu WHERE word=?1 AND variant=?2 AND body IS NOT NULL AND length(body) > 0
                 ORDER BY {ORDER_CLAUSE} LIMIT 1");
            if let Some(entry) = self
                .conn
                .query_row(&sql, params![word, v], Self::row_to_entry)
                .optional()?
            {
                return Ok(Some(entry));
            }
        }

        // 2. Fallback: ANY entry for this word with a body (pick best by overall_score)
        let sql = format!(
            "{SELECT_COLS} FROM nomtu WHERE word=?1 AND body IS NOT NULL AND length(body) > 0
             ORDER BY {ORDER_CLAUSE} LIMIT 1"
        );
        if let Some(entry) = self
            .conn
            .query_row(&sql, params![word], Self::row_to_entry)
            .optional()?
        {
            return Ok(Some(entry));
        }

        // 3. Semantic fallback: search describe for the word
        let pattern = format!("%{word}%");
        let sql = format!("{SELECT_COLS} FROM nomtu WHERE describe LIKE ?1 AND body IS NOT NULL AND length(body) > 0
             ORDER BY {ORDER_CLAUSE} LIMIT 1");
        let result = self
            .conn
            .query_row(&sql, params![pattern], Self::row_to_entry)
            .optional()?;
        Ok(result)
    }

    /// Backward-compatible alias for [`get_body`](Self::get_body).
    pub fn get_impl(
        &self,
        word: &str,
        variant: Option<&str>,
    ) -> Result<Option<NomtuEntry>, ResolverError> {
        self.get_body(word, variant)
    }

    /// Get all language variants for a word.
    pub fn get_all_variants(
        &self,
        word: &str,
        variant: Option<&str>,
    ) -> Result<Vec<NomtuEntry>, ResolverError> {
        let sql = if variant.is_some() {
            format!(
                "{SELECT_COLS} FROM nomtu WHERE word=?1 AND variant=?2
             ORDER BY overall_score DESC, id DESC"
            )
        } else {
            format!(
                "{SELECT_COLS} FROM nomtu WHERE word=?1 AND variant IS NULL
             ORDER BY overall_score DESC, id DESC"
            )
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
    pub fn get_all_impls(
        &self,
        word: &str,
        variant: Option<&str>,
    ) -> Result<Vec<NomtuEntry>, ResolverError> {
        self.get_all_variants(word, variant)
    }

    /// Mark an entry as canonical (and unmark others for the same word+variant).
    pub fn set_canonical(
        &self,
        word: &str,
        variant: Option<&str>,
        language: &str,
    ) -> Result<(), ResolverError> {
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
    pub fn get_impl_by_language(
        &self,
        word: &str,
        variant: Option<&str>,
        language: &str,
    ) -> Result<Option<NomtuEntry>, ResolverError> {
        let sql = if variant.is_some() {
            format!(
                "{SELECT_COLS} FROM nomtu WHERE word=?1 AND variant=?2 AND language=?3
             ORDER BY overall_score DESC, id DESC LIMIT 1"
            )
        } else {
            format!(
                "{SELECT_COLS} FROM nomtu WHERE word=?1 AND variant IS NULL AND language=?2
             ORDER BY overall_score DESC, id DESC LIMIT 1"
            )
        };

        let result = if let Some(v) = variant {
            self.conn
                .query_row(&sql, params![word, v, language], Self::row_to_entry)
                .optional()?
        } else {
            self.conn
                .query_row(&sql, params![word, language], Self::row_to_entry)
                .optional()?
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
        overall_score: f64,
    ) -> Result<(), ResolverError> {
        let entry = NomtuEntry {
            word: word.to_owned(),
            variant: variant.map(|v| v.to_owned()),
            language: language.to_owned(),
            body: Some(body.to_owned()),
            signature: signature.map(|s| s.to_owned()),
            source_path: source_path.map(|s| s.to_owned()),
            hash: hash.map(|s| s.to_owned()),
            overall_score,
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
        overall_score: f64,
    ) -> Result<(), ResolverError> {
        self.import_nomtu(
            word,
            variant,
            language,
            body,
            signature,
            source_path,
            hash,
            overall_score,
        )
    }

    fn row_to_entry(row: &rusqlite::Row) -> rusqlite::Result<NomtuEntry> {
        // Column order matches SELECT_COLS (48 columns):
        //  0: id, 1: word, 2: variant, 3: kind, 4: hash, 5: body_hash,
        //  6: describe, 7: concept, 8: labels,
        //  9: input_type, 10: output_type, 11: effects, 12: pre, 13: post,
        // 14: signature, 15: depends_on,
        // 16: security, 17: reliability, 18: performance,
        // 19: readability, 20: testability, 21: portability,
        // 22: composability, 23: maturity, 24: overall_score,
        // 25: audit_passed, 26: audit_max_severity, 27: audit_findings,
        // 28: source_repo, 29: source_path, 30: source_line,
        // 31: source_commit, 32: author, 33: language,
        // 34: body, 35: rust_body, 36: translate_confidence,
        // 37: community_id, 38: callers_count, 39: callees_count,
        // 40: is_entry_point,
        // 41: bc_path, 42: bc_hash, 43: bc_size,
        // 44: capabilities, 45: supervision, 46: schedule,
        // 47: version, 48: tests, 49: is_canonical,
        // 50: deprecated_by, 51: created_at, 52: updated_at
        let labels_json: String = row.get(8).unwrap_or_else(|_| "[]".to_owned());
        let labels: Vec<String> = serde_json::from_str(&labels_json).unwrap_or_default();
        let effects_json: String = row.get(11).unwrap_or_else(|_| "[]".to_owned());
        let effects: Vec<String> = serde_json::from_str(&effects_json).unwrap_or_default();
        let depends_on_json: String = row.get(15).unwrap_or_else(|_| "[]".to_owned());
        let depends_on: Vec<String> = serde_json::from_str(&depends_on_json).unwrap_or_default();
        Ok(NomtuEntry {
            id: row.get(0)?,
            word: row.get(1)?,
            variant: row.get(2)?,
            kind: row.get(3)?,
            hash: row.get(4)?,
            body_hash: row.get(5)?,
            describe: row.get(6)?,
            concept: row.get(7)?,
            labels,
            input_type: row.get(9)?,
            output_type: row.get(10)?,
            effects,
            pre: row.get(12)?,
            post: row.get(13)?,
            signature: row.get(14)?,
            depends_on,
            security: row.get(16)?,
            reliability: row.get(17)?,
            performance: row.get(18)?,
            readability: row.get(19)?,
            testability: row.get(20)?,
            portability: row.get(21)?,
            composability: row.get(22)?,
            maturity: row.get(23)?,
            overall_score: row.get(24)?,
            audit_passed: row.get(25)?,
            audit_max_severity: row.get(26)?,
            audit_findings: row.get(27)?,
            source_repo: row.get(28)?,
            source_path: row.get(29)?,
            source_line: row.get(30)?,
            source_commit: row.get(31)?,
            author: row.get(32)?,
            language: row.get(33)?,
            body: row.get(34)?,
            rust_body: row.get(35)?,
            translate_confidence: row.get(36)?,
            community_id: row.get(37)?,
            callers_count: row.get(38)?,
            callees_count: row.get(39)?,
            is_entry_point: row.get(40)?,
            bc_path: row.get(41)?,
            bc_hash: row.get(42)?,
            bc_size: row.get(43)?,
            capabilities: row.get(44)?,
            supervision: row.get(45)?,
            schedule: row.get(46)?,
            version: row.get(47)?,
            tests: row.get(48)?,
            is_canonical: row.get(49)?,
            deprecated_by: row.get(50)?,
            created_at: row.get(51)?,
            updated_at: row.get(52)?,
        })
    }
}

/// Column list for SELECT queries (must match `row_to_entry` field order).
const SELECT_COLS: &str = "SELECT id, word, variant, kind, hash, body_hash, \
     describe, concept, labels, \
     input_type, output_type, effects, pre, post, signature, depends_on, \
     security, reliability, performance, \
     readability, testability, portability, composability, maturity, overall_score, \
     audit_passed, audit_max_severity, audit_findings, \
     source_repo, source_path, source_line, source_commit, author, language, \
     body, rust_body, translate_confidence, \
     community_id, callers_count, callees_count, is_entry_point, \
     bc_path, bc_hash, bc_size, \
     capabilities, supervision, schedule, \
     version, tests, is_canonical, deprecated_by, created_at, updated_at";

/// Default ORDER BY clause: canonical first, then Rust, then overall_score, then newest.
const ORDER_CLAUSE: &str = "is_canonical DESC, \
     CASE WHEN language='rust' THEN 0 ELSE 1 END, \
     overall_score DESC, \
     id DESC";

// ── ADOPT-6: Datalog-style dictionary queries (Flix-inspired) ────────

/// A compound dictionary query with multiple attribute constraints.
/// Supports AND, OR, and NOT operators for .nomtu attribute filtering.
#[derive(Debug, Clone)]
pub enum DictQuery {
    /// Single attribute comparison: security > 0.9
    Comparison { field: String, op: String, value: String },
    /// Logical AND of sub-queries
    And(Vec<DictQuery>),
    /// Logical OR of sub-queries
    Or(Vec<DictQuery>),
    /// Logical NOT
    Not(Box<DictQuery>),
}

/// Parse a compound query string into a DictQuery.
/// Format: "security>0.9 and license=MIT and not deprecated"
pub fn parse_dict_query(query: &str) -> Result<DictQuery, ResolverError> {
    let parts: Vec<&str> = query.split(" and ").collect();
    let mut conditions = Vec::new();
    for part in parts {
        let part = part.trim();
        if let Some(rest) = part.strip_prefix("not ") {
            conditions.push(DictQuery::Not(Box::new(parse_single_condition(rest.trim())?)));
        } else {
            conditions.push(parse_single_condition(part)?);
        }
    }
    if conditions.len() == 1 {
        Ok(conditions.into_iter().next().unwrap())
    } else {
        Ok(DictQuery::And(conditions))
    }
}

fn parse_single_condition(s: &str) -> Result<DictQuery, ResolverError> {
    // Parse: field>value, field<value, field=value, field>=value, field<=value
    // Also: bare word like "deprecated" -> Comparison { field: "deprecated", op: "=", value: "true" }
    for op in &[">=", "<=", "!=", ">", "<", "="] {
        if let Some(idx) = s.find(op) {
            return Ok(DictQuery::Comparison {
                field: s[..idx].trim().to_string(),
                op: op.to_string(),
                value: s[idx + op.len()..].trim().trim_matches('"').to_string(),
            });
        }
    }
    // Bare word: "deprecated" -> field=true
    Ok(DictQuery::Comparison {
        field: s.trim().to_string(),
        op: "=".to_string(),
        value: "true".to_string(),
    })
}

// ── ADOPT-7: Bidirectional contract inference via unification ───────

/// A contract shape: input type -> output type.
///
/// Inspired by Prolog's bidirectional unification — types propagate
/// both forward (output of A becomes input of B) and backward
/// (required input of B constrains output of A).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractShape {
    pub input_type: Option<String>,
    pub output_type: Option<String>,
}

/// Result of contract inference for a single node in a flow chain.
#[derive(Debug, Clone)]
pub struct InferredContract {
    /// Node name in the flow.
    pub node: String,
    /// Inferred contract shape.
    pub contract: ContractShape,
    /// Whether this was inferred (`true`) or known from the dictionary (`false`).
    pub inferred: bool,
}

/// Given a flow chain (list of node names) and the known contracts for some nodes,
/// infer the contract shapes that intermediate nodes must satisfy.
///
/// Uses bidirectional unification:
/// - **Forward pass**: propagate output types as input types of the next node.
/// - **Backward pass**: propagate required input types as output types of the previous node.
///
/// Returns inferred contracts for all nodes in the chain.
pub fn infer_flow_contracts(
    chain: &[String],
    known_contracts: &std::collections::HashMap<String, ContractShape>,
) -> Vec<InferredContract> {
    if chain.is_empty() {
        return vec![];
    }

    // 1. Initialize contracts from known_contracts (clone known, default unknown)
    let mut contracts: Vec<ContractShape> = chain
        .iter()
        .map(|name| {
            known_contracts
                .get(name)
                .cloned()
                .unwrap_or(ContractShape {
                    input_type: None,
                    output_type: None,
                })
        })
        .collect();

    // 2. Forward pass: if node[i].output_type is known and node[i+1].input_type is unknown, set it
    for i in 0..contracts.len() - 1 {
        if let Some(out) = contracts[i].output_type.clone() {
            if contracts[i + 1].input_type.is_none() {
                contracts[i + 1].input_type = Some(out);
            }
        }
    }

    // 3. Backward pass: if node[i+1].input_type is known and node[i].output_type is unknown, set it
    for i in (0..contracts.len() - 1).rev() {
        if let Some(inp) = contracts[i + 1].input_type.clone() {
            if contracts[i].output_type.is_none() {
                contracts[i].output_type = Some(inp);
            }
        }
    }

    // 4. Build results with inferred flag
    chain
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let was_known = known_contracts.get(name).is_some_and(|k| *k == contracts[i]);
            InferredContract {
                node: name.clone(),
                contract: contracts[i].clone(),
                inferred: !was_known,
            }
        })
        .collect()
}

// ── ADOPT-10: Structural interface satisfaction (Go-inspired) ───────

/// Check if two contract shapes are structurally compatible.
/// Compatible means: the output of `producer` can flow into the input of `consumer`.
/// This is Go-style structural satisfaction — no explicit implements needed.
pub fn contracts_compatible(producer: &ContractShape, consumer: &ContractShape) -> bool {
    match (&producer.output_type, &consumer.input_type) {
        (Some(out), Some(inp)) => {
            // Exact match
            out == inp
            // Or one is a supertype of the other (simplified: check prefix)
            || out.starts_with(inp)
            || inp.starts_with(out)
        }
        // If either is None (unspecified), assume compatible
        (None, _) | (_, None) => true,
    }
}

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
        assert!(matches!(
            resolver.resolve(&nom_ref),
            Err(ResolverError::NotFound { .. })
        ));
    }

    #[test]
    fn semantic_search() {
        let resolver = Resolver::open_in_memory().unwrap();
        resolver
            .upsert(&sample_entry("hash", Some("argon2")))
            .unwrap();
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
            overall_score: 0.85,
            ..NomtuEntry::default()
        };
        resolver.upsert(&entry).unwrap();

        let found = resolver.get_body("hash", Some("sha256")).unwrap().unwrap();
        assert_eq!(found.word, "hash");
        assert_eq!(found.variant.as_deref(), Some("sha256"));
        assert_eq!(found.language, "rust");
        assert!(found.body.as_deref().unwrap().contains("fn hash"));
        assert!((found.overall_score - 0.85).abs() < 1e-9);
    }

    #[test]
    fn canonical_prefers_rust() {
        let resolver = Resolver::open_in_memory().unwrap();

        // Insert a high-quality Python impl
        resolver
            .import_nomtu(
                "sort",
                None,
                "python",
                "def sort(lst): return sorted(lst)",
                None,
                None,
                Some("py1"),
                0.95,
            )
            .unwrap();
        // Insert a lower-quality Rust impl
        resolver
            .import_nomtu(
                "sort",
                None,
                "rust",
                "fn sort(v: &mut Vec<i32>) { v.sort(); }",
                None,
                None,
                Some("rs1"),
                0.70,
            )
            .unwrap();

        // get_body should prefer Rust even though Python has higher quality
        let found = resolver.get_body("sort", None).unwrap().unwrap();
        assert_eq!(found.language, "rust");
    }

    #[test]
    fn canonical_flag_overrides_language() {
        let resolver = Resolver::open_in_memory().unwrap();

        resolver
            .import_nomtu(
                "sort",
                None,
                "rust",
                "fn sort() {}",
                None,
                None,
                Some("rs1"),
                0.70,
            )
            .unwrap();
        resolver
            .import_nomtu(
                "sort",
                None,
                "python",
                "def sort(): pass",
                None,
                None,
                Some("py1"),
                0.95,
            )
            .unwrap();

        // Mark Python as canonical
        resolver.set_canonical("sort", None, "python").unwrap();

        let found = resolver.get_body("sort", None).unwrap().unwrap();
        assert_eq!(found.language, "python");
        assert!(found.is_canonical);
    }

    #[test]
    fn import_nomtu_convenience() {
        let resolver = Resolver::open_in_memory().unwrap();
        resolver
            .import_nomtu(
                "encrypt",
                Some("aes"),
                "rust",
                "fn encrypt(key: &[u8], data: &[u8]) -> Vec<u8> { todo!() }",
                Some(r#"{"inputs":["key","data"],"outputs":["ciphertext"],"effects":["crypto"]}"#),
                Some("crypto/src/aes.rs"),
                Some("hash456"),
                0.90,
            )
            .unwrap();

        let found = resolver.get_body("encrypt", Some("aes")).unwrap().unwrap();
        assert_eq!(found.language, "rust");
        assert!(found.body.as_deref().unwrap().contains("encrypt"));
        assert_eq!(found.source_path.as_deref(), Some("crypto/src/aes.rs"));
        assert_eq!(found.hash.as_deref(), Some("hash456"));
    }

    #[test]
    fn get_all_variants_returns_all_languages() {
        let resolver = Resolver::open_in_memory().unwrap();

        resolver
            .import_nomtu(
                "parse",
                None,
                "rust",
                "fn parse() {}",
                None,
                None,
                Some("rs1"),
                0.80,
            )
            .unwrap();
        resolver
            .import_nomtu(
                "parse",
                None,
                "python",
                "def parse(): pass",
                None,
                None,
                Some("py1"),
                0.75,
            )
            .unwrap();
        resolver
            .import_nomtu(
                "parse",
                None,
                "go",
                "func parse() {}",
                None,
                None,
                Some("go1"),
                0.70,
            )
            .unwrap();

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
        resolver
            .import_nomtu(
                "fmt",
                None,
                "rust",
                "fn fmt() {}",
                None,
                None,
                Some("rs1"),
                0.80,
            )
            .unwrap();
        resolver
            .import_nomtu(
                "fmt",
                None,
                "go",
                "func fmt() {}",
                None,
                None,
                Some("go1"),
                0.90,
            )
            .unwrap();

        let go_impl = resolver
            .get_impl_by_language("fmt", None, "go")
            .unwrap()
            .unwrap();
        assert_eq!(go_impl.language, "go");

        let missing = resolver
            .get_impl_by_language("fmt", None, "python")
            .unwrap();
        assert!(missing.is_none());
    }

    // ── Backward compatibility aliases ──────────────────────────────

    // ── Contract-shape search tests ────────────────────────────────

    #[test]
    fn parse_contract_full_form() {
        let (inp, out) = Resolver::parse_contract_signature("in: bytes -> out: hash");
        assert_eq!(inp.as_deref(), Some("bytes"));
        assert_eq!(out.as_deref(), Some("hash"));
    }

    #[test]
    fn parse_contract_shorthand() {
        let (inp, out) = Resolver::parse_contract_signature("bytes -> hash");
        assert_eq!(inp.as_deref(), Some("bytes"));
        assert_eq!(out.as_deref(), Some("hash"));
    }

    #[test]
    fn parse_contract_output_only() {
        let (inp, out) = Resolver::parse_contract_signature("-> hash");
        assert_eq!(inp, None);
        assert_eq!(out.as_deref(), Some("hash"));
    }

    #[test]
    fn parse_contract_input_only() {
        let (inp, out) = Resolver::parse_contract_signature("bytes ->");
        assert_eq!(inp.as_deref(), Some("bytes"));
        assert_eq!(out, None);
    }

    #[test]
    fn parse_contract_no_arrow() {
        // No arrow — treated as keyword matching both columns
        let (inp, out) = Resolver::parse_contract_signature("bytes");
        assert_eq!(inp.as_deref(), Some("bytes"));
        assert_eq!(out.as_deref(), Some("bytes"));
    }

    #[test]
    fn search_by_contract_exact() {
        let resolver = Resolver::open_in_memory().unwrap();
        let entry = sample_entry("hash", Some("argon2"));
        // sample_entry has input_type=Some("bytes"), output_type=Some("hash")
        resolver.upsert(&entry).unwrap();

        let results = resolver.search_by_contract("bytes -> hash", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].word, "hash");
    }

    #[test]
    fn search_by_contract_partial_match() {
        let resolver = Resolver::open_in_memory().unwrap();
        let mut entry = sample_entry("compress", Some("gzip"));
        entry.input_type = Some("raw_bytes".to_owned());
        entry.output_type = Some("compressed_bytes".to_owned());
        entry.hash = Some("gzip1".to_owned());
        resolver.upsert(&entry).unwrap();

        // Partial match: "bytes" should match "raw_bytes" and "compressed_bytes"
        let results = resolver.search_by_contract("bytes -> bytes", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].word, "compress");
    }

    #[test]
    fn search_by_contract_output_only() {
        let resolver = Resolver::open_in_memory().unwrap();
        let entry = sample_entry("hash", Some("sha256"));
        resolver.upsert(&entry).unwrap();

        // Search only by output type
        let results = resolver.search_by_contract("-> hash", 10).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_by_contract_no_match() {
        let resolver = Resolver::open_in_memory().unwrap();
        let entry = sample_entry("hash", Some("sha256"));
        resolver.upsert(&entry).unwrap();

        let results = resolver
            .search_by_contract("string -> integer", 10)
            .unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn search_by_contract_empty_returns_empty() {
        let resolver = Resolver::open_in_memory().unwrap();
        let results = resolver.search_by_contract("->", 10).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn backward_compat_get_impl() {
        let resolver = Resolver::open_in_memory().unwrap();
        resolver
            .import_atom(
                "sort",
                None,
                "rust",
                "fn sort() {}",
                None,
                None,
                Some("rs1"),
                0.80,
            )
            .unwrap();
        let found = resolver.get_impl("sort", None).unwrap().unwrap();
        assert_eq!(found.language, "rust");
    }

    #[test]
    fn backward_compat_get_all_impls() {
        let resolver = Resolver::open_in_memory().unwrap();
        resolver
            .import_atom("x", None, "rust", "fn x() {}", None, None, Some("r1"), 0.8)
            .unwrap();
        resolver
            .import_atom("x", None, "go", "func x() {}", None, None, Some("g1"), 0.7)
            .unwrap();
        let all = resolver.get_all_impls("x", None).unwrap();
        assert_eq!(all.len(), 2);
    }

    // ── Contract inference tests (ADOPT-7) ─────────────────────────

    #[test]
    fn infers_intermediate_contract_forward() {
        // Chain: A -> B -> C
        // Known: A outputs "bytes", C requires "bytes" input
        // Infer: B must accept "bytes" and produce "bytes"
        let chain = vec!["A".into(), "B".into(), "C".into()];
        let mut known = std::collections::HashMap::new();
        known.insert("A".into(), ContractShape { input_type: None, output_type: Some("bytes".into()) });
        known.insert("C".into(), ContractShape { input_type: Some("bytes".into()), output_type: None });

        let result = infer_flow_contracts(&chain, &known);
        let b = result.iter().find(|r| r.node == "B").unwrap();
        assert_eq!(b.contract.input_type, Some("bytes".into()));
        assert_eq!(b.contract.output_type, Some("bytes".into()));
        assert!(b.inferred);
    }

    #[test]
    fn forward_propagation_only() {
        // Chain: A -> B -> C
        // Known: A outputs "text"
        // Infer: B input is "text", but output and C's input remain unknown
        let chain = vec!["A".into(), "B".into(), "C".into()];
        let mut known = std::collections::HashMap::new();
        known.insert("A".into(), ContractShape { input_type: None, output_type: Some("text".into()) });

        let result = infer_flow_contracts(&chain, &known);
        let b = result.iter().find(|r| r.node == "B").unwrap();
        assert_eq!(b.contract.input_type, Some("text".into()));
    }

    #[test]
    fn backward_propagation() {
        // Chain: A -> B -> C
        // Known: C requires "hash" input
        // Infer: B must output "hash"
        let chain = vec!["A".into(), "B".into(), "C".into()];
        let mut known = std::collections::HashMap::new();
        known.insert("C".into(), ContractShape { input_type: Some("hash".into()), output_type: None });

        let result = infer_flow_contracts(&chain, &known);
        let b = result.iter().find(|r| r.node == "B").unwrap();
        assert_eq!(b.contract.output_type, Some("hash".into()));
    }

    #[test]
    fn empty_chain_returns_empty() {
        let result = infer_flow_contracts(&[], &std::collections::HashMap::new());
        assert!(result.is_empty());
    }

    #[test]
    fn single_node_uses_known_contract() {
        let chain = vec!["A".into()];
        let mut known = std::collections::HashMap::new();
        known.insert("A".into(), ContractShape { input_type: Some("bytes".into()), output_type: Some("hash".into()) });

        let result = infer_flow_contracts(&chain, &known);
        assert_eq!(result.len(), 1);
        assert!(!result[0].inferred);
    }

    // ── DictQuery tests (ADOPT-6) ─────────────────────────────────────

    #[test]
    fn parses_single_comparison() {
        let q = parse_dict_query("security>0.9").unwrap();
        match q {
            DictQuery::Comparison { field, op, value } => {
                assert_eq!(field, "security");
                assert_eq!(op, ">");
                assert_eq!(value, "0.9");
            }
            _ => panic!("expected Comparison"),
        }
    }

    #[test]
    fn parses_compound_and_query() {
        let q = parse_dict_query("security>0.9 and license=MIT").unwrap();
        match q {
            DictQuery::And(parts) => assert_eq!(parts.len(), 2),
            _ => panic!("expected And"),
        }
    }

    #[test]
    fn parses_not_condition() {
        let q = parse_dict_query("not deprecated").unwrap();
        match q {
            DictQuery::Not(_) => {},
            _ => panic!("expected Not"),
        }
    }

    #[test]
    fn parses_complex_query() {
        let q = parse_dict_query("security>0.9 and license=MIT and not deprecated").unwrap();
        match q {
            DictQuery::And(parts) => {
                assert_eq!(parts.len(), 3);
                assert!(matches!(&parts[2], DictQuery::Not(_)));
            }
            _ => panic!("expected And"),
        }
    }

    // ── Structural compatibility tests (ADOPT-10) ─────────────────────

    #[test]
    fn exact_type_match_is_compatible() {
        let producer = ContractShape { input_type: None, output_type: Some("bytes".into()) };
        let consumer = ContractShape { input_type: Some("bytes".into()), output_type: None };
        assert!(contracts_compatible(&producer, &consumer));
    }

    #[test]
    fn different_types_incompatible() {
        let producer = ContractShape { input_type: None, output_type: Some("text".into()) };
        let consumer = ContractShape { input_type: Some("number".into()), output_type: None };
        assert!(!contracts_compatible(&producer, &consumer));
    }

    #[test]
    fn unspecified_type_is_compatible() {
        let producer = ContractShape { input_type: None, output_type: None };
        let consumer = ContractShape { input_type: Some("bytes".into()), output_type: None };
        assert!(contracts_compatible(&producer, &consumer));
    }

    #[test]
    fn prefix_subtype_is_compatible() {
        // "hash_bytes" starts with "hash" — compatible
        let producer = ContractShape { input_type: None, output_type: Some("hash_bytes".into()) };
        let consumer = ContractShape { input_type: Some("hash".into()), output_type: None };
        assert!(contracts_compatible(&producer, &consumer));
    }
}
