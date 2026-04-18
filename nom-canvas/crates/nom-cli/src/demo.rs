/// Golden path demo commands — show the full Nom pipeline end-to-end.

#[derive(Debug, Clone, PartialEq)]
pub enum DemoKind {
    NomxHighlight,
    DragNodeCanvas,
    WireCompose,
    IntentResolve,
}

pub struct DemoRunner {
    pub kind: DemoKind,
    pub dry_run: bool,
    pub verbose: bool,
}

impl DemoRunner {
    pub fn new(kind: DemoKind) -> Self {
        Self {
            kind,
            dry_run: false,
            verbose: false,
        }
    }

    pub fn dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }

    pub fn verbose(mut self) -> Self {
        self.verbose = true;
        self
    }

    pub fn run(&self) -> DemoResult {
        let steps: &[&str] = match self.kind {
            DemoKind::NomxHighlight => &[
                "parse_source",
                "lex_tokens",
                "classify_kinds",
                "render_highlights",
            ],
            DemoKind::DragNodeCanvas => &[
                "create_block",
                "insert_db",
                "assign_entity_ref",
                "render_node",
            ],
            DemoKind::WireCompose => &[
                "create_connector",
                "validate_wire",
                "score_confidence",
                "run_composition",
                "emit_artifact",
            ],
            DemoKind::IntentResolve => {
                &["parse_intent", "bm25_rank", "classify_react", "route_skill"]
            }
        };

        let total = steps.len() as u32;
        let mut output = String::new();

        for (i, step) in steps.iter().enumerate() {
            if self.verbose {
                output.push_str(&format!("  [{}/{}] {}\n", i + 1, total, step));
            } else {
                output.push_str(&format!("{}\n", step));
            }
        }

        DemoResult::success_result(total, &output)
    }
}

#[derive(Debug, Clone)]
pub struct DemoResult {
    pub success: bool,
    pub steps_completed: u32,
    pub steps_total: u32,
    pub output: String,
    pub errors: Vec<String>,
}

impl DemoResult {
    pub fn success_result(steps: u32, output: &str) -> Self {
        Self {
            success: true,
            steps_completed: steps,
            steps_total: steps,
            output: output.to_string(),
            errors: vec![],
        }
    }

    pub fn partial_result(completed: u32, total: u32, errors: Vec<String>) -> Self {
        Self {
            success: errors.is_empty(),
            steps_completed: completed,
            steps_total: total,
            output: String::new(),
            errors,
        }
    }

    pub fn completion_ratio(&self) -> f32 {
        if self.steps_total == 0 {
            return 0.0;
        }
        self.steps_completed as f32 / self.steps_total as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_runner_new() {
        let runner = DemoRunner::new(DemoKind::NomxHighlight);
        assert_eq!(runner.kind, DemoKind::NomxHighlight);
        assert!(!runner.dry_run);
        assert!(!runner.verbose);
    }

    #[test]
    fn demo_nomx_highlight_run() {
        let result = DemoRunner::new(DemoKind::NomxHighlight).dry_run().run();
        assert!(result.success);
        assert_eq!(result.steps_completed, 4);
        assert_eq!(result.steps_total, 4);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn demo_wire_compose_run() {
        let result = DemoRunner::new(DemoKind::WireCompose).dry_run().run();
        assert!(result.success);
        assert_eq!(result.steps_completed, 5);
        assert_eq!(result.steps_total, 5);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn demo_result_completion_ratio() {
        let result = DemoResult {
            success: true,
            steps_completed: 3,
            steps_total: 4,
            output: String::new(),
            errors: vec![],
        };
        assert!((result.completion_ratio() - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn demo_result_partial() {
        let result = DemoResult::partial_result(2, 5, vec!["step failed".to_string()]);
        assert!(!result.success);
        assert_eq!(result.steps_completed, 2);
        assert_eq!(result.steps_total, 5);
        assert_eq!(result.errors.len(), 1);
    }
}
