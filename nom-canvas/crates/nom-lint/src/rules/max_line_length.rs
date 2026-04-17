#![deny(unsafe_code)]

use crate::diagnostic::{Diagnostic, Severity};
use crate::rule_trait::private::RuleInternal;
use crate::rule_trait::RuleResult;
use crate::span::Span;

/// Flags any line whose Unicode scalar count exceeds `max_cols`.
/// No auto-fix is provided because line wrapping requires semantic judgment.
pub struct MaxLineLength {
    max_cols: u32,
}

impl MaxLineLength {
    pub fn new(max_cols: u32) -> Self {
        Self { max_cols }
    }
}

impl RuleInternal for MaxLineLength {
    fn name(&self) -> &'static str {
        "max-line-length"
    }

    fn check(&self, source: &str) -> RuleResult {
        let mut diags = Vec::new();
        let mut line_start: u32 = 0;

        for line in source.lines() {
            let char_count = line.chars().count() as u32;

            if char_count > self.max_cols {
                // Span covers the entire line (byte-based, matching Span semantics).
                let line_byte_len = line.len() as u32;
                diags.push(Diagnostic {
                    span: Span::new(line_start, line_start + line_byte_len),
                    severity: Severity::Warning,
                    code: "max-line-length",
                    message: format!(
                        "line is {} chars, exceeds max of {}",
                        char_count, self.max_cols
                    ),
                    fix: None,
                });
            }

            // Advance past line bytes + newline separator.
            let after_line = line_start as usize + line.len();
            let sep_len = if source.as_bytes().get(after_line) == Some(&b'\r') {
                if source.as_bytes().get(after_line + 1) == Some(&b'\n') {
                    2
                } else {
                    1
                }
            } else if source.as_bytes().get(after_line) == Some(&b'\n') {
                1
            } else {
                0
            };
            line_start += line.len() as u32 + sep_len;
        }

        Ok(diags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule_trait::private::RuleInternal;

    #[test]
    fn line_exceeds_80_flagged() {
        let rule = MaxLineLength::new(80);
        // 81 'a' characters
        let long_line: String = "a".repeat(81);
        let result = rule.check(&long_line).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].code, "max-line-length");
        assert_eq!(result[0].severity, Severity::Warning);
        assert!(result[0].fix.is_none());
    }

    #[test]
    fn line_exactly_at_max_ok() {
        let rule = MaxLineLength::new(80);
        let exact_line: String = "b".repeat(80);
        let result = rule.check(&exact_line).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn multi_line_only_long_flagged() {
        let rule = MaxLineLength::new(10);
        let source = "short\nthis line is definitely longer than ten chars\nok\n";
        let result = rule.check(source).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].span.start, 6); // byte offset after "short\n"
    }

    #[test]
    fn empty_file_empty_vec() {
        let rule = MaxLineLength::new(80);
        let result = rule.check("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn utf8_multibyte_chars_counted_by_char() {
        let rule = MaxLineLength::new(3);
        // "éàü" = 3 Unicode scalars but 6 bytes. Should NOT be flagged (3 == max).
        let result = rule.check("éàü").unwrap();
        assert!(result.is_empty(), "3 chars == max should not be flagged");

        // "éàüx" = 4 Unicode scalars > max=3, should be flagged.
        let result2 = rule.check("éàüx").unwrap();
        assert_eq!(result2.len(), 1);
    }
}
