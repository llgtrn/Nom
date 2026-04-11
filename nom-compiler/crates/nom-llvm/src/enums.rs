use crate::context::ModuleCompiler;
use nom_ast::EnumDef;

pub fn compile_enum(_mc: &mut ModuleCompiler, _enum_def: &EnumDef) -> Result<(), crate::LlvmError> {
    Err(crate::LlvmError::Unsupported("enum compilation not yet implemented".into()))
}
