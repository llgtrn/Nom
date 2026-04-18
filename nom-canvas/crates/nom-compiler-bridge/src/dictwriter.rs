#![deny(unsafe_code)]

/// Write-side access to a Nom dictionary database.
///
/// All operations are synchronous. Use `DictWriter::new` with a real path for
/// production use; for tests use `rusqlite::Connection::open_in_memory()` and
/// `DictWriter::from_conn`.
pub struct DictWriter {
    conn: rusqlite::Connection,
}

impl DictWriter {
    /// Open a DictWriter connected to a SQLite file at `path`.
    /// Creates the `entries` table if it does not exist.
    pub fn new(path: &str) -> rusqlite::Result<Self> {
        let conn = rusqlite::Connection::open(path)?;
        Self::ensure_schema(&conn)?;
        Ok(Self { conn })
    }

    /// Construct from an existing connection (useful for in-memory test DBs).
    pub fn from_conn(conn: rusqlite::Connection) -> rusqlite::Result<Self> {
        Self::ensure_schema(&conn)?;
        Ok(Self { conn })
    }

    fn ensure_schema(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS entries (
                id     INTEGER PRIMARY KEY AUTOINCREMENT,
                word   TEXT    NOT NULL,
                kind   TEXT    NOT NULL,
                body   TEXT    NOT NULL,
                status TEXT    NOT NULL DEFAULT 'partial'
            );",
        )
    }

    /// Insert a new entry with status `partial`.
    ///
    /// Uses `INSERT OR IGNORE` so duplicate (word, kind) pairs are silently
    /// skipped. Returns the `rowid` of the inserted (or existing) row.
    pub fn insert_partial_entry(
        &self,
        word: &str,
        kind: &str,
        body: &str,
    ) -> rusqlite::Result<i64> {
        self.conn.execute(
            "INSERT OR IGNORE INTO entries (word, kind, body, status)
             VALUES (?1, ?2, ?3, 'partial')",
            rusqlite::params![word, kind, body],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Promote an entry from `partial` to `complete`.
    pub fn promote_to_complete(&self, entry_id: i64) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE entries SET status = 'complete' WHERE id = ?1",
            rusqlite::params![entry_id],
        )?;
        Ok(())
    }

    /// Return the current status of an entry, or `None` if the id is unknown.
    pub fn entry_status(&self, entry_id: i64) -> rusqlite::Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT status FROM entries WHERE id = ?1")?;
        let mut rows = stmt.query(rusqlite::params![entry_id])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn in_memory_writer() -> DictWriter {
        let conn = rusqlite::Connection::open_in_memory().expect("in-memory DB");
        DictWriter::from_conn(conn).expect("schema creation")
    }

    /// insert_partial_entry returns a positive rowid on success.
    #[test]
    fn test_insert_partial_entry() {
        let w = in_memory_writer();
        let id = w
            .insert_partial_entry("run", "verb", "execute an action")
            .expect("insert must succeed");
        assert!(id > 0, "rowid must be positive, got {id}");
    }

    /// Inserting a partial entry and then promoting it sets status to "complete".
    #[test]
    fn test_promote_to_complete() {
        let w = in_memory_writer();
        let id = w
            .insert_partial_entry("emit", "verb", "output a value")
            .expect("insert");
        w.promote_to_complete(id).expect("promote");
        let status = w.entry_status(id).expect("status query");
        assert_eq!(
            status.as_deref(),
            Some("complete"),
            "status must be 'complete' after promotion"
        );
    }

    /// entry_status for an unknown id returns Ok(None).
    #[test]
    fn test_entry_status_missing() {
        let w = in_memory_writer();
        let status = w.entry_status(99999).expect("query must not error");
        assert_eq!(
            status, None,
            "unknown entry_id must return None, not an error"
        );
    }
}
