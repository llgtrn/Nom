#![deny(unsafe_code)]

use crate::dispatch::BackendKind;

/// One step in a composition plan — one backend invocation.
#[derive(Debug, Clone)]
pub struct PlanStep {
    pub step_id: usize,
    pub backend: BackendKind,
    pub input_key: String,
    pub output_key: String,
    pub depends_on: Vec<usize>,  // step IDs this step waits for
}

/// A DAG of compose steps — executed by ComposeDispatcher.
#[derive(Debug, Clone, Default)]
pub struct CompositionPlan {
    pub steps: Vec<PlanStep>,
}

impl CompositionPlan {
    pub fn new() -> Self { Self::default() }

    pub fn add_step(&mut self, backend: BackendKind, input_key: impl Into<String>, output_key: impl Into<String>) -> usize {
        let id = self.steps.len();
        self.steps.push(PlanStep { step_id: id, backend, input_key: input_key.into(), output_key: output_key.into(), depends_on: vec![] });
        id
    }

    pub fn add_step_after(&mut self, backend: BackendKind, input_key: impl Into<String>, output_key: impl Into<String>, depends_on: Vec<usize>) -> usize {
        let id = self.steps.len();
        self.steps.push(PlanStep { step_id: id, backend, input_key: input_key.into(), output_key: output_key.into(), depends_on });
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
        let mut queue: std::collections::VecDeque<usize> = (0..n).filter(|&i| in_deg[i] == 0).collect();
        let mut order = Vec::new();
        while let Some(node) = queue.pop_front() {
            order.push(node);
            for &next in &adj[node] {
                in_deg[next] -= 1;
                if in_deg[next] == 0 { queue.push_back(next); }
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
        assert_eq!(a, 0); assert_eq!(b, 1);
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
}
