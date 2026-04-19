//! Structural verification test for `stdlib/self_host/ast_printer.nom`.
//!
//! This keeps the AST printer scaffold visible to CI while the real
//! self-host parser subset is still growing.

use std::path::PathBuf;

fn ast_printer_nom_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("stdlib/self_host/ast_printer.nom")
}

#[test]
fn self_host_ast_printer_structural_check() {
    let path = ast_printer_nom_path();
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));

    assert!(
        !source.trim().is_empty(),
        "ast_printer.nom must be non-empty (self-host AST printer structural gate)"
    );

    assert!(
        source.contains("nom self_host_ast_printer"),
        "ast_printer.nom must declare `nom self_host_ast_printer` module"
    );

    for name in &["PrintableDecl", "PrintableSourceFile"] {
        assert!(
            source.contains(&format!("struct {name}")),
            "ast_printer.nom must contain `struct {name}`"
        );
    }

    for name in &[
        "render_source_file",
        "render_decl",
        "empty_printable_source_file",
        "decl_separator",
    ] {
        assert!(
            source.contains(&format!("fn {name}")),
            "ast_printer.nom must contain `fn {name}`"
        );
    }
}
