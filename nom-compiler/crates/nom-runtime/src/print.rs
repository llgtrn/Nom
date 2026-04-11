use std::io::Write;
use std::slice;

/// Print a NomString to stdout (no newline).
#[unsafe(no_mangle)]
pub extern "C" fn nom_print(data: *const u8, len: i64) {
    unsafe {
        let bytes = slice::from_raw_parts(data, len as usize);
        let _ = std::io::stdout().write_all(bytes);
        let _ = std::io::stdout().flush();
    }
}

/// Print a NomString to stdout with a newline.
#[unsafe(no_mangle)]
pub extern "C" fn nom_println(data: *const u8, len: i64) {
    unsafe {
        let bytes = slice::from_raw_parts(data, len as usize);
        let _ = std::io::stdout().write_all(bytes);
        let _ = std::io::stdout().write_all(b"\n");
        let _ = std::io::stdout().flush();
    }
}

/// Print an i64 integer to stdout with a newline.
#[unsafe(no_mangle)]
pub extern "C" fn nom_print_int(value: i64) {
    println!("{}", value);
}

/// Print an f64 number to stdout with a newline.
#[unsafe(no_mangle)]
pub extern "C" fn nom_print_float(value: f64) {
    println!("{}", value);
}

/// Print a boolean to stdout with a newline.
#[unsafe(no_mangle)]
pub extern "C" fn nom_print_bool(value: i8) {
    println!("{}", if value != 0 { "true" } else { "false" });
}
