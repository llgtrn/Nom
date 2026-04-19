//! Dict CLI commands — AD-DB-DEMO (Wave AD end-to-end demo #3).
//!
//! Provides `nom dict list-kinds` and `nom dict status` by opening
//! `nom-compiler/nomdict.db` directly via rusqlite.

use std::path::Path;

/// Discover the nomdict.db path:
/// 1. `NOM_DICT_PATH` environment variable
/// 2. Relative to current dir or parent dirs (`nom-compiler/nomdict.db`)
pub fn find_dict_db() -> Result<String, String> {
    if let Ok(env_path) = std::env::var("NOM_DICT_PATH") {
        return Ok(env_path);
    }
    // Try relative to current dir and a few parent dirs
    for prefix in &[".", "..", "../..", "../../.."] {
        let path = Path::new(prefix).join("nom-compiler/nomdict.db");
        if path.exists() {
            return Ok(path.to_string_lossy().into_owned());
        }
    }
    Err("nomdict.db not found. Set NOM_DICT_PATH or run from project root.".to_string())
}

/// Row returned by `list_kinds`.
#[derive(serde::Serialize)]
pub struct KindRow {
    pub name: String,
    pub description: String,
    pub use_count: i64,
}

/// `nom dict list-kinds` — query kinds table and print use counts.
pub fn list_kinds(db_path: &str) -> Result<Vec<KindRow>, String> {
    let conn = rusqlite::Connection::open(db_path)
        .map_err(|e| format!("open db: {e}"))?;

    let mut stmt = conn.prepare(
        "SELECT k.name, k.description, COUNT(n.id) AS use_count
         FROM kinds k
         LEFT JOIN nomtu n ON n.kind = k.name
         GROUP BY k.name
         ORDER BY k.name"
    ).map_err(|e| format!("prepare: {e}"))?;

    let rows = stmt.query_map([], |row| {
        Ok(KindRow {
            name: row.get(0)?,
            description: row.get(1)?,
            use_count: row.get(2)?,
        })
    }).map_err(|e| format!("query: {e}"))?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| format!("row: {e}"))?);
    }
    Ok(out)
}

/// Print kinds as formatted text.
pub fn print_kinds(rows: &[KindRow]) {
    println!("{:<16} {:<8} {}", "KIND", "COUNT", "DESCRIPTION");
    println!("{}", "-".repeat(60));
    for r in rows {
        println!("{:<16} {:<8} {}", r.name, r.use_count, r.description);
    }
}

/// Print kinds as pretty JSON.
pub fn print_kinds_json(rows: &[KindRow]) {
    match serde_json::to_string_pretty(rows) {
        Ok(json) => println!("{json}"),
        Err(e) => eprintln!("json error: {e}"),
    }
}

/// `nom dict status` — show DB file size, table counts, entry counts.
pub fn dict_status(db_path: &str) -> Result<StatusInfo, String> {
    let conn = rusqlite::Connection::open(db_path)
        .map_err(|e| format!("open db: {e}"))?;

    let file_size = std::fs::metadata(db_path)
        .map(|m| m.len())
        .unwrap_or(0);

    let table_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
        [],
        |row| row.get(0),
    ).map_err(|e| format!("table count: {e}"))?;

    let entry_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM nomtu",
        [],
        |row| row.get(0),
    ).map_err(|e| format!("entry count: {e}"))?;

    let kind_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM kinds",
        [],
        |row| row.get(0),
    ).map_err(|e| format!("kind count: {e}"))?;

    Ok(StatusInfo {
        db_path: db_path.to_string(),
        file_size,
        table_count,
        entry_count,
        kind_count,
    })
}

#[derive(serde::Serialize)]
pub struct StatusInfo {
    pub db_path: String,
    pub file_size: u64,
    pub table_count: i64,
    pub entry_count: i64,
    pub kind_count: i64,
}

pub fn print_status(info: &StatusInfo) {
    println!("Dictionary Status");
    println!("  DB path:     {}", info.db_path);
    println!("  File size:   {} bytes", info.file_size);
    println!("  Tables:      {}", info.table_count);
    println!("  Kinds:       {}", info.kind_count);
    println!("  Entries:     {}", info.entry_count);
}

pub fn print_status_json(info: &StatusInfo) {
    match serde_json::to_string_pretty(info) {
        Ok(json) => println!("{json}"),
        Err(e) => eprintln!("json error: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dict_list_kinds_returns_at_least_one_row() {
        let db_path = find_dict_db().expect("db should be discoverable");
        let rows = list_kinds(&db_path).expect("query should succeed");
        assert!(
            !rows.is_empty(),
            "expected at least 1 kind row in nomdict.db"
        );
    }

    #[test]
    fn dict_status_returns_valid_counts() {
        let db_path = find_dict_db().expect("db should be discoverable");
        let info = dict_status(&db_path).expect("status query should succeed");
        assert!(info.file_size > 0, "db file should have size");
        assert!(info.table_count > 0, "db should have tables");
        assert!(info.kind_count > 0, "db should have kinds");
    }

    #[test]
    fn dict_list_kinds_json_roundtrip() {
        let rows = vec![
            KindRow {
                name: "verb".into(),
                description: "action".into(),
                use_count: 3,
            },
        ];
        let json = serde_json::to_string_pretty(&rows).unwrap();
        assert!(json.contains("verb"));
        assert!(json.contains("action"));
    }
}
