#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryTier {
    Ui,
    Interactive,
    Background,
    External,
}

impl TelemetryTier {
    pub fn default_sampling_ratio(self) -> f64 {
        match self {
            Self::Ui => 1.0,
            Self::Interactive => 0.5,
            Self::Background => 0.05,
            Self::External => 1.0,
        }
    }

    pub fn level_hint(self) -> &'static str {
        match self {
            Self::Ui => "info",
            Self::Interactive => "info",
            Self::Background => "debug",
            Self::External => "info",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sampling_ratios_are_correct() {
        assert_eq!(TelemetryTier::Ui.default_sampling_ratio(), 1.0);
        assert_eq!(TelemetryTier::Interactive.default_sampling_ratio(), 0.5);
        assert_eq!(TelemetryTier::Background.default_sampling_ratio(), 0.05);
        assert_eq!(TelemetryTier::External.default_sampling_ratio(), 1.0);
    }

    #[test]
    fn level_hints_are_correct() {
        assert_eq!(TelemetryTier::Ui.level_hint(), "info");
        assert_eq!(TelemetryTier::Interactive.level_hint(), "info");
        assert_eq!(TelemetryTier::Background.level_hint(), "debug");
        assert_eq!(TelemetryTier::External.level_hint(), "info");
    }

    #[test]
    fn background_ratio_is_low() {
        assert!(TelemetryTier::Background.default_sampling_ratio() < 0.1);
    }
}
