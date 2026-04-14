//! Meta test: every `stdlib/self_host/*.nom` file must have a
//! matching `tests/self_host_<name>.rs` acceptance test.
//!
//! Prevents the drift where a new scaffold lands without a parse gate.
//! The roll-up smoke test covers parsing for all files, but individual
//! gate tests are where per-phase assertions (module name, required
//! declarations, expected helpers) live — skipping the per-phase
//! test means losing that discipline.

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn every_self_host_nom_has_its_own_acceptance_test() {
    let self_host_dir = repo_root().join("stdlib/self_host");
    let tests_dir = repo_root().join("crates/nom-cli/tests");

    let nom_files: Vec<String> = std::fs::read_dir(&self_host_dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", self_host_dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("nom"))
        .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();

    // lexer.nom is the working Phase 1 unit; its acceptance lives in
    // phase4_acceptance.rs / store_cli.rs rather than a dedicated
    // self_host_lexer.rs file. Whitelist to avoid false positives.
    let whitelist = &["lexer"];

    let mut missing: Vec<String> = Vec::new();
    for stem in &nom_files {
        if whitelist.contains(&stem.as_str()) {
            continue;
        }
        let expected = tests_dir.join(format!("self_host_{stem}.rs"));
        if !expected.exists() {
            missing.push(format!(
                "{}.nom → expected test {}",
                stem,
                expected.display()
            ));
        }
    }

    assert!(
        missing.is_empty(),
        "self-host scaffolds missing acceptance tests:\n{}",
        missing.join("\n")
    );
}
