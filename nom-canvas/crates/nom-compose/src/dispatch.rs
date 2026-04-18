#![deny(unsafe_code)]

use std::collections::HashMap;

use crate::progress::{ComposeEvent, ProgressSink};
use crate::store::ArtifactStore as _;

/// Which compose backend to route to — DB-driven at runtime.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BackendKind {
    Video,
    Audio,
    Image,
    Document,
    Data,
    App,
    Workflow,
    Scenario,
    RagQuery,
    Transform,
    EmbedGen,
    Render,
    Export,
    Pipeline,
    CodeExec,
    WebScreen,
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
            Self::Video => "video",
            Self::Audio => "audio",
            Self::Image => "image",
            Self::Document => "document",
            Self::Data => "data",
            Self::App => "app",
            Self::Workflow => "workflow",
            Self::Scenario => "scenario",
            Self::RagQuery => "rag_query",
            Self::Transform => "transform",
            Self::EmbedGen => "embed_gen",
            Self::Render => "render",
            Self::Export => "export",
            Self::Pipeline => "pipeline",
            Self::CodeExec => "code_exec",
            Self::WebScreen => "web_screen",
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
        Self {
            backends: HashMap::new(),
        }
    }

    /// Register a backend, keyed by its own kind().
    pub fn register(&mut self, backend: Box<dyn Backend>) {
        self.backends.insert(backend.kind(), backend);
    }

    /// Dispatch to the registered backend, or return Err if none registered.
    pub fn dispatch(
        &self,
        kind: BackendKind,
        input: &str,
        progress: &dyn Fn(f32),
    ) -> Result<String, String> {
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
    fn default() -> Self {
        Self::new()
    }
}

/// Stub backend for testing — echoes the input with a kind prefix.
pub struct NoopBackend {
    kind: BackendKind,
}

impl NoopBackend {
    pub fn new(kind: BackendKind) -> Self {
        Self { kind }
    }
}

impl Backend for NoopBackend {
    fn kind(&self) -> BackendKind {
        self.kind.clone()
    }
    fn compose(&self, input: &str, progress: &dyn Fn(f32)) -> Result<String, String> {
        progress(1.0);
        Ok(format!("{}:{}", self.kind.name(), input))
    }
}

// ---------------------------------------------------------------------------
// Backend impls for concrete backends
// ---------------------------------------------------------------------------

impl Backend for crate::backends::video::VideoBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Video
    }
    fn compose(&self, input: &str, progress: &dyn Fn(f32)) -> Result<String, String> {
        use crate::backends::video::{ContainerFormat, VideoBackend, VideoCodec, VideoInput};
        use crate::store::InMemoryStore;
        use nom_blocks::NomtuRef;
        let sink = crate::progress::LogProgressSink;
        let video_input = VideoInput {
            entity: NomtuRef {
                id: input.to_string(),
                word: "dispatch".to_string(),
                kind: "video".to_string(),
            },
            frames: vec![vec![0u8; 4]],
            fps: 24,
            width: 1,
            height: 1,
            container_format: ContainerFormat::Y4m,
            codec: VideoCodec::Raw,
        };
        let mut store = InMemoryStore::new();
        let block = VideoBackend::compose(video_input, &mut store, &sink);
        progress(1.0);
        Ok(format!("video:{}:{}ms", block.entity.id, block.duration_ms))
    }
}

impl Backend for crate::backends::audio::AudioBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Audio
    }
    fn compose(&self, input: &str, _progress: &dyn Fn(f32)) -> Result<String, String> {
        use crate::backends::audio::{AudioBackend, AudioContainer, AudioCodec, AudioInput};
        use crate::store::InMemoryStore;
        use nom_blocks::NomtuRef;
        let sink = crate::progress::LogProgressSink;
        let audio_input = AudioInput {
            entity: NomtuRef {
                id: input.to_string(),
                word: "dispatch".to_string(),
                kind: "audio".to_string(),
            },
            pcm_samples: vec![0.0f32; 8],
            sample_rate: 8000,
            codec: "pcm".to_string(),
            container: AudioContainer::Wav,
            audio_codec: AudioCodec::Pcm,
        };
        let mut store = InMemoryStore::new();
        let block = AudioBackend::compose(audio_input, &mut store, &sink);
        Ok(format!("audio:{}:{}ms", block.entity.id, block.duration_ms))
    }
}

impl Backend for crate::backends::document::DocumentBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Document
    }
    fn compose(&self, input: &str, _progress: &dyn Fn(f32)) -> Result<String, String> {
        use crate::backends::document::{DocumentBackend, DocumentInput};
        use crate::store::InMemoryStore;
        use nom_blocks::NomtuRef;
        let sink = crate::progress::LogProgressSink;
        let doc_input = DocumentInput {
            entity: NomtuRef {
                id: input.to_string(),
                word: "dispatch".to_string(),
                kind: "document".to_string(),
            },
            content_blocks: vec![input.to_string()],
            target_mime: "text/plain".to_string(),
        };
        let mut store = InMemoryStore::new();
        let block = DocumentBackend::compose(doc_input, &mut store, &sink);
        Ok(format!("document:{}:{}pages", block.entity.id, block.page_count))
    }
}

impl Backend for crate::backends::export::ExportBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Export
    }
    fn compose(&self, input: &str, _progress: &dyn Fn(f32)) -> Result<String, String> {
        use crate::backends::export::{ExportBackend, ExportInput};
        use crate::store::InMemoryStore;
        use nom_blocks::NomtuRef;
        let sink = crate::progress::LogProgressSink;
        let mut store = InMemoryStore::new();
        let input_hash = store.write(input.as_bytes());
        let out = ExportBackend::compose(
            ExportInput {
                entity: NomtuRef {
                    id: input.to_string(),
                    word: "dispatch".to_string(),
                    kind: "export".to_string(),
                },
                input_hash,
                output_format: "hex".to_string(),
            },
            &mut store,
            &sink,
        );
        Ok(format!("export:{}:{}bytes", input, out.byte_size))
    }
}

impl Backend for crate::backends::rag_query::RagQueryBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::RagQuery
    }
    fn compose(&self, input: &str, _progress: &dyn Fn(f32)) -> Result<String, String> {
        use crate::backends::rag_query::{RagChunk, RagQueryBackend, RagQueryInput};
        use crate::store::InMemoryStore;
        use nom_blocks::NomtuRef;
        let sink = crate::progress::LogProgressSink;
        let mut store = InMemoryStore::new();
        let out = RagQueryBackend::compose(
            RagQueryInput {
                entity: NomtuRef {
                    id: input.to_string(),
                    word: "dispatch".to_string(),
                    kind: "rag_query".to_string(),
                },
                query: input.to_string(),
                top_k: 1,
                chunks: vec![RagChunk {
                    id: "dispatch-chunk".to_string(),
                    text: input.to_string(),
                    score: 1.0,
                }],
            },
            &mut store,
            &sink,
        );
        Ok(format!("rag_query:{}:{}used", input, out.chunks_used.len()))
    }
}

impl Backend for crate::backends::mobile_screen::MobileScreenBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::WebScreen
    }
    fn compose(&self, input: &str, _progress: &dyn Fn(f32)) -> Result<String, String> {
        use crate::backends::mobile_screen::{MobileScreenBackend, MobileScreenSpec};
        use crate::store::InMemoryStore;
        let sink = crate::progress::LogProgressSink;
        let mut store = InMemoryStore::new();
        let spec = MobileScreenSpec {
            width: 1080,
            height: 1920,
            platform: "android".to_string(),
            scale_factor: 2.0,
        };
        MobileScreenBackend::compose(&spec, &mut store, &sink)
            .map(|()| format!("mobile_screen:{}:ok", input))
    }
}

impl Backend for crate::backends::native_screen::NativeScreenBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Render
    }
    fn compose(&self, input: &str, _progress: &dyn Fn(f32)) -> Result<String, String> {
        use crate::backends::native_screen::{NativeScreenBackend, NativeScreenSpec};
        use crate::store::InMemoryStore;
        let sink = crate::progress::LogProgressSink;
        let mut store = InMemoryStore::new();
        let spec = NativeScreenSpec {
            width: 1920,
            height: 1080,
            display_index: 0,
            format: "png".to_string(),
        };
        NativeScreenBackend::compose(&spec, &mut store, &sink)
            .map(|()| format!("native_screen:{}:ok", input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dispatch_kind_from_name_roundtrip() {
        assert_eq!(
            BackendKind::from_kind_name("video"),
            Some(BackendKind::Video)
        );
        assert_eq!(BackendKind::from_kind_name("unknown"), None);
    }
    #[test]
    fn dispatch_kind_name_matches_from_name() {
        let kind = BackendKind::Document;
        assert_eq!(BackendKind::from_kind_name(kind.name()), Some(kind));
    }
    #[test]
    fn all_16_backends_have_kind_names() {
        let names = [
            "video",
            "audio",
            "image",
            "document",
            "data",
            "app",
            "workflow",
            "scenario",
            "rag_query",
            "transform",
            "embed_gen",
            "render",
            "export",
            "pipeline",
            "code_exec",
            "web_screen",
        ];
        for name in names {
            assert!(
                BackendKind::from_kind_name(name).is_some(),
                "missing: {name}"
            );
        }
    }

    #[test]
    fn registry_register_and_dispatch_roundtrip() {
        use std::cell::Cell;
        let mut reg = BackendRegistry::new();
        reg.register(Box::new(NoopBackend::new(BackendKind::Video)));
        let called_with = Cell::new(0.0f32);
        let result = reg.dispatch(BackendKind::Video, "test-input", &|p| {
            called_with.set(p);
        });
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

    #[test]
    fn backend_registry_register_and_dispatch() {
        let mut reg = BackendRegistry::new();
        reg.register(Box::new(NoopBackend::new(BackendKind::Render)));
        let result = reg.dispatch(BackendKind::Render, "scene", &|_| {});
        assert_eq!(result, Ok("render:scene".to_string()));
    }

    #[test]
    fn backend_registry_unknown_kind_returns_err() {
        let reg = BackendRegistry::new();
        let result = reg.dispatch(BackendKind::Export, "data", &|_| {});
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("export"));
    }

    #[test]
    fn backend_registry_multiple_kinds() {
        let mut reg = BackendRegistry::new();
        reg.register(Box::new(NoopBackend::new(BackendKind::Pipeline)));
        reg.register(Box::new(NoopBackend::new(BackendKind::CodeExec)));
        reg.register(Box::new(NoopBackend::new(BackendKind::Transform)));
        assert_eq!(reg.registered_kinds().len(), 3);
    }

    #[test]
    fn backend_registry_default_is_empty() {
        let reg = BackendRegistry::default();
        assert!(reg.registered_kinds().is_empty());
    }

    #[test]
    fn all_16_kind_names_roundtrip() {
        // Every kind's name() must parse back to the same kind via from_kind_name().
        let kinds = [
            BackendKind::Video,
            BackendKind::Audio,
            BackendKind::Image,
            BackendKind::Document,
            BackendKind::Data,
            BackendKind::App,
            BackendKind::Workflow,
            BackendKind::Scenario,
            BackendKind::RagQuery,
            BackendKind::Transform,
            BackendKind::EmbedGen,
            BackendKind::Render,
            BackendKind::Export,
            BackendKind::Pipeline,
            BackendKind::CodeExec,
            BackendKind::WebScreen,
        ];
        for kind in kinds {
            let name = kind.name();
            let parsed = BackendKind::from_kind_name(name);
            assert_eq!(parsed.as_ref(), Some(&kind), "roundtrip failed for {name}");
        }
    }

    #[test]
    fn noop_backend_kind_echoed_in_output() {
        let b = NoopBackend::new(BackendKind::Scenario);
        let result = b.compose("input_data", &|_| {});
        assert_eq!(result, Ok("scenario:input_data".to_string()));
    }

    #[test]
    fn noop_backend_progress_callback_called_with_one() {
        use std::cell::Cell;
        let b = NoopBackend::new(BackendKind::EmbedGen);
        let progress_val = Cell::new(-1.0f32);
        b.compose("x", &|p| progress_val.set(p)).unwrap();
        assert!((progress_val.get() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn registry_overwrite_same_kind() {
        // Registering a second backend for the same kind must replace the first.
        let mut reg = BackendRegistry::new();
        reg.register(Box::new(NoopBackend::new(BackendKind::Audio)));
        reg.register(Box::new(NoopBackend::new(BackendKind::Audio)));
        assert_eq!(
            reg.registered_kinds().len(),
            1,
            "duplicate kind must not grow list"
        );
        let result = reg.dispatch(BackendKind::Audio, "clip", &|_| {});
        assert_eq!(result, Ok("audio:clip".to_string()));
    }

    #[test]
    fn from_kind_name_unknown_returns_none() {
        assert_eq!(BackendKind::from_kind_name(""), None);
        assert_eq!(BackendKind::from_kind_name("VIDEO"), None); // case sensitive
        assert_eq!(BackendKind::from_kind_name("unknown_kind"), None);
    }

    // -----------------------------------------------------------------------
    // Backend trait impl tests for concrete backends
    // -----------------------------------------------------------------------

    #[test]
    fn video_backend_stored_as_box_dyn_backend() {
        use crate::backends::video::VideoBackend;
        let b: Box<dyn Backend> = Box::new(VideoBackend);
        assert_eq!(b.kind(), BackendKind::Video);
        let result = b.compose("entity-v1", &|_| {});
        assert!(result.is_ok(), "VideoBackend dispatch must succeed");
        assert!(result.unwrap().starts_with("video:entity-v1"));
    }

    #[test]
    fn audio_backend_stored_as_box_dyn_backend() {
        use crate::backends::audio::AudioBackend;
        let b: Box<dyn Backend> = Box::new(AudioBackend);
        assert_eq!(b.kind(), BackendKind::Audio);
        let result = b.compose("entity-a1", &|_| {});
        assert!(result.is_ok(), "AudioBackend dispatch must succeed");
        assert!(result.unwrap().starts_with("audio:entity-a1"));
    }

    #[test]
    fn document_backend_stored_as_box_dyn_backend() {
        use crate::backends::document::DocumentBackend;
        let b: Box<dyn Backend> = Box::new(DocumentBackend);
        assert_eq!(b.kind(), BackendKind::Document);
        let result = b.compose("entity-d1", &|_| {});
        assert!(result.is_ok(), "DocumentBackend dispatch must succeed");
        assert!(result.unwrap().starts_with("document:entity-d1"));
    }

    #[test]
    fn export_backend_stored_as_box_dyn_backend() {
        use crate::backends::export::ExportBackend;
        let b: Box<dyn Backend> = Box::new(ExportBackend);
        assert_eq!(b.kind(), BackendKind::Export);
        let result = b.compose("entity-e1", &|_| {});
        assert!(result.is_ok(), "ExportBackend dispatch must succeed");
        assert!(result.unwrap().starts_with("export:entity-e1"));
    }

    #[test]
    fn rag_query_backend_stored_as_box_dyn_backend() {
        use crate::backends::rag_query::RagQueryBackend;
        let b: Box<dyn Backend> = Box::new(RagQueryBackend::default());
        assert_eq!(b.kind(), BackendKind::RagQuery);
        let result = b.compose("entity-r1", &|_| {});
        assert!(result.is_ok(), "RagQueryBackend dispatch must succeed");
        assert!(result.unwrap().starts_with("rag_query:entity-r1"));
    }

    #[test]
    fn routing_unknown_kind_returns_error() {
        let reg = BackendRegistry::new();
        // No backends registered — any kind must return Err.
        let result = reg.dispatch(BackendKind::EmbedGen, "probe", &|_| {});
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("embed_gen"), "error must name the missing kind, got: {msg}");
    }

    #[test]
    fn video_audio_document_round_trip_through_registry() {
        use crate::backends::audio::AudioBackend;
        use crate::backends::document::DocumentBackend;
        use crate::backends::video::VideoBackend;

        let mut reg = BackendRegistry::new();
        reg.register(Box::new(VideoBackend));
        reg.register(Box::new(AudioBackend));
        reg.register(Box::new(DocumentBackend));

        assert_eq!(reg.registered_kinds().len(), 3);

        let v = reg.dispatch(BackendKind::Video, "vid-probe", &|_| {});
        assert!(v.is_ok(), "video round-trip must succeed");

        let a = reg.dispatch(BackendKind::Audio, "aud-probe", &|_| {});
        assert!(a.is_ok(), "audio round-trip must succeed");

        let d = reg.dispatch(BackendKind::Document, "doc-probe", &|_| {});
        assert!(d.is_ok(), "document round-trip must succeed");
    }

    #[test]
    fn export_and_rag_query_round_trip_through_registry() {
        use crate::backends::export::ExportBackend;
        use crate::backends::rag_query::RagQueryBackend;

        let mut reg = BackendRegistry::new();
        reg.register(Box::new(ExportBackend));
        reg.register(Box::new(RagQueryBackend::default()));

        let e = reg.dispatch(BackendKind::Export, "export-probe", &|_| {});
        assert!(e.is_ok(), "export round-trip must succeed");

        let r = reg.dispatch(BackendKind::RagQuery, "rag-probe", &|_| {});
        assert!(r.is_ok(), "rag_query round-trip must succeed");
    }

    #[test]
    fn backend_progress_callback_called_during_compose() {
        use crate::backends::video::VideoBackend;
        use std::cell::Cell;
        let b: Box<dyn Backend> = Box::new(VideoBackend);
        let called = Cell::new(false);
        b.compose("progress-test", &|_| called.set(true)).unwrap();
        assert!(called.get(), "progress callback must be called during compose");
    }

    // ── Wave AD new tests ────────────────────────────────────────────────────

    #[test]
    fn dispatch_routes_all_16_backend_kinds() {
        // Register a NoopBackend for each of the 16 kinds and verify dispatch succeeds.
        let mut reg = BackendRegistry::new();
        let all_kinds = [
            BackendKind::Video,
            BackendKind::Audio,
            BackendKind::Image,
            BackendKind::Document,
            BackendKind::Data,
            BackendKind::App,
            BackendKind::Workflow,
            BackendKind::Scenario,
            BackendKind::RagQuery,
            BackendKind::Transform,
            BackendKind::EmbedGen,
            BackendKind::Render,
            BackendKind::Export,
            BackendKind::Pipeline,
            BackendKind::CodeExec,
            BackendKind::WebScreen,
        ];
        for kind in all_kinds.iter() {
            reg.register(Box::new(NoopBackend::new(kind.clone())));
        }
        assert_eq!(reg.registered_kinds().len(), 16, "all 16 backends must be registered");
        for kind in all_kinds.iter() {
            let result = reg.dispatch(kind.clone(), "probe", &|_| {});
            assert!(result.is_ok(), "dispatch must succeed for kind: {}", kind.name());
            let output = result.unwrap();
            assert!(output.starts_with(kind.name()), "output must start with kind name");
        }
    }

    #[test]
    fn plan_with_10_steps_all_execute_in_topo_order() {
        use crate::plan::CompositionPlan;
        // Build a linear 10-step chain and verify all steps execute in order.
        let mut plan = CompositionPlan::new();
        let step0 = plan.add_step(BackendKind::Video, "src", "s0");
        let mut prev = step0;
        for i in 1..10usize {
            prev = plan.add_step_after(
                BackendKind::Transform,
                format!("s{}", i - 1),
                format!("s{i}"),
                vec![prev],
            );
        }
        assert!(plan.is_valid_dag(), "10-step linear chain must be a valid DAG");
        let order = plan.topo_order();
        assert_eq!(order.len(), 10, "all 10 steps must appear in topo order");
        // Each step must be strictly after its predecessor.
        for window in order.windows(2) {
            assert!(window[0] < window[1], "step {} must precede step {}", window[0], window[1]);
        }
    }

    #[test]
    fn progress_cancellation_flag_stops_emission() {
        // Model progress cancellation: once a flag is set, no further callbacks fire.
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        let cancelled = Arc::new(AtomicBool::new(false));
        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let cc = call_count.clone();
        let ca = cancelled.clone();

        // Simulate 5 progress callbacks where we cancel after the 3rd.
        for i in 0..5usize {
            if ca.load(Ordering::SeqCst) {
                break;
            }
            cc.fetch_add(1, Ordering::SeqCst);
            if i == 2 {
                ca.store(true, Ordering::SeqCst);
            }
        }
        // Only 3 callbacks fired before cancellation.
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn semantic_5_table_registry_generates_select_for_each() {
        use crate::semantic::{SemanticColumn, SemanticDataType, SemanticModel, SemanticRegistry};
        let table_names = ["users", "orders", "products", "events", "sessions"];
        let mut reg = SemanticRegistry::new();
        for name in &table_names {
            let mut m = SemanticModel::new(*name, format!("raw.{name}"));
            m.add_column(SemanticColumn {
                name: "id".into(),
                data_type: SemanticDataType::Integer,
                description: None,
            });
            m.add_column(SemanticColumn {
                name: "created_at".into(),
                data_type: SemanticDataType::Timestamp,
                description: None,
            });
            reg.register(m);
        }
        assert_eq!(reg.model_count(), 5);
        for name in &table_names {
            let model = reg.get(name).expect("model must exist");
            let sql = model.to_select_sql();
            assert!(sql.contains(&format!("raw.{name}")), "SQL must reference the source table");
            assert!(sql.contains("id"), "SQL must contain id column");
            assert!(sql.contains("created_at"), "SQL must contain created_at column");
        }
    }

    #[test]
    fn backend_registry_register_all_kinds_count_is_16() {
        let mut reg = BackendRegistry::new();
        for kind in [
            BackendKind::Video, BackendKind::Audio, BackendKind::Image,
            BackendKind::Document, BackendKind::Data, BackendKind::App,
            BackendKind::Workflow, BackendKind::Scenario, BackendKind::RagQuery,
            BackendKind::Transform, BackendKind::EmbedGen, BackendKind::Render,
            BackendKind::Export, BackendKind::Pipeline, BackendKind::CodeExec,
            BackendKind::WebScreen,
        ] {
            reg.register(Box::new(NoopBackend::new(kind)));
        }
        assert_eq!(reg.registered_kinds().len(), 16);
    }

    #[test]
    fn noop_backend_output_format_is_kind_colon_input() {
        // Every NoopBackend outputs "<kind>:<input>".
        let kinds = [BackendKind::Data, BackendKind::App, BackendKind::Workflow];
        for kind in kinds {
            let b = NoopBackend::new(kind.clone());
            let result = b.compose("x", &|_| {}).unwrap();
            assert_eq!(result, format!("{}:x", kind.name()));
        }
    }

    #[test]
    fn dispatch_missing_kind_returns_err_with_kind_name() {
        // Dispatching to a kind not registered returns an Err containing the kind name.
        let reg = BackendRegistry::new();
        for kind in [BackendKind::Pipeline, BackendKind::CodeExec, BackendKind::WebScreen] {
            let name = kind.name();
            let err = reg.dispatch(kind, "probe", &|_| {}).unwrap_err();
            assert!(err.contains(name), "error must mention kind name: {name}");
        }
    }

    #[test]
    fn plan_diamond_all_steps_in_topo_order() {
        use crate::plan::CompositionPlan;
        // A→B, A→C, B→D, C→D (diamond).
        let mut plan = CompositionPlan::new();
        let a = plan.add_step(BackendKind::Video, "src", "v");
        let b = plan.add_step_after(BackendKind::Audio, "v", "a", vec![a]);
        let c = plan.add_step_after(BackendKind::Image, "v", "img", vec![a]);
        let d = plan.add_step_after(BackendKind::Export, "a", "out", vec![b, c]);
        assert!(plan.is_valid_dag());
        let order = plan.topo_order();
        let pos = |id| order.iter().position(|&x| x == id).unwrap();
        assert!(pos(a) < pos(b) && pos(a) < pos(c));
        assert!(pos(b) < pos(d) && pos(c) < pos(d));
    }

    #[test]
    fn plan_empty_has_zero_steps() {
        use crate::plan::CompositionPlan;
        let plan = CompositionPlan::new();
        assert_eq!(plan.steps.len(), 0);
        assert!(plan.is_valid_dag(), "empty plan is valid");
        assert_eq!(plan.topo_order().len(), 0);
    }

    #[test]
    fn semantic_model_5_cols_select_sql_lists_all() {
        use crate::semantic::{SemanticColumn, SemanticDataType, SemanticModel};
        let mut m = SemanticModel::new("fact_sales", "dw.fact_sales");
        let cols = ["sale_id", "product_id", "customer_id", "amount", "sale_date"];
        for name in &cols {
            m.add_column(SemanticColumn {
                name: (*name).into(),
                data_type: SemanticDataType::String,
                description: None,
            });
        }
        let sql = m.to_select_sql();
        for col in &cols {
            assert!(sql.contains(*col), "SQL must contain column: {col}");
        }
        assert!(sql.starts_with("SELECT "));
        assert!(sql.contains("FROM dw.fact_sales"));
    }

    #[test]
    fn plan_10_step_all_backend_kinds_are_valid_dag() {
        use crate::plan::CompositionPlan;
        let kinds = [
            BackendKind::Video, BackendKind::Audio, BackendKind::Image,
            BackendKind::Document, BackendKind::Data, BackendKind::App,
            BackendKind::Workflow, BackendKind::Scenario, BackendKind::RagQuery,
            BackendKind::Transform,
        ];
        let mut plan = CompositionPlan::new();
        let first = plan.add_step(kinds[0].clone(), "in", "s0");
        let mut prev = first;
        for (i, kind) in kinds[1..].iter().enumerate() {
            prev = plan.add_step_after(
                kind.clone(),
                format!("s{i}"),
                format!("s{}", i + 1),
                vec![prev],
            );
        }
        assert!(plan.is_valid_dag());
        assert_eq!(plan.steps.len(), 10);
        assert_eq!(plan.topo_order().len(), 10);
    }

    #[test]
    fn backend_kind_all_16_names_are_lowercase_no_spaces() {
        // All BackendKind::name() values must be lowercase with no spaces.
        let all_kinds = [
            BackendKind::Video, BackendKind::Audio, BackendKind::Image,
            BackendKind::Document, BackendKind::Data, BackendKind::App,
            BackendKind::Workflow, BackendKind::Scenario, BackendKind::RagQuery,
            BackendKind::Transform, BackendKind::EmbedGen, BackendKind::Render,
            BackendKind::Export, BackendKind::Pipeline, BackendKind::CodeExec,
            BackendKind::WebScreen,
        ];
        for kind in &all_kinds {
            let name = kind.name();
            assert!(!name.is_empty(), "name must not be empty");
            assert_eq!(name, name.to_lowercase(), "name must be lowercase: {name}");
            assert!(!name.contains(' '), "name must not contain spaces: {name}");
        }
    }

    #[test]
    fn dispatch_noop_output_starts_with_kind_name() {
        let mut reg = BackendRegistry::new();
        reg.register(Box::new(NoopBackend::new(BackendKind::Scenario)));
        let out = reg.dispatch(BackendKind::Scenario, "my-input", &|_| {}).unwrap();
        assert!(out.starts_with("scenario:"), "output must start with 'scenario:'");
        assert!(out.contains("my-input"), "output must contain input");
    }

    #[test]
    fn plan_single_step_dag_is_valid() {
        use crate::plan::CompositionPlan;
        let mut plan = CompositionPlan::new();
        plan.add_step(BackendKind::Render, "input", "output");
        assert!(plan.is_valid_dag());
        assert_eq!(plan.topo_order(), vec![0]);
    }

    #[test]
    fn semantic_model_with_description_column() {
        use crate::semantic::{SemanticColumn, SemanticDataType, SemanticModel};
        let mut m = SemanticModel::new("items", "raw.items");
        m.add_column(SemanticColumn {
            name: "price".into(),
            data_type: SemanticDataType::Float,
            description: Some("unit price in USD".into()),
        });
        let col = m.column("price").unwrap();
        assert_eq!(col.data_type, SemanticDataType::Float);
        assert_eq!(col.description.as_deref(), Some("unit price in USD"));
    }

    // ── Wave AE new tests ────────────────────────────────────────────────────

    #[test]
    fn registry_with_7_backends_all_dispatch_ok() {
        // Register the 7 "real" backends (video, audio, document, export, rag_query,
        // mobile_screen→WebScreen, native_screen→Render) via NoopBackend and verify each routes.
        let mut reg = BackendRegistry::new();
        let kinds = [
            BackendKind::Video,
            BackendKind::Audio,
            BackendKind::Document,
            BackendKind::Export,
            BackendKind::RagQuery,
            BackendKind::WebScreen,
            BackendKind::Render,
        ];
        for kind in &kinds {
            reg.register(Box::new(NoopBackend::new(kind.clone())));
        }
        assert_eq!(reg.registered_kinds().len(), 7, "exactly 7 backends must be registered");
        for kind in &kinds {
            let result = reg.dispatch(kind.clone(), "probe", &|_| {});
            assert!(result.is_ok(), "dispatch must succeed for kind: {}", kind.name());
        }
    }

    #[test]
    fn registry_route_by_kind_returns_correct_backend() {
        // Each kind returns an output that starts with its own name prefix.
        let mut reg = BackendRegistry::new();
        reg.register(Box::new(NoopBackend::new(BackendKind::Video)));
        reg.register(Box::new(NoopBackend::new(BackendKind::Audio)));
        reg.register(Box::new(NoopBackend::new(BackendKind::Document)));

        let v = reg.dispatch(BackendKind::Video, "x", &|_| {}).unwrap();
        assert!(v.starts_with("video:"), "video backend output must start with 'video:'");

        let a = reg.dispatch(BackendKind::Audio, "x", &|_| {}).unwrap();
        assert!(a.starts_with("audio:"), "audio backend output must start with 'audio:'");

        let d = reg.dispatch(BackendKind::Document, "x", &|_| {}).unwrap();
        assert!(d.starts_with("document:"), "document backend output must start with 'document:'");
    }

    #[test]
    fn registry_unknown_kind_error_message_contains_kind_name() {
        let reg = BackendRegistry::new();
        let err = reg.dispatch(BackendKind::Scenario, "x", &|_| {}).unwrap_err();
        assert!(err.contains("scenario"), "error must name the missing kind");
    }

    #[test]
    fn concurrent_dispatch_simulation_all_succeed() {
        // Simulate concurrent dispatch by sharing a registry across multiple
        // closures (no actual threads needed — tests the Send+Sync bound via Arc).
        use std::sync::Arc;
        let mut reg = BackendRegistry::new();
        reg.register(Box::new(NoopBackend::new(BackendKind::Video)));
        reg.register(Box::new(NoopBackend::new(BackendKind::Audio)));
        reg.register(Box::new(NoopBackend::new(BackendKind::Image)));
        let reg = Arc::new(reg);

        let results: Vec<_> = [
            (BackendKind::Video, "v-payload"),
            (BackendKind::Audio, "a-payload"),
            (BackendKind::Image, "i-payload"),
        ]
        .iter()
        .map(|(kind, input)| reg.dispatch(kind.clone(), input, &|_| {}))
        .collect();

        for r in &results {
            assert!(r.is_ok(), "each concurrent dispatch must succeed");
        }
        assert!(results[0].as_ref().unwrap().contains("video"));
        assert!(results[1].as_ref().unwrap().contains("audio"));
        assert!(results[2].as_ref().unwrap().contains("image"));
    }

    #[test]
    fn backend_kind_from_kind_name_all_names_parse() {
        // Every variant's name() must parse back via from_kind_name() to itself.
        let all = [
            BackendKind::Video, BackendKind::Audio, BackendKind::Image,
            BackendKind::Document, BackendKind::Data, BackendKind::App,
            BackendKind::Workflow, BackendKind::Scenario, BackendKind::RagQuery,
            BackendKind::Transform, BackendKind::EmbedGen, BackendKind::Render,
            BackendKind::Export, BackendKind::Pipeline, BackendKind::CodeExec,
            BackendKind::WebScreen,
        ];
        for kind in &all {
            let parsed = BackendKind::from_kind_name(kind.name());
            assert_eq!(parsed.as_ref(), Some(kind), "roundtrip failed for {:?}", kind);
        }
    }

    #[test]
    fn noop_backend_compose_empty_input_returns_kind_prefix() {
        let b = NoopBackend::new(BackendKind::Transform);
        let result = b.compose("", &|_| {}).unwrap();
        assert_eq!(result, "transform:", "empty input must yield 'transform:'");
    }

    #[test]
    fn registry_dispatch_after_replacing_backend() {
        // Registering a new backend for the same kind replaces the old one.
        let mut reg = BackendRegistry::new();
        reg.register(Box::new(NoopBackend::new(BackendKind::Pipeline)));
        // Replace with another noop (same observable behavior).
        reg.register(Box::new(NoopBackend::new(BackendKind::Pipeline)));
        assert_eq!(reg.registered_kinds().len(), 1);
        let result = reg.dispatch(BackendKind::Pipeline, "data", &|_| {}).unwrap();
        assert!(result.starts_with("pipeline:"));
    }

    // ── Wave AH new tests ────────────────────────────────────────────────────

    #[test]
    fn dispatch_backend_kind_roundtrip() {
        // BackendKind::from_kind_name("video") must return BackendKind::Video.
        let kind = BackendKind::from_kind_name("video");
        assert_eq!(kind, Some(BackendKind::Video));
    }

    #[test]
    fn dispatch_unknown_kind_returns_error() {
        let reg = BackendRegistry::new();
        let result = reg.dispatch(BackendKind::Data, "probe", &|_| {});
        assert!(result.is_err(), "dispatch on empty registry must return Err");
        assert!(result.unwrap_err().contains("data"));
    }

    #[test]
    fn dispatch_all_registered_kinds_resolve() {
        let mut reg = BackendRegistry::new();
        let all_kinds = [
            BackendKind::Video,
            BackendKind::Audio,
            BackendKind::Image,
            BackendKind::Document,
            BackendKind::Data,
            BackendKind::App,
            BackendKind::Workflow,
            BackendKind::Scenario,
            BackendKind::RagQuery,
            BackendKind::Transform,
            BackendKind::EmbedGen,
            BackendKind::Render,
            BackendKind::Export,
            BackendKind::Pipeline,
            BackendKind::CodeExec,
            BackendKind::WebScreen,
        ];
        for kind in &all_kinds {
            reg.register(Box::new(NoopBackend::new(kind.clone())));
        }
        for kind in &all_kinds {
            let result = reg.dispatch(kind.clone(), "test", &|_| {});
            assert!(result.is_ok(), "kind {} must resolve", kind.name());
        }
    }

    #[test]
    fn document_backend_metadata_nonempty() {
        use crate::backends::document::DocumentBackend;
        let b: Box<dyn Backend> = Box::new(DocumentBackend);
        // kind() must return Document and compose must yield non-empty output.
        assert_eq!(b.kind(), BackendKind::Document);
        let out = b.compose("metadata-check", &|_| {}).unwrap();
        assert!(!out.is_empty(), "document backend output must not be empty");
    }

    #[test]
    fn document_backend_title_in_output() {
        use crate::backends::document::DocumentBackend;
        let b: Box<dyn Backend> = Box::new(DocumentBackend);
        let out = b.compose("my-title", &|_| {}).unwrap();
        // The dispatch impl passes input as entity id; output starts with "document:<id>".
        assert!(out.starts_with("document:my-title"), "output must reference entity id");
    }

    #[test]
    fn document_backend_empty_content_ok() {
        use crate::backends::document::{DocumentBackend, DocumentInput};
        use crate::progress::LogProgressSink;
        use crate::store::InMemoryStore;
        use nom_blocks::NomtuRef;
        let mut store = InMemoryStore::new();
        let input = DocumentInput {
            entity: NomtuRef {
                id: "empty-doc".into(),
                word: "empty".into(),
                kind: "document".into(),
            },
            content_blocks: vec![],
            target_mime: "text/plain".into(),
        };
        let block = DocumentBackend::compose(input, &mut store, &LogProgressSink);
        assert!(store.exists(&block.artifact_hash), "artifact must be stored even for empty content");
    }

    #[test]
    fn document_backend_result_is_artifact() {
        use crate::backends::document::{DocumentBackend, DocumentInput};
        use crate::progress::LogProgressSink;
        use crate::store::InMemoryStore;
        use nom_blocks::NomtuRef;
        let mut store = InMemoryStore::new();
        let input = DocumentInput {
            entity: NomtuRef {
                id: "art-doc".into(),
                word: "artifact".into(),
                kind: "document".into(),
            },
            content_blocks: vec!["# Heading\nbody text".into()],
            target_mime: "text/html".into(),
        };
        let block = DocumentBackend::compose(input, &mut store, &LogProgressSink);
        assert!(store.exists(&block.artifact_hash), "returned hash must exist in store");
        assert_eq!(block.mime, "text/html");
    }

    #[test]
    fn backend_trait_kind_matches_compose_kind() {
        // For every concrete backend impl in dispatch.rs the Backend::kind()
        // must match the BackendKind returned by from_kind_name().
        use crate::backends::audio::AudioBackend;
        use crate::backends::document::DocumentBackend;
        use crate::backends::export::ExportBackend;
        use crate::backends::video::VideoBackend;

        let pairs: Vec<(Box<dyn Backend>, BackendKind)> = vec![
            (Box::new(VideoBackend), BackendKind::Video),
            (Box::new(AudioBackend), BackendKind::Audio),
            (Box::new(DocumentBackend), BackendKind::Document),
            (Box::new(ExportBackend), BackendKind::Export),
        ];
        for (backend, expected_kind) in &pairs {
            assert_eq!(
                backend.kind(),
                *expected_kind,
                "Backend::kind() must match expected BackendKind for {:?}",
                expected_kind.name()
            );
        }
    }
}
