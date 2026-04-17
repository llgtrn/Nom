use crate::span::Span;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fix {
    pub span: Span,
    pub replacement: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub span: Span,
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    pub fix: Option<Fix>,
}

#[derive(Debug, thiserror::Error)]
pub enum DiagnosticError {
    #[error("overlapping fixes: first {first:?} second {second:?}")]
    OverlappingFixes { first: Span, second: Span },
}

/// Apply a slice of fixes to source. Fixes are sorted by span.start descending
/// so back-to-front application keeps offsets stable. Overlapping fixes
/// return an error.
pub fn apply_fixes(source: &str, fixes: &[Fix]) -> Result<String, DiagnosticError> {
    if fixes.is_empty() {
        return Ok(source.to_string());
    }

    // Sort descending by start
    let mut sorted: Vec<&Fix> = fixes.iter().collect();
    sorted.sort_by(|a, b| b.span.start.cmp(&a.span.start));

    // Check overlaps: after sorting descending, each pair (window[0], window[1])
    // has window[0].start >= window[1].start. They overlap if window[0].start < window[1].end.
    for window in sorted.windows(2) {
        let later = window[0];   // higher start
        let earlier = window[1]; // lower start
        if later.span.start < earlier.span.end {
            return Err(DiagnosticError::OverlappingFixes {
                first: earlier.span,
                second: later.span,
            });
        }
    }

    let mut result = source.to_string();
    for fix in &sorted {
        let start = fix.span.start as usize;
        let end = fix.span.end as usize;
        result.replace_range(start..end, &fix.replacement);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_fix() {
        let source = "hello world";
        let fix = Fix { span: Span::new(6, 11), replacement: "Nom".to_string() };
        let result = apply_fixes(source, &[fix]).unwrap();
        assert_eq!(result, "hello Nom");
    }

    #[test]
    fn multi_non_overlapping_fixes() {
        let source = "abcdef";
        let fixes = vec![
            Fix { span: Span::new(0, 1), replacement: "X".to_string() },
            Fix { span: Span::new(4, 6), replacement: "YZ".to_string() },
        ];
        let result = apply_fixes(source, &fixes).unwrap();
        assert_eq!(result, "XbcdYZ");
    }

    #[test]
    fn detect_overlap_error() {
        let source = "hello world";
        let fixes = vec![
            Fix { span: Span::new(0, 5), replacement: "A".to_string() },
            Fix { span: Span::new(3, 8), replacement: "B".to_string() },
        ];
        let result = apply_fixes(source, &fixes);
        assert!(matches!(result, Err(DiagnosticError::OverlappingFixes { .. })));
    }

    #[test]
    fn fix_at_eof() {
        let source = "hello";
        let fix = Fix { span: Span::new(5, 5), replacement: " world".to_string() };
        let result = apply_fixes(source, &[fix]).unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn empty_fixes() {
        let source = "unchanged";
        let result = apply_fixes(source, &[]).unwrap();
        assert_eq!(result, "unchanged");
    }

    #[test]
    fn adjacent_fixes_not_overlapping() {
        let source = "abcd";
        let fixes = vec![
            Fix { span: Span::new(0, 2), replacement: "XX".to_string() },
            Fix { span: Span::new(2, 4), replacement: "YY".to_string() },
        ];
        let result = apply_fixes(source, &fixes).unwrap();
        assert_eq!(result, "XXYY");
    }
}
