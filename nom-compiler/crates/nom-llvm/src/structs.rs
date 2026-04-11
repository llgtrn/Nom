use crate::context::ModuleCompiler;
use nom_ast::StructDef;

pub fn compile_struct(mc: &mut ModuleCompiler, struct_def: &StructDef) -> Result<(), crate::LlvmError> {
    let name = &struct_def.name.name;
    let mut field_types = Vec::new();
    for field in &struct_def.fields {
        let llvm_ty = crate::types::resolve_type(mc, &field.type_ann)?;
        field_types.push(llvm_ty);
    }
    let struct_type = mc.context.opaque_struct_type(name);
    struct_type.set_body(&field_types, false);
    mc.struct_types.insert(name.clone(), struct_type);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::NomCompiler;
    use nom_ast::*;
    use nom_planner::CompositionPlan;

    #[test]
    fn compiles_point_struct() {
        let compiler = NomCompiler::new();
        let plan = CompositionPlan {
            source_path: Some("test.nom".into()),
            flows: vec![],
            nomiz: "{}".into(),
        };
        let output = compiler.compile_plan(&plan);
        // We need to test compile_struct directly
        drop(output);

        // Build a ModuleCompiler manually
        let module = compiler.context.create_module("test");
        let builder = compiler.context.create_builder();
        let mut mc = ModuleCompiler {
            context: &compiler.context,
            module,
            builder,
            named_values: std::collections::HashMap::new(),
            struct_types: std::collections::HashMap::new(),
            functions: std::collections::HashMap::new(),
        };

        let struct_def = StructDef {
            name: Identifier::new("Point", Span::default()),
            fields: vec![
                StructField {
                    name: Identifier::new("x", Span::default()),
                    type_ann: TypeExpr::Named(Identifier::new("number", Span::default())),
                    is_pub: false,
                },
                StructField {
                    name: Identifier::new("y", Span::default()),
                    type_ann: TypeExpr::Named(Identifier::new("number", Span::default())),
                    is_pub: false,
                },
            ],
            is_pub: false,
            span: Span::default(),
        };

        compile_struct(&mut mc, &struct_def).unwrap();

        assert!(mc.struct_types.contains_key("Point"));
        let point_ty = mc.struct_types.get("Point").unwrap();
        assert_eq!(point_ty.count_fields(), 2, "Point should have 2 fields");
    }
}
