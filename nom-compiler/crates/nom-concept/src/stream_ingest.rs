//! Stream-and-discard disk discipline for corpus ingestion.
//!
//! Enforces peak-disk, bandwidth-throttle, and checkpoint rules so large
//! corpus ingestion never accumulates unbounded disk usage.

/// Configuration for stream-and-discard ingestion discipline.
#[derive(Debug, Clone, PartialEq)]
pub struct StreamConfig {
    /// Maximum disk usage at any point (bytes).
    pub peak_disk_bytes: u64,
    /// Bandwidth throttle in kbps; 0 = no throttle.
    pub bandwidth_throttle_kbps: u32,
    /// Save progress every N repos.
    pub checkpoint_interval: u32,
}

impl StreamConfig {
    pub fn new(
        peak_disk_bytes: u64,
        bandwidth_throttle_kbps: u32,
        checkpoint_interval: u32,
    ) -> Self {
        Self { peak_disk_bytes, bandwidth_throttle_kbps, checkpoint_interval }
    }
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            peak_disk_bytes: 10 * 1024 * 1024 * 1024, // 10 GB
            bandwidth_throttle_kbps: 0,
            checkpoint_interval: 10,
        }
    }
}

/// Tracks ingestion progress so a run can be resumed after interruption.
#[derive(Debug, Clone, PartialEq)]
pub struct IngestCheckpoint {
    pub repos_processed: u32,
    pub entries_ingested: u64,
    pub last_repo: String,
}

impl IngestCheckpoint {
    pub fn new(
        repos_processed: u32,
        entries_ingested: u64,
        last_repo: impl Into<String>,
    ) -> Self {
        Self { repos_processed, entries_ingested, last_repo: last_repo.into() }
    }

    /// Returns `true` when a checkpoint should be written at `repos_processed`.
    pub fn should_checkpoint(config: &StreamConfig, repos_processed: u32) -> bool {
        repos_processed > 0 && repos_processed % config.checkpoint_interval == 0
    }
}

/// Enforces stream-and-discard discipline during ingestion.
#[derive(Debug, Clone)]
pub struct StreamIngestor {
    pub config: StreamConfig,
}

impl StreamIngestor {
    pub fn new(config: StreamConfig) -> Self {
        Self { config }
    }

    /// Estimates disk bytes needed for `source_lines` lines (200 bytes/line average).
    pub fn estimate_disk_bytes(source_lines: u64) -> u64 {
        source_lines * 200
    }

    /// Returns `true` when `current_bytes` exceeds the configured peak.
    pub fn exceeds_peak(&self, current_bytes: u64) -> bool {
        current_bytes > self.config.peak_disk_bytes
    }

    /// Returns `true` when adding `repo_size_bytes` would exceed the peak.
    pub fn should_skip(&self, repo_size_bytes: u64, current_disk_bytes: u64) -> bool {
        self.exceeds_peak(current_disk_bytes + repo_size_bytes)
    }

    /// Simulates ingestion of a repo list.
    ///
    /// Each element is `(repo_name, repo_size_bytes)`.
    /// Returns `(count_ingested, total_entries)` where each repo contributes
    /// `repo_size / 1000` entries.
    pub fn simulate_ingest(repos: &[(&str, u64)]) -> (u32, u64) {
        let mut count: u32 = 0;
        let mut entries: u64 = 0;
        for (_name, size) in repos {
            count += 1;
            entries += size / 1000;
        }
        (count, entries)
    }
}

/// Tracks repos that were skipped due to disk constraints.
#[derive(Debug, Clone, Default)]
pub struct SkipList {
    pub skipped: Vec<String>,
}

impl SkipList {
    pub fn new() -> Self {
        Self { skipped: Vec::new() }
    }

    pub fn add(&mut self, repo: impl Into<String>) {
        self.skipped.push(repo.into());
    }

    pub fn count(&self) -> usize {
        self.skipped.len()
    }

    pub fn contains(&self, repo: &str) -> bool {
        self.skipped.iter().any(|r| r == repo)
    }
}

#[cfg(test)]
mod stream_ingest_tests {
    use super::*;

    #[test]
    fn stream_config_default_values() {
        let cfg = StreamConfig::default();
        assert_eq!(cfg.peak_disk_bytes, 10 * 1024 * 1024 * 1024);
        assert_eq!(cfg.bandwidth_throttle_kbps, 0);
        assert_eq!(cfg.checkpoint_interval, 10);
    }

    #[test]
    fn should_checkpoint_true_at_interval() {
        let cfg = StreamConfig::new(1_000_000, 0, 5);
        assert!(IngestCheckpoint::should_checkpoint(&cfg, 5));
        assert!(IngestCheckpoint::should_checkpoint(&cfg, 10));
        assert!(IngestCheckpoint::should_checkpoint(&cfg, 20));
    }

    #[test]
    fn should_checkpoint_false_at_non_interval() {
        let cfg = StreamConfig::new(1_000_000, 0, 5);
        assert!(!IngestCheckpoint::should_checkpoint(&cfg, 0));
        assert!(!IngestCheckpoint::should_checkpoint(&cfg, 3));
        assert!(!IngestCheckpoint::should_checkpoint(&cfg, 7));
    }

    #[test]
    fn estimate_disk_bytes_calculation() {
        assert_eq!(StreamIngestor::estimate_disk_bytes(0), 0);
        assert_eq!(StreamIngestor::estimate_disk_bytes(1), 200);
        assert_eq!(StreamIngestor::estimate_disk_bytes(500), 100_000);
    }

    #[test]
    fn exceeds_peak_true_when_over_limit() {
        let ingestor = StreamIngestor::new(StreamConfig::new(1_000, 0, 10));
        assert!(ingestor.exceeds_peak(1_001));
        assert!(!ingestor.exceeds_peak(1_000));
        assert!(!ingestor.exceeds_peak(999));
    }

    #[test]
    fn should_skip_false_when_under_limit() {
        let ingestor = StreamIngestor::new(StreamConfig::new(10_000, 0, 10));
        // 3000 + 5000 = 8000 <= 10000 → should not skip
        assert!(!ingestor.should_skip(5_000, 3_000));
        // 3000 + 8000 = 11000 > 10000 → should skip
        assert!(ingestor.should_skip(8_000, 3_000));
    }

    #[test]
    fn simulate_ingest_returns_correct_counts() {
        let repos = [("alpha", 5_000u64), ("beta", 3_000u64), ("gamma", 2_000u64)];
        let (count, entries) = StreamIngestor::simulate_ingest(&repos);
        assert_eq!(count, 3);
        // 5000/1000 + 3000/1000 + 2000/1000 = 5 + 3 + 2 = 10
        assert_eq!(entries, 10);
    }

    #[test]
    fn skip_list_add_and_count() {
        let mut sl = SkipList::new();
        assert_eq!(sl.count(), 0);
        sl.add("repo-a");
        sl.add("repo-b");
        assert_eq!(sl.count(), 2);
    }

    #[test]
    fn skip_list_contains_lookups() {
        let mut sl = SkipList::new();
        sl.add("repo-x");
        assert!(sl.contains("repo-x"));
        assert!(!sl.contains("repo-y"));
    }
}
