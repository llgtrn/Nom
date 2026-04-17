//! Media vendor abstraction — facade over local (nom-llvm, candle) + cloud
//! (Anthropic, OpenAI, Gemini, StabilityAI) providers.
#![deny(unsafe_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Capability {
    Text,
    Image,
    Video,
    Audio,
    Embedding,
    Code,
    ToolUse,
    Vision,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cost {
    pub cents_per_1k_input_tokens: u32,
    pub cents_per_1k_output_tokens: u32,
    pub fixed_cents_per_request: u32,
}

impl Cost {
    pub const FREE: Cost = Cost {
        cents_per_1k_input_tokens: 0,
        cents_per_1k_output_tokens: 0,
        fixed_cents_per_request: 0,
    };
    /// Estimated total cost for a single request.
    pub fn total_cents(&self, input_tokens: u32, output_tokens: u32) -> u64 {
        let input =
            (input_tokens as u64 * self.cents_per_1k_input_tokens as u64) / 1000;
        let output =
            (output_tokens as u64 * self.cents_per_1k_output_tokens as u64) / 1000;
        input + output + self.fixed_cents_per_request as u64
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenerateRequest {
    pub capability: Capability,
    pub prompt: String,
    pub max_output_tokens: u32,
    pub temperature: f32,
    pub seed: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenerateResponse {
    pub content: Vec<u8>,
    pub mime_type: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub elapsed_ms: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum VendorError {
    #[error("capability {0:?} not supported by {1}")]
    UnsupportedCapability(Capability, String),
    #[error("vendor {0} request timed out")]
    Timeout(String),
    #[error("vendor {0} returned error: {1}")]
    Upstream(String, String),
}

pub trait MediaVendor: Send + Sync {
    fn name(&self) -> &str;
    fn capabilities(&self) -> &[Capability];
    fn cost_per_request(&self, capability: Capability) -> Cost;
    fn generate(
        &self,
        request: &GenerateRequest,
    ) -> Result<GenerateResponse, VendorError>;
    /// Does this vendor advertise the given capability?
    fn supports(&self, capability: Capability) -> bool {
        self.capabilities().iter().any(|c| *c == capability)
    }
}

/// Stub vendor for tests + local development.
pub struct StubVendor {
    pub name: String,
    pub caps: Vec<Capability>,
    pub cost: Cost,
}

impl StubVendor {
    pub fn new(name: impl Into<String>, caps: Vec<Capability>) -> Self {
        Self { name: name.into(), caps, cost: Cost::FREE }
    }
    pub fn with_cost(mut self, cost: Cost) -> Self {
        self.cost = cost;
        self
    }
}

impl MediaVendor for StubVendor {
    fn name(&self) -> &str {
        &self.name
    }
    fn capabilities(&self) -> &[Capability] {
        &self.caps
    }
    fn cost_per_request(&self, _capability: Capability) -> Cost {
        self.cost
    }
    fn generate(
        &self,
        request: &GenerateRequest,
    ) -> Result<GenerateResponse, VendorError> {
        if !self.supports(request.capability) {
            return Err(VendorError::UnsupportedCapability(
                request.capability,
                self.name.clone(),
            ));
        }
        Ok(GenerateResponse {
            content: request.prompt.as_bytes().to_vec(),
            mime_type: "text/plain".to_string(),
            input_tokens: request.prompt.split_whitespace().count() as u32,
            output_tokens: 0,
            elapsed_ms: 0,
        })
    }
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_free_total_is_zero() {
        assert_eq!(Cost::FREE.total_cents(1_000_000, 1_000_000), 0);
    }

    #[test]
    fn cost_total_cents_arithmetic() {
        // 1000 in @ 2 c/1k + 500 out @ 4 c/1k + 1 fixed = 2 + 2 + 1 = 5
        let cost = Cost {
            cents_per_1k_input_tokens: 2,
            cents_per_1k_output_tokens: 4,
            fixed_cents_per_request: 1,
        };
        assert_eq!(cost.total_cents(1000, 500), 5);
    }

    #[test]
    fn stub_supports_declared_capabilities() {
        let v = StubVendor::new("v", vec![Capability::Text, Capability::Image]);
        assert!(v.supports(Capability::Text));
        assert!(v.supports(Capability::Image));
    }

    #[test]
    fn stub_does_not_support_undeclared_capabilities() {
        let v = StubVendor::new("v", vec![Capability::Text]);
        assert!(!v.supports(Capability::Video));
        assert!(!v.supports(Capability::Audio));
    }

    #[test]
    fn generate_unsupported_capability_returns_error_with_vendor_name() {
        let v = StubVendor::new("openai-stub", vec![Capability::Text]);
        let req = GenerateRequest {
            capability: Capability::Video,
            prompt: "hello".into(),
            max_output_tokens: 100,
            temperature: 0.7,
            seed: None,
        };
        let err = v.generate(&req).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("openai-stub"), "expected vendor name in: {msg}");
    }

    #[test]
    fn generate_supported_capability_echoes_prompt_bytes() {
        let v = StubVendor::new("local", vec![Capability::Text]);
        let req = GenerateRequest {
            capability: Capability::Text,
            prompt: "hello world".into(),
            max_output_tokens: 100,
            temperature: 0.0,
            seed: None,
        };
        let resp = v.generate(&req).unwrap();
        assert_eq!(resp.content, b"hello world");
        assert_eq!(resp.mime_type, "text/plain");
    }

    #[test]
    fn cost_per_request_returns_configured_cost() {
        let custom = Cost {
            cents_per_1k_input_tokens: 5,
            cents_per_1k_output_tokens: 10,
            fixed_cents_per_request: 2,
        };
        let v = StubVendor::new("v", vec![Capability::Code]).with_cost(custom);
        let returned = v.cost_per_request(Capability::Code);
        assert_eq!(returned, custom);
    }

    #[test]
    fn with_cost_builder_chains() {
        let cost = Cost {
            cents_per_1k_input_tokens: 1,
            cents_per_1k_output_tokens: 2,
            fixed_cents_per_request: 0,
        };
        let v = StubVendor::new("builder-test", vec![]).with_cost(cost);
        assert_eq!(v.cost.cents_per_1k_input_tokens, 1);
        assert_eq!(v.cost.cents_per_1k_output_tokens, 2);
    }

    #[test]
    fn vendor_error_display_timeout() {
        let e = VendorError::Timeout("openai-stub".into());
        let msg = e.to_string();
        assert_eq!(msg, "vendor openai-stub request timed out");
    }
}
