//! Content-addressed filesystem body store.
//! Replaces legacy entries.body_bytes BLOB column.
//! Path: <root>/store/<hash>/body.<ext>

use std::path::{Path, PathBuf};
use std::fs;
use std::io;

pub struct BodyStore {
    root: PathBuf,
}

impl BodyStore {
    pub fn new(root: &Path) -> Self {
        Self { root: root.join("store") }
    }

    /// Store body bytes, returns the path
    pub fn store(&self, hash: &str, ext: &str, data: &[u8]) -> io::Result<PathBuf> {
        let dir = self.root.join(hash);
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("body.{}", ext));
        fs::write(&path, data)?;
        Ok(path)
    }

    /// Read body bytes by hash
    pub fn read(&self, hash: &str, ext: &str) -> io::Result<Vec<u8>> {
        let path = self.root.join(hash).join(format!("body.{}", ext));
        fs::read(&path)
    }

    /// Check if body exists
    pub fn exists(&self, hash: &str, ext: &str) -> bool {
        self.root.join(hash).join(format!("body.{}", ext)).exists()
    }

    /// Get path for a body
    pub fn path(&self, hash: &str, ext: &str) -> PathBuf {
        self.root.join(hash).join(format!("body.{}", ext))
    }

    /// Delete a body
    pub fn delete(&self, hash: &str) -> io::Result<()> {
        let dir = self.root.join(hash);
        if dir.exists() {
            fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }

    /// List all stored hashes
    pub fn list_hashes(&self) -> io::Result<Vec<String>> {
        let mut hashes = Vec::new();
        if !self.root.exists() { return Ok(hashes); }
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    hashes.push(name.to_string());
                }
            }
        }
        Ok(hashes)
    }

    /// Get total size of all stored bodies
    pub fn total_size(&self) -> io::Result<u64> {
        let mut total = 0u64;
        for hash in self.list_hashes()? {
            let dir = self.root.join(&hash);
            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                total += entry.metadata()?.len();
            }
        }
        Ok(total)
    }

    /// Migrate a body from SQLite BLOB to filesystem
    pub fn migrate_from_blob(&self, hash: &str, ext: &str, blob: &[u8]) -> io::Result<PathBuf> {
        if self.exists(hash, ext) {
            return Ok(self.path(hash, ext));
        }
        self.store(hash, ext, blob)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn store_and_read_round_trip() {
        let dir = tempdir().unwrap();
        let store = BodyStore::new(dir.path());
        let data = b"hello world";
        let path = store.store("abc123", "bc", data).unwrap();
        assert!(path.exists());
        let read = store.read("abc123", "bc").unwrap();
        assert_eq!(read, data);
    }

    #[test]
    fn exists_returns_false_for_missing() {
        let dir = tempdir().unwrap();
        let store = BodyStore::new(dir.path());
        assert!(!store.exists("nonexistent", "bc"));
    }

    #[test]
    fn delete_removes_directory() {
        let dir = tempdir().unwrap();
        let store = BodyStore::new(dir.path());
        store.store("abc123", "bc", b"data").unwrap();
        assert!(store.exists("abc123", "bc"));
        store.delete("abc123").unwrap();
        assert!(!store.exists("abc123", "bc"));
    }

    #[test]
    fn list_hashes_returns_stored() {
        let dir = tempdir().unwrap();
        let store = BodyStore::new(dir.path());
        store.store("hash1", "bc", b"a").unwrap();
        store.store("hash2", "avif", b"b").unwrap();
        let mut hashes = store.list_hashes().unwrap();
        hashes.sort();
        assert_eq!(hashes, vec!["hash1", "hash2"]);
    }

    #[test]
    fn migrate_from_blob_is_idempotent() {
        let dir = tempdir().unwrap();
        let store = BodyStore::new(dir.path());
        let p1 = store.migrate_from_blob("h1", "bc", b"data").unwrap();
        let p2 = store.migrate_from_blob("h1", "bc", b"different").unwrap();
        assert_eq!(p1, p2);
        // First write wins
        assert_eq!(store.read("h1", "bc").unwrap(), b"data");
    }
}
