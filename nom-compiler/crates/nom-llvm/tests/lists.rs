//! Integration tests for the LLVM `list[T]` lowering (Task D).
//!
//! The tests compile small Nom programs end-to-end and inspect the emitted
//! LLVM IR. They do not run the bitcode via `lli` because the bundled
//! Windows LLVM distribution does not ship `lli.exe`; IR-level checks are
//! portable across platforms and catch the relevant lowering shape.
//!
//! AST is constructed directly — no dependency on the deleted nom-parser crate.

use nom_ast::{
    AssignStmt, Block, BlockStmt, Expr, FnDef, ForStmt, Identifier, LetStmt, Literal, Span,
    Statement, TypeExpr,
};
use nom_llvm::compile;
use nom_planner::{CompositionPlan, ConcurrencyStrategy, FlowPlan, MemoryStrategy};

fn ident(name: &str) -> Identifier {
    Identifier::new(name, Span::default())
}

fn named_type(name: &str) -> TypeExpr {
    TypeExpr::Named(ident(name))
}

fn lst_t() -> TypeExpr {
    TypeExpr::Generic(
        Identifier::new("list", Span::default()),
        vec![TypeExpr::Named(Identifier::new("integer", Span::default()))],
    )
}

fn int_lit(n: i64) -> Expr {
    Expr::Literal(Literal::Integer(n))
}

fn ident_expr(name: &str) -> Expr {
    Expr::Ident(Identifier::new(name, Span::default()))
}

fn push_stmt(list: &str, v: i64) -> BlockStmt {
    BlockStmt::Expr(Expr::MethodCall(
        Box::new(ident_expr(list)),
        Identifier::new("push", Span::default()),
        vec![int_lit(v)],
    ))
}

fn make_plan(name: &str, stmts: Vec<Statement>) -> CompositionPlan {
    CompositionPlan {
        source_path: Some(format!("{name}.nom")),
        flows: vec![FlowPlan {
            name: name.into(),
            classifier: "nom".into(),
            agent: None,
            graph: None,
            nodes: vec![],
            edges: vec![],
            branches: vec![],
            memory_strategy: MemoryStrategy::Stack,
            concurrency_strategy: ConcurrencyStrategy::Sequential,
            qualifier: "once".to_owned(),
            on_fail: "abort".to_owned(),
            effect_summary: vec![],
            imperative_stmts: stmts,
        }],
        nomiz: "{}".into(),
    }
}

#[test]
fn test_list_empty_and_length() {
    // fn main() -> integer {
    //   let xs: list[integer] = []
    //   return xs.length
    // }
    let fn_def = FnDef {
        name: ident("main"),
        params: vec![],
        return_type: Some(named_type("integer")),
        body: Block {
            stmts: vec![
                BlockStmt::Let(LetStmt {
                    name: ident("xs"),
                    mutable: false,
                    type_ann: Some(lst_t()),
                    value: Expr::Array(vec![]),
                    span: Span::default(),
                }),
                BlockStmt::Return(Some(Expr::FieldAccess(
                    Box::new(ident_expr("xs")),
                    ident("length"),
                ))),
            ],
            span: Span::default(),
        },
        is_async: false,
        is_pub: false,
        span: Span::default(),
    };
    let plan = make_plan("listlen", vec![Statement::FnDef(fn_def)]);
    let out = compile(&plan).expect("compile");
    assert!(
        out.ir_text.contains("%NomList"),
        "IR should reference %NomList, got:\n{}",
        out.ir_text
    );
    assert!(
        out.ir_text.contains("nom_list_new"),
        "IR should call nom_list_new for empty list literal, got:\n{}",
        out.ir_text
    );
    assert!(
        out.ir_text.contains("nom_list_len"),
        "IR should call nom_list_len for .length, got:\n{}",
        out.ir_text
    );
}

#[test]
fn test_list_push_and_get() {
    // Build the AST directly to sidestep parser quirks with bare assignment
    // vs method-call statements at the block level.
    let fn_def = FnDef {
        name: Identifier::new("main", Span::default()),
        params: vec![],
        return_type: Some(TypeExpr::Named(Identifier::new("integer", Span::default()))),
        body: Block {
            stmts: vec![
                BlockStmt::Let(LetStmt {
                    name: Identifier::new("xs", Span::default()),
                    mutable: false,
                    type_ann: Some(lst_t()),
                    value: Expr::Array(vec![]),
                    span: Span::default(),
                }),
                push_stmt("xs", 1),
                push_stmt("xs", 2),
                push_stmt("xs", 3),
                BlockStmt::Return(Some(Expr::Index(
                    Box::new(ident_expr("xs")),
                    Box::new(int_lit(1)),
                ))),
            ],
            span: Span::default(),
        },
        is_async: false,
        is_pub: false,
        span: Span::default(),
    };

    let plan = CompositionPlan {
        source_path: Some("listpush.nom".into()),
        flows: vec![FlowPlan {
            name: "listpush".into(),
            classifier: "nom".into(),
            agent: None,
            graph: None,
            nodes: vec![],
            edges: vec![],
            branches: vec![],
            memory_strategy: MemoryStrategy::Stack,
            concurrency_strategy: ConcurrencyStrategy::Sequential,
            qualifier: "once".to_owned(),
            on_fail: "abort".to_owned(),
            effect_summary: vec![],
            imperative_stmts: vec![nom_ast::Statement::FnDef(fn_def)],
        }],
        nomiz: "{}".into(),
    };

    let out = compile(&plan).expect("compile");
    assert!(
        out.ir_text.contains("nom_list_new"),
        "IR should call nom_list_new, got:\n{}",
        out.ir_text
    );
    // Expect three pushes for the three .push calls.
    let push_count = out.ir_text.matches("call void @nom_list_push").count();
    assert!(
        push_count >= 3,
        "IR should contain >=3 nom_list_push calls, found {} in:\n{}",
        push_count,
        out.ir_text
    );
    assert!(
        out.ir_text.contains("nom_list_get"),
        "IR should call nom_list_get for list indexing, got:\n{}",
        out.ir_text
    );
}

#[test]
fn test_list_sum_via_for() {
    // fn main() -> integer {
    //   let xs: list[integer] = [10, 20, 30]
    //   let s: integer = 0
    //   for x in xs { s = s + x }
    //   return s
    // }
    let fn_def = FnDef {
        name: ident("main"),
        params: vec![],
        return_type: Some(named_type("integer")),
        body: Block {
            stmts: vec![
                BlockStmt::Let(LetStmt {
                    name: ident("xs"),
                    mutable: false,
                    type_ann: Some(lst_t()),
                    value: Expr::Array(vec![int_lit(10), int_lit(20), int_lit(30)]),
                    span: Span::default(),
                }),
                BlockStmt::Let(LetStmt {
                    name: ident("s"),
                    mutable: true,
                    type_ann: Some(named_type("integer")),
                    value: int_lit(0),
                    span: Span::default(),
                }),
                BlockStmt::For(ForStmt {
                    binding: ident("x"),
                    iterable: ident_expr("xs"),
                    body: Block {
                        stmts: vec![BlockStmt::Assign(AssignStmt {
                            target: ident_expr("s"),
                            value: Expr::BinaryOp(
                                Box::new(ident_expr("s")),
                                nom_ast::BinOp::Add,
                                Box::new(ident_expr("x")),
                            ),
                            span: Span::default(),
                        })],
                        span: Span::default(),
                    },
                    span: Span::default(),
                }),
                BlockStmt::Return(Some(ident_expr("s"))),
            ],
            span: Span::default(),
        },
        is_async: false,
        is_pub: false,
        span: Span::default(),
    };
    let plan = make_plan("listsum", vec![Statement::FnDef(fn_def)]);
    let out = compile(&plan).expect("compile");
    assert!(
        out.ir_text.contains("nom_list_new"),
        "IR should call nom_list_new, got:\n{}",
        out.ir_text
    );
    let push_count = out.ir_text.matches("call void @nom_list_push").count();
    assert!(
        push_count >= 3,
        "IR should contain 3 nom_list_push calls for [10,20,30], found {} in:\n{}",
        push_count,
        out.ir_text
    );
    assert!(
        out.ir_text.contains("nom_list_len"),
        "for-in on list should call nom_list_len, got:\n{}",
        out.ir_text
    );
    assert!(
        out.ir_text.contains("nom_list_get"),
        "for-in on list should call nom_list_get, got:\n{}",
        out.ir_text
    );
    assert!(
        out.ir_text.contains("for_list_cond"),
        "IR should contain the list-for loop blocks, got:\n{}",
        out.ir_text
    );
}
