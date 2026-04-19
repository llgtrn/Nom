//! Structural verification test for `stdlib/self_host/resolver.nom`.
//!
//! Mirrors the existing self-host scaffold gates: ensure the resolver
//! stage exists, keeps its expected module name, and retains the
//! declarations the next bootstrap step will build on.

use std::path::PathBuf;

fn resolver_nom_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("stdlib/self_host/resolver.nom")
}

#[test]
fn self_host_resolver_structural_check() {
    let path = resolver_nom_path();
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));

    assert!(
        !source.trim().is_empty(),
        "resolver.nom must be non-empty (self-host resolver structural gate)"
    );

    assert!(
        source.contains("nom self_host_resolver"),
        "resolver.nom must declare `nom self_host_resolver` module"
    );

    for name in &["ModuleRef", "ResolvedBinding", "ResolvedProgram"] {
        assert!(
            source.contains(&format!("struct {name}")),
            "resolver.nom must contain `struct {name}`"
        );
    }

    for name in &[
        "nom_resolve",
        "empty_program",
        "binding_kind_local",
        "binding_kind_import",
    ] {
        assert!(
            source.contains(&format!("fn {name}")),
            "resolver.nom must contain `fn {name}`"
        );
    }

    assert!(
        source.contains("return imports.length"),
        "resolver.nom must implement `import_count()` with `imports.length` instead of the placeholder zero return"
    );

    assert!(
        source.contains("unresolved_count: 0"),
        "resolver.nom must seed `nom_resolve()` with zero unresolved bindings until identifier-walk logic lands"
    );
}
