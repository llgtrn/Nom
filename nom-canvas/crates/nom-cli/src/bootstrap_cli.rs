/// Status of a single bootstrap stage.
#[derive(Debug, Clone)]
pub struct BootstrapStageStatus {
    pub stage_name: String,
    pub nomx_file: String,
    pub is_bootstrapped: bool,
    pub source_lines: usize,
}

impl BootstrapStageStatus {
    pub fn new(
        stage_name: impl Into<String>,
        nomx_file: impl Into<String>,
        source_lines: usize,
        bootstrapped: bool,
    ) -> Self {
        Self {
            stage_name: stage_name.into(),
            nomx_file: nomx_file.into(),
            is_bootstrapped: bootstrapped,
            source_lines,
        }
    }
}

/// Aggregated report across all bootstrap stages.
#[derive(Debug)]
pub struct BootstrapStatusReport {
    pub stages: Vec<BootstrapStageStatus>,
}

impl BootstrapStatusReport {
    pub fn new() -> Self {
        Self { stages: Vec::new() }
    }

    pub fn add_stage(&mut self, stage: BootstrapStageStatus) {
        self.stages.push(stage);
    }

    pub fn bootstrapped_count(&self) -> usize {
        self.stages.iter().filter(|s| s.is_bootstrapped).count()
    }

    pub fn total_stages(&self) -> usize {
        self.stages.len()
    }

    pub fn completion_percent(&self) -> f32 {
        let total = self.total_stages();
        if total == 0 {
            return 0.0;
        }
        self.bootstrapped_count() as f32 / total as f32 * 100.0
    }

    pub fn next_target(&self) -> Option<&BootstrapStageStatus> {
        self.stages.iter().find(|s| !s.is_bootstrapped)
    }

    pub fn is_fixpoint_ready(&self) -> bool {
        !self.stages.is_empty() && self.stages.iter().all(|s| s.is_bootstrapped)
    }
}

impl Default for BootstrapStatusReport {
    fn default() -> Self {
        Self::new()
    }
}

/// CLI runner for bootstrap fixpoint checks.
pub struct BootstrapCliRunner;

impl BootstrapCliRunner {
    pub fn new() -> Self {
        Self
    }

    pub fn build_seed_report() -> BootstrapStatusReport {
        let mut report = BootstrapStatusReport::new();
        report.add_stage(BootstrapStageStatus::new("lexer", "lexer.nomx", 255, true));
        report.add_stage(BootstrapStageStatus::new("parser", "parser.nomx", 0, false));
        report.add_stage(BootstrapStageStatus::new("resolver", "resolver.nomx", 0, false));
        report.add_stage(BootstrapStageStatus::new("type_checker", "type_checker.nomx", 0, false));
        report.add_stage(BootstrapStageStatus::new("codegen", "codegen.nomx", 0, false));
        report
    }

    pub fn format_status(report: &BootstrapStatusReport) -> String {
        let bootstrapped = report.bootstrapped_count();
        let total = report.total_stages();
        let percent = report.completion_percent();
        let next = report
            .next_target()
            .map(|s| s.nomx_file.as_str())
            .unwrap_or("none");
        format!(
            "Bootstrap: {}/{} stages ({:.1}%) | Next: {}",
            bootstrapped, total, percent, next
        )
    }
}

impl Default for BootstrapCliRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod bootstrap_cli_tests {
    use super::*;

    #[test]
    fn test_stage_status_fields() {
        let stage = BootstrapStageStatus::new("lexer", "lexer.nomx", 255, true);
        assert_eq!(stage.stage_name, "lexer");
        assert_eq!(stage.nomx_file, "lexer.nomx");
        assert_eq!(stage.source_lines, 255);
        assert!(stage.is_bootstrapped);
    }

    #[test]
    fn test_bootstrapped_count() {
        let mut report = BootstrapStatusReport::new();
        report.add_stage(BootstrapStageStatus::new("a", "a.nomx", 10, true));
        report.add_stage(BootstrapStageStatus::new("b", "b.nomx", 0, false));
        report.add_stage(BootstrapStageStatus::new("c", "c.nomx", 5, true));
        assert_eq!(report.bootstrapped_count(), 2);
    }

    #[test]
    fn test_completion_percent() {
        let mut report = BootstrapStatusReport::new();
        report.add_stage(BootstrapStageStatus::new("a", "a.nomx", 10, true));
        report.add_stage(BootstrapStageStatus::new("b", "b.nomx", 0, false));
        report.add_stage(BootstrapStageStatus::new("c", "c.nomx", 0, false));
        report.add_stage(BootstrapStageStatus::new("d", "d.nomx", 0, false));
        // 1/4 = 25.0
        assert!((report.completion_percent() - 25.0).abs() < 0.01);
    }

    #[test]
    fn test_next_target_returns_first_non_bootstrapped() {
        let mut report = BootstrapStatusReport::new();
        report.add_stage(BootstrapStageStatus::new("lexer", "lexer.nomx", 255, true));
        report.add_stage(BootstrapStageStatus::new("parser", "parser.nomx", 0, false));
        report.add_stage(BootstrapStageStatus::new("resolver", "resolver.nomx", 0, false));
        let next = report.next_target().expect("should have next");
        assert_eq!(next.nomx_file, "parser.nomx");
    }

    #[test]
    fn test_is_fixpoint_ready_false_when_not_all_done() {
        let mut report = BootstrapStatusReport::new();
        report.add_stage(BootstrapStageStatus::new("lexer", "lexer.nomx", 255, true));
        report.add_stage(BootstrapStageStatus::new("parser", "parser.nomx", 0, false));
        assert!(!report.is_fixpoint_ready());
    }

    #[test]
    fn test_is_fixpoint_ready_true_when_all_done() {
        let mut report = BootstrapStatusReport::new();
        report.add_stage(BootstrapStageStatus::new("lexer", "lexer.nomx", 255, true));
        report.add_stage(BootstrapStageStatus::new("parser", "parser.nomx", 100, true));
        assert!(report.is_fixpoint_ready());
    }

    #[test]
    fn test_build_seed_report_has_5_stages() {
        let report = BootstrapCliRunner::build_seed_report();
        assert_eq!(report.total_stages(), 5);
    }

    #[test]
    fn test_build_seed_report_lexer_is_bootstrapped() {
        let report = BootstrapCliRunner::build_seed_report();
        let lexer = report.stages.iter().find(|s| s.nomx_file == "lexer.nomx");
        assert!(lexer.is_some());
        assert!(lexer.unwrap().is_bootstrapped);
    }

    #[test]
    fn test_format_status_contains_percentage() {
        let report = BootstrapCliRunner::build_seed_report();
        let status = BootstrapCliRunner::format_status(&report);
        assert!(status.contains('%'), "expected '%' in: {}", status);
        assert!(status.contains("20.0%"), "expected '20.0%' in: {}", status);
    }
}
