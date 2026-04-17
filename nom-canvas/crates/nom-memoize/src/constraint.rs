#![deny(unsafe_code)]
use crate::tracked::TrackedSnapshot;

/// Captures what a memoized function read and validates that those reads
/// are still valid before returning a cached result (typst comemo pattern)
#[derive(Clone, Debug)]
pub struct Constraint {
    /// All snapshots taken from Tracked<T> during computation
    snapshots: Vec<TrackedSnapshot>,
    /// Hash of all inputs at computation time
    input_hash: u64,
}

impl Constraint {
    /// Create a new constraint for tracking
    pub fn new(input_hash: u64) -> Self {
        Self { snapshots: Vec::new(), input_hash }
    }

    /// Record a tracked read
    pub fn record(&mut self, snapshot: TrackedSnapshot) {
        self.snapshots.push(snapshot);
    }

    /// Validate: cached result is valid if the input hash matches
    /// and all tracked values have the same version
    pub fn validate(&self, current_input_hash: u64, current_versions: &[u64]) -> bool {
        if self.input_hash != current_input_hash { return false; }
        if current_versions.len() < self.snapshots.len() { return false; }
        for (snap, &current_version) in self.snapshots.iter().zip(current_versions) {
            if snap.version != current_version { return false; }
        }
        true
    }

    pub fn snapshot_count(&self) -> usize { self.snapshots.len() }
    pub fn input_hash(&self) -> u64 { self.input_hash }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracked::TrackedSnapshot;

    #[test]
    fn constraint_validates_matching() {
        let mut c = Constraint::new(12345);
        c.record(TrackedSnapshot { version: 1, access_count_at_snapshot: 0 });
        assert!(c.validate(12345, &[1]));
    }

    #[test]
    fn constraint_rejects_stale_version() {
        let mut c = Constraint::new(12345);
        c.record(TrackedSnapshot { version: 1, access_count_at_snapshot: 0 });
        // version changed from 1 to 2 → cache miss
        assert!(!c.validate(12345, &[2]));
    }

    #[test]
    fn constraint_rejects_changed_input() {
        let c = Constraint::new(12345);
        assert!(!c.validate(99999, &[]));
    }
}
