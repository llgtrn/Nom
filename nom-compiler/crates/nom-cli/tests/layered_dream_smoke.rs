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
        let (code, stdout, stderr) =
            run_nom(&["app", "dream", fake_hash, "--tier", "app", "--json"]);
        // Non-epic (empty dict) exits 2; that's fine.
        assert!(
            code == 0 || code == 2,
            "expected exit 0 or 2 for tier=app, got {code}\nstdout={stdout}\nstderr={stderr}"
        );
        let v: serde_json::Value =
            serde_json::from_str(&stdout).expect("--json output must be valid JSON");
        assert_eq!(v["tier"], "app", "JSON must contain tier=app, got: {v}");
        // Verify shape of a LayeredDreamReport.
        assert!(v["leaf"].is_object(), "must have a 'leaf' object");
        assert!(
            v["child_reports"].is_array(),
            "must have 'child_reports' array"
        );
        assert!(
            v["pareto_front"].is_array(),
            "must have 'pareto_front' array"
        );
    }

    /// `nom app dream <hash> --tier concept` (no --target-id) → exit 1 + error message.
    #[test]
    fn tier_concept_without_target_id_exits_1() {
        let fake_hash = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let (code, _stdout, stderr) = run_nom(&["app", "dream", fake_hash, "--tier", "concept"]);
        assert_eq!(
            code, 1,
            "expected exit 1 for missing --target-id\nstderr={stderr}"
        );
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
            "app",
            "dream",
            fake_hash,
            "--tier",
            "module",
            "--target-id",
            fake_hash,
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
        let (code, _stdout, stderr) = run_nom(&["app", "dream", fake_hash, "--tier", "bogus"]);
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

    // ── M5b: --repo-id smoke tests ────────────────────────────────────────────

    /// `nom app dream <hash> --tier app --repo-id <fake> --json` with no dict
    /// should exit 0 or 2 (non-epic is fine), parse as valid JSON, and have
    /// `child_reports` as an empty array (the repo doesn't exist in the in-memory
    /// dict, so the graph is empty / no concepts to recurse into).
    ///
    /// Note: when `--dict` doesn't exist, the CLI uses an in-memory dict.
    /// `materialize_concept_graph_from_db` on an in-memory dict with an unknown
    /// repo_id returns an empty graph (not an error), so child_reports is [].
    #[test]
    fn repo_id_with_unknown_repo_returns_empty_child_reports() {
        let fake_hash = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
        let (code, stdout, stderr) = run_nom(&[
            "app",
            "dream",
            fake_hash,
            "--tier",
            "app",
            "--repo-id",
            "nonexistent_repo_xyz",
            "--json",
        ]);
        assert!(
            code == 0 || code == 2,
            "expected exit 0 or 2 for tier=app with unknown repo-id, got {code}\nstdout={stdout}\nstderr={stderr}"
        );
        let v: serde_json::Value =
            serde_json::from_str(&stdout).expect("--json output must be valid JSON");
        assert_eq!(v["tier"], "app", "JSON must contain tier=app, got: {v}");
        let child_reports = v["child_reports"]
            .as_array()
            .expect("child_reports must be an array");
        assert!(
            child_reports.is_empty(),
            "child_reports must be empty for an unknown repo (empty graph), got: {child_reports:?}"
        );
        // M7c: root report must carry me_collisions and ce_unmet fields (both empty at root).
        assert!(
            v["me_collisions"].is_array(),
            "JSON must contain 'me_collisions' array (M7c), got: {v}"
        );
        assert!(
            v["ce_unmet"].is_array(),
            "JSON must contain 'ce_unmet' array (M7c), got: {v}"
        );
        assert!(
            v["me_collisions"].as_array().unwrap().is_empty(),
            "root me_collisions must be empty (no parent for app tier), got: {:?}",
            v["me_collisions"]
        );
        assert!(
            v["ce_unmet"].as_array().unwrap().is_empty(),
            "root ce_unmet must be empty (no parent for app tier), got: {:?}",
            v["ce_unmet"]
        );
        // stderr should be clean (no error for an unknown repo that returns an empty graph).
        assert!(
            stderr.is_empty(),
            "stderr must be empty for an unknown repo in an in-memory dict, got: {stderr}"
        );
    }

    // ── M5c: --pareto-front smoke test ────────────────────────────────────────

    /// `nom app dream <hash> --tier app --pareto-front` exits 2 (non-epic, empty
    /// dict) and stdout contains either "Pareto front" or "Pareto front: empty".
    #[test]
    fn pareto_front_flag_prints_pareto_section() {
        let fake_hash = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        let (code, stdout, stderr) =
            run_nom(&["app", "dream", fake_hash, "--tier", "app", "--pareto-front"]);
        assert_eq!(
            code, 2,
            "expected exit 2 (non-epic with empty dict), got {code}\nstdout={stdout}\nstderr={stderr}"
        );
        assert!(
            stdout.contains("Pareto front"),
            "expected 'Pareto front' in stdout, got: {stdout}\nstderr={stderr}"
        );
    }
}
