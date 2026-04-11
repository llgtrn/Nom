use std::slice;

/// A Nom string: pointer + length (no null terminator).
#[repr(C)]
pub struct NomString {
    pub data: *const u8,
    pub len: i64,
}

/// Create a NomString from a pointer and length.
#[unsafe(no_mangle)]
pub extern "C" fn nom_string_new(data: *const u8, len: i64) -> NomString {
    NomString { data, len }
}

/// Get the length of a NomString.
#[unsafe(no_mangle)]
pub extern "C" fn nom_string_len(s: *const NomString) -> i64 {
    unsafe { (*s).len }
}

/// Concatenate two NomStrings. Returns a new heap-allocated NomString.
#[unsafe(no_mangle)]
pub extern "C" fn nom_string_concat(a: *const NomString, b: *const NomString) -> NomString {
    unsafe {
        let a_slice = slice::from_raw_parts((*a).data, (*a).len as usize);
        let b_slice = slice::from_raw_parts((*b).data, (*b).len as usize);
        let mut result = Vec::with_capacity(a_slice.len() + b_slice.len());
        result.extend_from_slice(a_slice);
        result.extend_from_slice(b_slice);
        let len = result.len() as i64;
        let ptr = result.as_ptr();
        std::mem::forget(result); // Leak to C — caller must free
        NomString { data: ptr, len }
    }
}

/// Compare two NomStrings for equality. Returns 1 if equal, 0 if not.
#[unsafe(no_mangle)]
pub extern "C" fn nom_string_eq(a: *const NomString, b: *const NomString) -> i32 {
    unsafe {
        let a_slice = slice::from_raw_parts((*a).data, (*a).len as usize);
        let b_slice = slice::from_raw_parts((*b).data, (*b).len as usize);
        if a_slice == b_slice { 1 } else { 0 }
    }
}

/// Free a heap-allocated NomString.
#[unsafe(no_mangle)]
pub extern "C" fn nom_string_free(s: NomString) {
    if !s.data.is_null() && s.len > 0 {
        unsafe {
            let _ = Vec::from_raw_parts(s.data as *mut u8, s.len as usize, s.len as usize);
            // Vec drops and frees
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_new_and_len() {
        let data = b"hello";
        let s = nom_string_new(data.as_ptr(), 5);
        assert_eq!(nom_string_len(&s as *const _), 5);
    }

    #[test]
    fn string_concat() {
        let a_data = b"hello ";
        let b_data = b"world";
        let a = NomString { data: a_data.as_ptr(), len: 6 };
        let b = NomString { data: b_data.as_ptr(), len: 5 };
        let result = nom_string_concat(&a as *const _, &b as *const _);
        assert_eq!(result.len, 11);
        let result_slice = unsafe { std::slice::from_raw_parts(result.data, result.len as usize) };
        assert_eq!(result_slice, b"hello world");
        nom_string_free(result);
    }

    #[test]
    fn string_equality() {
        let a = NomString { data: b"abc".as_ptr(), len: 3 };
        let b = NomString { data: b"abc".as_ptr(), len: 3 };
        let c = NomString { data: b"xyz".as_ptr(), len: 3 };
        assert_eq!(nom_string_eq(&a as *const _, &b as *const _), 1);
        assert_eq!(nom_string_eq(&a as *const _, &c as *const _), 0);
    }
}
