/// Plan step variants representing query execution stages.
#[derive(Debug, Clone, PartialEq)]
pub enum PlanStep {
    Scan,
    Filter,
    Project,
    Aggregate,
    Sort,
}

impl PlanStep {
    /// Returns true if this step is a terminal output step (Project or Aggregate).
    pub fn is_terminal(&self) -> bool {
        matches!(self, PlanStep::Project | PlanStep::Aggregate)
    }

    /// Returns a numeric code for this step.
    /// Scan=0, Filter=1, Project=2, Aggregate=3, Sort=4.
    pub fn step_code(&self) -> u8 {
        match self {
            PlanStep::Scan => 0,
            PlanStep::Filter => 1,
            PlanStep::Project => 2,
            PlanStep::Aggregate => 3,
            PlanStep::Sort => 4,
        }
    }
}

/// A single node in a query plan.
#[derive(Debug, Clone)]
pub struct QueryNode {
    pub step: PlanStep,
    pub estimated_rows: u64,
    pub cost: f64,
}

impl QueryNode {
    /// Returns true if this node's cost exceeds 100.0.
    pub fn is_expensive(&self) -> bool {
        self.cost > 100.0
    }

    /// Returns a human-readable label for this node.
    pub fn node_label(&self) -> String {
        format!(
            "[{}] ~{} rows ${:.1}",
            self.step.step_code(),
            self.estimated_rows,
            self.cost
        )
    }
}

/// An ordered query plan composed of QueryNodes.
#[derive(Debug, Clone)]
pub struct QueryPlan {
    pub nodes: Vec<QueryNode>,
    pub plan_id: u32,
}

impl QueryPlan {
    /// Creates an empty query plan with the given ID.
    pub fn new(plan_id: u32) -> Self {
        Self {
            nodes: Vec::new(),
            plan_id,
        }
    }

    /// Appends a node to the plan.
    pub fn add_node(&mut self, node: QueryNode) {
        self.nodes.push(node);
    }

    /// Sums the cost of all nodes in the plan.
    pub fn total_cost(&self) -> f64 {
        self.nodes.iter().map(|n| n.cost).sum()
    }

    /// Returns references to all nodes whose cost exceeds 100.0.
    pub fn expensive_nodes(&self) -> Vec<&QueryNode> {
        self.nodes.iter().filter(|n| n.is_expensive()).collect()
    }

    /// Returns true if any node is an Aggregate step.
    pub fn has_aggregate(&self) -> bool {
        self.nodes.iter().any(|n| n.step == PlanStep::Aggregate)
    }
}

/// Optimizer that transforms a plan to reduce redundant operations.
pub struct PlanOptimizer;

impl PlanOptimizer {
    /// Removes consecutive duplicate Sort steps, keeping only the first Sort in any run.
    pub fn remove_redundant_sorts(nodes: Vec<QueryNode>) -> Vec<QueryNode> {
        let mut result: Vec<QueryNode> = Vec::with_capacity(nodes.len());
        let mut last_was_sort = false;
        for node in nodes {
            if node.step == PlanStep::Sort {
                if last_was_sort {
                    // skip this consecutive Sort
                    continue;
                }
                last_was_sort = true;
            } else {
                last_was_sort = false;
            }
            result.push(node);
        }
        result
    }

    /// Estimates the speedup ratio of the optimized plan over the original.
    /// Returns original.total_cost() / optimized.total_cost().max(0.001).
    pub fn estimated_speedup(original: &QueryPlan, optimized: &QueryPlan) -> f64 {
        original.total_cost() / optimized.total_cost().max(0.001)
    }
}

/// Manages a collection of query plans and enables selection of the cheapest.
pub struct QueryPlanner {
    pub plans: Vec<QueryPlan>,
}

impl QueryPlanner {
    /// Creates a planner with no plans.
    pub fn new() -> Self {
        Self { plans: Vec::new() }
    }

    /// Adds a plan to the collection.
    pub fn add_plan(&mut self, plan: QueryPlan) {
        self.plans.push(plan);
    }

    /// Returns a reference to the plan with the lowest total cost, or None if empty.
    pub fn cheapest_plan(&self) -> Option<&QueryPlan> {
        self.plans
            .iter()
            .min_by(|a, b| a.total_cost().partial_cmp(&b.total_cost()).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// Returns the number of plans in the collection.
    pub fn plan_count(&self) -> usize {
        self.plans.len()
    }
}

impl Default for QueryPlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(step: PlanStep, estimated_rows: u64, cost: f64) -> QueryNode {
        QueryNode { step, estimated_rows, cost }
    }

    #[test]
    fn test_plan_step_is_terminal() {
        assert!(PlanStep::Project.is_terminal());
        assert!(PlanStep::Aggregate.is_terminal());
        assert!(!PlanStep::Scan.is_terminal());
        assert!(!PlanStep::Filter.is_terminal());
        assert!(!PlanStep::Sort.is_terminal());
    }

    #[test]
    fn test_plan_step_step_code() {
        assert_eq!(PlanStep::Scan.step_code(), 0);
        assert_eq!(PlanStep::Filter.step_code(), 1);
        assert_eq!(PlanStep::Project.step_code(), 2);
        assert_eq!(PlanStep::Aggregate.step_code(), 3);
        assert_eq!(PlanStep::Sort.step_code(), 4);
    }

    #[test]
    fn test_query_node_is_expensive() {
        let cheap = make_node(PlanStep::Scan, 100, 50.0);
        let pricey = make_node(PlanStep::Aggregate, 1000, 200.0);
        let boundary = make_node(PlanStep::Filter, 500, 100.0);
        assert!(!cheap.is_expensive());
        assert!(pricey.is_expensive());
        // exactly 100.0 is not > 100.0
        assert!(!boundary.is_expensive());
    }

    #[test]
    fn test_query_node_label_format() {
        let node = make_node(PlanStep::Sort, 42, 7.5);
        // Sort step_code = 4
        assert_eq!(node.node_label(), "[4] ~42 rows $7.5");
    }

    #[test]
    fn test_query_plan_total_cost() {
        let mut plan = QueryPlan::new(1);
        plan.add_node(make_node(PlanStep::Scan, 1000, 10.0));
        plan.add_node(make_node(PlanStep::Filter, 500, 5.5));
        plan.add_node(make_node(PlanStep::Project, 500, 2.0));
        let total = plan.total_cost();
        assert!((total - 17.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_query_plan_has_aggregate() {
        let mut plan_with = QueryPlan::new(2);
        plan_with.add_node(make_node(PlanStep::Scan, 100, 5.0));
        plan_with.add_node(make_node(PlanStep::Aggregate, 1, 50.0));
        assert!(plan_with.has_aggregate());

        let mut plan_without = QueryPlan::new(3);
        plan_without.add_node(make_node(PlanStep::Scan, 100, 5.0));
        plan_without.add_node(make_node(PlanStep::Project, 100, 3.0));
        assert!(!plan_without.has_aggregate());
    }

    #[test]
    fn test_plan_optimizer_remove_redundant_sorts() {
        let nodes = vec![
            make_node(PlanStep::Scan, 1000, 10.0),
            make_node(PlanStep::Sort, 1000, 20.0),
            make_node(PlanStep::Sort, 1000, 20.0), // consecutive duplicate — removed
            make_node(PlanStep::Filter, 500, 5.0),
            make_node(PlanStep::Sort, 500, 15.0),  // non-consecutive — kept
        ];
        let result = PlanOptimizer::remove_redundant_sorts(nodes);
        assert_eq!(result.len(), 4);
        // positions: Scan, Sort, Filter, Sort
        assert_eq!(result[0].step, PlanStep::Scan);
        assert_eq!(result[1].step, PlanStep::Sort);
        assert_eq!(result[2].step, PlanStep::Filter);
        assert_eq!(result[3].step, PlanStep::Sort);
    }

    #[test]
    fn test_plan_optimizer_estimated_speedup() {
        let mut original = QueryPlan::new(10);
        original.add_node(make_node(PlanStep::Sort, 1000, 100.0));
        original.add_node(make_node(PlanStep::Sort, 1000, 100.0));

        let mut optimized = QueryPlan::new(11);
        optimized.add_node(make_node(PlanStep::Sort, 1000, 100.0));

        let speedup = PlanOptimizer::estimated_speedup(&original, &optimized);
        assert!((speedup - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_query_planner_cheapest_plan() {
        let mut planner = QueryPlanner::new();
        assert!(planner.cheapest_plan().is_none());

        let mut expensive = QueryPlan::new(20);
        expensive.add_node(make_node(PlanStep::Scan, 5000, 300.0));

        let mut cheap = QueryPlan::new(21);
        cheap.add_node(make_node(PlanStep::Scan, 100, 10.0));

        planner.add_plan(expensive);
        planner.add_plan(cheap);

        assert_eq!(planner.plan_count(), 2);
        let cheapest = planner.cheapest_plan().expect("should have a cheapest plan");
        assert_eq!(cheapest.plan_id, 21);
        assert!((cheapest.total_cost() - 10.0).abs() < f64::EPSILON);
    }
}
