//! End-to-end test for the concept_demo example.
//!
//! Exercises the full pipeline:
//!   1. `nom store sync <tempdir>` — DB gets 2 concept_defs + 3 entities rows.
//!   2. `nom build status <tempdir>` — exit 0; auth_session_compose_demo resolved.
//!   3. auth/auth.nom must NOT have @hash yet (--write-locks not used yet).
//!   4. `nom build status <tempdir> --write-locks` — exit 0; "Wrote 1 hash lock".
//!   5. auth/auth.nom MUST have auth_session_compose_demo@<64-hex> after writeback.
//!   6. Second sync + status → still exit 0; 0 unresolved (idempotent).
//!
//! Gated to `#[cfg(not(windows))]` because the `nom` binary links `nom-llvm`
//! which requires LLVM-C.dll at start-up; the DLL is absent in typical
//! Windows dev/CI environments (same gate used in store_sync_smoke.rs).

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
        let dir = std::env::temp_dir().join(format!("nom-cdemo-{tag}-{pid}-{nanos}"));
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

    /// Path to the concept_demo example tree (relative to the workspace root).
    fn concept_demo_src() -> PathBuf {
        // CARGO_MANIFEST_DIR for nom-cli is
        //   nom-compiler/crates/nom-cli
        // The examples live at nom-compiler/examples/concept_demo.
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest
            .join("..") // nom-compiler/crates
            .join("..") // nom-compiler
            .join("examples")
            .join("concept_demo")
    }

    // ── Full pipeline test ───────────────────────────────────────────────────

    #[test]
    fn concept_demo_full_pipeline() {
        let src = concept_demo_src();
        assert!(
            src.exists(),
            "concept_demo source not found at {}",
            src.display()
        );

        let repo_dir = make_tmpdir("repo");
        let dict_dir = make_tmpdir("dict");

        // Copy the example tree into tempdir so write-locks can mutate it.
        copy_dir_all(&src, &repo_dir);

        // ── Step 1: sync ──────────────────────────────────────────────────────
        let (sc, so, se) = run_sync(&repo_dir, &dict_dir);
        assert_eq!(sc, 0, "sync failed: stderr={se}\nstdout={so}");

        // 2 concept_defs rows (authentication_demo + app_concept_demo).
        let dict = open_dict(&dict_dir);
        let auth_row = dict
            .find_concept_def("authentication_demo")
            .expect("query dict")
            .expect("authentication_demo must be in concept_defs");
        assert_eq!(auth_row.name, "authentication_demo");

        let app_row = dict
            .find_concept_def("app_concept_demo")
            .expect("query dict")
            .expect("app_concept_demo must be in concept_defs");
        assert_eq!(app_row.name, "app_concept_demo");

        // 3 entities rows (validate_token_demo, issue_session_demo, auth_session_compose_demo).
        let validate_rows = dict
            .find_entities_by_word("validate_token_demo")
            .expect("find validate_token_demo");
        assert_eq!(validate_rows.len(), 1, "expected 1 row for validate_token_demo");

        let issue_rows = dict
            .find_entities_by_word("issue_session_demo")
            .expect("find issue_session_demo");
        assert_eq!(issue_rows.len(), 1, "expected 1 row for issue_session_demo");

        let compose_rows = dict
            .find_entities_by_word("auth_session_compose_demo")
            .expect("find auth_session_compose_demo");
        assert_eq!(compose_rows.len(), 1, "expected 1 row for auth_session_compose_demo");

        // ── Step 2: build status (no --write-locks) ───────────────────────────
        let (bc, bo, be) = run_status(&repo_dir, &dict_dir, false);
        assert_eq!(bc, 0, "build status failed: stderr={be}\nstdout={bo}");
        assert!(
            bo.contains("auth_session_compose_demo"),
            "expected auth_session_compose_demo mentioned: {bo}"
        );

        // ── Step 3: auth.nom must NOT have @hash yet ──────────────────────────
        let auth_nom_path = repo_dir.join("auth").join("auth.nom");
        let auth_nom_before = std::fs::read_to_string(&auth_nom_path)
            .expect("read auth/auth.nom");
        assert!(
            !auth_nom_before.contains("auth_session_compose_demo@"),
            "auth.nom must not have @hash before --write-locks: {auth_nom_before}"
        );

        // ── Step 4: build status --write-locks ───────────────────────────────
        let (wc, wo, we) = run_status(&repo_dir, &dict_dir, true);
        assert_eq!(wc, 0, "build status --write-locks failed: stderr={we}\nstdout={wo}");
        assert!(
            wo.contains("Wrote") && wo.contains("hash lock"),
            "expected 'Wrote N hash lock' in output: {wo}"
        );

        // ── Step 5: auth.nom MUST have auth_session_compose_demo@<64-hex> ─────
        let auth_nom_after = std::fs::read_to_string(&auth_nom_path)
            .expect("read auth/auth.nom after write-locks");
        // Find the @ followed by exactly 64 hex characters.
        let at_pos = auth_nom_after.find("auth_session_compose_demo@");
        assert!(
            at_pos.is_some(),
            "auth_session_compose_demo@<hash> must be present after --write-locks: {auth_nom_after}"
        );
        let after_at = &auth_nom_after[at_pos.unwrap() + "auth_session_compose_demo@".len()..];
        let hash_part: String = after_at.chars().take(64).collect();
        assert_eq!(hash_part.len(), 64, "hash must be 64 chars: got `{hash_part}`");
        assert!(
            hash_part.chars().all(|c| c.is_ascii_hexdigit()),
            "hash must be hex: `{hash_part}`"
        );

        // ── Step 6: second sync + status → idempotent ────────────────────────
        let (sc2, so2, se2) = run_sync(&repo_dir, &dict_dir);
        assert_eq!(sc2, 0, "second sync failed: {se2}\n{so2}");

        let (bc2, bo2, be2) = run_status(&repo_dir, &dict_dir, false);
        assert_eq!(bc2, 0, "second status failed: {be2}\n{bo2}");
        // After write-locks, the ref is already pinned — no unresolved refs.
        assert!(
            bo2.contains("all clear") || bo2.contains("words resolved"),
            "expected clean status after lock writeback: {bo2}"
        );

        // Running --write-locks again must be idempotent (no additional insertions).
        let (wc2, wo2, we2) = run_status(&repo_dir, &dict_dir, true);
        assert_eq!(wc2, 0, "second --write-locks failed: {we2}\n{wo2}");
        // "Wrote 0 hash lock(s)" is acceptable — nothing new to write.
        let auth_nom_second = std::fs::read_to_string(&auth_nom_path)
            .expect("read auth/auth.nom second time");
        assert_eq!(
            auth_nom_after, auth_nom_second,
            "second --write-locks must not modify already-locked file"
        );
    }
}
