//! Integration tests for `nom store` subcommands.
//!
//! Each test runs the `nom` binary produced by cargo against a private
//! temp directory so tests never touch `data/nomdict.db` in the repo.
//! We use `CARGO_BIN_EXE_nom` instead of pulling in `assert_cmd`; this
//! keeps the workspace dependency graph lean.

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

/// Return the directory treated as NomDict root. We always pass the
/// `--dict` flag pointing at `<root>/nomdict.db` — the CLI handles the
/// legacy file-path convention and opens `<root>/data/nomdict.db`.
fn dict_flag(root: &Path) -> String {
    let p = root.join("nomdict.db");
    p.to_string_lossy().into_owned()
}

fn nom_bin() -> PathBuf {
    // Cargo sets CARGO_BIN_EXE_<name> for each [[bin]] target during tests.
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

// Tiny parseable .nom source.
const HELLO_SRC: &str = "flow hello_world\n  describe \"prints greeting\"\n";

// ── Tests ─────────────────────────────────────────────────────────────

#[test]
fn test_store_add_round_trip() {
    let root = make_tmpdir("add-rt");
    let file = root.join("hello.nom");
    write_file(&file, HELLO_SRC);
    let dict = dict_flag(&root);

    let (code, stdout, stderr) =
        run_nom(&["store", "add", file.to_str().unwrap(), "--dict", &dict]);
    assert_eq!(code, 0, "add exit: {code}, stderr={stderr}");
    let id = stdout.lines().next().expect("id on stdout").trim();
    assert_eq!(id.len(), 64, "id must be 64 hex chars, got {id:?}");
    assert!(id.chars().all(|c| c.is_ascii_hexdigit()));

    // Round-trip via get (non-JSON).
    let (code, stdout, _) = run_nom(&["store", "get", id, "--dict", &dict]);
    assert_eq!(code, 0);
    assert!(
        stdout.contains("hello_world"),
        "body missing word: {stdout}"
    );
    assert!(
        stdout.contains("flow hello_world"),
        "body_nom missing: {stdout}"
    );
}

#[test]
fn test_store_closure_three_entries() {
    let root = make_tmpdir("closure-3");
    let dict = dict_flag(&root);

    // Each `.nom` file holds a single declaration so its hash is stable
    // across inserts. We chain refs A->B->C by seeding the dict
    // directly via the `add` command and then injecting refs through
    // the CLI? We don't have a `ref` subcommand yet, so we reach into
    // the resolver-less path: for this test we assert the closure tool
    // works when refs exist — we populate them by using the dict
    // connection directly from nom-dict.
    use nom_dict::NomDict;
    use nom_types::{Contract, Entry, EntryKind, EntryStatus};

    // Open the same dict the CLI will see. NomDict stores at
    // <root>/data/nomdict.db so we use `root` itself.
    let d = NomDict::open(&root).unwrap();
    let mk = |id: &str, word: &str| Entry {
        id: id.into(),
        word: word.into(),
        variant: None,
        kind: EntryKind::Function,
        language: "nom".into(),
        describe: None,
        concept: None,
        body: None,
        body_nom: Some(format!("flow {word}\n")),
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
    for (id, w) in [(&a, "A"), (&b, "B"), (&c, "C")] {
        d.upsert_entry(&mk(id, w)).unwrap();
    }
    d.add_ref(&a, &b).unwrap();
    d.add_ref(&b, &c).unwrap();
    drop(d);

    let (code, stdout, stderr) = run_nom(&["store", "closure", &a, "--dict", &dict]);
    assert_eq!(code, 0, "closure exit: {code}, stderr={stderr}");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 3, "expected 3 lines, got {lines:?}");
    assert_eq!(lines[0], a);
    assert!(lines.contains(&b.as_str()));
    assert!(lines.contains(&c.as_str()));
}

#[test]
fn test_store_verify_broken_ref() {
    let root = make_tmpdir("verify-broken");
    let dict = dict_flag(&root);
    use nom_dict::NomDict;
    use nom_types::{Contract, Entry, EntryKind, EntryStatus};

    let d = NomDict::open(&root).unwrap();
    let a = "a".repeat(64);
    let b = "b".repeat(64);
    let ghost = "f".repeat(64);
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
    d.upsert_entry(&mk(&a, "A")).unwrap();
    d.upsert_entry(&mk(&b, "B")).unwrap();
    // Bypass FK by inserting the ghost target and then deleting it.
    d.upsert_entry(&mk(&ghost, "GHOST")).unwrap();
    d.add_ref(&a, &b).unwrap();
    d.add_ref(&b, &ghost).unwrap();
    // Now delete GHOST: FK on entry_refs.to_id has no ON DELETE CASCADE,
    // but it's a plain REFERENCES — attempting DELETE should either
    // cascade (it doesn't, per schema) or fail. We use a raw pragma to
    // disable FK just for this DELETE so we can synthesise a broken ref.
    d.connection()
        .execute("PRAGMA foreign_keys = OFF", [])
        .unwrap();
    d.connection()
        .execute("DELETE FROM entries WHERE id = ?1", [&ghost])
        .unwrap();
    d.connection()
        .execute("PRAGMA foreign_keys = ON", [])
        .unwrap();
    drop(d);

    let (code, stdout, _) = run_nom(&["store", "verify", &a, "--dict", &dict]);
    assert_eq!(code, 2, "expected exit 2 for broken ref, stdout={stdout}");
    assert!(stdout.contains("broken"), "stdout missing broken: {stdout}");
}

#[test]
fn test_store_gc_dry_run() {
    let root = make_tmpdir("gc-dry");
    let dict = dict_flag(&root);
    use nom_dict::NomDict;
    use nom_types::{Contract, Entry, EntryKind, EntryStatus};

    let d = NomDict::open(&root).unwrap();
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
    d.upsert_entry(&mk(&live, "live")).unwrap();
    d.upsert_entry(&mk(&dead, "dead")).unwrap();
    drop(d);

    // With no roots file, gc --dry-run should mark both as removable
    // and keep 0. Ensure no warnings turn into non-zero exit.
    let (code, stdout, _stderr) = run_nom(&["store", "gc", "--dict", &dict, "--dry-run"]);
    assert_eq!(code, 0, "gc dry-run exit: stdout={stdout}");
    assert!(
        stdout.contains("would remove"),
        "stdout missing label: {stdout}"
    );
    // Confirm the DB still contains both entries after dry-run.
    let d2 = NomDict::open(&root).unwrap();
    assert_eq!(d2.count().unwrap(), 2, "dry-run must not delete rows");
}

#[test]
fn test_store_get_prefix_match() {
    let root = make_tmpdir("get-prefix");
    let dict = dict_flag(&root);
    use nom_dict::NomDict;
    use nom_types::{Contract, Entry, EntryKind, EntryStatus};

    let id = format!("{}{}", "ab12cd34", "5".repeat(56));
    let d = NomDict::open(&root).unwrap();
    d.upsert_entry(&Entry {
        id: id.clone(),
        word: "hi".into(),
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
    })
    .unwrap();
    drop(d);

    // 8-char unique prefix should resolve.
    let (code, stdout, _) = run_nom(&["store", "get", "ab12cd34", "--dict", &dict]);
    assert_eq!(code, 0);
    assert!(
        stdout.contains(&id),
        "full id missing from output: {stdout}"
    );
}

#[test]
fn test_store_get_ambiguous_prefix() {
    let root = make_tmpdir("get-ambig");
    let dict = dict_flag(&root);
    use nom_dict::NomDict;
    use nom_types::{Contract, Entry, EntryKind, EntryStatus};

    let id1 = format!("{}{}", "abcdef00", "1".repeat(56));
    let id2 = format!("{}{}", "abcdef00", "2".repeat(56));
    let d = NomDict::open(&root).unwrap();
    for (id, w) in [(&id1, "a"), (&id2, "b")] {
        d.upsert_entry(&Entry {
            id: id.clone(),
            word: w.to_string(),
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
        })
        .unwrap();
    }
    drop(d);

    let (code, _stdout, stderr) = run_nom(&["store", "get", "abcdef00", "--dict", &dict]);
    assert_ne!(code, 0, "ambiguous prefix must not return success");
    assert!(
        stderr.contains("ambiguous") || stderr.contains("candidates"),
        "stderr missing ambiguity notice: {stderr}"
    );
}

#[test]
fn test_nom_build_by_hash() {
    let root = make_tmpdir("build-by-hash");
    let dict = dict_flag(&root);
    use nom_dict::NomDict;
    use nom_types::{Contract, Entry, EntryKind, EntryStatus};

    let id = format!("{}{}", "beef1234", "a".repeat(56));
    let d = NomDict::open(&root).unwrap();
    d.upsert_entry(&Entry {
        id: id.clone(),
        word: "hello".into(),
        variant: None,
        kind: EntryKind::Function,
        language: "nom".into(),
        describe: None,
        concept: None,
        body: None,
        // Minimal parseable body — a single flow declaration.
        body_nom: Some("flow hello\n".to_string()),
        body_bytes: None,
        body_kind: None,
        contract: Contract::default(),
        status: EntryStatus::Complete,
        translation_score: None,
        is_canonical: true,
        deprecated_by: None,
        created_at: "2026-04-12T00:00:00Z".into(),
        updated_at: None,
    })
    .unwrap();
    drop(d);

    // Attempt the build. We only verify the hash-prefix detection
    // produces the "materialized N closure entries" banner. The full
    // build pipeline may still error on stubs like missing dict
    // entries — that's downstream of this task's contract.
    let (_code, stdout, stderr) = run_nom(&["build", &id, "--dict", &dict, "--no-prelude"]);
    let combined = format!("{stdout}\n{stderr}");
    assert!(
        combined.contains("materialized") && combined.contains("closure entries"),
        "hash-prefix build path not taken:\nstdout={stdout}\nstderr={stderr}"
    );
}

#[test]
fn test_store_add_json_format() {
    let root = make_tmpdir("add-json");
    let file = root.join("hi.nom");
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
    assert_eq!(code, 0, "json add exit: {code}, stderr={stderr}");
    let line = stdout.trim();
    assert!(
        line.starts_with('{') && line.ends_with('}'),
        "not JSON: {line:?}"
    );
    assert!(line.contains("\"id\""), "missing id field: {line}");
    assert!(line.contains("\"status\""), "missing status field: {line}");
}
