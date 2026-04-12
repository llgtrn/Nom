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

pub mod checkpoint;
pub use checkpoint::IngestCheckpoint;

pub mod equivalence_gate;
pub use equivalence_gate::{GateError, GateOutcome, run_gate};

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
/// Duplicate detection: uses `INSERT OR IGNORE` — if a row with the same
/// SHA-256 id already exists, the insert is a no-op and the file is counted
/// as a duplicate. No prior SELECT is issued.
pub fn ingest_directory(
    path: &std::path::Path,
    dict: &nom_dict::NomDict,
) -> Result<IngestReport, CorpusError> {
    ingest_directory_with_conn(path, dict)
}

/// Internal implementation shared by [`ingest_directory`] and
/// [`ingest_parent`]. Takes a pre-opened [`nom_dict::NomDict`] so that
/// `ingest_parent` can reuse a single connection across all repos.
fn ingest_directory_with_conn(
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

    // Begin a per-repo transaction: all INSERTs commit atomically at the end.
    // If we crash mid-walk the RAII guard rolls back, leaving the DB clean so
    // the checkpoint mechanism can retry this repo from scratch.
    let tx = dict
        .begin_transaction()
        .map_err(|e| CorpusError::Skipped { reason: format!("begin_transaction: {e}") })?;

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
                    let s: String = line.chars().take(117).collect();
                    format!("{s} [Partial]")
                }
                None => format!("{} source, {} bytes [Partial]", lang, file_bytes_len),
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
            // §5.17.4: corpus-ingested entries land as Partial. The §5.2
            // equivalence gate (translator round-trip + contract test) is
            // what lifts them to Complete; not yet wired — see
            // nom_corpus::equivalence_gate module stub.
            status: EntryStatus::Partial,
            translation_score: None,
            is_canonical: true,
            deprecated_by: None,
            created_at: now,
            updated_at: None,
        };

        match dict.upsert_entry_if_new(&e) {
            Ok(true) => {
                *report.per_language.entry(lang.to_owned()).or_insert(0) += 1;
                report.files_ingested += 1;
                report.bytes_ingested += file_bytes_len;
            }
            Ok(false) => {
                report.duplicates += 1;
            }
            Err(e) => {
                eprintln!("nom: upsert error for {id}: {e}");
                report.files_skipped += 1;
            }
        }
    }

    tx.commit()
        .map_err(|e| CorpusError::Skipped { reason: format!("commit: {e}") })?;

    Ok(report)
}

// ── ingest_parent ─────────────────────────────────────────────────────────────

/// Result of [`ingest_parent`]: per-repo breakdown + aggregate.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ParentIngestReport {
    pub parent: String,
    /// Map from repo name (child-directory basename) to that repo's
    /// [`IngestReport`]. Only subdirectories that produced at least one
    /// ingested file are included.
    pub repos: std::collections::BTreeMap<String, IngestReport>,
    /// Aggregate across all repos.
    pub aggregate: IngestReport,
    /// Count of immediate-child directories that were skipped
    /// (hidden, or produced zero ingestible files).
    pub skipped_repos: u64,
    /// Count of repos skipped because they were already recorded in
    /// the checkpoint (completed in a prior interrupted run).
    pub resumed_repos: u64,
}

/// Walks `parent_dir`'s immediate children; for each child that is a
/// directory (not hidden), calls [`ingest_directory_with_conn`].
/// Aggregates all results. Reuses the same [`nom_dict::NomDict`]
/// connection across repos for performance.
///
/// A checkpoint file is maintained next to `dict_path` (see
/// [`IngestCheckpoint`]). Repos already present in the checkpoint are
/// skipped, enabling seamless resume after a crash. Pass
/// `reset_checkpoint = true` to delete the checkpoint before starting
/// (effectively a fresh run).
///
/// Progress is printed to stderr every 10 repos so operators see
/// activity during large runs (e.g. 231-repo corpus).
pub fn ingest_parent(
    parent: &std::path::Path,
    dict_path: &std::path::Path,
    reset_checkpoint: bool,
) -> Result<ParentIngestReport, CorpusError> {
    // Optionally wipe the checkpoint for a clean restart.
    if reset_checkpoint {
        let cp_path = IngestCheckpoint::path_for(dict_path);
        let _ = std::fs::remove_file(&cp_path); // best-effort; ignore error
    }

    let dict_db = nom_dict::NomDict::open_in_place(dict_path)
        .map_err(|e| CorpusError::Io(std::io::Error::other(e.to_string())))?;

    let mut checkpoint = IngestCheckpoint::load(dict_path, parent);

    let mut report = ParentIngestReport {
        parent: parent.to_string_lossy().into_owned(),
        ..Default::default()
    };

    // Collect and sort child dirs for deterministic ordering.
    let mut children: Vec<std::path::PathBuf> = Vec::new();
    for entry in std::fs::read_dir(parent)? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let ft = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if !ft.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }
        children.push(entry.path());
    }
    children.sort();

    let total_children = children.len() as u64;
    let mut processed: u64 = 0;
    for child in &children {
        let repo_name = child
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        // Skip repos already committed in a prior run.
        if checkpoint.is_completed(&repo_name) {
            report.resumed_repos += 1;
            processed += 1;
            if processed % 10 == 0 {
                eprintln!(
                    "nom: processed {}/{} repos ({} resumed from checkpoint), {} files ingested so far...",
                    processed, total_children, report.resumed_repos, report.aggregate.files_ingested
                );
            }
            continue;
        }

        let repo_report = match ingest_directory_with_conn(child, &dict_db) {
            Ok(r) => r,
            Err(_) => {
                report.skipped_repos += 1;
                processed += 1;
                if processed % 10 == 0 {
                    eprintln!(
                        "nom: processed {}/{} repos ({} resumed from checkpoint), {} files ingested so far...",
                        processed, total_children, report.resumed_repos, report.aggregate.files_ingested
                    );
                }
                continue;
            }
        };

        // Mark this repo done in the checkpoint regardless of whether it
        // produced files (no point re-scanning an empty repo).
        checkpoint.mark_completed(repo_name.clone());
        checkpoint.save(dict_path);

        if repo_report.files_ingested == 0 {
            report.skipped_repos += 1;
        } else {
            // Merge per_language counts into aggregate.
            for (lang, count) in &repo_report.per_language {
                *report.aggregate.per_language.entry(lang.clone()).or_insert(0) += count;
            }
            report.aggregate.files_ingested += repo_report.files_ingested;
            report.aggregate.bytes_ingested += repo_report.bytes_ingested;
            report.aggregate.duplicates += repo_report.duplicates;
            report.aggregate.files_skipped += repo_report.files_skipped;
            report.repos.insert(repo_name, repo_report);
        }

        processed += 1;
        if processed % 10 == 0 {
            eprintln!(
                "nom: processed {}/{} repos ({} resumed from checkpoint), {} files ingested so far...",
                processed, total_children, report.resumed_repos, report.aggregate.files_ingested
            );
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

// ── lift_partial ─────────────────────────────────────────────────────────────

/// Outcome returned by [`lift_partial`].
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum LiftReport {
    /// Entry was lifted. `partial_id` stays Partial; `complete_id` is the new
    /// Complete entry. `is_new` is `true` if the complete entry was freshly
    /// inserted, `false` if it already existed (dedup path).
    Lifted {
        partial_id: String,
        complete_id: String,
        is_new: bool,
    },
    /// The gate rejected translation (body could not be rendered in Nom).
    Rejected { reason: String },
    /// No translator for this language yet.
    NotYetImplemented { language: String },
}

/// Errors from [`lift_partial`].
#[derive(Debug, Error)]
pub enum LiftError {
    #[error("entry not found: {0}")]
    NotFound(String),
    #[error("entry is not Partial (status={0})")]
    NotPartial(String),
    #[error("body_bytes missing for entry {0}")]
    NoBody(String),
    #[error("gate error: {0}")]
    Gate(#[from] GateError),
    #[error("dict error: {0}")]
    Dict(String),
}

/// §5.2→§5.10 gate loop for a single Partial entry.
///
/// Fetches the entry from `dict`, runs [`run_gate`], and on success:
/// - Upserts a new Complete entry with `body_kind: nom_source`.
/// - Adds a `SupersededBy(partial_id → complete_id)` edge.
///
/// The original Partial entry is left untouched.
pub fn lift_partial(dict: &nom_dict::NomDict, id: &str) -> Result<LiftReport, LiftError> {
    use nom_types::{body_kind, Contract, EdgeType, Entry, EntryStatus, GraphEdge};

    // 1. Fetch entry.
    let entry = dict
        .get_entry(id)
        .map_err(|e| LiftError::Dict(e.to_string()))?
        .ok_or_else(|| LiftError::NotFound(id.to_string()))?;

    // 2. Verify it is Partial.
    if entry.status != EntryStatus::Partial {
        return Err(LiftError::NotPartial(entry.status.as_str().to_string()));
    }

    // 3. Extract body bytes.
    let raw_bytes = entry
        .body_bytes
        .as_deref()
        .filter(|b| !b.is_empty())
        .ok_or_else(|| LiftError::NoBody(id.to_string()))?;

    let body_kind_tag = entry
        .body_kind
        .as_deref()
        .unwrap_or("unknown");

    let language = &entry.language;

    // 4. Run the equivalence gate.
    let outcome = run_gate(id, body_kind_tag, raw_bytes, language)?;

    match outcome {
        GateOutcome::Lifted { nom_source_id, nom_body } => {
            // 5a. Build the new Complete entry.
            let describe = format!("{} source, lifted to Nom", language);
            let complete = Entry {
                id: nom_source_id.clone(),
                word: entry.word.clone(),
                variant: entry.variant.clone(),
                kind: entry.kind.clone(),
                language: "nom".to_string(),
                describe: Some(describe),
                concept: entry.concept.clone(),
                body: None,
                body_nom: None,
                body_bytes: Some(nom_body),
                body_kind: Some(body_kind::NOM_SOURCE.to_string()),
                contract: Contract::default(),
                status: EntryStatus::Complete,
                translation_score: None,
                is_canonical: true,
                deprecated_by: None,
                created_at: chrono_now(),
                updated_at: None,
            };

            let is_new = dict.upsert_entry_if_new(&complete).map_err(|e| LiftError::Dict(e.to_string()))?;

            // 5b. Add SupersededBy edge partial → complete.
            let edge = GraphEdge {
                edge_id: 0,
                from_id: id.to_string(),
                to_id: nom_source_id.clone(),
                edge_type: EdgeType::SupersededBy,
                confidence: 1.0,
            };
            dict.add_graph_edge(&edge).map_err(|e| LiftError::Dict(e.to_string()))?;

            Ok(LiftReport::Lifted {
                partial_id: id.to_string(),
                complete_id: nom_source_id,
                is_new,
            })
        }
        GateOutcome::PartialRejected { reason } => Ok(LiftReport::Rejected { reason }),
        GateOutcome::NotYetImplemented { language } => {
            Ok(LiftReport::NotYetImplemented { language })
        }
    }
}

// ── lift_all ──────────────────────────────────────────────────────────────────

/// Summary returned by [`lift_all`].
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct LiftAllReport {
    /// How many Partial entries were scanned.
    pub partials_scanned: u64,
    /// How many got lifted to a new Complete entry.
    pub lifted: u64,
    /// How many re-linked to an existing Complete (translator hash matched).
    pub relinked: u64,
    /// How many rejected (translator unsupported or parse error).
    pub rejected: u64,
    /// How many skipped (language not yet implemented).
    pub not_yet_implemented: u64,
    /// How many failed (harness-level errors: missing body_bytes, etc).
    pub errors: u64,
    /// Per-reason rejection count (top 20 distinct reasons; overflow → "(other)").
    pub rejection_reasons: std::collections::BTreeMap<String, u64>,
}

const MAX_REJECTION_REASONS: usize = 20;

/// Sweep every `status: Partial` entry in the dict through [`lift_partial`].
/// `max` caps the number of entries scanned per run (0 = unlimited).
pub fn lift_all(dict: &nom_dict::NomDict, max: usize) -> Result<LiftAllReport, LiftError> {
    let cap = if max == 0 { None } else { Some(max) };
    let ids = dict
        .list_partial_ids(cap)
        .map_err(|e| LiftError::Dict(e.to_string()))?;

    let total = ids.len() as u64;
    let mut report = LiftAllReport {
        partials_scanned: total,
        ..Default::default()
    };

    for (idx, id) in ids.iter().enumerate() {
        // Progress to stderr every 100 entries (suppress when small batches).
        if total >= 100 && idx > 0 && idx % 100 == 0 {
            eprintln!(
                "nom: lift-all: {}/{} scanned, {} lifted, {} relinked, {} rejected...",
                idx, total, report.lifted, report.relinked, report.rejected
            );
        }

        match lift_partial(dict, id) {
            Ok(LiftReport::Lifted { is_new, .. }) => {
                if is_new {
                    report.lifted += 1;
                } else {
                    report.relinked += 1;
                }
            }
            Ok(LiftReport::Rejected { reason }) => {
                report.rejected += 1;
                let current_distinct = report.rejection_reasons.len();
                if report.rejection_reasons.contains_key(&reason) {
                    *report.rejection_reasons.get_mut(&reason).unwrap() += 1;
                } else if current_distinct < MAX_REJECTION_REASONS {
                    report.rejection_reasons.insert(reason, 1);
                } else {
                    *report.rejection_reasons
                        .entry("(other)".to_string())
                        .or_insert(0) += 1;
                }
            }
            Ok(LiftReport::NotYetImplemented { .. }) => {
                report.not_yet_implemented += 1;
            }
            Err(LiftError::NoBody(_)) | Err(LiftError::NotFound(_)) => {
                report.errors += 1;
            }
            Err(e) => {
                // Gate errors and dict errors count as harness errors; keep going.
                let _ = e;
                report.errors += 1;
            }
        }
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
    fn ingest_parent_aggregates_across_repos() {
        use std::fs;
        use std::io::Write;
        use nom_dict::NomDict;

        let tmp = std::env::temp_dir().join("nom_corpus_ingest_parent_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // Three child repos, each with 2 .rs files.
        for repo in &["repo_a", "repo_b", "repo_c"] {
            let repo_dir = tmp.join(repo);
            fs::create_dir_all(&repo_dir).unwrap();
            for (i, name) in ["lib.rs", "main.rs"].iter().enumerate() {
                let mut f = fs::File::create(repo_dir.join(name)).unwrap();
                // Unique content per file to avoid cross-repo dedup.
                f.write_all(
                    format!("// {repo} file {i}\npub fn f_{repo}_{i}() {{}}")
                        .as_bytes(),
                )
                .unwrap();
            }
        }

        // A hidden dir should be skipped.
        let hidden = tmp.join(".hidden");
        fs::create_dir_all(&hidden).unwrap();
        let mut f = fs::File::create(hidden.join("skip.rs")).unwrap();
        f.write_all(b"fn hidden() {}").unwrap();

        // A plain file at the parent level should be skipped (not a dir).
        let mut f = fs::File::create(tmp.join("README.md")).unwrap();
        f.write_all(b"# top level").unwrap();

        // Use an in-memory dict via a temp file path.
        let db_path = tmp.join("test_parent.db");
        // NomDict::open_in_place creates the file; test uses that path.
        {
            let _dict = NomDict::open_in_place(&db_path).unwrap();
        }

        let report = super::ingest_parent(&tmp, &db_path, false).unwrap();

        assert_eq!(report.aggregate.files_ingested, 6, "3 repos × 2 files = 6");
        assert_eq!(report.repos.len(), 3, "all 3 repos produced files");
        assert_eq!(report.skipped_repos, 0, "no repos skipped");
        // Each repo has 2 rust files.
        for (_, repo_report) in &report.repos {
            assert_eq!(repo_report.files_ingested, 2);
            assert_eq!(repo_report.per_language["rust"], 2);
        }
        assert_eq!(report.aggregate.per_language["rust"], 6);

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

    /// `ingest_directory_with_conn` wraps the walk in a transaction: all files
    /// must be visible after the call (committed), and an explicit rollback must
    /// leave the dict empty.
    #[test]
    fn ingest_directory_uses_transaction() {
        use std::fs;
        use std::io::Write;
        use nom_dict::NomDict;

        // ── Part 1: normal commit path ──────────────────────────────────────
        let tmp = std::env::temp_dir().join("nom_corpus_tx_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let files = [
            ("a.rs", b"fn a() {}" as &[u8]),
            ("b.rs", b"fn b() {}"),
            ("c.rs", b"fn c() {}"),
        ];
        for (name, content) in &files {
            let mut f = fs::File::create(tmp.join(name)).unwrap();
            f.write_all(content).unwrap();
        }

        let dict = NomDict::open_in_memory().unwrap();
        let report = super::ingest_directory(&tmp, &dict).unwrap();
        assert_eq!(report.files_ingested, 3, "all 3 files should be committed");
        assert_eq!(dict.count().unwrap(), 3, "transaction must have committed");

        // ── Part 2: explicit rollback leaves the dict clean ─────────────────
        // Begin a new transaction, manually insert 2 entries, then roll back.
        let tx = dict.begin_transaction().unwrap();
        use nom_types::{Contract, Entry, EntryKind, EntryStatus};
        for i in 0..2u8 {
            let e = Entry {
                id: format!("rollback_test_{i:02x}"),
                word: format!("rollback_{i}"),
                variant: None,
                kind: EntryKind::Module,
                language: "rust".into(),
                describe: None,
                concept: None,
                body: None,
                body_nom: None,
                body_bytes: None,
                body_kind: None,
                contract: Contract::default(),
                status: EntryStatus::Complete,
                translation_score: None,
                is_canonical: true,
                deprecated_by: None,
                created_at: "2025-01-01T00:00:00Z".into(),
                updated_at: None,
            };
            dict.upsert_entry(&e).unwrap();
        }
        // Row count inside the transaction should be 5 (3 committed + 2 pending).
        assert_eq!(dict.count().unwrap(), 5);
        // Roll back: the 2 pending entries must disappear.
        tx.rollback().unwrap();
        assert_eq!(dict.count().unwrap(), 3, "rollback must discard the 2 pending rows");

        let _ = fs::remove_dir_all(&tmp);
    }

    /// Second run with the same dict + parent must skip all three repos
    /// (checkpoint records them as completed) and report resumed_repos == 3
    /// with zero new files ingested.
    #[test]
    fn ingest_parent_resumes_from_checkpoint() {
        use std::fs;
        use std::io::Write;
        use nom_dict::NomDict;

        let tmp = std::env::temp_dir().join("nom_corpus_checkpoint_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // Three child repos, each with 2 distinct .rs files.
        for repo in &["alpha", "beta", "gamma"] {
            let repo_dir = tmp.join(repo);
            fs::create_dir_all(&repo_dir).unwrap();
            for (i, name) in ["lib.rs", "main.rs"].iter().enumerate() {
                let mut f = fs::File::create(repo_dir.join(name)).unwrap();
                f.write_all(
                    format!("// {repo} file {i}\npub fn chk_{repo}_{i}() {{}}")
                        .as_bytes(),
                )
                .unwrap();
            }
        }

        let db_path = tmp.join("chk_test.db");
        {
            let _dict = NomDict::open_in_place(&db_path).unwrap();
        }

        // ── First run: ingest all 3 repos, checkpoint written after each ──
        let r1 = super::ingest_parent(&tmp, &db_path, false).unwrap();
        assert_eq!(r1.aggregate.files_ingested, 6, "first run: 3 repos × 2 files = 6");
        assert_eq!(r1.resumed_repos, 0, "first run: nothing resumed yet");

        // Verify checkpoint file exists and lists all 3 repos.
        let cp_path = IngestCheckpoint::path_for(&db_path);
        assert!(cp_path.exists(), "checkpoint file must be written after first run");
        let cp = IngestCheckpoint::load(&db_path, &tmp);
        assert!(cp.is_completed("alpha"));
        assert!(cp.is_completed("beta"));
        assert!(cp.is_completed("gamma"));

        // Tamper with a file inside one repo — to prove it isn't re-read.
        fs::write(tmp.join("beta").join("lib.rs"), b"fn tampered() {}").unwrap();

        // ── Second run: all repos already in checkpoint, nothing re-ingested ──
        let r2 = super::ingest_parent(&tmp, &db_path, false).unwrap();
        assert_eq!(r2.resumed_repos, 3, "second run: all 3 repos resumed");
        assert_eq!(
            r2.aggregate.files_ingested, 0,
            "second run: no new files (checkpoint skipped all repos)"
        );

        // The tampered file was NOT re-ingested (repo was skipped entirely).
        // Dict still holds only 6 entries from run 1.
        let dict_check = NomDict::open_in_place(&db_path).unwrap();
        assert_eq!(dict_check.count().unwrap(), 6, "dict must still hold exactly 6 entries");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn lift_partial_end_to_end() {
        use std::fs;
        use std::io::Write;
        use nom_dict::NomDict;

        let tmp = std::env::temp_dir().join("nom_corpus_lift_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // Single rust file with a liftable function.
        let src_dir = tmp.join("src");
        fs::create_dir_all(&src_dir).unwrap();
        let mut f = fs::File::create(src_dir.join("add.rs")).unwrap();
        f.write_all(b"fn add(a: i64, b: i64) -> i64 { a + b }").unwrap();

        let db_path = tmp.join("test.db");
        let dict = NomDict::open_in_place(&db_path).unwrap();

        // Ingest — lands as Partial.
        let report = ingest_directory(&src_dir, &dict).unwrap();
        assert_eq!(report.files_ingested, 1);

        // Find the partial entry (body_kind = "rust_source").
        let partials = dict.find_by_body_kind("rust_source", 10).unwrap();
        assert_eq!(partials.len(), 1, "expected exactly one rust_source entry");
        let partial_id = partials[0].id.clone();

        // Lift it.
        let result = super::lift_partial(&dict, &partial_id).unwrap();
        match result {
            LiftReport::Lifted { partial_id: pid, complete_id, is_new } => {
                assert_eq!(pid, partial_id);
                assert!(!complete_id.is_empty());
                assert!(is_new, "first lift must insert a new entry");

                // Verify the new complete entry exists with nom_source body_kind.
                let complete_entries = dict.find_by_body_kind("nom_source", 10).unwrap();
                assert_eq!(complete_entries.len(), 1, "expected one nom_source entry");
                let ce = &complete_entries[0];
                assert_eq!(ce.id, complete_id);
                assert_eq!(ce.status, nom_types::EntryStatus::Complete);
                assert!(ce.body_bytes.as_ref().map_or(false, |b| !b.is_empty()));

                // Verify the SupersededBy edge exists.
                let conn = dict.connection();
                let edge_count: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM entry_graph_edges WHERE from_id=?1 AND to_id=?2 AND edge_type='superseded_by'",
                        rusqlite::params![partial_id, complete_id],
                        |r| r.get(0),
                    )
                    .unwrap();
                assert_eq!(edge_count, 1, "SupersededBy edge must exist");
            }
            other => panic!("expected LiftReport::Lifted, got {other:?}"),
        }

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn lift_all_sweeps_partials() {
        use std::fs;
        use std::io::Write;
        use nom_dict::NomDict;

        let tmp = std::env::temp_dir().join("nom_corpus_lift_all_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let src_dir = tmp.join("src");
        fs::create_dir_all(&src_dir).unwrap();

        // File 1: liftable function.
        let mut f1 = fs::File::create(src_dir.join("add.rs")).unwrap();
        f1.write_all(b"fn add(a: i64, b: i64) -> i64 { a + b }").unwrap();

        // File 2: a struct — may translate or be rejected; either outcome is valid.
        let mut f2 = fs::File::create(src_dir.join("point.rs")).unwrap();
        f2.write_all(b"struct Point { x: f64, y: f64 }").unwrap();

        // File 3: just a comment — body content is minimal and likely rejected.
        let mut f3 = fs::File::create(src_dir.join("comment.rs")).unwrap();
        f3.write_all(b"// this is a comment").unwrap();

        let db_path = tmp.join("test_all.db");
        let dict = NomDict::open_in_place(&db_path).unwrap();

        let ingest_report = ingest_directory(&src_dir, &dict).unwrap();
        assert_eq!(ingest_report.files_ingested, 3, "all 3 files must be ingested");

        let report = super::lift_all(&dict, 0).unwrap();

        assert!(
            report.partials_scanned >= 3,
            "expected >= 3 scanned, got {}",
            report.partials_scanned
        );
        assert!(
            report.lifted >= 1,
            "expected >= 1 lifted, got {}",
            report.lifted
        );
        assert!(
            report.rejected >= 1 || report.not_yet_implemented >= 1,
            "expected >= 1 rejected or not-yet-implemented (got rejected={}, nyi={})",
            report.rejected,
            report.not_yet_implemented
        );

        // Verify a SupersededBy edge exists for at least one lifted entry.
        if report.lifted >= 1 {
            let conn = dict.connection();
            let edge_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM entry_graph_edges WHERE edge_type='superseded_by'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert!(
                edge_count >= 1,
                "expected >= 1 SupersededBy edge, got {edge_count}"
            );
        }

        let _ = fs::remove_dir_all(&tmp);
    }
}
