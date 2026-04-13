//! Handlers for `nom build` subcommands.
//!
//! `nom build status <repo>` loads the concept closure from the DB,
//! attempts to resolve unresolved refs via the stub resolver, and reports
//! build-readiness per concept.  It is read-only — no compilation or
//! codegen is performed.

use std::path::Path;

use nom_dict::NomDict;

use crate::store::{materialize_concept_graph_from_db, resolve_closure};

/// CLI entry point: `nom build status <repo> [--dict <path>] [--concept <name>]`.
///
/// Exit codes:
///   0 — all concepts in scope resolved cleanly (zero still-unresolved refs).
///   1 — at least one concept has unresolved refs, a DB error occurred, or
///       `--concept <name>` was given but no such concept was found in the repo.
pub fn cmd_build_status(
    repo: &Path,
    dict: &Path,
    concept_filter: Option<&str>,
) -> i32 {
    // ── open dict ────────────────────────────────────────────────────────────
    let dict_db = match open_dict_in_place(dict) {
        Some(d) => d,
        None => return 1,
    };

    // ── derive repo_id (basename of repo path) ───────────────────────────────
    let repo_id = repo
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    // ── materialise ConceptGraph from DB ─────────────────────────────────────
    let graph = match materialize_concept_graph_from_db(&dict_db, repo_id) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("nom build status: cannot materialise graph: {e}");
            return 1;
        }
    };

    if graph.concepts.is_empty() {
        println!("nom build status: no concepts found for repo `{repo_id}`.");
        println!("  Run `nom store sync <repo> --dict <path>` first.");
        // Treat as clean: nothing to fail.
        return 0;
    }

    // ── apply --concept filter ────────────────────────────────────────────────
    let concepts_in_scope: Vec<&nom_concept::ConceptDecl> = if let Some(name) = concept_filter {
        let filtered: Vec<&nom_concept::ConceptDecl> = graph
            .concepts
            .iter()
            .filter(|c| c.name == name)
            .collect();
        if filtered.is_empty() {
            eprintln!(
                "nom build status: concept `{name}` not found in repo `{repo_id}`."
            );
            eprintln!("  Available concepts:");
            for c in &graph.concepts {
                eprintln!("    {}", c.name);
            }
            return 1;
        }
        filtered
    } else {
        graph.concepts.iter().collect()
    };

    // ── walk closure + resolve for each concept ───────────────────────────────
    let mut any_unresolved = false;

    for concept in concepts_in_scope {
        // Walk the closure.  Cycle errors are printed but don't fail other concepts.
        let closure = match graph.closure(&concept.name) {
            Ok(c) => c,
            Err(nom_concept::ClosureError::Cycle { path }) => {
                eprintln!(
                    "nom build status: [{}] cycle detected: {path}",
                    concept.name
                );
                any_unresolved = true;
                continue;
            }
            Err(e) => {
                eprintln!("nom build status: [{}] closure error: {e}", concept.name);
                any_unresolved = true;
                continue;
            }
        };

        // Run the stub resolver against words_v2.
        let (resolved, still_unresolved, stats) = resolve_closure(&closure, &dict_db);

        let total_words = closure.word_hashes.len() + stats.resolved + stats.still_unresolved;

        println!("concept: {}", concept.name);
        println!(
            "  words resolved: {}/{total_words}",
            stats.resolved
        );
        println!("  word hashes in closure: {}", closure.word_hashes.len());

        if stats.ambiguous > 0 {
            println!("  ambiguous (picked smallest hash): {}", stats.ambiguous);
            for r in resolved.iter().filter(|r| !r.alternatives.is_empty()) {
                println!(
                    "    `{}` → {} (alternatives: {})",
                    r.word,
                    &r.hash[..16.min(r.hash.len())],
                    r.alternatives
                        .iter()
                        .map(|h| &h[..16.min(h.len())])
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }

        if still_unresolved.is_empty() {
            println!("  status: all clear");
        } else {
            any_unresolved = true;
            println!("  status: {} unresolved ref(s)", still_unresolved.len());
            for uref in &still_unresolved {
                let kind_str = uref
                    .kind
                    .as_deref()
                    .map(|k| format!("{k} "))
                    .unwrap_or_default();
                let matching_str = uref
                    .matching
                    .as_deref()
                    .map(|m| format!(" matching \"{m}\""))
                    .unwrap_or_default();
                println!(
                    "    unresolved: {kind_str}`{}`{matching_str} (from `{}`)",
                    uref.word, uref.referenced_from
                );
            }
        }
        println!();
    }

    if any_unresolved { 1 } else { 0 }
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn open_dict_in_place(dict: &Path) -> Option<NomDict> {
    // If dict points directly at a .db file, use open_in_place; otherwise
    // open the directory root (same logic as store::open_dict).
    let result = if dict.extension().is_some_and(|e| e == "db") {
        NomDict::open_in_place(dict)
    } else {
        NomDict::open(dict)
    };
    match result {
        Ok(d) => Some(d),
        Err(e) => {
            eprintln!("nom: cannot open dict at {}: {e}", dict.display());
            None
        }
    }
}
