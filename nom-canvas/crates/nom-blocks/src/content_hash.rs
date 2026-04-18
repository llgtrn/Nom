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

#[cfg(test)]
mod content_hash_integration_tests {
    use super::*;
    use std::collections::HashMap;

    /// Hashing the same string twice produces identical ContentHash values.
    #[test]
    fn same_string_same_hash() {
        let h1 = ContentHash::new("stable content");
        let h2 = ContentHash::new("stable content");
        assert_eq!(h1, h2);
    }

    /// Different strings produce different hashes (FNV-1a collision resistance).
    #[test]
    fn different_strings_different_hashes() {
        let h1 = ContentHash::new("foo");
        let h2 = ContentHash::new("bar");
        assert_ne!(h1, h2);
    }

    /// dedup_insert with the same content twice returns the same hash and is_new=false the second time.
    #[test]
    fn dedup_insert_same_content_same_hash() {
        let mut store = ContentStore::new();
        let (h1, new1) = store.dedup_insert("shared content");
        let (h2, new2) = store.dedup_insert("shared content");
        assert!(new1, "first insert must be new");
        assert!(!new2, "second insert must not be new");
        assert_eq!(h1, h2);
        assert_eq!(store.count(), 1);
    }

    /// Inserting different content via dedup_insert creates separate entries.
    #[test]
    fn dedup_insert_different_content_separate_entries() {
        let mut store = ContentStore::new();
        let (ha, na) = store.dedup_insert("entry-alpha");
        let (hb, nb) = store.dedup_insert("entry-beta");
        assert!(na);
        assert!(nb);
        assert_ne!(ha, hb);
        assert_eq!(store.count(), 2);
    }

    /// Retrieve content by hash after insertion.
    #[test]
    fn store_retrieval_by_hash() {
        let mut store = ContentStore::new();
        let hash = store.insert("retrievable");
        let got = store.get(&hash);
        assert_eq!(got, Some("retrievable"));
    }

    /// ContentHash implements Debug and the inner value is exposed via the tuple struct.
    #[test]
    fn content_hash_debug_format() {
        let h = ContentHash::new("test-debug");
        let dbg = format!("{:?}", h);
        // Debug output should contain the discriminant name and a numeric value.
        assert!(dbg.contains("ContentHash"), "debug output must contain type name");
    }

    /// ContentHash implements Hash+Eq so it works as a HashMap key.
    #[test]
    fn content_hash_as_hashmap_key() {
        let mut map: HashMap<ContentHash, &str> = HashMap::new();
        let h1 = ContentHash::new("key-one");
        let h2 = ContentHash::new("key-two");
        map.insert(h1, "value-one");
        map.insert(h2, "value-two");
        assert_eq!(map[&h1], "value-one");
        assert_eq!(map[&h2], "value-two");
    }

    /// Empty string hash is stable across calls.
    #[test]
    fn empty_string_hash_is_stable() {
        let h1 = ContentHash::new("");
        let h2 = ContentHash::new("");
        assert_eq!(h1, h2);
        // FNV-1a offset basis for empty input is the offset basis itself.
        assert_eq!(h1.0, 14_695_981_039_346_656_037u64);
    }
}
