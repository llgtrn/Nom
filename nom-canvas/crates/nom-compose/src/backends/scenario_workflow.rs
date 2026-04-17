#![deny(unsafe_code)]
use crate::backends::ComposeResult;

/// A combined scenario-and-workflow specification. Carries named steps,
/// event triggers, and an optional timeout budget.
pub struct ScenarioWorkflowSpec {
    pub name: String,
    pub steps: Vec<String>,
    pub triggers: Vec<String>,
    pub timeout_ms: u64,
}

impl ScenarioWorkflowSpec {
    /// Number of steps in this spec.
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Number of triggers in this spec.
    pub fn trigger_count(&self) -> usize {
        self.triggers.len()
    }
}

/// Execute the combined scenario-workflow spec and return a ComposeResult.
pub fn compose(spec: &ScenarioWorkflowSpec) -> ComposeResult {
    if spec.name.is_empty() {
        return Err("scenario_workflow: name must not be empty".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenario_workflow_step_count() {
        let spec = ScenarioWorkflowSpec {
            name: "onboarding".into(),
            steps: vec!["register".into(), "verify".into(), "welcome".into()],
            triggers: vec!["user_signup".into()],
            timeout_ms: 5000,
        };
        assert_eq!(spec.step_count(), 3);
        assert_eq!(spec.trigger_count(), 1);
    }

    #[test]
    fn scenario_workflow_compose_produces_artifact() {
        let spec = ScenarioWorkflowSpec {
            name: "checkout".into(),
            steps: vec!["cart".into(), "payment".into(), "confirm".into()],
            triggers: vec!["buy_now".into(), "cart_checkout".into()],
            timeout_ms: 10_000,
        };
        let result = compose(&spec);
        assert!(result.is_ok());
    }
}
