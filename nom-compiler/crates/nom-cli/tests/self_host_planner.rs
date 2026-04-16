//! Structural verification test for `stdlib/self_host/planner.nom`.
//!
//! Per the self-hosting roadmap (Phase 5): the Nom-in-Nom planner is a
//! scaffold. This test fixes its *structural* contract — any edit to
//! planner.nom must preserve the expected module name, struct names,
//! and function names. Catches regressions where a well-meaning update
//! accidentally removes a declaration.
//!
//! NOTE: This test was converted from a parse-gate test after nom-parser
//! was deleted. The .nom files use flow-style syntax that the current
//! S1-S6 pipeline does not accept. String-based structural checks
//! replace parse calls until the parser is rewritten in Nom.
//!
//! Functional contract (graph build + topological sort + cycle
//! detection) arrives incrementally; this test is the first guard rail.

use std::path::PathBuf;

fn planner_nom_path() -> PathBuf {
    // crates/nom-cli/tests -> crates/nom-cli -> crates -> nom-compiler
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("stdlib/self_host/planner.nom")
}

#[test]
fn self_host_planner_structural_check() {
    let path = planner_nom_path();
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));

    assert!(
        !source.trim().is_empty(),
        "planner.nom must be non-empty (self-hosting Phase 5 structural gate)"
    );

    // Module declaration
    assert!(
        source.contains("nom self_host_planner"),
        "planner.nom must declare `nom self_host_planner` module"
    );

    // Expected struct declarations
    for name in &["Node", "Edge", "CompositionPlan", "VerifiedAST"] {
        assert!(
            source.contains(&format!("struct {name}")),
            "planner.nom must contain `struct {name}`"
        );
    }

    // Expected function declarations
    for name in &[
        "nom_plan",
        "default_on_fail",
        "edge_kind_calls",
        "is_ordering_edge",
    ] {
        assert!(
            source.contains(&format!("fn {name}")),
            "planner.nom must contain `fn {name}`"
        );
    }
}
