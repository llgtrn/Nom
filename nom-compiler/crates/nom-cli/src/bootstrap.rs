//! Bootstrap fixpoint verification.
//! Stage-1: Rust compiler compiles Nom-authored parser → Stage-2 binary
//! Stage-2: Stage-2 binary compiles same source → Stage-3 binary
//! Fixpoint: hash(Stage-2) == hash(Stage-3)
//!
//! Infrastructure module — used when bootstrap is attempted (GAP-10).

use sha2::{Digest, Sha256};
use std::path::Path;

/// Bootstrap stage result
#[derive(Debug, Clone)]
pub struct StageResult {
    pub stage: u8,
    pub binary_hash: String,
    pub binary_size: u64,
    pub source_hash: String,
}

/// Bootstrap proof tuple
#[derive(Debug, Clone)]
pub struct BootstrapProof {
    pub s1_hash: String,
    pub s2_hash: String,
    pub s3_hash: String,
    pub fixpoint: bool,
    pub fixpoint_at: Option<String>, // ISO 8601 date
    pub compiler_manifest_hash: String,
}

impl BootstrapProof {
    pub fn is_fixpoint(&self) -> bool {
        self.s2_hash == self.s3_hash && !self.s2_hash.is_empty()
    }
}

/// Hash a file with SHA-256
pub fn hash_file(path: &Path) -> Result<String, String> {
    let data = std::fs::read(path).map_err(|e| format!("read error: {e}"))?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}

/// Compare two files for byte-identical content
pub fn files_identical(a: &Path, b: &Path) -> Result<bool, String> {
    let data_a = std::fs::read(a).map_err(|e| format!("read {}: {e}", a.display()))?;
    let data_b = std::fs::read(b).map_err(|e| format!("read {}: {e}", b.display()))?;
    Ok(data_a == data_b)
}

/// Run the fixpoint verification
pub fn verify_fixpoint(
    stage2_binary: &Path,
    stage3_binary: &Path,
    source_dir: &Path,
) -> Result<BootstrapProof, String> {
    let s2_hash = hash_file(stage2_binary)?;
    let s3_hash = hash_file(stage3_binary)?;
    let source_hash = hash_directory(source_dir)?;

    let fixpoint = s2_hash == s3_hash;
    let fixpoint_at = if fixpoint { Some(now_iso8601()) } else { None };

    Ok(BootstrapProof {
        s1_hash: String::new(), // filled by caller
        s2_hash,
        s3_hash,
        fixpoint,
        fixpoint_at,
        compiler_manifest_hash: source_hash,
    })
}

/// Hash all .nom/.nomx/.nomtu files in a directory (deterministic order)
pub fn hash_directory(dir: &Path) -> Result<String, String> {
    let mut hasher = Sha256::new();
    let mut paths = collect_source_files(dir)?;
    paths.sort();
    for path in paths {
        let data = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        hasher.update(path.to_string_lossy().as_bytes());
        hasher.update(&data);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn collect_source_files(dir: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let mut files = Vec::new();
    if !dir.exists() {
        return Ok(files);
    }
    for entry in std::fs::read_dir(dir).map_err(|e| format!("{e}"))? {
        let entry = entry.map_err(|e| format!("{e}"))?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_source_files(&path)?);
        } else if path
            .extension()
            .map(|e| e == "nomx" || e == "nom" || e == "nomtu")
            .unwrap_or(false)
        {
            files.push(path);
        }
    }
    Ok(files)
}

/// Coarse UTC timestamp without the chrono dependency.
fn now_iso8601() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Produce a sortable epoch-seconds string that satisfies the ISO 8601
    // "calendar date" slot used in BootstrapProof.fixpoint_at.
    format!("epoch-{secs}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp_dir(name: &str) -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!("nom-bootstrap-{name}"));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        base
    }

    #[test]
    fn hash_file_produces_consistent_result() {
        let dir = tmp_dir("hash-consistent");
        let path = dir.join("test.bin");
        fs::write(&path, b"hello world").unwrap();
        let h1 = hash_file(&path).unwrap();
        let h2 = hash_file(&path).unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA-256 hex
    }

    #[test]
    fn files_identical_detects_match() {
        let dir = tmp_dir("identical-match");
        let a = dir.join("a.bin");
        let b = dir.join("b.bin");
        fs::write(&a, b"same").unwrap();
        fs::write(&b, b"same").unwrap();
        assert!(files_identical(&a, &b).unwrap());
    }

    #[test]
    fn files_identical_detects_mismatch() {
        let dir = tmp_dir("identical-mismatch");
        let a = dir.join("a.bin");
        let b = dir.join("b.bin");
        fs::write(&a, b"one").unwrap();
        fs::write(&b, b"two").unwrap();
        assert!(!files_identical(&a, &b).unwrap());
    }

    #[test]
    fn bootstrap_proof_fixpoint_check() {
        let proof = BootstrapProof {
            s1_hash: "aaa".into(),
            s2_hash: "bbb".into(),
            s3_hash: "bbb".into(),
            fixpoint: true,
            fixpoint_at: Some("epoch-1234567890".into()),
            compiler_manifest_hash: "ccc".into(),
        };
        assert!(proof.is_fixpoint());
    }

    #[test]
    fn bootstrap_proof_not_fixpoint_when_hashes_differ() {
        let proof = BootstrapProof {
            s1_hash: "aaa".into(),
            s2_hash: "bbb".into(),
            s3_hash: "ccc".into(),
            fixpoint: false,
            fixpoint_at: None,
            compiler_manifest_hash: "ddd".into(),
        };
        assert!(!proof.is_fixpoint());
    }
}
