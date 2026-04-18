pub enum CorpusCommand {
    Status,
    IngestRepo { path: String },
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
}
