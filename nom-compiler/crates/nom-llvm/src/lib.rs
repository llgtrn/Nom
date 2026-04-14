//! nom-llvm: LLVM IR backend for the Nom compiler.
//!
//! Compiles Nom's imperative core (fn, struct, enum, control flow)
//! directly to LLVM IR bitcode (.bc). No Rust middle layer.

mod context;
mod enums;
mod expressions;
mod functions;
mod runtime;
mod statements;
mod structs;
mod types;

use nom_planner::CompositionPlan;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LlvmError {
    #[error("LLVM compilation error: {0}")]
    Compilation(String),
    #[error("unsupported AST node for LLVM backend: {0}")]
    Unsupported(String),
    #[error("type error: {0}")]
    Type(String),
    #[error("LLVM verification failed: {0}")]
    Verification(String),
}

/// Output from LLVM compilation.
pub struct LlvmOutput {
    /// LLVM IR as human-readable text (.ll format)
    pub ir_text: String,
    /// LLVM bitcode bytes (.bc format)
    pub bitcode: Vec<u8>,
}

/// Compile a CompositionPlan to LLVM IR.
pub fn compile(plan: &CompositionPlan) -> Result<LlvmOutput, LlvmError> {
    let compiler = context::NomCompiler::new();
    compiler.compile_plan(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom_planner::{CompositionPlan, ConcurrencyStrategy, FlowPlan, MemoryStrategy};

    #[test]
    fn compile_geometry_program() {
        let source = r#"
nom geometry
  struct Point {
    x: number,
    y: number
  }
  fn add(a: number, b: number) -> number {
    return a + b
  }
"#;

        // Step 1: Parse the source
        let parsed = nom_parser::parse_source(source).expect("should parse geometry program");

        // Step 2: Extract imperative statements (FnDef, StructDef, EnumDef)
        let mut imperative_stmts = Vec::new();
        for decl in &parsed.declarations {
            for stmt in &decl.statements {
                match stmt {
                    nom_ast::Statement::FnDef(_)
                    | nom_ast::Statement::StructDef(_)
                    | nom_ast::Statement::EnumDef(_) => {
                        imperative_stmts.push(stmt.clone());
                    }
                    _ => {}
                }
            }
        }
        assert!(
            imperative_stmts.len() >= 2,
            "expected at least 2 imperative stmts (struct + fn), got {}",
            imperative_stmts.len()
        );

        // Step 3: Create a CompositionPlan with those statements
        let plan = CompositionPlan {
            source_path: Some("geometry.nom".into()),
            flows: vec![FlowPlan {
                name: "geometry".into(),
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
        };

        // Step 4: Compile
        let output = compile(&plan).expect("LLVM compilation should succeed");

        // Step 5: Assert expected symbols in IR
        // Note: LLVM only emits named struct types in IR text when they are
        // referenced by a function or global. We verify compilation succeeded
        // (which means the struct was registered) and check function symbols.
        assert!(
            output.ir_text.contains("@add"),
            "IR should contain @add function, got:\n{}",
            output.ir_text
        );
        assert!(
            output.ir_text.contains("fadd"),
            "IR should contain fadd (float addition), got:\n{}",
            output.ir_text
        );
        assert!(!output.bitcode.is_empty(), "bitcode should be non-empty");

        // Verify struct was compiled by re-compiling and checking struct_types map
        let compiler = crate::context::NomCompiler::new();
        let module_name = "geometry_check";
        let module = compiler.context.create_module(module_name);
        let builder = compiler.context.create_builder();
        let mut mc = crate::context::ModuleCompiler {
            context: &compiler.context,
            module,
            builder,
            named_values: std::collections::HashMap::new(),
            struct_types: std::collections::HashMap::new(),
            functions: std::collections::HashMap::new(),
            value_types: std::collections::HashMap::new(),
            struct_fields: std::collections::HashMap::new(),
            loop_stack: Vec::new(),
            enum_variants: std::collections::HashMap::new(),
            variant_to_enum: std::collections::HashMap::new(),
            list_elem_types: std::collections::HashMap::new(),
        };
        for flow in &plan.flows {
            mc.compile_flow(flow).unwrap();
        }
        assert!(
            mc.struct_types.contains_key("Point"),
            "struct_types should contain Point after compilation"
        );
        let point_ty = mc.struct_types.get("Point").unwrap();
        assert_eq!(
            point_ty.count_fields(),
            2,
            "Point should have 2 fields (x, y)"
        );
    }
}
