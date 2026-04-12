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

// ── cmd_corpus_clone_ingest ──────────────────────────────────────────────────

pub fn cmd_corpus_clone_ingest(url: &str, dict: &Path, json: bool) -> i32 {
    let db_path = resolve_db_path(dict);
    let d = match nom_dict::NomDict::open_in_place(&db_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("nom: open dict {}: {e}", db_path.display());
            return 1;
        }
    };
    let report = match nom_corpus::clone_and_ingest(url, &d) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("nom: clone-and-ingest error: {e}");
            return 1;
        }
    };
    if json {
        println!("{}", serde_json::to_string(&report).unwrap_or_default());
    } else {
        println!("clone-and-ingest: {}", report.url);
        println!("  clone duration:  {:.1}s", report.clone_duration_secs);
        println!("  files ingested:  {}", report.ingest.files_ingested);
        println!("  files skipped:   {}", report.ingest.files_skipped);
        println!("  duplicates:      {}", report.ingest.duplicates);
    }
    0
}

pub fn cmd_corpus_clone_batch(list_path: &Path, dict: &Path, json: bool) -> i32 {
    let contents = match std::fs::read_to_string(list_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("nom: read {}: {e}", list_path.display());
            return 1;
        }
    };
    let urls: Vec<String> = contents
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.to_string())
        .collect();
    if urls.is_empty() {
        eprintln!("nom: no URLs in {} (lines starting with # are ignored)", list_path.display());
        return 1;
    }
    let db_path = resolve_db_path(dict);
    let d = match nom_dict::NomDict::open_in_place(&db_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("nom: open dict {}: {e}", db_path.display());
            return 1;
        }
    };
    let report = nom_corpus::clone_batch(&urls, &d);
    if json {
        println!("{}", serde_json::to_string(&report).unwrap_or_default());
    } else {
        println!("clone-batch summary:");
        println!("  total:            {}", report.total);
        println!("  succeeded:        {}", report.succeeded);
        println!("  failed:           {}", report.failed);
        println!("  files ingested:   {}", report.files_ingested);
        if !report.failures.is_empty() {
            println!("  failures:");
            for (url, err) in &report.failures {
                println!("    {url}: {err}");
            }
        }
    }
    if report.failed > 0 && report.succeeded == 0 { 1 } else { 0 }
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
