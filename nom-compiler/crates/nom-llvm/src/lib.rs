//! nom-llvm: LLVM IR backend for the Nom compiler.
//!
//! Compiles Nom's imperative core (fn, struct, enum, control flow)
//! directly to LLVM IR bitcode (.bc). No Rust middle layer.

mod context;
mod expressions;
mod functions;
mod runtime;
mod statements;
mod structs;
mod enums;
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
