//! Provider routing with quota tracking and strategy overrides.

use std::collections::HashMap;

/// How to select among available vendors.
#[derive(Debug, Clone, PartialEq)]
pub enum FallbackStrategy {
    /// Try vendors in order; pick first that is under quota.
    Fallback,
    /// Rotate through vendors in round-robin fashion.
    RoundRobin,
    /// Stay on one vendor until its quota is full, then move.
    FillFirst,
}

/// Per-vendor usage and rate-limit state.
#[derive(Debug, Clone)]
pub struct VendorQuota {
    pub used: u64,
    pub limit: u64,
    pub reset_at_ms: u64,
}

impl VendorQuota {
    fn is_over(&self) -> bool {
        self.used >= self.limit
    }
}

pub struct ProviderRouter {
    vendors: Vec<(String, VendorQuota)>,
    strategy: FallbackStrategy,
    cursor: usize,
    combo_strategies: HashMap<String, FallbackStrategy>,
}

impl ProviderRouter {
    pub fn new(strategy: FallbackStrategy) -> Self {
        ProviderRouter {
            vendors: Vec::new(),
            strategy,
            cursor: 0,
            combo_strategies: HashMap::new(),
        }
    }

    pub fn add_vendor(&mut self, name: impl Into<String>, limit: u64) {
        self.vendors.push((
            name.into(),
            VendorQuota {
                used: 0,
                limit,
                reset_at_ms: 0,
            },
        ));
    }

    /// Register a per-combo-key strategy override.
    pub fn set_combo_strategy(&mut self, combo_key: impl Into<String>, strategy: FallbackStrategy) {
        self.combo_strategies.insert(combo_key.into(), strategy);
    }

    /// Pick a vendor for the given optional combo key.
    /// Returns `None` when all vendors are over quota or the list is empty.
    pub fn pick_vendor(&mut self, combo_key: Option<&str>) -> Option<String> {
        if self.vendors.is_empty() {
            return None;
        }

        let strategy = match combo_key.and_then(|k| self.combo_strategies.get(k)) {
            Some(s) => s.clone(),
            None => self.strategy.clone(),
        };

        match strategy {
            FallbackStrategy::Fallback => {
                self.vendors
                    .iter()
                    .find(|(_, q)| !q.is_over())
                    .map(|(name, _)| name.clone())
            }
            FallbackStrategy::RoundRobin => {
                let len = self.vendors.len();
                for i in 0..len {
                    let idx = (self.cursor + i) % len;
                    if !self.vendors[idx].1.is_over() {
                        self.cursor = (idx + 1) % len;
                        return Some(self.vendors[idx].0.clone());
                    }
                }
                None
            }
            FallbackStrategy::FillFirst => {
                // Stay on current cursor vendor until it is full.
                if self.cursor < self.vendors.len()
                    && !self.vendors[self.cursor].1.is_over()
                {
                    return Some(self.vendors[self.cursor].0.clone());
                }
                // Advance cursor.
                let len = self.vendors.len();
                for i in 1..=len {
                    let idx = (self.cursor + i) % len;
                    if !self.vendors[idx].1.is_over() {
                        self.cursor = idx;
                        return Some(self.vendors[idx].0.clone());
                    }
                }
                None
            }
        }
    }

    /// Record usage against a vendor.
    pub fn record_use(&mut self, name: &str, amount: u64) {
        if let Some((_, q)) = self.vendors.iter_mut().find(|(n, _)| n == name) {
            q.used += amount;
        }
    }
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_router_returns_none() {
        let mut r = ProviderRouter::new(FallbackStrategy::Fallback);
        assert!(r.pick_vendor(None).is_none());
    }

    #[test]
    fn fallback_picks_first_non_full() {
        let mut r = ProviderRouter::new(FallbackStrategy::Fallback);
        r.add_vendor("a", 10);
        r.add_vendor("b", 10);
        r.record_use("a", 10); // saturate a
        assert_eq!(r.pick_vendor(None).as_deref(), Some("b"));
    }

    #[test]
    fn round_robin_rotates() {
        let mut r = ProviderRouter::new(FallbackStrategy::RoundRobin);
        r.add_vendor("x", 100);
        r.add_vendor("y", 100);
        let first = r.pick_vendor(None).unwrap();
        let second = r.pick_vendor(None).unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn fill_first_stays_until_full() {
        let mut r = ProviderRouter::new(FallbackStrategy::FillFirst);
        r.add_vendor("p", 2);
        r.add_vendor("q", 10);
        // p not full yet
        assert_eq!(r.pick_vendor(None).as_deref(), Some("p"));
        r.record_use("p", 2); // now full
        assert_eq!(r.pick_vendor(None).as_deref(), Some("q"));
    }

    #[test]
    fn all_over_quota_returns_none() {
        let mut r = ProviderRouter::new(FallbackStrategy::Fallback);
        r.add_vendor("a", 5);
        r.record_use("a", 5);
        assert!(r.pick_vendor(None).is_none());
    }

    #[test]
    fn combo_strategies_override() {
        let mut r = ProviderRouter::new(FallbackStrategy::Fallback);
        r.add_vendor("a", 100);
        r.add_vendor("b", 100);
        r.set_combo_strategy("vip", FallbackStrategy::RoundRobin);
        let f1 = r.pick_vendor(Some("vip")).unwrap();
        let f2 = r.pick_vendor(Some("vip")).unwrap();
        assert_ne!(f1, f2, "round-robin should rotate for vip combo");
    }

    #[test]
    fn quota_tracking_accumulates() {
        let mut r = ProviderRouter::new(FallbackStrategy::Fallback);
        r.add_vendor("svc", 20);
        r.record_use("svc", 10);
        r.record_use("svc", 5);
        let (_, q) = r.vendors.iter().find(|(n, _)| n == "svc").unwrap();
        assert_eq!(q.used, 15);
    }

    #[test]
    fn fallback_all_available_picks_first() {
        let mut r = ProviderRouter::new(FallbackStrategy::Fallback);
        r.add_vendor("first", 10);
        r.add_vendor("second", 10);
        assert_eq!(r.pick_vendor(None).as_deref(), Some("first"));
    }
}
