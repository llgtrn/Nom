#![deny(unsafe_code)]
use nom_blocks::NomtuRef;
use crate::store::ArtifactStore;
use crate::progress::{ProgressSink, ComposeEvent};

pub struct CodeExecInput {
    pub entity: NomtuRef,
    pub code: String,
    pub lang: String,
    pub timeout_ms: u64,
}

pub struct CodeExecResult {
    pub artifact_hash: [u8; 32],
    pub duration_ms: u64,
}

pub struct CodeExecBackend;

impl CodeExecBackend {
    pub fn compose(input: CodeExecInput, store: &mut dyn ArtifactStore, sink: &dyn ProgressSink) -> CodeExecResult {
        sink.emit(ComposeEvent::Started { backend: "code_exec".into(), entity_id: input.entity.id.clone() });
        // Stub: produce deterministic stdout bytes from code content
        let stdout = format!("[stub] exec {} ({} bytes, timeout {}ms)\n", input.lang, input.code.len(), input.timeout_ms);
        sink.emit(ComposeEvent::Progress { percent: 0.5, stage: "executing".into() });
        let artifact_hash = store.write(stdout.as_bytes());
        let byte_size = store.byte_size(&artifact_hash).unwrap_or(0);
        sink.emit(ComposeEvent::Completed { artifact_hash, byte_size });
        CodeExecResult { artifact_hash, duration_ms: 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InMemoryStore;
    use crate::progress::LogProgressSink;
    #[test]
    fn code_exec_compose_basic() {
        let mut store = InMemoryStore::new();
        let input = CodeExecInput {
            entity: NomtuRef { id: "exec1".into(), word: "hello".into(), kind: "script".into() },
            code: "print('hello')".into(),
            lang: "python".into(),
            timeout_ms: 5000,
        };
        let result = CodeExecBackend::compose(input, &mut store, &LogProgressSink);
        assert!(store.exists(&result.artifact_hash));
        let stdout = store.read(&result.artifact_hash).unwrap();
        assert!(stdout.starts_with(b"[stub]"));
    }
}
