//! `nom corpus` CLI dispatch per plan §5.17.
//!
//! Today: `scan` walks a local directory and reports per-language
//! file/byte counts. No dict writes; purely read-only file-system
//! introspection — the "what's here" phase before Phase-5 ingestion.

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
