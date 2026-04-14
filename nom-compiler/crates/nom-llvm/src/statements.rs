use crate::LlvmError;
use crate::context::ModuleCompiler;
use inkwell::types::BasicType;
use inkwell::values::BasicValueEnum;
use nom_ast::{Block, BlockStmt, Expr, ForStmt, IfExpr, MatchExpr, Pattern};

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
        BlockStmt::For(for_stmt) => {
            compile_for(mc, for_stmt)?;
            Ok(mc.context.i8_type().const_zero().into())
        }
        BlockStmt::Match(match_expr) => compile_match_value(mc, match_expr),
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
        BlockStmt::Break => {
            let (_cond_bb, end_bb) = mc
                .loop_stack
                .last()
                .ok_or_else(|| LlvmError::Compilation("break outside loop".into()))?;
            mc.builder
                .build_unconditional_branch(*end_bb)
                .map_err(|e| LlvmError::Compilation(e.to_string()))?;
            Ok(mc.context.i8_type().const_zero().into())
        }
        BlockStmt::Continue => {
            let (cond_bb, _end_bb) = mc
                .loop_stack
                .last()
                .ok_or_else(|| LlvmError::Compilation("continue outside loop".into()))?;
            mc.builder
                .build_unconditional_branch(*cond_bb)
                .map_err(|e| LlvmError::Compilation(e.to_string()))?;
            Ok(mc.context.i8_type().const_zero().into())
        }
    }
}

fn compile_let<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    let_stmt: &nom_ast::LetStmt,
) -> Result<(), LlvmError> {
    let name = &let_stmt.name.name;

    // Special-case `let xs: list[T] = [...]`: an array literal in a
    // list-typed context is lowered to a `nom_list_new(sizeof T)` call
    // followed by one `nom_list_push` per initializer element. Non-literal
    // RHSes (including `list` values returned from functions) fall through
    // to the normal value-compilation path.
    if let Some(nom_ast::TypeExpr::Generic(generic_ident, args)) = &let_stmt.type_ann {
        if generic_ident.name == "list" && args.len() == 1 {
            let elem_ty_expr = &args[0];
            return compile_list_let(mc, name, elem_ty_expr, &let_stmt.value);
        }
    }

    let val = crate::expressions::compile_expr(mc, &let_stmt.value)?;

    // Determine the LLVM type from the value or type annotation
    let llvm_ty = if let Some(type_ann) = &let_stmt.type_ann {
        crate::types::resolve_type(mc, type_ann)?
    } else {
        val.get_type()
    };

    let alloca = mc
        .builder
        .build_alloca(llvm_ty, name)
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    mc.builder
        .build_store(alloca, val)
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    mc.named_values.insert(name.clone(), (alloca, llvm_ty));

    // Track struct type for field access
    if let Some(type_ann) = &let_stmt.type_ann {
        if let nom_ast::TypeExpr::Named(ident) = type_ann {
            if mc.struct_types.contains_key(&ident.name) {
                mc.value_types.insert(name.clone(), ident.name.clone());
            }
        }
    } else if let nom_ast::Expr::StructInit { name: sname, .. } = &let_stmt.value {
        // Untyped let — infer the struct name from the literal itself.
        if mc.struct_types.contains_key(&sname.name) {
            mc.value_types.insert(name.clone(), sname.name.clone());
        }
    } else if let nom_ast::Expr::Ident(src_ident) = &let_stmt.value {
        // `let l = lex` where `lex` is a known struct-typed variable —
        // inherit the struct name so subsequent `l.field` access works.
        if let Some(struct_name) = mc.value_types.get(&src_ident.name).cloned() {
            mc.value_types.insert(name.clone(), struct_name);
        }
    } else if let nom_ast::Expr::Call(call_expr) = &let_stmt.value {
        // `let l = advance(l)` — if the callee is a user function with a
        // Named return type that matches a registered struct, propagate it.
        if let Some(fn_def_return_ty) = function_return_struct_name(mc, &call_expr.callee.name) {
            mc.value_types.insert(name.clone(), fn_def_return_ty);
        }
    }
    Ok(())
}

/// Look up the registered return struct name for a user function, if any.
/// Returns `None` for functions that don't return a Nom-struct-typed value
/// (including primitives, enums, tuples, and unknowns).
fn function_return_struct_name(mc: &ModuleCompiler, fn_name: &str) -> Option<String> {
    // The LLVM backend doesn't currently retain the AST return TypeExpr per
    // function, but the struct_types map + the LLVM function's return type
    // is enough: if the return LLVM type is a named struct present in
    // struct_types, we can recover the Nom name by matching.
    let func = mc.functions.get(fn_name).copied()?;
    let ret = func.get_type().get_return_type()?;
    if let inkwell::types::BasicTypeEnum::StructType(st) = ret {
        let target_name = st.get_name().and_then(|n| n.to_str().ok())?;
        if mc.struct_types.contains_key(target_name) {
            return Some(target_name.to_owned());
        }
    }
    None
}

/// Lower `let xs: list[T] = <init>`.
///
/// For array literals we emit `nom_list_new(stride)` and then a push per
/// initializer element, storing the final `%NomList` struct into the
/// variable's alloca. The element type is recorded in
/// `mc.list_elem_types` so subsequent index/push/for-in sites can
/// recover the stride.
fn compile_list_let<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    name: &str,
    elem_ty_expr: &nom_ast::TypeExpr,
    value: &Expr,
) -> Result<(), LlvmError> {
    let nom_list_ty = mc.nom_list_type();
    let elem_llvm_ty = crate::types::resolve_type(mc, elem_ty_expr)?;
    let stride = crate::expressions::type_store_size(mc, elem_llvm_ty);
    let i64_ty = mc.context.i64_type();
    let stride_val = i64_ty.const_int(stride as u64, false);

    // Alloca the list slot up-front so push calls can take &mut NomList.
    let alloca = mc
        .builder
        .build_alloca(nom_list_ty, name)
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;

    // Decide the initial list source: array literal → nom_list_new + pushes;
    // otherwise evaluate the expression (must yield a %NomList value) and
    // store it directly.
    match value {
        Expr::Array(elements) => {
            let new_fn = mc
                .functions
                .get("nom_list_new")
                .copied()
                .ok_or_else(|| LlvmError::Compilation("nom_list_new missing".into()))?;
            let call = mc
                .builder
                .build_call(new_fn, &[stride_val.into()], "list_init")
                .map_err(|e| LlvmError::Compilation(e.to_string()))?;
            let list_val = call
                .try_as_basic_value()
                .left()
                .ok_or_else(|| LlvmError::Compilation("nom_list_new returned void".into()))?;
            mc.builder
                .build_store(alloca, list_val)
                .map_err(|e| LlvmError::Compilation(e.to_string()))?;

            // Push each initializer element. The element's stack slot is
            // reused across pushes because `nom_list_push` does an internal
            // memcpy and does not capture the pointer.
            if !elements.is_empty() {
                let push_fn = mc
                    .functions
                    .get("nom_list_push")
                    .copied()
                    .ok_or_else(|| LlvmError::Compilation("nom_list_push missing".into()))?;
                let elem_slot = mc
                    .builder
                    .build_alloca(elem_llvm_ty, "list_push_slot")
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;
                for elem in elements {
                    let v = crate::expressions::compile_expr(mc, elem)?;
                    mc.builder
                        .build_store(elem_slot, v)
                        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
                    mc.builder
                        .build_call(
                            push_fn,
                            &[alloca.into(), elem_slot.into(), stride_val.into()],
                            "list_push",
                        )
                        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
                }
            }
        }
        _ => {
            // Non-literal RHS — compile and assume it's a %NomList value.
            let v = crate::expressions::compile_expr(mc, value)?;
            mc.builder
                .build_store(alloca, v)
                .map_err(|e| LlvmError::Compilation(e.to_string()))?;
        }
    }

    mc.named_values
        .insert(name.to_owned(), (alloca, nom_list_ty.into()));
    mc.list_elem_types
        .insert(name.to_owned(), elem_ty_expr.clone());
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

    let (ptr, _ty) = mc
        .named_values
        .get(name.as_str())
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
        return Err(LlvmError::Type(
            "if condition must be numeric or bool".into(),
        ));
    };

    let function = mc
        .builder
        .get_insert_block()
        .and_then(|bb| bb.get_parent())
        .ok_or_else(|| LlvmError::Compilation("no current function".into()))?;

    let then_bb = mc.context.append_basic_block(function, "then");
    let else_bb = mc.context.append_basic_block(function, "else");
    let merge_bb = mc.context.append_basic_block(function, "ifcont");

    mc.builder
        .build_conditional_branch(cond_bool, then_bb, else_bb)
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;

    // Then block. Track whether this branch flows into merge_bb — `return`,
    // `break`, and `continue` inside the block all plant a terminator that
    // diverts elsewhere, in which case the block is NOT a PHI incoming.
    mc.builder.position_at_end(then_bb);
    let then_val = compile_block(mc, &if_expr.then_body)?;
    let then_flows_to_merge = mc
        .builder
        .get_insert_block()
        .unwrap()
        .get_terminator()
        .is_none();
    if then_flows_to_merge {
        mc.builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    }
    let then_end_bb = mc.builder.get_insert_block().unwrap();

    // Else block. If the AST carries `else if` chains, lower them in
    // cascade: each additional condition lives in its own pair of blocks
    // inside the current else_bb, with the tail falling through to the
    // final else body (or an empty default). Without this pass every
    // `else if` branch was silently skipped and control jumped straight
    // into the final `else`, which is why the token-counting driver
    // matched every char to its fallthrough branch.
    mc.builder.position_at_end(else_bb);

    // Collect the incoming (value, basic_block) pairs from every branch
    // (then, each else-if, and the final else) that actually reaches the
    // merge block. We build them as we go.
    let mut incoming: Vec<(BasicValueEnum<'ctx>, inkwell::basic_block::BasicBlock<'ctx>)> =
        Vec::new();
    if then_flows_to_merge {
        incoming.push((then_val, then_end_bb));
    }

    for (cond_expr, body) in &if_expr.else_ifs {
        let ei_cond_val = crate::expressions::compile_expr(mc, cond_expr)?;
        let ei_cond_bool = if ei_cond_val.is_int_value() {
            let iv = ei_cond_val.into_int_value();
            if iv.get_type().get_bit_width() == 1 {
                iv
            } else {
                mc.builder
                    .build_int_compare(
                        inkwell::IntPredicate::NE,
                        iv,
                        iv.get_type().const_zero(),
                        "elseifcond",
                    )
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?
            }
        } else {
            return Err(LlvmError::Type(
                "else-if condition must be numeric or bool".into(),
            ));
        };
        let ei_then_bb = mc.context.append_basic_block(function, "elifthen");
        let ei_else_bb = mc.context.append_basic_block(function, "elifelse");
        mc.builder
            .build_conditional_branch(ei_cond_bool, ei_then_bb, ei_else_bb)
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
        // Then half of the else-if.
        mc.builder.position_at_end(ei_then_bb);
        let ei_val = compile_block(mc, body)?;
        if mc
            .builder
            .get_insert_block()
            .unwrap()
            .get_terminator()
            .is_none()
        {
            let ei_end = mc.builder.get_insert_block().unwrap();
            mc.builder
                .build_unconditional_branch(merge_bb)
                .map_err(|e| LlvmError::Compilation(e.to_string()))?;
            incoming.push((ei_val, ei_end));
        }
        // Fall through to the next condition (or final else).
        mc.builder.position_at_end(ei_else_bb);
    }

    // Final else body (or nothing). We're positioned at either the
    // original else_bb (no else-ifs) or the last ei_else_bb.
    let else_val: BasicValueEnum<'ctx> = if let Some(else_body) = &if_expr.else_body {
        compile_block(mc, else_body)?
    } else {
        mc.context.f64_type().const_float(0.0).into()
    };
    let else_flows_to_merge = mc
        .builder
        .get_insert_block()
        .unwrap()
        .get_terminator()
        .is_none();
    if else_flows_to_merge {
        mc.builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    }
    let else_end_bb = mc.builder.get_insert_block().unwrap();
    if else_flows_to_merge {
        incoming.push((else_val, else_end_bb));
    }

    // Merge block. If neither side reaches here the merge is unreachable;
    // we still position there so the caller gets a consistent insertion
    // point (dead code, LLVM will strip it after optimization).
    mc.builder.position_at_end(merge_bb);

    // `incoming` has already been populated above as we compiled each
    // reachable branch. Build a PHI only when we actually have more than
    // one edge into the merge block.

    match incoming.len() {
        0 => {
            // Both branches diverge — the merge block is unreachable. Plant
            // an `unreachable` terminator so LLVM verification passes, and
            // return a placeholder; any code after will be dead.
            let _ = mc.builder.build_unreachable();
            Ok(mc.context.i8_type().const_zero().into())
        }
        1 => Ok(incoming[0].0),
        _ => {
            // Build a PHI using the first value's type as the PHI type.
            let ty = incoming[0].0.get_type();
            if ty != incoming[1].0.get_type() {
                // Types mismatch — can't build a PHI. Fallback zero.
                return Ok(mc.context.i8_type().const_zero().into());
            }
            let phi = mc
                .builder
                .build_phi(ty, "iftmp")
                .map_err(|e| LlvmError::Compilation(e.to_string()))?;
            let incoming_ref: Vec<(
                &dyn inkwell::values::BasicValue<'ctx>,
                inkwell::basic_block::BasicBlock<'ctx>,
            )> = incoming
                .iter()
                .map(|(v, bb)| (v as &dyn inkwell::values::BasicValue<'ctx>, *bb))
                .collect();
            phi.add_incoming(&incoming_ref);
            Ok(phi.as_basic_value())
        }
    }
}

fn compile_while<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    while_stmt: &nom_ast::WhileStmt,
) -> Result<(), LlvmError> {
    let function = mc
        .builder
        .get_insert_block()
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
        return Err(LlvmError::Type(
            "while condition must be integer/bool".into(),
        ));
    };

    mc.builder
        .build_conditional_branch(cond_bool, body_bb, end_bb)
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;

    // Loop body
    mc.builder.position_at_end(body_bb);
    mc.loop_stack.push((loop_bb, end_bb));
    compile_block(mc, &while_stmt.body)?;
    mc.loop_stack.pop();
    if mc
        .builder
        .get_insert_block()
        .unwrap()
        .get_terminator()
        .is_none()
    {
        mc.builder
            .build_unconditional_branch(loop_bb)
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    }

    // Continue after loop
    mc.builder.position_at_end(end_bb);
    Ok(())
}

fn compile_for<'ctx>(mc: &mut ModuleCompiler<'ctx>, for_stmt: &ForStmt) -> Result<(), LlvmError> {
    let function = mc
        .builder
        .get_insert_block()
        .and_then(|bb| bb.get_parent())
        .ok_or_else(|| LlvmError::Compilation("no current function".into()))?;

    // Check if iterating over an array literal
    if let Expr::Array(elements) = &for_stmt.iterable {
        return compile_for_array(mc, for_stmt, elements);
    }

    // Check if iterating over a `list[T]` binding — emit a length-bounded
    // index loop that calls `nom_list_get` each iteration.
    if let Expr::Ident(id) = &for_stmt.iterable {
        if let Some(elem_ty_expr) = mc.list_elem_types.get(&id.name).cloned() {
            let (list_ptr, _) =
                mc.named_values.get(&id.name).copied().ok_or_else(|| {
                    LlvmError::Compilation(format!("undefined list: {}", id.name))
                })?;
            return compile_for_list(mc, for_stmt, list_ptr, &elem_ty_expr);
        }
    }

    // Numeric range: iterate from 0 to n (exclusive)
    let n_val = crate::expressions::compile_expr(mc, &for_stmt.iterable)?;
    let n_int = if n_val.is_int_value() {
        n_val.into_int_value()
    } else if n_val.is_float_value() {
        // Convert f64 to i64
        mc.builder
            .build_float_to_signed_int(n_val.into_float_value(), mc.context.i64_type(), "ftoi")
            .map_err(|e| LlvmError::Compilation(e.to_string()))?
    } else {
        return Err(LlvmError::Type("for loop iterable must be numeric".into()));
    };

    let i64_type = mc.context.i64_type();

    // Allocate loop counter
    let counter_alloca = mc
        .builder
        .build_alloca(i64_type, &for_stmt.binding.name)
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    mc.builder
        .build_store(counter_alloca, i64_type.const_int(0, false))
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;

    // Register the binding variable
    mc.named_values.insert(
        for_stmt.binding.name.clone(),
        (counter_alloca, i64_type.into()),
    );

    let cond_bb = mc.context.append_basic_block(function, "for_cond");
    let body_bb = mc.context.append_basic_block(function, "for_body");
    let end_bb = mc.context.append_basic_block(function, "for_end");

    mc.builder
        .build_unconditional_branch(cond_bb)
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;

    // Condition block: i < n
    mc.builder.position_at_end(cond_bb);
    let cur = mc
        .builder
        .build_load(i64_type, counter_alloca, "cur")
        .map_err(|e| LlvmError::Compilation(e.to_string()))?
        .into_int_value();
    let cmp = mc
        .builder
        .build_int_compare(inkwell::IntPredicate::SLT, cur, n_int, "forcmp")
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    mc.builder
        .build_conditional_branch(cmp, body_bb, end_bb)
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;

    // Body block
    mc.builder.position_at_end(body_bb);
    mc.loop_stack.push((cond_bb, end_bb));
    compile_block(mc, &for_stmt.body)?;
    mc.loop_stack.pop();

    // Increment counter
    if mc
        .builder
        .get_insert_block()
        .unwrap()
        .get_terminator()
        .is_none()
    {
        let cur_after = mc
            .builder
            .build_load(i64_type, counter_alloca, "cur_after")
            .map_err(|e| LlvmError::Compilation(e.to_string()))?
            .into_int_value();
        let next = mc
            .builder
            .build_int_add(cur_after, i64_type.const_int(1, false), "next")
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
        mc.builder
            .build_store(counter_alloca, next)
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
        mc.builder
            .build_unconditional_branch(cond_bb)
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    }

    // Continue after loop
    mc.builder.position_at_end(end_bb);
    Ok(())
}

fn compile_for_list<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    for_stmt: &ForStmt,
    list_ptr: inkwell::values::PointerValue<'ctx>,
    elem_ty_expr: &nom_ast::TypeExpr,
) -> Result<(), LlvmError> {
    let function = mc
        .builder
        .get_insert_block()
        .and_then(|bb| bb.get_parent())
        .ok_or_else(|| LlvmError::Compilation("no current function".into()))?;

    let i64_ty = mc.context.i64_type();
    let elem_llvm_ty = crate::types::resolve_type(mc, elem_ty_expr)?;
    let stride = crate::expressions::type_store_size(mc, elem_llvm_ty);
    let stride_val = i64_ty.const_int(stride, false);

    // Counter and binding allocas.
    let counter_alloca = mc
        .builder
        .build_alloca(i64_ty, "for_list_idx")
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    mc.builder
        .build_store(counter_alloca, i64_ty.const_int(0, false))
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    let binding_alloca = mc
        .builder
        .build_alloca(elem_llvm_ty, &for_stmt.binding.name)
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    mc.named_values.insert(
        for_stmt.binding.name.clone(),
        (binding_alloca, elem_llvm_ty),
    );

    let cond_bb = mc.context.append_basic_block(function, "for_list_cond");
    let body_bb = mc.context.append_basic_block(function, "for_list_body");
    let end_bb = mc.context.append_basic_block(function, "for_list_end");

    mc.builder
        .build_unconditional_branch(cond_bb)
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;

    // Condition: idx < list_len. Call nom_list_len each iteration so the
    // loop observes pushes that might occur inside the body (matches the
    // "read current length" semantics users expect from a mutable list).
    mc.builder.position_at_end(cond_bb);
    let cur_idx = mc
        .builder
        .build_load(i64_ty, counter_alloca, "cur_idx")
        .map_err(|e| LlvmError::Compilation(e.to_string()))?
        .into_int_value();
    let len_fn = mc
        .functions
        .get("nom_list_len")
        .copied()
        .ok_or_else(|| LlvmError::Compilation("nom_list_len missing".into()))?;
    let len_call = mc
        .builder
        .build_call(len_fn, &[list_ptr.into()], "list_len")
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    let len_val = len_call
        .try_as_basic_value()
        .left()
        .ok_or_else(|| LlvmError::Compilation("nom_list_len returned void".into()))?
        .into_int_value();
    let cmp = mc
        .builder
        .build_int_compare(inkwell::IntPredicate::SLT, cur_idx, len_val, "list_cmp")
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    mc.builder
        .build_conditional_branch(cmp, body_bb, end_bb)
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;

    // Body: load element via nom_list_get, bind, execute.
    mc.builder.position_at_end(body_bb);
    let cur_idx_body = mc
        .builder
        .build_load(i64_ty, counter_alloca, "cur_idx_body")
        .map_err(|e| LlvmError::Compilation(e.to_string()))?
        .into_int_value();
    let get_fn = mc
        .functions
        .get("nom_list_get")
        .copied()
        .ok_or_else(|| LlvmError::Compilation("nom_list_get missing".into()))?;
    let get_call = mc
        .builder
        .build_call(
            get_fn,
            &[list_ptr.into(), cur_idx_body.into(), stride_val.into()],
            "list_get",
        )
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    let elem_ptr = get_call
        .try_as_basic_value()
        .left()
        .ok_or_else(|| LlvmError::Compilation("nom_list_get returned void".into()))?
        .into_pointer_value();
    let elem_val = mc
        .builder
        .build_load(elem_llvm_ty, elem_ptr, "list_elem_val")
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    mc.builder
        .build_store(binding_alloca, elem_val)
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;

    mc.loop_stack.push((cond_bb, end_bb));
    compile_block(mc, &for_stmt.body)?;
    mc.loop_stack.pop();

    // Increment index.
    if mc
        .builder
        .get_insert_block()
        .unwrap()
        .get_terminator()
        .is_none()
    {
        let cur_idx_inc = mc
            .builder
            .build_load(i64_ty, counter_alloca, "cur_idx_inc")
            .map_err(|e| LlvmError::Compilation(e.to_string()))?
            .into_int_value();
        let next_idx = mc
            .builder
            .build_int_add(cur_idx_inc, i64_ty.const_int(1, false), "next_idx")
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
        mc.builder
            .build_store(counter_alloca, next_idx)
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
        mc.builder
            .build_unconditional_branch(cond_bb)
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    }

    mc.builder.position_at_end(end_bb);
    Ok(())
}

fn compile_for_array<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    for_stmt: &ForStmt,
    elements: &[Expr],
) -> Result<(), LlvmError> {
    let function = mc
        .builder
        .get_insert_block()
        .and_then(|bb| bb.get_parent())
        .ok_or_else(|| LlvmError::Compilation("no current function".into()))?;

    let i64_type = mc.context.i64_type();
    let len = elements.len() as u64;

    // Compile all elements and determine element type
    let mut compiled_elems = Vec::new();
    for elem in elements {
        compiled_elems.push(crate::expressions::compile_expr(mc, elem)?);
    }

    // Determine the element type from the first element (or default to i64)
    let elem_ty = if let Some(first) = compiled_elems.first() {
        first.get_type()
    } else {
        i64_type.into()
    };

    // Allocate array on stack and store elements
    let array_type = elem_ty.array_type(len as u32);
    let array_alloca = mc
        .builder
        .build_alloca(array_type, "for_arr")
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;

    for (i, val) in compiled_elems.iter().enumerate() {
        let idx = i64_type.const_int(i as u64, false);
        let elem_ptr = unsafe {
            mc.builder
                .build_in_bounds_gep(
                    array_type,
                    array_alloca,
                    &[i64_type.const_int(0, false), idx],
                    &format!("arr_elem_{}", i),
                )
                .map_err(|e| LlvmError::Compilation(e.to_string()))?
        };
        mc.builder
            .build_store(elem_ptr, *val)
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    }

    // Allocate index counter
    let counter_alloca = mc
        .builder
        .build_alloca(i64_type, "for_idx")
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    mc.builder
        .build_store(counter_alloca, i64_type.const_int(0, false))
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;

    // Allocate binding variable
    let binding_alloca = mc
        .builder
        .build_alloca(elem_ty, &for_stmt.binding.name)
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    mc.named_values
        .insert(for_stmt.binding.name.clone(), (binding_alloca, elem_ty));

    let cond_bb = mc.context.append_basic_block(function, "for_arr_cond");
    let body_bb = mc.context.append_basic_block(function, "for_arr_body");
    let end_bb = mc.context.append_basic_block(function, "for_arr_end");

    mc.builder
        .build_unconditional_branch(cond_bb)
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;

    // Condition: idx < len
    mc.builder.position_at_end(cond_bb);
    let cur_idx = mc
        .builder
        .build_load(i64_type, counter_alloca, "cur_idx")
        .map_err(|e| LlvmError::Compilation(e.to_string()))?
        .into_int_value();
    let cmp = mc
        .builder
        .build_int_compare(
            inkwell::IntPredicate::SLT,
            cur_idx,
            i64_type.const_int(len, false),
            "arrcmp",
        )
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    mc.builder
        .build_conditional_branch(cmp, body_bb, end_bb)
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;

    // Body: load element, bind, execute body
    mc.builder.position_at_end(body_bb);
    let cur_idx_body = mc
        .builder
        .build_load(i64_type, counter_alloca, "cur_idx_body")
        .map_err(|e| LlvmError::Compilation(e.to_string()))?
        .into_int_value();
    let elem_ptr = unsafe {
        mc.builder
            .build_in_bounds_gep(
                array_type,
                array_alloca,
                &[i64_type.const_int(0, false), cur_idx_body],
                "arr_elem_ptr",
            )
            .map_err(|e| LlvmError::Compilation(e.to_string()))?
    };
    let elem_val = mc
        .builder
        .build_load(elem_ty, elem_ptr, "arr_elem_val")
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    mc.builder
        .build_store(binding_alloca, elem_val)
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;

    mc.loop_stack.push((cond_bb, end_bb));
    compile_block(mc, &for_stmt.body)?;
    mc.loop_stack.pop();

    // Increment index
    if mc
        .builder
        .get_insert_block()
        .unwrap()
        .get_terminator()
        .is_none()
    {
        let cur_idx_inc = mc
            .builder
            .build_load(i64_type, counter_alloca, "cur_idx_inc")
            .map_err(|e| LlvmError::Compilation(e.to_string()))?
            .into_int_value();
        let next_idx = mc
            .builder
            .build_int_add(cur_idx_inc, i64_type.const_int(1, false), "next_idx")
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
        mc.builder
            .build_store(counter_alloca, next_idx)
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
        mc.builder
            .build_unconditional_branch(cond_bb)
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    }

    mc.builder.position_at_end(end_bb);
    Ok(())
}

pub fn compile_match_value<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    match_expr: &MatchExpr,
) -> Result<BasicValueEnum<'ctx>, LlvmError> {
    let subject_val = crate::expressions::compile_expr(mc, &match_expr.subject)?;

    // If the subject is an enum (registered struct type whose name is in
    // `enum_variants`), stash it in an alloca so arms can GEP to the tag
    // and payload fields. Non-enum subjects remain value-only.
    let subject_enum: Option<(
        String,
        inkwell::types::StructType<'ctx>,
        inkwell::values::PointerValue<'ctx>,
    )> = if subject_val.is_struct_value() {
        let sv = subject_val.into_struct_value();
        let sty = sv.get_type();
        let enum_name = sty
            .get_name()
            .and_then(|n| n.to_str().ok())
            .map(|s| s.to_owned());
        match enum_name {
            Some(nm) if mc.enum_variants.contains_key(&nm) => {
                let slot = mc
                    .builder
                    .build_alloca(sty, "subj_slot")
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;
                mc.builder
                    .build_store(slot, sv)
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;
                Some((nm, sty, slot))
            }
            _ => None,
        }
    } else {
        None
    };

    let function = mc
        .builder
        .get_insert_block()
        .and_then(|bb| bb.get_parent())
        .ok_or_else(|| LlvmError::Compilation("no current function".into()))?;

    let merge_bb = mc.context.append_basic_block(function, "match_end");

    // Result alloca is materialized lazily on the first non-terminating arm
    // body — we need to see a concrete body value to pick the right LLVM
    // type (integer, float, or struct). Variant patterns may produce int or
    // float results, so hard-coding f64 (as the legacy code did) was a bug
    // for integer-returning matches.
    let mut result_slot: Option<(
        inkwell::values::PointerValue<'ctx>,
        inkwell::types::BasicTypeEnum<'ctx>,
    )> = None;

    let mut arm_blocks = Vec::new();

    // Create basic blocks for each arm test and body
    for (i, _arm) in match_expr.arms.iter().enumerate() {
        let test_bb = mc
            .context
            .append_basic_block(function, &format!("match_test_{}", i));
        let body_bb = mc
            .context
            .append_basic_block(function, &format!("match_arm_{}", i));
        arm_blocks.push((test_bb, body_bb));
    }

    // Branch to the first test block
    if let Some((first_test, _)) = arm_blocks.first() {
        mc.builder
            .build_unconditional_branch(*first_test)
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    } else {
        // No arms at all — just branch to merge
        mc.builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    }

    for (i, arm) in match_expr.arms.iter().enumerate() {
        let (test_bb, body_bb) = arm_blocks[i];
        // The "next" block is either the next arm's test or the merge block
        let fallthrough_bb = if i + 1 < arm_blocks.len() {
            arm_blocks[i + 1].0
        } else {
            merge_bb
        };

        // --- Test block ---
        mc.builder.position_at_end(test_bb);

        // Payload bindings created in the test block need to be undone when
        // we leave the arm body so they don't leak into sibling arms. We
        // snapshot the keys we add per-arm.
        let mut arm_bound_names: Vec<String> = Vec::new();

        match &arm.pattern {
            Pattern::Wildcard => {
                // Always matches — branch directly to body
                mc.builder
                    .build_unconditional_branch(body_bb)
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;
            }
            Pattern::Literal(lit) => {
                let lit_val =
                    crate::expressions::compile_expr(mc, &nom_ast::Expr::Literal(lit.clone()))?;
                let is_str_subject = crate::expressions::is_string_value_pub(mc, &subject_val);
                let is_str_pat = crate::expressions::is_string_value_pub(mc, &lit_val);
                let matches = if subject_val.is_float_value() && lit_val.is_float_value() {
                    mc.builder
                        .build_float_compare(
                            inkwell::FloatPredicate::OEQ,
                            subject_val.into_float_value(),
                            lit_val.into_float_value(),
                            "match_cmp",
                        )
                        .map_err(|e| LlvmError::Compilation(e.to_string()))?
                } else if subject_val.is_int_value() && lit_val.is_int_value() {
                    mc.builder
                        .build_int_compare(
                            inkwell::IntPredicate::EQ,
                            subject_val.into_int_value(),
                            lit_val.into_int_value(),
                            "match_cmp",
                        )
                        .map_err(|e| LlvmError::Compilation(e.to_string()))?
                } else if is_str_subject && is_str_pat {
                    // String match arm: dispatch to `nom_string_eq` and
                    // coerce the i32 (0 / non-zero) result to i1.
                    let a_ptr = crate::expressions::materialize_string_ptr_pub(mc, subject_val)?;
                    let b_ptr = crate::expressions::materialize_string_ptr_pub(mc, lit_val)?;
                    let eq_fn = mc
                        .functions
                        .get("nom_string_eq")
                        .copied()
                        .or_else(|| mc.module.get_function("nom_string_eq"))
                        .ok_or_else(|| LlvmError::Compilation("nom_string_eq missing".into()))?;
                    let call = mc
                        .builder
                        .build_call(eq_fn, &[a_ptr.into(), b_ptr.into()], "match_str_eq")
                        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
                    let result = call
                        .try_as_basic_value()
                        .left()
                        .ok_or_else(|| {
                            LlvmError::Compilation("nom_string_eq returned void".into())
                        })?
                        .into_int_value();
                    mc.builder
                        .build_int_compare(
                            inkwell::IntPredicate::NE,
                            result,
                            result.get_type().const_zero(),
                            "match_str_bool",
                        )
                        .map_err(|e| LlvmError::Compilation(e.to_string()))?
                } else {
                    return Err(LlvmError::Type(
                        "match: incompatible subject and pattern types".into(),
                    ));
                };
                mc.builder
                    .build_conditional_branch(matches, body_bb, fallthrough_bb)
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;
            }
            Pattern::Binding(ident) => {
                // Bind subject to a name, then unconditionally enter body
                let ty = subject_val.get_type();
                let alloca = mc
                    .builder
                    .build_alloca(ty, &ident.name)
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;
                mc.builder
                    .build_store(alloca, subject_val)
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;
                mc.named_values.insert(ident.name.clone(), (alloca, ty));
                arm_bound_names.push(ident.name.clone());
                mc.builder
                    .build_unconditional_branch(body_bb)
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;
            }
            Pattern::Variant(qualified_ident, sub_patterns) => {
                // Resolve variant → discriminant index + payload types.
                let qualified = qualified_ident.name.as_str();
                let (enum_name, enum_ty, subj_slot) = match &subject_enum {
                    Some(triple) => triple.clone(),
                    None => {
                        return Err(LlvmError::Type(
                            "variant pattern requires enum-typed subject".into(),
                        ));
                    }
                };
                let (disc, payload_tys) = {
                    let variants = mc.enum_variants.get(&enum_name).ok_or_else(|| {
                        LlvmError::Compilation(format!("unknown enum in match: {}", enum_name,))
                    })?;
                    // Accept both `Enum::Variant` and bare `Variant` spellings.
                    let short = qualified.rsplit("::").next().unwrap_or(qualified);
                    variants
                        .iter()
                        .enumerate()
                        .find(|(_, (n, _))| n == short)
                        .map(|(i, (_, tys))| (i, tys.clone()))
                        .ok_or_else(|| {
                            LlvmError::Compilation(format!(
                                "unknown variant in match: {}",
                                qualified,
                            ))
                        })?
                };

                // Tag GEP + compare.
                let tag_ptr = mc
                    .builder
                    .build_struct_gep(enum_ty, subj_slot, 0, "match_tag_ptr")
                    .map_err(|e| LlvmError::Compilation(format!("tag GEP: {}", e)))?;
                let tag_val = mc
                    .builder
                    .build_load(mc.context.i8_type(), tag_ptr, "match_tag")
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?
                    .into_int_value();
                let disc_const = mc.context.i8_type().const_int(disc as u64, false);
                let matches = mc
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::EQ,
                        tag_val,
                        disc_const,
                        "variant_cmp",
                    )
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;

                // Create a payload-binding block that runs only if the tag
                // matches — otherwise the binding GEP/loads would produce
                // garbage for mismatched arms (and complicate verification).
                let bind_bb = mc
                    .context
                    .append_basic_block(function, &format!("match_bind_{}", i));
                mc.builder
                    .build_conditional_branch(matches, bind_bb, fallthrough_bb)
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;

                mc.builder.position_at_end(bind_bb);

                if sub_patterns.len() != payload_tys.len() {
                    return Err(LlvmError::Compilation(format!(
                        "variant {} expects {} payload fields in pattern, got {}",
                        qualified,
                        payload_tys.len(),
                        sub_patterns.len(),
                    )));
                }

                // Bind each sub-pattern. Wildcard skips; Binding loads the
                // payload value and registers a fresh alloca. Literal sub-
                // patterns are not supported here (the lexer pipeline does
                // not need them; leaving them out keeps the surface small).
                if !payload_tys.is_empty() {
                    let payload_ptr = mc
                        .builder
                        .build_struct_gep(enum_ty, subj_slot, 1, "match_payload_ptr")
                        .map_err(|e| LlvmError::Compilation(format!("payload GEP: {}", e)))?;

                    if payload_tys.len() == 1 {
                        // Single-field payload: reinterpret the byte slot as
                        // the field's LLVM type and load directly.
                        let field_llvm_ty = crate::types::resolve_type(mc, &payload_tys[0])?;
                        if let Pattern::Binding(ident) = &sub_patterns[0] {
                            let loaded = mc
                                .builder
                                .build_load(field_llvm_ty, payload_ptr, &ident.name)
                                .map_err(|e| LlvmError::Compilation(e.to_string()))?;
                            let alloca = mc
                                .builder
                                .build_alloca(field_llvm_ty, &ident.name)
                                .map_err(|e| LlvmError::Compilation(e.to_string()))?;
                            mc.builder
                                .build_store(alloca, loaded)
                                .map_err(|e| LlvmError::Compilation(e.to_string()))?;
                            mc.named_values
                                .insert(ident.name.clone(), (alloca, field_llvm_ty));
                            arm_bound_names.push(ident.name.clone());
                            // If this is a struct type (e.g. NomString), also
                            // tag its struct-type name for `.field` access.
                            if let nom_ast::TypeExpr::Named(id) = &payload_tys[0] {
                                let n = id.name.as_str();
                                if matches!(n, "text" | "string" | "String") {
                                    // string access goes through the `.length`
                                    // special case; no type tag needed.
                                } else if mc.struct_types.contains_key(n) {
                                    mc.value_types.insert(ident.name.clone(), n.to_owned());
                                }
                            }
                        } else if matches!(&sub_patterns[0], Pattern::Wildcard) {
                            // nothing to bind
                        } else {
                            return Err(LlvmError::Unsupported(
                                "non-binding sub-pattern in variant".into(),
                            ));
                        }
                    } else {
                        // Multi-field payload: load the anonymous tuple
                        // struct through the payload pointer, then extract
                        // each bound field by index.
                        let elem_tys: Vec<inkwell::types::BasicTypeEnum<'ctx>> = payload_tys
                            .iter()
                            .map(|t| crate::types::resolve_type(mc, t))
                            .collect::<Result<_, _>>()?;
                        let tuple_ty = mc.context.struct_type(&elem_tys, false);
                        let tup_val = mc
                            .builder
                            .build_load(tuple_ty, payload_ptr, "variant_tuple")
                            .map_err(|e| LlvmError::Compilation(e.to_string()))?
                            .into_struct_value();
                        for (idx, sub) in sub_patterns.iter().enumerate() {
                            match sub {
                                Pattern::Wildcard => {}
                                Pattern::Binding(ident) => {
                                    let field_val = mc
                                        .builder
                                        .build_extract_value(tup_val, idx as u32, &ident.name)
                                        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
                                    let field_ty = field_val.get_type();
                                    let alloca = mc
                                        .builder
                                        .build_alloca(field_ty, &ident.name)
                                        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
                                    mc.builder
                                        .build_store(alloca, field_val)
                                        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
                                    mc.named_values
                                        .insert(ident.name.clone(), (alloca, field_ty));
                                    arm_bound_names.push(ident.name.clone());
                                }
                                _ => {
                                    return Err(LlvmError::Unsupported(
                                        "non-binding sub-pattern in variant".into(),
                                    ));
                                }
                            }
                        }
                    }
                }

                mc.builder
                    .build_unconditional_branch(body_bb)
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;
            }
        }

        // --- Body block ---
        mc.builder.position_at_end(body_bb);
        let body_val = compile_block(mc, &arm.body)?;

        // Store result if block didn't terminate (e.g., return)
        if mc
            .builder
            .get_insert_block()
            .unwrap()
            .get_terminator()
            .is_none()
        {
            // Lazy-initialize the result slot to the body value's LLVM type.
            let (slot, slot_ty) = match result_slot {
                Some(pair) => pair,
                None => {
                    let ty = body_val.get_type();
                    // Allocate in the function's entry block so the alloca
                    // dominates all arm exits. Inserting in a preceding
                    // basic block keeps the IR well-formed even when arms
                    // contain control flow; we simply place it at the entry
                    // block's terminator position.
                    let entry_bb = function.get_first_basic_block().unwrap();
                    let current_bb = mc.builder.get_insert_block().unwrap();
                    // Position before the entry terminator (the branch we
                    // emitted earlier to the first test block).
                    if let Some(first_instr) = entry_bb.get_first_instruction() {
                        mc.builder.position_before(&first_instr);
                    } else {
                        mc.builder.position_at_end(entry_bb);
                    }
                    let alloca = mc
                        .builder
                        .build_alloca(ty, "match_result")
                        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
                    mc.builder.position_at_end(current_bb);
                    result_slot = Some((alloca, ty));
                    (alloca, ty)
                }
            };
            // Only store if the body value matches the slot type — skip
            // heterogeneous arms (caller shouldn't mix types, but stay safe).
            if body_val.get_type() == slot_ty {
                mc.builder
                    .build_store(slot, body_val)
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;
            }
            mc.builder
                .build_unconditional_branch(merge_bb)
                .map_err(|e| LlvmError::Compilation(e.to_string()))?;
        }

        // Remove arm-scoped bindings so they don't leak into sibling arms.
        for n in &arm_bound_names {
            mc.named_values.remove(n);
            mc.value_types.remove(n);
        }
    }

    mc.builder.position_at_end(merge_bb);
    if let Some((slot, ty)) = result_slot {
        let result = mc
            .builder
            .build_load(ty, slot, "match_val")
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
        Ok(result)
    } else {
        // Every arm terminated (e.g. each `return`s). Return a dummy; the
        // merge block is unreachable in that case. Use i8 zero to match the
        // void-placeholder convention used elsewhere.
        Ok(mc.context.i8_type().const_zero().into())
    }
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
            value_types: std::collections::HashMap::new(),
            struct_fields: std::collections::HashMap::new(),
            loop_stack: Vec::new(),
            enum_variants: std::collections::HashMap::new(),
            variant_to_enum: std::collections::HashMap::new(),
            list_elem_types: std::collections::HashMap::new(),
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
        assert!(
            ir.contains("alloca"),
            "IR should contain alloca for let, got:\n{}",
            ir
        );
        assert!(
            ir.contains("store"),
            "IR should contain store for let, got:\n{}",
            ir
        );
    }

    #[test]
    fn compiles_match_on_number() {
        // fn classify(x: number) -> number {
        //   match x {
        //     1.0 => return 10.0
        //     2.0 => return 20.0
        //     _ => return 0.0
        //   }
        // }
        let compiler = NomCompiler::new();
        let module = compiler.context.create_module("test_match");
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
            enum_variants: std::collections::HashMap::new(),
            variant_to_enum: std::collections::HashMap::new(),
            list_elem_types: std::collections::HashMap::new(),
        };
        crate::runtime::declare_runtime_functions(&mut mc);

        let fn_def = FnDef {
            name: Identifier::new("classify", Span::default()),
            params: vec![FnParam {
                name: Identifier::new("x", Span::default()),
                type_ann: TypeExpr::Named(Identifier::new("number", Span::default())),
            }],
            return_type: Some(TypeExpr::Named(Identifier::new("number", Span::default()))),
            body: Block {
                stmts: vec![BlockStmt::Match(MatchExpr {
                    subject: Box::new(Expr::Ident(Identifier::new("x", Span::default()))),
                    arms: vec![
                        MatchArm {
                            pattern: Pattern::Literal(Literal::Number(1.0)),
                            body: Block {
                                stmts: vec![BlockStmt::Return(Some(Expr::Literal(
                                    Literal::Number(10.0),
                                )))],
                                span: Span::default(),
                            },
                        },
                        MatchArm {
                            pattern: Pattern::Literal(Literal::Number(2.0)),
                            body: Block {
                                stmts: vec![BlockStmt::Return(Some(Expr::Literal(
                                    Literal::Number(20.0),
                                )))],
                                span: Span::default(),
                            },
                        },
                        MatchArm {
                            pattern: Pattern::Wildcard,
                            body: Block {
                                stmts: vec![BlockStmt::Return(Some(Expr::Literal(
                                    Literal::Number(0.0),
                                )))],
                                span: Span::default(),
                            },
                        },
                    ],
                    span: Span::default(),
                })],
                span: Span::default(),
            },
            is_async: false,
            is_pub: false,
            span: Span::default(),
        };

        crate::functions::compile_fn(&mut mc, &fn_def).unwrap();

        let ir = mc.module.print_to_string().to_string();
        assert!(
            ir.contains("match_test_0") || ir.contains("match_cmp"),
            "IR should contain match comparison blocks, got:\n{}",
            ir
        );
        assert!(
            ir.contains("ret double"),
            "IR should contain ret double instructions, got:\n{}",
            ir
        );
    }

    #[test]
    fn compiles_for_loop() {
        // fn count(n: integer) -> integer {
        //   let mut sum: integer = 0
        //   for i in n {
        //     sum = sum + i
        //   }
        //   return sum
        // }
        let compiler = NomCompiler::new();
        let module = compiler.context.create_module("test_for");
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
            enum_variants: std::collections::HashMap::new(),
            variant_to_enum: std::collections::HashMap::new(),
            list_elem_types: std::collections::HashMap::new(),
        };
        crate::runtime::declare_runtime_functions(&mut mc);

        let fn_def = FnDef {
            name: Identifier::new("count", Span::default()),
            params: vec![FnParam {
                name: Identifier::new("n", Span::default()),
                type_ann: TypeExpr::Named(Identifier::new("integer", Span::default())),
            }],
            return_type: Some(TypeExpr::Named(Identifier::new("integer", Span::default()))),
            body: Block {
                stmts: vec![
                    BlockStmt::Let(LetStmt {
                        name: Identifier::new("sum", Span::default()),
                        mutable: true,
                        type_ann: Some(TypeExpr::Named(Identifier::new(
                            "integer",
                            Span::default(),
                        ))),
                        value: Expr::Literal(Literal::Integer(0)),
                        span: Span::default(),
                    }),
                    BlockStmt::For(ForStmt {
                        binding: Identifier::new("i", Span::default()),
                        iterable: Expr::Ident(Identifier::new("n", Span::default())),
                        body: Block {
                            stmts: vec![BlockStmt::Assign(AssignStmt {
                                target: Expr::Ident(Identifier::new("sum", Span::default())),
                                value: Expr::BinaryOp(
                                    Box::new(Expr::Ident(Identifier::new("sum", Span::default()))),
                                    BinOp::Add,
                                    Box::new(Expr::Ident(Identifier::new("i", Span::default()))),
                                ),
                                span: Span::default(),
                            })],
                            span: Span::default(),
                        },
                        span: Span::default(),
                    }),
                    BlockStmt::Return(Some(Expr::Ident(Identifier::new("sum", Span::default())))),
                ],
                span: Span::default(),
            },
            is_async: false,
            is_pub: false,
            span: Span::default(),
        };

        crate::functions::compile_fn(&mut mc, &fn_def).unwrap();

        let ir = mc.module.print_to_string().to_string();
        assert!(
            ir.contains("for_cond"),
            "IR should contain for_cond block, got:\n{}",
            ir
        );
        assert!(
            ir.contains("for_body"),
            "IR should contain for_body block, got:\n{}",
            ir
        );
        assert!(
            ir.contains("icmp slt"),
            "IR should contain icmp slt for loop condition, got:\n{}",
            ir
        );
        assert!(
            ir.contains("for_end"),
            "IR should contain for_end block, got:\n{}",
            ir
        );
    }

    #[test]
    fn compiles_break_in_while() {
        // fn find_first() -> integer {
        //   let mut i: integer = 0
        //   while true {
        //     if i > 5 {
        //       break
        //     }
        //     i = i + 1
        //   }
        //   return i
        // }
        let compiler = NomCompiler::new();
        let module = compiler.context.create_module("test_break");
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
            enum_variants: std::collections::HashMap::new(),
            variant_to_enum: std::collections::HashMap::new(),
            list_elem_types: std::collections::HashMap::new(),
        };
        crate::runtime::declare_runtime_functions(&mut mc);

        let fn_def = FnDef {
            name: Identifier::new("find_first", Span::default()),
            params: vec![],
            return_type: Some(TypeExpr::Named(Identifier::new("integer", Span::default()))),
            body: Block {
                stmts: vec![
                    BlockStmt::Let(LetStmt {
                        name: Identifier::new("i", Span::default()),
                        mutable: true,
                        type_ann: Some(TypeExpr::Named(Identifier::new(
                            "integer",
                            Span::default(),
                        ))),
                        value: Expr::Literal(Literal::Integer(0)),
                        span: Span::default(),
                    }),
                    BlockStmt::While(WhileStmt {
                        condition: Expr::Literal(Literal::Bool(true)),
                        body: Block {
                            stmts: vec![
                                BlockStmt::If(IfExpr {
                                    condition: Box::new(Expr::BinaryOp(
                                        Box::new(Expr::Ident(Identifier::new(
                                            "i",
                                            Span::default(),
                                        ))),
                                        BinOp::Gt,
                                        Box::new(Expr::Literal(Literal::Integer(5))),
                                    )),
                                    then_body: Block {
                                        stmts: vec![BlockStmt::Break],
                                        span: Span::default(),
                                    },
                                    else_ifs: vec![],
                                    else_body: None,
                                    span: Span::default(),
                                }),
                                BlockStmt::Assign(AssignStmt {
                                    target: Expr::Ident(Identifier::new("i", Span::default())),
                                    value: Expr::BinaryOp(
                                        Box::new(Expr::Ident(Identifier::new(
                                            "i",
                                            Span::default(),
                                        ))),
                                        BinOp::Add,
                                        Box::new(Expr::Literal(Literal::Integer(1))),
                                    ),
                                    span: Span::default(),
                                }),
                            ],
                            span: Span::default(),
                        },
                        span: Span::default(),
                    }),
                    BlockStmt::Return(Some(Expr::Ident(Identifier::new("i", Span::default())))),
                ],
                span: Span::default(),
            },
            is_async: false,
            is_pub: false,
            span: Span::default(),
        };

        crate::functions::compile_fn(&mut mc, &fn_def).unwrap();

        let ir = mc.module.print_to_string().to_string();
        assert!(
            ir.contains("loopend"),
            "IR should contain loopend block (break target), got:\n{}",
            ir
        );
        assert!(
            ir.contains("loop") && ir.contains("loopbody"),
            "IR should contain loop structure, got:\n{}",
            ir
        );
    }
}
