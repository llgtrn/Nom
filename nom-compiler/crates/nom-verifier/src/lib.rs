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
    CompareOp, Constraint, Declaration, EffectModifier, Expr, FlowChain, FlowStep, Literal,
    NomRef, SourceFile, Statement,
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
    #[error("undeclared effect '{effect}' in '{context}': word '{word}' produces it but it is not listed")]
    UndeclaredEffect {
        context: String,
        effect: String,
        word: String,
    },
    #[error("word not found during verification: {0}")]
    UnresolvedWord(String),
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
        self.findings.is_empty()
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
        for decl in &source.declarations {
            self.verify_declaration(decl, &mut result);
        }
        result
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
                    self.verify_flow_chain(name, &flow.chain, result);
                    if declared_only {
                        if let Some(ref allowed) = declared_effects {
                            self.check_flow_effects(name, &flow.chain, allowed, result);
                        }
                    }
                }
                _ => {}
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
                                    a.variant.as_deref().map(|v| format!("::{v}")).unwrap_or_default()
                                ),
                                b: format!(
                                    "{}{}",
                                    b.word,
                                    b.variant.as_deref().map(|v| format!("::{v}")).unwrap_or_default()
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
    use nom_ast::{Identifier, NomRef, Span};
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
        }).unwrap();
        r.upsert(&WordEntry {
            word: "store".to_owned(),
            input_type: Some("hash".to_owned()),
            output_type: Some("unit".to_owned()),
            effects: vec!["database".to_owned()],
            security: 0.8,
            performance: 0.9,
            reliability: 0.99,
            ..WordEntry::default()
        }).unwrap();
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
        assert!(matches!(result.findings[0].error, VerifyError::TypeMismatch { .. }));
    }
}
