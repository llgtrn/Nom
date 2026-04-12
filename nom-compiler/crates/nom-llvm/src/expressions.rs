use crate::context::ModuleCompiler;
use crate::LlvmError;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::BasicValueEnum;
use nom_ast::{BinOp, Expr, Literal, UnaryOp};

/// Compile-time byte size for an LLVM element type. Used by `list[T]`
/// codegen to derive the `elem_size` argument passed to every runtime
/// entry point.
///
/// Inkwell does not expose a portable `size_of` that returns a Rust
/// integer at compile time (the `size_of` method on BasicType yields an
/// IntValue that only materializes at IR level). For the small set of
/// element types currently supported in Nom's surface language, a simple
/// static table is sufficient and avoids pulling TargetData plumbing in.
///
/// Layout assumptions baked in here MUST match the 8-byte alignment used
/// by `nom_runtime::list::layout_for`. If a new scalar is added with
/// larger alignment, both sides need updating together.
pub fn type_store_size<'ctx>(_mc: &ModuleCompiler<'ctx>, ty: BasicTypeEnum<'ctx>) -> u64 {
    match ty {
        BasicTypeEnum::IntType(it) => ((it.get_bit_width() as u64) + 7) / 8,
        BasicTypeEnum::FloatType(_) => 8, // f64
        BasicTypeEnum::PointerType(_) => 8,
        BasicTypeEnum::StructType(st) => {
            // Approximate: sum the store sizes of each field, rounding up
            // to 8-byte alignment at the struct boundary. Matches what the
            // tagged-union enum lowering does (see enums.rs). For Nom
            // structs whose fields are all {i64, f64, ptr, bool, struct},
            // this is accurate; packed/sub-word layouts would require the
            // DataLayout path.
            let mut total: u64 = 0;
            for i in 0..st.count_fields() {
                if let Some(fty) = st.get_field_type_at_index(i) {
                    let s = match fty {
                        BasicTypeEnum::IntType(it) => {
                            let bytes = ((it.get_bit_width() as u64) + 7) / 8;
                            bytes.max(1)
                        }
                        BasicTypeEnum::FloatType(_) => 8,
                        BasicTypeEnum::PointerType(_) => 8,
                        BasicTypeEnum::StructType(_) => 16,
                        BasicTypeEnum::ArrayType(at) => {
                            let n = at.len() as u64;
                            let elem = match at.get_element_type() {
                                BasicTypeEnum::IntType(eit) => {
                                    (((eit.get_bit_width() as u64) + 7) / 8).max(1)
                                }
                                _ => 8,
                            };
                            n * elem
                        }
                        _ => 8,
                    };
                    total += s;
                }
            }
            // Round to 8-byte alignment.
            ((total + 7) / 8) * 8
        }
        BasicTypeEnum::ArrayType(at) => {
            let n = at.len() as u64;
            let elem = match at.get_element_type() {
                BasicTypeEnum::IntType(eit) => (((eit.get_bit_width() as u64) + 7) / 8).max(1),
                _ => 8,
            };
            n * elem
        }
        BasicTypeEnum::VectorType(_) => 16,
    }
}

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
        Expr::Index(target, index) => compile_index(mc, target, index),
        Expr::Range(_, _) => Err(LlvmError::Unsupported(
            "range expression outside of indexing context".into(),
        )),
        Expr::MethodCall(receiver, method, args) => compile_method_call(mc, receiver, method, args),
        Expr::Closure(_, _) => Err(LlvmError::Unsupported("closure".into())),
        Expr::Array(elements) => compile_array(mc, elements),
        Expr::TupleExpr(elts) => compile_tuple(mc, elts),
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
            // Emit the bytes as a private global constant (with trailing NUL,
            // but length tracked separately) and materialize a NomString
            // struct value: { data: &bytes, len: <literal len> }.
            let global = mc.builder.build_global_string_ptr(s, "str")
                .map_err(|e| LlvmError::Compilation(e.to_string()))?;
            let data_ptr = global.as_pointer_value();
            let len = mc.context.i64_type().const_int(s.len() as u64, false);
            let nom_str_ty = mc.nom_string_type();
            // Build a constant struct { ptr, i64 }
            let struct_val = nom_str_ty.const_named_struct(&[data_ptr.into(), len.into()]);
            Ok(struct_val.into())
        }
        Literal::None => Ok(mc.context.i8_type().const_zero().into()),
    }
}

fn compile_ident<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    name: &str,
) -> Result<BasicValueEnum<'ctx>, LlvmError> {
    if let Some((ptr, ty)) = mc.named_values.get(name) {
        mc.builder
            .build_load(*ty, *ptr, name)
            .map_err(|e| LlvmError::Compilation(e.to_string()))
    } else if name.contains("::") {
        // Zero-arg enum variant referenced as `E::B` with no parens — e.g.
        // `let e = E::B`. Construct with an empty argument list.
        if let Some(enum_name) = mc.variant_to_enum.get(name).cloned() {
            return compile_enum_variant_ctor(mc, &enum_name, name, &[]);
        }
        Err(LlvmError::Compilation(format!("undefined variable: {}", name)))
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

    // String equality/inequality via runtime helper.
    if is_string_value(mc, &l) && is_string_value(mc, &r) && matches!(op, BinOp::Eq | BinOp::Neq) {
        let a_ptr = materialize_string_ptr(mc, l)?;
        let b_ptr = materialize_string_ptr(mc, r)?;
        let eq_fn = mc
            .functions
            .get("nom_string_eq")
            .copied()
            .or_else(|| mc.module.get_function("nom_string_eq"))
            .ok_or_else(|| LlvmError::Compilation("nom_string_eq runtime fn missing".into()))?;
        let call = mc
            .builder
            .build_call(eq_fn, &[a_ptr.into(), b_ptr.into()], "str_eq")
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
        let result = call
            .try_as_basic_value()
            .left()
            .ok_or_else(|| LlvmError::Compilation("nom_string_eq returned void".into()))?
            .into_int_value();
        // Result is i32: 1 == equal, 0 == not equal.
        let zero = result.get_type().const_zero();
        let cmp = mc
            .builder
            .build_int_compare(
                if matches!(op, BinOp::Eq) {
                    inkwell::IntPredicate::NE // eq path: nom_string_eq != 0 means equal
                } else {
                    inkwell::IntPredicate::EQ // neq path: nom_string_eq == 0 means not equal
                },
                result,
                zero,
                "str_eq_bool",
            )
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
        return Ok(cmp.into());
    }

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
    let fn_name = call.callee.name.as_str();

    // Enum variant construction: `Token::Integer(42)` has a `::`-qualified
    // callee that resolves to a registered enum variant. The postfix parser
    // merged `Enum::Variant` into a single Ident name; we look it up here.
    if fn_name.contains("::") {
        if let Some(enum_name) = mc.variant_to_enum.get(fn_name).cloned() {
            return compile_enum_variant_ctor(mc, &enum_name, fn_name, &call.args);
        }
    }

    // Builtin: `print(s)` / `println(s)` — when the single argument is a
    // string, decompose NomString into (data, len) and call the matching
    // runtime function. Falls through to normal lookup otherwise so users
    // can still define their own `print`/`println` functions.
    if matches!(fn_name, "print" | "println") && call.args.len() == 1 {
        let arg_val = compile_expr(mc, &call.args[0])?;
        if is_string_value(mc, &arg_val) {
            let rt_name = if fn_name == "print" { "nom_print" } else { "nom_println" };
            let rt_fn = mc
                .functions
                .get(rt_name)
                .copied()
                .or_else(|| mc.module.get_function(rt_name))
                .ok_or_else(|| LlvmError::Compilation(format!("{} runtime fn missing", rt_name)))?;
            let (data, len) = extract_string_parts(mc, arg_val)?;
            mc.builder
                .build_call(rt_fn, &[data.into(), len.into()], "print_call")
                .map_err(|e| LlvmError::Compilation(e.to_string()))?;
            return Ok(mc.context.i8_type().const_zero().into());
        }
        // Integer/float/bool shortcuts.
        if arg_val.is_int_value() {
            let iv = arg_val.into_int_value();
            let bw = iv.get_type().get_bit_width();
            if bw == 1 {
                let rt_fn = mc
                    .functions
                    .get("nom_print_bool")
                    .copied()
                    .ok_or_else(|| LlvmError::Compilation("nom_print_bool missing".into()))?;
                // Extend i1 to i8 for the runtime.
                let i8_ty = mc.context.i8_type();
                let ext = mc
                    .builder
                    .build_int_z_extend(iv, i8_ty, "bool_ext")
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;
                mc.builder
                    .build_call(rt_fn, &[ext.into()], "print_bool")
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;
            } else {
                let rt_fn = mc
                    .functions
                    .get("nom_print_int")
                    .copied()
                    .ok_or_else(|| LlvmError::Compilation("nom_print_int missing".into()))?;
                // Ensure i64 width.
                let i64_ty = mc.context.i64_type();
                let as_i64 = if bw < 64 {
                    mc.builder
                        .build_int_s_extend(iv, i64_ty, "int_ext")
                        .map_err(|e| LlvmError::Compilation(e.to_string()))?
                } else {
                    iv
                };
                mc.builder
                    .build_call(rt_fn, &[as_i64.into()], "print_int")
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;
            }
            return Ok(mc.context.i8_type().const_zero().into());
        }
        if arg_val.is_float_value() {
            let rt_fn = mc
                .functions
                .get("nom_print_float")
                .copied()
                .ok_or_else(|| LlvmError::Compilation("nom_print_float missing".into()))?;
            mc.builder
                .build_call(rt_fn, &[arg_val.into_float_value().into()], "print_float")
                .map_err(|e| LlvmError::Compilation(e.to_string()))?;
            return Ok(mc.context.i8_type().const_zero().into());
        }
    }

    // Builtins that take a *const NomString argument: parse_int, parse_float.
    if matches!(fn_name, "parse_int" | "parse_float") && call.args.len() == 1 {
        let rt_name = if fn_name == "parse_int" { "nom_parse_int" } else { "nom_parse_float" };
        let rt_fn = mc
            .functions
            .get(rt_name)
            .copied()
            .or_else(|| mc.module.get_function(rt_name))
            .ok_or_else(|| LlvmError::Compilation(format!("{} runtime fn missing", rt_name)))?;
        let arg_val = compile_expr(mc, &call.args[0])?;
        let str_ptr = materialize_string_ptr(mc, arg_val)?;
        let call_val = mc
            .builder
            .build_call(rt_fn, &[str_ptr.into()], &format!("{}_call", fn_name))
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
        return call_val
            .try_as_basic_value()
            .left()
            .ok_or_else(|| LlvmError::Compilation(format!("{} returned void", rt_name)));
    }

    // Builtin chr(byte: integer) -> text.
    if fn_name == "chr" && call.args.len() == 1 {
        let rt_fn = mc
            .functions
            .get("nom_chr")
            .copied()
            .or_else(|| mc.module.get_function("nom_chr"))
            .ok_or_else(|| LlvmError::Compilation("nom_chr runtime fn missing".into()))?;
        let arg_val = compile_expr(mc, &call.args[0])?;
        // Ensure i64 width for the byte argument.
        let i64_ty = mc.context.i64_type();
        let byte_val = if arg_val.is_int_value() {
            let iv = arg_val.into_int_value();
            if iv.get_type().get_bit_width() < 64 {
                mc.builder
                    .build_int_s_extend(iv, i64_ty, "chr_ext")
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?
            } else {
                iv
            }
        } else {
            return Err(LlvmError::Type("chr argument must be integer".into()));
        };
        let call_val = mc
            .builder
            .build_call(rt_fn, &[byte_val.into()], "chr_call")
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
        return call_val
            .try_as_basic_value()
            .left()
            .ok_or_else(|| LlvmError::Compilation("nom_chr returned void".into()));
    }

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

/// True if this BasicValueEnum is a NomString struct value (`{ i8*, i64 }`).
fn is_string_value<'ctx>(mc: &ModuleCompiler<'ctx>, val: &BasicValueEnum<'ctx>) -> bool {
    if !val.is_struct_value() {
        return false;
    }
    let st = val.into_struct_value().get_type();
    st == mc.nom_string_type()
}

/// Extract the (data_ptr, length_i64) pair from a NomString struct value.
fn extract_string_parts<'ctx>(
    mc: &ModuleCompiler<'ctx>,
    val: BasicValueEnum<'ctx>,
) -> Result<(inkwell::values::PointerValue<'ctx>, inkwell::values::IntValue<'ctx>), LlvmError> {
    let sv = val.into_struct_value();
    let data = mc
        .builder
        .build_extract_value(sv, 0, "str_data")
        .map_err(|e| LlvmError::Compilation(e.to_string()))?
        .into_pointer_value();
    let len = mc
        .builder
        .build_extract_value(sv, 1, "str_len")
        .map_err(|e| LlvmError::Compilation(e.to_string()))?
        .into_int_value();
    Ok((data, len))
}

/// Alloca a NomString slot, store the value into it, and return the pointer.
/// Used when calling runtime helpers that accept `*const NomString`.
fn materialize_string_ptr<'ctx>(
    mc: &ModuleCompiler<'ctx>,
    val: BasicValueEnum<'ctx>,
) -> Result<inkwell::values::PointerValue<'ctx>, LlvmError> {
    let nom_str_ty = mc.nom_string_type();
    let slot = mc
        .builder
        .build_alloca(nom_str_ty, "str_slot")
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    mc.builder
        .build_store(slot, val)
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    Ok(slot)
}

/// If `expr` is an `Expr::Ident` referring to a `list[T]` binding,
/// return (alloca_ptr, element TypeExpr). Used by push/index/for-in to
/// locate the list value without re-loading and by-value shenanigans.
fn resolve_list_ident<'ctx>(
    mc: &ModuleCompiler<'ctx>,
    expr: &Expr,
) -> Option<(inkwell::values::PointerValue<'ctx>, nom_ast::TypeExpr)> {
    let name = if let Expr::Ident(id) = expr { &id.name } else { return None };
    let elem_ty = mc.list_elem_types.get(name)?.clone();
    let (ptr, _ty) = mc.named_values.get(name)?;
    Some((*ptr, elem_ty))
}

fn compile_method_call<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    receiver: &Expr,
    method: &nom_ast::Identifier,
    args: &[Expr],
) -> Result<BasicValueEnum<'ctx>, LlvmError> {
    // `xs.push(v)` / `xs.length()` / `xs.len()` on a list[T] binding.
    if let Some((list_ptr, elem_ty_expr)) = resolve_list_ident(mc, receiver) {
        let elem_llvm_ty = crate::types::resolve_type(mc, &elem_ty_expr)?;
        let i64_ty = mc.context.i64_type();
        let stride = type_store_size(mc, elem_llvm_ty);
        let stride_val = i64_ty.const_int(stride, false);

        match method.name.as_str() {
            "push" => {
                if args.len() != 1 {
                    return Err(LlvmError::Compilation(
                        "list.push expects exactly one argument".into(),
                    ));
                }
                let v = compile_expr(mc, &args[0])?;
                let slot = mc
                    .builder
                    .build_alloca(elem_llvm_ty, "push_arg")
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;
                mc.builder
                    .build_store(slot, v)
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;
                let push_fn = mc
                    .functions
                    .get("nom_list_push")
                    .copied()
                    .ok_or_else(|| LlvmError::Compilation("nom_list_push missing".into()))?;
                mc.builder
                    .build_call(
                        push_fn,
                        &[list_ptr.into(), slot.into(), stride_val.into()],
                        "list_push",
                    )
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;
                return Ok(mc.context.i8_type().const_zero().into());
            }
            "length" | "len" => {
                let len_fn = mc
                    .functions
                    .get("nom_list_len")
                    .copied()
                    .ok_or_else(|| LlvmError::Compilation("nom_list_len missing".into()))?;
                let call = mc
                    .builder
                    .build_call(len_fn, &[list_ptr.into()], "list_len")
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;
                return call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| LlvmError::Compilation("nom_list_len returned void".into()));
            }
            _ => {}
        }
    }

    Err(LlvmError::Unsupported(format!(
        "method call: .{}(..) on receiver",
        method.name
    )))
}

fn compile_index<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    target: &Expr,
    index: &Expr,
) -> Result<BasicValueEnum<'ctx>, LlvmError> {
    // List indexing: `xs[i]` on a `list[T]` binding → nom_list_get + load T.
    if let Some((list_ptr, elem_ty_expr)) = resolve_list_ident(mc, target) {
        // Can't also be a range (`xs[a..b]`) for lists; reject explicitly.
        if matches!(index, Expr::Range(_, _)) {
            return Err(LlvmError::Unsupported("list range slicing".into()));
        }
        let elem_llvm_ty = crate::types::resolve_type(mc, &elem_ty_expr)?;
        let i64_ty = mc.context.i64_type();
        let stride = type_store_size(mc, elem_llvm_ty);
        let stride_val = i64_ty.const_int(stride, false);
        let idx_val = compile_expr(mc, index)?;
        let idx_i = if idx_val.is_int_value() {
            idx_val.into_int_value()
        } else {
            return Err(LlvmError::Type("list index must be integer".into()));
        };
        let get_fn = mc
            .functions
            .get("nom_list_get")
            .copied()
            .ok_or_else(|| LlvmError::Compilation("nom_list_get missing".into()))?;
        let call = mc
            .builder
            .build_call(
                get_fn,
                &[list_ptr.into(), idx_i.into(), stride_val.into()],
                "list_get",
            )
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
        let elem_ptr = call
            .try_as_basic_value()
            .left()
            .ok_or_else(|| LlvmError::Compilation("nom_list_get returned void".into()))?
            .into_pointer_value();
        let loaded = mc
            .builder
            .build_load(elem_llvm_ty, elem_ptr, "list_elem")
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
        return Ok(loaded);
    }
    // First: string slicing — Index(target, Range(lo, hi))
    if let Expr::Range(lo, hi) = index {
        let target_val = compile_expr(mc, target)?;
        if !is_string_value(mc, &target_val) {
            return Err(LlvmError::Unsupported(
                "range indexing only supported on strings".into(),
            ));
        }
        let lo_val = compile_expr(mc, lo)?;
        let hi_val = compile_expr(mc, hi)?;
        let lo_i = if lo_val.is_int_value() {
            lo_val.into_int_value()
        } else {
            return Err(LlvmError::Type("slice bound must be integer".into()));
        };
        let hi_i = if hi_val.is_int_value() {
            hi_val.into_int_value()
        } else {
            return Err(LlvmError::Type("slice bound must be integer".into()));
        };
        let str_ptr = materialize_string_ptr(mc, target_val)?;
        // Call nom_string_slice(str_ptr, lo, hi) -> NomString (struct by value).
        let slice_fn = mc
            .functions
            .get("nom_string_slice")
            .copied()
            .or_else(|| mc.module.get_function("nom_string_slice"))
            .ok_or_else(|| {
                LlvmError::Compilation("nom_string_slice runtime fn missing".into())
            })?;
        let call = mc
            .builder
            .build_call(slice_fn, &[str_ptr.into(), lo_i.into(), hi_i.into()], "str_slice")
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
        return call
            .try_as_basic_value()
            .left()
            .ok_or_else(|| LlvmError::Compilation("nom_string_slice returned void".into()));
    }

    let target_val = compile_expr(mc, target)?;
    // String single-byte indexing: returns an i64 (zero-extended byte).
    if is_string_value(mc, &target_val) {
        let idx_val = compile_expr(mc, index)?;
        let idx_i = if idx_val.is_int_value() {
            idx_val.into_int_value()
        } else {
            return Err(LlvmError::Type("string index must be integer".into()));
        };
        let (data_ptr, _len) = extract_string_parts(mc, target_val)?;
        let i8_ty = mc.context.i8_type();
        let byte_ptr = unsafe {
            mc.builder
                .build_in_bounds_gep(i8_ty, data_ptr, &[idx_i], "str_idx_ptr")
                .map_err(|e| LlvmError::Compilation(e.to_string()))?
        };
        let byte = mc
            .builder
            .build_load(i8_ty, byte_ptr, "str_byte")
            .map_err(|e| LlvmError::Compilation(e.to_string()))?
            .into_int_value();
        let ext = mc
            .builder
            .build_int_z_extend(byte, mc.context.i64_type(), "str_byte_ext")
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
        return Ok(ext.into());
    }

    Err(LlvmError::Unsupported(
        "index expression on non-string target".into(),
    ))
}

fn compile_field_access<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    obj: &Expr,
    field: &nom_ast::Identifier,
) -> Result<BasicValueEnum<'ctx>, LlvmError> {
    // Tuple field access: `pair.0`, `pair.1`, ... — the parser emits a
    // numeric identifier when the source uses a `.<integer>` suffix.
    // We look up the target as a struct-typed value (either an ident
    // whose alloca stores a struct, or any expression yielding a struct)
    // and emit `extractvalue`.
    if let Ok(idx) = field.name.parse::<u32>() {
        // Shortcut path: when the target is an ident whose stored type
        // is a struct type that is NOT a known named struct (i.e. a
        // tuple/anonymous struct), load it and extract.
        if let Expr::Ident(ident) = obj {
            if !mc.value_types.contains_key(&ident.name) {
                if let Some((ptr, ty)) = mc.named_values.get(&ident.name) {
                    if ty.is_struct_type() {
                        let loaded = mc
                            .builder
                            .build_load(*ty, *ptr, &ident.name)
                            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
                        let sv = loaded.into_struct_value();
                        let extracted = mc
                            .builder
                            .build_extract_value(sv, idx, &format!("{}.{}", ident.name, idx))
                            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
                        return Ok(extracted);
                    }
                }
            }
        }
        // Generic path: compile the sub-expression; if it's a struct value,
        // extract by index.
        let val = compile_expr(mc, obj)?;
        if val.is_struct_value() {
            let sv = val.into_struct_value();
            let extracted = mc
                .builder
                .build_extract_value(sv, idx, &format!("tup.{}", idx))
                .map_err(|e| LlvmError::Compilation(e.to_string()))?;
            return Ok(extracted);
        }
        return Err(LlvmError::Type(format!(
            "numeric field access `.{}` requires a tuple/struct value",
            idx
        )));
    }

    // String .length — works on any expression that evaluates to a NomString.
    if field.name == "length" {
        // List-typed binding: call nom_list_len.
        if let Some((list_ptr, _elem_ty)) = resolve_list_ident(mc, obj) {
            let len_fn = mc
                .functions
                .get("nom_list_len")
                .copied()
                .ok_or_else(|| LlvmError::Compilation("nom_list_len missing".into()))?;
            let call = mc
                .builder
                .build_call(len_fn, &[list_ptr.into()], "list_len")
                .map_err(|e| LlvmError::Compilation(e.to_string()))?;
            return call
                .try_as_basic_value()
                .left()
                .ok_or_else(|| LlvmError::Compilation("nom_list_len returned void".into()));
        }
        // Attempt to compile the object as a string first.
        // If it's an Ident whose type is known to be a struct, fall through
        // to the named-struct field-access path below.
        let is_struct_ident = if let Expr::Ident(ident) = obj {
            mc.value_types.contains_key(&ident.name)
        } else {
            false
        };
        if !is_struct_ident {
            let val = compile_expr(mc, obj)?;
            if is_string_value(mc, &val) {
                let (_data, len) = extract_string_parts(mc, val)?;
                return Ok(len.into());
            }
            // Not a string — fall through to struct field access if possible.
            return Err(LlvmError::Unsupported(
                "field access on non-string non-struct value".into(),
            ));
        }
    }

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

/// Lower `Enum::Variant(args)` to a tagged-union struct value.
///
/// Layout (see `enums.rs`): `%Enum = type { i8, [N x i8] }` where N is the
/// max byte size across all variant payloads. Construction:
///
/// 1. `alloca %Enum`
/// 2. `store i8 <disc>, ptr <tag_gep>` — write discriminant.
/// 3. For non-empty payloads: GEP to field 1 (payload byte array), reinterpret
///    as the actual payload type, store the compiled argument values.
/// 4. `load %Enum` — return the complete struct by value so it can be stored
///    into a `let` binding or passed as a function argument.
///
/// Payload shape conventions:
/// - 0 args: nothing to store.
/// - 1 arg: the payload byte array is bitcast to the argument's LLVM type.
/// - N args: the payload is treated as an anonymous tuple struct
///   `{ T0, ..., Tn }`, written one field at a time via `insertvalue` and then
///   stored to the payload GEP.
fn compile_enum_variant_ctor<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    enum_name: &str,
    qualified: &str,
    args: &[Expr],
) -> Result<BasicValueEnum<'ctx>, LlvmError> {
    // Discriminant index = position of the variant in the enum definition.
    let variant_name = qualified
        .rsplit("::")
        .next()
        .ok_or_else(|| LlvmError::Compilation(format!("malformed variant path: {}", qualified)))?;
    let variants = mc
        .enum_variants
        .get(enum_name)
        .ok_or_else(|| LlvmError::Compilation(format!("unknown enum: {}", enum_name)))?;
    let (disc, payload_tys) = variants
        .iter()
        .enumerate()
        .find(|(_, (n, _))| n == variant_name)
        .map(|(i, (_, tys))| (i, tys.clone()))
        .ok_or_else(|| {
            LlvmError::Compilation(format!("unknown variant: {}", qualified))
        })?;

    // Arg count check: error out on mismatch; helps catch parser/codegen
    // bugs early rather than silently miscompiling.
    if args.len() != payload_tys.len() {
        return Err(LlvmError::Compilation(format!(
            "variant {} expects {} args, got {}",
            qualified,
            payload_tys.len(),
            args.len()
        )));
    }

    // Compile arguments first so the builder stays positioned correctly.
    let mut compiled_args = Vec::with_capacity(args.len());
    for arg in args {
        compiled_args.push(compile_expr(mc, arg)?);
    }

    let enum_ty = *mc
        .struct_types
        .get(enum_name)
        .ok_or_else(|| LlvmError::Compilation(format!("enum type not defined: {}", enum_name)))?;
    let i8_ty = mc.context.i8_type();

    let enum_alloca = mc
        .builder
        .build_alloca(enum_ty, &format!("{}_ctor", enum_name))
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;

    // Zero the alloca so uninitialized payload bytes are deterministic. The
    // store-discriminant path only writes the tag, and stray bytes in the
    // `[N x i8]` payload would otherwise show up in reads for variants with
    // smaller payloads.
    mc.builder
        .build_store(enum_alloca, enum_ty.const_zero())
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;

    // Store the tag (field 0).
    let tag_ptr = mc
        .builder
        .build_struct_gep(enum_ty, enum_alloca, 0, "tag_ptr")
        .map_err(|e| LlvmError::Compilation(format!("tag GEP: {}", e)))?;
    mc.builder
        .build_store(tag_ptr, i8_ty.const_int(disc as u64, false))
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;

    // Store the payload (field 1, reinterpreted to the actual payload type).
    if !compiled_args.is_empty() {
        let payload_ptr = mc
            .builder
            .build_struct_gep(enum_ty, enum_alloca, 1, "payload_ptr")
            .map_err(|e| LlvmError::Compilation(format!("payload GEP: {}", e)))?;

        if compiled_args.len() == 1 {
            // Single payload value: bitcast the payload byte slot to the
            // value's type and store directly. In LLVM 15+ opaque pointers,
            // the "bitcast" is a no-op — the pointer carries no element
            // type — so we just reuse `payload_ptr` for the store.
            let v = compiled_args[0];
            mc.builder
                .build_store(payload_ptr, v)
                .map_err(|e| LlvmError::Compilation(e.to_string()))?;
        } else {
            // Multi-field payload: build an anonymous struct value
            // `{ T0, T1, ... }` via insertvalue and store it through the
            // payload pointer.
            let elem_tys: Vec<inkwell::types::BasicTypeEnum<'ctx>> =
                compiled_args.iter().map(|v| v.get_type()).collect();
            let tuple_ty = mc.context.struct_type(&elem_tys, false);
            let mut agg: inkwell::values::BasicValueEnum<'ctx> = tuple_ty.get_undef().into();
            for (i, val) in compiled_args.iter().enumerate() {
                let next = mc
                    .builder
                    .build_insert_value(
                        agg.into_struct_value(),
                        *val,
                        i as u32,
                        &format!("variant_fld{}", i),
                    )
                    .map_err(|e| LlvmError::Compilation(e.to_string()))?;
                agg = match next {
                    inkwell::values::AggregateValueEnum::StructValue(sv) => sv.into(),
                    inkwell::values::AggregateValueEnum::ArrayValue(av) => av.into(),
                };
            }
            mc.builder
                .build_store(payload_ptr, agg)
                .map_err(|e| LlvmError::Compilation(e.to_string()))?;
        }
    }

    // Load the finished enum struct and return it by value.
    let loaded = mc
        .builder
        .build_load(enum_ty, enum_alloca, "enum_val")
        .map_err(|e| LlvmError::Compilation(e.to_string()))?;
    Ok(loaded)
}

/// Lower `(a, b, c)` to an anonymous LLVM struct value `{ T0, T1, T2 }`
/// built up via a chain of `insertvalue` instructions starting from `undef`.
/// Returns the struct **value** (not a pointer); callers store into an
/// alloca when a pointer is required.
fn compile_tuple<'ctx>(
    mc: &mut ModuleCompiler<'ctx>,
    elements: &[Expr],
) -> Result<BasicValueEnum<'ctx>, LlvmError> {
    if elements.is_empty() {
        return Err(LlvmError::Unsupported("empty tuple".into()));
    }

    let mut compiled = Vec::with_capacity(elements.len());
    for elem in elements {
        compiled.push(compile_expr(mc, elem)?);
    }

    // Build the anonymous tuple struct type from the compiled element types.
    let elem_tys: Vec<inkwell::types::BasicTypeEnum<'ctx>> =
        compiled.iter().map(|v| v.get_type()).collect();
    let tuple_ty = mc.context.struct_type(&elem_tys, false);

    // Start from `undef` and insert each field.
    let mut agg: inkwell::values::BasicValueEnum<'ctx> = tuple_ty.get_undef().into();
    for (i, val) in compiled.iter().enumerate() {
        let next = mc
            .builder
            .build_insert_value(agg.into_struct_value(), *val, i as u32, &format!("tup{}", i))
            .map_err(|e| LlvmError::Compilation(e.to_string()))?;
        // build_insert_value returns an AggregateValueEnum; convert back.
        agg = match next {
            inkwell::values::AggregateValueEnum::StructValue(sv) => sv.into(),
            inkwell::values::AggregateValueEnum::ArrayValue(av) => av.into(),
        };
    }
    Ok(agg)
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
            enum_variants: std::collections::HashMap::new(),
            variant_to_enum: std::collections::HashMap::new(),
            list_elem_types: std::collections::HashMap::new(),
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
            enum_variants: std::collections::HashMap::new(),
            variant_to_enum: std::collections::HashMap::new(),
            list_elem_types: std::collections::HashMap::new(),
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
