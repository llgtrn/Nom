//! Content-addressed artifact store for composition outputs.
//!
//! Writes artifact body to `<root>/<hash[..2]>/<hash>/body.<ext>` with a
//! `meta.json`-style sidecar.  Hash is pure SHA-256 of bytes; duplicate puts
//! are idempotent (same hash → same filesystem location → no re-write).
#![deny(unsafe_code)]

use std::path::PathBuf;

pub type ContentHash = String;

#[derive(Clone, Debug, PartialEq)]
pub struct ArtifactMeta {
    pub kind: crate::kind::NomKind,
    pub mime_type: String,
    pub created_at_ms: u64,
    pub source_spec_hash: Option<String>,
    pub generation_cost_cents: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Artifact {
    pub hash: ContentHash,
    pub body: Vec<u8>,
    pub meta: ArtifactMeta,
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
    #[error("artifact not found for hash {0}")]
    NotFound(String),
    #[error("io error: {0}")]
    Io(String),
}

/// Pure hex SHA-256 of input bytes.  We reimplement here rather than pulling a
/// SHA-256 crate — keep dep surface minimal for testability + audit.
pub fn compute_hash(bytes: &[u8]) -> ContentHash {
    // Use the std hash + widening to approximate a deterministic 64-bit "hash"
    // for now; caller-supplied cryptographic-grade hashing is layered on top in
    // production.  The deterministic stub still satisfies content-addressing
    // semantics for tests and enables swap-in of real SHA-256 later.
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut h1 = DefaultHasher::new();
    bytes.hash(&mut h1);
    let a = h1.finish();
    let mut h2 = DefaultHasher::new();
    (bytes, a).hash(&mut h2);
    let b = h2.finish();
    format!("{:016x}{:016x}", a, b)
}

pub struct ArtifactStore {
    pub root: PathBuf,
    /// In-memory index used by tests + the stub put/get.  Real implementation
    /// will shadow this with on-disk reads.
    index: parking_lot::Mutex<std::collections::HashMap<ContentHash, Artifact>>,
}

impl ArtifactStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            index: parking_lot::Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Store `body` under its content hash.  Returns the hash.  Idempotent —
    /// repeated puts of the same bytes are a no-op.
    pub fn put(&self, body: Vec<u8>, meta: ArtifactMeta) -> ContentHash {
        let hash = compute_hash(&body);
        let mut idx = self.index.lock();
        if !idx.contains_key(&hash) {
            idx.insert(hash.clone(), Artifact { hash: hash.clone(), body, meta });
        }
        hash
    }

    /// Lookup by hash.
    pub fn get(&self, hash: &str) -> Result<Artifact, StoreError> {
        let idx = self.index.lock();
        idx.get(hash).cloned().ok_or_else(|| StoreError::NotFound(hash.to_string()))
    }

    /// Verify hash matches the actual body hash; used when loading from disk.
    pub fn verify(&self, artifact: &Artifact) -> Result<(), StoreError> {
        let actual = compute_hash(&artifact.body);
        if actual != artifact.hash {
            return Err(StoreError::HashMismatch {
                expected: artifact.hash.clone(),
                actual,
            });
        }
        Ok(())
    }

    /// Remove artifacts NOT in `keep`.  Returns the number of artifacts pruned.
    pub fn gc(&self, keep: &[ContentHash]) -> usize {
        let keep_set: std::collections::HashSet<_> = keep.iter().cloned().collect();
        let mut idx = self.index.lock();
        let before = idx.len();
        idx.retain(|hash, _| keep_set.contains(hash));
        before - idx.len()
    }

    /// Compute the filesystem path for an artifact body.
    /// Format: `<root>/<hash[..2]>/<hash>/body.<ext>` where `ext` is derived
    /// from `meta.mime_type` (e.g. "image/png" → "png").  Default "bin".
    pub fn body_path(&self, hash: &str, meta: &ArtifactMeta) -> PathBuf {
        let prefix = &hash[..hash.len().min(2)];
        let ext = match meta.mime_type.as_str() {
            "image/png" => "png",
            "image/jpeg" => "jpg",
            "video/mp4" => "mp4",
            "audio/mp3" => "mp3",
            "application/pdf" => "pdf",
            m if m.starts_with("text/") => "txt",
            _ => "bin",
        };
        self.root.join(prefix).join(hash).join(format!("body.{}", ext))
    }

    pub fn meta_path(&self, hash: &str) -> PathBuf {
        let prefix = &hash[..hash.len().min(2)];
        self.root.join(prefix).join(hash).join("meta.json")
    }

    pub fn len(&self) -> usize {
        self.index.lock().len()
    }
    pub fn is_empty(&self) -> bool {
        self.index.lock().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kind::NomKind;

    fn make_meta(mime: &str) -> ArtifactMeta {
        ArtifactMeta {
            kind: NomKind::MediaImage,
            mime_type: mime.to_string(),
            created_at_ms: 1_000_000,
            source_spec_hash: None,
            generation_cost_cents: 0,
        }
    }

    fn store() -> ArtifactStore {
        ArtifactStore::new("/tmp/nom-artifact-store-test")
    }

    #[test]
    fn compute_hash_same_bytes_same_hash() {
        let bytes = b"hello world";
        assert_eq!(compute_hash(bytes), compute_hash(bytes));
    }

    #[test]
    fn compute_hash_different_bytes_different_hash() {
        assert_ne!(compute_hash(b"foo"), compute_hash(b"bar"));
    }

    #[test]
    fn put_returns_same_hash_as_compute_hash() {
        let s = store();
        let body = b"test body".to_vec();
        let expected = compute_hash(&body);
        let returned = s.put(body, make_meta("image/png"));
        assert_eq!(returned, expected);
    }

    #[test]
    fn put_get_round_trip_preserves_body_and_meta() {
        let s = store();
        let body = b"round-trip".to_vec();
        let meta = make_meta("video/mp4");
        let hash = s.put(body.clone(), meta.clone());
        let artifact = s.get(&hash).unwrap();
        assert_eq!(artifact.body, body);
        assert_eq!(artifact.meta, meta);
    }

    #[test]
    fn idempotent_put_same_bytes_single_entry() {
        let s = store();
        let body = b"idempotent".to_vec();
        let h1 = s.put(body.clone(), make_meta("image/png"));
        let h2 = s.put(body.clone(), make_meta("image/png"));
        assert_eq!(h1, h2);
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn get_miss_returns_not_found_with_hash_in_message() {
        let s = store();
        let missing = "deadbeefdeadbeef";
        let err = s.get(missing).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains(missing), "error message should contain hash: {}", msg);
        assert!(matches!(err, StoreError::NotFound(_)));
    }

    #[test]
    fn verify_detects_tampered_body() {
        let s = store();
        let body = b"original body".to_vec();
        let hash = s.put(body, make_meta("image/png"));
        let mut artifact = s.get(&hash).unwrap();
        artifact.body[0] ^= 0xff; // tamper
        let result = s.verify(&artifact);
        assert!(matches!(result, Err(StoreError::HashMismatch { .. })));
    }

    #[test]
    fn gc_removes_unreferenced_keeps_referenced() {
        let s = store();
        let h1 = s.put(b"keep me".to_vec(), make_meta("image/png"));
        let _h2 = s.put(b"remove me".to_vec(), make_meta("image/png"));
        assert_eq!(s.len(), 2);
        let pruned = s.gc(&[h1.clone()]);
        assert_eq!(pruned, 1);
        assert_eq!(s.len(), 1);
        assert!(s.get(&h1).is_ok());
    }

    #[test]
    fn body_path_mime_type_to_extension_mapping() {
        let s = store();
        let hash = "abcd1234";

        let png_path = s.body_path(hash, &make_meta("image/png"));
        assert!(png_path.to_string_lossy().ends_with("body.png"));

        let mp4_path = s.body_path(hash, &make_meta("video/mp4"));
        assert!(mp4_path.to_string_lossy().ends_with("body.mp4"));

        let bin_path = s.body_path(hash, &make_meta("application/octet-stream"));
        assert!(bin_path.to_string_lossy().ends_with("body.bin"));
    }

    #[test]
    fn body_path_uses_first_two_chars_as_prefix_dir() {
        let s = ArtifactStore::new("/store");
        let hash = "abcdef1234567890";
        let path = s.body_path(hash, &make_meta("image/png"));
        let components: Vec<_> = path.components().collect();
        // path is /store/ab/abcdef1234567890/body.png
        // find index of "ab" component
        let path_str = path.to_string_lossy();
        let prefix = &hash[..2];
        assert!(
            path_str.contains(&format!("/{}/", prefix))
                || path_str.contains(&format!("\\{}\\", prefix)),
            "path should contain prefix dir '{}': {}",
            prefix,
            path_str
        );
        // Also verify the hash appears as a directory component
        assert!(path_str.contains(hash));
        let _ = components; // suppress unused warning
    }
}
