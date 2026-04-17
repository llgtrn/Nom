//! Multi-step composition plan: sequenced backend invocations with typed handoffs.
#![deny(unsafe_code)]

use std::collections::HashMap;
use crate::kind::NomKind;

pub type StepId = u32;

#[derive(Clone, Debug, PartialEq)]
pub struct PlanStep {
    pub id: StepId,
    pub kind: NomKind,
    /// Inputs: (step_id → output_key).  output_key=None means "use primary output".
    pub inputs: Vec<(StepId, Option<String>)>,
    /// Named params sent to this backend's ComposeSpec.
    pub params: Vec<(String, String)>,
    pub output_key: Option<String>,
}

impl PlanStep {
    pub fn new(id: StepId, kind: NomKind) -> Self {
        Self { id, kind, inputs: vec![], params: vec![], output_key: None }
    }
    pub fn with_input(mut self, from: StepId, key: Option<String>) -> Self { self.inputs.push((from, key)); self }
    pub fn with_param(mut self, k: impl Into<String>, v: impl Into<String>) -> Self { self.params.push((k.into(), v.into())); self }
    pub fn with_output_key(mut self, key: impl Into<String>) -> Self { self.output_key = Some(key.into()); self }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct CompositionPlan {
    pub steps: Vec<PlanStep>,
    pub final_step: Option<StepId>,
}

impl CompositionPlan {
    pub fn new() -> Self { Self::default() }
    pub fn add_step(&mut self, step: PlanStep) -> Result<(), PlanError> {
        if self.steps.iter().any(|s| s.id == step.id) {
            return Err(PlanError::DuplicateStepId(step.id));
        }
        // Validate all referenced input steps already exist in the plan.
        for (from, _) in &step.inputs {
            if !self.steps.iter().any(|s| s.id == *from) {
                return Err(PlanError::UnknownInput { step: step.id, referenced: *from });
            }
        }
        self.steps.push(step);
        Ok(())
    }
    pub fn set_final(&mut self, id: StepId) -> Result<(), PlanError> {
        if !self.steps.iter().any(|s| s.id == id) {
            return Err(PlanError::UnknownStep(id));
        }
        self.final_step = Some(id);
        Ok(())
    }
    pub fn step(&self, id: StepId) -> Option<&PlanStep> { self.steps.iter().find(|s| s.id == id) }
    pub fn predecessors(&self, id: StepId) -> Vec<StepId> {
        self.step(id).map(|s| s.inputs.iter().map(|(from, _)| *from).collect()).unwrap_or_default()
    }
    pub fn successors(&self, id: StepId) -> Vec<StepId> {
        self.steps.iter().filter(|s| s.inputs.iter().any(|(f, _)| *f == id)).map(|s| s.id).collect()
    }
    pub fn step_count(&self) -> usize { self.steps.len() }

    /// Topological order (Kahn).  Returns error on cycle.
    pub fn execution_order(&self) -> Result<Vec<StepId>, PlanError> {
        let mut in_degree: HashMap<StepId, usize> = self.steps.iter().map(|s| (s.id, s.inputs.len())).collect();
        let mut ready: Vec<StepId> = in_degree.iter().filter(|(_, d)| **d == 0).map(|(id, _)| *id).collect();
        ready.sort();
        let mut out = Vec::with_capacity(self.steps.len());
        while let Some(id) = ready.pop() {
            out.push(id);
            for succ in self.successors(id) {
                if let Some(d) = in_degree.get_mut(&succ) {
                    *d = d.saturating_sub(1);
                    if *d == 0 { ready.push(succ); ready.sort(); }
                }
            }
        }
        if out.len() != self.steps.len() {
            let mut cycle_participants: Vec<StepId> = in_degree.iter().filter(|(_, d)| **d > 0).map(|(id, _)| *id).collect();
            cycle_participants.sort();
            return Err(PlanError::CyclicDependencies(cycle_participants));
        }
        Ok(out)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    #[error("duplicate step id {0}")]
    DuplicateStepId(StepId),
    #[error("step {step} references unknown input step {referenced}")]
    UnknownInput { step: StepId, referenced: StepId },
    #[error("unknown step id {0}")]
    UnknownStep(StepId),
    #[error("cyclic dependencies among steps {0:?}")]
    CyclicDependencies(Vec<StepId>),
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── PlanStep ──────────────────────────────────────────────────────────────

    #[test]
    fn plan_step_new_defaults() {
        let s = PlanStep::new(1, NomKind::MediaStoryboard);
        assert_eq!(s.id, 1);
        assert_eq!(s.kind, NomKind::MediaStoryboard);
        assert!(s.inputs.is_empty());
        assert!(s.params.is_empty());
        assert!(s.output_key.is_none());
    }

    #[test]
    fn plan_step_builder_chain() {
        let s = PlanStep::new(2, NomKind::MediaVideo)
            .with_input(1, Some("frames".into()))
            .with_param("fps", "24")
            .with_output_key("video_out");
        assert_eq!(s.inputs, vec![(1, Some("frames".into()))]);
        assert_eq!(s.params, vec![("fps".into(), "24".into())]);
        assert_eq!(s.output_key, Some("video_out".into()));
    }

    // ── CompositionPlan construction ──────────────────────────────────────────

    #[test]
    fn composition_plan_new_empty() {
        let p = CompositionPlan::new();
        assert_eq!(p.step_count(), 0);
        assert!(p.final_step.is_none());
    }

    #[test]
    fn add_step_no_inputs_ok() {
        let mut p = CompositionPlan::new();
        p.add_step(PlanStep::new(1, NomKind::MediaStoryboard)).unwrap();
        assert_eq!(p.step_count(), 1);
    }

    #[test]
    fn add_step_with_prior_input_ok() {
        let mut p = CompositionPlan::new();
        p.add_step(PlanStep::new(1, NomKind::MediaStoryboard)).unwrap();
        let s2 = PlanStep::new(2, NomKind::MediaVideo).with_input(1, None);
        p.add_step(s2).unwrap();
        assert_eq!(p.step_count(), 2);
    }

    #[test]
    fn add_step_duplicate_id_errors() {
        let mut p = CompositionPlan::new();
        p.add_step(PlanStep::new(1, NomKind::MediaImage)).unwrap();
        let err = p.add_step(PlanStep::new(1, NomKind::MediaAudio)).unwrap_err();
        assert!(matches!(err, PlanError::DuplicateStepId(1)));
    }

    #[test]
    fn add_step_unknown_input_errors() {
        let mut p = CompositionPlan::new();
        let s = PlanStep::new(2, NomKind::MediaVideo).with_input(99, None);
        let err = p.add_step(s).unwrap_err();
        assert!(matches!(err, PlanError::UnknownInput { step: 2, referenced: 99 }));
    }

    // ── set_final ─────────────────────────────────────────────────────────────

    #[test]
    fn set_final_existing_step_ok() {
        let mut p = CompositionPlan::new();
        p.add_step(PlanStep::new(1, NomKind::MediaVideo)).unwrap();
        p.set_final(1).unwrap();
        assert_eq!(p.final_step, Some(1));
    }

    #[test]
    fn set_final_unknown_step_errors() {
        let mut p = CompositionPlan::new();
        let err = p.set_final(42).unwrap_err();
        assert!(matches!(err, PlanError::UnknownStep(42)));
    }

    // ── lookup ────────────────────────────────────────────────────────────────

    #[test]
    fn step_lookup_hit_and_miss() {
        let mut p = CompositionPlan::new();
        p.add_step(PlanStep::new(5, NomKind::MediaAudio)).unwrap();
        assert!(p.step(5).is_some());
        assert!(p.step(99).is_none());
    }

    // ── graph traversal ───────────────────────────────────────────────────────

    #[test]
    fn predecessors_and_successors_linear_chain() {
        let mut p = CompositionPlan::new();
        p.add_step(PlanStep::new(1, NomKind::MediaStoryboard)).unwrap();
        p.add_step(PlanStep::new(2, NomKind::MediaVideo).with_input(1, None)).unwrap();
        p.add_step(PlanStep::new(3, NomKind::MediaAudio).with_input(2, None)).unwrap();

        assert_eq!(p.predecessors(1), vec![]);
        assert_eq!(p.predecessors(2), vec![1]);
        assert_eq!(p.predecessors(3), vec![2]);

        assert_eq!(p.successors(1), vec![2]);
        assert_eq!(p.successors(2), vec![3]);
        assert_eq!(p.successors(3), vec![]);
    }

    // ── execution_order ───────────────────────────────────────────────────────

    #[test]
    fn execution_order_linear_three_steps() {
        let mut p = CompositionPlan::new();
        p.add_step(PlanStep::new(1, NomKind::MediaStoryboard)).unwrap();
        p.add_step(PlanStep::new(2, NomKind::MediaVideo).with_input(1, None)).unwrap();
        p.add_step(PlanStep::new(3, NomKind::MediaAudio).with_input(2, None)).unwrap();
        let order = p.execution_order().unwrap();
        assert_eq!(order, vec![1, 2, 3]);
    }

    #[test]
    fn execution_order_diamond() {
        // 1 → 2, 1 → 3, 2 → 4, 3 → 4
        let mut p = CompositionPlan::new();
        p.add_step(PlanStep::new(1, NomKind::MediaStoryboard)).unwrap();
        p.add_step(PlanStep::new(2, NomKind::MediaImage).with_input(1, None)).unwrap();
        p.add_step(PlanStep::new(3, NomKind::MediaAudio).with_input(1, None)).unwrap();
        p.add_step(
            PlanStep::new(4, NomKind::MediaVideo)
                .with_input(2, None)
                .with_input(3, None),
        ).unwrap();
        let order = p.execution_order().unwrap();
        assert_eq!(order[0], 1);
        assert_eq!(*order.last().unwrap(), 4);
        assert_eq!(order.len(), 4);
    }

    #[test]
    fn execution_order_cycle_returns_error() {
        // Bypass add_step validation to inject a cycle: 1 → 2 → 1
        let mut p = CompositionPlan::new();
        p.steps.push(PlanStep {
            id: 1,
            kind: NomKind::MediaVideo,
            inputs: vec![(2, None)],
            params: vec![],
            output_key: None,
        });
        p.steps.push(PlanStep {
            id: 2,
            kind: NomKind::MediaAudio,
            inputs: vec![(1, None)],
            params: vec![],
            output_key: None,
        });
        let err = p.execution_order().unwrap_err();
        assert!(matches!(err, PlanError::CyclicDependencies(_)));
    }

    // ── step_count ────────────────────────────────────────────────────────────

    #[test]
    fn step_count_accurate() {
        let mut p = CompositionPlan::new();
        assert_eq!(p.step_count(), 0);
        p.add_step(PlanStep::new(1, NomKind::DataQuery)).unwrap();
        assert_eq!(p.step_count(), 1);
        p.add_step(PlanStep::new(2, NomKind::DataTransform).with_input(1, None)).unwrap();
        assert_eq!(p.step_count(), 2);
    }
}
