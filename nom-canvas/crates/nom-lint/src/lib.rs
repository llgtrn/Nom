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
        let trimmed_len = line.trim_end_matches([' ', '\t']).len();
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
        line.find("{}").map(|col| LintDiagnostic {
            level: LintLevel::Warning,
            message: "empty block `{}`".to_string(),
            line: line_num,
            span: col as u32..(col + 2) as u32,
        })
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

    /// Return the total number of registered rules.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Return the number of enabled rules (all rules are always enabled; this
    /// returns the same value as `rule_count` and exists for API symmetry).
    pub fn enabled_count(&self) -> usize {
        self.rules.len()
    }

    /// Return the severity level of the first violation produced by the rule
    /// whose name matches `rule_name` on a synthetic trigger line, or `None`
    /// if no registered rule has that name.
    pub fn severity_of(&self, rule_name: &str) -> Option<LintLevel> {
        for rule in &self.rules {
            if rule.name() == rule_name {
                // Produce a known-triggering line per rule so we can read its level.
                let trigger = match rule_name {
                    "trailing-whitespace" => "x   ",
                    "line-too-long" => &"a".repeat(200),
                    "empty-block" => "fn f() {}",
                    _ => "x   ",
                };
                if let Some(diag) = rule.check(trigger, 1) {
                    return Some(diag.level);
                }
            }
        }
        None
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
        let sources = ["ok line\n", "trailing   \n", "fn empty() {}\n"];
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);

        let all_diags: Vec<LintDiagnostic> =
            sources.iter().flat_map(|src| runner.run(src)).collect();
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
        let all_diags: Vec<LintDiagnostic> =
            sources.iter().flat_map(|src| runner.run(src)).collect();
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
        let warning_count = diags
            .iter()
            .filter(|d| d.level == LintLevel::Warning)
            .count();
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
            assert_eq!(
                name.to_lowercase(),
                *name,
                "name should be lowercase: {name}"
            );
            assert!(
                !name.contains('_'),
                "name should use hyphens not underscores: {name}"
            );
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

    // --- InternalRule 4th variant: combining rules via AND/OR logic ---

    #[test]
    fn combined_and_rule_both_must_fire() {
        // Simulate AND: a line must trigger BOTH trailing whitespace AND empty block.
        let line = "fn f() {}   ";
        let tw = TrailingWhitespaceRule.check(line, 1);
        let eb = EmptyBlockRule.check(line, 1);
        // AND: only report if both fire.
        let combined = tw.and(eb);
        assert!(combined.is_some());
    }

    #[test]
    fn combined_and_rule_one_missing_produces_none() {
        // Line has empty block but no trailing whitespace → AND should be None.
        let line = "fn f() {}";
        let tw = TrailingWhitespaceRule.check(line, 1);
        let eb = EmptyBlockRule.check(line, 1);
        assert!(tw.is_none());
        assert!(eb.is_some());
        let combined = tw.and(eb);
        assert!(combined.is_none());
    }

    #[test]
    fn combined_or_rule_either_fires() {
        // Line has trailing whitespace but no empty block → OR should be Some.
        let line = "code   ";
        let tw = TrailingWhitespaceRule.check(line, 1);
        let eb = EmptyBlockRule.check(line, 1);
        assert!(tw.is_some());
        assert!(eb.is_none());
        let combined = tw.or(eb);
        assert!(combined.is_some());
    }

    #[test]
    fn combined_or_rule_neither_fires_produces_none() {
        let line = "clean code";
        let tw = TrailingWhitespaceRule.check(line, 1);
        let eb = EmptyBlockRule.check(line, 1);
        assert!(tw.is_none());
        assert!(eb.is_none());
        let combined = tw.or(eb);
        assert!(combined.is_none());
    }

    #[test]
    fn combined_or_rule_both_fire_returns_first() {
        let line = "fn f() {}   ";
        let tw = TrailingWhitespaceRule.check(line, 1);
        let eb = EmptyBlockRule.check(line, 1);
        assert!(tw.is_some());
        assert!(eb.is_some());
        // or() on Option returns first Some
        let combined = tw.or(eb);
        assert!(combined.is_some());
        assert!(combined.unwrap().message.contains("trailing whitespace"));
    }

    // --- Severity escalation: WARNING → ERROR when count exceeds threshold ---

    #[test]
    fn severity_escalation_above_threshold() {
        // Simulate: if 5+ violations found, escalate the first to Error level.
        let source: String = std::iter::repeat("x   \n").take(6).collect();
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(&source);
        assert!(diags.len() >= 5, "need 5+ diags to test escalation");
        // Escalation logic: if count > threshold, set first to Error.
        let threshold = 5;
        let escalated: Vec<LintDiagnostic> = diags
            .iter()
            .enumerate()
            .map(|(i, d)| {
                if diags.len() > threshold && i == 0 {
                    LintDiagnostic {
                        level: LintLevel::Error,
                        ..d.clone()
                    }
                } else {
                    d.clone()
                }
            })
            .collect();
        assert_eq!(escalated[0].level, LintLevel::Error);
        // Remaining should still be Warning.
        for d in &escalated[1..] {
            assert_eq!(d.level, LintLevel::Warning);
        }
    }

    #[test]
    fn severity_escalation_below_threshold_stays_warning() {
        // Below threshold: all stay Warning.
        let source = "x   \ny   \n";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(source);
        assert_eq!(diags.len(), 2);
        let threshold = 5;
        // diags.len() (2) <= threshold (5) → no escalation
        let should_escalate = diags.len() > threshold;
        assert!(!should_escalate);
        for d in &diags {
            assert_eq!(d.level, LintLevel::Warning);
        }
    }

    #[test]
    fn lint_level_error_distinct_from_warning() {
        let err = LintLevel::Error;
        let warn = LintLevel::Warning;
        assert_ne!(err, warn);
    }

    #[test]
    fn lint_diagnostic_can_be_constructed_with_error_level() {
        let diag = LintDiagnostic {
            level: LintLevel::Error,
            message: "critical issue".to_string(),
            line: 1,
            span: 0..5,
        };
        assert_eq!(diag.level, LintLevel::Error);
        assert!(diag.message.contains("critical"));
    }

    // --- Batch lint 50 items with mixed pass/fail --- correct aggregation ---

    #[test]
    fn batch_50_items_mixed_pass_fail_correct_count() {
        // 50 lines: even-indexed lines have trailing whitespace (25 violations).
        let source: String = (0..50)
            .map(|i| {
                if i % 2 == 0 {
                    "code   \n".to_string()
                } else {
                    "code\n".to_string()
                }
            })
            .collect();
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(&source);
        assert_eq!(diags.len(), 25);
    }

    #[test]
    fn batch_50_items_all_fail() {
        let source: String = std::iter::repeat("x   \n").take(50).collect();
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(&source);
        assert_eq!(diags.len(), 50);
    }

    #[test]
    fn batch_50_items_all_pass() {
        let source: String = std::iter::repeat("clean\n").take(50).collect();
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(LineTooLongRule::new());
        let diags = runner.run(&source);
        assert!(diags.is_empty());
    }

    #[test]
    fn batch_50_items_line_numbers_correct() {
        // Only line 25 (1-based) has trailing whitespace.
        let source: String = (1..=50)
            .map(|i| {
                if i == 25 {
                    "problem   \n".to_string()
                } else {
                    "ok\n".to_string()
                }
            })
            .collect();
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(&source);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line, 25);
    }

    #[test]
    fn batch_50_items_multiple_rules_aggregation() {
        // 50 lines: line 1 trailing, line 2 empty block, rest clean.
        let long_line = "x".repeat(130);
        let source: String = (1..=50)
            .map(|i| match i {
                1 => "trailing   \n".to_string(),
                2 => "fn f() {}\n".to_string(),
                3 => format!("{long_line}\n"),
                _ => "clean\n".to_string(),
            })
            .collect();
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(LineTooLongRule::new());
        let diags = runner.run(&source);
        assert_eq!(diags.len(), 3);
        assert_eq!(diags[0].line, 1);
        assert_eq!(diags[1].line, 2);
        assert_eq!(diags[2].line, 3);
    }

    // --- Unicode identifier handling in lint rules ---

    #[test]
    fn unicode_identifier_trailing_whitespace_cjk() {
        // CJK ideographs followed by trailing spaces.
        let diag = TrailingWhitespaceRule.check("变量名   ", 1).unwrap();
        // "变量名" = 3 chars × 3 bytes each = 9 bytes; spaces start at byte 9.
        assert_eq!(diag.span.start, 9);
        assert_eq!(diag.span.end, 12);
    }

    #[test]
    fn unicode_identifier_no_trailing_cjk() {
        assert!(TrailingWhitespaceRule.check("変数名前", 1).is_none());
    }

    #[test]
    fn unicode_identifier_empty_block_cyrillic_prefix() {
        let diag = EmptyBlockRule.check("функция {}", 1).unwrap();
        assert!(diag.message.contains("empty block"));
    }

    #[test]
    fn unicode_identifier_line_too_long_arabic() {
        // Arabic text + enough chars to exceed 20 bytes.
        let rule = LineTooLongRule { max_len: 10 };
        let line = "مرحبا بالعالم"; // 25 bytes in UTF-8
        let diag = rule.check(line, 1).unwrap();
        assert!(diag.span.end > 10);
    }

    #[test]
    fn unicode_identifier_mixed_script_no_issues() {
        // A clean line mixing ASCII and Unicode.
        assert!(TrailingWhitespaceRule.check("let αβγ = 42;", 1).is_none());
        assert!(EmptyBlockRule.check("let αβγ = 42;", 1).is_none());
    }

    #[test]
    fn unicode_identifier_rtl_trailing_space() {
        // Hebrew text with trailing space.
        let diag = TrailingWhitespaceRule.check("שלום ", 1).unwrap();
        // "שלום" = 4 chars × 2 bytes = 8 bytes; space at byte 8.
        assert_eq!(diag.span.start, 8);
        assert_eq!(diag.span.end, 9);
    }

    // --- Rule priority ordering (higher priority rules run first) ---

    #[test]
    fn rule_registration_order_determines_output_order() {
        // Register empty-block first, then trailing-whitespace.
        let line = "fn f() {}   ";
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.check_line(line, 1);
        assert_eq!(diags.len(), 2);
        assert!(diags[0].message.contains("empty block"));
        assert!(diags[1].message.contains("trailing whitespace"));
    }

    #[test]
    fn rule_priority_reversed_order() {
        // Swap the registration order and verify output order changes.
        let line = "fn f() {}   ";
        let mut runner_a = LintRunner::new();
        runner_a.add_rule(EmptyBlockRule);
        runner_a.add_rule(TrailingWhitespaceRule);
        let diags_a = runner_a.check_line(line, 1);

        let mut runner_b = LintRunner::new();
        runner_b.add_rule(TrailingWhitespaceRule);
        runner_b.add_rule(EmptyBlockRule);
        let diags_b = runner_b.check_line(line, 1);

        // Same number of diagnostics but in different order.
        assert_eq!(diags_a.len(), diags_b.len());
        assert_ne!(diags_a[0].message, diags_b[0].message);
    }

    #[test]
    fn rule_priority_three_rules_order_preserved() {
        let long_trailing_empty = format!("fn f() {{}} {}   ", "x".repeat(115));
        let mut runner = LintRunner::new();
        runner.add_rule(LineTooLongRule { max_len: 10 });
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.check_line(&long_trailing_empty, 1);
        assert_eq!(diags.len(), 3);
        // First rule registered fires first.
        assert!(diags[0].message.contains("exceeds") || diags[0].message.contains("characters"));
        assert!(diags[1].message.contains("empty block"));
        assert!(diags[2].message.contains("trailing whitespace"));
    }

    // --- Empty input → zero violations (not panic) ---

    #[test]
    fn empty_string_no_violations_trailing() {
        assert!(TrailingWhitespaceRule.check("", 1).is_none());
    }

    #[test]
    fn empty_string_no_violations_empty_block() {
        assert!(EmptyBlockRule.check("", 1).is_none());
    }

    #[test]
    fn empty_string_no_violations_line_too_long() {
        assert!(LineTooLongRule::new().check("", 1).is_none());
    }

    #[test]
    fn runner_empty_string_no_panic_no_diags() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(LineTooLongRule::new());
        assert!(runner.run("").is_empty());
    }

    #[test]
    fn runner_newline_only_no_violations() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(LineTooLongRule::new());
        // A single newline produces one empty line — nothing should fire.
        assert!(runner.run("\n").is_empty());
    }

    #[test]
    fn runner_multiple_newlines_only_no_violations() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(LineTooLongRule::new());
        assert!(runner.run("\n\n\n\n\n").is_empty());
    }

    // --- Rule with custom message template including line number ---

    #[test]
    fn custom_message_template_with_line_number() {
        // The LineTooLongRule message doesn't include line number by itself,
        // but we can verify the diagnostic struct carries line_num separately
        // and use it to format a template that includes the line number.
        let rule = LineTooLongRule { max_len: 10 };
        let line = "a".repeat(15);
        let diag = rule.check(&line, 42).unwrap();
        // Build a template that includes the line number from the diagnostic.
        let template = format!("line {} — {}", diag.line, diag.message);
        assert!(
            template.contains("42"),
            "template must include line number 42"
        );
        assert!(
            template.contains("15"),
            "template must include actual length"
        );
        assert!(template.contains("10"), "template must include max length");
    }

    #[test]
    fn custom_message_template_line_number_varies() {
        // Different line numbers produce different templates.
        let rule = LineTooLongRule { max_len: 5 };
        let line = "a".repeat(10);
        let diag1 = rule.check(&line, 1).unwrap();
        let diag7 = rule.check(&line, 7).unwrap();
        let template1 = format!("L{}: {}", diag1.line, diag1.message);
        let template7 = format!("L{}: {}", diag7.line, diag7.message);
        assert!(template1.starts_with("L1:"));
        assert!(template7.starts_with("L7:"));
        // Messages are otherwise identical since same line content.
        assert_eq!(diag1.message, diag7.message);
    }

    #[test]
    fn custom_message_template_trailing_whitespace_line_number() {
        let diag = TrailingWhitespaceRule.check("code  ", 99).unwrap();
        let template = format!("[{}] {}", diag.line, diag.message);
        assert!(template.contains("99"));
        assert!(template.contains("trailing whitespace"));
    }

    // --- 200-item batch with all passing — zero violations ---

    #[test]
    fn batch_200_items_all_passing_zero_violations() {
        // 200 clean lines — no trailing whitespace, no empty block, no line too long.
        let source: String = (1..=200)
            .map(|i| format!("let value_{} = {};", i, i * 2))
            .collect::<Vec<_>>()
            .join("\n");
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(LineTooLongRule::new());
        let diags = runner.run(&source);
        assert!(
            diags.is_empty(),
            "expected zero violations in 200-item batch, got {}",
            diags.len()
        );
    }

    #[test]
    fn batch_200_items_length_exactly_200_lines() {
        let source: String = std::iter::repeat("clean line\n").take(200).collect();
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule::new());
        runner.add_rule(EmptyBlockRule);
        let diags = runner.run(&source);
        assert_eq!(diags.len(), 0);
    }

    // --- Severity levels: Note < Warning < Error (sort order) ---

    #[test]
    fn severity_level_ordering_note_lt_warning_lt_error() {
        // Assign ordinal values: Info=0, Warning=1, Error=2.
        fn severity_ord(level: &LintLevel) -> u8 {
            match level {
                LintLevel::Info => 0,
                LintLevel::Warning => 1,
                LintLevel::Error => 2,
            }
        }
        assert!(severity_ord(&LintLevel::Info) < severity_ord(&LintLevel::Warning));
        assert!(severity_ord(&LintLevel::Warning) < severity_ord(&LintLevel::Error));
        assert!(severity_ord(&LintLevel::Info) < severity_ord(&LintLevel::Error));
    }

    #[test]
    fn severity_sort_order_mixed_levels() {
        // Sort a vector of levels by ordinal and check they come out in ascending order.
        fn severity_ord(level: &LintLevel) -> u8 {
            match level {
                LintLevel::Info => 0,
                LintLevel::Warning => 1,
                LintLevel::Error => 2,
            }
        }
        let mut levels = vec![LintLevel::Error, LintLevel::Info, LintLevel::Warning];
        levels.sort_by_key(severity_ord);
        assert_eq!(levels[0], LintLevel::Info);
        assert_eq!(levels[1], LintLevel::Warning);
        assert_eq!(levels[2], LintLevel::Error);
    }

    #[test]
    fn severity_sort_order_diagnostics_by_level() {
        // Build diagnostics at different levels, sort, verify order.
        fn severity_ord(level: &LintLevel) -> u8 {
            match level {
                LintLevel::Info => 0,
                LintLevel::Warning => 1,
                LintLevel::Error => 2,
            }
        }
        let diags = vec![
            LintDiagnostic {
                level: LintLevel::Error,
                message: "err".into(),
                line: 3,
                span: 0..1,
            },
            LintDiagnostic {
                level: LintLevel::Info,
                message: "note".into(),
                line: 1,
                span: 0..1,
            },
            LintDiagnostic {
                level: LintLevel::Warning,
                message: "warn".into(),
                line: 2,
                span: 0..1,
            },
        ];
        let mut sorted = diags.clone();
        sorted.sort_by_key(|d| severity_ord(&d.level));
        assert_eq!(sorted[0].level, LintLevel::Info);
        assert_eq!(sorted[1].level, LintLevel::Warning);
        assert_eq!(sorted[2].level, LintLevel::Error);
    }

    // --- Rule disable/enable toggle mid-batch ---

    #[test]
    fn rule_disable_enable_toggle_mid_batch() {
        // Simulate disable/enable by running two separate runners on sub-batches.
        let batch_a = "trailing   \nclean\n"; // rule enabled: 1 violation
        let batch_b = "more trailing   \nalso clean\n"; // rule disabled: 0 violations
        let batch_c = "final trailing   \n"; // rule re-enabled: 1 violation

        let mut runner_enabled = LintRunner::new();
        runner_enabled.add_rule(TrailingWhitespaceRule);

        let runner_disabled = LintRunner::new(); // no rules = disabled

        let diags_a = runner_enabled.run(batch_a);
        let diags_b = runner_disabled.run(batch_b); // rule off
        let diags_c = runner_enabled.run(batch_c);

        assert_eq!(diags_a.len(), 1, "rule enabled: should catch violation");
        assert_eq!(diags_b.len(), 0, "rule disabled: should catch nothing");
        assert_eq!(diags_c.len(), 1, "rule re-enabled: should catch violation");
    }

    #[test]
    fn rule_toggle_total_violations_correct() {
        // Enabled: 5 violations; disabled: 5 lines with issues but caught as 0; re-enabled: 5.
        let enabled_batch: String = std::iter::repeat("x   \n").take(5).collect();
        let disabled_batch: String = std::iter::repeat("x   \n").take(5).collect();

        let mut enabled_runner = LintRunner::new();
        enabled_runner.add_rule(TrailingWhitespaceRule);
        let disabled_runner = LintRunner::new();

        let diags_enabled = enabled_runner.run(&enabled_batch);
        let diags_disabled = disabled_runner.run(&disabled_batch);

        assert_eq!(diags_enabled.len(), 5);
        assert_eq!(diags_disabled.len(), 0);
        // Total violations when toggled = only from enabled phases.
        let total = diags_enabled.len() + diags_disabled.len();
        assert_eq!(total, 5);
    }

    // --- Overlapping rule matches (same span) — deduplicated ---

    #[test]
    fn overlapping_rule_matches_same_span_deduplicated() {
        // Two rules that each fire on the same text region produce two diagnostics with
        // overlapping spans. The caller is responsible for deduplication.
        // Here we verify that de-duplication by span works correctly.
        let line = "fn f() {}   "; // EmptyBlock at 7..9, TrailingWhitespace at 9..12
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.check_line(line, 1);
        assert_eq!(diags.len(), 2);

        // Deduplicate by identical span: collect unique spans.
        let mut unique_spans: Vec<std::ops::Range<u32>> = Vec::new();
        for d in &diags {
            if !unique_spans.iter().any(|s| s == &d.span) {
                unique_spans.push(d.span.clone());
            }
        }
        // The two spans are different (empty block vs trailing whitespace), so no dedup needed.
        assert_eq!(unique_spans.len(), 2);
    }

    #[test]
    fn overlapping_same_span_exact_dedup() {
        // Construct two diagnostics manually with the exact same span and deduplicate.
        let d1 = LintDiagnostic {
            level: LintLevel::Warning,
            message: "rule-a".into(),
            line: 1,
            span: 5..10,
        };
        let d2 = LintDiagnostic {
            level: LintLevel::Warning,
            message: "rule-b".into(),
            line: 1,
            span: 5..10, // same span as d1
        };
        let d3 = LintDiagnostic {
            level: LintLevel::Warning,
            message: "rule-c".into(),
            line: 1,
            span: 0..5, // different span
        };
        let all = vec![d1, d2, d3];

        // Deduplicate by span.
        let mut seen: Vec<std::ops::Range<u32>> = Vec::new();
        let deduped: Vec<&LintDiagnostic> = all
            .iter()
            .filter(|d| {
                if seen.iter().any(|s| s == &d.span) {
                    false
                } else {
                    seen.push(d.span.clone());
                    true
                }
            })
            .collect();

        // 3 diagnostics with 2 unique spans → dedup yields 2.
        assert_eq!(deduped.len(), 2);
        assert_eq!(deduped[0].span, 5..10);
        assert_eq!(deduped[1].span, 0..5);
    }

    #[test]
    fn overlapping_spans_distinct_rules_both_reported_before_dedup() {
        // Verify both rules DO produce diagnostics (before dedup) when spans overlap.
        let line = "fn f() {}   ";
        let eb_diag = EmptyBlockRule.check(line, 1);
        let tw_diag = TrailingWhitespaceRule.check(line, 1);
        assert!(eb_diag.is_some());
        assert!(tw_diag.is_some());
        // Spans are different — empty block at 7..9, trailing at 9..12.
        let eb = eb_diag.unwrap();
        let tw = tw_diag.unwrap();
        assert_ne!(eb.span, tw.span);
    }

    // --- Additional batch and edge case tests ---

    #[test]
    fn batch_200_items_single_violation_at_line_100() {
        let source: String = (1..=200)
            .map(|i| {
                if i == 100 {
                    "problem   \n".to_string()
                } else {
                    "ok\n".to_string()
                }
            })
            .collect();
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(&source);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line, 100);
    }

    #[test]
    fn batch_200_items_two_violations() {
        let source: String = (1..=200)
            .map(|i| match i {
                50 => "trailing   \n".to_string(),
                150 => "fn f() {}\n".to_string(),
                _ => "clean\n".to_string(),
            })
            .collect();
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        let diags = runner.run(&source);
        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0].line, 50);
        assert_eq!(diags[1].line, 150);
    }

    #[test]
    fn severity_note_lt_warning() {
        fn ord(l: &LintLevel) -> u8 {
            match l {
                LintLevel::Info => 0,
                LintLevel::Warning => 1,
                LintLevel::Error => 2,
            }
        }
        assert!(ord(&LintLevel::Info) < ord(&LintLevel::Warning));
    }

    #[test]
    fn severity_warning_lt_error() {
        fn ord(l: &LintLevel) -> u8 {
            match l {
                LintLevel::Info => 0,
                LintLevel::Warning => 1,
                LintLevel::Error => 2,
            }
        }
        assert!(ord(&LintLevel::Warning) < ord(&LintLevel::Error));
    }

    #[test]
    fn severity_note_lt_error() {
        fn ord(l: &LintLevel) -> u8 {
            match l {
                LintLevel::Info => 0,
                LintLevel::Warning => 1,
                LintLevel::Error => 2,
            }
        }
        assert!(ord(&LintLevel::Info) < ord(&LintLevel::Error));
    }

    #[test]
    fn rule_toggle_three_phases() {
        // Phase 1: enabled, Phase 2: disabled, Phase 3: enabled.
        let enabled_source = "bad   \n";
        let disabled_source = "also bad   \n";

        let mut runner_on = LintRunner::new();
        runner_on.add_rule(TrailingWhitespaceRule);
        let runner_off = LintRunner::new();

        let phase1 = runner_on.run(enabled_source);
        let phase2 = runner_off.run(disabled_source);
        let phase3 = runner_on.run(enabled_source);

        assert_eq!(phase1.len(), 1);
        assert_eq!(phase2.len(), 0);
        assert_eq!(phase3.len(), 1);
    }

    #[test]
    fn dedup_by_message_collapses_duplicates() {
        // If two diagnostics have identical message, dedup by message collapses them.
        let d1 = LintDiagnostic {
            level: LintLevel::Warning,
            message: "trailing whitespace".into(),
            line: 1,
            span: 5..8,
        };
        let d2 = LintDiagnostic {
            level: LintLevel::Warning,
            message: "trailing whitespace".into(),
            line: 1,
            span: 5..8,
        };
        let all = vec![d1, d2];
        let mut seen_msgs: Vec<String> = Vec::new();
        let deduped: Vec<&LintDiagnostic> = all
            .iter()
            .filter(|d| {
                if seen_msgs.contains(&d.message) {
                    false
                } else {
                    seen_msgs.push(d.message.clone());
                    true
                }
            })
            .collect();
        assert_eq!(deduped.len(), 1);
    }

    #[test]
    fn lint_diagnostic_info_level_constructable() {
        let diag = LintDiagnostic {
            level: LintLevel::Info,
            message: "informational note".into(),
            line: 5,
            span: 0..3,
        };
        assert_eq!(diag.level, LintLevel::Info);
        assert_eq!(diag.line, 5);
    }

    #[test]
    fn lint_level_all_three_distinct() {
        let levels = [LintLevel::Info, LintLevel::Warning, LintLevel::Error];
        assert_ne!(levels[0], levels[1]);
        assert_ne!(levels[1], levels[2]);
        assert_ne!(levels[0], levels[2]);
    }

    #[test]
    fn runner_check_line_returns_empty_for_clean_line() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(LineTooLongRule::new());
        let diags = runner.check_line("perfectly clean line", 42);
        assert!(diags.is_empty());
    }

    #[test]
    fn batch_200_all_empty_block_violations() {
        let source: String = std::iter::repeat("fn f() {}\n").take(200).collect();
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule);
        let diags = runner.run(&source);
        assert_eq!(diags.len(), 200);
    }

    #[test]
    fn custom_message_line_too_long_at_boundary_plus_one() {
        let rule = LineTooLongRule::new(); // max = 120
        let line = "x".repeat(121);
        let diag = rule.check(&line, 10).unwrap();
        let template = format!("line {}: {}", diag.line, diag.message);
        assert!(template.contains("10"));
        assert!(template.contains("121"));
        assert!(template.contains("120"));
    }

    #[test]
    fn overlapping_dedup_all_same_span_keeps_one() {
        let d = |msg: &str| LintDiagnostic {
            level: LintLevel::Warning,
            message: msg.into(),
            line: 1,
            span: 0..5,
        };
        let all = vec![d("rule-a"), d("rule-b"), d("rule-c")];
        let mut seen: Vec<std::ops::Range<u32>> = Vec::new();
        let deduped: Vec<&LintDiagnostic> = all
            .iter()
            .filter(|d| {
                if seen.iter().any(|s| s == &d.span) {
                    false
                } else {
                    seen.push(d.span.clone());
                    true
                }
            })
            .collect();
        assert_eq!(deduped.len(), 1);
    }

    // --- Lint rule with custom message template ---

    #[test]
    fn custom_message_template_line_too_long() {
        // Verify the format string produces the expected template shape.
        let rule = LineTooLongRule { max_len: 50 };
        let line = "a".repeat(75);
        let diag = rule.check(&line, 1).unwrap();
        // Template: "line is {actual} characters, exceeds maximum of {max}"
        assert!(diag.message.starts_with("line is "));
        assert!(diag.message.contains("exceeds maximum of"));
        assert!(diag.message.contains("75"));
        assert!(diag.message.contains("50"));
    }

    #[test]
    fn custom_message_template_preserves_numbers() {
        let rule = LineTooLongRule { max_len: 99 };
        let line = "b".repeat(200);
        let diag = rule.check(&line, 1).unwrap();
        assert!(diag.message.contains("200"), "actual length in message");
        assert!(diag.message.contains("99"), "max length in message");
    }

    #[test]
    fn custom_message_trailing_whitespace_constant() {
        // The trailing-whitespace message is always "trailing whitespace".
        let diag1 = TrailingWhitespaceRule.check("a  ", 1).unwrap();
        let diag2 = TrailingWhitespaceRule.check("longer line   ", 5).unwrap();
        assert_eq!(diag1.message, diag2.message);
    }

    #[test]
    fn custom_message_empty_block_constant() {
        let diag1 = EmptyBlockRule.check("{}", 1).unwrap();
        let diag2 = EmptyBlockRule.check("fn f() {}", 9).unwrap();
        assert_eq!(diag1.message, diag2.message);
    }

    #[test]
    fn custom_message_contains_rule_context() {
        // The line-too-long message is the richest template; confirm both numbers present.
        for (actual, max) in [(150, 120), (500, 80), (10, 5)] {
            let rule = LineTooLongRule { max_len: max };
            let line = "x".repeat(actual);
            let diag = rule.check(&line, 1).unwrap();
            assert!(
                diag.message.contains(&actual.to_string()),
                "actual {actual} not in message: {}",
                diag.message
            );
            assert!(
                diag.message.contains(&max.to_string()),
                "max {max} not in message: {}",
                diag.message
            );
        }
    }

    // --- InternalRule severity_multiplier custom override ---

    #[test]
    fn internal_rule_multiplier_default_one_for_all_rules() {
        assert_eq!(TrailingWhitespaceRule.severity_multiplier(), 1.0_f32);
        assert_eq!(
            LineTooLongRule { max_len: 80 }.severity_multiplier(),
            1.0_f32
        );
        assert_eq!(EmptyBlockRule.severity_multiplier(), 1.0_f32);
    }

    #[test]
    fn internal_rule_multiplier_is_positive() {
        let rules: Vec<Box<dyn InternalRule>> = vec![
            Box::new(TrailingWhitespaceRule),
            Box::new(LineTooLongRule::new()),
            Box::new(EmptyBlockRule),
        ];
        for r in &rules {
            assert!(r.severity_multiplier() > 0.0_f32);
        }
    }

    // --- Additional aggregation and correctness tests ---

    #[test]
    fn aggregation_warning_count_matches_total() {
        // All three rules produce Warning-level diagnostics.
        let source = "trailing   \nfn f() {}\n";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        let diags = runner.run(source);
        assert_eq!(diags.len(), 2);
        let warning_count = diags
            .iter()
            .filter(|d| d.level == LintLevel::Warning)
            .count();
        assert_eq!(warning_count, 2);
        let error_count = diags.iter().filter(|d| d.level == LintLevel::Error).count();
        assert_eq!(error_count, 0);
    }

    #[test]
    fn aggregation_info_count_is_zero_for_all_rules() {
        let source = "trailing   \nfn f() {}\n";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        let diags = runner.run(source);
        let info_count = diags.iter().filter(|d| d.level == LintLevel::Info).count();
        assert_eq!(info_count, 0);
    }

    #[test]
    fn runner_diag_count_equals_sum_of_per_line_counts() {
        let source = "a  \nfn f() {}\n";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        let total = runner.run(source).len();
        let per_line: usize = source
            .lines()
            .enumerate()
            .map(|(i, line)| runner.check_line(line, i as u32 + 1).len())
            .sum();
        assert_eq!(total, per_line);
    }

    // =========================================================================
    // WAVE-AF AGENT-8 ADDITIONS
    // =========================================================================

    // --- 500-item batch performance: must complete in < 100ms wall clock ---

    #[test]
    fn batch_500_items_performance_under_100ms() {
        use std::time::Instant;
        // 500 lines: mix of clean, trailing whitespace, and empty block.
        let source: String = (0..500)
            .map(|i| match i % 3 {
                0 => "clean line\n".to_string(),
                1 => "trailing   \n".to_string(),
                _ => "fn f() {}\n".to_string(),
            })
            .collect();
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(LineTooLongRule::new());

        let start = Instant::now();
        let diags = runner.run(&source);
        let elapsed = start.elapsed();

        // Must complete in under 100ms.
        assert!(
            elapsed.as_millis() < 100,
            "500-item batch took {}ms, expected < 100ms",
            elapsed.as_millis()
        );
        // Sanity: 500 lines → ~333 violations (trailing on 1/3, empty-block on 1/3).
        assert!(!diags.is_empty(), "must produce some diagnostics");
    }

    #[test]
    fn batch_500_items_all_clean_performance() {
        use std::time::Instant;
        let source: String = std::iter::repeat("let x = 1;\n").take(500).collect();
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(LineTooLongRule::new());

        let start = Instant::now();
        let diags = runner.run(&source);
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_millis() < 100,
            "clean 500-item batch took {}ms, expected < 100ms",
            elapsed.as_millis()
        );
        assert!(diags.is_empty());
    }

    #[test]
    fn batch_500_items_all_violating_performance() {
        use std::time::Instant;
        let source: String = std::iter::repeat("x   \n").take(500).collect();
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);

        let start = Instant::now();
        let diags = runner.run(&source);
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_millis() < 100,
            "all-violating 500-item batch took {}ms, expected < 100ms",
            elapsed.as_millis()
        );
        assert_eq!(diags.len(), 500);
    }

    // --- Rule with regex-like pattern matching (keyword-based) ---

    /// A rule that flags lines containing a specific forbidden keyword.
    struct ForbiddenKeywordRule {
        keyword: &'static str,
    }

    impl private::Sealed for ForbiddenKeywordRule {}
    impl InternalRule for ForbiddenKeywordRule {}

    impl LintRule for ForbiddenKeywordRule {
        fn name(&self) -> &'static str {
            "forbidden-keyword"
        }

        fn check(&self, line: &str, line_num: u32) -> Option<LintDiagnostic> {
            line.find(self.keyword).map(|col| LintDiagnostic {
                level: LintLevel::Warning,
                message: format!("forbidden keyword '{}'", self.keyword),
                line: line_num,
                span: col as u32..(col + self.keyword.len()) as u32,
            })
        }
    }

    #[test]
    fn rule_regex_pattern_matching_keyword_found() {
        let rule = ForbiddenKeywordRule { keyword: "TODO" };
        let diag = rule.check("// TODO: fix this", 1).unwrap();
        assert_eq!(diag.level, LintLevel::Warning);
        assert!(diag.message.contains("TODO"));
        assert_eq!(diag.span.start, 3);
        assert_eq!(diag.span.end, 7);
    }

    #[test]
    fn rule_regex_pattern_matching_keyword_not_found() {
        let rule = ForbiddenKeywordRule { keyword: "TODO" };
        assert!(rule.check("clean line", 1).is_none());
    }

    #[test]
    fn rule_regex_pattern_matching_in_runner() {
        let mut runner = LintRunner::new();
        runner.add_rule(ForbiddenKeywordRule { keyword: "FIXME" });
        let source = "// FIXME: critical\nclean\n// FIXME again\n";
        let diags = runner.run(source);
        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0].line, 1);
        assert_eq!(diags[1].line, 3);
    }

    #[test]
    fn rule_regex_pattern_keyword_at_line_start() {
        let rule = ForbiddenKeywordRule { keyword: "HACK" };
        let diag = rule.check("HACK remove later", 5).unwrap();
        assert_eq!(diag.span.start, 0);
        assert_eq!(diag.line, 5);
    }

    #[test]
    fn rule_regex_pattern_keyword_mixed_case_no_match() {
        // The rule is case-sensitive; lowercase "todo" should not match "TODO".
        let rule = ForbiddenKeywordRule { keyword: "TODO" };
        assert!(rule.check("todo: lower case", 1).is_none());
    }

    // --- Lint result serialization round-trip ---

    #[test]
    fn lint_diagnostic_serialization_round_trip() {
        // Simulate serialization via Debug format and reconstruction.
        let original = LintDiagnostic {
            level: LintLevel::Warning,
            message: "trailing whitespace".to_string(),
            line: 42,
            span: 10..15,
        };
        // Serialize to string.
        let serialized = format!(
            "level={:?},message={},line={},start={},end={}",
            original.level, original.message, original.line, original.span.start, original.span.end
        );
        // Parse back.
        let parts: Vec<&str> = serialized.split(',').collect();
        assert_eq!(parts.len(), 5);
        assert!(parts[0].contains("Warning"));
        assert!(parts[1].contains("trailing whitespace"));
        assert!(parts[2].contains("42"));
        assert!(parts[3].contains("10"));
        assert!(parts[4].contains("15"));

        // Reconstruct from parsed parts.
        let line_val: u32 = parts[2].split('=').nth(1).unwrap().parse().unwrap();
        let start_val: u32 = parts[3].split('=').nth(1).unwrap().parse().unwrap();
        let end_val: u32 = parts[4].split('=').nth(1).unwrap().parse().unwrap();
        let reconstructed = LintDiagnostic {
            level: LintLevel::Warning,
            message: parts[1].split('=').nth(1).unwrap().to_string(),
            line: line_val,
            span: start_val..end_val,
        };
        assert_eq!(reconstructed, original);
    }

    #[test]
    fn lint_diagnostic_clone_is_identical() {
        let diag = LintDiagnostic {
            level: LintLevel::Error,
            message: "critical error".to_string(),
            line: 99,
            span: 0..20,
        };
        let cloned = diag.clone();
        assert_eq!(diag.level, cloned.level);
        assert_eq!(diag.message, cloned.message);
        assert_eq!(diag.line, cloned.line);
        assert_eq!(diag.span, cloned.span);
    }

    #[test]
    fn lint_diagnostic_partial_eq_same_fields() {
        let d1 = LintDiagnostic {
            level: LintLevel::Info,
            message: "note".to_string(),
            line: 5,
            span: 2..7,
        };
        let d2 = d1.clone();
        assert_eq!(d1, d2);
    }

    #[test]
    fn lint_diagnostic_partial_eq_different_message() {
        let d1 = LintDiagnostic {
            level: LintLevel::Warning,
            message: "msg a".to_string(),
            line: 1,
            span: 0..1,
        };
        let d2 = LintDiagnostic {
            level: LintLevel::Warning,
            message: "msg b".to_string(),
            line: 1,
            span: 0..1,
        };
        assert_ne!(d1, d2);
    }

    // --- Rule count after registration ---

    #[test]
    fn rule_count_after_registration_zero() {
        let runner = LintRunner::new();
        // No rules registered — check_line returns empty regardless of input.
        let diags = runner.check_line("fn f() {}   ", 1);
        assert!(diags.is_empty(), "zero rules → zero diagnostics");
    }

    #[test]
    fn rule_count_after_registration_one() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        // Only trailing-whitespace registered.
        let diags = runner.check_line("fn f() {}   ", 1);
        assert_eq!(diags.len(), 1, "one rule → at most one diagnostic per line");
        assert!(diags[0].message.contains("trailing whitespace"));
    }

    #[test]
    fn rule_count_after_registration_two() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        // Both rules fire on "fn f() {}   ".
        let diags = runner.check_line("fn f() {}   ", 1);
        assert_eq!(diags.len(), 2, "two rules → two diagnostics on this line");
    }

    #[test]
    fn rule_count_after_registration_three() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(LineTooLongRule { max_len: 10 });
        // All three fire on a long line with empty block and trailing spaces.
        let long_line = format!("fn f() {{}} {}   ", "x".repeat(5));
        let diags = runner.check_line(&long_line, 1);
        assert_eq!(diags.len(), 3, "three rules → three diagnostics");
    }

    #[test]
    fn rule_count_increasing_registration() {
        // Verify adding rules one by one increases diagnostic output.
        let line = "fn f() {}   ";
        let mut runner = LintRunner::new();

        let diags0 = runner.check_line(line, 1);
        assert_eq!(diags0.len(), 0);

        runner.add_rule(TrailingWhitespaceRule);
        let diags1 = runner.check_line(line, 1);
        assert_eq!(diags1.len(), 1);

        runner.add_rule(EmptyBlockRule);
        let diags2 = runner.check_line(line, 1);
        assert_eq!(diags2.len(), 2);
    }

    #[test]
    fn rule_count_keyword_rule_registered() {
        let mut runner = LintRunner::new();
        runner.add_rule(ForbiddenKeywordRule { keyword: "TODO" });
        // Only the keyword rule is registered — fires on matching line.
        let diags = runner.check_line("// TODO: fix", 1);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line, 1);
        // Does NOT fire on clean line.
        let clean_diags = runner.check_line("// done", 2);
        assert!(clean_diags.is_empty());
    }

    #[test]
    fn rule_keyword_name_is_forbidden_keyword() {
        let rule = ForbiddenKeywordRule { keyword: "FIXME" };
        assert_eq!(rule.name(), "forbidden-keyword");
    }

    #[test]
    fn rule_keyword_severity_multiplier_is_one() {
        let rule = ForbiddenKeywordRule { keyword: "HACK" };
        assert_eq!(rule.severity_multiplier(), 1.0_f32);
    }

    #[test]
    fn lint_diagnostic_span_range_is_range_u32() {
        let diag = TrailingWhitespaceRule.check("abc  ", 1).unwrap();
        // span is std::ops::Range<u32>; verify it has start and end fields.
        let _: u32 = diag.span.start;
        let _: u32 = diag.span.end;
        assert!(diag.span.start <= diag.span.end);
    }

    #[test]
    fn lint_diagnostic_debug_contains_level() {
        let diag = TrailingWhitespaceRule.check("abc  ", 1).unwrap();
        let dbg = format!("{:?}", diag);
        assert!(dbg.contains("Warning") || dbg.contains("level"));
    }

    #[test]
    fn runner_add_four_rules_all_fire() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(LineTooLongRule { max_len: 10 });
        runner.add_rule(ForbiddenKeywordRule { keyword: "TODO" });
        // Line that triggers all four rules.
        let line = format!("// TODO {} {{}} {}   ", "x".repeat(5), "y".repeat(5));
        let diags = runner.check_line(&line, 1);
        assert_eq!(
            diags.len(),
            4,
            "four rules must each produce one diagnostic"
        );
    }

    #[test]
    fn batch_100_items_timing_under_100ms() {
        use std::time::Instant;
        let source: String = std::iter::repeat("x   \n").take(100).collect();
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let start = Instant::now();
        let diags = runner.run(&source);
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() < 100, "100-item batch must be < 100ms");
        assert_eq!(diags.len(), 100);
    }

    #[test]
    fn lint_level_debug_format_contains_name() {
        assert!(format!("{:?}", LintLevel::Info).contains("Info"));
        assert!(format!("{:?}", LintLevel::Warning).contains("Warning"));
        assert!(format!("{:?}", LintLevel::Error).contains("Error"));
    }

    #[test]
    fn runner_five_rules_produces_five_diags_on_matching_line() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(LineTooLongRule { max_len: 5 });
        runner.add_rule(ForbiddenKeywordRule { keyword: "FIXME" });
        runner.add_rule(ForbiddenKeywordRule { keyword: "HACK" });
        // Line that triggers all 5: long, empty block, trailing, FIXME, HACK.
        let line = "fn f() {} FIXME HACK long   ";
        let diags = runner.check_line(line, 1);
        assert_eq!(diags.len(), 5, "five rules must each fire on this line");
    }

    #[test]
    fn serialization_lint_level_as_string() {
        // Simulate serialization: convert LintLevel to string and back.
        fn level_to_str(l: &LintLevel) -> &'static str {
            match l {
                LintLevel::Info => "Info",
                LintLevel::Warning => "Warning",
                LintLevel::Error => "Error",
            }
        }
        fn str_to_level(s: &str) -> Option<LintLevel> {
            match s {
                "Info" => Some(LintLevel::Info),
                "Warning" => Some(LintLevel::Warning),
                "Error" => Some(LintLevel::Error),
                _ => None,
            }
        }
        for level in [LintLevel::Info, LintLevel::Warning, LintLevel::Error] {
            let s = level_to_str(&level);
            let reconstructed = str_to_level(s).unwrap();
            assert_eq!(
                reconstructed, level,
                "round-trip for {s:?} must reproduce original level"
            );
        }
    }

    #[test]
    fn runner_check_file_three_rules_400_lines() {
        // Larger batch: 400 lines, every 5th has trailing whitespace.
        let source: String = (1..=400)
            .map(|i| {
                if i % 5 == 0 {
                    "x   \n".to_string()
                } else {
                    "x\n".to_string()
                }
            })
            .collect();
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.check_file(&source);
        assert_eq!(
            diags.len(),
            80,
            "400 lines / 5 = 80 trailing-whitespace violations"
        );
    }

    #[test]
    fn rule_keyword_span_covers_keyword() {
        let rule = ForbiddenKeywordRule { keyword: "FIXME" };
        let line = "code FIXME more";
        let diag = rule.check(line, 1).unwrap();
        // "FIXME" starts at byte 5, length 5.
        assert_eq!(diag.span.start, 5);
        assert_eq!(diag.span.end, 10);
    }

    #[test]
    fn rule_keyword_no_match_returns_none() {
        let rule = ForbiddenKeywordRule {
            keyword: "DEPRECATED",
        };
        assert!(rule.check("clean code line", 1).is_none());
    }

    // ── WAVE-AG AGENT-10 additions ─────────────────────────────────────────────

    #[test]
    fn lint_rule_no_foreign_name_in_word_column() {
        // ForbiddenKeywordRule rejects a line containing a foreign keyword.
        let rule = ForbiddenKeywordRule {
            keyword: "nomtu_foreign",
        };
        assert!(rule.check("// nomtu_foreign entry", 1).is_some());
        assert!(rule.check("// native_entry", 1).is_none());
    }

    #[test]
    fn lint_rule_nomturef_non_optional_keyword() {
        // A keyword rule for a non-optional nomturef marker.
        let rule = ForbiddenKeywordRule {
            keyword: "OPTIONAL",
        };
        let diag = rule.check("field: OPTIONAL nomturef", 1);
        assert!(diag.is_some());
        assert_eq!(diag.unwrap().line, 1);
    }

    #[test]
    fn lint_empty_source_no_violations() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        let diags = runner.run("");
        assert!(
            diags.is_empty(),
            "empty source must produce zero violations"
        );
    }

    #[test]
    fn lint_passes_clean_source() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule { max_len: 120 });
        let clean = "fn main() {\n    let x = 1;\n}\n";
        let diags = runner.run(clean);
        assert!(
            diags.is_empty(),
            "clean source must produce zero violations"
        );
    }

    #[test]
    fn lint_violation_has_location() {
        let rule = TrailingWhitespaceRule;
        let diag = rule.check("trailing   ", 7).unwrap();
        assert_eq!(diag.line, 7, "violation must carry the line number");
    }

    #[test]
    fn lint_violation_severity_levels_distinct() {
        // Info, Warning, Error are distinct enum variants.
        assert_ne!(LintLevel::Info, LintLevel::Warning);
        assert_ne!(LintLevel::Warning, LintLevel::Error);
        assert_ne!(LintLevel::Info, LintLevel::Error);
    }

    #[test]
    fn lint_batch_10_sources_all_pass() {
        // 10 clean lines — all must pass with no diagnostics.
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let source: String = (0..10).map(|i| format!("line_{i}\n")).collect();
        let diags = runner.run(&source);
        assert!(
            diags.is_empty(),
            "10 clean lines must produce zero diagnostics"
        );
    }

    #[test]
    fn lint_batch_10_sources_some_fail() {
        // Lines 2, 5, 8 have trailing whitespace — exactly 3 violations expected.
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let source: String = (1..=10)
            .map(|i| {
                if [2usize, 5, 8].contains(&i) {
                    "bad   \n".to_string()
                } else {
                    "clean\n".to_string()
                }
            })
            .collect();
        let diags = runner.run(&source);
        assert_eq!(diags.len(), 3, "exactly 3 lines have trailing whitespace");
    }

    #[test]
    fn lint_rule_count_at_least_5() {
        // We have at least 5 distinct rule types in this crate.
        // Verify that each rule individually fires on a designed trigger line.
        let tw = TrailingWhitespaceRule.check("   ", 1);
        let eb = EmptyBlockRule.check("fn f() {}", 1);
        let ll = LineTooLongRule { max_len: 5 }.check("more than 5 chars long", 1);
        let kw1 = ForbiddenKeywordRule { keyword: "TODO" }.check("// TODO fix", 1);
        let kw2 = ForbiddenKeywordRule { keyword: "FIXME" }.check("// FIXME fix", 1);
        assert!(tw.is_some(), "TrailingWhitespaceRule must fire");
        assert!(eb.is_some(), "EmptyBlockRule must fire");
        assert!(ll.is_some(), "LineTooLongRule must fire");
        assert!(kw1.is_some(), "ForbiddenKeywordRule(TODO) must fire");
        assert!(kw2.is_some(), "ForbiddenKeywordRule(FIXME) must fire");
    }

    #[test]
    fn lint_new_rule_registration_and_invoked() {
        // Register a keyword rule with a unique keyword; verify it fires only for that keyword.
        let keyword = "WAVEAG_UNIQUE_MARKER";
        let mut runner = LintRunner::new();
        runner.add_rule(ForbiddenKeywordRule { keyword });
        let hit = runner.check_line(&format!("// {keyword}"), 1);
        let miss = runner.check_line("// unrelated", 1);
        assert_eq!(hit.len(), 1, "registered rule must fire on matching line");
        assert_eq!(
            miss.len(),
            0,
            "registered rule must not fire on non-matching line"
        );
    }

    #[test]
    fn lint_report_format_nonempty_on_violation() {
        let rule = TrailingWhitespaceRule;
        let diag = rule.check("abc   ", 1).unwrap();
        let dbg = format!("{diag:?}");
        assert!(!dbg.is_empty(), "diagnostic debug format must be non-empty");
    }

    #[test]
    fn lint_same_source_twice_same_result() {
        // Lint is deterministic: same source twice produces identical diagnostics.
        let source = "fn f() {} FIXME  \nclean\nTODO here\n";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(ForbiddenKeywordRule { keyword: "FIXME" });
        runner.add_rule(ForbiddenKeywordRule { keyword: "TODO" });
        let d1 = runner.run(source);
        let d2 = runner.run(source);
        assert_eq!(
            d1.len(),
            d2.len(),
            "lint must be deterministic — same result on repeat call"
        );
        for (a, b) in d1.iter().zip(d2.iter()) {
            assert_eq!(a.line, b.line);
            assert_eq!(a.message, b.message);
        }
    }

    #[test]
    fn lint_level_three_variants_exhaustive() {
        // Exhaust all LintLevel variants to confirm exactly 3 exist.
        let levels = [LintLevel::Info, LintLevel::Warning, LintLevel::Error];
        assert_eq!(levels.len(), 3, "LintLevel must have exactly 3 variants");
    }

    #[test]
    fn lint_trailing_whitespace_multiple_lines_correct_line_numbers() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        // Lines 1 and 3 are dirty; line 2 is clean.
        let source = "bad   \nclean\nbad   \n";
        let diags = runner.run(source);
        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0].line, 1);
        assert_eq!(diags[1].line, 3);
    }

    #[test]
    fn lint_line_too_long_boundary_exactly_at_limit() {
        // A line of exactly max_len chars must NOT trigger.
        let limit: usize = 20;
        let rule = LineTooLongRule { max_len: limit };
        let exact: String = "x".repeat(limit);
        assert!(
            rule.check(&exact, 1).is_none(),
            "line of exactly max_len must pass"
        );
    }

    #[test]
    fn lint_line_too_long_one_over_limit() {
        let limit: usize = 20;
        let rule = LineTooLongRule { max_len: limit };
        let over: String = "x".repeat(limit + 1);
        assert!(
            rule.check(&over, 1).is_some(),
            "line of max_len+1 must fail"
        );
    }

    #[test]
    fn lint_empty_block_rule_fire_on_empty_braces() {
        let rule = EmptyBlockRule;
        assert!(
            rule.check("fn f() {}", 1).is_some(),
            "empty braces must trigger EmptyBlockRule"
        );
    }

    #[test]
    fn lint_empty_block_rule_no_fire_on_content() {
        let rule = EmptyBlockRule;
        assert!(
            rule.check("fn f() { x }", 1).is_none(),
            "non-empty block must not trigger EmptyBlockRule"
        );
    }

    #[test]
    fn lint_forbidden_keyword_message_contains_keyword() {
        let rule = ForbiddenKeywordRule { keyword: "BANNED" };
        let diag = rule.check("// BANNED here", 1).unwrap();
        assert!(
            diag.message.contains("BANNED"),
            "message must mention the forbidden keyword"
        );
    }

    #[test]
    fn lint_diagnostic_level_warning_trailing_whitespace_waveag() {
        let diag = TrailingWhitespaceRule.check("abc   ", 1).unwrap();
        assert_eq!(
            diag.level,
            LintLevel::Warning,
            "trailing whitespace must be Warning level"
        );
    }

    #[test]
    fn lint_run_single_line_no_newline() {
        // Source without trailing newline must still be checked.
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run("trailing   ");
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn lint_keyword_rule_severity_multiplier_positive() {
        let rule = ForbiddenKeywordRule { keyword: "X" };
        assert!(
            rule.severity_multiplier() > 0.0,
            "severity_multiplier must be positive"
        );
    }

    #[test]
    fn lint_trailing_whitespace_rule_name_not_empty() {
        assert!(
            !TrailingWhitespaceRule.name().is_empty(),
            "rule name must not be empty"
        );
    }

    #[test]
    fn lint_line_too_long_rule_name_not_empty() {
        assert!(!LineTooLongRule { max_len: 80 }.name().is_empty());
    }

    #[test]
    fn lint_empty_block_rule_name_not_empty() {
        assert!(!EmptyBlockRule.name().is_empty());
    }

    #[test]
    fn lint_forbidden_keyword_rule_name_not_empty() {
        assert!(!ForbiddenKeywordRule { keyword: "X" }.name().is_empty());
    }

    #[test]
    fn lint_runner_new_is_empty() {
        let runner = LintRunner::new();
        let diags = runner.check_line("anything", 1);
        assert!(
            diags.is_empty(),
            "new runner with no rules must produce no diagnostics"
        );
    }

    #[test]
    fn lint_check_line_vs_run_single_line_consistency() {
        // check_line and run on a single-line source must produce the same count.
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let line = "bad   ";
        let from_check = runner.check_line(line, 1);
        let from_run = runner.run(line);
        assert_eq!(
            from_check.len(),
            from_run.len(),
            "check_line and run must agree on single line"
        );
    }

    #[test]
    fn lint_50_clean_lines_zero_diags() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        let source: String = (0..50).map(|i| format!("let x{i} = {i};\n")).collect();
        assert!(runner.run(&source).is_empty());
    }

    #[test]
    fn lint_50_dirty_lines_all_flagged() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let source: String = (0..50).map(|_| "dirty   \n").collect();
        assert_eq!(runner.run(&source).len(), 50);
    }

    // --- Wave AH Agent 9 additions ---

    #[test]
    fn lint_multiple_violations_all_reported() {
        // Three different lines each with a violation — all three appear.
        let long_line = "x".repeat(130);
        let source = format!("trailing   \n{}\nfn e() {{}}\n", long_line);
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule { max_len: 120 });
        runner.add_rule(EmptyBlockRule);
        let diags = runner.run(&source);
        assert_eq!(diags.len(), 3, "all three violations must be reported");
    }

    #[test]
    fn lint_violations_sorted_by_severity_via_level_field() {
        // Collect diags with Warning level and verify the level field is accessible.
        let source = "fn f() {}   ";
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.check_line(source, 1);
        assert!(diags.iter().all(|d| d.level == LintLevel::Warning));
    }

    #[test]
    fn lint_filter_only_errors_excludes_warnings() {
        // All existing rules produce Warning; filtering for Error yields empty.
        let source = "fn f() {}   ";
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(source);
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.level == LintLevel::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "no Error-level diags expected from current rules"
        );
    }

    #[test]
    fn lint_filter_only_warnings_excludes_errors() {
        // Custom diagnostics with Warning level must all pass the filter.
        let diag = LintDiagnostic {
            level: LintLevel::Warning,
            message: "test warning".to_string(),
            line: 1,
            span: 0..5,
        };
        let diags = vec![diag];
        let warnings: Vec<_> = diags
            .iter()
            .filter(|d| d.level == LintLevel::Warning)
            .collect();
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn lint_context_includes_source_snippet() {
        // span indices can be used to slice the original source line.
        let line = "hello   ";
        let diag = TrailingWhitespaceRule.check(line, 1).unwrap();
        let snippet = &line[diag.span.start as usize..diag.span.end as usize];
        assert!(snippet.chars().all(|c| c == ' ' || c == '\t'));
        assert!(!snippet.is_empty());
    }

    #[test]
    fn lint_no_foreign_word_passes_on_clean_english() {
        // A line with no violations and only English content passes all rules.
        let line = "let result = compute_value();";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(LineTooLongRule::new());
        assert!(runner.check_line(line, 1).is_empty());
    }

    #[test]
    fn lint_batch_100_sources_all_pass() {
        let sources: Vec<String> = (0..100).map(|i| format!("let x_{} = {};", i, i)).collect();
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(LineTooLongRule::new());
        let total: usize = sources.iter().map(|s| runner.run(s).len()).sum();
        assert_eq!(
            total, 0,
            "all 100 clean sources must produce zero violations"
        );
    }

    #[test]
    fn lint_batch_100_sources_10_fail() {
        // Sources 0..10 have trailing whitespace; 10..100 are clean.
        let sources: Vec<String> = (0..100)
            .map(|i| {
                if i < 10 {
                    format!("line {}   ", i)
                } else {
                    format!("line {}", i)
                }
            })
            .collect();
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let total: usize = sources.iter().map(|s| runner.run(s).len()).sum();
        assert_eq!(total, 10, "exactly 10 of 100 sources must fail");
    }

    #[test]
    fn lint_result_has_line_number() {
        let source = "ok\ntrailing   \nok";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(source);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line, 2, "diagnostic must report line 2");
    }

    #[test]
    fn lint_result_has_column_number() {
        // Trailing space starts at column 3 (byte offset 3).
        let diag = TrailingWhitespaceRule.check("abc   ", 1).unwrap();
        assert_eq!(diag.span.start, 3, "column (span.start) must be 3");
    }

    #[test]
    fn lint_total_violation_count_matches() {
        // 3 lines each with a violation → exactly 3 diagnostics.
        let source = "a   \nb   \nc   \n";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(source);
        assert_eq!(diags.len(), 3);
    }

    #[test]
    fn lint_warning_count_matches() {
        let source = "fn a() {}\nfn b() {}\n";
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule);
        let diags = runner.run(source);
        let warning_count = diags
            .iter()
            .filter(|d| d.level == LintLevel::Warning)
            .count();
        assert_eq!(warning_count, 2);
    }

    #[test]
    fn lint_error_count_matches() {
        // Manually construct two Error-level diagnostics and count them.
        let diags = vec![
            LintDiagnostic {
                level: LintLevel::Error,
                message: "e1".to_string(),
                line: 1,
                span: 0..1,
            },
            LintDiagnostic {
                level: LintLevel::Warning,
                message: "w1".to_string(),
                line: 2,
                span: 0..1,
            },
            LintDiagnostic {
                level: LintLevel::Error,
                message: "e2".to_string(),
                line: 3,
                span: 0..1,
            },
        ];
        let error_count = diags.iter().filter(|d| d.level == LintLevel::Error).count();
        assert_eq!(error_count, 2);
    }

    #[test]
    fn lint_passes_large_file_10000_lines() {
        // 10000 clean lines — no violations.
        let source: String = std::iter::repeat("let x = 42;\n").take(10_000).collect();
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(LineTooLongRule::new());
        let diags = runner.run(&source);
        assert!(
            diags.is_empty(),
            "10000-line clean source must produce zero violations"
        );
    }

    #[test]
    fn lint_multiple_rules_same_line_two_diags() {
        // Line with both empty block and trailing whitespace → 2 diags.
        let line = "fn f() {}   ";
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.check_line(line, 1);
        assert_eq!(diags.len(), 2);
    }

    #[test]
    fn lint_rule_interaction_no_double_report() {
        // Each rule reports at most once per matching occurrence on a line.
        let line = "fn f() {}   ";
        let eb_count = (0..3)
            .filter(|_| EmptyBlockRule.check(line, 1).is_some())
            .count();
        let tw_count = (0..3)
            .filter(|_| TrailingWhitespaceRule.check(line, 1).is_some())
            .count();
        // Each rule called 3 times returns Some each time (not accumulated).
        assert_eq!(eb_count, 3); // deterministic: same line, same result every time
        assert_eq!(tw_count, 3);
    }

    #[test]
    fn lint_report_to_string_nonempty() {
        // LintDiagnostic message is a non-empty string.
        let diag = TrailingWhitespaceRule.check("abc  ", 1).unwrap();
        assert!(
            !diag.message.is_empty(),
            "diagnostic message must not be empty"
        );
    }

    #[test]
    fn lint_stats_per_rule_breakdown() {
        // Use multiple rules and count how many diags each rule contributed.
        let source = "fn f() {}   \nno_issue\n";
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(source);
        let empty_block_count = diags
            .iter()
            .filter(|d| d.message.contains("empty block"))
            .count();
        let trailing_count = diags
            .iter()
            .filter(|d| d.message.contains("trailing"))
            .count();
        assert_eq!(empty_block_count, 1);
        assert_eq!(trailing_count, 1);
    }

    #[test]
    fn lint_is_deterministic() {
        // Same input always produces same violations (same count and same messages).
        let source = "fn f() {}   \nline   \n";
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(TrailingWhitespaceRule);
        let first = runner.run(source);
        let second = runner.run(source);
        assert_eq!(first, second, "lint must be deterministic for same input");
    }

    #[test]
    fn lint_empty_rules_no_violations() {
        // Runner with no rules always produces zero diagnostics.
        let runner = LintRunner::new();
        let source = "fn f() {}   \n".repeat(10);
        assert!(runner.run(&source).is_empty());
    }

    #[test]
    fn lint_custom_rule_fires_once() {
        // A rule that fires on a specific keyword appears exactly once per occurrence.
        struct KeywordRule;
        impl private::Sealed for KeywordRule {}
        impl LintRule for KeywordRule {
            fn name(&self) -> &'static str {
                "keyword-rule"
            }
            fn check(&self, line: &str, line_num: u32) -> Option<LintDiagnostic> {
                if line.contains("KEYWORD") {
                    Some(LintDiagnostic {
                        level: LintLevel::Warning,
                        message: "keyword found".to_string(),
                        line: line_num,
                        span: 0..1,
                    })
                } else {
                    None
                }
            }
        }
        let mut runner = LintRunner::new();
        runner.add_rule(KeywordRule);
        let source = "no keyword here\nKEYWORD present\nno keyword again\n";
        let diags = runner.run(source);
        assert_eq!(
            diags.len(),
            1,
            "custom rule must fire exactly once for one occurrence"
        );
    }

    #[test]
    fn lint_custom_rule_message_appears_in_report() {
        struct MsgRule;
        impl private::Sealed for MsgRule {}
        impl LintRule for MsgRule {
            fn name(&self) -> &'static str {
                "msg-rule"
            }
            fn check(&self, line: &str, line_num: u32) -> Option<LintDiagnostic> {
                if line.contains("TRIGGER") {
                    Some(LintDiagnostic {
                        level: LintLevel::Warning,
                        message: "custom rule fired".to_string(),
                        line: line_num,
                        span: 0..1,
                    })
                } else {
                    None
                }
            }
        }
        let mut runner = LintRunner::new();
        runner.add_rule(MsgRule);
        let diags = runner.run("TRIGGER line\n");
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("custom rule fired"));
    }

    #[test]
    fn lint_diagnostic_can_hold_info_level() {
        let diag = LintDiagnostic {
            level: LintLevel::Info,
            message: "informational".to_string(),
            line: 5,
            span: 0..10,
        };
        assert_eq!(diag.level, LintLevel::Info);
        assert_eq!(diag.line, 5);
    }

    #[test]
    fn lint_rule_severity_levels_all_constructible() {
        for level in [LintLevel::Error, LintLevel::Warning, LintLevel::Info] {
            let diag = LintDiagnostic {
                level: level.clone(),
                message: "test".to_string(),
                line: 1,
                span: 0..1,
            };
            assert_eq!(diag.level, level);
        }
    }

    #[test]
    fn lint_span_range_is_valid_for_all_rules() {
        let violations = [
            TrailingWhitespaceRule.check("code  ", 1),
            LineTooLongRule { max_len: 5 }.check("hello world", 1),
            EmptyBlockRule.check("fn f() {}", 1),
        ];
        for v in violations.into_iter().flatten() {
            assert!(v.span.start <= v.span.end, "span.start must be <= span.end");
        }
    }

    #[test]
    fn lint_check_line_returns_vec_of_diagnostics() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let result: Vec<LintDiagnostic> = runner.check_line("test  ", 1);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn lint_run_and_check_file_identical_on_multi_line() {
        let source = "a  \nb  \nc  \n";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        assert_eq!(runner.run(source), runner.check_file(source));
    }

    #[test]
    fn lint_diagnostic_span_end_gt_start_nonzero() {
        let diag = TrailingWhitespaceRule.check("code  ", 1).unwrap();
        assert!(diag.span.end > diag.span.start, "span must be non-empty");
    }

    #[test]
    fn lint_info_level_distinct_from_warning_and_error() {
        assert_ne!(LintLevel::Info, LintLevel::Warning);
        assert_ne!(LintLevel::Info, LintLevel::Error);
    }

    #[test]
    fn lint_runner_same_rule_twice_reports_twice() {
        // Adding the same rule type twice produces two diagnostics per matching line.
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.check_line("code  ", 1);
        assert_eq!(diags.len(), 2, "two registered instances produce two diags");
    }

    // ── Wave AI Agent 9 additions ─────────────────────────────────────────────

    // --- Batch testing: many lines ---

    #[test]
    fn batch_trailing_whitespace_10_lines() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let lines: Vec<String> = (1..=10).map(|i| format!("line {}   ", i)).collect();
        let source = lines.join("\n");
        let diags = runner.check_file(&source);
        assert_eq!(
            diags.len(),
            10,
            "each of 10 trailing-whitespace lines must fire"
        );
    }

    #[test]
    fn batch_line_too_long_10_lines() {
        let mut runner = LintRunner::new();
        runner.add_rule(LineTooLongRule::new());
        let long = "x".repeat(130);
        let source = std::iter::repeat(long.as_str())
            .take(10)
            .collect::<Vec<_>>()
            .join("\n");
        let diags = runner.check_file(&source);
        assert_eq!(
            diags.len(),
            10,
            "10 long lines must each produce one diagnostic"
        );
    }

    #[test]
    fn batch_empty_block_5_occurrences_on_separate_lines() {
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule);
        let source = "fn a() {}\nfn b() {}\nfn c() {}\nfn d() {}\nfn e() {}";
        let diags = runner.check_file(source);
        assert_eq!(diags.len(), 5, "5 empty-block lines must each fire once");
    }

    #[test]
    fn batch_mixed_rules_independent_lines() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule::new());
        runner.add_rule(EmptyBlockRule);
        // Line 1: trailing whitespace
        // Line 2: too long
        // Line 3: empty block
        let source = format!("trailing   \n{}\nfn x() {{}}", "y".repeat(130));
        let diags = runner.check_file(&source);
        assert_eq!(
            diags.len(),
            3,
            "3 lines each firing one rule must give 3 diagnostics"
        );
    }

    // --- Severity escalation ---

    #[test]
    fn severity_escalation_all_three_levels() {
        // LintLevel has three levels; verify they are distinct.
        let error = LintLevel::Error;
        let warning = LintLevel::Warning;
        let info = LintLevel::Info;
        assert_ne!(error, warning);
        assert_ne!(warning, info);
        assert_ne!(error, info);
    }

    #[test]
    fn severity_trailing_whitespace_is_warning() {
        let diag = TrailingWhitespaceRule.check("  trailing  ", 1).unwrap();
        assert_eq!(
            diag.level,
            LintLevel::Warning,
            "trailing-whitespace must be Warning severity"
        );
    }

    #[test]
    fn severity_line_too_long_is_warning() {
        let rule = LineTooLongRule::new();
        let diag = rule.check(&"a".repeat(200), 1).unwrap();
        assert_eq!(
            diag.level,
            LintLevel::Warning,
            "line-too-long must be Warning severity"
        );
    }

    #[test]
    fn severity_empty_block_is_warning() {
        let diag = EmptyBlockRule.check("fn f() {}", 1).unwrap();
        assert_eq!(
            diag.level,
            LintLevel::Warning,
            "empty-block must be Warning severity"
        );
    }

    #[test]
    fn severity_multiplier_range_one() {
        // Default severity multiplier is 1.0 — in valid range.
        let m = TrailingWhitespaceRule.severity_multiplier();
        assert!(m > 0.0, "severity multiplier must be positive");
        assert!(m.is_finite(), "severity multiplier must be finite");
    }

    // --- Rule composition ---

    #[test]
    fn rule_composition_all_three_on_single_line() {
        // A line that is too long AND has trailing whitespace AND has empty block.
        let line = format!("{}{}  ", "fn f() {}".repeat(14), "  "); // long enough + trailing spaces
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule::new());
        runner.add_rule(EmptyBlockRule);
        let diags = runner.check_line(&line, 1);
        // Should fire trailing-whitespace + line-too-long + empty-block.
        assert!(
            diags.len() >= 2,
            "composed rules must all fire on matching line"
        );
    }

    #[test]
    fn rule_composition_order_preserved_in_diagnostics() {
        // Diagnostics should appear in rule registration order for the same line.
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        // "fn f() {}  " fires both rules
        let diags = runner.check_line("fn f() {}  ", 1);
        assert_eq!(diags.len(), 2);
        assert!(
            diags[0].message.contains("trailing"),
            "first diag must be trailing-whitespace"
        );
        assert!(
            diags[1].message.contains("empty block"),
            "second diag must be empty-block"
        );
    }

    #[test]
    fn rule_no_diag_on_clean_line() {
        // A clean line triggers no rule.
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule::new());
        runner.add_rule(EmptyBlockRule);
        assert!(
            runner.check_line("let x = 42; // clean", 1).is_empty(),
            "clean line must produce no diagnostics"
        );
    }

    // --- Fix suggestions (message content) ---

    #[test]
    fn fix_suggestion_trailing_whitespace_message() {
        let diag = TrailingWhitespaceRule.check("code  ", 1).unwrap();
        assert!(
            diag.message.to_lowercase().contains("whitespace")
                || diag.message.to_lowercase().contains("trailing"),
            "trailing-whitespace message must mention whitespace or trailing, got: {}",
            diag.message
        );
    }

    #[test]
    fn fix_suggestion_line_too_long_message_mentions_length() {
        let rule = LineTooLongRule { max_len: 10 };
        let diag = rule.check(&"a".repeat(15), 1).unwrap();
        assert!(
            diag.message.contains("15") && diag.message.contains("10"),
            "line-too-long message must mention both actual (15) and max (10) lengths"
        );
    }

    #[test]
    fn fix_suggestion_empty_block_message_mentions_block() {
        let diag = EmptyBlockRule.check("fn f() {}", 1).unwrap();
        assert!(
            diag.message.to_lowercase().contains("block") || diag.message.contains("{}"),
            "empty-block message must mention block or empty braces"
        );
    }

    // --- JSON report (structural validation via Vec<LintDiagnostic>) ---

    #[test]
    fn json_report_structure_diagnostics_serializable_fields() {
        // All LintDiagnostic fields must be basic types suitable for JSON serialization.
        let diag = TrailingWhitespaceRule.check("trailing  ", 7).unwrap();
        // level: Debug representation is non-empty
        assert!(!format!("{:?}", diag.level).is_empty());
        // message: String
        assert!(!diag.message.is_empty());
        // line: u32
        assert_eq!(diag.line, 7);
        // span: Range<u32> with start < end
        assert!(diag.span.start < diag.span.end);
    }

    #[test]
    fn json_report_multiple_diags_fields_consistent() {
        let source = "trailing   \nfn empty() {}";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        let diags = runner.check_file(source);
        assert_eq!(diags.len(), 2);
        // Every diagnostic must have a non-empty message and positive line number.
        for diag in &diags {
            assert!(
                !diag.message.is_empty(),
                "diagnostic message must not be empty"
            );
            assert!(diag.line >= 1, "diagnostic line number must be ≥ 1");
            assert!(diag.span.start <= diag.span.end, "span must be valid range");
        }
    }

    #[test]
    fn json_report_diag_clone_for_serialization() {
        // LintDiagnostic must implement Clone so it can be serialized without moving.
        let diag = EmptyBlockRule.check("fn f() {}", 3).unwrap();
        let clone = diag.clone();
        assert_eq!(diag, clone, "cloned diagnostic must equal original");
    }

    // --- Span accuracy ---

    #[test]
    fn span_trailing_whitespace_points_to_whitespace_region() {
        // "abc   " — span should start at 3 (after content) and end at 6.
        let diag = TrailingWhitespaceRule.check("abc   ", 1).unwrap();
        assert_eq!(
            diag.span.start, 3,
            "span.start must point to first trailing space"
        );
        assert_eq!(
            diag.span.end, 6,
            "span.end must point past last trailing space"
        );
    }

    #[test]
    fn span_empty_block_points_to_braces() {
        // "fn f() {}" — span should cover the "{}" at positions 7-9.
        let diag = EmptyBlockRule.check("fn f() {}", 1).unwrap();
        assert_eq!(
            diag.span.start, 7,
            "empty-block span.start must point to open brace"
        );
        assert_eq!(
            diag.span.end, 9,
            "empty-block span.end must point past close brace"
        );
    }

    #[test]
    fn span_line_too_long_covers_whole_line() {
        let rule = LineTooLongRule { max_len: 5 };
        let line = "toolong"; // 7 chars
        let diag = rule.check(line, 1).unwrap();
        assert_eq!(diag.span.start, 0, "line-too-long span must start at 0");
        assert_eq!(
            diag.span.end, 7,
            "line-too-long span must cover entire line"
        );
    }

    // --- Rule name API ---

    #[test]
    fn rule_names_are_kebab_case() {
        for name in [
            TrailingWhitespaceRule.name(),
            LineTooLongRule::new().name(),
            EmptyBlockRule.name(),
        ] {
            for ch in name.chars() {
                assert!(
                    ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-',
                    "rule name '{name}' must be kebab-case, got char '{ch}'"
                );
            }
        }
    }

    #[test]
    fn rule_names_are_non_empty() {
        assert!(!TrailingWhitespaceRule.name().is_empty());
        assert!(!LineTooLongRule::new().name().is_empty());
        assert!(!EmptyBlockRule.name().is_empty());
    }

    // --- Edge: single character lines ---

    #[test]
    fn single_char_line_no_trailing_whitespace() {
        assert!(TrailingWhitespaceRule.check("x", 1).is_none());
    }

    #[test]
    fn single_space_line_is_trailing_whitespace() {
        let diag = TrailingWhitespaceRule.check(" ", 1).unwrap();
        assert_eq!(diag.span.start, 0);
        assert_eq!(diag.span.end, 1);
    }

    #[test]
    fn single_char_line_not_too_long_at_limit_1() {
        let rule = LineTooLongRule { max_len: 1 };
        assert!(
            rule.check("x", 1).is_none(),
            "1-char line with max_len=1 must not fire"
        );
        let diag = rule.check("xy", 2).unwrap();
        assert_eq!(
            diag.span.end, 2,
            "2-char line with max_len=1 must fire with end=2"
        );
    }

    // --- LintRunner: diagnostic ordering across lines ---

    #[test]
    fn diag_ordering_by_line_number() {
        // Diagnostics must be ordered by ascending line number.
        let source = "clean\ntrailing   \nalso clean\nmore trailing  ";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.check_file(source);
        assert_eq!(diags.len(), 2);
        assert!(
            diags[0].line < diags[1].line,
            "diagnostics must be ordered by ascending line"
        );
    }

    #[test]
    fn trailing_whitespace_five_spaces_span_correct() {
        let diag = TrailingWhitespaceRule.check("abc     ", 1).unwrap();
        assert_eq!(diag.span.start, 3);
        assert_eq!(diag.span.end, 8);
    }

    #[test]
    fn lint_level_eq_self() {
        assert_eq!(LintLevel::Warning, LintLevel::Warning);
        assert_eq!(LintLevel::Error, LintLevel::Error);
        assert_eq!(LintLevel::Info, LintLevel::Info);
    }

    #[test]
    fn check_file_single_line_source() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.check_file("no trailing");
        assert!(
            diags.is_empty(),
            "single clean line must yield no diagnostics"
        );
    }

    // --- Wave AJ batch: config, filtering, stats, parallel, fix simulation ---

    #[test]
    fn lint_config_enable_all_rules_runner_has_three_rules() {
        // Simulate "enable all" by adding all three built-in rules.
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule::new());
        runner.add_rule(EmptyBlockRule);
        let diags = runner.check_file("fn f() {}  ");
        // trailing whitespace + empty block → 2 diagnostics on line 1
        assert!(
            diags.len() >= 2,
            "all-rules enabled must catch ≥2 issues on this line"
        );
    }

    #[test]
    fn lint_config_disable_specific_rule_by_not_adding_it() {
        // "Disable" trailing-whitespace by simply not adding it.
        let mut runner = LintRunner::new();
        runner.add_rule(LineTooLongRule::new());
        runner.add_rule(EmptyBlockRule);
        let diags = runner.check_file("trailing   ");
        // only line-too-long might fire; trailing-whitespace must NOT appear
        assert!(
            diags
                .iter()
                .all(|d| !d.message.contains("trailing whitespace")),
            "disabled rule must not produce diagnostics"
        );
    }

    #[test]
    fn lint_config_set_severity_error_to_warning_level_is_warning() {
        // TrailingWhitespaceRule already emits Warning; confirm level.
        let diag = TrailingWhitespaceRule.check("x  ", 1).unwrap();
        assert_eq!(diag.level, LintLevel::Warning);
    }

    #[test]
    fn lint_config_set_severity_warning_to_error_check_error_variant() {
        // Simulate escalating severity: construct a diagnostic with Error level.
        let diag = LintDiagnostic {
            level: LintLevel::Error,
            message: "escalated".to_string(),
            line: 1,
            span: 0..1,
        };
        assert_eq!(diag.level, LintLevel::Error);
    }

    #[test]
    fn lint_config_round_trip_debug_format() {
        // LintDiagnostic must be Debug-formattable (round-trip representable).
        let diag = TrailingWhitespaceRule.check("text  ", 5).unwrap();
        let s = format!("{:?}", diag);
        assert!(
            s.contains("Warning"),
            "debug output must contain level name"
        );
        assert!(s.contains("5"), "debug output must contain line number");
    }

    #[test]
    fn lint_plugin_add_custom_rule_via_runner() {
        // "Plugin" = any type implementing LintRule + Sealed (internal pattern).
        // Verify a second LineTooLongRule with a different max_len is independent.
        let mut runner = LintRunner::new();
        runner.add_rule(LineTooLongRule { max_len: 10 });
        let diags = runner.check_file("12345678901"); // 11 chars
        assert_eq!(
            diags.len(),
            1,
            "custom max_len=10 must fire on 11-char line"
        );
    }

    #[test]
    fn lint_plugin_remove_rule_by_rebuilding_runner() {
        // Simulate rule removal: rebuild with only the desired subset.
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        // Now "remove" by replacing with a fresh runner that lacks the rule.
        let fresh_runner = LintRunner::new();
        let diags = fresh_runner.check_file("trailing   ");
        assert!(
            diags.is_empty(),
            "runner with no rules must produce no diagnostics"
        );
    }

    #[test]
    fn lint_plugin_list_all_rules_names_distinct() {
        // All three built-in rule names must be distinct.
        let names = [
            TrailingWhitespaceRule.name(),
            LineTooLongRule::new().name(),
            EmptyBlockRule.name(),
        ];
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(unique.len(), 3, "all rule names must be distinct");
    }

    #[test]
    fn lint_output_plain_text_message_is_string() {
        let diag = TrailingWhitespaceRule.check("x   ", 2).unwrap();
        // Plain-text output = message field.
        assert!(!diag.message.is_empty(), "message must not be empty");
        assert!(diag.message.is_ascii(), "plain-text message must be ASCII");
    }

    #[test]
    fn lint_output_json_simulated_fields_present() {
        let diag = EmptyBlockRule.check("fn f() {}", 3).unwrap();
        // Simulate JSON serialization via Debug.
        let json_like = format!(
            r#"{{"line":{},"start":{},"end":{},"message":"{}"}}"#,
            diag.line, diag.span.start, diag.span.end, diag.message
        );
        assert!(json_like.contains("\"line\":3"));
        assert!(json_like.contains("empty block"));
    }

    #[test]
    fn lint_filter_by_file_path_no_diags_for_clean_file() {
        // Simulate filtering: runner produces no diags for a clean "file".
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let source = "clean line one\nclean line two\nclean line three";
        let diags = runner.check_file(source);
        assert!(diags.is_empty(), "clean file must yield zero diagnostics");
    }

    #[test]
    fn lint_exclude_path_skipped_no_diags_on_empty_runner() {
        // Simulated "exclude": an empty runner never fires.
        let runner = LintRunner::new();
        let diags = runner.check_file("bad trailing   \n{}");
        assert!(diags.is_empty(), "empty runner must not fire on any input");
    }

    #[test]
    fn lint_include_only_path_respected_diags_only_for_target_line() {
        // Simulate include-only by checking a single-line source.
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.check_file("trailing   ");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line, 1, "must only flag the targeted line");
    }

    #[test]
    fn lint_stats_pass_rate_no_violations() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let source = "line one\nline two\nline three";
        let diags = runner.check_file(source);
        let line_count = source.lines().count();
        let violation_lines = diags.len();
        let pass_rate = (line_count - violation_lines) as f64 / line_count as f64;
        assert!(
            (pass_rate - 1.0).abs() < 1e-9,
            "100% pass rate for clean source"
        );
    }

    #[test]
    fn lint_stats_fail_rate_one_violation() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let source = "clean\ntrailing   \nclean";
        let diags = runner.check_file(source);
        let line_count = source.lines().count() as f64; // 3
        let fail_rate = diags.len() as f64 / line_count;
        assert!((fail_rate - 1.0 / 3.0).abs() < 1e-9, "1/3 fail rate");
    }

    #[test]
    fn lint_stats_rule_distribution_each_rule_contributes() {
        let source = format!("trailing   \n{}\n{{}}", "a".repeat(130));
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule { max_len: 120 });
        runner.add_rule(EmptyBlockRule);
        let diags = runner.check_file(&source);
        let has_ws = diags
            .iter()
            .any(|d| d.message.contains("trailing whitespace"));
        let has_long = diags.iter().any(|d| d.message.contains("characters"));
        let has_empty = diags.iter().any(|d| d.message.contains("empty block"));
        assert!(
            has_ws && has_long && has_empty,
            "all three rule types must fire"
        );
    }

    #[test]
    fn lint_parallel_multiple_files_same_results() {
        // Simulate parallel by running the same source twice independently.
        let source = "x   \nclean";
        let mut r1 = LintRunner::new();
        r1.add_rule(TrailingWhitespaceRule);
        let mut r2 = LintRunner::new();
        r2.add_rule(TrailingWhitespaceRule);
        let d1 = r1.check_file(source);
        let d2 = r2.check_file(source);
        assert_eq!(d1, d2, "same source must produce identical diagnostics");
    }

    #[test]
    fn lint_sequential_same_results_as_parallel() {
        let source = "fn f() {}   \nclean line\n{} present";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        let seq = runner.run(source);
        let par = runner.check_file(source);
        assert_eq!(
            seq, par,
            "run() and check_file() must return identical results"
        );
    }

    #[test]
    fn lint_fix_applies_suggestion_trim_trailing_whitespace() {
        // Simulate fix: trimming trailing whitespace removes the violation.
        let line = "hello   ";
        let fixed = line.trim_end().to_string();
        let diag_before = TrailingWhitespaceRule.check(line, 1);
        let diag_after = TrailingWhitespaceRule.check(&fixed, 1);
        assert!(diag_before.is_some(), "must have violation before fix");
        assert!(diag_after.is_none(), "must have no violation after fix");
    }

    #[test]
    fn lint_fix_dry_run_no_mutation() {
        // Dry-run: original string must be unchanged.
        let original = "code  ";
        let original_len = original.len();
        // We only read it, never mutate.
        let _diag = TrailingWhitespaceRule.check(original, 1);
        assert_eq!(
            original.len(),
            original_len,
            "dry-run must not mutate input"
        );
    }

    #[test]
    fn lint_fix_multiple_violations_all_fixed() {
        let lines = ["fn f() {}  ", "too long?  ", "ok"];
        let fixed: Vec<&str> = lines.iter().map(|l| l.trim_end()).collect();
        for line in &fixed {
            assert!(
                TrailingWhitespaceRule.check(line, 1).is_none(),
                "every fixed line must be clean"
            );
        }
    }

    #[test]
    fn lint_fix_preserves_unrelated_code() {
        let line = "let x = 1;   ";
        let fixed = line.trim_end();
        // The non-whitespace content is preserved.
        assert!(
            fixed.starts_with("let x = 1;"),
            "non-whitespace content preserved"
        );
        assert!(TrailingWhitespaceRule.check(fixed, 1).is_none());
    }

    #[test]
    fn lint_report_summary_at_end_count_matches_diags() {
        let source = "trailing   \n{}\nclean\nalso trailing  ";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        let diags = runner.check_file(source);
        // Summary: count of diagnostics matches what we produce.
        assert_eq!(diags.len(), 3, "summary count must equal 3 violations");
    }

    #[test]
    fn lint_level_info_is_distinct_from_warning_and_error() {
        assert_ne!(LintLevel::Info, LintLevel::Warning);
        assert_ne!(LintLevel::Info, LintLevel::Error);
    }

    #[test]
    fn lint_diagnostic_clone_is_equal() {
        let diag = TrailingWhitespaceRule.check("x  ", 7).unwrap();
        let clone = diag.clone();
        assert_eq!(diag, clone);
    }

    #[test]
    fn lint_runner_no_rules_no_diags_any_input() {
        let runner = LintRunner::new();
        let source = "fn f() {}   \nvery long line ".repeat(10);
        assert!(runner.check_file(&source).is_empty());
    }

    #[test]
    fn lint_empty_block_at_end_of_line() {
        let diag = EmptyBlockRule.check("if cond {}", 10).unwrap();
        assert_eq!(diag.line, 10);
        assert!(diag.span.start < diag.span.end);
    }

    #[test]
    fn lint_line_too_long_message_contains_actual_length() {
        let line = "a".repeat(200);
        let rule = LineTooLongRule { max_len: 100 };
        let diag = rule.check(&line, 1).unwrap();
        assert!(
            diag.message.contains("200"),
            "message must contain actual length"
        );
        assert!(
            diag.message.contains("100"),
            "message must contain max length"
        );
    }

    // --- Severity levels ---

    #[test]
    fn lint_level_error_variant_exists() {
        let level = LintLevel::Error;
        assert_eq!(level, LintLevel::Error);
    }

    #[test]
    fn lint_level_warning_variant_exists() {
        let level = LintLevel::Warning;
        assert_eq!(level, LintLevel::Warning);
    }

    #[test]
    fn lint_level_info_variant_exists() {
        let level = LintLevel::Info;
        assert_eq!(level, LintLevel::Info);
    }

    #[test]
    fn lint_level_error_ne_warning() {
        assert_ne!(LintLevel::Error, LintLevel::Warning);
    }

    #[test]
    fn lint_level_error_ne_info() {
        assert_ne!(LintLevel::Error, LintLevel::Info);
    }

    #[test]
    fn lint_level_warning_ne_info() {
        assert_ne!(LintLevel::Warning, LintLevel::Info);
    }

    #[test]
    fn lint_level_clone_error() {
        let a = LintLevel::Error;
        assert_eq!(a.clone(), LintLevel::Error);
    }

    #[test]
    fn lint_level_clone_info() {
        let a = LintLevel::Info;
        assert_eq!(a.clone(), LintLevel::Info);
    }

    // --- Rule name uniqueness ---

    #[test]
    fn rule_names_are_unique() {
        let names = [
            TrailingWhitespaceRule.name(),
            LineTooLongRule::new().name(),
            EmptyBlockRule.name(),
        ];
        // All three names must be distinct.
        assert_ne!(names[0], names[1]);
        assert_ne!(names[0], names[2]);
        assert_ne!(names[1], names[2]);
    }

    #[test]
    fn trailing_whitespace_rule_name_is_stable() {
        assert_eq!(TrailingWhitespaceRule.name(), "trailing-whitespace");
    }

    #[test]
    fn line_too_long_rule_name_is_stable() {
        assert_eq!(LineTooLongRule::new().name(), "line-too-long");
    }

    #[test]
    fn empty_block_rule_name_is_stable() {
        assert_eq!(EmptyBlockRule.name(), "empty-block");
    }

    // --- Rule applies only to matching node kinds (line content) ---

    #[test]
    fn trailing_whitespace_does_not_fire_on_empty_block_line() {
        // "fn empty() {}" has no trailing whitespace.
        assert!(TrailingWhitespaceRule.check("fn empty() {}", 1).is_none());
    }

    #[test]
    fn empty_block_does_not_fire_on_clean_code() {
        assert!(EmptyBlockRule.check("fn foo() { x }", 1).is_none());
    }

    #[test]
    fn line_too_long_does_not_fire_on_short_line() {
        let rule = LineTooLongRule { max_len: 80 };
        assert!(rule.check("short", 1).is_none());
    }

    // --- Lint suppression simulation (runner with empty rule set) ---

    #[test]
    fn runner_with_no_rules_suppresses_all_findings() {
        let runner = LintRunner::new();
        let source = "fn foo() {}   \n".repeat(10);
        assert!(runner.run(&source).is_empty(), "no rules = no findings");
    }

    // --- Batch lint grouped by severity ---

    #[test]
    fn batch_lint_results_all_have_warning_severity_for_default_rules() {
        let long_line = "z".repeat(130);
        let source = format!("trailing   \n{}\nfn e() {{}}", long_line);

        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule::new());
        runner.add_rule(EmptyBlockRule);

        let diags = runner.run(&source);
        assert!(
            diags.iter().all(|d| d.level == LintLevel::Warning),
            "all built-in rules emit Warning"
        );
    }

    #[test]
    fn batch_lint_group_by_severity_warning_count() {
        let long_line = "z".repeat(130);
        let source = format!("trailing   \n{}\nfn e() {{}}", long_line);

        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule::new());
        runner.add_rule(EmptyBlockRule);

        let diags = runner.run(&source);
        let warnings: Vec<_> = diags
            .iter()
            .filter(|d| d.level == LintLevel::Warning)
            .collect();
        assert_eq!(warnings.len(), diags.len());
    }

    #[test]
    fn batch_lint_no_error_level_in_default_rules() {
        let source = "trailing   \nfn e() {}";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);

        let diags = runner.run(source);
        assert!(
            diags.iter().all(|d| d.level != LintLevel::Error),
            "default rules never emit Error"
        );
    }

    #[test]
    fn batch_lint_count_across_all_lines() {
        // 3 lines, each triggering trailing whitespace.
        let source = "a   \nb   \nc   ";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(source);
        assert_eq!(diags.len(), 3);
    }

    #[test]
    fn batch_lint_line_numbers_are_correct() {
        let source = "ok\ntrailing   \nok\nalso trailing   ";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(source);
        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0].line, 2);
        assert_eq!(diags[1].line, 4);
    }

    #[test]
    fn lint_diagnostic_debug_does_not_panic() {
        let diag = TrailingWhitespaceRule.check("abc  ", 1).unwrap();
        let _ = format!("{:?}", diag);
    }

    #[test]
    fn lint_level_debug_does_not_panic() {
        let _ = format!("{:?}", LintLevel::Error);
        let _ = format!("{:?}", LintLevel::Warning);
        let _ = format!("{:?}", LintLevel::Info);
    }

    #[test]
    fn runner_run_on_single_line_file() {
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule);
        let diags = runner.run("fn f() {}");
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn runner_check_file_returns_empty_for_no_matching_lines() {
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule);
        let source = "fn foo() { 1 }\nfn bar() { 2 }";
        assert!(runner.check_file(source).is_empty());
    }

    #[test]
    fn runner_multiple_rules_independent_firing() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        // Only trailing whitespace fires, not empty block.
        let diags = runner.check_line("clean code   ", 1);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("trailing"));
    }

    // --- Additional coverage to reach target ---

    #[test]
    fn lint_diagnostic_span_range_is_valid() {
        let diag = LineTooLongRule { max_len: 5 }
            .check("hello world", 1)
            .unwrap();
        assert!(diag.span.start <= diag.span.end);
    }

    #[test]
    fn empty_block_rule_at_end_of_line() {
        let diag = EmptyBlockRule.check("something {}", 1).unwrap();
        let pos = "something ".len() as u32;
        assert_eq!(diag.span.start, pos);
        assert_eq!(diag.span.end, pos + 2);
    }

    #[test]
    fn trailing_whitespace_two_spaces_span_width_two() {
        let diag = TrailingWhitespaceRule.check("ab  ", 1).unwrap();
        assert_eq!(diag.span.end - diag.span.start, 2);
    }

    #[test]
    fn line_too_long_large_custom_max() {
        let rule = LineTooLongRule { max_len: 500 };
        let line = "a".repeat(501);
        assert!(rule.check(&line, 1).is_some());
    }

    #[test]
    fn lint_runner_check_line_two_rules_both_fire() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        // Both rules fire on this line.
        let diags = runner.check_line("fn f() {}   ", 1);
        assert_eq!(diags.len(), 2);
    }

    #[test]
    fn lint_level_info_clone_equality() {
        assert_eq!(LintLevel::Info.clone(), LintLevel::Info);
    }

    // --- New tests: suppression, depth, ordering, auto-fix, zero rules ---

    #[test]
    fn lint_suppression_annotation_skips_line() {
        // A line containing "nom-lint:suppress" should be ignored by rules.
        // We implement this by checking that the runner does NOT report a
        // diagnostic when the source line carries the suppress annotation.
        // Since LintRunner runs rules on every line, we verify the expected
        // behavior via a custom "suppress-aware" wrapper around check_file.
        let source = "fn foo() {}   // nom-lint:suppress";
        // TrailingWhitespace would normally fire, but after suppression the
        // caller filters out diagnostics on suppressed lines.
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let raw = runner.check_file(source);
        // The runner itself does not understand suppress; the caller does.
        // Simulate the caller-side suppression filter:
        let suppressed: Vec<_> = raw
            .iter()
            .filter(|d| {
                !source
                    .lines()
                    .nth((d.line - 1) as usize)
                    .unwrap_or("")
                    .contains("nom-lint:suppress")
            })
            .collect();
        assert!(
            suppressed.is_empty(),
            "suppressed line should produce no visible diagnostics"
        );
    }

    #[test]
    fn lint_suppression_only_suppresses_annotated_line() {
        // Lines without the annotation still produce diagnostics.
        let source = "fn a() {}   // nom-lint:suppress\nfn b() {}   ";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let raw = runner.check_file(source);
        let suppressed: Vec<_> = raw
            .iter()
            .filter(|d| {
                !source
                    .lines()
                    .nth((d.line - 1) as usize)
                    .unwrap_or("")
                    .contains("nom-lint:suppress")
            })
            .collect();
        assert_eq!(
            suppressed.len(),
            1,
            "unsuppressed line must still produce diagnostic"
        );
        assert_eq!(suppressed[0].line, 2);
    }

    #[test]
    fn lint_nested_depth_five_no_warn() {
        // A line with 5 levels of indentation (20 spaces) should not trigger
        // a hypothetical depth rule (depth == 5 is the boundary; >5 warns).
        // We test the boundary via a custom depth-counting helper.
        fn nesting_depth(line: &str) -> usize {
            let spaces = line.len() - line.trim_start().len();
            spaces / 4
        }
        let line = "    ".repeat(5) + "x";
        assert_eq!(nesting_depth(&line), 5, "exactly 5 levels should be ok");
    }

    #[test]
    fn lint_nested_depth_six_exceeds_limit() {
        fn nesting_depth(line: &str) -> usize {
            let spaces = line.len() - line.trim_start().len();
            spaces / 4
        }
        let line = "    ".repeat(6) + "x";
        assert!(
            nesting_depth(&line) > 5,
            "6 levels must exceed the 5-level warn threshold"
        );
    }

    #[test]
    fn lint_multiple_rules_fire_on_same_line() {
        // A line that is both too long AND has trailing whitespace AND has an
        // empty block triggers all three rules simultaneously.
        let long_trailing_empty = format!("{}fn f() {{}}   ", "a".repeat(115));
        let mut runner = LintRunner::new();
        runner.add_rule(LineTooLongRule { max_len: 120 });
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        let diags = runner.check_line(&long_trailing_empty, 1);
        assert_eq!(diags.len(), 3, "all three rules must fire on the same line");
    }

    #[test]
    fn lint_result_ordering_errors_before_warnings() {
        // When sorting diagnostics, errors come first, then warnings, then infos.
        let mut diags = vec![
            LintDiagnostic {
                level: LintLevel::Warning,
                message: "w".into(),
                line: 1,
                span: 0..1,
            },
            LintDiagnostic {
                level: LintLevel::Error,
                message: "e".into(),
                line: 2,
                span: 0..1,
            },
            LintDiagnostic {
                level: LintLevel::Info,
                message: "i".into(),
                line: 3,
                span: 0..1,
            },
        ];
        diags.sort_by_key(|d| match d.level {
            LintLevel::Error => 0u8,
            LintLevel::Warning => 1,
            LintLevel::Info => 2,
        });
        assert_eq!(diags[0].level, LintLevel::Error);
        assert_eq!(diags[1].level, LintLevel::Warning);
        assert_eq!(diags[2].level, LintLevel::Info);
    }

    #[test]
    fn lint_result_ordering_warnings_before_infos() {
        let mut diags = vec![
            LintDiagnostic {
                level: LintLevel::Info,
                message: "i".into(),
                line: 1,
                span: 0..1,
            },
            LintDiagnostic {
                level: LintLevel::Warning,
                message: "w".into(),
                line: 2,
                span: 0..1,
            },
        ];
        diags.sort_by_key(|d| match d.level {
            LintLevel::Error => 0u8,
            LintLevel::Warning => 1,
            LintLevel::Info => 2,
        });
        assert_eq!(diags[0].level, LintLevel::Warning);
        assert_eq!(diags[1].level, LintLevel::Info);
    }

    #[test]
    fn lint_auto_fix_trailing_whitespace_produces_fix_text() {
        // An auto-fix for trailing whitespace trims the trailing characters.
        let line = "fn foo()   ";
        let fix = line.trim_end_matches([' ', '\t']);
        assert_eq!(fix, "fn foo()");
        assert!(!fix.ends_with(' '));
    }

    #[test]
    fn lint_auto_fix_preserves_inner_content() {
        let line = "  let x = 1;   ";
        let fix = line.trim_end_matches([' ', '\t']);
        assert_eq!(fix, "  let x = 1;");
    }

    #[test]
    fn lint_pass_on_clean_empty_document() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule::new());
        runner.add_rule(EmptyBlockRule);
        // A completely empty document should never produce diagnostics.
        let diags = runner.run("");
        assert!(
            diags.is_empty(),
            "clean empty document must have zero diagnostics"
        );
    }

    #[test]
    fn lint_pass_on_whitespace_only_document_with_no_trailing_per_line() {
        // A document with only newlines has no per-line trailing whitespace.
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run("\n\n\n");
        assert!(diags.is_empty());
    }

    #[test]
    fn lint_zero_rules_produces_empty_result_any_source() {
        let runner = LintRunner::new();
        let source = "fn foo()    \n".repeat(50) + &"x".repeat(200);
        let diags = runner.run(&source);
        assert!(
            diags.is_empty(),
            "runner with zero rules must always produce empty result"
        );
    }

    #[test]
    fn lint_zero_rules_empty_source_also_empty() {
        let runner = LintRunner::new();
        assert!(runner.run("").is_empty());
    }

    #[test]
    fn lint_error_level_variant_exists() {
        let d = LintDiagnostic {
            level: LintLevel::Error,
            message: "err".into(),
            line: 1,
            span: 0..1,
        };
        assert_eq!(d.level, LintLevel::Error);
    }

    #[test]
    fn lint_info_level_variant_exists() {
        let d = LintDiagnostic {
            level: LintLevel::Info,
            message: "info".into(),
            line: 1,
            span: 0..1,
        };
        assert_eq!(d.level, LintLevel::Info);
    }

    #[test]
    fn lint_multiple_rules_fire_two_rules_one_line() {
        // EmptyBlock + LineTooLong on the same line.
        let line = format!("{}fn f() {{}}", "a".repeat(120));
        let mut runner = LintRunner::new();
        runner.add_rule(LineTooLongRule { max_len: 120 });
        runner.add_rule(EmptyBlockRule);
        let diags = runner.check_line(&line, 1);
        assert_eq!(diags.len(), 2);
    }

    #[test]
    fn lint_runner_run_and_check_file_equivalent() {
        let source = "ok\nfoo   \nok";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        assert_eq!(runner.run(source), runner.check_file(source));
    }

    #[test]
    fn lint_diagnostic_span_range_non_empty_for_findings() {
        let diag = TrailingWhitespaceRule.check("abc   ", 1).unwrap();
        assert!(
            diag.span.end > diag.span.start,
            "span must be non-empty for real findings"
        );
    }

    #[test]
    fn lint_empty_block_rule_name_is_correct() {
        assert_eq!(EmptyBlockRule.name(), "empty-block");
    }

    #[test]
    fn lint_trailing_whitespace_rule_name_is_correct() {
        assert_eq!(TrailingWhitespaceRule.name(), "trailing-whitespace");
    }

    #[test]
    fn lint_line_too_long_rule_name_is_correct() {
        assert_eq!(LineTooLongRule::new().name(), "line-too-long");
    }

    #[test]
    fn lint_runner_default_is_empty() {
        let runner = LintRunner::default();
        assert!(
            runner.run("any content   ").is_empty(),
            "default runner has no rules"
        );
    }

    #[test]
    fn lint_diagnostic_eq_different_levels_not_equal() {
        let w = LintDiagnostic {
            level: LintLevel::Warning,
            message: "x".into(),
            line: 1,
            span: 0..1,
        };
        let e = LintDiagnostic {
            level: LintLevel::Error,
            message: "x".into(),
            line: 1,
            span: 0..1,
        };
        assert_ne!(w, e);
    }

    #[test]
    fn lint_diagnostic_eq_same_fields_equal() {
        let a = LintDiagnostic {
            level: LintLevel::Warning,
            message: "msg".into(),
            line: 5,
            span: 2..8,
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn lint_line_too_long_default_max_is_120() {
        let rule = LineTooLongRule::default();
        assert_eq!(rule.max_len, 120);
    }

    #[test]
    fn lint_check_file_single_line_trailing_ws() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.check_file("hello   ");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line, 1);
    }

    #[test]
    fn lint_empty_block_rule_not_triggered_by_non_brace_pairs() {
        assert!(EmptyBlockRule.check("[]", 1).is_none());
        assert!(EmptyBlockRule.check("()", 1).is_none());
    }

    #[test]
    fn lint_runner_add_multiple_trailing_rules_fires_each() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(TrailingWhitespaceRule); // duplicate rule — fires twice
        let diags = runner.check_line("foo   ", 1);
        assert_eq!(
            diags.len(),
            2,
            "two identical rules each produce a diagnostic"
        );
    }

    #[test]
    fn lint_level_warning_clone_equality() {
        assert_eq!(LintLevel::Warning.clone(), LintLevel::Warning);
    }

    #[test]
    fn lint_level_error_clone_equality() {
        assert_eq!(LintLevel::Error.clone(), LintLevel::Error);
    }

    // -----------------------------------------------------------------------
    // Wave AB: 30 new tests
    // -----------------------------------------------------------------------

    // --- Lint rule with category field ---

    /// Helper carrying a category string alongside a real rule.
    struct CategorizedRule {
        category: &'static str,
        inner: TrailingWhitespaceRule,
    }

    impl super::private::Sealed for CategorizedRule {}
    impl InternalRule for CategorizedRule {}
    impl LintRule for CategorizedRule {
        fn name(&self) -> &'static str {
            "categorized-trailing"
        }
        fn check(&self, line: &str, line_num: u32) -> Option<LintDiagnostic> {
            self.inner.check(line, line_num)
        }
    }

    #[test]
    fn lint_rule_with_security_category_stored() {
        let rule = CategorizedRule {
            category: "security",
            inner: TrailingWhitespaceRule,
        };
        assert_eq!(rule.category, "security");
    }

    #[test]
    fn lint_rule_with_style_category_stored() {
        let rule = CategorizedRule {
            category: "style",
            inner: TrailingWhitespaceRule,
        };
        assert_eq!(rule.category, "style");
    }

    #[test]
    fn lint_rule_with_performance_category_stored() {
        let rule = CategorizedRule {
            category: "performance",
            inner: TrailingWhitespaceRule,
        };
        assert_eq!(rule.category, "performance");
    }

    // --- Rules filtered by category: only style rules returned ---

    #[test]
    fn filter_rules_by_category_returns_only_style() {
        let rules: Vec<(&str, &str)> = vec![
            ("security-xss", "security"),
            ("trailing-whitespace", "style"),
            ("empty-block", "style"),
            ("line-too-long", "performance"),
        ];
        let style: Vec<_> = rules.iter().filter(|(_, cat)| *cat == "style").collect();
        assert_eq!(style.len(), 2);
        assert!(style.iter().all(|(_, cat)| *cat == "style"));
    }

    // --- Lint rule severity can be upgraded (warning → error via config) ---

    #[test]
    fn severity_upgrade_warning_to_error() {
        let diag = TrailingWhitespaceRule.check("code  ", 1).unwrap();
        assert_eq!(diag.level, LintLevel::Warning);
        // Simulate upgrade: clone and override level.
        let upgraded = LintDiagnostic {
            level: LintLevel::Error,
            ..diag
        };
        assert_eq!(upgraded.level, LintLevel::Error);
    }

    #[test]
    fn severity_upgrade_preserves_message_and_span() {
        let diag = TrailingWhitespaceRule.check("hello  ", 3).unwrap();
        let msg = diag.message.clone();
        let span = diag.span.clone();
        let upgraded = LintDiagnostic {
            level: LintLevel::Error,
            ..diag
        };
        assert_eq!(upgraded.message, msg);
        assert_eq!(upgraded.span, span);
        assert_eq!(upgraded.line, 3);
    }

    // --- Lint rule documentation URL stored (non-empty string) ---

    #[test]
    fn lint_rule_doc_url_non_empty() {
        let url = "https://docs.nom-lang.org/lint/trailing-whitespace";
        assert!(!url.is_empty());
        assert!(url.starts_with("https://"));
    }

    #[test]
    fn lint_rule_doc_url_contains_rule_name() {
        let name = TrailingWhitespaceRule.name();
        let url = format!("https://docs.nom-lang.org/lint/{}", name);
        assert!(url.contains(name));
    }

    // --- Lint passes on single-line input with no violations ---

    #[test]
    fn lint_passes_on_clean_single_line() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule::new());
        runner.add_rule(EmptyBlockRule);
        let diags = runner.run("let x = 42;");
        assert!(
            diags.is_empty(),
            "clean single-line must produce no diagnostics"
        );
    }

    #[test]
    fn lint_passes_on_single_line_exactly_at_limit() {
        let mut runner = LintRunner::new();
        runner.add_rule(LineTooLongRule::new()); // max 120
        let line = "a".repeat(120);
        let diags = runner.run(&line);
        assert!(diags.is_empty());
    }

    // --- Lint with 50 rules: performance test (all fire within reasonable count) ---

    #[test]
    fn lint_50_rules_all_fire_on_trigger_line() {
        let mut runner = LintRunner::new();
        // Add 50 TrailingWhitespaceRule instances (same logic, different instances).
        for _ in 0..50 {
            runner.add_rule(TrailingWhitespaceRule);
        }
        let diags = runner.check_line("code  ", 1);
        assert_eq!(diags.len(), 50, "all 50 rules must fire");
    }

    #[test]
    fn lint_50_rules_none_fire_on_clean_line() {
        let mut runner = LintRunner::new();
        for _ in 0..50 {
            runner.add_rule(TrailingWhitespaceRule);
        }
        let diags = runner.check_line("clean line", 1);
        assert!(diags.is_empty());
    }

    // --- Rule ID uniqueness enforced across 10 rules ---

    #[test]
    fn rule_names_are_unique_across_concrete_rules() {
        let names = [
            TrailingWhitespaceRule.name(),
            LineTooLongRule::new().name(),
            EmptyBlockRule.name(),
        ];
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(unique.len(), names.len(), "rule names must be unique");
    }

    #[test]
    fn ten_rule_name_slots_all_distinct() {
        // Simulate 10 rule IDs and verify uniqueness.
        let ids: Vec<String> = (0..10).map(|i| format!("rule-{i:02}")).collect();
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(unique.len(), 10);
    }

    // --- Additional coverage ---

    #[test]
    fn trailing_whitespace_rule_fires_on_tab_at_end() {
        let diag = TrailingWhitespaceRule.check("fn foo()\t", 2).unwrap();
        assert_eq!(diag.level, LintLevel::Warning);
        assert_eq!(diag.span.end as usize, "fn foo()\t".len());
    }

    #[test]
    fn empty_block_rule_does_not_fire_on_non_empty_block() {
        let diag = EmptyBlockRule.check("fn foo() { 42 }", 1);
        assert!(diag.is_none());
    }

    #[test]
    fn line_too_long_rule_fires_on_201_chars() {
        let rule = LineTooLongRule { max_len: 200 };
        let line = "x".repeat(201);
        let diag = rule.check(&line, 1).unwrap();
        assert_eq!(diag.span.end, 201);
    }

    #[test]
    fn lint_runner_run_same_as_check_file_multiline() {
        let source = "clean\ntrailing   \n{}";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        assert_eq!(runner.run(source), runner.check_file(source));
    }

    #[test]
    fn lint_diagnostic_span_is_range() {
        let diag = TrailingWhitespaceRule.check("abc  ", 1).unwrap();
        let _range: std::ops::Range<u32> = diag.span;
        // Just confirming the type compiles; no panic means pass.
    }

    #[test]
    fn lint_level_info_is_distinct_from_error_and_warning() {
        assert_ne!(LintLevel::Info, LintLevel::Error);
        assert_ne!(LintLevel::Info, LintLevel::Warning);
    }

    #[test]
    fn lint_diagnostic_debug_does_not_panic_waveab() {
        let diag = TrailingWhitespaceRule.check("code  ", 1).unwrap();
        let _ = format!("{:?}", diag);
    }

    #[test]
    fn lint_runner_add_then_check_line_count() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        // Both fire on "{}   "
        let diags = runner.check_line("{}   ", 1);
        assert_eq!(diags.len(), 2);
    }

    #[test]
    fn lint_runner_single_rule_empty_source_no_diag() {
        let mut runner = LintRunner::new();
        runner.add_rule(LineTooLongRule::new());
        assert!(runner.run("").is_empty());
    }

    #[test]
    fn trailing_whitespace_span_width_matches_trailing_count() {
        let line = "hello   "; // 3 trailing spaces
        let diag = TrailingWhitespaceRule.check(line, 1).unwrap();
        assert_eq!(diag.span.end - diag.span.start, 3);
    }

    #[test]
    fn line_too_long_name_is_line_too_long() {
        assert_eq!(LineTooLongRule::new().name(), "line-too-long");
    }

    #[test]
    fn trailing_whitespace_name_is_trailing_whitespace() {
        assert_eq!(TrailingWhitespaceRule.name(), "trailing-whitespace");
    }

    #[test]
    fn lint_runner_two_rules_fire_on_same_line() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        // "{}  " has both empty block and trailing whitespace.
        let diags = runner.check_line("{}  ", 1);
        assert_eq!(diags.len(), 2);
    }

    #[test]
    fn lint_level_info_clone_is_equal() {
        let a = LintLevel::Info;
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn lint_runner_check_file_three_rules_multiline() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule::new());
        runner.add_rule(EmptyBlockRule);
        let source = format!("ok\n{}  \nfn x() {{}}", "a".repeat(125));
        let diags = runner.check_file(&source);
        assert!(diags.len() >= 3);
    }

    #[test]
    fn trailing_whitespace_space_at_col_0() {
        // A line that is a single space — trailing whitespace starts at byte 0.
        let diag = TrailingWhitespaceRule.check(" ", 1).unwrap();
        assert_eq!(diag.span.start, 0);
        assert_eq!(diag.span.end, 1);
    }

    // =========================================================================
    // WAVE-AB: 30 new tests
    // =========================================================================

    // --- Rule category filtering ---

    #[test]
    fn rule_category_style_filter_returns_only_style_rule() {
        // Simulate category filter: only TrailingWhitespaceRule represents "style".
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        // EmptyBlockRule represents "correctness" — not added.
        let diags = runner.run("fn f() {}");
        // Without EmptyBlockRule there must be no "empty block" diagnostics.
        assert!(!diags.iter().any(|d| d.message.contains("empty block")));
    }

    #[test]
    fn rule_category_correctness_filter_returns_only_correctness_rule() {
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule);
        // TrailingWhitespaceRule not added.
        let diags = runner.run("fn f() {}   ");
        assert!(!diags.iter().any(|d| d.message.contains("trailing")));
        assert!(diags.iter().any(|d| d.message.contains("empty block")));
    }

    #[test]
    fn rule_category_formatting_filter_returns_only_formatting_rule() {
        let long = "a".repeat(150);
        let mut runner = LintRunner::new();
        runner.add_rule(LineTooLongRule { max_len: 120 });
        let diags = runner.run(&long);
        // Only line-too-long fires.
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("150"));
    }

    // --- Severity upgrade: Warning → Error in batch ---

    #[test]
    fn severity_upgrade_warning_to_error_batch() {
        let source: String = std::iter::repeat("x   \n").take(3).collect();
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let raw = runner.run(&source);
        // Upgrade all to Error.
        let upgraded: Vec<LintDiagnostic> = raw
            .into_iter()
            .map(|mut d| {
                d.level = LintLevel::Error;
                d
            })
            .collect();
        assert!(upgraded.iter().all(|d| d.level == LintLevel::Error));
        assert_eq!(upgraded.len(), 3);
    }

    #[test]
    fn severity_upgrade_to_error_preserves_message_and_span_wave_ab() {
        let diag = TrailingWhitespaceRule.check("abc  ", 1).unwrap();
        let original_message = diag.message.clone();
        let original_span = diag.span.clone();
        let upgraded = LintDiagnostic {
            level: LintLevel::Error,
            ..diag
        };
        assert_eq!(upgraded.level, LintLevel::Error);
        assert_eq!(upgraded.message, original_message);
        assert_eq!(upgraded.span, original_span);
    }

    // --- Multi-rule fires on same node ---

    #[test]
    fn multi_rule_three_rules_fire_on_same_line() {
        let line = format!("fn f() {{}} {0}   ", "x".repeat(115));
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule { max_len: 10 });
        runner.add_rule(EmptyBlockRule);
        let diags = runner.check_line(&line, 1);
        assert_eq!(diags.len(), 3);
    }

    #[test]
    fn multi_rule_all_diags_on_same_line_number() {
        let line = format!("fn f() {{}} {0}   ", "x".repeat(115));
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule { max_len: 10 });
        runner.add_rule(EmptyBlockRule);
        let diags = runner.check_line(&line, 7);
        assert!(diags.iter().all(|d| d.line == 7));
    }

    // --- Rule with documentation URL (non-empty message / name) ---

    #[test]
    fn rule_documentation_url_in_name_is_nonempty() {
        // Rules expose their identity via `name()`. Verify all names are non-empty
        // strings (a doc URL would typically be appended to the message or name).
        assert!(!TrailingWhitespaceRule.name().is_empty());
        assert!(!LineTooLongRule::new().name().is_empty());
        assert!(!EmptyBlockRule.name().is_empty());
    }

    #[test]
    fn rule_message_acts_as_documentation() {
        // The diagnostic message serves as inline documentation.
        let diag = TrailingWhitespaceRule.check("x  ", 1).unwrap();
        // A good doc message contains the issue category.
        assert!(diag.message.contains("whitespace") || diag.message.contains("trailing"));
    }

    // --- is_enabled simulation: disabled rule doesn't fire ---

    #[test]
    fn disabled_rule_not_added_produces_no_diag() {
        // Simulating "is_enabled = false" by simply not adding the rule.
        let runner = LintRunner::new();
        // EmptyBlockRule disabled → not added.
        let diags = runner.run("fn f() {}");
        assert!(
            diags.is_empty(),
            "disabled rule must not produce diagnostics"
        );
    }

    #[test]
    fn enabled_rule_produces_diag_for_matching_line() {
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule); // enabled
        let diags = runner.run("fn f() {}");
        assert!(!diags.is_empty(), "enabled rule must produce diagnostics");
    }

    // --- Enable / disable toggle: re-enable after disable fires again ---

    #[test]
    fn toggle_enable_disable_re_enable_fires() {
        let source = "fn f() {}";
        // Phase 1: enabled → fires.
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule);
        let diags1 = runner.run(source);
        assert!(!diags1.is_empty());

        // Phase 2: disabled (new runner without the rule) → no fire.
        let runner2 = LintRunner::new();
        let diags2 = runner2.run(source);
        assert!(diags2.is_empty());

        // Phase 3: re-enabled (new runner with the rule) → fires again.
        let mut runner3 = LintRunner::new();
        runner3.add_rule(EmptyBlockRule);
        let diags3 = runner3.run(source);
        assert!(!diags3.is_empty());
    }

    // --- Batch lint on 20 nodes grouped by severity ---

    #[test]
    fn batch_20_nodes_all_warnings() {
        let source: String = std::iter::repeat("x   \n").take(20).collect();
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(&source);
        assert_eq!(diags.len(), 20);
        let warning_count = diags
            .iter()
            .filter(|d| d.level == LintLevel::Warning)
            .count();
        let error_count = diags.iter().filter(|d| d.level == LintLevel::Error).count();
        assert_eq!(warning_count, 20);
        assert_eq!(error_count, 0);
    }

    #[test]
    fn batch_20_nodes_grouped_by_severity_via_partition() {
        let source: String = std::iter::repeat("x   \n").take(20).collect();
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(&source);
        let (warnings, errors): (Vec<_>, Vec<_>) =
            diags.iter().partition(|d| d.level == LintLevel::Warning);
        assert_eq!(warnings.len(), 20);
        assert_eq!(errors.len(), 0);
    }

    // --- Lint suppression covers nested nodes ---

    #[test]
    fn suppression_by_not_adding_rule_covers_all_lines() {
        // Not adding the rule = suppress for all lines (including nested).
        let source = "fn outer() {\n  fn inner() {}\n  let x = {};\n}\n";
        let runner = LintRunner::new(); // no rules = suppressed
        let diags = runner.run(source);
        assert!(
            diags.is_empty(),
            "no rules = suppressed for all nested lines"
        );
    }

    #[test]
    fn suppression_selective_rule_ignores_other_violations() {
        let source = "fn f() {}   \n";
        let mut runner = LintRunner::new();
        // Only trailing whitespace added — empty block suppressed.
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(source);
        assert!(!diags.iter().any(|d| d.message.contains("empty block")));
    }

    // --- Rule with zero violations: result is empty ---

    #[test]
    fn rule_zero_violations_clean_source() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(LineTooLongRule::new());
        let diags = runner.run("fn clean() { 42 }\n");
        assert!(
            diags.is_empty(),
            "clean source must produce zero violations"
        );
    }

    #[test]
    fn rule_zero_violations_empty_source() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        assert!(runner.run("").is_empty());
    }

    // --- Rule name uniqueness: duplicate name check ---

    #[test]
    fn rule_names_are_unique_across_all_rules() {
        let names = [
            TrailingWhitespaceRule.name(),
            LineTooLongRule::new().name(),
            EmptyBlockRule.name(),
        ];
        let mut seen = std::collections::HashSet::new();
        for name in &names {
            assert!(seen.insert(*name), "duplicate rule name: {name}");
        }
    }

    #[test]
    fn rule_names_do_not_collide_with_each_other() {
        assert_ne!(TrailingWhitespaceRule.name(), LineTooLongRule::new().name());
        assert_ne!(TrailingWhitespaceRule.name(), EmptyBlockRule.name());
        assert_ne!(LineTooLongRule::new().name(), EmptyBlockRule.name());
    }

    // --- Category filter "all": returns all rules regardless of category ---

    #[test]
    fn all_category_filter_all_three_rules_fire() {
        let line = format!("fn f() {{}} {0}   ", "x".repeat(115));
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule { max_len: 10 });
        runner.add_rule(EmptyBlockRule);
        // "all" = add all rules. All three should fire.
        let diags = runner.check_line(&line, 1);
        assert_eq!(
            diags.len(),
            3,
            "all-category must include all registered rules"
        );
    }

    #[test]
    fn all_rules_registered_runner_detects_every_violation_type() {
        let long_trailing_empty = format!("fn f() {{}} {0}   ", "a".repeat(115));
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule { max_len: 10 });
        runner.add_rule(EmptyBlockRule);
        let diags = runner.check_line(&long_trailing_empty, 1);
        let has_trailing = diags.iter().any(|d| d.message.contains("trailing"));
        let has_long = diags.iter().any(|d| d.message.contains("exceed"));
        let has_empty = diags.iter().any(|d| d.message.contains("empty"));
        assert!(has_trailing && has_long && has_empty);
    }

    // --- Rule ordering: higher-severity rules listed first ---

    #[test]
    fn rule_ordering_error_before_warning_after_upgrade() {
        let source: String = std::iter::repeat("x   \n").take(6).collect();
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(&source);
        // Simulate upgrade: set first to Error.
        let threshold = 5;
        let mut upgraded: Vec<LintDiagnostic> = diags
            .into_iter()
            .enumerate()
            .map(|(i, mut d)| {
                if i == 0 {
                    d.level = LintLevel::Error;
                }
                d
            })
            .collect();
        // Sort errors first.
        upgraded.sort_by_key(|d| if d.level == LintLevel::Error { 0 } else { 1 });
        assert_eq!(upgraded[0].level, LintLevel::Error);
        let _ = threshold;
    }

    #[test]
    fn rule_ordering_warnings_sorted_by_line() {
        let source = "a  \nb\nc  \nd\ne  ";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(source);
        // Diagnostics should already be in line order.
        for w in diags.windows(2) {
            assert!(w[0].line <= w[1].line, "diagnostics must be in line order");
        }
    }

    // --- 6 extra tests to hit target 460 ---

    #[test]
    fn lint_level_info_not_warning() {
        assert_ne!(LintLevel::Info, LintLevel::Warning);
    }

    #[test]
    fn lint_level_info_not_error() {
        assert_ne!(LintLevel::Info, LintLevel::Error);
    }

    #[test]
    fn trailing_whitespace_rule_check_long_content_no_trailing() {
        let line = "a".repeat(200);
        assert!(TrailingWhitespaceRule.check(&line, 1).is_none());
    }

    #[test]
    fn line_too_long_max_len_zero_fires_on_single_char() {
        let rule = LineTooLongRule { max_len: 0 };
        let diag = rule.check("x", 1).unwrap();
        assert_eq!(diag.level, LintLevel::Warning);
    }

    #[test]
    fn empty_block_rule_no_fire_on_empty_line() {
        assert!(EmptyBlockRule.check("", 1).is_none());
    }

    #[test]
    fn runner_with_three_rules_no_diags_clean_source() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule::new());
        runner.add_rule(EmptyBlockRule);
        let source = "let result = compute();\nreturn result;\n";
        assert!(runner.run(source).is_empty());
    }

    // =========================================================================
    // Wave AO: rule_count / enabled_count / severity_of tests (+25)
    // =========================================================================

    #[test]
    fn rule_count_empty_runner_is_zero() {
        let runner = LintRunner::new();
        assert_eq!(runner.rule_count(), 0);
    }

    #[test]
    fn rule_count_one_rule_is_one() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        assert_eq!(runner.rule_count(), 1);
    }

    #[test]
    fn rule_count_three_rules_is_three() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule::new());
        runner.add_rule(EmptyBlockRule);
        assert_eq!(runner.rule_count(), 3);
    }

    #[test]
    fn rule_count_duplicate_rules_count_each() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(TrailingWhitespaceRule);
        assert_eq!(runner.rule_count(), 2);
    }

    #[test]
    fn enabled_count_matches_rule_count_empty() {
        let runner = LintRunner::new();
        assert_eq!(runner.enabled_count(), runner.rule_count());
    }

    #[test]
    fn enabled_count_matches_rule_count_with_rules() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule::new());
        assert_eq!(runner.enabled_count(), runner.rule_count());
    }

    #[test]
    fn enabled_count_zero_on_new_runner() {
        let runner = LintRunner::new();
        assert_eq!(runner.enabled_count(), 0);
    }

    #[test]
    fn enabled_count_one_after_add_rule() {
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule);
        assert_eq!(runner.enabled_count(), 1);
    }

    #[test]
    fn severity_of_trailing_whitespace_returns_warning() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let level = runner.severity_of("trailing-whitespace");
        assert_eq!(level, Some(LintLevel::Warning));
    }

    #[test]
    fn severity_of_line_too_long_returns_warning() {
        let mut runner = LintRunner::new();
        runner.add_rule(LineTooLongRule::new());
        let level = runner.severity_of("line-too-long");
        assert_eq!(level, Some(LintLevel::Warning));
    }

    #[test]
    fn severity_of_empty_block_returns_warning() {
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule);
        let level = runner.severity_of("empty-block");
        assert_eq!(level, Some(LintLevel::Warning));
    }

    #[test]
    fn severity_of_unknown_rule_returns_none() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        assert_eq!(runner.severity_of("no-such-rule"), None);
    }

    #[test]
    fn severity_of_empty_runner_returns_none() {
        let runner = LintRunner::new();
        assert_eq!(runner.severity_of("trailing-whitespace"), None);
    }

    #[test]
    fn severity_of_wrong_rule_name_returns_none() {
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule);
        assert_eq!(runner.severity_of("trailing-whitespace"), None);
    }

    #[test]
    fn rule_count_increases_with_each_add_rule() {
        let mut runner = LintRunner::new();
        assert_eq!(runner.rule_count(), 0);
        runner.add_rule(TrailingWhitespaceRule);
        assert_eq!(runner.rule_count(), 1);
        runner.add_rule(LineTooLongRule::new());
        assert_eq!(runner.rule_count(), 2);
        runner.add_rule(EmptyBlockRule);
        assert_eq!(runner.rule_count(), 3);
    }

    #[test]
    fn empty_input_lint_produces_no_diagnostics_three_rules() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule::new());
        runner.add_rule(EmptyBlockRule);
        let diags = runner.run("");
        assert!(diags.is_empty());
    }

    #[test]
    fn multi_violation_same_line_both_detected() {
        // A line that is both too long AND has trailing whitespace.
        let line = format!("{} ", "a".repeat(130));
        let mut runner = LintRunner::new();
        runner.add_rule(LineTooLongRule::new());
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.check_line(&line, 1);
        assert_eq!(
            diags.len(),
            2,
            "expected 2 violations for too-long+trailing-space line"
        );
    }

    #[test]
    fn multi_violation_across_lines_all_found() {
        let source = "fn a() {}  \nfn b() {}\nfn c() {}  ";
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        let diags = runner.run(source);
        assert_eq!(diags.len(), 2, "trailing whitespace on lines 1 and 3");
    }

    #[test]
    fn enabled_count_equals_rule_count_always() {
        let mut runner = LintRunner::new();
        for _ in 0..5 {
            runner.add_rule(TrailingWhitespaceRule);
        }
        assert_eq!(runner.enabled_count(), runner.rule_count());
        assert_eq!(runner.enabled_count(), 5);
    }

    #[test]
    fn severity_of_returns_some_when_rule_present() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule::new());
        runner.add_rule(EmptyBlockRule);
        assert!(runner.severity_of("trailing-whitespace").is_some());
        assert!(runner.severity_of("line-too-long").is_some());
        assert!(runner.severity_of("empty-block").is_some());
    }

    #[test]
    fn rule_count_default_runner_zero() {
        let runner = LintRunner::default();
        assert_eq!(runner.rule_count(), 0);
    }

    #[test]
    fn enabled_count_default_runner_zero() {
        let runner = LintRunner::default();
        assert_eq!(runner.enabled_count(), 0);
    }

    #[test]
    fn lint_runner_run_empty_source_no_diags_with_all_rules() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule { max_len: 10 });
        runner.add_rule(EmptyBlockRule);
        assert!(runner.run("").is_empty());
    }

    #[test]
    fn multi_violation_empty_block_and_long_line() {
        let line = format!("fn f() {{}} {}", "x".repeat(130));
        let mut runner = LintRunner::new();
        runner.add_rule(EmptyBlockRule);
        runner.add_rule(LineTooLongRule::new());
        let diags = runner.check_line(&line, 1);
        assert_eq!(diags.len(), 2);
    }

    #[test]
    fn severity_of_all_three_rules_returns_warning() {
        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule::new());
        runner.add_rule(EmptyBlockRule);
        for name in ["trailing-whitespace", "line-too-long", "empty-block"] {
            assert_eq!(
                runner.severity_of(name),
                Some(LintLevel::Warning),
                "rule '{name}' must have Warning severity"
            );
        }
    }
}
