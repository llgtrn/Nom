//! Acceptance test for `stdlib/self_host/codegen.nom`.
//!
//! Per the self-hosting roadmap (Phase 6): the Nom-in-Nom codegen is
//! a scaffold today. This test fixes its *syntactic* contract — any
//! edit must still parse via nom_parser. Catches regressions where
//! an update uses an aspirational feature the parser doesn't accept
//! yet (tuple returns, generic lists, enum variants with payloads).
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
fn self_host_codegen_parses() {
    let path = codegen_nom_path();
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let sf = nom_parser::parse_source(&source).unwrap_or_else(|e| {
        panic!(
            "codegen.nom must parse as valid Nom (self-hosting Phase 6 gate) — \
             parse error: {e}"
        )
    });
    let names: Vec<String> = sf
        .declarations
        .iter()
        .map(|d| d.name.name.clone())
        .collect();
    assert!(
        names.contains(&"self_host_codegen".to_string()),
        "expected `self_host_codegen` module: {names:?}"
    );
}
