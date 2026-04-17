#![deny(unsafe_code)]
use crate::tracked::TrackedSnapshot;

/// Captures what a memoized function read and validates that those reads
/// are still valid before returning a cached result (typst comemo pattern).
/// Validation checks (method_id, return_hash) pairs — the cached result is
/// only valid if every method call returns the same hash as when it was computed.
#[derive(Clone, Debug)]
pub struct Constraint {
    /// Snapshots of (method_id, return_hash) pairs from the computation
    snapshots: Vec<TrackedSnapshot>,
    /// Hash of all inputs at computation time
    input_hash: u64,
}

impl Constraint {
    /// Create a new constraint for tracking
    pub fn new(input_hash: u64) -> Self {
        Self { snapshots: Vec::new(), input_hash }
    }

    /// Record a tracked snapshot
    pub fn record(&mut self, snapshot: TrackedSnapshot) {
        self.snapshots.push(snapshot);
    }

    /// Validate: cached result is valid if the input hash matches
    /// and all tracked value snapshots have matching (method_id, return_hash) pairs.
    pub fn validate(&self, current_input_hash: u64, current_snapshots: &[TrackedSnapshot]) -> bool {
        if self.input_hash != current_input_hash { return false; }
        if current_snapshots.len() < self.snapshots.len() { return false; }
        for (recorded, current) in self.snapshots.iter().zip(current_snapshots) {
            if recorded.version != current.version { return false; }
            if recorded.method_call_pairs.len() != current.method_call_pairs.len() { return false; }
            for (rec_pair, cur_pair) in recorded.method_call_pairs.iter().zip(&current.method_call_pairs) {
                if rec_pair.0 != cur_pair.0 { return false; } // method_id mismatch
                if rec_pair.1 != cur_pair.1 { return false; } // return_hash mismatch
            }
        }
        true
    }

    pub fn snapshot_count(&self) -> usize { self.snapshots.len() }
    pub fn input_hash(&self) -> u64 { self.input_hash }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::Hash128;
    use crate::tracked::TrackedSnapshot;

    fn snap(version: u64, pairs: Vec<(u32, Hash128)>) -> TrackedSnapshot {
        TrackedSnapshot { version, method_call_pairs: pairs }
    }

    #[test]
    fn constraint_validates_matching_no_calls() {
        let mut c = Constraint::new(12345);
        c.record(snap(1, vec![]));
        assert!(c.validate(12345, &[snap(1, vec![])]));
    }

    #[test]
    fn constraint_validates_with_matching_pairs() {
        let h = Hash128::of_str("result");
        let mut c = Constraint::new(42);
        c.record(snap(1, vec![(7, h)]));
        assert!(c.validate(42, &[snap(1, vec![(7, h)])]));
    }

    #[test]
    fn constraint_rejects_changed_return_hash() {
        let h1 = Hash128::of_str("result_v1");
        let h2 = Hash128::of_str("result_v2");
        let mut c = Constraint::new(42);
        c.record(snap(1, vec![(7, h1)]));
        // Same method_id but different return hash → stale
        assert!(!c.validate(42, &[snap(1, vec![(7, h2)])]));
    }

    #[test]
    fn constraint_rejects_stale_version() {
        let mut c = Constraint::new(12345);
        c.record(snap(1, vec![]));
        assert!(!c.validate(12345, &[snap(2, vec![])]));
    }

    #[test]
    fn constraint_rejects_changed_input() {
        let c = Constraint::new(12345);
        assert!(!c.validate(99999, &[]));
    }

    #[test]
    fn constraint_rejects_mismatched_method_id() {
        let h = Hash128::of_str("result");
        let mut c = Constraint::new(42);
        c.record(snap(1, vec![(7, h)]));
        // method_id changed from 7 to 8
        assert!(!c.validate(42, &[snap(1, vec![(8, h)])]));
    }

    #[test]
    fn constraint_input_hash_accessor() {
        let c = Constraint::new(99);
        assert_eq!(c.input_hash(), 99);
    }

    #[test]
    fn constraint_snapshot_count_zero_initially() {
        let c = Constraint::new(1);
        assert_eq!(c.snapshot_count(), 0);
    }

    #[test]
    fn constraint_snapshot_count_increments_on_record() {
        let mut c = Constraint::new(1);
        c.record(snap(1, vec![]));
        assert_eq!(c.snapshot_count(), 1);
        c.record(snap(2, vec![]));
        assert_eq!(c.snapshot_count(), 2);
    }

    #[test]
    fn constraint_validates_no_snapshots_matching_input() {
        // No recorded snapshots, matching input → valid
        let c = Constraint::new(7);
        assert!(c.validate(7, &[]));
    }

    #[test]
    fn constraint_rejects_fewer_current_snapshots_than_recorded() {
        let mut c = Constraint::new(1);
        c.record(snap(1, vec![]));
        c.record(snap(2, vec![]));
        // Only provide 1 current snapshot when 2 were recorded
        assert!(!c.validate(1, &[snap(1, vec![])]));
    }

    #[test]
    fn constraint_validates_multiple_matching_snapshots() {
        let h = Hash128::of_str("v");
        let mut c = Constraint::new(5);
        c.record(snap(1, vec![(3, h)]));
        c.record(snap(2, vec![(4, h)]));
        assert!(c.validate(5, &[snap(1, vec![(3, h)]), snap(2, vec![(4, h)])]));
    }

    #[test]
    fn constraint_new_with_hash() {
        let c = Constraint::new(0xdeadbeef);
        assert_eq!(c.input_hash(), 0xdeadbeef);
        assert_eq!(c.snapshot_count(), 0);
    }

    #[test]
    fn constraint_validate_empty_calls_passes() {
        let c = Constraint::new(42);
        assert!(c.validate(42, &[]));
    }

    #[test]
    fn constraint_record_increments_count() {
        let mut c = Constraint::new(1);
        assert_eq!(c.snapshot_count(), 0);
        c.record(snap(1, vec![]));
        assert_eq!(c.snapshot_count(), 1);
        c.record(snap(2, vec![]));
        assert_eq!(c.snapshot_count(), 2);
        c.record(snap(3, vec![]));
        assert_eq!(c.snapshot_count(), 3);
    }

    #[test]
    fn constraint_snapshot_count_matches_recorded() {
        let mut c = Constraint::new(100);
        let h = Hash128::of_str("data");
        c.record(snap(1, vec![(1, h), (2, h)]));
        c.record(snap(2, vec![(3, h)]));
        assert_eq!(c.snapshot_count(), 2);
        // validate with matching snapshots confirms both recorded correctly
        assert!(c.validate(100, &[snap(1, vec![(1, h), (2, h)]), snap(2, vec![(3, h)])]));
    }
}
