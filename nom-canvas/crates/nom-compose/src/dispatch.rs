//! `ComposeDispatcher` — routes `ComposeSpec` to the matching backend by kind.

use std::{collections::HashMap, sync::Arc};

use crate::{
    backend_trait::{
        ComposeError, ComposeOutput, ComposeSpec, CompositionBackend, InterruptFlag, ProgressSink,
    },
    kind::NomKind,
};

/// Type alias kept private; callers work through the struct API.
type BackendMap = HashMap<NomKind, Arc<dyn CompositionBackend>>;

pub struct ComposeDispatcher {
    backends: BackendMap,
}

impl ComposeDispatcher {
    pub fn new() -> Self {
        ComposeDispatcher {
            backends: HashMap::new(),
        }
    }

    /// Register a backend.  A second registration for the same kind replaces the first.
    pub fn register(&mut self, backend: Arc<dyn CompositionBackend>) {
        self.backends.insert(backend.kind(), backend);
    }

    /// Dispatch a spec to the registered backend for `spec.kind`.
    pub fn dispatch(
        &self,
        spec: &ComposeSpec,
        progress: &dyn ProgressSink,
        interrupt: &InterruptFlag,
    ) -> Result<ComposeOutput, ComposeError> {
        match self.backends.get(&spec.kind) {
            Some(b) => b.compose(spec, progress, interrupt),
            None => Err(ComposeError::BackendFailure {
                reason: format!("no backend registered for {:?}", spec.kind),
            }),
        }
    }
}

impl Default for ComposeDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend_trait::{ComposeError, ComposeOutput, ComposeSpec, InterruptFlag};

    // Minimal stub backend
    struct StubBackend {
        kind: NomKind,
        name: &'static str,
    }

    struct NullProgress;
    impl ProgressSink for NullProgress {
        fn notify(&self, _percent: u32, _message: &str) {}
    }

    impl CompositionBackend for StubBackend {
        fn kind(&self) -> NomKind {
            self.kind
        }
        fn name(&self) -> &str {
            self.name
        }
        fn compose(
            &self,
            _spec: &ComposeSpec,
            _progress: &dyn ProgressSink,
            interrupt: &InterruptFlag,
        ) -> Result<ComposeOutput, ComposeError> {
            if interrupt.is_set() {
                return Err(ComposeError::Cancelled);
            }
            Ok(ComposeOutput {
                bytes: b"stub".to_vec(),
                mime_type: "application/octet-stream".into(),
                cost_cents: 0,
            })
        }
    }

    #[test]
    fn empty_dispatcher_returns_error() {
        let d = ComposeDispatcher::new();
        let spec = ComposeSpec {
            kind: NomKind::MediaVideo,
            params: vec![],
        };
        let flag = InterruptFlag::new();
        let err = d.dispatch(&spec, &NullProgress, &flag).unwrap_err();
        assert!(matches!(err, ComposeError::BackendFailure { .. }));
    }

    #[test]
    fn register_and_dispatch_round_trip() {
        let mut d = ComposeDispatcher::new();
        d.register(Arc::new(StubBackend {
            kind: NomKind::MediaImage,
            name: "stub-image",
        }));
        let spec = ComposeSpec {
            kind: NomKind::MediaImage,
            params: vec![],
        };
        let flag = InterruptFlag::new();
        let out = d.dispatch(&spec, &NullProgress, &flag).unwrap();
        assert_eq!(out.bytes, b"stub");
    }

    #[test]
    fn unknown_kind_message_is_helpful() {
        let mut d = ComposeDispatcher::new();
        d.register(Arc::new(StubBackend {
            kind: NomKind::MediaImage,
            name: "img",
        }));
        let spec = ComposeSpec {
            kind: NomKind::MediaVideo,
            params: vec![],
        };
        let flag = InterruptFlag::new();
        let err = d.dispatch(&spec, &NullProgress, &flag).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("no backend registered for"), "got: {msg}");
        assert!(msg.contains("MediaVideo"), "got: {msg}");
    }

    #[test]
    fn multiple_backends_by_kind() {
        let mut d = ComposeDispatcher::new();
        d.register(Arc::new(StubBackend {
            kind: NomKind::MediaImage,
            name: "img",
        }));
        d.register(Arc::new(StubBackend {
            kind: NomKind::MediaAudio,
            name: "audio",
        }));
        let flag = InterruptFlag::new();
        for kind in [NomKind::MediaImage, NomKind::MediaAudio] {
            let spec = ComposeSpec { kind, params: vec![] };
            d.dispatch(&spec, &NullProgress, &flag).unwrap();
        }
    }

    #[test]
    fn interrupt_surfaces_through_dispatch() {
        let mut d = ComposeDispatcher::new();
        d.register(Arc::new(StubBackend {
            kind: NomKind::DataTransform,
            name: "xform",
        }));
        let spec = ComposeSpec {
            kind: NomKind::DataTransform,
            params: vec![],
        };
        let flag = InterruptFlag::new();
        flag.set();
        let err = d.dispatch(&spec, &NullProgress, &flag).unwrap_err();
        assert!(matches!(err, ComposeError::Cancelled));
    }
}
