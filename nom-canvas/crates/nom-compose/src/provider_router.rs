#![deny(unsafe_code)]

use crate::dispatch::BackendKind;
use crate::vendor_trait::MediaVendor;

/// Fallback level — 9router 3-tier pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FallbackLevel {
    Primary = 0,
    Secondary = 1,
    Tertiary = 2,
}

impl FallbackLevel {
    /// Retry delay in ms: min(1000 * 2^level, 120_000) — 9router spec.
    pub fn retry_delay_ms(&self) -> u64 {
        let level = *self as u32;
        (1000u64 * 2u64.pow(level)).min(120_000)
    }
}

/// Registered vendor with priority.
pub struct VendorEntry {
    pub vendor: Box<dyn MediaVendor>,
    pub level: FallbackLevel,
}

/// Routes a compose request to the best available vendor.
pub struct ProviderRouter {
    vendors: Vec<VendorEntry>,
}

impl ProviderRouter {
    pub fn new() -> Self { Self { vendors: Vec::new() } }

    pub fn register(&mut self, vendor: impl MediaVendor + 'static, level: FallbackLevel) {
        self.vendors.push(VendorEntry { vendor: Box::new(vendor), level });
    }

    /// Find the best vendor for a backend kind (lowest level = highest priority).
    pub fn route(&self, kind: &BackendKind) -> Option<&dyn MediaVendor> {
        self.vendors.iter()
            .filter(|e| e.vendor.supports(kind))
            .min_by_key(|e| e.level as u8)
            .map(|e| e.vendor.as_ref())
    }

    pub fn vendor_count(&self) -> usize { self.vendors.len() }

    pub fn vendors_for(&self, kind: &BackendKind) -> Vec<&dyn MediaVendor> {
        let mut entries: Vec<&VendorEntry> = self.vendors.iter().filter(|e| e.vendor.supports(kind)).collect();
        entries.sort_by_key(|e| e.level as u8);
        entries.iter().map(|e| e.vendor.as_ref()).collect()
    }
}

impl Default for ProviderRouter { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vendor_trait::StubVendor;

    #[test]
    fn router_fallback_level_retry_delay() {
        assert_eq!(FallbackLevel::Primary.retry_delay_ms(), 1000);
        assert_eq!(FallbackLevel::Secondary.retry_delay_ms(), 2000);
        assert_eq!(FallbackLevel::Tertiary.retry_delay_ms(), 4000);
    }
    #[test]
    fn router_routes_to_primary() {
        let mut r = ProviderRouter::new();
        r.register(StubVendor { name: "fallback", kind: BackendKind::Video }, FallbackLevel::Secondary);
        r.register(StubVendor { name: "primary", kind: BackendKind::Video }, FallbackLevel::Primary);
        let v = r.route(&BackendKind::Video).unwrap();
        assert_eq!(v.name(), "primary");
    }
    #[test]
    fn router_returns_none_for_unsupported() {
        let r = ProviderRouter::new();
        assert!(r.route(&BackendKind::Video).is_none());
    }
    #[test]
    fn router_vendors_for_returns_sorted() {
        let mut r = ProviderRouter::new();
        r.register(StubVendor { name: "b", kind: BackendKind::Audio }, FallbackLevel::Secondary);
        r.register(StubVendor { name: "a", kind: BackendKind::Audio }, FallbackLevel::Primary);
        let vs = r.vendors_for(&BackendKind::Audio);
        assert_eq!(vs[0].name(), "a");
        assert_eq!(vs[1].name(), "b");
    }
}
