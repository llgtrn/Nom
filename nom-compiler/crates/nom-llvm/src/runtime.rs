use crate::context::ModuleCompiler;
use inkwell::AddressSpace;

pub fn declare_runtime_functions(mc: &mut ModuleCompiler) {
    let i64_type = mc.context.i64_type();
    let i8_ptr_type = mc.context.ptr_type(AddressSpace::default());
    let void_type = mc.context.void_type();

    let print_type = void_type.fn_type(&[i8_ptr_type.into(), i64_type.into()], false);
    let print_fn = mc.module.add_function("nom_print", print_type, None);
    mc.functions.insert("nom_print".into(), print_fn);

    let alloc_type = i8_ptr_type.fn_type(&[i64_type.into()], false);
    let alloc_fn = mc.module.add_function("nom_alloc", alloc_type, None);
    mc.functions.insert("nom_alloc".into(), alloc_fn);
}
