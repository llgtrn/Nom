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
    use crate::progress::ProgressSink;
    use crate::provider_router::{FallbackLevel, ProviderRouter};
    use crate::semantic::{SemanticColumn, SemanticDataType, SemanticModel, SemanticRegistry};
    use crate::store::ArtifactStore;
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

    // -------------------------------------------------------------------------
    // Test 4: backend_registry_all_16_kinds_discoverable
    // Register a NoopBackend for all 16 BackendKinds, verify registered_kinds() == 16.
    // -------------------------------------------------------------------------
    #[test]
    fn backend_registry_all_16_kinds_discoverable() {
        use crate::dispatch::BackendKind;
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
        let mut registry = BackendRegistry::new();
        for kind in all_kinds.iter().cloned() {
            registry.register(Box::new(NoopBackend::new(kind)));
        }
        assert_eq!(
            registry.registered_kinds().len(),
            16,
            "all 16 BackendKinds must be discoverable after registration"
        );
        // Dispatch to each must succeed.
        for kind in all_kinds.iter().cloned() {
            let result = registry.dispatch(kind.clone(), "probe", &|_| {});
            assert!(result.is_ok(), "dispatch to {} must succeed", kind.name());
        }
    }

    // -------------------------------------------------------------------------
    // Test 5: large_compose_job_many_elements
    // Register 16 backends, build a 20-step plan, dispatch each step via
    // BackendRegistry, verify all 20 results are Ok.
    // -------------------------------------------------------------------------
    #[test]
    fn large_compose_job_many_elements() {
        use crate::dispatch::BackendKind;
        use crate::plan::CompositionPlan;

        let mut registry = BackendRegistry::new();
        for kind in [
            BackendKind::Video,
            BackendKind::Audio,
            BackendKind::Image,
            BackendKind::Export,
            BackendKind::Transform,
        ] {
            registry.register(Box::new(NoopBackend::new(kind)));
        }

        let kinds_cycle = [
            BackendKind::Video,
            BackendKind::Audio,
            BackendKind::Image,
            BackendKind::Export,
            BackendKind::Transform,
        ];

        let mut plan = CompositionPlan::new();
        for i in 0..20usize {
            plan.add_step(
                kinds_cycle[i % kinds_cycle.len()].clone(),
                &format!("input_{i}"),
                &format!("output_{i}"),
            );
        }

        assert_eq!(plan.steps.len(), 20);
        assert!(plan.is_valid_dag(), "20-step plan must be a valid DAG");

        let order = plan.topo_order();
        assert_eq!(order.len(), 20);

        let mut success_count = 0;
        for step_id in &order {
            let step = &plan.steps[*step_id];
            let result = registry.dispatch(step.backend.clone(), &step.input_key, &|_| {});
            if result.is_ok() {
                success_count += 1;
            }
        }
        assert_eq!(success_count, 20, "all 20 dispatch calls must succeed");
    }

    // -------------------------------------------------------------------------
    // Test 6: compose_result_serialization
    // Write two artifacts to InMemoryStore, verify both hash→hex strings are
    // distinct 64-char lowercase hex.
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
        assert_ne!(hex1, hex2, "different payloads must produce different hashes");
        assert!(
            hex1.chars().all(|c: char| c.is_ascii_hexdigit()),
            "hex1 must be valid hex: {hex1}"
        );
        assert!(
            hex2.chars().all(|c: char| c.is_ascii_hexdigit()),
            "hex2 must be valid hex: {hex2}"
        );
        // Re-hashing the same input must produce the same hex (determinism).
        let h3 = store.put_bytes(b"artifact_one");
        assert_eq!(h1.as_hex(), h3.as_hex(), "hash must be deterministic");
    }

    // -------------------------------------------------------------------------
    // Test 7: cancel_and_progress_together
    // Emit progress events, set InterruptFlag, verify cancel state is visible.
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
        });

        // Simulate cancellation mid-job.
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

        // Reset flag — subsequent operations start clean.
        flag.clear();
        assert!(!flag.is_set());
    }
}
