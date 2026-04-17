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
            Self::Null => "null", Self::Bool(_) => "bool", Self::Int(_) => "int",
            Self::Float(_) => "float", Self::Str(_) => "str", Self::List(_) => "list",
        }
    }
}

/// A simple AST node for safe expression parsing.
#[derive(Debug, Clone)]
pub enum Expr {
    Literal(SandboxValue),
    Var(String),
    BinOp { op: BinOpKind, left: Box<Expr>, right: Box<Expr> },
    If { cond: Box<Expr>, then: Box<Expr>, else_: Box<Expr> },
    Call { name: String, args: Vec<Expr> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOpKind { Add, Sub, Mul, Div, Eq, Neq, Lt, Gt, And, Or }

/// Error from sandbox evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum SandboxError {
    UndefinedVar(String),
    DivisionByZero,
    TypeMismatch { expected: &'static str, got: &'static str },
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
            Self::TypeMismatch { expected, got } => write!(f, "type mismatch: expected {expected}, got {got}"),
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
pub struct DepthLimitSanitizer { pub max_depth: usize }
impl DepthLimitSanitizer {
    pub fn check(&self, expr: &Expr) -> Result<(), SandboxError> {
        self.check_depth(expr, 0)
    }
    fn check_depth(&self, expr: &Expr, depth: usize) -> Result<(), SandboxError> {
        if depth > self.max_depth { return Err(SandboxError::DepthLimitExceeded); }
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
pub struct AllowedFunctionsSanitizer { pub allowed: Vec<&'static str> }
impl AllowedFunctionsSanitizer {
    pub fn default_safe() -> Self {
        Self { allowed: vec!["len", "upper", "lower", "trim", "abs", "min", "max", "concat", "to_str", "to_int"] }
    }
    pub fn check(&self, expr: &Expr) -> Result<(), SandboxError> {
        match expr {
            Expr::Call {
                name, args,
            } => {
                if !self.allowed.contains(&name.as_str()) {
                    return Err(SandboxError::UnknownFunction(name.clone()));
                }
                args.iter().try_for_each(|a| self.check(a))
            }
            Expr::BinOp { left, right, .. } => { self.check(left)?; self.check(right) }
            Expr::If { cond, then, else_ } => { self.check(cond)?; self.check(then)?; self.check(else_) }
            _ => Ok(()),
        }
    }
}

/// --- Sanitizer 3: NoSideEffectsSanitizer ---
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
                if matches!(op, BinOpKind::Add | BinOpKind::Sub | BinOpKind::Mul | BinOpKind::Div) {
                    if let (Expr::Literal(SandboxValue::Str(_)), Expr::Literal(SandboxValue::Int(_)))
                        | (Expr::Literal(SandboxValue::Int(_)), Expr::Literal(SandboxValue::Str(_))) = (left.as_ref(), right.as_ref()) {
                        return Err(SandboxError::TypeMismatch { expected: "numeric", got: "str" });
                    }
                }
                Ok(())
            }
            Expr::If { cond, then, else_ } => { self.check(cond)?; self.check(then)?; self.check(else_) }
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
            Expr::Var(name) if name == "this" => Err(SandboxError::ForbiddenIdentifier("this".into())),
            Expr::Var(_) | Expr::Literal(_) => Ok(()),
            Expr::BinOp { left, right, .. } => { self.check(left)?; self.check(right) }
            Expr::If { cond, then, else_ } => { self.check(cond)?; self.check(then)?; self.check(else_) }
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
            Expr::BinOp { left, right, .. } => { self.check(left)?; self.check(right) }
            Expr::If { cond, then, else_ } => { self.check(cond)?; self.check(then)?; self.check(else_) }
            Expr::Call { args, .. } => args.iter().try_for_each(|a| self.check(a)),
        }
    }
}

/// --- Sanitizer 7: DollarValidateSanitizer ---
/// Allows only known n8n dollar-prefixed variables; rejects unknown `$`-prefixed names.
pub struct DollarValidateSanitizer;
const ALLOWED_DOLLAR_VARS: &[&str] = &[
    "$input", "$json", "$node", "$workflow", "$item", "$items", "$runIndex",
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
            Expr::BinOp { left, right, .. } => { self.check(left)?; self.check(right) }
            Expr::If { cond, then, else_ } => { self.check(cond)?; self.check(then)?; self.check(else_) }
            Expr::Call { args, .. } => args.iter().try_for_each(|a| self.check(a)),
        }
    }
}

pub struct EvalContext {
    vars: std::collections::HashMap<String, SandboxValue>,
}

impl EvalContext {
    pub fn new() -> Self { Self { vars: std::collections::HashMap::new() } }
    pub fn set(&mut self, name: impl Into<String>, val: SandboxValue) { self.vars.insert(name.into(), val); }
    pub fn get(&self, name: &str) -> Option<&SandboxValue> { self.vars.get(name) }
}

impl Default for EvalContext { fn default() -> Self { Self::new() } }

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

pub fn eval_expr(expr: &Expr, ctx: &EvalContext) -> Result<SandboxValue, SandboxError> {
    match expr {
        Expr::Literal(v) => Ok(v.clone()),
        Expr::Var(name) => ctx.get(name).cloned().ok_or_else(|| SandboxError::UndefinedVar(name.clone())),
        Expr::BinOp { op, left, right } => {
            let l = eval_expr(left, ctx)?;
            let r = eval_expr(right, ctx)?;
            eval_binop(*op, l, r)
        }
        Expr::If { cond, then, else_ } => {
            if eval_expr(cond, ctx)?.is_truthy() { eval_expr(then, ctx) } else { eval_expr(else_, ctx) }
        }
        Expr::Call { name, args } => {
            let evaled: Result<Vec<_>, _> = args.iter().map(|a| eval_expr(a, ctx)).collect();
            eval_call(name, evaled?)
        }
    }
}

fn eval_binop(op: BinOpKind, l: SandboxValue, r: SandboxValue) -> Result<SandboxValue, SandboxError> {
    match (op, &l, &r) {
        (BinOpKind::Add, SandboxValue::Int(a), SandboxValue::Int(b)) => Ok(SandboxValue::Int(a + b)),
        (BinOpKind::Add, SandboxValue::Float(a), SandboxValue::Float(b)) => Ok(SandboxValue::Float(a + b)),
        (BinOpKind::Add, SandboxValue::Str(a), SandboxValue::Str(b)) => Ok(SandboxValue::Str(format!("{}{}", a, b))),
        (BinOpKind::Sub, SandboxValue::Int(a), SandboxValue::Int(b)) => Ok(SandboxValue::Int(a - b)),
        (BinOpKind::Mul, SandboxValue::Int(a), SandboxValue::Int(b)) => Ok(SandboxValue::Int(a * b)),
        (BinOpKind::Div, SandboxValue::Int(a), SandboxValue::Int(b)) => {
            if *b == 0 { Err(SandboxError::DivisionByZero) } else { Ok(SandboxValue::Int(a / b)) }
        }
        (BinOpKind::Eq, a, b) => Ok(SandboxValue::Bool(a == b)),
        (BinOpKind::Neq, a, b) => Ok(SandboxValue::Bool(a != b)),
        (BinOpKind::Lt, SandboxValue::Int(a), SandboxValue::Int(b)) => Ok(SandboxValue::Bool(a < b)),
        (BinOpKind::Gt, SandboxValue::Int(a), SandboxValue::Int(b)) => Ok(SandboxValue::Bool(a > b)),
        (BinOpKind::And, a, b) => Ok(SandboxValue::Bool(a.is_truthy() && b.is_truthy())),
        (BinOpKind::Or, a, b) => Ok(SandboxValue::Bool(a.is_truthy() || b.is_truthy())),
        _ => Err(SandboxError::TypeMismatch { expected: "compatible types", got: "incompatible" }),
    }
}

fn eval_call(name: &str, args: Vec<SandboxValue>) -> Result<SandboxValue, SandboxError> {
    match name {
        "len" => match args.first() {
            Some(SandboxValue::Str(s)) => Ok(SandboxValue::Int(s.len() as i64)),
            Some(SandboxValue::List(l)) => Ok(SandboxValue::Int(l.len() as i64)),
            _ => Err(SandboxError::TypeMismatch { expected: "str or list", got: "other" }),
        },
        "upper" => match args.first() {
            Some(SandboxValue::Str(s)) => Ok(SandboxValue::Str(s.to_uppercase())),
            _ => Err(SandboxError::TypeMismatch { expected: "str", got: "other" }),
        },
        "lower" => match args.first() {
            Some(SandboxValue::Str(s)) => Ok(SandboxValue::Str(s.to_lowercase())),
            _ => Err(SandboxError::TypeMismatch { expected: "str", got: "other" }),
        },
        "trim" => match args.first() {
            Some(SandboxValue::Str(s)) => Ok(SandboxValue::Str(s.trim().to_string())),
            _ => Err(SandboxError::TypeMismatch { expected: "str", got: "other" }),
        },
        "abs" => match args.first() {
            Some(SandboxValue::Int(n)) => Ok(SandboxValue::Int(n.abs())),
            _ => Err(SandboxError::TypeMismatch { expected: "int", got: "other" }),
        },
        "to_str" => match args.first() {
            Some(v) => Ok(SandboxValue::Str(format!("{:?}", v))),
            None => Ok(SandboxValue::Str(String::new())),
        },
        "to_int" => match args.first() {
            Some(SandboxValue::Int(n)) => Ok(SandboxValue::Int(*n)),
            Some(SandboxValue::Float(f)) => Ok(SandboxValue::Int(*f as i64)),
            _ => Err(SandboxError::TypeMismatch { expected: "numeric", got: "other" }),
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
        assert_eq!(eval_expr(&Expr::Literal(SandboxValue::Int(42)), &ctx), Ok(SandboxValue::Int(42)));
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
        assert_eq!(eval_expr(&Expr::Var("x".into()), &ctx), Ok(SandboxValue::Int(99)));
        assert_eq!(eval_expr(&Expr::Var("y".into()), &ctx), Err(SandboxError::UndefinedVar("y".into())));
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
        let deep = (0..20).fold(Expr::Literal(SandboxValue::Int(0)), |acc, _| {
            Expr::BinOp { op: BinOpKind::Add, left: Box::new(acc), right: Box::new(Expr::Literal(SandboxValue::Int(1))) }
        });
        assert_eq!(DepthLimitSanitizer { max_depth: 5 }.check(&deep), Err(SandboxError::DepthLimitExceeded));
    }
    #[test]
    fn sanitizer_blocked_function() {
        let expr = Expr::Call { name: "exec".into(), args: vec![] };
        assert!(AllowedFunctionsSanitizer::default_safe().check(&expr).is_err());
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
        let deep = (0..17).fold(Expr::Literal(SandboxValue::Int(0)), |acc, _| {
            Expr::BinOp {
                op: BinOpKind::Add,
                left: Box::new(acc),
                right: Box::new(Expr::Literal(SandboxValue::Int(1))),
            }
        });
        let result = sanitize(&deep);
        assert_eq!(result, Err(SandboxError::DepthLimitExceeded), "nesting > 16 must be rejected");
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
        assert_eq!(eval_expr(&expr, &ctx), Ok(SandboxValue::Str("hello world".into())));
    }

    #[test]
    fn eval_expr_depth_limit_respected() {
        // Build a BinOp chain 18 levels deep — exceeds default max_depth of 16.
        let deep = (0..18).fold(Expr::Literal(SandboxValue::Int(0)), |acc, _| {
            Expr::BinOp {
                op: BinOpKind::Add,
                left: Box::new(acc),
                right: Box::new(Expr::Literal(SandboxValue::Int(1))),
            }
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
    fn graph_rag_query_returns_ranked() {
        // Verify that retrieve() returns results sorted by score descending.
        use crate::dag::Dag;
        use crate::node::ExecNode;
        use crate::graph_rag::{GraphRagRetriever, node_vec};

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
                results[i].score, results[i + 1].score
            );
        }
    }

    #[test]
    fn graph_rag_empty_graph_returns_empty() {
        use crate::dag::Dag;
        use crate::graph_rag::{GraphRagRetriever, node_vec};

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
        use crate::node::ExecNode;
        use crate::graph_rag::{GraphRagRetriever, node_vec};

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
}
