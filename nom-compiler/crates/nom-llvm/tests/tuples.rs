//! Integration tests for tuple expressions, tuple field access, and
//! tuple-typed function returns (Task B).
//!
//! These tests compile small Nom programs end-to-end and inspect the
//! emitted LLVM IR to confirm the expected lowering shape (insertvalue /
//! extractvalue / anonymous struct returns). They do not run the bitcode
//! via `lli`; verifying at the IR level keeps tests portable across
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
fn test_tuple_construct() {
    // Use non-constant tuple elements (loaded locals) so LLVM cannot
    // constant-fold the insertvalue chain away.
    let source = r#"
nom tupctor
  fn main() -> integer {
    let a = 3
    let b = 7
    let p = (a, b)
    return p.0 + p.1
  }
"#;
    let plan = plan_from_source(source, "tupctor");
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
    let source = r#"
nom tupret
  fn pair() -> (integer, integer) {
    return (1, 2)
  }
  fn main() -> integer {
    let p = pair()
    return p.0
  }
"#;
    let plan = plan_from_source(source, "tupret");
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
    // Use a local so the int slot cannot be constant-folded into a
    // whole-struct literal store — we want to see `insertvalue` in IR.
    let source = r#"
nom tuphet
  fn main() -> integer {
    let n = 5
    let p = ("hi", n)
    return p.1
  }
"#;
    let plan = plan_from_source(source, "tuphet");
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
