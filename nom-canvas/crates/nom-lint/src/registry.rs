use crate::diagnostic::Diagnostic;
use crate::rule_trait::Rule;

pub struct LintRegistry {
    linters: Vec<Box<dyn Rule>>,
}

impl LintRegistry {
    pub fn new() -> Self {
        Self { linters: Vec::new() }
    }

    pub fn add_linter<R: Rule + 'static>(&mut self, rule: R) {
        self.linters.push(Box::new(rule));
    }

    /// Run all registered rules against `source`. Tolerates per-rule errors
    /// by logging them to stderr and skipping that rule's diagnostics.
    pub fn run(&self, source: &str) -> Vec<Diagnostic> {
        let mut all = Vec::new();
        for linter in &self.linters {
            match linter.check(source) {
                Ok(diags) => all.extend(diags),
                Err(e) => eprintln!("nom-lint: rule error skipped: {e}"),
            }
        }
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{Diagnostic, Severity};
    use crate::rule_trait::private::RuleInternal;
    use crate::span::Span;

    struct AlwaysWarn;
    impl RuleInternal for AlwaysWarn {
        fn name(&self) -> &'static str { "always-warn" }
        fn check(&self, _source: &str) -> crate::RuleResult {
            Ok(vec![Diagnostic {
                span: Span::new(0, 0),
                severity: Severity::Warning,
                code: "T001",
                message: "always warns".to_string(),
                fix: None,
            }])
        }
    }

    struct NeverWarn;
    impl RuleInternal for NeverWarn {
        fn name(&self) -> &'static str { "never-warn" }
        fn check(&self, _source: &str) -> crate::RuleResult { Ok(vec![]) }
    }

    #[test]
    fn empty_registry_returns_empty() {
        let registry = LintRegistry::new();
        assert!(registry.run("anything").is_empty());
    }

    #[test]
    fn one_rule_adds_diagnostics() {
        let mut registry = LintRegistry::new();
        registry.add_linter(AlwaysWarn);
        let diags = registry.run("source");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "T001");
    }

    #[test]
    fn multiple_rules_compose() {
        let mut registry = LintRegistry::new();
        registry.add_linter(AlwaysWarn);
        registry.add_linter(NeverWarn);
        registry.add_linter(AlwaysWarn);
        let diags = registry.run("source");
        assert_eq!(diags.len(), 2);
    }
}
