//! Phase 4 B3 — pure hash-rewrite pass.
//!
//! Given a resolved [`ResolutionTable`] (`name -> 64-char id`) and a
//! parsed [`SourceFile`], [`rewrite_with_hashes`] returns a brand new
//! `SourceFile` where every identifier expression or call site whose
//! `name` field appears in the table is rewritten to the hash-pinned
//! form `#<hash>@<name>`.
//!
//! The transform is pure — the input AST is not mutated.
//!
//! Why touch the identifier *name* instead of adding a side field? The
//! single consumer today is `nom fmt`, which already stringifies
//! identifiers verbatim. Stamping the pin into the name keeps the
//! downstream pipeline a no-op and matches how stored `body_nom` is
//! meant to read. Task C revisits whether a richer representation is
//! warranted; for now a minimal, reversible prefix is sufficient.

use nom_ast::{
    BlockStmt, CallExpr, Declaration, Expr, IfExpr, Identifier, MatchArm, MatchExpr, NomRef,
    SourceFile, Statement, Block, FlowStep, FlowChain, BranchBlock, BranchArm,
    LetStmt, AssignStmt, ForStmt, WhileStmt, NeedStmt, RequireStmt, Constraint,
    FnDef, DescribeStmt, ContractStmt, ImplBlock, TraitDef, AgentReceiveStmt,
    AgentScheduleStmt, FlowStmt, GraphQueryStmt, GraphQueryExpr, GraphTraverseExpr,
    GraphSetExpr, GraphConstraintStmt, TestGivenStmt, TestWhenStmt, TestThenStmt,
    TestAndStmt,
};

use crate::v2::ResolutionTable;

/// Produce a new `SourceFile` with every reference to a resolved name
/// rewritten to `#<hash>@<name>`. Names not present in `table` pass
/// through untouched.
pub fn rewrite_with_hashes(src: &SourceFile, table: &ResolutionTable) -> SourceFile {
    let mut out = src.clone();
    for decl in &mut out.declarations {
        rewrite_declaration(decl, table);
    }
    out
}

fn rewrite_declaration(decl: &mut Declaration, t: &ResolutionTable) {
    for stmt in &mut decl.statements {
        rewrite_statement(stmt, t);
    }
}

fn rewrite_statement(stmt: &mut Statement, t: &ResolutionTable) {
    match stmt {
        Statement::Need(NeedStmt { reference, constraint, .. }) => {
            rewrite_nom_ref(reference, t);
            if let Some(c) = constraint {
                rewrite_constraint(c, t);
            }
        }
        Statement::Require(RequireStmt { constraint, .. }) => rewrite_constraint(constraint, t),
        Statement::Effects(_) => {}
        Statement::Flow(FlowStmt { chain, .. }) => rewrite_flow_chain(chain, t),
        Statement::Describe(DescribeStmt { .. }) => {}
        Statement::Contract(ContractStmt { preconditions, postconditions, .. }) => {
            for e in preconditions {
                rewrite_expr(e, t);
            }
            for e in postconditions {
                rewrite_expr(e, t);
            }
        }
        Statement::Implement(_) => {}
        Statement::Given(TestGivenStmt { config, .. })
        | Statement::When(TestWhenStmt { config, .. }) => {
            for (_, e) in config {
                rewrite_expr(e, t);
            }
        }
        Statement::Then(TestThenStmt { assertion, .. })
        | Statement::And(TestAndStmt { assertion, .. }) => rewrite_expr(assertion, t),
        Statement::GraphNode(_) | Statement::GraphEdge(_) => {}
        Statement::GraphQuery(GraphQueryStmt { expr, .. }) => rewrite_graph_query(expr, t),
        Statement::GraphConstraint(GraphConstraintStmt { expr, .. }) => rewrite_expr(expr, t),
        Statement::AgentCapability(_)
        | Statement::AgentSupervise(_)
        | Statement::AgentState(_) => {}
        Statement::AgentReceive(AgentReceiveStmt { chain, .. }) => rewrite_flow_chain(chain, t),
        Statement::AgentSchedule(AgentScheduleStmt { action, .. }) => {
            rewrite_flow_chain(action, t);
        }
        Statement::Let(LetStmt { value, .. }) => rewrite_expr(value, t),
        Statement::Assign(AssignStmt { target, value, .. }) => {
            rewrite_expr(target, t);
            rewrite_expr(value, t);
        }
        Statement::If(if_expr) => rewrite_if(if_expr, t),
        Statement::For(ForStmt { iterable, body, .. }) => {
            rewrite_expr(iterable, t);
            rewrite_block(body, t);
        }
        Statement::While(WhileStmt { condition, body, .. }) => {
            rewrite_expr(condition, t);
            rewrite_block(body, t);
        }
        Statement::Match(m) => rewrite_match(m, t),
        Statement::Return(Some(e)) => rewrite_expr(e, t),
        Statement::Return(None) => {}
        Statement::FnDef(FnDef { body, .. }) => rewrite_block(body, t),
        Statement::StructDef(_) | Statement::EnumDef(_) => {}
        Statement::ExprStmt(e) => rewrite_expr(e, t),
        Statement::TraitDef(TraitDef { methods, .. }) => {
            for m in methods {
                rewrite_block(&mut m.body, t);
            }
        }
        Statement::ImplBlock(ImplBlock { methods, .. }) => {
            for m in methods {
                rewrite_block(&mut m.body, t);
            }
        }
        Statement::Use(_) | Statement::Mod(_) => {}
    }
}

fn rewrite_block(b: &mut Block, t: &ResolutionTable) {
    for s in &mut b.stmts {
        rewrite_block_stmt(s, t);
    }
}

fn rewrite_block_stmt(s: &mut BlockStmt, t: &ResolutionTable) {
    match s {
        BlockStmt::Let(LetStmt { value, .. }) => rewrite_expr(value, t),
        BlockStmt::Assign(AssignStmt { target, value, .. }) => {
            rewrite_expr(target, t);
            rewrite_expr(value, t);
        }
        BlockStmt::Expr(e) => rewrite_expr(e, t),
        BlockStmt::If(i) => rewrite_if(i, t),
        BlockStmt::For(ForStmt { iterable, body, .. }) => {
            rewrite_expr(iterable, t);
            rewrite_block(body, t);
        }
        BlockStmt::While(WhileStmt { condition, body, .. }) => {
            rewrite_expr(condition, t);
            rewrite_block(body, t);
        }
        BlockStmt::Match(m) => rewrite_match(m, t),
        BlockStmt::Return(Some(e)) => rewrite_expr(e, t),
        BlockStmt::Return(None) | BlockStmt::Break | BlockStmt::Continue => {}
    }
}

fn rewrite_if(i: &mut IfExpr, t: &ResolutionTable) {
    rewrite_expr(&mut i.condition, t);
    rewrite_block(&mut i.then_body, t);
    for (cond, body) in &mut i.else_ifs {
        rewrite_expr(cond, t);
        rewrite_block(body, t);
    }
    if let Some(eb) = &mut i.else_body {
        rewrite_block(eb, t);
    }
}

fn rewrite_match(m: &mut MatchExpr, t: &ResolutionTable) {
    rewrite_expr(&mut m.subject, t);
    for MatchArm { body, .. } in &mut m.arms {
        rewrite_block(body, t);
    }
}

fn rewrite_constraint(c: &mut Constraint, t: &ResolutionTable) {
    rewrite_expr(&mut c.left, t);
    rewrite_expr(&mut c.right, t);
}

fn rewrite_flow_chain(chain: &mut FlowChain, t: &ResolutionTable) {
    for step in &mut chain.steps {
        match step {
            FlowStep::Ref(r) => rewrite_nom_ref(r, t),
            FlowStep::Literal(_) => {}
            FlowStep::Branch(BranchBlock { arms, .. }) => {
                for BranchArm { chain: ch, .. } in arms {
                    rewrite_flow_chain(ch, t);
                }
            }
            FlowStep::Call(c) => rewrite_call(c, t),
        }
    }
}

fn rewrite_graph_query(q: &mut GraphQueryExpr, t: &ResolutionTable) {
    match q {
        GraphQueryExpr::Ref(r) => rewrite_nom_ref(r, t),
        GraphQueryExpr::Traverse(GraphTraverseExpr { source, edge, target, .. }) => {
            rewrite_graph_query(source, t);
            rewrite_nom_ref(edge, t);
            rewrite_graph_query(target, t);
        }
        GraphQueryExpr::SetOp(GraphSetExpr { operands, .. }) => {
            for op in operands {
                rewrite_graph_query(op, t);
            }
        }
    }
}

fn rewrite_nom_ref(r: &mut NomRef, t: &ResolutionTable) {
    pin_ident(&mut r.word, t);
}

fn rewrite_call(c: &mut CallExpr, t: &ResolutionTable) {
    pin_ident(&mut c.callee, t);
    for a in &mut c.args {
        rewrite_expr(a, t);
    }
}

fn rewrite_expr(e: &mut Expr, t: &ResolutionTable) {
    match e {
        Expr::Ident(id) => pin_ident(id, t),
        Expr::Literal(_) => {}
        Expr::FieldAccess(inner, _) => rewrite_expr(inner, t),
        Expr::BinaryOp(l, _, r) => {
            rewrite_expr(l, t);
            rewrite_expr(r, t);
        }
        Expr::Call(c) => rewrite_call(c, t),
        Expr::UnaryOp(_, inner) => rewrite_expr(inner, t),
        Expr::Index(a, b) | Expr::Range(a, b) => {
            rewrite_expr(a, t);
            rewrite_expr(b, t);
        }
        Expr::MethodCall(recv, _, args) => {
            rewrite_expr(recv, t);
            for a in args {
                rewrite_expr(a, t);
            }
        }
        Expr::IfExpr(i) => rewrite_if(i, t),
        Expr::MatchExpr(m) => rewrite_match(m, t),
        Expr::Block(b) => rewrite_block(b, t),
        Expr::Closure(_, body) => rewrite_expr(body, t),
        Expr::Array(items) | Expr::TupleExpr(items) => {
            for i in items {
                rewrite_expr(i, t);
            }
        }
        Expr::Await(inner) | Expr::Cast(inner, _) | Expr::Try(inner) => {
            rewrite_expr(inner, t)
        }
        Expr::StructInit { fields, .. } => {
            for (_, v) in fields {
                rewrite_expr(v, t);
            }
        }
    }
}

/// Rewrite a single [`Identifier`] if its name is in the table. Idempotent
/// (already-pinned names, i.e. ones starting with `#`, are left alone).
fn pin_ident(id: &mut Identifier, t: &ResolutionTable) {
    if id.name.starts_with('#') {
        return;
    }
    if let Some(hash) = t.get(&id.name) {
        id.name = format!("#{hash}@{}", id.name);
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::resolve_use_statements;
    use nom_dict::NomDict;
    use nom_parser::parse_source;
    use nom_types::{Contract, Entry, EntryKind, EntryStatus};

    fn make_entry(id: &str, word: &str) -> Entry {
        Entry {
            id: id.into(),
            word: word.into(),
            variant: None,
            kind: EntryKind::Function,
            language: "nom".into(),
            describe: None,
            concept: None,
            body: None,
            body_nom: None,
            contract: Contract::default(),
            status: EntryStatus::Complete,
            translation_score: None,
            is_canonical: true,
            deprecated_by: None,
            created_at: "2026-04-12T00:00:00Z".into(),
            updated_at: None,
        }
    }

    #[test]
    fn rewrite_pins_call_site_in_body() {
        // Seed a dict with entry A named `a`.
        let a_id = "a".repeat(64);
        let dict = NomDict::open_in_memory().unwrap();
        dict.upsert_entry(&make_entry(&a_id, "a")).unwrap();

        // Entry B's body calls `a(1)`.
        let src = "system main\n  use a\n  let x = a(1)\n";
        let sf = parse_source(src).unwrap();
        let table = resolve_use_statements(&sf, &dict).unwrap();

        let rewritten = rewrite_with_hashes(&sf, &table);
        // Dump every ident/call name and confirm the pinned prefix is present
        // and the original bare call is gone.
        let rendered = dump_idents(&rewritten);
        assert!(
            rendered.iter().any(|n| n == &format!("#{a_id}@a")),
            "expected pinned ident, got {:?}",
            rendered
        );
        // No remaining bare `a` identifier referring to the function.
        // (The `x` binding name on the LHS of `let` is not an ident expr.)
        assert!(
            !rendered.iter().any(|n| n == "a"),
            "bare `a` should have been rewritten: {:?}",
            rendered
        );
    }

    #[test]
    fn rewrite_is_pure_and_idempotent() {
        let a_id = "b".repeat(64);
        let dict = NomDict::open_in_memory().unwrap();
        dict.upsert_entry(&make_entry(&a_id, "a")).unwrap();
        let src = "system main\n  use a\n  let x = a(1)\n";
        let sf = parse_source(src).unwrap();
        let table = resolve_use_statements(&sf, &dict).unwrap();

        // Pure: input not mutated after rewrite.
        let before_names = dump_idents(&sf);
        let r1 = rewrite_with_hashes(&sf, &table);
        let after_names = dump_idents(&sf);
        assert_eq!(before_names, after_names, "input AST must not be mutated");

        // Idempotent: running rewrite on already-rewritten AST leaves it alone.
        let r2 = rewrite_with_hashes(&r1, &table);
        assert_eq!(dump_idents(&r1), dump_idents(&r2));
    }

    /// Collect every identifier name reachable from a SourceFile in
    /// traversal order. Used by tests to make assertions about what got
    /// rewritten.
    fn dump_idents(sf: &SourceFile) -> Vec<String> {
        let mut out = Vec::new();
        for d in &sf.declarations {
            for s in &d.statements {
                visit_stmt(s, &mut out);
            }
        }
        out
    }

    fn visit_stmt(s: &Statement, out: &mut Vec<String>) {
        match s {
            Statement::Let(LetStmt { value, .. }) => visit_expr(value, out),
            Statement::ExprStmt(e) => visit_expr(e, out),
            Statement::Flow(FlowStmt { chain, .. }) => {
                for step in &chain.steps {
                    if let FlowStep::Ref(r) = step {
                        out.push(r.word.name.clone());
                    }
                }
            }
            _ => {}
        }
    }

    fn visit_expr(e: &Expr, out: &mut Vec<String>) {
        match e {
            Expr::Ident(id) => out.push(id.name.clone()),
            Expr::Call(c) => {
                out.push(c.callee.name.clone());
                for a in &c.args {
                    visit_expr(a, out);
                }
            }
            Expr::BinaryOp(l, _, r) => {
                visit_expr(l, out);
                visit_expr(r, out);
            }
            _ => {}
        }
    }
}
