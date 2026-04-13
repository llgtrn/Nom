//! Closure walker for concept graphs.
//!
//! Given a starting concept name, walks its `index_into_db2` recursively to gather:
//! 1. All transitively-referenced word hashes (atoms + composed modules).
//! 2. All transitively-referenced concepts (via `IndexClause::Extends`).
//! 3. A topological order in which they should be built (leaves first, root last).
//! 4. Detected cycles, surfaced as errors.
//!
//! See `research/language-analysis/08-layered-concept-component-architecture.md` §4.3.

use std::collections::{HashMap, HashSet};

use thiserror::Error;

use crate::{CompositionDecl, ConceptDecl, EntityRef, IndexClause, NomtuFile, NomtuItem};

// ── Public types ─────────────────────────────────────────────────────────────

/// The computed closure of a concept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConceptClosure {
    /// Root concept name (the entry point).
    pub root: String,
    /// All transitively-referenced word hashes (deduplicated, ordered: leaves first).
    pub word_hashes: Vec<String>,
    /// All transitively-referenced concepts (deduplicated, ordered: leaves first, root last).
    pub concepts: Vec<String>,
    /// Concepts and words still missing a hash (the resolver hasn't pinned them yet).
    pub unresolved: Vec<UnresolvedRef>,
}

/// A reference that has no pinned hash yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedRef {
    /// Kind of the entity ("function", "module", "concept", etc.), if known.
    pub kind: Option<String>,
    /// The bare word name.
    pub word: String,
    /// The prose matching hint, if any.
    pub matching: Option<String>,
    /// The parent concept or `.nomtu` name that referenced this.
    pub referenced_from: String,
}

#[derive(Debug, Error)]
pub enum ClosureError {
    #[error("unknown concept `{0}` (not in graph)")]
    UnknownConcept(String),
    #[error("cycle detected: {path}")]
    Cycle { path: String },
    #[error("missing dependency: concept `{name}` references unknown word `{word}`")]
    MissingWord { name: String, word: String },
}

// ── ConceptGraph ─────────────────────────────────────────────────────────────

/// An in-memory view over a set of parsed concepts and modules.
///
/// Materialized by the caller (typically from `concept_defs` and `words_v2` in nom-dict).
/// This type does **not** depend on nom-dict or nom-cli.
pub struct ConceptGraph {
    /// All concept declarations in scope.
    pub concepts: Vec<ConceptDecl>,
    /// All `.nomtu` files in scope.
    pub modules: Vec<NomtuFile>,
}

// ── DFS state ─────────────────────────────────────────────────────────────────

/// White/Gray/Black coloring for cycle detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Color {
    White, // not yet visited
    Gray,  // on the current DFS stack
    Black, // fully processed
}

struct Walker<'g> {
    /// Index: concept name → ConceptDecl
    concept_index: HashMap<&'g str, &'g ConceptDecl>,
    /// Index: word → CompositionDecl (from modules)
    composition_index: HashMap<&'g str, &'g CompositionDecl>,

    // DFS state
    color: HashMap<String, Color>,
    /// Concepts in post-order (leaves first).
    concept_order: Vec<String>,
    /// Word hashes in post-order (leaves first, each unique).
    word_order: Vec<String>,
    /// Deduplicated sets (for O(1) "already emitted?" checks).
    seen_concepts: HashSet<String>,
    seen_words: HashSet<String>,
    /// Unresolved refs accumulated during the walk.
    unresolved: Vec<UnresolvedRef>,
    /// Gray path for cycle reporting (concept names only).
    gray_path: Vec<String>,
}

impl<'g> Walker<'g> {
    fn new(graph: &'g ConceptGraph) -> Self {
        let concept_index: HashMap<&str, &ConceptDecl> = graph
            .concepts
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect();

        let composition_index: HashMap<&str, &CompositionDecl> = graph
            .modules
            .iter()
            .flat_map(|f| f.items.iter())
            .filter_map(|item| match item {
                NomtuItem::Composition(c) => Some((c.word.as_str(), c)),
                NomtuItem::Entity(_) => None,
            })
            .collect();

        Walker {
            concept_index,
            composition_index,
            color: HashMap::new(),
            concept_order: Vec::new(),
            word_order: Vec::new(),
            seen_concepts: HashSet::new(),
            seen_words: HashSet::new(),
            unresolved: Vec::new(),
            gray_path: Vec::new(),
        }
    }

    /// Entry point.
    fn walk(&mut self, root: &str) -> Result<(), ClosureError> {
        if !self.concept_index.contains_key(root) {
            return Err(ClosureError::UnknownConcept(root.to_string()));
        }
        self.visit_concept(root)
    }

    // ── concept visitor ───────────────────────────────────────────────────────

    fn visit_concept(&mut self, name: &str) -> Result<(), ClosureError> {
        match self.color.get(name).copied().unwrap_or(Color::White) {
            Color::Black => return Ok(()), // already finished
            Color::Gray => {
                // Back-edge → cycle. Build path string.
                let idx = self.gray_path.iter().position(|n| n == name).unwrap_or(0);
                let cycle_names: Vec<&str> = self.gray_path[idx..]
                    .iter()
                    .map(|s| s.as_str())
                    .collect();
                let path = format!("{} -> {}", cycle_names.join(" -> "), name);
                return Err(ClosureError::Cycle { path });
            }
            Color::White => {}
        }

        self.color.insert(name.to_string(), Color::Gray);
        self.gray_path.push(name.to_string());

        // Retrieve the decl.
        let decl = self
            .concept_index
            .get(name)
            .copied()
            .ok_or_else(|| ClosureError::UnknownConcept(name.to_string()))?;

        // Clone the index so we don't hold an immutable borrow while mutating self.
        let index_clauses: Vec<IndexClause> = decl.index.clone();

        // Phase 1: detect cycles in Extends chains. Walk the base-concept DAG
        // using gray/black coloring, but WITHOUT walking entity refs. This
        // surfaces cycles early and establishes post-order for concept names.
        self.visit_bases_for_cycle_check(&index_clauses)?;

        // Phase 2: compute the effective refs (pure, no side effects).
        // change-sets (adding/removing) are applied here before anything is
        // emitted, so removed refs never reach visit_entity_ref.
        let effective_refs: Vec<EntityRef> =
            Self::compute_effective_refs(&self.concept_index, &index_clauses)?;

        // Phase 3: walk effective entity refs — only those surviving the change-set.
        for eref in &effective_refs {
            self.visit_entity_ref(eref, name)?;
        }

        // Post-order: emit this concept name.
        self.color.insert(name.to_string(), Color::Black);
        self.gray_path.pop();
        if self.seen_concepts.insert(name.to_string()) {
            self.concept_order.push(name.to_string());
        }

        Ok(())
    }

    /// Walk only base concepts for cycle detection and post-order concept emission.
    /// Does NOT visit entity refs of any base concept — those are handled by the
    /// derived concept after its change-set is applied.
    fn visit_bases_for_cycle_check(&mut self, clauses: &[IndexClause]) -> Result<(), ClosureError> {
        for clause in clauses {
            if let IndexClause::Extends { base, .. } = clause {
                if !self.concept_index.contains_key(base.as_str()) {
                    return Err(ClosureError::UnknownConcept(base.clone()));
                }
                // Use a dedicated light-weight traversal that only tracks concept
                // topology, not entity refs.
                self.check_concept_topology(base)?;
            }
        }
        Ok(())
    }

    /// Cycle-detect and post-order-emit concept names. No entity-ref side effects.
    fn check_concept_topology(&mut self, name: &str) -> Result<(), ClosureError> {
        match self.color.get(name).copied().unwrap_or(Color::White) {
            Color::Black => return Ok(()),
            Color::Gray => {
                let idx = self.gray_path.iter().position(|n| n == name).unwrap_or(0);
                let cycle_names: Vec<&str> = self.gray_path[idx..]
                    .iter()
                    .map(|s| s.as_str())
                    .collect();
                let path = format!("{} -> {}", cycle_names.join(" -> "), name);
                return Err(ClosureError::Cycle { path });
            }
            Color::White => {}
        }

        self.color.insert(name.to_string(), Color::Gray);
        self.gray_path.push(name.to_string());

        let decl = self
            .concept_index
            .get(name)
            .copied()
            .ok_or_else(|| ClosureError::UnknownConcept(name.to_string()))?;
        let clauses: Vec<IndexClause> = decl.index.clone();

        self.visit_bases_for_cycle_check(&clauses)?;

        self.color.insert(name.to_string(), Color::Black);
        self.gray_path.pop();
        // Emit into concept_order in post-order so base concepts appear before
        // any derived concept that extends them.
        if self.seen_concepts.insert(name.to_string()) {
            self.concept_order.push(name.to_string());
        }

        Ok(())
    }

    /// Pure computation: flatten `index` clauses into an effective `EntityRef` list
    /// by applying `Extends` change-sets. Does NOT mutate walker state.
    /// Assumes the call graph is acyclic (verified by `visit_bases_for_cycle_check`).
    fn compute_effective_refs(
        concept_index: &HashMap<&'g str, &'g ConceptDecl>,
        clauses: &[IndexClause],
    ) -> Result<Vec<EntityRef>, ClosureError> {
        let mut result: Vec<EntityRef> = Vec::new();

        for clause in clauses {
            match clause {
                IndexClause::Uses(refs) => {
                    result.extend_from_slice(refs);
                }
                IndexClause::Extends { base, change_set } => {
                    let base_decl = concept_index
                        .get(base.as_str())
                        .copied()
                        .ok_or_else(|| ClosureError::UnknownConcept(base.clone()))?;
                    let base_clauses: Vec<IndexClause> = base_decl.index.clone();
                    let mut base_refs =
                        Self::compute_effective_refs(concept_index, &base_clauses)?;

                    // Apply removing.
                    let removing_words: HashSet<&str> = change_set
                        .removing
                        .iter()
                        .map(|r| r.word.as_str())
                        .collect();
                    base_refs.retain(|r| !removing_words.contains(r.word.as_str()));

                    // Apply adding.
                    base_refs.extend_from_slice(&change_set.adding);

                    // Sort for determinism before appending.
                    base_refs.sort_by(|a, b| a.word.cmp(&b.word));

                    result.extend(base_refs);
                }
            }
        }

        // Deduplicate by word (stable, first occurrence wins).
        let mut seen: HashSet<String> = HashSet::new();
        let deduped: Vec<EntityRef> = result
            .into_iter()
            .filter(|r| seen.insert(r.word.clone()))
            .collect();

        Ok(deduped)
    }

    // ── entity-ref visitor ────────────────────────────────────────────────────

    fn visit_entity_ref(&mut self, eref: &EntityRef, parent: &str) -> Result<(), ClosureError> {
        match &eref.hash {
            None => {
                // Unresolved: record it.
                self.unresolved.push(UnresolvedRef {
                    kind: eref.kind.clone(),
                    word: eref.word.clone(),
                    matching: eref.matching.clone(),
                    referenced_from: parent.to_string(),
                });
            }
            Some(hash) => {
                // Check if this word is a composition and recurse into its composes list.
                if let Some(comp) = self.composition_index.get(eref.word.as_str()).copied() {
                    // Clone to avoid borrow issues.
                    let composes: Vec<EntityRef> = comp.composes.clone();
                    for child_ref in &composes {
                        self.visit_entity_ref(child_ref, &eref.word)?;
                    }
                }
                // Post-order: emit this hash after its dependencies.
                if self.seen_words.insert(hash.clone()) {
                    self.word_order.push(hash.clone());
                }
            }
        }
        Ok(())
    }
}

// ── ConceptGraph::closure ─────────────────────────────────────────────────────

impl ConceptGraph {
    /// Walk the closure starting from `root`.
    ///
    /// Returns the ordered hash list + concept list + unresolved refs.
    /// Concepts and words are ordered leaves-first, root-concept last.
    pub fn closure(&self, root: &str) -> Result<ConceptClosure, ClosureError> {
        let mut walker = Walker::new(self);
        walker.walk(root)?;
        Ok(ConceptClosure {
            root: root.to_string(),
            word_hashes: walker.word_order,
            concepts: walker.concept_order,
            unresolved: walker.unresolved,
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChangeSet, CompositionDecl, ConceptDecl, EntityRef, IndexClause, NomtuFile, NomtuItem};

    // ── helpers ──────────────────────────────────────────────────────────────

    fn concept(name: &str, index: Vec<IndexClause>) -> ConceptDecl {
        ConceptDecl {
            name: name.to_string(),
            intent: String::new(),
            index,
            exposes: vec![],
            acceptance: vec![],
            objectives: vec![],
        }
    }

    fn uses(refs: Vec<EntityRef>) -> IndexClause {
        IndexClause::Uses(refs)
    }

    fn eref_resolved(kind: &str, word: &str, hash: &str) -> EntityRef {
        EntityRef {
            kind: Some(kind.to_string()),
            word: word.to_string(),
            hash: Some(hash.to_string()),
            matching: None,
        }
    }

    fn eref_unresolved(kind: &str, word: &str, matching: Option<&str>) -> EntityRef {
        EntityRef {
            kind: Some(kind.to_string()),
            word: word.to_string(),
            hash: None,
            matching: matching.map(|s| s.to_string()),
        }
    }

    fn composition(word: &str, composes: Vec<EntityRef>) -> NomtuFile {
        NomtuFile {
            items: vec![NomtuItem::Composition(CompositionDecl {
                word: word.to_string(),
                composes,
                glue: None,
                contracts: vec![],
            })],
        }
    }

    fn empty_graph(concepts: Vec<ConceptDecl>) -> ConceptGraph {
        ConceptGraph {
            concepts,
            modules: vec![],
        }
    }

    // ── test 1: single concept, no deps ───────────────────────────────────────

    #[test]
    fn single_concept_no_deps_returns_just_root() {
        let graph = empty_graph(vec![concept("auth", vec![])]);
        let closure = graph.closure("auth").unwrap();

        assert_eq!(closure.root, "auth");
        assert_eq!(closure.word_hashes, Vec::<String>::new());
        assert_eq!(closure.concepts, vec!["auth"]);
        assert!(closure.unresolved.is_empty());
    }

    // ── test 2: concept with one resolved module ───────────────────────────────

    #[test]
    fn concept_with_one_module_resolved_returns_module_hash() {
        // Concept `auth` uses `the module foo@a1b2`
        // Module `foo` composes `the function bar@b2c3`
        let graph = ConceptGraph {
            concepts: vec![concept(
                "auth",
                vec![uses(vec![eref_resolved("module", "foo", "a1b2")])],
            )],
            modules: vec![composition(
                "foo",
                vec![eref_resolved("function", "bar", "b2c3")],
            )],
        };

        let closure = graph.closure("auth").unwrap();
        // bar@b2c3 is a leaf (no further composition), foo@a1b2 is added after it.
        assert_eq!(closure.word_hashes, vec!["b2c3", "a1b2"]);
        assert_eq!(closure.concepts, vec!["auth"]);
        assert!(closure.unresolved.is_empty());
    }

    // ── test 3: unresolved ref ─────────────────────────────────────────────────

    #[test]
    fn concept_with_unresolved_ref_collects_into_unresolved() {
        let graph = empty_graph(vec![concept(
            "auth",
            vec![uses(vec![eref_unresolved(
                "function",
                "login",
                Some("handles user authentication"),
            )])],
        )]);

        let closure = graph.closure("auth").unwrap();

        assert!(closure.word_hashes.is_empty());
        assert_eq!(closure.concepts, vec!["auth"]);
        assert_eq!(closure.unresolved.len(), 1);

        let uref = &closure.unresolved[0];
        assert_eq!(uref.kind.as_deref(), Some("function"));
        assert_eq!(uref.word, "login");
        assert_eq!(
            uref.matching.as_deref(),
            Some("handles user authentication")
        );
        assert_eq!(uref.referenced_from, "auth");
    }

    // ── test 4: cycle detection ────────────────────────────────────────────────

    #[test]
    fn cycle_detection_concept_a_uses_b_uses_a() {
        // Concept A extends B; concept B extends A → cycle.
        let graph = empty_graph(vec![
            concept(
                "a",
                vec![IndexClause::Extends {
                    base: "b".to_string(),
                    change_set: ChangeSet::default(),
                }],
            ),
            concept(
                "b",
                vec![IndexClause::Extends {
                    base: "a".to_string(),
                    change_set: ChangeSet::default(),
                }],
            ),
        ]);

        let result = graph.closure("a");
        match result {
            Err(ClosureError::Cycle { path }) => {
                assert!(path.contains('a') && path.contains('b'),
                    "Expected path to mention both a and b, got: {path}");
            }
            other => panic!("expected Cycle error, got: {other:?}"),
        }
    }

    // ── test 5: extends change-set adds and removes ────────────────────────────

    #[test]
    fn extends_change_set_adds_and_removes() {
        // Concept A uses [x@h1, y@h2, z@h3].
        // Concept B extends A with adding [w@h4] removing [y].
        // Closure of B should contain hashes for x, z, w (not y).
        let graph = empty_graph(vec![
            concept(
                "a",
                vec![uses(vec![
                    eref_resolved("function", "x", "h1"),
                    eref_resolved("function", "y", "h2"),
                    eref_resolved("function", "z", "h3"),
                ])],
            ),
            concept(
                "b",
                vec![IndexClause::Extends {
                    base: "a".to_string(),
                    change_set: ChangeSet {
                        adding: vec![eref_resolved("function", "w", "h4")],
                        removing: vec![eref_unresolved("function", "y", None)],
                    },
                }],
            ),
        ]);

        let closure = graph.closure("b").unwrap();

        assert!(closure.word_hashes.contains(&"h1".to_string()), "should contain h1 (x)");
        assert!(closure.word_hashes.contains(&"h3".to_string()), "should contain h3 (z)");
        assert!(closure.word_hashes.contains(&"h4".to_string()), "should contain h4 (w)");
        assert!(!closure.word_hashes.contains(&"h2".to_string()), "should NOT contain h2 (y)");

        // Both A and B should be in concepts; A before B.
        assert!(closure.concepts.contains(&"a".to_string()));
        assert!(closure.concepts.contains(&"b".to_string()));
        let pos_a = closure.concepts.iter().position(|c| c == "a").unwrap();
        let pos_b = closure.concepts.iter().position(|c| c == "b").unwrap();
        assert!(pos_a < pos_b, "a (base) should come before b (derived)");
    }

    // ── test 6: unknown root ───────────────────────────────────────────────────

    #[test]
    fn unknown_root_returns_unknown_concept() {
        let graph = empty_graph(vec![]);
        let result = graph.closure("nonexistent");
        match result {
            Err(ClosureError::UnknownConcept(name)) => {
                assert_eq!(name, "nonexistent");
            }
            other => panic!("expected UnknownConcept, got: {other:?}"),
        }
    }

    // ── test 7 (bonus): topological order is deterministic ────────────────────

    #[test]
    fn topological_order_is_stable() {
        // Graph: root concept uses three functions (sorted order should be stable).
        let graph = ConceptGraph {
            concepts: vec![concept(
                "root",
                vec![uses(vec![
                    eref_resolved("function", "gamma", "hg"),
                    eref_resolved("function", "alpha", "ha"),
                    eref_resolved("function", "beta", "hb"),
                ])],
            )],
            modules: vec![],
        };

        let c1 = graph.closure("root").unwrap();
        let c2 = graph.closure("root").unwrap();

        assert_eq!(c1.word_hashes, c2.word_hashes, "word_hashes must be identical across calls");
        assert_eq!(c1.concepts, c2.concepts, "concepts must be identical across calls");
    }
}
