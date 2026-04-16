//! Structural verification test for `stdlib/self_host/codegen.nom`.
//!
//! Per the self-hosting roadmap (Phase 6): the Nom-in-Nom codegen is
//! a scaffold. This test fixes its *structural* contract — any edit
//! must preserve the expected module name, struct names, and function
//! names. Catches regressions where an update silently removes or
//! renames a declaration.
//!
//! NOTE: This test was converted from a parse-gate test after nom-parser
//! was deleted. The .nom files use flow-style syntax that the current
//! S1-S6 pipeline does not accept. String-based structural checks
//! replace parse calls until the parser is rewritten in Nom.
//!
//! Parallel to `self_host_planner.rs` — same pattern for the same
//! reason: pin the scaffold's surface so gradual growth has a
//! regression net.

use std::path::PathBuf;

fn codegen_nom_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("stdlib/self_host/codegen.nom")
}

#[test]
fn self_host_codegen_structural_check() {
    let path = codegen_nom_path();
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));

    assert!(
        !source.trim().is_empty(),
        "codegen.nom must be non-empty (self-hosting Phase 6 structural gate)"
    );

    // Module declaration
    assert!(
        source.contains("nom self_host_codegen"),
        "codegen.nom must declare `nom self_host_codegen` module"
    );

    // Expected struct declarations
    for name in &["GeneratedSource", "CompositionPlan"] {
        assert!(
            source.contains(&format!("struct {name}")),
            "codegen.nom must contain `struct {name}`"
        );
    }

    // Expected function declarations
    for name in &[
        "nom_codegen",
        "default_entry_symbol",
        "rust_ty_i64",
        "lower_type",
    ] {
        assert!(
            source.contains(&format!("fn {name}")),
            "codegen.nom must contain `fn {name}`"
        );
    }
}
