use crate::context::ModuleCompiler;
use nom_ast::FnDef;

pub fn compile_fn(_mc: &mut ModuleCompiler, _fn_def: &FnDef) -> Result<(), crate::LlvmError> {
    Err(crate::LlvmError::Unsupported("function compilation not yet implemented".into()))
}
