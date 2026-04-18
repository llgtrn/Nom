#![deny(unsafe_code)]

use std::sync::Arc;

use crate::context::{ComposeContext, ComposeResult};
use crate::hybrid::HybridResolver;

/// Orchestrates one or more composition requests through the HybridResolver.
pub struct ComposeOrchestrator {
    resolver: Arc<HybridResolver>,
}

impl ComposeOrchestrator {
    /// Create an orchestrator backed by the given resolver.
    pub fn new(resolver: Arc<HybridResolver>) -> Self {
        Self { resolver }
    }

    /// Run N compose requests and return results in the same order.
    ///
    /// Currently executes sequentially. The API surface is parallel-ready:
    /// callers may pass any number of requests and results are index-matched.
    pub fn run_parallel(
        &self,
        requests: Vec<ComposeContext>,
    ) -> Vec<Result<ComposeResult, String>> {
        requests.iter().map(|ctx| self.resolver.resolve(ctx)).collect()
    }

    /// Run a single compose request.
    pub fn run(&self, ctx: &ComposeContext) -> Result<ComposeResult, String> {
        self.resolver.resolve(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{ComposeContext, ComposeTier};
    use crate::dispatch::UnifiedDispatcher;
    use crate::glue::{AiGlueOrchestrator, StubLlmFn};

    fn make_orchestrator_empty() -> ComposeOrchestrator {
        let dispatcher = Arc::new(UnifiedDispatcher::new());
        let llm = StubLlmFn { response: "glue-code".to_string() };
        let glue = AiGlueOrchestrator::new(Box::new(llm));
        let resolver = Arc::new(crate::hybrid::HybridResolver::new(dispatcher, glue));
        ComposeOrchestrator::new(resolver)
    }

    fn make_orchestrator_with_video() -> ComposeOrchestrator {
        let mut dispatcher = UnifiedDispatcher::new();
        dispatcher.register("video", |ctx| Ok(format!("video:{}", ctx.entity_id)));
        let dispatcher = Arc::new(dispatcher);
        let llm = StubLlmFn { response: "fallback".to_string() };
        let glue = AiGlueOrchestrator::new(Box::new(llm));
        let resolver = Arc::new(crate::hybrid::HybridResolver::new(dispatcher, glue));
        ComposeOrchestrator::new(resolver)
    }

    #[test]
    fn test_orchestrator_single_compose() {
        let orch = make_orchestrator_with_video();
        let ctx = ComposeContext::new("video", "scene-42");
        let result = orch.run(&ctx).expect("single compose must succeed");
        assert!(
            matches!(result.tier_used, ComposeTier::DbDriven),
            "registered kind must use DbDriven tier"
        );
        assert_eq!(result.artifact, "video:scene-42");
    }

    #[test]
    fn test_orchestrator_parallel_two_requests() {
        let orch = make_orchestrator_with_video();
        let requests = vec![
            ComposeContext::new("video", "clip-1"),
            ComposeContext::new("video", "clip-2"),
        ];
        let results = orch.run_parallel(requests);
        assert_eq!(results.len(), 2, "must return one result per request");
        assert!(results[0].is_ok(), "first request must succeed");
        assert!(results[1].is_ok(), "second request must succeed");
        assert_eq!(results[0].as_ref().unwrap().artifact, "video:clip-1");
        assert_eq!(results[1].as_ref().unwrap().artifact, "video:clip-2");
    }

    #[test]
    fn test_orchestrator_empty_requests() {
        let orch = make_orchestrator_empty();
        let results = orch.run_parallel(vec![]);
        assert!(
            results.is_empty(),
            "empty request list must produce empty result list"
        );
    }
}
