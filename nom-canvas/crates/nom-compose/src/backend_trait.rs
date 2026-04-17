//! Core trait that every composition backend must implement.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use crate::kind::NomKind;

// ─── public types ────────────────────────────────────────────────────────────

/// Input specification handed to a backend.
#[derive(Debug, Clone)]
pub struct ComposeSpec {
    pub kind: NomKind,
    /// Key-value parameters; keys starting with `"credential:"` are sensitive.
    pub params: Vec<(String, String)>,
}

/// Successful output from a backend.
#[derive(Debug, Clone)]
pub struct ComposeOutput {
    pub bytes: Vec<u8>,
    pub mime_type: String,
    /// Cost in hundredths of a cent (micro-billing).
    pub cost_cents: u64,
}

/// Backends call this to stream progress to callers.
pub trait ProgressSink: Send + Sync {
    fn notify(&self, percent: u32, message: &str);
}

/// Shared cancellation flag.  Backends must poll `is_set()` at checkpoints.
pub struct InterruptFlag(pub Arc<AtomicBool>);

impl InterruptFlag {
    pub fn new() -> Self {
        InterruptFlag(Arc::new(AtomicBool::new(false)))
    }

    pub fn set(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_set(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

impl Default for InterruptFlag {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors returned by backends.
#[derive(Debug, thiserror::Error)]
pub enum ComposeError {
    #[error("composition timed out")]
    Timeout,
    #[error("composition was cancelled")]
    Cancelled,
    #[error("invalid spec: {0}")]
    InvalidSpec(String),
    #[error("backend failure: {reason}")]
    BackendFailure { reason: String },
}

// ─── trait ───────────────────────────────────────────────────────────────────

/// Every pluggable backend implements this.
pub trait CompositionBackend: Send + Sync {
    fn kind(&self) -> NomKind;
    fn name(&self) -> &str;
    fn compose(
        &self,
        spec: &ComposeSpec,
        progress: &dyn ProgressSink,
        interrupt: &InterruptFlag,
    ) -> Result<ComposeOutput, ComposeError>;
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_and_output_round_trip() {
        let spec = ComposeSpec {
            kind: NomKind::MediaImage,
            params: vec![("width".into(), "512".into())],
        };
        assert_eq!(spec.kind, NomKind::MediaImage);
        assert_eq!(spec.params[0].0, "width");

        let out = ComposeOutput {
            bytes: vec![1, 2, 3],
            mime_type: "image/png".into(),
            cost_cents: 7,
        };
        assert_eq!(out.bytes.len(), 3);
        assert_eq!(out.cost_cents, 7);
    }

    #[test]
    fn interrupt_flag_read_write() {
        let flag = InterruptFlag::new();
        assert!(!flag.is_set());
        flag.set();
        assert!(flag.is_set());
    }

    #[test]
    fn error_display_backend_failure() {
        let e = ComposeError::BackendFailure {
            reason: "quota exceeded".into(),
        };
        assert!(e.to_string().contains("quota exceeded"));
    }

    #[test]
    fn error_display_cancelled() {
        let e = ComposeError::Cancelled;
        assert!(e.to_string().contains("cancel"));
    }
}
