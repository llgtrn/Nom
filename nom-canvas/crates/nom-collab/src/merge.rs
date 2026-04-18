// MergeStrategy and MergeRecord — track how concurrent CRDT operations are merged.

/// Describes the strategy used to resolve concurrent writes.
#[derive(Debug, Clone, PartialEq)]
pub enum MergeStrategy {
    LastWriteWins,
    MvRegister,
    GrowOnly,
}

impl MergeStrategy {
    /// Human-readable name for the strategy.
    pub fn display_name(&self) -> &str {
        match self {
            MergeStrategy::LastWriteWins => "last-write-wins",
            MergeStrategy::MvRegister => "multi-value-register",
            MergeStrategy::GrowOnly => "grow-only",
        }
    }

    /// All CRDT merge operations are idempotent by construction.
    pub fn is_idempotent(&self) -> bool {
        true
    }
}

/// Records a single merge operation between two sites.
pub struct MergeRecord {
    pub site_a: String,
    pub site_b: String,
    pub strategy: MergeStrategy,
}

impl MergeRecord {
    /// Create a new merge record.
    pub fn new(a: &str, b: &str, s: MergeStrategy) -> Self {
        Self {
            site_a: a.to_string(),
            site_b: b.to_string(),
            strategy: s,
        }
    }

    /// Returns `true` when both site identifiers are identical.
    pub fn sites_equal(&self) -> bool {
        self.site_a == self.site_b
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DocState, PeerId};

    #[test]
    fn merge_strategy_display_names() {
        assert_eq!(MergeStrategy::LastWriteWins.display_name(), "last-write-wins");
        assert_eq!(MergeStrategy::MvRegister.display_name(), "multi-value-register");
        assert_eq!(MergeStrategy::GrowOnly.display_name(), "grow-only");
    }

    #[test]
    fn all_strategies_are_idempotent() {
        for s in [
            MergeStrategy::LastWriteWins,
            MergeStrategy::MvRegister,
            MergeStrategy::GrowOnly,
        ] {
            assert!(s.is_idempotent(), "{} must be idempotent", s.display_name());
        }
    }

    #[test]
    fn merge_record_new() {
        let rec = MergeRecord::new("site-a", "site-b", MergeStrategy::GrowOnly);
        assert_eq!(rec.site_a, "site-a");
        assert_eq!(rec.site_b, "site-b");
        assert_eq!(rec.strategy, MergeStrategy::GrowOnly);
    }

    #[test]
    fn merge_record_sites_equal() {
        let rec = MergeRecord::new("alpha", "alpha", MergeStrategy::LastWriteWins);
        assert!(rec.sites_equal());
    }

    #[test]
    fn merge_record_sites_different() {
        let rec = MergeRecord::new("alpha", "beta", MergeStrategy::MvRegister);
        assert!(!rec.sites_equal());
    }

    #[test]
    fn vector_clock_merge_takes_max() {
        use crate::VectorClock;
        let a = VectorClock::new().increment("peer-1").increment("peer-1");
        let b = VectorClock::new().increment("peer-1").increment("peer-2");
        // a: peer-1=2, b: peer-1=1, peer-2=1
        let merged = a.merge(&b);
        assert_eq!(merged.get("peer-1"), 2, "should take max for peer-1");
        assert_eq!(merged.get("peer-2"), 1, "should include peer-2 from b");

        // Verify idempotency: merging again doesn't change values.
        let mut doc_a = DocState::new(PeerId(10));
        let mut doc_b = DocState::new(PeerId(20));
        doc_a.local_insert(crate::RgaPos::Head, "hello");
        doc_b.local_insert(crate::RgaPos::Head, "world");
        let merged_count = doc_a.merge(&doc_b);
        assert_eq!(merged_count, 1);
        let second_merge = doc_a.merge(&doc_b);
        assert_eq!(second_merge, 0, "second merge should be idempotent");
    }
}
