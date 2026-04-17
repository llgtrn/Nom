#![deny(unsafe_code)]

/// Severity level for a lint diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LintLevel {
    Error,
    Warning,
    Info,
}

/// A single lint finding produced by a rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintDiagnostic {
    pub message: String,
    pub span_start: usize,
    pub span_end: usize,
    pub level: LintLevel,
}

/// A lint rule that inspects source text and returns diagnostics.
pub trait LintRule {
    fn check(&self, source: &str) -> Vec<LintDiagnostic>;
}

// ---------------------------------------------------------------------------
// Concrete rules
// ---------------------------------------------------------------------------

/// Flags lines that end with one or more space or tab characters.
pub struct TrailingWhitespaceRule;

impl LintRule for TrailingWhitespaceRule {
    fn check(&self, source: &str) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        let mut offset = 0usize;

        for line in source.split('\n') {
            let line_end = offset + line.len();

            // Detect trailing whitespace (spaces or tabs before end of line).
            let trimmed_len = line.trim_end_matches(|c| c == ' ' || c == '\t').len();
            if trimmed_len < line.len() {
                let span_start = offset + trimmed_len;
                diagnostics.push(LintDiagnostic {
                    message: "trailing whitespace".to_string(),
                    span_start,
                    span_end: line_end,
                    level: LintLevel::Warning,
                });
            }

            // +1 for the '\n' separator (except possibly the last line).
            offset = line_end + 1;
        }

        diagnostics
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

impl LintRule for LineTooLongRule {
    fn check(&self, source: &str) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        let mut offset = 0usize;

        for line in source.split('\n') {
            let line_len = line.len();
            if line_len > self.max_len {
                diagnostics.push(LintDiagnostic {
                    message: format!(
                        "line is {} characters, exceeds maximum of {}",
                        line_len, self.max_len
                    ),
                    span_start: offset,
                    span_end: offset + line_len,
                    level: LintLevel::Warning,
                });
            }
            offset += line_len + 1;
        }

        diagnostics
    }
}

/// Flags occurrences of `{}` — braces with nothing between them.
pub struct EmptyBlockRule;

impl LintRule for EmptyBlockRule {
    fn check(&self, source: &str) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        let bytes = source.as_bytes();
        let mut i = 0usize;

        while i + 1 < bytes.len() {
            if bytes[i] == b'{' && bytes[i + 1] == b'}' {
                diagnostics.push(LintDiagnostic {
                    message: "empty block `{}`".to_string(),
                    span_start: i,
                    span_end: i + 2,
                    level: LintLevel::Warning,
                });
                i += 2;
            } else {
                i += 1;
            }
        }

        diagnostics
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

    /// Run all registered rules against `source` and return the combined diagnostics.
    pub fn run(&self, source: &str) -> Vec<LintDiagnostic> {
        self.rules
            .iter()
            .flat_map(|rule| rule.check(source))
            .collect()
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

    #[test]
    fn trailing_whitespace_detected() {
        let source = "fn foo() {   \n    let x = 1;\n}";
        let diags = TrailingWhitespaceRule.check(source);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].level, LintLevel::Warning);
        assert!(diags[0].message.contains("trailing whitespace"));
        // span_start should point to the first trailing space on line 0.
        // Line 0 is "fn foo() {   " — 13 chars; trailing spaces start at index 10.
        assert_eq!(diags[0].span_start, 10);
        assert_eq!(diags[0].span_end, 13);
    }

    #[test]
    fn line_too_long_detected() {
        let long_line = "x".repeat(130);
        let source = format!("short line\n{}\nanother short line", long_line);
        let rule = LineTooLongRule { max_len: 120 };
        let diags = rule.check(&source);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].level, LintLevel::Warning);
        assert!(diags[0].message.contains("130"));
        assert!(diags[0].message.contains("120"));
    }

    #[test]
    fn empty_block_detected() {
        let source = "fn foo() {}\nfn bar() {\n    // not empty\n}";
        let diags = EmptyBlockRule.check(source);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].level, LintLevel::Warning);
        assert!(diags[0].message.contains("empty block"));
        assert_eq!(diags[0].span_start, 9);
        assert_eq!(diags[0].span_end, 11);
    }

    #[test]
    fn lint_runner_combines_rules() {
        // Source has: trailing whitespace on line 0, a long line, and an empty block.
        let long_line = "y".repeat(130);
        let source = format!("let x = 1;   \n{}\nfn empty() {{}}", long_line);

        let mut runner = LintRunner::new();
        runner.add_rule(TrailingWhitespaceRule);
        runner.add_rule(LineTooLongRule { max_len: 120 });
        runner.add_rule(EmptyBlockRule);

        let diags = runner.run(&source);
        // Expect at least one diagnostic from each rule.
        assert!(diags.iter().any(|d| d.message.contains("trailing whitespace")));
        assert!(diags.iter().any(|d| d.message.contains("130")));
        assert!(diags.iter().any(|d| d.message.contains("empty block")));
    }
}
