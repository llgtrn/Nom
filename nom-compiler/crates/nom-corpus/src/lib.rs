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

// ── ingest_directory ─────────────────────────────────────────────────────────

/// 100 KiB: max size for "other" extension files.  Source files for known
/// languages go up to MAX_FILE_BYTES (2 MiB) before being skipped.
const MAX_OTHER_BYTES: u64 = 100 * 1024;

/// Map a language tag (from `ext_to_language`) to the corresponding
/// `body_kind` constant. Returns `body_kind::OTHER_SOURCE` for
/// unmapped tags (`"other"`, `"scala"`, `"shell"`, `"markdown"`, `"config"`).
fn lang_to_body_kind(lang: &str) -> &'static str {
    use nom_types::body_kind;
    match lang {
        "rust"       => body_kind::RUST_SOURCE,
        "typescript" => body_kind::TYPESCRIPT_SOURCE,
        "javascript" => body_kind::JAVASCRIPT_SOURCE,
        "python"     => body_kind::PYTHON_SOURCE,
        "go"         => body_kind::GO_SOURCE,
        "java"       => body_kind::JAVA_SOURCE,
        "kotlin"     => body_kind::KOTLIN_SOURCE,
        "c"          => body_kind::C_SOURCE,
        "cpp"        => body_kind::CPP_SOURCE,
        "swift"      => body_kind::SWIFT_SOURCE,
        "ruby"       => body_kind::RUBY_SOURCE,
        "php"        => body_kind::PHP_SOURCE,
        _            => body_kind::OTHER_SOURCE,
    }
}

/// Summary returned by [`ingest_directory`].
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct IngestReport {
    pub root: String,
    pub files_ingested: u64,
    pub files_skipped: u64,
    pub bytes_ingested: u64,
    /// Per-language file count (language tag → count).
    pub per_language: std::collections::BTreeMap<String, u64>,
    /// How many files were duplicate (same SHA-256 already in dict).
    pub duplicates: u64,
}

/// Walk `path` recursively, hash each eligible source file, and upsert
/// one v2 `Entry` row per file into `dict`.
///
/// Skips the same directories as [`scan_directory`] plus applies an extra
/// rule: if a file's extension maps to `"other"` and its size exceeds 100
/// KiB it is skipped (likely a binary asset).  All other files ≤ 2 MiB
/// are ingested.
///
/// Duplicate detection: if `dict.get_entry(sha256_hex)` already returns
/// `Some(_)`, the file is counted as a duplicate and skipped (no
/// re-upsert). This is cheap because the row already exists.
pub fn ingest_directory(
    path: &std::path::Path,
    dict: &nom_dict::NomDict,
) -> Result<IngestReport, CorpusError> {
    use sha2::{Digest, Sha256};
    use walkdir::WalkDir;
    use nom_types::{Contract, Entry, EntryKind, EntryStatus};

    let root = path.to_string_lossy().into_owned();
    let mut report = IngestReport {
        root,
        ..Default::default()
    };

    let walker = WalkDir::new(path).into_iter();
    for entry in walker.filter_entry(|e| {
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
            Err(_) => {
                report.files_skipped += 1;
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => {
                report.files_skipped += 1;
                continue;
            }
        };
        let file_bytes_len = metadata.len();
        let ext = entry
            .path()
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let lang = ext_to_language(&ext);

        // Apply size limits: known-lang files up to MAX_FILE_BYTES,
        // "other" files only up to MAX_OTHER_BYTES.
        let size_limit = if lang == "other" { MAX_OTHER_BYTES } else { MAX_FILE_BYTES };
        if file_bytes_len > size_limit {
            report.files_skipped += 1;
            continue;
        }

        // Read bytes.
        let bytes = match std::fs::read(entry.path()) {
            Ok(b) => b,
            Err(_) => {
                report.files_skipped += 1;
                continue;
            }
        };

        // SHA-256 → hex id.
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let id = format!("{:x}", hasher.finalize());

        // Dedup: skip if already present.
        match dict.get_entry(&id) {
            Ok(Some(_)) => {
                report.duplicates += 1;
                continue;
            }
            Ok(None) => {}
            Err(_) => {
                report.files_skipped += 1;
                continue;
            }
        }

        // Build word: language prefix + cleaned file stem, ≤ 60 chars.
        let stem = entry
            .path()
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("file");
        let cleaned: String = stem
            .to_ascii_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
            .collect();
        let word_candidate = format!("{}_{}", lang, cleaned);
        let word: String = word_candidate.chars().take(60).collect();

        // Describe: first non-empty UTF-8 line ≤ 120 chars, else synthetic.
        let describe = {
            let maybe_text = std::str::from_utf8(&bytes).ok();
            let first_line = maybe_text
                .and_then(|s| s.lines().find(|l| !l.trim().is_empty()))
                .map(|l| l.trim())
                .filter(|l| !l.is_empty());
            match first_line {
                Some(line) => {
                    let s: String = line.chars().take(120).collect();
                    s
                }
                None => format!("{} source, {} bytes", lang, file_bytes_len),
            }
        };

        let body_kind = lang_to_body_kind(lang).to_owned();

        let now = chrono_now();
        let e = Entry {
            id: id.clone(),
            word,
            variant: Some(lang.to_owned()),
            kind: EntryKind::Module,
            language: lang.to_owned(),
            describe: Some(describe),
            concept: None,
            body: None,
            body_nom: None,
            body_bytes: Some(bytes.clone()),
            body_kind: Some(body_kind),
            contract: Contract::default(),
            status: EntryStatus::Complete,
            translation_score: None,
            is_canonical: true,
            deprecated_by: None,
            created_at: now,
            updated_at: None,
        };

        match dict.upsert_entry(&e) {
            Ok(_) => {
                *report.per_language.entry(lang.to_owned()).or_insert(0) += 1;
                report.files_ingested += 1;
                report.bytes_ingested += file_bytes_len;
            }
            Err(_) => {
                report.files_skipped += 1;
            }
        }
    }

    Ok(report)
}

/// ISO-8601 timestamp for `created_at` fields (UTC, second precision).
fn chrono_now() -> String {
    // Use std::time to avoid adding a chrono dep.  SQLite datetime('now')
    // format: "YYYY-MM-DD HH:MM:SS".
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (y, mo, d, h, mi, s) = secs_to_ymd_hms(secs);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}")
}

/// Minimal UTC calendar decomposition without external deps.
fn secs_to_ymd_hms(secs: u64) -> (u64, u64, u64, u64, u64, u64) {
    let s = secs % 60;
    let mins = secs / 60;
    let mi = mins % 60;
    let hours = mins / 60;
    let h = hours % 24;
    let days = hours / 24;
    // Gregorian calendar from day-count (days since 1970-01-01).
    // Adapted from https://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    (y, mo, d, h, mi, s)
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
    fn lang_to_body_kind_all_languages() {
        use nom_types::body_kind;
        assert_eq!(super::lang_to_body_kind("rust"),       body_kind::RUST_SOURCE);
        assert_eq!(super::lang_to_body_kind("typescript"), body_kind::TYPESCRIPT_SOURCE);
        assert_eq!(super::lang_to_body_kind("javascript"), body_kind::JAVASCRIPT_SOURCE);
        assert_eq!(super::lang_to_body_kind("python"),     body_kind::PYTHON_SOURCE);
        assert_eq!(super::lang_to_body_kind("go"),         body_kind::GO_SOURCE);
        assert_eq!(super::lang_to_body_kind("java"),       body_kind::JAVA_SOURCE);
        assert_eq!(super::lang_to_body_kind("kotlin"),     body_kind::KOTLIN_SOURCE);
        assert_eq!(super::lang_to_body_kind("c"),          body_kind::C_SOURCE);
        assert_eq!(super::lang_to_body_kind("cpp"),        body_kind::CPP_SOURCE);
        assert_eq!(super::lang_to_body_kind("swift"),      body_kind::SWIFT_SOURCE);
        assert_eq!(super::lang_to_body_kind("ruby"),       body_kind::RUBY_SOURCE);
        assert_eq!(super::lang_to_body_kind("php"),        body_kind::PHP_SOURCE);
        assert_eq!(super::lang_to_body_kind("other"),      body_kind::OTHER_SOURCE);
        assert_eq!(super::lang_to_body_kind("markdown"),   body_kind::OTHER_SOURCE);
        assert_eq!(super::lang_to_body_kind("config"),     body_kind::OTHER_SOURCE);
    }

    #[test]
    fn ingest_directory_populates_dict() {
        use std::fs;
        use std::io::Write;
        use nom_dict::NomDict;
        use nom_types::body_kind;

        let tmp = std::env::temp_dir().join("nom_corpus_ingest_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // Three small source files.
        let files = [
            ("lib.rs",   b"pub fn add(a: i32, b: i32) -> i32 { a + b }" as &[u8]),
            ("main.py",  b"def greet(name): return f'hello {name}'"),
            ("index.ts", b"export const id = (x: number) => x;"),
        ];
        for (name, content) in &files {
            let mut f = fs::File::create(tmp.join(name)).unwrap();
            f.write_all(content).unwrap();
        }

        // Ingest into an in-memory dict.
        let dict = NomDict::open_in_memory().unwrap();
        let report = super::ingest_directory(&tmp, &dict).unwrap();

        assert_eq!(report.files_ingested, 3, "expected 3 ingested, got {}", report.files_ingested);
        assert_eq!(report.duplicates, 0);
        assert_eq!(report.files_skipped, 0);

        // Per-language counts.
        assert_eq!(report.per_language["rust"], 1);
        assert_eq!(report.per_language["python"], 1);
        assert_eq!(report.per_language["typescript"], 1);

        // Body kind filtering works via find_by_body_kind.
        let rust_entries = dict.find_by_body_kind(body_kind::RUST_SOURCE, 10).unwrap();
        assert_eq!(rust_entries.len(), 1);
        let rs = &rust_entries[0];
        assert!(rs.body_bytes.as_ref().map_or(false, |b| !b.is_empty()));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn ingest_directory_dedup() {
        use std::fs;
        use std::io::Write;
        use nom_dict::NomDict;

        let tmp = std::env::temp_dir().join("nom_corpus_dedup_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let subdir = tmp.join("sub");
        fs::create_dir_all(&subdir).unwrap();

        // Same content in two locations → same SHA-256 → dedup.
        let content = b"fn identical() {}";
        let mut f1 = fs::File::create(tmp.join("a.rs")).unwrap();
        f1.write_all(content).unwrap();
        let mut f2 = fs::File::create(subdir.join("b.rs")).unwrap();
        f2.write_all(content).unwrap();

        let dict = NomDict::open_in_memory().unwrap();
        let report = super::ingest_directory(&tmp, &dict).unwrap();

        assert_eq!(report.files_ingested, 1, "first copy must be ingested");
        assert_eq!(report.duplicates, 1, "second copy must be detected as dup");

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
