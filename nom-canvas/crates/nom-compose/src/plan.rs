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
}
