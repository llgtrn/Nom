// Type inference for Nom IR — infers types from expressions

use crate::ir::{IrType, IrValue};

/// Type inference result.
#[derive(Debug, Clone, PartialEq)]
pub enum InferResult {
    Known(IrType),
    Unknown,
    Conflict(IrType, IrType), // two incompatible types found
}

impl InferResult {
    pub fn is_known(&self) -> bool { matches!(self, Self::Known(_)) }
    pub fn unwrap_known(self) -> IrType {
        match self { Self::Known(t) => t, _ => panic!("not known") }
    }
}

/// A type constraint: variable name → expected type.
#[derive(Debug, Clone)]
pub struct TypeConstraint {
    pub var_name: String,
    pub expected: IrType,
    pub source: &'static str, // "literal", "annotation", "inferred"
}

/// Type environment: maps variable names to inferred types.
#[derive(Debug, Clone, Default)]
pub struct TypeEnv {
    pub bindings: std::collections::HashMap<String, IrType>,
}

impl TypeEnv {
    pub fn new() -> Self { Self::default() }

    pub fn bind(&mut self, name: impl Into<String>, ty: IrType) {
        self.bindings.insert(name.into(), ty);
    }

    pub fn lookup(&self, name: &str) -> Option<&IrType> {
        self.bindings.get(name)
    }

    pub fn binding_count(&self) -> usize { self.bindings.len() }
}

/// Type inferencer for Nom expressions.
pub struct TypeInferencer {
    pub env: TypeEnv,
    pub constraints: Vec<TypeConstraint>,
}

impl TypeInferencer {
    pub fn new() -> Self {
        Self { env: TypeEnv::new(), constraints: Vec::new() }
    }

    pub fn add_constraint(&mut self, var: impl Into<String>, ty: IrType, source: &'static str) {
        let var = var.into();
        self.constraints.push(TypeConstraint { var_name: var.clone(), expected: ty.clone(), source });
        self.env.bind(var, ty);
    }

    /// Infer type of a literal value.
    pub fn infer_literal(&self, value: &IrValue) -> InferResult {
        InferResult::Known(value.type_of())
    }

    /// Solve all constraints and return unified type env.
    pub fn solve(&self) -> TypeEnv {
        self.env.clone() // stub: constraints already applied via add_constraint
    }

    pub fn constraint_count(&self) -> usize { self.constraints.len() }
}

impl Default for TypeInferencer {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod type_infer_tests {
    use super::*;

    #[test]
    fn test_type_env_bind_lookup() {
        let mut env = TypeEnv::new();
        env.bind("x", IrType::Int(64));
        assert_eq!(env.lookup("x"), Some(&IrType::Int(64)));
    }

    #[test]
    fn test_type_env_missing() {
        let env = TypeEnv::new();
        assert!(env.lookup("z").is_none());
    }

    #[test]
    fn test_infer_result_is_known() {
        let r = InferResult::Known(IrType::Bool);
        assert!(r.is_known());
        let u = InferResult::Unknown;
        assert!(!u.is_known());
    }

    #[test]
    fn test_add_constraint() {
        let mut inf = TypeInferencer::new();
        inf.add_constraint("x", IrType::Int(64), "literal");
        assert_eq!(inf.constraint_count(), 1);
        assert_eq!(inf.env.lookup("x"), Some(&IrType::Int(64)));
    }

    #[test]
    fn test_solve_returns_env() {
        let mut inf = TypeInferencer::new();
        inf.add_constraint("a", IrType::Float(64), "annotation");
        inf.add_constraint("b", IrType::Bool, "inferred");
        let env = inf.solve();
        assert_eq!(env.binding_count(), 2);
    }

    #[test]
    fn test_type_env_binding_count() {
        let mut env = TypeEnv::new();
        env.bind("x", IrType::Int(64));
        env.bind("y", IrType::Float(64));
        assert_eq!(env.binding_count(), 2);
    }

    #[test]
    fn test_conflict_variant() {
        let r = InferResult::Conflict(IrType::Int(64), IrType::Float(64));
        assert!(!r.is_known());
    }

    #[test]
    fn test_infer_literal_int() {
        let inf = TypeInferencer::new();
        assert_eq!(inf.infer_literal(&IrValue::Int(42)), InferResult::Known(IrType::Int(64)));
    }

    #[test]
    fn test_infer_literal_float() {
        let inf = TypeInferencer::new();
        assert_eq!(inf.infer_literal(&IrValue::Float(3.14)), InferResult::Known(IrType::Float(64)));
    }

    #[test]
    fn test_infer_literal_bool() {
        let inf = TypeInferencer::new();
        assert_eq!(inf.infer_literal(&IrValue::Bool(true)), InferResult::Known(IrType::Bool));
    }

    #[test]
    fn test_infer_literal_str() {
        let inf = TypeInferencer::new();
        assert_eq!(inf.infer_literal(&IrValue::Str("hello".into())), InferResult::Known(IrType::Str));
    }

    #[test]
    fn test_infer_literal_unit() {
        let inf = TypeInferencer::new();
        assert_eq!(inf.infer_literal(&IrValue::Unit), InferResult::Known(IrType::Unit));
    }
}
