//! `nom-corpus` — mass ingestion of public ecosystems per §5.17.
//!
//! Ingests PyPI (top-500 packages) + GitHub (top-500 per ecosystem:
//! JS/TS, Python, Rust, Go, Java/Kotlin, C/C++, Swift, Ruby, PHP)
//! under a **stream-and-discard disk discipline**: shallow-clone,
//! ingest, delete source tree, move on. Peak disk = max(per-repo
//! source) + current-dict.
//!
//! Per §4.4.6 each ingested body lands as its compiled artifact (`.bc`
//! for code), tagged with `body_kind`. `nom-corpus` drives the
//! ingest-compile-discard loop; actual codegen is delegated to the
//! upstream language's compiler (rustc for Rust crates, clang for C/C++
//! headers + implementations, tsc+wasm for TypeScript, …).

use thiserror::Error;

/// Source ecosystem for `nom corpus ingest`. Each variant maps to a
/// concrete driver in `src/drivers/` (pypi.rs, github.rs, …) once
/// those land. The enum is closed to prevent silent drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Ecosystem {
    PyPI,
    GitHubJavaScript,
    GitHubTypeScript,
    GitHubPython,
    GitHubRust,
    GitHubGo,
    GitHubJavaKotlin,
    GitHubCCpp,
    GitHubSwift,
    GitHubRuby,
    GitHubPHP,
}

impl Ecosystem {
    /// Every ecosystem in stable order. Mirrors `nom_types::body_kind::ALL`,
    /// `nom_media::Modality::ALL`, and `nom_ux::Platform::ALL` for consistent
    /// enumeration across scaffolded Phase-5 crates.
    pub const ALL: &'static [Ecosystem] = &[
        Ecosystem::PyPI,
        Ecosystem::GitHubJavaScript,
        Ecosystem::GitHubTypeScript,
        Ecosystem::GitHubPython,
        Ecosystem::GitHubRust,
        Ecosystem::GitHubGo,
        Ecosystem::GitHubJavaKotlin,
        Ecosystem::GitHubCCpp,
        Ecosystem::GitHubSwift,
        Ecosystem::GitHubRuby,
        Ecosystem::GitHubPHP,
    ];

    /// Canonical CLI flag value. Parsed by `nom corpus ingest --ecosystem <v>`.
    pub const fn flag_value(self) -> &'static str {
        match self {
            Ecosystem::PyPI => "pypi",
            Ecosystem::GitHubJavaScript => "github-js",
            Ecosystem::GitHubTypeScript => "github-ts",
            Ecosystem::GitHubPython => "github-python",
            Ecosystem::GitHubRust => "github-rust",
            Ecosystem::GitHubGo => "github-go",
            Ecosystem::GitHubJavaKotlin => "github-jvm",
            Ecosystem::GitHubCCpp => "github-ccpp",
            Ecosystem::GitHubSwift => "github-swift",
            Ecosystem::GitHubRuby => "github-ruby",
            Ecosystem::GitHubPHP => "github-php",
        }
    }
}

/// Parse the `--ecosystem <v>` flag. Case-insensitive.
pub fn ecosystem_from_str(s: &str) -> Option<Ecosystem> {
    match s.to_ascii_lowercase().as_str() {
        "pypi" => Some(Ecosystem::PyPI),
        "github-js" | "js" | "javascript" => Some(Ecosystem::GitHubJavaScript),
        "github-ts" | "ts" | "typescript" => Some(Ecosystem::GitHubTypeScript),
        "github-python" | "python" | "py" => Some(Ecosystem::GitHubPython),
        "github-rust" | "rust" | "rs" => Some(Ecosystem::GitHubRust),
        "github-go" | "go" | "golang" => Some(Ecosystem::GitHubGo),
        "github-jvm" | "java" | "kotlin" | "jvm" => Some(Ecosystem::GitHubJavaKotlin),
        "github-ccpp" | "c" | "cpp" | "c++" | "ccpp" => Some(Ecosystem::GitHubCCpp),
        "github-swift" | "swift" => Some(Ecosystem::GitHubSwift),
        "github-ruby" | "ruby" | "rb" => Some(Ecosystem::GitHubRuby),
        "github-php" | "php" => Some(Ecosystem::GitHubPHP),
        _ => None,
    }
}

/// Per-run configuration per §5.17.2 disk-management protocol.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CorpusConfig {
    /// Max packages/repos to ingest. Default per plan: 500.
    pub top_n: usize,
    /// Shallow-clone size cutoff; repos above this are skipped.
    /// Default: 2 GB.
    pub max_clone_bytes: u64,
    /// Bandwidth throttle (bytes/sec per source). Default: 20 MB/s.
    pub bandwidth_limit_bytes_per_sec: u64,
    /// If set, only ingest packages whose name matches (regex-like).
    pub include_pattern: Option<String>,
}

impl Default for CorpusConfig {
    fn default() -> Self {
        Self {
            top_n: 500,
            max_clone_bytes: 2 * 1024 * 1024 * 1024, // 2 GB
            bandwidth_limit_bytes_per_sec: 20 * 1024 * 1024, // 20 MB/s
            include_pattern: None,
        }
    }
}

// ── Scan types ───────────────────────────────────────────────────────────────

/// Per-language count + bytes.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct LanguageStats {
    pub file_count: u64,
    pub total_bytes: u64,
}

/// Result of `scan_directory`. Maps language tag (e.g. `"rust"`,
/// `"typescript"`, `"python"`) to its `LanguageStats`. Extension →
/// language mapping is hardcoded; unknown extensions land in `"other"`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ScanReport {
    pub root: String,
    pub total_files: u64,
    pub total_bytes: u64,
    pub languages: std::collections::BTreeMap<String, LanguageStats>,
}

// ── scan_directory ───────────────────────────────────────────────────────────

/// Directory names that will be pruned during the walk (case-sensitive).
const SKIP_DIRS: &[&str] = &[
    "target",
    "node_modules",
    ".git",
    "dist",
    "build",
    "__pycache__",
    ".idea",
    ".vscode",
];

/// Maximum file size included in counts (2 MiB). §5.17.2 big-file skip.
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

/// Map a lowercase file extension to a language tag.
fn ext_to_language(ext: &str) -> &'static str {
    match ext {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "mjs" | "jsx" => "javascript",
        "py" => "python",
        "go" => "go",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" | "hh" => "cpp",
        "swift" => "swift",
        "rb" => "ruby",
        "php" => "php",
        "scala" => "scala",
        "sh" | "bash" => "shell",
        "md" => "markdown",
        "toml" | "yaml" | "yml" | "json" => "config",
        _ => "other",
    }
}

/// Walk `path` recursively, classify files by extension, and return a
/// [`ScanReport`]. Skips hidden directories, common vendored dirs, and
/// files > 2 MiB.
pub fn scan_directory(path: &std::path::Path) -> Result<ScanReport, CorpusError> {
    use walkdir::WalkDir;

    let root = path.to_string_lossy().into_owned();
    let mut report = ScanReport {
        root,
        ..Default::default()
    };

    let walker = WalkDir::new(path).into_iter();
    for entry in walker.filter_entry(|e| {
        // Prune skip-dirs and hidden dirs at the directory level.
        if e.file_type().is_dir() {
            let name = e.file_name().to_string_lossy();
            if name.starts_with('.') {
                return false;
            }
            if SKIP_DIRS.contains(&name.as_ref()) {
                return false;
            }
        }
        true
    }) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue, // permission errors: skip silently
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let file_bytes = metadata.len();
        if file_bytes > MAX_FILE_BYTES {
            continue;
        }
        let ext = entry
            .path()
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let lang = ext_to_language(&ext);
        let stats = report.languages.entry(lang.to_string()).or_default();
        stats.file_count += 1;
        stats.total_bytes += file_bytes;
        report.total_files += 1;
        report.total_bytes += file_bytes;
    }

    Ok(report)
}

// ── Errors ───────────────────────────────────────────────────────────────────

/// Errors produced by `nom-corpus`. Minimal until drivers land.
#[derive(Debug, Error)]
pub enum CorpusError {
    #[error("unknown ecosystem: {0} (expected pypi|github-{{js,ts,python,rust,go,jvm,ccpp,swift,ruby,php}})")]
    UnknownEcosystem(String),
    #[error("driver not yet implemented: {0:?}")]
    DriverNotYetImplemented(Ecosystem),
    #[error("repo skipped: {reason}")]
    Skipped { reason: String },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecosystem_round_trips_through_flag_value() {
        for e in Ecosystem::ALL {
            assert_eq!(ecosystem_from_str(e.flag_value()), Some(*e));
        }
    }

    #[test]
    fn ecosystem_all_covers_every_variant() {
        for e in Ecosystem::ALL {
            // Exhaustive-match sentinel (iter 24 pattern).
            let _: () = match e {
                Ecosystem::PyPI
                | Ecosystem::GitHubJavaScript
                | Ecosystem::GitHubTypeScript
                | Ecosystem::GitHubPython
                | Ecosystem::GitHubRust
                | Ecosystem::GitHubGo
                | Ecosystem::GitHubJavaKotlin
                | Ecosystem::GitHubCCpp
                | Ecosystem::GitHubSwift
                | Ecosystem::GitHubRuby
                | Ecosystem::GitHubPHP => (),
            };
        }
        assert_eq!(Ecosystem::ALL.len(), 11);
    }

    #[test]
    fn ecosystem_from_str_accepts_aliases() {
        assert_eq!(ecosystem_from_str("rust"), Some(Ecosystem::GitHubRust));
        assert_eq!(ecosystem_from_str("TS"), Some(Ecosystem::GitHubTypeScript));
        assert_eq!(ecosystem_from_str("cpp"), Some(Ecosystem::GitHubCCpp));
        assert_eq!(ecosystem_from_str("haskell"), None);
    }

    #[test]
    fn default_config_matches_plan_defaults() {
        let c = CorpusConfig::default();
        assert_eq!(c.top_n, 500);
        assert_eq!(c.max_clone_bytes, 2 * 1024 * 1024 * 1024);
        assert_eq!(c.bandwidth_limit_bytes_per_sec, 20 * 1024 * 1024);
        assert!(c.include_pattern.is_none());
    }

    #[test]
    fn scan_directory_counts_files_by_extension() {
        use std::fs;
        use std::io::Write;

        let tmp = std::env::temp_dir().join("nom_corpus_scan_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // Create a few source files of different languages.
        let files = [
            ("main.rs", b"fn main() {}" as &[u8]),
            ("lib.rs", b"pub fn foo() {}"),
            ("index.ts", b"export const x = 1;"),
            ("script.py", b"print('hi')"),
            ("README.md", b"# Hello"),
        ];
        for (name, content) in &files {
            let mut f = fs::File::create(tmp.join(name)).unwrap();
            f.write_all(content).unwrap();
        }

        // Create a skipped dir that should not be counted.
        let skip = tmp.join("node_modules");
        fs::create_dir_all(&skip).unwrap();
        let mut f = fs::File::create(skip.join("big.js")).unwrap();
        f.write_all(b"ignored").unwrap();

        let report = scan_directory(&tmp).unwrap();
        assert_eq!(report.total_files, 5, "expected 5 files, got {}", report.total_files);
        assert_eq!(report.languages["rust"].file_count, 2);
        assert_eq!(report.languages["typescript"].file_count, 1);
        assert_eq!(report.languages["python"].file_count, 1);
        assert_eq!(report.languages["markdown"].file_count, 1);
        assert!(!report.languages.contains_key("javascript"), "node_modules should be skipped");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn scan_directory_skips_large_files() {
        use std::fs;
        use std::io::Write;

        let tmp = std::env::temp_dir().join("nom_corpus_scan_large_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // Small file: should be counted.
        let mut f = fs::File::create(tmp.join("small.rs")).unwrap();
        f.write_all(b"fn small() {}").unwrap();

        // Large file: > 2 MiB, should be skipped.
        {
            let mut f = fs::File::create(tmp.join("large.rs")).unwrap();
            let big = vec![b'x'; 3 * 1024 * 1024];
            f.write_all(&big).unwrap();
            // Drop flushes and closes the file so metadata is up-to-date.
        }

        let report = scan_directory(&tmp).unwrap();
        assert_eq!(report.total_files, 1);
        assert_eq!(report.languages["rust"].file_count, 1);

        let _ = fs::remove_dir_all(&tmp);
    }
}
