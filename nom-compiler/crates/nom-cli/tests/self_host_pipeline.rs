//! Pipeline test: every self-host scaffold in `stdlib/self_host/`
//! must traverse the full parse → plan → codegen pipeline without
//! error, not just parse.
//!
//! Stronger than `self_host_smoke.rs` (which only parses). Catches
//! regressions where a scaffold parses but breaks planning or codegen
//! (e.g. references an unknown type name, uses a primitive the
//! planner can't lower).
//!
//! Whole-file Windows-gated: nom-llvm links LLVM-C.dll at runtime
//! and the test exe fails to start (STATUS_DLL_NOT_FOUND) before
//! #[ignore] can skip anything. Linux CI runs it.

#![cfg(not(windows))]

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

fn pipeline_compile(source: &str) -> Result<Vec<u8>, String> {
    let sf = nom_parser::parse_source(source).map_err(|e| format!("parse: {e}"))?;
    let resolver = nom_resolver::Resolver::open_in_memory()
        .map_err(|e| format!("resolver: {e}"))?;
    let planner = nom_planner::Planner::new(&resolver);
    let plan = planner.plan_unchecked(&sf).map_err(|e| format!("plan: {e}"))?;
    let output = nom_llvm::compile(&plan).map_err(|e| format!("codegen: {e}"))?;
    Ok(output.bitcode)
}

#[test]
fn every_self_host_nom_file_compiles_to_bc() {
    let dir = self_host_dir();
    let entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()));

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

    let mut failures: Vec<(PathBuf, String)> = Vec::new();
    let mut successes: usize = 0;
    for path in &nom_files {
        let src = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        match pipeline_compile(&src) {
            Ok(bc) => {
                assert!(
                    !bc.is_empty(),
                    "{}: pipeline succeeded but returned empty bitcode",
                    path.display()
                );
                successes += 1;
            }
            Err(e) => failures.push((path.clone(), e)),
        }
    }

    if !failures.is_empty() {
        let msg = failures
            .iter()
            .map(|(p, e)| format!("  {}: {e}", p.display()))
            .collect::<Vec<_>>()
            .join("\n");
        panic!(
            "{} of {} self-host scaffolds failed pipeline compile:\n{}",
            failures.len(),
            nom_files.len(),
            msg
        );
    }

    assert!(
        successes >= 5,
        "expected ≥5 scaffolds to round-trip, got {successes}"
    );
}
