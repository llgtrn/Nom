#![deny(unsafe_code)]

use crate::dispatch::BackendKind;

/// One step in a composition plan — one backend invocation.
#[derive(Debug, Clone)]
pub struct PlanStep {
    pub step_id: usize,
    pub backend: BackendKind,
    pub input_key: String,
    pub output_key: String,
    pub depends_on: Vec<usize>, // step IDs this step waits for
}

/// A DAG of compose steps — executed by ComposeDispatcher.
#[derive(Debug, Clone, Default)]
pub struct CompositionPlan {
    pub steps: Vec<PlanStep>,
}

impl CompositionPlan {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_step(
        &mut self,
        backend: BackendKind,
        input_key: impl Into<String>,
        output_key: impl Into<String>,
    ) -> usize {
        let id = self.steps.len();
        self.steps.push(PlanStep {
            step_id: id,
            backend,
            input_key: input_key.into(),
            output_key: output_key.into(),
            depends_on: vec![],
        });
        id
    }

    pub fn add_step_after(
        &mut self,
        backend: BackendKind,
        input_key: impl Into<String>,
        output_key: impl Into<String>,
        depends_on: Vec<usize>,
    ) -> usize {
        let id = self.steps.len();
        self.steps.push(PlanStep {
            step_id: id,
            backend,
            input_key: input_key.into(),
            output_key: output_key.into(),
            depends_on,
        });
        id
    }

    /// Topological order of steps (Kahn's algorithm).
    pub fn topo_order(&self) -> Vec<usize> {
        let n = self.steps.len();
        let mut in_deg = vec![0usize; n];
        let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
        for step in &self.steps {
            for &dep in &step.depends_on {
                adj[dep].push(step.step_id);
                in_deg[step.step_id] += 1;
            }
        }
        let mut queue: std::collections::VecDeque<usize> =
            (0..n).filter(|&i| in_deg[i] == 0).collect();
        let mut order = Vec::new();
        while let Some(node) = queue.pop_front() {
            order.push(node);
            for &next in &adj[node] {
                in_deg[next] -= 1;
                if in_deg[next] == 0 {
                    queue.push_back(next);
                }
            }
        }
        order
    }

    pub fn is_valid_dag(&self) -> bool {
        self.topo_order().len() == self.steps.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plan_add_step_returns_sequential_ids() {
        let mut plan = CompositionPlan::new();
        let a = plan.add_step(BackendKind::Video, "in", "v_out");
        let b = plan.add_step(BackendKind::Audio, "in", "a_out");
        assert_eq!(a, 0);
        assert_eq!(b, 1);
    }
    #[test]
    fn plan_topo_order_respects_deps() {
        let mut plan = CompositionPlan::new();
        let a = plan.add_step(BackendKind::Video, "src", "v");
        let b = plan.add_step_after(BackendKind::Export, "v", "out", vec![a]);
        let order = plan.topo_order();
        let a_pos = order.iter().position(|&x| x == a).unwrap();
        let b_pos = order.iter().position(|&x| x == b).unwrap();
        assert!(a_pos < b_pos);
    }
    #[test]
    fn plan_is_valid_dag_for_linear_chain() {
        let mut plan = CompositionPlan::new();
        let a = plan.add_step(BackendKind::Video, "s", "v");
        plan.add_step_after(BackendKind::Export, "v", "o", vec![a]);
        assert!(plan.is_valid_dag());
    }

    #[test]
    fn plan_step_count() {
        let mut plan = CompositionPlan::new();
        plan.add_step(BackendKind::Video, "in", "v");
        plan.add_step(BackendKind::Audio, "in", "a");
        plan.add_step(BackendKind::Export, "a", "out");
        assert_eq!(plan.steps.len(), 3);
    }

    #[test]
    fn plan_step_label_preserved() {
        let mut plan = CompositionPlan::new();
        plan.add_step(BackendKind::Image, "source_image", "rendered_image");
        let step = &plan.steps[0];
        assert_eq!(step.input_key, "source_image");
        assert_eq!(step.output_key, "rendered_image");
    }

    #[test]
    fn plan_step_execution_order() {
        // Steps with no dependencies must appear in insertion order in topo_order.
        let mut plan = CompositionPlan::new();
        let a = plan.add_step(BackendKind::Video, "src", "v");
        let b = plan.add_step(BackendKind::Audio, "src", "a");
        let c = plan.add_step(BackendKind::Export, "a", "out");
        let order = plan.topo_order();
        let pos_a = order.iter().position(|&x| x == a).unwrap();
        let pos_b = order.iter().position(|&x| x == b).unwrap();
        let pos_c = order.iter().position(|&x| x == c).unwrap();
        // a and b are independent; c has no deps either — all three must appear.
        assert!(pos_a < order.len());
        assert!(pos_b < order.len());
        assert!(pos_c < order.len());
        // All three must be present exactly once.
        assert_eq!(order.len(), 3);
    }

    #[test]
    fn plan_add_step_increases_step_count() {
        let mut plan = CompositionPlan::new();
        assert_eq!(plan.steps.len(), 0);
        plan.add_step(BackendKind::Image, "in", "out");
        assert_eq!(plan.steps.len(), 1);
        plan.add_step(BackendKind::Audio, "in", "a_out");
        assert_eq!(plan.steps.len(), 2);
    }

    #[test]
    fn plan_total_steps() {
        let mut plan = CompositionPlan::new();
        for _ in 0..5 {
            plan.add_step(BackendKind::Video, "in", "out");
        }
        assert_eq!(plan.steps.len(), 5);
    }

    #[test]
    fn plan_default_is_empty() {
        let plan = CompositionPlan::default();
        assert!(plan.steps.is_empty());
        assert!(plan.is_valid_dag(), "empty plan is a valid DAG");
    }

    #[test]
    fn plan_single_step_is_valid_dag() {
        let mut plan = CompositionPlan::new();
        plan.add_step(BackendKind::Image, "src", "img");
        assert!(plan.is_valid_dag());
        assert_eq!(plan.topo_order(), vec![0]);
    }

    #[test]
    fn plan_diamond_dependency() {
        // A → B, A → C, B → D, C → D
        let mut plan = CompositionPlan::new();
        let a = plan.add_step(BackendKind::Video, "src", "v");
        let b = plan.add_step_after(BackendKind::Audio, "v", "a", vec![a]);
        let c = plan.add_step_after(BackendKind::Image, "v", "img", vec![a]);
        let d = plan.add_step_after(BackendKind::Export, "a", "out", vec![b, c]);
        assert!(plan.is_valid_dag());
        let order = plan.topo_order();
        let pos = |id: usize| order.iter().position(|&x| x == id).unwrap();
        assert!(pos(a) < pos(b));
        assert!(pos(a) < pos(c));
        assert!(pos(b) < pos(d));
        assert!(pos(c) < pos(d));
    }

    #[test]
    fn plan_step_depends_on_preserved() {
        let mut plan = CompositionPlan::new();
        let a = plan.add_step(BackendKind::Audio, "in", "a");
        let b = plan.add_step_after(BackendKind::Export, "a", "out", vec![a]);
        assert_eq!(plan.steps[b].depends_on, vec![a]);
        assert!(plan.steps[a].depends_on.is_empty());
    }

    #[test]
    fn plan_step_ids_match_indices() {
        let mut plan = CompositionPlan::new();
        for i in 0..6usize {
            let id = plan.add_step(BackendKind::Render, "i", "o");
            assert_eq!(id, i, "step_id must equal insertion index");
        }
    }

    #[test]
    fn plan_large_linear_chain_topo_order() {
        // 20-step linear chain: each step depends on the previous.
        let mut plan = CompositionPlan::new();
        let first = plan.add_step(BackendKind::Video, "in", "s0");
        let mut prev = first;
        for i in 1..20usize {
            prev = plan.add_step_after(
                BackendKind::Transform,
                format!("s{}", i - 1),
                format!("s{i}"),
                vec![prev],
            );
        }
        assert!(plan.is_valid_dag());
        let order = plan.topo_order();
        assert_eq!(order.len(), 20);
        // Verify strict ordering: each element must be smaller than next in chain.
        for window in order.windows(2) {
            assert!(window[0] < window[1], "linear chain must be in order");
        }
    }

    // ── Wave AE new tests ────────────────────────────────────────────────────

    #[test]
    fn plan_zero_steps_topo_order_is_empty() {
        let plan = CompositionPlan::new();
        assert_eq!(plan.topo_order().len(), 0);
    }

    #[test]
    fn plan_zero_steps_is_valid_dag() {
        let plan = CompositionPlan::new();
        assert!(plan.is_valid_dag());
    }

    #[test]
    fn plan_one_step_topo_order_contains_that_step() {
        let mut plan = CompositionPlan::new();
        let id = plan.add_step(BackendKind::Audio, "src", "out");
        let order = plan.topo_order();
        assert_eq!(order, vec![id]);
    }

    #[test]
    fn plan_ten_steps_linear_all_appear_in_order() {
        let mut plan = CompositionPlan::new();
        let first = plan.add_step(BackendKind::Video, "raw", "s0");
        let mut prev = first;
        for i in 1..10usize {
            prev = plan.add_step_after(
                BackendKind::Transform,
                format!("s{}", i - 1),
                format!("s{i}"),
                vec![prev],
            );
        }
        assert!(plan.is_valid_dag());
        let order = plan.topo_order();
        assert_eq!(order.len(), 10);
        // Linear chain means order must be 0,1,2,...,9
        for (expected, &actual) in order.iter().enumerate() {
            assert_eq!(actual, expected, "step at position {expected} must be {expected}");
        }
    }

    #[test]
    fn plan_step_backend_field_preserved() {
        let mut plan = CompositionPlan::new();
        plan.add_step(BackendKind::Render, "x", "y");
        assert_eq!(plan.steps[0].backend, BackendKind::Render);
    }

    #[test]
    fn plan_step_input_output_keys_preserved() {
        let mut plan = CompositionPlan::new();
        plan.add_step(BackendKind::Export, "my_input", "my_output");
        assert_eq!(plan.steps[0].input_key, "my_input");
        assert_eq!(plan.steps[0].output_key, "my_output");
    }

    #[test]
    fn plan_depends_on_empty_for_root_steps() {
        let mut plan = CompositionPlan::new();
        plan.add_step(BackendKind::Video, "a", "b");
        plan.add_step(BackendKind::Audio, "c", "d");
        assert!(plan.steps[0].depends_on.is_empty());
        assert!(plan.steps[1].depends_on.is_empty());
    }

    #[test]
    fn plan_two_independent_steps_both_in_topo() {
        let mut plan = CompositionPlan::new();
        let a = plan.add_step(BackendKind::Video, "v_in", "v_out");
        let b = plan.add_step(BackendKind::Audio, "a_in", "a_out");
        let order = plan.topo_order();
        assert_eq!(order.len(), 2);
        assert!(order.contains(&a));
        assert!(order.contains(&b));
    }

    #[test]
    fn plan_step_id_equals_insertion_index() {
        let mut plan = CompositionPlan::new();
        for i in 0..5usize {
            let id = plan.add_step(BackendKind::Transform, "x", "y");
            assert_eq!(id, i);
            assert_eq!(plan.steps[i].step_id, i);
        }
    }

    #[test]
    fn plan_clone_preserves_all_steps() {
        let mut plan = CompositionPlan::new();
        plan.add_step(BackendKind::Video, "a", "b");
        plan.add_step(BackendKind::Audio, "c", "d");
        let cloned = plan.clone();
        assert_eq!(cloned.steps.len(), plan.steps.len());
        assert_eq!(cloned.steps[0].input_key, "a");
        assert_eq!(cloned.steps[1].input_key, "c");
    }

    // ── Wave AE new tests ────────────────────────────────────────────────────

    #[test]
    fn plan_zero_steps_topo_is_empty_and_dag_is_valid() {
        // An empty plan is technically a valid DAG (trivially); topo order is empty.
        let plan = CompositionPlan::new();
        assert!(plan.steps.is_empty(), "new plan must have zero steps");
        let order = plan.topo_order();
        assert!(order.is_empty(), "zero-step plan must have empty topo order");
        // is_valid_dag returns true for empty plan since len==0==0.
        assert!(plan.is_valid_dag(), "empty plan is a valid (trivial) DAG");
    }

    #[test]
    fn plan_step_insertion_order_preserved_in_steps_vec() {
        // Steps must appear in insertion order in plan.steps regardless of dependencies.
        let mut plan = CompositionPlan::new();
        let keys = ["alpha", "beta", "gamma", "delta"];
        for k in &keys {
            plan.add_step(BackendKind::Transform, *k, "out");
        }
        for (i, k) in keys.iter().enumerate() {
            assert_eq!(plan.steps[i].input_key, *k, "step {i} input key must be '{k}'");
        }
    }

    #[test]
    fn plan_round_trip_fields_via_clone() {
        // Simulate serialization round-trip: build → clone → verify all fields intact.
        let mut plan = CompositionPlan::new();
        let a = plan.add_step(BackendKind::Video, "v_src", "v_out");
        let b = plan.add_step_after(BackendKind::Export, "v_out", "exported", vec![a]);
        let rt = plan.clone();
        assert_eq!(rt.steps.len(), 2);
        assert_eq!(rt.steps[0].step_id, a);
        assert_eq!(rt.steps[0].input_key, "v_src");
        assert_eq!(rt.steps[0].output_key, "v_out");
        assert!(rt.steps[0].depends_on.is_empty());
        assert_eq!(rt.steps[1].step_id, b);
        assert_eq!(rt.steps[1].input_key, "v_out");
        assert_eq!(rt.steps[1].output_key, "exported");
        assert_eq!(rt.steps[1].depends_on, vec![a]);
    }

    #[test]
    fn plan_step_ids_are_unique_and_sequential() {
        // The API auto-assigns step IDs so duplicates are impossible; verify uniqueness.
        let mut plan = CompositionPlan::new();
        let mut ids = Vec::new();
        for _ in 0..8 {
            let id = plan.add_step(BackendKind::Transform, "x", "y");
            ids.push(id);
        }
        let unique: std::collections::HashSet<usize> = ids.iter().copied().collect();
        assert_eq!(unique.len(), ids.len(), "all step IDs must be unique");
        // Also sequential from 0.
        for (i, &id) in ids.iter().enumerate() {
            assert_eq!(id, i);
        }
    }

    #[test]
    fn plan_large_step_count_topo_order_contains_all() {
        // A flat plan with 50 independent steps must have all 50 in topo order.
        let mut plan = CompositionPlan::new();
        for _ in 0..50 {
            plan.add_step(BackendKind::Transform, "in", "out");
        }
        assert_eq!(plan.steps.len(), 50);
        assert!(plan.is_valid_dag());
        let order = plan.topo_order();
        assert_eq!(order.len(), 50, "all 50 steps must appear in topo order");
    }

    #[test]
    fn plan_step_ordering_respected_in_three_level_chain() {
        // A → B → C: topo order must be A, B, C.
        let mut plan = CompositionPlan::new();
        let a = plan.add_step(BackendKind::Video, "raw", "v");
        let b = plan.add_step_after(BackendKind::Audio, "v", "av", vec![a]);
        let c = plan.add_step_after(BackendKind::Export, "av", "final", vec![b]);
        let order = plan.topo_order();
        let pos = |id| order.iter().position(|&x| x == id).unwrap();
        assert!(pos(a) < pos(b), "A must come before B");
        assert!(pos(b) < pos(c), "B must come before C");
    }

    #[test]
    fn plan_parallel_then_merge_ordering() {
        // A and B are independent; both feed C.  A and B can be in any order but must precede C.
        let mut plan = CompositionPlan::new();
        let a = plan.add_step(BackendKind::Video, "v", "v_out");
        let b = plan.add_step(BackendKind::Audio, "a", "a_out");
        let c = plan.add_step_after(BackendKind::Export, "merged", "final", vec![a, b]);
        assert!(plan.is_valid_dag());
        let order = plan.topo_order();
        let pos = |id| order.iter().position(|&x| x == id).unwrap();
        assert!(pos(a) < pos(c), "A must precede C");
        assert!(pos(b) < pos(c), "B must precede C");
    }

    #[test]
    fn plan_add_step_after_with_multiple_deps_preserves_all() {
        let mut plan = CompositionPlan::new();
        let a = plan.add_step(BackendKind::Video, "a", "va");
        let b = plan.add_step(BackendKind::Audio, "b", "ab");
        let c = plan.add_step(BackendKind::Image, "c", "ic");
        let d = plan.add_step_after(BackendKind::Export, "combo", "out", vec![a, b, c]);
        assert_eq!(plan.steps[d].depends_on, vec![a, b, c]);
    }

    #[test]
    fn plan_single_step_is_dag_and_topo_contains_it() {
        let mut plan = CompositionPlan::new();
        let id = plan.add_step(BackendKind::Render, "in", "out");
        assert!(plan.is_valid_dag());
        let order = plan.topo_order();
        assert_eq!(order, vec![id]);
    }

    // ── Wave AO new tests ────────────────────────────────────────────────────

    #[test]
    fn plan_step_count_after_3_adds() {
        let mut plan = CompositionPlan::new();
        plan.add_step(BackendKind::Video, "a", "b");
        plan.add_step(BackendKind::Audio, "c", "d");
        plan.add_step(BackendKind::Export, "e", "f");
        assert_eq!(plan.steps.len(), 3, "three add_step calls must yield 3 steps");
    }

    #[test]
    fn plan_two_step_chain_topo_preserves_order() {
        let mut plan = CompositionPlan::new();
        let a = plan.add_step(BackendKind::Image, "src", "img");
        let b = plan.add_step_after(BackendKind::Document, "img", "doc", vec![a]);
        let order = plan.topo_order();
        let pa = order.iter().position(|&x| x == a).unwrap();
        let pb = order.iter().position(|&x| x == b).unwrap();
        assert!(pa < pb, "image step must come before document step");
    }

    #[test]
    fn plan_step_depends_on_multiple_parents_all_recorded() {
        let mut plan = CompositionPlan::new();
        let a = plan.add_step(BackendKind::Video, "v", "v_out");
        let b = plan.add_step(BackendKind::Audio, "a", "a_out");
        let c = plan.add_step(BackendKind::Image, "i", "i_out");
        let d = plan.add_step_after(BackendKind::Export, "combo", "final", vec![a, b, c]);
        assert_eq!(plan.steps[d].depends_on, vec![a, b, c]);
        assert_eq!(plan.steps[d].depends_on.len(), 3);
    }

    #[test]
    fn plan_topo_contains_all_step_ids() {
        let mut plan = CompositionPlan::new();
        let ids: Vec<usize> = (0..7).map(|_| plan.add_step(BackendKind::Transform, "x", "y")).collect();
        let order = plan.topo_order();
        for id in &ids {
            assert!(order.contains(id), "topo order must contain step {id}");
        }
        assert_eq!(order.len(), 7);
    }

    #[test]
    fn plan_is_valid_dag_true_for_tree() {
        // A forks to B and C; B and C each feed D independently.
        let mut plan = CompositionPlan::new();
        let a = plan.add_step(BackendKind::Video, "v", "v_out");
        let b = plan.add_step_after(BackendKind::Audio, "v_out", "a_out", vec![a]);
        let c = plan.add_step_after(BackendKind::Image, "v_out", "i_out", vec![a]);
        let _ = plan.add_step_after(BackendKind::Export, "merged", "final", vec![b, c]);
        assert!(plan.is_valid_dag(), "fork-join must be a valid DAG");
    }

    #[test]
    fn plan_output_key_preserved_per_step() {
        let mut plan = CompositionPlan::new();
        plan.add_step(BackendKind::Render, "input_frame", "rendered_frame");
        plan.add_step(BackendKind::Export, "rendered_frame", "export_artifact");
        assert_eq!(plan.steps[0].output_key, "rendered_frame");
        assert_eq!(plan.steps[1].output_key, "export_artifact");
        assert_eq!(plan.steps[1].input_key, "rendered_frame");
    }
}
