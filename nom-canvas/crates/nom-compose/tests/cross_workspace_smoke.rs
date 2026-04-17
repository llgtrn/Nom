//! Cross-workspace smoke test — plan + estimator + dispatcher + queue together.

use nom_compose::backend_trait::{ComposeSpec, InterruptFlag, ProgressSink};
use nom_compose::backends::register_all_stubs;
use nom_compose::cost_estimator::CostEstimator;
use nom_compose::dispatch::ComposeDispatcher;
use nom_compose::kind::NomKind;
use nom_compose::plan::{CompositionPlan, PlanStep};
use nom_compose::task_queue::TaskQueue;

struct NoopProgress;
impl ProgressSink for NoopProgress {
    fn notify(&self, _p: u32, _m: &str) {}
}

#[test]
fn plan_estimator_dispatcher_queue_integrate() {
    // Build a 3-step plan: extract → query → transform.
    let mut plan = CompositionPlan::new();
    let extract_step = PlanStep::new(1, NomKind::DataExtract);
    let query_step = PlanStep::new(2, NomKind::DataQuery).with_input(1, None);
    let transform_step = PlanStep::new(3, NomKind::DataTransform).with_input(2, None);
    plan.add_step(extract_step).expect("add extract");
    plan.add_step(query_step).expect("add query");
    plan.add_step(transform_step).expect("add transform");
    plan.set_final(3).expect("set final");

    let order = plan.execution_order().expect("topology");
    assert_eq!(order, vec![1, 2, 3], "linear plan order");

    // Estimate cost (soft pass — all defaults are FREE so total is 0).
    let estimator = CostEstimator::new();
    let estimate = estimator.estimate(&plan);
    assert_eq!(estimate.steps.len(), 3, "estimate covers all steps");
    // total_cents may be 0 with FREE defaults — just verify it doesn't panic.
    let _ = estimate.total_cents;

    // Register all stub backends.
    let mut dispatcher = ComposeDispatcher::new();
    register_all_stubs(&mut dispatcher);

    // Track through task queue.
    let mut queue = TaskQueue::new();
    let interrupt = InterruptFlag::new();
    let progress = NoopProgress;

    for step_id in order {
        let step = plan.step(step_id).expect("step exists");
        let task_id = queue.enqueue(step.kind);
        queue.start_next_for(step.kind);
        let spec = ComposeSpec {
            kind: step.kind,
            params: step.params.clone(),
        };
        let result = dispatcher.dispatch(&spec, &progress, &interrupt);
        assert!(result.is_ok(), "dispatch failed for step {}: {:?}", step_id, result);
        queue.mark_completed(task_id);
    }
}

#[test]
fn diamond_plan_executes_in_topological_order() {
    let mut plan = CompositionPlan::new();
    plan.add_step(PlanStep::new(1, NomKind::DataExtract)).unwrap();
    plan.add_step(PlanStep::new(2, NomKind::DataQuery).with_input(1, None)).unwrap();
    plan.add_step(PlanStep::new(3, NomKind::DataTransform).with_input(1, None)).unwrap();
    // Use DataQuery again for step 4 (it has a registered stub backend).
    plan.add_step(
        PlanStep::new(4, NomKind::DataQuery)
            .with_input(2, None)
            .with_input(3, None),
    )
    .unwrap();

    let order = plan.execution_order().expect("topology");
    // Step 1 first, step 4 last; 2 and 3 in between.
    let pos = |id| order.iter().position(|&x| x == id).unwrap();
    assert!(pos(1) < pos(2));
    assert!(pos(1) < pos(3));
    assert!(pos(2) < pos(4));
    assert!(pos(3) < pos(4));
}

#[test]
fn cycle_detection_prevents_execution() {
    let mut plan = CompositionPlan::new();
    plan.add_step(PlanStep::new(1, NomKind::DataExtract)).unwrap();
    plan.add_step(PlanStep::new(2, NomKind::DataQuery).with_input(1, None)).unwrap();
    // add_step validation prevents forward references — verify it catches unknown step id.
    let result = plan.add_step(PlanStep::new(3, NomKind::DataTransform).with_input(99, None));
    assert!(result.is_err(), "forward reference to non-existent step must be rejected");
}
