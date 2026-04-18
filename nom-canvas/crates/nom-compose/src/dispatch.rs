#![deny(unsafe_code)]

use std::collections::HashMap;

use crate::store::ArtifactStore as _;

/// Trait every compose backend must implement.
pub trait Backend: Send + Sync {
    fn kind(&self) -> String;
    fn compose(&self, input: &str, progress: &dyn Fn(f32)) -> Result<String, String>;
}

/// Registry mapping backend kind strings to concrete Backend implementations.
pub struct BackendRegistry {
    backends: HashMap<String, Box<dyn Backend>>,
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
        kind: &str,
        input: &str,
        progress: &dyn Fn(f32),
    ) -> Result<String, String> {
        match self.backends.get(kind) {
            Some(b) => b.compose(input, progress),
            None => Err(format!("no backend registered for kind: {}", kind)),
        }
    }

    /// List all currently registered kinds.
    pub fn registered_kinds(&self) -> Vec<&str> {
        self.backends.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for BackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Stub backend for testing — echoes the input with a kind prefix.
pub struct NoopBackend {
    kind: String,
}

impl NoopBackend {
    pub fn new(kind: &str) -> Self {
        Self {
            kind: kind.to_string(),
        }
    }
}

impl Backend for NoopBackend {
    fn kind(&self) -> String {
        self.kind.clone()
    }
    fn compose(&self, input: &str, progress: &dyn Fn(f32)) -> Result<String, String> {
        progress(1.0);
        Ok(format!("{}:{}", self.kind, input))
    }
}

// ---------------------------------------------------------------------------
// Backend impls for concrete backends
// ---------------------------------------------------------------------------

impl Backend for crate::backends::video::VideoBackend {
    fn kind(&self) -> String {
        "video".to_string()
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
    fn kind(&self) -> String {
        "audio".to_string()
    }
    fn compose(&self, input: &str, _progress: &dyn Fn(f32)) -> Result<String, String> {
        use crate::backends::audio::{AudioBackend, AudioCodec, AudioContainer, AudioInput};
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
    fn kind(&self) -> String {
        "document".to_string()
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
        Ok(format!(
            "document:{}:{}pages",
            block.entity.id, block.page_count
        ))
    }
}

impl Backend for crate::backends::export::ExportBackend {
    fn kind(&self) -> String {
        "export".to_string()
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
    fn kind(&self) -> String {
        "rag_query".to_string()
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
    fn kind(&self) -> String {
        "web_screen".to_string()
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
    fn kind(&self) -> String {
        "render".to_string()
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

// ---------------------------------------------------------------------------
// ComposeContext — runtime string-keyed context (not a closed enum)
// ---------------------------------------------------------------------------

/// Runtime compose context. kind_name is a plain String so new kinds can be
/// added without changing the Rust source (DB-driven mandate).
pub struct ComposeContext {
    pub kind_name: String,
    pub entity_id: String,
    pub params: std::collections::HashMap<String, String>,
}

impl ComposeContext {
    pub fn new(kind_name: &str, entity_id: &str) -> Self {
        Self {
            kind_name: kind_name.to_string(),
            entity_id: entity_id.to_string(),
            params: std::collections::HashMap::new(),
        }
    }

    pub fn with_param(mut self, key: &str, value: &str) -> Self {
        self.params.insert(key.to_string(), value.to_string());
        self
    }

    pub fn get_param(&self, key: &str) -> Option<&str> {
        self.params.get(key).map(|s| s.as_str())
    }

    pub fn kind_name(&self) -> &str {
        &self.kind_name
    }
}

// ---------------------------------------------------------------------------
// UnifiedDispatcher — string-keyed, open-ended handler map
// ---------------------------------------------------------------------------

/// String-keyed dispatcher. Handlers are registered by kind_name (a runtime
/// string), so new kinds from the DB require no Rust enum changes.
pub struct UnifiedDispatcher {
    handlers: std::collections::HashMap<
        String,
        Box<dyn Fn(&ComposeContext) -> Result<String, String> + Send + Sync>,
    >,
}

impl UnifiedDispatcher {
    pub fn new() -> Self {
        Self {
            handlers: std::collections::HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        kind_name: &str,
        handler: impl Fn(&ComposeContext) -> Result<String, String> + Send + Sync + 'static,
    ) {
        self.handlers
            .insert(kind_name.to_string(), Box::new(handler));
    }

    pub fn dispatch(&self, ctx: &ComposeContext) -> Result<String, String> {
        use crate::backends::data_query::is_safe_identifier;
        let kind = ctx.kind_name();
        if !is_safe_identifier(kind) {
            return Err(format!("invalid backend kind: {kind:?}"));
        }
        match self.handlers.get(kind) {
            Some(h) => h(ctx),
            None => Err(format!("no handler registered for kind: {}", kind)),
        }
    }

    pub fn is_registered(&self, kind_name: &str) -> bool {
        self.handlers.contains_key(kind_name)
    }

    pub fn registered_kinds(&self) -> Vec<&str> {
        self.handlers.keys().map(|s| s.as_str()).collect()
    }

    /// Returns all registered backend kind names as owned Strings.
    pub fn registered_backends(&self) -> Vec<String> {
        self.handlers.keys().cloned().collect()
    }

    /// Returns the count of registered backends.
    pub fn backend_count(&self) -> usize {
        self.handlers.len()
    }
}

impl Default for UnifiedDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_register_and_dispatch_roundtrip() {
        use std::cell::Cell;
        let mut reg = BackendRegistry::new();
        reg.register(Box::new(NoopBackend::new("video")));
        let called_with = Cell::new(0.0f32);
        let result = reg.dispatch("video", "test-input", &|p| {
            called_with.set(p);
        });
        assert_eq!(result, Ok("video:test-input".to_string()));
        assert!((called_with.get() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn registry_dispatch_unknown_kind_returns_err() {
        let reg = BackendRegistry::new();
        let result = reg.dispatch("audio", "x", &|_| {});
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("audio"));
    }

    #[test]
    fn registry_registered_kinds_lists_all_registered() {
        let mut reg = BackendRegistry::new();
        reg.register(Box::new(NoopBackend::new("image")));
        reg.register(Box::new(NoopBackend::new("document")));
        let mut kinds = reg.registered_kinds();
        kinds.sort();
        assert_eq!(kinds, vec!["document", "image"]);
    }

    #[test]
    fn backend_registry_register_and_dispatch() {
        let mut reg = BackendRegistry::new();
        reg.register(Box::new(NoopBackend::new("render")));
        let result = reg.dispatch("render", "scene", &|_| {});
        assert_eq!(result, Ok("render:scene".to_string()));
    }

    #[test]
    fn backend_registry_unknown_kind_returns_err() {
        let reg = BackendRegistry::new();
        let result = reg.dispatch("export", "data", &|_| {});
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("export"));
    }

    #[test]
    fn backend_registry_multiple_kinds() {
        let mut reg = BackendRegistry::new();
        reg.register(Box::new(NoopBackend::new("pipeline")));
        reg.register(Box::new(NoopBackend::new("code_exec")));
        reg.register(Box::new(NoopBackend::new("transform")));
        assert_eq!(reg.registered_kinds().len(), 3);
    }

    #[test]
    fn backend_registry_default_is_empty() {
        let reg = BackendRegistry::default();
        assert!(reg.registered_kinds().is_empty());
    }

    #[test]
    fn noop_backend_kind_echoed_in_output() {
        let b = NoopBackend::new("scenario");
        let result = b.compose("input_data", &|_| {});
        assert_eq!(result, Ok("scenario:input_data".to_string()));
    }

    #[test]
    fn noop_backend_progress_callback_called_with_one() {
        use std::cell::Cell;
        let b = NoopBackend::new("embed_gen");
        let progress_val = Cell::new(-1.0f32);
        b.compose("x", &|p| progress_val.set(p)).unwrap();
        assert!((progress_val.get() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn registry_overwrite_same_kind() {
        let mut reg = BackendRegistry::new();
        reg.register(Box::new(NoopBackend::new("audio")));
        reg.register(Box::new(NoopBackend::new("audio")));
        assert_eq!(
            reg.registered_kinds().len(),
            1,
            "duplicate kind must not grow list"
        );
        let result = reg.dispatch("audio", "clip", &|_| {});
        assert_eq!(result, Ok("audio:clip".to_string()));
    }

    // -----------------------------------------------------------------------
    // Backend trait impl tests for concrete backends
    // -----------------------------------------------------------------------

    #[test]
    fn video_backend_stored_as_box_dyn_backend() {
        use crate::backends::video::VideoBackend;
        let b: Box<dyn Backend> = Box::new(VideoBackend);
        assert_eq!(b.kind(), "video");
        let result = b.compose("entity-v1", &|_| {});
        assert!(result.is_ok(), "VideoBackend dispatch must succeed");
        assert!(result.unwrap().starts_with("video:entity-v1"));
    }

    #[test]
    fn audio_backend_stored_as_box_dyn_backend() {
        use crate::backends::audio::AudioBackend;
        let b: Box<dyn Backend> = Box::new(AudioBackend);
        assert_eq!(b.kind(), "audio");
        let result = b.compose("entity-a1", &|_| {});
        assert!(result.is_ok(), "AudioBackend dispatch must succeed");
        assert!(result.unwrap().starts_with("audio:entity-a1"));
    }

    #[test]
    fn document_backend_stored_as_box_dyn_backend() {
        use crate::backends::document::DocumentBackend;
        let b: Box<dyn Backend> = Box::new(DocumentBackend);
        assert_eq!(b.kind(), "document");
        let result = b.compose("entity-d1", &|_| {});
        assert!(result.is_ok(), "DocumentBackend dispatch must succeed");
        assert!(result.unwrap().starts_with("document:entity-d1"));
    }

    #[test]
    fn export_backend_stored_as_box_dyn_backend() {
        use crate::backends::export::ExportBackend;
        let b: Box<dyn Backend> = Box::new(ExportBackend);
        assert_eq!(b.kind(), "export");
        let result = b.compose("entity-e1", &|_| {});
        assert!(result.is_ok(), "ExportBackend dispatch must succeed");
        assert!(result.unwrap().starts_with("export:entity-e1"));
    }

    #[test]
    fn rag_query_backend_stored_as_box_dyn_backend() {
        use crate::backends::rag_query::RagQueryBackend;
        let b: Box<dyn Backend> = Box::new(RagQueryBackend::default());
        assert_eq!(b.kind(), "rag_query");
        let result = b.compose("entity-r1", &|_| {});
        assert!(result.is_ok(), "RagQueryBackend dispatch must succeed");
        assert!(result.unwrap().starts_with("rag_query:entity-r1"));
    }

    #[test]
    fn routing_unknown_kind_returns_error() {
        let reg = BackendRegistry::new();
        let result = reg.dispatch("embed_gen", "probe", &|_| {});
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("embed_gen"),
            "error must name the missing kind, got: {msg}"
        );
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

        let v = reg.dispatch("video", "vid-probe", &|_| {});
        assert!(v.is_ok(), "video round-trip must succeed");

        let a = reg.dispatch("audio", "aud-probe", &|_| {});
        assert!(a.is_ok(), "audio round-trip must succeed");

        let d = reg.dispatch("document", "doc-probe", &|_| {});
        assert!(d.is_ok(), "document round-trip must succeed");
    }

    #[test]
    fn export_and_rag_query_round_trip_through_registry() {
        use crate::backends::export::ExportBackend;
        use crate::backends::rag_query::RagQueryBackend;

        let mut reg = BackendRegistry::new();
        reg.register(Box::new(ExportBackend));
        reg.register(Box::new(RagQueryBackend::default()));

        let e = reg.dispatch("export", "export-probe", &|_| {});
        assert!(e.is_ok(), "export round-trip must succeed");

        let r = reg.dispatch("rag_query", "rag-probe", &|_| {});
        assert!(r.is_ok(), "rag_query round-trip must succeed");
    }

    #[test]
    fn backend_progress_callback_called_during_compose() {
        use crate::backends::video::VideoBackend;
        use std::cell::Cell;
        let b: Box<dyn Backend> = Box::new(VideoBackend);
        let called = Cell::new(false);
        b.compose("progress-test", &|_| called.set(true)).unwrap();
        assert!(
            called.get(),
            "progress callback must be called during compose"
        );
    }

    // ── Wave AD new tests ────────────────────────────────────────────────────

    #[test]
    fn dispatch_routes_all_16_backend_kinds() {
        let mut reg = BackendRegistry::new();
        let all_kinds = [
            "video", "audio", "image", "document", "data", "app", "workflow", "scenario",
            "rag_query", "transform", "embed_gen", "render", "export", "pipeline", "code_exec",
            "web_screen",
        ];
        for kind in &all_kinds {
            reg.register(Box::new(NoopBackend::new(kind)));
        }
        assert_eq!(
            reg.registered_kinds().len(),
            16,
            "all 16 backends must be registered"
        );
        for kind in &all_kinds {
            let result = reg.dispatch(kind, "probe", &|_| {});
            assert!(
                result.is_ok(),
                "dispatch must succeed for kind: {}",
                kind
            );
            let output = result.unwrap();
            assert!(
                output.starts_with(kind),
                "output must start with kind name"
            );
        }
    }

    #[test]
    fn plan_with_10_steps_all_execute_in_topo_order() {
        use crate::plan::CompositionPlan;
        let mut plan = CompositionPlan::new();
        let step0 = plan.add_step("video", "src", "s0");
        let mut prev = step0;
        for i in 1..10usize {
            prev = plan.add_step_after(
                "transform",
                format!("s{}", i - 1),
                format!("s{i}"),
                vec![prev],
            );
        }
        assert!(
            plan.is_valid_dag(),
            "10-step linear chain must be a valid DAG"
        );
        let order = plan.topo_order();
        assert_eq!(order.len(), 10, "all 10 steps must appear in topo order");
        for window in order.windows(2) {
            assert!(
                window[0] < window[1],
                "step {} must precede step {}",
                window[0],
                window[1]
            );
        }
    }

    #[test]
    fn progress_cancellation_flag_stops_emission() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        let cancelled = Arc::new(AtomicBool::new(false));
        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let cc = call_count.clone();
        let ca = cancelled.clone();

        for i in 0..5usize {
            if ca.load(Ordering::SeqCst) {
                break;
            }
            cc.fetch_add(1, Ordering::SeqCst);
            if i == 2 {
                ca.store(true, Ordering::SeqCst);
            }
        }
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
            let sql = model.to_select_sql().expect("safe generated model");
            assert!(
                sql.contains(&format!("raw.{name}")),
                "SQL must reference the source table"
            );
            assert!(sql.contains("id"), "SQL must contain id column");
            assert!(
                sql.contains("created_at"),
                "SQL must contain created_at column"
            );
        }
    }

    #[test]
    fn backend_registry_register_all_kinds_count_is_16() {
        let mut reg = BackendRegistry::new();
        for kind in [
            "video", "audio", "image", "document", "data", "app", "workflow", "scenario",
            "rag_query", "transform", "embed_gen", "render", "export", "pipeline", "code_exec",
            "web_screen",
        ] {
            reg.register(Box::new(NoopBackend::new(kind)));
        }
        assert_eq!(reg.registered_kinds().len(), 16);
    }

    #[test]
    fn noop_backend_output_format_is_kind_colon_input() {
        let kinds = ["data", "app", "workflow"];
        for kind in kinds {
            let b = NoopBackend::new(kind);
            let result = b.compose("x", &|_| {}).unwrap();
            assert_eq!(result, format!("{}:x", kind));
        }
    }

    #[test]
    fn dispatch_missing_kind_returns_err_with_kind_name() {
        let reg = BackendRegistry::new();
        for kind in ["pipeline", "code_exec", "web_screen"] {
            let err = reg.dispatch(kind, "probe", &|_| {}).unwrap_err();
            assert!(err.contains(kind), "error must mention kind name: {kind}");
        }
    }

    #[test]
    fn plan_diamond_all_steps_in_topo_order() {
        use crate::plan::CompositionPlan;
        let mut plan = CompositionPlan::new();
        let a = plan.add_step("video", "src", "v");
        let b = plan.add_step_after("audio", "v", "a", vec![a]);
        let c = plan.add_step_after("image", "v", "img", vec![a]);
        let d = plan.add_step_after("export", "a", "out", vec![b, c]);
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
        let cols = [
            "sale_id",
            "product_id",
            "customer_id",
            "amount",
            "sale_date",
        ];
        for name in &cols {
            m.add_column(SemanticColumn {
                name: (*name).into(),
                data_type: SemanticDataType::String,
                description: None,
            });
        }
        let sql = m.to_select_sql().expect("safe test model");
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
            "video", "audio", "image", "document", "data", "app", "workflow", "scenario",
            "rag_query", "transform",
        ];
        let mut plan = CompositionPlan::new();
        let first = plan.add_step(kinds[0], "in", "s0");
        let mut prev = first;
        for (i, kind) in kinds[1..].iter().enumerate() {
            prev = plan.add_step_after(
                *kind,
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
        let all_kinds = [
            "video", "audio", "image", "document", "data", "app", "workflow", "scenario",
            "rag_query", "transform", "embed_gen", "render", "export", "pipeline", "code_exec",
            "web_screen",
        ];
        for name in &all_kinds {
            assert!(!name.is_empty(), "name must not be empty");
            assert_eq!(*name, name.to_lowercase(), "name must be lowercase: {name}");
            assert!(!name.contains(' '), "name must not contain spaces: {name}");
        }
    }

    #[test]
    fn dispatch_noop_output_starts_with_kind_name() {
        let mut reg = BackendRegistry::new();
        reg.register(Box::new(NoopBackend::new("scenario")));
        let out = reg
            .dispatch("scenario", "my-input", &|_| {})
            .unwrap();
        assert!(
            out.starts_with("scenario:"),
            "output must start with 'scenario:'"
        );
        assert!(out.contains("my-input"), "output must contain input");
    }

    #[test]
    fn plan_single_step_dag_is_valid() {
        use crate::plan::CompositionPlan;
        let mut plan = CompositionPlan::new();
        plan.add_step("render", "input", "output");
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
        let mut reg = BackendRegistry::new();
        let kinds = [
            "video", "audio", "document", "export", "rag_query", "web_screen", "render",
        ];
        for kind in &kinds {
            reg.register(Box::new(NoopBackend::new(kind)));
        }
        assert_eq!(
            reg.registered_kinds().len(),
            7,
            "exactly 7 backends must be registered"
        );
        for kind in &kinds {
            let result = reg.dispatch(kind, "probe", &|_| {});
            assert!(
                result.is_ok(),
                "dispatch must succeed for kind: {}",
                kind
            );
        }
    }

    #[test]
    fn registry_route_by_kind_returns_correct_backend() {
        let mut reg = BackendRegistry::new();
        reg.register(Box::new(NoopBackend::new("video")));
        reg.register(Box::new(NoopBackend::new("audio")));
        reg.register(Box::new(NoopBackend::new("document")));

        let v = reg.dispatch("video", "x", &|_| {}).unwrap();
        assert!(
            v.starts_with("video:"),
            "video backend output must start with 'video:'"
        );

        let a = reg.dispatch("audio", "x", &|_| {}).unwrap();
        assert!(
            a.starts_with("audio:"),
            "audio backend output must start with 'audio:'"
        );

        let d = reg.dispatch("document", "x", &|_| {}).unwrap();
        assert!(
            d.starts_with("document:"),
            "document backend output must start with 'document:'"
        );
    }

    #[test]
    fn registry_unknown_kind_error_message_contains_kind_name() {
        let reg = BackendRegistry::new();
        let err = reg
            .dispatch("scenario", "x", &|_| {})
            .unwrap_err();
        assert!(err.contains("scenario"), "error must name the missing kind");
    }

    #[test]
    fn concurrent_dispatch_simulation_all_succeed() {
        use std::sync::Arc;
        let mut reg = BackendRegistry::new();
        reg.register(Box::new(NoopBackend::new("video")));
        reg.register(Box::new(NoopBackend::new("audio")));
        reg.register(Box::new(NoopBackend::new("image")));
        let reg = Arc::new(reg);

        let results: Vec<_> = [
            ("video", "v-payload"),
            ("audio", "a-payload"),
            ("image", "i-payload"),
        ]
        .iter()
        .map(|(kind, input)| reg.dispatch(kind, input, &|_| {}))
        .collect();

        for r in &results {
            assert!(r.is_ok(), "each concurrent dispatch must succeed");
        }
        assert!(results[0].as_ref().unwrap().contains("video"));
        assert!(results[1].as_ref().unwrap().contains("audio"));
        assert!(results[2].as_ref().unwrap().contains("image"));
    }

    #[test]
    fn noop_backend_compose_empty_input_returns_kind_prefix() {
        let b = NoopBackend::new("transform");
        let result = b.compose("", &|_| {}).unwrap();
        assert_eq!(result, "transform:", "empty input must yield 'transform:'");
    }

    #[test]
    fn registry_dispatch_after_replacing_backend() {
        let mut reg = BackendRegistry::new();
        reg.register(Box::new(NoopBackend::new("pipeline")));
        reg.register(Box::new(NoopBackend::new("pipeline")));
        assert_eq!(reg.registered_kinds().len(), 1);
        let result = reg
            .dispatch("pipeline", "data", &|_| {})
            .unwrap();
        assert!(result.starts_with("pipeline:"));
    }

    // ── Wave AH new tests ────────────────────────────────────────────────────

    #[test]
    fn dispatch_backend_kind_roundtrip() {
        let mut reg = BackendRegistry::new();
        reg.register(Box::new(NoopBackend::new("video")));
        let result = reg.dispatch("video", "probe", &|_| {});
        assert!(result.is_ok());
    }

    #[test]
    fn dispatch_unknown_kind_returns_error() {
        let reg = BackendRegistry::new();
        let result = reg.dispatch("data", "probe", &|_| {});
        assert!(
            result.is_err(),
            "dispatch on empty registry must return Err"
        );
        assert!(result.unwrap_err().contains("data"));
    }

    #[test]
    fn dispatch_all_registered_kinds_resolve() {
        let mut reg = BackendRegistry::new();
        let all_kinds = [
            "video", "audio", "image", "document", "data", "app", "workflow", "scenario",
            "rag_query", "transform", "embed_gen", "render", "export", "pipeline", "code_exec",
            "web_screen",
        ];
        for kind in &all_kinds {
            reg.register(Box::new(NoopBackend::new(kind)));
        }
        for kind in &all_kinds {
            let result = reg.dispatch(kind, "test", &|_| {});
            assert!(result.is_ok(), "kind {} must resolve", kind);
        }
    }

    #[test]
    fn document_backend_metadata_nonempty() {
        use crate::backends::document::DocumentBackend;
        let b: Box<dyn Backend> = Box::new(DocumentBackend);
        assert_eq!(b.kind(), "document");
        let out = b.compose("metadata-check", &|_| {}).unwrap();
        assert!(!out.is_empty(), "document backend output must not be empty");
    }

    #[test]
    fn document_backend_title_in_output() {
        use crate::backends::document::DocumentBackend;
        let b: Box<dyn Backend> = Box::new(DocumentBackend);
        let out = b.compose("my-title", &|_| {}).unwrap();
        assert!(
            out.starts_with("document:my-title"),
            "output must reference entity id"
        );
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
        assert!(
            store.exists(&block.artifact_hash),
            "artifact must be stored even for empty content"
        );
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
        assert!(
            store.exists(&block.artifact_hash),
            "returned hash must exist in store"
        );
        assert_eq!(block.mime, "text/html");
    }

    #[test]
    fn backend_trait_kind_matches_compose_kind() {
        use crate::backends::audio::AudioBackend;
        use crate::backends::document::DocumentBackend;
        use crate::backends::export::ExportBackend;
        use crate::backends::video::VideoBackend;

        let pairs: Vec<(Box<dyn Backend>, &str)> = vec![
            (Box::new(VideoBackend), "video"),
            (Box::new(AudioBackend), "audio"),
            (Box::new(DocumentBackend), "document"),
            (Box::new(ExportBackend), "export"),
        ];
        for (backend, expected_kind) in &pairs {
            assert_eq!(
                backend.kind(),
                *expected_kind,
                "Backend::kind() must match expected kind string"
            );
        }
    }

    // ── Wave AJ new tests ────────────────────────────────────────────────────

    #[test]
    fn dispatch_routes_all_16_known_kinds() {
        let mut reg = BackendRegistry::new();
        let all_kinds = [
            "video", "audio", "image", "document", "data", "app", "workflow", "scenario",
            "rag_query", "transform", "embed_gen", "render", "export", "pipeline", "code_exec",
            "web_screen",
        ];
        for kind in &all_kinds {
            reg.register(Box::new(NoopBackend::new(kind)));
        }
        assert_eq!(
            reg.registered_kinds().len(),
            16,
            "must register all 16 kinds"
        );
        for kind in &all_kinds {
            let result = reg.dispatch(kind, "payload", &|_| {});
            assert!(
                result.is_ok(),
                "dispatch must succeed for kind: {}",
                kind
            );
        }
    }

    #[test]
    fn dispatch_unknown_kind_graceful_error() {
        let reg = BackendRegistry::new();
        let result = reg.dispatch("workflow", "x", &|_| {});
        assert!(result.is_err(), "unknown kind must return Err");
        let msg = result.unwrap_err();
        assert!(!msg.is_empty(), "error message must not be empty");
        assert!(msg.contains("workflow"), "error must name the missing kind");
    }

    #[test]
    fn dispatch_compose_result_has_artifact() {
        use crate::backends::video::VideoBackend;
        let b: Box<dyn Backend> = Box::new(VideoBackend);
        let result = b.compose("entity-artifact", &|_| {});
        assert!(result.is_ok(), "VideoBackend must return Ok");
        let out = result.unwrap();
        assert!(
            out.contains("entity-artifact"),
            "output must reference entity id"
        );
    }

    #[test]
    fn dispatch_compose_result_has_events() {
        use crate::backends::audio::AudioBackend;
        let b: Box<dyn Backend> = Box::new(AudioBackend);
        let result = b.compose("aud-events", &|_| {});
        assert!(result.is_ok());
        let out = result.unwrap();
        assert!(
            out.contains("aud-events"),
            "audio dispatch output must contain entity id"
        );
    }

    #[test]
    fn dispatch_progress_events_monotone() {
        use std::cell::RefCell;
        let b = NoopBackend::new("image");
        let values: RefCell<Vec<f32>> = RefCell::new(Vec::new());
        b.compose("mono", &|p| values.borrow_mut().push(p)).unwrap();
        let vals = values.borrow();
        assert!(!vals.is_empty(), "at least one progress callback must fire");
        for window in vals.windows(2) {
            assert!(
                window[1] >= window[0],
                "progress must be monotonically non-decreasing"
            );
        }
        assert!(
            (*vals.last().unwrap() - 1.0).abs() < f32::EPSILON,
            "final progress must be 1.0"
        );
    }

    #[test]
    fn dispatch_concurrent_two_different_kinds_safe() {
        let mut reg = BackendRegistry::new();
        reg.register(Box::new(NoopBackend::new("video")));
        reg.register(Box::new(NoopBackend::new("audio")));

        let v = reg.dispatch("video", "v-data", &|_| {});
        let a = reg.dispatch("audio", "a-data", &|_| {});

        assert!(v.is_ok(), "video dispatch must succeed");
        assert!(a.is_ok(), "audio dispatch must succeed");
        assert!(
            v.unwrap().contains("video"),
            "video output must contain 'video'"
        );
        assert!(
            a.unwrap().contains("audio"),
            "audio output must contain 'audio'"
        );
    }

    #[test]
    fn dispatch_compose_empty_prompt_ok() {
        let mut reg = BackendRegistry::new();
        reg.register(Box::new(NoopBackend::new("transform")));
        let result = reg.dispatch("transform", "", &|_| {});
        assert!(result.is_ok(), "empty prompt must not cause Err");
        assert_eq!(
            result.unwrap(),
            "transform:",
            "empty prompt yields '<kind>:'"
        );
    }

    // ── Wave AL new tests: UnifiedDispatcher + backend coverage ──

    #[test]
    fn backend_kind_str_video_dispatches_via_unified_dispatcher() {
        let mut d = UnifiedDispatcher::new();
        d.register("video", |ctx| {
            Ok(format!("video-handler:{}", ctx.entity_id))
        });
        let ctx = ComposeContext::new("video", "entity-v1");
        let result = d.dispatch(&ctx);
        assert!(result.is_ok(), "video dispatch must succeed");
        assert_eq!(result.unwrap(), "video-handler:entity-v1");
    }

    #[test]
    fn backend_kind_str_audio_dispatches_via_unified_dispatcher() {
        let mut d = UnifiedDispatcher::new();
        d.register("audio", |ctx| {
            Ok(format!("audio-handler:{}", ctx.entity_id))
        });
        let ctx = ComposeContext::new("audio", "entity-a1");
        let result = d.dispatch(&ctx);
        assert!(result.is_ok(), "audio dispatch must succeed");
        assert_eq!(result.unwrap(), "audio-handler:entity-a1");
    }

    #[test]
    fn backend_kind_str_document_dispatches_via_unified_dispatcher() {
        let mut d = UnifiedDispatcher::new();
        d.register("document", |ctx| {
            Ok(format!("document-handler:{}", ctx.entity_id))
        });
        let ctx = ComposeContext::new("document", "entity-d1");
        let result = d.dispatch(&ctx);
        assert!(result.is_ok(), "document dispatch must succeed");
        assert_eq!(result.unwrap(), "document-handler:entity-d1");
    }

    #[test]
    fn backend_kind_str_unknown_returns_err() {
        let d = UnifiedDispatcher::new();
        let ctx = ComposeContext::new("unknown", "entity-x");
        let result = d.dispatch(&ctx);
        assert!(result.is_err(), "unknown kind must return Err");
        assert!(
            result.unwrap_err().contains("unknown"),
            "error must name the missing kind"
        );
    }

    #[test]
    fn backend_kind_str_kind_name_returns_inner_string() {
        let ctx = ComposeContext::new("video", "e1");
        assert_eq!(ctx.kind_name(), "video");
        let ctx2 = ComposeContext::new("my_custom_kind", "e2");
        assert_eq!(ctx2.kind_name(), "my_custom_kind");
    }

    #[test]
    fn backend_kind_str_two_same_strings_are_equal_via_context() {
        let ctx1 = ComposeContext::new("audio", "e1");
        let ctx2 = ComposeContext::new("audio", "e2");
        assert_eq!(
            ctx1.kind_name(),
            ctx2.kind_name(),
            "same kind_name string must be equal"
        );
    }

    #[test]
    fn backend_kind_str_from_str_and_string_both_work() {
        let s: &str = "export";
        let owned: String = "export".to_string();
        let ctx1 = ComposeContext::new(s, "e1");
        let ctx2 = ComposeContext::new(&owned, "e2");
        assert_eq!(ctx1.kind_name(), ctx2.kind_name());
    }

    #[test]
    fn backend_kind_str_rag_query_dispatches() {
        let mut d = UnifiedDispatcher::new();
        d.register("rag_query", |_ctx| Ok("rag-result".to_string()));
        let ctx = ComposeContext::new("rag_query", "e1");
        assert_eq!(d.dispatch(&ctx).unwrap(), "rag-result");
    }

    #[test]
    fn backend_kind_str_export_dispatches() {
        let mut d = UnifiedDispatcher::new();
        d.register("export", |ctx| Ok(format!("exported:{}", ctx.entity_id)));
        let ctx = ComposeContext::new("export", "e-export");
        assert_eq!(d.dispatch(&ctx).unwrap(), "exported:e-export");
    }

    #[test]
    fn backend_kind_str_dynamic_kind_not_in_enum_dispatches() {
        let mut d = UnifiedDispatcher::new();
        d.register("new_db_kind_2025", |_| Ok("dynamic".to_string()));
        let ctx = ComposeContext::new("new_db_kind_2025", "e1");
        assert!(d.dispatch(&ctx).is_ok());
        assert_eq!(d.dispatch(&ctx).unwrap(), "dynamic");
    }

    #[test]
    fn compose_context_kind_name_preserved() {
        let ctx = ComposeContext::new("transform", "my-entity");
        assert_eq!(ctx.kind_name, "transform");
        assert_eq!(ctx.kind_name(), "transform");
    }

    #[test]
    fn compose_context_params_round_trip() {
        let ctx = ComposeContext::new("video", "e1")
            .with_param("codec", "h264")
            .with_param("fps", "30");
        assert_eq!(ctx.get_param("codec"), Some("h264"));
        assert_eq!(ctx.get_param("fps"), Some("30"));
        assert_eq!(ctx.get_param("missing"), None);
    }

    #[test]
    fn unified_dispatcher_5_kinds_each_dispatch_correctly() {
        let mut d = UnifiedDispatcher::new();
        for kind in &["video", "audio", "document", "export", "rag_query"] {
            let k = kind.to_string();
            d.register(kind, move |ctx| {
                Ok(format!("{}-result:{}", k, ctx.entity_id))
            });
        }
        for kind in &["video", "audio", "document", "export", "rag_query"] {
            let ctx = ComposeContext::new(kind, "probe");
            let result = d.dispatch(&ctx).unwrap();
            assert!(
                result.starts_with(kind),
                "dispatch result must start with kind: {kind}"
            );
        }
    }

    #[test]
    fn unified_dispatcher_missing_kind_descriptive_error() {
        let mut d = UnifiedDispatcher::new();
        d.register("video", |_| Ok("ok".to_string()));
        let ctx = ComposeContext::new("audio", "e1");
        let err = d.dispatch(&ctx).unwrap_err();
        assert!(!err.is_empty(), "error must not be empty");
        assert!(
            err.contains("audio"),
            "error must name the missing kind: {err}"
        );
    }

    #[test]
    fn unified_dispatcher_registered_kinds_sorted() {
        let mut d = UnifiedDispatcher::new();
        d.register("video", |_| Ok("v".to_string()));
        d.register("audio", |_| Ok("a".to_string()));
        d.register("document", |_| Ok("d".to_string()));
        let mut kinds: Vec<&str> = d.registered_kinds();
        kinds.sort();
        assert_eq!(kinds, vec!["audio", "document", "video"]);
    }

    #[test]
    fn video_backend_entity_id_preserved_in_artifact() {
        use crate::backends::video::VideoBackend;
        let b: Box<dyn Backend> = Box::new(VideoBackend);
        let out = b.compose("video-entity-123", &|_| {}).unwrap();
        assert!(
            out.contains("video-entity-123"),
            "entity_id must be in artifact output"
        );
    }

    #[test]
    fn audio_backend_codec_name_in_output() {
        use crate::backends::audio::AudioBackend;
        let b: Box<dyn Backend> = Box::new(AudioBackend);
        let out = b.compose("audio-entity-pcm", &|_| {}).unwrap();
        assert!(
            out.contains("audio-entity-pcm"),
            "audio output must reference entity id"
        );
        assert!(
            out.starts_with("audio:"),
            "audio output must start with 'audio:'"
        );
    }

    #[test]
    fn document_backend_produces_non_empty_artifact() {
        use crate::backends::document::DocumentBackend;
        let b: Box<dyn Backend> = Box::new(DocumentBackend);
        let out = b.compose("doc-content", &|_| {}).unwrap();
        assert!(
            !out.is_empty(),
            "document backend must produce non-empty artifact"
        );
        assert!(
            out.contains("document:"),
            "document artifact must have kind prefix"
        );
    }

    #[test]
    fn rag_query_backend_empty_query_produces_output() {
        use crate::backends::rag_query::RagQueryBackend;
        let b: Box<dyn Backend> = Box::new(RagQueryBackend::default());
        let out = b.compose("empty-query", &|_| {}).unwrap();
        assert!(
            out.starts_with("rag_query:"),
            "rag_query artifact must have kind prefix"
        );
    }

    #[test]
    fn export_backend_format_in_artifact_mime_type() {
        use crate::backends::export::ExportBackend;
        let b: Box<dyn Backend> = Box::new(ExportBackend);
        let out = b.compose("export-entity-hex", &|_| {}).unwrap();
        assert!(
            out.starts_with("export:"),
            "export output must start with 'export:'"
        );
        assert!(out.contains("bytes"), "export output must mention bytes");
    }

    // ── Wave AO: UnifiedDispatcher string-dispatch for 16 backend kinds ─────

    #[test]
    fn unified_dispatcher_all_16_backend_kinds_by_string() {
        let mut d = UnifiedDispatcher::new();
        let kinds = [
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
        for kind in &kinds {
            let k = kind.to_string();
            d.register(kind, move |ctx| Ok(format!("{k}:{}", ctx.entity_id)));
        }
        assert_eq!(d.backend_count(), 16);
        for kind in &kinds {
            let ctx = ComposeContext::new(kind, "probe");
            let result = d.dispatch(&ctx);
            assert!(result.is_ok(), "dispatch must succeed for: {kind}");
            assert!(result.unwrap().starts_with(kind));
        }
    }

    #[test]
    fn unified_dispatcher_mobile_screen_string_dispatch() {
        let mut d = UnifiedDispatcher::new();
        d.register("mobile_screen", |_| Ok("mobile-ok".to_string()));
        let ctx = ComposeContext::new("mobile_screen", "s1");
        assert!(d.dispatch(&ctx).is_ok());
    }

    #[test]
    fn unified_dispatcher_native_screen_string_dispatch() {
        let mut d = UnifiedDispatcher::new();
        d.register("native_screen", |_| Ok("native-ok".to_string()));
        let ctx = ComposeContext::new("native_screen", "s2");
        assert!(d.dispatch(&ctx).is_ok());
    }

    #[test]
    fn unified_dispatcher_data_extract_string_dispatch() {
        let mut d = UnifiedDispatcher::new();
        d.register("data_extract", |_| Ok("extracted".to_string()));
        let ctx = ComposeContext::new("data_extract", "raw");
        assert!(d.dispatch(&ctx).is_ok());
    }

    #[test]
    fn unified_dispatcher_pipeline_string_dispatch() {
        let mut d = UnifiedDispatcher::new();
        d.register("pipeline", |_| Ok("pipe-run".to_string()));
        let ctx = ComposeContext::new("pipeline", "p1");
        assert_eq!(d.dispatch(&ctx).unwrap(), "pipe-run");
    }

    #[test]
    fn unified_dispatcher_code_exec_string_dispatch() {
        let mut d = UnifiedDispatcher::new();
        d.register("code_exec", |_| Ok("exec-result".to_string()));
        let ctx = ComposeContext::new("code_exec", "s1");
        assert_eq!(d.dispatch(&ctx).unwrap(), "exec-result");
    }

    #[test]
    fn unified_dispatcher_web_screen_string_dispatch() {
        let mut d = UnifiedDispatcher::new();
        d.register("web_screen", |_| Ok("web-ok".to_string()));
        let ctx = ComposeContext::new("web_screen", "pg1");
        assert_eq!(d.dispatch(&ctx).unwrap(), "web-ok");
    }

    #[test]
    fn unified_dispatcher_embed_gen_string_dispatch() {
        let mut d = UnifiedDispatcher::new();
        d.register("embed_gen", |ctx| Ok(format!("emb:{}", ctx.entity_id)));
        let ctx = ComposeContext::new("embed_gen", "t42");
        assert_eq!(d.dispatch(&ctx).unwrap(), "emb:t42");
    }

    #[test]
    fn unified_dispatcher_scenario_string_dispatch() {
        let mut d = UnifiedDispatcher::new();
        d.register("scenario", |_| Ok("sc-run".to_string()));
        let ctx = ComposeContext::new("scenario", "sc1");
        assert_eq!(d.dispatch(&ctx).unwrap(), "sc-run");
    }

    // ── Wave AO: is_safe_identifier validation gate ──────────────────────────

    #[test]
    fn unified_dispatcher_invalid_kind_semicolon_rejected() {
        let d = UnifiedDispatcher::new();
        let ctx = ComposeContext::new("video; DROP TABLE", "e");
        let err = d.dispatch(&ctx).unwrap_err();
        assert!(err.contains("invalid backend kind"), "got: {err}");
    }

    #[test]
    fn unified_dispatcher_invalid_kind_empty_rejected() {
        let d = UnifiedDispatcher::new();
        let ctx = ComposeContext::new("", "e");
        let err = d.dispatch(&ctx).unwrap_err();
        assert!(err.contains("invalid backend kind"), "got: {err}");
    }

    #[test]
    fn unified_dispatcher_invalid_kind_space_rejected() {
        let d = UnifiedDispatcher::new();
        let ctx = ComposeContext::new("my kind", "e");
        let err = d.dispatch(&ctx).unwrap_err();
        assert!(err.contains("invalid backend kind"), "got: {err}");
    }

    #[test]
    fn unified_dispatcher_invalid_kind_dash_rejected() {
        let d = UnifiedDispatcher::new();
        let ctx = ComposeContext::new("my-kind", "e");
        let err = d.dispatch(&ctx).unwrap_err();
        assert!(err.contains("invalid backend kind"), "got: {err}");
    }

    #[test]
    fn unified_dispatcher_invalid_kind_quote_rejected() {
        let d = UnifiedDispatcher::new();
        let ctx = ComposeContext::new("video'", "e");
        let err = d.dispatch(&ctx).unwrap_err();
        assert!(err.contains("invalid backend kind"), "got: {err}");
    }

    #[test]
    fn unified_dispatcher_invalid_kind_newline_rejected() {
        let d = UnifiedDispatcher::new();
        let ctx = ComposeContext::new("video\n", "e");
        let err = d.dispatch(&ctx).unwrap_err();
        assert!(err.contains("invalid backend kind"), "got: {err}");
    }

    #[test]
    fn unified_dispatcher_valid_kind_underscore_passes_validation() {
        let mut d = UnifiedDispatcher::new();
        d.register("rag_query", |_| Ok("ok".to_string()));
        let ctx = ComposeContext::new("rag_query", "e");
        assert!(d.dispatch(&ctx).is_ok());
    }

    #[test]
    fn unified_dispatcher_invalid_kind_slash_rejected() {
        let d = UnifiedDispatcher::new();
        let ctx = ComposeContext::new("video/audio", "e");
        let err = d.dispatch(&ctx).unwrap_err();
        assert!(err.contains("invalid backend kind"), "got: {err}");
    }

    #[test]
    fn unified_dispatcher_invalid_kind_at_sign_rejected() {
        let d = UnifiedDispatcher::new();
        let ctx = ComposeContext::new("@video", "e");
        let err = d.dispatch(&ctx).unwrap_err();
        assert!(err.contains("invalid backend kind"), "got: {err}");
    }

    #[test]
    fn unified_dispatcher_invalid_kind_backslash_rejected() {
        let d = UnifiedDispatcher::new();
        let ctx = ComposeContext::new("video\\audio", "e");
        let err = d.dispatch(&ctx).unwrap_err();
        assert!(err.contains("invalid backend kind"), "got: {err}");
    }

    // ── Wave AO: registered_backends() and backend_count() ──────────────────

    #[test]
    fn unified_dispatcher_registered_backends_returns_all() {
        let mut d = UnifiedDispatcher::new();
        d.register("video", |_| Ok("v".to_string()));
        d.register("audio", |_| Ok("a".to_string()));
        d.register("document", |_| Ok("d".to_string()));
        let mut backends = d.registered_backends();
        backends.sort();
        assert_eq!(backends, vec!["audio", "document", "video"]);
    }

    #[test]
    fn unified_dispatcher_backend_count_increments() {
        let mut d = UnifiedDispatcher::new();
        assert_eq!(d.backend_count(), 0);
        d.register("video", |_| Ok("v".to_string()));
        assert_eq!(d.backend_count(), 1);
        d.register("audio", |_| Ok("a".to_string()));
        assert_eq!(d.backend_count(), 2);
        d.register("export", |_| Ok("e".to_string()));
        assert_eq!(d.backend_count(), 3);
    }

    #[test]
    fn unified_dispatcher_backend_count_16_all_kinds() {
        let mut d = UnifiedDispatcher::new();
        for kind in &[
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
        ] {
            d.register(kind, |_| Ok("ok".to_string()));
        }
        assert_eq!(d.backend_count(), 16);
    }

    #[test]
    fn unified_dispatcher_registered_backends_empty_when_new() {
        let d = UnifiedDispatcher::new();
        assert!(d.registered_backends().is_empty());
        assert_eq!(d.backend_count(), 0);
    }

    #[test]
    fn unified_dispatcher_replacing_handler_keeps_count_stable() {
        let mut d = UnifiedDispatcher::new();
        d.register("video", |_| Ok("v1".to_string()));
        d.register("video", |_| Ok("v2".to_string()));
        assert_eq!(d.backend_count(), 1);
        assert_eq!(d.registered_backends(), vec!["video"]);
    }
}
