/// Severity level for a lint diagnostic produced by a rule pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LintSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

impl LintSeverity {
    /// Numeric level: Error=3, Warning=2, Info=1, Hint=0.
    pub fn level(&self) -> u8 {
        match self {
            LintSeverity::Error => 3,
            LintSeverity::Warning => 2,
            LintSeverity::Info => 1,
            LintSeverity::Hint => 0,
        }
    }

    /// True only for Error — diagnostics that block a build/commit.
    pub fn is_blocking(&self) -> bool {
        matches!(self, LintSeverity::Error)
    }
}

/// A single diagnostic emitted by a `LintRule`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintDiagnostic {
    pub rule_id: String,
    pub message: String,
    pub severity: LintSeverity,
    pub line: u32,
    pub col: u32,
    pub auto_fix: Option<String>,
}

impl LintDiagnostic {
    pub fn new(
        rule_id: impl Into<String>,
        message: impl Into<String>,
        severity: LintSeverity,
        line: u32,
        col: u32,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            message: message.into(),
            severity,
            line,
            col,
            auto_fix: None,
        }
    }

    /// Attach an auto-fix suggestion to this diagnostic.
    pub fn with_fix(mut self, fix: impl Into<String>) -> Self {
        self.auto_fix = Some(fix.into());
        self
    }

    /// Returns true when an auto-fix is available.
    pub fn has_fix(&self) -> bool {
        self.auto_fix.is_some()
    }
}

/// A data-driven lint rule that checks a source string for a text pattern.
pub struct LintRule {
    pub id: String,
    pub description: String,
    pub default_severity: LintSeverity,
}

impl LintRule {
    pub fn new(
        id: impl Into<String>,
        description: impl Into<String>,
        severity: LintSeverity,
    ) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            default_severity: severity,
        }
    }

    /// Find every occurrence of `pattern` in `source` and return a diagnostic
    /// for each match at (line=1, col=byte_index).
    pub fn check_pattern(&self, source: &str, pattern: &str) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        let mut search = source;
        let mut offset = 0usize;

        while let Some(pos) = search.find(pattern) {
            let abs_pos = offset + pos;
            diagnostics.push(
                LintDiagnostic::new(
                    self.id.clone(),
                    format!("pattern '{}' found", pattern),
                    self.default_severity.clone(),
                    1,
                    abs_pos as u32,
                ),
            );
            let advance = pos + pattern.len().max(1);
            offset += advance;
            search = &search[advance..];
        }
        diagnostics
    }
}

/// Runs a collection of `LintRule`s against source text.
pub struct LintPass {
    pub rules: Vec<LintRule>,
}

impl LintPass {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add_rule(&mut self, rule: LintRule) {
        self.rules.push(rule);
    }

    /// Run all rules against `source` and collect every diagnostic.
    pub fn run(&self, source: &str) -> Vec<LintDiagnostic> {
        self.rules
            .iter()
            .flat_map(|r| r.check_pattern(source, &r.id))
            .collect()
    }

    /// Count diagnostics whose severity is blocking (Error).
    pub fn blocking_count(diagnostics: &[LintDiagnostic]) -> usize {
        diagnostics.iter().filter(|d| d.severity.is_blocking()).count()
    }

    /// Return diagnostics that carry an auto-fix suggestion.
    pub fn auto_fixable(diagnostics: &[LintDiagnostic]) -> Vec<&LintDiagnostic> {
        diagnostics.iter().filter(|d| d.has_fix()).collect()
    }
}

impl Default for LintPass {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod lint_rules_tests {
    use super::*;

    // 1. LintSeverity::is_blocking()
    #[test]
    fn severity_is_blocking() {
        assert!(LintSeverity::Error.is_blocking());
        assert!(!LintSeverity::Warning.is_blocking());
        assert!(!LintSeverity::Info.is_blocking());
        assert!(!LintSeverity::Hint.is_blocking());
    }

    // 2. LintSeverity::level() ordering
    #[test]
    fn severity_level_ordering() {
        assert!(LintSeverity::Error.level() > LintSeverity::Warning.level());
        assert!(LintSeverity::Warning.level() > LintSeverity::Info.level());
        assert!(LintSeverity::Info.level() > LintSeverity::Hint.level());
        assert_eq!(LintSeverity::Error.level(), 3);
        assert_eq!(LintSeverity::Hint.level(), 0);
    }

    // 3. LintDiagnostic::with_fix() + has_fix()
    #[test]
    fn diagnostic_with_fix_and_has_fix() {
        let d = LintDiagnostic::new("r1", "msg", LintSeverity::Warning, 1, 0);
        assert!(!d.has_fix());
        let d = d.with_fix("suggested fix");
        assert!(d.has_fix());
        assert_eq!(d.auto_fix.as_deref(), Some("suggested fix"));
    }

    // 4. LintRule::check_pattern() finds matches
    #[test]
    fn check_pattern_finds_matches() {
        let rule = LintRule::new("todo", "Flags TODO comments", LintSeverity::Warning);
        let source = "let x = 1; // todo: fix this\nlet y = 2; // todo: later";
        let diags = rule.check_pattern(source, "todo");
        assert_eq!(diags.len(), 2);
    }

    // 5. LintRule::check_pattern() empty for no match
    #[test]
    fn check_pattern_no_match_is_empty() {
        let rule = LintRule::new("todo", "Flags TODO comments", LintSeverity::Warning);
        let source = "let x = 1; // all good here";
        let diags = rule.check_pattern(source, "todo");
        assert!(diags.is_empty());
    }

    // 6. LintPass::run() collects from all rules
    #[test]
    fn lint_pass_run_collects_all_rules() {
        let mut pass = LintPass::new();
        pass.add_rule(LintRule::new("foo", "desc", LintSeverity::Error));
        pass.add_rule(LintRule::new("bar", "desc", LintSeverity::Warning));
        // "foo" appears twice, "bar" appears once
        let source = "foo foo bar";
        let diags = pass.run(source);
        assert_eq!(diags.len(), 3);
    }

    // 7. blocking_count() counts only errors
    #[test]
    fn blocking_count_counts_only_errors() {
        let diags = vec![
            LintDiagnostic::new("r1", "m", LintSeverity::Error, 1, 0),
            LintDiagnostic::new("r2", "m", LintSeverity::Warning, 1, 0),
            LintDiagnostic::new("r3", "m", LintSeverity::Error, 2, 0),
            LintDiagnostic::new("r4", "m", LintSeverity::Info, 3, 0),
        ];
        assert_eq!(LintPass::blocking_count(&diags), 2);
    }

    // 8. auto_fixable() filters fixable diagnostics
    #[test]
    fn auto_fixable_filters_fixable() {
        let d1 = LintDiagnostic::new("r1", "m", LintSeverity::Error, 1, 0).with_fix("fix me");
        let d2 = LintDiagnostic::new("r2", "m", LintSeverity::Warning, 1, 0);
        let d3 = LintDiagnostic::new("r3", "m", LintSeverity::Info, 1, 0).with_fix("also fix");
        let diags = vec![d1, d2, d3];
        let fixable = LintPass::auto_fixable(&diags);
        assert_eq!(fixable.len(), 2);
    }

    // 9. LintDiagnostic::new() sets fields correctly
    #[test]
    fn diagnostic_new_sets_fields() {
        let d = LintDiagnostic::new("no-unused", "unused variable", LintSeverity::Error, 10, 5);
        assert_eq!(d.rule_id, "no-unused");
        assert_eq!(d.message, "unused variable");
        assert_eq!(d.severity, LintSeverity::Error);
        assert_eq!(d.line, 10);
        assert_eq!(d.col, 5);
        assert!(d.auto_fix.is_none());
    }
}
