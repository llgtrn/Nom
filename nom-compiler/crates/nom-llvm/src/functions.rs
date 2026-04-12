use crate::context::ModuleCompiler;
use crate::LlvmError;
use nom_ast::FnDef;

pub fn compile_fn<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    fn_def: &FnDef,
) -> Result<(), LlvmError> {
    let name = &fn_def.name.name;

    // Resolve parameter types
    let mut param_types = Vec::new();
    for param in &fn_def.params {
        let ty = crate::types::resolve_type(mc, &param.type_ann)?;
        param_types.push(ty.into());
    }

    // Resolve return type
    let ret_type = if let Some(ret) = &fn_def.return_type {
        Some(crate::types::resolve_type(mc, ret)?)
    } else {
        None
    };

    // Create function type
    let fn_type = match ret_type {
        Some(inkwell::types::BasicTypeEnum::FloatType(ft)) => ft.fn_type(&param_types, false),
        Some(inkwell::types::BasicTypeEnum::IntType(it)) => it.fn_type(&param_types, false),
        Some(inkwell::types::BasicTypeEnum::PointerType(pt)) => pt.fn_type(&param_types, false),
        Some(inkwell::types::BasicTypeEnum::StructType(st)) => st.fn_type(&param_types, false),
        Some(inkwell::types::BasicTypeEnum::ArrayType(at)) => at.fn_type(&param_types, false),
        Some(inkwell::types::BasicTypeEnum::VectorType(vt)) => vt.fn_type(&param_types, false),
        None => mc.context.void_type().fn_type(&param_types, false),
    };

    let function = mc.module.add_function(name, fn_type, None);
    mc.functions.insert(name.clone(), function);

    // Create entry basic block
    let entry = mc.context.append_basic_block(function, "entry");
    mc.builder.position_at_end(entry);

    // Save old named_values and value_types, start fresh for this function scope
    let old_named_values = std::mem::take(&mut mc.named_values);
    let old_value_types = std::mem::take(&mut mc.value_types);

    // Create allocas for params and store argument values
    for (i, param) in fn_def.params.iter().enumerate() {
        let param_name = &param.name.name;
        let param_ty = crate::types::resolve_type(mc, &param.type_ann)?;
        let alloca = mc.builder
            .build_alloca(param_ty, param_name)
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;

        let arg_val = function.get_nth_param(i as u32)
            .ok_or_else(|| LlvmError::Compilation(format!("missing param {}", i)))?;

        // Set parameter name in IR for readability
        arg_val.set_name(param_name);

        mc.builder
            .build_store(alloca, arg_val)
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
        mc.named_values.insert(param_name.clone(), (alloca, param_ty));

        // Track struct type for field access
        if let nom_ast::TypeExpr::Named(ident) = &param.type_ann {
            if mc.struct_types.contains_key(&ident.name) {
                mc.value_types.insert(param_name.clone(), ident.name.clone());
            }
        }
    }

    // Compile function body
    crate::statements::compile_block(mc, &fn_def.body)?;

    // Add default return if no terminator on current block
    let current_bb = mc.builder.get_insert_block().unwrap();
    if current_bb.get_terminator().is_none() {
        match ret_type {
            Some(inkwell::types::BasicTypeEnum::FloatType(ft)) => {
                mc.builder.build_return(Some(&ft.const_float(0.0)))
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;
            }
            Some(inkwell::types::BasicTypeEnum::IntType(it)) => {
                mc.builder.build_return(Some(&it.const_zero()))
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;
            }
            Some(inkwell::types::BasicTypeEnum::PointerType(pt)) => {
                mc.builder.build_return(Some(&pt.const_null()))
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;
            }
            Some(_) => {
                mc.builder.build_return(None)
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;
            }
            None => {
                mc.builder.build_return(None)
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;
            }
        }
    }

    // Restore old named_values and value_types
    mc.named_values = old_named_values;
    mc.value_types = old_value_types;

    // Verify function
    if !function.verify(true) {
        return Err(LlvmError::Verification(format!(
            "function '{}' failed LLVM verification",
            name
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::NomCompiler;
    use crate::runtime::declare_runtime_functions;
    use nom_ast::*;

    #[test]
    fn compiles_add_function() {
        let compiler = NomCompiler::new();
        let module = compiler.context.create_module("test");
        let builder = compiler.context.create_builder();
        let mut mc = ModuleCompiler {
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
        declare_runtime_functions(&mut mc);

        // fn add(a: number, b: number) -> number { return a + b }
        let fn_def = FnDef {
            name: Identifier::new("add", Span::default()),
            params: vec![
                FnParam {
                    name: Identifier::new("a", Span::default()),
                    type_ann: TypeExpr::Named(Identifier::new("number", Span::default())),
                },
                FnParam {
                    name: Identifier::new("b", Span::default()),
                    type_ann: TypeExpr::Named(Identifier::new("number", Span::default())),
                },
            ],
            return_type: Some(TypeExpr::Named(Identifier::new("number", Span::default()))),
            body: Block {
                stmts: vec![BlockStmt::Return(Some(Expr::BinaryOp(
                    Box::new(Expr::Ident(Identifier::new("a", Span::default()))),
                    BinOp::Add,
                    Box::new(Expr::Ident(Identifier::new("b", Span::default()))),
                )))],
                span: Span::default(),
            },
            is_async: false,
            is_pub: false,
            span: Span::default(),
        };

        compile_fn(&mut mc, &fn_def).unwrap();

        let ir = mc.module.print_to_string().to_string();
        assert!(
            ir.contains("define double @add(double"),
            "IR should contain function signature, got:\n{}",
            ir
        );
        assert!(ir.contains("fadd"), "IR should contain fadd, got:\n{}", ir);
        assert!(ir.contains("ret double"), "IR should contain ret double, got:\n{}", ir);
    }
}
