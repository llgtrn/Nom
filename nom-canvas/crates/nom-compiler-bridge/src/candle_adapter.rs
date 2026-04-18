//! In-process LLM inference adapter for nom-compiler-bridge.
//! Implements InferenceFn using a stub that simulates responses from
//! compact generative models. Real inference requires `candle-core` —
//! this stub compiles without GPU deps and is replaceable when weights are added.

/// Compute device for inference.
#[derive(Debug, Clone, PartialEq)]
pub enum BackendDevice {
    Cpu,
    Cuda(usize), // device index
}

impl BackendDevice {
    pub fn is_cpu(&self) -> bool {
        matches!(self, Self::Cpu)
    }
}

/// Model configuration.
#[derive(Debug, Clone)]
pub struct ModelConfig {
    pub model_id: String,
    pub device: BackendDevice,
    pub max_tokens: usize,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            model_id: "phi-3-mini".to_string(),
            device: BackendDevice::Cpu,
            max_tokens: 256,
        }
    }
}

/// Adapter trait for in-process inference. Mirrors `ReActLlmFn` from nom-compose
/// but declared locally so this crate has no dependency on nom-compose.
pub trait InferenceFn: Send + Sync {
    fn infer(&self, prompt: &str) -> Result<String, String>;
    fn model_name(&self) -> &str;
}

/// In-process LLM adapter.
/// Stub impl — replace `generate` with real candle inference when
/// `candle-core` is added to Cargo.toml.
pub struct CandleAdapter {
    pub config: ModelConfig,
}

impl CandleAdapter {
    pub fn new(config: ModelConfig) -> Self {
        Self { config }
    }

    pub fn new_cpu(model_id: impl Into<String>) -> Self {
        Self::new(ModelConfig {
            model_id: model_id.into(),
            device: BackendDevice::Cpu,
            max_tokens: 256,
        })
    }

    fn generate(&self, prompt: &str) -> Result<String, String> {
        // Stub: real impl would load weights and run a forward pass.
        // Pattern: "compose <kind> for: <input>" → generate .nomx response.
        if prompt.contains("compose") {
            Ok(format!(
                "define result that compose-output for: {}",
                &prompt[..prompt.len().min(40)]
            ))
        } else {
            Ok(format!("result: {}", &prompt[..prompt.len().min(30)]))
        }
    }
}

impl InferenceFn for CandleAdapter {
    fn infer(&self, prompt: &str) -> Result<String, String> {
        self.generate(prompt)
    }

    fn model_name(&self) -> &str {
        &self.config.model_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candle_adapter_cpu_device() {
        let adapter = CandleAdapter::new_cpu("phi-3-mini");
        assert_eq!(adapter.config.device, BackendDevice::Cpu);
        assert!(adapter.config.device.is_cpu());
    }

    #[test]
    fn test_candle_adapter_generate_compose_prompt() {
        let adapter = CandleAdapter::new_cpu("phi-3-mini");
        let result = adapter.infer("compose video for: scene-1").unwrap();
        assert!(result.starts_with("define result that compose-output for:"));
    }

    #[test]
    fn test_candle_adapter_model_name() {
        let adapter = CandleAdapter::new_cpu("gemma-2b");
        assert_eq!(adapter.model_name(), "gemma-2b");
    }

    #[test]
    fn test_backend_device_is_cpu() {
        assert!(BackendDevice::Cpu.is_cpu());
        assert!(!BackendDevice::Cuda(0).is_cpu());
    }
}
