use crate::context::ModuleCompiler;
use inkwell::AddressSpace;

pub fn declare_runtime_functions(mc: &mut ModuleCompiler) {
    let i8_type = mc.context.i8_type();
    let i32_type = mc.context.i32_type();
    let i64_type = mc.context.i64_type();
    let f64_type = mc.context.f64_type();
    let i8_ptr_type = mc.context.ptr_type(AddressSpace::default());
    let void_type = mc.context.void_type();

    // nom_print(data: *const i8, len: i64) -> void
    let print_type = void_type.fn_type(&[i8_ptr_type.into(), i64_type.into()], false);
    let print_fn = mc.module.add_function("nom_print", print_type, None);
    mc.functions.insert("nom_print".into(), print_fn);

    // nom_println(data: *const i8, len: i64) -> void
    let println_type = void_type.fn_type(&[i8_ptr_type.into(), i64_type.into()], false);
    let println_fn = mc.module.add_function("nom_println", println_type, None);
    mc.functions.insert("nom_println".into(), println_fn);

    // nom_print_int(value: i64) -> void
    let print_int_type = void_type.fn_type(&[i64_type.into()], false);
    let print_int_fn = mc.module.add_function("nom_print_int", print_int_type, None);
    mc.functions.insert("nom_print_int".into(), print_int_fn);

    // nom_print_float(value: f64) -> void
    let print_float_type = void_type.fn_type(&[f64_type.into()], false);
    let print_float_fn = mc.module.add_function("nom_print_float", print_float_type, None);
    mc.functions.insert("nom_print_float".into(), print_float_fn);

    // nom_print_bool(value: i8) -> void
    let print_bool_type = void_type.fn_type(&[i8_type.into()], false);
    let print_bool_fn = mc.module.add_function("nom_print_bool", print_bool_type, None);
    mc.functions.insert("nom_print_bool".into(), print_bool_fn);

    // nom_alloc(size: i64) -> *mut i8
    let alloc_type = i8_ptr_type.fn_type(&[i64_type.into()], false);
    let alloc_fn = mc.module.add_function("nom_alloc", alloc_type, None);
    mc.functions.insert("nom_alloc".into(), alloc_fn);

    // nom_free(ptr: *mut i8, size: i64) -> void
    let free_type = void_type.fn_type(&[i8_ptr_type.into(), i64_type.into()], false);
    let free_fn = mc.module.add_function("nom_free", free_type, None);
    mc.functions.insert("nom_free".into(), free_fn);

    // nom_string_concat(a: *const NomString, b: *const NomString) -> NomString
    // Returns the NomString struct by value.
    let nom_str_ty = mc.nom_string_type();
    let string_concat_type = nom_str_ty.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
    let string_concat_fn = mc.module.add_function("nom_string_concat", string_concat_type, None);
    mc.functions.insert("nom_string_concat".into(), string_concat_fn);

    // nom_string_eq(a: *const NomString, b: *const NomString) -> i32
    let string_eq_type = i32_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
    let string_eq_fn = mc.module.add_function("nom_string_eq", string_eq_type, None);
    mc.functions.insert("nom_string_eq".into(), string_eq_fn);

    // nom_string_slice(s: *const NomString, lo: i64, hi: i64) -> NomString
    let string_slice_type =
        nom_str_ty.fn_type(&[i8_ptr_type.into(), i64_type.into(), i64_type.into()], false);
    let string_slice_fn = mc.module.add_function("nom_string_slice", string_slice_type, None);
    mc.functions.insert("nom_string_slice".into(), string_slice_fn);

    // nom_read_file(path: *const i8, path_len: i64) -> NomString (ptr)
    let read_file_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i64_type.into()], false);
    let read_file_fn = mc.module.add_function("nom_read_file", read_file_type, None);
    mc.functions.insert("nom_read_file".into(), read_file_fn);

    // nom_write_file(path: *const i8, path_len: i64, data: *const i8, data_len: i64) -> i32
    let write_file_type = i32_type.fn_type(
        &[i8_ptr_type.into(), i64_type.into(), i8_ptr_type.into(), i64_type.into()],
        false,
    );
    let write_file_fn = mc.module.add_function("nom_write_file", write_file_type, None);
    mc.functions.insert("nom_write_file".into(), write_file_fn);

    // nom_panic(msg: *const i8, msg_len: i64) -> void
    let panic_type = void_type.fn_type(&[i8_ptr_type.into(), i64_type.into()], false);
    let panic_fn = mc.module.add_function("nom_panic", panic_type, None);
    mc.functions.insert("nom_panic".into(), panic_fn);
}
