#![deny(unsafe_code)]

use crate::context::{ComposeContext, ComposeResult};

/// Hook called before and after each compose step.
pub trait StepMiddleware: Send + Sync {
    fn name(&self) -> &str;
    fn before_step(&self, _ctx: &ComposeContext) -> Result<(), String> {
        Ok(())
    }
    fn after_step(&self, _ctx: &ComposeContext, _result: &Result<ComposeResult, String>) {}
}

/// Registry that wraps every dispatch call with middleware hooks.
pub struct MiddlewareRegistry {
    middlewares: Vec<Box<dyn StepMiddleware>>,
}

impl MiddlewareRegistry {
    pub fn new() -> Self {
        Self {
            middlewares: vec![],
        }
    }

    pub fn register(&mut self, m: Box<dyn StepMiddleware>) {
        self.middlewares.push(m);
    }

    /// Run all before_step hooks; return first error if any.
    pub fn run_before(&self, ctx: &ComposeContext) -> Result<(), String> {
        for m in &self.middlewares {
            m.before_step(ctx)?;
        }
        Ok(())
    }

    /// Run all after_step hooks.
    pub fn run_after(&self, ctx: &ComposeContext, result: &Result<ComposeResult, String>) {
        for m in &self.middlewares {
            m.after_step(ctx, result);
        }
    }

    pub fn len(&self) -> usize {
        self.middlewares.len()
    }

    pub fn is_empty(&self) -> bool {
        self.middlewares.is_empty()
    }
}

impl Default for MiddlewareRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Built-in: logging middleware (stub — writes nothing to keep tests clean).
pub struct LoggingMiddleware;

impl StepMiddleware for LoggingMiddleware {
    fn name(&self) -> &str {
        "logging"
    }

    fn before_step(&self, _ctx: &ComposeContext) -> Result<(), String> {
        Ok(())
    }
}

/// Built-in: latency middleware that records timing by kind.
pub struct LatencyMiddleware {
    pub recorded_calls: std::sync::Mutex<Vec<String>>,
}

impl LatencyMiddleware {
    pub fn new() -> Self {
        Self {
            recorded_calls: std::sync::Mutex::new(vec![]),
        }
    }
}

impl Default for LatencyMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl StepMiddleware for LatencyMiddleware {
    fn name(&self) -> &str {
        "latency"
    }

    fn after_step(&self, ctx: &ComposeContext, _result: &Result<ComposeResult, String>) {
        let mut calls = self.recorded_calls.lock().unwrap();
        calls.push(ctx.kind.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{ComposeContext, ComposeResult, ComposeTier};

    #[test]
    fn test_middleware_registry_empty() {
        let reg = MiddlewareRegistry::new();
        assert!(reg.is_empty(), "new registry must be empty");
        assert_eq!(reg.len(), 0, "new registry length must be 0");
    }

    #[test]
    fn test_middleware_registry_register_and_len() {
        let mut reg = MiddlewareRegistry::new();
        reg.register(Box::new(LoggingMiddleware));
        assert_eq!(
            reg.len(),
            1,
            "registry must have one middleware after register"
        );
        assert!(!reg.is_empty());
        reg.register(Box::new(LatencyMiddleware::new()));
        assert_eq!(reg.len(), 2, "registry must have two middlewares");
    }

    #[test]
    fn test_middleware_before_returns_ok() {
        let mut reg = MiddlewareRegistry::new();
        reg.register(Box::new(LoggingMiddleware));
        let ctx = ComposeContext::new("render", "input-data");
        let result = reg.run_before(&ctx);
        assert!(
            result.is_ok(),
            "run_before must return Ok when no middleware errors"
        );
    }

    #[test]
    fn test_latency_middleware_records_kind() {
        let latency = std::sync::Arc::new(LatencyMiddleware::new());
        let ctx = ComposeContext::new("video", "some-input");
        let ok_result: Result<ComposeResult, String> =
            Ok(ComposeResult::new("artifact", ComposeTier::DbDriven, 1.0));
        latency.after_step(&ctx, &ok_result);
        let calls = latency.recorded_calls.lock().unwrap();
        assert_eq!(calls.len(), 1, "latency must record one call");
        assert_eq!(calls[0], "video", "recorded kind must match context kind");
    }

    #[test]
    fn test_logging_middleware_name() {
        let m = LoggingMiddleware;
        assert_eq!(
            m.name(),
            "logging",
            "LoggingMiddleware name must be 'logging'"
        );
    }
}
