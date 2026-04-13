//! Smoke tests for `nom locale` subcommands (M3a scaffold).
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

    fn run(args: &[&str]) -> (i32, String, String) {
        let out = Command::new(nom_bin())
            .args(args)
            .output()
            .expect("spawn nom");
        let code = out.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        (code, stdout, stderr)
    }

    #[test]
    fn locale_list_exits_zero_and_contains_both_packs() {
        let (code, stdout, stderr) = run(&["locale", "list"]);
        assert_eq!(code, 0, "expected exit 0, stderr={stderr}");
        assert!(stdout.contains("vi-VN"), "expected vi-VN in list output: {stdout}");
        assert!(stdout.contains("en-US"), "expected en-US in list output: {stdout}");
    }

    #[test]
    fn locale_validate_known_tag_exits_zero_with_valid() {
        let (code, stdout, stderr) = run(&["locale", "validate", "vi-VN"]);
        assert_eq!(code, 0, "expected exit 0 for vi-VN, stderr={stderr}");
        assert!(
            stdout.contains("valid: vi-VN"),
            "expected 'valid: vi-VN' in output: {stdout}"
        );
    }

    #[test]
    fn locale_validate_bad_tag_exits_one() {
        let (code, _stdout, _stderr) = run(&["locale", "validate", "not_a_tag"]);
        assert_eq!(code, 1, "expected exit 1 for invalid tag");
    }
}
