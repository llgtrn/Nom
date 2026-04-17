#![deny(unsafe_code)]
pub mod store;
pub mod progress;
pub mod deep_think;
pub mod backends;
pub mod dispatch;
pub mod plan;
pub mod task_queue;

pub use store::{ArtifactStore, InMemoryStore};
pub use progress::{ProgressSink, LogProgressSink, ComposeEvent};
pub use deep_think::{DeepThinkStream, DeepThinkConfig, DeepThinkStep};
pub use dispatch::BackendKind;
pub use plan::{CompositionPlan, PlanStep};
pub use task_queue::{TaskQueue, ComposeTask, TaskState};
pub use backends::{
    document::DocumentBackend,
    video::VideoBackend,
    image::ImageBackend,
    audio::AudioBackend,
    data::DataBackend,
    app::AppBackend,
    code_exec::CodeExecBackend,
    web_screen::WebScreenBackend,
    workflow::WorkflowBackend,
    scenario::ScenarioBackend,
    rag_query::RagQueryBackend,
    transform::TransformBackend,
    embed_gen::EmbedGenBackend,
    render::RenderBackend,
    export::ExportBackend,
    pipeline::PipelineBackend,
};
