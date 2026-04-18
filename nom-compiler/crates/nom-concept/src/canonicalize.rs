//! Partial→Complete canonicalization lift (§5.10).
//!
//! Defines `CanonicalForm` quality levels, `CanonicalizationChecker` to
//! evaluate whether an entry meets promotion criteria, and `PartialLifter`
//! to perform the actual lift and track how many entries were promoted.

/// The canonicalized quality of a dictionary entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalForm {
    /// Only a hash + word are present. Minimum viable entry.
    Minimal,
    /// Has a body but incomplete metadata (missing fields or low confidence).
    Partial,
    /// All required fields are present and confidence meets threshold.
    Complete,
    /// Complete + round-trip tested (source → IR → source is stable).
    Verified,
}

impl CanonicalForm {
    /// Returns `true` for every level that can still be promoted further.
    /// `Verified` is the terminal state.
    pub fn is_promotable(&self) -> bool {
        !matches!(self, CanonicalForm::Verified)
    }

    /// Advance to the next quality level.
    /// `Verified` stays `Verified` (idempotent terminal).
    pub fn promote(&self) -> CanonicalForm {
        match self {
            CanonicalForm::Minimal => CanonicalForm::Partial,
            CanonicalForm::Partial => CanonicalForm::Complete,
            CanonicalForm::Complete => CanonicalForm::Verified,
            CanonicalForm::Verified => CanonicalForm::Verified,
        }
    }
}

/// Requirements that must be satisfied for an entry to reach `Complete`.
#[derive(Debug, Clone)]
pub struct CanonicalRequirements {
    pub needs_body: bool,
    pub needs_metadata: bool,
    pub needs_kind: bool,
    pub min_confidence: f32,
}

impl CanonicalRequirements {
    /// Strict defaults: everything required, confidence ≥ 0.6.
    pub fn default_requirements() -> Self {
        CanonicalRequirements {
            needs_body: true,
            needs_metadata: true,
            needs_kind: true,
            min_confidence: 0.6,
        }
    }
}

/// Checks the current state of an entry and returns the appropriate
/// `CanonicalForm` quality level.
pub struct CanonicalizationChecker {
    pub requirements: CanonicalRequirements,
}

impl CanonicalizationChecker {
    pub fn new(requirements: CanonicalRequirements) -> Self {
        CanonicalizationChecker { requirements }
    }

    /// Evaluate entry fields and return the quality level they correspond to.
    ///
    /// - `Complete`  — all required checks pass (body, metadata, kind, confidence)
    /// - `Partial`   — has body + kind but metadata is missing/requirements not fully met
    /// - `Minimal`   — no body present
    pub fn check_entry(
        &self,
        has_body: bool,
        has_metadata: bool,
        has_kind: bool,
        confidence: f32,
    ) -> CanonicalForm {
        let body_ok = !self.requirements.needs_body || has_body;
        let meta_ok = !self.requirements.needs_metadata || has_metadata;
        let kind_ok = !self.requirements.needs_kind || has_kind;
        let conf_ok = confidence >= self.requirements.min_confidence;

        if body_ok && meta_ok && kind_ok && conf_ok {
            CanonicalForm::Complete
        } else if has_body && has_kind {
            CanonicalForm::Partial
        } else {
            CanonicalForm::Minimal
        }
    }
}

/// Performs the Partial→Complete lift with validation.
///
/// Only promotes entries that reach `Complete` or `Verified` quality;
/// entries that remain `Partial` or `Minimal` are left untouched.
pub struct PartialLifter {
    checker: CanonicalizationChecker,
    lifted_count: u32,
}

impl PartialLifter {
    pub fn new(checker: CanonicalizationChecker) -> Self {
        PartialLifter {
            checker,
            lifted_count: 0,
        }
    }

    /// Attempt to lift an entry.
    ///
    /// Returns `Some(Complete)` or `Some(Verified)` if the entry qualifies,
    /// and increments the internal counter. Returns `None` for `Partial` or
    /// `Minimal` entries.
    pub fn try_lift(
        &mut self,
        has_body: bool,
        has_metadata: bool,
        has_kind: bool,
        confidence: f32,
    ) -> Option<CanonicalForm> {
        let form = self.checker.check_entry(has_body, has_metadata, has_kind, confidence);
        match form {
            CanonicalForm::Complete | CanonicalForm::Verified => {
                self.lifted_count += 1;
                Some(form)
            }
            _ => None,
        }
    }

    /// Total number of entries successfully lifted so far.
    pub fn lifted_count(&self) -> u32 {
        self.lifted_count
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod canonicalize_tests {
    use super::*;

    fn default_checker() -> CanonicalizationChecker {
        CanonicalizationChecker::new(CanonicalRequirements::default_requirements())
    }

    // ── CanonicalForm::promote ────────────────────────────────────────────────

    #[test]
    fn promote_minimal() {
        assert_eq!(CanonicalForm::Minimal.promote(), CanonicalForm::Partial);
    }

    #[test]
    fn promote_complete_to_verified() {
        assert_eq!(CanonicalForm::Complete.promote(), CanonicalForm::Verified);
    }

    #[test]
    fn verified_is_terminal() {
        let v = CanonicalForm::Verified;
        assert!(!v.is_promotable());
        assert_eq!(v.promote(), CanonicalForm::Verified);
    }

    // ── CanonicalizationChecker ───────────────────────────────────────────────

    #[test]
    fn complete_when_all_met() {
        let checker = default_checker();
        let form = checker.check_entry(true, true, true, 0.8);
        assert_eq!(form, CanonicalForm::Complete);
    }

    #[test]
    fn partial_when_no_metadata() {
        let checker = default_checker();
        // has_body=true, has_metadata=false, has_kind=true — lands Partial
        let form = checker.check_entry(true, false, true, 0.9);
        assert_eq!(form, CanonicalForm::Partial);
    }

    #[test]
    fn minimal_when_no_body() {
        let checker = default_checker();
        let form = checker.check_entry(false, true, true, 0.9);
        assert_eq!(form, CanonicalForm::Minimal);
    }

    // ── PartialLifter ─────────────────────────────────────────────────────────

    #[test]
    fn lifts_complete_entry() {
        let mut lifter = PartialLifter::new(default_checker());
        let result = lifter.try_lift(true, true, true, 0.9);
        assert_eq!(result, Some(CanonicalForm::Complete));
        assert_eq!(lifter.lifted_count(), 1);
    }

    #[test]
    fn skips_partial() {
        let mut lifter = PartialLifter::new(default_checker());
        // Missing metadata → Partial → should NOT be lifted
        let result = lifter.try_lift(true, false, true, 0.9);
        assert_eq!(result, None);
        assert_eq!(lifter.lifted_count(), 0);
    }

    #[test]
    fn count_tracks_lifts() {
        let mut lifter = PartialLifter::new(default_checker());
        lifter.try_lift(true, true, true, 0.9);  // lifted
        lifter.try_lift(true, false, true, 0.9); // skipped (partial)
        lifter.try_lift(true, true, true, 0.7);  // lifted
        assert_eq!(lifter.lifted_count(), 2);
    }
}
