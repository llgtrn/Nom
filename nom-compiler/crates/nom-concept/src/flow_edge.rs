//! T2.1 first slice — verifier at flow edge.
//!
//! When the S6 stage emits a [`NomFile`] / [`NomtuFile`], we walk its
//! `composes` chains (concepts) and `Uses` ref lists looking for
//! structural smells that don't require contract algebra to detect:
//!
//!   1. [`FlowEdgeFinding::ConsecutiveDuplicate`] — `composes A then A`
//!      almost always indicates a typo. Caught here.
//!   2. [`FlowEdgeFinding::LoopReference`] — `composes A then B then A`
//!      — the same entity appears more than once in a single chain. A
//!      composition is meant to be a directed path, not a loop.
//!   3. [`FlowEdgeFinding::SelfReference`] — a concept's `composes` /
//!      `index` references the concept by its own name. Direct
//!      self-recursion on a composition is almost always wrong.
//!
//! Future cycles add solver-backed contract checks (B's `requires`
//! must follow from A's `ensures`), entity-typed-slot resolution,
//! and effect propagation. Those checks need dictionary lookups and
//! are not in S6's current scope; this slice ships what's checkable
//! with only the [`PipelineOutput`] in hand.

use crate::{
    CompositionDecl, ConceptDecl, EntityRef, IndexClause, NomFile, NomtuFile, NomtuItem,
};

/// One structural finding from the flow-edge verifier. Pure data —
/// callers turn this into diagnostics, LSP markers, or warnings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowEdgeFinding {
    /// Two adjacent steps in a composes chain refer to the same entity.
    ConsecutiveDuplicate {
        decl_name: String,
        chain_position: usize,
        entity: String,
    },
    /// The same entity appears more than once anywhere in a composes
    /// chain. (Note: `ConsecutiveDuplicate` is a stricter case of this
    /// — when both fire, both are reported.)
    LoopReference {
        decl_name: String,
        entity: String,
        first_position: usize,
        repeat_position: usize,
    },
    /// A declaration's chain references the declaration itself by name.
    SelfReference {
        decl_name: String,
        chain_position: usize,
    },
}

/// Verify the structural well-formedness of every composes chain in
/// every `concept` block of `file`. Returns one finding per smell.
pub fn check_nom_file(file: &NomFile) -> Vec<FlowEdgeFinding> {
    let mut out = Vec::new();
    for c in &file.concepts {
        check_concept(c, &mut out);
    }
    out
}

/// Verify every composition's `composes` chain in `file`.
pub fn check_nomtu_file(file: &NomtuFile) -> Vec<FlowEdgeFinding> {
    let mut out = Vec::new();
    for item in &file.items {
        if let NomtuItem::Composition(comp) = item {
            check_composition(comp, &mut out);
        }
    }
    out
}

fn check_concept(c: &ConceptDecl, out: &mut Vec<FlowEdgeFinding>) {
    for clause in &c.index {
        if let IndexClause::Uses(refs) = clause {
            walk_chain(&c.name, refs, out);
        }
    }
}

fn check_composition(comp: &CompositionDecl, out: &mut Vec<FlowEdgeFinding>) {
    walk_chain(&comp.word, &comp.composes, out);
}

/// Single chain check: emit findings for adjacent duplicates, loop
/// references, and self-references. The `decl_name` is the surrounding
/// declaration so callers can group findings by source decl.
fn walk_chain(decl_name: &str, chain: &[EntityRef], out: &mut Vec<FlowEdgeFinding>) {
    // Track first occurrence of each entity word for loop detection.
    let mut first_seen: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::new();

    for (i, eref) in chain.iter().enumerate() {
        let word = eref.word.as_str();
        if word.is_empty() {
            // Typed-slot refs (.nomx v2) carry word="" — they refer to
            // a kind+intent rather than a named entity. Skip those for
            // structural duplicate detection; a future cycle adds
            // hash-based loop detection once the resolver fills hashes.
            continue;
        }

        // Self-reference check.
        if word == decl_name {
            out.push(FlowEdgeFinding::SelfReference {
                decl_name: decl_name.to_string(),
                chain_position: i,
            });
        }

        // Consecutive duplicate check (against previous step).
        if i > 0 {
            let prev = chain[i - 1].word.as_str();
            if !prev.is_empty() && prev == word {
                out.push(FlowEdgeFinding::ConsecutiveDuplicate {
                    decl_name: decl_name.to_string(),
                    chain_position: i,
                    entity: word.to_string(),
                });
            }
        }

        // Loop-reference check (any earlier occurrence).
        if let Some(&first) = first_seen.get(word) {
            // ConsecutiveDuplicate already covers i = first + 1; the
            // loop case is the strictly-non-adjacent variant.
            if i != first + 1 {
                out.push(FlowEdgeFinding::LoopReference {
                    decl_name: decl_name.to_string(),
                    entity: word.to_string(),
                    first_position: first,
                    repeat_position: i,
                });
            }
        } else {
            first_seen.insert(word, i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eref(word: &str) -> EntityRef {
        EntityRef {
            kind: Some("function".into()),
            word: word.into(),
            hash: None,
            matching: None,
            typed_slot: false,
            confidence_threshold: None,
        }
    }

    fn typed_slot_eref() -> EntityRef {
        EntityRef {
            kind: Some("function".into()),
            word: String::new(),
            hash: None,
            matching: Some("compute".into()),
            typed_slot: true,
            confidence_threshold: None,
        }
    }

    fn concept_with_uses(name: &str, refs: Vec<EntityRef>) -> ConceptDecl {
        ConceptDecl {
            name: name.to_string(),
            intent: "test".to_string(),
            index: vec![IndexClause::Uses(refs)],
            exposes: Vec::new(),
            acceptance: Vec::new(),
            objectives: Vec::new(),
        }
    }

    #[test]
    fn clean_chain_yields_no_findings() {
        let c = concept_with_uses("pipeline", vec![eref("a"), eref("b"), eref("c")]);
        let file = NomFile { concepts: vec![c] };
        assert!(check_nom_file(&file).is_empty());
    }

    #[test]
    fn consecutive_duplicate_is_flagged() {
        let c = concept_with_uses("pipeline", vec![eref("a"), eref("a"), eref("b")]);
        let file = NomFile { concepts: vec![c] };
        let findings = check_nom_file(&file);
        assert!(findings
            .iter()
            .any(|f| matches!(f, FlowEdgeFinding::ConsecutiveDuplicate { entity, .. } if entity == "a")));
    }

    #[test]
    fn loop_reference_flagged_only_when_non_adjacent() {
        let c = concept_with_uses(
            "pipeline",
            vec![eref("a"), eref("b"), eref("a")],
        );
        let file = NomFile { concepts: vec![c] };
        let findings = check_nom_file(&file);
        // Loop finding fires for the non-adjacent repeat of `a`.
        assert!(findings.iter().any(|f| matches!(
            f,
            FlowEdgeFinding::LoopReference { entity, first_position: 0, repeat_position: 2, .. }
            if entity == "a"
        )));
        // No consecutive-duplicate for `a` because positions 0 and 2 are not adjacent.
        assert!(!findings
            .iter()
            .any(|f| matches!(f, FlowEdgeFinding::ConsecutiveDuplicate { entity, .. } if entity == "a")));
    }

    #[test]
    fn self_reference_is_flagged() {
        let c = concept_with_uses("pipeline", vec![eref("pipeline")]);
        let file = NomFile { concepts: vec![c] };
        let findings = check_nom_file(&file);
        assert!(findings
            .iter()
            .any(|f| matches!(f, FlowEdgeFinding::SelfReference { decl_name, .. } if decl_name == "pipeline")));
    }

    #[test]
    fn typed_slot_refs_are_skipped_for_word_based_checks() {
        // Two typed-slot refs in a row produce no "consecutive duplicate"
        // finding because their `word` is empty — there's nothing to
        // duplicate-match on. Future cycle adds hash-based detection.
        let c = concept_with_uses("pipeline", vec![typed_slot_eref(), typed_slot_eref()]);
        let file = NomFile { concepts: vec![c] };
        assert!(check_nom_file(&file).is_empty());
    }

    #[test]
    fn nomtu_composition_chains_also_checked() {
        let comp = CompositionDecl {
            word: "build_pipeline".into(),
            composes: vec![eref("step_a"), eref("step_a"), eref("step_b")],
            glue: None,
            contracts: Vec::new(),
            effects: Vec::new(),
        };
        let file = NomtuFile {
            items: vec![NomtuItem::Composition(comp)],
        };
        let findings = check_nomtu_file(&file);
        assert!(findings
            .iter()
            .any(|f| matches!(f, FlowEdgeFinding::ConsecutiveDuplicate { entity, .. } if entity == "step_a")));
    }
}
