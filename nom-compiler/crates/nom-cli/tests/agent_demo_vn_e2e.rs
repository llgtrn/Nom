//! End-to-end test for the agent_demo_vn example (Vietnamese locale-pack).
//!
//! Validates motivation 02: Vietnamese ASCII keyword aliases lex identically
//! to their English counterparts, so the full pipeline works unchanged.
//!
//! Exercises the full pipeline:
//!   1. `nom store sync <tempdir>` — DB gets 2 concept_defs + 6 words_v2 rows.
//!   2. `nom build status <tempdir>` — exit 1 (MECE collision present);
//!      both concepts mentioned in stdout.
//!   3. `agent.nom` must NOT have any @hash yet (--write-locks not used yet).
//!   4. `nom build status <tempdir> --write-locks` — exit 1 (MECE unchanged)
//!      but lock writeback IS applied; stdout has "Wrote N hash lock(s)" N >= 6.
//!   5. `agent.nom` MUST have `read_file@<64-hex>` (and other tools) after writeback.
//!   6. Second sync + status → still exit 1 (MECE collision persists);
//!      idempotent: second --write-locks does not further modify source files.
//!
//! Gated to `#[cfg(not(windows))]` because the `nom` binary links `nom-llvm`
//! which requires LLVM-C.dll at start-up; the DLL is absent in typical
//! Windows dev/CI environments (same gate used in agent_demo_e2e.rs).

#[cfg(not(windows))]
mod tests {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use nom_dict::NomDict;

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn make_tmpdir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("nom-ademo-vn-{tag}-{pid}-{nanos}"));
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
        let code = out.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        (code, stdout, stderr)
    }

    fn run_status(
        repo: &Path,
        dict_root: &Path,
        write_locks: bool,
    ) -> (i32, String, String) {
        let mut args = vec![
            "build".to_string(),
            "status".to_string(),
            repo.to_string_lossy().into_owned(),
            "--dict".to_string(),
            dict_flag(dict_root),
        ];
        if write_locks {
            args.push("--write-locks".to_string());
        }
        let out = Command::new(nom_bin())
            .args(&args)
            .output()
            .expect("spawn nom build status");
        let code = out.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        (code, stdout, stderr)
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

    fn open_dict(root: &Path) -> NomDict {
        NomDict::open(root).expect("open NomDict")
    }

    /// Path to the agent_demo_vn example tree (relative to workspace root).
    fn agent_demo_vn_src() -> PathBuf {
        // CARGO_MANIFEST_DIR for nom-cli is
        //   nom-compiler/crates/nom-cli
        // The examples live at nom-compiler/examples/agent_demo_vn.
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest
            .join("..") // nom-compiler/crates
            .join("..") // nom-compiler
            .join("examples")
            .join("agent_demo_vn")
    }

    // ── Full pipeline test ───────────────────────────────────────────────────

    #[test]
    fn agent_demo_vn_full_pipeline() {
        let src = agent_demo_vn_src();
        assert!(
            src.exists(),
            "agent_demo_vn source not found at {}",
            src.display()
        );

        let repo_dir = make_tmpdir("repo");
        let dict_dir = make_tmpdir("dict");

        // Copy the example tree into tempdir so write-locks can mutate it.
        copy_dir_all(&src, &repo_dir);

        // ── Step 1: sync ──────────────────────────────────────────────────────
        let (sc, so, se) = run_sync(&repo_dir, &dict_dir);
        assert_eq!(sc, 0, "sync failed: stderr={se}\nstdout={so}");

        // 2 concept_defs rows (minimal_safe_agent + agent_safety_policy).
        let dict = open_dict(&dict_dir);
        let agent_row = dict
            .find_concept_def("minimal_safe_agent")
            .expect("query dict")
            .expect("minimal_safe_agent must be in concept_defs");
        assert_eq!(agent_row.name, "minimal_safe_agent");

        let policy_row = dict
            .find_concept_def("agent_safety_policy")
            .expect("query dict")
            .expect("agent_safety_policy must be in concept_defs");
        assert_eq!(policy_row.name, "agent_safety_policy");

        // 6 words_v2 rows — one per entity in the three tool .nomtu files.
        for word in &[
            "read_file",
            "write_file",
            "list_dir",
            "fetch_url",
            "search_web",
            "run_command",
        ] {
            let rows = dict
                .find_words_v2_by_word(word)
                .unwrap_or_else(|e| panic!("find_words_v2_by_word({word}) error: {e}"));
            assert_eq!(rows.len(), 1, "expected 1 row for {word}");
        }

        // ── Step 2: build status (no --write-locks) ───────────────────────────
        // MECE-ME violation is present (security + speed shared), so exit 1.
        let (bc, bo, be) = run_status(&repo_dir, &dict_dir, false);
        assert_eq!(
            bc, 1,
            "expected exit 1 due to MECE-ME violation: stderr={be}\nstdout={bo}"
        );
        assert!(
            bo.contains("minimal_safe_agent"),
            "expected minimal_safe_agent mentioned: {bo}"
        );
        assert!(
            bo.contains("agent_safety_policy"),
            "expected agent_safety_policy mentioned: {bo}"
        );
        // VN locale: MECE must fire (validator is keyword-agnostic).
        assert!(
            bo.contains("MECE"),
            "expected MECE violation in first status: {bo}"
        );
        assert!(
            bo.contains("security"),
            "expected 'security' axis in MECE output: {bo}"
        );
        assert!(
            bo.contains("speed"),
            "expected 'speed' axis in MECE output: {bo}"
        );

        // ── Step 3: agent.nom must NOT have @hash yet ─────────────────────────
        let agent_nom_path = repo_dir.join("agent.nom");
        let agent_nom_before = std::fs::read_to_string(&agent_nom_path).expect("read agent.nom");
        assert!(
            !agent_nom_before.contains("read_file@"),
            "agent.nom must not have @hash before --write-locks: {agent_nom_before}"
        );

        // ── Step 4: build status --write-locks ───────────────────────────────
        // Exits 1 due to MECE-ME violation; write-locks still applied.
        let (wc, wo, we) = run_status(&repo_dir, &dict_dir, true);
        assert_eq!(
            wc, 1,
            "expected exit 1 (MECE violation) from --write-locks: stderr={we}\nstdout={wo}"
        );
        assert!(
            wo.contains("Wrote") && wo.contains("hash lock"),
            "expected 'Wrote N hash lock' in output: {wo}"
        );
        // At minimum 7 locks: 6 tool refs in agent.nom + 1 read_file ref in safety.nom.
        let wrote_n = wo
            .lines()
            .find(|l| l.contains("Wrote") && l.contains("hash lock"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        assert!(
            wrote_n >= 6,
            "expected at least 6 hash locks written, got {wrote_n}: {wo}"
        );

        // ── Step 5: agent.nom MUST have read_file@<64-hex> after writeback ────
        // The lock writeback must understand Vietnamese-form lines:
        //   `cai ham read_file khop "..."` → `cai ham read_file@<hash> khop "..."`
        let agent_nom_after =
            std::fs::read_to_string(&agent_nom_path).expect("read agent.nom after write-locks");
        let at_pos = agent_nom_after.find("read_file@");
        assert!(
            at_pos.is_some(),
            "read_file@<hash> must be present in agent.nom after --write-locks.\n\
             This means apply_hash_locks must understand `cai ham <word>` lines.\n\
             File contents:\n{agent_nom_after}"
        );
        let after_at = &agent_nom_after[at_pos.unwrap() + "read_file@".len()..];
        let hash_part: String = after_at.chars().take(64).collect();
        assert_eq!(hash_part.len(), 64, "hash must be 64 chars: got `{hash_part}`");
        assert!(
            hash_part.chars().all(|c| c.is_ascii_hexdigit()),
            "hash must be hex: `{hash_part}`"
        );

        // ── Step 6: second sync + status → idempotent (resolver), MECE fails ──
        let (sc2, so2, se2) = run_sync(&repo_dir, &dict_dir);
        assert_eq!(sc2, 0, "second sync failed: {se2}\n{so2}");

        let (bc2, bo2, be2) = run_status(&repo_dir, &dict_dir, false);
        // MECE-ME violation: minimal_safe_agent composes agent_safety_policy and
        // both declare "security" and "speed" → exit 1.
        assert_eq!(
            bc2, 1,
            "expected exit 1 due to MECE-ME violation: {be2}\n{bo2}"
        );
        assert!(bo2.contains("MECE"), "expected 'MECE' in status output: {bo2}");
        assert!(
            bo2.contains("security"),
            "expected 'security' axis in MECE output: {bo2}"
        );
        assert!(
            bo2.contains("speed"),
            "expected 'speed' axis in MECE output: {bo2}"
        );
        assert!(
            bo2.contains("minimal_safe_agent"),
            "expected 'minimal_safe_agent' in MECE output: {bo2}"
        );
        assert!(
            bo2.contains("agent_safety_policy"),
            "expected 'agent_safety_policy' in MECE output: {bo2}"
        );

        // Running --write-locks again must be idempotent (no additional insertions).
        let (wc2, wo2, we2) = run_status(&repo_dir, &dict_dir, true);
        // MECE violation still present → exit 1.
        assert_eq!(
            wc2, 1,
            "expected exit 1 from --write-locks due to MECE violation: {we2}\n{wo2}"
        );
        let agent_nom_second =
            std::fs::read_to_string(&agent_nom_path).expect("read agent.nom second time");
        assert_eq!(
            agent_nom_after, agent_nom_second,
            "second --write-locks must not modify already-locked file"
        );
    }
}
