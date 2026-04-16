//! Integration tests for the LLVM string-value foundation (Task A).
//!
//! These tests compile small Nom programs end-to-end and inspect the emitted
//! LLVM IR to confirm the expected lowering shape. They do not run the
//! bitcode via `lli` (the shipped LLVM on this machine does not include
//! `lli.exe`); verifying at the IR level keeps tests portable across
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
fn test_string_length() {
    // fn main() -> integer {
    //   let s: text = "hello"
    //   return s.length
    // }
    let fn_def = FnDef {
        name: ident("main"),
        params: vec![],
        return_type: Some(named_type("integer")),
        body: Block {
            stmts: vec![
                BlockStmt::Let(LetStmt {
                    name: ident("s"),
                    mutable: false,
                    type_ann: Some(named_type("text")),
                    value: Expr::Literal(Literal::Text("hello".into())),
                    span: Span::default(),
                }),
                BlockStmt::Return(Some(Expr::FieldAccess(
                    Box::new(Expr::Ident(ident("s"))),
                    ident("length"),
                ))),
            ],
            span: Span::default(),
        },
        is_async: false,
        is_pub: false,
        span: Span::default(),
    };
    let plan = make_plan("strlen", vec![Statement::FnDef(fn_def)]);
    let out = compile(&plan).expect("compile");
    // A NomString struct type must be present.
    assert!(
        out.ir_text.contains("%NomString"),
        "IR should reference %NomString type, got:\n{}",
        out.ir_text
    );
    // Must extract the length field (index 1) from the NomString value.
    assert!(
        out.ir_text.contains("extractvalue"),
        "IR should contain extractvalue for length, got:\n{}",
        out.ir_text
    );
    // Returning the length as i64 — so the function returns i64.
    assert!(
        out.ir_text.contains("ret i64"),
        "IR should contain ret i64 for length, got:\n{}",
        out.ir_text
    );
}

#[test]
fn test_string_index() {
    // fn main() -> integer {
    //   let s: text = "hello"
    //   return s[1]
    // }
    let fn_def = FnDef {
        name: ident("main"),
        params: vec![],
        return_type: Some(named_type("integer")),
        body: Block {
            stmts: vec![
                BlockStmt::Let(LetStmt {
                    name: ident("s"),
                    mutable: false,
                    type_ann: Some(named_type("text")),
                    value: Expr::Literal(Literal::Text("hello".into())),
                    span: Span::default(),
                }),
                BlockStmt::Return(Some(Expr::Index(
                    Box::new(Expr::Ident(ident("s"))),
                    Box::new(Expr::Literal(Literal::Integer(1))),
                ))),
            ],
            span: Span::default(),
        },
        is_async: false,
        is_pub: false,
        span: Span::default(),
    };
    let plan = make_plan("stridx", vec![Statement::FnDef(fn_def)]);
    let out = compile(&plan).expect("compile");
    // Expect a GEP against i8 and a load of i8, then zero-extend to i64.
    assert!(
        out.ir_text.contains("getelementptr"),
        "IR should contain GEP for string index, got:\n{}",
        out.ir_text
    );
    assert!(
        out.ir_text.contains("load i8"),
        "IR should load i8 from string data, got:\n{}",
        out.ir_text
    );
    assert!(
        out.ir_text.contains("zext i8"),
        "IR should zext i8 to i64 for index result, got:\n{}",
        out.ir_text
    );
}

#[test]
fn test_string_slice() {
    // fn main() {
    //   let s: text = "hello world"
    //   let w: text = s[6..11]
    //   print(w)
    // }
    let fn_def = FnDef {
        name: ident("main"),
        params: vec![],
        return_type: None,
        body: Block {
            stmts: vec![
                BlockStmt::Let(LetStmt {
                    name: ident("s"),
                    mutable: false,
                    type_ann: Some(named_type("text")),
                    value: Expr::Literal(Literal::Text("hello world".into())),
                    span: Span::default(),
                }),
                BlockStmt::Let(LetStmt {
                    name: ident("w"),
                    mutable: false,
                    type_ann: Some(named_type("text")),
                    value: Expr::Index(
                        Box::new(Expr::Ident(ident("s"))),
                        Box::new(Expr::Range(
                            Box::new(Expr::Literal(Literal::Integer(6))),
                            Box::new(Expr::Literal(Literal::Integer(11))),
                        )),
                    ),
                    span: Span::default(),
                }),
                BlockStmt::Expr(Expr::Call(CallExpr {
                    callee: ident("print"),
                    args: vec![Expr::Ident(ident("w"))],
                    span: Span::default(),
                })),
            ],
            span: Span::default(),
        },
        is_async: false,
        is_pub: false,
        span: Span::default(),
    };
    let plan = make_plan("strslice", vec![Statement::FnDef(fn_def)]);
    let out = compile(&plan).expect("compile");
    // Must declare and call nom_string_slice.
    assert!(
        out.ir_text.contains("nom_string_slice"),
        "IR should reference nom_string_slice, got:\n{}",
        out.ir_text
    );
    // And should call nom_print for the final print(w).
    assert!(
        out.ir_text.contains("nom_print"),
        "IR should reference nom_print (builtin print of string), got:\n{}",
        out.ir_text
    );
}

#[test]
fn test_string_eq() {
    // Build the AST directly — the existing parser treats `Token::Eq`
    // as the equality token rather than `EqEq`, which is a pre-existing
    // quirk outside the scope of this task. We exercise the codegen path
    // by constructing the expected AST shape by hand.
    let fn_def = FnDef {
        name: Identifier::new("main", Span::default()),
        params: vec![],
        return_type: Some(TypeExpr::Named(Identifier::new("bool", Span::default()))),
        body: Block {
            stmts: vec![
                BlockStmt::Let(LetStmt {
                    name: Identifier::new("a", Span::default()),
                    mutable: false,
                    type_ann: Some(TypeExpr::Named(Identifier::new("text", Span::default()))),
                    value: Expr::Literal(Literal::Text("abc".into())),
                    span: Span::default(),
                }),
                BlockStmt::Let(LetStmt {
                    name: Identifier::new("b", Span::default()),
                    mutable: false,
                    type_ann: Some(TypeExpr::Named(Identifier::new("text", Span::default()))),
                    value: Expr::Literal(Literal::Text("abc".into())),
                    span: Span::default(),
                }),
                BlockStmt::Return(Some(Expr::BinaryOp(
                    Box::new(Expr::Ident(Identifier::new("a", Span::default()))),
                    BinOp::Eq,
                    Box::new(Expr::Ident(Identifier::new("b", Span::default()))),
                ))),
            ],
            span: Span::default(),
        },
        is_async: false,
        is_pub: false,
        span: Span::default(),
    };

    let plan = CompositionPlan {
        source_path: Some("streq.nom".into()),
        flows: vec![FlowPlan {
            name: "streq".into(),
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
        out.ir_text.contains("nom_string_eq"),
        "IR should reference nom_string_eq for text equality, got:\n{}",
        out.ir_text
    );
}
