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
}
