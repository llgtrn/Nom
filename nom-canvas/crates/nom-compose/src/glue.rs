#![deny(unsafe_code)]

use std::sync::Arc;
use crate::chain::{Chain, DispatchRunnable, IntentRunnable, LlmRunnable, RagRunnable, Runnable, ValidateRunnable};
use crate::context::ComposeContext;
use crate::dispatch::BackendRegistry;

/// Trait for LLM adapters — 4 concrete implementations are provided.
pub trait ReActLlmFn: Send + Sync {
    fn complete(&self, prompt: &str) -> Result<String, String>;
    fn name(&self) -> &str;
}

/// Adapter 1: Stub — returns a hardcoded response for testing.
pub struct StubLlmFn {
    pub response: String,
}

impl ReActLlmFn for StubLlmFn {
    fn complete(&self, prompt: &str) -> Result<String, String> {
        if !self.response.is_empty() {
            return Ok(self.response.clone());
        }
        // Generate a simple .nomx template when no hardcoded response is set.
        let task = prompt.split_whitespace().last().unwrap_or("task");
        Ok(format!(
            "define compose_{} that generates output\n  input -> process -> output\nend",
            task
        ))
    }

    fn name(&self) -> &str {
        "stub"
    }
}

/// Adapter 2: NomCli — spawns the nom-compiler CLI process.
pub struct NomCliLlmFn {
    pub nom_binary: String,
}

impl ReActLlmFn for NomCliLlmFn {
    fn complete(&self, _prompt: &str) -> Result<String, String> {
        // Stub implementation: real impl would spawn self.nom_binary
        Ok("nom_cli_response".to_string())
    }

    fn name(&self) -> &str {
        "nom_cli"
    }
}

/// Adapter 3: Mcp — delegates via MCP tool call.
pub struct McpLlmFn {
    pub tool_name: String,
}

impl ReActLlmFn for McpLlmFn {
    fn complete(&self, _prompt: &str) -> Result<String, String> {
        // Stub implementation: real impl would invoke self.tool_name via MCP
        Ok("mcp_response".to_string())
    }

    fn name(&self) -> &str {
        "mcp"
    }
}

/// Adapter 4: RealLlm — external API call (stub, no API keys required).
pub struct RealLlmFn {
    pub endpoint: String,
}

impl ReActLlmFn for RealLlmFn {
    fn complete(&self, _prompt: &str) -> Result<String, String> {
        // Stub implementation: real impl would POST to self.endpoint
        Ok("real_llm_response".to_string())
    }

    fn name(&self) -> &str {
        "real_llm"
    }
}

/// The glue blueprint produced by the orchestrator.
#[derive(Debug, Clone)]
pub struct GlueBlueprint {
    pub kind: String,
    /// Generated .nomx glue code for this request.
    pub nomx_source: String,
    pub confidence: f32,
    pub llm_name: String,
}

/// Orchestrator that generates .nomx glue for unknown kinds via ReAct loop.
pub struct AiGlueOrchestrator {
    llm: Arc<dyn ReActLlmFn>,
    registry: Option<Arc<BackendRegistry>>,
    resolver: Option<nom_canvas_intent::IntentResolver>,
    dag: Option<Arc<nom_canvas_graph::Dag>>
}

impl AiGlueOrchestrator {
    pub fn new(llm: Box<dyn ReActLlmFn>) -> Self {
        Self {
            llm: Arc::from(llm),
            registry: None,
            resolver: None,
            dag: None,
        }
    }

    /// Attach a backend registry for dispatch.
    pub fn with_registry(mut self, registry: BackendRegistry) -> Self {
        self.registry = Some(Arc::new(registry));
        self
    }

    /// Attach an intent resolver.
    pub fn with_resolver(mut self, resolver: nom_canvas_intent::IntentResolver) -> Self {
        self.resolver = Some(resolver);
        self
    }

    /// Attach a DAG for RAG retrieval.
    pub fn with_dag(mut self, dag: nom_canvas_graph::Dag) -> Self {
        self.dag = Some(Arc::new(dag));
        self
    }

    /// Generate a .nomx glue blueprint for the given compose context.
    pub fn generate_blueprint(&self, ctx: &ComposeContext) -> Result<GlueBlueprint, String> {
        let prompt = format!("compose {} for: {}", ctx.kind, ctx.input);
        let response = self.llm.complete(&prompt)?;
        Ok(GlueBlueprint {
            kind: ctx.kind.clone(),
            nomx_source: response,
            confidence: 0.7,
            llm_name: self.llm.name().to_string(),
        })
    }

    /// Execute a blueprint by building and running a chain of Intent → RAG → LLM → Validate → Dispatch.
    pub fn execute_blueprint(&self, blueprint: &GlueBlueprint) -> Result<String, String> {
        let mut chain = Chain::new();

        if let Some(resolver) = &self.resolver {
            chain = chain.add_step(Box::new(IntentRunnable::new(resolver.grammar_kinds.clone())));
        }

        if let Some(dag) = &self.dag {
            chain = chain.add_step(Box::new(RagRunnable::new(Arc::clone(dag), 3, 2)));
        }

        chain = chain.add_step(Box::new(LlmRunnable::new(Arc::clone(&self.llm))));
        chain = chain.add_step(Box::new(ValidateRunnable::new()));

        if let Some(registry) = &self.registry {
            chain = chain.add_step(Box::new(
                DispatchRunnable::new(Arc::clone(registry)).with_kind(blueprint.kind.clone()),
            ));
        }

        let result = chain.run(blueprint.nomx_source.clone());
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ComposeContext;

    #[test]
    fn test_stub_llm_generates_blueprint() {
        let llm = StubLlmFn {
            response: "define compose_video that renders frames".to_string(),
        };
        let orchestrator = AiGlueOrchestrator::new(Box::new(llm));
        let ctx = ComposeContext::new("video", "my-scene");
        let blueprint = orchestrator.generate_blueprint(&ctx).unwrap();
        assert_eq!(blueprint.kind, "video");
        assert_eq!(
            blueprint.nomx_source,
            "define compose_video that renders frames"
        );
        assert_eq!(blueprint.llm_name, "stub");
    }

    #[test]
    fn test_glue_blueprint_has_nomx_source() {
        let llm = StubLlmFn {
            response: "nomx-code-here".to_string(),
        };
        let orchestrator = AiGlueOrchestrator::new(Box::new(llm));
        let ctx = ComposeContext::new("audio", "track-1");
        let blueprint = orchestrator.generate_blueprint(&ctx).unwrap();
        assert!(
            !blueprint.nomx_source.is_empty(),
            "nomx_source must not be empty"
        );
        assert_eq!(blueprint.nomx_source, "nomx-code-here");
        assert!(
            blueprint.confidence > 0.0 && blueprint.confidence <= 1.0,
            "confidence must be in (0, 1]"
        );
    }

    #[test]
    fn test_all_four_adapters_implement_trait() {
        let adapters: Vec<Box<dyn ReActLlmFn>> = vec![
            Box::new(StubLlmFn {
                response: "stub".to_string(),
            }),
            Box::new(NomCliLlmFn {
                nom_binary: "nom".to_string(),
            }),
            Box::new(McpLlmFn {
                tool_name: "nom_tool".to_string(),
            }),
            Box::new(RealLlmFn {
                endpoint: "http://localhost:8080".to_string(),
            }),
        ];
        let expected_names = ["stub", "nom_cli", "mcp", "real_llm"];
        let expected_responses = [
            "stub",
            "nom_cli_response",
            "mcp_response",
            "real_llm_response",
        ];
        for (i, adapter) in adapters.iter().enumerate() {
            assert_eq!(adapter.name(), expected_names[i]);
            let result = adapter.complete("test prompt").unwrap();
            assert_eq!(result, expected_responses[i]);
        }
    }

    #[test]
    fn test_execute_blueprint_returns_chain_result() {
        let llm = StubLlmFn {
            response: "code".to_string(),
        };
        let orchestrator = AiGlueOrchestrator::new(Box::new(llm));
        let blueprint = GlueBlueprint {
            kind: "image".to_string(),
            nomx_source: "code".to_string(),
            confidence: 0.8,
            llm_name: "stub".to_string(),
        };
        let artifact = orchestrator.execute_blueprint(&blueprint).unwrap();
        // Chain runs LLM → Validate with no registry/resolver/dag.
        assert_eq!(artifact, "code");
    }

    #[test]
    fn test_execute_blueprint_with_registry() {
        let llm = StubLlmFn {
            response: "processed".to_string(),
        };
        let mut registry = BackendRegistry::new();
        registry.register(Box::new(crate::dispatch::NoopBackend::new("image")));
        let orchestrator = AiGlueOrchestrator::new(Box::new(llm)).with_registry(registry);
        let blueprint = GlueBlueprint {
            kind: "image".to_string(),
            nomx_source: "scene".to_string(),
            confidence: 0.8,
            llm_name: "stub".to_string(),
        };
        let artifact = orchestrator.execute_blueprint(&blueprint).unwrap();
        // Chain: LLM → Validate → Dispatch
        // LLM returns "processed", Validate passes it, Dispatch routes to image backend.
        assert_eq!(artifact, "image:processed");
    }

    #[test]
    fn test_execute_blueprint_with_resolver() {
        let llm = StubLlmFn {
            response: "result".to_string(),
        };
        let resolver =
            nom_canvas_intent::IntentResolver::new(vec!["video".to_string(), "image".to_string()]);
        let orchestrator = AiGlueOrchestrator::new(Box::new(llm)).with_resolver(resolver);
        let blueprint = GlueBlueprint {
            kind: "video".to_string(),
            nomx_source: "render a video".to_string(),
            confidence: 0.8,
            llm_name: "stub".to_string(),
        };
        let artifact = orchestrator.execute_blueprint(&blueprint).unwrap();
        // Chain: Intent → LLM → Validate
        // Intent resolves "render a video" → "video", LLM completes "video" → "result".
        assert_eq!(artifact, "result");
    }
}
