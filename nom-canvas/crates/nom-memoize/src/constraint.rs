use parking_lot::Mutex;
use std::marker::PhantomData;

/// A single tracked read: which method was called and the hash of the result.
pub struct Read {
    pub method_name: &'static str,
    pub hash: u64,
}

/// Accumulates the set of reads made through a [`Tracked`](crate::tracked::Tracked) wrapper
/// during one memoized computation, and can later validate that the same reads
/// still produce the same hashes (i.e. the underlying data is unchanged).
pub struct Constraint<T: ?Sized> {
    reads: Mutex<Vec<Read>>,
    _marker: PhantomData<T>,
}

impl<T: ?Sized> Constraint<T> {
    /// Create an empty constraint set.
    pub fn new() -> Self {
        Self { reads: Mutex::new(Vec::new()), _marker: PhantomData }
    }

    /// Record that `method_name` was accessed and produced a value whose hash
    /// is `hash`.
    pub fn record(&self, method_name: &'static str, hash: u64) {
        self.reads.lock().push(Read { method_name, hash });
    }

    /// Returns `true` iff every previously recorded read still hashes to its
    /// stored value according to `new_value_hash_at_method`.
    pub fn validate(&self, new_value_hash_at_method: &dyn Fn(&'static str) -> u64) -> bool {
        let reads = self.reads.lock();
        reads.iter().all(|r| new_value_hash_at_method(r.method_name) == r.hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::fast_hash;

    #[test]
    fn new_validates_true_with_no_reads() {
        let c: Constraint<str> = Constraint::new();
        assert!(c.validate(&|_| 0));
    }

    #[test]
    fn record_then_validate_same_hash_returns_true() {
        let c: Constraint<str> = Constraint::new();
        let h = fast_hash(&"value");
        c.record("field_a", h);
        assert!(c.validate(&|_name| fast_hash(&"value")));
    }

    #[test]
    fn changed_hash_returns_false() {
        let c: Constraint<str> = Constraint::new();
        c.record("field_a", fast_hash(&"original"));
        assert!(!c.validate(&|_name| fast_hash(&"changed")));
    }

    #[test]
    fn multiple_reads_and_composed() {
        let c: Constraint<str> = Constraint::new();
        c.record("field_a", fast_hash(&"a"));
        c.record("field_b", fast_hash(&"b"));
        // Both match → true
        assert!(c.validate(&|name| match name {
            "field_a" => fast_hash(&"a"),
            "field_b" => fast_hash(&"b"),
            _ => 0,
        }));
        // One differs → false
        assert!(!c.validate(&|name| match name {
            "field_a" => fast_hash(&"a"),
            "field_b" => fast_hash(&"CHANGED"),
            _ => 0,
        }));
    }

    #[test]
    fn phantom_data_variance() {
        // Constraint<str> and Constraint<[u8]> are distinct types — compile-time check.
        let _a: Constraint<str> = Constraint::new();
        let _b: Constraint<[u8]> = Constraint::new();
    }
}
