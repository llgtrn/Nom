#![deny(unsafe_code)]
use std::collections::HashMap;

/// 32-byte content-addressed hash derived from FNV-1a expansion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    /// Derive a `ContentHash` from raw bytes using FNV-1a 64-bit expanded to 32 bytes.
    ///
    /// Layout:
    ///  bytes  0- 7 = h0.to_le_bytes()
    ///  bytes  8-15 = h1.to_le_bytes()  (h1 = h0 rotated left 17)
    ///  bytes 16-23 = (h0 ^ h1).to_le_bytes()
    ///  bytes 24-31 = (h0.wrapping_add(h1)).to_le_bytes()
    pub fn from_bytes(data: &[u8]) -> Self {
        const FNV_OFFSET: u64 = 14695981039346656037;
        const FNV_PRIME: u64 = 1099511628211;
        let mut h0: u64 = FNV_OFFSET;
        for &b in data {
            h0 ^= b as u64;
            h0 = h0.wrapping_mul(FNV_PRIME);
        }
        let h1 = h0.rotate_left(17);
        let h2 = h0 ^ h1;
        let h3 = h0.wrapping_add(h1);
        let mut out = [0u8; 32];
        out[0..8].copy_from_slice(&h0.to_le_bytes());
        out[8..16].copy_from_slice(&h1.to_le_bytes());
        out[16..24].copy_from_slice(&h2.to_le_bytes());
        out[24..32].copy_from_slice(&h3.to_le_bytes());
        Self(out)
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
        // Simple deterministic hash (real impl would use sha2 crate)
        // FNV-1a → 32-byte expansion via mixing
        let mut h = [0u8; 32];
        let mut state: u64 = 14695981039346656037;
        for &b in data {
            state ^= b as u64;
            state = state.wrapping_mul(1099511628211);
        }
        // Fill 32 bytes from 64-bit hash using expansion
        for i in 0..8 {
            let shift = (i % 4) * 8;
            let word = if i < 4 { state } else { state.rotate_left(32) };
            h[i * 4..(i + 1) * 4].copy_from_slice(&((word >> shift) as u32).to_le_bytes());
        }
        h
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
