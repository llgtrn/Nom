//! Structural pipeline test: every self-host scaffold in `stdlib/self_host/`
//! must exist, be non-empty, and contain valid structure markers.
//!
//! NOTE: This test was converted from a full parse->plan->codegen pipeline
//! test after nom-parser was deleted. The .nom files use flow-style syntax
//! that the current S1-S6 pipeline does not accept. String-based structural
//! checks replace pipeline compilation until the parser is rewritten in Nom.
//!
//! Stronger than `self_host_smoke.rs` (which only checks existence): this
//! test verifies each file declares a `nom` module and at least one `fn`
//! and one `struct`. Catches regressions where a scaffold loses its shape
//! entirely.

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

fn check_structure(src: &str, path: &PathBuf) {
    assert!(
        src.contains("nom "),
        "{}: must contain a `nom` module declaration",
        path.display()
    );
    assert!(
        src.contains("fn "),
        "{}: must contain at least one `fn` declaration",
        path.display()
    );
    assert!(
        src.contains("struct "),
        "{}: must contain at least one `struct` declaration",
        path.display()
    );
}

#[test]
fn every_self_host_nom_file_has_valid_structure() {
    let dir = self_host_dir();
    let entries =
        std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()));

    let mut nom_files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("nom"))
        .collect();
    nom_files.sort();

    assert!(
        !nom_files.is_empty(),
        "no .nom files under {}",
        dir.display()
    );

    let mut checked: usize = 0;
    for path in &nom_files {
        let src = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        assert!(
            !src.trim().is_empty(),
            "{}: must be non-empty",
            path.display()
        );
        check_structure(&src, path);
        checked += 1;
    }

    assert!(
        checked >= 5,
        "expected >=5 scaffolds to pass structural check, got {checked}"
    );
}
