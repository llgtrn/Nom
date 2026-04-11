use std::alloc::{Layout, alloc, dealloc};

/// Allocate `size` bytes of memory. Returns a pointer.
#[unsafe(no_mangle)]
pub extern "C" fn nom_alloc(size: i64) -> *mut u8 {
    if size <= 0 {
        return std::ptr::null_mut();
    }
    unsafe {
        let layout = Layout::from_size_align(size as usize, 8).unwrap();
        alloc(layout)
    }
}

/// Free memory allocated by nom_alloc.
#[unsafe(no_mangle)]
pub extern "C" fn nom_free(ptr: *mut u8, size: i64) {
    if ptr.is_null() || size <= 0 {
        return;
    }
    unsafe {
        let layout = Layout::from_size_align(size as usize, 8).unwrap();
        dealloc(ptr, layout);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_and_free() {
        let ptr = nom_alloc(1024);
        assert!(!ptr.is_null());
        nom_free(ptr, 1024);
    }

    #[test]
    fn zero_alloc_returns_null() {
        let ptr = nom_alloc(0);
        assert!(ptr.is_null());
    }
}
