//! Structural verification test for `stdlib/self_host/ast.nom` (Phase 3).
//!
//! NOTE: This test was converted from a parse-gate test after nom-parser
//! was deleted. The .nom files use flow-style syntax that the current
//! S1-S6 pipeline does not accept. String-based structural checks
//! replace parse calls until the parser is rewritten in Nom.
//!
//! Verifies that ast.nom exists, is non-empty, declares the expected
//! module name, and contains the expected type and function declarations.

use std::path::PathBuf;

fn ast_nom_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("stdlib/self_host/ast.nom")
}

#[test]
fn self_host_ast_structural_check() {
    let path = ast_nom_path();
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));

    assert!(
        !source.trim().is_empty(),
        "ast.nom must be non-empty (Phase 3 structural gate)"
    );

    // Module declaration
    assert!(
        source.contains("nom self_host_ast"),
        "ast.nom must declare `nom self_host_ast` module"
    );

    // Expected struct declarations
    for name in &["Param", "FnBody", "Decl"] {
        assert!(
            source.contains(&format!("struct {name}")),
            "ast.nom must contain `struct {name}`"
        );
    }

    // Expected function declarations
    for name in &[
        "is_nullary",
        "is_empty_body",
        "decl_kind_fn",
        "prim_type_integer",
    ] {
        assert!(
            source.contains(&format!("fn {name}")),
            "ast.nom must contain `fn {name}`"
        );
    }
}
