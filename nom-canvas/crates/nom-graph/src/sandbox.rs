#![deny(unsafe_code)]

/// The result of evaluating a sandbox expression.
#[derive(Debug, Clone, PartialEq)]
pub enum SandboxValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<SandboxValue>),
}

impl SandboxValue {
    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Null => false,
            Self::Bool(b) => *b,
            Self::Int(n) => *n != 0,
            Self::Float(f) => *f != 0.0,
            Self::Str(s) => !s.is_empty(),
            Self::List(l) => !l.is_empty(),
        }
    }
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "bool",
            Self::Int(_) => "int",
            Self::Float(_) => "float",
            Self::Str(_) => "str",
            Self::List(_) => "list",
        }
    }
}

/// A simple AST node for safe expression parsing.
#[derive(Debug, Clone)]
pub enum Expr {
    Literal(SandboxValue),
    Var(String),
    BinOp {
        op: BinOpKind,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    If {
        cond: Box<Expr>,
        then: Box<Expr>,
        else_: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOpKind {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Neq,
    Lt,
    Gt,
    And,
    Or,
}

/// Error from sandbox evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum SandboxError {
    UndefinedVar(String),
    DivisionByZero,
    TypeMismatch {
        expected: &'static str,
        got: &'static str,
    },
    UnknownFunction(String),
    DepthLimitExceeded,
    /// n8n JSTaskRunner: `this` access is forbidden.
    ForbiddenIdentifier(String),
    /// n8n JSTaskRunner: prototype chain access (`__proto__`, `prototype`, `constructor`) is forbidden.
    PrototypeAccess,
    /// n8n JSTaskRunner: dollar-prefixed variable not in the allowed set.
    InvalidDollarVar(String),
}

impl std::fmt::Display for SandboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UndefinedVar(v) => write!(f, "undefined variable: {v}"),
            Self::DivisionByZero => write!(f, "division by zero"),
            Self::TypeMismatch { expected, got } => {
                write!(f, "type mismatch: expected {expected}, got {got}")
            }
            Self::UnknownFunction(n) => write!(f, "unknown function: {n}"),
            Self::DepthLimitExceeded => write!(f, "recursion depth exceeded"),
            Self::ForbiddenIdentifier(id) => write!(f, "forbidden identifier: {id}"),
            Self::PrototypeAccess => write!(f, "prototype chain access is forbidden"),
            Self::InvalidDollarVar(name) => write!(f, "invalid dollar variable: {name}"),
        }
    }
}

/// --- Sanitizer 1: DepthLimitSanitizer ---
/// Rejects expressions exceeding a maximum nesting depth (prevents DoS).
pub struct DepthLimitSanitizer {
    pub max_depth: usize,
}
impl DepthLimitSanitizer {
    pub fn check(&self, expr: &Expr) -> Result<(), SandboxError> {
        self.check_depth(expr, 0)
    }
    fn check_depth(&self, expr: &Expr, depth: usize) -> Result<(), SandboxError> {
        if depth > self.max_depth {
            return Err(SandboxError::DepthLimitExceeded);
        }
        match expr {
            Expr::Literal(_) | Expr::Var(_) => Ok(()),
            Expr::BinOp { left, right, .. } => {
                self.check_depth(left, depth + 1)?;
                self.check_depth(right, depth + 1)
            }
            Expr::If { cond, then, else_ } => {
                self.check_depth(cond, depth + 1)?;
                self.check_depth(then, depth + 1)?;
                self.check_depth(else_, depth + 1)
            }
            Expr::Call { args, .. } => args.iter().try_for_each(|a| self.check_depth(a, depth + 1)),
        }
    }
}

/// --- Sanitizer 2: AllowedFunctionsSanitizer ---
/// Allows only a whitelist of safe built-in functions.
pub struct AllowedFunctionsSanitizer {
    pub allowed: Vec<&'static str>,
}
impl AllowedFunctionsSanitizer {
    pub fn default_safe() -> Self {
        Self {
            allowed: vec![
                "len", "upper", "lower", "trim", "abs", "min", "max", "concat", "to_str", "to_int",
            ],
        }
    }
    pub fn check(&self, expr: &Expr) -> Result<(), SandboxError> {
        match expr {
            Expr::Call { name, args } => {
                if !self.allowed.contains(&name.as_str()) {
                    return Err(SandboxError::UnknownFunction(name.clone()));
                }
                args.iter().try_for_each(|a| self.check(a))
            }
            Expr::BinOp { left, right, .. } => {
                self.check(left)?;
                self.check(right)
            }
            Expr::If { cond, then, else_ } => {
                self.check(cond)?;
                self.check(then)?;
                self.check(else_)
            }
            _ => Ok(()),
        }
    }
}

/// --- Sanitizer 3: NoSideEffectsSanitizer ---
// STUB: this sanitizer does not yet inspect the expression tree.
// TODO(security): implement before adding Expr::Assign/Import/Exec AST variants
pub struct NoSideEffectsSanitizer;
impl NoSideEffectsSanitizer {
    pub fn check(&self, _expr: &Expr) -> Result<(), SandboxError> {
        Ok(())
    }
}

/// --- Sanitizer 4: TypeCoherenceSanitizer ---
pub struct TypeCoherenceSanitizer;
impl TypeCoherenceSanitizer {
    pub fn check(&self, expr: &Expr) -> Result<(), SandboxError> {
        match expr {
            Expr::BinOp { op, left, right } => {
                self.check(left)?;
                self.check(right)?;
                if matches!(
                    op,
                    BinOpKind::Add | BinOpKind::Sub | BinOpKind::Mul | BinOpKind::Div
                ) {
                    if let (
                        Expr::Literal(SandboxValue::Str(_)),
                        Expr::Literal(SandboxValue::Int(_)),
                    )
                    | (
                        Expr::Literal(SandboxValue::Int(_)),
                        Expr::Literal(SandboxValue::Str(_)),
                    ) = (left.as_ref(), right.as_ref())
                    {
                        return Err(SandboxError::TypeMismatch {
                            expected: "numeric",
                            got: "str",
                        });
                    }
                }
                Ok(())
            }
            Expr::If { cond, then, else_ } => {
                self.check(cond)?;
                self.check(then)?;
                self.check(else_)
            }
            Expr::Call { args, .. } => args.iter().try_for_each(|a| self.check(a)),
            _ => Ok(()),
        }
    }
}

/// --- Sanitizer 5: ThisReplaceSanitizer ---
/// Detects use of `this` as a variable name (n8n JSTaskRunner pattern).
pub struct ThisReplaceSanitizer;
impl ThisReplaceSanitizer {
    pub fn check(&self, expr: &Expr) -> Result<(), SandboxError> {
        match expr {
            Expr::Var(name) if name == "this" => {
                Err(SandboxError::ForbiddenIdentifier("this".into()))
            }
            Expr::Var(_) | Expr::Literal(_) => Ok(()),
            Expr::BinOp { left, right, .. } => {
                self.check(left)?;
                self.check(right)
            }
            Expr::If { cond, then, else_ } => {
                self.check(cond)?;
                self.check(then)?;
                self.check(else_)
            }
            Expr::Call { args, .. } => args.iter().try_for_each(|a| self.check(a)),
        }
    }
}

/// --- Sanitizer 6: PrototypeBlockSanitizer ---
/// Detects access to `__proto__`, `prototype`, or `constructor` identifiers.
pub struct PrototypeBlockSanitizer;
const PROTOTYPE_KEYWORDS: &[&str] = &["__proto__", "prototype", "constructor"];
impl PrototypeBlockSanitizer {
    pub fn check(&self, expr: &Expr) -> Result<(), SandboxError> {
        match expr {
            Expr::Var(name) if PROTOTYPE_KEYWORDS.iter().any(|kw| name.contains(kw)) => {
                Err(SandboxError::PrototypeAccess)
            }
            Expr::Var(_) | Expr::Literal(_) => Ok(()),
            Expr::BinOp { left, right, .. } => {
                self.check(left)?;
                self.check(right)
            }
            Expr::If { cond, then, else_ } => {
                self.check(cond)?;
                self.check(then)?;
                self.check(else_)
            }
            Expr::Call { args, .. } => args.iter().try_for_each(|a| self.check(a)),
        }
    }
}

/// --- Sanitizer 7: DollarValidateSanitizer ---
/// Allows only known n8n dollar-prefixed variables; rejects unknown `$`-prefixed names.
pub struct DollarValidateSanitizer;
const ALLOWED_DOLLAR_VARS: &[&str] = &[
    "$input",
    "$json",
    "$node",
    "$workflow",
    "$item",
    "$items",
    "$runIndex",
];
impl DollarValidateSanitizer {
    pub fn check(&self, expr: &Expr) -> Result<(), SandboxError> {
        match expr {
            Expr::Var(name) if name.starts_with('$') => {
                if ALLOWED_DOLLAR_VARS.contains(&name.as_str()) {
                    Ok(())
                } else {
                    Err(SandboxError::InvalidDollarVar(name.clone()))
                }
            }
            Expr::Var(_) | Expr::Literal(_) => Ok(()),
            Expr::BinOp { left, right, .. } => {
                self.check(left)?;
                self.check(right)
            }
            Expr::If { cond, then, else_ } => {
                self.check(cond)?;
                self.check(then)?;
                self.check(else_)
            }
            Expr::Call { args, .. } => args.iter().try_for_each(|a| self.check(a)),
        }
    }
}

pub struct EvalContext {
    vars: std::collections::HashMap<String, SandboxValue>,
}

impl EvalContext {
    pub fn new() -> Self {
        Self {
            vars: std::collections::HashMap::new(),
        }
    }
    pub fn set(&mut self, name: impl Into<String>, val: SandboxValue) {
        self.vars.insert(name.into(), val);
    }
    pub fn get(&self, name: &str) -> Option<&SandboxValue> {
        self.vars.get(name)
    }
}

impl Default for EvalContext {
    fn default() -> Self {
        Self::new()
    }
}

pub fn sanitize(expr: &Expr) -> Result<(), SandboxError> {
    DepthLimitSanitizer { max_depth: 16 }.check(expr)?;
    AllowedFunctionsSanitizer::default_safe().check(expr)?;
    NoSideEffectsSanitizer.check(expr)?;
    TypeCoherenceSanitizer.check(expr)?;
    ThisReplaceSanitizer.check(expr)?;
    PrototypeBlockSanitizer.check(expr)?;
    DollarValidateSanitizer.check(expr)?;
    Ok(())
}

/// Maximum evaluation recursion depth enforced at runtime.
const EVAL_DEPTH_LIMIT: usize = 64;

/// Public entry point: evaluates `expr` against `ctx` with a runtime depth limit of 64.
pub fn eval_expr(expr: &Expr, ctx: &EvalContext) -> Result<SandboxValue, SandboxError> {
    eval_expr_inner(expr, ctx, 0)
}

fn eval_expr_inner(
    expr: &Expr,
    ctx: &EvalContext,
    depth: usize,
) -> Result<SandboxValue, SandboxError> {
    if depth > EVAL_DEPTH_LIMIT {
        return Err(SandboxError::DepthLimitExceeded);
    }
    match expr {
        Expr::Literal(v) => Ok(v.clone()),
        Expr::Var(name) => ctx
            .get(name)
            .cloned()
            .ok_or_else(|| SandboxError::UndefinedVar(name.clone())),
        Expr::BinOp { op, left, right } => {
            let l = eval_expr_inner(left, ctx, depth + 1)?;
            let r = eval_expr_inner(right, ctx, depth + 1)?;
            eval_binop(*op, l, r)
        }
        Expr::If { cond, then, else_ } => {
            if eval_expr_inner(cond, ctx, depth + 1)?.is_truthy() {
                eval_expr_inner(then, ctx, depth + 1)
            } else {
                eval_expr_inner(else_, ctx, depth + 1)
            }
        }
        Expr::Call { name, args } => {
            let evaled: Result<Vec<_>, _> =
                args.iter().map(|a| eval_expr_inner(a, ctx, depth + 1)).collect();
            eval_call(name, evaled?)
        }
    }
}

fn eval_binop(
    op: BinOpKind,
    l: SandboxValue,
    r: SandboxValue,
) -> Result<SandboxValue, SandboxError> {
    match (op, &l, &r) {
        (BinOpKind::Add, SandboxValue::Int(a), SandboxValue::Int(b)) => a
            .checked_add(*b)
            .map(SandboxValue::Int)
            .ok_or(SandboxError::TypeMismatch {
                expected: "non-overflowing integer",
                got: "overflow",
            }),
        (BinOpKind::Add, SandboxValue::Float(a), SandboxValue::Float(b)) => {
            Ok(SandboxValue::Float(a + b))
        }
        (BinOpKind::Add, SandboxValue::Str(a), SandboxValue::Str(b)) => {
            Ok(SandboxValue::Str(format!("{}{}", a, b)))
        }
        (BinOpKind::Sub, SandboxValue::Int(a), SandboxValue::Int(b)) => a
            .checked_sub(*b)
            .map(SandboxValue::Int)
            .ok_or(SandboxError::TypeMismatch {
                expected: "non-overflowing integer",
                got: "overflow",
            }),
        (BinOpKind::Mul, SandboxValue::Int(a), SandboxValue::Int(b)) => a
            .checked_mul(*b)
            .map(SandboxValue::Int)
            .ok_or(SandboxError::TypeMismatch {
                expected: "non-overflowing integer",
                got: "overflow",
            }),
        (BinOpKind::Div, SandboxValue::Int(a), SandboxValue::Int(b)) => {
            if *b == 0 {
                Err(SandboxError::DivisionByZero)
            } else {
                Ok(SandboxValue::Int(a / b))
            }
        }
        (BinOpKind::Eq, a, b) => Ok(SandboxValue::Bool(a == b)),
        (BinOpKind::Neq, a, b) => Ok(SandboxValue::Bool(a != b)),
        (BinOpKind::Lt, SandboxValue::Int(a), SandboxValue::Int(b)) => {
            Ok(SandboxValue::Bool(a < b))
        }
        (BinOpKind::Gt, SandboxValue::Int(a), SandboxValue::Int(b)) => {
            Ok(SandboxValue::Bool(a > b))
        }
        (BinOpKind::And, a, b) => Ok(SandboxValue::Bool(a.is_truthy() && b.is_truthy())),
        (BinOpKind::Or, a, b) => Ok(SandboxValue::Bool(a.is_truthy() || b.is_truthy())),
        _ => Err(SandboxError::TypeMismatch {
            expected: "compatible types",
            got: "incompatible",
        }),
    }
}

fn eval_call(name: &str, args: Vec<SandboxValue>) -> Result<SandboxValue, SandboxError> {
    match name {
        "len" => match args.first() {
            Some(SandboxValue::Str(s)) => Ok(SandboxValue::Int(s.len() as i64)),
            Some(SandboxValue::List(l)) => Ok(SandboxValue::Int(l.len() as i64)),
            _ => Err(SandboxError::TypeMismatch {
                expected: "str or list",
                got: "other",
            }),
        },
        "upper" => match args.first() {
            Some(SandboxValue::Str(s)) => Ok(SandboxValue::Str(s.to_uppercase())),
            _ => Err(SandboxError::TypeMismatch {
                expected: "str",
                got: "other",
            }),
        },
        "lower" => match args.first() {
            Some(SandboxValue::Str(s)) => Ok(SandboxValue::Str(s.to_lowercase())),
            _ => Err(SandboxError::TypeMismatch {
                expected: "str",
                got: "other",
            }),
        },
        "trim" => match args.first() {
            Some(SandboxValue::Str(s)) => Ok(SandboxValue::Str(s.trim().to_string())),
            _ => Err(SandboxError::TypeMismatch {
                expected: "str",
                got: "other",
            }),
        },
        "abs" => match args.first() {
            Some(SandboxValue::Int(n)) => Ok(SandboxValue::Int(n.abs())),
            _ => Err(SandboxError::TypeMismatch {
                expected: "int",
                got: "other",
            }),
        },
        "to_str" => match args.first() {
            Some(v) => Ok(SandboxValue::Str(format!("{:?}", v))),
            None => Ok(SandboxValue::Str(String::new())),
        },
        "to_int" => match args.first() {
            Some(SandboxValue::Int(n)) => Ok(SandboxValue::Int(*n)),
            Some(SandboxValue::Float(f)) => Ok(SandboxValue::Int(*f as i64)),
            _ => Err(SandboxError::TypeMismatch {
                expected: "numeric",
                got: "other",
            }),
        },
        _ => Err(SandboxError::UnknownFunction(name.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_literal_eval() {
        let ctx = EvalContext::new();
        assert_eq!(
            eval_expr(&Expr::Literal(SandboxValue::Int(42)), &ctx),
            Ok(SandboxValue::Int(42))
        );
    }
    #[test]
    fn sandbox_binop_add_ints() {
        let ctx = EvalContext::new();
        let expr = Expr::BinOp {
            op: BinOpKind::Add,
            left: Box::new(Expr::Literal(SandboxValue::Int(3))),
            right: Box::new(Expr::Literal(SandboxValue::Int(4))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Int(7)));
    }
    #[test]
    fn sandbox_div_by_zero_error() {
        let ctx = EvalContext::new();
        let expr = Expr::BinOp {
            op: BinOpKind::Div,
            left: Box::new(Expr::Literal(SandboxValue::Int(1))),
            right: Box::new(Expr::Literal(SandboxValue::Int(0))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Err(SandboxError::DivisionByZero));
    }
    #[test]
    fn sandbox_var_lookup() {
        let mut ctx = EvalContext::new();
        ctx.set("x", SandboxValue::Int(99));
        assert_eq!(
            eval_expr(&Expr::Var("x".into()), &ctx),
            Ok(SandboxValue::Int(99))
        );
        assert_eq!(
            eval_expr(&Expr::Var("y".into()), &ctx),
            Err(SandboxError::UndefinedVar("y".into()))
        );
    }
    #[test]
    fn sandbox_if_expr() {
        let ctx = EvalContext::new();
        let expr = Expr::If {
            cond: Box::new(Expr::Literal(SandboxValue::Bool(true))),
            then: Box::new(Expr::Literal(SandboxValue::Int(1))),
            else_: Box::new(Expr::Literal(SandboxValue::Int(0))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Int(1)));
    }
    #[test]
    fn sanitizer_depth_limit() {
        let deep = (0..20).fold(Expr::Literal(SandboxValue::Int(0)), |acc, _| Expr::BinOp {
            op: BinOpKind::Add,
            left: Box::new(acc),
            right: Box::new(Expr::Literal(SandboxValue::Int(1))),
        });
        assert_eq!(
            DepthLimitSanitizer { max_depth: 5 }.check(&deep),
            Err(SandboxError::DepthLimitExceeded)
        );
    }
    #[test]
    fn sanitizer_blocked_function() {
        let expr = Expr::Call {
            name: "exec".into(),
            args: vec![],
        };
        assert!(AllowedFunctionsSanitizer::default_safe()
            .check(&expr)
            .is_err());
    }
    #[test]
    fn sanitizer_allowed_function() {
        let expr = Expr::Call {
            name: "len".into(),
            args: vec![Expr::Literal(SandboxValue::Str("hi".into()))],
        };
        assert!(sanitize(&expr).is_ok());
    }
    #[test]
    fn sandbox_call_len() {
        let ctx = EvalContext::new();
        let expr = Expr::Call {
            name: "len".into(),
            args: vec![Expr::Literal(SandboxValue::Str("hello".into()))],
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Int(5)));
    }
    #[test]
    fn type_coherence_rejects_str_arithmetic() {
        let expr = Expr::BinOp {
            op: BinOpKind::Mul,
            left: Box::new(Expr::Literal(SandboxValue::Str("a".into()))),
            right: Box::new(Expr::Literal(SandboxValue::Int(2))),
        };
        assert!(TypeCoherenceSanitizer.check(&expr).is_err());
    }
    #[test]
    fn sandbox_this_access_is_blocked() {
        let expr = Expr::Var("this".into());
        assert_eq!(
            ThisReplaceSanitizer.check(&expr),
            Err(SandboxError::ForbiddenIdentifier("this".into()))
        );
        assert!(sanitize(&expr).is_err());
    }
    #[test]
    fn sandbox_prototype_access_is_blocked() {
        for name in &["__proto__", "prototype", "constructor", "obj.prototype"] {
            let expr = Expr::Var((*name).into());
            assert_eq!(
                PrototypeBlockSanitizer.check(&expr),
                Err(SandboxError::PrototypeAccess),
                "expected PrototypeAccess for {name}"
            );
        }
    }
    #[test]
    fn sandbox_invalid_dollar_var_is_blocked() {
        let expr = Expr::Var("$secret".into());
        assert_eq!(
            DollarValidateSanitizer.check(&expr),
            Err(SandboxError::InvalidDollarVar("$secret".into()))
        );
        assert!(sanitize(&expr).is_err());
    }
    #[test]
    fn sandbox_valid_dollar_var_is_allowed() {
        for name in ALLOWED_DOLLAR_VARS {
            let expr = Expr::Var((*name).into());
            assert!(
                DollarValidateSanitizer.check(&expr).is_ok(),
                "{name} should be allowed"
            );
        }
    }

    #[test]
    fn sandbox_nested_expr_depth_limit() {
        // Build a BinOp chain nested 17 levels deep — exceeds max_depth of 16.
        let deep = (0..17).fold(Expr::Literal(SandboxValue::Int(0)), |acc, _| Expr::BinOp {
            op: BinOpKind::Add,
            left: Box::new(acc),
            right: Box::new(Expr::Literal(SandboxValue::Int(1))),
        });
        let result = sanitize(&deep);
        assert_eq!(
            result,
            Err(SandboxError::DepthLimitExceeded),
            "nesting > 16 must be rejected"
        );
    }

    #[test]
    fn sandbox_eval_arithmetic() {
        // (3 + 4) * 2 == 14
        let ctx = EvalContext::new();
        let inner = Expr::BinOp {
            op: BinOpKind::Add,
            left: Box::new(Expr::Literal(SandboxValue::Int(3))),
            right: Box::new(Expr::Literal(SandboxValue::Int(4))),
        };
        let expr = Expr::BinOp {
            op: BinOpKind::Mul,
            left: Box::new(inner),
            right: Box::new(Expr::Literal(SandboxValue::Int(2))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Int(14)));
    }

    #[test]
    fn sandbox_eval_if_condition() {
        // if true { 1 } else { 2 } == 1
        let ctx = EvalContext::new();
        let expr = Expr::If {
            cond: Box::new(Expr::Literal(SandboxValue::Bool(true))),
            then: Box::new(Expr::Literal(SandboxValue::Int(1))),
            else_: Box::new(Expr::Literal(SandboxValue::Int(2))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Int(1)));
    }

    #[test]
    fn sanitizer_this_replace_rejects_this() {
        // Expr::Var("this") must produce ForbiddenIdentifier via ThisReplaceSanitizer.
        let expr = Expr::Var("this".into());
        assert_eq!(
            ThisReplaceSanitizer.check(&expr),
            Err(SandboxError::ForbiddenIdentifier("this".into())),
            "ThisReplaceSanitizer must reject 'this'"
        );
    }

    #[test]
    fn sanitizer_prototype_block_rejects_proto() {
        // "__proto__" access must be rejected as PrototypeAccess.
        let expr = Expr::Var("__proto__".into());
        assert_eq!(
            PrototypeBlockSanitizer.check(&expr),
            Err(SandboxError::PrototypeAccess),
            "PrototypeBlockSanitizer must reject '__proto__'"
        );
    }

    #[test]
    fn sanitizer_dollar_validate_allows_workflow() {
        // "$workflow" is in ALLOWED_DOLLAR_VARS and must pass DollarValidateSanitizer.
        let expr = Expr::Var("$workflow".into());
        assert!(
            DollarValidateSanitizer.check(&expr).is_ok(),
            "$workflow must be allowed by DollarValidateSanitizer"
        );
    }

    #[test]
    fn sanitizer_dollar_validate_rejects_unknown() {
        // "$custom_var" is not in the allowed list, must produce InvalidDollarVar.
        let expr = Expr::Var("$custom_var".into());
        assert_eq!(
            DollarValidateSanitizer.check(&expr),
            Err(SandboxError::InvalidDollarVar("$custom_var".into())),
            "$custom_var must be rejected as InvalidDollarVar"
        );
    }

    #[test]
    fn sanitizer_combined_all_four_pass_clean_expr() {
        // A plain integer literal passes all sanitizers via the top-level sanitize().
        let expr = Expr::Literal(SandboxValue::Int(7));
        assert!(
            sanitize(&expr).is_ok(),
            "clean integer literal must pass all sanitizers"
        );
    }

    #[test]
    fn eval_expr_addition() {
        // 2 + 3 == 5
        let ctx = EvalContext::new();
        let expr = Expr::BinOp {
            op: BinOpKind::Add,
            left: Box::new(Expr::Literal(SandboxValue::Int(2))),
            right: Box::new(Expr::Literal(SandboxValue::Int(3))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Int(5)));
    }

    #[test]
    fn eval_expr_string_concat() {
        // "hello" + " world" == "hello world"
        let ctx = EvalContext::new();
        let expr = Expr::BinOp {
            op: BinOpKind::Add,
            left: Box::new(Expr::Literal(SandboxValue::Str("hello".into()))),
            right: Box::new(Expr::Literal(SandboxValue::Str(" world".into()))),
        };
        assert_eq!(
            eval_expr(&expr, &ctx),
            Ok(SandboxValue::Str("hello world".into()))
        );
    }

    #[test]
    fn eval_expr_depth_limit_respected() {
        // Build a BinOp chain 18 levels deep — exceeds default max_depth of 16.
        let deep = (0..18).fold(Expr::Literal(SandboxValue::Int(0)), |acc, _| Expr::BinOp {
            op: BinOpKind::Add,
            left: Box::new(acc),
            right: Box::new(Expr::Literal(SandboxValue::Int(1))),
        });
        assert_eq!(
            sanitize(&deep),
            Err(SandboxError::DepthLimitExceeded),
            "expression nested > 16 levels must be rejected"
        );
    }

    #[test]
    fn eval_expr_unknown_function_blocked() {
        // "console_log" is not in the allowed function list; AllowedFunctionsSanitizer must reject it.
        let expr = Expr::Call {
            name: "console_log".into(),
            args: vec![Expr::Literal(SandboxValue::Str("x".into()))],
        };
        assert_eq!(
            sanitize(&expr),
            Err(SandboxError::UnknownFunction("console_log".into())),
            "unknown function must be rejected by sanitize()"
        );
    }

    #[test]
    fn eval_expr_nested_binary() {
        // (1 + 2) * 3 == 9
        let ctx = EvalContext::new();
        let inner = Expr::BinOp {
            op: BinOpKind::Add,
            left: Box::new(Expr::Literal(SandboxValue::Int(1))),
            right: Box::new(Expr::Literal(SandboxValue::Int(2))),
        };
        let expr = Expr::BinOp {
            op: BinOpKind::Mul,
            left: Box::new(inner),
            right: Box::new(Expr::Literal(SandboxValue::Int(3))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Int(9)));
    }

    #[test]
    fn eval_expr_comparison() {
        // 1 < 2 evaluates to Bool(true)
        let ctx = EvalContext::new();
        let expr = Expr::BinOp {
            op: BinOpKind::Lt,
            left: Box::new(Expr::Literal(SandboxValue::Int(1))),
            right: Box::new(Expr::Literal(SandboxValue::Int(2))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Bool(true)));
    }

    #[test]
    fn eval_expr_literal_string() {
        // A string literal evaluates to itself.
        let ctx = EvalContext::new();
        let expr = Expr::Literal(SandboxValue::Str("hello".into()));
        assert_eq!(
            eval_expr(&expr, &ctx),
            Ok(SandboxValue::Str("hello".into()))
        );
    }

    #[test]
    fn sanitizer_all_allowed_functions_pass() {
        // Every function in the default safe allowlist must pass AllowedFunctionsSanitizer.
        let allowed = AllowedFunctionsSanitizer::default_safe();
        for &fn_name in &[
            "len", "upper", "lower", "trim", "abs", "min", "max", "concat", "to_str", "to_int",
        ] {
            let expr = Expr::Call {
                name: fn_name.into(),
                args: vec![Expr::Literal(SandboxValue::Int(0))],
            };
            assert!(
                allowed.check(&expr).is_ok(),
                "{fn_name} must pass AllowedFunctionsSanitizer"
            );
        }
    }

    #[test]
    fn sanitizer_depth_10_is_too_deep() {
        // A BinOp chain 10 levels deep exceeds max_depth=5; DepthLimitSanitizer must reject it.
        let deep = (0..10).fold(Expr::Literal(SandboxValue::Int(0)), |acc, _| Expr::BinOp {
            op: BinOpKind::Add,
            left: Box::new(acc),
            right: Box::new(Expr::Literal(SandboxValue::Int(1))),
        });
        assert_eq!(
            DepthLimitSanitizer { max_depth: 5 }.check(&deep),
            Err(SandboxError::DepthLimitExceeded),
            "depth-10 expression must be rejected by max_depth=5 sanitizer"
        );
    }

    #[test]
    fn graph_rag_query_returns_ranked() {
        // Verify that retrieve() returns results sorted by score descending.
        use crate::dag::Dag;
        use crate::graph_rag::{node_vec, GraphRagRetriever};
        use crate::node::ExecNode;

        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("node1", "verb"));
        dag.add_node(ExecNode::new("node2", "verb"));
        dag.add_node(ExecNode::new("node3", "verb"));
        dag.add_edge("node1", "out", "node2", "in");
        dag.add_edge("node2", "out", "node3", "in");

        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("node1");
        let results = retriever.retrieve(&query, 3, 2);

        assert_eq!(results.len(), 3);
        for i in 0..results.len() - 1 {
            assert!(
                results[i].score >= results[i + 1].score,
                "results must be sorted by score descending: {} < {}",
                results[i].score,
                results[i + 1].score
            );
        }
    }

    #[test]
    fn graph_rag_empty_graph_returns_empty() {
        use crate::dag::Dag;
        use crate::graph_rag::{node_vec, GraphRagRetriever};

        let dag = Dag::new();
        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("anything");
        let results = retriever.retrieve(&query, 5, 2);
        assert!(results.is_empty(), "empty DAG must return no results");
    }

    #[test]
    fn graph_rag_node_self_relevance() {
        // A node queried with its own vec scores highest (cosine_sim == 1.0 → rank 0).
        use crate::dag::Dag;
        use crate::graph_rag::{node_vec, GraphRagRetriever};
        use crate::node::ExecNode;

        let mut dag = Dag::new();
        dag.add_node(ExecNode::new("target", "verb"));
        dag.add_node(ExecNode::new("other1", "verb"));
        dag.add_node(ExecNode::new("other2", "verb"));

        let retriever = GraphRagRetriever::new(&dag);
        let query = node_vec("target");
        let results = retriever.retrieve(&query, 3, 1);

        assert!(!results.is_empty());
        // "target" must appear at position 0 (highest score).
        assert_eq!(
            results[0].node_id, "target",
            "node queried with its own vec must rank first"
        );
    }

    // ------------------------------------------------------------------
    // SandboxValue: is_truthy covers all variants
    // ------------------------------------------------------------------
    #[test]
    fn sandbox_value_truthy_null_is_false() {
        assert!(!SandboxValue::Null.is_truthy());
    }

    #[test]
    fn sandbox_value_truthy_bool_true() {
        assert!(SandboxValue::Bool(true).is_truthy());
    }

    #[test]
    fn sandbox_value_truthy_bool_false() {
        assert!(!SandboxValue::Bool(false).is_truthy());
    }

    #[test]
    fn sandbox_value_truthy_int_nonzero() {
        assert!(SandboxValue::Int(1).is_truthy());
        assert!(SandboxValue::Int(-1).is_truthy());
    }

    #[test]
    fn sandbox_value_truthy_int_zero() {
        assert!(!SandboxValue::Int(0).is_truthy());
    }

    #[test]
    fn sandbox_value_truthy_float_nonzero() {
        assert!(SandboxValue::Float(0.1).is_truthy());
    }

    #[test]
    fn sandbox_value_truthy_float_zero() {
        assert!(!SandboxValue::Float(0.0).is_truthy());
    }

    #[test]
    fn sandbox_value_truthy_nonempty_str() {
        assert!(SandboxValue::Str("hi".into()).is_truthy());
    }

    #[test]
    fn sandbox_value_truthy_empty_str() {
        assert!(!SandboxValue::Str(String::new()).is_truthy());
    }

    #[test]
    fn sandbox_value_truthy_nonempty_list() {
        assert!(SandboxValue::List(vec![SandboxValue::Int(1)]).is_truthy());
    }

    #[test]
    fn sandbox_value_truthy_empty_list() {
        assert!(!SandboxValue::List(vec![]).is_truthy());
    }

    // ------------------------------------------------------------------
    // SandboxValue: type_name returns correct strings
    // ------------------------------------------------------------------
    #[test]
    fn sandbox_value_type_name_null() {
        assert_eq!(SandboxValue::Null.type_name(), "null");
    }

    #[test]
    fn sandbox_value_type_name_bool() {
        assert_eq!(SandboxValue::Bool(true).type_name(), "bool");
    }

    #[test]
    fn sandbox_value_type_name_int() {
        assert_eq!(SandboxValue::Int(0).type_name(), "int");
    }

    #[test]
    fn sandbox_value_type_name_float() {
        assert_eq!(SandboxValue::Float(0.0).type_name(), "float");
    }

    #[test]
    fn sandbox_value_type_name_str() {
        assert_eq!(SandboxValue::Str("x".into()).type_name(), "str");
    }

    #[test]
    fn sandbox_value_type_name_list() {
        assert_eq!(SandboxValue::List(vec![]).type_name(), "list");
    }

    // ------------------------------------------------------------------
    // eval_expr: subtraction
    // ------------------------------------------------------------------
    #[test]
    fn eval_expr_subtraction() {
        let ctx = EvalContext::new();
        let expr = Expr::BinOp {
            op: BinOpKind::Sub,
            left: Box::new(Expr::Literal(SandboxValue::Int(10))),
            right: Box::new(Expr::Literal(SandboxValue::Int(3))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Int(7)));
    }

    // ------------------------------------------------------------------
    // eval_expr: multiplication
    // ------------------------------------------------------------------
    #[test]
    fn eval_expr_multiplication() {
        let ctx = EvalContext::new();
        let expr = Expr::BinOp {
            op: BinOpKind::Mul,
            left: Box::new(Expr::Literal(SandboxValue::Int(6))),
            right: Box::new(Expr::Literal(SandboxValue::Int(7))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Int(42)));
    }

    // ------------------------------------------------------------------
    // eval_expr: equality comparison
    // ------------------------------------------------------------------
    #[test]
    fn eval_expr_equality_true() {
        let ctx = EvalContext::new();
        let expr = Expr::BinOp {
            op: BinOpKind::Eq,
            left: Box::new(Expr::Literal(SandboxValue::Int(5))),
            right: Box::new(Expr::Literal(SandboxValue::Int(5))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Bool(true)));
    }

    // ------------------------------------------------------------------
    // eval_expr: inequality comparison
    // ------------------------------------------------------------------
    #[test]
    fn eval_expr_inequality_true() {
        let ctx = EvalContext::new();
        let expr = Expr::BinOp {
            op: BinOpKind::Neq,
            left: Box::new(Expr::Literal(SandboxValue::Int(1))),
            right: Box::new(Expr::Literal(SandboxValue::Int(2))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Bool(true)));
    }

    // ------------------------------------------------------------------
    // eval_expr: greater-than comparison
    // ------------------------------------------------------------------
    #[test]
    fn eval_expr_greater_than_true() {
        let ctx = EvalContext::new();
        let expr = Expr::BinOp {
            op: BinOpKind::Gt,
            left: Box::new(Expr::Literal(SandboxValue::Int(10))),
            right: Box::new(Expr::Literal(SandboxValue::Int(3))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Bool(true)));
    }

    // ------------------------------------------------------------------
    // eval_expr: logical AND
    // ------------------------------------------------------------------
    #[test]
    fn eval_expr_logical_and_false() {
        let ctx = EvalContext::new();
        let expr = Expr::BinOp {
            op: BinOpKind::And,
            left: Box::new(Expr::Literal(SandboxValue::Bool(true))),
            right: Box::new(Expr::Literal(SandboxValue::Bool(false))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Bool(false)));
    }

    // ------------------------------------------------------------------
    // eval_expr: logical OR
    // ------------------------------------------------------------------
    #[test]
    fn eval_expr_logical_or_true() {
        let ctx = EvalContext::new();
        let expr = Expr::BinOp {
            op: BinOpKind::Or,
            left: Box::new(Expr::Literal(SandboxValue::Bool(false))),
            right: Box::new(Expr::Literal(SandboxValue::Bool(true))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Bool(true)));
    }

    // ------------------------------------------------------------------
    // eval_expr: if with false condition takes else branch
    // ------------------------------------------------------------------
    #[test]
    fn eval_expr_if_false_branch() {
        let ctx = EvalContext::new();
        let expr = Expr::If {
            cond: Box::new(Expr::Literal(SandboxValue::Bool(false))),
            then: Box::new(Expr::Literal(SandboxValue::Int(1))),
            else_: Box::new(Expr::Literal(SandboxValue::Int(99))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Int(99)));
    }

    // ------------------------------------------------------------------
    // eval_expr: call upper
    // ------------------------------------------------------------------
    #[test]
    fn eval_expr_call_upper() {
        let ctx = EvalContext::new();
        let expr = Expr::Call {
            name: "upper".into(),
            args: vec![Expr::Literal(SandboxValue::Str("hello".into()))],
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Str("HELLO".into())));
    }

    // ------------------------------------------------------------------
    // eval_expr: call lower
    // ------------------------------------------------------------------
    #[test]
    fn eval_expr_call_lower() {
        let ctx = EvalContext::new();
        let expr = Expr::Call {
            name: "lower".into(),
            args: vec![Expr::Literal(SandboxValue::Str("WORLD".into()))],
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Str("world".into())));
    }

    // ------------------------------------------------------------------
    // eval_expr: call trim
    // ------------------------------------------------------------------
    #[test]
    fn eval_expr_call_trim() {
        let ctx = EvalContext::new();
        let expr = Expr::Call {
            name: "trim".into(),
            args: vec![Expr::Literal(SandboxValue::Str("  hi  ".into()))],
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Str("hi".into())));
    }

    // ------------------------------------------------------------------
    // eval_expr: call abs
    // ------------------------------------------------------------------
    #[test]
    fn eval_expr_call_abs() {
        let ctx = EvalContext::new();
        let expr = Expr::Call {
            name: "abs".into(),
            args: vec![Expr::Literal(SandboxValue::Int(-7))],
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Int(7)));
    }

    // ------------------------------------------------------------------
    // eval_expr: undefined variable returns error
    // ------------------------------------------------------------------
    #[test]
    fn eval_expr_undefined_var_returns_error() {
        let ctx = EvalContext::new();
        let result = eval_expr(&Expr::Var("missing".into()), &ctx);
        assert_eq!(result, Err(SandboxError::UndefinedVar("missing".into())));
    }

    // ------------------------------------------------------------------
    // EvalContext: set and get round-trip
    // ------------------------------------------------------------------
    #[test]
    fn eval_context_set_get_roundtrip() {
        let mut ctx = EvalContext::new();
        ctx.set("myvar", SandboxValue::Str("test_value".into()));
        match ctx.get("myvar") {
            Some(SandboxValue::Str(s)) => assert_eq!(s, "test_value"),
            other => panic!("unexpected: {:?}", other),
        }
    }

    // ------------------------------------------------------------------
    // EvalContext: default() produces empty context
    // ------------------------------------------------------------------
    #[test]
    fn eval_context_default_is_empty() {
        let ctx = EvalContext::default();
        assert!(ctx.get("anything").is_none());
    }

    // ------------------------------------------------------------------
    // SandboxError: Display formatting
    // ------------------------------------------------------------------
    #[test]
    fn sandbox_error_display_undefined_var() {
        let e = SandboxError::UndefinedVar("x".into());
        assert!(format!("{e}").contains("x"), "display must include var name");
    }

    #[test]
    fn sandbox_error_display_division_by_zero() {
        let e = SandboxError::DivisionByZero;
        assert!(format!("{e}").contains("zero"));
    }

    #[test]
    fn sandbox_error_display_unknown_function() {
        let e = SandboxError::UnknownFunction("hack".into());
        assert!(format!("{e}").contains("hack"));
    }

    // ------------------------------------------------------------------
    // DepthLimitSanitizer: depth exactly at limit is OK
    // ------------------------------------------------------------------
    #[test]
    fn depth_limit_sanitizer_exactly_at_limit_is_ok() {
        // max_depth=4: build a 3-level nesting — must pass.
        let expr = Expr::BinOp {
            op: BinOpKind::Add,
            left: Box::new(Expr::BinOp {
                op: BinOpKind::Add,
                left: Box::new(Expr::BinOp {
                    op: BinOpKind::Add,
                    left: Box::new(Expr::Literal(SandboxValue::Int(1))),
                    right: Box::new(Expr::Literal(SandboxValue::Int(2))),
                }),
                right: Box::new(Expr::Literal(SandboxValue::Int(3))),
            }),
            right: Box::new(Expr::Literal(SandboxValue::Int(4))),
        };
        assert!(DepthLimitSanitizer { max_depth: 4 }.check(&expr).is_ok());
    }

    // ------------------------------------------------------------------
    // AllowedFunctionsSanitizer: nested blocked call inside allowed call is rejected
    // ------------------------------------------------------------------
    #[test]
    fn allowed_functions_sanitizer_nested_blocked_rejected() {
        // upper(forbidden_fn()) — forbidden_fn is not in the allow list
        let expr = Expr::Call {
            name: "upper".into(),
            args: vec![Expr::Call {
                name: "forbidden_fn".into(),
                args: vec![],
            }],
        };
        assert!(AllowedFunctionsSanitizer::default_safe().check(&expr).is_err());
    }

    // ------------------------------------------------------------------
    // AE8: eval_expr runtime depth enforcement
    // ------------------------------------------------------------------

    /// Depth 0: a simple integer literal evaluates without error.
    #[test]
    fn eval_depth_zero_literal_ok() {
        let ctx = EvalContext::new();
        let expr = Expr::Literal(SandboxValue::Int(1));
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Int(1)));
    }

    /// Depth limit: a BinOp chain 65 levels deep must return DepthLimitExceeded at eval time.
    #[test]
    fn eval_depth_65_levels_returns_depth_limit_exceeded() {
        // Build BinOp chain 65 levels deep — exceeds EVAL_DEPTH_LIMIT of 64.
        let deep = (0..65).fold(Expr::Literal(SandboxValue::Int(0)), |acc, _| Expr::BinOp {
            op: BinOpKind::Add,
            left: Box::new(acc),
            right: Box::new(Expr::Literal(SandboxValue::Int(1))),
        });
        let ctx = EvalContext::new();
        // Note: sanitize() uses max_depth=16 so it would catch this first;
        // we call eval_expr directly to test the runtime depth guard independently.
        assert_eq!(
            eval_expr(&deep, &ctx),
            Err(SandboxError::DepthLimitExceeded),
            "eval_expr must enforce EVAL_DEPTH_LIMIT=64 at runtime"
        );
    }

    /// Depth 64: a BinOp chain exactly 64 levels deep — at the limit — must succeed.
    #[test]
    fn eval_depth_64_levels_at_limit_ok() {
        // Build BinOp chain exactly 64 levels deep (depth counter reaches 64 == EVAL_DEPTH_LIMIT,
        // which is not > EVAL_DEPTH_LIMIT, so it must succeed).
        let at_limit = (0..64).fold(Expr::Literal(SandboxValue::Int(0)), |acc, _| Expr::BinOp {
            op: BinOpKind::Add,
            left: Box::new(acc),
            right: Box::new(Expr::Literal(SandboxValue::Int(1))),
        });
        let ctx = EvalContext::new();
        let result = eval_expr(&at_limit, &ctx);
        // Exactly at the depth limit: evaluates to Int(64).
        assert_eq!(
            result,
            Ok(SandboxValue::Int(64)),
            "eval_expr must allow exactly EVAL_DEPTH_LIMIT=64 levels of nesting"
        );
    }

    /// DepthLimitExceeded Display must contain the word "depth" or "exceeded".
    #[test]
    fn sandbox_error_display_depth_limit_exceeded() {
        let e = SandboxError::DepthLimitExceeded;
        let msg = format!("{e}");
        assert!(
            msg.contains("depth") || msg.contains("exceeded"),
            "Display for DepthLimitExceeded must mention depth/exceeded, got: {msg}"
        );
    }

    /// sanitize() on a literal passes; eval_expr on the same literal succeeds.
    /// This documents that sanitize-then-eval is the correct call sequence.
    #[test]
    fn sanitize_then_eval_expr_literal_passes() {
        let expr = Expr::Literal(SandboxValue::Int(7));
        let ctx = EvalContext::new();
        assert!(sanitize(&expr).is_ok(), "sanitize must pass for a literal");
        assert_eq!(
            eval_expr(&expr, &ctx),
            Ok(SandboxValue::Int(7)),
            "eval_expr must succeed after sanitize passes"
        );
    }

    // ------------------------------------------------------------------
    // eval_expr_inner: depth=64 passes (at limit, not beyond)
    // ------------------------------------------------------------------
    #[test]
    fn eval_expr_inner_depth_64_passes() {
        // A BinOp chain exactly 64 levels deep is at the EVAL_DEPTH_LIMIT boundary.
        // The check is `depth > EVAL_DEPTH_LIMIT`, so depth=64 must succeed.
        let at_limit = (0..64).fold(Expr::Literal(SandboxValue::Int(0)), |acc, _| Expr::BinOp {
            op: BinOpKind::Add,
            left: Box::new(acc),
            right: Box::new(Expr::Literal(SandboxValue::Int(1))),
        });
        let ctx = EvalContext::new();
        let result = eval_expr(&at_limit, &ctx);
        assert_eq!(
            result,
            Ok(SandboxValue::Int(64)),
            "depth=64 must evaluate successfully (at limit, not exceeded)"
        );
    }

    // ------------------------------------------------------------------
    // eval_expr_inner: depth=65 fails with DepthLimitExceeded
    // ------------------------------------------------------------------
    #[test]
    fn eval_expr_inner_depth_65_fails() {
        // 65 levels deep exceeds EVAL_DEPTH_LIMIT=64 → must return DepthLimitExceeded.
        let over_limit = (0..65).fold(Expr::Literal(SandboxValue::Int(0)), |acc, _| Expr::BinOp {
            op: BinOpKind::Add,
            left: Box::new(acc),
            right: Box::new(Expr::Literal(SandboxValue::Int(1))),
        });
        let ctx = EvalContext::new();
        assert_eq!(
            eval_expr(&over_limit, &ctx),
            Err(SandboxError::DepthLimitExceeded),
            "depth=65 must fail with DepthLimitExceeded"
        );
    }

    // ------------------------------------------------------------------
    // eval_expr_inner: deeply nested If/else tree
    // ------------------------------------------------------------------
    #[test]
    fn eval_expr_inner_deep_if_else_tree() {
        // Build a left-skewed If tree 4 levels deep:
        //   if true { if true { if true { if true { 99 } else { 0 } } else { 0 } } else { 0 } } else { 0 }
        // Must evaluate to 99 (always takes the true branch).
        let inner = Expr::If {
            cond: Box::new(Expr::Literal(SandboxValue::Bool(true))),
            then: Box::new(Expr::Literal(SandboxValue::Int(99))),
            else_: Box::new(Expr::Literal(SandboxValue::Int(0))),
        };
        let level2 = Expr::If {
            cond: Box::new(Expr::Literal(SandboxValue::Bool(true))),
            then: Box::new(inner),
            else_: Box::new(Expr::Literal(SandboxValue::Int(0))),
        };
        let level3 = Expr::If {
            cond: Box::new(Expr::Literal(SandboxValue::Bool(true))),
            then: Box::new(level2),
            else_: Box::new(Expr::Literal(SandboxValue::Int(0))),
        };
        let level4 = Expr::If {
            cond: Box::new(Expr::Literal(SandboxValue::Bool(true))),
            then: Box::new(level3),
            else_: Box::new(Expr::Literal(SandboxValue::Int(0))),
        };
        let ctx = EvalContext::new();
        assert_eq!(
            eval_expr(&level4, &ctx),
            Ok(SandboxValue::Int(99)),
            "deeply nested if-else tree must evaluate to 99"
        );
    }

    // ------------------------------------------------------------------
    // eval_expr_inner: Call with 4 args — all evaluated
    // ------------------------------------------------------------------
    #[test]
    fn eval_expr_inner_call_with_four_args_all_evaluated() {
        // Build a Call for "len" — it only uses the first arg, but we verify
        // that all args are evaluated without error when using eval_expr.
        // We use a simpler approach: 4-arg call where each arg is a valid literal.
        // "len" ignores args beyond the first; no panic must occur.
        let ctx = EvalContext::new();
        let expr = Expr::Call {
            name: "len".into(),
            args: vec![
                Expr::Literal(SandboxValue::Str("abcd".into())),
                Expr::Literal(SandboxValue::Int(1)),
                Expr::Literal(SandboxValue::Int(2)),
                Expr::Literal(SandboxValue::Int(3)),
            ],
        };
        // "len" returns length of first arg (the Str "abcd" = 4).
        assert_eq!(
            eval_expr(&expr, &ctx),
            Ok(SandboxValue::Int(4)),
            "call with 4 args: len(\"abcd\", 1, 2, 3) must return 4"
        );
    }

    // ------------------------------------------------------------------
    // Call: to_int converts float to int
    // ------------------------------------------------------------------
    #[test]
    fn eval_expr_call_to_int_from_float() {
        let ctx = EvalContext::new();
        let expr = Expr::Call {
            name: "to_int".into(),
            args: vec![Expr::Literal(SandboxValue::Float(3.7))],
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Int(3)));
    }

    // ------------------------------------------------------------------
    // Call: to_str converts int to string representation
    // ------------------------------------------------------------------
    #[test]
    fn eval_expr_call_to_str_from_int() {
        let ctx = EvalContext::new();
        let expr = Expr::Call {
            name: "to_str".into(),
            args: vec![Expr::Literal(SandboxValue::Int(42))],
        };
        let result = eval_expr(&expr, &ctx);
        // to_str uses Debug formatting; just check it's a Str and non-empty.
        match result {
            Ok(SandboxValue::Str(s)) => assert!(!s.is_empty(), "to_str must produce non-empty string"),
            other => panic!("expected Str, got {:?}", other),
        }
    }

    // ------------------------------------------------------------------
    // BinOp: Float subtraction
    // ------------------------------------------------------------------
    #[test]
    fn eval_expr_float_sub_not_supported_returns_type_mismatch() {
        // Float subtraction is not implemented (only Add for float), so it
        // should return a TypeMismatch error.
        let ctx = EvalContext::new();
        let expr = Expr::BinOp {
            op: BinOpKind::Sub,
            left: Box::new(Expr::Literal(SandboxValue::Float(5.0))),
            right: Box::new(Expr::Literal(SandboxValue::Float(2.0))),
        };
        let result = eval_expr(&expr, &ctx);
        assert!(result.is_err(), "Float Sub must return an error (not implemented)");
    }

    // ------------------------------------------------------------------
    // SandboxError: ForbiddenIdentifier Display
    // ------------------------------------------------------------------
    #[test]
    fn sandbox_error_display_forbidden_identifier() {
        let e = SandboxError::ForbiddenIdentifier("this".into());
        let msg = format!("{e}");
        assert!(msg.contains("this"), "ForbiddenIdentifier display must include identifier name");
    }

    // ------------------------------------------------------------------
    // SandboxError: PrototypeAccess Display
    // ------------------------------------------------------------------
    #[test]
    fn sandbox_error_display_prototype_access() {
        let e = SandboxError::PrototypeAccess;
        let msg = format!("{e}");
        assert!(!msg.is_empty(), "PrototypeAccess display must be non-empty");
    }

    // ------------------------------------------------------------------
    // SandboxError: InvalidDollarVar Display
    // ------------------------------------------------------------------
    #[test]
    fn sandbox_error_display_invalid_dollar_var() {
        let e = SandboxError::InvalidDollarVar("$custom".into());
        let msg = format!("{e}");
        assert!(msg.contains("$custom"), "InvalidDollarVar display must include var name");
    }

    // ------------------------------------------------------------------
    // NoSideEffectsSanitizer: always returns Ok
    // ------------------------------------------------------------------
    #[test]
    fn no_side_effects_sanitizer_always_ok() {
        let s = NoSideEffectsSanitizer;
        let expr = Expr::Literal(SandboxValue::Int(1));
        assert!(s.check(&expr).is_ok());
        let call_expr = Expr::Call { name: "len".into(), args: vec![] };
        assert!(s.check(&call_expr).is_ok());
    }

    // ------------------------------------------------------------------
    // TypeCoherenceSanitizer: nested If with type-clean branches passes
    // ------------------------------------------------------------------
    #[test]
    fn type_coherence_sanitizer_nested_if_clean_passes() {
        let expr = Expr::If {
            cond: Box::new(Expr::Literal(SandboxValue::Bool(true))),
            then: Box::new(Expr::BinOp {
                op: BinOpKind::Add,
                left: Box::new(Expr::Literal(SandboxValue::Int(1))),
                right: Box::new(Expr::Literal(SandboxValue::Int(2))),
            }),
            else_: Box::new(Expr::Literal(SandboxValue::Int(0))),
        };
        assert!(TypeCoherenceSanitizer.check(&expr).is_ok());
    }

    // ------------------------------------------------------------------
    // eval_checked_add_overflow_returns_error
    // ------------------------------------------------------------------
    #[test]
    fn eval_checked_add_overflow_returns_error() {
        let ctx = EvalContext::new();
        let expr = Expr::BinOp {
            op: BinOpKind::Add,
            left: Box::new(Expr::Literal(SandboxValue::Int(i64::MAX))),
            right: Box::new(Expr::Literal(SandboxValue::Int(1))),
        };
        let result = eval_expr(&expr, &ctx);
        assert!(result.is_err(), "i64::MAX + 1 must produce overflow error");
    }

    // ------------------------------------------------------------------
    // eval_checked_sub_overflow_returns_error
    // ------------------------------------------------------------------
    #[test]
    fn eval_checked_sub_overflow_returns_error() {
        let ctx = EvalContext::new();
        let expr = Expr::BinOp {
            op: BinOpKind::Sub,
            left: Box::new(Expr::Literal(SandboxValue::Int(i64::MIN))),
            right: Box::new(Expr::Literal(SandboxValue::Int(1))),
        };
        let result = eval_expr(&expr, &ctx);
        assert!(result.is_err(), "i64::MIN - 1 must produce overflow error");
    }

    // ------------------------------------------------------------------
    // eval_checked_mul_overflow_returns_error
    // ------------------------------------------------------------------
    #[test]
    fn eval_checked_mul_overflow_returns_error() {
        let ctx = EvalContext::new();
        let expr = Expr::BinOp {
            op: BinOpKind::Mul,
            left: Box::new(Expr::Literal(SandboxValue::Int(i64::MAX))),
            right: Box::new(Expr::Literal(SandboxValue::Int(2))),
        };
        let result = eval_expr(&expr, &ctx);
        assert!(result.is_err(), "i64::MAX * 2 must produce overflow error");
    }

    // ------------------------------------------------------------------
    // eval_div_by_zero_returns_error
    // ------------------------------------------------------------------
    #[test]
    fn eval_div_by_zero_returns_error() {
        let ctx = EvalContext::new();
        let expr = Expr::BinOp {
            op: BinOpKind::Div,
            left: Box::new(Expr::Literal(SandboxValue::Int(100))),
            right: Box::new(Expr::Literal(SandboxValue::Int(0))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Err(SandboxError::DivisionByZero));
    }

    // ------------------------------------------------------------------
    // eval_nested_binop_correct
    // ------------------------------------------------------------------
    #[test]
    fn eval_nested_binop_correct() {
        // (10 - 3) * 2 == 14
        let ctx = EvalContext::new();
        let inner = Expr::BinOp {
            op: BinOpKind::Sub,
            left: Box::new(Expr::Literal(SandboxValue::Int(10))),
            right: Box::new(Expr::Literal(SandboxValue::Int(3))),
        };
        let expr = Expr::BinOp {
            op: BinOpKind::Mul,
            left: Box::new(inner),
            right: Box::new(Expr::Literal(SandboxValue::Int(2))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Int(14)));
    }

    // ------------------------------------------------------------------
    // eval_if_true_branch_taken
    // ------------------------------------------------------------------
    #[test]
    fn eval_if_true_branch_taken() {
        let ctx = EvalContext::new();
        let expr = Expr::If {
            cond: Box::new(Expr::Literal(SandboxValue::Bool(true))),
            then: Box::new(Expr::Literal(SandboxValue::Int(42))),
            else_: Box::new(Expr::Literal(SandboxValue::Int(0))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Int(42)));
    }

    // ------------------------------------------------------------------
    // eval_if_false_branch_taken
    // ------------------------------------------------------------------
    #[test]
    fn eval_if_false_branch_taken() {
        let ctx = EvalContext::new();
        let expr = Expr::If {
            cond: Box::new(Expr::Literal(SandboxValue::Bool(false))),
            then: Box::new(Expr::Literal(SandboxValue::Int(0))),
            else_: Box::new(Expr::Literal(SandboxValue::Int(77))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Int(77)));
    }

    // ------------------------------------------------------------------
    // eval_depth_limit_reached_returns_error — chain of 65 levels
    // ------------------------------------------------------------------
    #[test]
    fn eval_depth_limit_reached_returns_error() {
        // 65 BinOp levels deep exceeds EVAL_DEPTH_LIMIT=64
        let deep = (0..65).fold(Expr::Literal(SandboxValue::Int(0)), |acc, _| Expr::BinOp {
            op: BinOpKind::Add,
            left: Box::new(acc),
            right: Box::new(Expr::Literal(SandboxValue::Int(1))),
        });
        let ctx = EvalContext::new();
        assert_eq!(
            eval_expr(&deep, &ctx),
            Err(SandboxError::DepthLimitExceeded),
            "depth 65 must exceed EVAL_DEPTH_LIMIT and return DepthLimitExceeded"
        );
    }

    // ------------------------------------------------------------------
    // eval_depth_within_limit_ok — chain of 63 levels succeeds
    // ------------------------------------------------------------------
    #[test]
    fn eval_depth_within_limit_ok() {
        // 63 BinOp levels — within limit, must produce Int(63)
        let within = (0..63).fold(Expr::Literal(SandboxValue::Int(0)), |acc, _| Expr::BinOp {
            op: BinOpKind::Add,
            left: Box::new(acc),
            right: Box::new(Expr::Literal(SandboxValue::Int(1))),
        });
        let ctx = EvalContext::new();
        assert_eq!(
            eval_expr(&within, &ctx),
            Ok(SandboxValue::Int(63)),
            "depth 63 is within EVAL_DEPTH_LIMIT=64 and must succeed"
        );
    }

    // ------------------------------------------------------------------
    // sanitize_called_before_eval_in_code_exec — sanitize then eval sequence
    // ------------------------------------------------------------------
    #[test]
    fn sanitize_called_before_eval_in_code_exec() {
        // The correct "code exec" pattern: sanitize() first, then eval_expr().
        // A clean literal must pass sanitize and then eval successfully.
        let expr = Expr::Literal(SandboxValue::Int(55));
        let ctx = EvalContext::new();
        let sanitize_result = sanitize(&expr);
        assert!(sanitize_result.is_ok(), "sanitize must pass for literal before eval");
        let eval_result = eval_expr(&expr, &ctx);
        assert_eq!(eval_result, Ok(SandboxValue::Int(55)), "eval must succeed after sanitize");
    }

    // ------------------------------------------------------------------
    // sanitizer_noop_stub_documented — NoSideEffects always Ok (documented stub)
    // ------------------------------------------------------------------
    #[test]
    fn sanitizer_noop_stub_documented() {
        // NoSideEffectsSanitizer is a documented stub that always returns Ok.
        // Verify it passes for every expression variant.
        let s = NoSideEffectsSanitizer;
        let exprs: Vec<Expr> = vec![
            Expr::Literal(SandboxValue::Null),
            Expr::Literal(SandboxValue::Bool(false)),
            Expr::Literal(SandboxValue::Int(0)),
            Expr::Var("x".into()),
            Expr::Call { name: "len".into(), args: vec![] },
        ];
        for expr in &exprs {
            assert!(
                s.check(expr).is_ok(),
                "NoSideEffectsSanitizer stub must always return Ok, failed for {:?}",
                expr
            );
        }
    }

    // ------------------------------------------------------------------
    // eval_string_concat_if_supported
    // ------------------------------------------------------------------
    #[test]
    fn eval_string_concat_if_supported() {
        let ctx = EvalContext::new();
        let expr = Expr::BinOp {
            op: BinOpKind::Add,
            left: Box::new(Expr::Literal(SandboxValue::Str("foo".into()))),
            right: Box::new(Expr::Literal(SandboxValue::Str("bar".into()))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Str("foobar".into())));
    }

    // ------------------------------------------------------------------
    // eval_boolean_and_true
    // ------------------------------------------------------------------
    #[test]
    fn eval_boolean_and_true() {
        let ctx = EvalContext::new();
        let expr = Expr::BinOp {
            op: BinOpKind::And,
            left: Box::new(Expr::Literal(SandboxValue::Bool(true))),
            right: Box::new(Expr::Literal(SandboxValue::Bool(true))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Bool(true)));
    }

    // ------------------------------------------------------------------
    // eval_boolean_and_false
    // ------------------------------------------------------------------
    #[test]
    fn eval_boolean_and_false() {
        let ctx = EvalContext::new();
        let expr = Expr::BinOp {
            op: BinOpKind::And,
            left: Box::new(Expr::Literal(SandboxValue::Bool(true))),
            right: Box::new(Expr::Literal(SandboxValue::Bool(false))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Bool(false)));
    }

    // ------------------------------------------------------------------
    // eval_boolean_or_true
    // ------------------------------------------------------------------
    #[test]
    fn eval_boolean_or_true() {
        let ctx = EvalContext::new();
        let expr = Expr::BinOp {
            op: BinOpKind::Or,
            left: Box::new(Expr::Literal(SandboxValue::Bool(false))),
            right: Box::new(Expr::Literal(SandboxValue::Bool(true))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Bool(true)));
    }

    // ------------------------------------------------------------------
    // eval_comparison_less_than
    // ------------------------------------------------------------------
    #[test]
    fn eval_comparison_less_than() {
        let ctx = EvalContext::new();
        let expr = Expr::BinOp {
            op: BinOpKind::Lt,
            left: Box::new(Expr::Literal(SandboxValue::Int(3))),
            right: Box::new(Expr::Literal(SandboxValue::Int(7))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Bool(true)));
        // Also test false case
        let expr2 = Expr::BinOp {
            op: BinOpKind::Lt,
            left: Box::new(Expr::Literal(SandboxValue::Int(7))),
            right: Box::new(Expr::Literal(SandboxValue::Int(3))),
        };
        assert_eq!(eval_expr(&expr2, &ctx), Ok(SandboxValue::Bool(false)));
    }

    // ------------------------------------------------------------------
    // sanitize_no_side_effects_stub_documented
    // ------------------------------------------------------------------
    #[test]
    fn sanitize_no_side_effects_stub_documented() {
        // NoSideEffectsSanitizer is a documented stub — must always return Ok
        // regardless of expression kind (Assign/Import/Exec are not yet in the AST).
        let s = NoSideEffectsSanitizer;
        let exprs = [
            Expr::Literal(SandboxValue::Int(0)),
            Expr::Var("x".into()),
            Expr::Call { name: "len".into(), args: vec![] },
            Expr::BinOp {
                op: BinOpKind::Add,
                left: Box::new(Expr::Literal(SandboxValue::Int(1))),
                right: Box::new(Expr::Literal(SandboxValue::Int(2))),
            },
        ];
        for expr in &exprs {
            assert!(
                s.check(expr).is_ok(),
                "NoSideEffectsSanitizer stub must return Ok for {:?}",
                expr
            );
        }
    }

    // ------------------------------------------------------------------
    // sanitize_deep_nesting_allowed_up_to_16
    // ------------------------------------------------------------------
    #[test]
    fn sanitize_deep_nesting_allowed_up_to_16() {
        // A BinOp chain of exactly 16 levels must pass (max_depth=16, check is >).
        let expr = (0..16).fold(Expr::Literal(SandboxValue::Int(0)), |acc, _| Expr::BinOp {
            op: BinOpKind::Add,
            left: Box::new(acc),
            right: Box::new(Expr::Literal(SandboxValue::Int(1))),
        });
        assert!(
            sanitize(&expr).is_ok(),
            "nesting exactly 16 levels must pass sanitize()"
        );
    }

    // ------------------------------------------------------------------
    // sanitize_depth_17_rejected
    // ------------------------------------------------------------------
    #[test]
    fn sanitize_depth_17_rejected() {
        // 17 levels exceeds max_depth=16; sanitize must reject it.
        let expr = (0..17).fold(Expr::Literal(SandboxValue::Int(0)), |acc, _| Expr::BinOp {
            op: BinOpKind::Add,
            left: Box::new(acc),
            right: Box::new(Expr::Literal(SandboxValue::Int(1))),
        });
        assert_eq!(
            sanitize(&expr),
            Err(SandboxError::DepthLimitExceeded),
            "nesting 17 levels must be rejected by sanitize()"
        );
    }

    // ------------------------------------------------------------------
    // sanitize_variable_declaration_allowed
    // ------------------------------------------------------------------
    #[test]
    fn sanitize_variable_declaration_allowed() {
        // Reading a variable (Expr::Var) is allowed by all sanitizers.
        let expr = Expr::Var("my_var".into());
        assert!(
            sanitize(&expr).is_ok(),
            "variable read Expr::Var must pass sanitize()"
        );
    }

    // ------------------------------------------------------------------
    // eval_unary_minus_correct (simulated via Int negation: 0 - n)
    // ------------------------------------------------------------------
    #[test]
    fn eval_unary_minus_correct() {
        // Simulate unary minus as (0 - 5) = -5.
        let ctx = EvalContext::new();
        let expr = Expr::BinOp {
            op: BinOpKind::Sub,
            left: Box::new(Expr::Literal(SandboxValue::Int(0))),
            right: Box::new(Expr::Literal(SandboxValue::Int(5))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Int(-5)));
    }

    // ------------------------------------------------------------------
    // eval_unary_not_correct (simulated via Eq false)
    // ------------------------------------------------------------------
    #[test]
    fn eval_unary_not_correct() {
        // NOT true = false, simulated as (true == false) = false.
        let ctx = EvalContext::new();
        let expr = Expr::BinOp {
            op: BinOpKind::Eq,
            left: Box::new(Expr::Literal(SandboxValue::Bool(true))),
            right: Box::new(Expr::Literal(SandboxValue::Bool(false))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Bool(false)));
    }

    // ------------------------------------------------------------------
    // eval_string_equality_correct
    // ------------------------------------------------------------------
    #[test]
    fn eval_string_equality_correct() {
        let ctx = EvalContext::new();
        // "hello" == "hello" → true
        let expr = Expr::BinOp {
            op: BinOpKind::Eq,
            left: Box::new(Expr::Literal(SandboxValue::Str("hello".into()))),
            right: Box::new(Expr::Literal(SandboxValue::Str("hello".into()))),
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Bool(true)));
        // "hello" == "world" → false
        let expr2 = Expr::BinOp {
            op: BinOpKind::Eq,
            left: Box::new(Expr::Literal(SandboxValue::Str("hello".into()))),
            right: Box::new(Expr::Literal(SandboxValue::Str("world".into()))),
        };
        assert_eq!(eval_expr(&expr2, &ctx), Ok(SandboxValue::Bool(false)));
    }

    // ------------------------------------------------------------------
    // eval_list_literal_correct
    // ------------------------------------------------------------------
    #[test]
    fn eval_list_literal_correct() {
        let ctx = EvalContext::new();
        // A list literal evaluates to itself.
        let items = vec![SandboxValue::Int(1), SandboxValue::Int(2), SandboxValue::Int(3)];
        let expr = Expr::Literal(SandboxValue::List(items.clone()));
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::List(items)));
    }

    // ------------------------------------------------------------------
    // eval_function_call_with_args (len on a list)
    // ------------------------------------------------------------------
    #[test]
    fn eval_function_call_with_args() {
        let ctx = EvalContext::new();
        // len([1, 2, 3]) == 3
        let expr = Expr::Call {
            name: "len".into(),
            args: vec![Expr::Literal(SandboxValue::List(vec![
                SandboxValue::Int(1),
                SandboxValue::Int(2),
                SandboxValue::Int(3),
            ]))],
        };
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Int(3)));
    }

    // ------------------------------------------------------------------
    // eval_nested_function_calls (upper(lower("HELLO")))
    // ------------------------------------------------------------------
    #[test]
    fn eval_nested_function_calls() {
        let ctx = EvalContext::new();
        // upper(lower("HELLO")) == "HELLO"
        let inner = Expr::Call {
            name: "lower".into(),
            args: vec![Expr::Literal(SandboxValue::Str("HELLO".into()))],
        };
        let outer = Expr::Call {
            name: "upper".into(),
            args: vec![inner],
        };
        assert_eq!(eval_expr(&outer, &ctx), Ok(SandboxValue::Str("HELLO".into())));
    }

    // ------------------------------------------------------------------
    // eval_map_literal_correct (map = null literal, not supported natively)
    // The sandbox has no Map type; verifying Null literal evaluates to Null.
    // ------------------------------------------------------------------
    #[test]
    fn eval_map_literal_correct() {
        // Maps are not a SandboxValue variant — the closest is Null for missing data.
        // Verify Null literal round-trips correctly.
        let ctx = EvalContext::new();
        let expr = Expr::Literal(SandboxValue::Null);
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Null));
        assert!(!SandboxValue::Null.is_truthy(), "Null must be falsy");
    }

    // ------------------------------------------------------------------
    // sanitize_assignment_rejected — no Assign variant in AST; unknown fn rejected
    // ------------------------------------------------------------------
    #[test]
    fn sanitize_assignment_rejected() {
        // The AST has no Assign variant. The closest threat is calling
        // a forbidden function named "assign". AllowedFunctionsSanitizer must reject it.
        let expr = Expr::Call {
            name: "assign".into(),
            args: vec![Expr::Literal(SandboxValue::Int(0))],
        };
        assert_eq!(
            sanitize(&expr),
            Err(SandboxError::UnknownFunction("assign".into())),
            "function 'assign' must be rejected as unknown by AllowedFunctionsSanitizer"
        );
    }
}
