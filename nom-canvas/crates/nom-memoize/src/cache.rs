use std::cell::RefCell;
use std::collections::HashMap;
use parking_lot::Mutex;

/// An in-memory key→value byte-blob cache protected by a mutex so it can be
/// shared across threads if needed (e.g. a global cache).  For the common
/// single-threaded memoize pattern use the [`CACHE`] thread-local instead.
pub struct MemoizeCache {
    entries: Mutex<HashMap<u64, Vec<u8>>>,
}

impl MemoizeCache {
    pub fn new() -> Self {
        Self { entries: Mutex::new(HashMap::new()) }
    }

    /// Return a clone of the stored bytes for `key`, or `None`.
    pub fn get(&self, key: u64) -> Option<Vec<u8>> {
        self.entries.lock().get(&key).cloned()
    }

    /// Store `value` under `key`, overwriting any previous entry.
    pub fn insert(&self, key: u64, value: Vec<u8>) {
        self.entries.lock().insert(key, value);
    }

    /// Remove all cached entries.
    pub fn clear(&self) {
        self.entries.lock().clear();
    }
}

thread_local! {
    /// Per-thread memoization cache.  Use [`flush_thread_local`] to clear it.
    pub static CACHE: RefCell<MemoizeCache> = RefCell::new(MemoizeCache::new());
}

/// Clear the calling thread's memoization cache.
pub fn flush_thread_local() {
    CACHE.with(|c| c.borrow().clear());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_get_returns_none() {
        let cache = MemoizeCache::new();
        assert!(cache.get(1).is_none());
    }

    #[test]
    fn insert_then_get_returns_value() {
        let cache = MemoizeCache::new();
        cache.insert(42, vec![1, 2, 3]);
        assert_eq!(cache.get(42), Some(vec![1, 2, 3]));
    }

    #[test]
    fn clear_wipes_all_entries() {
        let cache = MemoizeCache::new();
        cache.insert(1, vec![10]);
        cache.insert(2, vec![20]);
        cache.clear();
        assert!(cache.get(1).is_none());
        assert!(cache.get(2).is_none());
    }

    #[test]
    fn thread_local_round_trip() {
        CACHE.with(|c| {
            c.borrow().clear(); // start clean
            c.borrow().insert(99, vec![7, 8, 9]);
        });
        CACHE.with(|c| {
            assert_eq!(c.borrow().get(99), Some(vec![7, 8, 9]));
        });
        flush_thread_local();
    }

    #[test]
    fn flush_clears_thread_local() {
        CACHE.with(|c| c.borrow().insert(5, vec![5]));
        flush_thread_local();
        CACHE.with(|c| assert!(c.borrow().get(5).is_none()));
    }

    #[test]
    fn independent_thread_locals() {
        // Insert in main thread.
        CACHE.with(|c| c.borrow().insert(100, vec![100]));

        let handle = std::thread::spawn(|| {
            // Spawned thread has its own empty cache.
            CACHE.with(|c| {
                assert!(c.borrow().get(100).is_none(), "spawned thread must not see main-thread entry");
                c.borrow().insert(200, vec![200]);
            });
        });
        handle.join().unwrap();

        // Main thread still has its own entry, not the spawned one.
        CACHE.with(|c| {
            assert_eq!(c.borrow().get(100), Some(vec![100]));
            assert!(c.borrow().get(200).is_none());
        });

        flush_thread_local();
    }
}
