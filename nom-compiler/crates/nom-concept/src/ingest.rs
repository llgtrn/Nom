//! Ingestion pipeline events — tracks sources feeding entries into the dict.

// ── IngestSource ─────────────────────────────────────────────────────────────

/// The origin of an ingestion batch.
#[derive(Debug, Clone, PartialEq)]
pub enum IngestSource {
    /// A package from the PyPI registry.
    PypiPackage,
    /// A repository from GitHub.
    GithubRepo,
    /// A path on the local filesystem.
    LocalPath,
    /// An arbitrary URL source.
    Url,
}

// ── IngestRecord ─────────────────────────────────────────────────────────────

/// Record of one completed ingestion batch.
#[derive(Debug, Clone)]
pub struct IngestRecord {
    /// Where the data came from.
    pub source: IngestSource,
    /// Human-readable name of the package / repo / path ingested.
    pub name: String,
    /// Number of dict entries produced by this batch.
    pub entry_count: u64,
    /// Wall-clock time taken to complete the batch, in milliseconds.
    pub elapsed_ms: u64,
}

impl IngestRecord {
    /// Create a new record with zero entries and zero elapsed time.
    pub fn new(source: IngestSource, name: impl Into<String>) -> Self {
        Self {
            source,
            name: name.into(),
            entry_count: 0,
            elapsed_ms: 0,
        }
    }
}

// ── IngestPipeline ────────────────────────────────────────────────────────────

/// Accumulates ingestion records across many batches.
#[derive(Debug, Default)]
pub struct IngestPipeline {
    /// All records ingested so far.
    pub records: Vec<IngestRecord>,
    /// Running total of entries across all records.
    pub total_entries: u64,
}

impl IngestPipeline {
    /// Create an empty pipeline.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a record and increment the total entry count.
    pub fn ingest(&mut self, record: IngestRecord) {
        self.total_entries += record.entry_count;
        self.records.push(record);
    }

    /// Running total of dict entries produced across all batches.
    pub fn total_entries(&self) -> u64 {
        self.total_entries
    }

    /// Number of records (batches) ingested.
    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    /// Count of records per source variant: `[pypi, github, local, url]`.
    pub fn source_counts(&self) -> [usize; 4] {
        let mut counts = [0usize; 4];
        for r in &self.records {
            match r.source {
                IngestSource::PypiPackage => counts[0] += 1,
                IngestSource::GithubRepo => counts[1] += 1,
                IngestSource::LocalPath => counts[2] += 1,
                IngestSource::Url => counts[3] += 1,
            }
        }
        counts
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ingest_record_new() {
        let r = IngestRecord::new(IngestSource::PypiPackage, "requests");
        assert_eq!(r.name, "requests");
        assert_eq!(r.source, IngestSource::PypiPackage);
        assert_eq!(r.entry_count, 0);
        assert_eq!(r.elapsed_ms, 0);
    }

    #[test]
    fn ingest_pipeline_ingest() {
        let mut pipeline = IngestPipeline::new();
        let mut r = IngestRecord::new(IngestSource::GithubRepo, "tokio");
        r.entry_count = 42;
        pipeline.ingest(r);
        assert_eq!(pipeline.record_count(), 1);
        assert_eq!(pipeline.total_entries(), 42);
    }

    #[test]
    fn total_entries_accumulates() {
        let mut pipeline = IngestPipeline::new();
        let mut r1 = IngestRecord::new(IngestSource::PypiPackage, "numpy");
        r1.entry_count = 100;
        let mut r2 = IngestRecord::new(IngestSource::GithubRepo, "serde");
        r2.entry_count = 50;
        pipeline.ingest(r1);
        pipeline.ingest(r2);
        assert_eq!(pipeline.total_entries(), 150);
    }

    #[test]
    fn record_count() {
        let mut pipeline = IngestPipeline::new();
        assert_eq!(pipeline.record_count(), 0);
        pipeline.ingest(IngestRecord::new(IngestSource::LocalPath, "/tmp/pkg"));
        pipeline.ingest(IngestRecord::new(IngestSource::Url, "https://example.com"));
        assert_eq!(pipeline.record_count(), 2);
    }

    #[test]
    fn source_counts() {
        let mut pipeline = IngestPipeline::new();
        pipeline.ingest(IngestRecord::new(IngestSource::PypiPackage, "flask"));
        pipeline.ingest(IngestRecord::new(IngestSource::PypiPackage, "django"));
        pipeline.ingest(IngestRecord::new(IngestSource::GithubRepo, "actix"));
        pipeline.ingest(IngestRecord::new(IngestSource::LocalPath, "/src"));
        let counts = pipeline.source_counts();
        assert_eq!(counts, [2, 1, 1, 0]);
    }

    #[test]
    fn ingest_multiple_sources() {
        let mut pipeline = IngestPipeline::new();
        let sources = [
            IngestSource::PypiPackage,
            IngestSource::GithubRepo,
            IngestSource::LocalPath,
            IngestSource::Url,
        ];
        for (i, source) in sources.into_iter().enumerate() {
            let mut r = IngestRecord::new(source, format!("item-{i}"));
            r.entry_count = (i as u64 + 1) * 10;
            pipeline.ingest(r);
        }
        assert_eq!(pipeline.record_count(), 4);
        // 10 + 20 + 30 + 40 = 100
        assert_eq!(pipeline.total_entries(), 100);
        assert_eq!(pipeline.source_counts(), [1, 1, 1, 1]);
    }
}

// ── Corpus pipeline ───────────────────────────────────────────────────────────

/// Ecosystem for corpus ingestion.
#[derive(Debug, Clone, PartialEq)]
pub enum CorpusEcosystem {
    PyPi,
    GitHub,
    RustCrates,
    NpmPackages,
}

impl CorpusEcosystem {
    pub fn name(&self) -> &'static str {
        match self {
            CorpusEcosystem::PyPi => "pypi",
            CorpusEcosystem::GitHub => "github",
            CorpusEcosystem::RustCrates => "crates.io",
            CorpusEcosystem::NpmPackages => "npm",
        }
    }
}

/// A batch of repos to ingest from one ecosystem.
#[derive(Debug, Clone)]
pub struct CorpusBatch {
    pub ecosystem: CorpusEcosystem,
    pub repo_urls: Vec<String>,
    pub max_entries: usize,
}

impl CorpusBatch {
    pub fn new(ecosystem: CorpusEcosystem, max_entries: usize) -> Self {
        Self { ecosystem, repo_urls: Vec::new(), max_entries }
    }

    pub fn add_repo(&mut self, url: impl Into<String>) {
        self.repo_urls.push(url.into());
    }

    pub fn repo_count(&self) -> usize {
        self.repo_urls.len()
    }
}

/// Result from processing one corpus batch.
#[derive(Debug, Clone)]
pub struct CorpusBatchResult {
    pub ecosystem: String,
    pub repos_processed: usize,
    pub entries_ingested: usize,
    pub errors: Vec<String>,
}

impl CorpusBatchResult {
    pub fn success_rate(&self) -> f64 {
        if self.repos_processed == 0 { return 0.0; }
        (self.repos_processed - self.errors.len()) as f64 / self.repos_processed as f64
    }
}

/// Orchestrates multi-ecosystem corpus ingestion.
pub struct CorpusOrchestrator {
    pub target_total: usize,
}

impl CorpusOrchestrator {
    pub fn new(target_total: usize) -> Self { Self { target_total } }

    pub fn plan_batches(&self) -> Vec<CorpusBatch> {
        let per_eco = self.target_total / 4;
        vec![
            CorpusBatch::new(CorpusEcosystem::PyPi, per_eco),
            CorpusBatch::new(CorpusEcosystem::GitHub, per_eco),
            CorpusBatch::new(CorpusEcosystem::RustCrates, per_eco),
            CorpusBatch::new(CorpusEcosystem::NpmPackages, per_eco),
        ]
    }

    pub fn simulate_batch(&self, batch: &CorpusBatch) -> CorpusBatchResult {
        CorpusBatchResult {
            ecosystem: batch.ecosystem.name().into(),
            repos_processed: batch.repo_urls.len(),
            entries_ingested: batch.repo_urls.len() * 10,
            errors: vec![],
        }
    }
}

#[cfg(test)]
mod corpus_pipeline_tests {
    use super::*;

    #[test]
    fn test_corpus_ecosystem_names() {
        assert_eq!(CorpusEcosystem::PyPi.name(), "pypi");
        assert_eq!(CorpusEcosystem::GitHub.name(), "github");
        assert_eq!(CorpusEcosystem::RustCrates.name(), "crates.io");
        assert_eq!(CorpusEcosystem::NpmPackages.name(), "npm");
    }

    #[test]
    fn test_corpus_batch_add() {
        let mut batch = CorpusBatch::new(CorpusEcosystem::PyPi, 100);
        batch.add_repo("https://github.com/pypa/pip");
        batch.add_repo("https://github.com/numpy/numpy");
        assert_eq!(batch.repo_count(), 2);
    }

    #[test]
    fn test_corpus_orchestrator_plan() {
        let orch = CorpusOrchestrator::new(400);
        let batches = orch.plan_batches();
        assert_eq!(batches.len(), 4);
        assert_eq!(batches[0].max_entries, 100);
    }

    #[test]
    fn test_simulate_batch() {
        let orch = CorpusOrchestrator::new(400);
        let mut batch = CorpusBatch::new(CorpusEcosystem::GitHub, 50);
        batch.add_repo("https://github.com/rust-lang/rust");
        let result = orch.simulate_batch(&batch);
        assert_eq!(result.repos_processed, 1);
        assert_eq!(result.entries_ingested, 10);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_batch_result_success_rate() {
        let r = CorpusBatchResult {
            ecosystem: "pypi".into(),
            repos_processed: 10,
            entries_ingested: 100,
            errors: vec!["err1".into()],
        };
        let rate = r.success_rate();
        assert!((rate - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_empty_batch_success_rate() {
        let r = CorpusBatchResult {
            ecosystem: "npm".into(),
            repos_processed: 0,
            entries_ingested: 0,
            errors: vec![],
        };
        assert_eq!(r.success_rate(), 0.0);
    }
}
