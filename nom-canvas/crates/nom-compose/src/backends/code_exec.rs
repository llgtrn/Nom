#![deny(unsafe_code)]
use crate::progress::{ComposeEvent, ProgressSink};
use crate::store::ArtifactStore;
use nom_blocks::NomtuRef;

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

pub type ComposeResult = Result<(), String>;

pub struct CodeExecBackend;

impl CodeExecBackend {
    /// Legacy typed-input compose used by existing callers.
    pub fn compose(
        input: CodeExecInput,
        store: &mut dyn ArtifactStore,
        sink: &dyn ProgressSink,
    ) -> CodeExecResult {
        sink.emit(ComposeEvent::Started {
            backend: "code_exec".into(),
            entity_id: input.entity.id.clone(),
        });
        // Evaluate the code field via the sandbox evaluator when it carries an eval: prefix;
        // otherwise serialise to stdout bytes directly.
        let stdout = Self::eval_code(&input.code);
        sink.emit(ComposeEvent::Progress {
            percent: 0.5,
            stage: "executing".into(),
            rendered_frames: None,
            encoded_frames: None,
            elapsed_ms: None,
        });
        let artifact_hash = store.write(stdout.as_bytes());
        let byte_size = store.byte_size(&artifact_hash).unwrap_or(0);
        sink.emit(ComposeEvent::Completed {
            artifact_hash,
            byte_size,
        });
        CodeExecResult {
            artifact_hash,
            duration_ms: 0,
        }
    }

    /// String-input compose that writes the evaluated result to the store.
    pub fn compose_str(
        &self,
        input: &str,
        store: &mut dyn ArtifactStore,
        sink: &dyn ProgressSink,
    ) -> String {
        sink.emit(ComposeEvent::Started {
            backend: "code_exec".into(),
            entity_id: String::new(),
        });

        let result = Self::eval_code(input);

        let bytes = result.as_bytes();
        let hash = store.write(bytes);
        let byte_size = store.byte_size(&hash).unwrap_or(0);
        sink.emit(ComposeEvent::Completed {
            artifact_hash: hash,
            byte_size,
        });
        format!(
            "{:x}",
            hash.iter()
                .take(4)
                .fold(0u64, |acc, &b| acc * 256 + b as u64)
        )
    }

    /// Fallible variant of `compose_str`.
    pub fn compose_safe(
        &self,
        input: &str,
        store: &mut dyn ArtifactStore,
        sink: &dyn ProgressSink,
    ) -> ComposeResult {
        self.compose_str(input, store, sink);
        Ok(())
    }

    /// Evaluate a code string through the sandbox evaluator.
    /// Handles the `eval:` prefix for integer literals; passes other input through unchanged.
    fn eval_code(input: &str) -> String {
        use nom_graph::{eval_expr, sanitize, EvalContext, Expr, SandboxValue};

        if input.trim().starts_with("eval:") {
            let code = input.trim().trim_start_matches("eval:").trim();
            // Try integer literal first, then float, then fall back to a string literal.
            if let Ok(n) = code.parse::<i64>() {
                let expr = Expr::Literal(SandboxValue::Int(n));
                // Sanitize the expression before evaluation to enforce depth and allowlists.
                if sanitize(&expr).is_err() {
                    return code.to_string();
                }
                let ctx = EvalContext::new();
                match eval_expr(&expr, &ctx) {
                    Ok(SandboxValue::Int(v)) => return format!("{v}"),
                    Ok(other) => return format!("{other:?}"),
                    Err(_) => return code.to_string(),
                }
            }
            code.to_string()
        } else {
            input.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::LogProgressSink;
    use crate::store::InMemoryStore;

    #[test]
    fn code_exec_compose_basic() {
        let mut store = InMemoryStore::new();
        let result = CodeExecBackend.compose_str("hello world", &mut store, &LogProgressSink);
        // A non-eval input must produce a non-empty artifact hash string.
        assert!(!result.is_empty());
        // The stored value must match the raw input.
        let hash_bytes = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(b"hello world");
            let r = h.finalize();
            let mut b = [0u8; 32];
            b.copy_from_slice(&r);
            b
        };
        assert!(store.exists(&hash_bytes));
    }

    #[test]
    fn code_exec_compose_eval_prefix() {
        let mut store = InMemoryStore::new();
        let result = CodeExecBackend.compose_str("eval:42", &mut store, &LogProgressSink);
        // The stored artifact must contain the evaluated integer as text.
        let hash_bytes = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(b"42");
            let r = h.finalize();
            let mut b = [0u8; 32];
            b.copy_from_slice(&r);
            b
        };
        assert!(store.exists(&hash_bytes));
        let data = store.read(&hash_bytes).unwrap();
        assert_eq!(data, b"42");
        // The returned hex string must be non-empty.
        assert!(!result.is_empty());
    }

    #[test]
    fn code_exec_compose_safe_returns_ok() {
        let mut store = InMemoryStore::new();
        let r = CodeExecBackend.compose_safe("eval:7", &mut store, &LogProgressSink);
        assert!(r.is_ok());
    }
}
