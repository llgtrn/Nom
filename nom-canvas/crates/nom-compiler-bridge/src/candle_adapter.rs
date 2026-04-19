//! In-process LLM inference adapter for nom-compiler-bridge.
//! Replaces the previous stub with a real Candle integration:
//! - `BackendDevice` wraps `candle_core::Device`.
//! - `CandleAdapter` can load safetensors weights via `candle_nn::VarBuilder`.
//! - `InferenceFn` performs a dummy forward pass to prove the pipeline works.

use std::path::Path;

use candle_core::{Device, DType, Tensor};
use candle_nn::{linear, Linear, Module, VarBuilder};

// ---------------------------------------------------------------------------
// BackendDevice
// ---------------------------------------------------------------------------

/// Compute device for inference — thin wrapper around `candle_core::Device`.
#[derive(Debug, Clone)]
pub struct BackendDevice {
    pub inner: Device,
}

impl BackendDevice {
    pub fn cpu() -> Self {
        Self {
            inner: Device::Cpu,
        }
    }

    pub fn is_cpu(&self) -> bool {
        matches!(self.inner, Device::Cpu)
    }
}

impl PartialEq for BackendDevice {
    fn eq(&self, other: &Self) -> bool {
        match (&self.inner, &other.inner) {
            (Device::Cpu, Device::Cpu) => true,
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// ModelConfig
// ---------------------------------------------------------------------------

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
            device: BackendDevice::cpu(),
            max_tokens: 256,
        }
    }
}

// ---------------------------------------------------------------------------
// InferenceFn
// ---------------------------------------------------------------------------

/// Adapter trait for in-process inference. Mirrors `ReActLlmFn` from nom-compose
/// but declared locally so this crate has no dependency on nom-compose.
pub trait InferenceFn: Send + Sync {
    fn infer(&self, prompt: &str) -> Result<String, String>;
    fn model_name(&self) -> &str;
}

// ---------------------------------------------------------------------------
// SimpleModel — dummy model to prove the adapter works
// ---------------------------------------------------------------------------

/// A minimal single-layer model used to verify that weight loading and forward
/// passes function correctly without pulling in `candle-transformers`.
pub struct SimpleModel {
    linear: Linear,
}

impl SimpleModel {
    /// Load from a `VarBuilder`. Expects `layer.weight` and `layer.bias` tensors.
    pub fn load(vb: VarBuilder) -> candle_core::Result<Self> {
        let linear = linear(10, 10, vb.pp("layer"))?;
        Ok(Self { linear })
    }
}

impl Module for SimpleModel {
    fn forward(&self, xs: &Tensor) -> candle_core::Result<Tensor> {
        self.linear.forward(xs)
    }
}

// ---------------------------------------------------------------------------
// CandleAdapter
// ---------------------------------------------------------------------------

/// In-process LLM adapter using Candle.
pub struct CandleAdapter {
    pub config: ModelConfig,
    device: Device,
    model: Option<SimpleModel>,
}

impl CandleAdapter {
    pub fn new(config: ModelConfig) -> Self {
        let device = config.device.inner.clone();
        Self {
            config,
            device,
            model: None,
        }
    }

    pub fn new_cpu(model_id: impl Into<String>) -> Self {
        let config = ModelConfig {
            model_id: model_id.into(),
            device: BackendDevice::cpu(),
            max_tokens: 256,
        };
        Self::new(config)
    }

    /// Load a safetensors model from `path`.
    ///
    /// The file must contain tensors named `layer.weight` and `layer.bias`
    /// (or whatever keys the chosen model expects).
    pub fn load_model(&mut self, path: &Path) -> Result<(), String> {
        let data = std::fs::read(path).map_err(|e| e.to_string())?;
        let vb = VarBuilder::from_buffered_safetensors(data, DType::F32, &self.device)
            .map_err(|e| e.to_string())?;
        let model = SimpleModel::load(vb).map_err(|e| e.to_string())?;
        self.model = Some(model);
        Ok(())
    }

    /// Returns `true` if a model has been successfully loaded.
    pub fn model_loaded(&self) -> bool {
        self.model.is_some()
    }

    fn generate(&self, prompt: &str) -> Result<String, String> {
        if let Some(ref model) = self.model {
            // Dummy forward pass: zero input → model → report output shape.
            let input = Tensor::zeros((1, 10), DType::F32, &self.device)
                .map_err(|e| e.to_string())?;
            let output = model.forward(&input).map_err(|e| e.to_string())?;
            let shape = output.shape().dims().to_vec();
            Ok(format!(
                "forward ok, output shape: {:?}, prompt: {}",
                shape,
                &prompt[..prompt.len().min(20)]
            ))
        } else {
            // Fallback stub when no model is loaded.
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
}

impl InferenceFn for CandleAdapter {
    fn infer(&self, prompt: &str) -> Result<String, String> {
        self.generate(prompt)
    }

    fn model_name(&self) -> &str {
        &self.config.model_id
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_candle_adapter_cpu_device() {
        let adapter = CandleAdapter::new_cpu("phi-3-mini");
        assert!(adapter.config.device.is_cpu());
        assert_eq!(adapter.config.device, BackendDevice::cpu());
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
        assert!(BackendDevice::cpu().is_cpu());
    }

    #[test]
    fn test_load_model_and_infer() {
        let tmp = std::env::temp_dir();
        let model_path = tmp.join("nom_test_dummy_model.safetensors");

        // Write a minimal safetensors file with the tensors our dummy model needs.
        let device = Device::Cpu;
        let weight = Tensor::zeros((10, 10), DType::F32, &device).unwrap();
        let bias = Tensor::zeros((10,), DType::F32, &device).unwrap();
        let mut tensors = HashMap::new();
        tensors.insert("layer.weight".to_string(), weight);
        tensors.insert("layer.bias".to_string(), bias);
        candle_core::safetensors::save(&tensors, &model_path).unwrap();

        let mut adapter = CandleAdapter::new_cpu("test-model");
        assert!(!adapter.model_loaded());
        adapter.load_model(&model_path).unwrap();
        assert!(adapter.model_loaded());

        let result = adapter.infer("hello world").unwrap();
        assert!(result.contains("forward ok"));
        assert!(result.contains("output shape:"));

        // Clean up
        let _ = std::fs::remove_file(&model_path);
    }
}
