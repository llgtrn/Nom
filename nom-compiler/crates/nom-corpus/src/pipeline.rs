//! Corpus ingestion pipeline — skip-list, checkpoint, progress.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Ingestion progress tracker
pub struct IngestionProgress {
    pub total: usize,
    pub processed: usize,
    pub skipped: usize,
    pub failed: usize,
    pub skip_list: HashSet<String>,
    checkpoint_path: Option<PathBuf>,
}

impl IngestionProgress {
    pub fn new() -> Self {
        Self {
            total: 0,
            processed: 0,
            skipped: 0,
            failed: 0,
            skip_list: HashSet::new(),
            checkpoint_path: None,
        }
    }

    pub fn with_checkpoint(path: &Path) -> Self {
        let mut progress = Self::new();
        progress.checkpoint_path = Some(path.to_path_buf());
        progress.load_checkpoint();
        progress
    }

    pub fn should_skip(&self, id: &str) -> bool {
        self.skip_list.contains(id)
    }

    pub fn mark_processed(&mut self, id: &str) {
        self.processed += 1;
        self.skip_list.insert(id.to_string());
        if self.processed % 100 == 0 {
            self.save_checkpoint();
        }
    }

    pub fn mark_skipped(&mut self) {
        self.skipped += 1;
    }
    pub fn mark_failed(&mut self, _id: &str, _error: &str) {
        self.failed += 1;
    }

    pub fn completion_pct(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.processed + self.skipped) as f64 / self.total as f64 * 100.0
    }

    fn save_checkpoint(&self) {
        if let Some(path) = &self.checkpoint_path {
            let data: Vec<String> = self.skip_list.iter().cloned().collect();
            let json = serde_json::to_string(&data).unwrap_or_default();
            let _ = std::fs::write(path, json);
        }
    }

    fn load_checkpoint(&mut self) {
        if let Some(path) = &self.checkpoint_path {
            if let Ok(data) = std::fs::read_to_string(path) {
                if let Ok(ids) = serde_json::from_str::<Vec<String>>(&data) {
                    self.skip_list = ids.into_iter().collect();
                    self.processed = self.skip_list.len();
                }
            }
        }
    }
}

/// Ingestion source descriptor
pub struct IngestionSource {
    pub name: String,
    pub source_type: SourceType,
    pub url_or_path: String,
}

pub enum SourceType {
    GitRepo,
    PyPiPackage,
    NpmPackage,
    LocalDirectory,
}

/// Bandwidth throttle (bytes per second)
pub struct BandwidthThrottle {
    pub bytes_per_second: u64,
    bytes_this_second: u64,
    last_reset: std::time::Instant,
}

impl BandwidthThrottle {
    pub fn new(bytes_per_second: u64) -> Self {
        Self {
            bytes_per_second,
            bytes_this_second: 0,
            last_reset: std::time::Instant::now(),
        }
    }

    pub fn should_wait(&mut self, bytes: u64) -> bool {
        let now = std::time::Instant::now();
        if now.duration_since(self.last_reset).as_secs() >= 1 {
            self.bytes_this_second = 0;
            self.last_reset = now;
        }
        self.bytes_this_second += bytes;
        self.bytes_this_second > self.bytes_per_second
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_tracks_counts() {
        let mut p = IngestionProgress::new();
        p.total = 100;
        p.mark_processed("a");
        p.mark_processed("b");
        p.mark_skipped();
        assert_eq!(p.processed, 2);
        assert_eq!(p.skipped, 1);
        assert!((p.completion_pct() - 3.0).abs() < 0.01);
    }

    #[test]
    fn skip_list_prevents_reprocessing() {
        let mut p = IngestionProgress::new();
        p.mark_processed("pkg1");
        assert!(p.should_skip("pkg1"));
        assert!(!p.should_skip("pkg2"));
    }

    #[test]
    fn throttle_limits_bandwidth() {
        let mut t = BandwidthThrottle::new(1000);
        assert!(!t.should_wait(500));
        assert!(t.should_wait(600));
    }
}
