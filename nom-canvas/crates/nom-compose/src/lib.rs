#![deny(unsafe_code)]
pub mod ai_compiler_loop;
pub mod author_session;
pub mod animate;
pub mod backends;
pub mod cancel;
pub mod cancellation;
pub mod composition;
pub mod context;
pub mod credential_store;
pub mod deep_think;
pub mod dream_tree;
pub mod detection;
pub mod diffusion;
pub mod dispatch;
pub mod flow_graph;
pub mod glue;
pub mod glue_cache;
pub mod hybrid;
pub mod middleware;
pub mod orchestrator;
pub mod plan;
pub mod progress;
pub mod provider_router;
pub mod semantic;
pub mod store;
pub mod streaming;
pub mod task_queue;
pub mod timeline;
pub mod vendor_trait;

pub use backends::{
    ad_creative::{AdCreativeSpec, AdFormat},
    app::AppBackend,
    app_bundle::{AppBundleSpec, AppTarget},
    audio::AudioBackend,
    code_exec::CodeExecBackend,
    data::DataBackend,
    data_extract::{DataExtractSpec, ExtractMode},
    data_frame::{DataFrame, DataFrameSpec},
    data_loader::{DataBatch, DataLoader, DataLoaderConfig, DataSourceKind as DataLoaderSourceKind, LoadStrategy},
    data_query::DataQuerySpec,
    document::DocumentBackend,
    embed_gen::EmbedGenBackend,
    export::ExportBackend,
    image::{BlendMode, ImageBackend, ImageComposite, ImageLayer, PixelFormat},
    mesh::{MeshBackend, MeshPrimitive, MeshSpec},
    mobile_screen::{MobileScreenBackend, MobileScreenSpec},
    native_screen::{NativeScreenBackend, NativeScreenSpec},
    pipeline::PipelineBackend,
    presentation::{PresentationSlide, PresentationSpec},
    rag_query::RagQueryBackend,
    render::RenderBackend,
    scenario::ScenarioBackend,
    scenario_workflow::{compose as compose_scenario_workflow, ScenarioWorkflowSpec},
    storyboard::{
        Storyboard, StoryboardBackend, StoryboardFrame, StoryboardPanel, StoryboardSpec,
        StoryboardTransition,
    },
    transform::TransformBackend,
    video::VideoBackend,
    web_screen::WebScreenBackend,
    workflow::WorkflowBackend,
};
pub use cancellation::{make_cancel_signal, CancelSignal};
pub use context::{
    get_video_config, pop_video_config, push_video_config, ComposeContext, ComposeResult,
    ComposeTier, VideoConfigContext,
};
pub use credential_store::{Credential, CredentialStore};
pub use deep_think::{DeepThinkConfig, DeepThinkStep, DeepThinkStream};
pub mod codec_validation;
pub use animate::{interpolate, spring, ExtrapolateMode, SpringConfig};
pub use codec_validation::{
    validate_codec_pixel_format, PixelFormat as ValidationPixelFormat,
    VideoCodec as ValidationCodec,
};
pub use composition::{CompositionConfig, CompositionRegistry, VideoCodec};
pub use dispatch::ComposeContext as DispatchComposeContext;
pub use dispatch::{Backend, BackendRegistry, NoopBackend, UnifiedDispatcher};
pub use flow_graph::{FlowEdge, FlowGraph, FlowNode, FlowNodeKind};
pub use glue::{AiGlueOrchestrator, GlueBlueprint, ReActLlmFn, StubLlmFn};
pub use glue_cache::{CachedGlue, GlueCache, GlueStatus};
pub use hybrid::HybridResolver;
pub use middleware::{LatencyMiddleware, LoggingMiddleware, MiddlewareRegistry, StepMiddleware};
pub use orchestrator::ComposeOrchestrator;
pub use plan::{CompositionPlan, PlanStep};
pub use progress::{ComposeEvent, LogProgressSink, ProgressSink};
pub use provider_router::{FallbackLevel, ProviderRouter, VendorEntry};
pub use semantic::{SemanticColumn, SemanticDataType, SemanticModel, SemanticRegistry};
pub use store::{ArtifactStore, InMemoryStore};
pub use streaming::{StreamToken, SwitchableStream};
pub use task_queue::{ComposeTask, TaskQueue, TaskState};
pub use timeline::{current_frame_in_sequence, is_frame_active, SequenceContext};
pub use vendor_trait::{CostEstimate, MediaVendor, StubVendor, VendorCapability};
pub mod pipeline;
pub use pipeline::{ComponentOutput, ComponentPipeline, DocumentRetriever, TextSplitter};
pub mod video;
pub use video::{FrameCapture, PipelineStage, TwoStagePipeline};
pub mod video_capture;
pub use video_capture::{FfmpegConfig, FfmpegEncoder, FrameCapture as VideoCaptureFrame, VideoCapturePipeline};
pub mod reverse;
pub use reverse::{DetectedComponent, ReverseInput, ReverseOrchestrator, ReverseResult};
pub mod inspector;
pub use inspector::{InspectFinding, InspectReport, InspectTarget, NomInspector};
pub mod sherlock;
pub use sherlock::{SherlockAdapter, SherlockResult, SherlockSite, SherlockStatus};
pub mod segmentation;
pub mod sherlock_native;
pub mod vision;
pub mod layout;
pub mod vision_orchestrator;
pub mod donut_pipeline;
pub mod codegen_pipeline;
pub mod vision_bridge;
pub mod pipeline_context;
pub mod streaming_result;
pub use streaming_result::{PartialResult, ResultBuffer, StreamingOutput};
pub mod storyboard;
pub use storyboard::{StoryboardExecutor, StoryboardPhase, StoryboardPlan, StoryboardStep};
pub mod image_dispatch;
pub use image_dispatch::{DispatchRecord, ImageDispatcher, ModelCapability, ModelDescriptor, ModelRegistry};
pub mod image_pipeline;
pub use image_pipeline::{ImageStageKind, ImageStage, ImagePipeline, PipelineResult as ImagePipelineResult, ImagePipelineRunner};
pub mod audio_encode;
pub use audio_encode::{AudioBuffer, AudioEncoder, AudioFormat, RodioBackend};
pub mod n8n_workflow;
pub use n8n_workflow::{NodeStatus, WorkflowGraph, WorkflowNode, WorkflowRunner};
pub mod pdf_compose;
pub use pdf_compose::{PageSize, PdfComposer, PdfDocument, PdfExportOptions, PdfPage};
pub mod video_encode;
pub use video_encode::{GpuVideoEncoder, VideoCodec as VideoEncodeCodec, VideoEncoder, VideoFrame};
pub mod web_compose;
pub use web_compose::{ComponentKind, WebAppSpec, WebComponent, WebComposer};
pub mod ad_creative;
pub use ad_creative::{AdComposer, AdDimension};
pub use ad_creative::AdCreativeSpec as AdCreativeSpecComposer;
pub use ad_creative::AdFormat as AdFormatComposer;
pub mod mobile_compose;
pub use mobile_compose::{MobileAppSpec, MobileComponent, MobileComposer, MobilePlatform, MobileScreen};
pub mod mesh_compose;
pub use mesh_compose::{Mesh, MeshComposer, MeshFace, MeshVertex};
pub mod llama_compose;
pub use llama_compose::{
    LlamaPipeline, LlamaPipelineNode, PipelineCombinator, PipelineOutput,
    PipelineStage as LlamaPipelineStage,
};
pub mod app_bundle;
pub use app_bundle::{BundleArtifact, BundleBuilder, BundleManifest, BundleOutput, BundleTarget};
pub mod completion_engine;
pub use completion_engine::{CompletionEngine, CompletionItem, CompletionKind, CompletionList, CompletionQuery};
pub mod native_screen;
pub use native_screen::{
    ScreenTarget, CaptureResolution, CaptureBuffer, ScreenCapture,
    NativeScreenBackend as NativeScreenCaptureBackend,
};
pub mod video_timeline;
pub use video_timeline::{ClipKind, TimelineClip, VideoTimeline, ClipOverlap, TimelineRenderer};
pub mod export_bundle;
pub use export_bundle::{ExportFormat, ExportTarget, ExportJob, ExportQueue, ExportResult};
pub mod data_compose;
pub use data_compose::{DataSourceKind, DataSource, DataQuery, DataResult, DataComposer};
pub mod font_compose;
pub use font_compose::{FontStyle, FontWeight, FontSpec, FontFamily, FontComposer};
pub mod storyboard_compose;
pub use storyboard_compose::SceneType as StorySceneType;
pub use storyboard_compose::StoryboardPanel as StoryboardComposePanel;
pub use storyboard_compose::StoryboardAct;
pub use storyboard_compose::Storyboard as StoryboardCompose;
pub use storyboard_compose::StoryboardComposer;

#[cfg(test)]
mod integration_tests {
    use crate::backends::data_query::DataQuerySpec;
    use crate::dispatch::{BackendRegistry, NoopBackend};
    use crate::provider_router::{FallbackLevel, ProviderRouter};
    use crate::semantic::{SemanticColumn, SemanticDataType, SemanticModel, SemanticRegistry};
    use crate::vendor_trait::StubMediaVendor;

    // -------------------------------------------------------------------------
    // Test 1: backend_registry_with_real_backends
    // Register a NoopBackend for "video", dispatch "video" input, verify Ok result.
    // -------------------------------------------------------------------------
    #[test]
    fn backend_registry_with_real_backends() {
        let mut registry = BackendRegistry::new();
        registry.register(Box::new(NoopBackend::new("video")));

        let result = registry.dispatch("video", "test-scene", &|_| {});
        assert!(
            result.is_ok(),
            "dispatch to registered video backend must succeed"
        );

        let output = result.unwrap();
        assert_eq!(
            output, "video:test-scene",
            "NoopBackend must echo input prefixed with kind name"
        );

        // Dispatching an unregistered kind must return Err.
        let err = registry.dispatch("audio", "x", &|_| {});
        assert!(
            err.is_err(),
            "dispatch to unregistered audio backend must fail"
        );
    }

    // -------------------------------------------------------------------------
    // Test 2: semantic_model_feeds_data_query
    // -------------------------------------------------------------------------
    #[test]
    fn semantic_model_feeds_data_query() {
        let mut model = SemanticModel::new("events", "raw.events");
        model.add_column(SemanticColumn {
            name: "event_id".to_string(),
            data_type: SemanticDataType::Integer,
            description: None,
        });
        model.add_column(SemanticColumn {
            name: "event_name".to_string(),
            data_type: SemanticDataType::String,
            description: None,
        });

        let mut registry = SemanticRegistry::new();
        registry.register(model);

        let spec = DataQuerySpec {
            model_name: "events".to_string(),
            columns: vec!["event_id".to_string(), "event_name".to_string()],
            where_clause: None,
            limit: None,
        };

        let sql = spec
            .to_sql(&registry)
            .expect("model must be found in registry");
        assert!(
            sql.contains("event_id"),
            "SQL must contain column name 'event_id', got: {sql}"
        );
        assert!(
            sql.contains("event_name"),
            "SQL must contain column name 'event_name', got: {sql}"
        );
        assert!(
            sql.contains("raw.events"),
            "SQL must reference source table, got: {sql}"
        );
    }

    // -------------------------------------------------------------------------
    // Test 3: provider_router_with_stub_vendor
    // -------------------------------------------------------------------------
    #[test]
    fn provider_router_with_stub_vendor() {
        let mut router = ProviderRouter::new();
        router.register(
            StubMediaVendor {
                name: "stub-audio",
                kind: "audio".to_string(),
            },
            FallbackLevel::Primary,
        );

        let result = router.compose_with_fallback("audio", "some-audio-input", &|_| {}, false);

        assert!(result.is_ok(), "StubMediaVendor must return Ok");
        assert_eq!(
            result.unwrap(),
            "stub_output",
            "StubMediaVendor must return literal 'stub_output'"
        );
    }

    // -------------------------------------------------------------------------
    // Test 4: backend_registry_all_16_kinds_discoverable
    // -------------------------------------------------------------------------
    #[test]
    fn backend_registry_all_16_kinds_discoverable() {
        let all_kinds = [
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
        let mut registry = BackendRegistry::new();
        for kind in &all_kinds {
            registry.register(Box::new(NoopBackend::new(kind)));
        }
        assert_eq!(
            registry.registered_kinds().len(),
            16,
            "all 16 backend kinds must be discoverable after registration"
        );
        for kind in &all_kinds {
            let result = registry.dispatch(kind, "probe", &|_| {});
            assert!(result.is_ok(), "dispatch to {} must succeed", kind);
        }
    }

    // -------------------------------------------------------------------------
    // Test 5: large_compose_job_many_elements
    // -------------------------------------------------------------------------
    #[test]
    fn large_compose_job_many_elements() {
        use crate::plan::CompositionPlan;

        let mut registry = BackendRegistry::new();
        for kind in ["video", "audio", "image", "export", "transform"] {
            registry.register(Box::new(NoopBackend::new(kind)));
        }

        let kinds_cycle = ["video", "audio", "image", "export", "transform"];

        let mut plan = CompositionPlan::new();
        for i in 0..20usize {
            plan.add_step(
                kinds_cycle[i % kinds_cycle.len()],
                format!("input_{i}"),
                format!("output_{i}"),
            );
        }

        assert_eq!(plan.steps.len(), 20);
        assert!(plan.is_valid_dag(), "20-step plan must be a valid DAG");

        let order = plan.topo_order();
        assert_eq!(order.len(), 20);

        let mut success_count = 0;
        for step_id in &order {
            let step = &plan.steps[*step_id];
            let result = registry.dispatch(&step.backend, &step.input_key, &|_| {});
            if result.is_ok() {
                success_count += 1;
            }
        }
        assert_eq!(success_count, 20, "all 20 dispatch calls must succeed");
    }

    // -------------------------------------------------------------------------
    // Test 6: compose_result_serialization
    // -------------------------------------------------------------------------
    #[test]
    fn compose_result_serialization() {
        use crate::store::{ArtifactStore, InMemoryStore};
        let mut store = InMemoryStore::new();
        let h1 = store.put_bytes(b"artifact_one");
        let h2 = store.put_bytes(b"artifact_two");
        let hex1 = h1.as_hex();
        let hex2 = h2.as_hex();
        assert_eq!(hex1.len(), 64, "hex must be 64 chars");
        assert_eq!(hex2.len(), 64, "hex must be 64 chars");
        assert_ne!(
            hex1, hex2,
            "different payloads must produce different hashes"
        );
        assert!(
            hex1.chars().all(|c: char| c.is_ascii_hexdigit()),
            "hex1 must be valid hex: {hex1}"
        );
        assert!(
            hex2.chars().all(|c: char| c.is_ascii_hexdigit()),
            "hex2 must be valid hex: {hex2}"
        );
        let h3 = store.put_bytes(b"artifact_one");
        assert_eq!(h1.as_hex(), h3.as_hex(), "hash must be deterministic");
    }

    // -------------------------------------------------------------------------
    // Test 7: cancel_and_progress_together
    // -------------------------------------------------------------------------
    #[test]
    fn cancel_and_progress_together() {
        use crate::cancel::InterruptFlag;
        use crate::progress::{ComposeEvent, ProgressSink, VecProgressSink};

        let flag = InterruptFlag::new();
        let sink = VecProgressSink::new();

        sink.emit(ComposeEvent::Started {
            backend: "video".into(),
            entity_id: "e1".into(),
        });
        sink.emit(ComposeEvent::Progress {
            percent: 0.3,
            stage: "pre-cancel".into(),
            rendered_frames: None,
            encoded_frames: None,
            elapsed_ms: None,
        });

        flag.set();
        assert!(flag.is_set(), "flag must be set after set()");

        sink.emit(ComposeEvent::Failed {
            reason: "cancelled by user".into(),
        });

        let events = sink.take();
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], ComposeEvent::Started { .. }));
        assert!(matches!(events[1], ComposeEvent::Progress { .. }));
        assert!(matches!(events[2], ComposeEvent::Failed { .. }));

        flag.clear();
        assert!(!flag.is_set());
    }
}
