//! Structural verification test for `stdlib/self_host/parser.nom` (Phase 2).
//!
//! NOTE: This test was converted from a parse-gate test after nom-parser
//! was deleted. The .nom files use flow-style syntax that the current
//! S1-S6 pipeline does not accept. String-based structural checks
//! replace parse calls until the parser is rewritten in Nom.
//!
//! Verifies that parser.nom exists, is non-empty, declares the expected
//! module name, and contains the expected type and function declarations.

use std::path::PathBuf;

fn parser_nom_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("stdlib/self_host/parser.nom")
}

#[test]
fn self_host_parser_structural_check() {
    let path = parser_nom_path();
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));

    assert!(
        !source.trim().is_empty(),
        "parser.nom must be non-empty (Phase 2 structural gate)"
    );

    // Module declaration
    assert!(
        source.contains("nom self_host_parser"),
        "parser.nom must declare `nom self_host_parser` module"
    );

    // Expected struct declarations
    for name in &["TokenStream", "SourceFile"] {
        assert!(
            source.contains(&format!("struct {name}")),
            "parser.nom must contain `struct {name}`"
        );
    }

    // Expected function declarations
    for name in &[
        "nom_parse",
        "empty_source_file",
        "source_file_has_known_classifier",
        "nom_classifier",
        "flow_classifier",
        "is_known_classifier",
    ] {
        assert!(
            source.contains(&format!("fn {name}")),
            "parser.nom must contain `fn {name}`"
        );
    }
}
