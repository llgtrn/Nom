//! M5a smoke tests: layered dream tier scaffolding.
//!
//! Verifies the four CLI contracts added in M5a:
//!   1. `--tier app --json` emits a `LayeredDreamReport` with `"tier":"app"`.
//!   2. `--tier concept` without `--target-id` exits 1 with the expected error.
//!   3. `--tier module --target-id <hash>` exits 2 with "not yet implemented".
//!   4. `--tier bogus` exits 1 with the unknown-tier error.
//!
//! Gated to `#[cfg(not(windows))]` because the `nom` binary links `nom-llvm`
//! which requires LLVM-C.dll on Windows; the DLL is absent in typical
//! Windows dev/CI environments (same gate as `concept_status_smoke.rs`).

#[cfg(not(windows))]
mod tests {
    use std::path::PathBuf;
    use std::process::Command;

    fn nom_bin() -> PathBuf {
        PathBuf::from(env!("CARGO_BIN_EXE_nom"))
    }

    fn run_nom(args: &[&str]) -> (i32, String, String) {
        let out = Command::new(nom_bin())
            .args(args)
            .output()
            .expect("spawn nom");
        let code = out.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        (code, stdout, stderr)
    }

    /// `nom app dream <hash> --tier app --json` → JSON containing `"tier":"app"`.
    #[test]
    fn tier_app_json_contains_tier_field() {
        let fake_hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let (code, stdout, stderr) = run_nom(&[
            "app", "dream", fake_hash,
            "--tier", "app",
            "--json",
        ]);
        // Non-epic (empty dict) exits 2; that's fine.
        assert!(
            code == 0 || code == 2,
            "expected exit 0 or 2 for tier=app, got {code}\nstdout={stdout}\nstderr={stderr}"
        );
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .expect("--json output must be valid JSON");
        assert_eq!(
            v["tier"], "app",
            "JSON must contain tier=app, got: {v}"
        );
        // Verify shape of a LayeredDreamReport.
        assert!(v["leaf"].is_object(), "must have a 'leaf' object");
        assert!(v["child_reports"].is_array(), "must have 'child_reports' array");
        assert!(v["pareto_front"].is_array(), "must have 'pareto_front' array");
    }

    /// `nom app dream <hash> --tier concept` (no --target-id) → exit 1 + error message.
    #[test]
    fn tier_concept_without_target_id_exits_1() {
        let fake_hash = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let (code, _stdout, stderr) = run_nom(&[
            "app", "dream", fake_hash,
            "--tier", "concept",
        ]);
        assert_eq!(code, 1, "expected exit 1 for missing --target-id\nstderr={stderr}");
        assert!(
            stderr.contains("--target-id required for tier=concept"),
            "expected --target-id error in stderr, got: {stderr}"
        );
    }

    /// `nom app dream <hash> --tier module --target-id <hash>` → exit 2 + "not yet implemented".
    #[test]
    fn tier_module_exits_2_with_not_yet_implemented() {
        let fake_hash = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
        let (code, _stdout, stderr) = run_nom(&[
            "app", "dream", fake_hash,
            "--tier", "module",
            "--target-id", fake_hash,
        ]);
        assert_eq!(code, 2, "expected exit 2 for module tier\nstderr={stderr}");
        assert!(
            stderr.contains("not yet implemented"),
            "expected 'not yet implemented' in stderr, got: {stderr}"
        );
        assert!(
            stderr.contains("M5b"),
            "expected M5b mention in stderr, got: {stderr}"
        );
    }

    /// `nom app dream <hash> --tier bogus` → exit 1 + unknown-tier error.
    #[test]
    fn tier_bogus_exits_1_with_unknown_tier_error() {
        let fake_hash = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
        let (code, _stdout, stderr) = run_nom(&[
            "app", "dream", fake_hash,
            "--tier", "bogus",
        ]);
        assert_eq!(code, 1, "expected exit 1 for unknown tier\nstderr={stderr}");
        assert!(
            stderr.contains("unknown dream tier"),
            "expected 'unknown dream tier' in stderr, got: {stderr}"
        );
        assert!(
            stderr.contains("bogus"),
            "expected the bad tier name echoed in stderr, got: {stderr}"
        );
    }
}
