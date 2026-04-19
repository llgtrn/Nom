// NamingRule — lint checks for Nom naming conventions.

/// A violation produced by the naming linter.
#[derive(Debug, Clone)]
pub struct NamingViolation {
    pub name: String,
    pub reason: String,
}

/// Banned brand-name prefixes that must not appear in identifiers.
const BANNED_PREFIXES: &[&str] = &[
    "affine", "figma", "notion", "linear",
    "n8n", "sherlock", "haystack", "llamaindex", "llama_index",
    "refly", "comfy", "dify", "openai", "anthropic", "claude",
    "zed", "remotion", "tooljet", "rowboat", "bolt", "higgsfield",
    "ffmpeg", "yt_dlp", "surreal", "greptime", "datafusion",
    "temporal", "ollama", "vllm", "spider",
];

/// Stateless linter for Nom naming conventions.
pub struct NamingLinter;

impl NamingLinter {
    /// Returns a violation if `name` contains uppercase ASCII characters or hyphens.
    pub fn check_snake_case(name: &str) -> Option<NamingViolation> {
        let has_upper = name.chars().any(|c| c.is_ascii_uppercase());
        let has_hyphen = name.contains('-');
        if has_upper || has_hyphen {
            Some(NamingViolation {
                name: name.to_string(),
                reason: "name must be snake_case (no uppercase letters or hyphens)".to_string(),
            })
        } else {
            None
        }
    }

    /// Returns a violation if `name` starts with a banned foreign-brand prefix.
    pub fn check_no_foreign_brand(name: &str) -> Option<NamingViolation> {
        let lower = name.to_lowercase();
        for prefix in BANNED_PREFIXES {
            if lower.starts_with(prefix) {
                return Some(NamingViolation {
                    name: name.to_string(),
                    reason: format!("name starts with banned brand prefix '{prefix}'"),
                });
            }
        }
        None
    }

    /// Returns a violation if `name.len() > max`.
    pub fn check_length(name: &str, max: usize) -> Option<NamingViolation> {
        if name.len() > max {
            Some(NamingViolation {
                name: name.to_string(),
                reason: format!(
                    "name length {} exceeds maximum of {}",
                    name.len(),
                    max
                ),
            })
        } else {
            None
        }
    }

    /// Run all checks against `name` with a maximum length of 64 bytes.
    pub fn lint_all(name: &str) -> Vec<NamingViolation> {
        let mut violations = Vec::new();
        if let Some(v) = Self::check_snake_case(name) {
            violations.push(v);
        }
        if let Some(v) = Self::check_no_foreign_brand(name) {
            violations.push(v);
        }
        if let Some(v) = Self::check_length(name, 64) {
            violations.push(v);
        }
        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake_case_valid() {
        assert!(NamingLinter::check_snake_case("my_valid_name").is_none());
    }

    #[test]
    fn snake_case_violation_uppercase() {
        let v = NamingLinter::check_snake_case("MyName");
        assert!(v.is_some());
        assert_eq!(v.unwrap().name, "MyName");
    }

    #[test]
    fn no_foreign_brand_affine() {
        let v = NamingLinter::check_no_foreign_brand("affine_canvas");
        assert!(v.is_some());
        let v = v.unwrap();
        assert!(v.reason.contains("affine"));
    }

    #[test]
    fn no_foreign_brand_n8n() {
        let v = NamingLinter::check_no_foreign_brand("n8n_workflow");
        assert!(v.is_some());
        assert!(v.unwrap().reason.contains("n8n"));
    }

    #[test]
    fn no_foreign_brand_sherlock() {
        let v = NamingLinter::check_no_foreign_brand("sherlock_adapter");
        assert!(v.is_some());
        assert!(v.unwrap().reason.contains("sherlock"));
    }

    #[test]
    fn no_foreign_brand_haystack() {
        let v = NamingLinter::check_no_foreign_brand("haystack_component");
        assert!(v.is_some());
        assert!(v.unwrap().reason.contains("haystack"));
    }

    #[test]
    fn no_foreign_brand_ffmpeg() {
        let v = NamingLinter::check_no_foreign_brand("ffmpeg_config");
        assert!(v.is_some());
        assert!(v.unwrap().reason.contains("ffmpeg"));
    }

    #[test]
    fn no_foreign_brand_tooljet() {
        let v = NamingLinter::check_no_foreign_brand("tooljet_source");
        assert!(v.is_some());
        assert!(v.unwrap().reason.contains("tooljet"));
    }

    #[test]
    fn no_foreign_brand_valid() {
        assert!(NamingLinter::check_no_foreign_brand("canvas_node").is_none());
    }

    #[test]
    fn name_too_long() {
        let long_name = "a".repeat(65);
        let v = NamingLinter::check_length(&long_name, 64);
        assert!(v.is_some());
    }

    #[test]
    fn name_length_ok() {
        let exact = "a".repeat(64);
        assert!(NamingLinter::check_length(&exact, 64).is_none());
    }

    #[test]
    fn lint_all_clean() {
        let violations = NamingLinter::lint_all("clean_name");
        assert!(violations.is_empty());
    }

    #[test]
    fn lint_all_multiple_violations() {
        // "FigmaNode" — uppercase (snake_case) + banned brand "figma"
        let violations = NamingLinter::lint_all("FigmaNode");
        assert!(violations.len() >= 2, "expected at least 2 violations, got {}", violations.len());
    }
}
