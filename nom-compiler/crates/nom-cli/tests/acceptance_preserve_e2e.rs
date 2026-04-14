//! End-to-end test for `nom build verify-acceptance`.
//!
//! Exercises the full structural preservation pipeline:
//!   1. Sync agent_demo into a temp dict.
//!   2. Run `nom build report --format json --out baseline.json`.
//!   3. Modify agent.nom to DROP one predicate.
//!   4. Re-sync + run `nom build verify-acceptance --prior baseline.json`.
//!   5. Assert exit 1 + stdout contains "Dropped" + the dropped predicate text.
//!   6. Revert the modification; re-sync + re-run verify-acceptance.
//!   7. Assert exit 0.
//!
//! Gated to `#[cfg(not(windows))]` — same reason as agent_demo_e2e.rs
//! (LLVM-C.dll unavailable on Windows CI).

#[cfg(not(windows))]
mod tests {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn make_tmpdir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("nom-accept-{tag}-{pid}-{nanos}"));
        std::fs::create_dir_all(&dir).expect("create tmp");
        dir
    }

    fn nom_bin() -> PathBuf {
        PathBuf::from(env!("CARGO_BIN_EXE_nom"))
    }

    fn dict_flag(root: &Path) -> String {
        root.join("nomdict.db").to_string_lossy().into_owned()
    }

    fn run_sync(repo: &Path, dict_root: &Path) -> (i32, String, String) {
        let out = Command::new(nom_bin())
            .args([
                "store",
                "sync",
                &repo.to_string_lossy(),
                "--dict",
                &dict_flag(dict_root),
            ])
            .output()
            .expect("spawn nom store sync");
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }

    fn run_build_report(repo: &Path, dict_root: &Path, out_file: &Path) -> (i32, String, String) {
        let out = Command::new(nom_bin())
            .args([
                "build",
                "report",
                &repo.to_string_lossy(),
                "--dict",
                &dict_flag(dict_root),
                "--format",
                "json",
                "--out",
                &out_file.to_string_lossy(),
            ])
            .output()
            .expect("spawn nom build report");
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }

    fn run_verify_acceptance(repo: &Path, dict_root: &Path, prior: &Path) -> (i32, String, String) {
        let out = Command::new(nom_bin())
            .args([
                "build",
                "verify-acceptance",
                &repo.to_string_lossy(),
                "--dict",
                &dict_flag(dict_root),
                "--prior",
                &prior.to_string_lossy(),
            ])
            .output()
            .expect("spawn nom build verify-acceptance");
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }

    fn copy_dir_all(src: &Path, dst: &Path) {
        std::fs::create_dir_all(dst).expect("create dst dir");
        for entry in std::fs::read_dir(src).expect("read src dir") {
            let entry = entry.expect("dir entry");
            let ty = entry.file_type().expect("file_type");
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            if ty.is_dir() {
                copy_dir_all(&src_path, &dst_path);
            } else {
                std::fs::copy(&src_path, &dst_path).expect("copy file");
            }
        }
    }

    fn agent_demo_src() -> PathBuf {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest
            .join("..") // crates/
            .join("..") // nom-compiler/
            .join("examples")
            .join("agent_demo")
    }

    // ── The predicate we will drop and restore ───────────────────────────────

    /// Text of the predicate line we remove in step 3.
    const DROPPED_PREDICATE: &str =
        "  this works when every exposed tool has at least one require clause.";

    // ── Main e2e test ─────────────────────────────────────────────────────────

    #[test]
    fn acceptance_preservation_drop_then_restore() {
        let src = agent_demo_src();
        assert!(src.exists(), "agent_demo not found at {}", src.display());

        // ── Step 1: copy + sync ───────────────────────────────────────────────
        let repo_dir = make_tmpdir("repo");
        let dict_dir = make_tmpdir("dict");
        copy_dir_all(&src, &repo_dir);

        let (sc, so, se) = run_sync(&repo_dir, &dict_dir);
        assert_eq!(sc, 0, "initial sync failed: stderr={se}\nstdout={so}");

        // ── Step 2: generate baseline report ──────────────────────────────────
        let baseline = dict_dir.join("baseline.json");
        let (rc, _ro, re) = run_build_report(&repo_dir, &dict_dir, &baseline);
        // Exit 1 is OK here (agent_demo has MECE violations); we just need the
        // JSON file to exist and be parseable.
        assert!(rc == 0 || rc == 1, "build report exited {rc}: stderr={re}");
        assert!(
            baseline.exists(),
            "baseline.json must exist after --out write"
        );

        // Sanity: the baseline must contain the predicate we are about to drop.
        let baseline_text = std::fs::read_to_string(&baseline).expect("read baseline");
        assert!(
            baseline_text.contains("every exposed tool has at least one require clause"),
            "baseline must contain the target predicate: {baseline_text}"
        );

        // ── Step 3: drop one predicate ────────────────────────────────────────
        let agent_nom = repo_dir.join("agent.nom");
        let original = std::fs::read_to_string(&agent_nom).expect("read agent.nom");
        let modified = original
            .lines()
            .filter(|l| l.trim() != DROPPED_PREDICATE.trim())
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert_ne!(
            original, modified,
            "modified source must differ from original (predicate not found?)"
        );
        std::fs::write(&agent_nom, &modified).expect("write modified agent.nom");

        // Re-sync after modification so the DB reflects the current source.
        let (sc2, so2, se2) = run_sync(&repo_dir, &dict_dir);
        assert_eq!(
            sc2, 0,
            "re-sync after modification failed: stderr={se2}\nstdout={so2}"
        );

        // ── Step 4: verify-acceptance must exit 1 ────────────────────────────
        let (vc, vo, ve) = run_verify_acceptance(&repo_dir, &dict_dir, &baseline);
        assert_eq!(
            vc, 1,
            "expected exit 1 when predicate dropped: stderr={ve}\nstdout={vo}"
        );

        // ── Step 5: stdout must mention "Dropped" + the dropped predicate ─────
        assert!(
            vo.contains("Dropped"),
            "stdout must contain 'Dropped': {vo}"
        );
        assert!(
            vo.contains("every exposed tool has at least one require clause"),
            "stdout must mention the dropped predicate text: {vo}"
        );

        // ── Step 6: revert the modification ──────────────────────────────────
        std::fs::write(&agent_nom, &original).expect("restore agent.nom");

        // Re-sync after revert.
        let (sc3, so3, se3) = run_sync(&repo_dir, &dict_dir);
        assert_eq!(
            sc3, 0,
            "re-sync after revert failed: stderr={se3}\nstdout={so3}"
        );

        // ── Step 7: verify-acceptance must exit 0 ────────────────────────────
        let (vc2, vo2, ve2) = run_verify_acceptance(&repo_dir, &dict_dir, &baseline);
        assert_eq!(
            vc2, 0,
            "expected exit 0 when predicate restored: stderr={ve2}\nstdout={vo2}"
        );
    }
}
