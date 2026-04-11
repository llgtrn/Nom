//! Directory scanning for source file discovery.

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Default directories to skip during scanning.
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
];

/// Scan a directory and return all source file paths.
pub fn scan_directory(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    let walker = WalkDir::new(dir).into_iter();
    for entry in walker.filter_entry(|e| !should_skip(e)) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        if is_source_file(path) {
            files.push(path.to_path_buf());
        }
    }

    files
}

fn should_skip(entry: &walkdir::DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }
    entry
        .file_name()
        .to_str()
        .map(|name| IGNORED_DIRS.iter().any(|d| d.eq_ignore_ascii_case(name)))
        .unwrap_or(false)
}

fn is_source_file(path: &Path) -> bool {
    let ext = match path.extension().and_then(|e| e.to_str()) {
        Some(e) => e,
        None => return false,
    };
    matches!(
        ext,
        "rs" | "ts"
            | "tsx"
            | "js"
            | "jsx"
            | "mjs"
            | "cjs"
            | "py"
            | "pyi"
            | "c"
            | "h"
            | "cpp"
            | "cc"
            | "cxx"
            | "hpp"
            | "go"
    )
}
