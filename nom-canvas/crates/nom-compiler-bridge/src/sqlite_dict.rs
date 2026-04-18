#![deny(unsafe_code)]
// SqliteDictReader: Wave C concrete DictReader backed by nom-dict + nom-grammar
// Only compiled when `compiler` feature is enabled

use nom_blocks::block_model::NomtuRef;
use nom_blocks::dict_reader::{ClauseShape, DictReader, GrammarKindRow};

/// Wave C concrete DictReader using nom-grammar's SQLite connection.
/// Owned by BridgeState; the bridge holds the ONLY writer connection.
/// nom-blocks never opens SQLite directly.
#[cfg(feature = "compiler")]
pub struct SqliteDictReader {
    state: std::sync::Arc<crate::shared::SharedState>,
}

#[cfg(feature = "compiler")]
impl SqliteDictReader {
    pub fn new(state: std::sync::Arc<crate::shared::SharedState>) -> Self {
        Self { state }
    }

    fn open_grammar_conn(&self) -> Option<rusqlite::Connection> {
        rusqlite::Connection::open(&self.state.grammar_path).ok()
    }

    fn open_dict_conn(&self) -> Option<rusqlite::Connection> {
        rusqlite::Connection::open_with_flags(
            &self.state.dict_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .ok()
    }
}

#[cfg(feature = "compiler")]
impl DictReader for SqliteDictReader {
    fn is_known_kind(&self, kind: &str) -> bool {
        // Fast path: check cached grammar kinds first
        let cached = self.state.cached_grammar_kinds();
        if !cached.is_empty() {
            return cached.iter().any(|k| k.name == kind);
        }
        // Fallback: query grammar DB
        if let Some(conn) = self.open_grammar_conn() {
            let result: rusqlite::Result<i64> = conn.query_row(
                "SELECT COUNT(*) FROM kinds WHERE name = ?1",
                rusqlite::params![kind],
                |row| row.get(0),
            );
            return result.ok().map(|n| n > 0).unwrap_or(false);
        }
        false
    }

    fn list_kinds(&self) -> Vec<GrammarKindRow> {
        let cached = self.state.cached_grammar_kinds();
        if !cached.is_empty() {
            return cached
                .into_iter()
                .map(|kind| GrammarKindRow {
                    name: kind.name,
                    description: kind.description,
                })
                .collect();
        }

        let Some(conn) = self.open_grammar_conn() else {
            return vec![];
        };
        let mut stmt = match conn.prepare("SELECT name, description FROM kinds ORDER BY name") {
            Ok(stmt) => stmt,
            Err(_) => return vec![],
        };
        stmt.query_map([], |row| {
            Ok(GrammarKindRow {
                name: row.get(0)?,
                description: row.get(1)?,
            })
        })
        .ok()
        .map(|rows| rows.filter_map(|row| row.ok()).collect())
        .unwrap_or_default()
    }

    fn clause_shapes_for(&self, kind: &str) -> Vec<ClauseShape> {
        let Some(conn) = self.open_grammar_conn() else {
            return vec![];
        };
        let mut stmt = match conn.prepare(
            "SELECT name, grammar_shape, is_required, description FROM clause_shapes WHERE kind = ?1 ORDER BY name"
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let shapes: Vec<ClauseShape> = stmt
            .query_map(rusqlite::params![kind], |row| {
                Ok(ClauseShape {
                    name: row.get(0)?,
                    grammar_shape: row.get(1)?,
                    is_required: row.get::<_, i64>(2)? != 0,
                    description: row.get(3)?,
                })
            })
            .ok()
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default();
        shapes
    }

    fn lookup_entity(&self, word: &str, kind: &str) -> Option<NomtuRef> {
        let conn = self.open_dict_conn()?;
        let result: rusqlite::Result<(String, String, String)> = conn.query_row(
            "SELECT id, word, kind FROM entries WHERE word = ?1 AND kind = ?2 LIMIT 1",
            rusqlite::params![word, kind],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        );
        result
            .ok()
            .map(|(id, word, kind)| NomtuRef::new(id, word, kind))
    }
}

// Stub for when compiler feature is NOT enabled — returns None/empty for all queries
#[cfg(not(feature = "compiler"))]
pub struct SqliteDictReader;

#[cfg(not(feature = "compiler"))]
impl SqliteDictReader {
    pub fn new_stub() -> Self {
        Self
    }
}

#[cfg(not(feature = "compiler"))]
impl DictReader for SqliteDictReader {
    fn is_known_kind(&self, _kind: &str) -> bool {
        false
    }
    fn list_kinds(&self) -> Vec<GrammarKindRow> {
        vec![]
    }
    fn clause_shapes_for(&self, _kind: &str) -> Vec<ClauseShape> {
        vec![]
    }
    fn lookup_entity(&self, _word: &str, _kind: &str) -> Option<NomtuRef> {
        None
    }
}

#[cfg(test)]
#[cfg(not(feature = "compiler"))]
mod tests {
    use super::*;

    #[test]
    fn sqlite_dict_stub_is_known_kind_false() {
        let reader = SqliteDictReader::new_stub();
        assert!(!reader.is_known_kind("verb"));
        assert!(!reader.is_known_kind("concept"));
        assert!(!reader.is_known_kind(""));
    }

    #[test]
    fn sqlite_dict_stub_clause_shapes_empty() {
        let reader = SqliteDictReader::new_stub();
        assert!(reader.clause_shapes_for("verb").is_empty());
        assert!(reader.clause_shapes_for("anything").is_empty());
        assert!(reader.list_kinds().is_empty());
    }

    #[test]
    fn sqlite_dict_stub_lookup_none() {
        let reader = SqliteDictReader::new_stub();
        assert!(reader.lookup_entity("run", "verb").is_none());
        assert!(reader.lookup_entity("", "").is_none());
    }

    #[test]
    fn sqlite_dict_stub_reader_trait_implemented() {
        // Calling via trait reference proves the trait impl is complete
        let reader = SqliteDictReader::new_stub();
        let boxed: &dyn DictReader = &reader;
        assert!(!boxed.is_known_kind("anything"));
        assert!(boxed.clause_shapes_for("anything").is_empty());
        assert!(boxed.lookup_entity("word", "kind").is_none());
    }

    #[test]
    fn sqlite_dict_new_stub_creates_reader() {
        // new_stub() does not panic and returns a usable reader
        let reader = SqliteDictReader::new_stub();
        // basic smoke: trait methods callable
        let _ = reader.is_known_kind("test");
    }
}
