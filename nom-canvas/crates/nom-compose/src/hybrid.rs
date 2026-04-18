#![deny(unsafe_code)]

use std::sync::Arc;

use crate::context::{ComposeContext, ComposeResult, ComposeTier};
use crate::dispatch::UnifiedDispatcher;
use crate::glue::AiGlueOrchestrator;

/// Three-tier resolver: DB-driven → Provider → AiLeading.
pub struct HybridResolver {
    dispatcher: Arc<UnifiedDispatcher>,
    glue_orchestrator: AiGlueOrchestrator,
}

impl HybridResolver {
    pub fn new(dispatcher: Arc<UnifiedDispatcher>, glue: AiGlueOrchestrator) -> Self {
        Self {
            dispatcher,
            glue_orchestrator: glue,
        }
    }

    /// Resolve a compose request across all three tiers.
    ///
    /// Tier 1 (DbDriven): attempt dispatch via UnifiedDispatcher using the
    ///   kind string directly.
    /// Tier 2 (Provider): attempt dispatch via UnifiedDispatcher using a
    ///   provider-prefixed kind string.
    /// Tier 3 (AiLeading): fall through to AiGlueOrchestrator.
    pub fn resolve(&self, ctx: &ComposeContext) -> Result<ComposeResult, String> {
        // Tier 1: try direct DB-driven dispatch
        use crate::dispatch::ComposeContext as DispatchCtx;
        let dispatch_ctx = DispatchCtx::new(&ctx.kind, &ctx.input);
        if let Ok(artifact) = self.dispatcher.dispatch(&dispatch_ctx) {
            return Ok(ComposeResult::new(artifact, ComposeTier::DbDriven, 1.0));
        }

        // Tier 2: try provider-routed dispatch (provider_ prefix convention)
        let provider_kind = format!("provider_{}", ctx.kind);
        let provider_ctx = DispatchCtx::new(&provider_kind, &ctx.input);
        if let Ok(artifact) = self.dispatcher.dispatch(&provider_ctx) {
            return Ok(ComposeResult::new(artifact, ComposeTier::Provider, 0.9));
        }

        // Tier 3: AiGlueOrchestrator
        let blueprint = self.glue_orchestrator.generate_blueprint(ctx)?;
        let artifact = self.glue_orchestrator.execute_blueprint(&blueprint)?;
        Ok(ComposeResult::new(
            artifact,
            ComposeTier::AiLeading,
            blueprint.confidence,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ComposeContext;
    use crate::dispatch::UnifiedDispatcher;
    use crate::glue::{AiGlueOrchestrator, StubLlmFn};

    fn make_resolver_empty() -> HybridResolver {
        let dispatcher = Arc::new(UnifiedDispatcher::new());
        let llm = StubLlmFn {
            response: "ai-glue-code".to_string(),
        };
        let glue = AiGlueOrchestrator::new(Box::new(llm));
        HybridResolver::new(dispatcher, glue)
    }

    #[test]
    fn test_hybrid_resolver_falls_through_to_ai_tier() {
        let resolver = make_resolver_empty();
        let ctx = ComposeContext::new("unknown_kind", "some-input");
        let result = resolver.resolve(&ctx).unwrap();
        // With empty dispatcher, must fall through to AiLeading tier
        assert!(
            matches!(result.tier_used, ComposeTier::AiLeading),
            "unregistered kind must use AiLeading tier"
        );
        assert_eq!(result.artifact, "artifact:unknown_kind");
    }

    #[test]
    fn test_hybrid_result_has_correct_tier() {
        let mut dispatcher = UnifiedDispatcher::new();
        dispatcher.register("video", |ctx| {
            Ok(format!("video-dispatch:{}", ctx.entity_id))
        });
        let dispatcher = Arc::new(dispatcher);
        let llm = StubLlmFn {
            response: "fallback".to_string(),
        };
        let glue = AiGlueOrchestrator::new(Box::new(llm));
        let resolver = HybridResolver::new(dispatcher, glue);

        // Known kind — should hit Tier 1 (DbDriven)
        let ctx = ComposeContext::new("video", "scene-1");
        let result = resolver.resolve(&ctx).unwrap();
        assert!(
            matches!(result.tier_used, ComposeTier::DbDriven),
            "registered kind must use DbDriven tier"
        );
        assert_eq!(result.artifact, "video-dispatch:scene-1");
        assert!((result.confidence - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_hybrid_resolver_provider_tier() {
        let mut dispatcher = UnifiedDispatcher::new();
        dispatcher.register(
            "provider_audio",
            |_| Ok("provider-audio-result".to_string()),
        );
        let dispatcher = Arc::new(dispatcher);
        let llm = StubLlmFn {
            response: "fallback".to_string(),
        };
        let glue = AiGlueOrchestrator::new(Box::new(llm));
        let resolver = HybridResolver::new(dispatcher, glue);

        // audio is not registered directly, but provider_audio is — should hit Tier 2
        let ctx = ComposeContext::new("audio", "clip-1");
        let result = resolver.resolve(&ctx).unwrap();
        assert!(
            matches!(result.tier_used, ComposeTier::Provider),
            "provider-registered kind must use Provider tier"
        );
        assert_eq!(result.artifact, "provider-audio-result");
    }
}
