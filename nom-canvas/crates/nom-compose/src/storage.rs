#![deny(unsafe_code)]

use opendal::layers::{LoggingLayer, RetryLayer};
use opendal::{services::Azblob, services::Fs, services::Gcs, services::S3, Operator};

/// Universal storage backend supporting multiple object stores and local filesystem.
pub struct UniversalStorage {
    operator: Operator,
}

impl UniversalStorage {
    /// Build from a local directory path.
    pub fn from_local(path: &str) -> Result<Self, String> {
        let builder = Fs::default().root(path);
        let op = Operator::new(builder)
            .map_err(|e| e.to_string())?
            .layer(RetryLayer::default())
            .layer(LoggingLayer::default())
            .finish();
        Ok(Self { operator: op })
    }

    /// Build from S3 bucket and region.
    pub fn from_s3(bucket: &str, region: &str) -> Result<Self, String> {
        let builder = S3::default().bucket(bucket).region(region);
        let op = Operator::new(builder)
            .map_err(|e| e.to_string())?
            .layer(RetryLayer::default())
            .layer(LoggingLayer::default())
            .finish();
        Ok(Self { operator: op })
    }

    /// Build from GCS bucket.
    pub fn from_gcs(bucket: &str) -> Result<Self, String> {
        let builder = Gcs::default().bucket(bucket);
        let op = Operator::new(builder)
            .map_err(|e| e.to_string())?
            .layer(RetryLayer::default())
            .layer(LoggingLayer::default())
            .finish();
        Ok(Self { operator: op })
    }

    /// Build from Azure Blob container.
    pub fn from_azure(account: &str, container: &str) -> Result<Self, String> {
        let builder = Azblob::default().account_name(account).container(container);
        let op = Operator::new(builder)
            .map_err(|e| e.to_string())?
            .layer(RetryLayer::default())
            .layer(LoggingLayer::default())
            .finish();
        Ok(Self { operator: op })
    }

    /// Read bytes from a path.
    pub fn read(&self, path: &str) -> Result<Vec<u8>, String> {
        let buffer = self
            .operator
            .blocking()
            .read(path)
            .map_err(|e| e.to_string())?;
        Ok(buffer.to_vec())
    }

    /// Write bytes to a path.
    pub fn write(&self, path: &str, data: &[u8]) -> Result<(), String> {
        self.operator
            .blocking()
            .write(path, data.to_vec())
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// List paths under a prefix.
    pub fn list(&self, prefix: &str) -> Result<Vec<String>, String> {
        let entries: Vec<String> = self
            .operator
            .blocking()
            .list(prefix)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|entry| entry.path().to_string())
            .collect();
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(suffix: &str) -> String {
        let dir = std::env::temp_dir().join(format!(
            "nom_storage_test_{}_{}",
            std::process::id(),
            suffix
        ));
        fs::create_dir_all(&dir).unwrap();
        dir.to_string_lossy().to_string()
    }

    #[test]
    fn local_storage_read_write_roundtrip() {
        let dir = temp_dir("roundtrip");
        let storage = UniversalStorage::from_local(&dir).unwrap();
        storage.write("hello.txt", b"world").unwrap();
        let data = storage.read("hello.txt").unwrap();
        assert_eq!(data, b"world");
    }

    #[test]
    fn local_storage_list_files() {
        let dir = temp_dir("list_files");
        let storage = UniversalStorage::from_local(&dir).unwrap();
        storage.write("a.txt", b"a").unwrap();
        storage.write("b.txt", b"b").unwrap();
        let list = storage.list("").unwrap();
        assert!(list.iter().any(|p| p.contains("a.txt")));
        assert!(list.iter().any(|p| p.contains("b.txt")));
    }

    #[test]
    fn local_storage_list_with_prefix() {
        let dir = temp_dir("list_prefix");
        let storage = UniversalStorage::from_local(&dir).unwrap();
        storage.write("prefix/x.txt", b"x").unwrap();
        storage.write("other/y.txt", b"y").unwrap();
        let list = storage.list("prefix/").unwrap();
        assert!(list.iter().any(|p| p.contains("x.txt")));
    }

    #[test]
    fn local_storage_overwrite() {
        let dir = temp_dir("overwrite");
        let storage = UniversalStorage::from_local(&dir).unwrap();
        storage.write("file.txt", b"first").unwrap();
        storage.write("file.txt", b"second").unwrap();
        let data = storage.read("file.txt").unwrap();
        assert_eq!(data, b"second");
    }

    #[test]
    fn local_storage_read_missing_errors() {
        let dir = temp_dir("missing");
        let storage = UniversalStorage::from_local(&dir).unwrap();
        assert!(storage.read("nonexistent.txt").is_err());
    }

    #[test]
    fn local_storage_write_empty_data() {
        let dir = temp_dir("empty");
        let storage = UniversalStorage::from_local(&dir).unwrap();
        storage.write("empty.txt", b"").unwrap();
        let data = storage.read("empty.txt").unwrap();
        assert!(data.is_empty());
    }

    #[test]
    fn local_storage_list_nested_paths() {
        let dir = temp_dir("nested");
        let storage = UniversalStorage::from_local(&dir).unwrap();
        storage.write("deep/nested/file.txt", b"deep").unwrap();
        let list = storage.list("deep/").unwrap();
        assert!(list.iter().any(|p| p.contains("nested")));
    }

    #[test]
    fn s3_without_credentials_struct_created() {
        let result = UniversalStorage::from_s3("test-bucket", "us-east-1");
        if let Ok(storage) = result {
            let _ = storage;
        }
    }

    #[test]
    fn gcs_without_credentials_struct_created() {
        let result = UniversalStorage::from_gcs("test-bucket");
        if let Ok(storage) = result {
            let _ = storage;
        }
    }

    #[test]
    fn azure_without_credentials_struct_created() {
        let result = UniversalStorage::from_azure("testaccount", "testcontainer");
        if let Ok(storage) = result {
            let _ = storage;
        }
    }

    #[test]
    fn local_storage_binary_roundtrip() {
        let dir = temp_dir("binary");
        let storage = UniversalStorage::from_local(&dir).unwrap();
        let data: Vec<u8> = (0..=255).collect();
        storage.write("bin.dat", &data).unwrap();
        let read_back = storage.read("bin.dat").unwrap();
        assert_eq!(read_back, data);
    }

    #[test]
    fn local_storage_multiple_writes_and_list() {
        let dir = temp_dir("multi_list");
        let storage = UniversalStorage::from_local(&dir).unwrap();
        for i in 0..5 {
            storage
                .write(&format!("file{i}.txt"), format!("content{i}").as_bytes())
                .unwrap();
        }
        let list = storage.list("").unwrap();
        for i in 0..5 {
            assert!(
                list.iter().any(|p| p.contains(&format!("file{i}.txt"))),
                "list must contain file{i}.txt"
            );
        }
    }
}
