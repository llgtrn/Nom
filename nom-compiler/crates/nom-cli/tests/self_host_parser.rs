//! Acceptance test for `stdlib/self_host/parser.nom` (Phase 2).
//! Same contract as self_host_planner.rs: parses via nom_parser.

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
fn self_host_parser_parses() {
    let path = parser_nom_path();
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let sf = nom_parser::parse_source(&source).unwrap_or_else(|e| {
        panic!("parser.nom must parse (Phase 2 gate) — parse error: {e}")
    });
    let names: Vec<String> = sf.declarations.iter().map(|d| d.name.name.clone()).collect();
    assert!(
        names.contains(&"self_host_parser".to_string()),
        "expected `self_host_parser` module: {names:?}"
    );
}
