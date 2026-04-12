//! SQLite-backed dictionary for the v2 content-addressed Nom store.
//!
//! `id = sha256(canonicalize(ast, contract))` is the sole identity
//! column on the `entries` table. Structured side tables hold scores,
//! signatures, findings, refs, graph edges and translations.
//! Unbounded metadata lives in the EAV `entry_meta` table.
//!
//! Layout: `data/nomdict.db`. WAL mode is enabled for concurrent reads.

use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use nom_types::{
    Contract, EdgeType, Entry, EntryKind, EntryScores, EntrySignature, EntryStatus, GraphEdge,
    SecurityFinding, Severity, Translation,
};
use rusqlite::{Connection, OptionalExtension, params};

// ── Schema ──────────────────────────────────────────────────────────

/// The v2 schema SQL. Shared so nom-resolver (and any other crate that
/// needs raw queries against the same DB) can initialise the same
/// tables.
pub const V2_SCHEMA_SQL: &str = r#"
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
"#;

// ── EntryFilter ──────────────────────────────────────────────────────

/// Filter options for [`NomDict::find_entries`]. All fields are optional;
/// unset fields do not restrict the query. `limit` defaults to 50.
#[derive(Debug, Clone, Default)]
pub struct EntryFilter {
    pub body_kind: Option<String>,
    pub language: Option<String>,
    pub status: Option<EntryStatus>,
    pub kind: Option<EntryKind>,
    pub limit: usize,
}

// ── NomDict ─────────────────────────────────────────────────────────

/// SQLite-backed v2 dictionary.
pub struct NomDict {
    conn: Connection,
    root: PathBuf,
}

impl NomDict {
    /// Open or create the v2 dictionary at `<root>/data/nomdict.db`.
    /// Empty / missing DBs are handled gracefully: `count()` returns 0,
    /// `get_entry` returns `None`.
    pub fn open(root: &Path) -> Result<Self> {
        let db_dir = root.join("data");
        std::fs::create_dir_all(&db_dir)?;
        let db_path = db_dir.join("nomdict.db");
        let conn = Connection::open(&db_path)
            .with_context(|| format!("failed to open NomDict at {}", db_path.display()))?;

        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "cache_size", "-64000")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(std::time::Duration::from_secs(30))?;

        let dict = Self {
            conn,
            root: root.to_path_buf(),
        };
        dict.init_schema()?;
        Ok(dict)
    }

    /// Open or create the v2 dictionary at an explicit `.db` file path.
    /// Unlike [`open`] (which expects a root dir and appends `data/nomdict.db`),
    /// this accepts the exact path to the SQLite file. Parent directories are
    /// created if missing. Used by `nom corpus ingest` which takes `--dict
    /// nomdict.db` pointing directly at the DB file.
    pub fn open_in_place(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let conn = Connection::open(db_path)
            .with_context(|| format!("failed to open NomDict at {}", db_path.display()))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "cache_size", "-64000")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(std::time::Duration::from_secs(30))?;
        let dict = Self {
            conn,
            root: db_path.parent().unwrap_or(Path::new(".")).to_path_buf(),
        };
        dict.init_schema()?;
        Ok(dict)
    }

    /// Open an in-memory v2 dictionary (for tests).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let dict = Self {
            conn,
            root: PathBuf::new(),
        };
        dict.init_schema()?;
        Ok(dict)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(V2_SCHEMA_SQL)?;
        // Best-effort migration: add body_bytes to pre-existing DBs that were
        // created before this column was part of V2_SCHEMA_SQL. SQLite returns
        // "duplicate column name" when it already exists — ignore that error.
        let _ = self.conn
            .execute_batch("ALTER TABLE entries ADD COLUMN body_bytes BLOB");
        Ok(())
    }

    /// Raw access for advanced query consumers (nom-resolver).
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Begin a SQLite transaction on this connection.
    ///
    /// Returns a RAII guard; call [`rusqlite::Transaction::commit`] on the
    /// guard to persist, or let it drop to roll back. Uses
    /// `unchecked_transaction` (same as [`Self::bulk_upsert`]) so `&self`
    /// suffices — no `&mut self` needed.
    ///
    /// All `upsert_entry` / `get_entry` calls made while the guard is live
    /// operate inside the same transaction, giving per-repo atomic commits.
    pub fn begin_transaction(&self) -> rusqlite::Result<rusqlite::Transaction<'_>> {
        self.conn.unchecked_transaction()
    }

    /// Database file path (empty `PathBuf` for in-memory).
    pub fn db_path(&self) -> PathBuf {
        if self.root.as_os_str().is_empty() {
            PathBuf::new()
        } else {
            self.root.join("data/nomdict.db")
        }
    }

    // ── Counts / queries ───────────────────────────────────────────

    /// Total number of `entries` rows. Returns 0 on a fresh DB.
    pub fn count(&self) -> Result<usize> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))?;
        Ok(n as usize)
    }

    /// §4.4.6: histogram of `body_kind` values across the dict.
    /// Returns `(kind_or_null, count)` pairs sorted by count desc.
    /// The NULL-bucket is returned as `"(untagged)"` so all rows are one
    /// consistent `(String, usize)` shape. Used by `nom store stats`.
    pub fn body_kind_histogram(&self) -> Result<Vec<(String, usize)>> {
        let mut stmt = self.conn.prepare(
            "SELECT COALESCE(body_kind, '(untagged)') AS k, COUNT(*) AS n
             FROM entries
             GROUP BY body_kind
             ORDER BY n DESC, k ASC",
        )?;
        let rows = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    // ── Upsert / setters ──────────────────────────────────────────

    /// Insert or replace the main `entries` row. Returns the entry id.
    /// Re-upserting an existing id preserves the original `created_at`
    /// and bumps `updated_at` to now.
    pub fn upsert_entry(&self, entry: &Entry) -> Result<String> {
        self.conn.execute(
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

    /// §5.17.2 bulk-ingestion path: try-insert without a prior
    /// existence check. Returns `true` if the row was newly inserted,
    /// `false` if the `id` already existed and the INSERT was a
    /// no-op.
    ///
    /// Unlike `upsert_entry`, this does NOT replace on conflict — the
    /// existing row is preserved. Designed for the corpus-ingest path
    /// where duplicates are expected (dedup is the point) and we
    /// don't want the overhead of a SELECT-then-INSERT.
    pub fn upsert_entry_if_new(&self, entry: &Entry) -> Result<bool> {
        let changed = self.conn.execute(
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

    /// Insert or replace the `entry_scores` row for an entry.
    pub fn set_scores(&self, id: &str, scores: &EntryScores) -> Result<()> {
        self.conn.execute(
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

    /// Add a (key, value) metadata row. `(id, key, value)` is the PK so
    /// the same key can have many values (e.g. multiple `tag`
    /// values for a dedup'd entry).
    pub fn add_meta(&self, id: &str, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO entry_meta (id, key, value) VALUES (?1, ?2, ?3)",
            params![id, key, value],
        )?;
        Ok(())
    }

    /// Insert or replace the signature for an entry.
    pub fn set_signature(&self, id: &str, sig: &EntrySignature) -> Result<()> {
        self.conn.execute(
            "INSERT INTO entry_signatures (id, visibility, is_async, is_method, return_type, params_json)
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

    /// Append a security finding (many per entry).
    pub fn add_finding(&self, id: &str, finding: &SecurityFinding) -> Result<()> {
        self.conn.execute(
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

    /// Add a structural closure ref (idempotent).
    pub fn add_ref(&self, from_id: &str, to_id: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO entry_refs (from_id, to_id) VALUES (?1, ?2)",
            params![from_id, to_id],
        )?;
        Ok(())
    }

    /// Append a semantic graph edge.
    pub fn add_graph_edge(&self, edge: &GraphEdge) -> Result<()> {
        self.conn.execute(
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
    pub fn add_translation(&self, t: &Translation) -> Result<()> {
        self.conn.execute(
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

    // ── Getters ────────────────────────────────────────────────────

    /// Fetch a single entry by id.
    pub fn get_entry(&self, id: &str) -> Result<Option<Entry>> {
        let row = self
            .conn
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

    /// §4.4.6: fetch just the canonical-format bytes for an entry.
    /// Returns None if either the entry doesn't exist or has no
    /// body_bytes (legacy row).
    pub fn get_entry_bytes(&self, id: &str) -> Result<Option<Vec<u8>>> {
        let result: Option<Option<Vec<u8>>> = self
            .conn
            .query_row(
                "SELECT body_bytes FROM entries WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(result.flatten())
    }

    /// Look up every entry whose `word` column equals `word`. Returns an empty
    /// vec when nothing matches (caller distinguishes NotFound from Ambiguous
    /// based on the result length). Uses the `idx_entries_word` index.
    pub fn find_by_word(&self, word: &str) -> Result<Vec<Entry>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, word, variant, kind, language, describe, concept, body, body_nom,
                    input_type, output_type, pre, post, status, translation_score,
                    is_canonical, deprecated_by, created_at, updated_at, body_kind,
                    body_bytes
             FROM entries WHERE word = ?1 ORDER BY id",
        )?;
        let rows = stmt
            .query_map(params![word], row_to_entry)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// §4.4.6: return entries whose `body_kind` equals the given tag.
    /// Return ids of entries with `status = 'partial'`, ordered by id for
    /// deterministic batch-resumption semantics. `max = None` returns all;
    /// `max = Some(n)` caps at `n` rows.
    pub fn list_partial_ids(&self, max: Option<usize>) -> Result<Vec<String>> {
        let sql = match max {
            Some(n) => format!(
                "SELECT id FROM entries WHERE status = 'partial' ORDER BY id LIMIT {}",
                n
            ),
            None => "SELECT id FROM entries WHERE status = 'partial' ORDER BY id".to_string(),
        };
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Unified filter query. Each present field adds an AND clause.
    /// Results ordered by id for determinism. An empty `EntryFilter`
    /// returns the first `limit` entries (default 50).
    pub fn find_entries(&self, f: &EntryFilter) -> Result<Vec<Entry>> {
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
        let param_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|b| b.as_ref()).collect();
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(param_refs.iter()), row_to_entry)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Parallel to [`nom_resolver::Resolver::find_by_body_kind`] but on
    /// the v2 DIDS store — used by `nom build <hash>` closure walks to
    /// filter to entries with the right canonical format (e.g. only
    /// bitcode-ready entries when linking an app).
    pub fn find_by_body_kind(&self, kind: &str, limit: usize) -> Result<Vec<Entry>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, word, variant, kind, language, describe, concept, body, body_nom,
                    input_type, output_type, pre, post, status, translation_score,
                    is_canonical, deprecated_by, created_at, updated_at, body_kind,
                    body_bytes
             FROM entries WHERE body_kind = ?1 ORDER BY id LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(params![kind, limit as i64], row_to_entry)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Fetch all (key, value) metadata rows for an entry.
    pub fn get_meta(&self, id: &str) -> Result<Vec<(String, String)>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT key, value FROM entry_meta WHERE id = ?1 ORDER BY key, value")?;
        let rows = stmt
            .query_map(params![id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// List the structural refs out of an entry (`entry_refs.to_id`).
    pub fn get_refs(&self, id: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT to_id FROM entry_refs WHERE from_id = ?1 ORDER BY to_id")?;
        let rows = stmt
            .query_map(params![id], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Fetch the (optional) scores row for an entry.
    pub fn get_scores(&self, id: &str) -> Result<Option<EntryScores>> {
        let row = self
            .conn
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

    /// Fetch all security findings for an entry.
    pub fn get_findings(&self, id: &str) -> Result<Vec<SecurityFinding>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT finding_id, id, severity, category, rule_id, message, evidence, line, remediation
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

    /// Transitive closure of `entry_refs` starting at `root_id`.
    /// Terminates on cycles via a visited-set. The returned order is
    /// BFS from the root; the root itself is the first element.
    pub fn closure(&self, root_id: &str) -> Result<Vec<String>> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        let mut queue: VecDeque<String> = VecDeque::new();
        queue.push_back(root_id.to_string());
        while let Some(cur) = queue.pop_front() {
            if !seen.insert(cur.clone()) {
                continue;
            }
            out.push(cur.clone());
            let mut stmt = self
                .conn
                .prepare_cached("SELECT to_id FROM entry_refs WHERE from_id = ?1")?;
            let next: Vec<String> = stmt
                .query_map(params![cur], |row| row.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            for n in next {
                if !seen.contains(&n) {
                    queue.push_back(n);
                }
            }
        }
        Ok(out)
    }

    // ── Bulk insert (for seeding tests / ingestion) ────────────────

    /// Bulk-insert many entries in a single transaction. Skips entries
    /// whose id already exists.
    pub fn bulk_upsert(&self, entries: &[Entry]) -> Result<usize> {
        let tx = self.conn.unchecked_transaction()?;
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

    /// Bulk-insert scores in one transaction.
    pub fn bulk_set_scores(&self, scores: &[EntryScores]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
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
}

// ── Helpers ─────────────────────────────────────────────────────────

fn row_to_entry(row: &rusqlite::Row) -> rusqlite::Result<Entry> {
    let kind_str: String = row.get(3)?;
    let status_str: String = row.get(13)?;
    Ok(Entry {
        id: row.get(0)?,
        word: row.get(1)?,
        variant: row.get(2)?,
        kind: EntryKind::from_str(&kind_str),
        language: row.get(4)?,
        describe: row.get(5)?,
        concept: row.get(6)?,
        body: row.get(7)?,
        body_nom: row.get(8)?,
        contract: Contract {
            input_type: row.get(9)?,
            output_type: row.get(10)?,
            pre: row.get(11)?,
            post: row.get(12)?,
        },
        status: EntryStatus::from_str(&status_str),
        translation_score: row.get(14)?,
        is_canonical: row.get(15)?,
        deprecated_by: row.get(16)?,
        created_at: row.get(17)?,
        updated_at: row.get(18)?,
        body_kind: row.get(19)?,
        body_bytes: row.get::<_, Option<Vec<u8>>>(20)?,
    })
}

// Re-export so callers can construct edges without another use-line.
pub use nom_types::EdgeType as __ReexportedEdgeType;
#[allow(dead_code)]
fn _compile_check(_: EdgeType) {}

// ── Legacy v1 shims ─────────────────────────────────────────────────
//
// Return empty results so nom-cli's `dict`, `stats`, and search
// commands keep compiling. Task B replaces these with real v2
// implementations that read from the new tables.

use nom_types::{Atom, NomtuEntry};

/// Legacy return shape for `store_atoms`. Matches the v1 API so nom-cli keeps compiling.
pub struct StoreResult {
    pub stored: usize,
    pub skipped: usize,
}

impl NomDict {
    /// Legacy: insert a slice of v1 atoms. No-op until Task B migrates callers.
    pub fn store_atoms(&self, _atoms: &[Atom]) -> Result<StoreResult> {
        Ok(StoreResult { stored: 0, skipped: 0 })
    }

    /// Legacy: return every v1 atom. Empty until Task B migrates callers.
    pub fn load_all(&self) -> Result<Vec<Atom>> {
        Ok(Vec::new())
    }

    /// Legacy: `(kind, count)` histogram. Empty until Task B.
    pub fn stats_by_kind(&self) -> Result<Vec<(String, usize)>> {
        Ok(Vec::new())
    }

    /// Legacy: `(language, count)` histogram. Empty until Task B.
    pub fn stats_by_language(&self) -> Result<Vec<(String, usize)>> {
        Ok(Vec::new())
    }

    /// Legacy: `(concept, count)` histogram. Empty until Task B.
    pub fn dictionary_summary(&self) -> Result<Vec<(String, usize)>> {
        Ok(Vec::new())
    }
}

// Silence unused-import warning when only Atom is touched in the shim block.
#[allow(dead_code)]
fn _legacy_compile_check(_: NomtuEntry) {}

// ── Unit tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use nom_types::{Contract, EntryKind, EntryStatus};

    fn make_entry(id: &str, word: &str) -> Entry {
        Entry {
            id: id.into(),
            word: word.into(),
            variant: None,
            kind: EntryKind::Function,
            language: "nom".into(),
            describe: None,
            concept: None,
            body: None,
            body_nom: None,
            body_bytes: None,
            body_kind: None,
            contract: Contract::default(),
            status: EntryStatus::Complete,
            translation_score: None,
            is_canonical: true,
            deprecated_by: None,
            created_at: "2026-04-12T00:00:00Z".into(),
            updated_at: None,
        }
    }

    #[test]
    fn empty_dict_count_is_zero() {
        let d = NomDict::open_in_memory().unwrap();
        assert_eq!(d.count().unwrap(), 0);
        assert!(d.get_entry("nonexistent").unwrap().is_none());
    }

    #[test]
    fn upsert_then_get_roundtrip() {
        let d = NomDict::open_in_memory().unwrap();
        let e = make_entry("abc123", "greet");
        d.upsert_entry(&e).unwrap();
        assert_eq!(d.count().unwrap(), 1);
        let fetched = d.get_entry("abc123").unwrap().unwrap();
        assert_eq!(fetched.word, "greet");
        assert_eq!(fetched.id, "abc123");
    }

    #[test]
    fn meta_multi_value() {
        let d = NomDict::open_in_memory().unwrap();
        d.upsert_entry(&make_entry("x", "w")).unwrap();
        d.add_meta("x", "tag", "a").unwrap();
        d.add_meta("x", "tag", "b").unwrap();
        let m = d.get_meta("x").unwrap();
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn closure_terminates_on_cycle() {
        let d = NomDict::open_in_memory().unwrap();
        for w in ["A", "B", "C"] {
            d.upsert_entry(&make_entry(w, w)).unwrap();
        }
        d.add_ref("A", "B").unwrap();
        d.add_ref("B", "C").unwrap();
        d.add_ref("C", "A").unwrap();
        let closure = d.closure("A").unwrap();
        let set: std::collections::HashSet<_> = closure.iter().collect();
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn eav_extensibility_no_schema_change() {
        // A brand new facet name requires zero ALTER TABLE — EAV stores it directly.
        let d = NomDict::open_in_memory().unwrap();
        d.upsert_entry(&make_entry("e1", "target")).unwrap();
        d.add_meta("e1", "quantum_safe", "true").unwrap();
        d.add_meta("e1", "wasm_target", "wasi-p2").unwrap();
        let rows = d.get_meta("e1").unwrap();
        let mut found = std::collections::HashMap::new();
        for (k, v) in rows {
            found.insert(k, v);
        }
        assert_eq!(found.get("quantum_safe").map(String::as_str), Some("true"));
        assert_eq!(found.get("wasm_target").map(String::as_str), Some("wasi-p2"));
    }

    #[test]
    fn typed_query_uses_indexes() {
        // Property: filtering by score threshold AND absence of a Critical finding
        // should use the declared indexes (not a full scan).
        //
        // We don't insert 1M rows in a unit test — instead we assert the query
        // plan uses indexes on the relevant columns, which is what makes the
        // 100 ms target on a 1M corpus achievable. The perf test with a large
        // synthetic corpus belongs in a benchmark, not the test suite.
        let d = NomDict::open_in_memory().unwrap();
        // populate a handful of rows so the planner has real stats
        for i in 0..32 {
            let id = format!("e{i:02}");
            d.upsert_entry(&make_entry(&id, &id)).unwrap();
            d.set_scores(
                &id,
                &EntryScores {
                    id: id.clone(),
                    security: Some(if i % 2 == 0 { 0.95 } else { 0.4 }),
                    reliability: Some(0.8),
                    performance: Some(0.7),
                    readability: Some(0.8),
                    testability: Some(0.6),
                    portability: Some(0.7),
                    composability: Some(0.8),
                    maturity: Some(0.5),
                    overall_score: Some(0.7),
                },
            )
            .unwrap();
            if i % 3 == 0 {
                d.add_finding(
                    &id,
                    &SecurityFinding {
                        finding_id: 0,
                        id: id.clone(),
                        severity: Severity::Critical,
                        category: "injection".to_string(),
                        rule_id: None,
                        message: None,
                        evidence: None,
                        line: None,
                        remediation: None,
                    },
                )
                .unwrap();
            }
        }

        // Run the canonical v2 query. Should complete trivially under 100 ms
        // on this size; the real perf target is verified by the query plan
        // using the indexes below.
        let sql = "SELECT e.id FROM entries e \
                   JOIN entry_scores s ON s.id = e.id \
                   WHERE s.security > 0.9 \
                   AND NOT EXISTS ( \
                       SELECT 1 FROM entry_security_findings f \
                       WHERE f.id = e.id AND f.severity = 'Critical')";
        let start = std::time::Instant::now();
        let count: i64 = d
            .connection()
            .prepare(sql)
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .count() as i64;
        let elapsed = start.elapsed();
        assert!(count >= 0);
        assert!(
            elapsed.as_millis() < 500,
            "query took {elapsed:?}, expected < 500 ms on tiny corpus"
        );

        // Verify the query plan touches indexes, not full scans.
        let plan: Vec<String> = d
            .connection()
            .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
            .unwrap()
            .query_map([], |row| row.get::<_, String>(3))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        let plan_text = plan.join(" | ");
        assert!(
            plan_text.contains("USING INDEX") || plan_text.contains("USING COVERING INDEX"),
            "query plan did not use any index: {plan_text}"
        );
    }

    /// §5.17.2: upsert_entry_if_new returns true on first insert, false on
    /// duplicate id, and true again for a distinct id.
    #[test]
    fn upsert_entry_if_new_deduplicates() {
        let d = NomDict::open_in_memory().unwrap();

        let a = make_entry("id-a", "alpha");
        assert!(d.upsert_entry_if_new(&a).unwrap(), "first insert must return true");
        assert_eq!(d.count().unwrap(), 1);

        // Same id again — INSERT OR IGNORE is a no-op.
        assert!(!d.upsert_entry_if_new(&a).unwrap(), "duplicate id must return false");
        assert_eq!(d.count().unwrap(), 1, "row count unchanged after duplicate");

        // Different id — fresh insert.
        let b = make_entry("id-b", "beta");
        assert!(d.upsert_entry_if_new(&b).unwrap(), "distinct id must return true");
        assert_eq!(d.count().unwrap(), 2);
    }

    /// §4.4.6: NomDict::find_by_body_kind filters to entries with the
    /// given canonical-format tag. Mirrors the resolver-side query.
    #[test]
    fn v2_find_by_body_kind() {
        use nom_types::body_kind;
        let d = NomDict::open_in_memory().unwrap();

        let mut bc_entry = make_entry("bc_abc", "encode_av1");
        bc_entry.body_kind = Some(body_kind::BC.to_owned());
        d.upsert_entry(&bc_entry).unwrap();

        let mut avif_entry = make_entry("av_def", "photo");
        avif_entry.body_kind = Some(body_kind::AVIF.to_owned());
        d.upsert_entry(&avif_entry).unwrap();

        let legacy = make_entry("leg_ghi", "untagged");
        d.upsert_entry(&legacy).unwrap();

        let bcs = d.find_by_body_kind(body_kind::BC, 10).unwrap();
        assert_eq!(bcs.len(), 1);
        assert_eq!(bcs[0].word, "encode_av1");

        let avifs = d.find_by_body_kind(body_kind::AVIF, 10).unwrap();
        assert_eq!(avifs.len(), 1);
        assert_eq!(avifs[0].word, "photo");

        let flacs = d.find_by_body_kind(body_kind::FLAC, 10).unwrap();
        assert!(flacs.is_empty());

        // Also confirms find_by_word works after the SELECT column-list fix.
        let by_word = d.find_by_word("encode_av1").unwrap();
        assert_eq!(by_word.len(), 1);
        assert_eq!(by_word[0].body_kind.as_deref(), Some(body_kind::BC));
    }

    /// §4.4.6: body_kind_histogram aggregates counts per tag, including
    /// the `(untagged)` bucket for NULL body_kind rows.
    #[test]
    fn body_kind_histogram_counts() {
        use nom_types::body_kind;
        let d = NomDict::open_in_memory().unwrap();

        let mut a = make_entry("a", "w1");
        a.body_kind = Some(body_kind::BC.to_owned());
        d.upsert_entry(&a).unwrap();

        let mut b = make_entry("b", "w2");
        b.body_kind = Some(body_kind::BC.to_owned());
        d.upsert_entry(&b).unwrap();

        let mut c = make_entry("c", "w3");
        c.body_kind = Some(body_kind::AVIF.to_owned());
        d.upsert_entry(&c).unwrap();

        // One legacy entry with NULL body_kind.
        d.upsert_entry(&make_entry("d", "w4")).unwrap();

        let hist = d.body_kind_histogram().unwrap();
        // Sorted by count desc: bc (2), avif (1), (untagged) (1).
        // Tiebreak: (untagged) before avif alphabetically.
        let as_map: std::collections::HashMap<String, usize> =
            hist.iter().cloned().collect();
        assert_eq!(as_map.get(body_kind::BC), Some(&2));
        assert_eq!(as_map.get(body_kind::AVIF), Some(&1));
        assert_eq!(as_map.get("(untagged)"), Some(&1));
        assert_eq!(hist[0].0, body_kind::BC); // highest count first
        assert_eq!(hist[0].1, 2);
    }

    /// `find_entries` filters correctly across language, status, body_kind,
    /// kind, and combinations thereof.
    #[test]
    fn find_entries_filters_correctly() {
        use nom_types::body_kind;
        let d = NomDict::open_in_memory().unwrap();

        // Entry 1: rust function, complete, body_kind=bc (compiled bitcode)
        let mut e1 = make_entry("aaa", "parse_input");
        e1.language = "rust".into();
        e1.status = EntryStatus::Complete;
        e1.kind = EntryKind::Function;
        e1.body_kind = Some(body_kind::BC.to_owned());
        d.upsert_entry(&e1).unwrap();

        // Entry 2: typescript module, partial, body_kind=bc
        let mut e2 = make_entry("bbb", "ui_module");
        e2.language = "typescript".into();
        e2.status = EntryStatus::Partial;
        e2.kind = EntryKind::Module;
        e2.body_kind = Some(body_kind::BC.to_owned());
        d.upsert_entry(&e2).unwrap();

        // Entry 3: python function, opaque, no body_kind
        let mut e3 = make_entry("ccc", "helper_fn");
        e3.language = "python".into();
        e3.status = EntryStatus::Opaque;
        e3.kind = EntryKind::Function;
        d.upsert_entry(&e3).unwrap();

        // --- filter by language ---
        let rust_entries = d.find_entries(&EntryFilter {
            language: Some("rust".into()),
            limit: 50,
            ..EntryFilter::default()
        }).unwrap();
        assert_eq!(rust_entries.len(), 1);
        assert_eq!(rust_entries[0].word, "parse_input");

        let ts_entries = d.find_entries(&EntryFilter {
            language: Some("typescript".into()),
            limit: 50,
            ..EntryFilter::default()
        }).unwrap();
        assert_eq!(ts_entries.len(), 1);
        assert_eq!(ts_entries[0].word, "ui_module");

        // --- filter by status ---
        let partial_entries = d.find_entries(&EntryFilter {
            status: Some(EntryStatus::Partial),
            limit: 50,
            ..EntryFilter::default()
        }).unwrap();
        assert_eq!(partial_entries.len(), 1);
        assert_eq!(partial_entries[0].word, "ui_module");

        let opaque_entries = d.find_entries(&EntryFilter {
            status: Some(EntryStatus::Opaque),
            limit: 50,
            ..EntryFilter::default()
        }).unwrap();
        assert_eq!(opaque_entries.len(), 1);
        assert_eq!(opaque_entries[0].word, "helper_fn");

        // --- filter by kind ---
        let fn_entries = d.find_entries(&EntryFilter {
            kind: Some(EntryKind::Function),
            limit: 50,
            ..EntryFilter::default()
        }).unwrap();
        assert_eq!(fn_entries.len(), 2);

        let mod_entries = d.find_entries(&EntryFilter {
            kind: Some(EntryKind::Module),
            limit: 50,
            ..EntryFilter::default()
        }).unwrap();
        assert_eq!(mod_entries.len(), 1);
        assert_eq!(mod_entries[0].word, "ui_module");

        // --- filter by body_kind ---
        let bc_entries = d.find_entries(&EntryFilter {
            body_kind: Some(body_kind::BC.to_owned()),
            limit: 50,
            ..EntryFilter::default()
        }).unwrap();
        assert_eq!(bc_entries.len(), 2); // e1 (parse_input) + e2 (ui_module) both BC

        // --- combined: language + status ---
        let rust_complete = d.find_entries(&EntryFilter {
            language: Some("rust".into()),
            status: Some(EntryStatus::Complete),
            limit: 50,
            ..EntryFilter::default()
        }).unwrap();
        assert_eq!(rust_complete.len(), 1);
        assert_eq!(rust_complete[0].word, "parse_input");

        // Combined: language + status that matches nothing
        let rust_partial = d.find_entries(&EntryFilter {
            language: Some("rust".into()),
            status: Some(EntryStatus::Partial),
            limit: 50,
            ..EntryFilter::default()
        }).unwrap();
        assert!(rust_partial.is_empty());

        // --- no filter = all 3 entries ---
        let all = d.find_entries(&EntryFilter { limit: 50, ..EntryFilter::default() }).unwrap();
        assert_eq!(all.len(), 3);

        // --- limit is respected ---
        let capped = d.find_entries(&EntryFilter { limit: 2, ..EntryFilter::default() }).unwrap();
        assert_eq!(capped.len(), 2);
    }

    #[test]
    fn list_partial_ids_returns_only_partials() {
        let d = NomDict::open_in_memory().unwrap();

        let mut p1 = make_entry("partial-aaa", "p1");
        p1.status = EntryStatus::Partial;
        d.upsert_entry(&p1).unwrap();

        let mut p2 = make_entry("partial-bbb", "p2");
        p2.status = EntryStatus::Partial;
        d.upsert_entry(&p2).unwrap();

        // Complete entry should not appear.
        let complete = make_entry("complete-ccc", "c1");
        d.upsert_entry(&complete).unwrap();

        let ids = d.list_partial_ids(None).unwrap();
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0], "partial-aaa");
        assert_eq!(ids[1], "partial-bbb");

        // Test max cap.
        let capped = d.list_partial_ids(Some(1)).unwrap();
        assert_eq!(capped.len(), 1);
        assert_eq!(capped[0], "partial-aaa");
    }
}
