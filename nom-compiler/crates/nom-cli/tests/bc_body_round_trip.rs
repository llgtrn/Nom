//! Round-trip test: precompile writes body_bytes; load_bc_bytes reads them back.
//!
//! Steps:
//!   1. Create a temp dict and insert a minimal Rust nomtu entry via SQL.
//!   2. Run `nom precompile --dict <tmp>` (uses rustc --emit=llvm-bc).
//!   3. Open the DB directly, assert body_bytes IS NOT NULL and LENGTH > 0.
//!   4. Assert bc_hash == sha256(body_bytes).
//!   5. Assert body_bytes == on-disk .bc file (if artifact_path is non-NULL).
//!   6. Invariant gate: COUNT(*) WHERE body_kind='bc' AND body_bytes IS NULL = 0.
//!
//! The `bc_body_round_trip` CLI test is gated to non-Windows because the `nom`
//! binary links nom-llvm which needs LLVM-C.dll at startup (STATUS_DLL_NOT_FOUND
//! on Windows without LLVM installed). CI on Linux runs it. The `load_bc_bytes_unit`
//! test runs everywhere — it uses the in-memory resolver only.

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
    let dir = std::env::temp_dir().join(format!("nom-bc-rt-{tag}-{pid}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("create tmp");
    dir
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

/// Check whether `rustc --emit=llvm-bc` works at all on this machine.
/// Returns false if rustc is missing or cannot compile LLVM bitcode.
#[cfg(not(windows))]
fn rustc_can_emit_bc() -> bool {
    let tmp = make_tmpdir("probe");
    let rs = tmp.join("probe.rs");
    let bc = tmp.join("probe.bc");
    std::fs::write(&rs, "pub fn probe() {}").ok();
    let status = Command::new("rustc")
        .args([
            "--emit=llvm-bc",
            "--crate-type=cdylib",
            "--edition=2021",
            "-o",
        ])
        .arg(&bc)
        .arg(&rs)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    let ok = status.map(|s| s.success()).unwrap_or(false) && bc.exists();
    let _ = std::fs::remove_dir_all(&tmp);
    ok
}

/// Compute SHA-256 hex of bytes using the same algorithm as nom precompile.
#[cfg(not(windows))]
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
#[cfg(not(windows))]
fn bc_body_round_trip() {
    if !rustc_can_emit_bc() {
        eprintln!("bc_body_round_trip: skipped (rustc cannot emit llvm-bc on this machine)");
        return;
    }

    let root = make_tmpdir("main");
    let dict_path = root.join("nomdict.db");
    let dict_str = dict_path.to_string_lossy().into_owned();
    let bc_dir = root.join("bc-out");
    std::fs::create_dir_all(&bc_dir).expect("bc dir");
    let bc_dir_str = bc_dir.to_string_lossy().into_owned();

    // ── 1. Populate a Rust nomtu entry directly via rusqlite ─────────────────
    // We bypass `nom extract` so the test has no tree-sitter dep and is fast.
    {
        use rusqlite::Connection;
        let conn = Connection::open(&dict_path).expect("open dict");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS nomtu (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                word        TEXT NOT NULL,
                variant     TEXT,
                language    TEXT NOT NULL DEFAULT 'rust',
                body        TEXT,
                body_kind   TEXT,
                body_bytes  BLOB,
                bc_path     TEXT,
                bc_hash     TEXT,
                bc_size     INTEGER,
                artifact_path TEXT,
                UNIQUE(word, variant, language)
            );",
        )
        .expect("create table");
        // Insert a trivial Rust function body that compiles cleanly.
        conn.execute(
            "INSERT OR IGNORE INTO nomtu (word, language, body) VALUES ('add_two', 'rust', 'pub fn add_two(x: i64) -> i64 { x + 2 }')",
            [],
        )
        .expect("insert entry");
    }

    // ── 2. Run nom precompile ─────────────────────────────────────────────────
    let (code, stdout, stderr) = run_nom(&[
        "precompile",
        "--dict",
        &dict_str,
        "--output-dir",
        &bc_dir_str,
        "--word",
        "add_two",
    ]);
    assert_eq!(
        code, 0,
        "nom precompile failed:\nstdout={stdout}\nstderr={stderr}"
    );

    // ── 3. Verify body_bytes in the DB ────────────────────────────────────────
    use rusqlite::Connection;
    let conn = Connection::open(&dict_path).expect("open dict after precompile");

    let (body_bytes, bc_hash_db, artifact_path_opt): (
        Option<Vec<u8>>,
        Option<String>,
        Option<String>,
    ) = conn
        .query_row(
            "SELECT body_bytes, bc_hash, artifact_path FROM nomtu WHERE word = 'add_two'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("query nomtu row");

    let bytes = body_bytes.expect("body_bytes must be non-NULL after successful precompile");
    assert!(
        !bytes.is_empty(),
        "body_bytes must be non-empty after precompile"
    );

    // ── 4. bc_hash == sha256(body_bytes) ─────────────────────────────────────
    let expected_hash = sha256_hex(&bytes);
    let stored_hash = bc_hash_db.expect("bc_hash must be set");
    assert_eq!(
        stored_hash, expected_hash,
        "bc_hash in DB does not match sha256(body_bytes)"
    );

    // ── 5. body_bytes == on-disk .bc file ────────────────────────────────────
    if let Some(ref path) = artifact_path_opt {
        if Path::new(path).exists() {
            let disk_bytes =
                std::fs::read(path).unwrap_or_else(|e| panic!("read artifact_path {path}: {e}"));
            assert_eq!(
                bytes, disk_bytes,
                "body_bytes in DB does not match on-disk .bc file at {path}"
            );
        }
    }

    // ── 6. Invariant gate: no bc rows with NULL body_bytes ───────────────────
    let null_bc_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM nomtu \
             WHERE body_kind = 'bc' \
               AND (body_bytes IS NULL OR length(body_bytes) = 0)",
            [],
            |row| row.get(0),
        )
        .expect("invariant query");
    assert_eq!(
        null_bc_count, 0,
        "invariant 15 violated: {null_bc_count} bc rows have NULL/empty body_bytes"
    );
}

/// Unit test: Resolver::load_bc_bytes returns bytes that were stored in the DB.
#[test]
fn load_bc_bytes_unit() {
    use nom_resolver::Resolver;
    use nom_types::NomtuEntry;

    let resolver = Resolver::open_in_memory().expect("in-memory resolver");

    // Insert a row with known body_bytes.
    let test_bytes: Vec<u8> = b"fake-bitcode-payload".to_vec();
    let mut entry = NomtuEntry::default();
    entry.word = "test_bc".to_owned();
    entry.language = "rust".to_owned();
    entry.body_kind = Some("bc".to_owned());
    entry.body_bytes = Some(test_bytes.clone());
    resolver.upsert(&entry).expect("upsert");

    // Resolve it back to get the integer id.
    let resolved = resolver.resolve_exact("test_bc", None).expect("resolve");

    // load_bc_bytes should return the stored bytes.
    let loaded = resolver.load_bc_bytes(resolved.id).expect("load_bc_bytes");
    assert_eq!(loaded, test_bytes, "load_bc_bytes must return stored bytes");
}
