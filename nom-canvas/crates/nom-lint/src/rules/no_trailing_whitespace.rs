#![deny(unsafe_code)]

use crate::diagnostic::{Diagnostic, Fix, Severity};
use crate::rule_trait::private::RuleInternal;
use crate::rule_trait::RuleResult;
use crate::span::Span;

/// Flags lines that end with trailing whitespace (spaces or tabs) before a
/// newline or end-of-file. Emits a `Warning` with a `Fix` that removes the
/// trailing characters.
pub struct NoTrailingWhitespace;

impl NoTrailingWhitespace {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoTrailingWhitespace {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleInternal for NoTrailingWhitespace {
    fn name(&self) -> &'static str {
        "no-trailing-whitespace"
    }

    fn check(&self, source: &str) -> RuleResult {
        let mut diags = Vec::new();
        // Track byte offset of the current line start.
        let mut line_start: u32 = 0;

        for line in source.lines() {
            let trimmed = line.trim_end_matches(|c| c == ' ' || c == '\t');
            let trimmed_len = trimmed.len() as u32;
            let line_len = line.len() as u32;

            if trimmed_len < line_len {
                let ws_start = line_start + trimmed_len;
                let ws_end = line_start + line_len;
                diags.push(Diagnostic {
                    span: Span::new(ws_start, ws_end),
                    severity: Severity::Warning,
                    code: "no-trailing-whitespace",
                    message: "trailing whitespace".to_string(),
                    fix: Some(Fix {
                        span: Span::new(ws_start, ws_end),
                        replacement: String::new(),
                    }),
                });
            }

            // Advance past line bytes + newline character(s).
            // `str::lines()` strips `\n`, `\r\n`, and `\r`; we need to account
            // for the actual separator consumed from `source`.
            let consumed_start = line_start as usize;
            let after_line = consumed_start + line.len();
            let sep_len = if source.as_bytes().get(after_line) == Some(&b'\r') {
                // Could be \r\n or bare \r
                if source.as_bytes().get(after_line + 1) == Some(&b'\n') {
                    2
                } else {
                    1
                }
            } else if source.as_bytes().get(after_line) == Some(&b'\n') {
                1
            } else {
                0 // end-of-file, no trailing newline
            };
            line_start += line_len + sep_len;
        }

        Ok(diags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule_trait::private::RuleInternal;

    #[test]
    fn trailing_space_flagged() {
        let rule = NoTrailingWhitespace::new();
        let result = rule.check("hello   \nworld\n").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].code, "no-trailing-whitespace");
        assert_eq!(result[0].severity, Severity::Warning);
        // span covers the three trailing spaces (bytes 5..8)
        assert_eq!(result[0].span, Span::new(5, 8));
    }

    #[test]
    fn trailing_tab_flagged() {
        let rule = NoTrailingWhitespace::new();
        let result = rule.check("line\t\nclean\n").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].span, Span::new(4, 5));
    }

    #[test]
    fn no_trailing_empty_vec() {
        let rule = NoTrailingWhitespace::new();
        let result = rule.check("clean\nlines\n").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn whitespace_only_line_flagged() {
        let rule = NoTrailingWhitespace::new();
        // A line containing only spaces is entirely trailing whitespace.
        let result = rule.check("   \n").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].span, Span::new(0, 3));
        // Fix replacement is empty string.
        assert_eq!(result[0].fix.as_ref().unwrap().replacement, "");
    }

    #[test]
    fn crlf_handled() {
        let rule = NoTrailingWhitespace::new();
        // "abc  \r\ndef\r\n" — trailing spaces before \r\n on first line.
        let result = rule.check("abc  \r\ndef\r\n").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].span, Span::new(3, 5));
    }
}
