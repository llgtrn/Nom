//! Integration tests for the LLVM `list[T]` lowering (Task D).
//!
//! The tests compile small Nom programs end-to-end and inspect the emitted
//! LLVM IR. They do not run the bitcode via `lli` because the bundled
//! Windows LLVM distribution does not ship `lli.exe`; IR-level checks are
//! portable across platforms and catch the relevant lowering shape.

use nom_llvm::compile;
use nom_planner::{CompositionPlan, ConcurrencyStrategy, FlowPlan, MemoryStrategy};

fn plan_from_source(source: &str, name: &str) -> CompositionPlan {
    let parsed = nom_parser::parse_source(source).expect("should parse");
    let mut imperative_stmts = Vec::new();
    for decl in &parsed.declarations {
        for stmt in &decl.statements {
            match stmt {
                nom_ast::Statement::FnDef(_)
                | nom_ast::Statement::StructDef(_)
                | nom_ast::Statement::EnumDef(_) => imperative_stmts.push(stmt.clone()),
                _ => {}
            }
        }
    }
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
            imperative_stmts,
        }],
        nomiz: "{}".into(),
    }
}

#[test]
fn test_list_empty_and_length() {
    let source = r#"
nom listlen
  fn main() -> integer {
    let xs: list[integer] = []
    return xs.length
  }
"#;
    let plan = plan_from_source(source, "listlen");
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
    use nom_ast::{
        Block, BlockStmt, Expr, FnDef, Identifier, LetStmt, Literal, Span, TypeExpr,
    };

    fn lst_t() -> TypeExpr {
        TypeExpr::Generic(
            Identifier::new("list", Span::default()),
            vec![TypeExpr::Named(Identifier::new("integer", Span::default()))],
        )
    }
    fn int_lit(n: i64) -> Expr {
        Expr::Literal(Literal::Integer(n))
    }
    fn ident(name: &str) -> Expr {
        Expr::Ident(Identifier::new(name, Span::default()))
    }
    fn push_stmt(list: &str, v: i64) -> BlockStmt {
        BlockStmt::Expr(Expr::MethodCall(
            Box::new(ident(list)),
            Identifier::new("push", Span::default()),
            vec![int_lit(v)],
        ))
    }

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
                    Box::new(ident("xs")),
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
    // let xs: list[integer] = [10, 20, 30]
    // let s = 0
    // for x in xs { s = s + x }
    // return s
    let source = r#"
nom listsum
  fn main() -> integer {
    let xs: list[integer] = [10, 20, 30]
    let s: integer = 0
    for x in xs {
      s = s + x
    }
    return s
  }
"#;
    let plan = plan_from_source(source, "listsum");
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
