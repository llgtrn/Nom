//! Pre-execution cost estimation for composition plans.
#![deny(unsafe_code)]

use std::collections::HashMap;
use crate::kind::NomKind;
use crate::plan::{CompositionPlan, StepId};
use crate::vendor_trait::Cost;

#[derive(Clone, Debug, PartialEq)]
pub struct StepEstimate {
    pub step_id: StepId,
    pub kind: NomKind,
    pub input_tokens_est: u32,
    pub output_tokens_est: u32,
    pub estimated_cents: u64,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct PlanEstimate {
    pub steps: Vec<StepEstimate>,
    pub total_cents: u64,
    pub max_parallel_cents: u64,  // peak concurrent cost if fan-out executed simultaneously
}

#[derive(Default)]
pub struct CostEstimator {
    /// Per-kind default token counts (can be overridden by caller).
    default_input_tokens: HashMap<NomKind, u32>,
    default_output_tokens: HashMap<NomKind, u32>,
    /// Per-kind cost.
    costs: HashMap<NomKind, Cost>,
}

impl CostEstimator {
    pub fn new() -> Self {
        let mut e = Self::default();
        e.seed_defaults();
        e
    }

    fn seed_defaults(&mut self) {
        // Conservative defaults — callers override per-step via `set_default_tokens`.
        for (kind, i_tok, o_tok) in [
            (NomKind::MediaVideo, 500u32, 0u32),
            (NomKind::MediaImage, 80, 0),
            (NomKind::MediaAudio, 200, 0),
            (NomKind::Media3D, 100, 0),
            (NomKind::MediaStoryboard, 1000, 500),
            (NomKind::MediaNovelVideo, 5000, 2000),
            (NomKind::ScreenWeb, 200, 0),
            (NomKind::ScreenNative, 100, 0),
            (NomKind::DataExtract, 0, 500),
            (NomKind::DataQuery, 300, 200),
            (NomKind::DataTransform, 0, 100),
            (NomKind::ConceptDocument, 800, 400),
            (NomKind::ScenarioWorkflow, 100, 50),
        ] {
            self.default_input_tokens.insert(kind, i_tok);
            self.default_output_tokens.insert(kind, o_tok);
        }
    }

    pub fn set_cost(&mut self, kind: NomKind, cost: Cost) {
        self.costs.insert(kind, cost);
    }

    pub fn set_default_tokens(&mut self, kind: NomKind, input: u32, output: u32) {
        self.default_input_tokens.insert(kind, input);
        self.default_output_tokens.insert(kind, output);
    }

    pub fn estimate(&self, plan: &CompositionPlan) -> PlanEstimate {
        let mut steps: Vec<StepEstimate> = Vec::with_capacity(plan.steps.len());
        for step in &plan.steps {
            let input_tokens = self.default_input_tokens.get(&step.kind).copied().unwrap_or(0);
            let output_tokens = self.default_output_tokens.get(&step.kind).copied().unwrap_or(0);
            let cost = self.costs.get(&step.kind).copied().unwrap_or(Cost::FREE);
            let cents = cost.total_cents(input_tokens, output_tokens);
            steps.push(StepEstimate {
                step_id: step.id,
                kind: step.kind,
                input_tokens_est: input_tokens,
                output_tokens_est: output_tokens,
                estimated_cents: cents,
            });
        }
        let total_cents: u64 = steps.iter().map(|s| s.estimated_cents).sum();
        // max_parallel_cents = max cents across all successor-independent groups.
        // Simpler heuristic: max estimated_cents of any single step.
        let max_parallel_cents = steps.iter().map(|s| s.estimated_cents).max().unwrap_or(0);
        PlanEstimate { steps, total_cents, max_parallel_cents }
    }

    /// Shortcut: estimate a single kind with given token budgets.
    pub fn estimate_one(&self, kind: NomKind, input_tokens: u32, output_tokens: u32) -> u64 {
        let cost = self.costs.get(&kind).copied().unwrap_or(Cost::FREE);
        cost.total_cents(input_tokens, output_tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::PlanStep;

    // ── seed_defaults ─────────────────────────────────────────────────────────

    #[test]
    fn new_seeds_default_tokens_for_at_least_10_kinds() {
        let e = CostEstimator::new();
        let kinds = [
            NomKind::MediaVideo,
            NomKind::MediaImage,
            NomKind::MediaAudio,
            NomKind::Media3D,
            NomKind::MediaStoryboard,
            NomKind::MediaNovelVideo,
            NomKind::ScreenWeb,
            NomKind::ScreenNative,
            NomKind::DataExtract,
            NomKind::DataQuery,
        ];
        for kind in &kinds {
            assert!(
                e.default_input_tokens.contains_key(kind) || e.default_output_tokens.contains_key(kind),
                "{:?} has no default tokens",
                kind,
            );
        }
        assert!(e.default_input_tokens.len() >= 10);
    }

    // ── set_cost ──────────────────────────────────────────────────────────────

    #[test]
    fn set_cost_overrides_per_kind_cost() {
        let mut e = CostEstimator::new();
        let custom = Cost {
            cents_per_1k_input_tokens: 5,
            cents_per_1k_output_tokens: 10,
            fixed_cents_per_request: 2,
        };
        e.set_cost(NomKind::MediaVideo, custom);
        assert_eq!(e.costs[&NomKind::MediaVideo], custom);
    }

    // ── set_default_tokens ────────────────────────────────────────────────────

    #[test]
    fn set_default_tokens_overrides_per_kind_tokens() {
        let mut e = CostEstimator::new();
        e.set_default_tokens(NomKind::MediaVideo, 9999, 8888);
        assert_eq!(e.default_input_tokens[&NomKind::MediaVideo], 9999);
        assert_eq!(e.default_output_tokens[&NomKind::MediaVideo], 8888);
    }

    // ── estimate empty plan ───────────────────────────────────────────────────

    #[test]
    fn estimate_empty_plan_returns_zero_total_and_empty_steps() {
        let e = CostEstimator::new();
        let plan = CompositionPlan::new();
        let est = e.estimate(&plan);
        assert!(est.steps.is_empty());
        assert_eq!(est.total_cents, 0);
        assert_eq!(est.max_parallel_cents, 0);
    }

    // ── estimate single step ──────────────────────────────────────────────────

    #[test]
    fn estimate_single_step_correct_cents() {
        let mut e = CostEstimator::new();
        let cost = Cost {
            cents_per_1k_input_tokens: 2,
            cents_per_1k_output_tokens: 4,
            fixed_cents_per_request: 1,
        };
        // 1000 in @ 2c/1k + 0 out + 1 fixed = 3
        e.set_cost(NomKind::MediaVideo, cost);
        e.set_default_tokens(NomKind::MediaVideo, 1000, 0);

        let mut plan = CompositionPlan::new();
        plan.add_step(PlanStep::new(1, NomKind::MediaVideo)).unwrap();
        let est = e.estimate(&plan);

        assert_eq!(est.steps.len(), 1);
        assert_eq!(est.steps[0].step_id, 1);
        assert_eq!(est.steps[0].estimated_cents, 3);
        assert_eq!(est.total_cents, 3);
    }

    // ── estimate 3-step plan ──────────────────────────────────────────────────

    #[test]
    fn estimate_three_step_plan_sums_cents_correctly() {
        let mut e = CostEstimator::new();
        let cost = Cost {
            cents_per_1k_input_tokens: 0,
            cents_per_1k_output_tokens: 0,
            fixed_cents_per_request: 10,
        };
        e.set_cost(NomKind::MediaVideo, cost);
        e.set_cost(NomKind::MediaAudio, cost);
        e.set_cost(NomKind::MediaImage, cost);

        let mut plan = CompositionPlan::new();
        plan.add_step(PlanStep::new(1, NomKind::MediaVideo)).unwrap();
        plan.add_step(PlanStep::new(2, NomKind::MediaAudio).with_input(1, None)).unwrap();
        plan.add_step(PlanStep::new(3, NomKind::MediaImage).with_input(2, None)).unwrap();

        let est = e.estimate(&plan);
        assert_eq!(est.steps.len(), 3);
        assert_eq!(est.total_cents, 30);
    }

    // ── FREE cost → 0 cents ───────────────────────────────────────────────────

    #[test]
    fn estimate_uses_free_cost_when_unset_yields_zero_cents() {
        let e = CostEstimator::new();
        // No cost set for DataTransform — Cost::FREE applies.
        let mut plan = CompositionPlan::new();
        plan.add_step(PlanStep::new(1, NomKind::DataTransform)).unwrap();
        let est = e.estimate(&plan);
        assert_eq!(est.steps[0].estimated_cents, 0);
        assert_eq!(est.total_cents, 0);
    }

    // ── estimate_one ──────────────────────────────────────────────────────────

    #[test]
    fn estimate_one_direct_with_given_tokens() {
        let mut e = CostEstimator::new();
        let cost = Cost {
            cents_per_1k_input_tokens: 3,
            cents_per_1k_output_tokens: 6,
            fixed_cents_per_request: 0,
        };
        e.set_cost(NomKind::ConceptDocument, cost);
        // 2000 in @ 3c/1k = 6, 1000 out @ 6c/1k = 6 → 12
        let cents = e.estimate_one(NomKind::ConceptDocument, 2000, 1000);
        assert_eq!(cents, 12);
    }

    // ── max_parallel_cents ────────────────────────────────────────────────────

    #[test]
    fn max_parallel_cents_equals_max_of_step_estimates() {
        let mut e = CostEstimator::new();
        e.set_cost(NomKind::MediaVideo, Cost { cents_per_1k_input_tokens: 0, cents_per_1k_output_tokens: 0, fixed_cents_per_request: 7 });
        e.set_cost(NomKind::MediaAudio, Cost { cents_per_1k_input_tokens: 0, cents_per_1k_output_tokens: 0, fixed_cents_per_request: 3 });
        e.set_cost(NomKind::MediaImage, Cost { cents_per_1k_input_tokens: 0, cents_per_1k_output_tokens: 0, fixed_cents_per_request: 15 });

        let mut plan = CompositionPlan::new();
        plan.add_step(PlanStep::new(1, NomKind::MediaVideo)).unwrap();
        plan.add_step(PlanStep::new(2, NomKind::MediaAudio).with_input(1, None)).unwrap();
        plan.add_step(PlanStep::new(3, NomKind::MediaImage).with_input(2, None)).unwrap();

        let est = e.estimate(&plan);
        assert_eq!(est.max_parallel_cents, 15);
    }
}
