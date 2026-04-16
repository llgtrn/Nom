//! Integration tests for tuple expressions, tuple field access, and
//! tuple-typed function returns (Task B).
//!
//! These tests compile small Nom programs end-to-end and inspect the
//! emitted LLVM IR to confirm the expected lowering shape (insertvalue /
//! extractvalue / anonymous struct returns). They do not run the bitcode
//! via `lli`; verifying at the IR level keeps tests portable across
//! Windows LLVM distributions.
//!
//! AST is constructed directly — no dependency on the deleted nom-parser crate.

use nom_ast::{
    BinOp, Block, BlockStmt, CallExpr, Expr, FnDef, Identifier, LetStmt, Literal, Span, Statement,
    TypeExpr,
};
use nom_llvm::compile;
use nom_planner::{CompositionPlan, ConcurrencyStrategy, FlowPlan, MemoryStrategy};

fn ident(name: &str) -> Identifier {
    Identifier::new(name, Span::default())
}

fn named_type(name: &str) -> TypeExpr {
    TypeExpr::Named(ident(name))
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
fn test_tuple_construct() {
    // fn main() -> integer {
    //   let a = 3
    //   let b = 7
    //   let p = (a, b)
    //   return p.0 + p.1
    // }
    let fn_def = FnDef {
        name: ident("main"),
        params: vec![],
        return_type: Some(named_type("integer")),
        body: Block {
            stmts: vec![
                BlockStmt::Let(LetStmt {
                    name: ident("a"),
                    mutable: false,
                    type_ann: None,
                    value: Expr::Literal(Literal::Integer(3)),
                    span: Span::default(),
                }),
                BlockStmt::Let(LetStmt {
                    name: ident("b"),
                    mutable: false,
                    type_ann: None,
                    value: Expr::Literal(Literal::Integer(7)),
                    span: Span::default(),
                }),
                BlockStmt::Let(LetStmt {
                    name: ident("p"),
                    mutable: false,
                    type_ann: None,
                    value: Expr::TupleExpr(vec![Expr::Ident(ident("a")), Expr::Ident(ident("b"))]),
                    span: Span::default(),
                }),
                BlockStmt::Return(Some(Expr::BinaryOp(
                    Box::new(Expr::FieldAccess(
                        Box::new(Expr::Ident(ident("p"))),
                        ident("0"),
                    )),
                    BinOp::Add,
                    Box::new(Expr::FieldAccess(
                        Box::new(Expr::Ident(ident("p"))),
                        ident("1"),
                    )),
                ))),
            ],
            span: Span::default(),
        },
        is_async: false,
        is_pub: false,
        span: Span::default(),
    };
    let plan = make_plan("tupctor", vec![Statement::FnDef(fn_def)]);
    let out = compile(&plan).expect("compile");
    assert!(
        out.ir_text.contains("insertvalue"),
        "IR should contain insertvalue for tuple construction, got:\n{}",
        out.ir_text
    );
    assert!(
        out.ir_text.contains("extractvalue"),
        "IR should contain extractvalue for tuple field access, got:\n{}",
        out.ir_text
    );
    assert!(
        out.ir_text.contains("ret i64"),
        "IR should return i64 sum, got:\n{}",
        out.ir_text
    );
}

#[test]
fn test_tuple_return() {
    // fn pair() -> (integer, integer) { return (1, 2) }
    // fn main() -> integer { let p = pair(); return p.0 }
    let pair_fn = FnDef {
        name: ident("pair"),
        params: vec![],
        return_type: Some(TypeExpr::Tuple(vec![
            named_type("integer"),
            named_type("integer"),
        ])),
        body: Block {
            stmts: vec![BlockStmt::Return(Some(Expr::TupleExpr(vec![
                Expr::Literal(Literal::Integer(1)),
                Expr::Literal(Literal::Integer(2)),
            ])))],
            span: Span::default(),
        },
        is_async: false,
        is_pub: false,
        span: Span::default(),
    };
    let main_fn = FnDef {
        name: ident("main"),
        params: vec![],
        return_type: Some(named_type("integer")),
        body: Block {
            stmts: vec![
                BlockStmt::Let(LetStmt {
                    name: ident("p"),
                    mutable: false,
                    type_ann: None,
                    value: Expr::Call(CallExpr {
                        callee: ident("pair"),
                        args: vec![],
                        span: Span::default(),
                    }),
                    span: Span::default(),
                }),
                BlockStmt::Return(Some(Expr::FieldAccess(
                    Box::new(Expr::Ident(ident("p"))),
                    ident("0"),
                ))),
            ],
            span: Span::default(),
        },
        is_async: false,
        is_pub: false,
        span: Span::default(),
    };
    let plan = make_plan(
        "tupret",
        vec![Statement::FnDef(pair_fn), Statement::FnDef(main_fn)],
    );
    let out = compile(&plan).expect("compile");
    // Function `pair` should return an anonymous struct `{ i64, i64 }`.
    assert!(
        out.ir_text.contains("{ i64, i64 }"),
        "IR should contain anonymous struct type {{ i64, i64 }} for pair() return, got:\n{}",
        out.ir_text
    );
    // The returned value must flow as an aggregate ret.
    assert!(
        out.ir_text.contains("ret { i64, i64 }"),
        "IR should contain ret {{ i64, i64 }} for pair() body, got:\n{}",
        out.ir_text
    );
    assert!(
        out.ir_text.contains("extractvalue"),
        "IR should contain extractvalue for p.0, got:\n{}",
        out.ir_text
    );
}

#[test]
fn test_heterogeneous_tuple() {
    // fn main() -> integer {
    //   let n = 5
    //   let p = ("hi", n)
    //   return p.1
    // }
    let fn_def = FnDef {
        name: ident("main"),
        params: vec![],
        return_type: Some(named_type("integer")),
        body: Block {
            stmts: vec![
                BlockStmt::Let(LetStmt {
                    name: ident("n"),
                    mutable: false,
                    type_ann: None,
                    value: Expr::Literal(Literal::Integer(5)),
                    span: Span::default(),
                }),
                BlockStmt::Let(LetStmt {
                    name: ident("p"),
                    mutable: false,
                    type_ann: None,
                    value: Expr::TupleExpr(vec![
                        Expr::Literal(Literal::Text("hi".into())),
                        Expr::Ident(ident("n")),
                    ]),
                    span: Span::default(),
                }),
                BlockStmt::Return(Some(Expr::FieldAccess(
                    Box::new(Expr::Ident(ident("p"))),
                    ident("1"),
                ))),
            ],
            span: Span::default(),
        },
        is_async: false,
        is_pub: false,
        span: Span::default(),
    };
    let plan = make_plan("tuphet", vec![Statement::FnDef(fn_def)]);
    let out = compile(&plan).expect("compile");
    // Tuple is (NomString, i64) — struct must contain NomString and i64.
    // The anonymous tuple type in IR should look like `{ %NomString, i64 }`.
    assert!(
        out.ir_text.contains("%NomString"),
        "IR should reference NomString for heterogeneous tuple, got:\n{}",
        out.ir_text
    );
    assert!(
        out.ir_text.contains("{ %NomString, i64 }"),
        "IR should contain anonymous struct {{ %NomString, i64 }}, got:\n{}",
        out.ir_text
    );
    assert!(
        out.ir_text.contains("insertvalue"),
        "IR should contain insertvalue for tuple ctor, got:\n{}",
        out.ir_text
    );
    assert!(
        out.ir_text.contains("extractvalue"),
        "IR should contain extractvalue for p.1, got:\n{}",
        out.ir_text
    );
    assert!(
        out.ir_text.contains("ret i64"),
        "IR should return the i64 tuple slot, got:\n{}",
        out.ir_text
    );
}
