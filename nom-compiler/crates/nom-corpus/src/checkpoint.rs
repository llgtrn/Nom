use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Resumable checkpoint for `ingest_parent`. Stored as a JSON file
/// next to the dict DB at `<dict_path>.checkpoint.json`. Written
/// atomically after each repo's transaction commits.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct IngestCheckpoint {
    /// Absolute path of the parent directory this checkpoint belongs to.
    /// Mismatches trigger a fresh start (user asked us to ingest a
    /// different corpus).
    pub parent: String,
    /// Set of repo directory names (immediate-child basenames) whose
    /// per-repo transaction has committed.
    pub completed_repos: BTreeSet<String>,
}

impl IngestCheckpoint {
    /// Derive the checkpoint path from the dict path.
    pub fn path_for(dict_path: &Path) -> PathBuf {
        let mut p = dict_path.to_path_buf();
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "nomdict.db".to_string());
        p.set_file_name(format!("{name}.checkpoint.json"));
        p
    }

    /// Load the checkpoint from disk. Returns `Default::default()` if
    /// the file doesn't exist or fails to parse (treat as fresh start).
    pub fn load(dict_path: &Path, expected_parent: &Path) -> Self {
        let path = Self::path_for(dict_path);
        let expected = expected_parent.to_string_lossy().into_owned();
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(_) => return Self { parent: expected, ..Default::default() },
        };
        let mut c: IngestCheckpoint = match serde_json::from_slice(&bytes) {
            Ok(c) => c,
            Err(_) => return Self { parent: expected, ..Default::default() },
        };
        if c.parent != expected {
            // Different parent directory — don't reuse state.
            c = Self { parent: expected, ..Default::default() };
        }
        c
    }

    /// Atomically persist the checkpoint: write to a temp file then
    /// rename. Best-effort; failure is logged but not fatal (next
    /// repo will try again).
    pub fn save(&self, dict_path: &Path) {
        let path = Self::path_for(dict_path);
        let tmp = path.with_extension("json.tmp");
        let bytes = match serde_json::to_vec_pretty(self) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("nom: checkpoint serialize error (non-fatal): {e}");
                return;
            }
        };
        if let Err(e) = std::fs::write(&tmp, &bytes) {
            eprintln!("nom: checkpoint write error (non-fatal): {e}");
            return;
        }
        if let Err(e) = std::fs::rename(&tmp, &path) {
            eprintln!("nom: checkpoint rename error (non-fatal): {e}");
        }
    }

    /// True if the named repo is already committed.
    pub fn is_completed(&self, repo_name: &str) -> bool {
        self.completed_repos.contains(repo_name)
    }

    /// Mark a repo completed. Caller is responsible for calling save()
    /// after mutation.
    pub fn mark_completed(&mut self, repo_name: String) {
        self.completed_repos.insert(repo_name);
    }
}
