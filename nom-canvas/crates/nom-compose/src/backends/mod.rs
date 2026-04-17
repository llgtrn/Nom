//! Concrete composition backends for each `NomKind`.
//!
//! Every submodule exposes:
//!   - A `*Spec` struct describing the inputs
//!   - A `validate()` fn with typed error
//!   - A `Stub*Backend` that implements `CompositionBackend` and returns
//!     an empty/placeholder `ComposeOutput` (runtime crates provide real
//!     implementations).
#![deny(unsafe_code)]

pub mod audio;
pub mod data_extract;
pub mod data_frame;
pub mod data_query;
pub mod image;
pub mod mesh;
pub mod native_screen;
pub mod scenario_workflow;
pub mod storyboard_narrative;
pub mod video;
pub mod web_screen;

// ── Re-exports ─────────────────────────────────────────────────────────────

pub use audio::{AudioFormat, AudioSource, AudioSpec, StubAudioBackend, WordTiming};
pub use data_extract::{ExtractSpec, LayoutBlock, OutputFormat, StubDataExtractBackend, xy_cut};
pub use data_frame::{CellValue, DataFrame, DType, Series, StubDataFrameBackend};
pub use data_query::{
    PipelineStage, QueryIntent, QueryLanguage, QueryResult, QuerySpec, StubDataQueryBackend,
};
pub use image::{ImageFormat, ImageSpec, InferenceLocation, StubImageBackend};
pub use mesh::{AnimationClip, ExportFormat, MaterialRef, MeshGeometry, MeshSceneSpec, StubMeshBackend};
pub use native_screen::{NativeSpec, OptLevel, StubNativeScreenBackend, TargetTriple};
pub use scenario_workflow::{
    NodeActivation, NodeKey, NodeState, OnError, RetryPolicy, StubScenarioWorkflowBackend,
    WorkflowError, WorkflowNode, WorkflowSpec, next_ready, validate,
};
pub use storyboard_narrative::{
    NarrativePhase, NarrativeResult, StoryboardPhase, StoryboardResult, StubNarrativeBackend,
    StubStoryboardBackend,
};
pub use video::{FrameFormat, StubVideoBackend, VideoCodec, VideoSpec};
pub use web_screen::{DataBinding, LayoutKind, LayoutSpec, ScreenSpec, StubWebScreenBackend, WidgetSpec};

// ── Helper: register all stub backends into a dispatcher ───────────────────

use crate::backend_trait::CompositionBackend;
use crate::dispatch::ComposeDispatcher;
use std::sync::Arc;

/// Register one stub backend for every `NomKind` variant into the dispatcher.
/// Intended for bring-up + tests; production code replaces individual entries
/// by calling `dispatcher.register(Arc::new(RealBackend { .. }))`.
pub fn register_all_stubs(dispatcher: &mut ComposeDispatcher) {
    let stubs: Vec<Arc<dyn CompositionBackend>> = vec![
        Arc::new(StubVideoBackend),
        Arc::new(StubImageBackend),
        Arc::new(StubWebScreenBackend),
        Arc::new(StubNativeScreenBackend),
        Arc::new(StubDataExtractBackend),
        Arc::new(StubDataQueryBackend),
        Arc::new(StubStoryboardBackend),
        Arc::new(StubNarrativeBackend),
        Arc::new(StubAudioBackend),
        Arc::new(StubDataFrameBackend),
        Arc::new(StubMeshBackend),
        Arc::new(StubScenarioWorkflowBackend),
    ];
    for backend in stubs {
        dispatcher.register(backend);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend_trait::{ComposeSpec, InterruptFlag, ProgressSink};
    use crate::kind::NomKind;

    struct NoopProgress;
    impl ProgressSink for NoopProgress {
        fn notify(&self, _p: u32, _m: &str) {}
    }

    #[test]
    fn register_all_stubs_covers_11_kinds() {
        let mut dispatcher = ComposeDispatcher::new();
        register_all_stubs(&mut dispatcher);
        let interrupt = InterruptFlag::new();
        let progress = NoopProgress;
        for kind in [
            NomKind::MediaVideo,
            NomKind::MediaImage,
            NomKind::ScreenWeb,
            NomKind::ScreenNative,
            NomKind::DataExtract,
            NomKind::DataQuery,
            NomKind::MediaStoryboard,
            NomKind::MediaNovelVideo,
            NomKind::MediaAudio,
            NomKind::DataTransform,
            NomKind::Media3D,
        ] {
            let spec = ComposeSpec { kind, params: vec![] };
            let result = dispatcher.dispatch(&spec, &progress, &interrupt);
            assert!(result.is_ok(), "dispatch failed for kind {:?}", kind);
        }
    }
}
