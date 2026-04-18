// Type checker with constraint solver and unification for Nom concepts.

use std::collections::HashMap;

/// Concrete checked types produced by the type checker.
#[derive(Debug, Clone, PartialEq)]
pub enum CheckedType {
    Int(u32),
    Float(u32),
    Bool,
    Str,
    Unit,
    Unknown,
    Error(String),
}

impl CheckedType {
    /// Human-readable name of the type.
    pub fn type_name(&self) -> String {
        match self {
            CheckedType::Int(bits) => format!("Int{}", bits),
            CheckedType::Float(bits) => format!("Float{}", bits),
            CheckedType::Bool => "Bool".to_string(),
            CheckedType::Str => "Str".to_string(),
            CheckedType::Unit => "Unit".to_string(),
            CheckedType::Unknown => "Unknown".to_string(),
            CheckedType::Error(msg) => format!("Error({})", msg),
        }
    }

    /// True for `Int` and `Float` variants.
    pub fn is_numeric(&self) -> bool {
        matches!(self, CheckedType::Int(_) | CheckedType::Float(_))
    }

    /// True for the `Error` variant.
    pub fn is_error(&self) -> bool {
        matches!(self, CheckedType::Error(_))
    }

    /// Unify two types according to the Nom widening rules.
    pub fn unify(a: &CheckedType, b: &CheckedType) -> CheckedType {
        match (a, b) {
            (CheckedType::Error(e), _) => CheckedType::Error(e.clone()),
            (_, CheckedType::Error(e)) => CheckedType::Error(e.clone()),
            (CheckedType::Int(x), CheckedType::Int(y)) => CheckedType::Int((*x).max(*y)),
            (CheckedType::Float(x), CheckedType::Float(y)) => CheckedType::Float((*x).max(*y)),
            (CheckedType::Int(x), CheckedType::Float(y)) => CheckedType::Float((*x).max(*y)),
            (CheckedType::Float(x), CheckedType::Int(y)) => CheckedType::Float((*x).max(*y)),
            (CheckedType::Bool, CheckedType::Bool) => CheckedType::Bool,
            (CheckedType::Str, CheckedType::Str) => CheckedType::Str,
            (CheckedType::Unit, CheckedType::Unit) => CheckedType::Unit,
            (CheckedType::Unknown, other) | (other, CheckedType::Unknown) => other.clone(),
            _ => CheckedType::Error("type mismatch".to_string()),
        }
    }
}

/// A single type constraint: variable `left` must have type `right`.
#[derive(Debug, Clone)]
pub struct TypeConstraint {
    pub left: String,
    pub right: CheckedType,
}

impl TypeConstraint {
    pub fn new(left: impl Into<String>, right: CheckedType) -> Self {
        TypeConstraint { left: left.into(), right }
    }
}

/// Typing environment: maps variable names to their resolved types.
#[derive(Debug, Clone, Default)]
pub struct TypeContext {
    pub bindings: HashMap<String, CheckedType>,
}

impl TypeContext {
    pub fn new() -> Self {
        TypeContext { bindings: HashMap::new() }
    }

    /// Bind a name to a type, overwriting any previous binding.
    pub fn bind(&mut self, name: impl Into<String>, typ: CheckedType) {
        self.bindings.insert(name.into(), typ);
    }

    /// Look up the type bound to `name`, if any.
    pub fn lookup(&self, name: &str) -> Option<&CheckedType> {
        self.bindings.get(name)
    }

    /// Apply one constraint. If the name is already bound, unify the existing
    /// type with the constraint type. Returns false (without updating) when
    /// unification yields an Error.
    pub fn apply_constraint(&mut self, constraint: TypeConstraint) -> bool {
        let unified = match self.bindings.get(&constraint.left) {
            Some(existing) => CheckedType::unify(existing, &constraint.right),
            None => constraint.right.clone(),
        };
        if unified.is_error() {
            false
        } else {
            self.bindings.insert(constraint.left, unified);
            true
        }
    }
}

/// Stateless helper for common type-checking operations.
pub struct TypeChecker;

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker
    }

    /// Returns Error if either operand is non-numeric; otherwise returns the
    /// unified numeric type.
    pub fn check_numeric_op(a: &CheckedType, b: &CheckedType) -> CheckedType {
        if !a.is_numeric() || !b.is_numeric() {
            CheckedType::Error("expected numeric types".to_string())
        } else {
            CheckedType::unify(a, b)
        }
    }

    /// Returns true when unifying `declared` with `actual` succeeds (not Error).
    pub fn check_assignment(declared: &CheckedType, actual: &CheckedType) -> bool {
        !CheckedType::unify(declared, actual).is_error()
    }

    /// Apply a slice of constraints to `ctx`. Returns error messages for failed ones.
    pub fn check_constraints(
        ctx: &mut TypeContext,
        constraints: &[TypeConstraint],
    ) -> Vec<String> {
        let mut errors = Vec::new();
        for c in constraints {
            if !ctx.apply_constraint(c.clone()) {
                errors.push(format!(
                    "constraint failed: {} cannot unify with {}",
                    c.left,
                    c.right.type_name()
                ));
            }
        }
        errors
    }
}

#[cfg(test)]
mod type_check_tests {
    use super::*;

    #[test]
    fn test_is_numeric() {
        assert!(CheckedType::Int(32).is_numeric());
        assert!(CheckedType::Float(64).is_numeric());
        assert!(!CheckedType::Bool.is_numeric());
        assert!(!CheckedType::Str.is_numeric());
        assert!(!CheckedType::Unit.is_numeric());
        assert!(!CheckedType::Unknown.is_numeric());
        assert!(!CheckedType::Error("x".into()).is_numeric());
    }

    #[test]
    fn test_unify_same_int() {
        let result = CheckedType::unify(&CheckedType::Int(32), &CheckedType::Int(32));
        assert_eq!(result, CheckedType::Int(32));
    }

    #[test]
    fn test_unify_int_float_yields_float() {
        let result = CheckedType::unify(&CheckedType::Int(32), &CheckedType::Float(64));
        assert_eq!(result, CheckedType::Float(64));
    }

    #[test]
    fn test_unify_different_yields_error() {
        let result = CheckedType::unify(&CheckedType::Bool, &CheckedType::Str);
        assert!(result.is_error());
    }

    #[test]
    fn test_context_bind_and_lookup() {
        let mut ctx = TypeContext::new();
        ctx.bind("x", CheckedType::Int(32));
        assert_eq!(ctx.lookup("x"), Some(&CheckedType::Int(32)));
        assert_eq!(ctx.lookup("y"), None);
    }

    #[test]
    fn test_apply_constraint_success() {
        let mut ctx = TypeContext::new();
        let c = TypeConstraint::new("a", CheckedType::Bool);
        assert!(ctx.apply_constraint(c));
        assert_eq!(ctx.lookup("a"), Some(&CheckedType::Bool));
    }

    #[test]
    fn test_apply_constraint_conflict_returns_false() {
        let mut ctx = TypeContext::new();
        ctx.bind("b", CheckedType::Bool);
        let c = TypeConstraint::new("b", CheckedType::Int(32));
        assert!(!ctx.apply_constraint(c));
        assert_eq!(ctx.lookup("b"), Some(&CheckedType::Bool));
    }

    #[test]
    fn test_check_numeric_op_error_on_non_numeric() {
        let result = TypeChecker::check_numeric_op(&CheckedType::Bool, &CheckedType::Int(32));
        assert!(result.is_error());
    }

    #[test]
    fn test_check_constraints_collects_errors() {
        let mut ctx = TypeContext::new();
        ctx.bind("x", CheckedType::Bool);
        let constraints = vec![
            TypeConstraint::new("x", CheckedType::Int(32)),
            TypeConstraint::new("y", CheckedType::Str),
        ];
        let errors = TypeChecker::check_constraints(&mut ctx, &constraints);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains('x'));
        assert_eq!(ctx.lookup("y"), Some(&CheckedType::Str));
    }
}
