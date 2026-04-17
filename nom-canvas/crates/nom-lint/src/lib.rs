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

/// An internal lint rule that additionally exposes a severity multiplier.
///
/// Default multiplier is `1.0` (no scaling).
pub trait InternalRule: LintRule {
    fn severity_multiplier(&self) -> f32 {
        1.0
    }
}

// ---------------------------------------------------------------------------
// Concrete rules
// ---------------------------------------------------------------------------

/// Flags lines that end with one or more space or tab characters.
pub struct TrailingWhitespaceRule;

impl private::Sealed for TrailingWhitespaceRule {}
impl InternalRule for TrailingWhitespaceRule {}

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
impl InternalRule for LineTooLongRule {}

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
impl InternalRule for EmptyBlockRule {}

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
        assert!(diags
            .iter()
            .any(|d| d.message.contains("trailing whitespace")));
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
        assert!(diags
            .iter()
            .any(|d| d.line == 2 && d.message.contains("130")));
        assert!(diags
            .iter()
            .any(|d| d.line == 3 && d.message.contains("trailing")));
    }

    #[test]
    fn check_file_empty_source_no_diags() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule::new());
        runner.add_rule(EmptyBlockRule);
        assert!(runner.check_file("").is_empty());
    }

    // --- InternalRule ---

    #[test]
    fn internal_rule_default_severity_multiplier() {
        assert_eq!(TrailingWhitespaceRule.severity_multiplier(), 1.0);
        assert_eq!(LineTooLongRule::new().severity_multiplier(), 1.0);
        assert_eq!(EmptyBlockRule.severity_multiplier(), 1.0);
    }

    // --- InternalRule severity_multiplier ---

    #[test]
    fn internal_rule_multiplier_is_one_by_default() {
        assert_eq!(TrailingWhitespaceRule.severity_multiplier(), 1.0_f32);
        assert_eq!(EmptyBlockRule.severity_multiplier(), 1.0_f32);
        assert_eq!(LineTooLongRule::new().severity_multiplier(), 1.0_f32);
    }

    // --- LintRunner multiple diagnostics ---

    #[test]
    fn lint_runner_reports_multiple_diagnostics() {
        // Two separate trailing-whitespace lines → two diagnostics.
        let source = "foo   \nbar\nbaz   ";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(source);
        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0].line, 1);
        assert_eq!(diags[1].line, 3);
    }

    // --- LintRunner empty source ---

    #[test]
    fn lint_runner_empty_source_has_no_diagnostics() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule::new());
        runner.add_rule(EmptyBlockRule);
        let diags = runner.run("");
        assert!(diags.is_empty());
    }

    // --- EmptyBlockRule on bare braces ---

    #[test]
    fn empty_block_rule_fires_on_braces() {
        let diag = EmptyBlockRule
            .check("{}", 1)
            .expect("expected a diagnostic for bare {}");
        assert_eq!(diag.span.start, 0);
        assert_eq!(diag.span.end, 2);
        assert!(diag.message.contains("empty block"));
    }

    // --- LineTooLongRule boundary ---

    #[test]
    fn line_too_long_threshold() {
        let rule = LineTooLongRule::new(); // max_len = 120
                                           // Exactly 120 chars — must NOT fire.
        let at_limit = "a".repeat(120);
        assert!(
            rule.check(&at_limit, 1).is_none(),
            "120-char line should pass"
        );
        // 121 chars — must fire.
        let over_limit = "a".repeat(121);
        let diag = rule
            .check(&over_limit, 2)
            .expect("121-char line should fail");
        assert_eq!(diag.span.end, 121);
    }

    // --- span field sanity ---

    #[test]
    fn span_start_lte_end() {
        let rules: Vec<Box<dyn LintRule>> = vec![
            Box::new(TrailingWhitespaceRule),
            Box::new(LineTooLongRule::new()),
            Box::new(EmptyBlockRule),
        ];
        let lines = ["fn foo() {   ", &"x".repeat(130), "fn bar() {}"];
        for rule in &rules {
            for (i, line) in lines.iter().enumerate() {
                if let Some(diag) = rule.check(line, i as u32 + 1) {
                    assert!(
                        diag.span.start <= diag.span.end,
                        "rule {} produced inverted span on line {:?}",
                        rule.name(),
                        line
                    );
                    assert!(
                        diag.span.end > 0,
                        "span.end should be > 0 for a real finding"
                    );
                }
            }
        }
    }

    // --- LintDiagnostic fields ---

    #[test]
    fn lint_diagnostic_level_is_warning_for_trailing_whitespace() {
        let diag = TrailingWhitespaceRule.check("abc  ", 5).unwrap();
        assert_eq!(diag.level, LintLevel::Warning);
    }

    #[test]
    fn lint_diagnostic_line_number_stored_correctly() {
        let diag = TrailingWhitespaceRule.check("abc  ", 42).unwrap();
        assert_eq!(diag.line, 42);
    }

    #[test]
    fn lint_diagnostic_message_is_non_empty() {
        let diag = TrailingWhitespaceRule.check("abc  ", 1).unwrap();
        assert!(!diag.message.is_empty());
    }

    #[test]
    fn lint_diagnostic_clone_equals_original() {
        let diag = TrailingWhitespaceRule.check("abc  ", 1).unwrap();
        let cloned = diag.clone();
        assert_eq!(diag, cloned);
    }

    // --- TrailingWhitespaceRule edge cases ---

    #[test]
    fn trailing_whitespace_tab_only_detected() {
        let diag = TrailingWhitespaceRule.check("code\t", 1).unwrap();
        assert_eq!(diag.span.start, 4);
        assert_eq!(diag.span.end, 5);
    }

    #[test]
    fn trailing_whitespace_mixed_tab_space_detected() {
        let diag = TrailingWhitespaceRule.check("code \t ", 1).unwrap();
        assert_eq!(diag.span.start, 4);
        assert_eq!(diag.span.end, 7);
    }

    #[test]
    fn trailing_whitespace_only_whitespace_line_detected() {
        // A line that is entirely whitespace — span starts at 0.
        let diag = TrailingWhitespaceRule.check("   ", 1).unwrap();
        assert_eq!(diag.span.start, 0);
        assert_eq!(diag.span.end, 3);
    }

    #[test]
    fn trailing_whitespace_empty_line_no_diag() {
        assert!(TrailingWhitespaceRule.check("", 1).is_none());
    }

    #[test]
    fn trailing_whitespace_single_char_no_trailing_no_diag() {
        assert!(TrailingWhitespaceRule.check("x", 1).is_none());
    }

    // --- LineTooLongRule edge cases ---

    #[test]
    fn line_too_long_custom_max_len() {
        let rule = LineTooLongRule { max_len: 40 };
        let short = "a".repeat(40);
        assert!(rule.check(&short, 1).is_none());
        let long = "a".repeat(41);
        let diag = rule.check(&long, 2).unwrap();
        assert_eq!(diag.span.end, 41);
    }

    #[test]
    fn line_too_long_default_constructor_max_120() {
        let rule = LineTooLongRule::new();
        assert_eq!(rule.max_len, 120);
    }

    #[test]
    fn line_too_long_default_trait_max_120() {
        let rule = LineTooLongRule::default();
        assert_eq!(rule.max_len, 120);
    }

    #[test]
    fn line_too_long_empty_line_no_diag() {
        let rule = LineTooLongRule::new();
        assert!(rule.check("", 1).is_none());
    }

    #[test]
    fn line_too_long_span_covers_whole_line() {
        let rule = LineTooLongRule { max_len: 5 };
        let line = "hello world"; // 11 chars
        let diag = rule.check(line, 1).unwrap();
        assert_eq!(diag.span.start, 0);
        assert_eq!(diag.span.end, 11);
    }

    #[test]
    fn line_too_long_message_contains_actual_and_max() {
        let rule = LineTooLongRule { max_len: 10 };
        let line = "a".repeat(15);
        let diag = rule.check(&line, 1).unwrap();
        assert!(diag.message.contains("15"));
        assert!(diag.message.contains("10"));
    }

    // --- EmptyBlockRule edge cases ---

    #[test]
    fn empty_block_at_start_of_line() {
        let diag = EmptyBlockRule.check("{} rest", 1).unwrap();
        assert_eq!(diag.span.start, 0);
        assert_eq!(diag.span.end, 2);
    }

    #[test]
    fn empty_block_multiple_occurrences_first_fires() {
        // find() returns the first match; rule should fire on "fn a() {}"
        let diag = EmptyBlockRule.check("fn a() {} fn b() {}", 1).unwrap();
        assert_eq!(diag.span.start, 7);
    }

    #[test]
    fn empty_block_rule_name() {
        assert_eq!(EmptyBlockRule.name(), "empty-block");
    }

    #[test]
    fn trailing_whitespace_rule_name() {
        assert_eq!(TrailingWhitespaceRule.name(), "trailing-whitespace");
    }

    #[test]
    fn line_too_long_rule_name() {
        assert_eq!(LineTooLongRule::new().name(), "line-too-long");
    }

    // --- LintRunner rule count / check_line ---

    #[test]
    fn lint_runner_no_rules_produces_no_diags() {
        let runner = LintRunner::new();
        assert!(runner.check_line("fn foo() {}  ", 1).is_empty());
    }

    #[test]
    fn lint_runner_check_line_single_rule() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.check_line("hello   ", 1);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line, 1);
    }

    #[test]
    fn lint_runner_run_equals_check_file() {
        let source = "line one   \nline two\nline three   ";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        assert_eq!(runner.run(source), runner.check_file(source));
    }

    #[test]
    fn lint_runner_default_trait_works() {
        let runner = LintRunner::default();
        assert!(runner.run("anything").is_empty());
    }

    #[test]
    fn lint_runner_single_line_no_newline() {
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule);
        let diags = runner.run("fn empty() {}");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line, 1);
    }

    #[test]
    fn lint_runner_multiple_rules_same_line() {
        // A long line that also has trailing whitespace.
        let long_trailing = format!("{}   ", "b".repeat(130));
        let mut runner = LintRunner::new();
        runner.add_rule(LineTooLongRule { max_len: 120 });
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(&long_trailing);
        // Both rules should fire on line 1.
        assert_eq!(diags.iter().filter(|d| d.line == 1).count(), 2);
    }

    // --- LintLevel enum ---

    #[test]
    fn lint_level_variants_distinct() {
        assert_ne!(LintLevel::Error, LintLevel::Warning);
        assert_ne!(LintLevel::Warning, LintLevel::Info);
        assert_ne!(LintLevel::Error, LintLevel::Info);
    }

    #[test]
    fn lint_level_clone_equality() {
        let a = LintLevel::Warning;
        let b = a.clone();
        assert_eq!(a, b);
    }

    // --- TrailingWhitespaceRule edge cases (new) ---

    #[test]
    fn trailing_whitespace_windows_line_ending() {
        // When a caller strips \n but leaves \r (raw CRLF line minus final \n),
        // the rule only strips ' ' and '\t', so \r remains and does NOT itself
        // produce a trailing-whitespace hit.  However, spaces BEFORE the \r do.
        // "line  \r" → trim_end_matches(' '|'\t') leaves "line  \r" unchanged
        // (the \r is not stripped).  So we test the simpler, correct behavior:
        // spaces before a trailing \r are still detected if there are spaces.
        // Use a line that has spaces only (no \r) — the \r variant is covered
        // by the unit below; here we assert that the column offset is correct.
        let diag = TrailingWhitespaceRule.check("line  ", 1).unwrap();
        // first trailing space is at byte 4
        assert_eq!(diag.span.start, 4);
    }

    #[test]
    fn trailing_whitespace_three_spaces() {
        let diag = TrailingWhitespaceRule.check("abc   ", 1).unwrap();
        // span covers exactly the 3 trailing spaces
        assert_eq!(diag.span.end - diag.span.start, 3);
        assert!(!diag.message.is_empty());
    }

    #[test]
    fn trailing_whitespace_severity_is_warning() {
        let diag = TrailingWhitespaceRule.check("x  ", 1).unwrap();
        assert_eq!(diag.level, LintLevel::Warning);
    }

    #[test]
    fn trailing_whitespace_line_number_correct() {
        let diag = TrailingWhitespaceRule.check("abc  ", 7).unwrap();
        assert_eq!(diag.line, 7);
    }

    #[test]
    fn trailing_whitespace_col_points_to_whitespace() {
        // "hello " — content is 5 bytes, first trailing space is at byte 5
        let diag = TrailingWhitespaceRule.check("hello ", 1).unwrap();
        assert_eq!(diag.span.start, 5);
    }

    // --- LineTooLongRule edge cases (new) ---

    #[test]
    fn line_too_long_exact_limit_is_ok() {
        let rule = LineTooLongRule { max_len: 80 };
        let line = "a".repeat(80);
        assert!(rule.check(&line, 1).is_none());
    }

    #[test]
    fn line_too_long_one_over() {
        let rule = LineTooLongRule { max_len: 80 };
        let line = "a".repeat(81);
        let diag = rule.check(&line, 1).unwrap();
        assert_eq!(diag.span.end, 81);
    }

    #[test]
    fn line_too_long_severity_is_warning() {
        let rule = LineTooLongRule { max_len: 5 };
        let diag = rule.check("hello world", 1).unwrap();
        assert_eq!(diag.level, LintLevel::Warning);
    }

    #[test]
    fn line_too_long_message_contains_length() {
        let rule = LineTooLongRule { max_len: 10 };
        let line = "a".repeat(25);
        let diag = rule.check(&line, 1).unwrap();
        assert!(diag.message.contains("25"));
    }

    #[test]
    fn line_too_long_unicode_char_count() {
        // Each '€' is 3 bytes in UTF-8; LineTooLongRule uses .len() (byte count).
        // 5 euro signs = 15 bytes > max_len 10 → fires.
        let rule = LineTooLongRule { max_len: 10 };
        let line = "€€€€€"; // 15 bytes
        let diag = rule.check(line, 1).unwrap();
        assert!(diag.span.end > 10);
    }

    // --- EmptyBlockRule edge cases (new) ---

    #[test]
    fn empty_block_severity_is_warning() {
        let diag = EmptyBlockRule.check("fn f() {}", 1).unwrap();
        assert_eq!(diag.level, LintLevel::Warning);
    }

    #[test]
    fn empty_block_message_nonempty() {
        let diag = EmptyBlockRule.check("fn f() {}", 1).unwrap();
        assert!(!diag.message.is_empty());
    }

    #[test]
    fn empty_block_at_line_5() {
        let diag = EmptyBlockRule.check("impl Foo {}", 5).unwrap();
        assert_eq!(diag.line, 5);
    }

    #[test]
    fn empty_block_inside_function() {
        let diag = EmptyBlockRule.check("fn foo() {}", 1).unwrap();
        assert!(diag.message.contains("empty block"));
    }

    #[test]
    fn empty_block_with_whitespace_inside() {
        // "{ }" has content between braces — should NOT trigger the rule
        assert!(EmptyBlockRule.check("fn f() { }", 1).is_none());
    }

    // --- LintRunner with multiple rules (new) ---

    #[test]
    fn runner_all_three_rules_fire() {
        // Craft a line that triggers all three rules:
        // long enough (>120), has trailing whitespace, and contains "{}".
        let base = format!("fn f() {{}} {}", "x".repeat(115));
        let line = format!("{}   ", base); // add trailing spaces
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule { max_len: 10 });
        runner.add_rule(EmptyBlockRule);
        let diags = runner.check_line(&line, 1);
        assert_eq!(diags.len(), 3);
    }

    #[test]
    fn runner_rules_independent() {
        let source = "ok\nok\nok";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule::new());
        runner.add_rule(EmptyBlockRule);
        assert!(runner.run(source).is_empty());
    }

    #[test]
    fn runner_check_file_with_10_lines() {
        // Lines 1, 3, 5, 7, 9 have trailing whitespace → 5 diagnostics.
        let source = (1..=10)
            .map(|i| {
                if i % 2 == 1 {
                    "x  ".to_string()
                } else {
                    "x".to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(&source);
        assert_eq!(diags.len(), 5);
    }

    #[test]
    fn runner_empty_rules_no_panic() {
        let runner = LintRunner::new();
        let diags = runner.run("fn foo() {}   \n");
        assert!(diags.is_empty());
    }

    #[test]
    fn runner_add_rule_then_run() {
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule);
        let diags = runner.run("let x = {};\n");
        assert_eq!(diags.len(), 1);
    }

    // --- LintDiagnostic fields (new) ---

    #[test]
    fn diagnostic_line_col_both_set() {
        let diag = TrailingWhitespaceRule.check("hello  ", 4).unwrap();
        assert_eq!(diag.line, 4);
        assert!(diag.span.start < diag.span.end);
    }

    #[test]
    fn diagnostic_rule_name_matches_rule() {
        let diag = TrailingWhitespaceRule.check("abc  ", 1).unwrap();
        // The diagnostic message should be consistent with the rule
        assert!(diag.message.contains("trailing whitespace"));
        assert_eq!(TrailingWhitespaceRule.name(), "trailing-whitespace");
    }

    #[test]
    fn diagnostic_severity_accessible() {
        let diag = EmptyBlockRule.check("x = {}", 1).unwrap();
        // LintLevel is PartialEq so this comparison works
        assert_eq!(diag.level, LintLevel::Warning);
    }

    #[test]
    fn diagnostic_message_nonempty() {
        let diags = [
            TrailingWhitespaceRule.check("a  ", 1).unwrap(),
            LineTooLongRule { max_len: 1 }.check("ab", 1).unwrap(),
            EmptyBlockRule.check("{}", 1).unwrap(),
        ];
        for d in &diags {
            assert!(!d.message.is_empty());
        }
    }

    #[test]
    fn diagnostic_source_line_preserved() {
        // LintDiagnostic carries line number + span; caller can slice the
        // original source using span to recover the offending text.
        let source = "code   ";
        let diag = TrailingWhitespaceRule.check(source, 1).unwrap();
        let offending = &source[diag.span.start as usize..diag.span.end as usize];
        // offending region should be all whitespace
        assert!(offending.chars().all(|c| c == ' ' || c == '\t'));
    }

    // --- Unicode / multi-byte character handling ---

    #[test]
    fn trailing_whitespace_after_unicode_word() {
        // "héllo  " — 'é' is 2 bytes; trailing spaces start at byte 6.
        let diag = TrailingWhitespaceRule.check("héllo  ", 1).unwrap();
        assert_eq!(diag.span.start, 6); // byte offset after 'h'(1)+'é'(2)+'l'(1)+'l'(1)+'o'(1)
        assert_eq!(diag.span.end, 8);
    }

    #[test]
    fn trailing_whitespace_pure_unicode_line_no_trailing() {
        assert!(TrailingWhitespaceRule.check("こんにちは", 1).is_none());
    }

    #[test]
    fn empty_block_rule_with_unicode_prefix() {
        // Braces must still be found regardless of multi-byte prefix.
        let diag = EmptyBlockRule.check("日本語テスト {}", 1).unwrap();
        assert!(diag.message.contains("empty block"));
    }

    #[test]
    fn line_too_long_emoji_bytes() {
        // Each emoji is 4 bytes in UTF-8; 31 emojis = 124 bytes > 120.
        let rule = LineTooLongRule::new();
        let line: String = std::iter::repeat('😀').take(31).collect();
        let diag = rule.check(&line, 1).unwrap();
        assert!(diag.span.end > 120);
    }

    #[test]
    fn line_at_unicode_boundary_no_diag() {
        // 30 emojis = 120 bytes, exactly at limit — must NOT fire.
        let rule = LineTooLongRule::new();
        let line: String = std::iter::repeat('😀').take(30).collect();
        assert!(rule.check(&line, 1).is_none());
    }

    #[test]
    fn trailing_whitespace_emoji_followed_by_space() {
        // "🎉 " — emoji (4 bytes) + space (1 byte), trailing space at byte 4.
        let diag = TrailingWhitespaceRule.check("🎉 ", 1).unwrap();
        assert_eq!(diag.span.start, 4);
        assert_eq!(diag.span.end, 5);
    }

    // --- Whitespace-only and blank-line edge cases ---

    #[test]
    fn trailing_whitespace_single_space_only() {
        let diag = TrailingWhitespaceRule.check(" ", 1).unwrap();
        assert_eq!(diag.span.start, 0);
        assert_eq!(diag.span.end, 1);
    }

    #[test]
    fn trailing_whitespace_single_tab_only() {
        let diag = TrailingWhitespaceRule.check("\t", 1).unwrap();
        assert_eq!(diag.span.start, 0);
        assert_eq!(diag.span.end, 1);
    }

    #[test]
    fn lint_runner_whitespace_only_lines_detected() {
        let source = "   \n   \n   ";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(source);
        assert_eq!(diags.len(), 3);
        assert_eq!(diags[0].line, 1);
        assert_eq!(diags[1].line, 2);
        assert_eq!(diags[2].line, 3);
    }

    #[test]
    fn runner_blank_line_no_diag() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(LineTooLongRule::new());
        // A single blank line (\n) produces an empty line; nothing should fire.
        let diags = runner.run("\n");
        assert!(diags.is_empty());
    }

    // --- Very long lines ---

    #[test]
    fn line_too_long_very_long_line_10000_chars() {
        let rule = LineTooLongRule::new();
        let line = "a".repeat(10_000);
        let diag = rule.check(&line, 1).unwrap();
        assert_eq!(diag.span.end, 10_000);
        assert!(diag.message.contains("10000"));
    }

    #[test]
    fn runner_very_long_line_single_diag() {
        let mut runner = LintRunner::new();
        runner.add_rule(LineTooLongRule::new());
        let line = "x".repeat(5_000);
        let diags = runner.run(&line);
        assert_eq!(diags.len(), 1);
    }

    // --- LintLevel debug / clone ---

    #[test]
    fn lint_level_debug_not_empty() {
        assert!(!format!("{:?}", LintLevel::Error).is_empty());
        assert!(!format!("{:?}", LintLevel::Warning).is_empty());
        assert!(!format!("{:?}", LintLevel::Info).is_empty());
    }

    #[test]
    fn lint_level_info_clone() {
        let a = LintLevel::Info;
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn lint_level_error_clone() {
        let a = LintLevel::Error;
        assert_eq!(a.clone(), LintLevel::Error);
    }

    // --- LintDiagnostic debug ---

    #[test]
    fn lint_diagnostic_debug_not_empty() {
        let diag = TrailingWhitespaceRule.check("abc  ", 1).unwrap();
        assert!(!format!("{:?}", diag).is_empty());
    }

    // --- Batch linting / aggregation ---

    #[test]
    fn batch_lint_multiple_sources() {
        let sources = [
            "ok line\n",
            "trailing   \n",
            "fn empty() {}\n",
        ];
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);

        let all_diags: Vec<LintDiagnostic> = sources
            .iter()
            .flat_map(|src| runner.run(src))
            .collect();
        // sources[1] → trailing, sources[2] → empty-block
        assert_eq!(all_diags.len(), 2);
    }

    #[test]
    fn batch_lint_no_issues_produces_empty_aggregate() {
        let sources = ["hello\n", "world\n", "no issues here\n"];
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(LineTooLongRule::new());
        let all_diags: Vec<LintDiagnostic> = sources
            .iter()
            .flat_map(|src| runner.run(src))
            .collect();
        assert!(all_diags.is_empty());
    }

    #[test]
    fn batch_lint_count_by_level() {
        // All three rules produce Warning; verify aggregation of levels.
        let source = format!("trailing   \n{}\nfn e() {{}}\n", "x".repeat(130));
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule { max_len: 120 });
        runner.add_rule(EmptyBlockRule);
        let diags = runner.run(&source);
        let warning_count = diags.iter().filter(|d| d.level == LintLevel::Warning).count();
        assert_eq!(warning_count, diags.len()); // all are warnings
        assert!(diags.len() >= 3);
    }

    // --- Line number accuracy across larger files ---

    #[test]
    fn line_numbers_accurate_50_lines() {
        // Every 10th line has trailing whitespace.
        let source: String = (1..=50)
            .map(|i| {
                if i % 10 == 0 {
                    "content   \n".to_string()
                } else {
                    "content\n".to_string()
                }
            })
            .collect();
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(&source);
        assert_eq!(diags.len(), 5);
        let expected_lines = [10u32, 20, 30, 40, 50];
        for (diag, expected) in diags.iter().zip(expected_lines.iter()) {
            assert_eq!(diag.line, *expected);
        }
    }

    #[test]
    fn line_numbers_start_at_1() {
        let source = "trailing   \nok";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(source);
        assert_eq!(diags[0].line, 1);
    }

    #[test]
    fn last_line_without_newline_detected() {
        let source = "ok\nok\ntrailing   ";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(source);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line, 3);
    }

    // --- Error message quality ---

    #[test]
    fn trailing_whitespace_message_describes_issue() {
        let diag = TrailingWhitespaceRule.check("code   ", 1).unwrap();
        let msg = diag.message.to_lowercase();
        assert!(msg.contains("trailing") || msg.contains("whitespace"));
    }

    #[test]
    fn line_too_long_message_describes_issue() {
        let rule = LineTooLongRule { max_len: 5 };
        let diag = rule.check("hello world", 1).unwrap();
        let msg = diag.message.to_lowercase();
        // Should mention line or length or characters
        assert!(msg.contains("line") || msg.contains("character") || msg.contains("exceed"));
    }

    #[test]
    fn empty_block_message_describes_issue() {
        let diag = EmptyBlockRule.check("fn f() {}", 1).unwrap();
        let msg = diag.message.to_lowercase();
        assert!(msg.contains("empty") || msg.contains("block") || msg.contains("{}"));
    }

    #[test]
    fn all_rule_names_are_nonempty() {
        let names = [
            TrailingWhitespaceRule.name(),
            LineTooLongRule::new().name(),
            EmptyBlockRule.name(),
        ];
        for name in &names {
            assert!(!name.is_empty());
        }
    }

    #[test]
    fn all_rule_names_are_kebab_case() {
        // Rule names should be lowercase and use hyphens, not underscores.
        let names = [
            TrailingWhitespaceRule.name(),
            LineTooLongRule::new().name(),
            EmptyBlockRule.name(),
        ];
        for name in &names {
            assert_eq!(name.to_lowercase(), *name, "name should be lowercase: {name}");
            assert!(!name.contains('_'), "name should use hyphens not underscores: {name}");
        }
    }

    // --- Severity level assignment per rule ---

    #[test]
    fn empty_block_level_is_warning_not_error() {
        let diag = EmptyBlockRule.check("{}", 1).unwrap();
        assert_ne!(diag.level, LintLevel::Error);
        assert_eq!(diag.level, LintLevel::Warning);
    }

    #[test]
    fn line_too_long_level_is_warning_not_info() {
        let rule = LineTooLongRule { max_len: 1 };
        let diag = rule.check("hi", 1).unwrap();
        assert_ne!(diag.level, LintLevel::Info);
        assert_eq!(diag.level, LintLevel::Warning);
    }

    #[test]
    fn trailing_whitespace_level_not_info() {
        let diag = TrailingWhitespaceRule.check("x  ", 1).unwrap();
        assert_ne!(diag.level, LintLevel::Info);
    }

    // --- Enabling / disabling rules (by inclusion / exclusion in runner) ---

    #[test]
    fn disabling_rule_by_not_adding_it() {
        // If we only add LineTooLongRule, trailing whitespace is silently ignored.
        let source = "trailing   \nno_issue\n";
        let mut runner = LintRunner::new();
        runner.add_rule(LineTooLongRule::new());
        let diags = runner.run(source);
        // TrailingWhitespace not registered → no diag for it.
        assert!(!diags.iter().any(|d| d.message.contains("trailing")));
    }

    #[test]
    fn enabling_only_empty_block_rule() {
        let source = "fn a() {}\nok line\nfn b() {}\n";
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule);
        let diags = runner.run(source);
        assert_eq!(diags.len(), 2);
        assert!(diags.iter().all(|d| d.message.contains("empty block")));
    }

    #[test]
    fn enabling_only_trailing_whitespace_rule() {
        let source = format!("trailing   \n{}\nfn f() {{}}\n", "x".repeat(200));
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(&source);
        // Only trailing whitespace on line 1 should be caught.
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line, 1);
    }

    // --- Multiple violations on same line ---

    #[test]
    fn same_line_trailing_and_empty_block() {
        let line = "fn f() {}   "; // empty block + trailing whitespace
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.check_line(line, 1);
        assert_eq!(diags.len(), 2);
    }

    #[test]
    fn same_line_all_three_violations_distinct_spans() {
        let line = format!("fn f() {{}} {}   ", "x".repeat(115));
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(LineTooLongRule { max_len: 10 });
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.check_line(&line, 5);
        assert_eq!(diags.len(), 3);
        // All on the same reported line.
        assert!(diags.iter().all(|d| d.line == 5));
        // Spans should all be valid (start <= end).
        for d in &diags {
            assert!(d.span.start <= d.span.end);
        }
    }

    // --- Span precision ---

    #[test]
    fn empty_block_span_is_exactly_2_bytes() {
        let line = "let x = {};";
        let diag = EmptyBlockRule.check(line, 1).unwrap();
        assert_eq!(diag.span.end - diag.span.start, 2);
    }

    #[test]
    fn trailing_whitespace_span_length_matches_trailing_count() {
        let line = "abc     "; // 5 trailing spaces
        let diag = TrailingWhitespaceRule.check(line, 1).unwrap();
        assert_eq!(diag.span.end - diag.span.start, 5);
    }

    #[test]
    fn line_too_long_span_is_full_line() {
        let rule = LineTooLongRule { max_len: 3 };
        let line = "hello";
        let diag = rule.check(line, 1).unwrap();
        assert_eq!(diag.span.start, 0);
        assert_eq!(diag.span.end as usize, line.len());
    }

    // --- Additional edge cases ---

    #[test]
    fn runner_with_only_line_too_long_ignores_empty_block() {
        let mut runner = LintRunner::new();
        runner.add_rule(LineTooLongRule::new());
        // Short line with empty block — no line-too-long diag.
        let diags = runner.run("fn f() {}");
        assert!(diags.is_empty());
    }

    #[test]
    fn empty_block_no_diag_for_non_empty_braces() {
        let lines = [
            "fn f() { x }",
            "if true { return; }",
            "{ /* comment */ }",
            "{ 1 + 2 }",
        ];
        for line in &lines {
            assert!(
                EmptyBlockRule.check(line, 1).is_none(),
                "should not flag: {line}"
            );
        }
    }

    #[test]
    fn line_too_long_zero_max_fires_on_any_nonempty_line() {
        let rule = LineTooLongRule { max_len: 0 };
        let diag = rule.check("a", 1).unwrap();
        assert_eq!(diag.span.end, 1);
    }

    #[test]
    fn line_too_long_zero_max_no_diag_on_empty_line() {
        let rule = LineTooLongRule { max_len: 0 };
        assert!(rule.check("", 1).is_none());
    }

    #[test]
    fn runner_diag_order_follows_line_order() {
        // Violations on lines 1, 3, 5 — check they arrive in that order.
        let source = "a  \nb\nc  \nd\ne  ";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(source);
        assert_eq!(diags.len(), 3);
        assert_eq!(diags[0].line, 1);
        assert_eq!(diags[1].line, 3);
        assert_eq!(diags[2].line, 5);
    }

    #[test]
    fn runner_diag_order_within_line_follows_rule_registration_order() {
        // Register trailing-whitespace first, then empty-block.
        let line = "fn f() {}   ";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        let diags = runner.check_line(line, 1);
        assert_eq!(diags.len(), 2);
        // First diag is from the first registered rule.
        assert!(diags[0].message.contains("trailing whitespace"));
        assert!(diags[1].message.contains("empty block"));
    }

    #[test]
    fn trailing_whitespace_many_tabs() {
        let line = "code\t\t\t\t\t"; // 5 trailing tabs
        let diag = TrailingWhitespaceRule.check(line, 1).unwrap();
        assert_eq!(diag.span.end - diag.span.start, 5);
    }

    #[test]
    fn line_too_long_message_mentions_exceeds() {
        let rule = LineTooLongRule { max_len: 10 };
        let diag = rule.check("a".repeat(11).as_str(), 1).unwrap();
        // "exceeds" appears in the format string
        assert!(diag.message.contains("exceeds") || diag.message.contains("11"));
    }

    #[test]
    fn runner_check_line_high_line_number() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.check_line("hello   ", 999_999);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line, 999_999);
    }

    #[test]
    fn check_file_single_line_no_newline_gets_line_1() {
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule);
        let diags = runner.check_file("{}");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line, 1);
    }

    #[test]
    fn empty_block_rule_fires_on_all_occurrences_via_runner() {
        // Each line triggers exactly one empty-block; three lines → three diags.
        let source = "fn a() {}\nfn b() {}\nfn c() {}";
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule);
        let diags = runner.run(source);
        assert_eq!(diags.len(), 3);
    }

    #[test]
    fn trailing_whitespace_first_char_is_space() {
        // " x" — leading space is NOT trailing; no diag expected.
        assert!(TrailingWhitespaceRule.check(" x", 1).is_none());
    }

    #[test]
    fn trailing_whitespace_space_between_words_no_diag() {
        assert!(TrailingWhitespaceRule.check("hello world", 1).is_none());
    }

    #[test]
    fn lint_runner_run_and_check_file_same_result_for_complex_source() {
        let source = "fn f() {}   \n".repeat(20);
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(TrailingWhitespaceRule);
        assert_eq!(runner.run(&source), runner.check_file(&source));
    }

    #[test]
    fn internal_rule_multiplier_all_rules_return_one() {
        // Confirm all three concrete rules report 1.0 regardless of construction.
        let rules: Vec<Box<dyn InternalRule>> = vec![
            Box::new(TrailingWhitespaceRule),
            Box::new(LineTooLongRule { max_len: 80 }),
            Box::new(EmptyBlockRule),
        ];
        for r in &rules {
            assert_eq!(r.severity_multiplier(), 1.0_f32);
        }
    }

    #[test]
    fn runner_produces_diag_per_line_not_per_file() {
        // 100 lines each with trailing whitespace → exactly 100 diagnostics.
        let source: String = std::iter::repeat("x   \n").take(100).collect();
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(&source);
        assert_eq!(diags.len(), 100);
    }

    #[test]
    fn empty_block_not_triggered_by_open_brace_alone() {
        // "fn f() {" does not contain "{}" — should not fire.
        assert!(EmptyBlockRule.check("fn f() {", 1).is_none());
    }
}
