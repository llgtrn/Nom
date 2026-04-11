use crate::context::ModuleCompiler;
use nom_ast::StructDef;

pub fn compile_struct(_mc: &mut ModuleCompiler, _struct_def: &StructDef) -> Result<(), crate::LlvmError> {
    Err(crate::LlvmError::Unsupported("struct compilation not yet implemented".into()))
}
