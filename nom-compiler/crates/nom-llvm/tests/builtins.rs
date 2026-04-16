//! Integration tests for parse_int, parse_float, and chr builtins (Task E).
//!
//! These tests compile small Nom programs end-to-end and inspect the emitted
//! LLVM IR to confirm the expected lowering shape. They do not execute the
//! bitcode via `lli`; verifying at the IR level keeps tests portable across
//! Windows LLVM distributions.
//!
//! AST is constructed directly — no dependency on the deleted nom-parser crate.

use nom_ast::{
    Block, BlockStmt, CallExpr, Expr, FnDef, Identifier, LetStmt, Literal, Span, Statement,
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

fn call(name: &str, args: Vec<Expr>) -> Expr {
    Expr::Call(CallExpr {
        callee: ident(name),
        args,
        span: Span::default(),
    })
}

#[test]
fn test_parse_int_call() {
    // fn main() -> integer { return parse_int("42") }
    let fn_def = FnDef {
        name: ident("main"),
        params: vec![],
        return_type: Some(named_type("integer")),
        body: Block {
            stmts: vec![BlockStmt::Return(Some(call(
                "parse_int",
                vec![Expr::Literal(Literal::Text("42".into()))],
            )))],
            span: Span::default(),
        },
        is_async: false,
        is_pub: false,
        span: Span::default(),
    };
    let plan = make_plan("parseint", vec![Statement::FnDef(fn_def)]);
    let out = compile(&plan).expect("compile");
    // Must call nom_parse_int.
    assert!(
        out.ir_text.contains("nom_parse_int"),
        "IR should reference nom_parse_int, got:\n{}",
        out.ir_text
    );
    // The NomString must be stored into a stack slot before passing the ptr.
    assert!(
        out.ir_text.contains("alloca"),
        "IR should contain alloca for NomString slot, got:\n{}",
        out.ir_text
    );
    // The function returns i64.
    assert!(
        out.ir_text.contains("ret i64"),
        "IR should contain ret i64, got:\n{}",
        out.ir_text
    );
}

#[test]
fn test_parse_float_call() {
    // fn main() -> number { return parse_float("3.14") }
    let fn_def = FnDef {
        name: ident("main"),
        params: vec![],
        return_type: Some(named_type("number")),
        body: Block {
            stmts: vec![BlockStmt::Return(Some(call(
                "parse_float",
                vec![Expr::Literal(Literal::Text("3.14".into()))],
            )))],
            span: Span::default(),
        },
        is_async: false,
        is_pub: false,
        span: Span::default(),
    };
    let plan = make_plan("parsefloat", vec![Statement::FnDef(fn_def)]);
    let out = compile(&plan).expect("compile");
    // Must call nom_parse_float.
    assert!(
        out.ir_text.contains("nom_parse_float"),
        "IR should reference nom_parse_float, got:\n{}",
        out.ir_text
    );
    // The function returns double.
    assert!(
        out.ir_text.contains("ret double"),
        "IR should contain ret double, got:\n{}",
        out.ir_text
    );
}

#[test]
fn test_chr_returns_string() {
    // fn main() -> integer {
    //   let c: text = chr(65)
    //   return c.length
    // }
    let fn_def = FnDef {
        name: ident("main"),
        params: vec![],
        return_type: Some(named_type("integer")),
        body: Block {
            stmts: vec![
                BlockStmt::Let(LetStmt {
                    name: ident("c"),
                    mutable: false,
                    type_ann: Some(named_type("text")),
                    value: call("chr", vec![Expr::Literal(Literal::Integer(65))]),
                    span: Span::default(),
                }),
                BlockStmt::Return(Some(Expr::FieldAccess(
                    Box::new(Expr::Ident(ident("c"))),
                    ident("length"),
                ))),
            ],
            span: Span::default(),
        },
        is_async: false,
        is_pub: false,
        span: Span::default(),
    };
    let plan = make_plan("chrcall", vec![Statement::FnDef(fn_def)]);
    let out = compile(&plan).expect("compile");
    // Must call nom_chr with an i64 argument.
    assert!(
        out.ir_text.contains("nom_chr"),
        "IR should reference nom_chr, got:\n{}",
        out.ir_text
    );
    // Must extract the length field (field 1) from the returned NomString.
    assert!(
        out.ir_text.contains("extractvalue"),
        "IR should contain extractvalue for .length on chr result, got:\n{}",
        out.ir_text
    );
    // The outer function returns i64.
    assert!(
        out.ir_text.contains("ret i64"),
        "IR should contain ret i64 for the length, got:\n{}",
        out.ir_text
    );
}
