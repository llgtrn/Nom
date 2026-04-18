use std::collections::HashMap;

/// FNV-1a based content hash, same pattern as AppManifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContentHash {
    value: u64,
}

impl ContentHash {
    /// Compute FNV-1a hash from raw bytes.
    pub fn from_bytes(data: &[u8]) -> ContentHash {
        let mut hash: u64 = 14695981039346656037u64;
        for &byte in data {
            hash = (hash ^ byte as u64).wrapping_mul(1099511628211u64);
        }
        ContentHash { value: hash }
    }

    /// Compute FNV-1a hash from a string slice.
    pub fn from_str(s: &str) -> ContentHash {
        Self::from_bytes(s.as_bytes())
    }

    /// Return the raw u64 value.
    pub fn as_u64(&self) -> u64 {
        self.value
    }

    /// Return the first 8 hex characters of the 16-character hex representation.
    pub fn hex_short(&self) -> String {
        let full = format!("{:016x}", self.value);
        full[..8].to_string()
    }
}

/// A content-addressed store mapping `ContentHash` keys to cloneable values.
pub struct ContentAddressStore<V: Clone> {
    entries: HashMap<u64, V>,
}

impl<V: Clone> ContentAddressStore<V> {
    pub fn new() -> Self {
        ContentAddressStore {
            entries: HashMap::new(),
        }
    }

    pub fn insert(&mut self, hash: ContentHash, value: V) {
        self.entries.insert(hash.value, value);
    }

    pub fn get(&self, hash: &ContentHash) -> Option<&V> {
        self.entries.get(&hash.value)
    }

    pub fn contains(&self, hash: &ContentHash) -> bool {
        self.entries.contains_key(&hash.value)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

impl<V: Clone> Default for ContentAddressStore<V> {
    fn default() -> Self {
        Self::new()
    }
}

/// A shareable specialization entry that tracks how many apps reference it.
#[derive(Debug, Clone)]
pub struct ShareEntry {
    pub hash: ContentHash,
    pub app_id: String,
    pub specialization_key: String,
    pub ref_count: u32,
}

impl ShareEntry {
    pub fn new(hash: ContentHash, app_id: String, specialization_key: String) -> Self {
        ShareEntry {
            hash,
            app_id,
            specialization_key,
            ref_count: 1,
        }
    }

    /// Increment the reference count by one.
    pub fn increment_ref(&mut self) {
        self.ref_count += 1;
    }

    /// Returns `true` if more than one app references this entry.
    pub fn is_shared(&self) -> bool {
        self.ref_count > 1
    }
}

/// Cross-app store that manages shared specialization entries.
pub struct CrossAppStore {
    store: ContentAddressStore<ShareEntry>,
}

impl CrossAppStore {
    pub fn new() -> Self {
        CrossAppStore {
            store: ContentAddressStore::new(),
        }
    }

    pub fn register(&mut self, entry: ShareEntry) {
        self.store.insert(entry.hash, entry);
    }

    pub fn lookup(&self, hash: &ContentHash) -> Option<&ShareEntry> {
        self.store.get(hash)
    }

    /// Count entries where `is_shared()` is true.
    pub fn shared_count(&self) -> usize {
        self.store
            .entries
            .values()
            .filter(|e| e.is_shared())
            .count()
    }
}

impl Default for CrossAppStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod content_address_tests {
    use super::*;

    #[test]
    fn content_hash_from_str_deterministic() {
        let h1 = ContentHash::from_str("hello");
        let h2 = ContentHash::from_str("hello");
        assert_eq!(h1, h2);
    }

    #[test]
    fn content_hash_different_strings_different_hash() {
        let h1 = ContentHash::from_str("foo");
        let h2 = ContentHash::from_str("bar");
        assert_ne!(h1, h2);
    }

    #[test]
    fn content_hash_hex_short_length() {
        let h = ContentHash::from_str("test");
        assert_eq!(h.hex_short().len(), 8);
    }

    #[test]
    fn content_address_store_insert_and_get() {
        let mut store: ContentAddressStore<String> = ContentAddressStore::new();
        let hash = ContentHash::from_str("key");
        store.insert(hash, "value".to_string());
        assert_eq!(store.get(&hash), Some(&"value".to_string()));
    }

    #[test]
    fn content_address_store_contains() {
        let mut store: ContentAddressStore<u32> = ContentAddressStore::new();
        let hash = ContentHash::from_str("abc");
        assert!(!store.contains(&hash));
        store.insert(hash, 42);
        assert!(store.contains(&hash));
    }

    #[test]
    fn share_entry_increment_ref() {
        let hash = ContentHash::from_str("spec-1");
        let mut entry = ShareEntry::new(hash, "app-a".to_string(), "opt-level-3".to_string());
        assert_eq!(entry.ref_count, 1);
        entry.increment_ref();
        assert_eq!(entry.ref_count, 2);
    }

    #[test]
    fn share_entry_is_shared_after_two_refs() {
        let hash = ContentHash::from_str("spec-2");
        let mut entry = ShareEntry::new(hash, "app-b".to_string(), "inline-threshold".to_string());
        assert!(!entry.is_shared());
        entry.increment_ref();
        assert!(entry.is_shared());
    }

    #[test]
    fn cross_app_store_register_and_lookup() {
        let mut cas = CrossAppStore::new();
        let hash = ContentHash::from_str("nomtu-abc");
        let entry = ShareEntry::new(hash, "app-1".to_string(), "specialize-loop".to_string());
        cas.register(entry);
        let found = cas.lookup(&hash).expect("entry must be present");
        assert_eq!(found.app_id, "app-1");
        assert_eq!(found.specialization_key, "specialize-loop");
    }

    #[test]
    fn cross_app_store_shared_count() {
        let mut cas = CrossAppStore::new();

        let h1 = ContentHash::from_str("entry-1");
        let mut e1 = ShareEntry::new(h1, "app-x".to_string(), "k1".to_string());
        e1.increment_ref(); // ref_count = 2 → shared
        cas.register(e1);

        let h2 = ContentHash::from_str("entry-2");
        let e2 = ShareEntry::new(h2, "app-y".to_string(), "k2".to_string()); // ref_count = 1 → not shared
        cas.register(e2);

        assert_eq!(cas.shared_count(), 1);
    }
}
