/// StoryboardPhase — the 5 phases of a storyboard pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoryboardPhase {
    Concept,
    Script,
    VisualPlan,
    Render,
    Export,
}

impl StoryboardPhase {
    /// Returns the lowercase name of the phase.
    pub fn phase_name(&self) -> &str {
        match self {
            StoryboardPhase::Concept => "concept",
            StoryboardPhase::Script => "script",
            StoryboardPhase::VisualPlan => "visualplan",
            StoryboardPhase::Render => "render",
            StoryboardPhase::Export => "export",
        }
    }

    /// Returns the zero-based index of the phase (0..4).
    pub fn phase_index(&self) -> usize {
        match self {
            StoryboardPhase::Concept => 0,
            StoryboardPhase::Script => 1,
            StoryboardPhase::VisualPlan => 2,
            StoryboardPhase::Render => 3,
            StoryboardPhase::Export => 4,
        }
    }

    /// Returns the next phase, or None if this is Export (the last phase).
    pub fn next(&self) -> Option<StoryboardPhase> {
        match self {
            StoryboardPhase::Concept => Some(StoryboardPhase::Script),
            StoryboardPhase::Script => Some(StoryboardPhase::VisualPlan),
            StoryboardPhase::VisualPlan => Some(StoryboardPhase::Render),
            StoryboardPhase::Render => Some(StoryboardPhase::Export),
            StoryboardPhase::Export => None,
        }
    }
}

/// StoryboardStep — a single step within a storyboard pipeline.
#[derive(Debug, Clone)]
pub struct StoryboardStep {
    pub phase: StoryboardPhase,
    pub title: String,
    pub estimated_ms: u64,
}

impl StoryboardStep {
    /// Creates a new StoryboardStep.
    pub fn new(phase: StoryboardPhase, title: impl Into<String>, estimated_ms: u64) -> Self {
        Self {
            phase,
            title: title.into(),
            estimated_ms,
        }
    }

    /// Returns true if this step belongs to the Render phase.
    pub fn is_render_phase(&self) -> bool {
        self.phase == StoryboardPhase::Render
    }
}

/// StoryboardPlan — an ordered list of steps with a name.
#[derive(Debug, Clone)]
pub struct StoryboardPlan {
    pub steps: Vec<StoryboardStep>,
    pub name: String,
}

impl StoryboardPlan {
    /// Creates a new empty StoryboardPlan with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            steps: Vec::new(),
            name: name.into(),
        }
    }

    /// Appends a step to the plan.
    pub fn add_step(&mut self, step: StoryboardStep) {
        self.steps.push(step);
    }

    /// Returns the sum of estimated_ms across all steps.
    pub fn total_estimated_ms(&self) -> u64 {
        self.steps.iter().map(|s| s.estimated_ms).sum()
    }

    /// Returns the number of steps in the plan.
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Returns unique phase names in the order they first appear.
    pub fn phases_present(&self) -> Vec<String> {
        let mut seen = Vec::new();
        for step in &self.steps {
            let name = step.phase.phase_name().to_string();
            if !seen.contains(&name) {
                seen.push(name);
            }
        }
        seen
    }
}

/// StoryboardExecutor — advances through a StoryboardPlan tracking current state.
#[derive(Debug)]
pub struct StoryboardExecutor {
    pub plan: StoryboardPlan,
    pub current_step: usize,
    pub completed_steps: Vec<String>,
}

impl StoryboardExecutor {
    /// Creates a new executor for the given plan.
    pub fn new(plan: StoryboardPlan) -> Self {
        Self {
            plan,
            current_step: 0,
            completed_steps: Vec::new(),
        }
    }

    /// Advances to the next step and returns a reference to it, or None if all steps are done.
    pub fn advance(&mut self) -> Option<&StoryboardStep> {
        if self.current_step >= self.plan.step_count() {
            return None;
        }
        let step = &self.plan.steps[self.current_step];
        self.completed_steps.push(step.title.clone());
        self.current_step += 1;
        // Return reference to the step we just advanced past (now at current_step - 1).
        Some(&self.plan.steps[self.current_step - 1])
    }

    /// Returns true when all steps have been advanced through.
    pub fn is_complete(&self) -> bool {
        self.current_step >= self.plan.step_count()
    }

    /// Returns progress as a percentage (0.0–100.0). Returns 0.0 if the plan is empty.
    pub fn progress_pct(&self) -> f32 {
        if self.plan.step_count() == 0 {
            return 0.0;
        }
        self.current_step as f32 / self.plan.step_count() as f32 * 100.0
    }
}

#[cfg(test)]
mod storyboard_tests {
    use super::*;

    #[test]
    fn phase_index() {
        assert_eq!(StoryboardPhase::Concept.phase_index(), 0);
        assert_eq!(StoryboardPhase::Script.phase_index(), 1);
        assert_eq!(StoryboardPhase::VisualPlan.phase_index(), 2);
        assert_eq!(StoryboardPhase::Render.phase_index(), 3);
        assert_eq!(StoryboardPhase::Export.phase_index(), 4);
    }

    #[test]
    fn next_from_concept() {
        assert_eq!(
            StoryboardPhase::Concept.next(),
            Some(StoryboardPhase::Script)
        );
        assert_eq!(
            StoryboardPhase::Script.next(),
            Some(StoryboardPhase::VisualPlan)
        );
        assert_eq!(
            StoryboardPhase::VisualPlan.next(),
            Some(StoryboardPhase::Render)
        );
        assert_eq!(
            StoryboardPhase::Render.next(),
            Some(StoryboardPhase::Export)
        );
    }

    #[test]
    fn next_from_export_is_none() {
        assert_eq!(StoryboardPhase::Export.next(), None);
    }

    #[test]
    fn is_render_phase() {
        let render_step = StoryboardStep::new(StoryboardPhase::Render, "Render frames", 5000);
        assert!(render_step.is_render_phase());

        let script_step = StoryboardStep::new(StoryboardPhase::Script, "Write script", 1000);
        assert!(!script_step.is_render_phase());
    }

    #[test]
    fn add_and_count() {
        let mut plan = StoryboardPlan::new("test-plan");
        assert_eq!(plan.step_count(), 0);

        plan.add_step(StoryboardStep::new(StoryboardPhase::Concept, "Ideate", 500));
        plan.add_step(StoryboardStep::new(StoryboardPhase::Script, "Draft", 1000));
        assert_eq!(plan.step_count(), 2);
    }

    #[test]
    fn total_estimated_ms() {
        let mut plan = StoryboardPlan::new("timing-plan");
        plan.add_step(StoryboardStep::new(StoryboardPhase::Concept, "Ideate", 300));
        plan.add_step(StoryboardStep::new(StoryboardPhase::Script, "Draft", 700));
        plan.add_step(StoryboardStep::new(StoryboardPhase::Render, "Render", 2000));
        assert_eq!(plan.total_estimated_ms(), 3000);
    }

    #[test]
    fn advance_returns_steps() {
        let mut plan = StoryboardPlan::new("exec-plan");
        plan.add_step(StoryboardStep::new(StoryboardPhase::Concept, "Ideate", 100));
        plan.add_step(StoryboardStep::new(StoryboardPhase::Export, "Export", 200));

        let mut executor = StoryboardExecutor::new(plan);

        let step1 = executor.advance().expect("first advance must return a step");
        assert_eq!(step1.title, "Ideate");

        let step2 = executor.advance().expect("second advance must return a step");
        assert_eq!(step2.title, "Export");

        assert!(executor.advance().is_none(), "third advance must return None");
    }

    #[test]
    fn is_complete_after_all() {
        let mut plan = StoryboardPlan::new("complete-plan");
        plan.add_step(StoryboardStep::new(StoryboardPhase::Concept, "Step A", 50));

        let mut executor = StoryboardExecutor::new(plan);
        assert!(!executor.is_complete());

        executor.advance();
        assert!(executor.is_complete());
    }

    #[test]
    fn progress_pct() {
        let mut plan = StoryboardPlan::new("progress-plan");
        plan.add_step(StoryboardStep::new(StoryboardPhase::Script, "A", 100));
        plan.add_step(StoryboardStep::new(StoryboardPhase::Render, "B", 100));
        plan.add_step(StoryboardStep::new(StoryboardPhase::Export, "C", 100));
        plan.add_step(StoryboardStep::new(StoryboardPhase::Concept, "D", 100));

        let mut executor = StoryboardExecutor::new(plan);
        assert!((executor.progress_pct() - 0.0).abs() < f32::EPSILON);

        executor.advance();
        assert!((executor.progress_pct() - 25.0).abs() < 0.001);

        executor.advance();
        executor.advance();
        executor.advance();
        assert!((executor.progress_pct() - 100.0).abs() < 0.001);

        // Empty plan returns 0.0.
        let empty_executor = StoryboardExecutor::new(StoryboardPlan::new("empty"));
        assert!((empty_executor.progress_pct() - 0.0).abs() < f32::EPSILON);
    }
}
