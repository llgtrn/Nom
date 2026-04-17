#![deny(unsafe_code)]

// ---------------------------------------------------------------------------
// Sealed supertrait (yara-x pattern) — prevents external implementations.
// ---------------------------------------------------------------------------

mod private {
    pub trait Sealed {}
}

/// Severity level for a lint diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LintLevel {
    Error,
    Warning,
    Info,
}

/// A single lint finding produced by a rule.
///
/// `line` is the 1-based line number within the file.
/// `span` is the column range within that line (0-based, byte offsets).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintDiagnostic {
    pub level: LintLevel,
    pub message: String,
    pub line: u32,
    pub span: std::ops::Range<u32>,
}

/// A lint rule that inspects one line at a time and returns a diagnostic.
///
/// This trait is sealed — it cannot be implemented outside of `nom-lint`.
pub trait LintRule: private::Sealed {
    fn name(&self) -> &'static str;
    fn check(&self, line: &str, line_num: u32) -> Option<LintDiagnostic>;
}

// ---------------------------------------------------------------------------
// Concrete rules
// ---------------------------------------------------------------------------

/// Flags lines that end with one or more space or tab characters.
pub struct TrailingWhitespaceRule;

impl private::Sealed for TrailingWhitespaceRule {}

impl LintRule for TrailingWhitespaceRule {
    fn name(&self) -> &'static str {
        "trailing-whitespace"
    }

    fn check(&self, line: &str, line_num: u32) -> Option<LintDiagnostic> {
        let trimmed_len = line.trim_end_matches(|c| c == ' ' || c == '\t').len();
        if trimmed_len < line.len() {
            Some(LintDiagnostic {
                level: LintLevel::Warning,
                message: "trailing whitespace".to_string(),
                line: line_num,
                span: trimmed_len as u32..line.len() as u32,
            })
        } else {
            None
        }
    }
}

/// Flags lines whose length exceeds `max_len` characters.
pub struct LineTooLongRule {
    pub max_len: usize,
}

impl LineTooLongRule {
    /// Create a rule with the default maximum of 120 characters.
    pub fn new() -> Self {
        Self { max_len: 120 }
    }
}

impl Default for LineTooLongRule {
    fn default() -> Self {
        Self::new()
    }
}

impl private::Sealed for LineTooLongRule {}

impl LintRule for LineTooLongRule {
    fn name(&self) -> &'static str {
        "line-too-long"
    }

    fn check(&self, line: &str, line_num: u32) -> Option<LintDiagnostic> {
        let line_len = line.len();
        if line_len > self.max_len {
            Some(LintDiagnostic {
                level: LintLevel::Warning,
                message: format!(
                    "line is {} characters, exceeds maximum of {}",
                    line_len, self.max_len
                ),
                line: line_num,
                span: 0..line_len as u32,
            })
        } else {
            None
        }
    }
}

/// Flags occurrences of `{}` — braces with nothing between them.
pub struct EmptyBlockRule;

impl private::Sealed for EmptyBlockRule {}

impl LintRule for EmptyBlockRule {
    fn name(&self) -> &'static str {
        "empty-block"
    }

    fn check(&self, line: &str, line_num: u32) -> Option<LintDiagnostic> {
        if let Some(col) = line.find("{}") {
            Some(LintDiagnostic {
                level: LintLevel::Warning,
                message: "empty block `{}`".to_string(),
                line: line_num,
                span: col as u32..(col + 2) as u32,
            })
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Runner
// ---------------------------------------------------------------------------

/// Collects lint rules and runs them against source text.
pub struct LintRunner {
    rules: Vec<Box<dyn LintRule>>,
}

impl LintRunner {
    /// Create an empty runner with no rules attached.
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Register a rule with the runner.
    pub fn add_rule(&mut self, rule: impl LintRule + 'static) {
        self.rules.push(Box::new(rule));
    }

    /// Run all registered rules against a single `line` (1-based `line_num`).
    pub fn check_line(&self, line: &str, line_num: u32) -> Vec<LintDiagnostic> {
        self.rules
            .iter()
            .filter_map(|r| r.check(line, line_num))
            .collect()
    }

    /// Run all registered rules against every line of `source`.
    pub fn check_file(&self, source: &str) -> Vec<LintDiagnostic> {
        source
            .lines()
            .enumerate()
            .flat_map(|(i, line)| self.check_line(line, i as u32 + 1))
            .collect()
    }

    /// Run all registered rules against `source` and return the combined diagnostics.
    ///
    /// Equivalent to `check_file`.
    pub fn run(&self, source: &str) -> Vec<LintDiagnostic> {
        self.check_file(source)
    }
}

impl Default for LintRunner {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- TrailingWhitespaceRule ---

    #[test]
    fn trailing_whitespace_detected() {
        // "fn foo() {   " — trailing spaces start at column 10.
        let diag = TrailingWhitespaceRule
            .check("fn foo() {   ", 1)
            .expect("expected a diagnostic");
        assert_eq!(diag.level, LintLevel::Warning);
        assert!(diag.message.contains("trailing whitespace"));
        assert_eq!(diag.line, 1);
        assert_eq!(diag.span.start, 10);
        assert_eq!(diag.span.end, 13);
    }

    #[test]
    fn trailing_whitespace_clean_line_no_diag() {
        assert!(TrailingWhitespaceRule.check("fn foo() {}", 1).is_none());
    }

    // --- LineTooLongRule ---

    #[test]
    fn line_too_long_detected() {
        let long_line = "x".repeat(130);
        let rule = LineTooLongRule { max_len: 120 };
        let diag = rule.check(&long_line, 2).expect("expected a diagnostic");
        assert_eq!(diag.level, LintLevel::Warning);
        assert!(diag.message.contains("130"));
        assert!(diag.message.contains("120"));
        assert_eq!(diag.line, 2);
        assert_eq!(diag.span.start, 0);
        assert_eq!(diag.span.end, 130);
    }

    #[test]
    fn line_within_limit_no_diag() {
        let rule = LineTooLongRule { max_len: 120 };
        assert!(rule.check("short", 1).is_none());
    }

    // --- EmptyBlockRule ---

    #[test]
    fn empty_block_detected() {
        let diag = EmptyBlockRule
            .check("fn foo() {}", 3)
            .expect("expected a diagnostic");
        assert_eq!(diag.level, LintLevel::Warning);
        assert!(diag.message.contains("empty block"));
        assert_eq!(diag.line, 3);
        assert_eq!(diag.span.start, 9);
        assert_eq!(diag.span.end, 11);
    }

    #[test]
    fn non_empty_block_no_diag() {
        assert!(EmptyBlockRule.check("fn bar() { x }", 1).is_none());
    }

    // --- LintRunner::run (single-pass) ---

    #[test]
    fn lint_runner_combines_rules() {
        let long_line = "y".repeat(130);
        // line 1: trailing whitespace, line 2: too long, line 3: empty block
        let source = format!("let x = 1;   \n{}\nfn empty() {{}}", long_line);

        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule { max_len: 120 });
        runner.add_rule(EmptyBlockRule);

        let diags = runner.run(&source);
        assert!(diags.iter().any(|d| d.message.contains("trailing whitespace")));
        assert!(diags.iter().any(|d| d.message.contains("130")));
        assert!(diags.iter().any(|d| d.message.contains("empty block")));
    }

    // --- LintRunner::check_file (multi-line) ---

    #[test]
    fn check_file_assigns_correct_line_numbers() {
        let source = "ok line\ntrailing   \nalso ok";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);

        let diags = runner.check_file(source);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line, 2); // second line carries the trailing whitespace
    }

    #[test]
    fn check_file_multiple_issues_across_lines() {
        let long_line = "z".repeat(130);
        let source = format!("good\n{}\nbad trailing   \ngood again", long_line);

        let mut runner = LintRunner::new();
        runner.add_rule(LineTooLongRule { max_len: 120 });
        runner.add_rule(TrailingWhitespaceRule);

        let diags = runner.check_file(source.as_str());
        // Line 2 triggers LineTooLong, line 3 triggers TrailingWhitespace.
        assert!(diags.iter().any(|d| d.line == 2 && d.message.contains("130")));
        assert!(diags.iter().any(|d| d.line == 3 && d.message.contains("trailing")));
    }

    #[test]
    fn check_file_empty_source_no_diags() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule::new());
        runner.add_rule(EmptyBlockRule);
        assert!(runner.check_file("").is_empty());
    }

    // --- span field sanity ---

    #[test]
    fn span_start_lte_end() {
        let rules: Vec<Box<dyn LintRule>> = vec![
            Box::new(TrailingWhitespaceRule),
            Box::new(LineTooLongRule::new()),
            Box::new(EmptyBlockRule),
        ];
        let lines = [
            "fn foo() {   ",
            &"x".repeat(130),
            "fn bar() {}",
        ];
        for rule in &rules {
            for (i, line) in lines.iter().enumerate() {
                if let Some(diag) = rule.check(line, i as u32 + 1) {
                    assert!(
                        diag.span.start <= diag.span.end,
                        "rule {} produced inverted span on line {:?}",
                        rule.name(),
                        line
                    );
                    assert!(diag.span.end > 0, "span.end should be > 0 for a real finding");
                }
            }
        }
    }
}
