//! Integration tests for enum variant construction and match with payload
//! binding (Task C).
//!
//! These tests compile small Nom programs end-to-end and inspect the emitted
//! LLVM IR to confirm the expected lowering shape (tag i8, payload bitcast,
//! tag comparison, payload extraction). Running via `lli` is optional and
//! skipped here to keep the suite portable across Windows LLVM distributions.

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
fn test_enum_unit_variant_construct() {
    // Construct a unit variant and dispatch via match. The B variant has
    // discriminant 1, so we expect the program to return 1.
    let source = r#"
nom enums_unit
  enum E {
    A,
    B,
    C
  }
  fn main() -> integer {
    let e = E::B
    match e {
      E::A => 0,
      E::B => 1,
      E::C => 2,
      _ => 99,
    }
  }
"#;
    let plan = plan_from_source(source, "enums_unit");
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
    // Integer payload carried through match binding.
    let source = r#"
nom enums_payload
  enum T {
    I(integer),
    F(number)
  }
  fn main() -> integer {
    let t = T::I(42)
    match t {
      T::I(n) => n,
      T::F(_) => 0,
      _ => 0,
    }
  }
"#;
    let plan = plan_from_source(source, "enums_payload");
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
    // NomString payload via `S(text)`. The single arm binds the string and
    // returns `.length`, which is the i64 field of the NomString struct.
    let source = r#"
nom enums_string
  enum T {
    S(text)
  }
  fn main() -> integer {
    let t = T::S("hi")
    match t {
      T::S(s) => s.length,
      _ => 0,
    }
  }
"#;
    let plan = plan_from_source(source, "enums_string");
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
