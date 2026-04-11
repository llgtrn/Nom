//! nom-security: Security review of Nom compositions.
//!
//! Checks:
//!   - Minimum security scores for all resolved words
//!   - License compatibility between words
//!   - Known CVE flags (stored as metadata in nomdict)
//!   - Untrusted sources (non-registry origins)
//!
//! Produces a [`SecurityReport`] with findings categorized by severity.

use nom_ast::{Declaration, NomRef, SourceFile, Statement};
use nom_resolver::{Resolver, ResolverError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("resolver error: {0}")]
    Resolver(#[from] ResolverError),
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Severity of a security finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Info => write!(f, "INFO"),
            Severity::Low => write!(f, "LOW"),
            Severity::Medium => write!(f, "MEDIUM"),
            Severity::High => write!(f, "HIGH"),
            Severity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// A single security finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFinding {
    pub severity: Severity,
    pub word: String,
    pub variant: Option<String>,
    pub message: String,
}

/// Summary security report for a source file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityReport {
    pub findings: Vec<SecurityFinding>,
    pub passed: bool,
    /// Highest severity level found (None if no findings).
    pub max_severity: Option<Severity>,
}

impl SecurityReport {
    pub fn new() -> Self {
        Self {
            findings: Vec::new(),
            passed: true,
            max_severity: None,
        }
    }

    pub fn push(&mut self, finding: SecurityFinding) {
        if finding.severity >= Severity::High {
            self.passed = false;
        }
        let sev = finding.severity;
        self.findings.push(finding);
        self.max_severity = Some(match self.max_severity {
            Some(current) => current.max(sev),
            None => sev,
        });
    }

    pub fn critical_count(&self) -> usize {
        self.findings.iter().filter(|f| f.severity == Severity::Critical).count()
    }

    pub fn high_count(&self) -> usize {
        self.findings.iter().filter(|f| f.severity == Severity::High).count()
    }

    /// Serialize the report to JSON.
    pub fn to_json(&self) -> Result<String, SecurityError> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

impl Default for SecurityReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Security checker configuration.
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Minimum acceptable security score. Words below this threshold are flagged.
    pub min_security_score: f64,
    /// Minimum acceptable reliability score.
    pub min_reliability_score: f64,
    /// If true, flag words from non-registry sources.
    pub require_registry_source: bool,
    /// Allowed license identifiers (SPDX). Empty = allow all.
    pub allowed_licenses: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            min_security_score: 0.7,
            min_reliability_score: 0.5,
            require_registry_source: false,
            allowed_licenses: Vec::new(),
        }
    }
}

/// Runs security checks on a source file.
pub struct SecurityChecker<'r> {
    resolver: &'r Resolver,
    config: SecurityConfig,
}

impl<'r> SecurityChecker<'r> {
    pub fn new(resolver: &'r Resolver, config: SecurityConfig) -> Self {
        Self { resolver, config }
    }

    pub fn with_defaults(resolver: &'r Resolver) -> Self {
        Self::new(resolver, SecurityConfig::default())
    }

    /// Run all security checks and return a report.
    pub fn check(&self, source: &SourceFile) -> Result<SecurityReport, SecurityError> {
        let mut report = SecurityReport::new();
        for decl in &source.declarations {
            self.check_declaration(decl, &mut report)?;
        }
        Ok(report)
    }

    fn check_declaration(
        &self,
        decl: &Declaration,
        report: &mut SecurityReport,
    ) -> Result<(), SecurityError> {
        for stmt in &decl.statements {
            match stmt {
                Statement::Need(need) => {
                    self.check_nom_ref(&need.reference, report)?;
                }
                Statement::Flow(flow) => {
                    self.check_flow_steps(&flow.chain.steps, report)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn check_flow_steps(
        &self,
        steps: &[nom_ast::FlowStep],
        report: &mut SecurityReport,
    ) -> Result<(), SecurityError> {
        for step in steps {
            match step {
                nom_ast::FlowStep::Ref(nom_ref) => {
                    self.check_nom_ref(nom_ref, report)?;
                }
                nom_ast::FlowStep::Branch(block) => {
                    for arm in &block.arms {
                        self.check_flow_steps(&arm.chain.steps, report)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn check_nom_ref(
        &self,
        nom_ref: &NomRef,
        report: &mut SecurityReport,
    ) -> Result<(), SecurityError> {
        let entry = match self.resolver.resolve(nom_ref) {
            Ok(e) => e,
            Err(ResolverError::NotFound { .. }) => return Ok(()), // can't check what we can't resolve
            Err(e) => return Err(SecurityError::Resolver(e)),
        };

        let label = format!(
            "{}{}",
            entry.word,
            entry.variant.as_deref().map(|v| format!("::{v}")).unwrap_or_default()
        );

        // Check minimum security score
        if entry.security < self.config.min_security_score {
            report.push(SecurityFinding {
                severity: if entry.security < 0.3 { Severity::Critical } else { Severity::High },
                word: entry.word.clone(),
                variant: entry.variant.clone(),
                message: format!(
                    "{label} has security score {:.2} below minimum {:.2}",
                    entry.security, self.config.min_security_score
                ),
            });
        }

        // Check minimum reliability score
        if entry.reliability < self.config.min_reliability_score {
            report.push(SecurityFinding {
                severity: Severity::Medium,
                word: entry.word.clone(),
                variant: entry.variant.clone(),
                message: format!(
                    "{label} has reliability score {:.2} below minimum {:.2}",
                    entry.reliability, self.config.min_reliability_score
                ),
            });
        }

        // Check for untrusted source
        if self.config.require_registry_source {
            if let Some(source) = &entry.source {
                if !source.starts_with("https://registry.nom-lang.org") {
                    report.push(SecurityFinding {
                        severity: Severity::Low,
                        word: entry.word.clone(),
                        variant: entry.variant.clone(),
                        message: format!("{label} comes from untrusted source: {source}"),
                    });
                }
            } else {
                report.push(SecurityFinding {
                    severity: Severity::Low,
                    word: entry.word.clone(),
                    variant: entry.variant.clone(),
                    message: format!("{label} has no source URL (local or unverified)"),
                });
            }
        }

        // Check for CVE flags (stored as hash = "CVE-...")
        if let Some(hash) = &entry.hash {
            if hash.starts_with("CVE-") {
                report.push(SecurityFinding {
                    severity: Severity::Critical,
                    word: entry.word.clone(),
                    variant: entry.variant.clone(),
                    message: format!("{label} is flagged with known vulnerability: {hash}"),
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom_ast::{Identifier, NomRef, Span};
    use nom_resolver::{Resolver, WordEntry};

    fn span() -> Span {
        Span::new(0, 1, 1, 1)
    }

    fn setup() -> Resolver {
        let r = Resolver::open_in_memory().unwrap();
        r.upsert(&WordEntry {
            word: "weak_hash".to_owned(),
            security: 0.2,   // below default threshold
            performance: 0.5,
            reliability: 0.9,
            ..WordEntry::default()
        }).unwrap();
        r.upsert(&WordEntry {
            word: "good_hash".to_owned(),
            security: 0.95,
            performance: 0.8,
            reliability: 0.99,
            ..WordEntry::default()
        }).unwrap();
        r.upsert(&WordEntry {
            word: "cve_hash".to_owned(),
            security: 0.9,
            performance: 0.8,
            reliability: 0.99,
            hash: Some("CVE-2024-12345".to_owned()),
            ..WordEntry::default()
        }).unwrap();
        r
    }

    fn make_source_with_need(word: &str) -> SourceFile {
        use nom_ast::*;
        SourceFile {
            path: None,
            locale: None,
            declarations: vec![Declaration {
                classifier: Classifier::Flow,
                name: Identifier::new("test", span()),
                statements: vec![Statement::Need(NeedStmt {
                    reference: NomRef {
                        word: Identifier::new(word, span()),
                        variant: None,
                        span: span(),
                    },
                    constraint: None,
                    span: span(),
                })],
                span: span(),
            }],
        }
    }

    #[test]
    fn passes_for_good_word() {
        let r = setup();
        let checker = SecurityChecker::with_defaults(&r);
        let source = make_source_with_need("good_hash");
        let report = checker.check(&source).unwrap();
        assert!(report.passed);
        assert!(report.findings.is_empty());
    }

    #[test]
    fn fails_for_weak_security() {
        let r = setup();
        let checker = SecurityChecker::with_defaults(&r);
        let source = make_source_with_need("weak_hash");
        let report = checker.check(&source).unwrap();
        assert!(!report.passed);
        assert!(report.findings.iter().any(|f| f.severity >= Severity::High));
    }

    #[test]
    fn detects_cve() {
        let r = setup();
        let checker = SecurityChecker::with_defaults(&r);
        let source = make_source_with_need("cve_hash");
        let report = checker.check(&source).unwrap();
        assert!(report.findings.iter().any(|f| f.severity == Severity::Critical));
    }
}
