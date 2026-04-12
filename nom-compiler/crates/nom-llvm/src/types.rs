use crate::context::ModuleCompiler;
use inkwell::types::BasicTypeEnum;
use nom_ast::TypeExpr;

pub fn resolve_type<'ctx>(
    mc: &ModuleCompiler<'ctx>,
    type_expr: &TypeExpr,
) -> Result<BasicTypeEnum<'ctx>, crate::LlvmError> {
    match type_expr {
        TypeExpr::Named(ident) => resolve_type_name(mc, &ident.name),
        TypeExpr::Generic(_ident, _args) => {
            Ok(mc.context.ptr_type(inkwell::AddressSpace::default()).into())
        }
        TypeExpr::Unit => Ok(mc.context.i8_type().into()),
        TypeExpr::Tuple(_) => Ok(mc.context.ptr_type(inkwell::AddressSpace::default()).into()),
        TypeExpr::Ref { .. } => Ok(mc.context.ptr_type(inkwell::AddressSpace::default()).into()),
        TypeExpr::Function { .. } => Ok(mc.context.ptr_type(inkwell::AddressSpace::default()).into()),
    }
}

pub fn resolve_type_name<'ctx>(
    mc: &ModuleCompiler<'ctx>,
    name: &str,
) -> Result<BasicTypeEnum<'ctx>, crate::LlvmError> {
    match name {
        "number" | "f64" | "float" | "real" => Ok(mc.context.f64_type().into()),
        "integer" | "i64" | "int" => Ok(mc.context.i64_type().into()),
        "i32" => Ok(mc.context.i32_type().into()),
        "bool" | "yes" | "no" => Ok(mc.context.bool_type().into()),
        "text" | "string" | "String" => Ok(mc.nom_string_type().into()),
        "bytes" => Ok(mc.context.ptr_type(inkwell::AddressSpace::default()).into()),
        _ => {
            if let Some(struct_ty) = mc.struct_types.get(name) {
                Ok((*struct_ty).into())
            } else {
                Err(crate::LlvmError::Type(format!("unknown type: {}", name)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::NomCompiler;
    use nom_planner::CompositionPlan;

    fn make_mc(compiler: &NomCompiler) -> ModuleCompiler<'_> {
        let plan = CompositionPlan {
            source_path: Some("test.nom".into()),
            flows: vec![],
            nomiz: "{}".into(),
        };
        let module_name = plan.source_path.as_deref().unwrap_or("test");
        let ctx = &compiler.context;
        let module = ctx.create_module(module_name);
        let builder = ctx.create_builder();
        ModuleCompiler {
            context: ctx,
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
    fn number_maps_to_f64() {
        let compiler = NomCompiler::new();
        let mc = make_mc(&compiler);
        let ty = resolve_type_name(&mc, "number").unwrap();
        assert!(ty.is_float_type(), "expected f64 type");
    }

    #[test]
    fn bool_maps_to_i1() {
        let compiler = NomCompiler::new();
        let mc = make_mc(&compiler);
        let ty = resolve_type_name(&mc, "bool").unwrap();
        assert!(ty.is_int_type(), "expected int type for bool");
        if let BasicTypeEnum::IntType(int_ty) = ty {
            assert_eq!(int_ty.get_bit_width(), 1, "bool should be i1");
        }
    }
}
