use crate::runtime::declare_runtime_functions;
use crate::{LlvmError, LlvmOutput};
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::builder::Builder;
use inkwell::values::FunctionValue;
use nom_ast::Statement;
use nom_planner::{CompositionPlan, FlowPlan};
use std::collections::HashMap;

pub struct NomCompiler {
    pub(crate) context: Context,
}

pub struct ModuleCompiler<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    pub named_values: HashMap<String, (inkwell::values::PointerValue<'ctx>, inkwell::types::BasicTypeEnum<'ctx>)>,
    pub struct_types: HashMap<String, inkwell::types::StructType<'ctx>>,
    pub functions: HashMap<String, FunctionValue<'ctx>>,
    /// Maps variable name -> struct type name (e.g. "p" -> "Point")
    pub value_types: HashMap<String, String>,
    /// Maps struct type name -> ordered field names (e.g. "Point" -> ["x", "y"])
    pub struct_fields: HashMap<String, Vec<String>>,
}

impl NomCompiler {
    pub fn new() -> Self {
        Self {
            context: Context::create(),
        }
    }

    pub fn compile_plan(&self, plan: &CompositionPlan) -> Result<LlvmOutput, LlvmError> {
        let module_name = plan.source_path.as_deref().unwrap_or("nom_module");
        let module = self.context.create_module(module_name);
        let builder = self.context.create_builder();
        module.set_source_file_name(module_name);

        let mut mc = ModuleCompiler {
            context: &self.context,
            module,
            builder,
            named_values: HashMap::new(),
            struct_types: HashMap::new(),
            functions: HashMap::new(),
            value_types: HashMap::new(),
            struct_fields: HashMap::new(),
        };

        declare_runtime_functions(&mut mc);

        for flow in &plan.flows {
            mc.compile_flow(flow)?;
        }

        mc.module.verify().map_err(|e| LlvmError::Verification(e.to_string()))?;

        let ir_text = mc.module.print_to_string().to_string();
        let bitcode = mc.module.write_bitcode_to_memory().as_slice().to_vec();

        Ok(LlvmOutput { ir_text, bitcode })
    }
}

impl<'ctx> ModuleCompiler<'ctx> {
    pub fn compile_flow(&mut self, flow: &FlowPlan) -> Result<(), LlvmError> {
        for stmt in &flow.imperative_stmts {
            self.compile_top_level_statement(stmt)?;
        }
        Ok(())
    }

    pub fn compile_top_level_statement(&mut self, stmt: &Statement) -> Result<(), LlvmError> {
        match stmt {
            Statement::FnDef(fn_def) => {
                crate::functions::compile_fn(self, fn_def)?;
            }
            Statement::StructDef(struct_def) => {
                crate::structs::compile_struct(self, struct_def)?;
            }
            Statement::EnumDef(enum_def) => {
                crate::enums::compile_enum(self, enum_def)?;
            }
            other => {
                return Err(LlvmError::Unsupported(format!(
                    "top-level statement: {:?}",
                    std::mem::discriminant(other)
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom_planner::CompositionPlan;

    #[test]
    fn empty_plan_produces_valid_ir() {
        let plan = CompositionPlan {
            source_path: Some("test.nom".into()),
            flows: vec![],
            nomiz: "{}".into(),
        };
        let compiler = NomCompiler::new();
        let output = compiler.compile_plan(&plan).unwrap();
        assert!(output.ir_text.contains("source_filename"));
        assert!(!output.bitcode.is_empty());
    }
}
