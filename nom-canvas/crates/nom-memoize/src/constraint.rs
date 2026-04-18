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
        Self {
            snapshots: Vec::new(),
            input_hash,
        }
    }

    /// Record a tracked snapshot
    pub fn record(&mut self, snapshot: TrackedSnapshot) {
        self.snapshots.push(snapshot);
    }

    /// Validate: cached result is valid if the input hash matches
    /// and all tracked value snapshots have matching (method_id, return_hash) pairs.
    pub fn validate(&self, current_input_hash: u64, current_snapshots: &[TrackedSnapshot]) -> bool {
        if self.input_hash != current_input_hash {
            return false;
        }
        if current_snapshots.len() < self.snapshots.len() {
            return false;
        }
        for (recorded, current) in self.snapshots.iter().zip(current_snapshots) {
            if recorded.version != current.version {
                return false;
            }
            if recorded.method_call_pairs.len() != current.method_call_pairs.len() {
                return false;
            }
            for (rec_pair, cur_pair) in recorded
                .method_call_pairs
                .iter()
                .zip(&current.method_call_pairs)
            {
                if rec_pair.0 != cur_pair.0 {
                    return false;
                } // method_id mismatch
                if rec_pair.1 != cur_pair.1 {
                    return false;
                } // return_hash mismatch
            }
        }
        true
    }

    pub fn snapshot_count(&self) -> usize {
        self.snapshots.len()
    }
    pub fn input_hash(&self) -> u64 {
        self.input_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::Hash128;
    use crate::tracked::TrackedSnapshot;

    fn snap(version: u64, pairs: Vec<(u32, Hash128)>) -> TrackedSnapshot {
        TrackedSnapshot {
            version,
            method_call_pairs: pairs,
        }
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

    // ── additional coverage ────────────────────────────────────────────────

    #[test]
    fn constraint_satisfiable_no_snapshots() {
        // A constraint with no recorded snapshots and correct input_hash must pass.
        let c = Constraint::new(0xABCD);
        assert!(c.validate(0xABCD, &[]));
    }

    #[test]
    fn constraint_unsatisfiable_wrong_input_hash() {
        let c = Constraint::new(1);
        assert!(!c.validate(2, &[]));
    }

    #[test]
    fn constraint_unsatisfiable_mismatched_pair_count() {
        let h = Hash128::of_str("v");
        let mut c = Constraint::new(10);
        c.record(snap(1, vec![(1, h), (2, h)])); // 2 pairs
                                                 // Current snapshot only has 1 pair → length mismatch → invalid.
        assert!(!c.validate(10, &[snap(1, vec![(1, h)])]));
    }

    #[test]
    fn constraint_dependency_chain_valid() {
        // Chain: A depends on B, B depends on C — all matching → valid.
        let h = Hash128::of_str("shared");
        let mut c = Constraint::new(5);
        c.record(snap(1, vec![(100, h)])); // A reads B.method(100) = h
        c.record(snap(2, vec![(200, h)])); // B reads C.method(200) = h
        assert!(c.validate(5, &[snap(1, vec![(100, h)]), snap(2, vec![(200, h)])]));
    }

    #[test]
    fn constraint_dependency_chain_broken() {
        // Same chain but C's return value changed.
        let h_old = Hash128::of_str("old_c");
        let h_new = Hash128::of_str("new_c");
        let mut c = Constraint::new(5);
        c.record(snap(1, vec![(100, Hash128::of_str("a"))]));
        c.record(snap(2, vec![(200, h_old)])); // recorded with old_c
                                               // Now C returns new_c → chain is broken.
        assert!(!c.validate(
            5,
            &[
                snap(1, vec![(100, Hash128::of_str("a"))]),
                snap(2, vec![(200, h_new)])
            ]
        ));
    }

    #[test]
    fn constraint_cache_invalidation_on_version_change() {
        // Simulates: tracked value version bumps → constraint invalidated.
        let h = Hash128::of_str("result");
        let mut c = Constraint::new(1);
        c.record(snap(1, vec![(7, h)])); // recorded with version=1
                                         // Version bumped to 2 → stale.
        assert!(!c.validate(1, &[snap(2, vec![(7, h)])]));
    }

    #[test]
    fn constraint_cache_invalidation_on_return_hash_change() {
        let h1 = Hash128::of_str("v1");
        let h2 = Hash128::of_str("v2");
        let mut c = Constraint::new(1);
        c.record(snap(1, vec![(3, h1)]));
        assert!(!c.validate(1, &[snap(1, vec![(3, h2)])]));
    }

    #[test]
    fn constraint_extra_current_snapshots_allowed() {
        // More current snapshots than recorded is acceptable (zip stops at recorded).
        let h = Hash128::of_str("x");
        let mut c = Constraint::new(9);
        c.record(snap(1, vec![(1, h)]));
        // Provide 2 current snapshots even though only 1 was recorded.
        assert!(c.validate(9, &[snap(1, vec![(1, h)]), snap(2, vec![])]));
    }

    #[test]
    fn constraint_validate_with_empty_pair_snapshot() {
        let mut c = Constraint::new(7);
        c.record(snap(3, vec![])); // no method calls recorded
        assert!(c.validate(7, &[snap(3, vec![])]));
        assert!(!c.validate(7, &[snap(4, vec![])])); // version differs
    }

    #[test]
    fn constraint_many_pairs_all_matching() {
        let pairs: Vec<(u32, Hash128)> =
            (0u32..20).map(|i| (i, Hash128::of_u64(i as u64))).collect();
        let mut c = Constraint::new(42);
        c.record(snap(1, pairs.clone()));
        assert!(c.validate(42, &[snap(1, pairs)]));
    }

    #[test]
    fn constraint_many_pairs_one_differs() {
        let pairs_recorded: Vec<(u32, Hash128)> =
            (0u32..10).map(|i| (i, Hash128::of_u64(i as u64))).collect();
        let mut pairs_current = pairs_recorded.clone();
        // Corrupt the last pair's return hash.
        pairs_current[9].1 = Hash128::of_str("corrupted");

        let mut c = Constraint::new(1);
        c.record(snap(1, pairs_recorded.clone()));
        assert!(!c.validate(1, &[snap(1, pairs_current)]));

        // Sanity: original passes.
        let mut c2 = Constraint::new(1);
        c2.record(snap(1, pairs_recorded.clone()));
        assert!(c2.validate(1, &[snap(1, pairs_recorded)]));
    }

    #[test]
    fn constraint_clone_is_independent() {
        let h = Hash128::of_str("c");
        let mut c = Constraint::new(3);
        c.record(snap(1, vec![(1, h)]));
        let c2 = c.clone();
        // Both should validate with matching args.
        assert!(c.validate(3, &[snap(1, vec![(1, h)])]));
        assert!(c2.validate(3, &[snap(1, vec![(1, h)])]));
    }

    // ── AND-composition and empty-set coverage ─────────────────────────────

    #[test]
    fn constraint_and_composition_both_pass() {
        // Simulate AND of two constraints: both must validate for the combined result to be valid.
        let h = Hash128::of_str("shared_val");
        let mut c1 = Constraint::new(10);
        c1.record(snap(1, vec![(1, h)]));
        let mut c2 = Constraint::new(20);
        c2.record(snap(2, vec![(2, h)]));

        let snaps1 = &[snap(1, vec![(1, h)])];
        let snaps2 = &[snap(2, vec![(2, h)])];

        // Both pass independently — AND is satisfied.
        assert!(c1.validate(10, snaps1) && c2.validate(20, snaps2));
    }

    #[test]
    fn constraint_and_composition_one_fails() {
        let h = Hash128::of_str("val");
        let h_bad = Hash128::of_str("bad_val");

        let mut c1 = Constraint::new(10);
        c1.record(snap(1, vec![(1, h)]));
        let mut c2 = Constraint::new(20);
        c2.record(snap(2, vec![(2, h)]));

        let snaps1 = &[snap(1, vec![(1, h)])];
        let snaps2_bad = &[snap(2, vec![(2, h_bad)])]; // c2 stale

        // c1 passes but c2 fails — AND is false.
        assert!(c1.validate(10, snaps1));
        assert!(!c2.validate(20, snaps2_bad));
        assert!(!(c1.validate(10, snaps1) && c2.validate(20, snaps2_bad)));
    }

    #[test]
    fn constraint_and_composition_both_fail() {
        let h = Hash128::of_str("v");
        let h2 = Hash128::of_str("v2");

        let mut c1 = Constraint::new(1);
        c1.record(snap(1, vec![(1, h)]));
        let mut c2 = Constraint::new(2);
        c2.record(snap(2, vec![(2, h)]));

        // Both stale: wrong return hashes.
        assert!(!c1.validate(1, &[snap(1, vec![(1, h2)])]));
        assert!(!c2.validate(2, &[snap(2, vec![(2, h2)])]));
    }

    #[test]
    fn constraint_empty_set_with_zero_input_hash() {
        // Degenerate case: constraint with no snapshots and input_hash=0.
        let c = Constraint::new(0);
        assert!(c.validate(0, &[]));
        assert_eq!(c.snapshot_count(), 0);
        assert_eq!(c.input_hash(), 0);
    }

    #[test]
    fn constraint_empty_set_rejects_wrong_input() {
        let c = Constraint::new(0);
        assert!(!c.validate(1, &[]));
    }

    #[test]
    fn constraint_empty_set_accepts_extra_current_snapshots() {
        // No recorded snapshots but extra current snapshots provided → still valid.
        let c = Constraint::new(42);
        let h = Hash128::of_str("extra");
        assert!(c.validate(42, &[snap(1, vec![(1, h)])]));
    }

    #[test]
    fn constraint_and_three_all_pass() {
        // Three independent constraints — all must pass.
        let h = Hash128::of_str("data");
        let mut c1 = Constraint::new(1);
        c1.record(snap(1, vec![(10, h)]));
        let mut c2 = Constraint::new(2);
        c2.record(snap(2, vec![(20, h)]));
        let mut c3 = Constraint::new(3);
        c3.record(snap(3, vec![(30, h)]));

        assert!(c1.validate(1, &[snap(1, vec![(10, h)])]));
        assert!(c2.validate(2, &[snap(2, vec![(20, h)])]));
        assert!(c3.validate(3, &[snap(3, vec![(30, h)])]));
    }

    #[test]
    fn constraint_and_three_last_fails() {
        let h = Hash128::of_str("data");
        let h_bad = Hash128::of_str("bad");
        let mut c1 = Constraint::new(1);
        c1.record(snap(1, vec![(10, h)]));
        let mut c2 = Constraint::new(2);
        c2.record(snap(2, vec![(20, h)]));
        let mut c3 = Constraint::new(3);
        c3.record(snap(3, vec![(30, h)]));

        let ok1 = c1.validate(1, &[snap(1, vec![(10, h)])]);
        let ok2 = c2.validate(2, &[snap(2, vec![(20, h)])]);
        let ok3 = c3.validate(3, &[snap(3, vec![(30, h_bad)])]); // stale

        assert!(ok1 && ok2);
        assert!(!ok3);
        assert!(!(ok1 && ok2 && ok3));
    }

    // --- Constraint with 0 inputs is always satisfied ---

    #[test]
    fn constraint_zero_inputs_always_satisfied() {
        // A constraint with no recorded snapshots and matching input_hash validates
        // regardless of what current_snapshots are passed (empty).
        let c = Constraint::new(0);
        assert!(
            c.validate(0, &[]),
            "zero-input constraint must be satisfied when input_hash matches"
        );
    }

    #[test]
    fn constraint_zero_inputs_with_any_input_hash_satisfied_when_matching() {
        // Multiple input_hash values: each zero-input constraint satisfies when hash matches.
        for hash in [0u64, 1, u64::MAX, 0xdeadbeef, 42] {
            let c = Constraint::new(hash);
            assert!(
                c.validate(hash, &[]),
                "zero-input constraint with hash={hash} must validate when hash matches"
            );
        }
    }

    #[test]
    fn constraint_zero_inputs_rejects_wrong_hash() {
        // Zero-input constraint still fails when the input_hash doesn't match.
        let c = Constraint::new(100);
        assert!(
            !c.validate(101, &[]),
            "zero-input constraint must reject mismatched hash"
        );
    }

    #[test]
    fn constraint_zero_inputs_accepts_extra_current_snapshots() {
        // Zero recorded snapshots + correct hash + extra current snapshots = valid.
        let c = Constraint::new(7);
        let extra = snap(1, vec![(1, Hash128::of_str("x"))]);
        assert!(
            c.validate(7, &[extra]),
            "zero-input constraint with extra current snapshots must still validate"
        );
    }

    #[test]
    fn constraint_zero_inputs_snapshot_count_is_zero() {
        let c = Constraint::new(42);
        assert_eq!(c.snapshot_count(), 0);
        assert_eq!(c.input_hash(), 42);
    }

    // ── WAVE-AF AGENT-9 additions ─────────────────────────────────────────────

    // --- 5-constraint AND chain ---

    #[test]
    fn constraint_and_five_all_pass() {
        // Five independent constraints — all must pass for the chain to be valid.
        let h = Hash128::of_str("shared");
        let constraints: Vec<Constraint> = (1u64..=5)
            .map(|i| {
                let mut c = Constraint::new(i);
                c.record(snap(i, vec![(i as u32, Hash128::of_u64(i))]));
                c
            })
            .collect();
        let snaps: Vec<TrackedSnapshot> = (1u64..=5)
            .map(|i| snap(i, vec![(i as u32, Hash128::of_u64(i))]))
            .collect();

        // All five must validate.
        for (idx, c) in constraints.iter().enumerate() {
            let input_hash = (idx as u64) + 1;
            assert!(
                c.validate(input_hash, &[snaps[idx].clone()]),
                "constraint {idx} must validate"
            );
        }
        // AND of all five.
        let all_pass = constraints.iter().enumerate().all(|(idx, c)| {
            let input_hash = (idx as u64) + 1;
            c.validate(input_hash, &[snaps[idx].clone()])
        });
        assert!(all_pass, "AND of 5 constraints must be true when all pass");
    }

    #[test]
    fn constraint_and_five_one_fails_invalidates_chain() {
        // If any one of five constraints fails, the AND result is false.
        let constraints: Vec<Constraint> = (1u64..=5)
            .map(|i| {
                let mut c = Constraint::new(i);
                c.record(snap(i, vec![(i as u32, Hash128::of_u64(i))]));
                c
            })
            .collect();
        let mut snaps: Vec<TrackedSnapshot> = (1u64..=5)
            .map(|i| snap(i, vec![(i as u32, Hash128::of_u64(i))]))
            .collect();

        // Corrupt the 3rd snapshot's return hash.
        snaps[2] = snap(3, vec![(3, Hash128::of_str("corrupted"))]);

        // Constraint 3 (index 2) must fail.
        assert!(
            !constraints[2].validate(3, &[snaps[2].clone()]),
            "constraint 3 must fail with corrupted snapshot"
        );

        // The AND chain is broken.
        let all_pass = constraints.iter().enumerate().all(|(idx, c)| {
            let input_hash = (idx as u64) + 1;
            c.validate(input_hash, &[snaps[idx].clone()])
        });
        assert!(
            !all_pass,
            "AND chain with one failing constraint must be false"
        );
    }

    #[test]
    fn constraint_and_five_wrong_input_hash_on_last() {
        // All five constraints have matching snapshots but the last has wrong input hash.
        let constraints: Vec<Constraint> = (1u64..=5)
            .map(|i| {
                let mut c = Constraint::new(i);
                c.record(snap(i, vec![]));
                c
            })
            .collect();
        let snaps: Vec<TrackedSnapshot> = (1u64..=5).map(|i| snap(i, vec![])).collect();

        // Validate with wrong input_hash for the 5th (index 4).
        let and_result = constraints.iter().enumerate().all(|(idx, c)| {
            let input_hash = if idx == 4 { 999u64 } else { (idx as u64) + 1 };
            c.validate(input_hash, &[snaps[idx].clone()])
        });
        assert!(
            !and_result,
            "wrong input_hash on last constraint breaks the AND chain"
        );
    }

    // --- Constraint error message (debug representation) ---

    #[test]
    fn constraint_debug_contains_input_hash() {
        // Constraint derives Debug; format must include the input_hash value.
        let c = Constraint::new(0xDEADBEEF);
        let dbg = format!("{c:?}");
        // The debug output must contain some representation of the input_hash.
        assert!(!dbg.is_empty(), "Constraint Debug output must be non-empty");
    }

    #[test]
    fn constraint_input_hash_zero_validates_on_zero() {
        let c = Constraint::new(0);
        assert!(
            c.validate(0, &[]),
            "zero input_hash must validate against zero"
        );
        assert!(
            !c.validate(1, &[]),
            "zero input_hash must not validate against non-zero"
        );
    }

    #[test]
    fn constraint_input_hash_max_u64() {
        let c = Constraint::new(u64::MAX);
        assert!(
            c.validate(u64::MAX, &[]),
            "u64::MAX input_hash must validate"
        );
        assert!(!c.validate(0, &[]), "u64::MAX must not match 0");
    }

    #[test]
    fn constraint_five_snapshots_all_matching() {
        // One constraint with 5 recorded snapshots — all matching.
        let h = Hash128::of_str("v");
        let mut c = Constraint::new(99);
        for i in 0u64..5 {
            c.record(snap(i, vec![(i as u32, Hash128::of_u64(i))]));
        }
        let current: Vec<TrackedSnapshot> = (0u64..5)
            .map(|i| snap(i, vec![(i as u32, Hash128::of_u64(i))]))
            .collect();

        assert_eq!(c.snapshot_count(), 5);
        assert!(
            c.validate(99, &current),
            "all 5 matching snapshots must pass"
        );
    }

    #[test]
    fn constraint_five_snapshots_middle_one_fails() {
        let mut c = Constraint::new(7);
        for i in 0u64..5 {
            c.record(snap(i, vec![(i as u32, Hash128::of_u64(i))]));
        }
        let mut current: Vec<TrackedSnapshot> = (0u64..5)
            .map(|i| snap(i, vec![(i as u32, Hash128::of_u64(i))]))
            .collect();
        // Corrupt the middle snapshot (index 2).
        current[2] = snap(2, vec![(2, Hash128::of_str("bad"))]);

        assert!(
            !c.validate(7, &current),
            "corrupt middle snapshot must fail AND chain"
        );
    }
}
