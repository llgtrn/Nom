//! Smoke tests for `nom corpus register-axis` and `nom corpus list-axes` (M7a).
//!
//! Gated to `#[cfg(not(windows))]` because the `nom` binary links `nom-llvm`
//! and cannot load its DLLs in the cargo-test spawned process on Windows.
//! Run on Linux/macOS CI where the dynamic linker resolves them.

#[cfg(not(windows))]
use std::path::{Path, PathBuf};
#[cfg(not(windows))]
use std::process::Command;

#[cfg(not(windows))]
fn make_tmpdir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("nom-corpus-axes-{tag}-{pid}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("create tmp");
    dir
}

#[cfg(not(windows))]
fn dict_flag(root: &Path) -> String {
    root.join("nomdict.db").to_string_lossy().into_owned()
}

#[cfg(not(windows))]
fn nom_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_nom"))
}

#[cfg(not(windows))]
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

/// Smoke: register-axis succeeds, list-axes shows the registered row.
#[cfg(not(windows))]
#[test]
fn corpus_register_and_list_axes_smoke() {
    let root = make_tmpdir("reg-axes");
    let dict = dict_flag(&root);

    // Register a required axis.
    let (code, stdout, stderr) = run_nom(&[
        "corpus",
        "register-axis",
        "security",
        "--scope",
        "concept",
        "--cardinality",
        "at_least_one",
        "--repo-id",
        "test",
        "--dict",
        &dict,
    ]);
    assert_eq!(code, 0, "register-axis exit code: {code}, stderr={stderr}");
    assert!(
        stdout.contains("registered:"),
        "output must contain 'registered:': {stdout}"
    );
    assert!(
        stdout.contains("axis=security"),
        "output must name axis: {stdout}"
    );
    assert!(
        stdout.contains("scope=concept"),
        "output must name scope: {stdout}"
    );

    // List the registered axis.
    let (code, stdout, stderr) = run_nom(&[
        "corpus",
        "list-axes",
        "--scope",
        "concept",
        "--repo-id",
        "test",
        "--dict",
        &dict,
    ]);
    assert_eq!(code, 0, "list-axes exit code: {code}, stderr={stderr}");
    assert!(
        stdout.contains("security"),
        "list output must include axis 'security': {stdout}"
    );
    assert!(
        stdout.contains("at_least_one"),
        "list output must include cardinality: {stdout}"
    );
}
