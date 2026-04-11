//! nom-verifier: Contract compatibility checker for Nom composition graphs.
//!
//! The verifier walks a parsed [`SourceFile`] and checks:
//!
//! 1. **Type compatibility** — the `output_type` of step N must match the
//!    `input_type` of step N+1 in every flow chain.
//!
//! 2. **Constraint satisfaction** — `require` constraints (e.g. `security>0.9`)
//!    are checked against the resolved scores of referenced words.
//!
//! 3. **Effect propagation** — effects declared by `effects only [...]` must
//!    be a superset of all effects produced by the words in the declaration.

use nom_ast::{
    BinOp, BranchArm, BranchBlock, BranchCondition, Classifier, CompareOp, Constraint,
    ContractStmt, Declaration, EffectModifier, Expr, FlowChain, FlowStep, GraphQueryExpr,
    GraphSetExpr, GraphSetOp, GraphTraverseExpr, Literal, NomRef, SourceFile, Statement, UnaryOp,
};
use nom_resolver::{Resolver, ResolverError, WordEntry};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VerifyError {
    #[error("resolver error: {0}")]
    Resolver(#[from] ResolverError),
    #[error(
        "type mismatch in flow '{flow}': step '{a}' outputs '{out}' but step '{b}' expects '{inp}'"
    )]
    TypeMismatch {
        flow: String,
        a: String,
        b: String,
        out: String,
        inp: String,
    },
    #[error("constraint not satisfied in '{context}': {constraint}")]
    ConstraintFailed { context: String, constraint: String },
    #[error(
        "undeclared effect '{effect}' in '{context}': word '{word}' produces it but it is not listed"
    )]
    UndeclaredEffect {
        context: String,
        effect: String,
        word: String,
    },
    #[error("word not found during verification: {0}")]
    UnresolvedWord(String),
    #[error("warning in '{context}': {message}")]
    Warning { context: String, message: String },
}

impl VerifyError {
    /// Returns true if this is a hard error (not a warning).
    pub fn is_error(&self) -> bool {
        !matches!(self, VerifyError::Warning { .. })
    }
}

/// A single verification finding.
#[derive(Debug)]
pub struct Finding {
    pub error: VerifyError,
    /// The name of the declaration where the finding was raised.
    pub declaration: String,
}

/// Summary result of verifying a source file.
#[derive(Debug, Default)]
pub struct VerifyResult {
    pub findings: Vec<Finding>,
}

impl VerifyResult {
    pub fn ok(&self) -> bool {
        self.findings.iter().all(|f| !f.error.is_error())
    }

    pub fn error_count(&self) -> usize {
        self.findings.iter().filter(|f| f.error.is_error()).count()
    }

    pub fn warning_count(&self) -> usize {
        self.findings.iter().filter(|f| !f.error.is_error()).count()
    }

    pub fn push(&mut self, decl: &str, error: VerifyError) {
        self.findings.push(Finding {
            error,
            declaration: decl.to_owned(),
        });
    }
}

/// Verifier holds a reference to a [`Resolver`] for database lookups.
pub struct Verifier<'r> {
    resolver: &'r Resolver,
}

impl<'r> Verifier<'r> {
    pub fn new(resolver: &'r Resolver) -> Self {
        Self { resolver }
    }

    /// Verify an entire source file and return accumulated findings.
    pub fn verify(&self, source: &SourceFile) -> VerifyResult {
        let mut result = VerifyResult::default();

        // Structural checks across the whole source file
        self.check_duplicate_declarations(source, &mut result);
        self.check_empty_declarations(source, &mut result);

        for decl in &source.declarations {
            self.verify_declaration(decl, &mut result);
        }
        result
    }

    /// Warn if two declarations share the same name.
    fn check_duplicate_declarations(&self, source: &SourceFile, result: &mut VerifyResult) {
        let mut seen = std::collections::HashSet::new();
        for decl in &source.declarations {
            let key = format!("{}::{}", decl.classifier.as_str(), decl.name.name);
            if !seen.insert(key.clone()) {
                result.push(
                    &decl.name.name,
                    VerifyError::ConstraintFailed {
                        context: decl.name.name.clone(),
                        constraint: format!(
                            "duplicate declaration: {} '{}' defined more than once",
                            decl.classifier.as_str(),
                            decl.name.name
                        ),
                    },
                );
            }
        }
    }

    /// Warn if a declaration has no statements at all.
    fn check_empty_declarations(&self, source: &SourceFile, result: &mut VerifyResult) {
        for decl in &source.declarations {
            if decl.statements.is_empty() {
                result.push(
                    &decl.name.name,
                    VerifyError::Warning {
                        context: decl.name.name.clone(),
                        message: "empty declaration has no statements".to_owned(),
                    },
                );
            }
        }
    }

    fn verify_declaration(&self, decl: &Declaration, result: &mut VerifyResult) {
        let name = &decl.name.name;

        // Collect declared effects (from `effects only [...]` statements)
        let mut declared_effects: Option<Vec<String>> = None;
        let mut declared_only = false;

        for stmt in &decl.statements {
            if let Statement::Effects(e) = stmt {
                let names: Vec<String> = e.effects.iter().map(|id| id.name.clone()).collect();
                declared_only = matches!(e.modifier, Some(EffectModifier::Only));
                declared_effects = Some(names);
            }
        }

        for stmt in &decl.statements {
            match stmt {
                Statement::Need(need) => {
                    self.verify_need(name, &need.reference, need.constraint.as_ref(), result);
                }
                Statement::Require(req) => {
                    self.verify_constraint(name, &req.constraint, None, result);
                }
                Statement::Flow(flow) => {
                    self.verify_declared_chain(
                        name,
                        &flow.chain,
                        declared_only,
                        declared_effects.as_ref(),
                        result,
                    );
                }
                Statement::GraphQuery(query) => {
                    let ctx = format!("{name}::{}", query.name.name);
                    self.verify_declared_graph_query(
                        &ctx,
                        &query.expr,
                        declared_only,
                        declared_effects.as_ref(),
                        result,
                    );
                }
                Statement::AgentReceive(receive) => {
                    let ctx = format!("{name}::receive");
                    self.verify_declared_chain(
                        &ctx,
                        &receive.chain,
                        declared_only,
                        declared_effects.as_ref(),
                        result,
                    );
                }
                Statement::AgentSchedule(schedule) => {
                    let ctx = format!("{name}::schedule({})", schedule.interval);
                    self.verify_declared_chain(
                        &ctx,
                        &schedule.action,
                        declared_only,
                        declared_effects.as_ref(),
                        result,
                    );
                }
                // Imperative code quality checks
                Statement::FnDef(fndef) => {
                    // Warn on functions without return type annotation
                    if fndef.return_type.is_none() && !fndef.body.stmts.is_empty() {
                        result.push(
                            name,
                            VerifyError::Warning {
                                context: format!("{name}::fn {}", fndef.name.name),
                                message: "function has no return type annotation — add '-> type' for clarity".to_owned(),
                            },
                        );
                    }
                }
                _ => {}
            }
        }

        // Check: nom declarations with functions should have describe
        if decl.classifier == Classifier::Nom {
            let has_fn = decl.statements.iter().any(|s| matches!(s, Statement::FnDef(_)));
            let has_describe = decl.statements.iter().any(|s| matches!(s, Statement::Describe(_)));
            if has_fn && !has_describe {
                result.push(
                    name,
                    VerifyError::Warning {
                        context: name.to_owned(),
                        message: "nom with functions should have a 'describe' statement for documentation".to_owned(),
                    },
                );
            }
        }
    }

    fn verify_declared_chain(
        &self,
        ctx: &str,
        chain: &FlowChain,
        declared_only: bool,
        allowed_effects: Option<&Vec<String>>,
        result: &mut VerifyResult,
    ) {
        self.verify_flow_chain(ctx, chain, result);
        if declared_only {
            if let Some(allowed) = allowed_effects {
                self.check_flow_effects(ctx, chain, allowed, result);
            }
        }
    }

    fn verify_declared_graph_query(
        &self,
        ctx: &str,
        expr: &GraphQueryExpr,
        declared_only: bool,
        allowed_effects: Option<&Vec<String>>,
        result: &mut VerifyResult,
    ) {
        if let Some(chain) = Self::graph_query_expr_to_flow_chain(expr) {
            self.verify_declared_chain(ctx, &chain, declared_only, allowed_effects, result);
            return;
        }

        match expr {
            GraphQueryExpr::Ref(_) => {}
            GraphQueryExpr::Traverse(GraphTraverseExpr { source, target, .. }) => {
                self.verify_declared_graph_query(
                    ctx,
                    source,
                    declared_only,
                    allowed_effects,
                    result,
                );
                self.verify_declared_graph_query(
                    ctx,
                    target,
                    declared_only,
                    allowed_effects,
                    result,
                );
            }
            GraphQueryExpr::SetOp(GraphSetExpr { operands, .. }) => {
                for operand in operands {
                    self.verify_declared_graph_query(
                        ctx,
                        operand,
                        declared_only,
                        allowed_effects,
                        result,
                    );
                }
            }
        }
    }

    fn verify_need(
        &self,
        ctx: &str,
        nom_ref: &NomRef,
        constraint: Option<&Constraint>,
        result: &mut VerifyResult,
    ) {
        match self.resolver.resolve(nom_ref) {
            Ok(entry) => {
                if let Some(c) = constraint {
                    self.verify_constraint_against_entry(ctx, c, &entry, result);
                }
            }
            Err(e) => result.push(ctx, VerifyError::Resolver(e)),
        }
    }

    fn verify_flow_chain(&self, ctx: &str, chain: &FlowChain, result: &mut VerifyResult) {
        let steps: Vec<Option<WordEntry>> = chain
            .steps
            .iter()
            .map(|step| self.resolve_step(step))
            .collect();

        // Check pairwise type compatibility
        for i in 0..steps.len().saturating_sub(1) {
            let a_entry = &steps[i];
            let b_entry = &steps[i + 1];
            if let (Some(a), Some(b)) = (a_entry, b_entry) {
                if let (Some(out), Some(inp)) = (&a.output_type, &b.input_type) {
                    if out != inp && inp != "any" && out != "any" {
                        result.push(
                            ctx,
                            VerifyError::TypeMismatch {
                                flow: ctx.to_owned(),
                                a: format!(
                                    "{}{}",
                                    a.word,
                                    a.variant
                                        .as_deref()
                                        .map(|v| format!("::{v}"))
                                        .unwrap_or_default()
                                ),
                                b: format!(
                                    "{}{}",
                                    b.word,
                                    b.variant
                                        .as_deref()
                                        .map(|v| format!("::{v}"))
                                        .unwrap_or_default()
                                ),
                                out: out.clone(),
                                inp: inp.clone(),
                            },
                        );
                    }
                }
            }
        }

        // Recurse into branch blocks
        for step in &chain.steps {
            if let FlowStep::Branch(block) = step {
                for arm in &block.arms {
                    self.verify_flow_chain(ctx, &arm.chain, result);
                }
            }
        }

        // Cycle detection: check for duplicate word names in the linear chain
        let mut seen = std::collections::HashSet::new();
        for step in &chain.steps {
            if let FlowStep::Ref(nom_ref) = step {
                let key = format!(
                    "{}{}",
                    nom_ref.word.name,
                    nom_ref
                        .variant
                        .as_ref()
                        .map(|v| format!("::{}", v.name))
                        .unwrap_or_default()
                );
                if !seen.insert(key.clone()) {
                    result.push(
                        ctx,
                        VerifyError::Warning {
                            context: ctx.to_owned(),
                            message: format!(
                                "possible cycle: '{}' appears more than once in flow chain",
                                key
                            ),
                        },
                    );
                }
            }
        }
    }

    fn check_flow_effects(
        &self,
        ctx: &str,
        chain: &FlowChain,
        allowed: &[String],
        result: &mut VerifyResult,
    ) {
        for step in &chain.steps {
            if let FlowStep::Ref(nom_ref) = step {
                if let Ok(entry) = self.resolver.resolve(nom_ref) {
                    for effect in &entry.effects {
                        if !allowed.contains(effect) {
                            result.push(
                                ctx,
                                VerifyError::UndeclaredEffect {
                                    context: ctx.to_owned(),
                                    effect: effect.clone(),
                                    word: entry.word.clone(),
                                },
                            );
                        }
                    }
                }
            }
            if let FlowStep::Branch(block) = step {
                for arm in &block.arms {
                    self.check_flow_effects(ctx, &arm.chain, allowed, result);
                }
            }
        }
    }

    fn resolve_step(&self, step: &FlowStep) -> Option<WordEntry> {
        match step {
            FlowStep::Ref(nom_ref) => self.resolver.resolve(nom_ref).ok(),
            _ => None,
        }
    }

    fn verify_constraint(
        &self,
        ctx: &str,
        constraint: &Constraint,
        entry: Option<&WordEntry>,
        result: &mut VerifyResult,
    ) {
        // If we have an entry, delegate to the specialised check
        if let Some(e) = entry {
            self.verify_constraint_against_entry(ctx, constraint, e, result);
            return;
        }
        // Generic literal-vs-literal check
        if let (Expr::Literal(Literal::Number(l)), Expr::Literal(Literal::Number(r))) =
            (&constraint.left, &constraint.right)
        {
            if !apply_op(*l, constraint.op, *r) {
                result.push(
                    ctx,
                    VerifyError::ConstraintFailed {
                        context: ctx.to_owned(),
                        constraint: format!("{l} {:?} {r}", constraint.op),
                    },
                );
            }
        }
    }

    fn verify_constraint_against_entry(
        &self,
        ctx: &str,
        constraint: &Constraint,
        entry: &WordEntry,
        result: &mut VerifyResult,
    ) {
        // left side: metric name (ident), right side: threshold (number)
        let metric = match &constraint.left {
            Expr::Ident(id) => id.name.as_str(),
            _ => return,
        };
        let threshold = match &constraint.right {
            Expr::Literal(Literal::Number(f)) => *f,
            Expr::Literal(Literal::Integer(i)) => *i as f64,
            Expr::Ident(_id) => {
                // could be a unit-suffixed number like `50ms` — skip for now
                return;
            }
            _ => return,
        };
        let value = match metric {
            "security" => entry.security,
            "performance" => entry.performance,
            "reliability" => entry.reliability,
            "readability" => entry.readability,
            "testability" => entry.testability,
            "portability" => entry.portability,
            "composability" => entry.composability,
            "maturity" => entry.maturity,
            "overall_score" => entry.overall_score,
            _ => return,
        };
        if !apply_op(value, constraint.op, threshold) {
            result.push(
                ctx,
                VerifyError::ConstraintFailed {
                    context: ctx.to_owned(),
                    constraint: format!(
                        "{metric}({value:.2}) {} {threshold}",
                        op_str(constraint.op)
                    ),
                },
            );
        }
    }

    fn graph_query_expr_to_flow_chain(expr: &GraphQueryExpr) -> Option<FlowChain> {
        match expr {
            GraphQueryExpr::Ref(reference) => Some(FlowChain {
                steps: vec![FlowStep::Ref(reference.clone())],
            }),
            GraphQueryExpr::Traverse(GraphTraverseExpr {
                source,
                edge,
                target,
                ..
            }) => {
                let mut source_chain = Self::graph_query_expr_to_flow_chain(source)?;
                let target_chain = Self::graph_query_expr_to_flow_chain(target)?;
                source_chain.steps.push(FlowStep::Ref(edge.clone()));
                source_chain.steps.extend(target_chain.steps);
                Some(source_chain)
            }
            GraphQueryExpr::SetOp(GraphSetExpr { op, operands, span }) => {
                if *op == GraphSetOp::Difference || operands.is_empty() {
                    return None;
                }
                let label = match op {
                    GraphSetOp::Union => "union",
                    GraphSetOp::Intersection => "intersect",
                    GraphSetOp::Difference => return None,
                };
                let mut arms = Vec::new();
                for operand in operands {
                    arms.push(BranchArm {
                        condition: BranchCondition::Named,
                        label: Some(label.to_owned()),
                        chain: Self::graph_query_expr_to_flow_chain(operand)?,
                    });
                }
                Some(FlowChain {
                    steps: vec![FlowStep::Branch(BranchBlock { arms, span: *span })],
                })
            }
        }
    }
}

// ── Property-based test generation from contracts (ADOPT-1) ─────────────────

/// A property-based test case generated from a contract's pre/post conditions.
#[derive(Debug, Clone)]
pub struct PropertyTest {
    pub name: String,
    pub description: String,
    pub input_constraints: Vec<String>,
    pub expected_postconditions: Vec<String>,
}

/// Generate property-based test descriptions from contract pre/post conditions.
/// Returns a list of human-readable test case descriptions that should hold.
pub fn generate_contract_tests(contract: &ContractStmt) -> Vec<PropertyTest> {
    let mut tests = Vec::new();

    // 1. For each precondition, generate a boundary test and a violation test
    for (i, pre) in contract.preconditions.iter().enumerate() {
        let pre_str = expr_to_string(pre);

        // Boundary test: exercise the exact boundary of the precondition
        if let Some(boundary) = extract_boundary_test(pre, &contract.postconditions) {
            tests.push(boundary);
        } else {
            // Generic boundary test when we can't parse the specific expression
            tests.push(PropertyTest {
                name: format!("boundary_precondition_{}", i),
                description: format!(
                    "When input is at the boundary of precondition `{}`, all postconditions must hold",
                    pre_str
                ),
                input_constraints: vec![format!("boundary of: {}", pre_str)],
                expected_postconditions: contract
                    .postconditions
                    .iter()
                    .map(|p| expr_to_string(p))
                    .collect(),
            });
        }

        // Violation test: input that violates this precondition should be rejected
        tests.push(PropertyTest {
            name: format!("violation_precondition_{}", i),
            description: format!(
                "When precondition `{}` is violated, the contract should reject the input",
                pre_str
            ),
            input_constraints: vec![format!("violates: {}", pre_str)],
            expected_postconditions: vec!["input rejected".to_owned()],
        });
    }

    // 2. For each postcondition, generate a universal property test
    for (i, post) in contract.postconditions.iter().enumerate() {
        let post_str = expr_to_string(post);
        let pre_strs: Vec<String> = contract.preconditions.iter().map(|p| expr_to_string(p)).collect();

        tests.push(PropertyTest {
            name: format!("postcondition_holds_{}", i),
            description: format!(
                "For any valid input satisfying all preconditions, `{}` must hold",
                post_str
            ),
            input_constraints: pre_strs,
            expected_postconditions: vec![post_str],
        });
    }

    // 3. For each typed input, generate large-input stress tests
    for input in &contract.inputs {
        let input_name = &input.name.name;
        let type_name = input
            .typ
            .as_ref()
            .map(|t| t.name.as_str())
            .unwrap_or("unknown");

        if type_name == "bytes" || type_name == "text" || type_name == "string" {
            tests.push(PropertyTest {
                name: format!("large_input_{}", input_name),
                description: format!(
                    "When {} is large (1000+ elements), all postconditions must still hold",
                    input_name
                ),
                input_constraints: vec![format!("{}.length == 1000", input_name)],
                expected_postconditions: contract
                    .postconditions
                    .iter()
                    .map(|p| expr_to_string(p))
                    .collect(),
            });
        }
    }

    tests
}

/// Try to extract a specific boundary test from a comparison-style precondition.
/// e.g., `data.length > 0` -> boundary at `data.length == 1`
fn extract_boundary_test(pre: &Expr, postconditions: &[Expr]) -> Option<PropertyTest> {
    match pre {
        Expr::BinaryOp(left, op, right) => {
            let left_str = expr_to_string(left);
            let right_str = expr_to_string(right);
            let (boundary_desc, boundary_constraint) = match op {
                BinOp::Gt => {
                    // x > N -> boundary at x == N+1 (or N+1 conceptually)
                    (
                        format!("{} == {} + 1 (minimum valid)", left_str, right_str),
                        format!("{} == {} + 1", left_str, right_str),
                    )
                }
                BinOp::Gte => {
                    // x >= N -> boundary at x == N
                    (
                        format!("{} == {} (minimum valid)", left_str, right_str),
                        format!("{} == {}", left_str, right_str),
                    )
                }
                BinOp::Lt => {
                    // x < N -> boundary at x == N-1
                    (
                        format!("{} == {} - 1 (maximum valid)", left_str, right_str),
                        format!("{} == {} - 1", left_str, right_str),
                    )
                }
                BinOp::Lte => {
                    // x <= N -> boundary at x == N
                    (
                        format!("{} == {} (maximum valid)", left_str, right_str),
                        format!("{} == {}", left_str, right_str),
                    )
                }
                _ => return None,
            };

            Some(PropertyTest {
                name: format!("boundary_{}", sanitize_name(&left_str)),
                description: format!(
                    "When {}, all postconditions must hold",
                    boundary_desc
                ),
                input_constraints: vec![boundary_constraint],
                expected_postconditions: postconditions
                    .iter()
                    .map(|p| expr_to_string(p))
                    .collect(),
            })
        }
        _ => None,
    }
}

/// Convert an Expr to a human-readable string representation.
fn expr_to_string(expr: &Expr) -> String {
    match expr {
        Expr::Ident(id) => id.name.clone(),
        Expr::Literal(lit) => match lit {
            Literal::Number(n) => format!("{n}"),
            Literal::Integer(i) => format!("{i}"),
            Literal::Text(s) => format!("\"{s}\""),
            Literal::Bool(b) => format!("{b}"),
            Literal::None => "none".to_owned(),
        },
        Expr::FieldAccess(obj, field) => {
            format!("{}.{}", expr_to_string(obj), field.name)
        }
        Expr::BinaryOp(left, op, right) => {
            let op_s = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                BinOp::Mod => "%",
                BinOp::And => "&&",
                BinOp::Or => "||",
                BinOp::Gt => ">",
                BinOp::Lt => "<",
                BinOp::Gte => ">=",
                BinOp::Lte => "<=",
                BinOp::Eq => "==",
                BinOp::Neq => "!=",
                BinOp::BitAnd => "&",
                BinOp::BitOr => "|",
            };
            format!("{} {} {}", expr_to_string(left), op_s, expr_to_string(right))
        }
        Expr::Call(call) => {
            let args: Vec<String> = call.args.iter().map(|a| expr_to_string(a)).collect();
            format!("{}({})", call.callee.name, args.join(", "))
        }
        Expr::UnaryOp(op, inner) => {
            let op_s = match op {
                UnaryOp::Not => "!",
                UnaryOp::Neg => "-",
                UnaryOp::Ref => "&",
                UnaryOp::RefMut => "&mut ",
            };
            format!("{}{}", op_s, expr_to_string(inner))
        }
        Expr::MethodCall(obj, method, args) => {
            let args_s: Vec<String> = args.iter().map(|a| expr_to_string(a)).collect();
            format!("{}.{}({})", expr_to_string(obj), method.name, args_s.join(", "))
        }
        Expr::Index(obj, idx) => {
            format!("{}[{}]", expr_to_string(obj), expr_to_string(idx))
        }
        _ => "<expr>".to_owned(),
    }
}

/// Sanitize an expression string into a valid test name component.
fn sanitize_name(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

fn apply_op(left: f64, op: CompareOp, right: f64) -> bool {
    match op {
        CompareOp::Gt => left > right,
        CompareOp::Lt => left < right,
        CompareOp::Gte => left >= right,
        CompareOp::Lte => left <= right,
        CompareOp::Eq => (left - right).abs() < 1e-9,
        CompareOp::Neq => (left - right).abs() >= 1e-9,
    }
}

fn op_str(op: CompareOp) -> &'static str {
    match op {
        CompareOp::Gt => ">",
        CompareOp::Lt => "<",
        CompareOp::Gte => ">=",
        CompareOp::Lte => "<=",
        CompareOp::Eq => "=",
        CompareOp::Neq => "!=",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom_ast::{
        AgentReceiveStmt, AgentScheduleStmt, Classifier, Declaration, GraphQueryExpr,
        GraphQueryStmt, GraphSetExpr, GraphSetOp, Identifier, NomRef, SourceFile, Span, Statement,
        TypedParam,
    };
    use nom_resolver::{Resolver, WordEntry};

    fn span() -> Span {
        Span::new(0, 1, 1, 1)
    }

    fn setup_resolver() -> Resolver {
        let r = Resolver::open_in_memory().unwrap();
        r.upsert(&WordEntry {
            word: "hash".to_owned(),
            variant: Some("argon2".to_owned()),
            input_type: Some("bytes".to_owned()),
            output_type: Some("hash".to_owned()),
            effects: vec!["cpu".to_owned()],
            security: 0.95,
            performance: 0.7,
            reliability: 0.99,
            ..WordEntry::default()
        })
        .unwrap();
        r.upsert(&WordEntry {
            word: "store".to_owned(),
            input_type: Some("hash".to_owned()),
            output_type: Some("unit".to_owned()),
            effects: vec!["database".to_owned()],
            security: 0.8,
            performance: 0.9,
            reliability: 0.99,
            ..WordEntry::default()
        })
        .unwrap();
        r
    }

    #[test]
    fn compatible_chain_no_findings() {
        let resolver = setup_resolver();
        let verifier = Verifier::new(&resolver);

        let chain = FlowChain {
            steps: vec![
                FlowStep::Ref(NomRef {
                    word: Identifier::new("hash", span()),
                    variant: Some(Identifier::new("argon2", span())),
                    span: span(),
                }),
                FlowStep::Ref(NomRef {
                    word: Identifier::new("store", span()),
                    variant: None,
                    span: span(),
                }),
            ],
        };

        let mut result = VerifyResult::default();
        verifier.verify_flow_chain("test_flow", &chain, &mut result);
        assert!(result.ok(), "expected no findings: {:?}", result.findings);
    }

    #[test]
    fn type_mismatch_detected() {
        let resolver = setup_resolver();
        let verifier = Verifier::new(&resolver);

        // hash -> hash (output of hash is "hash", input of hash::argon2 is "bytes") — mismatch
        let chain = FlowChain {
            steps: vec![
                FlowStep::Ref(NomRef {
                    word: Identifier::new("store", span()),
                    variant: None,
                    span: span(),
                }),
                FlowStep::Ref(NomRef {
                    word: Identifier::new("hash", span()),
                    variant: Some(Identifier::new("argon2", span())),
                    span: span(),
                }),
            ],
        };

        let mut result = VerifyResult::default();
        verifier.verify_flow_chain("test_flow", &chain, &mut result);
        assert!(!result.ok());
        assert!(matches!(
            result.findings[0].error,
            VerifyError::TypeMismatch { .. }
        ));
    }

    #[test]
    fn graph_query_chain_is_verified() {
        let resolver = setup_resolver();
        let verifier = Verifier::new(&resolver);

        let source = SourceFile {
            path: None,
            locale: None,
            declarations: vec![Declaration {
                classifier: Classifier::Graph,
                name: Identifier::new("social", span()),
                statements: vec![Statement::GraphQuery(GraphQueryStmt {
                    name: Identifier::new("friends_of", span()),
                    params: vec![TypedParam {
                        name: Identifier::new("user", span()),
                        typ: None,
                        span: span(),
                    }],
                    expr: GraphQueryExpr::SetOp(GraphSetExpr {
                        op: GraphSetOp::Union,
                        operands: vec![
                            GraphQueryExpr::Ref(NomRef {
                                word: Identifier::new("hash", span()),
                                variant: Some(Identifier::new("argon2", span())),
                                span: span(),
                            }),
                            GraphQueryExpr::Ref(NomRef {
                                word: Identifier::new("store", span()),
                                variant: None,
                                span: span(),
                            }),
                        ],
                        span: span(),
                    }),
                    span: span(),
                })],
                span: span(),
            }],
        };

        let result = verifier.verify(&source);
        assert!(
            result.ok(),
            "expected graph query to verify: {:?}",
            result.findings
        );
    }

    #[test]
    fn agent_schedule_chain_mismatch_is_detected() {
        let resolver = setup_resolver();
        let verifier = Verifier::new(&resolver);

        let source = SourceFile {
            path: None,
            locale: None,
            declarations: vec![Declaration {
                classifier: Classifier::Agent,
                name: Identifier::new("monitor", span()),
                statements: vec![
                    Statement::AgentReceive(AgentReceiveStmt {
                        chain: FlowChain {
                            steps: vec![FlowStep::Ref(NomRef {
                                word: Identifier::new("hash", span()),
                                variant: Some(Identifier::new("argon2", span())),
                                span: span(),
                            })],
                        },
                        span: span(),
                    }),
                    Statement::AgentSchedule(AgentScheduleStmt {
                        interval: "5m".to_owned(),
                        action: FlowChain {
                            steps: vec![
                                FlowStep::Ref(NomRef {
                                    word: Identifier::new("store", span()),
                                    variant: None,
                                    span: span(),
                                }),
                                FlowStep::Ref(NomRef {
                                    word: Identifier::new("hash", span()),
                                    variant: Some(Identifier::new("argon2", span())),
                                    span: span(),
                                }),
                            ],
                        },
                        span: span(),
                    }),
                ],
                span: span(),
            }],
        };

        let result = verifier.verify(&source);
        assert!(!result.ok());
        assert!(
            result
                .findings
                .iter()
                .any(|finding| matches!(finding.error, VerifyError::TypeMismatch { .. }))
        );
    }

    // ── Property test generation tests ──────────────────────────────────────

    fn make_hash_contract() -> ContractStmt {
        ContractStmt {
            inputs: vec![TypedParam {
                name: Identifier::new("data", span()),
                typ: Some(Identifier::new("bytes", span())),
                span: span(),
            }],
            outputs: vec![TypedParam {
                name: Identifier::new("hash", span()),
                typ: Some(Identifier::new("bytes", span())),
                span: span(),
            }],
            effects: vec![],
            preconditions: vec![
                // pre: data.length > 0
                Expr::BinaryOp(
                    Box::new(Expr::FieldAccess(
                        Box::new(Expr::Ident(Identifier::new("data", span()))),
                        Identifier::new("length", span()),
                    )),
                    BinOp::Gt,
                    Box::new(Expr::Literal(Literal::Integer(0))),
                ),
            ],
            postconditions: vec![
                // post: hash.length == 32
                Expr::BinaryOp(
                    Box::new(Expr::FieldAccess(
                        Box::new(Expr::Ident(Identifier::new("hash", span()))),
                        Identifier::new("length", span()),
                    )),
                    BinOp::Eq,
                    Box::new(Expr::Literal(Literal::Integer(32))),
                ),
            ],
            span: span(),
        }
    }

    #[test]
    fn generates_property_tests_from_contract() {
        let contract = make_hash_contract();
        let tests = generate_contract_tests(&contract);
        assert!(!tests.is_empty(), "should generate at least one test");
        assert!(
            tests.iter().any(|t| t.name.contains("boundary")),
            "should contain a boundary test, got: {:?}",
            tests.iter().map(|t| &t.name).collect::<Vec<_>>()
        );
        assert!(
            tests.iter().any(|t| t.name.contains("postcondition")),
            "should contain a postcondition test, got: {:?}",
            tests.iter().map(|t| &t.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn boundary_test_extracts_comparison_details() {
        let contract = make_hash_contract();
        let tests = generate_contract_tests(&contract);
        let boundary = tests.iter().find(|t| t.name.contains("boundary")).unwrap();
        // The boundary test for `data.length > 0` should reference the minimum valid
        assert!(
            boundary.description.contains("minimum valid"),
            "boundary description should mention minimum valid, got: {}",
            boundary.description
        );
        // And expect the postcondition to hold
        assert!(
            !boundary.expected_postconditions.is_empty(),
            "boundary test should have expected postconditions"
        );
    }

    #[test]
    fn violation_test_generated_for_precondition() {
        let contract = make_hash_contract();
        let tests = generate_contract_tests(&contract);
        let violation = tests.iter().find(|t| t.name.contains("violation")).unwrap();
        assert!(
            violation.description.contains("violated"),
            "violation test should describe precondition violation, got: {}",
            violation.description
        );
        assert!(
            violation.expected_postconditions.iter().any(|p| p.contains("rejected")),
            "violation test should expect rejection"
        );
    }

    #[test]
    fn large_input_test_for_bytes_type() {
        let contract = make_hash_contract();
        let tests = generate_contract_tests(&contract);
        let large = tests.iter().find(|t| t.name.contains("large_input"));
        assert!(
            large.is_some(),
            "should generate a large input test for bytes-typed input"
        );
        let large = large.unwrap();
        assert!(
            large.input_constraints.iter().any(|c| c.contains("1000")),
            "large input test should reference size 1000"
        );
    }

    #[test]
    fn postcondition_test_includes_all_preconditions() {
        let contract = make_hash_contract();
        let tests = generate_contract_tests(&contract);
        let post_test = tests.iter().find(|t| t.name.contains("postcondition_holds")).unwrap();
        // The postcondition test should list all preconditions as input constraints
        assert!(
            !post_test.input_constraints.is_empty(),
            "postcondition test should have input constraints from preconditions"
        );
        assert!(
            post_test.input_constraints.iter().any(|c| c.contains("data.length")),
            "input constraints should reference data.length"
        );
    }

    #[test]
    fn empty_contract_produces_no_tests() {
        let contract = ContractStmt {
            inputs: vec![],
            outputs: vec![],
            effects: vec![],
            preconditions: vec![],
            postconditions: vec![],
            span: span(),
        };
        let tests = generate_contract_tests(&contract);
        assert!(tests.is_empty(), "empty contract should produce no tests");
    }

    #[test]
    fn expr_to_string_renders_field_access() {
        let expr = Expr::FieldAccess(
            Box::new(Expr::Ident(Identifier::new("data", span()))),
            Identifier::new("length", span()),
        );
        assert_eq!(expr_to_string(&expr), "data.length");
    }

    #[test]
    fn expr_to_string_renders_binary_op() {
        let expr = Expr::BinaryOp(
            Box::new(Expr::Ident(Identifier::new("x", span()))),
            BinOp::Gt,
            Box::new(Expr::Literal(Literal::Integer(5))),
        );
        assert_eq!(expr_to_string(&expr), "x > 5");
    }
}
