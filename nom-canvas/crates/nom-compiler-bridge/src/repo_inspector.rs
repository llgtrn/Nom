/// GitHub repository analysis adapter — bridges a repo URL to the Nom compiler pipeline.

#[derive(Debug, Clone, PartialEq)]
pub enum RepoLanguage {
    Rust,
    Python,
    TypeScript,
    Go,
    Java,
    Other(String),
}

impl RepoLanguage {
    pub fn from_extension(ext: &str) -> Self {
        match ext {
            "rs" => RepoLanguage::Rust,
            "py" => RepoLanguage::Python,
            "ts" | "tsx" => RepoLanguage::TypeScript,
            "go" => RepoLanguage::Go,
            "java" => RepoLanguage::Java,
            other => RepoLanguage::Other(other.to_string()),
        }
    }

    pub fn label(&self) -> &str {
        match self {
            RepoLanguage::Rust => "rust",
            RepoLanguage::Python => "python",
            RepoLanguage::TypeScript => "typescript",
            RepoLanguage::Go => "go",
            RepoLanguage::Java => "java",
            RepoLanguage::Other(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RepoFile {
    pub path: String,
    pub language: RepoLanguage,
    pub line_count: u32,
}

#[derive(Debug, Clone)]
pub struct RepoProfile {
    pub url: String,
    pub primary_language: RepoLanguage,
    pub files: Vec<RepoFile>,
    pub has_tests: bool,
    pub has_ci: bool,
    pub pattern: String,
}

impl RepoProfile {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            primary_language: RepoLanguage::Other("unknown".to_string()),
            files: Vec::new(),
            has_tests: false,
            has_ci: false,
            pattern: "single_crate".to_string(),
        }
    }

    pub fn add_file(&mut self, file: RepoFile) {
        self.files.push(file);
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn to_nomx(&self) -> String {
        format!(
            "define repo that language({}) pattern({}) files({}) tests({})",
            self.primary_language.label(),
            self.pattern,
            self.file_count(),
            self.has_tests,
        )
    }
}

pub struct RepoInspector;

impl RepoInspector {
    /// Stub: analyze URL to produce a `RepoProfile` without an actual git clone.
    pub fn inspect_url(url: &str) -> RepoProfile {
        let lower = url.to_lowercase();

        let primary_language = if lower.contains("rust") || lower.ends_with(".git") && lower.contains("nom") {
            RepoLanguage::Rust
        } else if lower.contains("python") {
            RepoLanguage::Python
        } else {
            RepoLanguage::TypeScript
        };

        let pattern = if lower.contains("nom") || lower.contains("canvas") {
            "monorepo".to_string()
        } else {
            "single_crate".to_string()
        };

        let has_tests = !lower.contains("no-test");
        let has_ci = true;

        RepoProfile {
            url: url.to_string(),
            primary_language,
            files: Vec::new(),
            has_tests,
            has_ci,
            pattern,
        }
    }

    /// Detect layout pattern from a slice of files.
    pub fn detect_pattern(files: &[RepoFile]) -> String {
        use std::collections::HashMap;

        if files.is_empty() {
            return "single_lang".to_string();
        }

        let mut counts: HashMap<String, usize> = HashMap::new();
        for f in files {
            let ext = std::path::Path::new(&f.path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown")
                .to_string();
            *counts.entry(ext).or_insert(0) += 1;
        }

        if counts.len() > 1 {
            "polyglot".to_string()
        } else {
            "single_lang".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_language_from_ext() {
        assert_eq!(RepoLanguage::from_extension("rs"), RepoLanguage::Rust);
        assert_eq!(RepoLanguage::from_extension("py"), RepoLanguage::Python);
        assert_eq!(RepoLanguage::from_extension("ts"), RepoLanguage::TypeScript);
        assert_eq!(RepoLanguage::from_extension("tsx"), RepoLanguage::TypeScript);
        assert_eq!(RepoLanguage::from_extension("go"), RepoLanguage::Go);
        assert_eq!(RepoLanguage::from_extension("java"), RepoLanguage::Java);
        assert_eq!(
            RepoLanguage::from_extension("cpp"),
            RepoLanguage::Other("cpp".to_string())
        );
    }

    #[test]
    fn repo_language_label() {
        assert_eq!(RepoLanguage::Rust.label(), "rust");
        assert_eq!(RepoLanguage::Python.label(), "python");
        assert_eq!(RepoLanguage::TypeScript.label(), "typescript");
        assert_eq!(RepoLanguage::Go.label(), "go");
        assert_eq!(RepoLanguage::Java.label(), "java");
        assert_eq!(RepoLanguage::Other("zig".to_string()).label(), "zig");
    }

    #[test]
    fn repo_file_new() {
        let f = RepoFile {
            path: "src/main.rs".to_string(),
            language: RepoLanguage::Rust,
            line_count: 42,
        };
        assert_eq!(f.path, "src/main.rs");
        assert_eq!(f.language, RepoLanguage::Rust);
        assert_eq!(f.line_count, 42);
    }

    #[test]
    fn repo_profile_add_file() {
        let mut profile = RepoProfile::new("https://github.com/example/repo");
        assert_eq!(profile.file_count(), 0);
        profile.add_file(RepoFile {
            path: "src/lib.rs".to_string(),
            language: RepoLanguage::Rust,
            line_count: 100,
        });
        assert_eq!(profile.file_count(), 1);
        profile.add_file(RepoFile {
            path: "src/main.rs".to_string(),
            language: RepoLanguage::Rust,
            line_count: 50,
        });
        assert_eq!(profile.file_count(), 2);
    }

    #[test]
    fn repo_profile_to_nomx() {
        let mut profile = RepoProfile::new("https://github.com/example/nom-canvas");
        profile.primary_language = RepoLanguage::Rust;
        profile.pattern = "monorepo".to_string();
        profile.has_tests = true;
        profile.add_file(RepoFile {
            path: "src/lib.rs".to_string(),
            language: RepoLanguage::Rust,
            line_count: 80,
        });
        let nomx = profile.to_nomx();
        assert_eq!(
            nomx,
            "define repo that language(rust) pattern(monorepo) files(1) tests(true)"
        );
    }

    #[test]
    fn inspector_inspect_url_rust() {
        let profile = RepoInspector::inspect_url("https://github.com/some-org/rust-parser");
        assert_eq!(profile.primary_language, RepoLanguage::Rust);
        assert!(profile.has_ci);
    }

    #[test]
    fn inspector_inspect_url_python() {
        let profile = RepoInspector::inspect_url("https://github.com/some-org/python-tools");
        assert_eq!(profile.primary_language, RepoLanguage::Python);
        assert!(profile.has_ci);
    }

    #[test]
    fn inspect_url_has_tests() {
        let with_tests = RepoInspector::inspect_url("https://github.com/example/my-project");
        assert!(with_tests.has_tests);

        let without_tests = RepoInspector::inspect_url("https://github.com/example/no-test-repo");
        assert!(!without_tests.has_tests);
    }
}
