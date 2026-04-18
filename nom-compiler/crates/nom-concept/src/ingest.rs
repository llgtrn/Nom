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
