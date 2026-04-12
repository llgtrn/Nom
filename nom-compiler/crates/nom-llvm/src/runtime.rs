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

    // nom_parse_int(s: *const NomString) -> i64
    let parse_int_type = i64_type.fn_type(&[i8_ptr_type.into()], false);
    let parse_int_fn = mc.module.add_function("nom_parse_int", parse_int_type, None);
    mc.functions.insert("nom_parse_int".into(), parse_int_fn);

    // nom_parse_float(s: *const NomString) -> f64
    let parse_float_type = f64_type.fn_type(&[i8_ptr_type.into()], false);
    let parse_float_fn = mc.module.add_function("nom_parse_float", parse_float_type, None);
    mc.functions.insert("nom_parse_float".into(), parse_float_fn);

    // nom_chr(byte: i64) -> NomString (struct by value)
    let chr_type = nom_str_ty.fn_type(&[i64_type.into()], false);
    let chr_fn = mc.module.add_function("nom_chr", chr_type, None);
    mc.functions.insert("nom_chr".into(), chr_fn);

    // ── List (generic) runtime helpers ──────────────────────────────────────
    // All entry points take an explicit elem_size so a single monomorphic
    // runtime supports every `list[T]` instantiation.
    let nom_list_ty = mc.nom_list_type();

    // nom_list_new(elem_size: i64) -> NomList
    let list_new_type = nom_list_ty.fn_type(&[i64_type.into()], false);
    let list_new_fn = mc.module.add_function("nom_list_new", list_new_type, None);
    mc.functions.insert("nom_list_new".into(), list_new_fn);

    // nom_list_with_capacity(elem_size: i64, cap: i64) -> NomList
    let list_wc_type = nom_list_ty.fn_type(&[i64_type.into(), i64_type.into()], false);
    let list_wc_fn = mc.module.add_function("nom_list_with_capacity", list_wc_type, None);
    mc.functions.insert("nom_list_with_capacity".into(), list_wc_fn);

    // nom_list_push(list: *mut NomList, elem: *const i8, elem_size: i64) -> void
    let list_push_type = void_type.fn_type(
        &[i8_ptr_type.into(), i8_ptr_type.into(), i64_type.into()],
        false,
    );
    let list_push_fn = mc.module.add_function("nom_list_push", list_push_type, None);
    mc.functions.insert("nom_list_push".into(), list_push_fn);

    // nom_list_get(list: *const NomList, idx: i64, elem_size: i64) -> *mut i8
    let list_get_type = i8_ptr_type.fn_type(
        &[i8_ptr_type.into(), i64_type.into(), i64_type.into()],
        false,
    );
    let list_get_fn = mc.module.add_function("nom_list_get", list_get_type, None);
    mc.functions.insert("nom_list_get".into(), list_get_fn);

    // nom_list_len(list: *const NomList) -> i64
    let list_len_type = i64_type.fn_type(&[i8_ptr_type.into()], false);
    let list_len_fn = mc.module.add_function("nom_list_len", list_len_type, None);
    mc.functions.insert("nom_list_len".into(), list_len_fn);

    // nom_list_free_sized(list: *mut NomList, elem_size: i64) -> void
    let list_free_type = void_type.fn_type(&[i8_ptr_type.into(), i64_type.into()], false);
    let list_free_fn = mc.module.add_function("nom_list_free_sized", list_free_type, None);
    mc.functions.insert("nom_list_free_sized".into(), list_free_fn);
}
