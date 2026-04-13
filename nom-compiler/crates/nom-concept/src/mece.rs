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

// ── Tests ─────────────────────────────────────────────────────────────────────

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
        assert!(report.me_collisions.is_empty(), "expected 0 collisions: {report:?}");
    }

    // ── test 3 ────────────────────────────────────────────────────────────────

    #[test]
    fn parent_and_child_share_axis_collides() {
        let parent = concept("agent", &["security", "speed"]);
        let child = concept("policy", &["security", "privacy"]);
        let report = check_mece(&parent, &[&child]);

        assert_eq!(report.union.len(), 4);
        assert_eq!(report.me_collisions.len(), 1, "expected exactly 1 collision: {report:?}");

        let collision = &report.me_collisions[0];
        assert_eq!(collision.axis, "security");
        assert_eq!(collision.bindings.len(), 2);

        let sources: Vec<&str> = collision.bindings.iter().map(|b| b.source_concept.as_str()).collect();
        assert!(sources.contains(&"agent"), "agent must be in collision sources");
        assert!(sources.contains(&"policy"), "policy must be in collision sources");
    }

    // ── test 4 ────────────────────────────────────────────────────────────────

    #[test]
    fn case_insensitive_axis_collapse() {
        let parent = concept("upper_case_agent", &["Security"]);
        let child = concept("lower_case_policy", &["security"]);
        let report = check_mece(&parent, &[&child]);

        assert_eq!(report.union.len(), 2);
        assert_eq!(report.me_collisions.len(), 1, "Security and security must collide: {report:?}");

        let collision = &report.me_collisions[0];
        assert_eq!(collision.axis, "security");
    }

    // ── test 5 ────────────────────────────────────────────────────────────────

    #[test]
    fn agent_demo_realistic_collision() {
        // Mirrors the agent_demo example:
        //   minimal_safe_agent: security, composability, speed
        //   agent_safety_policy: security, privacy, speed
        let parent = concept("minimal_safe_agent", &["security", "composability", "speed"]);
        let child = concept("agent_safety_policy", &["security", "privacy", "speed"]);
        let report = check_mece(&parent, &[&child]);

        assert_eq!(report.union.len(), 6);
        assert_eq!(
            report.me_collisions.len(),
            2,
            "expected 2 collisions (security + speed): {report:?}"
        );

        let axes: Vec<&str> = report.me_collisions.iter().map(|c| c.axis.as_str()).collect();
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

    // ── test 6 ────────────────────────────────────────────────────────────────

    #[test]
    fn stub_note_always_present() {
        // Single concept, no children.
        let parent = concept("any", &[]);
        let report = check_mece(&parent, &[]);
        assert_eq!(
            report.stub_notes.len(),
            1,
            "exactly one stub note expected"
        );
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
