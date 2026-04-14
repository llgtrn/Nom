//! End-to-end test for `nom build report <repo>`.
//!
//! Exercises the full pipeline:
//!   1. Sync agent_demo into a temp dict.
//!   2. Run `nom build report <tempdir> --format json`; parse ReportBundle;
//!      assert schema_version == 1, at least 2 concepts, at least one MECE
//!      collision, OverallVerdict::NeedsAttention.
//!   3. Same run with `--format human`; assert stdout contains key strings.
//!   4. `--out report.json` writes the file; re-read + parse succeeds.
//!   5. `--concept minimal_safe_agent` filters to just that one concept.
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
        let dir = std::env::temp_dir().join(format!("nom-report-{tag}-{pid}-{nanos}"));
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

    fn run_report(
        repo: &Path,
        dict_root: &Path,
        format: &str,
        out_file: Option<&Path>,
        concept_filter: Option<&str>,
    ) -> (i32, String, String) {
        let mut args = vec![
            "build".to_string(),
            "report".to_string(),
            repo.to_string_lossy().into_owned(),
            "--dict".to_string(),
            dict_flag(dict_root),
            "--format".to_string(),
            format.to_string(),
        ];
        if let Some(p) = out_file {
            args.push("--out".to_string());
            args.push(p.to_string_lossy().into_owned());
        }
        if let Some(c) = concept_filter {
            args.push("--concept".to_string());
            args.push(c.to_string());
        }
        let out = Command::new(nom_bin())
            .args(&args)
            .output()
            .expect("spawn nom build report");
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
            .join("..") // crates
            .join("..") // nom-compiler
            .join("examples")
            .join("agent_demo")
    }

    // ── sync once, reuse across subtests ─────────────────────────────────────

    fn setup() -> (PathBuf, PathBuf) {
        let src = agent_demo_src();
        let repo_dir = make_tmpdir("repo");
        let dict_dir = make_tmpdir("dict");
        copy_dir_all(&src, &repo_dir);
        let (sc, so, se) = run_sync(&repo_dir, &dict_dir);
        assert_eq!(sc, 0, "sync failed: stderr={se}\nstdout={so}");
        (repo_dir, dict_dir)
    }

    // ── Test 1: JSON format — structural assertions ───────────────────────────

    #[test]
    fn build_report_json_structural_assertions() {
        let src = agent_demo_src();
        assert!(src.exists(), "agent_demo not found at {}", src.display());

        let (repo_dir, dict_dir) = setup();

        let (rc, ro, re) = run_report(&repo_dir, &dict_dir, "json", None, None);

        // Exit 1 because there are MECE violations and/or unresolved slots.
        assert_eq!(
            rc, 1,
            "expected exit 1 (NeedsAttention): stderr={re}\nstdout={ro}"
        );

        // Must parse as JSON.
        let v: serde_json::Value =
            serde_json::from_str(&ro).expect("stdout must be valid JSON for --format json");

        // schema_version == 1
        assert_eq!(
            v["schema_version"].as_u64().unwrap_or(0),
            1,
            "schema_version must be 1"
        );

        // At least 2 concepts.
        let concepts = v["concepts"].as_array().expect("concepts must be array");
        assert!(
            concepts.len() >= 2,
            "expected at least 2 concepts, got {}",
            concepts.len()
        );

        // At least one concept has a MECE collision.
        let has_mece = concepts.iter().any(|c| {
            c["mece"]["me_collisions"]
                .as_array()
                .map(|a| !a.is_empty())
                .unwrap_or(false)
        });
        assert!(
            has_mece,
            "expected at least one MECE ME collision in the report"
        );

        // OverallVerdict::NeedsAttention
        let overall_kind = v["overall"]["kind"].as_str().unwrap_or("");
        assert_eq!(
            overall_kind, "needs_attention",
            "expected overall.kind == needs_attention, got {overall_kind}"
        );

        // overall.reasons must be non-empty
        let reasons = v["overall"]["reasons"]
            .as_array()
            .expect("reasons must be array");
        assert!(
            !reasons.is_empty(),
            "expected at least one reason in overall.reasons"
        );
    }

    // ── Test 2: human format — key string assertions ──────────────────────────

    #[test]
    fn build_report_human_format_key_strings() {
        let src = agent_demo_src();
        assert!(src.exists(), "agent_demo not found at {}", src.display());

        let (repo_dir, dict_dir) = setup();

        let (rc, ro, re) = run_report(&repo_dir, &dict_dir, "human", None, None);

        // Exit 1 (same as JSON).
        assert_eq!(
            rc, 1,
            "expected exit 1 (human format): stderr={re}\nstdout={ro}"
        );

        // Must contain "═══ concept"
        assert!(
            ro.contains("═══ concept"),
            "human output must contain '═══ concept': {ro}"
        );

        // Must contain "MECE:"
        assert!(
            ro.contains("MECE:"),
            "human output must contain 'MECE:': {ro}"
        );

        // Must contain "OVERALL:"
        assert!(
            ro.contains("OVERALL:"),
            "human output must contain 'OVERALL:': {ro}"
        );

        // Must contain "NEEDS ATTENTION"
        assert!(
            ro.contains("NEEDS ATTENTION"),
            "human output must contain 'NEEDS ATTENTION': {ro}"
        );
    }

    // ── Test 3: --out writes file; roundtrip succeeds ─────────────────────────

    #[test]
    fn build_report_out_file_roundtrip() {
        let src = agent_demo_src();
        assert!(src.exists(), "agent_demo not found at {}", src.display());

        let (repo_dir, dict_dir) = setup();

        let out_file = dict_dir.join("report.json");
        let (rc, ro, re) = run_report(&repo_dir, &dict_dir, "json", Some(&out_file), None);

        // Exit 1 (MECE violations).
        assert_eq!(rc, 1, "expected exit 1: stderr={re}\nstdout={ro}");

        // stdout must be empty when --out is used.
        assert!(
            ro.trim().is_empty(),
            "stdout must be empty when --out is set, got: {ro}"
        );

        // File must exist and parse.
        assert!(
            out_file.exists(),
            "report.json must exist after --out write"
        );

        let content = std::fs::read_to_string(&out_file).expect("read report.json");
        let v: serde_json::Value =
            serde_json::from_str(&content).expect("report.json must be valid JSON");

        assert_eq!(
            v["schema_version"].as_u64().unwrap_or(0),
            1,
            "roundtrip schema_version must be 1"
        );
    }

    // ── Test 4: --concept filter ──────────────────────────────────────────────

    #[test]
    fn build_report_concept_filter() {
        let src = agent_demo_src();
        assert!(src.exists(), "agent_demo not found at {}", src.display());

        let (repo_dir, dict_dir) = setup();

        let (rc, ro, re) = run_report(
            &repo_dir,
            &dict_dir,
            "json",
            None,
            Some("minimal_safe_agent"),
        );

        // Exit code depends on whether this one concept has issues.
        // We only assert it's 0 or 1 (not an error crash).
        assert!(
            rc == 0 || rc == 1,
            "expected exit 0 or 1 for --concept filter, got {rc}: stderr={re}\nstdout={ro}"
        );

        // Parse as JSON.
        let v: serde_json::Value =
            serde_json::from_str(&ro).expect("filtered report must be valid JSON");

        // Must have exactly 1 concept named minimal_safe_agent.
        let concepts = v["concepts"].as_array().expect("concepts array");
        assert_eq!(
            concepts.len(),
            1,
            "filtered report must have exactly 1 concept, got {}",
            concepts.len()
        );
        assert_eq!(
            concepts[0]["name"].as_str().unwrap_or(""),
            "minimal_safe_agent",
            "filtered concept must be minimal_safe_agent"
        );
    }
}
