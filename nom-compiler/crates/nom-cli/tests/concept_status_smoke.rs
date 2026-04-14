//! Smoke tests for `nom build status`.
//!
//! Gated to `#[cfg(not(windows))]` because the `nom` binary links `nom-llvm`
//! which requires LLVM-C.dll on Windows; the DLL is absent in typical
//! Windows dev/CI environments (same gate as `store_sync_smoke.rs`).
//!
//! Test plan:
//!   1. Empty closure — concept with zero index clauses → 0 words, 0 unresolved.
//!   2. Resolved hash — .nomtu synced first; .nom references the resulting hash
//!      via a resolved EntityRef; status reports 1 word resolved.
//!   3. Prose-matching — .nomtu synced; .nom references the word by name only
//!      (no @hash); stub resolver picks the only matching word.
//!   4. Ambiguous matching — two .nomtu files each declare the same word name
//!      (different content → different hashes); status resolves to alphabetically
//!      smallest hash and surfaces alternatives count = 1.
//!   5. Unknown concept name (`--concept does_not_exist`) → exit 1 with diagnostic.

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
        let dir = std::env::temp_dir().join(format!("nom-bstatus-{tag}-{pid}-{nanos}"));
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
            .expect("spawn nom");
        let code = out.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        (code, stdout, stderr)
    }

    fn run_status(repo: &Path, dict_root: &Path, concept: Option<&str>) -> (i32, String, String) {
        let mut args = vec![
            "build".to_string(),
            "status".to_string(),
            repo.to_string_lossy().into_owned(),
            "--dict".to_string(),
            dict_flag(dict_root),
        ];
        if let Some(name) = concept {
            args.push("--concept".to_string());
            args.push(name.to_string());
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

    fn open_dict(root: &Path) -> NomDict {
        NomDict::open(root).expect("open NomDict")
    }

    // ── Test 1: empty closure ────────────────────────────────────────────────

    #[test]
    fn status_empty_concept_reports_zero_words_and_clear() {
        let repo_dir = make_tmpdir("t1");
        let dict_dir = make_tmpdir("t1-d");

        // A .nom with one concept that has zero index clauses.
        let nom_src = r#"
the concept empty_concept is
  intended to demonstrate zero deps.

  exposes nothing.
"#;
        std::fs::write(repo_dir.join("empty.nom"), nom_src).expect("write");

        let (sc, _, se) = run_sync(&repo_dir, &dict_dir);
        assert_eq!(sc, 0, "sync failed: {se}");

        let (code, stdout, stderr) = run_status(&repo_dir, &dict_dir, None);
        assert_eq!(code, 0, "expected exit 0, stderr={stderr}\nstdout={stdout}");
        assert!(
            stdout.contains("empty_concept"),
            "expected concept name in output: {stdout}"
        );
        assert!(
            stdout.contains("all clear"),
            "expected 'all clear' for zero-dep concept: {stdout}"
        );
        // 0 resolved words.
        assert!(
            stdout.contains("words resolved: 0"),
            "expected 0 resolved words: {stdout}"
        );
    }

    // ── Test 2: resolved hash ─────────────────────────────────────────────────

    #[test]
    fn status_concept_with_resolved_hash_reports_one_word_resolved() {
        let repo_dir = make_tmpdir("t2");
        let dict_dir = make_tmpdir("t2-d");

        // 1. Sync the .nomtu so the hash lands in entities.
        let nomtu_src = "the function foo_compute is\n  given x of text, returns y of text.\n";
        std::fs::write(repo_dir.join("util.nomtu"), nomtu_src).expect("write nomtu");

        let (sc, _, se) = run_sync(&repo_dir, &dict_dir);
        assert_eq!(sc, 0, "sync nomtu failed: {se}");

        // Retrieve the hash that was assigned.
        let dict = open_dict(&dict_dir);
        let rows = dict
            .find_entities_by_word("foo_compute")
            .expect("find entities");
        assert_eq!(rows.len(), 1, "expected 1 row for foo_compute");
        let foo_hash = &rows[0].hash;

        // 2. Write a .nom that references foo_compute with a resolved @hash.
        let nom_src = format!(
            r#"
the concept compute_concept is
  intended to use the foo_compute word.

  uses the function foo_compute@{foo_hash}.
"#
        );
        std::fs::write(repo_dir.join("compute.nom"), nom_src).expect("write nom");

        let (sc2, _, se2) = run_sync(&repo_dir, &dict_dir);
        assert_eq!(sc2, 0, "sync nom failed: {se2}");

        let (code, stdout, stderr) = run_status(&repo_dir, &dict_dir, None);
        // The resolved hash is already in word_hashes (closure walker puts it
        // directly into word_hashes, not unresolved).  Status should report clean.
        assert_eq!(code, 0, "expected exit 0, stderr={stderr}\nstdout={stdout}");
        assert!(
            stdout.contains("compute_concept"),
            "expected compute_concept in output: {stdout}"
        );
        assert!(
            stdout.contains("all clear"),
            "expected all clear (resolved hash has no unresolved refs): {stdout}"
        );
    }

    // ── Test 3: prose-matching (stub resolves by word name) ───────────────────

    #[test]
    fn status_concept_with_prose_ref_resolves_by_word_name() {
        let repo_dir = make_tmpdir("t3");
        let dict_dir = make_tmpdir("t3-d");

        // 1. Sync .nomtu so `login_verify` is in entities.
        let nomtu_src = "the function login_verify is\n  given credentials, returns yes or no.\n";
        std::fs::write(repo_dir.join("login.nomtu"), nomtu_src).expect("write nomtu");

        let (sc, _, se) = run_sync(&repo_dir, &dict_dir);
        assert_eq!(sc, 0, "sync nomtu failed: {se}");

        // 2. Write a .nom with an unresolved ref (no @hash) — the stub resolver
        //    should find `login_verify` in entities by word name.
        let nom_src = r#"
the concept auth_concept is
  intended to verify user credentials.

  uses the function login_verify matching "verifies".
"#;
        std::fs::write(repo_dir.join("auth.nom"), nom_src).expect("write nom");

        let (sc2, _, se2) = run_sync(&repo_dir, &dict_dir);
        assert_eq!(sc2, 0, "sync nom failed: {se2}");

        let (code, stdout, stderr) = run_status(&repo_dir, &dict_dir, None);
        assert_eq!(
            code, 0,
            "expected exit 0 (stub resolves by word name), stderr={stderr}\nstdout={stdout}"
        );
        assert!(
            stdout.contains("auth_concept"),
            "expected auth_concept: {stdout}"
        );
        assert!(stdout.contains("all clear"), "expected all clear: {stdout}");
        // Exactly 1 word resolved (no alternatives).
        assert!(
            stdout.contains("words resolved: 1"),
            "expected 1 resolved word: {stdout}"
        );
    }

    // ── Test 4: ambiguous matching ────────────────────────────────────────────

    #[test]
    fn status_concept_with_ambiguous_word_picks_smallest_hash_and_surfaces_alternatives() {
        let repo_dir = make_tmpdir("t4");
        let dict_dir = make_tmpdir("t4-d");

        // Two .nomtu files each declare a function named `login_ambiguous`
        // but with different signatures → different hashes.
        let nomtu_a = "the function login_ambiguous is\n  given user_a, returns yes or no.\n";
        let nomtu_b = "the function login_ambiguous is\n  given user_b, returns yes or no.\n";
        std::fs::write(repo_dir.join("login_a.nomtu"), nomtu_a).expect("write a");
        std::fs::write(repo_dir.join("login_b.nomtu"), nomtu_b).expect("write b");

        let (sc, _, se) = run_sync(&repo_dir, &dict_dir);
        assert_eq!(sc, 0, "sync failed: {se}");

        // Verify both landed.
        let dict = open_dict(&dict_dir);
        let rows = dict
            .find_entities_by_word("login_ambiguous")
            .expect("find entities");
        assert_eq!(rows.len(), 2, "expected 2 rows for login_ambiguous");

        // Write a .nom referencing `login_ambiguous` by name only.
        let nom_src = r#"
the concept ambiguous_concept is
  intended to demonstrate ambiguous resolution.

  uses the function login_ambiguous matching "any login".
"#;
        std::fs::write(repo_dir.join("ambig.nom"), nom_src).expect("write nom");

        let (sc2, _, se2) = run_sync(&repo_dir, &dict_dir);
        assert_eq!(sc2, 0, "sync nom failed: {se2}");

        let (code, stdout, stderr) = run_status(&repo_dir, &dict_dir, None);
        // Should exit 0 — ambiguous resolves to smallest hash, not an error.
        assert_eq!(
            code, 0,
            "expected exit 0 (ambiguous → picked smallest hash), stderr={stderr}\nstdout={stdout}"
        );
        assert!(
            stdout.contains("ambiguous_concept"),
            "expected ambiguous_concept in output: {stdout}"
        );
        assert!(
            stdout.contains("ambiguous"),
            "expected 'ambiguous' in output to surface alternatives: {stdout}"
        );
        assert!(stdout.contains("all clear"), "expected all clear: {stdout}");
    }

    // ── Test 5: unknown concept name → exit 1 ────────────────────────────────

    #[test]
    fn status_unknown_concept_name_exits_one_with_diagnostic() {
        let repo_dir = make_tmpdir("t5");
        let dict_dir = make_tmpdir("t5-d");

        // Sync one real concept so the repo is non-empty.
        let nom_src = r#"
the concept real_concept is
  intended to be the only one.
"#;
        std::fs::write(repo_dir.join("real.nom"), nom_src).expect("write");

        let (sc, _, se) = run_sync(&repo_dir, &dict_dir);
        assert_eq!(sc, 0, "sync failed: {se}");

        let (code, _stdout, stderr) = run_status(&repo_dir, &dict_dir, Some("does_not_exist"));
        assert_eq!(code, 1, "expected exit 1 for unknown concept");
        assert!(
            stderr.contains("does_not_exist"),
            "expected concept name in error: {stderr}"
        );
    }
}
