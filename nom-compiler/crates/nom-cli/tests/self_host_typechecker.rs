//! Structural verification test for `stdlib/self_host/typechecker.nom`.
//!
//! Keeps the self-host typechecker scaffold pinned while the real
//! implementation grows incrementally.

use std::path::PathBuf;

fn typechecker_nom_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("stdlib/self_host/typechecker.nom")
}

#[test]
fn self_host_typechecker_structural_check() {
    let path = typechecker_nom_path();
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));

    assert!(
        !source.trim().is_empty(),
        "typechecker.nom must be non-empty (self-host typechecker structural gate)"
    );

    assert!(
        source.contains("nom self_host_typechecker"),
        "typechecker.nom must declare `nom self_host_typechecker` module"
    );

    for name in &["TypedBinding", "TypeError", "TypedProgram"] {
        assert!(
            source.contains(&format!("struct {name}")),
            "typechecker.nom must contain `struct {name}`"
        );
    }

    for name in &[
        "nom_typecheck",
        "empty_typed_program",
        "prim_integer",
        "types_compatible",
    ] {
        assert!(
            source.contains(&format!("fn {name}")),
            "typechecker.nom must contain `fn {name}`"
        );
    }

    for name in &["default_entry_type", "has_type_errors", "is_supported_primitive"] {
        assert!(
            source.contains(&format!("fn {name}")),
            "typechecker.nom must contain `fn {name}`"
        );
    }
}
