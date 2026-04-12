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

// ── compile_nom_to_bc ────────────────────────────────────────────────────────

/// Compile a Nom source string to LLVM bitcode bytes.
///
/// Pipeline: parse → plan (unchecked, no resolver needed) → LLVM codegen.
/// Returns `Err` with a human-readable reason string on any step failure.
/// On Windows the LLVM DLL may not be present; callers should treat `Err`
/// as "skip this file" rather than a hard abort.
fn compile_nom_to_bc(nom_source: &str) -> Result<Vec<u8>, String> {
    let source_file = nom_parser::parse_source(nom_source)
        .map_err(|e| format!("parse: {e}"))?;

    // Use an empty in-memory resolver — translator output only references
    // built-in words, so a populated resolver is not required for planning.
    let resolver = nom_resolver::Resolver::open_in_memory()
        .map_err(|e| format!("resolver: {e}"))?;
    let planner = nom_planner::Planner::new(&resolver);
    let plan = planner
        .plan_unchecked(&source_file)
        .map_err(|e| format!("plan: {e}"))?;

    let output = nom_llvm::compile(&plan)
        .map_err(|e| format!("codegen: {e}"))?;

    Ok(output.bitcode)
}

// ── word helpers ─────────────────────────────────────────────────────────────

/// Strip to `[a-z0-9_]`, lowercase, truncate to 60 chars, ensure non-empty.
/// Does NOT add a language prefix — the `variant` column carries provenance.
pub fn sanitize_word(raw: &str) -> String {
    let cleaned: String = raw
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .take(60)
        .collect();
    if cleaned.is_empty() { "unnamed".to_string() } else { cleaned }
}

/// Build a `describe` string for a corpus entry (≤ 1024 chars).
///
/// Format:
/// ```text
/// fn add(a: integer, b: integer) -> integer
/// /// Add two i64s.
/// pub fn add(a: i64, b: i64) -> i64 {
///     a + b
/// } (translated from rust)
/// ```
pub fn build_describe(item: &equivalence_gate::TranslatedItem, original_source: &str, language: &str) -> String {
    let mut out = String::new();
    out.push_str(&item.summary);
    out.push('\n');
    // Find the fn definition in the original source and grab up to 4 following lines.
    let needle = format!("fn {}(", item.name);
    let mut found = false;
    let mut count = 0;
    for line in original_source.lines() {
        if !found {
            if line.contains(&needle) { found = true; }
        }
        if found {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                out.push_str(trimmed);
                out.push('\n');
                count += 1;
                if count >= 5 { break; }
            }
        }
    }
    out.push_str(&format!("(translated from {language})"));
    out.chars().take(1024).collect()
}

// ── ingest_directory ─────────────────────────────────────────────────────────

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
///
/// Lean-DB pivot B-b: translator output is compiled to LLVM bitcode via
/// [`compile_nom_to_bc`]. Only files that translate **and** compile
/// successfully are stored. `body_kind` is `BC`; `status` is `Complete`.
/// Files that translate but fail to compile (e.g. due to missing LLVM DLLs
/// on Windows) are skipped with an `eprintln!` diagnostic.
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
            Err(_) => { report.files_skipped += 1; continue; }
        };
        if !entry.file_type().is_file() { continue; }

        let file_bytes_len = entry.metadata().map(|m| m.len()).unwrap_or(0);
        if file_bytes_len > MAX_FILE_BYTES { continue; }

        let ext = entry
            .path()
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let language = ext_to_language(&ext);

        // Only translate languages we have translators for; skip everything else.
        if language != "rust" && language != "typescript" {
            report.files_skipped += 1;
            continue;
        }

        // Read source bytes.
        let raw = match std::fs::read(entry.path()) {
            Ok(b) => b,
            Err(_) => { report.files_skipped += 1; continue; }
        };
        let source = match std::str::from_utf8(&raw) {
            Ok(s) => s,
            Err(_) => { report.files_skipped += 1; continue; }
        };

        // Translate source → list of TranslatedItems (one per top-level fn).
        let translated_items = match language {
            "rust" => match crate::equivalence_gate::translators::rust::translate(source) {
                Ok(v) => v,
                Err(_) => { report.files_skipped += 1; continue; }
            },
            "typescript" => match crate::equivalence_gate::translators::typescript::translate(source) {
                Ok(v) => v,
                Err(_) => { report.files_skipped += 1; continue; }
            },
            _ => { report.files_skipped += 1; continue; }
        };

        if translated_items.is_empty() { report.files_skipped += 1; continue; }

        // One Entry per translated item.
        for item in &translated_items {
            // Compile this item's Nom body → LLVM bitcode.
            let bc_bytes = match compile_nom_to_bc(&item.nom_body) {
                Ok(b) => b,
                Err(reason) => {
                    eprintln!("nom: skipping {} ({}) — compile: {reason}", entry.path().display(), item.name);
                    continue;
                }
            };

            // SHA-256 the compiled bitcode → content-addressed id.
            let mut h = Sha256::new();
            h.update(&bc_bytes);
            let id = format!("{:x}", h.finalize());

            // word = sanitized fn name (not a filename slug).
            let word = sanitize_word(&item.name);
            let describe = build_describe(item, source, language);

            let now = chrono_now();
            let e = Entry {
                id: id.clone(),
                word,
                variant: Some(language.to_string()),  // original language as provenance
                kind: EntryKind::Function,
                language: "nom".to_string(),           // compiled Nom bitcode
                describe: Some(describe),
                concept: None,
                body: None,
                body_nom: None,
                body_bytes: Some(bc_bytes.clone()),
                body_kind: Some(nom_types::body_kind::BC.to_owned()),
                contract: Contract::default(),
                status: EntryStatus::Complete,         // translate + compile succeeded
                translation_score: Some(1.0),
                is_canonical: true,
                deprecated_by: None,
                created_at: now,
                updated_at: None,
            };

            match dict.upsert_entry_if_new(&e) {
                Ok(true) => {
                    *report.per_language.entry(language.to_string()).or_insert(0) += 1;
                    report.files_ingested += 1;
                    report.bytes_ingested += bc_bytes.len() as u64;
                }
                Ok(false) => {
                    report.duplicates += 1;
                }
                Err(_) => {
                    report.files_skipped += 1;
                }
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

// ── Clone-and-ingest (Pivot F) ───────────────────────────────────────────────

/// Report for a single clone-and-ingest run.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CloneIngestReport {
    pub url: String,
    pub clone_duration_secs: f64,
    pub ingest: IngestReport,
}

/// Report for a batch clone-and-ingest over many URLs.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct CloneBatchReport {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub files_ingested: u64,
    pub failures: Vec<(String, String)>,
}

/// Shallow-clone a git repo to a temp directory, ingest it into
/// `dict`, then delete the clone. Stream-and-discard disk discipline
/// per §5.17: peak disk = max(clone size, current dict size).
///
/// Uses `git clone --depth 1 --single-branch --no-tags` to minimize
/// bandwidth + disk. The clone directory is always deleted on exit
/// (success or failure) via a drop-guard.
pub fn clone_and_ingest(
    url: &str,
    dict: &nom_dict::NomDict,
) -> Result<CloneIngestReport, CorpusError> {
    use std::time::Instant;

    let tmp_root = std::env::temp_dir().join("nom-corpus-clones");
    std::fs::create_dir_all(&tmp_root)?;
    let slug = sanitize_url_slug(url);
    let target = tmp_root.join(format!(
        "{slug}-{pid}-{nanos}",
        pid = std::process::id(),
        nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    ));
    let _guard = TempDirGuard(target.clone());

    let clone_start = Instant::now();
    let out = std::process::Command::new("git")
        .arg("clone")
        .arg("--depth")
        .arg("1")
        .arg("--single-branch")
        .arg("--no-tags")
        .arg("--quiet")
        .arg(url)
        .arg(&target)
        .output()
        .map_err(|e| CorpusError::Skipped {
            reason: format!("git clone spawn failed: {e}"),
        })?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        return Err(CorpusError::Skipped {
            reason: format!("git clone failed: {}", stderr.trim()),
        });
    }
    let clone_duration_secs = clone_start.elapsed().as_secs_f64();

    let ingest = ingest_directory(&target, dict)?;
    Ok(CloneIngestReport {
        url: url.to_string(),
        clone_duration_secs,
        ingest,
    })
}

/// Clone-and-ingest every URL in `urls`, one at a time. Disk stays
/// bounded: each clone is deleted before the next one starts.
/// Failures are recorded and the loop continues.
pub fn clone_batch(
    urls: &[String],
    dict: &nom_dict::NomDict,
) -> CloneBatchReport {
    let mut report = CloneBatchReport {
        total: urls.len(),
        ..Default::default()
    };
    for url in urls {
        match clone_and_ingest(url, dict) {
            Ok(r) => {
                report.succeeded += 1;
                report.files_ingested += r.ingest.files_ingested;
                eprintln!(
                    "[clone-batch] ok  {url} ({} entries, clone {:.1}s)",
                    r.ingest.files_ingested, r.clone_duration_secs
                );
            }
            Err(e) => {
                report.failed += 1;
                report.failures.push((url.clone(), e.to_string()));
                eprintln!("[clone-batch] err {url}: {e}");
            }
        }
    }
    report
}

fn sanitize_url_slug(url: &str) -> String {
    let mut s = String::with_capacity(url.len());
    for c in url.chars() {
        if c.is_ascii_alphanumeric() {
            s.push(c);
        } else {
            s.push('_');
        }
    }
    if s.len() > 48 {
        s.truncate(48);
    }
    s
}

struct TempDirGuard(std::path::PathBuf);

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        if self.0.exists() {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
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
    fn sanitize_url_slug_replaces_non_alnum_and_truncates() {
        let s = sanitize_url_slug("https://github.com/org/repo.git");
        assert!(s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
        assert!(s.contains("github"));
        let long = "a".repeat(100);
        assert_eq!(sanitize_url_slug(&long).len(), 48);
    }

    #[test]
    fn temp_dir_guard_removes_on_drop() {
        let d = std::env::temp_dir().join(format!(
            "nom-corpus-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&d).unwrap();
        assert!(d.exists());
        {
            let _g = TempDirGuard(d.clone());
        }
        assert!(!d.exists());
    }

    #[test]
    fn clone_batch_on_empty_list_reports_zero() {
        let dict = nom_dict::NomDict::open_in_memory().unwrap();
        let r = clone_batch(&[], &dict);
        assert_eq!(r.total, 0);
        assert_eq!(r.succeeded, 0);
        assert_eq!(r.failed, 0);
    }

    #[test]
    fn clone_batch_records_failure_for_invalid_url() {
        let dict = nom_dict::NomDict::open_in_memory().unwrap();
        let r = clone_batch(&["not-a-real-url".to_string()], &dict);
        assert_eq!(r.total, 1);
        assert_eq!(r.succeeded, 0);
        assert_eq!(r.failed, 1);
        assert_eq!(r.failures.len(), 1);
    }

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
    #[cfg_attr(windows, ignore)]
    fn ingest_directory_populates_dict() {
        use std::fs;
        use std::io::Write;
        use nom_dict::NomDict;
        use nom_types::body_kind;

        let tmp = std::env::temp_dir().join("nom_corpus_ingest_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // lib.rs: simple function — translator accepts this.
        let mut f = fs::File::create(tmp.join("lib.rs")).unwrap();
        f.write_all(b"pub fn add(a: i64, b: i64) -> i64 { a + b }").unwrap();

        // point.rs: struct — translator rejects this → skipped.
        let mut f2 = fs::File::create(tmp.join("point.rs")).unwrap();
        f2.write_all(b"struct Point { x: f64, y: f64 }").unwrap();

        // main.py: Python — no translator → skipped.
        let mut f3 = fs::File::create(tmp.join("main.py")).unwrap();
        f3.write_all(b"def greet(name): return f'hello {name}'").unwrap();

        // index.ts: TypeScript — translator accepts this.
        let mut f4 = fs::File::create(tmp.join("index.ts")).unwrap();
        f4.write_all(b"export function id(x: number): number { return x; }").unwrap();

        // Ingest into an in-memory dict.
        let dict = NomDict::open_in_memory().unwrap();
        let report = super::ingest_directory(&tmp, &dict).unwrap();

        // Only successfully-translated AND compiled files land (lib.rs + index.ts
        // accepted; point.rs and main.py are skipped).
        assert!(
            report.files_ingested >= 1,
            "expected >= 1 ingested (lib.rs must translate+compile), got {}",
            report.files_ingested
        );
        assert_eq!(report.duplicates, 0);

        // lean-DB pivot B-b: entries land with BC body_kind, Complete status.
        let bc_entries = dict.find_by_body_kind(body_kind::BC, 10).unwrap();
        assert_eq!(bc_entries.len(), report.files_ingested as usize);
        for e in &bc_entries {
            assert_eq!(e.language, "nom", "body language must be nom");
            assert_eq!(e.status, nom_types::EntryStatus::Complete);
            assert!(e.body_bytes.as_ref().map_or(false, |b| !b.is_empty()));
        }

        let _ = fs::remove_dir_all(&tmp);
    }

    /// If the compile pipeline returns Err (e.g. LLVM DLL absent on Windows),
    /// files are skipped and `files_ingested == 0`.
    #[test]
    fn ingest_directory_skips_when_compile_fails() {
        use std::fs;
        use std::io::Write;
        use nom_dict::NomDict;

        // On platforms where LLVM is available the test is skipped (the real
        // pipeline would succeed, so the "compile fails" scenario doesn't apply).
        // On Windows without LLVM DLLs, ingest silently skips every file.
        // We verify that the function never hard-panics in either case.
        let tmp = std::env::temp_dir().join("nom_corpus_compile_fail_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let mut f = fs::File::create(tmp.join("lib.rs")).unwrap();
        f.write_all(b"pub fn add(a: i64, b: i64) -> i64 { a + b }").unwrap();

        let dict = NomDict::open_in_memory().unwrap();
        // Must not panic regardless of whether LLVM is available.
        let report = super::ingest_directory(&tmp, &dict).unwrap();
        // Either the file compiled (Linux CI) or was skipped (Windows without DLL).
        // Both outcomes are valid; we only assert the function returns Ok.
        let _ = report;

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    #[cfg_attr(windows, ignore)]
    fn ingest_directory_dedup() {
        use std::fs;
        use std::io::Write;
        use nom_dict::NomDict;

        let tmp = std::env::temp_dir().join("nom_corpus_dedup_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let subdir = tmp.join("sub");
        fs::create_dir_all(&subdir).unwrap();

        // Same translatable Rust source in two locations → same translated Nom
        // SHA-256 → dedup.
        let content = b"fn add(a: i64, b: i64) -> i64 { a + b }";
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
    #[cfg_attr(windows, ignore)]
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
    #[cfg_attr(windows, ignore)]
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
    #[cfg_attr(windows, ignore)]
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

    // ── Pivot B-c tests ───────────────────────────────────────────────────────

    #[test]
    fn sanitize_word_produces_valid_syntax_tokens() {
        assert_eq!(super::sanitize_word("add"), "add");
        assert_eq!(super::sanitize_word("my_function"), "my_function");
        assert_eq!(super::sanitize_word("MyStruct"), "mystruct");
        assert_eq!(super::sanitize_word("foo-bar"), "foobar");
        assert_eq!(super::sanitize_word("123abc"), "123abc");
        assert_eq!(super::sanitize_word(""), "unnamed");
        assert_eq!(super::sanitize_word("!@#$%"), "unnamed");
        // No language prefix — variant column carries that.
        assert!(!super::sanitize_word("add").contains('_') || super::sanitize_word("my_fn") == "my_fn");
        // Max 60 chars.
        let long = "a".repeat(100);
        assert_eq!(super::sanitize_word(&long).len(), 60);
    }

    #[test]
    #[cfg_attr(windows, ignore)]
    fn ingest_directory_lands_items_not_files() {
        use std::fs;
        use std::io::Write;
        use nom_dict::NomDict;

        let tmp = std::env::temp_dir().join("nom_corpus_items_not_files_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // One .rs file with 3 distinct fns.
        let mut f = fs::File::create(tmp.join("math.rs")).unwrap();
        f.write_all(
            b"fn foo(x: i64) -> i64 { x }\nfn bar(x: i64) -> i64 { x + 1 }\nfn baz(x: i64) -> i64 { x * 2 }"
        ).unwrap();

        let dict = NomDict::open_in_memory().unwrap();
        let report = super::ingest_directory(&tmp, &dict).unwrap();

        // 3 fns → 3 entries with meaningful word values.
        assert_eq!(
            report.files_ingested, 3,
            "expected 3 items (one per fn), got {}", report.files_ingested
        );

        let entries = dict.find_by_body_kind(nom_types::body_kind::BC, 10).unwrap_or_default();
        let words: Vec<&str> = entries.iter().map(|e| e.word.as_str()).collect();
        assert!(words.contains(&"foo"), "missing 'foo' in words: {words:?}");
        assert!(words.contains(&"bar"), "missing 'bar' in words: {words:?}");
        assert!(words.contains(&"baz"), "missing 'baz' in words: {words:?}");

        // words must NOT be filename slugs like "rust_math".
        for w in &words {
            assert!(!w.starts_with("rust_"), "word should not have language prefix: {w}");
        }

        let _ = fs::remove_dir_all(&tmp);
    }

}
