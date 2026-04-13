//! SQLite-backed dictionary for the v2 content-addressed Nom store.
//!
//! `id = sha256(canonicalize(ast, contract))` is the sole identity
//! column on the `entries` table. Structured side tables hold scores,
//! signatures, findings, refs, graph edges and translations.
//! Unbounded metadata lives in the EAV `entry_meta` table.
//!
//! Layout: `data/nomdict.db`. WAL mode is enabled for concurrent reads.

pub mod freshness;

use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use nom_types::{
    Contract, EdgeType, Entry, EntryKind, EntryScores, EntrySignature, EntryStatus, GraphEdge,
    SecurityFinding, Severity, Translation,
};
use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};

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

CREATE TABLE IF NOT EXISTS concepts (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    describe    TEXT,
    created_at  TEXT DEFAULT (datetime('now')),
    updated_at  TEXT
);
CREATE INDEX IF NOT EXISTS idx_concepts_name ON concepts(name);

CREATE TABLE IF NOT EXISTS concept_members (
    concept_id  TEXT NOT NULL REFERENCES concepts(id) ON DELETE CASCADE,
    entry_id    TEXT NOT NULL REFERENCES entries(id),
    added_at    TEXT DEFAULT (datetime('now')),
    PRIMARY KEY (concept_id, entry_id)
);
CREATE INDEX IF NOT EXISTS idx_concept_members_entry ON concept_members(entry_id);
"#;

// ── Concept ──────────────────────────────────────────────────────────

/// A named concept grouping related nomtu entries by domain
/// (e.g. "cryptography", "web-server-handlers", "image-codecs").
/// Concept names are first-class Nom syntax tokens — a .nom file can
/// write `use cryptography@<hash>` to import an entire concept domain.
/// The `id` is the hex SHA-256 of the name (trimmed, as-is casing).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Concept {
    pub id: String,
    pub name: String,
    pub describe: Option<String>,
    pub created_at: String,
    pub updated_at: Option<String>,
}

impl Concept {
    /// Derive the canonical id for a concept name: `sha256(name.trim())`.
    pub fn id_for(name: &str) -> String {
        let hash = Sha256::digest(name.trim().as_bytes());
        format!("{hash:x}")
    }
}

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
        // Best-effort migration: rename old `drafts`/`draft_members` tables
        // (landed in commit 500eac7) to the new canonical names. SQLite's
        // ALTER TABLE RENAME TO is safe since 3.25. If the old tables don't
        // exist these are harmless no-ops (CREATE IF NOT EXISTS below handles
        // fresh DBs).
        let _ = self.conn
            .execute_batch("ALTER TABLE drafts RENAME TO concepts");
        let _ = self.conn
            .execute_batch("ALTER TABLE draft_members RENAME TO concept_members");
        self.conn.execute_batch(V2_SCHEMA_SQL)?;
        // Best-effort migration: add body_bytes to pre-existing DBs that were
        // created before this column was part of V2_SCHEMA_SQL. SQLite returns
        // "duplicate column name" when it already exists — ignore that error.
        let _ = self.conn
            .execute_batch("ALTER TABLE entries ADD COLUMN body_bytes BLOB");
        // Additive V3 tables: concept_defs (DB1) + words_v2 (DB2-v2).
        // CREATE TABLE IF NOT EXISTS makes this idempotent.
        self.conn.execute_batch(V3_SCHEMA_ADDITIONS_SQL)?;
        // Additive V4 tables: required_axes (M7a MECE CE-check registry).
        self.conn.execute_batch(V4_SCHEMA_ADDITIONS_SQL)?;
        // Additive V5 tables: dict_meta (freshness tracking, spec 2026-04-14).
        self.conn.execute_batch(V5_SCHEMA_ADDITIONS_SQL)?;
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

    /// Count of rows in `concept_defs` (DB1). Zero before any `nom store sync`.
    pub fn count_concept_defs(&self) -> Result<i64> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM concept_defs", [], |row| row.get(0))?;
        Ok(n)
    }

    /// Count of rows in `required_axes` (M7a registry). Zero until an axis
    /// is registered via `nom corpus register-axis`.
    pub fn count_required_axes(&self) -> Result<i64> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM required_axes", [], |row| row.get(0))?;
        Ok(n)
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

    /// Histogram of `status` values across the dict — Partial entries
    /// are the ones the LLM authoring loop needs to lift to Complete;
    /// the total-vs-Complete ratio is a dict-health metric. Sorted
    /// count desc with status-name tiebreaker for determinism.
    pub fn status_histogram(&self) -> Result<Vec<(String, usize)>> {
        let mut stmt = self.conn.prepare(
            "SELECT status AS s, COUNT(*) AS n
             FROM entries
             GROUP BY status
             ORDER BY n DESC, s ASC",
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

    /// Case-insensitive substring search on the `describe` column.
    /// Used by the MCP `search_nomtu` tool so an LLM can find entries
    /// by what they do (e.g. `query = "sha256"` returns anything whose
    /// describe mentions SHA-256 hashing).
    pub fn search_describe(&self, query: &str, limit: usize) -> Result<Vec<Entry>> {
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, word, variant, kind, language, describe, concept, body, body_nom,
                    input_type, output_type, pre, post, status, translation_score,
                    is_canonical, deprecated_by, created_at, updated_at, body_kind, body_bytes
             FROM entries
             WHERE describe LIKE ?1 COLLATE NOCASE
             ORDER BY id
             LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(params![pattern, limit as i64], row_to_entry)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Resolve a hash prefix (≥ 8 hex chars) to a full 64-char id.
    /// Returns `Ok(id)` on unique match, `Err(msg)` on not-found or ambiguous.
    pub fn resolve_prefix(&self, hash: &str) -> Result<String> {
        use anyhow::bail;
        if hash.len() < 8 {
            bail!("hash prefix too short (need ≥ 8 hex chars): {hash}");
        }
        if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            bail!("not a valid hex prefix: {hash}");
        }
        let pattern = format!("{hash}%");
        let mut stmt = self.conn.prepare_cached(
            "SELECT id FROM entries WHERE id LIKE ?1 ORDER BY id",
        )?;
        let ids: Vec<String> = stmt
            .query_map([pattern], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        match ids.len() {
            0 => bail!("no entry matching prefix {hash}"),
            1 => Ok(ids.into_iter().next().unwrap()),
            _ => {
                let candidates = ids.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
                bail!("hash prefix {hash} is ambiguous ({} candidates): {candidates}", ids.len())
            }
        }
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

    // ── Concept CRUD ───────────────────────────────────────────────

    /// Insert or replace a concept. The `id` is expected to be
    /// `Concept::id_for(&concept.name)`. Idempotent on repeated calls.
    pub fn upsert_concept(&self, concept: &Concept) -> Result<()> {
        self.conn.execute(
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

    /// Look up a concept by its human-readable name. Returns `None` if not found.
    pub fn get_concept_by_name(&self, name: &str) -> Result<Option<Concept>> {
        let row = self
            .conn
            .query_row(
                "SELECT id, name, describe, created_at, updated_at
                 FROM concepts WHERE name = ?1",
                params![name.trim()],
                row_to_concept,
            )
            .optional()?;
        Ok(row)
    }

    /// Return all concepts ordered alphabetically by name.
    pub fn list_concepts(&self) -> Result<Vec<Concept>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, name, describe, created_at, updated_at FROM concepts ORDER BY name",
        )?;
        let rows = stmt
            .query_map([], row_to_concept)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Delete a concept (and cascade-delete its membership rows).
    /// The referenced entries are NOT deleted — only the grouping is removed.
    pub fn delete_concept(&self, name: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM concepts WHERE name = ?1",
            params![name.trim()],
        )?;
        Ok(())
    }

    /// Add one entry to a concept. Uses `INSERT OR IGNORE` so it is safe
    /// to call on already-existing members. Returns `true` if the row
    /// was newly inserted, `false` if it was already present.
    pub fn add_concept_member(&self, concept_id: &str, entry_id: &str) -> Result<bool> {
        let changed = self.conn.execute(
            "INSERT OR IGNORE INTO concept_members (concept_id, entry_id) VALUES (?1, ?2)",
            params![concept_id, entry_id],
        )?;
        Ok(changed == 1)
    }

    /// Remove one entry from a concept (no-op if not a member).
    pub fn remove_concept_member(&self, concept_id: &str, entry_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM concept_members WHERE concept_id = ?1 AND entry_id = ?2",
            params![concept_id, entry_id],
        )?;
        Ok(())
    }

    /// Fetch all entries belonging to a concept, ordered by entry id.
    pub fn get_concept_members(&self, concept_id: &str) -> Result<Vec<Entry>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT e.id, e.word, e.variant, e.kind, e.language, e.describe, e.concept,
                    e.body, e.body_nom, e.input_type, e.output_type, e.pre, e.post,
                    e.status, e.translation_score, e.is_canonical, e.deprecated_by,
                    e.created_at, e.updated_at, e.body_kind, e.body_bytes
             FROM entries e
             JOIN concept_members cm ON cm.entry_id = e.id
             WHERE cm.concept_id = ?1
             ORDER BY e.id",
        )?;
        let rows = stmt
            .query_map(params![concept_id], row_to_entry)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Return the count of members in a concept.
    pub fn count_concept_members(&self, concept_id: &str) -> Result<usize> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM concept_members WHERE concept_id = ?1",
            params![concept_id],
            |row| row.get(0),
        )?;
        Ok(n as usize)
    }

    /// Bulk-add every entry matching `filter` to the concept identified by
    /// `concept_id`. Returns the count of rows newly inserted (entries
    /// already in the concept are silently skipped).
    pub fn add_concept_members_by_filter(
        &self,
        concept_id: &str,
        filter: &EntryFilter,
    ) -> Result<usize> {
        let entries = self.find_entries(filter)?;
        let tx = self.conn.unchecked_transaction()?;
        let mut added = 0usize;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT OR IGNORE INTO concept_members (concept_id, entry_id) VALUES (?1, ?2)",
            )?;
            for e in &entries {
                let changed = stmt.execute(params![concept_id, e.id])?;
                if changed == 1 {
                    added += 1;
                }
            }
        }
        tx.commit()?;
        Ok(added)
    }

    // ── concept_defs CRUD (DB1 — doc 08 §2.1) ─────────────────────────

    /// Insert or replace a `concept_defs` row. Idempotent: on conflict
    /// (`name` PK) all mutable fields are overwritten and `updated_at` is
    /// bumped.
    pub fn upsert_concept_def(&self, row: &ConceptRow) -> Result<()> {
        self.conn.execute(
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

    /// Fetch a `concept_defs` row by its primary key (`name`).
    pub fn find_concept_def(&self, name: &str) -> Result<Option<ConceptRow>> {
        let row = self
            .conn
            .query_row(
                "SELECT name, repo_id, intent, index_into_db2, exposes, acceptance,
                        objectives, src_path, src_hash, body_hash
                 FROM concept_defs WHERE name = ?1",
                params![name],
                row_to_concept_def,
            )
            .optional()?;
        Ok(row)
    }

    /// Return all `concept_defs` rows for a given `repo_id`, ordered by name.
    pub fn list_concept_defs_in_repo(&self, repo_id: &str) -> Result<Vec<ConceptRow>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT name, repo_id, intent, index_into_db2, exposes, acceptance,
                    objectives, src_path, src_hash, body_hash
             FROM concept_defs WHERE repo_id = ?1 ORDER BY name",
        )?;
        let rows = stmt
            .query_map(params![repo_id], row_to_concept_def)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    // ── required_axes CRUD (M7a — doc 08 §9.2) ───────────────────────

    /// Register (or replace) a required axis for a given repo + scope.
    ///
    /// Uses `INSERT OR REPLACE` semantics via the PRIMARY KEY
    /// `(repo_id, scope, axis)` — calling with the same key twice is a
    /// silent idempotent update.
    ///
    /// Validation:
    /// - `scope` must be one of `"app"`, `"concept"`, `"module"`.
    /// - `cardinality` must be one of `"at_least_one"`, `"exactly_one"`.
    /// - `axis` is normalised to `axis.trim().to_ascii_lowercase()`.
    pub fn register_required_axis(
        &self,
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
        self.conn.execute(
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
    /// ordered by axis for determinism.
    pub fn list_required_axes(
        &self,
        repo_id: &str,
        scope: &str,
    ) -> rusqlite::Result<Vec<RequiredAxis>> {
        let mut stmt = self.conn.prepare_cached(
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

    /// Delete a `required_axes` row. Returns `true` if a row was deleted,
    /// `false` if no matching row existed.
    pub fn unregister_required_axis(
        &self,
        repo_id: &str,
        scope: &str,
        axis: &str,
    ) -> rusqlite::Result<bool> {
        let axis_norm = axis.trim().to_ascii_lowercase();
        let n = self.conn.execute(
            "DELETE FROM required_axes WHERE repo_id = ?1 AND scope = ?2 AND axis = ?3",
            rusqlite::params![repo_id, scope, axis_norm],
        )?;
        Ok(n > 0)
    }

    /// Seed the M7b canonical "standard required axes" set into this
    /// dict's `required_axes` registry for the given `repo_id`. Idempotent
    /// (uses `register_required_axis`'s INSERT OR REPLACE semantics); safe
    /// to call repeatedly.
    ///
    /// The five axes — `correctness`, `safety`, `performance`,
    /// `dependency`, `documentation` — are the default MECE set for any
    /// app-scope concept. Authors can override by calling
    /// `unregister_required_axis` before their own `register_required_axis`,
    /// or by using a different `repo_id`.
    ///
    /// Returns the list of `(scope, axis, cardinality)` tuples that were
    /// written so the caller can display what landed.
    pub fn seed_standard_axes(
        &self,
        repo_id: &str,
    ) -> rusqlite::Result<Vec<(String, String, String)>> {
        const STANDARD: &[(&str, &str, &str)] = &[
            // (scope, axis, cardinality)
            ("app", "correctness", "at_least_one"),
            ("app", "safety", "at_least_one"),
            ("app", "performance", "at_least_one"),
            ("app", "dependency", "at_least_one"),
            ("app", "documentation", "at_least_one"),
        ];
        let mut seeded = Vec::with_capacity(STANDARD.len());
        for (scope, axis, cardinality) in STANDARD {
            self.register_required_axis(repo_id, scope, axis, cardinality)?;
            seeded.push((scope.to_string(), axis.to_string(), cardinality.to_string()));
        }
        Ok(seeded)
    }

    // ── words_v2 CRUD (DB2-v2 — doc 08 §2.2) ──────────────────────────

    /// Insert or replace a `words_v2` row. Idempotent: on conflict
    /// (`hash` PK) all mutable fields are overwritten and `updated_at` is
    /// bumped.
    pub fn upsert_word_v2(&self, row: &WordV2Row) -> Result<()> {
        self.conn.execute(
            "INSERT INTO words_v2
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

    /// Fetch a `words_v2` row by its `hash` primary key.
    pub fn find_word_v2(&self, hash: &str) -> Result<Option<WordV2Row>> {
        let row = self
            .conn
            .query_row(
                "SELECT hash, word, kind, signature, contracts, body_kind, body_size,
                        origin_ref, bench_ids, authored_in, composed_of
                 FROM words_v2 WHERE hash = ?1",
                params![hash],
                row_to_word_v2,
            )
            .optional()?;
        Ok(row)
    }

    /// Return all `words_v2` rows whose `word` column equals `word`,
    /// ordered by hash for determinism.
    pub fn find_words_v2_by_word(&self, word: &str) -> Result<Vec<WordV2Row>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT hash, word, kind, signature, contracts, body_kind, body_size,
                    origin_ref, bench_ids, authored_in, composed_of
             FROM words_v2 WHERE word = ?1 ORDER BY hash",
        )?;
        let rows = stmt
            .query_map(params![word], row_to_word_v2)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Return all `words_v2` rows whose `kind` column equals `kind`,
    /// ordered by hash for determinism (§10.3.1 alphabetical-smallest tiebreak).
    pub fn find_words_v2_by_kind(&self, kind: &str) -> Result<Vec<WordV2Row>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT hash, word, kind, signature, contracts, body_kind, body_size,
                    origin_ref, bench_ids, authored_in, composed_of
             FROM words_v2 WHERE kind = ?1 ORDER BY hash",
        )?;
        let rows = stmt
            .query_map(params![kind], row_to_word_v2)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Total count of rows in `words_v2`.
    pub fn count_words_v2(&self) -> Result<i64> {
        let n: i64 =
            self.conn.query_row("SELECT COUNT(*) FROM words_v2", [], |row| row.get(0))?;
        Ok(n)
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

fn row_to_concept(row: &rusqlite::Row) -> rusqlite::Result<Concept> {
    Ok(Concept {
        id: row.get(0)?,
        name: row.get(1)?,
        describe: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

fn row_to_concept_def(row: &rusqlite::Row) -> rusqlite::Result<ConceptRow> {
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
}

fn row_to_word_v2(row: &rusqlite::Row) -> rusqlite::Result<WordV2Row> {
    Ok(WordV2Row {
        hash: row.get(0)?,
        word: row.get(1)?,
        kind: row.get(2)?,
        signature: row.get(3)?,
        contracts: row.get(4)?,
        body_kind: row.get(5)?,
        body_size: row.get(6)?,
        origin_ref: row.get(7)?,
        bench_ids: row.get(8)?,
        authored_in: row.get(9)?,
        composed_of: row.get(10)?,
    })
}

// Re-export so callers can construct edges without another use-line.
pub use nom_types::EdgeType as __ReexportedEdgeType;
#[allow(dead_code)]
fn _compile_check(_: EdgeType) {}

// ── V3 schema additions (additive — DB1 concept_defs + DB2 words_v2) ──

/// Additive SQL appended by `init_schema` after `V2_SCHEMA_SQL`.
/// Does NOT modify any existing table. Uses `CREATE TABLE IF NOT EXISTS`
/// so it is safe to call on a DB that already has these tables.
///
/// `concept_defs` = DB1 (doc 08 §2.1): one row per `.nom` concept file.
/// `words_v2`     = DB2-v2 (doc 08 §2.2): one row per nomtu hash.
///
/// Note: the legacy `concepts` table (entry-grouping, id PK) is kept
/// unchanged.  The new DB1 table is named `concept_defs` to avoid the
/// name collision.
pub const V3_SCHEMA_ADDITIONS_SQL: &str = r#"
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
"#;

// ── V4 schema additions (additive — M7a required_axes registry) ─────

/// Additive SQL appended by `init_schema` after `V3_SCHEMA_ADDITIONS_SQL`.
/// Does NOT modify any existing table. Uses `CREATE TABLE IF NOT EXISTS`
/// so it is safe to call on a DB that already has these tables.
///
/// `required_axes` = M7a (doc 08 §9.2): per-scope required quality axes.
pub const V4_SCHEMA_ADDITIONS_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS required_axes (
    axis          TEXT NOT NULL,
    scope         TEXT NOT NULL,
    cardinality   TEXT NOT NULL,
    repo_id       TEXT NOT NULL,
    registered_at TEXT NOT NULL,
    PRIMARY KEY (repo_id, scope, axis)
);
CREATE INDEX IF NOT EXISTS idx_required_axes_repo_scope ON required_axes(repo_id, scope);
"#;

/// V5 additions: `dict_meta` key-value table for dict-level state that isn't
/// per-entry (the `entry_meta` table covers per-entry metadata). Phase 1 of
/// the graph-durability spec (docs/superpowers/specs/2026-04-14-graph-
/// durability-design.md) uses `dict_last_source_hash` to track whether the
/// dict is fresh against the working-tree source files.
pub const V5_SCHEMA_ADDITIONS_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS dict_meta (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
"#;

// ── RequiredAxis (M7a — doc 08 §9.2) ────────────────────────────────

/// One row in the `required_axes` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredAxis {
    pub repo_id: String,
    pub scope: String,
    pub axis: String,
    pub cardinality: String,
    pub registered_at: String,
}

// ── ConceptRow (DB1 — doc 08 §2.1) ──────────────────────────────────

/// One row in the `concept_defs` table.
/// JSON fields (`index_into_db2`, `exposes`, `acceptance`, `objectives`)
/// are stored as raw strings — nom-dict has no dependency on nom-concept;
/// the caller is responsible for serialisation/deserialisation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConceptRow {
    pub name: String,
    pub repo_id: String,
    pub intent: String,
    /// JSON array of EntityRef.
    pub index_into_db2: String,
    /// JSON array; default `"[]"`.
    pub exposes: String,
    /// JSON array; default `"[]"`.
    pub acceptance: String,
    /// JSON array; default `"[]"`.
    pub objectives: String,
    pub src_path: String,
    pub src_hash: String,
    pub body_hash: Option<String>,
}

// ── WordV2Row (DB2-v2 — doc 08 §2.2) ────────────────────────────────

/// One row in the `words_v2` table.
/// All `Option` fields map to nullable SQL columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordV2Row {
    pub hash: String,
    pub word: String,
    pub kind: String,
    pub signature: Option<String>,
    /// JSON array of ContractClause; nullable.
    pub contracts: Option<String>,
    pub body_kind: Option<String>,
    pub body_size: Option<i64>,
    pub origin_ref: Option<String>,
    pub bench_ids: Option<String>,
    /// Path to .nomtu file; NULL if ingested from corpus.
    pub authored_in: Option<String>,
    /// JSON list of hashes; NULL if atomic entry.
    pub composed_of: Option<String>,
}

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

    /// `status_histogram` aggregates counts per status with deterministic
    /// sort (count desc, status-name asc tiebreaker).
    #[test]
    fn status_histogram_counts() {
        use nom_types::EntryStatus;
        let d = NomDict::open_in_memory().unwrap();

        // 2 Complete, 3 Partial, 1 Opaque.
        let mk = |id: &str, s: EntryStatus| {
            let mut e = make_entry(id, id);
            e.status = s;
            e
        };
        d.upsert_entry(&mk("c1", EntryStatus::Complete)).unwrap();
        d.upsert_entry(&mk("c2", EntryStatus::Complete)).unwrap();
        d.upsert_entry(&mk("p1", EntryStatus::Partial)).unwrap();
        d.upsert_entry(&mk("p2", EntryStatus::Partial)).unwrap();
        d.upsert_entry(&mk("p3", EntryStatus::Partial)).unwrap();
        d.upsert_entry(&mk("o1", EntryStatus::Opaque)).unwrap();

        let hist = d.status_histogram().unwrap();
        let as_map: std::collections::HashMap<String, usize> = hist.iter().cloned().collect();
        assert_eq!(as_map.get("partial"), Some(&3));
        assert_eq!(as_map.get("complete"), Some(&2));
        assert_eq!(as_map.get("opaque"), Some(&1));
        // Partial is highest; tiebreak kicks in only on equal counts.
        assert_eq!(hist[0].0, "partial");
        assert_eq!(hist[0].1, 3);
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

    // ── Concept tests ─────────────────────────────────────────────────

    fn make_concept(name: &str) -> Concept {
        Concept {
            id: Concept::id_for(name),
            name: name.to_string(),
            describe: None,
            created_at: "2026-04-12T00:00:00Z".to_string(),
            updated_at: None,
        }
    }

    #[test]
    fn upsert_concept_roundtrips() {
        let d = NomDict::open_in_memory().unwrap();
        let concept = Concept {
            id: Concept::id_for("cryptography"),
            name: "cryptography".to_string(),
            describe: Some("Hashing, signing, and encryption entries".to_string()),
            created_at: "2026-04-12T00:00:00Z".to_string(),
            updated_at: None,
        };
        d.upsert_concept(&concept).unwrap();

        let fetched = d.get_concept_by_name("cryptography").unwrap().unwrap();
        assert_eq!(fetched.id, concept.id);
        assert_eq!(fetched.name, "cryptography");
        assert_eq!(fetched.describe.as_deref(), Some("Hashing, signing, and encryption entries"));

        let all = d.list_concepts().unwrap();
        assert_eq!(all.len(), 1);

        // Upsert again — idempotent, still only one row.
        d.upsert_concept(&concept).unwrap();
        let all2 = d.list_concepts().unwrap();
        assert_eq!(all2.len(), 1);
    }

    #[test]
    fn add_concept_member_and_list() {
        let d = NomDict::open_in_memory().unwrap();

        let entry = make_entry("entry-aaa", "sha256_hash");
        d.upsert_entry(&entry).unwrap();

        let concept = make_concept("crypto");
        d.upsert_concept(&concept).unwrap();

        // First add returns true (newly inserted).
        let added = d.add_concept_member(&concept.id, "entry-aaa").unwrap();
        assert!(added, "first add must return true");

        // Second add is a no-op.
        let added2 = d.add_concept_member(&concept.id, "entry-aaa").unwrap();
        assert!(!added2, "duplicate add must return false");

        // Count reflects exactly 1.
        assert_eq!(d.count_concept_members(&concept.id).unwrap(), 1);

        // get_concept_members returns the entry.
        let members = d.get_concept_members(&concept.id).unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].word, "sha256_hash");

        // Remove and verify gone.
        d.remove_concept_member(&concept.id, "entry-aaa").unwrap();
        assert_eq!(d.count_concept_members(&concept.id).unwrap(), 0);
    }

    #[test]
    fn delete_concept_cascades_members() {
        let d = NomDict::open_in_memory().unwrap();

        let entry = make_entry("e-x", "some_fn");
        d.upsert_entry(&entry).unwrap();

        let concept = make_concept("image-codecs");
        d.upsert_concept(&concept).unwrap();
        d.add_concept_member(&concept.id, "e-x").unwrap();
        assert_eq!(d.count_concept_members(&concept.id).unwrap(), 1);

        // Deleting the concept must cascade-delete member rows.
        d.delete_concept("image-codecs").unwrap();
        assert!(d.get_concept_by_name("image-codecs").unwrap().is_none());
        // Entry itself must still exist.
        assert!(d.get_entry("e-x").unwrap().is_some());
        // Membership count query for the (now-deleted) concept id returns 0.
        assert_eq!(d.count_concept_members(&concept.id).unwrap(), 0);
    }

    #[test]
    fn add_concept_members_by_filter_dedupes() {
        let d = NomDict::open_in_memory().unwrap();

        for (id, lang) in [("r1", "rust"), ("r2", "rust"), ("py1", "python")] {
            let mut e = make_entry(id, id);
            e.language = lang.to_string();
            d.upsert_entry(&e).unwrap();
        }

        let concept = make_concept("rust-domain");
        d.upsert_concept(&concept).unwrap();

        let filter = EntryFilter {
            language: Some("rust".to_string()),
            limit: 50,
            ..EntryFilter::default()
        };
        let added = d.add_concept_members_by_filter(&concept.id, &filter).unwrap();
        assert_eq!(added, 2, "two rust entries should be added");
        assert_eq!(d.count_concept_members(&concept.id).unwrap(), 2);

        // Running again must not double-count.
        let added2 = d.add_concept_members_by_filter(&concept.id, &filter).unwrap();
        assert_eq!(added2, 0, "re-run must add 0 (all already members)");
        assert_eq!(d.count_concept_members(&concept.id).unwrap(), 2);
    }

    // ── DB1 / DB2-v2 tests ────────────────────────────────────────────

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

    fn make_word_v2_row(hash: &str, word: &str) -> WordV2Row {
        WordV2Row {
            hash: hash.to_string(),
            word: word.to_string(),
            kind: "Function".to_string(),
            signature: Some("(token: str) -> bool".to_string()),
            contracts: Some(r#"[{"pre":"token != null"}]"#.to_string()),
            body_kind: Some("bc".to_string()),
            body_size: Some(1024),
            origin_ref: Some("repo:myproject".to_string()),
            bench_ids: Some(r#"["bench-001"]"#.to_string()),
            authored_in: Some("src/auth.nomtu".to_string()),
            composed_of: None,
        }
    }

    /// Test 1: init_schema creates concept_defs and words_v2 tables.
    #[test]
    fn init_schema_creates_concept_defs_and_words_v2_tables() {
        let d = NomDict::open_in_memory().unwrap();
        let conn = d.connection();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(
            tables.contains(&"concept_defs".to_string()),
            "concept_defs table must exist; got: {tables:?}"
        );
        assert!(
            tables.contains(&"words_v2".to_string()),
            "words_v2 table must exist; got: {tables:?}"
        );
    }

    /// Test 2: running init twice must not error.
    #[test]
    fn concepts_idempotent_init() {
        let d = NomDict::open_in_memory().unwrap();
        // init_schema is called inside open_in_memory; calling it a second time
        // via execute_batch of the same SQL must be a no-op (CREATE IF NOT EXISTS).
        d.connection().execute_batch(V3_SCHEMA_ADDITIONS_SQL).unwrap();
    }

    /// Test 3: insert a ConceptRow and read it back identically.
    #[test]
    fn concept_def_round_trip() {
        let d = NomDict::open_in_memory().unwrap();
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
        d.upsert_concept_def(&row).unwrap();

        let fetched = d.find_concept_def("auth_system").unwrap().unwrap();
        assert_eq!(fetched.name, row.name);
        assert_eq!(fetched.repo_id, row.repo_id);
        assert_eq!(fetched.intent, row.intent);
        assert_eq!(fetched.index_into_db2, row.index_into_db2);
        assert_eq!(fetched.exposes, row.exposes);
        assert_eq!(fetched.acceptance, row.acceptance);
        assert_eq!(fetched.objectives, row.objectives);
        assert_eq!(fetched.src_path, row.src_path);
        assert_eq!(fetched.src_hash, row.src_hash);
        assert_eq!(fetched.body_hash, row.body_hash);
    }

    /// Test 4: upsert with a new intent overwrites the old one.
    #[test]
    fn concept_def_upsert_overwrites() {
        let d = NomDict::open_in_memory().unwrap();
        let mut row = make_concept_row("payments", "repo-pay");
        d.upsert_concept_def(&row).unwrap();

        let original = d.find_concept_def("payments").unwrap().unwrap();
        assert_eq!(original.intent, "intent of payments");

        row.intent = "Process Stripe + PayPal transactions".to_string();
        d.upsert_concept_def(&row).unwrap();

        let updated = d.find_concept_def("payments").unwrap().unwrap();
        assert_eq!(updated.intent, "Process Stripe + PayPal transactions");

        // Still only one row.
        let all = d.list_concept_defs_in_repo("repo-pay").unwrap();
        assert_eq!(all.len(), 1);
    }

    /// Test 5: insert a WordV2Row with all fields populated; read back identically.
    #[test]
    fn word_v2_round_trip() {
        let d = NomDict::open_in_memory().unwrap();
        let row = make_word_v2_row("cafebabe1234", "validate_token_jwt");
        d.upsert_word_v2(&row).unwrap();

        let fetched = d.find_word_v2("cafebabe1234").unwrap().unwrap();
        assert_eq!(fetched.hash, row.hash);
        assert_eq!(fetched.word, row.word);
        assert_eq!(fetched.kind, row.kind);
        assert_eq!(fetched.signature, row.signature);
        assert_eq!(fetched.contracts, row.contracts);
        assert_eq!(fetched.body_kind, row.body_kind);
        assert_eq!(fetched.body_size, row.body_size);
        assert_eq!(fetched.origin_ref, row.origin_ref);
        assert_eq!(fetched.bench_ids, row.bench_ids);
        assert_eq!(fetched.authored_in, row.authored_in);
        assert_eq!(fetched.composed_of, row.composed_of);

        assert_eq!(d.count_words_v2().unwrap(), 1);
    }

    /// Test 6: find_words_v2_by_word returns only rows matching the word.
    #[test]
    fn find_words_v2_by_word_filters_correctly() {
        let d = NomDict::open_in_memory().unwrap();

        d.upsert_word_v2(&make_word_v2_row("hash-jwt-1", "validate_token_jwt")).unwrap();
        d.upsert_word_v2(&make_word_v2_row("hash-jwt-2", "validate_token_jwt")).unwrap();
        d.upsert_word_v2(&make_word_v2_row("hash-other", "other")).unwrap();

        let jwt_rows = d.find_words_v2_by_word("validate_token_jwt").unwrap();
        assert_eq!(jwt_rows.len(), 2, "expected 2 rows for validate_token_jwt");
        assert!(jwt_rows.iter().all(|r| r.word == "validate_token_jwt"));

        let other_rows = d.find_words_v2_by_word("other").unwrap();
        assert_eq!(other_rows.len(), 1);

        let missing = d.find_words_v2_by_word("nonexistent").unwrap();
        assert!(missing.is_empty());

        assert_eq!(d.count_words_v2().unwrap(), 3);
    }

    /// Test 6b: find_words_v2_by_kind returns only rows matching the kind.
    #[test]
    fn find_words_v2_by_kind_filters_correctly() {
        let d = NomDict::open_in_memory().unwrap();

        // Two rows with kind="function" (default from make_word_v2_row), one with kind="screen".
        d.upsert_word_v2(&make_word_v2_row("hash-fn-a", "auth_user")).unwrap();
        d.upsert_word_v2(&make_word_v2_row("hash-fn-b", "validate_token")).unwrap();
        let mut screen_row = make_word_v2_row("hash-sc-1", "login_screen");
        screen_row.kind = "screen".to_string();
        d.upsert_word_v2(&screen_row).unwrap();

        let fn_rows = d.find_words_v2_by_kind("Function").unwrap();
        assert_eq!(fn_rows.len(), 2, "expected 2 rows for kind=Function, got {}", fn_rows.len());
        assert!(fn_rows.iter().all(|r| r.kind == "Function"));

        let sc_rows = d.find_words_v2_by_kind("screen").unwrap();
        assert_eq!(sc_rows.len(), 1, "expected 1 row for kind=screen");

        let missing = d.find_words_v2_by_kind("nonexistent_kind").unwrap();
        assert!(missing.is_empty());

        // Rows are ordered by hash for determinism.
        assert_eq!(fn_rows[0].hash, "hash-fn-a");
        assert_eq!(fn_rows[1].hash, "hash-fn-b");

        assert_eq!(d.count_words_v2().unwrap(), 3);
    }

    /// Test 7: the legacy `entries` table is unaffected by the additive changes.
    #[test]
    fn existing_entries_table_unaffected() {
        let d = NomDict::open_in_memory().unwrap();

        // Insert via the existing high-level API to confirm nothing broke.
        d.upsert_entry(&make_entry("entry-legacy-001", "old_fn")).unwrap();

        // Also confirm via raw SQL count.
        let count: i64 = d
            .connection()
            .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1, "entries table must still work after additive schema additions");

        // And the new tables start empty.
        assert_eq!(d.count_words_v2().unwrap(), 0);
        assert_eq!(
            d.list_concept_defs_in_repo("any-repo").unwrap().len(),
            0
        );
    }

    // ── M7a required_axes tests ───────────────────────────────────────

    /// Test RA-1: register and list round-trips correctly.
    #[test]
    fn register_and_list_roundtrips() {
        let d = NomDict::open_in_memory().unwrap();
        d.register_required_axis("repo-a", "concept", "security", "at_least_one").unwrap();
        d.register_required_axis("repo-a", "concept", "safety", "exactly_one").unwrap();

        let axes = d.list_required_axes("repo-a", "concept").unwrap();
        assert_eq!(axes.len(), 2, "expected 2 axes, got: {axes:?}");

        // Sorted by axis name: safety < security.
        assert_eq!(axes[0].axis, "safety");
        assert_eq!(axes[0].cardinality, "exactly_one");
        assert_eq!(axes[0].scope, "concept");
        assert_eq!(axes[0].repo_id, "repo-a");

        assert_eq!(axes[1].axis, "security");
        assert_eq!(axes[1].cardinality, "at_least_one");

        // Different scope is isolated.
        let app_axes = d.list_required_axes("repo-a", "app").unwrap();
        assert!(app_axes.is_empty(), "app scope must be empty");
    }

    /// Test RA-2: unknown scope is rejected.
    #[test]
    fn register_rejects_unknown_scope() {
        let d = NomDict::open_in_memory().unwrap();
        let err = d
            .register_required_axis("repo-x", "planet", "correctness", "at_least_one")
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("planet") || msg.contains("scope"),
            "error must mention invalid scope: {msg}"
        );
    }

    /// Test RA-3: unknown cardinality is rejected.
    #[test]
    fn register_rejects_unknown_cardinality() {
        let d = NomDict::open_in_memory().unwrap();
        let err = d
            .register_required_axis("repo-x", "app", "speed", "exactly_two")
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("exactly_two") || msg.contains("cardinality"),
            "error must mention invalid cardinality: {msg}"
        );
    }

    /// Test RA-4: axis is stored as trimmed lowercase; duplicate registrations
    ///            with different casing overwrite the same row.
    #[test]
    fn register_normalizes_axis_to_lowercase_and_trim() {
        let d = NomDict::open_in_memory().unwrap();
        d.register_required_axis("repo-b", "module", " Security ", "at_least_one").unwrap();

        let axes = d.list_required_axes("repo-b", "module").unwrap();
        assert_eq!(axes.len(), 1);
        assert_eq!(axes[0].axis, "security", "axis must be stored in lowercase");

        // Re-register with different casing + new cardinality → same row updated.
        d.register_required_axis("repo-b", "module", "SECURITY", "exactly_one").unwrap();
        let axes2 = d.list_required_axes("repo-b", "module").unwrap();
        assert_eq!(axes2.len(), 1, "must still be exactly one row after re-registration");
        assert_eq!(axes2[0].cardinality, "exactly_one");
    }

    /// Test RA-5: unregister returns false when the row does not exist.
    #[test]
    fn unregister_returns_false_for_missing_row() {
        let d = NomDict::open_in_memory().unwrap();
        let deleted = d.unregister_required_axis("repo-z", "app", "nonexistent").unwrap();
        assert!(!deleted, "must return false when no row matches");

        // Register then unregister → true, then false.
        d.register_required_axis("repo-z", "app", "performance", "at_least_one").unwrap();
        let deleted2 = d.unregister_required_axis("repo-z", "app", "performance").unwrap();
        assert!(deleted2, "must return true after deleting existing row");

        let deleted3 = d.unregister_required_axis("repo-z", "app", "performance").unwrap();
        assert!(!deleted3, "second delete must return false");
    }

    #[test]
    fn seed_standard_axes_writes_canonical_five_axes() {
        let d = NomDict::open_in_memory().unwrap();
        let seeded = d.seed_standard_axes("app-seed").unwrap();

        // Five axes: correctness, safety, performance, dependency, documentation.
        assert_eq!(seeded.len(), 5, "must seed exactly 5 axes");

        let axes: std::collections::HashSet<&str> =
            seeded.iter().map(|(_, a, _)| a.as_str()).collect();
        for expected in
            ["correctness", "safety", "performance", "dependency", "documentation"]
        {
            assert!(axes.contains(expected), "missing {expected} in seeded set");
        }

        // All at app-scope with at_least_one cardinality.
        for (scope, _, card) in &seeded {
            assert_eq!(scope, "app");
            assert_eq!(card, "at_least_one");
        }

        // Rows visible via list_required_axes.
        let listed = d.list_required_axes("app-seed", "app").unwrap();
        assert_eq!(listed.len(), 5);
    }

    #[test]
    fn seed_standard_axes_is_idempotent() {
        let d = NomDict::open_in_memory().unwrap();
        d.seed_standard_axes("app-idem").unwrap();
        d.seed_standard_axes("app-idem").unwrap();
        d.seed_standard_axes("app-idem").unwrap();
        let rows = d.list_required_axes("app-idem", "app").unwrap();
        assert_eq!(rows.len(), 5, "must stay at 5 after repeated seeds");
    }

    #[test]
    fn seed_standard_axes_scoped_per_repo_id() {
        let d = NomDict::open_in_memory().unwrap();
        d.seed_standard_axes("alpha").unwrap();
        d.seed_standard_axes("beta").unwrap();
        assert_eq!(d.list_required_axes("alpha", "app").unwrap().len(), 5);
        assert_eq!(d.list_required_axes("beta", "app").unwrap().len(), 5);
        assert_eq!(d.count_required_axes().unwrap(), 10);
    }
}
