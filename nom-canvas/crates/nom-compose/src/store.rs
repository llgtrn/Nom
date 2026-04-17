#![deny(unsafe_code)]
use std::collections::HashMap;

/// 32-byte content-addressed hash (SHA-256).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    /// Derive a `ContentHash` from raw bytes using SHA-256 (spec §14).
    pub fn from_bytes(data: &[u8]) -> Self {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&result);
        Self(bytes)
    }

    /// Encode the hash as a 64-character lowercase hex string.
    pub fn as_hex(&self) -> String {
        self.0.iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Raw byte array.
    pub fn as_bytes(&self) -> &[u8; 32] { &self.0 }
}

impl From<[u8; 32]> for ContentHash {
    fn from(b: [u8; 32]) -> Self { Self(b) }
}

impl From<ContentHash> for [u8; 32] {
    fn from(h: ContentHash) -> Self { h.0 }
}

pub trait ArtifactStore: Send + Sync {
    fn write(&mut self, data: &[u8]) -> [u8; 32];
    fn read(&self, hash: &[u8; 32]) -> Option<Vec<u8>>;
    fn exists(&self, hash: &[u8; 32]) -> bool;
    fn byte_size(&self, hash: &[u8; 32]) -> Option<u64>;

    /// Store `data` and return its `ContentHash`.
    /// Default implementation delegates to `write`.
    fn put_bytes(&mut self, data: &[u8]) -> ContentHash {
        ContentHash::from(self.write(data))
    }
}

pub struct InMemoryStore {
    blobs: HashMap<[u8; 32], Vec<u8>>,
}

impl InMemoryStore {
    pub fn new() -> Self { Self { blobs: HashMap::new() } }

    fn sha256(data: &[u8]) -> [u8; 32] {
        *ContentHash::from_bytes(data).as_bytes()
    }
}

impl Default for InMemoryStore { fn default() -> Self { Self::new() } }

impl ArtifactStore for InMemoryStore {
    fn write(&mut self, data: &[u8]) -> [u8; 32] {
        let hash = Self::sha256(data);
        self.blobs.entry(hash).or_insert_with(|| data.to_vec());
        hash
    }
    fn read(&self, hash: &[u8; 32]) -> Option<Vec<u8>> {
        self.blobs.get(hash).cloned()
    }
    fn exists(&self, hash: &[u8; 32]) -> bool { self.blobs.contains_key(hash) }
    fn byte_size(&self, hash: &[u8; 32]) -> Option<u64> {
        self.blobs.get(hash).map(|v| v.len() as u64)
    }

    /// Store `data` keyed by its `ContentHash`; returns the hash.
    fn put_bytes(&mut self, data: &[u8]) -> ContentHash {
        let ch = ContentHash::from_bytes(data);
        self.blobs.entry(*ch.as_bytes()).or_insert_with(|| data.to_vec());
        ch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn store_roundtrip() {
        let mut s = InMemoryStore::new();
        let hash = s.write(b"hello world");
        assert!(s.exists(&hash));
        assert_eq!(s.read(&hash).unwrap(), b"hello world");
        assert_eq!(s.byte_size(&hash).unwrap(), 11);
    }
    #[test]
    fn store_same_data_same_hash() {
        let mut s = InMemoryStore::new();
        let h1 = s.write(b"data");
        let h2 = s.write(b"data");
        assert_eq!(h1, h2);
    }

    #[test]
    fn content_hash_from_bytes_deterministic() {
        let h1 = ContentHash::from_bytes(b"hello");
        let h2 = ContentHash::from_bytes(b"hello");
        assert_eq!(h1, h2);
        // Different input must produce different hash
        let h3 = ContentHash::from_bytes(b"world");
        assert_ne!(h1, h3);
    }

    #[test]
    fn content_hash_sha256_known_value() {
        // SHA-256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        let h = ContentHash::from_bytes(b"");
        let hex = h.as_hex();
        assert_eq!(hex, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    }

    #[test]
    fn in_memory_store_put_returns_hash() {
        let mut s = InMemoryStore::new();
        let ch = s.put_bytes(b"artifact payload");
        // Retrieving via the raw bytes of the hash must succeed
        assert_eq!(s.read(ch.as_bytes()).unwrap(), b"artifact payload");
        // Calling again with same data returns the same hash
        let ch2 = s.put_bytes(b"artifact payload");
        assert_eq!(ch, ch2);
    }

    #[test]
    fn content_hash_as_hex_len_64() {
        let h = ContentHash::from_bytes(b"test data");
        let hex = h.as_hex();
        assert_eq!(hex.len(), 64);
        // Must be valid lowercase hex
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
