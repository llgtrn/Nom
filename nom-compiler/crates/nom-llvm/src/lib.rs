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

use inkwell::context::Context;
use inkwell::memory_buffer::MemoryBuffer;
use inkwell::targets::TargetMachine;
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
    let output = compiler.compile_plan(plan)?;
    normalize_output_module(output, "compiled_module")
}

/// Link multiple bitcode blobs into a single LLVM module.
/// Returns combined IR text and bitcode.
pub fn link_bitcodes(bitcode_blobs: &[Vec<u8>]) -> Result<LlvmOutput, LlvmError> {
    if bitcode_blobs.is_empty() {
        return Err(LlvmError::Compilation(
            "link_bitcodes: no bitcode blobs provided".into(),
        ));
    }

    let context = Context::create();

    // Parse the first blob as the base module
    let base_buf = MemoryBuffer::create_from_memory_range_copy(&bitcode_blobs[0], "blob_0");
    let base_module = inkwell::module::Module::parse_bitcode_from_buffer(&base_buf, &context)
        .map_err(|e| LlvmError::Compilation(format!("parse bitcode blob 0: {}", e)))?;

    // Link remaining blobs into the base module
    for (i, blob) in bitcode_blobs[1..].iter().enumerate() {
        let name = format!("blob_{}", i + 1);
        let buf = MemoryBuffer::create_from_memory_range_copy(blob, &name);
        let other = inkwell::module::Module::parse_bitcode_from_buffer(&buf, &context)
            .map_err(|e| LlvmError::Compilation(format!("parse bitcode blob {}: {}", i + 1, e)))?;
        base_module
            .link_in_module(other)
            .map_err(|e| LlvmError::Compilation(format!("link blob {}: {}", i + 1, e)))?;
    }

    normalize_module(base_module)
}

fn set_host_target_triple(module: &inkwell::module::Module<'_>) {
    let triple = TargetMachine::get_default_triple();
    module.set_triple(&triple);
}

fn normalize_output_module(output: LlvmOutput, module_name: &str) -> Result<LlvmOutput, LlvmError> {
    let context = Context::create();
    let buffer = MemoryBuffer::create_from_memory_range_copy(&output.bitcode, module_name);
    let module = inkwell::module::Module::parse_bitcode_from_buffer(&buffer, &context)
        .map_err(|e| LlvmError::Compilation(format!("parse emitted bitcode: {}", e)))?;
    normalize_module(module)
}

fn normalize_module(module: inkwell::module::Module<'_>) -> Result<LlvmOutput, LlvmError> {
    set_host_target_triple(&module);

    module
        .verify()
        .map_err(|e| LlvmError::Verification(e.to_string()))?;

    let ir_text = module.print_to_string().to_string();
    let bitcode = module.write_bitcode_to_memory().as_slice().to_vec();

    Ok(LlvmOutput { ir_text, bitcode })
}

/// Compile multiple CompositionPlans and link their bitcodes into a single module.
pub fn compile_plans(plans: &[CompositionPlan]) -> Result<LlvmOutput, LlvmError> {
    if plans.is_empty() {
        return Err(LlvmError::Compilation(
            "compile_plans: no plans provided".into(),
        ));
    }

    let blobs: Result<Vec<Vec<u8>>, LlvmError> = plans
        .iter()
        .map(|plan| compile(plan).map(|out| out.bitcode))
        .collect();

    link_bitcodes(&blobs?)
}

#[cfg(test)]
mod linking_tests {
    use super::*;
    use nom_ast::{
        BinOp, Block, BlockStmt, Expr, FnDef, FnParam, Identifier, Span, Statement, TypeExpr,
    };
    use nom_planner::{CompositionPlan, ConcurrencyStrategy, FlowPlan, MemoryStrategy};

    fn dummy_span() -> Span {
        Span::default()
    }

    fn ident(name: &str) -> Identifier {
        Identifier::new(name, dummy_span())
    }

    fn make_plan(name: &str, stmts: Vec<Statement>) -> CompositionPlan {
        CompositionPlan {
            source_path: Some(format!("{}.nom", name)),
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

    /// Build `fn <fn_name>(a: number, b: number) -> number { return a + b }`.
    fn add_fn_stmt(fn_name: &str) -> Statement {
        Statement::FnDef(FnDef {
            name: ident(fn_name),
            params: vec![
                FnParam {
                    name: ident("a"),
                    type_ann: TypeExpr::Named(ident("number")),
                },
                FnParam {
                    name: ident("b"),
                    type_ann: TypeExpr::Named(ident("number")),
                },
            ],
            return_type: Some(TypeExpr::Named(ident("number"))),
            body: Block {
                stmts: vec![BlockStmt::Return(Some(Expr::BinaryOp(
                    Box::new(Expr::Ident(ident("a"))),
                    BinOp::Add,
                    Box::new(Expr::Ident(ident("b"))),
                )))],
                span: dummy_span(),
            },
            is_async: false,
            is_pub: false,
            span: dummy_span(),
        })
    }

    #[test]
    fn link_bitcodes_two_modules() {
        let plan_a = make_plan("mod_a", vec![add_fn_stmt("add_a")]);
        let plan_b = make_plan("mod_b", vec![add_fn_stmt("add_b")]);

        let out_a = compile(&plan_a).expect("compile plan_a");
        let out_b = compile(&plan_b).expect("compile plan_b");

        let linked = link_bitcodes(&[out_a.bitcode, out_b.bitcode]).expect("link should succeed");

        assert!(
            linked.ir_text.contains("@add_a"),
            "linked IR should contain @add_a, got:\n{}",
            linked.ir_text
        );
        assert!(
            linked.ir_text.contains("@add_b"),
            "linked IR should contain @add_b, got:\n{}",
            linked.ir_text
        );
        assert!(
            !linked.bitcode.is_empty(),
            "linked bitcode should be non-empty"
        );
    }

    #[test]
    fn compile_plans_two_modules() {
        let plan_a = make_plan("mod_a", vec![add_fn_stmt("fn_alpha")]);
        let plan_b = make_plan("mod_b", vec![add_fn_stmt("fn_beta")]);

        let linked = compile_plans(&[plan_a, plan_b]).expect("compile_plans should succeed");

        assert!(
            linked.ir_text.contains("@fn_alpha"),
            "linked IR should contain @fn_alpha, got:\n{}",
            linked.ir_text
        );
        assert!(
            linked.ir_text.contains("@fn_beta"),
            "linked IR should contain @fn_beta, got:\n{}",
            linked.ir_text
        );
        assert!(!linked.bitcode.is_empty());
    }

    #[test]
    fn link_bitcodes_empty_returns_error() {
        let result = link_bitcodes(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn compile_plans_empty_returns_error() {
        let result = compile_plans(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn link_bitcodes_sets_host_target_triple() {
        let plan_a = make_plan("mod_a", vec![add_fn_stmt("add_a")]);
        let plan_b = make_plan("mod_b", vec![add_fn_stmt("add_b")]);

        let out_a = compile(&plan_a).expect("compile plan_a");
        let out_b = compile(&plan_b).expect("compile plan_b");

        let linked = link_bitcodes(&[out_a.bitcode, out_b.bitcode]).expect("link should succeed");

        assert!(
            linked.ir_text.contains("target triple = "),
            "linked IR should record a target triple, got:\n{}",
            linked.ir_text
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom_ast::{
        BinOp, Block, BlockStmt, Expr, FnDef, FnParam, Identifier, Span, Statement, TypeExpr,
    };
    use nom_planner::{CompositionPlan, ConcurrencyStrategy, FlowPlan, MemoryStrategy};

    fn ident(name: &str) -> Identifier {
        Identifier::new(name, Span::default())
    }

    fn named_type(name: &str) -> TypeExpr {
        TypeExpr::Named(ident(name))
    }

    /// Build a CompositionPlan containing `fn add(a: number, b: number) -> number { return a + b }`.
    ///
    /// Constructed directly from nom-ast types so there is no dependency on the
    /// deleted nom-parser crate. This is the S1-S6 pipeline route equivalent:
    ///   "the function add is given a of number, b of number, returns number by returning a plus b."
    /// produces a PipelineOutput that ast_bridge maps to exactly this FnDef shape.
    fn make_add_plan() -> CompositionPlan {
        let add_fn = FnDef {
            name: ident("add"),
            params: vec![
                FnParam {
                    name: ident("a"),
                    type_ann: named_type("number"),
                },
                FnParam {
                    name: ident("b"),
                    type_ann: named_type("number"),
                },
            ],
            return_type: Some(named_type("number")),
            body: Block {
                stmts: vec![BlockStmt::Return(Some(Expr::BinaryOp(
                    Box::new(Expr::Ident(ident("a"))),
                    BinOp::Add,
                    Box::new(Expr::Ident(ident("b"))),
                )))],
                span: Span::default(),
            },
            is_async: false,
            is_pub: false,
            span: Span::default(),
        };

        CompositionPlan {
            source_path: Some("add.nomtu".into()),
            flows: vec![FlowPlan {
                name: "add".into(),
                classifier: "nomtu".into(),
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
                imperative_stmts: vec![Statement::FnDef(add_fn)],
            }],
            nomiz: "{}".into(),
        }
    }

    #[test]
    fn compile_add_function() {
        let plan = make_add_plan();
        let output = compile(&plan).expect("LLVM compilation should succeed");

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
        assert!(
            output.ir_text.contains("target triple = "),
            "IR should record a target triple, got:\n{}",
            output.ir_text
        );
        assert!(!output.bitcode.is_empty(), "bitcode should be non-empty");
    }
}
