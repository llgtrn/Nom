//! Acceptance test for `stdlib/self_host/planner.nom`.
//!
//! Per the self-hosting roadmap (Phase 5): the Nom-in-Nom planner is a
//! scaffold today. This test fixes its *syntactic* contract — any edit
//! to planner.nom must still parse via nom_parser. Catches regressions
//! where a well-meaning update accidentally uses an aspirational
//! feature the parser doesn't accept yet (tuple returns, generic
//! lists, enum variants with payloads).
//!
//! Functional contract (graph build + topological sort + cycle
//! detection) arrives incrementally; this test is the first guard
//! rail.

use std::path::PathBuf;

fn planner_nom_path() -> PathBuf {
    // crates/nom-cli/tests → crates/nom-cli → crates → nom-compiler
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("stdlib/self_host/planner.nom")
}

#[test]
fn self_host_planner_parses() {
    let path = planner_nom_path();
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let sf = nom_parser::parse_source(&source).unwrap_or_else(|e| {
        panic!(
            "planner.nom must parse as valid Nom (self-hosting Phase 5 gate) — \
             parse error: {e}"
        )
    });
    // Sanity: the file parses as exactly one `nom` module whose name is
    // `self_host_planner`. Inner struct + fn decls live inside the
    // module body; surface parse success is the contract we're fixing
    // today.
    let names: Vec<String> = sf
        .declarations
        .iter()
        .map(|d| d.name.name.clone())
        .collect();
    assert!(
        names.contains(&"self_host_planner".to_string()),
        "expected `self_host_planner` module: {names:?}"
    );
}
