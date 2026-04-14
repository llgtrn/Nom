//! End-to-end test for `nom build manifest <repo>`.
//!
//! Exercises the full pipeline:
//!   1. Sync agent_demo into a temp dict.
//!   2. Run `nom build manifest <tempdir>` (compact JSON to stdout).
//!   3. Parse the JSON with serde_json::Value and verify structural invariants.
//!   4. Re-run with `--pretty --out manifest.json`; read file; assert
//!      it parses identically (same concepts, same build_order lengths).
//!
//! Exit code is 1 because the agent_demo has MECE-ME violations (security +
//! speed both declared by parent and child concept).
//!
//! Gated to `#[cfg(not(windows))]` — same reason as agent_demo_e2e.rs.

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
        let dir = std::env::temp_dir().join(format!("nom-manifest-{tag}-{pid}-{nanos}"));
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

    fn run_manifest(
        repo: &Path,
        dict_root: &Path,
        out_file: Option<&Path>,
        pretty: bool,
    ) -> (i32, String, String) {
        let mut args = vec![
            "build".to_string(),
            "manifest".to_string(),
            repo.to_string_lossy().into_owned(),
            "--dict".to_string(),
            dict_flag(dict_root),
        ];
        if pretty {
            args.push("--pretty".to_string());
        }
        if let Some(p) = out_file {
            args.push("--out".to_string());
            args.push(p.to_string_lossy().into_owned());
        }
        let out = Command::new(nom_bin())
            .args(&args)
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

    fn agent_demo_src() -> PathBuf {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest
            .join("..") // nom-compiler/crates
            .join("..") // nom-compiler
            .join("examples")
            .join("agent_demo")
    }

    // ── main test ────────────────────────────────────────────────────────────

    #[test]
    fn build_manifest_agent_demo_full() {
        let src = agent_demo_src();
        assert!(
            src.exists(),
            "agent_demo source not found at {}",
            src.display()
        );

        let repo_dir = make_tmpdir("repo");
        let dict_dir = make_tmpdir("dict");

        copy_dir_all(&src, &repo_dir);

        // ── Step 1: sync ──────────────────────────────────────────────────────
        let (sc, so, se) = run_sync(&repo_dir, &dict_dir);
        assert_eq!(sc, 0, "sync failed: stderr={se}\nstdout={so}");

        // ── Step 2: manifest (compact, stdout) ────────────────────────────────
        let (mc, mo, me) = run_manifest(&repo_dir, &dict_dir, None, false);

        // Exit 1 because MECE-ME violations are present.
        assert_eq!(mc, 1, "expected exit 1 (MECE violations): stderr={me}\nstdout={mo}");

        // JSON must parse.
        let v: serde_json::Value =
            serde_json::from_str(&mo).expect("stdout must be valid JSON");

        // ── Structural assertions ─────────────────────────────────────────────

        // manifest_version == 1
        assert_eq!(
            v["manifest_version"].as_u64().unwrap_or(0),
            1,
            "manifest_version must be 1"
        );

        // Two concepts (minimal_safe_agent + agent_safety_policy).
        let concepts = v["concepts"].as_array().expect("concepts must be an array");
        assert_eq!(
            concepts.len(),
            2,
            "expected 2 concepts, got {}: {:?}",
            concepts.len(),
            concepts
                .iter()
                .map(|c| c["name"].as_str().unwrap_or("?"))
                .collect::<Vec<_>>()
        );

        // Find minimal_safe_agent.
        let agent_cm = concepts
            .iter()
            .find(|c| c["name"].as_str() == Some("minimal_safe_agent"))
            .expect("minimal_safe_agent must be in manifest");

        // build_order must contain >= 6 function items.
        let build_order = agent_cm["build_order"]
            .as_array()
            .expect("build_order must be array");

        let fn_count = build_order
            .iter()
            .filter(|b| b["kind"].as_str() == Some("function"))
            .count();
        assert!(
            fn_count >= 6,
            "expected >= 6 function items in build_order, got {fn_count}: {:?}",
            build_order
                .iter()
                .map(|b| format!("{}:{}", b["kind"].as_str().unwrap_or("?"), b["word"].as_str().unwrap_or("?")))
                .collect::<Vec<_>>()
        );

        // build_order must include a concept ref to agent_safety_policy.
        let has_policy_ref = build_order.iter().any(|b| {
            b["kind"].as_str() == Some("concept")
                && b["word"].as_str() == Some("agent_safety_policy")
        });
        assert!(
            has_policy_ref,
            "agent_safety_policy must appear as a concept in build_order: {:?}",
            build_order
                .iter()
                .map(|b| format!("{}:{}", b["kind"].as_str().unwrap_or("?"), b["word"].as_str().unwrap_or("?")))
                .collect::<Vec<_>>()
        );

        // mece_violations must be non-empty.
        let mece_viol = agent_cm["mece_violations"]
            .as_array()
            .expect("mece_violations must be array");
        assert!(
            !mece_viol.is_empty(),
            "expected MECE violations in minimal_safe_agent"
        );

        // ── Step 3: re-run with --pretty --out manifest.json ──────────────────
        let out_file = dict_dir.join("manifest.json");
        let (mc2, mo2, me2) =
            run_manifest(&repo_dir, &dict_dir, Some(&out_file), true);

        // Still exit 1 (same MECE violations).
        assert_eq!(
            mc2, 1,
            "expected exit 1 from --pretty --out: stderr={me2}\nstdout={mo2}"
        );

        // stdout must be empty when --out is set.
        assert!(
            mo2.trim().is_empty(),
            "stdout must be empty when --out is set, got: {mo2}"
        );

        // The file must exist and parse as an identical manifest.
        let file_content =
            std::fs::read_to_string(&out_file).expect("manifest.json must exist after --out");

        let v2: serde_json::Value =
            serde_json::from_str(&file_content).expect("manifest.json must be valid JSON");

        assert_eq!(
            v["manifest_version"], v2["manifest_version"],
            "manifest_version must match between compact and pretty"
        );

        let concepts2 = v2["concepts"].as_array().expect("concepts2 must be array");
        assert_eq!(
            concepts.len(),
            concepts2.len(),
            "concept count must match between compact and pretty"
        );

        // Verify the concepts appear in the same order with the same names.
        for (a, b) in concepts.iter().zip(concepts2.iter()) {
            assert_eq!(
                a["name"], b["name"],
                "concept names must match between compact and pretty"
            );
            let bo_a = a["build_order"].as_array().map(|x| x.len()).unwrap_or(0);
            let bo_b = b["build_order"].as_array().map(|x| x.len()).unwrap_or(0);
            assert_eq!(
                bo_a, bo_b,
                "build_order length must match for concept {}",
                a["name"]
            );
        }
    }

    // ── effects + typed_slot assertions ──────────────────────────────────────

    /// Sync agent_demo, run manifest, and verify:
    ///   - `fetch_url` build item has 2 effect groups (benefit + hazard).
    ///   - The first group is "benefit" with "cache_hit" in its names.
    ///   - `minimal_safe_agent` build_order contains a typed-slot item with
    ///     `typed_slot=true`, `word=""`, `hash=null`.
    ///   - Existing concept / build-order assertions still pass (regression guard).
    #[test]
    fn build_manifest_effects_and_typed_slot() {
        let src = agent_demo_src();
        assert!(
            src.exists(),
            "agent_demo source not found at {}",
            src.display()
        );

        let repo_dir = make_tmpdir("effects-repo");
        let dict_dir = make_tmpdir("effects-dict");

        copy_dir_all(&src, &repo_dir);

        // Step 1: sync.
        let (sc, so, se) = run_sync(&repo_dir, &dict_dir);
        assert_eq!(sc, 0, "sync failed: stderr={se}\nstdout={so}");

        // Step 2: manifest.
        let (mc, mo, me) = run_manifest(&repo_dir, &dict_dir, None, false);
        // Exit 1 due to MECE violations — same as primary test.
        assert_eq!(mc, 1, "expected exit 1 (MECE violations): stderr={me}\nstdout={mo}");

        let v: serde_json::Value =
            serde_json::from_str(&mo).expect("stdout must be valid JSON");

        let concepts = v["concepts"].as_array().expect("concepts must be array");

        // ── regression: still 2 concepts ─────────────────────────────────────
        assert_eq!(
            concepts.len(),
            2,
            "regression: expected 2 concepts, got {}",
            concepts.len()
        );

        // ── find minimal_safe_agent ───────────────────────────────────────────
        let agent_cm = concepts
            .iter()
            .find(|c| c["name"].as_str() == Some("minimal_safe_agent"))
            .expect("minimal_safe_agent must be in manifest");

        let build_order = agent_cm["build_order"]
            .as_array()
            .expect("build_order must be array");

        // ── effects on fetch_url ──────────────────────────────────────────────
        let fetch_item = build_order
            .iter()
            .find(|b| b["word"].as_str() == Some("fetch_url"))
            .expect("fetch_url must be in build_order");

        let effects = fetch_item["effects"]
            .as_array()
            .expect("fetch_url must have an effects array");

        assert_eq!(
            effects.len(),
            2,
            "fetch_url must have 2 effect groups (benefit + hazard), got {}: {:?}",
            effects.len(),
            effects
        );

        let benefit_group = effects
            .iter()
            .find(|e| e["valence"].as_str() == Some("benefit"))
            .expect("fetch_url must have a benefit group");

        let benefit_names = benefit_group["names"]
            .as_array()
            .expect("benefit group must have names array");

        let has_cache_hit = benefit_names
            .iter()
            .any(|n| n.as_str() == Some("cache_hit"));
        assert!(
            has_cache_hit,
            "cache_hit must be in fetch_url benefit names: {:?}",
            benefit_names
        );

        // ── typed-slot item in build_order ────────────────────────────────────
        // Since the stub resolver now handles typed-slot refs via
        // find_entities_by_kind, the @Function item must resolve to a hash
        // (alphabetically-smallest among the 6 function entities synced from
        // agent_demo's tools/).  The source file is NOT rewritten (per doc 07
        // §3.5 typed-slot lines have no word anchor for @hash splicing).
        let typed_slot_item = build_order
            .iter()
            .find(|b| b["typed_slot"].as_bool() == Some(true))
            .expect("build_order must contain at least one typed-slot item (@Function)");

        assert_eq!(
            typed_slot_item["word"].as_str().unwrap_or("non-empty"),
            "",
            "typed-slot item must have empty word"
        );
        let ts_hash = typed_slot_item["hash"].as_str().unwrap_or("");
        assert!(
            !ts_hash.is_empty(),
            "typed-slot item must now have a resolved hash (stub resolver picks by kind); \
             got hash={}",
            typed_slot_item["hash"]
        );

        // ── regression: build_order still has >= 6 function items ─────────────
        let fn_count = build_order
            .iter()
            .filter(|b| b["kind"].as_str() == Some("function"))
            .count();
        assert!(
            fn_count >= 6,
            "regression: expected >= 6 function items, got {fn_count}"
        );
    }
}
