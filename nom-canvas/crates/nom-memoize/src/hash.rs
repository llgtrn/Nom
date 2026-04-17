use std::hash::{DefaultHasher, Hash, Hasher};

/// Compute a fast 64-bit hash of any `Hash`-able value.
pub fn fast_hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_input_same_hash() {
        assert_eq!(fast_hash(&"hello"), fast_hash(&"hello"));
        assert_eq!(fast_hash(&42u64), fast_hash(&42u64));
    }

    #[test]
    fn different_inputs_different_hashes() {
        assert_ne!(fast_hash(&"hello"), fast_hash(&"world"));
        assert_ne!(fast_hash(&1u64), fast_hash(&2u64));
    }
}
