//! Acceptance-predicate preservation engine — doc 09 M2.
//!
//! Detects when a build drops predicates that a prior build had.  This is
//! the **structural** layer: text-hash comparison + Jaccard rewording
//! heuristic.  Runtime SEMANTIC check (does the predicate still hold under
//! the new body?) is deferred to Phase-8 and is not implemented here.
//!
//! Doc 08 §9.1: "every iteration that swaps a child must re-evaluate ALL of
//! the parent's predicates and refuse swaps that drop or weaken any."

use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

// ── Core types ────────────────────────────────────────────────────────────────

/// One acceptance predicate, tagged with concept + stable text hash for
/// set-comparison.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PredicateBinding {
    pub concept: String,
    pub predicate: String,
    /// Stable hash of normalized predicate text (lowercase, collapsed
    /// whitespace, stripped trailing dot/quotes).  Used for set equality
    /// so minor formatting differences don't count as changes.
    pub text_hash: u64,
}

/// Result of comparing a prior predicate-set to a current predicate-set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PreservationReport {
    /// Present in BOTH (matched by `text_hash`).
    pub preserved: Vec<PredicateBinding>,
    /// Present in prior, missing in current — this is a **violation**.
    pub dropped: Vec<PredicateBinding>,
    /// Present in current, missing in prior — informational, not a violation.
    pub added: Vec<PredicateBinding>,
    /// Same concept, similar text but hash differs (Jaccard above threshold).
    /// Reported as a rewording rather than a hard drop+add so that minor
    /// editorial tweaks are visible without blocking the build.
    pub reworded: Vec<PredicateRewording>,
    /// Stub note explaining the deferred runtime semantic check.
    pub note: String,
}

/// A predicate that was reworded between builds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PredicateRewording {
    pub concept: String,
    pub before: String,
    pub after: String,
    /// Jaccard similarity on whitespace-token sets; 0.0–1.0.
    pub similarity: f64,
}

// PartialEq blanket impl requires a special approach for f64.
// We derive via the struct fields but f64 is already PartialEq.
impl Eq for PredicateRewording {}

// ── Normalization + hashing ───────────────────────────────────────────────────

/// Normalize a predicate for comparison: lowercase, collapse whitespace,
/// strip surrounding quotes and trailing period.
pub fn normalize_predicate(s: &str) -> String {
    let lowered = s.to_lowercase();
    let trimmed: String = lowered.split_whitespace().collect::<Vec<_>>().join(" ");
    trimmed.trim_end_matches('.').trim_matches('"').to_string()
}

/// Compute a stable hash of the normalized predicate text.
///
/// Stability guarantee: two strings that differ only in case or whitespace
/// produce the same hash.
pub fn predicate_text_hash(s: &str) -> u64 {
    let mut h = DefaultHasher::new();
    normalize_predicate(s).hash(&mut h);
    h.finish()
}

// ── Jaccard similarity ────────────────────────────────────────────────────────

/// Jaccard similarity on whitespace-token sets.
///
/// Returns a value in `[0.0, 1.0]`.  Empty-vs-empty returns 1.0.
pub fn jaccard_similarity(a: &str, b: &str) -> f64 {
    let set_a: HashSet<&str> = a.split_whitespace().collect();
    let set_b: HashSet<&str> = b.split_whitespace().collect();

    if set_a.is_empty() && set_b.is_empty() {
        return 1.0;
    }

    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();

    if union == 0 {
        1.0
    } else {
        intersection as f64 / union as f64
    }
}

// ── Building bindings ────────────────────────────────────────────────────────

/// Build the predicate-binding list from a parsed concept's acceptance vec.
pub fn bindings_for_concept(concept_name: &str, acceptance: &[String]) -> Vec<PredicateBinding> {
    acceptance
        .iter()
        .map(|pred| PredicateBinding {
            concept: concept_name.to_owned(),
            predicate: pred.clone(),
            text_hash: predicate_text_hash(pred),
        })
        .collect()
}

// ── Preservation check ────────────────────────────────────────────────────────

/// Compare prior vs current; return a `PreservationReport`.
///
/// `reword_threshold` — Jaccard-similarity cutoff above which a
/// dropped+added pair in the same concept is reported as a rewording
/// instead of a hard drop+add.  Recommend `0.5`.
pub fn check_preservation(
    prior: &[PredicateBinding],
    current: &[PredicateBinding],
    reword_threshold: f64,
) -> PreservationReport {
    // Index by text_hash for O(1) lookup.
    let prior_set: HashSet<u64> = prior.iter().map(|b| b.text_hash).collect();
    let current_set: HashSet<u64> = current.iter().map(|b| b.text_hash).collect();

    let preserved: Vec<PredicateBinding> = prior
        .iter()
        .filter(|b| current_set.contains(&b.text_hash))
        .cloned()
        .collect();

    let raw_dropped: Vec<PredicateBinding> = prior
        .iter()
        .filter(|b| !current_set.contains(&b.text_hash))
        .cloned()
        .collect();

    let raw_added: Vec<PredicateBinding> = current
        .iter()
        .filter(|b| !prior_set.contains(&b.text_hash))
        .cloned()
        .collect();

    // Rewording detection: for each raw_dropped predicate, look for a
    // raw_added predicate from the SAME concept with Jaccard > threshold.
    let mut dropped: Vec<PredicateBinding> = Vec::new();
    let mut added: Vec<PredicateBinding> = Vec::new();
    let mut reworded: Vec<PredicateRewording> = Vec::new();

    // Track which added indices were consumed by a rewording match.
    let mut added_consumed: Vec<bool> = vec![false; raw_added.len()];

    for drop_binding in &raw_dropped {
        let mut best_match: Option<(usize, f64)> = None;

        for (ai, add_binding) in raw_added.iter().enumerate() {
            if added_consumed[ai] {
                continue;
            }
            if add_binding.concept != drop_binding.concept {
                continue;
            }
            let sim = jaccard_similarity(
                &normalize_predicate(&drop_binding.predicate),
                &normalize_predicate(&add_binding.predicate),
            );
            if sim > reword_threshold {
                match best_match {
                    None => best_match = Some((ai, sim)),
                    Some((_, best_sim)) if sim > best_sim => best_match = Some((ai, sim)),
                    _ => {}
                }
            }
        }

        if let Some((ai, sim)) = best_match {
            added_consumed[ai] = true;
            reworded.push(PredicateRewording {
                concept: drop_binding.concept.clone(),
                before: drop_binding.predicate.clone(),
                after: raw_added[ai].predicate.clone(),
                similarity: sim,
            });
        } else {
            dropped.push(drop_binding.clone());
        }
    }

    // Remaining non-consumed added entries.
    for (ai, add_binding) in raw_added.iter().enumerate() {
        if !added_consumed[ai] {
            added.push(add_binding.clone());
        }
    }

    PreservationReport {
        preserved,
        dropped,
        added,
        reworded,
        note: "runtime semantic check deferred to Phase-8".to_string(),
    }
}

/// Any drop is a violation (M2 structural).
///
/// Future Phase-8 adds semantic check.
/// Rewordings and additions are informational only.
pub fn has_violations(report: &PreservationReport) -> bool {
    !report.dropped.is_empty()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // 1. normalize_predicate lowercases and collapses whitespace
    #[test]
    fn normalize_predicate_lowercases_and_collapses_whitespace() {
        assert_eq!(
            normalize_predicate("  This Works When X  "),
            "this works when x"
        );
    }

    // 2. normalize_predicate strips trailing dot
    #[test]
    fn normalize_predicate_strips_trailing_dot() {
        assert_eq!(normalize_predicate("x."), "x");
    }

    // 3. hash stable across formatting differences
    #[test]
    fn hash_stable_across_formatting() {
        let h1 = predicate_text_hash("users reach dashboard within 200 ms");
        let h2 = predicate_text_hash("  Users Reach Dashboard  Within  200 ms  ");
        assert_eq!(
            h1, h2,
            "hashes must match despite whitespace/case differences"
        );
    }

    // 4. clean when sets match
    #[test]
    fn check_preservation_clean_when_sets_match() {
        let concept = "my_concept";
        let predicates = vec![
            "this works when a is done.".to_string(),
            "this works when b is done.".to_string(),
        ];
        let bindings = bindings_for_concept(concept, &predicates);
        let report = check_preservation(&bindings, &bindings, 0.5);
        assert_eq!(report.dropped.len(), 0);
        assert_eq!(report.preserved.len(), 2);
        assert!(!has_violations(&report));
    }

    // 5. detects dropped predicates
    #[test]
    fn check_preservation_detects_dropped() {
        let concept = "my_concept";
        let prior_preds = vec![
            "this works when a.".to_string(),
            "this works when b.".to_string(),
            "this works when c.".to_string(),
        ];
        let current_preds = vec![
            "this works when a.".to_string(),
            "this works when b.".to_string(),
        ];
        let prior = bindings_for_concept(concept, &prior_preds);
        let current = bindings_for_concept(concept, &current_preds);
        let report = check_preservation(&prior, &current, 0.5);
        assert_eq!(report.dropped.len(), 1, "one predicate was dropped");
        assert!(has_violations(&report));
    }

    // 6. added predicates are informational, not violations
    #[test]
    fn check_preservation_reports_added_as_informational() {
        let concept = "my_concept";
        let prior_preds = vec![
            "this works when a.".to_string(),
            "this works when b.".to_string(),
        ];
        let current_preds = vec![
            "this works when a.".to_string(),
            "this works when b.".to_string(),
            "this works when c.".to_string(),
        ];
        let prior = bindings_for_concept(concept, &prior_preds);
        let current = bindings_for_concept(concept, &current_preds);
        let report = check_preservation(&prior, &current, 0.5);
        assert_eq!(report.added.len(), 1);
        assert_eq!(report.dropped.len(), 0);
        assert!(!has_violations(&report));
    }

    // 7. rewording detection: close enough text → rewording, not drop+add
    //
    // "users reach dashboard within 200 ms" vs
    // "users reach the dashboard within 200 ms"
    // Jaccard: intersection = {users,reach,dashboard,within,200,ms} = 6
    //          union        = {users,reach,the,dashboard,within,200,ms} = 7
    //          similarity   = 6/7 ≈ 0.857 — well above 0.5.
    #[test]
    fn check_preservation_detects_rewording() {
        let concept = "my_concept";
        let prior_preds = vec!["users reach dashboard within 200 ms.".to_string()];
        let current_preds = vec!["users reach the dashboard within 200 ms.".to_string()];
        let prior = bindings_for_concept(concept, &prior_preds);
        let current = bindings_for_concept(concept, &current_preds);
        let report = check_preservation(&prior, &current, 0.5);
        // Should be a rewording, not a drop.
        assert_eq!(
            report.reworded.len(),
            1,
            "expected one rewording: {:?}",
            report
        );
        assert_eq!(
            report.dropped.len(),
            0,
            "drop list must be empty for a rewording"
        );
        assert_eq!(
            report.added.len(),
            0,
            "add list must be empty for a rewording"
        );
        assert!(!has_violations(&report));
        assert!(
            report.reworded[0].similarity > 0.5,
            "similarity must exceed threshold: {}",
            report.reworded[0].similarity
        );
    }

    // 8. multiple concepts: one unchanged, one with a dropped predicate
    #[test]
    fn check_preservation_multiple_concepts() {
        let prior: Vec<PredicateBinding> = vec![
            PredicateBinding {
                concept: "concept_a".to_string(),
                predicate: "this works when a is ready.".to_string(),
                text_hash: predicate_text_hash("this works when a is ready."),
            },
            PredicateBinding {
                concept: "concept_b".to_string(),
                predicate: "this works when b is ready.".to_string(),
                text_hash: predicate_text_hash("this works when b is ready."),
            },
            PredicateBinding {
                concept: "concept_b".to_string(),
                predicate: "this works when b is validated.".to_string(),
                text_hash: predicate_text_hash("this works when b is validated."),
            },
        ];

        // Drop one predicate from concept_b; keep concept_a intact.
        let current: Vec<PredicateBinding> = vec![
            PredicateBinding {
                concept: "concept_a".to_string(),
                predicate: "this works when a is ready.".to_string(),
                text_hash: predicate_text_hash("this works when a is ready."),
            },
            PredicateBinding {
                concept: "concept_b".to_string(),
                predicate: "this works when b is ready.".to_string(),
                text_hash: predicate_text_hash("this works when b is ready."),
            },
        ];

        let report = check_preservation(&prior, &current, 0.5);

        assert_eq!(report.dropped.len(), 1);
        assert_eq!(report.dropped[0].concept, "concept_b");
        assert!(has_violations(&report));

        // Preserved should include both unchanged predicates.
        assert_eq!(report.preserved.len(), 2);
    }
}
