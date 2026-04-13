//! End-to-end CLI test for modality-canonical AVIF ingestion (§5.16.13 order #5).
//!
//! Pattern: after `nom store add-media <fixture.png>` (no --preserve-format),
//! open the dict directly, assert body_kind="avif", body_bytes>0, sha256==id,
//! run verify_avif_roundtrip → PSNR ≥ 30 dB, and check the invariant query.
//!
//! Gated #[cfg(not(windows))] for process-spawn tests because nom-cli links
//! nom-llvm which needs LLVM-C.dll at startup (STATUS_DLL_NOT_FOUND on Windows).
//! The library-level AVIF round-trip tests run cross-platform (in avif_roundtrip.rs).

#[cfg(not(windows))]
use std::path::{Path, PathBuf};
#[cfg(not(windows))]
use std::process::Command;

#[cfg(not(windows))]
fn fixtures_dir() -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest)
        .join("..")
        .join("nom-media")
        .join("tests")
        .join("fixtures")
}

#[cfg(not(windows))]
fn make_tmpdir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("nom-avif-ingest-{tag}-{pid}-{nanos}"));
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

#[cfg(not(windows))]
fn parse_id_line(stdout: &str) -> Option<String> {
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("id:") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

// ── Main end-to-end test ──────────────────────────────────────────────────────

/// End-to-end: `nom store add-media tiny.png` (default, modality-canonical) →
/// dict row has body_kind="avif", body_bytes > 0, sha256(body_bytes) == id,
/// verify_avif_roundtrip passes PSNR ≥ 30 dB.
///
/// Also checks the invariant query: no avif rows with NULL/empty body_bytes.
#[test]
#[cfg(not(windows))]
fn avif_ingest_png_to_avif() {
    use nom_media::verify_avif_roundtrip;
    use rusqlite::Connection;
    use sha2::{Digest, Sha256};

    let fixture = fixtures_dir().join("tiny.png");
    let root = make_tmpdir("main");
    let dict_path = root.join("nomdict.db");
    let dict_str = dict_path.to_string_lossy().into_owned();

    // ── 1. Ingest PNG via modality-canonical track (default — no --preserve-format)
    let (code, stdout, stderr) = run_nom(&[
        "store",
        "add-media",
        fixture.to_str().unwrap(),
        "--dict",
        &dict_str,
    ]);
    assert_eq!(code, 0, "add-media exit={code}\nstdout={stdout}\nstderr={stderr}");

    // ── 2. Parse id from output
    let id = parse_id_line(&stdout)
        .unwrap_or_else(|| panic!("no id: line in add-media output:\n{stdout}"));
    assert_eq!(id.len(), 64, "id must be 64 hex chars: {id:?}");
    assert!(id.chars().all(|c| c.is_ascii_hexdigit()), "id not hex: {id}");

    // ── 3. Open dict directly; assert body_kind="avif", body_bytes>0
    let conn = Connection::open(&dict_path).expect("open dict");

    let (body_kind, body_bytes_len, body_bytes): (String, usize, Vec<u8>) = conn
        .query_row(
            "SELECT body_kind, LENGTH(body_bytes), body_bytes \
             FROM nomtu WHERE id = ?1",
            rusqlite::params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, usize>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                ))
            },
        )
        .expect("query nomtu row by id");

    assert_eq!(
        body_kind, "avif",
        "body_kind must be 'avif' for modality-canonical PNG ingest, got {body_kind:?}"
    );
    assert!(
        body_bytes_len > 0,
        "body_bytes must be non-empty after avif ingest"
    );
    assert!(
        !body_bytes.is_empty(),
        "body_bytes blob must not be empty"
    );

    // ── 4. sha256(body_bytes) == id  (§4.4.6 invariant 15)
    let actual_hash = format!("{:x}", Sha256::digest(&body_bytes));
    assert_eq!(
        actual_hash, id,
        "sha256(body_bytes) mismatch: expected {id}, got {actual_hash}"
    );

    // ── 5. AVIF container validates + PSNR ≥ 30 dB
    let png_bytes = std::fs::read(&fixture).expect("read fixture");
    let psnr = verify_avif_roundtrip(&png_bytes, &body_bytes)
        .expect("verify_avif_roundtrip must pass on stored AVIF");
    assert!(
        psnr >= 30.0,
        "PSNR {psnr:.2} dB is below 30 dB threshold"
    );

    // ── 6. Invariant gate: no avif rows with NULL/empty body_bytes
    let null_avif_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM nomtu \
             WHERE body_kind = 'avif' \
               AND (body_bytes IS NULL OR length(body_bytes) = 0)",
            [],
            |row| row.get(0),
        )
        .expect("invariant query");
    assert_eq!(
        null_avif_count, 0,
        "invariant violated: {null_avif_count} avif rows have NULL/empty body_bytes"
    );

    // ── 7. Combined invariant: bc + avif rows with NULL body_bytes = 0
    let null_combined: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM nomtu \
             WHERE body_kind IN ('bc', 'avif') \
               AND (body_bytes IS NULL OR length(body_bytes) = 0)",
            [],
            |row| row.get(0),
        )
        .expect("combined invariant query");
    assert_eq!(
        null_combined, 0,
        "combined invariant violated: {null_combined} bc/avif rows have NULL/empty body_bytes"
    );
}

/// Verify --preserve-format stores PNG as PNG, not AVIF.
#[test]
#[cfg(not(windows))]
fn avif_ingest_preserve_format_keeps_png() {
    use rusqlite::Connection;

    let fixture = fixtures_dir().join("tiny.png");
    let root = make_tmpdir("preserve");
    let dict_path = root.join("nomdict.db");
    let dict_str = dict_path.to_string_lossy().into_owned();

    let (code, stdout, stderr) = run_nom(&[
        "store",
        "add-media",
        fixture.to_str().unwrap(),
        "--dict",
        &dict_str,
        "--preserve-format",
    ]);
    assert_eq!(code, 0, "add-media exit={code}\nstdout={stdout}\nstderr={stderr}");

    let id = parse_id_line(&stdout)
        .unwrap_or_else(|| panic!("no id: line:\n{stdout}"));

    let conn = Connection::open(&dict_path).expect("open dict");
    let body_kind: String = conn
        .query_row(
            "SELECT body_kind FROM nomtu WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        )
        .expect("query body_kind");

    assert_eq!(
        body_kind, "png",
        "--preserve-format must store PNG as PNG, got {body_kind:?}"
    );
}
