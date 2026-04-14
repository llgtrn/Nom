//! Round-trip integration tests for `nom store add-media`.
//!
//! Each test calls the store functions directly (not via process spawn) to
//! avoid STATUS_DLL_NOT_FOUND issues on Windows with the LLVM binary. Tests
//! are skipped on Windows via `#[cfg_attr(windows, ignore)]` per convention.
//!
//! The pattern is:
//!   1. `cmd_store_add_media(fixture, dict, json=false)` → expect exit 0.
//!   2. Parse the `id:` line from stdout (captured via a tempfile workaround).
//!   3. `cmd_store_get(id, dict, json=false)` → check body_kind + kind fields.
//!
//! Since the CLI functions print to stdout we test them via the `nom` binary
//! (using `CARGO_BIN_EXE_nom`) to capture output, same pattern as store_cli.rs.

use std::path::{Path, PathBuf};
use std::process::Command;

// ── Helpers ───────────────────────────────────────────────────────────

fn fixtures_dir() -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest)
        .join("..")
        .join("nom-media")
        .join("tests")
        .join("fixtures")
}

fn make_tmpdir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("nom-store-add-media-{tag}-{pid}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("create tmp");
    dir
}

fn dict_flag(root: &Path) -> String {
    root.join("nomdict.db").to_string_lossy().into_owned()
}

fn nom_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_nom"))
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

/// Parse `id: <hash>` from the human-readable add-media output.
fn parse_id_line(stdout: &str) -> Option<String> {
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("id:") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

/// Parse `body_kind: <tag>` from `nom store get` output.
fn parse_body_kind_line(stdout: &str) -> Option<String> {
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("body_kind:") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

/// Parse `kind: <tag>` from `nom store get` output.
fn parse_kind_line(stdout: &str) -> Option<String> {
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("kind:") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

// ── Round-trip test helper ────────────────────────────────────────────

/// Core assertion: add-media the fixture, get it back, verify fields.
///
/// `preserve_format`: if true, passes `--preserve-format` to stay on the
/// per-format track (PNG→PNG, JPEG→JPEG). If false (default), uses the
/// modality-canonical track (PNG/JPEG → AVIF per §4.4.6 invariant 17).
fn assert_add_media_roundtrip(fixture: &Path, expected_body_kind: &str, preserve_format: bool) {
    let root = make_tmpdir(expected_body_kind);
    let dict = dict_flag(&root);

    // Step 1: add-media
    let mut args = vec![
        "store",
        "add-media",
        fixture.to_str().unwrap(),
        "--dict",
        &dict,
    ];
    if preserve_format {
        args.push("--preserve-format");
    }
    let (code, stdout, stderr) = run_nom(&args);
    assert_eq!(
        code, 0,
        "add-media exit={code}\nstdout={stdout}\nstderr={stderr}"
    );

    // Step 2: parse id
    let id = parse_id_line(&stdout).unwrap_or_else(|| panic!("no id: line in output:\n{stdout}"));
    assert_eq!(id.len(), 64, "id must be 64 hex chars, got {id:?}");
    assert!(
        id.chars().all(|c| c.is_ascii_hexdigit()),
        "id not hex: {id}"
    );

    // Step 3: get-entry and verify fields
    let (gcode, gout, gerr) = run_nom(&["store", "get", &id, "--dict", &dict]);
    assert_eq!(gcode, 0, "get exit={gcode}\nstdout={gout}\nstderr={gerr}");

    let body_kind = parse_body_kind_line(&gout)
        .unwrap_or_else(|| panic!("no body_kind: line in get output:\n{gout}"));
    assert_eq!(
        body_kind, expected_body_kind,
        "body_kind mismatch: expected {expected_body_kind}, got {body_kind}"
    );

    let kind =
        parse_kind_line(&gout).unwrap_or_else(|| panic!("no kind: line in get output:\n{gout}"));
    assert_eq!(kind, "media_unit", "kind must be media_unit, got {kind}");
}

// ── §4.4.6 body_bytes round-trip helper ──────────────────────────────

/// Verify that after `add-media`, `NomDict::get_entry_bytes(id)` returns
/// `Some(bytes)` where `sha256(bytes) == id`. This is §4.4.6 invariant 15:
/// the BLOB stored in `body_bytes` is the canonical representation, and
/// the entry `id` is exactly the SHA-256 hex of those bytes.
///
/// `preserve_format`: passed through to the CLI flag; see `assert_add_media_roundtrip`.
#[cfg(not(windows))]
fn assert_body_bytes_stored(fixture: &Path, expected_body_kind: &str, preserve_format: bool) {
    use nom_dict::NomDict;
    use sha2::{Digest, Sha256};

    let root = make_tmpdir(&format!("body-bytes-{expected_body_kind}"));
    let dict = dict_flag(&root);

    // Step 1: add-media via binary
    let mut args = vec![
        "store",
        "add-media",
        fixture.to_str().unwrap(),
        "--dict",
        &dict,
    ];
    let preserve_format_str = "--preserve-format".to_string();
    if preserve_format {
        args.push(&preserve_format_str);
    }
    let (code, stdout, stderr) = run_nom(&args);
    assert_eq!(
        code, 0,
        "add-media exit={code}\nstdout={stdout}\nstderr={stderr}"
    );

    // Step 2: parse id
    let id = parse_id_line(&stdout).unwrap_or_else(|| panic!("no id: line in output:\n{stdout}"));

    // Step 3: open dict directly, call get_entry_bytes
    // NomDict::open expects a root dir that contains data/nomdict.db.
    // The CLI --dict flag points to the .db file directly, so open root.
    let dict = NomDict::open(&root).unwrap();
    let bytes = dict
        .get_entry_bytes(&id)
        .unwrap_or_else(|e| panic!("get_entry_bytes error: {e}"))
        .unwrap_or_else(|| panic!("get_entry_bytes returned None for id={id}"));

    // Step 4: verify sha256(bytes) == id
    let actual_hex = format!("{:x}", Sha256::digest(&bytes));
    assert_eq!(
        actual_hex, id,
        "body_bytes sha256 mismatch for {expected_body_kind}: sha256(bytes)={actual_hex} != id={id}"
    );
}

// ── Tests — per-format track (--preserve-format) ─────────────────────

#[test]
#[cfg(not(windows))]
fn body_bytes_stored_png_preserve_format() {
    // Per-format track: PNG stored as PNG.
    assert_body_bytes_stored(&fixtures_dir().join("tiny.png"), "png", true);
}

#[test]
#[cfg_attr(
    windows,
    ignore = "STATUS_DLL_NOT_FOUND on Windows — run on Linux/macOS"
)]
fn add_media_png_roundtrip_preserve_format() {
    // Per-format track: --preserve-format keeps PNG→PNG.
    assert_add_media_roundtrip(&fixtures_dir().join("tiny.png"), "png", true);
}

#[test]
#[cfg_attr(
    windows,
    ignore = "STATUS_DLL_NOT_FOUND on Windows — run on Linux/macOS"
)]
fn add_media_jpeg_roundtrip_preserve_format() {
    // Per-format track: --preserve-format keeps JPEG→JPEG.
    assert_add_media_roundtrip(&fixtures_dir().join("tiny.jpg"), "jpeg", true);
}

// ── Tests — modality-canonical track (default, no --preserve-format) ──

/// Body-bytes invariant: PNG input → canonical AVIF stored with sha256==id.
#[test]
#[cfg(not(windows))]
fn body_bytes_stored_png_canonical_avif() {
    // Default track: PNG input → AVIF canonical bytes stored.
    assert_body_bytes_stored(&fixtures_dir().join("tiny.png"), "avif", false);
}

/// PNG input with default track → body_kind="avif".
#[test]
#[cfg_attr(
    windows,
    ignore = "STATUS_DLL_NOT_FOUND on Windows — run on Linux/macOS"
)]
fn add_media_png_canonical_avif() {
    assert_add_media_roundtrip(&fixtures_dir().join("tiny.png"), "avif", false);
}

/// JPEG input with default track → body_kind="avif".
#[test]
#[cfg_attr(
    windows,
    ignore = "STATUS_DLL_NOT_FOUND on Windows — run on Linux/macOS"
)]
fn add_media_jpeg_canonical_avif() {
    assert_add_media_roundtrip(&fixtures_dir().join("tiny.jpg"), "avif", false);
}

/// AVIF input with default track → body_kind="avif" (ImageStill canonical).
#[test]
#[cfg_attr(
    windows,
    ignore = "STATUS_DLL_NOT_FOUND on Windows — run on Linux/macOS"
)]
fn add_media_avif_roundtrip() {
    assert_add_media_roundtrip(&fixtures_dir().join("tiny.avif"), "avif", false);
}

// ── Tests — non-image formats (unaffected by preserve_format) ─────────

#[test]
#[cfg_attr(
    windows,
    ignore = "STATUS_DLL_NOT_FOUND on Windows — run on Linux/macOS"
)]
fn add_media_flac_roundtrip() {
    assert_add_media_roundtrip(&fixtures_dir().join("tiny.flac"), "flac", false);
}

#[test]
#[cfg_attr(
    windows,
    ignore = "STATUS_DLL_NOT_FOUND on Windows — run on Linux/macOS"
)]
fn add_media_opus_roundtrip() {
    assert_add_media_roundtrip(&fixtures_dir().join("tiny.opus"), "opus", false);
}

#[test]
#[cfg_attr(
    windows,
    ignore = "STATUS_DLL_NOT_FOUND on Windows — run on Linux/macOS"
)]
fn add_media_aac_roundtrip() {
    assert_add_media_roundtrip(&fixtures_dir().join("tiny.aac"), "aac", false);
}

#[test]
#[cfg_attr(
    windows,
    ignore = "STATUS_DLL_NOT_FOUND on Windows — run on Linux/macOS"
)]
fn add_media_av1_roundtrip() {
    // The fixture uses the .av1.ivf double-extension; extract just the "ivf" ext.
    assert_add_media_roundtrip(&fixtures_dir().join("tiny.av1.ivf"), "av1", false);
}

#[test]
#[cfg_attr(
    windows,
    ignore = "STATUS_DLL_NOT_FOUND on Windows — run on Linux/macOS"
)]
fn add_media_webm_roundtrip() {
    assert_add_media_roundtrip(&fixtures_dir().join("tiny.webm"), "webm", false);
}

#[test]
#[cfg_attr(
    windows,
    ignore = "STATUS_DLL_NOT_FOUND on Windows — run on Linux/macOS"
)]
fn add_media_mp4_roundtrip() {
    assert_add_media_roundtrip(&fixtures_dir().join("tiny.mp4"), "mp4", false);
}

#[test]
#[cfg_attr(
    windows,
    ignore = "STATUS_DLL_NOT_FOUND on Windows — run on Linux/macOS"
)]
fn add_media_hevc_roundtrip() {
    assert_add_media_roundtrip(&fixtures_dir().join("tiny.hevc"), "hevc", false);
}
