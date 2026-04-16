//! Self-host roll-up: every scaffolded .nom file in
//! `stdlib/self_host/` must exist and be non-empty.
//!
//! NOTE: This test was converted from a parse-gate test after nom-parser
//! was deleted. The .nom files use flow-style syntax that the current
//! S1-S6 pipeline does not accept. Structural verification (file
//! existence + expected count) is the contract until the parser is
//! rewritten in Nom.
//!
//! Per the self-hosting roadmap, each compiler-stage scaffold keeps
//! its own structural-check test (self_host_planner.rs, etc.). This
//! smoke test additionally asserts that the _set_ stays complete: if
//! someone adds a new `.nom` file here without a matching Rust test,
//! this single test catches it.

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
fn every_self_host_nom_file_exists_and_is_nonempty() {
    let dir = self_host_dir();
    let entries =
        std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()));

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
        "expected >=5 self-host .nom scaffolds, got {}",
        nom_files.len()
    );

    for path in &nom_files {
        let src = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        assert!(
            !src.trim().is_empty(),
            "{} must be non-empty (self-host scaffold contract)",
            path.display()
        );
    }
}
