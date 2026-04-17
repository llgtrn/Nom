#![deny(unsafe_code)]

use std::collections::HashMap;

/// Which compose backend to route to — DB-driven at runtime.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BackendKind {
    Video, Audio, Image, Document, Data, App, Workflow, Scenario, RagQuery,
    Transform, EmbedGen, Render, Export, Pipeline, CodeExec, WebScreen,
}

impl BackendKind {
    pub fn from_kind_name(name: &str) -> Option<Self> {
        match name {
            "video" => Some(Self::Video),
            "audio" => Some(Self::Audio),
            "image" => Some(Self::Image),
            "document" => Some(Self::Document),
            "data" => Some(Self::Data),
            "app" => Some(Self::App),
            "workflow" => Some(Self::Workflow),
            "scenario" => Some(Self::Scenario),
            "rag_query" => Some(Self::RagQuery),
            "transform" => Some(Self::Transform),
            "embed_gen" => Some(Self::EmbedGen),
            "render" => Some(Self::Render),
            "export" => Some(Self::Export),
            "pipeline" => Some(Self::Pipeline),
            "code_exec" => Some(Self::CodeExec),
            "web_screen" => Some(Self::WebScreen),
            _ => None,
        }
    }
    pub fn name(&self) -> &'static str {
        match self {
            Self::Video => "video", Self::Audio => "audio", Self::Image => "image",
            Self::Document => "document", Self::Data => "data", Self::App => "app",
            Self::Workflow => "workflow", Self::Scenario => "scenario",
            Self::RagQuery => "rag_query", Self::Transform => "transform",
            Self::EmbedGen => "embed_gen", Self::Render => "render",
            Self::Export => "export", Self::Pipeline => "pipeline",
            Self::CodeExec => "code_exec", Self::WebScreen => "web_screen",
        }
    }
}

/// Trait every compose backend must implement.
pub trait Backend: Send + Sync {
    fn kind(&self) -> BackendKind;
    fn compose(&self, input: &str, progress: &dyn Fn(f32)) -> Result<String, String>;
}

/// Registry mapping BackendKind to a concrete Backend implementation.
pub struct BackendRegistry {
    backends: HashMap<BackendKind, Box<dyn Backend>>,
}

impl BackendRegistry {
    pub fn new() -> Self {
        Self { backends: HashMap::new() }
    }

    /// Register a backend, keyed by its own kind().
    pub fn register(&mut self, backend: Box<dyn Backend>) {
        self.backends.insert(backend.kind(), backend);
    }

    /// Dispatch to the registered backend, or return Err if none registered.
    pub fn dispatch(&self, kind: BackendKind, input: &str, progress: &dyn Fn(f32)) -> Result<String, String> {
        match self.backends.get(&kind) {
            Some(b) => b.compose(input, progress),
            None => Err(format!("no backend registered for kind: {}", kind.name())),
        }
    }

    /// List all currently registered kinds.
    pub fn registered_kinds(&self) -> Vec<BackendKind> {
        self.backends.keys().cloned().collect()
    }
}

impl Default for BackendRegistry {
    fn default() -> Self { Self::new() }
}

/// Stub backend for testing — echoes the input with a kind prefix.
pub struct NoopBackend {
    kind: BackendKind,
}

impl NoopBackend {
    pub fn new(kind: BackendKind) -> Self { Self { kind } }
}

impl Backend for NoopBackend {
    fn kind(&self) -> BackendKind { self.kind.clone() }
    fn compose(&self, input: &str, progress: &dyn Fn(f32)) -> Result<String, String> {
        progress(1.0);
        Ok(format!("{}:{}", self.kind.name(), input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dispatch_kind_from_name_roundtrip() {
        assert_eq!(BackendKind::from_kind_name("video"), Some(BackendKind::Video));
        assert_eq!(BackendKind::from_kind_name("unknown"), None);
    }
    #[test]
    fn dispatch_kind_name_matches_from_name() {
        let kind = BackendKind::Document;
        assert_eq!(BackendKind::from_kind_name(kind.name()), Some(kind));
    }
    #[test]
    fn all_16_backends_have_kind_names() {
        let names = ["video","audio","image","document","data","app","workflow","scenario",
                     "rag_query","transform","embed_gen","render","export","pipeline","code_exec","web_screen"];
        for name in names {
            assert!(BackendKind::from_kind_name(name).is_some(), "missing: {name}");
        }
    }

    #[test]
    fn registry_register_and_dispatch_roundtrip() {
        use std::cell::Cell;
        let mut reg = BackendRegistry::new();
        reg.register(Box::new(NoopBackend::new(BackendKind::Video)));
        let called_with = Cell::new(0.0f32);
        let result = reg.dispatch(BackendKind::Video, "test-input", &|p| { called_with.set(p); });
        assert_eq!(result, Ok("video:test-input".to_string()));
        assert!((called_with.get() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn registry_dispatch_unknown_kind_returns_err() {
        let reg = BackendRegistry::new();
        let result = reg.dispatch(BackendKind::Audio, "x", &|_| {});
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("audio"));
    }

    #[test]
    fn registry_registered_kinds_lists_all_registered() {
        let mut reg = BackendRegistry::new();
        reg.register(Box::new(NoopBackend::new(BackendKind::Image)));
        reg.register(Box::new(NoopBackend::new(BackendKind::Document)));
        let mut kinds = reg.registered_kinds();
        kinds.sort_by_key(|k| k.name());
        assert_eq!(kinds, vec![BackendKind::Document, BackendKind::Image]);
    }
}
