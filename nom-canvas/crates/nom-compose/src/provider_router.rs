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
    pub fn new() -> Self {
        Self {
            vendors: Vec::new(),
        }
    }

    pub fn register(&mut self, vendor: impl MediaVendor + 'static, level: FallbackLevel) {
        self.vendors.push(VendorEntry {
            vendor: Box::new(vendor),
            level,
        });
    }

    /// Find the best vendor for a backend kind (lowest level = highest priority).
    pub fn route(&self, kind: &BackendKind) -> Option<&dyn MediaVendor> {
        self.vendors
            .iter()
            .filter(|e| e.vendor.supports(kind))
            .min_by_key(|e| e.level as u8)
            .map(|e| e.vendor.as_ref())
    }

    pub fn vendor_count(&self) -> usize {
        self.vendors.len()
    }

    /// Compose via registered vendors with optional fallback through tiers.
    ///
    /// - `try_fallbacks: false` — only tries the primary (lowest-level) vendor.
    /// - `try_fallbacks: true`  — tries all tiers in priority order; on failure
    ///   waits `retry_delay_ms` before the next attempt (skipped in tests via
    ///   cfg(test) guard inside — callers should pass `false` in unit tests).
    pub fn compose_with_fallback(
        &self,
        kind: &BackendKind,
        input: &str,
        progress: &dyn Fn(f32),
        try_fallbacks: bool,
    ) -> Result<String, String> {
        let mut entries: Vec<&VendorEntry> = self
            .vendors
            .iter()
            .filter(|e| e.vendor.supports(kind))
            .collect();
        if entries.is_empty() {
            return Err(format!("no vendor registered for kind: {}", kind.name()));
        }
        entries.sort_by_key(|e| e.level as u8);

        if !try_fallbacks {
            return entries[0].vendor.compose(kind, input, progress);
        }

        let mut last_err = String::new();
        for entry in &entries {
            match entry.vendor.compose(kind, input, progress) {
                Ok(out) => return Ok(out),
                Err(e) => {
                    last_err = e;
                    // In non-test builds, wait before the next tier.
                    #[cfg(not(test))]
                    std::thread::sleep(std::time::Duration::from_millis(
                        entry.level.retry_delay_ms(),
                    ));
                }
            }
        }
        Err(last_err)
    }

    pub fn vendors_for(&self, kind: &BackendKind) -> Vec<&dyn MediaVendor> {
        let mut entries: Vec<&VendorEntry> = self
            .vendors
            .iter()
            .filter(|e| e.vendor.supports(kind))
            .collect();
        entries.sort_by_key(|e| e.level as u8);
        entries.iter().map(|e| e.vendor.as_ref()).collect()
    }
}

impl Default for ProviderRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vendor_trait::{FailingVendor, StubMediaVendor, StubVendor};

    #[test]
    fn router_fallback_level_retry_delay() {
        assert_eq!(FallbackLevel::Primary.retry_delay_ms(), 1000);
        assert_eq!(FallbackLevel::Secondary.retry_delay_ms(), 2000);
        assert_eq!(FallbackLevel::Tertiary.retry_delay_ms(), 4000);
    }
    #[test]
    fn router_routes_to_primary() {
        let mut r = ProviderRouter::new();
        r.register(
            StubVendor {
                name: "fallback",
                kind: BackendKind::Video,
            },
            FallbackLevel::Secondary,
        );
        r.register(
            StubVendor {
                name: "primary",
                kind: BackendKind::Video,
            },
            FallbackLevel::Primary,
        );
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
        r.register(
            StubVendor {
                name: "b",
                kind: BackendKind::Audio,
            },
            FallbackLevel::Secondary,
        );
        r.register(
            StubVendor {
                name: "a",
                kind: BackendKind::Audio,
            },
            FallbackLevel::Primary,
        );
        let vs = r.vendors_for(&BackendKind::Audio);
        assert_eq!(vs[0].name(), "a");
        assert_eq!(vs[1].name(), "b");
    }

    #[test]
    fn provider_router_primary_succeeds() {
        let mut r = ProviderRouter::new();
        r.register(
            StubMediaVendor {
                name: "primary",
                kind: BackendKind::Image,
            },
            FallbackLevel::Primary,
        );
        let result = r.compose_with_fallback(&BackendKind::Image, "hello", &|_| {}, false);
        assert_eq!(result, Ok("stub_output".to_string()));
    }

    #[test]
    fn provider_router_fallback_to_secondary_on_primary_failure() {
        let mut r = ProviderRouter::new();
        r.register(
            FailingVendor {
                name: "fail_primary",
                kind: BackendKind::Image,
            },
            FallbackLevel::Primary,
        );
        r.register(
            StubMediaVendor {
                name: "ok_secondary",
                kind: BackendKind::Image,
            },
            FallbackLevel::Secondary,
        );
        let result = r.compose_with_fallback(&BackendKind::Image, "hello", &|_| {}, true);
        assert_eq!(result, Ok("stub_output".to_string()));
    }

    #[test]
    fn provider_router_no_vendor_returns_err() {
        let r = ProviderRouter::new();
        let result = r.compose_with_fallback(&BackendKind::Image, "hello", &|_| {}, true);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("image"));
    }

    #[test]
    fn provider_router_compose_success() {
        let mut r = ProviderRouter::new();
        r.register(
            StubMediaVendor {
                name: "primary",
                kind: BackendKind::Data,
            },
            FallbackLevel::Primary,
        );
        r.register(
            StubMediaVendor {
                name: "secondary",
                kind: BackendKind::Data,
            },
            FallbackLevel::Secondary,
        );
        let result = r.compose_with_fallback(&BackendKind::Data, "input", &|_| {}, false);
        assert_eq!(result, Ok("stub_output".to_string()));
        // primary succeeds so vendor_count still 2 (no removal on success)
        assert_eq!(r.vendor_count(), 2);
    }

    #[test]
    fn provider_router_fallback_used_on_err() {
        let mut r = ProviderRouter::new();
        r.register(
            FailingVendor {
                name: "bad_primary",
                kind: BackendKind::Document,
            },
            FallbackLevel::Primary,
        );
        r.register(
            StubMediaVendor {
                name: "good_fallback",
                kind: BackendKind::Document,
            },
            FallbackLevel::Secondary,
        );
        let result = r.compose_with_fallback(&BackendKind::Document, "data", &|_| {}, true);
        assert_eq!(result, Ok("stub_output".to_string()));
    }

    #[test]
    fn provider_router_no_fallbacks_propagates_err() {
        let mut r = ProviderRouter::new();
        r.register(
            FailingVendor {
                name: "fail1",
                kind: BackendKind::Audio,
            },
            FallbackLevel::Primary,
        );
        r.register(
            FailingVendor {
                name: "fail2",
                kind: BackendKind::Audio,
            },
            FallbackLevel::Secondary,
        );
        r.register(
            FailingVendor {
                name: "fail3",
                kind: BackendKind::Audio,
            },
            FallbackLevel::Tertiary,
        );
        let result = r.compose_with_fallback(&BackendKind::Audio, "x", &|_| {}, true);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "error");
    }

    #[test]
    fn provider_router_progress_called() {
        use std::cell::Cell;
        let mut r = ProviderRouter::new();
        r.register(
            StubMediaVendor {
                name: "v",
                kind: BackendKind::Video,
            },
            FallbackLevel::Primary,
        );
        let called = Cell::new(false);
        let _ = r.compose_with_fallback(
            &BackendKind::Video,
            "clip",
            &|p| {
                if p >= 1.0 {
                    called.set(true);
                }
            },
            false,
        );
        assert!(called.get());
    }

    #[test]
    fn fallback_level_ordering() {
        assert!(FallbackLevel::Primary < FallbackLevel::Secondary);
        assert!(FallbackLevel::Secondary < FallbackLevel::Tertiary);
    }

    #[test]
    fn provider_router_vendor_count_tracks_registrations() {
        let mut r = ProviderRouter::new();
        assert_eq!(r.vendor_count(), 0);
        r.register(
            StubVendor {
                name: "a",
                kind: BackendKind::Data,
            },
            FallbackLevel::Primary,
        );
        assert_eq!(r.vendor_count(), 1);
        r.register(
            StubVendor {
                name: "b",
                kind: BackendKind::Data,
            },
            FallbackLevel::Secondary,
        );
        assert_eq!(r.vendor_count(), 2);
    }

    #[test]
    fn provider_router_route_picks_highest_priority_among_multiple() {
        let mut r = ProviderRouter::new();
        // Register tertiary before primary intentionally.
        r.register(
            StubVendor {
                name: "tertiary",
                kind: BackendKind::Document,
            },
            FallbackLevel::Tertiary,
        );
        r.register(
            StubVendor {
                name: "primary",
                kind: BackendKind::Document,
            },
            FallbackLevel::Primary,
        );
        r.register(
            StubVendor {
                name: "secondary",
                kind: BackendKind::Document,
            },
            FallbackLevel::Secondary,
        );
        let v = r.route(&BackendKind::Document).unwrap();
        assert_eq!(v.name(), "primary");
    }

    #[test]
    fn provider_router_all_failing_returns_last_error() {
        let mut r = ProviderRouter::new();
        r.register(
            FailingVendor {
                name: "f1",
                kind: BackendKind::Workflow,
            },
            FallbackLevel::Primary,
        );
        let result = r.compose_with_fallback(&BackendKind::Workflow, "x", &|_| {}, true);
        assert!(result.is_err());
    }

    #[test]
    fn provider_router_vendors_for_empty_when_no_match() {
        let mut r = ProviderRouter::new();
        r.register(
            StubVendor {
                name: "v",
                kind: BackendKind::Image,
            },
            FallbackLevel::Primary,
        );
        // Request kind that has no registered vendor.
        let vs = r.vendors_for(&BackendKind::Video);
        assert!(vs.is_empty());
    }

    #[test]
    fn fallback_exhaustion_all_tiers_fail() {
        let mut r = ProviderRouter::new();
        r.register(
            FailingVendor {
                name: "p",
                kind: BackendKind::Image,
            },
            FallbackLevel::Primary,
        );
        r.register(
            FailingVendor {
                name: "s",
                kind: BackendKind::Image,
            },
            FallbackLevel::Secondary,
        );
        r.register(
            FailingVendor {
                name: "t",
                kind: BackendKind::Image,
            },
            FallbackLevel::Tertiary,
        );
        let result = r.compose_with_fallback(&BackendKind::Image, "x", &|_| {}, true);
        assert!(
            result.is_err(),
            "all tiers failing must return Err"
        );
    }

    #[test]
    fn retry_delay_backoff_math() {
        // Primary = 2^0 * 1000 = 1000, Secondary = 2^1 * 1000 = 2000, Tertiary = 2^2 * 1000 = 4000
        assert_eq!(FallbackLevel::Primary.retry_delay_ms(), 1000);
        assert_eq!(FallbackLevel::Secondary.retry_delay_ms(), 2000);
        assert_eq!(FallbackLevel::Tertiary.retry_delay_ms(), 4000);
        // Each level doubles.
        assert_eq!(
            FallbackLevel::Secondary.retry_delay_ms(),
            FallbackLevel::Primary.retry_delay_ms() * 2
        );
        assert_eq!(
            FallbackLevel::Tertiary.retry_delay_ms(),
            FallbackLevel::Secondary.retry_delay_ms() * 2
        );
    }

    #[test]
    fn provider_router_tertiary_only_succeeds() {
        let mut r = ProviderRouter::new();
        r.register(
            StubMediaVendor {
                name: "tertiary_ok",
                kind: BackendKind::Render,
            },
            FallbackLevel::Tertiary,
        );
        let result = r.compose_with_fallback(&BackendKind::Render, "data", &|_| {}, false);
        assert_eq!(result, Ok("stub_output".to_string()));
    }

    #[test]
    fn provider_router_multiple_success_returns_primary_output() {
        let mut r = ProviderRouter::new();
        r.register(
            StubMediaVendor {
                name: "p",
                kind: BackendKind::Data,
            },
            FallbackLevel::Primary,
        );
        r.register(
            StubMediaVendor {
                name: "s",
                kind: BackendKind::Data,
            },
            FallbackLevel::Secondary,
        );
        // try_fallbacks=false: only primary is tried
        let result = r.compose_with_fallback(&BackendKind::Data, "in", &|_| {}, false);
        assert!(result.is_ok());
    }
}
