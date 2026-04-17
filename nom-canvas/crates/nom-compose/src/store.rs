#![deny(unsafe_code)]
use std::collections::HashMap;

pub trait ArtifactStore: Send + Sync {
    fn write(&mut self, data: &[u8]) -> [u8; 32];
    fn read(&self, hash: &[u8; 32]) -> Option<Vec<u8>>;
    fn exists(&self, hash: &[u8; 32]) -> bool;
    fn byte_size(&self, hash: &[u8; 32]) -> Option<u64>;
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
}
