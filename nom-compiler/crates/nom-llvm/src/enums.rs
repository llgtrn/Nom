use crate::context::ModuleCompiler;
use nom_ast::EnumDef;

pub fn compile_enum(mc: &mut ModuleCompiler, enum_def: &EnumDef) -> Result<(), crate::LlvmError> {
    let name = &enum_def.name.name;
    let i8_type = mc.context.i8_type();

    let mut max_payload_size: u32 = 0;
    for variant in &enum_def.variants {
        let mut variant_size: u32 = 0;
        for field_ty in &variant.fields {
            let llvm_ty = crate::types::resolve_type(mc, field_ty)?;
            let field_size = match llvm_ty {
                inkwell::types::BasicTypeEnum::FloatType(_) => 8,
                inkwell::types::BasicTypeEnum::IntType(t) => {
                    (t.get_bit_width() as u32 + 7) / 8
                }
                inkwell::types::BasicTypeEnum::PointerType(_) => 8,
                inkwell::types::BasicTypeEnum::StructType(t) => t.count_fields() * 8,
                _ => 8,
            };
            variant_size += field_size;
        }
        if variant_size > max_payload_size {
            max_payload_size = variant_size;
        }
    }

    let payload_type = i8_type.array_type(max_payload_size.max(1));
    let enum_type = mc.context.opaque_struct_type(name);
    enum_type.set_body(&[i8_type.into(), payload_type.into()], false);
    mc.struct_types.insert(name.clone(), enum_type);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::NomCompiler;
    use nom_ast::*;

    #[test]
    fn compiles_shape_enum() {
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
        };

        let enum_def = EnumDef {
            name: Identifier::new("Shape", Span::default()),
            variants: vec![
                EnumVariant {
                    name: Identifier::new("Circle", Span::default()),
                    fields: vec![TypeExpr::Named(Identifier::new("number", Span::default()))],
                },
                EnumVariant {
                    name: Identifier::new("Rectangle", Span::default()),
                    fields: vec![
                        TypeExpr::Named(Identifier::new("number", Span::default())),
                        TypeExpr::Named(Identifier::new("number", Span::default())),
                    ],
                },
                EnumVariant {
                    name: Identifier::new("None", Span::default()),
                    fields: vec![],
                },
            ],
            is_pub: false,
            span: Span::default(),
        };

        compile_enum(&mut mc, &enum_def).unwrap();

        assert!(mc.struct_types.contains_key("Shape"));
        let shape_ty = mc.struct_types.get("Shape").unwrap();
        // tag (i8) + payload array = 2 fields
        assert_eq!(shape_ty.count_fields(), 2, "Shape should have 2 fields (tag + payload)");
    }
}
