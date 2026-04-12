//! Self-host roll-up: every scaffolded .nom file in
//! `stdlib/self_host/` must parse via nom_parser.
//!
//! Per the self-hosting roadmap, each compiler-stage scaffold keeps
//! its own parse-gate test (self_host_planner.rs, self_host_codegen.rs,
//! …). This smoke test additionally asserts that the _set_ stays
//! complete: if someone adds a new `.nom` file here without a
//! matching Rust parse-gate test, this single test catches it.
//!
//! As scaffolds grow into real implementations, individual gate tests
//! add richer assertions; this smoke test stays minimal (parse only).

use std::path::PathBuf;

fn self_host_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("stdlib/self_host")
}

#[test]
fn every_self_host_nom_file_parses() {
    let dir = self_host_dir();
    let entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()));

    let mut nom_files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("nom"))
        .collect();
    nom_files.sort(); // deterministic order for failure messages

    assert!(
        !nom_files.is_empty(),
        "no .nom files under {}",
        dir.display()
    );

    // Keep a floor so shrinkage accidents are caught. Raise as new
    // scaffolds land; lower only on explicit retirement.
    assert!(
        nom_files.len() >= 5,
        "expected ≥5 self-host .nom scaffolds, got {}",
        nom_files.len()
    );

    for path in &nom_files {
        let src = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        nom_parser::parse_source(&src).unwrap_or_else(|e| {
            panic!(
                "{} must parse (self-host scaffold contract) — {e}",
                path.display()
            )
        });
    }
}
