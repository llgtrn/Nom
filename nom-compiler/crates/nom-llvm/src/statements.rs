use crate::context::ModuleCompiler;
use crate::LlvmError;
use inkwell::values::BasicValueEnum;
use nom_ast::{Block, BlockStmt, IfExpr};

/// Compile a single block statement, returning a value (i8 zero for void stmts).
pub fn compile_block_stmt<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    stmt: &BlockStmt,
) -> Result<BasicValueEnum<'ctx>, LlvmError> {
    match stmt {
        BlockStmt::Let(let_stmt) => {
            compile_let(mc, let_stmt)?;
            Ok(mc.context.i8_type().const_zero().into())
        }
        BlockStmt::Assign(assign_stmt) => {
            compile_assign(mc, assign_stmt)?;
            Ok(mc.context.i8_type().const_zero().into())
        }
        BlockStmt::Expr(expr) => crate::expressions::compile_expr(mc, expr),
        BlockStmt::If(if_expr) => compile_if_expr_value(mc, if_expr),
        BlockStmt::While(while_stmt) => {
            compile_while(mc, while_stmt)?;
            Ok(mc.context.i8_type().const_zero().into())
        }
        BlockStmt::For(_) => Err(LlvmError::Unsupported("for loop".into())),
        BlockStmt::Match(_) => Err(LlvmError::Unsupported("match expression".into())),
        BlockStmt::Return(expr) => {
            if let Some(e) = expr {
                let val = crate::expressions::compile_expr(mc, e)?;
                mc.builder
                    .build_return(Some(&val))
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;
            } else {
                mc.builder
                    .build_return(None)
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;
            }
            Ok(mc.context.i8_type().const_zero().into())
        }
        BlockStmt::Break => Err(LlvmError::Unsupported("break outside loop context".into())),
        BlockStmt::Continue => Err(LlvmError::Unsupported("continue outside loop context".into())),
    }
}

fn compile_let<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    let_stmt: &nom_ast::LetStmt,
) -> Result<(), LlvmError> {
    let val = crate::expressions::compile_expr(mc, &let_stmt.value)?;
    let name = &let_stmt.name.name;

    // Determine the LLVM type from the value or type annotation
    let llvm_ty = if let Some(type_ann) = &let_stmt.type_ann {
        crate::types::resolve_type(mc, type_ann)?
    } else {
        val.get_type()
    };

    let alloca = mc.builder
        .build_alloca(llvm_ty, name)
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    mc.builder
        .build_store(alloca, val)
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    mc.named_values.insert(name.clone(), (alloca, llvm_ty));
    Ok(())
}

fn compile_assign<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    assign_stmt: &nom_ast::AssignStmt,
) -> Result<(), LlvmError> {
    let val = crate::expressions::compile_expr(mc, &assign_stmt.value)?;

    // Extract variable name from target expression
    let name = match &assign_stmt.target {
        nom_ast::Expr::Ident(ident) => &ident.name,
        _ => return Err(LlvmError::Unsupported("non-ident assignment target".into())),
    };

    let (ptr, _ty) = mc.named_values.get(name.as_str())
        .ok_or_else(|| LlvmError::Compilation(format!("undefined variable: {}", name)))?;

    mc.builder
        .build_store(*ptr, val)
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    Ok(())
}

pub fn compile_if_expr_value<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    if_expr: &IfExpr,
) -> Result<BasicValueEnum<'ctx>, LlvmError> {
    let cond_val = crate::expressions::compile_expr(mc, &if_expr.condition)?;

    // Convert condition to i1 if needed
    let cond_bool = if cond_val.is_int_value() {
        let int_val = cond_val.into_int_value();
        if int_val.get_type().get_bit_width() == 1 {
            int_val
        } else {
            mc.builder
                .build_int_compare(
                    inkwell::IntPredicate::NE,
                    int_val,
                    int_val.get_type().const_zero(),
                    "ifcond",
                )
                .map_err(|e| LlvmError::Compilation(e.to_string()))?
        }
    } else if cond_val.is_float_value() {
        mc.builder
            .build_float_compare(
                inkwell::FloatPredicate::ONE,
                cond_val.into_float_value(),
                mc.context.f64_type().const_float(0.0),
                "ifcond",
            )
            .map_err(|e| LlvmError::Compilation(e.to_string()))?
    } else {
        return Err(LlvmError::Type("if condition must be numeric or bool".into()));
    };

    let function = mc.builder.get_insert_block()
        .and_then(|bb| bb.get_parent())
        .ok_or_else(|| LlvmError::Compilation("no current function".into()))?;

    let then_bb = mc.context.append_basic_block(function, "then");
    let else_bb = mc.context.append_basic_block(function, "else");
    let merge_bb = mc.context.append_basic_block(function, "ifcont");

    mc.builder
        .build_conditional_branch(cond_bool, then_bb, else_bb)
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;

    // Then block
    mc.builder.position_at_end(then_bb);
    let then_val = compile_block(mc, &if_expr.then_body)?;
    // Only branch if no terminator yet (return may have been emitted)
    if mc.builder.get_insert_block().unwrap().get_terminator().is_none() {
        mc.builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    }
    let then_end_bb = mc.builder.get_insert_block().unwrap();

    // Else block
    mc.builder.position_at_end(else_bb);
    let else_val = if let Some(else_body) = &if_expr.else_body {
        compile_block(mc, else_body)?
    } else {
        mc.context.f64_type().const_float(0.0).into()
    };
    if mc.builder.get_insert_block().unwrap().get_terminator().is_none() {
        mc.builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    }
    let else_end_bb = mc.builder.get_insert_block().unwrap();

    // Merge block with phi
    mc.builder.position_at_end(merge_bb);

    // If types match and are basic, build a phi node
    if then_val.get_type() == else_val.get_type() && then_val.is_float_value() {
        let phi = mc.builder
            .build_phi(mc.context.f64_type(), "iftmp")
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
        phi.add_incoming(&[(&then_val, then_end_bb), (&else_val, else_end_bb)]);
        Ok(phi.as_basic_value())
    } else if then_val.get_type() == else_val.get_type() && then_val.is_int_value() {
        let phi = mc.builder
            .build_phi(then_val.into_int_value().get_type(), "iftmp")
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
        phi.add_incoming(&[(&then_val, then_end_bb), (&else_val, else_end_bb)]);
        Ok(phi.as_basic_value())
    } else {
        // Fallback: return a zero i8
        Ok(mc.context.i8_type().const_zero().into())
    }
}

fn compile_while<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    while_stmt: &nom_ast::WhileStmt,
) -> Result<(), LlvmError> {
    let function = mc.builder.get_insert_block()
        .and_then(|bb| bb.get_parent())
        .ok_or_else(|| LlvmError::Compilation("no current function".into()))?;

    let loop_bb = mc.context.append_basic_block(function, "loop");
    let body_bb = mc.context.append_basic_block(function, "loopbody");
    let end_bb = mc.context.append_basic_block(function, "loopend");

    mc.builder
        .build_unconditional_branch(loop_bb)
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;

    // Loop header: evaluate condition
    mc.builder.position_at_end(loop_bb);
    let cond_val = crate::expressions::compile_expr(mc, &while_stmt.condition)?;
    let cond_bool = if cond_val.is_int_value() {
        let int_val = cond_val.into_int_value();
        if int_val.get_type().get_bit_width() == 1 {
            int_val
        } else {
            mc.builder
                .build_int_compare(
                    inkwell::IntPredicate::NE,
                    int_val,
                    int_val.get_type().const_zero(),
                    "whilecond",
                )
                .map_err(|e| LlvmError::Compilation(e.to_string()))?
        }
    } else {
        return Err(LlvmError::Type("while condition must be integer/bool".into()));
    };

    mc.builder
        .build_conditional_branch(cond_bool, body_bb, end_bb)
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;

    // Loop body
    mc.builder.position_at_end(body_bb);
    compile_block(mc, &while_stmt.body)?;
    if mc.builder.get_insert_block().unwrap().get_terminator().is_none() {
        mc.builder
            .build_unconditional_branch(loop_bb)
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    }

    // Continue after loop
    mc.builder.position_at_end(end_bb);
    Ok(())
}

pub fn compile_block<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    block: &Block,
) -> Result<BasicValueEnum<'ctx>, LlvmError> {
    let mut last_val: BasicValueEnum<'ctx> = mc.context.i8_type().const_zero().into();
    for stmt in &block.stmts {
        last_val = compile_block_stmt(mc, stmt)?;
    }
    Ok(last_val)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::NomCompiler;
    use nom_ast::*;

    fn setup_mc(compiler: &NomCompiler) -> ModuleCompiler<'_> {
        let module = compiler.context.create_module("test");
        let builder = compiler.context.create_builder();
        let f64_type = compiler.context.f64_type();
        let fn_type = f64_type.fn_type(&[], false);
        let function = module.add_function("__test_fn", fn_type, None);
        let entry = compiler.context.append_basic_block(function, "entry");
        builder.position_at_end(entry);
        ModuleCompiler {
            context: &compiler.context,
            module,
            builder,
            named_values: std::collections::HashMap::new(),
            struct_types: std::collections::HashMap::new(),
            functions: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn compiles_let_stmt() {
        let compiler = NomCompiler::new();
        let mut mc = setup_mc(&compiler);

        let let_stmt = LetStmt {
            name: Identifier::new("x", Span::default()),
            mutable: false,
            type_ann: Some(TypeExpr::Named(Identifier::new("number", Span::default()))),
            value: Expr::Literal(Literal::Number(42.0)),
            span: Span::default(),
        };

        compile_let(&mut mc, &let_stmt).unwrap();

        assert!(mc.named_values.contains_key("x"));

        // Add a return so the function is valid
        let val = mc.context.f64_type().const_float(0.0);
        mc.builder.build_return(Some(&val)).unwrap();

        let ir = mc.module.print_to_string().to_string();
        assert!(ir.contains("alloca"), "IR should contain alloca for let, got:\n{}", ir);
        assert!(ir.contains("store"), "IR should contain store for let, got:\n{}", ir);
    }
}
