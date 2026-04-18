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

/// Video composition config accessible within nested backends (pattern: use-video-config)
#[derive(Debug, Clone)]
pub struct VideoConfigContext {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub duration_frames: u32,
}

impl VideoConfigContext {
    pub fn new(width: u32, height: u32, fps: u32, duration_frames: u32) -> Self {
        Self { width, height, fps, duration_frames }
    }

    pub fn default_hd() -> Self { Self::new(1920, 1080, 30, 90) }

    pub fn duration_secs(&self) -> f32 { self.duration_frames as f32 / self.fps as f32 }
}

// Thread-local video config stack for nested backends
thread_local! {
    static VIDEO_CONFIG_STACK: std::cell::RefCell<Vec<VideoConfigContext>> =
        const { std::cell::RefCell::new(vec![]) };
}

pub fn push_video_config(config: VideoConfigContext) {
    VIDEO_CONFIG_STACK.with(|s| s.borrow_mut().push(config));
}

pub fn pop_video_config() -> Option<VideoConfigContext> {
    VIDEO_CONFIG_STACK.with(|s| s.borrow_mut().pop())
}

pub fn get_video_config() -> Result<VideoConfigContext, String> {
    VIDEO_CONFIG_STACK.with(|s| {
        s.borrow().last().cloned()
            .ok_or_else(|| "get_video_config() called outside a composition context".into())
    })
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

    /// duration_secs returns duration_frames / fps as f32.
    #[test]
    fn test_video_config_context_duration_secs() {
        let cfg = VideoConfigContext::new(1920, 1080, 30, 90);
        let secs = cfg.duration_secs();
        assert!(
            (secs - 3.0_f32).abs() < f32::EPSILON,
            "90 frames / 30 fps must be 3.0 secs, got {secs}"
        );

        let cfg2 = VideoConfigContext::new(1280, 720, 24, 48);
        let secs2 = cfg2.duration_secs();
        assert!(
            (secs2 - 2.0_f32).abs() < f32::EPSILON,
            "48 frames / 24 fps must be 2.0 secs, got {secs2}"
        );
    }

    /// push then pop returns the same config; second pop returns None.
    #[test]
    fn test_video_config_stack_push_pop() {
        // Ensure the thread-local is clean before this test.
        while pop_video_config().is_some() {}

        let cfg = VideoConfigContext::new(1280, 720, 60, 120);
        push_video_config(cfg.clone());

        let popped = pop_video_config().expect("must pop the pushed config");
        assert_eq!(popped.width, 1280);
        assert_eq!(popped.height, 720);
        assert_eq!(popped.fps, 60);
        assert_eq!(popped.duration_frames, 120);

        assert!(
            pop_video_config().is_none(),
            "second pop on empty stack must return None"
        );
    }

    /// get_video_config() on an empty stack must return Err.
    #[test]
    fn test_get_video_config_outside_context_errors() {
        // Drain any leftover state.
        while pop_video_config().is_some() {}

        let result = get_video_config();
        assert!(
            result.is_err(),
            "get_video_config() on empty stack must return Err"
        );
        let msg = result.unwrap_err();
        assert!(
            msg.contains("outside a composition context"),
            "error message must mention 'outside a composition context', got: {msg}"
        );
    }
}
