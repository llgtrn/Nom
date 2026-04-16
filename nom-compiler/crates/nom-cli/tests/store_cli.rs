//! Integration tests for `nom store` subcommands.
//!
//! Each test runs the `nom` binary produced by cargo against a private
//! temp directory so tests never touch `data/nomdict.db` in the repo.
//! We use `CARGO_BIN_EXE_nom` instead of pulling in `assert_cmd`; this
//! keeps the workspace dependency graph lean.
//!
//! Uses the split-dict (Dict { concepts, entities }) and the S1-S6
//! nom-concept pipeline. Source files use `.nomx` prose syntax.

use std::path::{Path, PathBuf};
use std::process::Command;

// ── Temp dir helpers ──────────────────────────────────────────────────

/// Create a fresh, unique directory under the system temp dir.
fn make_tmpdir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("nom-store-cli-{tag}-{pid}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("create tmp");
    dir
}

/// Return the dict flag value: the directory itself.
/// `open_dict` in store/mod.rs resolves a `.db` extension to the parent dir,
/// so passing the dir directly also works (no `.db` extension → treated as dir).
fn dict_flag(root: &Path) -> String {
    root.to_string_lossy().into_owned()
}

fn nom_bin() -> PathBuf {
    let path = env!("CARGO_BIN_EXE_nom");
    PathBuf::from(path)
}

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

fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, contents).expect("write file");
}

// Minimal parseable .nomx source using prose syntax.
const HELLO_SRC: &str = "the function hello_world is given a name of text, returns text.\n";

// ── Tests ─────────────────────────────────────────────────────────────

#[test]
fn test_store_add_round_trip() {
    let root = make_tmpdir("add-rt");
    let file = root.join("hello.nomx");
    write_file(&file, HELLO_SRC);
    let dict = dict_flag(&root);

    let (code, stdout, stderr) =
        run_nom(&["store", "add", file.to_str().unwrap(), "--dict", &dict]);
    assert_eq!(code, 0, "add exit: {code}\nstderr={stderr}");
    let id = stdout.lines().next().expect("id on stdout").trim();
    assert_eq!(id.len(), 64, "id must be 64 hex chars, got {id:?}");
    assert!(
        id.chars().all(|c| c.is_ascii_hexdigit()),
        "id must be hex: {id:?}"
    );
}

#[test]
fn test_store_closure_three_entries() {
    use nom_dict::{Dict, add_ref, upsert_entry};
    use nom_types::{Contract, Entry, EntryKind, EntryStatus};

    let root = make_tmpdir("closure-3");

    // Open the split dict directly so we can seed entries.
    let d = Dict::open_dir(&root).expect("open Dict");

    let mk = |id: &str, word: &str| Entry {
        id: id.into(),
        word: word.into(),
        variant: None,
        kind: EntryKind::Function,
        language: "nom".into(),
        describe: None,
        concept: None,
        body: None,
        body_nom: Some(format!("the function {word} is\n")),
        body_bytes: None,
        body_kind: None,
        contract: Contract::default(),
        status: EntryStatus::Complete,
        translation_score: None,
        is_canonical: true,
        deprecated_by: None,
        created_at: "2026-04-12T00:00:00Z".into(),
        updated_at: None,
    };

    let a = "a".repeat(64);
    let b = "b".repeat(64);
    let c = "c".repeat(64);
    for (id, w) in [(&a, "alpha"), (&b, "beta"), (&c, "gamma")] {
        upsert_entry(&d, &mk(id, w)).expect("upsert_entry");
    }
    add_ref(&d, &a, &b).expect("add_ref a->b");
    add_ref(&d, &b, &c).expect("add_ref b->c");
    drop(d);

    let dict = dict_flag(&root);
    let (code, stdout, stderr) = run_nom(&["store", "closure", &a, "--dict", &dict]);
    assert_eq!(code, 0, "closure exit: {code}\nstderr={stderr}");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 3, "expected 3 lines, got {lines:?}");
    assert_eq!(lines[0], a);
    assert!(lines.contains(&b.as_str()), "b missing from closure");
    assert!(lines.contains(&c.as_str()), "c missing from closure");
}

#[test]
fn test_store_gc_dry_run() {
    use nom_dict::{Dict, get_entry, upsert_entry};
    use nom_types::{Contract, Entry, EntryKind, EntryStatus};

    let root = make_tmpdir("gc-dry");

    let d = Dict::open_dir(&root).expect("open Dict");

    let live = "1".repeat(64);
    let dead = "2".repeat(64);
    let mk = |id: &str, w: &str| Entry {
        id: id.into(),
        word: w.into(),
        variant: None,
        kind: EntryKind::Function,
        language: "nom".into(),
        describe: None,
        concept: None,
        body: None,
        body_nom: None,
        body_bytes: None,
        body_kind: None,
        contract: Contract::default(),
        status: EntryStatus::Complete,
        translation_score: None,
        is_canonical: true,
        deprecated_by: None,
        created_at: "2026-04-12T00:00:00Z".into(),
        updated_at: None,
    };
    upsert_entry(&d, &mk(&live, "live")).expect("upsert live");
    upsert_entry(&d, &mk(&dead, "dead")).expect("upsert dead");
    drop(d);

    let dict = dict_flag(&root);
    // With no roots file, gc --dry-run should mark both as removable.
    let (code, stdout, _stderr) = run_nom(&["store", "gc", "--dict", &dict, "--dry-run"]);
    assert_eq!(code, 0, "gc dry-run exit: stdout={stdout}");
    assert!(
        stdout.contains("would remove"),
        "stdout missing 'would remove': {stdout}"
    );

    // Confirm the DB still contains both entries after dry-run.
    let d2 = Dict::open_dir(&root).expect("reopen Dict");
    assert!(
        get_entry(&d2, &live).expect("get live").is_some(),
        "dry-run must not delete live row"
    );
    assert!(
        get_entry(&d2, &dead).expect("get dead").is_some(),
        "dry-run must not delete dead row"
    );
}

#[test]
fn test_store_add_json_format() {
    let root = make_tmpdir("add-json");
    let file = root.join("hi.nomx");
    write_file(&file, HELLO_SRC);
    let dict = dict_flag(&root);

    let (code, stdout, stderr) = run_nom(&[
        "store",
        "add",
        file.to_str().unwrap(),
        "--dict",
        &dict,
        "--json",
    ]);
    assert_eq!(code, 0, "json add exit: {code}\nstderr={stderr}");
    let line = stdout.trim();
    assert!(
        line.starts_with('{') && line.ends_with('}'),
        "not JSON: {line:?}"
    );
    assert!(line.contains("\"id\""), "missing id field: {line}");
    assert!(line.contains("\"status\""), "missing status field: {line}");
}
