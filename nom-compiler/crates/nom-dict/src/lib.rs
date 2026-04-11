//! SQLite-backed atom dictionary using the unified .nomtu format.
//!
//! The `nomtu` table stores word/variant naming, typed signatures,
//! quality scores, and canonicality. WAL mode enables concurrent reads.
//!
//! Layout: `data/nomdict.db` (single file, ~500 bytes per entry)

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use nom_types::{Atom, parse_atom_kind};
use rusqlite::{Connection, params};

/// SQLite-backed atom dictionary.
pub struct NomDict {
    conn: Connection,
    root: PathBuf,
}

/// Summary of a store operation.
#[derive(Debug)]
pub struct StoreResult {
    pub stored: usize,
    pub deduplicated: usize,
    pub total: usize,
}

impl NomDict {
    /// Open or create the SQLite atom dictionary.
    pub fn open(root: &Path) -> Result<Self> {
        let db_dir = root.join("data");
        std::fs::create_dir_all(&db_dir)?;
        let db_path = db_dir.join("nomdict.db");
        let conn = Connection::open(&db_path)
            .with_context(|| format!("failed to open NomDict at {}", db_path.display()))?;

        // WAL mode for concurrent reads
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "cache_size", "-64000")?; // 64MB cache
        conn.busy_timeout(std::time::Duration::from_secs(30))?;

        let dict = Self {
            conn,
            root: root.to_path_buf(),
        };
        dict.create_tables()?;
        Ok(dict)
    }

    fn create_tables(&self) -> Result<()> {
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
            CREATE INDEX IF NOT EXISTS idx_nomtu_kind ON nomtu(kind);
            CREATE INDEX IF NOT EXISTS idx_nomtu_language ON nomtu(language);
            CREATE INDEX IF NOT EXISTS idx_nomtu_word_variant ON nomtu(word, variant);
            CREATE INDEX IF NOT EXISTS idx_nomtu_concept ON nomtu(concept);
            CREATE INDEX IF NOT EXISTS idx_nomtu_hash ON nomtu(hash);
            CREATE INDEX IF NOT EXISTS idx_nomtu_source_repo ON nomtu(source_repo);
            CREATE INDEX IF NOT EXISTS idx_nomtu_overall_score ON nomtu(overall_score);
            CREATE INDEX IF NOT EXISTS idx_nomtu_community ON nomtu(community_id);
            ",
        )?;
        Ok(())
    }

    /// Content hash for deduplication.
    fn content_hash(atom: &Atom) -> String {
        let key = format!(
            "{}:{}:{}:{}",
            atom.kind.as_str(),
            atom.name,
            atom.language,
            atom.source_path
        );
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in key.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{hash:016x}")
    }

    /// Build a description from atom fields.
    fn describe(atom: &Atom) -> String {
        match &atom.concept {
            Some(concept) => format!("implementation of {}: {}", concept, atom.name),
            None => format!("implementation of {}", atom.name),
        }
    }

    /// Extract input_type from signature (param types joined).
    fn extract_input_type(atom: &Atom) -> Option<String> {
        atom.signature.as_ref().map(|sig| {
            sig.params
                .iter()
                .map(|(_, ty)| ty.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
    }

    /// Extract output_type from signature.
    fn extract_output_type(atom: &Atom) -> Option<String> {
        atom.signature.as_ref().and_then(|sig| sig.returns.clone())
    }

    /// Extract effects from signature (async, method, etc.).
    fn extract_effects(atom: &Atom) -> String {
        let mut effects = Vec::new();
        if let Some(sig) = &atom.signature {
            if sig.is_async {
                effects.push("async");
            }
            if sig.is_method {
                effects.push("method");
            }
        }
        serde_json::to_string(&effects).unwrap_or_else(|_| "[]".to_string())
    }

    /// Store atoms with deduplication. Uses a transaction for speed.
    pub fn store_atoms(&self, atoms: &[Atom]) -> Result<StoreResult> {
        let mut stored = 0;
        let mut deduped = 0;

        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT OR IGNORE INTO nomtu
                 (word, variant, kind, hash, describe, concept, labels,
                  input_type, output_type, effects, signature,
                  source_path, language, body)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            )?;

            for atom in atoms {
                let hash = Self::content_hash(atom);
                let describe = Self::describe(atom);
                let labels_json = serde_json::to_string(&atom.labels)?;
                let input_type = Self::extract_input_type(atom);
                let output_type = Self::extract_output_type(atom);
                let effects = Self::extract_effects(atom);
                let sig_json = atom
                    .signature
                    .as_ref()
                    .map(serde_json::to_string)
                    .transpose()?;

                let rows = stmt.execute(params![
                    atom.name,
                    &hash,
                    atom.kind.as_str(),
                    hash,
                    describe,
                    atom.concept,
                    labels_json,
                    input_type,
                    output_type,
                    effects,
                    sig_json,
                    atom.source_path,
                    atom.language,
                    atom.body,
                ])?;

                if rows > 0 {
                    stored += 1;
                } else {
                    deduped += 1;
                }
            }
        }
        tx.commit()?;

        let total = self.count()?;
        Ok(StoreResult {
            stored,
            deduplicated: deduped,
            total,
        })
    }

    /// Count total entries in the nomtu table.
    pub fn count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM nomtu", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Query atoms by concept/variant (indexed).
    pub fn query_by_concept(&self, concept: &str) -> Result<Vec<Atom>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT * FROM nomtu WHERE variant = ?1")?;
        let atoms = stmt
            .query_map(params![concept], |row| Ok(Self::row_to_atom(row)))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(atoms)
    }

    /// Query atoms by kind (indexed).
    pub fn query_by_kind(&self, kind: &str) -> Result<Vec<Atom>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT * FROM nomtu WHERE kind = ?1")?;
        let atoms = stmt
            .query_map(params![kind], |row| Ok(Self::row_to_atom(row)))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(atoms)
    }

    /// Search atoms by word/variant/kind (LIKE matching).
    pub fn search(&self, query: &str) -> Result<Vec<Atom>> {
        let pattern = format!("%{query}%");
        let mut stmt = self.conn.prepare_cached(
            "SELECT * FROM nomtu WHERE word LIKE ?1 OR variant LIKE ?1 OR kind LIKE ?1 LIMIT 100",
        )?;
        let atoms = stmt
            .query_map(params![pattern], |row| Ok(Self::row_to_atom(row)))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(atoms)
    }

    /// Load atoms by multiple concepts/variants (for composition solver).
    pub fn load_by_concepts(&self, concepts: &[&str]) -> Result<Vec<Atom>> {
        if concepts.is_empty() {
            return Ok(vec![]);
        }
        let placeholders: Vec<String> = (1..=concepts.len()).map(|i| format!("?{i}")).collect();
        let sql = format!(
            "SELECT * FROM nomtu WHERE variant IN ({})",
            placeholders.join(",")
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::types::ToSql> = concepts
            .iter()
            .map(|c| c as &dyn rusqlite::types::ToSql)
            .collect();
        let atoms = stmt
            .query_map(params.as_slice(), |row| Ok(Self::row_to_atom(row)))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(atoms)
    }

    /// Load ALL atoms (streaming from SQLite).
    pub fn load_all(&self) -> Result<Vec<Atom>> {
        let mut stmt = self.conn.prepare_cached("SELECT * FROM nomtu")?;
        let atoms = stmt
            .query_map([], |row| Ok(Self::row_to_atom(row)))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(atoms)
    }

    /// Dictionary summary: variant (concept) -> count.
    pub fn dictionary_summary(&self) -> Result<Vec<(String, usize)>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT variant, COUNT(*) FROM nomtu WHERE variant IS NOT NULL GROUP BY variant ORDER BY COUNT(*) DESC",
        )?;
        let results = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(results)
    }

    /// Stats: count by kind.
    pub fn stats_by_kind(&self) -> Result<Vec<(String, usize)>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT kind, COUNT(*) FROM nomtu GROUP BY kind ORDER BY COUNT(*) DESC",
        )?;
        let results = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(results)
    }

    /// Stats: count by language.
    pub fn stats_by_language(&self) -> Result<Vec<(String, usize)>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT language, COUNT(*) FROM nomtu GROUP BY language ORDER BY COUNT(*) DESC",
        )?;
        let results = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(results)
    }

    /// Import atoms from NDJSON file (bulk migration).
    pub fn import_ndjson(&self, path: &Path) -> Result<usize> {
        let data = std::fs::read_to_string(path).context("failed to read NDJSON file")?;
        let mut count = 0;
        let mut batch = Vec::new();

        for line in data.lines() {
            if line.is_empty() {
                continue;
            }
            if let Ok(atom) = serde_json::from_str::<Atom>(line) {
                batch.push(atom);
                if batch.len() >= 5000 {
                    self.store_atoms(&batch)?;
                    count += batch.len();
                    batch.clear();
                    if count % 50000 == 0 {
                        eprintln!("  imported {count}...");
                    }
                }
            }
        }
        if !batch.is_empty() {
            count += batch.len();
            self.store_atoms(&batch)?;
        }

        Ok(count)
    }

    /// Import atoms from individual JSON files (legacy migration).
    pub fn import_json_dir(&self, atoms_dir: &Path) -> Result<usize> {
        let mut count = 0;
        let mut batch = Vec::new();

        for entry in std::fs::read_dir(atoms_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str == "index.json"
                || name_str == "concept_index.json"
                || name_str.ends_with(".tmp")
            {
                continue;
            }

            if let Ok(data) = std::fs::read_to_string(&path)
                && let Ok(atom) = serde_json::from_str::<Atom>(&data)
            {
                batch.push(atom);
                if batch.len() >= 5000 {
                    self.store_atoms(&batch)?;
                    count += batch.len();
                    batch.clear();
                    if count % 50000 == 0 {
                        eprintln!("  imported {count}...");
                    }
                }
            }
        }
        if !batch.is_empty() {
            count += batch.len();
            self.store_atoms(&batch)?;
        }

        Ok(count)
    }

    /// Database file path.
    pub fn db_path(&self) -> PathBuf {
        self.root.join("data/nomdict.db")
    }

    // ── Row mapping ─────────────────────────────────────────────────
    // Maps nomtu columns back to Atom struct.
    // nomtu columns by index (48 columns):
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

    fn row_to_atom(row: &rusqlite::Row) -> Atom {
        let word: String = row.get(1).unwrap_or_default();
        let kind_str: String = row.get(3).unwrap_or_default();
        let hash: Option<String> = row.get(4).unwrap_or(None);
        let concept: Option<String> = row.get(7).unwrap_or(None);
        let labels_json: String = row.get(8).unwrap_or_else(|_| "[]".to_string());
        let sig_json: Option<String> = row.get(14).unwrap_or(None);
        let source_path: String = row.get(29).unwrap_or_default();
        let language: String = row.get(33).unwrap_or_default();
        let body: Option<String> = row.get(34).unwrap_or(None);
        let labels: Vec<String> = serde_json::from_str(&labels_json).unwrap_or_default();

        Atom {
            id: hash.unwrap_or_else(|| format!("{}:{}:{}", source_path, kind_str, word)),
            kind: parse_atom_kind(&kind_str),
            name: word,
            source_path,
            language,
            labels,
            concept,
            signature: sig_json.and_then(|s| serde_json::from_str(&s).ok()),
            body,
        }
    }
}
