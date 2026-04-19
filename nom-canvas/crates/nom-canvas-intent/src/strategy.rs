/// Business model classification for strategy extraction.
#[derive(Debug, Clone, PartialEq)]
pub enum BusinessModel {
    B2BSaas,
    B2CSaas,
    Marketplace,
    OpenSource,
    Enterprise,
    Unknown,
}

/// A single competitive intelligence signal extracted from text.
#[derive(Debug, Clone)]
pub struct StrategySignal {
    pub signal_type: String, // "pricing", "positioning", "market", "tech"
    pub value: String,
    pub strength: f32, // 0.0–1.0
}

impl StrategySignal {
    pub fn new(signal_type: &str, value: &str, strength: f32) -> Self {
        Self {
            signal_type: signal_type.to_string(),
            value: value.to_string(),
            strength,
        }
    }
}

/// Aggregated strategy report for a page or product.
#[derive(Debug, Clone)]
pub struct StrategyReport {
    pub model: BusinessModel,
    pub signals: Vec<StrategySignal>,
    pub summary: String,
}

impl StrategyReport {
    pub fn new(model: BusinessModel, summary: &str) -> Self {
        Self {
            model,
            signals: Vec::new(),
            summary: summary.to_string(),
        }
    }

    pub fn add_signal(&mut self, signal: StrategySignal) {
        self.signals.push(signal);
    }

    pub fn signal_count(&self) -> usize {
        self.signals.len()
    }

    pub fn strong_signals(&self, threshold: f32) -> Vec<&StrategySignal> {
        self.signals
            .iter()
            .filter(|s| s.strength >= threshold)
            .collect()
    }

    pub fn dominant_model(&self) -> &BusinessModel {
        &self.model
    }
}

/// Keyword-based strategy extractor.
pub struct StrategyExtractor;

impl StrategyExtractor {
    /// Extract strategy from page text / metadata (keyword-based stub).
    pub fn extract(text: &str) -> StrategyReport {
        let lower = text.to_lowercase();

        if lower.contains("open source") || lower.contains("github") {
            let mut report = StrategyReport::new(BusinessModel::OpenSource, "Open source project");
            report.add_signal(StrategySignal::new("tech", "oss", 0.9));
            report
        } else if lower.contains("enterprise") {
            let mut report =
                StrategyReport::new(BusinessModel::Enterprise, "Enterprise-focused product");
            report.add_signal(StrategySignal::new(
                "positioning",
                "enterprise",
                0.85,
            ));
            report
        } else if lower.contains("marketplace") || lower.contains("sell") {
            let mut report =
                StrategyReport::new(BusinessModel::Marketplace, "Marketplace platform");
            report.add_signal(StrategySignal::new("market", "marketplace", 0.75));
            report
        } else if lower.contains("pricing") && lower.contains("plan") {
            let mut report =
                StrategyReport::new(BusinessModel::B2BSaas, "B2B SaaS with tiered pricing");
            report.add_signal(StrategySignal::new("pricing", "tiered", 0.8));
            report
        } else {
            let mut report = StrategyReport::new(BusinessModel::B2CSaas, "B2C SaaS product");
            report.add_signal(StrategySignal::new("market", "consumer", 0.5));
            report
        }
    }

    /// Serialize a strategy report to a Nom expression string.
    pub fn to_nomx(report: &StrategyReport) -> String {
        let model_str = match report.model {
            BusinessModel::B2BSaas => "b2b_saas",
            BusinessModel::B2CSaas => "b2c_saas",
            BusinessModel::Marketplace => "marketplace",
            BusinessModel::OpenSource => "open_source",
            BusinessModel::Enterprise => "enterprise",
            BusinessModel::Unknown => "unknown",
        };
        format!(
            "define strategy that model({}) signals({})",
            model_str,
            report.signal_count()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_new() {
        let s = StrategySignal::new("pricing", "tiered", 0.8);
        assert_eq!(s.signal_type, "pricing");
        assert_eq!(s.value, "tiered");
        assert!((s.strength - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_strategy_report_new() {
        let r = StrategyReport::new(BusinessModel::B2BSaas, "test summary");
        assert_eq!(r.model, BusinessModel::B2BSaas);
        assert_eq!(r.summary, "test summary");
        assert_eq!(r.signal_count(), 0);
    }

    #[test]
    fn test_add_signal() {
        let mut r = StrategyReport::new(BusinessModel::OpenSource, "oss");
        r.add_signal(StrategySignal::new("tech", "oss", 0.9));
        assert_eq!(r.signal_count(), 1);
    }

    #[test]
    fn test_strong_signals() {
        let mut r = StrategyReport::new(BusinessModel::Enterprise, "ent");
        r.add_signal(StrategySignal::new("positioning", "enterprise", 0.85));
        r.add_signal(StrategySignal::new("market", "smb", 0.3));
        let strong = r.strong_signals(0.7);
        assert_eq!(strong.len(), 1);
        assert_eq!(strong[0].value, "enterprise");
    }

    #[test]
    fn test_extract_b2b_saas() {
        let report = StrategyExtractor::extract("Check out our pricing plan for teams");
        assert_eq!(report.model, BusinessModel::B2BSaas);
        assert_eq!(report.signal_count(), 1);
        assert_eq!(report.signals[0].signal_type, "pricing");
    }

    #[test]
    fn test_extract_open_source() {
        let report = StrategyExtractor::extract("Available on GitHub as open source");
        assert_eq!(report.model, BusinessModel::OpenSource);
        assert_eq!(report.signals[0].value, "oss");
    }

    #[test]
    fn test_extract_enterprise() {
        let report = StrategyExtractor::extract("Built for enterprise customers at scale");
        assert_eq!(report.model, BusinessModel::Enterprise);
        assert_eq!(report.signals[0].signal_type, "positioning");
    }

    #[test]
    fn test_to_nomx() {
        let mut report = StrategyReport::new(BusinessModel::B2BSaas, "saas");
        report.add_signal(StrategySignal::new("pricing", "tiered", 0.8));
        let nomx = StrategyExtractor::to_nomx(&report);
        assert_eq!(nomx, "define strategy that model(b2b_saas) signals(1)");
    }
}
