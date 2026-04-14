//! `materialize_concept_graph_from_db` — rebuild a `ConceptGraph` from DB rows.

use nom_concept::{
    CompositionDecl, ConceptDecl, ConceptGraph, EntityRef, IndexClause,
    NomtuFile, NomtuItem,
};
use nom_dict::NomDict;

/// Rebuild a `ConceptGraph` from the rows already stored in `dict` for
/// `repo_id`.
///
/// Concept rows come from `concept_defs`; their `index_into_db2` JSON
/// deserialises back to `Vec<IndexClause>` (Serde already covers this via the
/// `nom-concept` Deserialize derives).
///
/// Word rows come from `entities`; their `composed_of` JSON deserialises to a
/// list of hash strings, which we use to reconstruct the `CompositionDecl`
/// needed by the closure walker's composition index.  Entities (rows with
/// `composed_of IS NULL`) are represented only by their hash in resolved
/// `EntityRef`s inside concept index clauses, so they need no separate
/// `NomtuFile` entry.
pub fn materialize_concept_graph_from_db(
    dict: &NomDict,
    repo_id: &str,
) -> Result<ConceptGraph, String> {
    // ── 1. Load concept rows ──────────────────────────────────────────
    let concept_rows = dict
        .list_concept_defs_in_repo(repo_id)
        .map_err(|e| format!("list_concept_defs_in_repo: {e}"))?;

    let mut concepts: Vec<ConceptDecl> = Vec::with_capacity(concept_rows.len());
    for row in &concept_rows {
        let index: Vec<IndexClause> = serde_json::from_str(&row.index_into_db2)
            .map_err(|e| format!("concept `{}` index_into_db2 JSON: {e}", row.name))?;

        let exposes: Vec<String> = serde_json::from_str(&row.exposes)
            .unwrap_or_default();
        let acceptance: Vec<String> = serde_json::from_str(&row.acceptance)
            .unwrap_or_default();
        let objectives: Vec<String> = serde_json::from_str(&row.objectives)
            .unwrap_or_default();

        concepts.push(ConceptDecl {
            name: row.name.clone(),
            intent: row.intent.clone(),
            index,
            exposes,
            acceptance,
            objectives,
        });
    }

    // ── 2. Load composition words (entities where composed_of IS NOT NULL) ──
    //
    // We only need CompositionDecls; entity words are fully represented by
    // their hash in the EntityRef inside concept index clauses, so no
    // NomtuFile is needed for them.
    //
    // Strategy: fetch every entities row for this repo via authored_in prefix
    // matching, then keep only those with a non-null `composed_of`.
    // NOTE: nom-dict doesn't expose a "list by repo" query for entities, so we
    // collect them by scanning the concept index clauses for any resolved hashes
    // and walking `composed_of` transitively.  For the status command this is
    // sufficient — we only need the compositions the concepts actually reference.
    let mut modules: Vec<NomtuFile> = Vec::new();
    let mut visited_hashes: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    let mut hash_queue: std::collections::VecDeque<String> =
        std::collections::VecDeque::new();

    // Seed queue from all resolved EntityRef hashes in concept index clauses.
    for concept in &concepts {
        collect_resolved_hashes_from_index(&concept.index, &mut hash_queue, &visited_hashes);
    }

    while let Some(hash) = hash_queue.pop_front() {
        if !visited_hashes.insert(hash.clone()) {
            continue;
        }
        let row = match dict.find_entity(&hash) {
            Ok(Some(r)) => r,
            Ok(None) => continue,
            Err(e) => {
                eprintln!("nom: materialize: find_entity {hash}: {e}");
                continue;
            }
        };

        if let Some(composed_of_json) = &row.composed_of {
            // Deserialise `composed_of` as a JSON array — may be hashes or word names
            // (the sync path stores word names when hashes aren't yet resolved).
            let composes_strs: Vec<String> =
                serde_json::from_str(composed_of_json).unwrap_or_default();

            // Build EntityRef list. Entries that look like hex-64 are hashes;
            // others are word names only (unresolved from the sync pass).
            let composes: Vec<EntityRef> = composes_strs
                .iter()
                .map(|s| {
                    if s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()) {
                        // It's a hash — enqueue for further traversal.
                        if !visited_hashes.contains(s) {
                            hash_queue.push_back(s.clone());
                        }
                        EntityRef {
                            kind: None,
                            word: s.clone(), // word unknown from hash alone; use hash as word
                            hash: Some(s.clone()),
                            matching: None,
                            typed_slot: false,
                            confidence_threshold: None,
                        }
                    } else {
                        // It's a word name from an unresolved composition.
                        EntityRef {
                            kind: None,
                            word: s.clone(),
                            hash: None,
                            matching: None,
                            typed_slot: false,
                            confidence_threshold: None,
                        }
                    }
                })
                .collect();

            modules.push(NomtuFile {
                items: vec![NomtuItem::Composition(CompositionDecl {
                    word: row.word.clone(),
                    composes,
                    glue: None,
                    contracts: vec![],
                    effects: vec![],
                })],
            });
        }
        // Entities (no composed_of) don't need a NomtuFile entry.
    }

    Ok(ConceptGraph { concepts, modules })
}

/// Walk all resolved hashes in `index` clauses and push them into `queue` if
/// not already in `visited`.
fn collect_resolved_hashes_from_index(
    index: &[IndexClause],
    queue: &mut std::collections::VecDeque<String>,
    visited: &std::collections::HashSet<String>,
) {
    for clause in index {
        match clause {
            IndexClause::Uses(refs) => {
                for eref in refs {
                    if let Some(h) = &eref.hash {
                        if !visited.contains(h) {
                            queue.push_back(h.clone());
                        }
                    }
                }
            }
            IndexClause::Extends { change_set, .. } => {
                for eref in &change_set.adding {
                    if let Some(h) = &eref.hash {
                        if !visited.contains(h) {
                            queue.push_back(h.clone());
                        }
                    }
                }
            }
        }
    }
}
