#![deny(unsafe_code)]
pub mod backends;
pub mod cancel;
pub mod credential_store;
pub mod deep_think;
pub mod dispatch;
pub mod plan;
pub mod progress;
pub mod provider_router;
pub mod semantic;
pub mod store;
pub mod task_queue;
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
    data_query::DataQuerySpec,
    document::DocumentBackend,
    embed_gen::EmbedGenBackend,
    export::ExportBackend,
    image::ImageBackend,
    mesh::{MeshBackend, MeshPrimitive, MeshSpec},
    mobile_screen::{MobileScreenBackend, MobileScreenSpec},
    native_screen::{NativeScreenBackend, NativeScreenSpec},
    pipeline::PipelineBackend,
    presentation::{PresentationSlide, PresentationSpec},
    rag_query::RagQueryBackend,
    render::RenderBackend,
    scenario::ScenarioBackend,
    scenario_workflow::{compose as compose_scenario_workflow, ScenarioWorkflowSpec},
    storyboard::{StoryboardBackend, StoryboardFrame, StoryboardSpec},
    transform::TransformBackend,
    video::VideoBackend,
    web_screen::WebScreenBackend,
    workflow::WorkflowBackend,
};
pub use credential_store::{Credential, CredentialStore};
pub use deep_think::{DeepThinkConfig, DeepThinkStep, DeepThinkStream};
pub use dispatch::{Backend, BackendKind, BackendRegistry, NoopBackend};
pub use plan::{CompositionPlan, PlanStep};
pub use progress::{ComposeEvent, LogProgressSink, ProgressSink};
pub use provider_router::{FallbackLevel, ProviderRouter, VendorEntry};
pub use semantic::{SemanticColumn, SemanticDataType, SemanticModel, SemanticRegistry};
pub use store::{ArtifactStore, InMemoryStore};
pub use task_queue::{ComposeTask, TaskQueue, TaskState};
pub use vendor_trait::{CostEstimate, MediaVendor, StubVendor, VendorCapability};

#[cfg(test)]
mod integration_tests {
    use crate::backends::data_query::DataQuerySpec;
    use crate::dispatch::{BackendKind, BackendRegistry, NoopBackend};
    use crate::provider_router::{FallbackLevel, ProviderRouter};
    use crate::semantic::{SemanticColumn, SemanticDataType, SemanticModel, SemanticRegistry};
    use crate::vendor_trait::StubMediaVendor;

    // -------------------------------------------------------------------------
    // Test 1: backend_registry_with_real_backends
    // Register a NoopBackend for Video, dispatch Video input, verify Ok result.
    // -------------------------------------------------------------------------
    #[test]
    fn backend_registry_with_real_backends() {
        let mut registry = BackendRegistry::new();
        registry.register(Box::new(NoopBackend::new(BackendKind::Video)));

        let result = registry.dispatch(BackendKind::Video, "test-scene", &|_| {});
        assert!(
            result.is_ok(),
            "dispatch to registered Video backend must succeed"
        );

        let output = result.unwrap();
        assert_eq!(
            output, "video:test-scene",
            "NoopBackend must echo input prefixed with kind name"
        );

        // Dispatching an unregistered kind must return Err.
        let err = registry.dispatch(BackendKind::Audio, "x", &|_| {});
        assert!(
            err.is_err(),
            "dispatch to unregistered Audio backend must fail"
        );
    }

    // -------------------------------------------------------------------------
    // Test 2: semantic_model_feeds_data_query
    // Create SemanticModel with 2 columns, register in SemanticRegistry,
    // create DataQuerySpec, call to_sql, verify SQL contains column names.
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
    // Register StubMediaVendor for Audio, compose_with_fallback, verify stub_output.
    // -------------------------------------------------------------------------
    #[test]
    fn provider_router_with_stub_vendor() {
        let mut router = ProviderRouter::new();
        router.register(
            StubMediaVendor {
                name: "stub-audio",
                kind: BackendKind::Audio,
            },
            FallbackLevel::Primary,
        );

        let result =
            router.compose_with_fallback(&BackendKind::Audio, "some-audio-input", &|_| {}, false);

        assert!(result.is_ok(), "StubMediaVendor must return Ok");
        assert_eq!(
            result.unwrap(),
            "stub_output",
            "StubMediaVendor must return literal 'stub_output'"
        );
    }
}
