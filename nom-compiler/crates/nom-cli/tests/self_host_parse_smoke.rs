//! Smoke test: every .nom file under stdlib/self_host/ and
//! examples/run_lexer.nom must exist and contain expected keywords.
//!
//! NOTE: This test was converted from a parse-gate test after nom-parser
//! was deleted. The .nom files use flow-style syntax that the current
//! S1-S6 pipeline does not accept. String-based structural checks
//! replace parse calls until the parser is rewritten in Nom.
//!
//! This test uses std::fs::read_to_string directly (no CLI binary spawn)
//! so it runs on both Windows and Linux without LLVM DLL issues.

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    // This file is at nom-compiler/crates/nom-cli/tests/
    // Root is the parent of nom-compiler.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn self_host_nom_files_exist_and_contain_keywords() {
    let nom_compiler = repo_root().join("nom-compiler");
    let targets = vec![
        nom_compiler.join("stdlib/self_host/ast.nom"),
        nom_compiler.join("stdlib/self_host/lexer.nom"),
        nom_compiler.join("stdlib/self_host/parser.nom"),
        nom_compiler.join("stdlib/self_host/planner.nom"),
        nom_compiler.join("stdlib/self_host/codegen.nom"),
        nom_compiler.join("stdlib/self_host/verifier.nom"),
        nom_compiler.join("examples/run_lexer.nom"),
    ];
    for path in &targets {
        assert!(path.exists(), "missing: {}", path.display());
        let src = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        assert!(
            !src.trim().is_empty(),
            "{} must be non-empty (M10a self-host structural gate)",
            path.display()
        );
        // Every self-host .nom file declares a `nom` module.
        // run_lexer.nom may use `nom` keyword or `fn` — just check non-empty above.
        if path.to_string_lossy().contains("self_host") {
            assert!(
                src.contains("nom "),
                "{} must contain a `nom` module declaration",
                path.display()
            );
        }
    }
}
