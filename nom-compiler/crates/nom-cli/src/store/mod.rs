//! `nom store` subcommands — v2 content-addressed dictionary CLI.
//!
//! Wires nom-parser → nom-types::canonical → nom-dict → nom-resolver::v2
//! so a user can ingest a `.nom` file, retrieve an entry by hash prefix,
//! walk the closure from a root, verify reachability, and GC to roots.
//!
//! Tasks A/B landed the storage layer; this module is the single CLI
//! surface that consumes them. `body_nom` is stored as the human-readable
//! pre-rewrite source per the Task B hazard report; `entry_refs` is
//! populated from the resolver output (missing refs → diagnostics +
//! Partial status, not a blocking error).
//!
//! Paths are UTF-8 safe and handle both forward and back slashes so the
//! CLI works unchanged on Windows and POSIX.

pub mod add_media;
pub mod commands;
pub mod materialize;
pub mod resolve;
pub mod sync;

pub use add_media::cmd_store_add_media;
pub use commands::*;
pub use materialize::materialize_concept_graph_from_db;
pub use resolve::{ResolvedRef, resolve_closure};
pub use sync::cmd_store_sync;

use std::path::{Path, PathBuf};

use nom_dict::NomDict;

// ── Shared helpers (used by commands.rs and sibling submodules) ──────

pub(super) fn open_dict(dict: &Path) -> Option<NomDict> {
    let root = if dict.extension().is_some_and(|e| e == "db") {
        // dict points to a .db file; NomDict::open expects the directory
        // that contains `data/nomdict.db`. Pick an ancestor that already
        // has a `data/` child, falling back to cwd for compatibility
        // with the legacy `--dict nomdict.db` convention.
        let parent = dict.parent().unwrap_or_else(|| Path::new("."));
        if parent.file_name().and_then(|n| n.to_str()) == Some("data") {
            parent.parent().unwrap_or(Path::new(".")).to_path_buf()
        } else {
            parent.to_path_buf()
        }
    } else {
        dict.to_path_buf()
    };
    match NomDict::open(&root) {
        Ok(d) => Some(d),
        Err(e) => {
            eprintln!("nom: cannot open nomdict at {}: {e}", root.display());
            None
        }
    }
}

/// Resolve a hash prefix against the dict. Returns the full 64-char id
/// on a unique match; an error message otherwise.
pub(super) fn resolve_prefix(dict: &NomDict, hash: &str) -> Result<String, String> {
    if hash.len() < 8 {
        return Err(format!(
            "nom: hash prefix too short (need ≥ 8 hex chars): {hash}"
        ));
    }
    if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("nom: not a hex string: {hash}"));
    }
    // Full id? get_entry fast path.
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

pub(super) fn chrono_like_now() -> String {
    // Keep dependencies minimal: use a coarse UTC timestamp so newly
    // upserted rows don't leave `created_at` empty. The dict's own
    // `datetime('now')` DEFAULT handles downstream re-upserts.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("epoch-{secs}")
}

pub(super) fn load_roots() -> std::io::Result<Vec<String>> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    let path = match home {
        Some(h) => h.join(".nom").join("roots.txt"),
        None => return Ok(Vec::new()),
    };
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&path)?;
    Ok(content
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect())
}

pub(super) fn json_array(items: &[String]) -> String {
    let escaped: Vec<String> = items
        .iter()
        .map(|s| format!("\"{}\"", escape_json(s)))
        .collect();
    format!("[{}]", escaped.join(","))
}

pub(super) fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

pub(super) fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_owned()
    } else {
        format!("{}…", &s[..n.saturating_sub(1)])
    }
}
