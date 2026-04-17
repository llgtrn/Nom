#![deny(unsafe_code)]

use crate::dispatch::BackendKind;

/// Cost estimate for a vendor API call (in microcents for precision).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CostEstimate {
    pub microcents_per_call: u64,
    pub max_microcents: u64,
}

impl CostEstimate {
    pub fn new(microcents_per_call: u64, max_microcents: u64) -> Self {
        Self {
            microcents_per_call,
            max_microcents,
        }
    }
    pub fn free() -> Self {
        Self {
            microcents_per_call: 0,
            max_microcents: 0,
        }
    }
    pub fn dollars_per_call(&self) -> f64 {
        self.microcents_per_call as f64 / 100_000.0
    }
}

/// Vendor capability flags.
#[derive(Debug, Clone, Default)]
pub struct VendorCapability {
    pub supports_streaming: bool,
    pub supports_batch: bool,
    pub max_input_bytes: usize,
    pub quality_tier: u8, // 1=basic, 2=standard, 3=premium
}

/// Trait for any backend vendor (9router pattern).
pub trait MediaVendor: Send + Sync {
    fn name(&self) -> &'static str;
    fn supports(&self, kind: &BackendKind) -> bool;
    fn capability(&self) -> VendorCapability;
    fn cost_estimate(&self) -> CostEstimate;
    /// Execute a compose call. Returns Ok(output) or Err(reason).
    fn compose(
        &self,
        kind: &BackendKind,
        input: &str,
        progress: &dyn Fn(f32),
    ) -> Result<String, String>;
}

/// A stub vendor for testing.
pub struct StubVendor {
    pub name: &'static str,
    pub kind: BackendKind,
}

impl MediaVendor for StubVendor {
    fn name(&self) -> &'static str {
        self.name
    }
    fn supports(&self, k: &BackendKind) -> bool {
        k == &self.kind
    }
    fn capability(&self) -> VendorCapability {
        VendorCapability {
            supports_streaming: false,
            supports_batch: false,
            max_input_bytes: 1024 * 1024,
            quality_tier: 1,
        }
    }
    fn cost_estimate(&self) -> CostEstimate {
        CostEstimate::free()
    }
    fn compose(
        &self,
        _kind: &BackendKind,
        input: &str,
        progress: &dyn Fn(f32),
    ) -> Result<String, String> {
        progress(1.0);
        Ok(format!("stub:{}", input))
    }
}

/// A vendor that always succeeds, returning "stub_output".
pub struct StubMediaVendor {
    pub name: &'static str,
    pub kind: BackendKind,
}

impl MediaVendor for StubMediaVendor {
    fn name(&self) -> &'static str {
        self.name
    }
    fn supports(&self, k: &BackendKind) -> bool {
        k == &self.kind
    }
    fn capability(&self) -> VendorCapability {
        VendorCapability {
            supports_streaming: false,
            supports_batch: false,
            max_input_bytes: 1024 * 1024,
            quality_tier: 2,
        }
    }
    fn cost_estimate(&self) -> CostEstimate {
        CostEstimate::free()
    }
    fn compose(
        &self,
        _kind: &BackendKind,
        _input: &str,
        progress: &dyn Fn(f32),
    ) -> Result<String, String> {
        progress(1.0);
        Ok("stub_output".to_string())
    }
}

/// A vendor that always fails with "error".
pub struct FailingVendor {
    pub name: &'static str,
    pub kind: BackendKind,
}

impl MediaVendor for FailingVendor {
    fn name(&self) -> &'static str {
        self.name
    }
    fn supports(&self, k: &BackendKind) -> bool {
        k == &self.kind
    }
    fn capability(&self) -> VendorCapability {
        VendorCapability::default()
    }
    fn cost_estimate(&self) -> CostEstimate {
        CostEstimate::free()
    }
    fn compose(
        &self,
        _kind: &BackendKind,
        _input: &str,
        _progress: &dyn Fn(f32),
    ) -> Result<String, String> {
        Err("error".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stub_vendor_supports_its_kind() {
        let v = StubVendor {
            name: "test",
            kind: BackendKind::Video,
        };
        assert!(v.supports(&BackendKind::Video));
        assert!(!v.supports(&BackendKind::Audio));
    }
    #[test]
    fn cost_estimate_free_is_zero() {
        let c = CostEstimate::free();
        assert_eq!(c.microcents_per_call, 0);
        assert_eq!(c.dollars_per_call(), 0.0);
    }
    #[test]
    fn cost_estimate_dollars_conversion() {
        let c = CostEstimate::new(100_000, 10_000_000);
        assert!((c.dollars_per_call() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn stub_vendor_compose_returns_ok() {
        let v = StubMediaVendor {
            name: "media_stub",
            kind: BackendKind::Image,
        };
        let result = v.compose(&BackendKind::Image, "payload", &|_| {});
        assert_eq!(result, Ok("stub_output".to_string()));
    }

    #[test]
    fn stub_vendor_kind_is_stub() {
        let v = StubMediaVendor {
            name: "media_stub",
            kind: BackendKind::Audio,
        };
        assert_eq!(v.name(), "media_stub");
        assert!(v.supports(&BackendKind::Audio));
        assert!(!v.supports(&BackendKind::Video));
    }
}
