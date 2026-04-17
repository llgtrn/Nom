#![deny(unsafe_code)]

use crate::diagnostic::{Diagnostic, Severity};
use crate::rule_trait::private::RuleInternal;
use crate::rule_trait::RuleResult;
use crate::span::Span;

/// Detects mixed indentation: a line that starts with one or more spaces
/// followed by a tab character. Emits a `Warning` per offending line; no
/// automatic fix is offered because indent style is a project-level choice.
pub struct NoTabIndentAfterSpace;

impl NoTabIndentAfterSpace {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoTabIndentAfterSpace {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleInternal for NoTabIndentAfterSpace {
    fn name(&self) -> &'static str {
        "no-tab-after-space"
    }

    fn check(&self, source: &str) -> RuleResult {
        let mut diags = Vec::new();
        let mut line_start: u32 = 0;

        for line in source.lines() {
            // Find the first non-whitespace character position.
            let indent_end = line
                .find(|c: char| c != ' ' && c != '\t')
                .unwrap_or(line.len());
            let indent = &line[..indent_end];

            // Violation: indent contains a space followed (anywhere) by a tab.
            // We scan left-to-right; a space followed by a tab anywhere in the
            // leading whitespace is enough.
            let mut saw_space = false;
            let mut tab_after_space: Option<usize> = None;
            for (i, ch) in indent.char_indices() {
                match ch {
                    ' ' => saw_space = true,
                    '\t' if saw_space => {
                        tab_after_space = Some(i);
                        break;
                    }
                    _ => {}
                }
            }

            if let Some(tab_offset) = tab_after_space {
                let abs_offset = line_start + tab_offset as u32;
                diags.push(Diagnostic {
                    span: Span::new(abs_offset, abs_offset + 1),
                    severity: Severity::Warning,
                    code: "no-tab-after-space",
                    message: "tab character in indentation after space".to_string(),
                    fix: None,
                });
            }

            // Advance line_start past the line bytes + separator.
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
    fn clean_space_indent_no_diag() {
        let rule = NoTabIndentAfterSpace::new();
        let result = rule.check("    clean\n    also_clean\n").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn clean_tab_indent_no_diag() {
        let rule = NoTabIndentAfterSpace::new();
        let result = rule.check("\tclean\n\t\talso_clean\n").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn space_then_tab_flagged() {
        let rule = NoTabIndentAfterSpace::new();
        // "  \tfoo\n" — two spaces then a tab.
        let result = rule.check("  \tfoo\n").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].code, "no-tab-after-space");
        assert_eq!(result[0].severity, Severity::Warning);
        // The tab is at byte offset 2.
        assert_eq!(result[0].span, Span::new(2, 3));
        // No fix provided.
        assert!(result[0].fix.is_none());
    }

    #[test]
    fn only_second_line_flagged() {
        let rule = NoTabIndentAfterSpace::new();
        let src = "    good\n  \tbad\n    good_again\n";
        let result = rule.check(src).unwrap();
        assert_eq!(result.len(), 1);
        // "    good\n" = 9 bytes; bad line tab at offset 2 → absolute 11.
        assert_eq!(result[0].span, Span::new(11, 12));
    }

    #[test]
    fn multiple_offending_lines() {
        let rule = NoTabIndentAfterSpace::new();
        let src = " \ta\n \tb\n";
        let result = rule.check(src).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn crlf_line_endings_handled() {
        let rule = NoTabIndentAfterSpace::new();
        // "  \tfoo\r\n" — CRLF line ending.
        let result = rule.check("  \tfoo\r\n").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].span, Span::new(2, 3));
    }

    #[test]
    fn empty_source_no_diag() {
        let rule = NoTabIndentAfterSpace::new();
        let result = rule.check("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn tab_before_space_is_fine() {
        // "\t  foo" — tab then spaces is acceptable per this rule.
        let rule = NoTabIndentAfterSpace::new();
        let result = rule.check("\t  foo\n").unwrap();
        assert!(result.is_empty());
    }
}
