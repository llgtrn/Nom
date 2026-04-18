//! MECE (Mutually-Exclusive, Collectively-Exhaustive) objectives validator.
//!
//! Per `research/language-analysis/08-layered-concept-component-architecture.md` §9.2.
//!
//! The ME check verifies that when a parent concept composes child concepts, no
//! two objectives in the union map to the same quality axis. The stub axis mapping
//! is `name.to_ascii_lowercase()`; Phase 9 corpus will replace this with a
//! synonym-aware registry.
//!
//! The CE check is deferred to Phase 9 (requires the corpus required-axis registry).

use std::collections::HashMap;

use crate::ConceptDecl;

/// One objective with its resolved axis.
///
/// Phase 9 will populate `axis` from the corpus synonym registry; the stub
/// simply lowercases the name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectiveBinding {
    /// The concept that declared this objective.
    pub source_concept: String,
    /// The objective name as written in the source.
    pub name: String,
    /// The resolved axis (stub: `name.to_ascii_lowercase()`).
    pub axis: String,
}

/// The full MECE validation report for one parent–children composition.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MeceReport {
    /// Union of all objectives across parent + children, one binding per occurrence.
    pub union: Vec<ObjectiveBinding>,
    /// Every axis that appears more than once in `union` (ME violations).
    pub me_collisions: Vec<MeCollision>,
    /// Axes required by doc 08 §9.2 but absent from the union.
    /// Empty in the stub; Phase 9 fills this from the corpus required-axis registry.
    pub ce_unmet: Vec<String>,
    /// Free-text caveats about what the stub does not yet check.
    pub stub_notes: Vec<String>,
}

/// One ME violation: a single axis claimed by two or more objectives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeCollision {
    /// The colliding axis label.
    pub axis: String,
    /// All bindings in `union` that map to this axis (≥ 2).
    pub bindings: Vec<ObjectiveBinding>,
}

/// Stub axis mapping: lowercase the name.
///
/// Phase 9 corpus replaces this with a synonym-aware mapping so that e.g.
/// `"latency"` and `"speed"` collapse to one axis.
pub fn stub_axis_of(name: &str) -> String {
    name.to_ascii_lowercase()
}

/// Check MECE for one concept composition.
///
/// `parent` is the composing concept; `children` are the concepts it pulls in
/// via `IndexClause::Uses` whose kind is `"concept"` (the caller — nom-cli —
/// derives this list from the closure walker output).
///
/// The returned [`MeceReport`] contains:
/// - `union`: every objective from parent + all children, one binding each.
/// - `me_collisions`: any axis that appears more than once (ME violation).
/// - `ce_unmet`: always empty in the stub (CE check deferred to Phase 9).
/// - `stub_notes`: exactly one note describing the deferred CE check.
pub fn check_mece(parent: &ConceptDecl, children: &[&ConceptDecl]) -> MeceReport {
    // ── 1. Build the union ────────────────────────────────────────────────────
    let mut union: Vec<ObjectiveBinding> = Vec::new();

    for name in &parent.objectives {
        union.push(ObjectiveBinding {
            source_concept: parent.name.clone(),
            name: name.clone(),
            axis: stub_axis_of(name),
        });
    }

    for child in children {
        for name in &child.objectives {
            union.push(ObjectiveBinding {
                source_concept: child.name.clone(),
                name: name.clone(),
                axis: stub_axis_of(name),
            });
        }
    }

    // ── 2. Group by axis → detect ME collisions ───────────────────────────────
    let mut axis_map: HashMap<String, Vec<ObjectiveBinding>> = HashMap::new();
    for binding in &union {
        axis_map
            .entry(binding.axis.clone())
            .or_default()
            .push(binding.clone());
    }

    let mut me_collisions: Vec<MeCollision> = axis_map
        .into_iter()
        .filter(|(_, bindings)| bindings.len() > 1)
        .map(|(axis, bindings)| MeCollision { axis, bindings })
        .collect();

    // Sort for deterministic output (axis names are unique keys).
    me_collisions.sort_by(|a, b| a.axis.cmp(&b.axis));

    // ── 3. CE check deferred ─────────────────────────────────────────────────
    let ce_unmet: Vec<String> = vec![];

    // ── 4. Stub notes ─────────────────────────────────────────────────────────
    let stub_notes = vec![
        "CE check deferred: Phase-9 corpus must register required-axis set per composition layer (doc 08 §9.2).".to_string(),
    ];

    MeceReport {
        union,
        me_collisions,
        ce_unmet,
        stub_notes,
    }
}

/// Same as [`check_mece`] but with a required-axes registry consulted for the
/// CE (Collectively-Exhaustive) check.
///
/// Each `required_axes` entry is a tuple of `(axis, cardinality)` where
/// cardinality is one of `"at_least_one"` | `"exactly_one"`.
///
/// - `at_least_one`: the union must contain AT LEAST one objective whose
///   resolved axis matches. Absence → `ce_unmet` entry.
/// - `exactly_one`: the union must contain EXACTLY one such objective.
///   Absence → `ce_unmet` entry. Duplicates → `ce_unmet` entry AND the
///   existing ME collision logic also fires.
///
/// When `required_axes` is empty the function returns `ce_unmet = []` and
/// **no** stub note (the registry is live; there are simply no requirements).
pub fn check_mece_with_required_axes(
    parent: &ConceptDecl,
    children: &[&ConceptDecl],
    required_axes: &[(String, String)],
) -> MeceReport {
    // ── 1. Build union + ME collisions (same logic as check_mece) ─────────
    let mut union: Vec<ObjectiveBinding> = Vec::new();

    for name in &parent.objectives {
        union.push(ObjectiveBinding {
            source_concept: parent.name.clone(),
            name: name.clone(),
            axis: stub_axis_of(name),
        });
    }

    for child in children {
        for name in &child.objectives {
            union.push(ObjectiveBinding {
                source_concept: child.name.clone(),
                name: name.clone(),
                axis: stub_axis_of(name),
            });
        }
    }

    let mut axis_map: HashMap<String, Vec<ObjectiveBinding>> = HashMap::new();
    for binding in &union {
        axis_map
            .entry(binding.axis.clone())
            .or_default()
            .push(binding.clone());
    }

    let mut me_collisions: Vec<MeCollision> = axis_map
        .iter()
        .filter(|(_, bindings)| bindings.len() > 1)
        .map(|(axis, bindings)| MeCollision {
            axis: axis.clone(),
            bindings: bindings.clone(),
        })
        .collect();

    me_collisions.sort_by(|a, b| a.axis.cmp(&b.axis));

    // ── 2. CE check against required_axes registry ────────────────────────
    let mut ce_unmet: Vec<String> = Vec::new();

    for (req_axis, cardinality) in required_axes {
        let req_axis_norm = req_axis.trim().to_ascii_lowercase();
        let count = union.iter().filter(|b| b.axis == req_axis_norm).count();

        match cardinality.as_str() {
            "at_least_one" => {
                if count == 0 {
                    ce_unmet.push(format!(
                        "axis={req_axis_norm} (cardinality=at_least_one): no objective covers this axis"
                    ));
                }
            }
            "exactly_one" => {
                if count == 0 {
                    ce_unmet.push(format!(
                        "axis={req_axis_norm} (cardinality=exactly_one): no objective covers this axis"
                    ));
                } else if count > 1 {
                    ce_unmet.push(format!(
                        "axis={req_axis_norm} (cardinality=exactly_one): {count} objectives cover this axis"
                    ));
                }
            }
            _ => {
                // Unknown cardinality: treat as unmet to surface the error.
                ce_unmet.push(format!(
                    "axis={req_axis_norm}: unknown cardinality '{cardinality}'"
                ));
            }
        }
    }

    // ── 3. No stub notes when registry is live ────────────────────────────
    MeceReport {
        union,
        me_collisions,
        ce_unmet,
        stub_notes: vec![],
    }
}

// ── Dream-system MECE types ───────────────────────────────────────────────────

/// A single weighted objective in the dreaming system.
#[derive(Debug, Clone, PartialEq)]
pub struct MeceObjective {
    pub id: u64,
    pub label: String,
    /// Relative importance in [0.0, 1.0]. The sum of all objectives in a set
    /// should equal 1.0 for a valid MECE partition.
    pub weight: f32,
}

impl MeceObjective {
    pub fn new(id: u64, label: impl Into<String>, weight: f32) -> Self {
        Self { id, label: label.into(), weight }
    }
}

/// A violation found during MECE validation of a dreaming objective set.
#[derive(Debug, Clone, PartialEq)]
pub enum MeceViolation {
    /// Two objectives are not mutually exclusive (same label).
    Overlap { a: u64, b: u64 },
    /// An aspect is not covered by any objective.
    GapInCoverage { missing: String },
    /// The weights do not sum to 1.0 (tolerance ±0.01).
    WeightSumNot1 { actual: f32 },
}

/// Validates a set of [`MeceObjective`]s for ME (mutual exclusivity) and
/// weight consistency.
pub struct MeceValidator;

impl MeceValidator {
    pub fn new() -> Self {
        Self
    }

    /// Returns `WeightSumNot1` if `|sum − 1.0| > 0.01`.
    pub fn validate_weights(objectives: &[MeceObjective]) -> Option<MeceViolation> {
        let sum: f32 = objectives.iter().map(|o| o.weight).sum();
        if (sum - 1.0_f32).abs() > 0.01 {
            Some(MeceViolation::WeightSumNot1 { actual: sum })
        } else {
            None
        }
    }

    /// Returns `Overlap{a, b}` for the first pair of objectives with identical
    /// labels.
    pub fn validate_labels(objectives: &[MeceObjective]) -> Option<MeceViolation> {
        for i in 0..objectives.len() {
            for j in (i + 1)..objectives.len() {
                if objectives[i].label == objectives[j].label {
                    return Some(MeceViolation::Overlap {
                        a: objectives[i].id,
                        b: objectives[j].id,
                    });
                }
            }
        }
        None
    }

    /// Runs all checks and returns every violation found.
    pub fn validate(objectives: &[MeceObjective]) -> Vec<MeceViolation> {
        let mut violations = Vec::new();
        if let Some(v) = Self::validate_weights(objectives) {
            violations.push(v);
        }
        if let Some(v) = Self::validate_labels(objectives) {
            violations.push(v);
        }
        violations
    }

    /// Returns `true` iff `validate()` produces no violations.
    pub fn is_valid(objectives: &[MeceObjective]) -> bool {
        Self::validate(objectives).is_empty()
    }
}

impl Default for MeceValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Score for a dreaming proposal.
///
/// `EPIC_SCORE_THRESHOLD = 95.0` — `nom app dream` iterates until the score
/// reaches this level.
#[derive(Debug, Clone, PartialEq)]
pub struct AppScore {
    /// Score in [0.0, 100.0].
    pub value: f32,
}

/// The threshold above which a dreaming run is considered epic.
pub const EPIC_SCORE_THRESHOLD: f32 = 95.0;

impl AppScore {
    pub fn new(value: f32) -> Self {
        Self { value }
    }

    /// Returns `true` if the score meets the epic threshold (≥ 95.0).
    pub fn is_epic(&self) -> bool {
        self.value >= EPIC_SCORE_THRESHOLD
    }

    /// Computes a score by penalising 10 points per violation, clamped to
    /// [0.0, 100.0].
    pub fn from_violations(
        _objectives: &[MeceObjective],
        violations: &[MeceViolation],
    ) -> Self {
        let penalty = violations.len() as f32 * 10.0;
        let value = (100.0_f32 - penalty).max(0.0);
        Self { value }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod mece_tests {
    use super::*;

    // ── test 1: MeceObjective fields ─────────────────────────────────────────

    #[test]
    fn objective_fields() {
        let obj = MeceObjective::new(1, "performance", 0.4);
        assert_eq!(obj.id, 1);
        assert_eq!(obj.label, "performance");
        assert!((obj.weight - 0.4).abs() < f32::EPSILON);
    }

    // ── test 2: validate_weights — valid sum ─────────────────────────────────

    #[test]
    fn validate_weights_valid_returns_none() {
        let objectives = vec![
            MeceObjective::new(1, "security", 0.5),
            MeceObjective::new(2, "performance", 0.5),
        ];
        assert!(MeceValidator::validate_weights(&objectives).is_none());
    }

    // ── test 3: validate_weights — invalid sum ───────────────────────────────

    #[test]
    fn validate_weights_invalid_returns_violation() {
        let objectives = vec![
            MeceObjective::new(1, "security", 0.3),
            MeceObjective::new(2, "performance", 0.3),
        ];
        let result = MeceValidator::validate_weights(&objectives);
        assert!(result.is_some());
        if let Some(MeceViolation::WeightSumNot1 { actual }) = result {
            assert!((actual - 0.6).abs() < 0.001);
        } else {
            panic!("expected WeightSumNot1");
        }
    }

    // ── test 4: validate_labels — duplicate returns Overlap ─────────────────

    #[test]
    fn validate_labels_duplicate_returns_overlap() {
        let objectives = vec![
            MeceObjective::new(10, "speed", 0.5),
            MeceObjective::new(11, "speed", 0.5),
        ];
        let result = MeceValidator::validate_labels(&objectives);
        assert!(result.is_some());
        if let Some(MeceViolation::Overlap { a, b }) = result {
            assert_eq!(a, 10);
            assert_eq!(b, 11);
        } else {
            panic!("expected Overlap");
        }
    }

    // ── test 5: validate_labels — unique labels returns None ─────────────────

    #[test]
    fn validate_labels_unique_returns_none() {
        let objectives = vec![
            MeceObjective::new(1, "speed", 0.5),
            MeceObjective::new(2, "safety", 0.5),
        ];
        assert!(MeceValidator::validate_labels(&objectives).is_none());
    }

    // ── test 6: validate — collects multiple violations ──────────────────────

    #[test]
    fn validate_collects_multiple_violations() {
        // Bad weights AND duplicate labels → 2 violations.
        let objectives = vec![
            MeceObjective::new(1, "dup", 0.2),
            MeceObjective::new(2, "dup", 0.2),
        ];
        let violations = MeceValidator::validate(&objectives);
        assert_eq!(violations.len(), 2, "expected 2 violations: {violations:?}");
    }

    // ── test 7: is_valid — true for clean set ────────────────────────────────

    #[test]
    fn is_valid_true_for_clean_set() {
        let objectives = vec![
            MeceObjective::new(1, "security", 0.4),
            MeceObjective::new(2, "performance", 0.3),
            MeceObjective::new(3, "usability", 0.3),
        ];
        assert!(MeceValidator::is_valid(&objectives));
    }

    // ── test 8: is_valid — false for invalid set ─────────────────────────────

    #[test]
    fn is_valid_false_for_invalid_set() {
        let objectives = vec![
            MeceObjective::new(1, "security", 0.2),
            MeceObjective::new(2, "security", 0.2),
        ];
        assert!(!MeceValidator::is_valid(&objectives));
    }

    // ── test 9: AppScore::is_epic ────────────────────────────────────────────

    #[test]
    fn app_score_is_epic_at_or_above_threshold() {
        assert!(AppScore::new(95.0).is_epic());
        assert!(AppScore::new(100.0).is_epic());
        assert!(!AppScore::new(94.9).is_epic());
        assert!(!AppScore::new(0.0).is_epic());
    }

    // ── test 10: AppScore::from_violations ───────────────────────────────────

    #[test]
    fn app_score_from_violations_decreases_per_violation() {
        let objectives = vec![
            MeceObjective::new(1, "a", 0.5),
            MeceObjective::new(2, "b", 0.5),
        ];
        let no_violations: Vec<MeceViolation> = vec![];
        let one_violation = vec![MeceViolation::WeightSumNot1 { actual: 0.5 }];
        let two_violations = vec![
            MeceViolation::WeightSumNot1 { actual: 0.5 },
            MeceViolation::Overlap { a: 1, b: 2 },
        ];

        let score_0 = AppScore::from_violations(&objectives, &no_violations);
        let score_1 = AppScore::from_violations(&objectives, &one_violation);
        let score_2 = AppScore::from_violations(&objectives, &two_violations);

        assert!((score_0.value - 100.0).abs() < f32::EPSILON);
        assert!((score_1.value - 90.0).abs() < f32::EPSILON);
        assert!((score_2.value - 80.0).abs() < f32::EPSILON);
        assert!(score_0.value > score_1.value);
        assert!(score_1.value > score_2.value);
    }
}

// ── Legacy tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConceptDecl;

    fn concept(name: &str, objectives: &[&str]) -> ConceptDecl {
        ConceptDecl {
            name: name.to_string(),
            intent: String::new(),
            index: vec![],
            exposes: vec![],
            acceptance: vec![],
            objectives: objectives.iter().map(|s| s.to_string()).collect(),
        }
    }

    // ── test 1 ────────────────────────────────────────────────────────────────

    #[test]
    fn single_concept_no_collisions() {
        let parent = concept("auth", &["security", "speed"]);
        let report = check_mece(&parent, &[]);

        assert_eq!(report.union.len(), 2);
        assert!(report.me_collisions.is_empty(), "expected 0 collisions");
        assert!(report.ce_unmet.is_empty());
    }

    // ── test 2 ────────────────────────────────────────────────────────────────

    #[test]
    fn parent_and_child_disjoint_no_collisions() {
        let parent = concept("auth", &["security"]);
        let child = concept("renderer", &["readability"]);
        let report = check_mece(&parent, &[&child]);

        assert_eq!(report.union.len(), 2);
        let axes: Vec<&str> = report.union.iter().map(|b| b.axis.as_str()).collect();
        assert!(axes.contains(&"security"));
        assert!(axes.contains(&"readability"));
        assert!(
            report.me_collisions.is_empty(),
            "expected 0 collisions: {report:?}"
        );
    }

    // ── test 3 ────────────────────────────────────────────────────────────────

    #[test]
    fn parent_and_child_share_axis_collides() {
        let parent = concept("agent", &["security", "speed"]);
        let child = concept("policy", &["security", "privacy"]);
        let report = check_mece(&parent, &[&child]);

        assert_eq!(report.union.len(), 4);
        assert_eq!(
            report.me_collisions.len(),
            1,
            "expected exactly 1 collision: {report:?}"
        );

        let collision = &report.me_collisions[0];
        assert_eq!(collision.axis, "security");
        assert_eq!(collision.bindings.len(), 2);

        let sources: Vec<&str> = collision
            .bindings
            .iter()
            .map(|b| b.source_concept.as_str())
            .collect();
        assert!(
            sources.contains(&"agent"),
            "agent must be in collision sources"
        );
        assert!(
            sources.contains(&"policy"),
            "policy must be in collision sources"
        );
    }

    // ── test 4 ────────────────────────────────────────────────────────────────

    #[test]
    fn case_insensitive_axis_collapse() {
        let parent = concept("upper_case_agent", &["Security"]);
        let child = concept("lower_case_policy", &["security"]);
        let report = check_mece(&parent, &[&child]);

        assert_eq!(report.union.len(), 2);
        assert_eq!(
            report.me_collisions.len(),
            1,
            "Security and security must collide: {report:?}"
        );

        let collision = &report.me_collisions[0];
        assert_eq!(collision.axis, "security");
    }

    // ── test 5 ────────────────────────────────────────────────────────────────

    #[test]
    fn agent_demo_realistic_collision() {
        // Mirrors the agent_demo example:
        //   minimal_safe_agent: security, composability, speed
        //   agent_safety_policy: security, privacy, speed
        let parent = concept(
            "minimal_safe_agent",
            &["security", "composability", "speed"],
        );
        let child = concept("agent_safety_policy", &["security", "privacy", "speed"]);
        let report = check_mece(&parent, &[&child]);

        assert_eq!(report.union.len(), 6);
        assert_eq!(
            report.me_collisions.len(),
            2,
            "expected 2 collisions (security + speed): {report:?}"
        );

        let axes: Vec<&str> = report
            .me_collisions
            .iter()
            .map(|c| c.axis.as_str())
            .collect();
        assert!(axes.contains(&"security"), "security collision expected");
        assert!(axes.contains(&"speed"), "speed collision expected");

        // Each collision must name both concepts.
        for collision in &report.me_collisions {
            let sources: Vec<&str> = collision
                .bindings
                .iter()
                .map(|b| b.source_concept.as_str())
                .collect();
            assert!(
                sources.contains(&"minimal_safe_agent"),
                "minimal_safe_agent must appear in collision for axis `{}`",
                collision.axis
            );
            assert!(
                sources.contains(&"agent_safety_policy"),
                "agent_safety_policy must appear in collision for axis `{}`",
                collision.axis
            );
        }
    }

    // ── CE tests (check_mece_with_required_axes) ─────────────────────────────

    /// CE-1: empty registry → ce_unmet is empty, no stub note.
    #[test]
    fn ce_check_empty_registry_vacuous_pass() {
        let parent = concept("app", &["security", "speed"]);
        let report = check_mece_with_required_axes(&parent, &[], &[]);

        assert!(
            report.ce_unmet.is_empty(),
            "empty registry must produce ce_unmet=[], got: {:?}",
            report.ce_unmet
        );
        assert!(
            report.stub_notes.is_empty(),
            "no stub notes when registry is live: {:?}",
            report.stub_notes
        );
    }

    /// CE-2: at_least_one axis present in union → passes.
    #[test]
    fn ce_check_at_least_one_present_passes() {
        let parent = concept("auth", &["security", "speed"]);
        let required = vec![("security".to_string(), "at_least_one".to_string())];
        let report = check_mece_with_required_axes(&parent, &[], &required);

        assert!(
            report.ce_unmet.is_empty(),
            "security present → ce_unmet must be empty: {:?}",
            report.ce_unmet
        );
    }

    /// CE-3: at_least_one axis absent from union → ce_unmet contains that axis.
    #[test]
    fn ce_check_at_least_one_absent_fails() {
        let parent = concept("renderer", &["speed", "readability"]);
        let required = vec![("safety".to_string(), "at_least_one".to_string())];
        let report = check_mece_with_required_axes(&parent, &[], &required);

        assert_eq!(
            report.ce_unmet.len(),
            1,
            "missing safety axis must fire one ce_unmet: {:?}",
            report.ce_unmet
        );
        assert!(
            report.ce_unmet[0].contains("safety"),
            "ce_unmet message must name the axis: {}",
            report.ce_unmet[0]
        );
    }

    /// CE-4: exactly_one axis with two bindings → both ce_unmet AND ME collision.
    #[test]
    fn ce_check_exactly_one_duplicate_fails() {
        let parent = concept("agent", &["speed", "security"]);
        let child = concept("policy", &["speed", "privacy"]);
        let required = vec![("speed".to_string(), "exactly_one".to_string())];
        let report = check_mece_with_required_axes(&parent, &[&child], &required);

        // CE unmet because speed appears twice.
        assert_eq!(
            report.ce_unmet.len(),
            1,
            "duplicate speed must fire ce_unmet: {:?}",
            report.ce_unmet
        );
        assert!(
            report.ce_unmet[0].contains("speed"),
            "ce_unmet must name axis: {}",
            report.ce_unmet[0]
        );
        assert!(
            report.ce_unmet[0].contains("2"),
            "ce_unmet must mention count 2: {}",
            report.ce_unmet[0]
        );

        // ME collision also fires for speed.
        let me_axes: Vec<&str> = report
            .me_collisions
            .iter()
            .map(|c| c.axis.as_str())
            .collect();
        assert!(
            me_axes.contains(&"speed"),
            "ME collision must also fire for speed: {:?}",
            me_axes
        );
    }

    // ── test 6 ────────────────────────────────────────────────────────────────

    #[test]
    fn stub_note_always_present() {
        // Single concept, no children.
        let parent = concept("any", &[]);
        let report = check_mece(&parent, &[]);
        assert_eq!(report.stub_notes.len(), 1, "exactly one stub note expected");
        assert!(
            report.stub_notes[0].contains("CE check deferred"),
            "stub note must mention CE check: {}",
            report.stub_notes[0]
        );
        assert!(
            report.stub_notes[0].contains("Phase-9"),
            "stub note must mention Phase-9: {}",
            report.stub_notes[0]
        );

        // Also check with children present.
        let child = concept("child", &["speed"]);
        let report2 = check_mece(&parent, &[&child]);
        assert_eq!(report2.stub_notes.len(), 1);
    }
}
