//! Heap-allocated growable list runtime for Nom.
//!
//! Matches the LLVM-level representation `%NomList = type { ptr, i64, i64 }`:
//! a raw element byte buffer, a current element count (`len`), and a
//! capacity measured in elements (`cap`).
//!
//! Element type is implicit at this level — the generated code passes the
//! element size (in bytes) into every entry point, and copies bytes verbatim.
//! This keeps the runtime monomorphic while letting the compiler specialize
//! the IR per instantiation.

use std::alloc::{self, Layout};
use std::ptr;

/// Nom's list representation: a heap-allocated fat pointer.
///
/// Fields are ordered to match the LLVM struct exactly so a function
/// returning `NomList` by value and a Nom-emitted load/store of `%NomList`
/// interoperate without any marshalling.
#[repr(C)]
pub struct NomList {
    pub data: *mut u8,
    pub len: i64,
    pub cap: i64,
}

/// Default initial capacity (in elements) used by `nom_list_new`. Zero-based
/// growth would require an extra branch per push; starting at a small
/// non-zero capacity amortizes allocator traffic for typical workloads.
const INITIAL_CAP: i64 = 4;

fn layout_for(elem_size: i64, cap: i64) -> Option<Layout> {
    if cap <= 0 || elem_size <= 0 {
        return None;
    }
    let bytes = (elem_size as usize).checked_mul(cap as usize)?;
    // Use a conservative 8-byte alignment: sufficient for pointers, i64, f64,
    // NomString ({ptr, i64}), and all current enum/struct layouts emitted by
    // nom-llvm. If higher-alignment scalars are added later this needs to
    // grow to match.
    Layout::from_size_align(bytes, 8).ok()
}

/// Allocate an empty list with a default starting capacity.
#[unsafe(no_mangle)]
pub extern "C" fn nom_list_new(elem_size: i64) -> NomList {
    nom_list_with_capacity(elem_size, INITIAL_CAP)
}

/// Allocate an empty list with a caller-specified starting capacity.
#[unsafe(no_mangle)]
pub extern "C" fn nom_list_with_capacity(elem_size: i64, cap: i64) -> NomList {
    if cap <= 0 {
        return NomList {
            data: ptr::null_mut(),
            len: 0,
            cap: 0,
        };
    }
    let layout = match layout_for(elem_size, cap) {
        Some(l) => l,
        None => {
            return NomList {
                data: ptr::null_mut(),
                len: 0,
                cap: 0,
            };
        }
    };
    let data = unsafe { alloc::alloc(layout) };
    if data.is_null() {
        alloc::handle_alloc_error(layout);
    }
    NomList { data, len: 0, cap }
}

/// Append an element. Doubles capacity when full. `elem` points to exactly
/// `elem_size` bytes to copy verbatim.
#[unsafe(no_mangle)]
pub extern "C" fn nom_list_push(list: *mut NomList, elem: *const u8, elem_size: i64) {
    if list.is_null() || elem.is_null() || elem_size <= 0 {
        return;
    }
    unsafe {
        let l = &mut *list;
        if l.len == l.cap {
            // Grow. Initial allocation (cap=0) jumps to INITIAL_CAP so a
            // caller that built the list manually with {null, 0, 0} still
            // works. Otherwise double.
            let new_cap = if l.cap == 0 { INITIAL_CAP } else { l.cap * 2 };
            let new_layout = match layout_for(elem_size, new_cap) {
                Some(l) => l,
                None => return,
            };
            let new_data = if l.data.is_null() {
                alloc::alloc(new_layout)
            } else {
                let old_layout = layout_for(elem_size, l.cap).unwrap();
                alloc::realloc(l.data, old_layout, new_layout.size())
            };
            if new_data.is_null() {
                alloc::handle_alloc_error(new_layout);
            }
            l.data = new_data;
            l.cap = new_cap;
        }
        let dst = l.data.add((l.len as usize) * (elem_size as usize));
        ptr::copy_nonoverlapping(elem, dst, elem_size as usize);
        l.len += 1;
    }
}

/// Return a pointer to the element at `idx`. Caller loads the value.
/// Returns a null pointer for out-of-range indices.
#[unsafe(no_mangle)]
pub extern "C" fn nom_list_get(list: *const NomList, idx: i64, elem_size: i64) -> *mut u8 {
    if list.is_null() || elem_size <= 0 {
        return ptr::null_mut();
    }
    unsafe {
        let l = &*list;
        if idx < 0 || idx >= l.len || l.data.is_null() {
            return ptr::null_mut();
        }
        l.data.add((idx as usize) * (elem_size as usize))
    }
}

/// Current element count.
#[unsafe(no_mangle)]
pub extern "C" fn nom_list_len(list: *const NomList) -> i64 {
    if list.is_null() {
        return 0;
    }
    unsafe { (*list).len }
}

/// Free the backing allocation. Does not walk element destructors — elements
/// are treated as POD. Callers holding NomStrings or other heap-managed
/// elements are responsible for freeing them before calling this.
#[unsafe(no_mangle)]
pub extern "C" fn nom_list_free(list: *mut NomList) {
    if list.is_null() {
        return;
    }
    unsafe {
        let l = &mut *list;
        if !l.data.is_null() && l.cap > 0 {
            // We do not know elem_size here — reconstruct the layout using
            // the 8-byte alignment invariant and the recorded cap. Because
            // we stored bytes with `layout_for` (size = elem_size * cap,
            // align = 8), the dealloc size is the full byte-size, which we
            // can recover from cap * stride. However, we do not record
            // elem_size in the struct. Callers that want free must supply
            // it; so this function only zeros the metadata. For now we do
            // a minimal dealloc using stride=1; this WILL leak if the
            // caller used larger elements. Prefer calling the matching
            // `nom_list_free_sized` (future work).
            //
            // Practical impact: Nom programs running under LLVM today do
            // not call `nom_list_free` — lists leak at program exit, which
            // is acceptable for the self-hosting lexer. Documenting the
            // limitation here so the successor task can address it.
            l.data = ptr::null_mut();
            l.len = 0;
            l.cap = 0;
        }
    }
}

/// Typed free: caller supplies `elem_size`, which (paired with the recorded
/// `cap`) reconstructs the original `Layout` used at allocation time.
/// Prefer this over `nom_list_free` when the element size is known.
#[unsafe(no_mangle)]
pub extern "C" fn nom_list_free_sized(list: *mut NomList, elem_size: i64) {
    if list.is_null() || elem_size <= 0 {
        return;
    }
    unsafe {
        let l = &mut *list;
        if !l.data.is_null() && l.cap > 0 {
            if let Some(layout) = layout_for(elem_size, l.cap) {
                alloc::dealloc(l.data, layout);
            }
            l.data = ptr::null_mut();
            l.len = 0;
            l.cap = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_list_is_empty_with_capacity() {
        let l = nom_list_new(8);
        assert_eq!(l.len, 0);
        assert!(l.cap >= 1, "expected non-zero capacity, got {}", l.cap);
        assert!(!l.data.is_null(), "expected non-null backing buffer");
        let mut l = l;
        nom_list_free_sized(&mut l as *mut _, 8);
    }

    #[test]
    fn push_and_get_i64_roundtrip() {
        let mut l = nom_list_new(8);
        let values: [i64; 3] = [10, 20, 30];
        for v in values.iter() {
            let ptr = v as *const i64 as *const u8;
            nom_list_push(&mut l as *mut _, ptr, 8);
        }
        assert_eq!(nom_list_len(&l as *const _), 3);
        for (i, want) in values.iter().enumerate() {
            let got_ptr = nom_list_get(&l as *const _, i as i64, 8);
            assert!(!got_ptr.is_null());
            let got = unsafe { *(got_ptr as *const i64) };
            assert_eq!(got, *want, "mismatch at idx {}", i);
        }
        nom_list_free_sized(&mut l as *mut _, 8);
    }

    #[test]
    fn push_triggers_growth_beyond_initial_cap() {
        let mut l = nom_list_new(8);
        // Push more than INITIAL_CAP to force at least one realloc.
        let n = (INITIAL_CAP as usize) * 3 + 1;
        for i in 0..n {
            let v = i as i64;
            let p = &v as *const i64 as *const u8;
            nom_list_push(&mut l as *mut _, p, 8);
        }
        assert_eq!(nom_list_len(&l as *const _), n as i64);
        for i in 0..n {
            let got_ptr = nom_list_get(&l as *const _, i as i64, 8);
            let got = unsafe { *(got_ptr as *const i64) };
            assert_eq!(got, i as i64);
        }
        assert!(l.cap >= n as i64);
        nom_list_free_sized(&mut l as *mut _, 8);
    }

    #[test]
    fn get_out_of_range_returns_null() {
        let l = nom_list_new(8);
        let p = nom_list_get(&l as *const _, 0, 8);
        assert!(p.is_null(), "empty list idx 0 must return null");
        let mut l = l;
        let v: i64 = 42;
        nom_list_push(&mut l as *mut _, &v as *const i64 as *const u8, 8);
        let p_neg = nom_list_get(&l as *const _, -1, 8);
        assert!(p_neg.is_null(), "negative idx must return null");
        let p_ov = nom_list_get(&l as *const _, 1, 8);
        assert!(p_ov.is_null(), "idx == len must return null");
        nom_list_free_sized(&mut l as *mut _, 8);
    }

    #[test]
    fn len_of_null_is_zero() {
        assert_eq!(nom_list_len(std::ptr::null()), 0);
    }

    #[test]
    fn with_capacity_zero_returns_null_backing() {
        let l = nom_list_with_capacity(8, 0);
        assert!(l.data.is_null());
        assert_eq!(l.len, 0);
        assert_eq!(l.cap, 0);
    }

    #[test]
    fn push_into_zero_cap_grows_to_initial() {
        let mut l = NomList {
            data: std::ptr::null_mut(),
            len: 0,
            cap: 0,
        };
        let v: i64 = 7;
        nom_list_push(&mut l as *mut _, &v as *const i64 as *const u8, 8);
        assert_eq!(nom_list_len(&l as *const _), 1);
        assert!(l.cap >= 1);
        let p = nom_list_get(&l as *const _, 0, 8);
        let got = unsafe { *(p as *const i64) };
        assert_eq!(got, 7);
        nom_list_free_sized(&mut l as *mut _, 8);
    }
}
