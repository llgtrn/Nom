use super::string::NomString;
use std::fs;
use std::slice;

/// Read an entire file into a NomString.
/// Returns a NomString with len=-1 on error.
#[unsafe(no_mangle)]
pub extern "C" fn nom_read_file(path_data: *const u8, path_len: i64) -> NomString {
    unsafe {
        let path_bytes = slice::from_raw_parts(path_data, path_len as usize);
        let path = match std::str::from_utf8(path_bytes) {
            Ok(s) => s,
            Err(_) => {
                return NomString {
                    data: std::ptr::null(),
                    len: -1,
                };
            }
        };
        match fs::read(path) {
            Ok(contents) => {
                let len = contents.len() as i64;
                let ptr = contents.as_ptr();
                std::mem::forget(contents);
                NomString { data: ptr, len }
            }
            Err(_) => NomString {
                data: std::ptr::null(),
                len: -1,
            },
        }
    }
}

/// Write a NomString to a file. Returns 0 on success, -1 on error.
#[unsafe(no_mangle)]
pub extern "C" fn nom_write_file(
    path_data: *const u8,
    path_len: i64,
    content_data: *const u8,
    content_len: i64,
) -> i32 {
    unsafe {
        let path_bytes = slice::from_raw_parts(path_data, path_len as usize);
        let path = match std::str::from_utf8(path_bytes) {
            Ok(s) => s,
            Err(_) => return -1,
        };
        let content = slice::from_raw_parts(content_data, content_len as usize);
        match fs::write(path, content) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }
}

/// Panic with an error message. Used for runtime errors.
#[unsafe(no_mangle)]
pub extern "C" fn nom_panic(msg_data: *const u8, msg_len: i64) -> ! {
    unsafe {
        let bytes = slice::from_raw_parts(msg_data, msg_len as usize);
        let msg = std::str::from_utf8_unchecked(bytes);
        panic!("Nom runtime error: {}", msg);
    }
}
