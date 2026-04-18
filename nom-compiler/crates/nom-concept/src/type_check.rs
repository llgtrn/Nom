/// TypeChecker — constraint-based type checking with unification.
/// Pattern from nom-concept TypeInferencer: IrValue::type_of() as canonical oracle.

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CheckedType {
    Int,
    Float,
    Bool,
    Str,
    Unit,
    Unknown,
    Error(String),
}

impl CheckedType {
    pub fn is_numeric(&self) -> bool {
        matches!(self, CheckedType::Int | CheckedType::Float)
    }

    pub fn is_error(&self) -> bool {
        matches!(self, CheckedType::Error(_))
    }

    pub fn unify(&self, other: &CheckedType) -> CheckedType {
        if self == other {
            return self.clone();
        }
        match (self, other) {
            (CheckedType::Unknown, t) | (t, CheckedType::Unknown) => t.clone(),
            (CheckedType::Int, CheckedType::Float) | (CheckedType::Float, CheckedType::Int) => {
                CheckedType::Float
            }
            _ => CheckedType::Error(format!("cannot unify {:?} with {:?}", self, other)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeConstraint {
    pub lhs: String,
    pub rhs: CheckedType,
}

impl TypeConstraint {
    pub fn new(lhs: impl Into<String>, rhs: CheckedType) -> Self {
        TypeConstraint { lhs: lhs.into(), rhs }
    }
}

#[derive(Debug, Default)]
pub struct TypeContext {
    bindings: std::collections::HashMap<String, CheckedType>,
    constraints: Vec<TypeConstraint>,
}

impl TypeContext {
    pub fn new() -> Self {
        TypeContext::default()
    }

    pub fn bind(&mut self, name: impl Into<String>, ty: CheckedType) {
        self.bindings.insert(name.into(), ty);
    }

    pub fn lookup(&self, name: &str) -> CheckedType {
        self.bindings.get(name).cloned().unwrap_or(CheckedType::Unknown)
    }

    pub fn add_constraint(&mut self, constraint: TypeConstraint) {
        self.constraints.push(constraint);
    }

    pub fn apply_constraints(&mut self) -> Vec<String> {
        let mut errors = Vec::new();
        let constraints = std::mem::take(&mut self.constraints);
        for c in &constraints {
            let current = self.lookup(&c.lhs);
            let unified = current.unify(&c.rhs);
            if unified.is_error() {
                errors.push(format!("type error for `{}`: {:?}", c.lhs, unified));
            } else {
                self.bind(c.lhs.clone(), unified);
            }
        }
        errors
    }
}

#[derive(Debug, Default)]
pub struct TypeChecker {
    ctx: TypeContext,
}

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker::default()
    }

    pub fn check_numeric_op(&mut self, lhs: &str, rhs: &str) -> CheckedType {
        let lt = self.ctx.lookup(lhs);
        let rt = self.ctx.lookup(rhs);
        if lt.is_numeric() && rt.is_numeric() {
            lt.unify(&rt)
        } else if lt == CheckedType::Unknown || rt == CheckedType::Unknown {
            CheckedType::Unknown
        } else {
            CheckedType::Error(format!("non-numeric operands: {:?}, {:?}", lt, rt))
        }
    }

    pub fn check_assignment(&mut self, var: &str, value_ty: CheckedType) -> bool {
        let existing = self.ctx.lookup(var);
        let unified = existing.unify(&value_ty);
        if unified.is_error() {
            return false;
        }
        self.ctx.bind(var, unified);
        true
    }

    pub fn bind(&mut self, name: impl Into<String>, ty: CheckedType) {
        self.ctx.bind(name, ty);
    }

    pub fn lookup(&self, name: &str) -> CheckedType {
        self.ctx.lookup(name)
    }

    pub fn add_constraint(&mut self, constraint: TypeConstraint) {
        self.ctx.add_constraint(constraint);
    }

    pub fn check_constraints(&mut self) -> Vec<String> {
        self.ctx.apply_constraints()
    }
}

#[cfg(test)]
mod type_check_tests {
    use super::*;

    #[test]
    fn test_checked_type_unify_same() {
        assert_eq!(CheckedType::Int.unify(&CheckedType::Int), CheckedType::Int);
    }

    #[test]
    fn test_checked_type_unify_unknown() {
        assert_eq!(CheckedType::Unknown.unify(&CheckedType::Bool), CheckedType::Bool);
    }

    #[test]
    fn test_checked_type_unify_int_float() {
        assert_eq!(CheckedType::Int.unify(&CheckedType::Float), CheckedType::Float);
    }

    #[test]
    fn test_checked_type_unify_mismatch_is_error() {
        assert!(CheckedType::Int.unify(&CheckedType::Str).is_error());
    }

    #[test]
    fn test_type_context_bind_lookup() {
        let mut ctx = TypeContext::new();
        ctx.bind("x", CheckedType::Int);
        assert_eq!(ctx.lookup("x"), CheckedType::Int);
        assert_eq!(ctx.lookup("y"), CheckedType::Unknown);
    }

    #[test]
    fn test_type_context_constraint_ok() {
        let mut ctx = TypeContext::new();
        ctx.bind("a", CheckedType::Int);
        ctx.add_constraint(TypeConstraint::new("a", CheckedType::Int));
        let errors = ctx.apply_constraints();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_type_context_constraint_error() {
        let mut ctx = TypeContext::new();
        ctx.bind("a", CheckedType::Bool);
        ctx.add_constraint(TypeConstraint::new("a", CheckedType::Int));
        let errors = ctx.apply_constraints();
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_checker_numeric_op() {
        let mut checker = TypeChecker::new();
        checker.bind("x", CheckedType::Int);
        checker.bind("y", CheckedType::Float);
        let result = checker.check_numeric_op("x", "y");
        assert_eq!(result, CheckedType::Float);
    }

    #[test]
    fn test_checker_assignment_valid() {
        let mut checker = TypeChecker::new();
        checker.bind("v", CheckedType::Int);
        assert!(checker.check_assignment("v", CheckedType::Int));
        assert_eq!(checker.lookup("v"), CheckedType::Int);
    }
}
