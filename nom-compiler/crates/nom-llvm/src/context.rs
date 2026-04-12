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
    /// Stack of loop context for break/continue support.
    /// Each entry is (condition_block, end_block).
    pub loop_stack: Vec<(inkwell::basic_block::BasicBlock<'ctx>, inkwell::basic_block::BasicBlock<'ctx>)>,
    /// Maps enum type name -> ordered variant info:
    /// (variant_name, field_type_annotations). The variant's position in the
    /// vector is its runtime discriminant index (stored as the i8 tag field
    /// of the enum's tagged-union struct).
    pub enum_variants: HashMap<String, Vec<(String, Vec<nom_ast::TypeExpr>)>>,
    /// Reverse lookup: variant_name (unqualified or qualified) -> enum_name.
    /// Enables `Token::Integer(42)` to find the owning enum without repeated
    /// scans; stored both as `"Token::Integer"` and `"Integer"` when the
    /// latter is unambiguous across all registered enums.
    pub variant_to_enum: HashMap<String, String>,
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
            loop_stack: Vec::new(),
            enum_variants: HashMap::new(),
            variant_to_enum: HashMap::new(),
        };

        declare_runtime_functions(&mut mc);

        for flow in &plan.flows {
            mc.compile_flow(flow)?;
        }

        // If there's a user-defined main, wrap it for C main() convention (i32 return)
        self.generate_entry_point(&mut mc)?;

        mc.module.verify().map_err(|e| LlvmError::Verification(e.to_string()))?;

        let ir_text = mc.module.print_to_string().to_string();
        let bitcode = mc.module.write_bitcode_to_memory().as_slice().to_vec();

        Ok(LlvmOutput { ir_text, bitcode })
    }

    /// If the compiled module contains a user-defined `main` function that does
    /// not return `i32`, rename it to `_nom_main` and generate a proper
    /// `i32 @main()` wrapper that calls it and truncates/converts the result.
    /// This is required for native executables and `lli` to work correctly.
    fn generate_entry_point(&self, mc: &mut ModuleCompiler) -> Result<(), LlvmError> {
        let user_main = match mc.module.get_function("main") {
            Some(f) => f,
            None => return Ok(()), // no main function, nothing to wrap
        };

        let ret_ty = user_main.get_type().get_return_type();
        let i32_type = mc.context.i32_type();

        // If main already returns i32, no wrapping needed
        if let Some(inkwell::types::BasicTypeEnum::IntType(it)) = ret_ty {
            if it.get_bit_width() == 32 {
                return Ok(());
            }
        }

        // Rename user main to _nom_main
        user_main.as_global_value().set_name("_nom_main");

        // Create proper C main: i32 @main()
        let main_type = i32_type.fn_type(&[], false);
        let main_fn = mc.module.add_function("main", main_type, None);
        let entry = mc.context.append_basic_block(main_fn, "entry");
        mc.builder.position_at_end(entry);

        // Call _nom_main
        let call = mc
            .builder
            .build_call(user_main, &[], "result")
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;

        // Convert return value to i32
        if let Some(ret_val) = call.try_as_basic_value().left() {
            let i32_val = if ret_val.is_float_value() {
                mc.builder
                    .build_float_to_signed_int(
                        ret_val.into_float_value(),
                        i32_type,
                        "to_i32",
                    )
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?
            } else if ret_val.is_int_value() {
                let int_val = ret_val.into_int_value();
                let bw = int_val.get_type().get_bit_width();
                if bw < 32 {
                    mc.builder
                        .build_int_z_extend(int_val, i32_type, "to_i32")
                        .map_err(|e| LlvmError::Compilation(e.to_string()))?
                } else if bw > 32 {
                    mc.builder
                        .build_int_truncate(int_val, i32_type, "to_i32")
                        .map_err(|e| LlvmError::Compilation(e.to_string()))?
                } else {
                    int_val
                }
            } else {
                i32_type.const_zero()
            };
            mc.builder
                .build_return(Some(&i32_val))
                .map_err(|e| LlvmError::Compilation(e.to_string()))?;
        } else {
            mc.builder
                .build_return(Some(&i32_type.const_zero()))
                .map_err(|e| LlvmError::Compilation(e.to_string()))?;
        }

        Ok(())
    }
}

impl<'ctx> ModuleCompiler<'ctx> {
    /// Return (creating if necessary) the opaque-named LLVM struct for Nom
    /// strings: `%NomString = type { i8*, i64 }`.
    ///
    /// The struct layout matches `#[repr(C)] struct NomString` in
    /// `nom-runtime` exactly: a data pointer followed by an i64 length.
    pub fn nom_string_type(&self) -> inkwell::types::StructType<'ctx> {
        if let Some(existing) = self.context.get_struct_type("NomString") {
            return existing;
        }
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let st = self.context.opaque_struct_type("NomString");
        st.set_body(&[ptr_ty.into(), i64_ty.into()], false);
        st
    }

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
    use nom_planner::{CompositionPlan, FlowPlan, MemoryStrategy, ConcurrencyStrategy};

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

    #[test]
    fn compiles_hello_world_with_main() {
        let source = r#"
nom hello
  fn main() -> integer {
    let x: integer = 42
    let y: integer = 8
    return x + y
  }
"#;

        // Parse the source
        let parsed = nom_parser::parse_source(source).expect("should parse hello program");

        // Extract imperative statements
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

        let plan = CompositionPlan {
            source_path: Some("hello.nom".into()),
            flows: vec![FlowPlan {
                name: "hello".into(),
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

        let output = crate::compile(&plan).expect("LLVM compilation should succeed");

        // The IR should have the _nom_main wrapper (original main renamed)
        assert!(
            output.ir_text.contains("@_nom_main"),
            "IR should contain @_nom_main (renamed user main), got:\n{}",
            output.ir_text
        );

        // The IR should have a proper i32 @main() entry point
        assert!(
            output.ir_text.contains("define i32 @main()"),
            "IR should contain 'define i32 @main()', got:\n{}",
            output.ir_text
        );

        assert!(!output.bitcode.is_empty(), "bitcode should be non-empty");
    }

    #[test]
    fn no_main_no_wrapper() {
        // Programs without a main function should not get a wrapper
        let source = r#"
nom lib
  fn add(a: integer, b: integer) -> integer {
    return a + b
  }
"#;

        let parsed = nom_parser::parse_source(source).expect("should parse");
        let mut imperative_stmts = Vec::new();
        for decl in &parsed.declarations {
            for stmt in &decl.statements {
                if matches!(stmt, nom_ast::Statement::FnDef(_)) {
                    imperative_stmts.push(stmt.clone());
                }
            }
        }

        let plan = CompositionPlan {
            source_path: Some("lib.nom".into()),
            flows: vec![FlowPlan {
                name: "lib".into(),
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

        let output = crate::compile(&plan).expect("LLVM compilation should succeed");

        // Should NOT have _nom_main or i32 @main() wrapper
        assert!(
            !output.ir_text.contains("@_nom_main"),
            "IR should NOT contain @_nom_main for library modules"
        );
        assert!(
            !output.ir_text.contains("define i32 @main()"),
            "IR should NOT contain i32 @main() wrapper for library modules"
        );
    }
}
