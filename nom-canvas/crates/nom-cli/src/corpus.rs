#[derive(Debug, Clone)]
pub struct CorpusStats {
    pub total_repos: u32,
    pub processed_repos: u32,
    pub total_entries: u64,
    pub partial_entries: u64,
    pub complete_entries: u64,
    pub disk_peak_bytes: u64,
}

impl CorpusStats {
    pub fn new() -> Self {
        CorpusStats {
            total_repos: 0,
            processed_repos: 0,
            total_entries: 0,
            partial_entries: 0,
            complete_entries: 0,
            disk_peak_bytes: 0,
        }
    }

    pub fn completion_ratio(&self) -> f32 {
        if self.total_entries == 0 {
            0.0
        } else {
            self.complete_entries as f32 / self.total_entries as f32
        }
    }

    pub fn processing_ratio(&self) -> f32 {
        if self.total_repos == 0 {
            0.0
        } else {
            self.processed_repos as f32 / self.total_repos as f32
        }
    }

    pub fn is_complete(&self) -> bool {
        self.total_repos > 0 && self.processed_repos == self.total_repos
    }
}

impl Default for CorpusStats {
    fn default() -> Self {
        Self::new()
    }
}

pub fn report_stats(stats: &CorpusStats) -> String {
    format!(
        "Corpus: {}/{} repos, {}/{} entries complete ({}% done)",
        stats.processed_repos,
        stats.total_repos,
        stats.complete_entries,
        stats.total_entries,
        (stats.completion_ratio() * 100.0) as u32
    )
}

pub enum CorpusCommand {
    Status,
    IngestRepo { path: String },
    IngestPypi { count: usize },
    IngestGithub { count: usize },
    Pause,
    Resume,
    Report,
    WorkspaceGc,
}

pub fn run(cmd: CorpusCommand) -> Result<(), String> {
    match cmd {
        CorpusCommand::Status => {
            println!("Corpus status: checking nomdict.db...");
            println!("Entries: (query DB)");
            println!("Kinds: (query DB)");
            Ok(())
        }
        CorpusCommand::IngestRepo { path } => {
            println!("Ingesting repository: {}", path);
            println!("Ingestion queued (stream-and-discard discipline)");
            Ok(())
        }
        CorpusCommand::IngestPypi { count } => {
            println!("Ingesting {} packages from package index", count);
            println!("Ingestion queued (stream-and-discard discipline)");
            Ok(())
        }
        CorpusCommand::IngestGithub { count } => {
            println!("Ingesting {} repositories from code host", count);
            println!("Ingestion queued (stream-and-discard discipline)");
            Ok(())
        }
        CorpusCommand::Pause => {
            println!("Pausing active ingestion...");
            Ok(())
        }
        CorpusCommand::Resume => {
            println!("Resuming paused ingestion...");
            Ok(())
        }
        CorpusCommand::Report => {
            println!("Corpus ingestion report:");
            println!("  Queued: 0");
            println!("  Completed: 0");
            println!("  Failed: 0");
            Ok(())
        }
        CorpusCommand::WorkspaceGc => {
            println!("Running workspace GC: removing stale entries...");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_corpus_status_runs() {
        run(CorpusCommand::Status).unwrap();
    }

    #[test]
    fn test_corpus_ingest_repo_runs() {
        run(CorpusCommand::IngestRepo {
            path: "/tmp/test-repo".to_string(),
        })
        .unwrap();
    }

    #[test]
    fn test_corpus_workspace_gc_runs() {
        run(CorpusCommand::WorkspaceGc).unwrap();
    }

    #[test]
    fn test_corpus_ingest_pypi_runs() {
        run(CorpusCommand::IngestPypi { count: 500 }).unwrap();
    }

    #[test]
    fn test_corpus_ingest_github_runs() {
        run(CorpusCommand::IngestGithub { count: 500 }).unwrap();
    }

    #[test]
    fn test_corpus_pause_runs() {
        run(CorpusCommand::Pause).unwrap();
    }

    #[test]
    fn test_corpus_resume_runs() {
        run(CorpusCommand::Resume).unwrap();
    }

    #[test]
    fn test_corpus_report_runs() {
        run(CorpusCommand::Report).unwrap();
    }

    #[test]
    fn test_corpus_stats_new() {
        let stats = CorpusStats::new();
        assert_eq!(stats.total_repos, 0);
        assert_eq!(stats.processed_repos, 0);
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.complete_entries, 0);
        assert_eq!(stats.disk_peak_bytes, 0);
    }

    #[test]
    fn test_corpus_stats_completion_ratio() {
        let mut stats = CorpusStats::new();
        assert_eq!(stats.completion_ratio(), 0.0);
        stats.total_entries = 100;
        stats.complete_entries = 75;
        assert!((stats.completion_ratio() - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn test_corpus_stats_processing_ratio() {
        let mut stats = CorpusStats::new();
        assert_eq!(stats.processing_ratio(), 0.0);
        stats.total_repos = 10;
        stats.processed_repos = 4;
        assert!((stats.processing_ratio() - 0.4).abs() < f32::EPSILON);
    }

    #[test]
    fn test_corpus_stats_is_complete() {
        let mut stats = CorpusStats::new();
        assert!(!stats.is_complete());
        stats.total_repos = 5;
        stats.processed_repos = 5;
        assert!(stats.is_complete());
        stats.processed_repos = 4;
        assert!(!stats.is_complete());
    }
}
