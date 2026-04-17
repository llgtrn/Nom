#![deny(unsafe_code)]

use crate::diagnostic::{Diagnostic, Fix, Severity};
use crate::rule_trait::private::RuleInternal;
use crate::rule_trait::RuleResult;
use crate::span::Span;

/// Requires that non-empty source files end with a newline (`\n`). Emits a
/// `Hint` with a `Fix` that appends `\n` at the end of the file.
pub struct RequireTrailingNewline;

impl RequireTrailingNewline {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RequireTrailingNewline {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleInternal for RequireTrailingNewline {
    fn name(&self) -> &'static str {
        "require-trailing-newline"
    }

    fn check(&self, source: &str) -> RuleResult {
        if source.is_empty() || source.ends_with('\n') {
            return Ok(Vec::new());
        }

        let end = source.len() as u32;
        Ok(vec![Diagnostic {
            span: Span::new(end, end),
            severity: Severity::Hint,
            code: "require-trailing-newline",
            message: "file does not end with a newline".to_string(),
            fix: Some(Fix {
                span: Span::new(end, end),
                replacement: "\n".to_string(),
            }),
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule_trait::private::RuleInternal;

    #[test]
    fn empty_source_no_diag() {
        let rule = RequireTrailingNewline::new();
        let result = rule.check("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn ends_with_newline_no_diag() {
        let rule = RequireTrailingNewline::new();
        let result = rule.check("hello\nworld\n").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn missing_newline_flagged() {
        let rule = RequireTrailingNewline::new();
        let result = rule.check("hello").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].code, "require-trailing-newline");
        assert_eq!(result[0].severity, Severity::Hint);
        // span is end..end (5..5)
        assert_eq!(result[0].span, Span::new(5, 5));
    }

    #[test]
    fn fix_appends_newline() {
        let rule = RequireTrailingNewline::new();
        let result = rule.check("hello").unwrap();
        let fix = result[0].fix.as_ref().unwrap();
        assert_eq!(fix.replacement, "\n");
        assert_eq!(fix.span, Span::new(5, 5));
    }

    #[test]
    fn multiline_missing_newline() {
        let rule = RequireTrailingNewline::new();
        let src = "line1\nline2\nline3";
        let result = rule.check(src).unwrap();
        assert_eq!(result.len(), 1);
        let end = src.len() as u32;
        assert_eq!(result[0].span, Span::new(end, end));
    }

    #[test]
    fn crlf_ending_is_not_newline() {
        // A file ending with \r without \n should be flagged.
        let rule = RequireTrailingNewline::new();
        let result = rule.check("hello\r").unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn crlf_then_lf_ending_is_fine() {
        let rule = RequireTrailingNewline::new();
        let result = rule.check("hello\r\n").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn single_newline_no_diag() {
        let rule = RequireTrailingNewline::new();
        let result = rule.check("\n").unwrap();
        assert!(result.is_empty());
    }
}
