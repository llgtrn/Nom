//! Smoke tests for `nom store sync`.
//!
//! Tests that spawn the `nom` binary are gated to `#[cfg(not(windows))]`
//! because the binary links `nom-llvm` which requires LLVM-C.dll at
//! start-up; the DLL is absent in typical Windows dev/CI environments
//! (same gate used in `bc_body_round_trip.rs`).
//!
//! Five scenarios:
//!   1. Empty repo → 0 concepts, 0 words, exit 0.
//!   2. Single `.nomtu` with two entities + one composition → 3 word rows.
//!   3. Single `.nom` with one concept → 1 concept_defs row.
//!   4. Idempotency: running sync twice does not duplicate rows.
//!   5. `target/`, `.git/`, `node_modules/`, `dist/`, `build/` are skipped.

#[cfg(not(windows))]
mod tests {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use nom_dict::NomDict;

    // ── Helpers ──────────────────────────────────────────────────────

    fn make_tmpdir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("nom-sync-{tag}-{pid}-{nanos}"));
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

    fn open_dict(root: &Path) -> NomDict {
        NomDict::open(root).expect("open NomDict")
    }

    // ── Fixtures ─────────────────────────────────────────────────────

    /// Two-entity + one-composition `.nomtu` fixture (doc 08 §6.3 style).
    const AUTH_FLOW_NOMTU: &str = r#"
the function validate_token_jwt_hmac_sha256 is
  given a token of text, returns yes or no.
  requires the token is non-empty.
  ensures the result reflects whether the token's signature verifies.

the function refresh_token_oauth_silent is
  given a refresh token of text, returns a new access token.
  ensures the old refresh token is invalidated after use.

the module auth_flow_compose composes
  the function validate_token_jwt_hmac_sha256 then
  the function refresh_token_oauth_silent
  with "validate first; refresh only when valid."
  ensures no stale token is issued.
"#;

    /// Single-concept `.nom` fixture (doc 08 §6.3 authentication_jwt_basic).
    const AUTH_JWT_NOM: &str = r#"
the concept authentication_jwt_basic is
  intended to let users with valid tokens reach the dashboard.

  uses the module auth_flow_compose,
       the function logout_session_invalidate_all_active.

  exposes auth_flow_compose.

  this works when users with valid tokens reach the dashboard
                within two hundred milliseconds.
  this works when invalid tokens are rejected
                before any database read.

  favor security then speed.
"#;

    // ── Test 1: empty repo ────────────────────────────────────────────

    #[test]
    fn sync_empty_repo_exits_ok_with_zero_counts() {
        let repo_dir = make_tmpdir("empty");
        let dict_dir = make_tmpdir("empty-d");

        let (code, stdout, stderr) = run_sync(&repo_dir, &dict_dir);

        assert_eq!(code, 0, "expected exit 0, stderr={stderr}");
        assert!(
            stdout.contains("0 concept(s)"),
            "expected '0 concept(s)' in: {stdout}"
        );
        assert!(
            stdout.contains("0 word(s)"),
            "expected '0 word(s)' in: {stdout}"
        );
    }

    // ── Test 2: single .nomtu with two entities + one composition ─────

    #[test]
    fn sync_nomtu_with_entities_and_composition() {
        let repo_dir = make_tmpdir("nomtu");
        let dict_dir = make_tmpdir("nomtu-d");
        std::fs::write(repo_dir.join("auth_flow.nomtu"), AUTH_FLOW_NOMTU)
            .expect("write fixture");

        let (code, stdout, stderr) = run_sync(&repo_dir, &dict_dir);

        assert_eq!(code, 0, "expected exit 0, stderr={stderr}");
        assert!(
            stdout.contains("0 concept(s)"),
            "expected 0 concepts, got: {stdout}"
        );
        assert!(
            stdout.contains("3 word(s)"),
            "expected 3 words (2 entities + 1 composition), got: {stdout}"
        );
        assert!(
            stdout.contains("2 entities"),
            "expected '2 entities', got: {stdout}"
        );
        assert!(
            stdout.contains("1 compositions"),
            "expected '1 compositions', got: {stdout}"
        );

        // Verify authored_in in DB.
        let dict = open_dict(&dict_dir);
        let jwt_rows = dict
            .find_words_v2_by_word("validate_token_jwt_hmac_sha256")
            .expect("find jwt");
        assert_eq!(jwt_rows.len(), 1, "expected 1 row for validate_token_jwt_hmac_sha256");
        let authored = jwt_rows[0].authored_in.as_deref().unwrap_or("");
        assert!(
            authored.ends_with("auth_flow.nomtu"),
            "authored_in should end with 'auth_flow.nomtu', got: {authored}"
        );

        let comp_rows = dict
            .find_words_v2_by_word("auth_flow_compose")
            .expect("find comp");
        assert_eq!(comp_rows.len(), 1, "expected 1 row for auth_flow_compose");
        assert_eq!(comp_rows[0].kind, "module", "composition kind must be 'module'");
    }

    // ── Test 3: single .nom with one concept ──────────────────────────

    #[test]
    fn sync_nom_concept_upserts_concept_def_row() {
        let repo_dir = make_tmpdir("nom");
        let dict_dir = make_tmpdir("nom-d");
        std::fs::write(repo_dir.join("auth.nom"), AUTH_JWT_NOM).expect("write fixture");

        let (code, stdout, stderr) = run_sync(&repo_dir, &dict_dir);

        assert_eq!(code, 0, "expected exit 0, stderr={stderr}");
        assert!(
            stdout.contains("1 concept(s)"),
            "expected 1 concept, got: {stdout}"
        );

        let dict = open_dict(&dict_dir);
        let row = dict
            .find_concept_def("authentication_jwt_basic")
            .expect("query dict")
            .expect("concept_def row should exist");

        assert_eq!(row.name, "authentication_jwt_basic");
        assert!(
            row.intent.contains("valid tokens reach the dashboard"),
            "intent mismatch, got: {}",
            row.intent
        );

        let objectives: Vec<String> =
            serde_json::from_str(&row.objectives).expect("objectives is valid JSON array");
        assert!(!objectives.is_empty(), "objectives array should not be empty");

        let acceptance: Vec<String> =
            serde_json::from_str(&row.acceptance).expect("acceptance is valid JSON array");
        assert!(!acceptance.is_empty(), "acceptance array should not be empty");
    }

    // ── Test 4: idempotency ───────────────────────────────────────────

    #[test]
    fn sync_is_idempotent_no_duplicate_rows() {
        let repo_dir = make_tmpdir("idem");
        let dict_dir = make_tmpdir("idem-d");
        let fixture =
            "the function hash_password_bcrypt is\n  given a password, returns a digest.\n";
        std::fs::write(repo_dir.join("crypto.nomtu"), fixture).expect("write");

        let (c1, _, e1) = run_sync(&repo_dir, &dict_dir);
        let (c2, _, e2) = run_sync(&repo_dir, &dict_dir);

        assert_eq!(c1, 0, "first run failed: {e1}");
        assert_eq!(c2, 0, "second run failed: {e2}");

        let dict = open_dict(&dict_dir);
        let rows = dict
            .find_words_v2_by_word("hash_password_bcrypt")
            .expect("find");
        assert_eq!(rows.len(), 1, "upsert must not create duplicate rows");
    }

    // ── Test 5: skip excluded directories ────────────────────────────

    #[test]
    fn sync_skips_excluded_directories() {
        let repo_dir = make_tmpdir("skip");
        let dict_dir = make_tmpdir("skip-d");

        let real_fixture =
            "the function real_business_logic is\n  given input, returns output.\n";
        std::fs::write(repo_dir.join("real.nomtu"), real_fixture).expect("write real");

        let ignored_fixture =
            "the function ignored_internal_build is\n  given x, returns y.\n";
        for name in &["target", ".git", "node_modules", "dist", "build"] {
            let dir = repo_dir.join(name);
            std::fs::create_dir_all(&dir).expect("mkdir skip");
            std::fs::write(dir.join("ignored.nomtu"), ignored_fixture).expect("write ignored");
        }

        let (code, stdout, stderr) = run_sync(&repo_dir, &dict_dir);

        assert_eq!(code, 0, "expected exit 0, stderr={stderr}");
        assert!(
            stdout.contains("1 word(s)"),
            "expected only 1 word (real_business_logic), got: {stdout}"
        );

        let dict = open_dict(&dict_dir);
        let ignored = dict
            .find_words_v2_by_word("ignored_internal_build")
            .expect("find ignored");
        assert!(ignored.is_empty(), "ignored_internal_build must not be indexed");
    }
}
