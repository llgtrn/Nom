//! M10b: run_lexer.nom must produce byte-deterministic .bc output.
//!
//! Linux-gated (LLVM DLL issues on Windows).  On Linux CI, this
//! compiles examples/run_lexer.nom via the shipped compiler and
//! asserts the SHA-256 of the produced bytes matches the pinned
//! golden hash.  If the hash drifts, the committer MUST either
//! (a) fix whatever made it non-deterministic, or (b) update the
//! golden hash in this file, explaining WHY in the commit message.
//!
//! Doc 04 §10.3.1 fixpoint prerequisite at the smallest scale.
//!
//! Bootstrap note: the pinned hash was computed from the local
//! run_lexer.bc artifact present on the dev machine (Windows).
//! M10a's commit message confirmed that artifact was built after the
//! last edit to run_lexer.nom and was not stale.  Linux CI will
//! verify the hash on first run; if it drifts, update it here and
//! explain the drift in the commit message.

#[cfg(not(windows))]
mod tests {
    use sha2::{Digest, Sha256};
    use std::path::PathBuf;
    use std::process::Command;

    /// SHA-256 of examples/run_lexer.bc produced by the current compiler.
    ///
    /// Bootstrap source: local artifact confirmed non-stale by M10a
    /// (built after last edit to run_lexer.nom, pre-M10a codegen unchanged).
    /// To update: re-run `nom build compile --target llvm examples/run_lexer.nom`
    /// on Linux, compute `sha256sum examples/run_lexer.bc`, and replace below.
    const EXPECTED_RUN_LEXER_BC_SHA256: &str =
        "085e8fa62568e65744d3f436012618cf6e656e05b29ae79f8107aa8702583296";

    fn repo_root() -> PathBuf {
        // This file is at nom-compiler/crates/nom-cli/tests/
        // Root is three parents up.
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
    fn run_lexer_bc_is_byte_deterministic() {
        let nom_compiler = repo_root().join("nom-compiler");
        let source = nom_compiler.join("examples/run_lexer.nom");
        assert!(source.exists(), "missing source: {}", source.display());

        // Compile to a temp copy of the source so we don't clobber the
        // checked-in examples/run_lexer.bc (which is git-ignored but
        // present as a reference artifact).
        let tmp_dir = std::env::temp_dir().join(format!("nom-m10b-{}", std::process::id()));
        std::fs::create_dir_all(&tmp_dir).expect("create temp dir");
        let tmp_source = tmp_dir.join("run_lexer.nom");
        std::fs::copy(&source, &tmp_source).unwrap_or_else(|e| panic!("copy source to tmp: {e}"));

        let bin = env!("CARGO_BIN_EXE_nom");
        // CLI form: `nom build compile --target llvm <file>`
        // The compiler writes <file>.with_extension("bc") next to the source.
        let out = Command::new(bin)
            .args([
                "build",
                "compile",
                "--target",
                "llvm",
                tmp_source.to_str().unwrap(),
            ])
            .output()
            .expect("run nom build compile");
        assert!(
            out.status.success(),
            "nom build compile failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );

        // The compiler writes the .bc next to the source file.
        let bc_path = tmp_source.with_extension("bc");
        let bytes =
            std::fs::read(&bc_path).unwrap_or_else(|e| panic!("read {}: {e}", bc_path.display()));

        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let actual = format!("{:x}", hasher.finalize());

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp_dir);

        assert_eq!(
            actual, EXPECTED_RUN_LEXER_BC_SHA256,
            "run_lexer.bc hash drift — either (a) fix non-determinism, \
             or (b) update EXPECTED_RUN_LEXER_BC_SHA256 to {actual} and \
             explain the drift in the commit message."
        );
    }
}
