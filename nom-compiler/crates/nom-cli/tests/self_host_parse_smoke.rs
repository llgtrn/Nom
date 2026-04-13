//! Smoke test: every .nom file under stdlib/self_host/ and
//! examples/run_lexer.nom must parse cleanly through the shipped
//! lexer + parser.  Gates regressions like the VN-removal churn.
//!
//! This test uses nom_parser::parse_source directly (no CLI binary spawn)
//! so it runs on both Windows and Linux without LLVM DLL issues.
//! The #[cfg(not(windows))] guard is omitted intentionally: the library
//! API does not link LLVM at test time.

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
fn self_host_nom_files_parse_cleanly() {
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
        nom_parser::parse_source(&src).unwrap_or_else(|e| {
            panic!(
                "{} must parse cleanly (M10a self-host parse gate) — {e}",
                path.display()
            )
        });
    }
}
