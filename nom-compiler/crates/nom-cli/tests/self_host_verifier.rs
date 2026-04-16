//! Structural verification test for `stdlib/self_host/verifier.nom` (Phase 4).
//!
//! NOTE: This test was converted from a parse-gate test after nom-parser
//! was deleted. The .nom files use flow-style syntax that the current
//! S1-S6 pipeline does not accept. String-based structural checks
//! replace parse calls until the parser is rewritten in Nom.
//!
//! Verifies that verifier.nom exists, is non-empty, declares the expected
//! module name, and contains the expected type and function declarations.

use std::path::PathBuf;

fn verifier_nom_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("stdlib/self_host/verifier.nom")
}

#[test]
fn self_host_verifier_structural_check() {
    let path = verifier_nom_path();
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));

    assert!(
        !source.trim().is_empty(),
        "verifier.nom must be non-empty (Phase 4 structural gate)"
    );

    // Module declaration
    assert!(
        source.contains("nom self_host_verifier"),
        "verifier.nom must declare `nom self_host_verifier` module"
    );

    // Expected struct declarations
    for name in &["SourceFile", "VerifiedAST"] {
        assert!(
            source.contains(&format!("struct {name}")),
            "verifier.nom must contain `struct {name}`"
        );
    }

    // Expected function declarations
    for name in &[
        "nom_verify",
        "effect_pure",
        "effect_reads",
        "is_readonly_effect",
    ] {
        assert!(
            source.contains(&format!("fn {name}")),
            "verifier.nom must contain `fn {name}`"
        );
    }
}
