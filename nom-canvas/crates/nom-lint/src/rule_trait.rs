/// Public sealed trait. External crates cannot implement this because
/// [`private::RuleInternal`] is not accessible outside this crate.
#[allow(private_bounds)]
pub trait Rule: private::RuleInternal {}

pub(crate) mod private {
    /// The real trait all linters implement. Kept private so external crates
    /// cannot provide their own implementations (sealed-trait pattern).
    pub trait RuleInternal: Send + Sync {
        fn name(&self) -> &'static str;
        fn check(&self, source: &str) -> crate::RuleResult;
    }
}

/// Blanket impl: every `RuleInternal` is automatically a `Rule`.
impl<T: private::RuleInternal> Rule for T {}

pub type RuleResult = Result<Vec<crate::diagnostic::Diagnostic>, crate::diagnostic::DiagnosticError>;

#[cfg(test)]
mod tests {
    use super::*;
    use super::private::RuleInternal;
    use crate::diagnostic::{Diagnostic, Severity};
    use crate::span::Span;

    struct TrailingWhitespace;

    impl RuleInternal for TrailingWhitespace {
        fn name(&self) -> &'static str {
            "trailing-whitespace"
        }

        fn check(&self, source: &str) -> crate::RuleResult {
            let mut diags = Vec::new();
            let mut offset = 0u32;
            for line in source.lines() {
                let trimmed = line.trim_end();
                if trimmed.len() < line.len() {
                    let start = offset + trimmed.len() as u32;
                    let end = offset + line.len() as u32;
                    diags.push(Diagnostic {
                        span: Span::new(start, end),
                        severity: Severity::Warning,
                        code: "L001",
                        message: "trailing whitespace".to_string(),
                        fix: None,
                    });
                }
                offset += line.len() as u32 + 1; // +1 for newline
            }
            Ok(diags)
        }
    }

    #[test]
    fn builtin_rule_emits_diagnostic() {
        let rule = TrailingWhitespace;
        let result = rule.check("hello   \nworld\n").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].code, "L001");
    }

    #[test]
    fn trait_object_usable() {
        let rule: Box<dyn Rule> = Box::new(TrailingWhitespace);
        let result = rule.check("clean line\n").unwrap();
        assert!(result.is_empty());
    }
}
