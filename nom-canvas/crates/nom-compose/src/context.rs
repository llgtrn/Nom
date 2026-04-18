#![deny(unsafe_code)]

/// Tier that determines how a composition request is handled.
#[derive(Debug, Clone)]
pub enum ComposeTier {
    /// Grammar kinds with Complete status routed through BackendRegistry.
    DbDriven,
    /// Registered MediaVendor routed through UnifiedDispatcher.
    Provider,
    /// AiGlueOrchestrator generates .nomx glue for this request.
    AiLeading,
}

/// Context for a hybrid composition request.
#[derive(Debug, Clone)]
pub struct ComposeContext {
    pub kind: String,
    pub input: String,
    pub tier: ComposeTier,
    pub intent_query: String,
    pub session_id: Option<String>,
}

impl ComposeContext {
    /// Create a context with DbDriven tier and empty intent_query.
    pub fn new(kind: impl Into<String>, input: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            input: input.into(),
            tier: ComposeTier::DbDriven,
            intent_query: String::new(),
            session_id: None,
        }
    }

    /// Override the tier for this context.
    pub fn with_tier(mut self, tier: ComposeTier) -> Self {
        self.tier = tier;
        self
    }

    /// Set the natural-language intent query attached to this request.
    pub fn with_intent(mut self, query: impl Into<String>) -> Self {
        self.intent_query = query.into();
        self
    }

    /// Set the session id for this context.
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }
}

/// Result returned by the hybrid composition system.
#[derive(Debug, Clone)]
pub struct ComposeResult {
    pub artifact: String,
    pub tier_used: ComposeTier,
    pub confidence: f32,
    /// Present when tier_used == AiLeading; holds the .nomx glue hash.
    pub glue_hash: Option<String>,
}

impl ComposeResult {
    /// Construct a result with no glue_hash (set it manually for AiLeading tier).
    pub fn new(artifact: impl Into<String>, tier: ComposeTier, confidence: f32) -> Self {
        Self {
            artifact: artifact.into(),
            tier_used: tier,
            confidence,
            glue_hash: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// new() must default to DbDriven tier with empty intent_query and no session_id.
    #[test]
    fn test_compose_context_defaults() {
        let ctx = ComposeContext::new("render", "my-input");
        assert_eq!(ctx.kind, "render");
        assert_eq!(ctx.input, "my-input");
        assert!(
            matches!(ctx.tier, ComposeTier::DbDriven),
            "default tier must be DbDriven"
        );
        assert!(
            ctx.intent_query.is_empty(),
            "default intent_query must be empty"
        );
        assert!(ctx.session_id.is_none(), "default session_id must be None");
    }

    /// AiLeading tier result can carry a glue_hash.
    #[test]
    fn test_compose_result_ai_leading() {
        let mut result = ComposeResult::new("artifact-data", ComposeTier::AiLeading, 0.87);
        result.glue_hash = Some("deadbeef01234567".to_string());

        assert_eq!(result.artifact, "artifact-data");
        assert!(
            matches!(result.tier_used, ComposeTier::AiLeading),
            "tier_used must be AiLeading"
        );
        assert!(
            (result.confidence - 0.87).abs() < f32::EPSILON,
            "confidence must match, got {}",
            result.confidence
        );
        assert_eq!(
            result.glue_hash.as_deref(),
            Some("deadbeef01234567"),
            "glue_hash must be set"
        );
    }

    /// Builder chain with_tier + with_intent sets fields correctly.
    #[test]
    fn test_compose_context_builder() {
        let ctx = ComposeContext::new("video", "scene-data")
            .with_tier(ComposeTier::Provider)
            .with_intent("render a cinematic sequence");

        assert!(
            matches!(ctx.tier, ComposeTier::Provider),
            "tier must be Provider after with_tier"
        );
        assert_eq!(
            ctx.intent_query, "render a cinematic sequence",
            "intent_query must match"
        );
        assert_eq!(ctx.kind, "video");
        assert_eq!(ctx.input, "scene-data");
    }
}
