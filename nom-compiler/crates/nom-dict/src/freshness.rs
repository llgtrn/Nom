//! Dict freshness tracking (Phase 1 of the graph-durability design spec,
//! 2026-04-14). Mirrors GitNexus's `status.ts` staleness mechanism: store
//! the source-tree hash in the dict meta table, compare to the current
//! working-tree hash to answer `is_stale()`.
//!
//! Source hash = SHA-256 over a **deterministic sorted map** of
//! `(rel_path, file_sha256)` entries for every file under `repo_root` that
//! matches the same ignore rules `nom-extract::scan` applies. Paths are
//! serialized as Unix-style forward-slash strings before hashing so the
//! digest is cross-platform stable.
//!
//! Intentionally narrow scope (per spec "Out of scope"):
//! - We track **source** freshness, not build-artifact freshness.
//! - No concurrent mutation (dict is opened per-process).
//! - No reverse lookup of "which files changed?" — the `is_stale` bool is
//!   enough for UX; callers run `git diff` if they need specifics.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::NomDict;

/// Key used in `dict_meta` for the source-hash column.
pub const SOURCE_HASH_KEY: &str = "dict_last_source_hash";

/// File extensions treated as source files for hashing. Mirrors (but is
/// intentionally narrower than) `nom-extract::scan::is_source_file` — we
/// hash anything that could end up in DB2, not build outputs. The set
/// stays in this crate (not `nom-extract`) to avoid a cycle.
const HASHED_EXTENSIONS: &[&str] = &[
    "nom", "nomx", "nomtu", "rs", "py", "ts", "tsx", "js", "jsx", "go", "c", "cpp", "cc", "h",
    "hpp", "java", "kt", "rb", "php", "swift", "scala", "lua", "sh", "md", "toml", "yaml", "yml",
    "json",
];

/// Directory names skipped when walking `repo_root`. Must stay in sync
/// with `nom-extract::scan::IGNORED_DIRS`; a mismatch would hash files
/// that never get ingested or skip files that do.
const IGNORED_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    ".next",
    "__pycache__",
    ".mypy_cache",
    ".pytest_cache",
    "dist",
    "build",
    ".omc",
    ".claude",
    ".gitnexus",
];

fn is_source_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| HASHED_EXTENSIONS.contains(&e))
        .unwrap_or(false)
}

fn is_ignored_dir(name: &str) -> bool {
    IGNORED_DIRS.iter().any(|d| d.eq_ignore_ascii_case(name))
}

fn collect_source_files(repo_root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack: Vec<PathBuf> = vec![repo_root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(it) => it,
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e),
        };
        for entry in entries {
            let entry = entry?;
            let ft = entry.file_type()?;
            let path = entry.path();
            if ft.is_dir() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if !is_ignored_dir(&name) {
                    stack.push(path);
                }
            } else if ft.is_file() && is_source_file(&path) {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

fn sha256_file(path: &Path) -> io::Result<String> {
    let bytes = fs::read(path)?;
    let digest = Sha256::digest(&bytes);
    Ok(format!("{digest:x}"))
}

/// Compute the deterministic SHA-256 of the source tree rooted at
/// `repo_root`. The hash input is the newline-joined `"<rel_path>\t<file_sha>"`
/// of every source file, paths sorted ascending, with forward slashes.
pub fn compute_source_hash(repo_root: &Path) -> io::Result<String> {
    let files = collect_source_files(repo_root)?;
    let mut map: BTreeMap<String, String> = BTreeMap::new();
    for path in &files {
        let rel = path
            .strip_prefix(repo_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let h = sha256_file(path)?;
        map.insert(rel, h);
    }
    let mut hasher = Sha256::new();
    for (rel, h) in &map {
        hasher.update(rel.as_bytes());
        hasher.update(b"\t");
        hasher.update(h.as_bytes());
        hasher.update(b"\n");
    }
    Ok(format!("{:x}", hasher.finalize()))
}

impl NomDict {
    /// SHA-256 over the current working-tree source files under `repo_root`.
    /// Deterministic, cross-platform (forward-slash paths).
    pub fn current_source_hash(&self, repo_root: &Path) -> io::Result<String> {
        compute_source_hash(repo_root)
    }

    /// Read the last-marked source hash from `dict_meta`. Returns `None` if
    /// the dict has never been marked fresh.
    pub fn stored_source_hash(&self) -> rusqlite::Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT value FROM dict_meta WHERE key = ?1")?;
        stmt.query_row(rusqlite::params![SOURCE_HASH_KEY], |row| {
            row.get::<_, String>(0)
        })
        .map(Some)
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })
    }

    /// Upsert `hash` as the dict's marked source-tree hash. Sets
    /// `updated_at` to the current Unix epoch seconds.
    pub fn mark_source_hash(&self, hash: &str) -> rusqlite::Result<()> {
        let updated_at = format!(
            "epoch-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        );
        self.conn.execute(
            "INSERT INTO dict_meta (key, value, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET
                 value = excluded.value,
                 updated_at = excluded.updated_at",
            rusqlite::params![SOURCE_HASH_KEY, hash, updated_at],
        )?;
        Ok(())
    }

    /// `true` iff the current working-tree hash differs from the stored
    /// hash (or no hash has been stored yet).
    pub fn is_stale(&self, repo_root: &Path) -> io::Result<bool> {
        let current = self.current_source_hash(repo_root)?;
        let stored = self
            .stored_source_hash()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("dict_meta read: {e}")))?;
        Ok(match stored {
            None => true, // never marked → report stale so caller marks
            Some(s) => s != current,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_repo(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("nom_freshness_{}_{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn fresh_dict_reports_stale() {
        let root = tmp_repo("fresh");
        fs::write(root.join("a.rs"), "fn a() {}").unwrap();
        let d = NomDict::open_in_memory().unwrap();
        // Never marked → is_stale must be true.
        assert!(d.is_stale(&root).unwrap());
        assert!(d.stored_source_hash().unwrap().is_none());
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn unchanged_source_reports_fresh() {
        let root = tmp_repo("unchanged");
        fs::write(root.join("a.rs"), "fn a() {}").unwrap();
        let d = NomDict::open_in_memory().unwrap();
        let h = d.current_source_hash(&root).unwrap();
        d.mark_source_hash(&h).unwrap();
        assert!(!d.is_stale(&root).unwrap(), "after marking, must be fresh");
        assert_eq!(d.stored_source_hash().unwrap().as_deref(), Some(h.as_str()));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn edited_source_reports_stale() {
        let root = tmp_repo("edited");
        fs::write(root.join("a.rs"), "fn a() {}").unwrap();
        let d = NomDict::open_in_memory().unwrap();
        let h = d.current_source_hash(&root).unwrap();
        d.mark_source_hash(&h).unwrap();
        // Edit the file → hash diverges.
        fs::write(root.join("a.rs"), "fn a() { /* edit */ }").unwrap();
        assert!(
            d.is_stale(&root).unwrap(),
            "edited source must report stale"
        );
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn mark_is_idempotent_and_upserts() {
        let root = tmp_repo("idempotent");
        fs::write(root.join("x.nom"), "use foo").unwrap();
        let d = NomDict::open_in_memory().unwrap();
        let h = d.current_source_hash(&root).unwrap();
        d.mark_source_hash(&h).unwrap();
        d.mark_source_hash(&h).unwrap();
        d.mark_source_hash("different-hash").unwrap();
        assert_eq!(
            d.stored_source_hash().unwrap().as_deref(),
            Some("different-hash"),
            "second mark must overwrite"
        );
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn compute_hash_is_deterministic() {
        let root = tmp_repo("determinism");
        fs::write(root.join("alpha.nom"), "x").unwrap();
        fs::write(root.join("beta.nom"), "y").unwrap();
        let a = compute_source_hash(&root).unwrap();
        let b = compute_source_hash(&root).unwrap();
        assert_eq!(a, b, "two reads must produce the same hash");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn ignored_dirs_are_skipped() {
        let root = tmp_repo("ignored");
        fs::write(root.join("a.rs"), "fn a() {}").unwrap();
        fs::create_dir_all(root.join("target")).unwrap();
        fs::write(root.join("target/garbage.rs"), "junk").unwrap();
        let d = NomDict::open_in_memory().unwrap();
        let h1 = d.current_source_hash(&root).unwrap();
        // Touch a file under target/ — hash must NOT change.
        fs::write(root.join("target/garbage.rs"), "more junk").unwrap();
        let h2 = d.current_source_hash(&root).unwrap();
        assert_eq!(h1, h2, "changes under target/ must be ignored");
        fs::remove_dir_all(&root).ok();
    }
}
