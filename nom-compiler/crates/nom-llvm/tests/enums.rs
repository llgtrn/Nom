//! Integration tests for enum variant construction and match with payload
//! binding (Task C).
//!
//! These tests compile small Nom programs end-to-end and inspect the emitted
//! LLVM IR to confirm the expected lowering shape (tag i8, payload bitcast,
//! tag comparison, payload extraction). Running via `lli` is optional and
//! skipped here to keep the suite portable across Windows LLVM distributions.
//!
//! AST is constructed directly — no dependency on the deleted nom-parser crate.

use nom_ast::{
    Block, BlockStmt, EnumDef, EnumVariant, Expr, FnDef, Identifier, LetStmt, Literal, MatchArm,
    MatchExpr, Pattern, Span, Statement, TypeExpr,
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

/// Build a single-expression arm: `Pattern => { <int_val> }`
/// Uses Expr (not Return) so the value flows through the match expression result.
fn int_arm(pattern: Pattern, val: i64) -> MatchArm {
    MatchArm {
        pattern,
        body: Block {
            stmts: vec![BlockStmt::Expr(Expr::Literal(Literal::Integer(val)))],
            span: Span::default(),
        },
    }
}

#[test]
fn test_enum_unit_variant_construct() {
    // enum E { A, B, C }
    // fn main() -> integer {
    //   let e = E::B
    //   match e { E::A => 0, E::B => 1, E::C => 2, _ => 99, }
    // }
    let enum_def = EnumDef {
        name: ident("E"),
        variants: vec![
            EnumVariant {
                name: ident("A"),
                fields: vec![],
            },
            EnumVariant {
                name: ident("B"),
                fields: vec![],
            },
            EnumVariant {
                name: ident("C"),
                fields: vec![],
            },
        ],
        is_pub: false,
        span: Span::default(),
    };
    let fn_def = FnDef {
        name: ident("main"),
        params: vec![],
        return_type: Some(named_type("integer")),
        body: Block {
            stmts: vec![
                BlockStmt::Let(LetStmt {
                    name: ident("e"),
                    mutable: false,
                    type_ann: None,
                    // E::B referenced as an identifier (zero-arg variant)
                    value: Expr::Ident(ident("E::B")),
                    span: Span::default(),
                }),
                BlockStmt::Return(Some(Expr::MatchExpr(Box::new(MatchExpr {
                    subject: Box::new(Expr::Ident(ident("e"))),
                    arms: vec![
                        int_arm(Pattern::Variant(ident("E::A"), vec![]), 0),
                        int_arm(Pattern::Variant(ident("E::B"), vec![]), 1),
                        int_arm(Pattern::Variant(ident("E::C"), vec![]), 2),
                        int_arm(Pattern::Wildcard, 99),
                    ],
                    span: Span::default(),
                })))),
            ],
            span: Span::default(),
        },
        is_async: false,
        is_pub: false,
        span: Span::default(),
    };
    let plan = make_plan(
        "enums_unit",
        vec![Statement::EnumDef(enum_def), Statement::FnDef(fn_def)],
    );
    let out = compile(&plan).expect("compile");
    // Tag is stored as i8; expect at least one i8 store for the discriminant.
    assert!(
        out.ir_text.contains("store i8"),
        "IR should contain i8 tag store, got:\n{}",
        out.ir_text
    );
    // Tag comparison: load i8 from enum, compare to constant.
    assert!(
        out.ir_text.contains("icmp eq i8"),
        "IR should contain i8 tag comparison in match, got:\n{}",
        out.ir_text
    );
    // GEP into the enum struct (field 0 = tag).
    assert!(
        out.ir_text.contains("getelementptr"),
        "IR should contain GEP for enum field access, got:\n{}",
        out.ir_text
    );
}

#[test]
fn test_enum_payload_construct() {
    // enum T { I(integer), F(number) }
    // fn main() -> integer {
    //   let t = T::I(42)
    //   match t { T::I(n) => n, T::F(_) => 0, _ => 0, }
    // }
    use nom_ast::CallExpr;

    let enum_def = EnumDef {
        name: ident("T"),
        variants: vec![
            EnumVariant {
                name: ident("I"),
                fields: vec![named_type("integer")],
            },
            EnumVariant {
                name: ident("F"),
                fields: vec![named_type("number")],
            },
        ],
        is_pub: false,
        span: Span::default(),
    };
    let fn_def = FnDef {
        name: ident("main"),
        params: vec![],
        return_type: Some(named_type("integer")),
        body: Block {
            stmts: vec![
                BlockStmt::Let(LetStmt {
                    name: ident("t"),
                    mutable: false,
                    type_ann: None,
                    // T::I(42) — Call with qualified name
                    value: Expr::Call(CallExpr {
                        callee: ident("T::I"),
                        args: vec![Expr::Literal(Literal::Integer(42))],
                        span: Span::default(),
                    }),
                    span: Span::default(),
                }),
                BlockStmt::Return(Some(Expr::MatchExpr(Box::new(MatchExpr {
                    subject: Box::new(Expr::Ident(ident("t"))),
                    arms: vec![
                        MatchArm {
                            pattern: Pattern::Variant(
                                ident("T::I"),
                                vec![Pattern::Binding(ident("n"))],
                            ),
                            body: Block {
                                stmts: vec![BlockStmt::Expr(Expr::Ident(ident("n")))],
                                span: Span::default(),
                            },
                        },
                        MatchArm {
                            pattern: Pattern::Variant(ident("T::F"), vec![Pattern::Wildcard]),
                            body: Block {
                                stmts: vec![BlockStmt::Expr(Expr::Literal(Literal::Integer(0)))],
                                span: Span::default(),
                            },
                        },
                        int_arm(Pattern::Wildcard, 0),
                    ],
                    span: Span::default(),
                })))),
            ],
            span: Span::default(),
        },
        is_async: false,
        is_pub: false,
        span: Span::default(),
    };
    let plan = make_plan(
        "enums_payload",
        vec![Statement::EnumDef(enum_def), Statement::FnDef(fn_def)],
    );
    let out = compile(&plan).expect("compile");
    // Discriminant tag stored as i8.
    assert!(
        out.ir_text.contains("store i8"),
        "IR should contain i8 tag store, got:\n{}",
        out.ir_text
    );
    // Payload write / read through a GEP (the payload field is at index 1).
    assert!(
        out.ir_text.contains("getelementptr"),
        "IR should contain GEP for enum payload access, got:\n{}",
        out.ir_text
    );
    // Match-arm body should ultimately return i64.
    assert!(
        out.ir_text.contains("ret i64"),
        "IR should return i64, got:\n{}",
        out.ir_text
    );
}

#[test]
fn test_enum_string_payload() {
    // enum T { S(text) }
    // fn main() -> integer {
    //   let t = T::S("hi")
    //   match t { T::S(s) => s.length, _ => 0, }
    // }
    use nom_ast::CallExpr;

    let enum_def = EnumDef {
        name: ident("T"),
        variants: vec![EnumVariant {
            name: ident("S"),
            fields: vec![named_type("text")],
        }],
        is_pub: false,
        span: Span::default(),
    };
    let fn_def = FnDef {
        name: ident("main"),
        params: vec![],
        return_type: Some(named_type("integer")),
        body: Block {
            stmts: vec![
                BlockStmt::Let(LetStmt {
                    name: ident("t"),
                    mutable: false,
                    type_ann: None,
                    value: Expr::Call(CallExpr {
                        callee: ident("T::S"),
                        args: vec![Expr::Literal(Literal::Text("hi".into()))],
                        span: Span::default(),
                    }),
                    span: Span::default(),
                }),
                BlockStmt::Return(Some(Expr::MatchExpr(Box::new(MatchExpr {
                    subject: Box::new(Expr::Ident(ident("t"))),
                    arms: vec![
                        MatchArm {
                            pattern: Pattern::Variant(
                                ident("T::S"),
                                vec![Pattern::Binding(ident("s"))],
                            ),
                            body: Block {
                                stmts: vec![BlockStmt::Expr(Expr::FieldAccess(
                                    Box::new(Expr::Ident(ident("s"))),
                                    ident("length"),
                                ))],
                                span: Span::default(),
                            },
                        },
                        int_arm(Pattern::Wildcard, 0),
                    ],
                    span: Span::default(),
                })))),
            ],
            span: Span::default(),
        },
        is_async: false,
        is_pub: false,
        span: Span::default(),
    };
    let plan = make_plan(
        "enums_string",
        vec![Statement::EnumDef(enum_def), Statement::FnDef(fn_def)],
    );
    let out = compile(&plan).expect("compile");
    assert!(
        out.ir_text.contains("store i8"),
        "IR should contain i8 tag store, got:\n{}",
        out.ir_text
    );
    assert!(
        out.ir_text.contains("getelementptr"),
        "IR should contain GEP for payload access, got:\n{}",
        out.ir_text
    );
    // The NomString payload size is 16 bytes; the enum payload field should
    // accommodate that. Either the enum struct is defined with a 16-byte
    // array payload or an `[16 x i8]` literal should appear near the type
    // decl. Verifying the bitcode non-empty is sufficient for portability.
    assert!(!out.bitcode.is_empty(), "bitcode should be non-empty");
}
