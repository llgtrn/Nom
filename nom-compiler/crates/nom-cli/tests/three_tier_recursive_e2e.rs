//! End-to-end test for M4: three-tier recursive byte ingest (doc 08 §4.3).
//!
//! Verifies that the recursive closure walker propagates word hashes from
//! nested concept refs all the way up to the root concept's build_order.
//!
//! Fixture: concept_demo example tree.
//!   - `app_concept_demo` (app.nom) uses concept `authentication_demo`.
//!   - `authentication_demo` (auth/auth.nom) uses module `auth_session_compose_demo`.
//!   - `auth_session_compose_demo` (auth/auth_helpers.nomtu) composes
//!       `validate_token_demo` (atomic function) +
//!       `issue_session_demo` (atomic function).
//!
//! After sync + manifest, `app_concept_demo`'s build_order must include
//! all three hashes (two atomic functions + one composition module) even
//! though they are defined two tiers below the root concept.
//!
//! Gated to `#[cfg(not(windows))]` — same reason as other e2e tests
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
        let dir = std::env::temp_dir().join(format!("nom-3tier-{tag}-{pid}-{nanos}"));
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

    fn run_manifest(repo: &Path, dict_root: &Path) -> (i32, String, String) {
        let out = Command::new(nom_bin())
            .args([
                "build",
                "manifest",
                &repo.to_string_lossy(),
                "--dict",
                &dict_flag(dict_root),
            ])
            .output()
            .expect("spawn nom build manifest");
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

    fn concept_demo_src() -> PathBuf {
        // CARGO_MANIFEST_DIR for nom-cli is nom-compiler/crates/nom-cli.
        // concept_demo lives at nom-compiler/examples/concept_demo.
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest
            .join("..") // nom-compiler/crates
            .join("..") // nom-compiler
            .join("examples")
            .join("concept_demo")
    }

    // ── main test ────────────────────────────────────────────────────────────

    /// Verifies the three-tier recursive closure: root concept (`app_concept_demo`)
    /// uses nested concept (`authentication_demo`) which uses a module
    /// (`auth_session_compose_demo`) that composes two atomic functions.
    ///
    /// After M4, the root concept's build_order must include the hashes of
    /// both atomic functions and the composition module — not just the direct
    /// concept ref hash.
    #[test]
    fn three_tier_recursive_closure_propagates_nested_hashes() {
        let src = concept_demo_src();
        assert!(
            src.exists(),
            "concept_demo source not found at {}",
            src.display()
        );

        let repo_dir = make_tmpdir("repo");
        let dict_dir = make_tmpdir("dict");
        copy_dir_all(&src, &repo_dir);

        // Step 1: sync — indexes all entities + concepts into the dict.
        let (sc, so, se) = run_sync(&repo_dir, &dict_dir);
        assert_eq!(sc, 0, "sync failed: stderr={se}\nstdout={so}");

        // Step 2: manifest — runs closure walker (including recursive descent).
        let (mc, mo, me) = run_manifest(&repo_dir, &dict_dir);
        // Expect exit 0 (no MECE violations in concept_demo).
        assert_eq!(mc, 0, "manifest failed: stderr={me}\nstdout={mo}");

        // Step 3: parse and assert.
        let v: serde_json::Value = serde_json::from_str(&mo).expect("stdout must be valid JSON");

        let concepts = v["concepts"].as_array().expect("concepts must be array");

        // Find app_concept_demo — the root that references authentication_demo.
        let app_cm = concepts
            .iter()
            .find(|c| c["name"].as_str() == Some("app_concept_demo"))
            .expect("app_concept_demo must be in manifest");

        let build_order = app_cm["build_order"]
            .as_array()
            .expect("build_order must be array");

        // Collect all words in build_order for diagnostics.
        let words: Vec<&str> = build_order
            .iter()
            .filter_map(|b| b["word"].as_str())
            .collect();

        // The two atomic entity words must appear in the root concept's build_order
        // (propagated up transitively through authentication_demo).
        assert!(
            words.contains(&"validate_token_demo"),
            "validate_token_demo must be in app_concept_demo's build_order via recursive descent; got: {words:?}"
        );
        assert!(
            words.contains(&"issue_session_demo"),
            "issue_session_demo must be in app_concept_demo's build_order via recursive descent; got: {words:?}"
        );

        // The composition module must also appear.
        assert!(
            words.contains(&"auth_session_compose_demo"),
            "auth_session_compose_demo must be in app_concept_demo's build_order; got: {words:?}"
        );

        // All three must have non-null, non-empty hashes (they were synced).
        for word in &[
            "validate_token_demo",
            "issue_session_demo",
            "auth_session_compose_demo",
        ] {
            let item = build_order
                .iter()
                .find(|b| b["word"].as_str() == Some(word))
                .unwrap_or_else(|| panic!("could not find {word} in build_order"));
            let hash = item["hash"].as_str().unwrap_or("");
            assert!(
                !hash.is_empty(),
                "{word} must have a non-empty hash in build_order; got: {item:?}"
            );
            assert_eq!(
                hash.len(),
                64,
                "{word} hash must be 64 hex chars; got: {hash}"
            );
        }

        // Post-order: atomic entities must come before the composition module.
        let pos_validate = words
            .iter()
            .position(|w| *w == "validate_token_demo")
            .unwrap();
        let pos_issue = words
            .iter()
            .position(|w| *w == "issue_session_demo")
            .unwrap();
        let pos_compose = words
            .iter()
            .position(|w| *w == "auth_session_compose_demo")
            .unwrap();

        assert!(
            pos_validate < pos_compose,
            "validate_token_demo ({pos_validate}) must come before auth_session_compose_demo ({pos_compose})"
        );
        assert!(
            pos_issue < pos_compose,
            "issue_session_demo ({pos_issue}) must come before auth_session_compose_demo ({pos_compose})"
        );

        // Each word must appear exactly once (deduplication).
        for word in &[
            "validate_token_demo",
            "issue_session_demo",
            "auth_session_compose_demo",
        ] {
            let count = words.iter().filter(|w| **w == *word).count();
            assert_eq!(
                count, 1,
                "{word} must appear exactly once in build_order, found {count}"
            );
        }
    }
}
