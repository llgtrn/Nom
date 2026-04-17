#![deny(unsafe_code)]

use crate::diagnostic::{Diagnostic, Fix, Severity};
use crate::rule_trait::private::RuleInternal;
use crate::rule_trait::RuleResult;
use crate::span::Span;

/// Flags every blank line beyond the first in a consecutive run of blank lines.
/// A "blank line" is a line whose content is entirely whitespace (or empty).
/// Emits `Info` with a `Fix` that deletes the extra blank line (including its
/// trailing newline separator).
pub struct NoDoubleBlankLines;

impl NoDoubleBlankLines {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoDoubleBlankLines {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleInternal for NoDoubleBlankLines {
    fn name(&self) -> &'static str {
        "no-double-blank-lines"
    }

    fn check(&self, source: &str) -> RuleResult {
        let mut diags = Vec::new();

        // Build a list of (line_content, line_byte_start, sep_len) tuples.
        // We need the byte start so we can construct spans that include the
        // newline separator (so the fix deletes the whole blank line).
        let mut entries: Vec<(&str, u32, u32)> = Vec::new();
        let mut offset: u32 = 0;

        for line in source.lines() {
            let line_len = line.len() as u32;
            let after = (offset + line_len) as usize;
            let sep_len: u32 = if source.as_bytes().get(after) == Some(&b'\r') {
                if source.as_bytes().get(after + 1) == Some(&b'\n') {
                    2
                } else {
                    1
                }
            } else if source.as_bytes().get(after) == Some(&b'\n') {
                1
            } else {
                0
            };
            entries.push((line, offset, sep_len));
            offset += line_len + sep_len;
        }

        // Walk entries tracking consecutive blank count.
        let mut consecutive_blanks: u32 = 0;

        for (line, line_start, sep_len) in &entries {
            let is_blank = line.trim().is_empty();
            if is_blank {
                consecutive_blanks += 1;
                if consecutive_blanks >= 2 {
                    // This is an extra blank line; flag it.
                    let line_len = line.len() as u32;
                    // Span covers line bytes + separator so the fix deletes the
                    // entire line including its newline.
                    let span_end = line_start + line_len + sep_len;
                    diags.push(Diagnostic {
                        span: Span::new(*line_start, span_end),
                        severity: Severity::Info,
                        code: "no-double-blank",
                        message: "consecutive blank lines".to_string(),
                        fix: Some(Fix {
                            span: Span::new(*line_start, span_end),
                            replacement: String::new(),
                        }),
                    });
                }
            } else {
                consecutive_blanks = 0;
            }
        }

        Ok(diags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule_trait::private::RuleInternal;

    #[test]
    fn two_consecutive_blanks_flagged() {
        let rule = NoDoubleBlankLines::new();
        // "a\n\n\nb" — two blank lines after "a"; the second blank is extra.
        let result = rule.check("a\n\n\nb").unwrap();
        assert_eq!(result.len(), 1, "only the 2nd blank should be flagged");
        assert_eq!(result[0].code, "no-double-blank");
        assert_eq!(result[0].severity, Severity::Info);
    }

    #[test]
    fn three_blanks_flags_second_and_third() {
        let rule = NoDoubleBlankLines::new();
        // "a\n\n\n\nb" — three blanks; the 2nd and 3rd are extra.
        let result = rule.check("a\n\n\n\nb").unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn single_blank_ok() {
        let rule = NoDoubleBlankLines::new();
        let result = rule.check("a\n\nb\n").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn crlf_handled() {
        let rule = NoDoubleBlankLines::new();
        // "a\r\n\r\n\r\nb" — two CRLF blank lines after "a".
        let result = rule.check("a\r\n\r\n\r\nb").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].code, "no-double-blank");
    }

    #[test]
    fn empty_file_ok() {
        let rule = NoDoubleBlankLines::new();
        let result = rule.check("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn fix_replacement_is_empty_string() {
        let rule = NoDoubleBlankLines::new();
        let result = rule.check("x\n\n\ny\n").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].fix.as_ref().unwrap().replacement, "");
    }
}
