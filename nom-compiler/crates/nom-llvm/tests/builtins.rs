//! Integration tests for parse_int, parse_float, and chr builtins (Task E).
//!
//! These tests compile small Nom programs end-to-end and inspect the emitted
//! LLVM IR to confirm the expected lowering shape. They do not execute the
//! bitcode via `lli`; verifying at the IR level keeps tests portable across
//! Windows LLVM distributions.

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
fn test_parse_int_call() {
    let source = r#"
nom parseint
  fn main() -> integer {
    return parse_int("42")
  }
"#;
    let plan = plan_from_source(source, "parseint");
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
    let source = r#"
nom parsefloat
  fn main() -> number {
    return parse_float("3.14")
  }
"#;
    let plan = plan_from_source(source, "parsefloat");
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
    let source = r#"
nom chrcall
  fn main() -> integer {
    let c: text = chr(65)
    return c.length
  }
"#;
    let plan = plan_from_source(source, "chrcall");
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
