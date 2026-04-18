use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintKind {
    Equal,
    LessThan,
    GreaterThan,
    Range,
}

impl ConstraintKind {
    pub fn is_inequality(&self) -> bool {
        matches!(self, ConstraintKind::LessThan | ConstraintKind::GreaterThan)
    }

    pub fn kind_name(&self) -> &'static str {
        match self {
            ConstraintKind::Equal => "equal",
            ConstraintKind::LessThan => "less_than",
            ConstraintKind::GreaterThan => "greater_than",
            ConstraintKind::Range => "range",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintVar(pub u32);

impl ConstraintVar {
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }

    pub fn var_key(&self) -> String {
        format!("v{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct Constraint {
    pub lhs: ConstraintVar,
    pub rhs: ConstraintVar,
    pub kind: ConstraintKind,
}

impl Constraint {
    pub fn is_satisfied_by(&self, lhs_val: f64, rhs_val: f64) -> bool {
        match &self.kind {
            ConstraintKind::Equal => (lhs_val - rhs_val).abs() < 1e-9,
            ConstraintKind::LessThan => lhs_val < rhs_val,
            ConstraintKind::GreaterThan => lhs_val > rhs_val,
            ConstraintKind::Range => lhs_val <= rhs_val,
        }
    }

    pub fn label(&self) -> String {
        format!("{} {} {}", self.lhs.var_key(), self.kind.kind_name(), self.rhs.var_key())
    }
}

#[derive(Debug, Default)]
pub struct ConstraintGraph {
    pub constraints: Vec<Constraint>,
    pub var_count: u32,
}

impl ConstraintGraph {
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
            var_count: 0,
        }
    }

    pub fn add_var(&mut self) -> ConstraintVar {
        self.var_count += 1;
        ConstraintVar(self.var_count)
    }

    pub fn add_constraint(&mut self, c: Constraint) {
        self.constraints.push(c);
    }

    pub fn constraints_for(&self, var: &ConstraintVar) -> Vec<&Constraint> {
        let key = var.var_key();
        self.constraints
            .iter()
            .filter(|c| c.lhs.var_key() == key || c.rhs.var_key() == key)
            .collect()
    }
}

pub struct ConstraintSolver;

impl ConstraintSolver {
    pub fn unsatisfied<'a>(
        graph: &'a ConstraintGraph,
        values: &HashMap<u32, f64>,
    ) -> Vec<&'a Constraint> {
        graph
            .constraints
            .iter()
            .filter(|c| {
                let lhs_val = values.get(&c.lhs.0).copied().unwrap_or(0.0);
                let rhs_val = values.get(&c.rhs.0).copied().unwrap_or(0.0);
                !c.is_satisfied_by(lhs_val, rhs_val)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn constraint_kind_is_inequality() {
        assert!(ConstraintKind::LessThan.is_inequality());
        assert!(ConstraintKind::GreaterThan.is_inequality());
        assert!(!ConstraintKind::Equal.is_inequality());
        assert!(!ConstraintKind::Range.is_inequality());
    }

    #[test]
    fn constraint_kind_kind_name() {
        assert_eq!(ConstraintKind::Equal.kind_name(), "equal");
        assert_eq!(ConstraintKind::LessThan.kind_name(), "less_than");
        assert_eq!(ConstraintKind::GreaterThan.kind_name(), "greater_than");
        assert_eq!(ConstraintKind::Range.kind_name(), "range");
    }

    #[test]
    fn constraint_var_is_valid() {
        assert!(!ConstraintVar(0).is_valid());
        assert!(ConstraintVar(1).is_valid());
        assert!(ConstraintVar(42).is_valid());
    }

    #[test]
    fn constraint_is_satisfied_by_equal() {
        let c = Constraint {
            lhs: ConstraintVar(1),
            rhs: ConstraintVar(2),
            kind: ConstraintKind::Equal,
        };
        assert!(c.is_satisfied_by(5.0, 5.0));
        assert!(c.is_satisfied_by(0.0, 0.0));
        assert!(!c.is_satisfied_by(1.0, 2.0));
    }

    #[test]
    fn constraint_is_satisfied_by_less_than() {
        let c = Constraint {
            lhs: ConstraintVar(1),
            rhs: ConstraintVar(2),
            kind: ConstraintKind::LessThan,
        };
        assert!(c.is_satisfied_by(1.0, 2.0));
        assert!(!c.is_satisfied_by(2.0, 1.0));
        assert!(!c.is_satisfied_by(3.0, 3.0));
    }

    #[test]
    fn constraint_label_format() {
        let c = Constraint {
            lhs: ConstraintVar(3),
            rhs: ConstraintVar(7),
            kind: ConstraintKind::LessThan,
        };
        assert_eq!(c.label(), "v3 less_than v7");
    }

    #[test]
    fn constraint_graph_add_var_increments() {
        let mut g = ConstraintGraph::new();
        let v1 = g.add_var();
        let v2 = g.add_var();
        let v3 = g.add_var();
        assert_eq!(v1, ConstraintVar(1));
        assert_eq!(v2, ConstraintVar(2));
        assert_eq!(v3, ConstraintVar(3));
        assert_eq!(g.var_count, 3);
    }

    #[test]
    fn constraint_graph_constraints_for() {
        let mut g = ConstraintGraph::new();
        let v1 = g.add_var();
        let v2 = g.add_var();
        let v3 = g.add_var();
        g.add_constraint(Constraint { lhs: v1.clone(), rhs: v2.clone(), kind: ConstraintKind::Equal });
        g.add_constraint(Constraint { lhs: v2.clone(), rhs: v3.clone(), kind: ConstraintKind::LessThan });
        g.add_constraint(Constraint { lhs: v1.clone(), rhs: v3.clone(), kind: ConstraintKind::GreaterThan });

        let for_v2 = g.constraints_for(&v2);
        assert_eq!(for_v2.len(), 2);

        let for_v1 = g.constraints_for(&v1);
        assert_eq!(for_v1.len(), 2);

        let for_v3 = g.constraints_for(&v3);
        assert_eq!(for_v3.len(), 2);
    }

    #[test]
    fn constraint_solver_unsatisfied_finds_violations() {
        let mut g = ConstraintGraph::new();
        let v1 = g.add_var();
        let v2 = g.add_var();
        g.add_constraint(Constraint { lhs: v1.clone(), rhs: v2.clone(), kind: ConstraintKind::LessThan });
        g.add_constraint(Constraint { lhs: v1.clone(), rhs: v2.clone(), kind: ConstraintKind::Equal });

        let mut values = HashMap::new();
        values.insert(v1.0, 10.0);
        values.insert(v2.0, 5.0);

        let violations = ConstraintSolver::unsatisfied(&g, &values);
        // 10 < 5 is false (violation), 10 == 5 is false (violation)
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn constraint_solver_unsatisfied_missing_values_default_zero() {
        let mut g = ConstraintGraph::new();
        let v1 = g.add_var();
        let v2 = g.add_var();
        // 0.0 > 0.0 is false → violation
        g.add_constraint(Constraint { lhs: v1.clone(), rhs: v2.clone(), kind: ConstraintKind::GreaterThan });
        // 0.0 == 0.0 is true → satisfied
        g.add_constraint(Constraint { lhs: v1.clone(), rhs: v2.clone(), kind: ConstraintKind::Equal });

        let values: HashMap<u32, f64> = HashMap::new();
        let violations = ConstraintSolver::unsatisfied(&g, &values);
        assert_eq!(violations.len(), 1);
        assert!(matches!(violations[0].kind, ConstraintKind::GreaterThan));
    }
}
