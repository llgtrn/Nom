//! CRDT merge strategies — vector clocks, merge operations, and conflict resolution.

use std::collections::HashMap;

/// Per-actor logical clock for happens-before ordering in a distributed system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorClock {
    /// Map from actor id to logical counter.
    pub clocks: HashMap<String, u64>,
}

impl VectorClock {
    /// Create a new empty vector clock.
    pub fn new() -> Self {
        Self {
            clocks: HashMap::new(),
        }
    }

    /// Increment the counter for `actor`.
    pub fn increment(&mut self, actor: impl Into<String>) {
        let entry = self.clocks.entry(actor.into()).or_insert(0);
        *entry += 1;
    }

    /// Return the current counter for `actor` (0 if never incremented).
    pub fn get(&self, actor: &str) -> u64 {
        *self.clocks.get(actor).unwrap_or(&0)
    }

    /// Returns `true` if `self` happened-before `other`.
    ///
    /// Condition: every actor counter in `self` is ≤ the corresponding
    /// counter in `other`, and at least one is strictly less.
    pub fn happens_before(&self, other: &VectorClock) -> bool {
        // Collect the union of all actors seen in either clock.
        let all_actors: std::collections::HashSet<&String> =
            self.clocks.keys().chain(other.clocks.keys()).collect();

        let mut any_strictly_less = false;
        for actor in all_actors {
            let s = self.get(actor.as_str());
            let o = other.get(actor.as_str());
            if s > o {
                return false;
            }
            if s < o {
                any_strictly_less = true;
            }
        }
        any_strictly_less
    }

    /// Merge two vector clocks by taking the component-wise maximum.
    pub fn merge(&self, other: &VectorClock) -> VectorClock {
        let all_actors: std::collections::HashSet<&String> =
            self.clocks.keys().chain(other.clocks.keys()).collect();

        let mut merged = HashMap::new();
        for actor in all_actors {
            let max_val = self.get(actor.as_str()).max(other.get(actor.as_str()));
            merged.insert(actor.clone(), max_val);
        }
        VectorClock { clocks: merged }
    }
}

impl Default for VectorClock {
    fn default() -> Self {
        Self::new()
    }
}

/// A merge operation pairing a local and a remote CRDT state.
///
/// `conflict` is `true` when neither clock happens-before the other,
/// meaning the two states were modified concurrently.
#[derive(Debug, Clone)]
pub struct CrdtMergeOp {
    /// The vector clock from the local replica.
    pub local_clock: VectorClock,
    /// The vector clock from the remote replica.
    pub remote_clock: VectorClock,
    /// Whether the two clocks are concurrent (neither happens-before the other).
    pub conflict: bool,
}

impl CrdtMergeOp {
    /// Create a new merge operation; `conflict` is computed automatically.
    pub fn new(local_clock: VectorClock, remote_clock: VectorClock) -> Self {
        let conflict = !local_clock.happens_before(&remote_clock)
            && !remote_clock.happens_before(&local_clock)
            && local_clock != remote_clock;
        Self {
            local_clock,
            remote_clock,
            conflict,
        }
    }

    /// Returns `true` if this merge has a conflict.
    pub fn is_conflict(&self) -> bool {
        self.conflict
    }
}

/// Strategy for resolving a CRDT merge operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeStrategy {
    /// Accept the most recently written value (remote wins).
    LastWriteWins,
    /// Retain the first written value (local wins).
    FirstWriteWins,
    /// Surface the conflict to a human; no automatic resolution.
    Manual,
}

impl MergeStrategy {
    /// Human-readable name of this strategy.
    pub fn strategy_name(&self) -> &str {
        match self {
            MergeStrategy::LastWriteWins => "last-write-wins",
            MergeStrategy::FirstWriteWins => "first-write-wins",
            MergeStrategy::Manual => "manual",
        }
    }

    /// Resolve a merge operation, returning `"local"`, `"remote"`, or `"conflict"`.
    pub fn resolve<'a>(&self, _op: &CrdtMergeOp) -> &'a str {
        match self {
            MergeStrategy::LastWriteWins => "remote",
            MergeStrategy::FirstWriteWins => "local",
            MergeStrategy::Manual => "conflict",
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod crdt_merge_tests {
    use super::*;

    #[test]
    fn vector_clock_increment_and_get() {
        let mut vc = VectorClock::new();
        assert_eq!(vc.get("alice"), 0);
        vc.increment("alice");
        vc.increment("alice");
        vc.increment("bob");
        assert_eq!(vc.get("alice"), 2);
        assert_eq!(vc.get("bob"), 1);
        assert_eq!(vc.get("carol"), 0);
    }

    #[test]
    fn vector_clock_happens_before_true() {
        // a: {alice: 1} → b: {alice: 2} — a happened before b
        let mut a = VectorClock::new();
        a.increment("alice");

        let mut b = VectorClock::new();
        b.increment("alice");
        b.increment("alice");

        assert!(a.happens_before(&b));
        assert!(!b.happens_before(&a));
    }

    #[test]
    fn vector_clock_happens_before_false_equal() {
        // Equal clocks: neither happens-before the other
        let mut a = VectorClock::new();
        a.increment("alice");

        let mut b = VectorClock::new();
        b.increment("alice");

        assert!(!a.happens_before(&b));
        assert!(!b.happens_before(&a));
    }

    #[test]
    fn vector_clock_merge_takes_max() {
        let mut a = VectorClock::new();
        a.increment("alice");
        a.increment("alice"); // alice: 2

        let mut b = VectorClock::new();
        b.increment("alice"); // alice: 1
        b.increment("bob");   // bob: 1

        let merged = a.merge(&b);
        assert_eq!(merged.get("alice"), 2); // max(2, 1)
        assert_eq!(merged.get("bob"), 1);   // max(0, 1)
    }

    #[test]
    fn crdt_merge_op_no_conflict_when_ordered() {
        let mut local = VectorClock::new();
        local.increment("alice");

        let mut remote = VectorClock::new();
        remote.increment("alice");
        remote.increment("alice");

        // local happened-before remote → no conflict
        let op = CrdtMergeOp::new(local, remote);
        assert!(!op.is_conflict());
    }

    #[test]
    fn crdt_merge_op_conflict_when_concurrent() {
        // alice advanced locally, bob advanced remotely → concurrent
        let mut local = VectorClock::new();
        local.increment("alice");

        let mut remote = VectorClock::new();
        remote.increment("bob");

        let op = CrdtMergeOp::new(local, remote);
        assert!(op.is_conflict());
    }

    #[test]
    fn merge_strategy_last_write_wins_resolve() {
        let op = CrdtMergeOp::new(VectorClock::new(), VectorClock::new());
        let strategy = MergeStrategy::LastWriteWins;
        assert_eq!(strategy.strategy_name(), "last-write-wins");
        assert_eq!(strategy.resolve(&op), "remote");
    }

    #[test]
    fn merge_strategy_first_write_wins_resolve() {
        let op = CrdtMergeOp::new(VectorClock::new(), VectorClock::new());
        let strategy = MergeStrategy::FirstWriteWins;
        assert_eq!(strategy.strategy_name(), "first-write-wins");
        assert_eq!(strategy.resolve(&op), "local");
    }

    #[test]
    fn merge_strategy_manual_resolve() {
        let mut local = VectorClock::new();
        local.increment("alice");
        let mut remote = VectorClock::new();
        remote.increment("bob");

        let op = CrdtMergeOp::new(local, remote);
        let strategy = MergeStrategy::Manual;
        assert_eq!(strategy.strategy_name(), "manual");
        assert_eq!(strategy.resolve(&op), "conflict");
    }
}
