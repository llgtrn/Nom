use crate::context::ModuleCompiler;
use crate::LlvmError;
use inkwell::types::BasicType;
use inkwell::values::BasicValueEnum;
use nom_ast::{BinOp, Expr, Literal, UnaryOp};

pub fn compile_expr<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    expr: &Expr,
) -> Result<BasicValueEnum<'ctx>, LlvmError> {
    match expr {
        Expr::Literal(lit) => compile_literal(mc, lit),
        Expr::Ident(ident) => compile_ident(mc, &ident.name),
        Expr::BinaryOp(lhs, op, rhs) => compile_binary_op(mc, lhs, *op, rhs),
        Expr::UnaryOp(op, operand) => compile_unary_op(mc, *op, operand),
        Expr::Call(call) => compile_call(mc, call),
        Expr::IfExpr(if_expr) => crate::statements::compile_if_expr_value(mc, if_expr),
        Expr::MatchExpr(match_expr) => crate::statements::compile_match_value(mc, match_expr),
        Expr::Block(block) => {
            let mut last_val: Option<BasicValueEnum<'ctx>> = None;
            for stmt in &block.stmts {
                last_val = Some(crate::statements::compile_block_stmt(mc, stmt)?);
            }
            last_val.ok_or_else(|| LlvmError::Compilation("empty block expression".into()))
        }
        Expr::FieldAccess(obj, field) => compile_field_access(mc, obj, field),
        Expr::Index(_, _) => Err(LlvmError::Unsupported("index expression".into())),
        Expr::MethodCall(_, _, _) => Err(LlvmError::Unsupported("method call".into())),
        Expr::Closure(_, _) => Err(LlvmError::Unsupported("closure".into())),
        Expr::Array(elements) => compile_array(mc, elements),
        Expr::TupleExpr(_) => Err(LlvmError::Unsupported("tuple expression".into())),
        Expr::Await(_) => Err(LlvmError::Unsupported("await expression".into())),
        Expr::Cast(_, _) => Err(LlvmError::Unsupported("cast expression".into())),
        Expr::Try(_) => Err(LlvmError::Unsupported("try/? expression".into())),
    }
}

fn compile_literal<'ctx>(
    mc: &ModuleCompiler<'ctx>,
    lit: &Literal,
) -> Result<BasicValueEnum<'ctx>, LlvmError> {
    match lit {
        Literal::Number(n) => Ok(mc.context.f64_type().const_float(*n).into()),
        Literal::Integer(i) => Ok(mc.context.i64_type().const_int(*i as u64, true).into()),
        Literal::Bool(b) => {
            Ok(mc.context.bool_type().const_int(*b as u64, false).into())
        }
        Literal::Text(s) => {
            let global = mc.builder.build_global_string_ptr(s, "str")
                .map_err(|e| LlvmError::Compilation(e.to_string()))?;
            Ok(global.as_pointer_value().into())
        }
        Literal::None => Ok(mc.context.i8_type().const_zero().into()),
    }
}

fn compile_ident<'ctx>(
    mc: &ModuleCompiler<'ctx>,
    name: &str,
) -> Result<BasicValueEnum<'ctx>, LlvmError> {
    if let Some((ptr, ty)) = mc.named_values.get(name) {
        mc.builder
            .build_load(*ty, *ptr, name)
            .map_err(|e| LlvmError::Compilation(e.to_string()))
    } else {
        Err(LlvmError::Compilation(format!("undefined variable: {}", name)))
    }
}

fn compile_binary_op<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    lhs: &Expr,
    op: BinOp,
    rhs: &Expr,
) -> Result<BasicValueEnum<'ctx>, LlvmError> {
    let l = compile_expr(mc, lhs)?;
    let r = compile_expr(mc, rhs)?;

    // Float operations
    if l.is_float_value() && r.is_float_value() {
        let lf = l.into_float_value();
        let rf = r.into_float_value();
        let result = match op {
            BinOp::Add => mc.builder.build_float_add(lf, rf, "fadd"),
            BinOp::Sub => mc.builder.build_float_sub(lf, rf, "fsub"),
            BinOp::Mul => mc.builder.build_float_mul(lf, rf, "fmul"),
            BinOp::Div => mc.builder.build_float_div(lf, rf, "fdiv"),
            BinOp::Mod => mc.builder.build_float_rem(lf, rf, "frem"),
            BinOp::Gt => {
                let cmp = mc.builder.build_float_compare(
                    inkwell::FloatPredicate::OGT, lf, rf, "fcmp"
                ).map_err(|e| LlvmError::Compilation(e.to_string()))?;
                return Ok(cmp.into());
            }
            BinOp::Lt => {
                let cmp = mc.builder.build_float_compare(
                    inkwell::FloatPredicate::OLT, lf, rf, "fcmp"
                ).map_err(|e| LlvmError::Compilation(e.to_string()))?;
                return Ok(cmp.into());
            }
            BinOp::Gte => {
                let cmp = mc.builder.build_float_compare(
                    inkwell::FloatPredicate::OGE, lf, rf, "fcmp"
                ).map_err(|e| LlvmError::Compilation(e.to_string()))?;
                return Ok(cmp.into());
            }
            BinOp::Lte => {
                let cmp = mc.builder.build_float_compare(
                    inkwell::FloatPredicate::OLE, lf, rf, "fcmp"
                ).map_err(|e| LlvmError::Compilation(e.to_string()))?;
                return Ok(cmp.into());
            }
            BinOp::Eq => {
                let cmp = mc.builder.build_float_compare(
                    inkwell::FloatPredicate::OEQ, lf, rf, "fcmp"
                ).map_err(|e| LlvmError::Compilation(e.to_string()))?;
                return Ok(cmp.into());
            }
            BinOp::Neq => {
                let cmp = mc.builder.build_float_compare(
                    inkwell::FloatPredicate::ONE, lf, rf, "fcmp"
                ).map_err(|e| LlvmError::Compilation(e.to_string()))?;
                return Ok(cmp.into());
            }
            _ => return Err(LlvmError::Unsupported(format!("float op {:?}", op))),
        };
        return result
            .map(|v| v.into())
            .map_err(|e| LlvmError::Compilation(e.to_string()));
    }

    // Integer operations
    if l.is_int_value() && r.is_int_value() {
        let li = l.into_int_value();
        let ri = r.into_int_value();
        let result = match op {
            BinOp::Add => mc.builder.build_int_add(li, ri, "iadd"),
            BinOp::Sub => mc.builder.build_int_sub(li, ri, "isub"),
            BinOp::Mul => mc.builder.build_int_mul(li, ri, "imul"),
            BinOp::Div => mc.builder.build_int_signed_div(li, ri, "idiv"),
            BinOp::Mod => mc.builder.build_int_signed_rem(li, ri, "irem"),
            BinOp::And => mc.builder.build_and(li, ri, "and"),
            BinOp::Or => mc.builder.build_or(li, ri, "or"),
            BinOp::BitAnd => mc.builder.build_and(li, ri, "bitand"),
            BinOp::BitOr => mc.builder.build_or(li, ri, "bitor"),
            BinOp::Gt => {
                let cmp = mc.builder.build_int_compare(
                    inkwell::IntPredicate::SGT, li, ri, "icmp"
                ).map_err(|e| LlvmError::Compilation(e.to_string()))?;
                return Ok(cmp.into());
            }
            BinOp::Lt => {
                let cmp = mc.builder.build_int_compare(
                    inkwell::IntPredicate::SLT, li, ri, "icmp"
                ).map_err(|e| LlvmError::Compilation(e.to_string()))?;
                return Ok(cmp.into());
            }
            BinOp::Gte => {
                let cmp = mc.builder.build_int_compare(
                    inkwell::IntPredicate::SGE, li, ri, "icmp"
                ).map_err(|e| LlvmError::Compilation(e.to_string()))?;
                return Ok(cmp.into());
            }
            BinOp::Lte => {
                let cmp = mc.builder.build_int_compare(
                    inkwell::IntPredicate::SLE, li, ri, "icmp"
                ).map_err(|e| LlvmError::Compilation(e.to_string()))?;
                return Ok(cmp.into());
            }
            BinOp::Eq => {
                let cmp = mc.builder.build_int_compare(
                    inkwell::IntPredicate::EQ, li, ri, "icmp"
                ).map_err(|e| LlvmError::Compilation(e.to_string()))?;
                return Ok(cmp.into());
            }
            BinOp::Neq => {
                let cmp = mc.builder.build_int_compare(
                    inkwell::IntPredicate::NE, li, ri, "icmp"
                ).map_err(|e| LlvmError::Compilation(e.to_string()))?;
                return Ok(cmp.into());
            }
        };
        return result
            .map(|v| v.into())
            .map_err(|e| LlvmError::Compilation(e.to_string()));
    }

    Err(LlvmError::Type(format!(
        "binary op {:?}: mismatched or unsupported operand types",
        op
    )))
}

fn compile_unary_op<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    op: UnaryOp,
    operand: &Expr,
) -> Result<BasicValueEnum<'ctx>, LlvmError> {
    let val = compile_expr(mc, operand)?;
    match op {
        UnaryOp::Neg => {
            if val.is_float_value() {
                mc.builder
                    .build_float_neg(val.into_float_value(), "fneg")
                    .map(|v| v.into())
                    .map_err(|e| LlvmError::Compilation(e.to_string()))
            } else if val.is_int_value() {
                mc.builder
                    .build_int_neg(val.into_int_value(), "ineg")
                    .map(|v| v.into())
                    .map_err(|e| LlvmError::Compilation(e.to_string()))
            } else {
                Err(LlvmError::Type("neg requires numeric type".into()))
            }
        }
        UnaryOp::Not => {
            if val.is_int_value() {
                mc.builder
                    .build_not(val.into_int_value(), "not")
                    .map(|v| v.into())
                    .map_err(|e| LlvmError::Compilation(e.to_string()))
            } else {
                Err(LlvmError::Type("not requires bool/int type".into()))
            }
        }
        UnaryOp::Ref | UnaryOp::RefMut => {
            Err(LlvmError::Unsupported("ref/refmut unary op".into()))
        }
    }
}

fn compile_call<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    call: &nom_ast::CallExpr,
) -> Result<BasicValueEnum<'ctx>, LlvmError> {
    let fn_name = &call.callee.name;
    let function = mc.functions.get(fn_name).copied()
        .or_else(|| mc.module.get_function(fn_name))
        .ok_or_else(|| LlvmError::Compilation(format!("undefined function: {}", fn_name)))?;

    let mut args = Vec::new();
    for arg in &call.args {
        let val = compile_expr(mc, arg)?;
        args.push(val.into());
    }

    let call_val = mc.builder
        .build_call(function, &args, "call")
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;

    // If the function returns void, return an i8 zero as a placeholder
    call_val.try_as_basic_value()
        .left()
        .ok_or_else(|| LlvmError::Compilation("void return".into()))
        .or_else(|_| Ok(mc.context.i8_type().const_zero().into()))
}

fn compile_field_access<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    obj: &Expr,
    field: &nom_ast::Identifier,
) -> Result<BasicValueEnum<'ctx>, LlvmError> {
    // Determine the variable name from the object expression
    let var_name = match obj {
        Expr::Ident(ident) => &ident.name,
        _ => return Err(LlvmError::Unsupported("field access on non-ident expression".into())),
    };

    // Look up the struct type name for this variable
    let struct_type_name = mc.value_types.get(var_name.as_str())
        .ok_or_else(|| LlvmError::Compilation(format!(
            "unknown struct type for variable '{}'", var_name
        )))?
        .clone();

    // Find the field index
    let field_names = mc.struct_fields.get(&struct_type_name)
        .ok_or_else(|| LlvmError::Compilation(format!(
            "no field info for struct '{}'", struct_type_name
        )))?;

    let field_index = field_names.iter().position(|f| f == &field.name)
        .ok_or_else(|| LlvmError::Compilation(format!(
            "struct '{}' has no field '{}'", struct_type_name, field.name
        )))?;

    // Get the pointer to the struct variable
    let (struct_ptr, _struct_ty) = mc.named_values.get(var_name.as_str())
        .ok_or_else(|| LlvmError::Compilation(format!("undefined variable: {}", var_name)))?;

    let struct_llvm_ty = mc.struct_types.get(&struct_type_name)
        .ok_or_else(|| LlvmError::Compilation(format!("unknown struct type: {}", struct_type_name)))?;

    // GEP to the field
    let field_ptr = mc.builder
        .build_struct_gep(*struct_llvm_ty, *struct_ptr, field_index as u32, &format!("{}.{}.ptr", var_name, field.name))
        .map_err(|e| LlvmError::Compilation(format!("struct GEP failed: {}", e)))?;

    // Determine the field type and load it
    let field_ty = struct_llvm_ty.get_field_type_at_index(field_index as u32)
        .ok_or_else(|| LlvmError::Compilation(format!("invalid field index {}", field_index)))?;

    mc.builder
        .build_load(field_ty, field_ptr, &format!("{}.{}", var_name, field.name))
        .map_err(|e| LlvmError::Compilation(e.to_string()))
}

fn compile_array<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    elements: &[Expr],
) -> Result<BasicValueEnum<'ctx>, LlvmError> {
    if elements.is_empty() {
        // Return a null pointer for empty arrays
        return Ok(mc.context.ptr_type(inkwell::AddressSpace::default()).const_null().into());
    }

    // Compile all elements
    let mut compiled = Vec::new();
    for elem in elements {
        compiled.push(compile_expr(mc, elem)?);
    }

    // Determine element type from first element
    let elem_ty = compiled[0].get_type();
    let array_type = elem_ty.array_type(compiled.len() as u32);

    // Allocate array on stack
    let array_alloca = mc.builder
        .build_alloca(array_type, "arr")
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;

    let i64_type = mc.context.i64_type();
    for (i, val) in compiled.iter().enumerate() {
        let idx = i64_type.const_int(i as u64, false);
        let elem_ptr = unsafe {
            mc.builder
                .build_in_bounds_gep(array_type, array_alloca, &[i64_type.const_int(0, false), idx], &format!("arr_{}", i))
                .map_err(|e| LlvmError::Compilation(e.to_string()))?
        };
        mc.builder
            .build_store(elem_ptr, *val)
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    }

    // Return pointer to the array
    Ok(array_alloca.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::NomCompiler;
    /// Helper: create a ModuleCompiler with a dummy function and positioned builder
    fn setup_mc(compiler: &NomCompiler) -> ModuleCompiler<'_> {
        let module = compiler.context.create_module("test");
        let builder = compiler.context.create_builder();

        // Create a dummy function so the builder has a block to insert into
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
            value_types: std::collections::HashMap::new(),
            struct_fields: std::collections::HashMap::new(),
            loop_stack: Vec::new(),
        }
    }

    #[test]
    fn compiles_number_literal() {
        let compiler = NomCompiler::new();
        let mut mc = setup_mc(&compiler);
        let expr = Expr::Literal(Literal::Number(3.14));
        let val = compile_expr(&mut mc, &expr).unwrap();
        assert!(val.is_float_value());
    }

    #[test]
    fn compiles_addition() {
        let compiler = NomCompiler::new();
        let mut mc = setup_mc(&compiler);

        // Use variables so LLVM doesn't constant-fold
        let f64_ty = compiler.context.f64_type();
        let alloca_a = mc.builder.build_alloca(f64_ty, "a").unwrap();
        mc.builder.build_store(alloca_a, f64_ty.const_float(1.0)).unwrap();
        mc.named_values.insert("a".into(), (alloca_a, f64_ty.into()));

        let alloca_b = mc.builder.build_alloca(f64_ty, "b").unwrap();
        mc.builder.build_store(alloca_b, f64_ty.const_float(2.0)).unwrap();
        mc.named_values.insert("b".into(), (alloca_b, f64_ty.into()));

        let expr = Expr::BinaryOp(
            Box::new(Expr::Ident(nom_ast::Identifier::new("a", nom_ast::Span::default()))),
            BinOp::Add,
            Box::new(Expr::Ident(nom_ast::Identifier::new("b", nom_ast::Span::default()))),
        );
        let val = compile_expr(&mut mc, &expr).unwrap();
        assert!(val.is_float_value());

        // Verify IR contains fadd
        mc.builder.build_return(Some(&val)).unwrap();
        let ir = mc.module.print_to_string().to_string();
        assert!(ir.contains("fadd"), "IR should contain fadd, got:\n{}", ir);
    }

    #[test]
    fn compiles_struct_field_access() {
        // struct Point { x: number, y: number }
        // fn get_x(p: Point) -> number {
        //   return p.x
        // }
        let compiler = NomCompiler::new();
        let module = compiler.context.create_module("test_field_access");
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
        };
        crate::runtime::declare_runtime_functions(&mut mc);

        // Define the struct first
        let struct_def = nom_ast::StructDef {
            name: nom_ast::Identifier::new("Point", nom_ast::Span::default()),
            fields: vec![
                nom_ast::StructField {
                    name: nom_ast::Identifier::new("x", nom_ast::Span::default()),
                    type_ann: nom_ast::TypeExpr::Named(nom_ast::Identifier::new("number", nom_ast::Span::default())),
                    is_pub: false,
                },
                nom_ast::StructField {
                    name: nom_ast::Identifier::new("y", nom_ast::Span::default()),
                    type_ann: nom_ast::TypeExpr::Named(nom_ast::Identifier::new("number", nom_ast::Span::default())),
                    is_pub: false,
                },
            ],
            is_pub: false,
            span: nom_ast::Span::default(),
        };
        crate::structs::compile_struct(&mut mc, &struct_def).unwrap();

        // Define fn get_x(p: Point) -> number { return p.x }
        let fn_def = nom_ast::FnDef {
            name: nom_ast::Identifier::new("get_x", nom_ast::Span::default()),
            params: vec![nom_ast::FnParam {
                name: nom_ast::Identifier::new("p", nom_ast::Span::default()),
                type_ann: nom_ast::TypeExpr::Named(nom_ast::Identifier::new("Point", nom_ast::Span::default())),
            }],
            return_type: Some(nom_ast::TypeExpr::Named(nom_ast::Identifier::new("number", nom_ast::Span::default()))),
            body: nom_ast::Block {
                stmts: vec![nom_ast::BlockStmt::Return(Some(
                    nom_ast::Expr::FieldAccess(
                        Box::new(nom_ast::Expr::Ident(nom_ast::Identifier::new("p", nom_ast::Span::default()))),
                        nom_ast::Identifier::new("x", nom_ast::Span::default()),
                    ),
                ))],
                span: nom_ast::Span::default(),
            },
            is_async: false,
            is_pub: false,
            span: nom_ast::Span::default(),
        };
        crate::functions::compile_fn(&mut mc, &fn_def).unwrap();

        let ir = mc.module.print_to_string().to_string();
        assert!(
            ir.contains("getelementptr") || ir.contains("extractvalue"),
            "IR should contain getelementptr or extractvalue for field access, got:\n{}",
            ir
        );
        assert!(
            ir.contains("get_x"),
            "IR should contain get_x function, got:\n{}",
            ir
        );
    }
}
