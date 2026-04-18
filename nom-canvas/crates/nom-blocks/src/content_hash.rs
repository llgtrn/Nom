/// FNV-1a 64-bit content hash newtype.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContentHash(pub u64);

impl ContentHash {
    /// Compute the FNV-1a 64-bit hash of `data`.
    pub fn new(data: &str) -> Self {
        let mut hash: u64 = 14_695_981_039_346_656_037;
        for byte in data.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(1_099_511_628_211);
        }
        ContentHash(hash)
    }
}

/// An in-memory store that maps [`ContentHash`] → owned `String`.
pub struct ContentStore {
    entries: Vec<(ContentHash, String)>,
}

impl ContentStore {
    /// Create an empty store.
    pub fn new() -> Self {
        ContentStore {
            entries: Vec::new(),
        }
    }

    /// Insert `content`, returning its hash.
    /// Duplicate entries are allowed; use [`dedup_insert`] if you need deduplication.
    pub fn insert(&mut self, content: &str) -> ContentHash {
        let hash = ContentHash::new(content);
        self.entries.push((hash, content.to_owned()));
        hash
    }

    /// Return the first stored string whose hash matches `hash`, or `None`.
    pub fn get(&self, hash: &ContentHash) -> Option<&str> {
        self.entries
            .iter()
            .find(|(h, _)| h == hash)
            .map(|(_, s)| s.as_str())
    }

    /// Return `true` if any entry with the given hash is present.
    pub fn contains(&self, hash: &ContentHash) -> bool {
        self.entries.iter().any(|(h, _)| h == hash)
    }

    /// Insert `content` only if no entry with the same hash already exists.
    ///
    /// Returns `(hash, true)` when the entry is new, `(hash, false)` when it already existed.
    pub fn dedup_insert(&mut self, content: &str) -> (ContentHash, bool) {
        let hash = ContentHash::new(content);
        if self.contains(&hash) {
            (hash, false)
        } else {
            self.entries.push((hash, content.to_owned()));
            (hash, true)
        }
    }

    /// Total number of entries (including duplicates, if any were inserted via [`insert`]).
    pub fn count(&self) -> usize {
        self.entries.len()
    }
}

impl Default for ContentStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_deterministic() {
        let h1 = ContentHash::new("hello world");
        let h2 = ContentHash::new("hello world");
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_different_inputs() {
        let h1 = ContentHash::new("alpha");
        let h2 = ContentHash::new("beta");
        assert_ne!(h1, h2);
    }

    #[test]
    fn store_insert_get() {
        let mut store = ContentStore::new();
        let hash = store.insert("my content");
        assert_eq!(store.get(&hash), Some("my content"));
    }

    #[test]
    fn store_contains() {
        let mut store = ContentStore::new();
        let hash = store.insert("presence check");
        assert!(store.contains(&hash));
        let absent = ContentHash::new("not inserted");
        assert!(!store.contains(&absent));
    }

    #[test]
    fn dedup_insert_new() {
        let mut store = ContentStore::new();
        let (hash, is_new) = store.dedup_insert("fresh entry");
        assert!(is_new);
        assert_eq!(store.get(&hash), Some("fresh entry"));
        assert_eq!(store.count(), 1);
    }

    #[test]
    fn dedup_insert_existing() {
        let mut store = ContentStore::new();
        let (h1, first) = store.dedup_insert("duplicate");
        let (h2, second) = store.dedup_insert("duplicate");
        assert!(first);
        assert!(!second);
        assert_eq!(h1, h2);
        assert_eq!(store.count(), 1);
    }
}
