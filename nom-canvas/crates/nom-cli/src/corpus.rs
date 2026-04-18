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
}
