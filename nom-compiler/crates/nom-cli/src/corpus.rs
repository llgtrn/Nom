//! `nom corpus` CLI dispatch per plan §5.17.
//!
//! `scan`  — walks a local directory and reports per-language file/byte
//!            counts (read-only, no dict writes).
//! `ingest` — walks a local directory, hashes each file, and upserts one
//!            v2 Entry per file into the nomdict (§5.17 source ingestion).

use std::path::Path;

// ── cmd_corpus_scan ──────────────────────────────────────────────────────────

pub fn cmd_corpus_scan(path: &Path, json: bool) -> i32 {
    let report = match nom_corpus::scan_directory(path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("nom: scan error: {e}");
            return 1;
        }
    };
    if json {
        println!(
            "{}",
            serde_json::to_string(&report).unwrap_or_default()
        );
    } else {
        println!("corpus scan: {}", report.root);
        println!("  total files:  {}", report.total_files);
        println!("  total bytes:  {}", report.total_bytes);
        println!();
        println!("  {:<15}  {:>6}  {:>12}", "language", "files", "bytes");
        println!("  {:<15}  {:>6}  {:>12}", "----------", "-----", "-----");
        for (lang, stats) in &report.languages {
            println!(
                "  {:<15}  {:>6}  {:>12}",
                lang, stats.file_count, stats.total_bytes
            );
        }
    }
    0
}

// ── cmd_corpus_ingest ────────────────────────────────────────────────────────

pub fn cmd_corpus_ingest(path: &Path, dict: &Path, json: bool) -> i32 {
    // Open (or create) the dict at the given path.
    // Follow the same .db-file convention as store::open_dict.
    let db_path = resolve_db_path(dict);
    let dict_db = match nom_dict::NomDict::open_in_place(&db_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("nom: cannot open dict {}: {e}", db_path.display());
            return 1;
        }
    };

    let report = match nom_corpus::ingest_directory(path, &dict_db) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("nom: ingest error: {e}");
            return 1;
        }
    };

    if json {
        println!(
            "{}",
            serde_json::to_string(&report).unwrap_or_default()
        );
    } else {
        println!("corpus ingest: {}", report.root);
        println!("  files ingested:  {}", report.files_ingested);
        println!("  files skipped:   {}", report.files_skipped);
        println!("  duplicates:      {}", report.duplicates);
        println!("  bytes ingested:  {}", report.bytes_ingested);
        if !report.per_language.is_empty() {
            println!();
            println!("  {:<15}  {:>6}", "language", "files");
            println!("  {:<15}  {:>6}", "--------", "-----");
            for (lang, count) in &report.per_language {
                println!("  {:<15}  {:>6}", lang, count);
            }
        }
    }
    0
}

// ── cmd_corpus_ingest_parent ─────────────────────────────────────────────────

pub fn cmd_corpus_ingest_parent(path: &Path, dict: &Path, reset_checkpoint: bool, json: bool) -> i32 {
    let db_path = resolve_db_path(dict);
    let report = match nom_corpus::ingest_parent(path, &db_path, reset_checkpoint) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("nom: ingest-parent error: {e}");
            return 1;
        }
    };

    if json {
        println!(
            "{}",
            serde_json::to_string(&report).unwrap_or_default()
        );
    } else {
        println!("corpus ingest-parent: {}", report.parent);
        println!("  repos ingested:  {}", report.repos.len());
        println!("  repos skipped:   {}", report.skipped_repos);
        println!("  repos resumed:   {}", report.resumed_repos);
        println!("  total files:     {}", report.aggregate.files_ingested);
        println!("  total bytes:     {}", report.aggregate.bytes_ingested);
        println!("  duplicates:      {}", report.aggregate.duplicates);
        if !report.aggregate.per_language.is_empty() {
            println!("  languages:");
            println!("    {:<15}  {:>6}", "language", "files");
            println!("    {:<15}  {:>6}", "--------", "-----");
            for (lang, count) in &report.aggregate.per_language {
                println!("    {:<15}  {:>6}", lang, count);
            }
        }
    }
    0
}

// ── cmd_corpus_lift_partial ──────────────────────────────────────────────────

pub fn cmd_corpus_lift_partial(hash: &str, dict: &Path, json: bool) -> i32 {
    let db_path = resolve_db_path(dict);
    let dict_db = match nom_dict::NomDict::open_in_place(&db_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("nom: cannot open dict {}: {e}", db_path.display());
            return 1;
        }
    };

    // Resolve hash prefix → full id (same logic as store::resolve_prefix).
    let id = match resolve_id_prefix(&dict_db, hash) {
        Ok(id) => id,
        Err(msg) => {
            eprintln!("{msg}");
            return 1;
        }
    };

    match nom_corpus::lift_partial(&dict_db, &id) {
        Ok(report) => {
            if json {
                println!("{}", serde_json::to_string(&report).unwrap_or_default());
            } else {
                match &report {
                    nom_corpus::LiftReport::Lifted { partial_id, complete_id, is_new } => {
                        if *is_new {
                            println!("lifted: {partial_id} → {complete_id}");
                        } else {
                            println!("re-linked existing complete entry: {partial_id} → {complete_id}");
                        }
                    }
                    nom_corpus::LiftReport::Rejected { reason } => {
                        println!("rejected: {reason}");
                    }
                    nom_corpus::LiftReport::NotYetImplemented { language } => {
                        println!("not yet implemented for language: {language}");
                    }
                }
            }
            0
        }
        Err(e) => {
            eprintln!("nom: lift-partial error: {e}");
            1
        }
    }
}

// ── cmd_corpus_lift_all ──────────────────────────────────────────────────────

pub fn cmd_corpus_lift_all(dict_path: &Path, max: usize, json: bool) -> i32 {
    let db_path = resolve_db_path(dict_path);
    let dict = match nom_dict::NomDict::open_in_place(&db_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("nom: open dict: {e}");
            return 1;
        }
    };
    match nom_corpus::lift_all(&dict, max) {
        Ok(report) => {
            if json {
                println!("{}", serde_json::to_string(&report).unwrap_or_default());
            } else {
                println!("corpus lift-all:");
                println!("  partials scanned:       {}", report.partials_scanned);
                println!("  lifted (new complete):  {}", report.lifted);
                println!("  relinked (existing):    {}", report.relinked);
                println!("  rejected:               {}", report.rejected);
                println!("  not yet implemented:    {}", report.not_yet_implemented);
                println!("  harness errors:         {}", report.errors);
                if !report.rejection_reasons.is_empty() {
                    println!("  top rejection reasons:");
                    let mut v: Vec<_> = report.rejection_reasons.iter().collect();
                    v.sort_by(|a, b| b.1.cmp(a.1));
                    for (reason, count) in v.iter().take(10) {
                        println!("    {:>6}  {}", count, reason);
                    }
                }
            }
            0
        }
        Err(e) => {
            eprintln!("nom: lift-all error: {e}");
            1
        }
    }
}

/// Resolve a hash prefix (≥ 8 hex chars) to a full 64-char entry id.
fn resolve_id_prefix(dict: &nom_dict::NomDict, hash: &str) -> Result<String, String> {
    if hash.len() < 8 {
        return Err(format!(
            "nom: hash prefix too short (need ≥ 8 hex chars): {hash}"
        ));
    }
    if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("nom: not a hex string: {hash}"));
    }
    if hash.len() == 64 {
        return match dict.get_entry(hash) {
            Ok(Some(e)) => Ok(e.id),
            Ok(None) => Err(format!("nom: no entry with id {hash}")),
            Err(e) => Err(format!("nom: dict error: {e}")),
        };
    }
    let pattern = format!("{hash}%");
    let mut stmt = dict
        .connection()
        .prepare_cached("SELECT id FROM entries WHERE id LIKE ?1 ORDER BY id")
        .map_err(|e| format!("nom: dict error: {e}"))?;
    let ids: Vec<String> = stmt
        .query_map([pattern], |row| row.get::<_, String>(0))
        .map_err(|e| format!("nom: dict error: {e}"))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| format!("nom: dict error: {e}"))?;
    match ids.len() {
        0 => Err(format!("nom: no entry matching prefix {hash}")),
        1 => Ok(ids.into_iter().next().unwrap()),
        _ => {
            let mut msg = format!(
                "nom: hash prefix {hash} is ambiguous ({} candidates):",
                ids.len()
            );
            for id in &ids {
                msg.push_str(&format!("\n  {id}"));
            }
            Err(msg)
        }
    }
}

/// Resolve a `--dict` argument (which may point directly at a `.db` file
/// or at a directory) to an absolute SQLite file path.
fn resolve_db_path(dict: &Path) -> std::path::PathBuf {
    // If it already has a `.db` extension (the common CLI convention
    // `--dict nomdict.db`), treat it as the literal file path.
    if dict.extension().map_or(false, |e| e == "db") {
        dict.to_path_buf()
    } else {
        // Treat as a root dir and append `data/nomdict.db` (NomDict::open
        // convention).
        dict.join("data/nomdict.db")
    }
}
