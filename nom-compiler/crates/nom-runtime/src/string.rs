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

/// Slice a NomString: returns a new heap-allocated NomString containing bytes
/// `s[lo..hi]` (half-open). Bounds are clamped to `[0, s.len]`; if `lo >= hi`
/// after clamping, returns an empty NomString with a null data pointer.
#[unsafe(no_mangle)]
pub extern "C" fn nom_string_slice(s: *const NomString, lo: i64, hi: i64) -> NomString {
    unsafe {
        let len = (*s).len;
        let lo_c = lo.max(0).min(len);
        let hi_c = hi.max(lo_c).min(len);
        let new_len = hi_c - lo_c;
        if new_len <= 0 {
            return NomString { data: std::ptr::null(), len: 0 };
        }
        let src = slice::from_raw_parts((*s).data.add(lo_c as usize), new_len as usize);
        let mut buf = Vec::with_capacity(new_len as usize);
        buf.extend_from_slice(src);
        let ptr = buf.as_ptr();
        std::mem::forget(buf);
        NomString { data: ptr, len: new_len }
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
    fn string_slice_basic() {
        let data = b"hello world";
        let s = NomString { data: data.as_ptr(), len: 11 };
        let sub = nom_string_slice(&s as *const _, 6, 11);
        assert_eq!(sub.len, 5);
        let bytes = unsafe { std::slice::from_raw_parts(sub.data, sub.len as usize) };
        assert_eq!(bytes, b"world");
        nom_string_free(sub);
    }

    #[test]
    fn string_slice_clamps_out_of_range() {
        let data = b"abc";
        let s = NomString { data: data.as_ptr(), len: 3 };
        let sub = nom_string_slice(&s as *const _, -2, 100);
        assert_eq!(sub.len, 3);
        let bytes = unsafe { std::slice::from_raw_parts(sub.data, sub.len as usize) };
        assert_eq!(bytes, b"abc");
        nom_string_free(sub);
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
