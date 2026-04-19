/// What kind of target to inspect
pub enum InspectTarget {
    YoutubeChannel { url: String },
    GithubRepo { url: String },
    Website { url: String },
    PersonUsername { username: String },
    CompanyDomain { domain: String },
    VideoFile { path: String },
    ImageFile { path: String },
}

impl InspectTarget {
    pub fn kind_label(&self) -> &str {
        match self {
            Self::YoutubeChannel { .. } => "youtube_channel",
            Self::GithubRepo { .. } => "github_repo",
            Self::Website { .. } => "website",
            Self::PersonUsername { .. } => "person",
            Self::CompanyDomain { .. } => "company",
            Self::VideoFile { .. } => "video",
            Self::ImageFile { .. } => "image",
        }
    }

    pub fn url_or_path(&self) -> &str {
        match self {
            Self::YoutubeChannel { url } => url,
            Self::GithubRepo { url } => url,
            Self::Website { url } => url,
            Self::PersonUsername { username } => username,
            Self::CompanyDomain { domain } => domain,
            Self::VideoFile { path } => path,
            Self::ImageFile { path } => path,
        }
    }
}

/// A detected pattern/component from inspection
#[derive(Debug)]
pub struct InspectFinding {
    pub category: String,
    pub key: String,
    pub value: String,
    pub confidence: f32,
}

impl InspectFinding {
    pub fn new(category: &str, key: &str, value: &str, confidence: f32) -> Self {
        Self {
            category: category.to_string(),
            key: key.to_string(),
            value: value.to_string(),
            confidence,
        }
    }
}

/// Full inspection report
pub struct InspectReport {
    pub target: InspectTarget,
    pub findings: Vec<InspectFinding>,
    pub nomx_entry: String,
    pub inspect_ms: u64,
}

impl InspectReport {
    pub fn new(target: InspectTarget) -> Self {
        Self {
            target,
            findings: Vec::new(),
            nomx_entry: String::new(),
            inspect_ms: 0,
        }
    }

    pub fn add_finding(&mut self, finding: InspectFinding) {
        self.findings.push(finding);
    }

    pub fn finding_count(&self) -> usize {
        self.findings.len()
    }

    pub fn findings_by_category(&self, category: &str) -> Vec<&InspectFinding> {
        self.findings
            .iter()
            .filter(|f| f.category == category)
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.findings.is_empty()
    }
}

/// The inspector — stub implementations for each target type
pub struct NomInspector {
    pub confidence_threshold: f32,
}

impl NomInspector {
    pub fn new(threshold: f32) -> Self {
        Self {
            confidence_threshold: threshold,
        }
    }

    /// Generate stub findings based on target kind
    pub fn inspect(target: InspectTarget) -> InspectReport {
        let mut report = InspectReport::new(target);
        match &report.target {
            InspectTarget::YoutubeChannel { .. } => {
                report.add_finding(InspectFinding::new(
                    "content_strategy",
                    "upload_frequency",
                    "weekly",
                    0.8,
                ));
                report.add_finding(InspectFinding::new(
                    "content_strategy",
                    "avg_duration",
                    "10min",
                    0.7,
                ));
                report.add_finding(InspectFinding::new(
                    "content_strategy",
                    "topics",
                    "tech,ai",
                    0.9,
                ));
                report.nomx_entry =
                    "define channel that uploads(weekly) covers(tech,ai) duration(10min)"
                        .to_string();
            }
            InspectTarget::GithubRepo { .. } => {
                report.add_finding(InspectFinding::new(
                    "architecture",
                    "language",
                    "rust",
                    0.95,
                ));
                report.add_finding(InspectFinding::new(
                    "architecture",
                    "pattern",
                    "modular",
                    0.8,
                ));
                report.add_finding(InspectFinding::new(
                    "tech_stack",
                    "build_tool",
                    "cargo",
                    0.9,
                ));
                report.nomx_entry =
                    "define repo that language(rust) pattern(modular) build(cargo)".to_string();
            }
            InspectTarget::Website { .. } => {
                report.add_finding(InspectFinding::new("tech_stack", "framework", "react", 0.8));
                report.add_finding(InspectFinding::new("design", "layout", "sidebar", 0.7));
                report.add_finding(InspectFinding::new("content", "purpose", "saas", 0.75));
                report.nomx_entry =
                    "define website that framework(react) layout(sidebar) type(saas)".to_string();
            }
            InspectTarget::PersonUsername { .. } => {
                report.add_finding(InspectFinding::new("profile", "platform", "github", 0.9));
                report.add_finding(InspectFinding::new("profile", "activity", "active", 0.8));
                report.add_finding(InspectFinding::new("profile", "focus", "engineering", 0.85));
                report.nomx_entry =
                    "define person that platform(github) activity(active) focus(engineering)"
                        .to_string();
            }
            InspectTarget::CompanyDomain { .. } => {
                report.add_finding(InspectFinding::new("strategy", "model", "b2b_saas", 0.85));
                report.add_finding(InspectFinding::new("tech_stack", "backend", "node", 0.7));
                report.add_finding(InspectFinding::new(
                    "strategy",
                    "market",
                    "developer_tools",
                    0.9,
                ));
                report.nomx_entry =
                    "define company that model(b2b_saas) market(developer_tools)".to_string();
            }
            InspectTarget::VideoFile { .. } => {
                report.add_finding(InspectFinding::new("media", "format", "mp4", 0.95));
                report.add_finding(InspectFinding::new("media", "duration", "unknown", 0.5));
                report.nomx_entry = "define media that format(mp4)".to_string();
            }
            InspectTarget::ImageFile { .. } => {
                report.add_finding(InspectFinding::new("media", "format", "mp4", 0.95));
                report.add_finding(InspectFinding::new("media", "duration", "unknown", 0.5));
                report.nomx_entry = "define media that format(mp4)".to_string();
            }
        }
        report
    }

    /// Detect target type from a raw URL or string
    pub fn detect_target(input: &str) -> InspectTarget {
        let lower = input.to_lowercase();
        if lower.contains("youtube.com") || lower.contains("youtu.be") {
            InspectTarget::YoutubeChannel {
                url: input.to_string(),
            }
        } else if lower.contains("github.com") {
            InspectTarget::GithubRepo {
                url: input.to_string(),
            }
        } else if lower.starts_with("http://") || lower.starts_with("https://") {
            InspectTarget::Website {
                url: input.to_string(),
            }
        } else if lower.ends_with(".mp4") || lower.ends_with(".mov") || lower.ends_with(".avi") {
            InspectTarget::VideoFile {
                path: input.to_string(),
            }
        } else if lower.ends_with(".jpg") || lower.ends_with(".png") || lower.ends_with(".gif") {
            InspectTarget::ImageFile {
                path: input.to_string(),
            }
        } else if !input.contains('.') && !input.contains('/') {
            InspectTarget::PersonUsername {
                username: input.to_string(),
            }
        } else if input.contains('.') {
            InspectTarget::CompanyDomain {
                domain: input.to_string(),
            }
        } else {
            InspectTarget::Website {
                url: input.to_string(),
            }
        }
    }

    /// Full pipeline: detect → inspect → return report
    pub fn inspect_url(input: &str) -> InspectReport {
        let target = Self::detect_target(input);
        Self::inspect(target)
    }

    /// Full pipeline that emits reverse-engineering output as a Nomx function.
    pub fn inspect_as_nomx_function(input: &str) -> InspectReport {
        let target = Self::detect_target(input);
        let reverse_input = match &target {
            InspectTarget::YoutubeChannel { url }
            | InspectTarget::GithubRepo { url }
            | InspectTarget::Website { url } => crate::reverse::ReverseInput::WebUrl(url.clone()),
            InspectTarget::VideoFile { path } | InspectTarget::ImageFile { path } => {
                crate::reverse::ReverseInput::ScreenshotPath(path.clone())
            }
            InspectTarget::PersonUsername { username } => {
                crate::reverse::ReverseInput::WebUrl(username.clone())
            }
            InspectTarget::CompanyDomain { domain } => {
                crate::reverse::ReverseInput::WebUrl(domain.clone())
            }
        };
        let mut report = Self::inspect(target);
        let components = crate::reverse::ReverseOrchestrator::detect_components(&reverse_input);
        report.nomx_entry =
            crate::reverse::ReverseOrchestrator::to_nomx_function(&reverse_input, &components);
        report
    }
}

/// Quality gate: LLM scores each inspection step; retry until DreamScore >=95.
#[derive(Debug, Clone)]
pub struct QualityGateConfig {
    pub min_score: u8,
    pub max_retries: u8,
}

impl Default for QualityGateConfig {
    fn default() -> Self {
        Self {
            min_score: 95,
            max_retries: 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct QualityGateResult {
    pub score: u8,
    pub passed: bool,
    pub attempts: u8,
    pub finding_count: usize,
    pub nomx_entry: String,
}

/// Wraps NomInspector with LLM quality scoring.
pub struct LlmQualityGate {
    pub config: QualityGateConfig,
}

impl LlmQualityGate {
    pub fn new(config: QualityGateConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self {
            config: QualityGateConfig::default(),
        }
    }

    /// Score a report using an LLM fn (stub: counts findings as proxy for quality).
    pub fn score_report(&self, report: &InspectReport) -> u8 {
        let base: u8 = 60;
        let bonus = (report.findings.len() as u8).saturating_mul(5).min(35);
        base.saturating_add(bonus)
    }

    /// Run inspect_url with quality gate; retry up to max_retries.
    pub fn inspect_with_quality(&self, url: &str) -> QualityGateResult {
        let mut attempts = 0u8;
        loop {
            attempts += 1;
            let report = NomInspector::inspect_as_nomx_function(url);
            let score = self.score_report(&report);
            if score >= self.config.min_score || attempts >= self.config.max_retries {
                return QualityGateResult {
                    score,
                    passed: score >= self.config.min_score,
                    attempts,
                    finding_count: report.findings.len(),
                    nomx_entry: report.nomx_entry.clone(),
                };
            }
        }
    }
}

#[cfg(test)]
mod quality_gate_tests {
    use super::*;

    #[test]
    fn test_quality_gate_defaults() {
        let gate = LlmQualityGate::with_defaults();
        assert_eq!(gate.config.min_score, 95);
        assert_eq!(gate.config.max_retries, 3);
    }

    #[test]
    fn test_score_report_base() {
        let gate = LlmQualityGate::with_defaults();
        let report = InspectReport::new(InspectTarget::Website { url: "x".into() });
        let score = gate.score_report(&report);
        assert_eq!(score, 60);
    }

    #[test]
    fn test_score_report_with_findings() {
        let gate = LlmQualityGate::with_defaults();
        let target = InspectTarget::GithubRepo {
            url: "https://github.com/nom/nom".into(),
        };
        let mut report = InspectReport::new(target);
        for i in 0..8 {
            report.add_finding(InspectFinding::new(
                "test",
                &format!("key{}", i),
                &format!("val{}", i),
                0.9,
            ));
        }
        let score = gate.score_report(&report);
        assert!(score >= 95);
    }

    #[test]
    fn test_inspect_with_quality_runs() {
        let gate = LlmQualityGate::with_defaults();
        let result = gate.inspect_with_quality("https://github.com/nom/nom");
        assert!(result.attempts >= 1);
        assert!(result.attempts <= 3);
        assert!(result
            .nomx_entry
            .starts_with("the function reverse_web_url_to_nomx is"));
    }

    #[test]
    fn test_quality_gate_result_fields() {
        let r = QualityGateResult {
            score: 80,
            passed: false,
            attempts: 2,
            finding_count: 0,
            nomx_entry: "empty".into(),
        };
        assert_eq!(r.score, 80);
        assert!(!r.passed);
        assert_eq!(r.attempts, 2);
    }

    #[test]
    fn test_custom_config() {
        let config = QualityGateConfig {
            min_score: 80,
            max_retries: 5,
        };
        let gate = LlmQualityGate::new(config);
        assert_eq!(gate.config.min_score, 80);
        assert_eq!(gate.config.max_retries, 5);
    }

    #[test]
    fn test_inspect_as_nomx_function_returns_function_entry() {
        let report = NomInspector::inspect_as_nomx_function("https://github.com/nom/nom");
        assert!(report
            .nomx_entry
            .starts_with("the function reverse_web_url_to_nomx is"));
        assert!(report.nomx_entry.contains("intended to reverse engineer"));
        assert!(report.nomx_entry.contains("returns nomx_component_tree"));
        assert!(report.finding_count() >= 3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspect_target_kind_label_youtube() {
        let t = InspectTarget::YoutubeChannel {
            url: "https://youtube.com/@test".to_string(),
        };
        assert_eq!(t.kind_label(), "youtube_channel");
    }

    #[test]
    fn inspect_target_kind_label_repo() {
        let t = InspectTarget::GithubRepo {
            url: "https://github.com/org/repo".to_string(),
        };
        assert_eq!(t.kind_label(), "github_repo");
    }

    #[test]
    fn detect_target_youtube_url() {
        let t = NomInspector::detect_target("https://youtube.com/@channel");
        assert_eq!(t.kind_label(), "youtube_channel");
    }

    #[test]
    fn detect_target_github_url() {
        let t = NomInspector::detect_target("https://github.com/rust-lang/rust");
        assert_eq!(t.kind_label(), "github_repo");
    }

    #[test]
    fn detect_target_person_username() {
        let t = NomInspector::detect_target("torvalds");
        assert_eq!(t.kind_label(), "person");
    }

    #[test]
    fn inspect_youtube_findings() {
        let target = InspectTarget::YoutubeChannel {
            url: "https://youtube.com/@test".to_string(),
        };
        let report = NomInspector::inspect(target);
        assert_eq!(report.finding_count(), 3);
        let cs = report.findings_by_category("content_strategy");
        assert_eq!(cs.len(), 3);
        assert!(report.nomx_entry.contains("channel"));
    }

    #[test]
    fn inspect_repo_findings() {
        let target = InspectTarget::GithubRepo {
            url: "https://github.com/rust-lang/rust".to_string(),
        };
        let report = NomInspector::inspect(target);
        assert_eq!(report.finding_count(), 3);
        let arch = report.findings_by_category("architecture");
        assert_eq!(arch.len(), 2);
        assert!(report.nomx_entry.contains("repo"));
    }

    #[test]
    fn inspect_report_add_finding() {
        let target = InspectTarget::Website {
            url: "https://example.com".to_string(),
        };
        let mut report = InspectReport::new(target);
        assert!(report.is_empty());
        report.add_finding(InspectFinding::new("test_cat", "key1", "val1", 0.9));
        assert_eq!(report.finding_count(), 1);
        assert!(!report.is_empty());
    }

    #[test]
    fn inspect_report_findings_by_category() {
        let target = InspectTarget::CompanyDomain {
            domain: "acme.com".to_string(),
        };
        let report = NomInspector::inspect(target);
        let strategy = report.findings_by_category("strategy");
        assert_eq!(strategy.len(), 2);
        let tech = report.findings_by_category("tech_stack");
        assert_eq!(tech.len(), 1);
    }

    #[test]
    fn inspect_url_full_pipeline() {
        let report = NomInspector::inspect_url("https://github.com/nom-lang/nom");
        assert_eq!(report.target.kind_label(), "github_repo");
        assert!(!report.is_empty());
        assert!(!report.nomx_entry.is_empty());
    }
}
