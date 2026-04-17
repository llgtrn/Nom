#![deny(unsafe_code)]
use crate::tracked::TrackedSnapshot;
use crate::hash::Hash128;

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
}
